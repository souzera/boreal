use chrono::{Duration, NaiveDateTime};
use rusqlite::Connection;

use super::room_rules;
use super::Ctx;
use crate::domain::room_state::{room_state, CleaningFact, CleaningStatus, RoomFacts, RoomState};
use crate::error::AppError;
use crate::models::cleaning::CleaningTask;
use crate::models::room_status::RoomStatus;
use crate::repositories::{block_repo, cleaning_repo, reservation_repo, settings_repo};

fn to_local(utc: &str, offset_seconds: i64) -> Option<NaiveDateTime> {
    chrono::DateTime::parse_from_rfc3339(utc).ok().map(|d| d.naive_utc() + Duration::seconds(offset_seconds))
}

/// Estado de cada quarto ativo, na ordem do código. Calculado a cada leitura a partir de hospedagens,
/// tarefas de limpeza e bloqueios; o decurso do tempo nunca conclui limpeza nem libera bloqueio.
pub(crate) fn statuses(ctx: &Ctx, conn: &Connection) -> Result<Vec<RoomStatus>, AppError> {
    let settings = settings_repo::load(conn)?;
    let now = ctx.local_now();
    let today = ctx.today();
    let offset = i64::from(ctx.utc.local_offset().local_minus_utc());
    let mut out = Vec::new();
    for (room_id, code, category_name, active) in reservation_repo::map_rooms(conn)? {
        if !active {
            continue;
        }
        let task: Option<CleaningTask> = match cleaning_repo::open_for_room(conn, room_id)? {
            Some(open) => Some(open),
            None => cleaning_repo::last_done_for_room(conn, room_id)?,
        };
        let cleaning = match &task {
            Some(t) => Some(CleaningFact {
                status: CleaningStatus::parse(&t.status).ok_or_else(|| AppError::Database("situação de limpeza desconhecida".into()))?,
                completed_at: t.completed_at.as_deref().and_then(|c| to_local(c, offset)),
            }),
            None => None,
        };
        let facts = RoomFacts {
            in_house: reservation_repo::has_in_house(conn, room_id)?,
            cleaning,
            blocked_now: block_repo::active_now(conn, room_id, today)?,
        };
        let state = room_state(&facts, room_rules::cleaning_minutes(conn, room_id, &settings)?, now);
        out.push(RoomStatus {
            room_id,
            code,
            category_name,
            state: state.as_str().into(),
            label_pt: state.label_pt().into(),
            free_at: match state {
                RoomState::CleanWaitingInterval { free_at } => Some(free_at.format("%Y-%m-%dT%H:%M").to_string()),
                _ => None,
            },
            cleaning_task_id: task.filter(|t| t.status != "done").map(|t| t.id),
        });
    }
    Ok(out)
}

#[derive(Clone)]
pub struct RoomStatusService {
    ctx: Ctx,
}

