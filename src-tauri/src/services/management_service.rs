//! Visão gerencial (spec §12.1): números e séries do período para o administrador. Somente leitura.

use chrono::{Duration, NaiveDate};

use super::Ctx;
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::dates;
use crate::domain::payment::Method;
use crate::error::AppError;
use crate::models::management::{BalanceSummary, DayNet, DayOccupancy, ManagementDashboard, MethodNet, PayablesSummary};
use crate::repositories::{closing_repo, management_repo, stay_repo};

const MAX_DAYS: i64 = 366;

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

#[derive(Clone)]
pub struct ManagementService {
    ctx: Ctx,
}

impl ManagementService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    pub fn dashboard(&self, token: &str, from: &str, to: &str) -> Result<ManagementDashboard, AppError> {
        let info = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&info, Permission::ViewFinancialReports)?;
        let (from_date, to_date) = match (dates::parse(from), dates::parse(to)) {
            (Some(f), Some(t)) => (f, t),
            _ => return Err(invalid("Informe o período com datas válidas.")),
        };
        if to_date < from_date {
            return Err(invalid("A data final não pode ser antes da inicial."));
        }
        if (to_date - from_date).num_days() + 1 > MAX_DAYS {
            return Err(invalid("O período pode ter no máximo 366 dias."));
        }

        let offset = self.ctx.utc.local_offset();
        let (start, end) = dates::utc_bounds(from_date, to_date, offset);
        let today = dates::format(self.ctx.today());
        let conn = self.ctx.conn()?;

        let rooms_total = management_repo::active_rooms(&conn)?;
        let nights = management_repo::occupied_nights(&conn, from, to)?;
        let days: Vec<NaiveDate> = (0..(to_date - from_date).num_days() + 1).map(|n| from_date + Duration::days(n)).collect();
        let occupancy = days
            .iter()
            .map(|d| {
                let date = dates::format(*d);
                let occupied = nights.get(&date).copied().unwrap_or(0);
                let rate_bp = if rooms_total == 0 { 0 } else { (occupied * 20_000 + rooms_total) / (2 * rooms_total) };
                DayOccupancy { date, occupied, rooms: rooms_total, rate_bp }
            })
            .collect();

        let received = closing_repo::received(&conn, &start, &end)?;
        let refunded = closing_repo::refunded(&conn, &start, &end)?;
        let mut methods: Vec<Method> = Method::selectable().to_vec();
        if received.contains_key("unspecified") || refunded.contains_key("unspecified") {
            methods.push(Method::Unspecified);
        }
        let receipts_by_method = methods
            .into_iter()
            .map(|m| {
                let received_cents = received.get(m.as_str()).copied().unwrap_or(0);
                let refunded_cents = refunded.get(m.as_str()).copied().unwrap_or(0);
                MethodNet {
                    method: m.as_str().into(),
                    label: m.label().into(),
                    received_cents,
                    refunded_cents,
                    net_cents: received_cents - refunded_cents,
                }
            })
            .collect();

        let offset_secs = offset.local_minus_utc();
        let in_by_day = management_repo::received_daily(&conn, offset_secs, &start, &end)?;
        let out_by_day = management_repo::refunded_daily(&conn, offset_secs, &start, &end)?;
        let receipts_daily = days
            .iter()
            .map(|d| {
                let date = dates::format(*d);
                let net_cents = in_by_day.get(&date).copied().unwrap_or(0) - out_by_day.get(&date).copied().unwrap_or(0);
                DayNet { date, net_cents }
            })
            .collect();

        let mut balances = BalanceSummary { to_receive_count: 0, to_receive_cents: 0, credit_count: 0, credit_cents: 0 };
        for id in management_repo::in_house_ids(&conn)? {
            let balance = stay_repo::balance(&conn, id)?;
            if balance > 0 {
                balances.to_receive_count += 1;
                balances.to_receive_cents += balance;
            } else if balance < 0 {
                balances.credit_count += 1;
                balances.credit_cents += -balance;
            }
        }

        let (overdue_count, overdue_cents) = management_repo::payables_overdue(&conn, &today)?;
        let by_category = management_repo::payables_open_by_category(&conn)?;
        let payables = PayablesSummary {
            overdue_count,
            overdue_cents,
            open_cents: by_category.iter().map(|c| c.open_cents).sum(),
            by_category,
        };

        Ok(ManagementDashboard {
            from: from.into(),
            to: to.into(),
            rooms_total,
            occupancy,
            arrivals: management_repo::arrivals(&conn, from, to)?,
            departures: management_repo::departures(&conn, from, to)?,
            in_house_now: management_repo::in_house(&conn)?,
            receipts_by_method,
            receipts_daily,
            balances,
            payables,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::models::payable::{NewPayable, PayInput};
    use crate::models::payment::ComponentInput;
    use crate::models::refund::RefundInput;
    use crate::models::stay::{CheckInInput, OccupantInput};
    use crate::services::test_support::{admin_token, book, harness, pay, recepcao_token, seed, Harness, Seed};

    // Fuso -03:00. "Hoje" é 21/10/2026, 09:00 local. Diária R$ 200,00; um quarto (101).

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

    fn dash(w: &World, from: &str, to: &str) -> ManagementDashboard {
        w.h.management.dashboard(&w.admin, from, to).unwrap()
    }

    fn check_in(w: &World, reservation: i64) -> i64 {
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        w.h.stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
            .unwrap()
    }

    fn method<'a>(d: &'a ManagementDashboard, key: &str) -> &'a MethodNet {
        d.receipts_by_method.iter().find(|m| m.method == key).unwrap()
    }

    #[test]
    fn reception_cannot_open_the_dashboard() {
        let w = world();
        let reception = recepcao_token(&w.h, &w.admin);

        let result = w.h.management.dashboard(&reception, "2026-10-21", "2026-10-21");

        assert!(matches!(result, Err(AppError::Forbidden)), "{result:?}");
    }

    #[test]
    fn invalid_periods_are_refused() {
        let w = world();
        for (from, to) in [("2026-10-22", "2026-10-21"), ("2026-01-01", "2027-01-02"), ("2026-13-01", "2026-13-02"), ("", "2026-10-21")] {
            let result = w.h.management.dashboard(&w.admin, from, to);
            assert!(matches!(result, Err(AppError::Validation(_))), "{from} a {to}: {result:?}");
        }
        assert!(w.h.management.dashboard(&w.admin, "2026-01-01", "2027-01-01").is_ok(), "366 dias cabem");
    }

    #[test]
    fn occupancy_counts_active_reservation_nights_and_ignores_cancelled() {
        let w = world();
        book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let cancelled = book(&w.h, &w.admin, &w.seed, "2026-10-25", "2026-10-26");
        let version = w.h.reservations.get(&w.admin, cancelled).unwrap().reservation.version;
        w.h.reservations.cancel(&w.admin, cancelled, version, None).unwrap();

        let d = dash(&w, "2026-10-21", "2026-10-25");

        let rates: Vec<(String, i64)> = d.occupancy.iter().map(|o| (o.date.clone(), o.rate_bp)).collect();
        assert_eq!(d.rooms_total, 1);
        assert_eq!(
            rates,
            [("2026-10-21", 10_000), ("2026-10-22", 10_000), ("2026-10-23", 0), ("2026-10-24", 0), ("2026-10-25", 0)]
                .map(|(date, bp)| (date.to_string(), bp))
        );
    }

    #[test]
    fn occupancy_with_no_active_rooms_is_zero_not_a_division_error() {
        let w = world();
        w.h.catalog.set_room_active(&w.admin, w.seed.room_id, false).unwrap();

        let d = dash(&w, "2026-10-21", "2026-10-21");

        assert_eq!((d.rooms_total, d.occupancy[0].rate_bp), (0, 0));
    }

    #[test]
    fn arrivals_departures_and_in_house_count_the_period() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        at(&w, "2026-10-21T17:00:00Z");
        check_in(&w, reservation);

        let today = dash(&w, "2026-10-21", "2026-10-21");
        let later = dash(&w, "2026-10-22", "2026-10-23");

        assert_eq!((today.arrivals, today.departures, today.in_house_now), (1, 0, 1));
        assert_eq!((later.arrivals, later.departures), (0, 1));
    }

    #[test]
    fn receipts_are_net_of_refunds_by_method_and_by_local_day() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        at(&w, "2026-10-21T17:00:00Z");
        let stay = check_in(&w, reservation);
        at(&w, "2026-10-22T02:30:00Z"); // 23:30 de 21/10 no horário local
        let receipt = pay(&w.h, &w.admin, stay, 10_000, "pix").unwrap();
        at(&w, "2026-10-22T15:00:00Z"); // 12:00 de 22/10
        pay(&w.h, &w.admin, stay, 4_000, "cash").unwrap();
        w.h.refunds
            .record(
                &w.admin,
                RefundInput {
                    payment_id: receipt.id,
                    components: vec![ComponentInput { method: "cash".into(), amount_cents: 2_000 }],
                    reason: "Ajuste".into(),
                },
            )
            .unwrap();

        let d = dash(&w, "2026-10-21", "2026-10-22");

        let daily: Vec<(String, i64)> = d.receipts_daily.iter().map(|x| (x.date.clone(), x.net_cents)).collect();
        assert_eq!(daily, [("2026-10-21".to_string(), 10_000), ("2026-10-22".to_string(), 2_000)]);
        assert_eq!((method(&d, "pix").received_cents, method(&d, "pix").net_cents), (10_000, 10_000));
        let cash = method(&d, "cash");
        assert_eq!((cash.received_cents, cash.refunded_cents, cash.net_cents), (4_000, 2_000, 2_000));
        assert_eq!(method(&d, "debit").label, "Cartão de débito");
        assert!(d.receipts_by_method.iter().all(|m| m.method != "unspecified"));
    }

    #[test]
    fn a_refund_bigger_than_the_day_receipts_gives_a_negative_day() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        at(&w, "2026-10-21T17:00:00Z");
        let stay = check_in(&w, reservation);
        let receipt = pay(&w.h, &w.admin, stay, 10_000, "cash").unwrap();
        at(&w, "2026-10-22T15:00:00Z");
        w.h.refunds
            .record(
                &w.admin,
                RefundInput {
                    payment_id: receipt.id,
                    components: vec![ComponentInput { method: "cash".into(), amount_cents: 3_000 }],
                    reason: "Ajuste".into(),
                },
            )
            .unwrap();

        let d = dash(&w, "2026-10-21", "2026-10-22");

        assert_eq!(d.receipts_daily.iter().map(|x| x.net_cents).collect::<Vec<_>>(), [10_000, -3_000]);
    }

    #[test]
    fn days_without_movement_are_present_with_zero() {
        let w = world();

        let d = dash(&w, "2026-10-21", "2026-10-23");

        assert_eq!(d.receipts_daily.len(), 3);
        assert!(d.receipts_daily.iter().all(|x| x.net_cents == 0));
        assert_eq!(d.receipts_by_method.len(), 5);
    }

    #[test]
    fn balances_sum_in_house_stays_to_receive_and_credit_separately() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        at(&w, "2026-10-21T17:00:00Z");
        let stay = check_in(&w, reservation);
        let balance = w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents;

        let owing = dash(&w, "2026-10-21", "2026-10-21").balances;
        assert_eq!((owing.to_receive_count, owing.to_receive_cents, owing.credit_count), (1, balance, 0));

        pay(&w.h, &w.admin, stay, balance + 1_500, "pix").unwrap_or_else(|e| panic!("{e:?}"));
        let credit = dash(&w, "2026-10-21", "2026-10-21").balances;
        assert_eq!((credit.to_receive_count, credit.credit_count, credit.credit_cents), (0, 1, 1_500));
    }

    #[test]
    fn payables_summary_has_overdue_open_and_by_category() {
        let w = world();
        let energy = w.h.payables.create_category(&w.admin, "Energia do mês").unwrap();
        let water = w.h.payables.create_category(&w.admin, "Água do mês").unwrap();
        let bill = |category_id: i64, cents: i64, due: &str| NewPayable {
            supplier: "Fornecedor".into(),
            description: "Conta".into(),
            amount_cents: cents,
            competence_date: "2026-10-01".into(),
            due_date: due.into(),
            category_id,
            notes: String::new(),
            paid_now: None,
        };
        w.h.payables.create(&w.admin, bill(energy, 50_000, "2026-10-20")).unwrap(); // vencida
        w.h.payables.create(&w.admin, bill(energy, 10_000, "2026-10-21")).unwrap(); // vence hoje: não é vencida
        let paid = w.h.payables.create(&w.admin, bill(water, 8_000, "2026-10-10")).unwrap();
        let version = w.h.payables.get(&w.admin, paid).unwrap().payable.version;
        w.h.payables.pay(&w.admin, paid, PayInput { amount_cents: 8_000, method: "cash".into(), note: String::new() }, version).unwrap();

        let p = dash(&w, "2026-10-21", "2026-10-21").payables;

        assert_eq!((p.overdue_count, p.overdue_cents, p.open_cents), (1, 50_000, 60_000));
        assert_eq!(p.by_category.len(), 1, "a conta quitada não entra");
        assert_eq!((p.by_category[0].category_name.as_str(), p.by_category[0].open_cents), ("Energia do mês", 60_000));
    }

    #[test]
    fn default_one_day_period_returns_a_single_day() {
        let w = world();

        let d = dash(&w, "2026-10-21", "2026-10-21");

        assert_eq!((d.occupancy.len(), d.receipts_daily.len()), (1, 1));
    }
}
