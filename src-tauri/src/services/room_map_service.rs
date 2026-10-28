use super::room_status_service;
use super::Ctx;
use crate::domain::dates;
use crate::error::AppError;
use crate::models::room_map::{MapCell, RoomMap, RoomRow};
use crate::repositories::reservation_repo;

/// Máximo de dias numa consulta do mapa.
const MAX_DAYS: u32 = 31;

#[derive(Clone)]
pub struct RoomMapService {
    ctx: Ctx,
}

impl RoomMapService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    /// Grade quarto × dia a partir de `from`, por `days` dias (1 a 31). Somente leitura.
    /// Mostra reservas pendentes, confirmadas, hospedadas e concluídas; canceladas e no-show liberam o quarto.
    pub fn room_map(&self, token: &str, from: &str, days: u32) -> Result<RoomMap, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let start = dates::parse(from).ok_or_else(|| AppError::Validation("Data inicial inválida. Use uma data válida.".into()))?;
        if !(1..=MAX_DAYS).contains(&days) {
            return Err(AppError::Validation(format!("O mapa mostra de 1 a {MAX_DAYS} dias por vez.")));
        }
        let end = start + chrono::Duration::days(i64::from(days) - 1);

        let conn = self.ctx.conn()?;
        let statuses = room_status_service::statuses(&self.ctx, &conn)?;
        let rooms = reservation_repo::map_rooms(&conn)?
            .into_iter()
            .map(|(id, code, category_name, active)| {
                let status = statuses.iter().find(|s| s.room_id == id);
                RoomRow {
                    id,
                    code,
                    category_name,
                    active,
                    state: status.map(|s| s.state.clone()),
                    state_label: status.map(|s| s.label_pt.clone()),
                    free_at: status.and_then(|s| s.free_at.clone()),
                }
            })
            .collect();

        let mut cells = Vec::new();
        for row in reservation_repo::map_reservations(&conn, start, end)? {
            let (Some(check_in), Some(check_out)) = (dates::parse(&row.check_in), dates::parse(&row.check_out)) else {
                return Err(AppError::Database(format!("reserva {} com datas inválidas", row.code)));
            };
            let mut day = check_in.max(start);
            while day <= check_out.min(end) {
                cells.push(MapCell {
                    room_id: row.room_id,
                    date: dates::format(day),
                    reservation_id: row.id,
                    code: row.code.clone(),
                    guest_name: row.guest_name.clone(),
                    status: row.status.clone(),
                    is_check_in: day == check_in,
                    is_check_out: day == check_out,
                });
                day += chrono::Duration::days(1);
            }
        }
        Ok(RoomMap { from: dates::format(start), to: dates::format(end), rooms, cells })
    }
}