impl RoomStatusService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    pub fn list(&self, token: &str) -> Result<Vec<RoomStatus>, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        statuses(&self.ctx, &conn)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::models::block::NewBlockInput;
    use crate::models::maintenance::NewOrderInput;
    use crate::services::test_support::{admin_token, book, checked_out_stay, harness, seed, Harness, Seed};

    // "Hoje" nos testes é 21/10/2026, 09:00 local (fuso -03:00).

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

    fn set_now(w: &World, rfc3339: &str) {
        w.h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    fn state(w: &World) -> (String, Option<String>) {
        let list = w.h.room_status.list(&w.admin).unwrap();
        assert_eq!(list.len(), 1);
        (list[0].state.clone(), list[0].free_at.clone())
    }

    fn task(w: &World) -> i64 {
        w.h.db.lock().unwrap().query_row("SELECT id FROM cleaning_tasks", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn state_matrix_follows_the_whole_cycle() {
        let w = world();
        assert_eq!(state(&w).0, "available");

        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        assert_eq!(state(&w).0, "available", "reserva futura ainda não ocupa o quarto");
        w.h.db.lock().unwrap().execute("UPDATE reservations SET status = 'checked_in' WHERE id = ?1", [reservation]).unwrap();
        assert_eq!(state(&w).0, "occupied");
        w.h.db.lock().unwrap().execute("UPDATE reservations SET status = 'confirmed' WHERE id = ?1", [reservation]).unwrap();
    }

    #[test]
    fn cleaning_cycle_after_checkout() {
        let w = world();
        checked_out_stay(&w.h, &w.admin, &w.seed);
        assert_eq!(state(&w).0, "awaiting_cleaning");
        let list = w.h.room_status.list(&w.admin).unwrap();
        assert_eq!(list[0].label_pt, "Aguardando limpeza");
        assert_eq!(list[0].cleaning_task_id, Some(task(&w)));

        w.h.cleaning.start(&w.admin, task(&w), 1).unwrap();
        assert_eq!(state(&w).0, "cleaning");

        set_now(&w, "2026-10-23T16:00:00Z"); // 13:00 locais
        w.h.cleaning.complete(&w.admin, task(&w), 2).unwrap();
        assert_eq!(state(&w), ("clean_waiting_interval".to_string(), Some("2026-10-23T14:00".to_string())));

        set_now(&w, "2026-10-23T16:59:00Z");
        assert_eq!(state(&w).0, "clean_waiting_interval");
        set_now(&w, "2026-10-23T17:00:00Z"); // 14:00 locais
        assert_eq!(state(&w).0, "available");
    }

    #[test]
    fn occupied_while_in_house() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        set_now(&w, "2026-10-21T17:00:00Z");
        let v = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = crate::models::stay::OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        w.h.stays
            .check_in(
                &w.admin,
                crate::models::stay::CheckInInput { reservation_id: reservation, expected_version: v, occupants: vec![holder], exceed_capacity: false },
            )
            .unwrap();

        assert_eq!(state(&w).0, "occupied");
        assert_eq!(w.h.room_status.list(&w.admin).unwrap()[0].label_pt, "Ocupado");
    }

    #[test]
    fn advancing_the_clock_never_frees_a_dirty_room() {
        let w = world();
        checked_out_stay(&w.h, &w.admin, &w.seed);

        set_now(&w, "2026-11-23T15:00:00Z");

        assert_eq!(state(&w).0, "awaiting_cleaning");
    }

    #[test]
    fn an_effective_block_today_shows_blocked_and_a_future_block_does_not() {
        let w = world();
        let input = |from: &str| NewBlockInput { room_id: w.seed.room_id, start_date: from.into(), end_date: None, reason: "Pintura".into() };

        let future = w.h.blocks.create(&w.admin, input("2026-10-25")).unwrap();
        assert_eq!(state(&w).0, "available");
        w.h.blocks.release(&w.admin, future, "Cancelado".into(), 1).unwrap();

        w.h.blocks.create(&w.admin, input("2026-10-21")).unwrap();
        assert_eq!(state(&w).0, "blocked");
        assert_eq!(w.h.room_status.list(&w.admin).unwrap()[0].label_pt, "Em manutenção ou bloqueado");
    }

    #[test]
    fn completed_maintenance_still_shows_blocked_until_release() {
        let w = world();
        let id = w
            .h
            .maintenance
            .open(
                &w.admin,
                NewOrderInput {
                    room_id: w.seed.room_id,
                    description: "Trocar o chuveiro".into(),
                    priority: "normal".into(),
                    planned_start: "2026-10-21".into(),
                    planned_end: Some("2026-10-23".into()),
                    responsible: String::new(),
                    notes: String::new(),
                },
            )
            .unwrap()
            .id;
        w.h.maintenance.start(&w.admin, id, 1).unwrap();
        w.h.maintenance.request_resolution(&w.admin, id, 2).unwrap();
        w.h.maintenance.complete(&w.admin, id, 3).unwrap();
        assert_eq!(state(&w).0, "blocked");

        set_now(&w, "2026-10-30T12:00:00Z"); // bem depois do fim previsto, sem liberação...
        assert_eq!(state(&w).0, "blocked");
        let (block, version): (i64, i64) =
            w.h.db.lock().unwrap().query_row("SELECT id, version FROM room_blocks", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        w.h.blocks.release(&w.admin, block, "Conferido".into(), version).unwrap();
        assert_eq!(state(&w).0, "available");
    }

    #[test]
    fn inactive_rooms_are_left_out() {
        let w = world();
        w.h.catalog.set_room_active(&w.admin, w.seed.room_id, false).unwrap();

        assert!(w.h.room_status.list(&w.admin).unwrap().is_empty());
    }

    #[test]
    fn room_map_rows_carry_the_state() {
        let w = world();
        checked_out_stay(&w.h, &w.admin, &w.seed);

        let map = w.h.room_map.room_map(&w.admin, "2026-10-23", 3).unwrap();

        assert_eq!(map.rooms[0].state.as_deref(), Some("awaiting_cleaning"));
        assert_eq!(map.rooms[0].state_label.as_deref(), Some("Aguardando limpeza"));
        assert!(map.rooms[0].free_at.is_none());
    }
}
