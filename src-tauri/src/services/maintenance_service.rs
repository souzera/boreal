use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Transaction;
use serde_json::json;

use super::reservation_service::parse_date;
use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::domain::cleaning::Priority;
use crate::domain::maintenance::{transition, OrderStatus};
use crate::error::AppError;
use crate::models::maintenance::{ApplyResult, MaintenanceOrder, NewOrderInput, OpenedOrder, OrderDetail};
use crate::repositories::block_repo::{self, NewBlock};
use crate::repositories::maintenance_repo::{self, NewOrder};
use crate::repositories::{catalog_repo, reservation_repo, sequence_repo};

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn clean_text(value: &str, max: usize, what: &str, required: bool) -> Result<String, AppError> {
    let value = value.trim();
    if required && value.is_empty() {
        return Err(invalid(format!("Informe {what}.")));
    }
    if value.chars().count() > max {
        return Err(invalid(format!("{what}: no máximo {max} caracteres.")));
    }
    Ok(value.to_string())
}

fn order_json(o: &MaintenanceOrder) -> serde_json::Value {
    json!({
        "code": o.code, "room": o.room_code, "status": o.status, "priority": o.priority,
        "planned_start": o.planned_start, "planned_end": o.planned_end
    })
}

#[derive(Clone)]
pub struct MaintenanceService {
    ctx: Ctx,
}