#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::models::catalog::{NewCategory, NewRoom};
    use crate::models::guest::GuestInput;
    use crate::models::rate::NewRateRule;
    use crate::models::reservation::{CancellationOverride, NewReservation};
    use crate::services::guest_service::CreateGuestOutcome;
    use crate::services::test_support::{admin_token, harness, Harness};

    struct World {
        h: Harness,
        admin: String,
        room_a: i64,
        room_b: i64,
        guest: i64,
    }

    fn world() -> World {
        let h = harness();
        let admin = admin_token(&h);
        let category = h
            .catalog
            .create_category(&admin, NewCategory { name: "Standard".into(), description: String::new() })
            .unwrap();
        for day in 0..7 {
            h.rates
                .create_rule(
                    &admin,
                    NewRateRule {
                        category_id: category,
                        rule_type: "weekday".into(),
                        weekday: Some(day),
                        start_date: None,
                        end_date: None,
                        date: None,
                        amount_cents: 10_000,
                        label: String::new(),
                    },
                )
                .unwrap();
        }
        let room = |code: &str| {
            h.catalog
                .create_room(
                    &admin,
                    NewRoom { code: code.into(), category_id: category, max_capacity: 2, description: String::new(), notes: String::new() },
                )
                .unwrap()
        };
        let (room_a, room_b) = (room("101"), room("102"));
        let guest = match h
            .guests
            .create(
                &admin,
                GuestInput { full_name: "Ana Souza".into(), phone: "(11) 99999-0000".into(), cpf: None, email: String::new(), address: String::new() },
                true,
            )
            .unwrap()
        {
            CreateGuestOutcome::Created { id, .. } => id,
            other => panic!("{other:?}"),
        };
        World { h, admin, room_a, room_b, guest }
    }

    fn book(w: &World, room: i64, from: &str, to: &str) -> i64 {
        w.h.reservations
            .create(
                &w.admin,
                NewReservation {
                    room_id: room,
                    guest_id: w.guest,
                    check_in_date: from.into(),
                    check_out_date: to.into(),
                    expected_check_in_time: None,
                    expected_check_out_time: None,
                    occupants: 2,
                    notes: String::new(),
                    exceed_capacity: false,
                },
            )
            .unwrap()
            .id
    }

    #[test]
    fn returns_only_reservations_inside_the_period() {
        let w = world();
        let inside = book(&w, w.room_a, "2026-10-25", "2026-10-27");
        book(&w, w.room_a, "2026-11-20", "2026-11-22");

        let map = w.h.room_map.room_map(&w.admin, "2026-10-24", 7).unwrap();

        assert_eq!((map.from.as_str(), map.to.as_str()), ("2026-10-24", "2026-10-30"));
        assert_eq!(map.rooms.len(), 2);
        let dates: Vec<_> = map.cells.iter().map(|c| c.date.as_str()).collect();
        assert_eq!(dates, ["2026-10-25", "2026-10-26", "2026-10-27"]);
        assert!(map.cells.iter().all(|c| c.reservation_id == inside && c.room_id == w.room_a));
    }

    #[test]
    fn marks_check_in_and_check_out_days() {
        let w = world();
        book(&w, w.room_a, "2026-10-25", "2026-10-27");

        let map = w.h.room_map.room_map(&w.admin, "2026-10-24", 7).unwrap();

        let flags: Vec<_> = map.cells.iter().map(|c| (c.is_check_in, c.is_check_out)).collect();
        assert_eq!(flags, [(true, false), (false, false), (false, true)]);
    }

    #[test]
    fn turnover_day_shows_departure_and_arrival() {
        let w = world();
        let leaving = book(&w, w.room_a, "2026-10-25", "2026-10-27");
        let arriving = book(&w, w.room_a, "2026-10-27", "2026-10-29");

        let map = w.h.room_map.room_map(&w.admin, "2026-10-27", 1).unwrap();

        assert_eq!(map.cells.len(), 2);
        let out = map.cells.iter().find(|c| c.reservation_id == leaving).unwrap();
        let into = map.cells.iter().find(|c| c.reservation_id == arriving).unwrap();
        assert!(out.is_check_out && !out.is_check_in);
        assert!(into.is_check_in && !into.is_check_out);
    }

    #[test]
    fn period_starting_mid_stay_shows_the_continuation() {
        let w = world();
        book(&w, w.room_a, "2026-10-22", "2026-10-28");

        let map = w.h.room_map.room_map(&w.admin, "2026-10-25", 2).unwrap();

        let dates: Vec<_> = map.cells.iter().map(|c| (c.date.as_str(), c.is_check_in)).collect();
        assert_eq!(dates, [("2026-10-25", false), ("2026-10-26", false)]);
    }

    #[test]
    fn cancelled_and_no_show_reservations_do_not_appear() {
        let w = world();
        let cancelled = book(&w, w.room_a, "2026-10-25", "2026-10-27");
        let version = w.h.reservations.get(&w.admin, cancelled).unwrap().reservation.version;
        w.h.reservations
            .cancel(&w.admin, cancelled, version, None::<CancellationOverride>)
            .unwrap();
        let kept = book(&w, w.room_b, "2026-10-25", "2026-10-26");

        let map = w.h.room_map.room_map(&w.admin, "2026-10-24", 7).unwrap();

        assert!(map.cells.iter().all(|c| c.reservation_id == kept));
        assert_eq!(map.cells.len(), 2);
    }

    #[test]
    fn inactive_rooms_are_listed_as_inactive() {
        let w = world();
        w.h.db.lock().unwrap().execute("UPDATE rooms SET active = 0 WHERE id = ?1", [w.room_b]).unwrap();

        let map = w.h.room_map.room_map(&w.admin, "2026-10-24", 7).unwrap();

        let inactive: Vec<_> = map.rooms.iter().filter(|r| !r.active).map(|r| r.code.as_str()).collect();
        assert_eq!(inactive, ["102"]);
    }

    #[test]
    fn period_is_limited_to_31_days_and_dates_are_validated() {
        let w = world();

        assert!(w.h.room_map.room_map(&w.admin, "2026-10-24", 31).is_ok());
        for days in [0, 32] {
            assert!(matches!(w.h.room_map.room_map(&w.admin, "2026-10-24", days), Err(AppError::Validation(_))));
        }
        assert!(matches!(w.h.room_map.room_map(&w.admin, "2026-13-40", 7), Err(AppError::Validation(_))));
    }

    #[test]
    fn requires_a_session() {
        let w = world();

        assert!(w.h.room_map.room_map("token-invalido", "2026-10-24", 7).is_err());
    }
}
