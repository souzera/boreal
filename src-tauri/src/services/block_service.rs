use serde_json::json;

use super::reservation_service::{conflict_message, parse_date};
use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::error::AppError;
use crate::models::block::{NewBlockInput, RoomBlock};
use crate::repositories::block_repo::{self, NewBlock};
use crate::repositories::{catalog_repo, reservation_repo};

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn clean_text(value: &str, max: usize, what: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(invalid(format!("Informe {what}.")));
    }
    if value.chars().count() > max {
        return Err(invalid(format!("{what}: no máximo {max} caracteres.")));
    }
    Ok(value.to_string())
}

fn block_json(b: &RoomBlock) -> serde_json::Value {
    json!({
        "room": b.room_code, "kind": b.kind, "status": b.status,
        "start_date": b.start_date, "end_date": b.end_date, "reason": b.reason
    })
}

#[derive(Clone)]
pub struct BlockService {
    ctx: Ctx,
}

impl BlockService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn touch(&self, token: &str) -> Result<SessionInfo, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())
    }

    /// Bloqueio manual por período (`fim` exclusivo) ou até liberação manual (sem `fim`).
    /// Recusado se o quarto tem reserva ativa ou hospedagem no período: para isso existe a ordem de manutenção.
    pub fn create(&self, token: &str, input: NewBlockInput) -> Result<i64, AppError> {
        let actor = self.touch(token)?;
        require(&actor, Permission::ReleaseRoom)?;
        let reason = clean_text(&input.reason, 500, "o motivo do bloqueio")?;
        let start = parse_date(&input.start_date, "Data de início")?;
        let end = match input.end_date.as_deref() {
            Some(e) => Some(parse_date(e, "Data de fim")?),
            None => None,
        };
        let today = self.ctx.today();
        if start < today {
            return Err(invalid("O bloqueio não pode começar no passado."));
        }
        if end.is_some_and(|e| e <= start) {
            return Err(invalid("O fim do bloqueio deve ser depois do início."));
        }

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let room = catalog_repo::find_room(&tx, input.room_id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        if !room.active {
            return Err(invalid("O quarto está desativado."));
        }
        let affected = reservation_repo::active_in_period(&tx, room.id, start, end, today)?;
        if !affected.is_empty() {
            return Err(invalid(format!(
                "{} Para indisponibilizar o quarto com reservas, abra uma ordem de manutenção.",
                conflict_message(&affected)
            )));
        }
        let existing = block_repo::overlapping(&tx, room.id, start, end, false)?;
        if !existing.is_empty() {
            return Err(invalid(format!("O quarto {} já tem um bloqueio nesse período.", room.code)));
        }

        let at = self.ctx.utc.now_utc();
        let start_text = input.start_date.trim().to_string();
        let end_text = input.end_date.as_deref().map(|e| e.trim().to_string());
        let id = block_repo::insert(
            &tx,
            &NewBlock {
                room_id: room.id,
                kind: "manual",
                status: "active",
                start_date: &start_text,
                end_date: end_text.as_deref(),
                reason: &reason,
                maintenance_order_id: None,
                created_by: actor.user_id,
            },
            at,
        )?;
        let created = block_repo::find(&tx, id)?.ok_or_else(|| invalid("Bloqueio não encontrado."))?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "blocks",
                operation: "block_created",
                entity: "room_block",
                entity_id: Some(id),
                before: None,
                after: Some(block_json(&created)),
                justification: Some(reason),
            },
        )?;
        tx.commit()?;
        Ok(id)
    }

    /// Encerra o bloqueio (qualquer tipo): o quarto volta a aceitar reservas e hóspedes.
    pub fn release(&self, token: &str, block_id: i64, justification: String, expected_version: i64) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        require(&actor, Permission::ReleaseRoom)?;
        let justification = clean_text(&justification, 500, "a justificativa da liberação")?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let before = block_repo::find(&tx, block_id)?.ok_or_else(|| invalid("Bloqueio não encontrado."))?;
        if before.version != expected_version || before.status == "released" {
            return Err(AppError::Conflict);
        }
        let at = self.ctx.utc.now_utc();
        block_repo::release(&tx, block_id, actor.user_id, &justification, expected_version, at)?;
        let after = block_repo::find(&tx, block_id)?.ok_or_else(|| invalid("Bloqueio não encontrado."))?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "blocks",
                operation: "block_released",
                entity: "room_block",
                entity_id: Some(block_id),
                before: Some(block_json(&before)),
                after: Some(block_json(&after)),
                justification: Some(justification),
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list(&self, token: &str, include_released: bool) -> Result<Vec<RoomBlock>, AppError> {
        self.touch(token)?;
        let conn = self.ctx.conn()?;
        block_repo::list(&conn, include_released)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::auth::permissions::Permission;
    use crate::error::AppError;
    use crate::models::block::NewBlockInput;
    use crate::models::stay::{CheckInInput, OccupantInput};
    use crate::services::test_support::{admin_token, book, harness, recepcao_token, seed, Harness, Seed};

    // "Hoje" nos testes é 21/10/2026, 09:00 local.

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

    fn input(w: &World, from: &str, to: Option<&str>) -> NewBlockInput {
        NewBlockInput { room_id: w.seed.room_id, start_date: from.into(), end_date: to.map(String::from), reason: "Pintura".into() }
    }

    fn blocks(w: &World) -> i64 {
        w.h.db.lock().unwrap().query_row("SELECT count(*) FROM room_blocks", [], |r| r.get(0)).unwrap()
    }

    fn message(result: Result<i64, AppError>) -> String {
        match result {
            Err(AppError::Validation(m)) => m,
            other => panic!("esperava recusa: {other:?}"),
        }
    }

    #[test]
    fn create_needs_release_room_and_valid_input() {
        let w = world();
        let rec = recepcao_token(&w.h, &w.admin);
        assert!(matches!(w.h.blocks.create(&rec, input(&w, "2026-11-01", Some("2026-11-03"))), Err(AppError::Forbidden)));

        let mut blank = input(&w, "2026-11-01", Some("2026-11-03"));
        blank.reason = "   ".into();
        assert!(matches!(w.h.blocks.create(&w.admin, blank), Err(AppError::Validation(_))));
        assert!(matches!(w.h.blocks.create(&w.admin, input(&w, "2026-11-03", Some("2026-11-03"))), Err(AppError::Validation(_))));
        assert!(matches!(w.h.blocks.create(&w.admin, input(&w, "2026-11-04", Some("2026-11-03"))), Err(AppError::Validation(_))));
        assert!(message(w.h.blocks.create(&w.admin, input(&w, "2026-10-20", None))).contains("passado"));
        assert!(matches!(w.h.blocks.create(&w.admin, input(&w, "não é data", None)), Err(AppError::Validation(_))));
        let mut unknown = input(&w, "2026-11-01", None);
        unknown.room_id = 999;
        assert!(matches!(w.h.blocks.create(&w.admin, unknown), Err(AppError::Validation(_))));
        w.h.catalog.set_room_active(&w.admin, w.seed.room_id, false).unwrap();
        assert!(matches!(w.h.blocks.create(&w.admin, input(&w, "2026-11-01", None)), Err(AppError::Validation(_))));

        assert_eq!(blocks(&w), 0);
    }

    #[test]
    fn recepcao_with_release_room_can_create() {
        let w = world();
        recepcao_token(&w.h, &w.admin);
        let rec_id = w.h.auth.login("rec", "Definitiva-456").unwrap().user.id;
        w.h.users.set_extra_permissions(&w.admin, rec_id, &[Permission::ReleaseRoom]).unwrap();
        let rec = w.h.auth.login("rec", "Definitiva-456").unwrap().token;

        assert!(w.h.blocks.create(&rec, input(&w, "2026-11-01", Some("2026-11-03"))).is_ok());
    }

    #[test]
    fn create_over_an_active_reservation_is_refused_listing_it() {
        let w = world();
        book(&w.h, &w.admin, &w.seed, "2026-10-25", "2026-10-27");

        let msg = message(w.h.blocks.create(&w.admin, input(&w, "2026-10-26", Some("2026-10-28"))));

        assert!(msg.contains("RES-000001") && msg.contains("ordem de manutenção"), "{msg}");
        assert_eq!(blocks(&w), 0);
        // O período que começa no dia da saída não toca a reserva.
        assert!(w.h.blocks.create(&w.admin, input(&w, "2026-10-27", Some("2026-10-29"))).is_ok());
    }

    #[test]
    fn create_over_a_stay_in_house_is_refused() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        w.h.utc.set(DateTime::parse_from_rfc3339("2026-10-21T17:00:00Z").unwrap().with_timezone(&Utc));
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        w.h.stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
            .unwrap();

        let msg = message(w.h.blocks.create(&w.admin, input(&w, "2026-10-22", Some("2026-10-24"))));

        assert!(msg.contains("RES-000001"), "{msg}");
        // Hóspede atrasado (passou do 23/10) continua ocupando o quarto: bloqueio a partir de hoje é recusado.
        w.h.utc.set(DateTime::parse_from_rfc3339("2026-10-26T17:00:00Z").unwrap().with_timezone(&Utc));
        assert!(message(w.h.blocks.create(&w.admin, input(&w, "2026-10-26", Some("2026-10-27")))).contains("RES-000001"));
    }

    #[test]
    fn overlapping_blocks_are_refused() {
        let w = world();
        w.h.blocks.create(&w.admin, input(&w, "2026-11-01", Some("2026-11-05"))).unwrap();

        assert!(message(w.h.blocks.create(&w.admin, input(&w, "2026-11-04", None))).contains("já tem um bloqueio"));
        assert!(w.h.blocks.create(&w.admin, input(&w, "2026-11-05", Some("2026-11-06"))).is_ok());
    }

    #[test]
    fn release_needs_permission_and_a_justification_and_double_release_is_conflict() {
        let w = world();
        let id = w.h.blocks.create(&w.admin, input(&w, "2026-11-01", None)).unwrap();
        let rec = recepcao_token(&w.h, &w.admin);

        assert!(matches!(w.h.blocks.release(&rec, id, "ok".into(), 1), Err(AppError::Forbidden)));
        assert!(matches!(w.h.blocks.release(&w.admin, id, "  ".into(), 1), Err(AppError::Validation(_))));
        assert!(matches!(w.h.blocks.release(&w.admin, id, "ok".into(), 9), Err(AppError::Conflict)));

        w.h.blocks.release(&w.admin, id, "Obra terminou".into(), 1).unwrap();

        assert!(matches!(w.h.blocks.release(&w.admin, id, "de novo".into(), 2), Err(AppError::Conflict)));
        assert!(w.h.blocks.list(&w.admin, false).unwrap().is_empty());
        assert_eq!(w.h.blocks.list(&w.admin, true).unwrap().len(), 1);
    }

    #[test]
    fn block_events_are_audited() {
        let w = world();
        let id = w.h.blocks.create(&w.admin, input(&w, "2026-11-01", None)).unwrap();
        w.h.blocks.release(&w.admin, id, "Obra terminou".into(), 1).unwrap();

        let ops: Vec<String> = {
            let conn = w.h.db.lock().unwrap();
            let mut stmt = conn.prepare("SELECT operation FROM audit_log WHERE module = 'blocks' ORDER BY id").unwrap();
            stmt.query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect()
        };
        assert_eq!(ops, vec!["block_created", "block_released"]);
    }
}
