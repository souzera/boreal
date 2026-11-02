//! Visão da Recepção (spec §12.2): chegadas, saídas, ocupação e riscos do dia numa só leitura,
//! composta a partir dos serviços e repositórios que já guardam as regras.

use chrono::Duration;

use super::alert_service::AlertService;
use super::reservation_service::{parse_date, parse_time};
use super::room_status_service::RoomStatusService;
use super::stay_service::StayService;
use super::Ctx;
use crate::domain::dates;
use crate::error::AppError;
use crate::models::alert::AlertQuery;
use crate::models::reception::{ArrivalItem, DepartureItem, ReceptionOverview, ReceptionSummary, RiskItem};
use crate::models::search::Group;
use crate::repositories::{reservation_repo, settings_repo, stay_repo};

/// Itens por lista; o total completo sempre acompanha.
const LIMIT: usize = 50;

fn group<T>(mut all: Vec<T>) -> Group<T> {
    let total = all.len() as i64;
    all.truncate(LIMIT);
    Group { items: all, total }
}

#[derive(Clone)]
pub struct ReceptionService {
    ctx: Ctx,
    alerts: AlertService,
    rooms: RoomStatusService,
}

impl ReceptionService {
    pub fn new(ctx: Ctx) -> Self {
        Self { alerts: AlertService::new(ctx.clone()), rooms: RoomStatusService::new(ctx.clone()), ctx }
    }