impl MaintenanceService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn touch(&self, token: &str) -> Result<SessionInfo, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())
    }

    #[allow(clippy::too_many_arguments)] // espelha os campos da auditoria
    fn audit(
        &self,
        tx: &Transaction,
        actor: &SessionInfo,
        operation: &'static str,
        entity: &'static str,
        entity_id: i64,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
        justification: Option<String>,
    ) -> Result<(), AppError> {
        audit::record(
            tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "maintenance",
                operation,
                entity,
                entity_id: Some(entity_id),
                before,
                after,
                justification,
            },
        )
    }

    fn load_order(tx: &rusqlite::Connection, id: i64, expected_version: i64) -> Result<MaintenanceOrder, AppError> {
        let order = maintenance_repo::find_order(tx, id)?.ok_or_else(|| invalid("Ordem de manutenção não encontrada."))?;
        if order.version != expected_version {
            return Err(AppError::Conflict);
        }
        Ok(order)
    }

    /// Abre a ordem. Sem reservas afetadas o quarto já fica indisponível (bloqueio efetivo); com reservas,
    /// um bloqueio preventivo impede novas reservas enquanto o conflito é resolvido (spec §8).
    pub fn open(&self, token: &str, input: NewOrderInput) -> Result<OpenedOrder, AppError> {
        let actor = self.touch(token)?;
        let description = clean_text(&input.description, 500, "a descrição da manutenção", true)?;
        let responsible = clean_text(&input.responsible, 120, "O responsável", false)?;
        let notes = clean_text(&input.notes, 1000, "As observações", false)?;
        let priority = Priority::parse(&input.priority).ok_or_else(|| invalid("Prioridade desconhecida."))?;
        let start = parse_date(&input.planned_start, "Início previsto")?;
        let end: Option<NaiveDate> = match input.planned_end.as_deref() {
            Some(e) => Some(parse_date(e, "Fim previsto")?),
            None => None,
        };
        let today = self.ctx.today();
        if start < today {
            return Err(invalid("O início previsto não pode ser no passado."));
        }
        if end.is_some_and(|e| e <= start) {
            return Err(invalid("O fim previsto deve ser depois do início."));
        }

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let room = catalog_repo::find_room(&tx, input.room_id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        if !room.active {
            return Err(invalid("O quarto está desativado."));
        }
        if !block_repo::overlapping(&tx, room.id, start, end, false)?.is_empty() {
            return Err(invalid(format!("O quarto {} já tem um bloqueio nesse período.", room.code)));
        }
        let affected = reservation_repo::active_in_period(&tx, room.id, start, end, today)?;

        let at = self.ctx.utc.now_utc();
        let code = sequence_repo::format_code("MAN", sequence_repo::next(&tx, "maintenance")?);
        let start_text = input.planned_start.trim().to_string();
        let end_text = input.planned_end.as_deref().map(|e| e.trim().to_string());
        let order_id = maintenance_repo::insert_order(
            &tx,
            &NewOrder {
                code: &code,
                room_id: room.id,
                description: &description,
                priority: priority.as_str(),
                planned_start: &start_text,
                planned_end: end_text.as_deref(),
                responsible: &responsible,
                notes: &notes,
                created_by: actor.user_id,
            },
            at,
        )?;
        for c in &affected {
            maintenance_repo::insert_conflict(&tx, order_id, c.reservation_id)?;
        }
        let block_status = if affected.is_empty() { "active" } else { "preventive" };
        block_repo::insert(
            &tx,
            &NewBlock {
                room_id: room.id,
                kind: "maintenance",
                status: block_status,
                start_date: &start_text,
                end_date: end_text.as_deref(),
                reason: &format!("Manutenção {code}: {description}"),
                maintenance_order_id: Some(order_id),
                created_by: actor.user_id,
            },
            at,
        )?;
        let created = maintenance_repo::find_order(&tx, order_id)?.ok_or_else(|| invalid("Ordem de manutenção não encontrada."))?;
        self.audit(
            &tx,
            &actor,
            "maintenance_opened",
            "maintenance_order",
            order_id,
            None,
            Some(json!({ "order": order_json(&created), "block": block_status, "affected_reservations": affected.len() })),
            None,
        )?;
        tx.commit()?;
        Ok(OpenedOrder { id: order_id, code, conflicts: affected })
    }

    /// Promove o bloqueio preventivo a efetivo se não restar conflito aberto.
    fn promote_if_clear(&self, tx: &Transaction, order: &MaintenanceOrder, at: DateTime<Utc>) -> Result<bool, AppError> {
        let block = block_repo::for_order(tx, order.id)?;
        let Some(block) = block else {
            return Ok(false);
        };
        if !maintenance_repo::open_conflicts(tx, order.id)?.is_empty() {
            return Ok(block.status != "preventive");
        }
        if block.status == "preventive" {
            block_repo::set_status(tx, block.id, "active", block.version, at)?;
        }
        Ok(true)
    }

    /// Reavalia os conflitos (reserva cancelada ou movida deixa de conflitar) e, com todos resolvidos,
    /// torna a indisponibilidade efetiva. Nada é cancelado nem alterado automaticamente.
    pub fn apply_unavailability(&self, token: &str, order_id: i64, expected_version: i64) -> Result<ApplyResult, AppError> {
        let actor = self.touch(token)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let order = Self::load_order(&tx, order_id, expected_version)?;
        if order.status == "completed" {
            return Err(invalid("A ordem de manutenção já foi concluída."));
        }
        let at = self.ctx.utc.now_utc();
        let (start, end) = (
            parse_date(&order.planned_start, "Início previsto")?,
            order.planned_end.as_deref().map(|e| parse_date(e, "Fim previsto")).transpose()?,
        );
        let still_touching: Vec<i64> =
            reservation_repo::active_in_period(&tx, order.room_id, start, end, self.ctx.today())?.iter().map(|c| c.reservation_id).collect();
        let mut resolved = 0;
        for conflict in maintenance_repo::open_conflicts(&tx, order.id)? {
            if !still_touching.contains(&conflict.reservation_id) {
                maintenance_repo::resolve_conflict(&tx, conflict.id, actor.user_id, "Resolvido: a reserva não ocupa mais o período.", at)?;
                resolved += 1;
            }
        }
        let effective = self.promote_if_clear(&tx, &order, at)?;
        let still_open = maintenance_repo::open_conflicts(&tx, order.id)?;
        self.audit(
            &tx,
            &actor,
            "maintenance_unavailability_applied",
            "maintenance_order",
            order.id,
            None,
            Some(json!({ "resolved_now": resolved, "still_open": still_open.len(), "effective": effective })),
            None,
        )?;
        tx.commit()?;
        Ok(ApplyResult { still_open, effective })
    }

    /// Resolução manual de um conflito (por exemplo, a reserva foi remanejada fora do sistema). Nota obrigatória.
    pub fn resolve_conflict_manually(&self, token: &str, conflict_id: i64, note: String) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        // Marcar uma reserva ativa como resolvida torna o bloqueio efetivo: é uma exceção autorizada.
        require(&actor, Permission::ReleaseRoom)?;
        let note = clean_text(&note, 500, "a nota da resolução", true)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let conflict = maintenance_repo::find_conflict(&tx, conflict_id)?.ok_or_else(|| invalid("Conflito não encontrado."))?;
        if conflict.status != "open" {
            return Err(AppError::Conflict);
        }
        maintenance_repo::resolve_conflict(&tx, conflict_id, actor.user_id, &note, self.ctx.utc.now_utc())?;
        self.audit(
            &tx,
            &actor,
            "maintenance_conflict_resolved",
            "maintenance_conflict",
            conflict_id,
            Some(json!({ "status": "open", "reservation": conflict.reservation_code })),
            Some(json!({ "status": "resolved" })),
            Some(note),
        )?;
        tx.commit()?;
        Ok(())
    }

    fn move_to(&self, token: &str, order_id: i64, expected_version: i64, to: OrderStatus) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let order = Self::load_order(&tx, order_id, expected_version)?;
        let from = OrderStatus::parse(&order.status).ok_or_else(|| AppError::Database("situação de manutenção desconhecida".into()))?;
        transition(from, to)?;
        let at = self.ctx.utc.now_utc();

        if to == OrderStatus::InProgress {
            let open = maintenance_repo::open_conflicts(&tx, order.id)?;
            if !open.is_empty() {
                let list = open.iter().map(|c| c.reservation_code.clone()).collect::<Vec<_>>().join(", ");
                return Err(invalid(format!(
                    "Resolva as reservas afetadas antes de iniciar a manutenção: {list}."
                )));
            }
            self.promote_if_clear(&tx, &order, at)?;
        }
        if to == OrderStatus::Completed {
            // A conclusão não libera o quarto: o bloqueio passa a aguardar a liberação autorizada (spec §8).
            if let Some(block) = block_repo::for_order(&tx, order.id)? {
                block_repo::set_status(&tx, block.id, "awaiting_release", block.version, at)?;
            }
        }
        let (actual_start, actual_end) = match to {
            OrderStatus::InProgress => (Some(at), None),
            OrderStatus::Completed => (None, Some(at)),
            _ => (None, None),
        };
        maintenance_repo::set_status(&tx, order.id, to.as_str(), actual_start, actual_end, expected_version, at)?;
        let operation = match to {
            OrderStatus::InProgress => "maintenance_started",
            OrderStatus::AwaitingResolution => "maintenance_resolution_requested",
            _ => "maintenance_completed",
        };
        self.audit(
            &tx,
            &actor,
            operation,
            "maintenance_order",
            order.id,
            Some(json!({ "status": order.status })),
            Some(json!({ "status": to.as_str(), "code": order.code })),
            None,
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn start(&self, token: &str, order_id: i64, expected_version: i64) -> Result<(), AppError> {
        self.move_to(token, order_id, expected_version, OrderStatus::InProgress)
    }

    pub fn request_resolution(&self, token: &str, order_id: i64, expected_version: i64) -> Result<(), AppError> {
        self.move_to(token, order_id, expected_version, OrderStatus::AwaitingResolution)
    }

    pub fn complete(&self, token: &str, order_id: i64, expected_version: i64) -> Result<(), AppError> {
        self.move_to(token, order_id, expected_version, OrderStatus::Completed)
    }

    pub fn list(&self, token: &str, only_open: bool) -> Result<Vec<MaintenanceOrder>, AppError> {
        self.touch(token)?;
        let conn = self.ctx.conn()?;
        maintenance_repo::list(&conn, only_open)
    }

    pub fn get(&self, token: &str, order_id: i64) -> Result<OrderDetail, AppError> {
        self.touch(token)?;
        let conn = self.ctx.conn()?;
        let order = maintenance_repo::find_order(&conn, order_id)?.ok_or_else(|| invalid("Ordem de manutenção não encontrada."))?;
        Ok(OrderDetail {
            conflicts: maintenance_repo::conflicts(&conn, order_id)?,
            block: block_repo::for_order(&conn, order_id)?,
            order,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::error::AppError;
    use crate::models::maintenance::NewOrderInput;
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

    fn order(w: &World, from: &str, to: Option<&str>) -> NewOrderInput {
        NewOrderInput {
            room_id: w.seed.room_id,
            description: "Trocar o chuveiro".into(),
            priority: "high".into(),
            planned_start: from.into(),
            planned_end: to.map(String::from),
            responsible: "Seu João".into(),
            notes: String::new(),
        }
    }

    fn count(w: &World, table: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    fn block_status(w: &World) -> String {
        w.h.db.lock().unwrap().query_row("SELECT status FROM room_blocks ORDER BY id DESC LIMIT 1", [], |r| r.get(0)).unwrap()
    }

    fn version(w: &World, id: i64) -> i64 {
        w.h.maintenance.get(&w.admin, id).unwrap().order.version
    }

    fn set_reservation_status(w: &World, id: i64, status: &str) {
        w.h.db.lock().unwrap().execute("UPDATE reservations SET status = ?1 WHERE id = ?2", rusqlite::params![status, id]).unwrap();
    }

    fn audits(w: &World, operation: &str) -> i64 {
        w.h.db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_log WHERE module = 'maintenance' AND operation = ?1", [operation], |r| r.get(0))
            .unwrap()
    }

    /// Ordem 10→12/11 com a reserva 09→11/11 afetada; devolve (ordem, reserva).
    fn with_conflict(w: &World) -> (i64, i64) {
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-11-09", "2026-11-11");
        let opened = w.h.maintenance.open(&w.admin, order(w, "2026-11-10", Some("2026-11-12"))).unwrap();
        (opened.id, reservation)
    }

    #[test]
    fn open_without_conflict_blocks_effectively_and_returns_a_code() {
        let w = world();

        let opened = w.h.maintenance.open(&w.admin, order(&w, "2026-11-10", Some("2026-11-12"))).unwrap();

        assert_eq!(opened.code, "MAN-000001");
        assert!(opened.conflicts.is_empty());
        assert_eq!(block_status(&w), "active");
        let detail = w.h.maintenance.get(&w.admin, opened.id).unwrap();
        assert_eq!(detail.order.status, "open");
        assert_eq!(detail.block.unwrap().kind, "maintenance");
        assert_eq!(audits(&w, "maintenance_opened"), 1);
    }

    #[test]
    fn open_with_conflicts_creates_a_preventive_block_and_lists_the_reservations() {
        let w = world();

        let (id, _) = with_conflict(&w);

        assert_eq!(block_status(&w), "preventive");
        let detail = w.h.maintenance.get(&w.admin, id).unwrap();
        assert_eq!(detail.conflicts.len(), 1);
        assert_eq!(detail.conflicts[0].reservation_code, "RES-000001");
        assert_eq!(detail.conflicts[0].status, "open");
    }

    #[test]
    fn preventive_block_refuses_new_reservations_in_the_period() {
        let w = world();
        with_conflict(&w);

        let result = w.h.reservations.create(
            &w.admin,
            crate::models::reservation::NewReservation {
                room_id: w.seed.room_id,
                guest_id: w.seed.guest_id,
                check_in_date: "2026-11-11".into(),
                check_out_date: "2026-11-12".into(),
                expected_check_in_time: None,
                expected_check_out_time: None,
                occupants: 1,
                notes: String::new(),
                exceed_capacity: false,
            },
        );

        match result {
            Err(AppError::Validation(m)) => assert!(m.contains("bloqueio preventivo"), "{m}"),
            other => panic!("esperava recusa: {other:?}"),
        }
    }

    #[test]
    fn applying_with_open_conflicts_keeps_the_preventive_block() {
        let w = world();
        let (id, _) = with_conflict(&w);

        let result = w.h.maintenance.apply_unavailability(&w.admin, id, 1).unwrap();

        assert!(!result.effective);
        assert_eq!(result.still_open.len(), 1);
        assert_eq!(block_status(&w), "preventive");
    }

    #[test]
    fn cancelling_the_affected_reservation_then_applying_makes_it_effective() {
        let w = world();
        let (id, reservation) = with_conflict(&w);
        set_reservation_status(&w, reservation, "cancelled");

        let result = w.h.maintenance.apply_unavailability(&w.admin, id, 1).unwrap();

        assert!(result.effective && result.still_open.is_empty());
        assert_eq!(block_status(&w), "active");
        assert_eq!(w.h.maintenance.get(&w.admin, id).unwrap().conflicts[0].status, "resolved");
        // Idempotente: aplicar de novo não muda nada nem falha.
        assert!(w.h.maintenance.apply_unavailability(&w.admin, id, 1).unwrap().effective);
    }

    #[test]
    fn manual_resolution_needs_a_note_and_is_audited() {
        let w = world();
        let (id, _) = with_conflict(&w);
        let conflict = w.h.maintenance.get(&w.admin, id).unwrap().conflicts[0].id;
        let rec = recepcao_token(&w.h, &w.admin);
        assert!(matches!(w.h.maintenance.resolve_conflict_manually(&rec, conflict, "Remanejado".into()), Err(AppError::Forbidden)));

        assert!(matches!(w.h.maintenance.resolve_conflict_manually(&w.admin, conflict, "  ".into()), Err(AppError::Validation(_))));
        w.h.maintenance.resolve_conflict_manually(&w.admin, conflict, "Hóspede remanejado para o quarto 102".into()).unwrap();

        assert!(matches!(w.h.maintenance.resolve_conflict_manually(&w.admin, conflict, "de novo".into()), Err(AppError::Conflict)));
        assert_eq!(audits(&w, "maintenance_conflict_resolved"), 1);
        // A reserva continua ativa: aplicar ainda promove, pois o conflito foi resolvido manualmente.
        assert!(w.h.maintenance.apply_unavailability(&w.admin, id, 1).unwrap().effective);
    }

    #[test]
    fn start_is_refused_with_open_conflicts() {
        let w = world();
        let (id, _) = with_conflict(&w);

        match w.h.maintenance.start(&w.admin, id, 1) {
            Err(AppError::Validation(m)) => assert!(m.contains("RES-000001"), "{m}"),
            other => panic!("esperava recusa: {other:?}"),
        }
        assert_eq!(w.h.maintenance.get(&w.admin, id).unwrap().order.status, "open");
    }

    #[test]
    fn state_machine_rejects_skipped_steps() {
        let w = world();
        let id = w.h.maintenance.open(&w.admin, order(&w, "2026-11-10", None)).unwrap().id;

        assert!(matches!(w.h.maintenance.complete(&w.admin, id, 1), Err(AppError::Validation(_))));
        assert!(matches!(w.h.maintenance.request_resolution(&w.admin, id, 1), Err(AppError::Validation(_))));
        w.h.maintenance.start(&w.admin, id, 1).unwrap();
        assert!(matches!(w.h.maintenance.start(&w.admin, id, 2), Err(AppError::Validation(_))));
        assert!(matches!(w.h.maintenance.complete(&w.admin, id, 2), Err(AppError::Validation(_))));
        w.h.maintenance.request_resolution(&w.admin, id, 2).unwrap();
        w.h.maintenance.complete(&w.admin, id, 3).unwrap();
        assert!(matches!(w.h.maintenance.start(&w.admin, id, 4), Err(AppError::Validation(_))));
    }

    #[test]
    fn complete_does_not_release_the_room() {
        let w = world();
        let id = w.h.maintenance.open(&w.admin, order(&w, "2026-10-21", Some("2026-10-25"))).unwrap().id;
        w.h.maintenance.start(&w.admin, id, 1).unwrap();
        w.h.maintenance.request_resolution(&w.admin, id, 2).unwrap();
        w.h.maintenance.complete(&w.admin, id, 3).unwrap();

        assert_eq!(block_status(&w), "awaiting_release");
        let detail = w.h.maintenance.get(&w.admin, id).unwrap();
        assert!(detail.order.actual_start.is_some() && detail.order.actual_end.is_some());
        let blocked = w.h.reservations.check_availability(&w.admin, w.seed.room_id, "2026-10-22", "2026-10-23", None).unwrap();
        assert!(!blocked.available);
    }

    #[test]
    fn release_after_completion_needs_release_room_and_a_completed_order() {
        let w = world();
        let id = w.h.maintenance.open(&w.admin, order(&w, "2026-10-21", Some("2026-10-25"))).unwrap().id;
        let block: (i64, i64) = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT id, version FROM room_blocks", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        // Antes de concluir a ordem, o bloqueio não pode ser liberado por fora.
        assert!(matches!(w.h.blocks.release(&w.admin, block.0, "Cansei".into(), block.1), Err(AppError::Validation(_))));

        w.h.maintenance.start(&w.admin, id, 1).unwrap();
        w.h.maintenance.request_resolution(&w.admin, id, 2).unwrap();
        w.h.maintenance.complete(&w.admin, id, 3).unwrap();
        let version: i64 = w.h.db.lock().unwrap().query_row("SELECT version FROM room_blocks", [], |r| r.get(0)).unwrap();

        let rec = recepcao_token(&w.h, &w.admin);
        assert!(matches!(w.h.blocks.release(&rec, block.0, "ok".into(), version), Err(AppError::Forbidden)));
        w.h.blocks.release(&w.admin, block.0, "Conferido pela administração".into(), version).unwrap();

        let free = w.h.reservations.check_availability(&w.admin, w.seed.room_id, "2026-10-22", "2026-10-23", None).unwrap();
        assert!(free.available);
    }

    #[test]
    fn double_complete_is_conflict() {
        let w = world();
        let id = w.h.maintenance.open(&w.admin, order(&w, "2026-11-10", None)).unwrap().id;
        w.h.maintenance.start(&w.admin, id, 1).unwrap();
        w.h.maintenance.request_resolution(&w.admin, id, 2).unwrap();
        w.h.maintenance.complete(&w.admin, id, 3).unwrap();

        assert!(matches!(w.h.maintenance.complete(&w.admin, id, 3), Err(AppError::Conflict)));
        assert_eq!(audits(&w, "maintenance_completed"), 1);
    }

    #[test]
    fn invalid_input_writes_nothing() {
        let w = world();
        let mut blank = order(&w, "2026-11-10", None);
        blank.description = "   ".into();
        let mut bad_priority = order(&w, "2026-11-10", None);
        bad_priority.priority = "x".into();
        let mut unknown = order(&w, "2026-11-10", None);
        unknown.room_id = 999;

        for input in [blank, bad_priority, unknown, order(&w, "2026-10-20", None), order(&w, "2026-11-12", Some("2026-11-12")), order(&w, "amanhã", None)] {
            assert!(matches!(w.h.maintenance.open(&w.admin, input), Err(AppError::Validation(_))));
        }
        w.h.catalog.set_room_active(&w.admin, w.seed.room_id, false).unwrap();
        assert!(matches!(w.h.maintenance.open(&w.admin, order(&w, "2026-11-10", None)), Err(AppError::Validation(_))));

        assert_eq!(count(&w, "maintenance_orders") + count(&w, "room_blocks") + count(&w, "maintenance_conflicts"), 0);
        // Recusas não consomem o código da sequência.
        w.h.catalog.set_room_active(&w.admin, w.seed.room_id, true).unwrap();
        assert_eq!(w.h.maintenance.open(&w.admin, order(&w, "2026-11-10", None)).unwrap().code, "MAN-000001");
    }

    #[test]
    fn order_overlapping_an_existing_block_is_refused() {
        let w = world();
        w.h.maintenance.open(&w.admin, order(&w, "2026-11-10", Some("2026-11-12"))).unwrap();

        assert!(matches!(w.h.maintenance.open(&w.admin, order(&w, "2026-11-11", None)), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "maintenance_orders"), 1);
    }

    #[test]
    fn order_with_a_stay_in_house_in_the_period_lists_it_as_a_conflict() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        w.h.utc.set(DateTime::parse_from_rfc3339("2026-10-21T17:00:00Z").unwrap().with_timezone(&Utc));
        let v = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        w.h.stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: v, occupants: vec![holder], exceed_capacity: false })
            .unwrap();

        let opened = w.h.maintenance.open(&w.admin, order(&w, "2026-10-22", Some("2026-10-24"))).unwrap();

        assert_eq!(opened.conflicts.len(), 1);
        assert_eq!(opened.conflicts[0].status, "checked_in");
        assert_eq!(block_status(&w), "preventive");
    }

    #[test]
    fn all_transitions_are_audited_and_version_checked() {
        let w = world();
        let id = w.h.maintenance.open(&w.admin, order(&w, "2026-11-10", None)).unwrap().id;
        assert!(matches!(w.h.maintenance.start(&w.admin, id, 9), Err(AppError::Conflict)));

        w.h.maintenance.start(&w.admin, id, version(&w, id)).unwrap();
        w.h.maintenance.request_resolution(&w.admin, id, version(&w, id)).unwrap();
        w.h.maintenance.complete(&w.admin, id, version(&w, id)).unwrap();

        for op in ["maintenance_started", "maintenance_resolution_requested", "maintenance_completed"] {
            assert_eq!(audits(&w, op), 1, "{op}");
        }
        assert_eq!(w.h.maintenance.list(&w.admin, true).unwrap().len(), 0);
        assert_eq!(w.h.maintenance.list(&w.admin, false).unwrap().len(), 1);
    }
}