    pub fn overview(&self, token: &str) -> Result<ReceptionOverview, AppError> {
        // Sessão válida antes de qualquer leitura; os serviços compostos também a renovam.
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let today = self.ctx.today();
        let now = self.ctx.local_now();

        let (arrivals, departures) = {
            let conn = self.ctx.conn()?;
            let settings = settings_repo::load(&conn)?;

            let mut arrivals = Vec::new();
            for r in reservation_repo::pending_arrivals(&conn, today)? {
                let date = parse_date(&r.check_in_date, "Data de entrada")?;
                let time = parse_time(&r.expected_check_in_time)
                    .ok_or_else(|| AppError::Database("horário previsto inválido na reserva".into()))?;
                let overdue = now >= date.and_time(time) + Duration::minutes(i64::from(settings.no_show_tolerance_minutes));
                arrivals.push(ArrivalItem {
                    reservation_id: r.id,
                    code: r.code,
                    guest_name: r.guest_name,
                    room_code: r.room_code,
                    check_in_date: r.check_in_date,
                    expected_time: r.expected_check_in_time,
                    status: r.status,
                    overdue,
                    deposit_overdue: r.deposit_overdue,
                });
            }

            let mut departures = Vec::new();
            for stay in stay_repo::list(&conn, None, "in_house", 1000, 0)?.0 {
                if parse_date(&stay.check_out_date, "Data de saída")? > today {
                    continue;
                }
                let (late, _) = StayService::assess_departure(&conn, &stay, &settings, now)?;
                departures.push(DepartureItem {
                    stay_id: stay.id,
                    code: stay.code,
                    guest_name: stay.guest_name,
                    room_code: stay.room_code,
                    check_out_date: stay.check_out_date,
                    expected_time: stay.expected_check_out_time,
                    balance_cents: stay_repo::balance(&conn, stay.id)?,
                    minutes_late: late.minutes_late,
                });
            }
            (arrivals, departures)
        };

        let rooms = self.rooms.list(token)?;
        let count = |state: &str| rooms.iter().filter(|r| r.state == state).count() as i64;

        let mut risks = Vec::new();
        let mut risk_total = 0;
        for kind in ["departure_risk", "maintenance_conflict"] {
            let page = self.alerts.list(
                token,
                AlertQuery { status: Some("active".into()), kind: Some(kind.into()), severity: None, page: 1, page_size: LIMIT as u32 },
            )?;
            risk_total += page.total as i64;
            risks.extend(page.items.into_iter().map(|a| RiskItem { kind: a.kind, title: a.title, detail: a.detail, link: a.link }));
        }
        risks.truncate(LIMIT);

        let summary = ReceptionSummary {
            arrivals: arrivals.len() as i64,
            departures: departures.len() as i64,
            occupied_rooms: count("occupied"),
            available_rooms: count("available"),
            total_rooms: rooms.len() as i64,
            critical_alerts: self.alerts.summary(token)?.critical,
        };
        Ok(ReceptionOverview {
            date: dates::format(today),
            summary,
            arrivals: group(arrivals),
            departures: group(departures),
            at_risk: Group { items: risks, total: risk_total },
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::models::block::NewBlockInput;
    use crate::models::catalog::NewRoom;
    use crate::models::maintenance::NewOrderInput;
    use crate::models::reservation::NewReservation;
    use crate::models::stay::{CheckInInput, OccupantInput};
    use crate::services::test_support::{admin_token, book, harness, pay, recepcao_token, seed, Harness, Seed};

    // Fuso -03:00. "Hoje" é 21/10/2026, 09:00 local. Check-in padrão 14:00, saída 12:00; tolerâncias 60 min (no-show) e 30 min (saída).

    struct World {
        h: Harness,
        admin: String,
        seed: Seed,
    }

    fn world() -> World {
        let h = harness();
        let admin = admin_token(&h);
        let seed = seed(&h, &admin, 20_000, 2);
        World { h, admin, seed }
    }

    fn at(w: &World, rfc3339: &str) {
        w.h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    fn room(w: &World, code: &str) -> i64 {
        w.h.catalog
            .create_room(
                &w.admin,
                NewRoom { code: code.into(), category_id: w.seed.category_id, max_capacity: 2, description: String::new(), notes: String::new() },
            )
            .unwrap()
    }

    fn book_in(w: &World, room_id: i64, from: &str, to: &str) -> i64 {
        w.h.reservations
            .create(
                &w.admin,
                NewReservation {
                    room_id,
                    guest_id: w.seed.guest_id,
                    check_in_date: from.into(),
                    check_out_date: to.into(),
                    expected_check_in_time: None,
                    expected_check_out_time: None,
                    occupants: 1,
                    notes: String::new(),
                    exceed_capacity: false,
                },
            )
            .unwrap()
            .id
    }

    fn check_in(w: &World, reservation: i64) -> i64 {
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        w.h.stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
            .unwrap()
    }

    fn overview(w: &World) -> ReceptionOverview {
        w.h.alerts.refresh(&w.admin).unwrap();
        w.h.reception.overview(&w.admin).unwrap()
    }

    #[test]
    fn counts_and_lists_arrivals_today_and_marks_overdue_ones() {
        let w = world();
        at(&w, "2026-10-20T12:00:00Z");
        let yesterday = book(&w.h, &w.admin, &w.seed, "2026-10-20", "2026-10-21");
        let room2 = room(&w, "102");
        at(&w, "2026-10-21T12:00:00Z");
        let today = book_in(&w, room2, "2026-10-21", "2026-10-23");

        let o = overview(&w);

        assert_eq!(o.date, "2026-10-21");
        assert_eq!(o.summary.arrivals, 2);
        let ids: Vec<i64> = o.arrivals.items.iter().map(|a| a.reservation_id).collect();
        assert_eq!(ids, [yesterday, today], "as atrasadas vêm primeiro");
        assert!(o.arrivals.items[0].overdue, "ontem às 14:00 já passou da tolerância");
        assert!(!o.arrivals.items[1].overdue, "hoje às 14:00 ainda é 09:00");
        assert_eq!(o.arrivals.items[1].expected_time, "14:00");
        assert_eq!(o.arrivals.items[1].room_code, "102");
    }

    #[test]
    fn cancelled_and_checked_in_reservations_are_not_arrivals() {
        let w = world();
        let cancelled = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-22");
        let version = w.h.reservations.get(&w.admin, cancelled).unwrap().reservation.version;
        w.h.reservations.cancel(&w.admin, cancelled, version, None).unwrap();
        let checked = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        at(&w, "2026-10-21T17:00:00Z");
        check_in(&w, checked);

        let o = overview(&w);

        assert_eq!(o.summary.arrivals, 0);
        assert!(o.arrivals.items.is_empty());
    }

    #[test]
    fn lists_departures_today_with_balance_and_minutes_late() {
        let w = world();
        at(&w, "2026-10-20T12:00:00Z");
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-20", "2026-10-21");
        at(&w, "2026-10-20T17:00:00Z");
        let stay = check_in(&w, reservation);
        let balance = w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents;
        pay(&w.h, &w.admin, stay, 5_000, "cash").unwrap();
        at(&w, "2026-10-21T16:00:00Z"); // 13:00 locais; a saída era às 12:00

        let o = overview(&w);

        assert_eq!(o.summary.departures, 1);
        let d = &o.departures.items[0];
        assert_eq!((d.stay_id, d.room_code.as_str(), d.expected_time.as_str()), (stay, "101", "12:00"));
        assert_eq!(d.minutes_late, 60);
        assert_eq!(d.balance_cents, balance - 5_000);
    }

    #[test]
    fn departure_of_a_past_date_still_shows_as_late() {
        let w = world();
        at(&w, "2026-10-20T12:00:00Z");
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-20", "2026-10-21");
        at(&w, "2026-10-20T17:00:00Z");
        check_in(&w, reservation);
        at(&w, "2026-10-22T12:00:00Z");

        let o = overview(&w);

        assert_eq!(o.summary.departures, 1);
        assert_eq!(o.departures.items[0].check_out_date, "2026-10-21");
        assert_eq!(o.departures.items[0].minutes_late, 21 * 60);
    }

    #[test]
    fn future_departures_are_not_listed() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-24");
        at(&w, "2026-10-21T17:00:00Z");
        check_in(&w, reservation);

        assert_eq!(overview(&w).summary.departures, 0);
    }

    #[test]
    fn rooms_count_uses_the_calculated_state() {
        let w = world();
        let blocked = room(&w, "102");
        room(&w, "103");
        let inactive = room(&w, "104");
        w.h.blocks
            .create(&w.admin, NewBlockInput { room_id: blocked, start_date: "2026-10-21".into(), end_date: None, reason: "Pintura".into() })
            .unwrap();
        w.h.catalog.set_room_active(&w.admin, inactive, false).unwrap();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        at(&w, "2026-10-21T17:00:00Z");
        check_in(&w, reservation);

        let s = overview(&w).summary;

        assert_eq!((s.occupied_rooms, s.available_rooms, s.total_rooms), (1, 1, 3));
    }

    #[test]
    fn at_risk_lists_only_critical_alerts() {
        let w = world();
        book(&w.h, &w.admin, &w.seed, "2026-10-25", "2026-10-28");
        let opened = w
            .h
            .maintenance
            .open(
                &w.admin,
                NewOrderInput {
                    room_id: w.seed.room_id,
                    description: "Trocar o chuveiro".into(),
                    priority: "high".into(),
                    planned_start: "2026-10-26".into(),
                    planned_end: Some("2026-10-27".into()),
                    responsible: String::new(),
                    notes: String::new(),
                },
            )
            .unwrap();
        book_in(&w, room(&w, "102"), "2026-10-21", "2026-10-22");

        let o = overview(&w);

        assert_eq!(o.summary.arrivals, 1, "a chegada de hoje aparece em Chegadas, não em risco");
        assert_eq!(o.at_risk.total, 1);
        assert_eq!(o.at_risk.items[0].kind, "maintenance_conflict");
        assert_eq!(o.at_risk.items[0].link, format!("/manutencao/{}", opened.id));
    }

    #[test]
    fn critical_alerts_matches_the_alert_summary() {
        let w = world();
        book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        w.h.alerts.refresh(&w.admin).unwrap();

        let expected = w.h.alerts.summary(&w.admin).unwrap().critical;

        assert_eq!(w.h.reception.overview(&w.admin).unwrap().summary.critical_alerts, expected);
    }

    #[test]
    fn lists_are_limited_to_fifty_with_the_full_total() {
        let w = world();
        for n in 0..55 {
            let id = room(&w, &format!("R{n:02}"));
            book_in(&w, id, "2026-10-21", "2026-10-22");
        }

        let o = overview(&w);

        assert_eq!(o.arrivals.items.len(), 50);
        assert_eq!(o.arrivals.total, 55);
        assert_eq!(o.summary.arrivals, 55);
    }

    #[test]
    fn reception_user_gets_the_overview() {
        let w = world();
        let reception = recepcao_token(&w.h, &w.admin);

        assert!(w.h.reception.overview(&reception).is_ok());
    }

    #[test]
    fn requires_a_session() {
        let w = world();

        assert!(w.h.reception.overview("invalido").is_err());
    }
}
