use chrono::NaiveDateTime;
use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::session::SessionInfo;
use crate::domain::cleaning::{suggest_priority, Priority};
use crate::error::AppError;
use crate::models::cleaning::{CleaningItem, CleaningTask};
use crate::repositories::{cleaning_repo, reservation_repo};

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn clean_justification(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(invalid("Informe a justificativa do ajuste de prioridade."));
    }
    if value.chars().count() > 500 {
        return Err(invalid("A justificativa pode ter no máximo 500 caracteres."));
    }
    Ok(value.to_string())
}

#[derive(Clone)]
pub struct CleaningService {
    ctx: Ctx,
}

impl CleaningService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn touch(&self, token: &str) -> Result<SessionInfo, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())
    }

    fn load(tx: &rusqlite::Connection, id: i64, expected_version: i64) -> Result<CleaningTask, AppError> {
        let task = cleaning_repo::find(tx, id)?.ok_or_else(|| invalid("Tarefa de limpeza não encontrada."))?;
        if task.version != expected_version {
            return Err(AppError::Conflict);
        }
        Ok(task)
    }

    #[allow(clippy::too_many_arguments)] // espelha os campos da auditoria
    fn audit(
        &self,
        tx: &rusqlite::Transaction,
        actor: &SessionInfo,
        operation: &'static str,
        task: &CleaningTask,
        before: serde_json::Value,
        after: serde_json::Value,
        justification: Option<String>,
    ) -> Result<(), AppError> {
        audit::record(
            tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "cleaning",
                operation,
                entity: "cleaning_task",
                entity_id: Some(task.id),
                before: Some(before),
                after: Some(after),
                justification,
            },
        )
    }

    /// Fila de limpeza: prioridade resolvida (ajuste manual ou sugerida), mais urgente primeiro,
    /// depois a saída mais antiga. A prioridade é recalculada a cada leitura.
    pub fn queue(&self, token: &str) -> Result<Vec<CleaningItem>, AppError> {
        self.touch(token)?;
        let conn = self.ctx.conn()?;
        let now = self.ctx.local_now();
        let mut items = Vec::new();
        for task in cleaning_repo::queue(&conn)? {
            let arrival: Option<NaiveDateTime> = reservation_repo::next_arrival(&conn, task.room_id)?;
            let suggested = suggest_priority(arrival, now);
            let priority = task.priority_override.as_deref().and_then(Priority::parse).unwrap_or(suggested);
            items.push(CleaningItem {
                task,
                priority: priority.as_str().into(),
                suggested_priority: suggested.as_str().into(),
                next_arrival: arrival.map(|a| a.format("%Y-%m-%dT%H:%M").to_string()),
            });
        }
        // `sort_by_key` é estável: a ordem de saída (mais antiga primeiro) se mantém dentro da prioridade.
        items.sort_by_key(|i| Priority::parse(&i.priority).map_or(u8::MAX, Priority::rank));
        Ok(items)
    }

    pub fn start(&self, token: &str, task_id: i64, expected_version: i64) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let task = Self::load(&tx, task_id, expected_version)?;
        if task.status != "awaiting" {
            return Err(AppError::Conflict);
        }
        let at = self.ctx.utc.now_utc();
        cleaning_repo::start(&tx, task_id, actor.user_id, expected_version, at)?;
        self.audit(
            &tx,
            &actor,
            "cleaning_started",
            &task,
            json!({ "status": "awaiting" }),
            json!({ "status": "in_progress", "room": task.room_code }),
            None,
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn complete(&self, token: &str, task_id: i64, expected_version: i64) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let task = Self::load(&tx, task_id, expected_version)?;
        match task.status.as_str() {
            "awaiting" => return Err(invalid("Inicie a limpeza antes de concluir.")),
            "in_progress" => {}
            _ => return Err(AppError::Conflict),
        }
        let at = self.ctx.utc.now_utc();
        cleaning_repo::complete(&tx, task_id, actor.user_id, expected_version, at)?;
        self.audit(
            &tx,
            &actor,
            "cleaning_completed",
            &task,
            json!({ "status": "in_progress" }),
            json!({ "status": "done", "room": task.room_code }),
            None,
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Ajusta a prioridade (`Some`) ou volta à sugerida (`None`). A justificativa é sempre obrigatória.
    pub fn set_priority(
        &self,
        token: &str,
        task_id: i64,
        priority: Option<String>,
        justification: String,
        expected_version: i64,
    ) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        let justification = clean_justification(&justification)?;
        let parsed = match priority.as_deref() {
            None => None,
            Some(p) => Some(Priority::parse(p).ok_or_else(|| invalid("Prioridade desconhecida."))?),
        };
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let task = Self::load(&tx, task_id, expected_version)?;
        if task.status == "done" {
            return Err(AppError::Conflict);
        }
        let at = self.ctx.utc.now_utc();
        cleaning_repo::set_priority(&tx, task_id, parsed, &justification, expected_version, at)?;
        self.audit(
            &tx,
            &actor,
            "cleaning_priority_changed",
            &task,
            json!({ "priority_override": task.priority_override }),
            json!({ "priority_override": parsed.map(Priority::as_str), "room": task.room_code }),
            Some(justification),
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::auth::permissions::Permission;
    use crate::error::AppError;
    use crate::models::stay::{CheckInInput, CheckoutInput, LateFeeChoice, OccupantInput, PaymentInput};
    use crate::services::test_support::{admin_token, book, harness, recepcao_token, seed, Harness, Seed};

    // "Hoje" nos testes é 21/10/2026, 09:00 local (12:00Z; fuso -03:00).

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

    /// Reserva 21→23/10, check-in 14:00 locais, conta quitada. Retorna (reserva, hospedagem).
    fn paid_stay(w: &World) -> (i64, i64) {
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        set_now(w, "2026-10-21T17:00:00Z");
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        let stay = w
            .h
            .stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
            .unwrap();
        let balance = w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents;
        w.h.stays
            .record_payment(&w.admin, stay, PaymentInput { amount_cents: balance, method: "cash".into(), note: String::new() })
            .unwrap();
        (reservation, stay)
    }

    /// Check-out no horário previsto (23/10 12:00 local = 15:00Z). Retorna o id da tarefa de limpeza.
    fn checked_out(w: &World) -> (i64, i64) {
        let (_, stay) = paid_stay(w);
        set_now(w, "2026-10-23T15:00:00Z");
        let version = w.h.stays.get(&w.admin, stay).unwrap().stay.version;
        w.h.stays
            .checkout(&w.admin, CheckoutInput { stay_id: stay, expected_version: version, late_fee: LateFeeChoice::NotApplicable, balance_exception: None })
            .unwrap();
        (stay, only_task(w))
    }

    fn only_task(w: &World) -> i64 {
        w.h.db.lock().unwrap().query_row("SELECT id FROM cleaning_tasks", [], |r| r.get(0)).unwrap()
    }

    fn task_version(w: &World, id: i64) -> i64 {
        w.h.db.lock().unwrap().query_row("SELECT version FROM cleaning_tasks WHERE id = ?1", [id], |r| r.get(0)).unwrap()
    }

    fn count(w: &World, table: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    fn column(w: &World, id: i64, column: &str) -> Option<String> {
        w.h.db
            .lock()
            .unwrap()
            .query_row(&format!("SELECT CAST({column} AS TEXT) FROM cleaning_tasks WHERE id = ?1"), [id], |r| r.get(0))
            .unwrap()
    }

    fn audits(w: &World, operation: &str) -> i64 {
        w.h.db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_log WHERE module = 'cleaning' AND operation = ?1", [operation], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn checkout_creates_one_awaiting_task_with_the_effective_time() {
        let w = world();
        let (stay, task) = checked_out(&w);

        assert_eq!(count(&w, "cleaning_tasks"), 1);
        assert_eq!(column(&w, task, "status").as_deref(), Some("awaiting"));
        assert_eq!(column(&w, task, "ready_since").as_deref(), Some("2026-10-23T15:00:00Z"));
        assert_eq!(column(&w, task, "stay_id"), Some(stay.to_string()));
        assert_eq!(column(&w, task, "room_id"), Some(w.seed.room_id.to_string()));
    }

    #[test]
    fn checkout_failure_leaves_no_task() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        set_now(&w, "2026-10-21T17:00:00Z");
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        let stay = w
            .h
            .stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
            .unwrap();
        set_now(&w, "2026-10-23T15:00:00Z");
        let v = w.h.stays.get(&w.admin, stay).unwrap().stay.version;

        // Saldo em aberto e sem exceção: recusado.
        let result = w
            .h
            .stays
            .checkout(&w.admin, CheckoutInput { stay_id: stay, expected_version: v, late_fee: LateFeeChoice::NotApplicable, balance_exception: None });

        assert!(result.is_err());
        assert_eq!(count(&w, "cleaning_tasks"), 0);
    }

    #[test]
    fn double_checkout_does_not_duplicate_task() {
        let w = world();
        let (stay, _) = checked_out(&w);

        let again = w
            .h
            .stays
            .checkout(&w.admin, CheckoutInput { stay_id: stay, expected_version: 1, late_fee: LateFeeChoice::NotApplicable, balance_exception: None });

        assert!(again.is_err());
        assert_eq!(count(&w, "cleaning_tasks"), 1);
    }

    #[test]
    fn start_then_complete_records_user_and_time() {
        let w = world();
        let (_, task) = checked_out(&w);

        set_now(&w, "2026-10-23T16:00:00Z");
        w.h.cleaning.start(&w.admin, task, task_version(&w, task)).unwrap();
        set_now(&w, "2026-10-23T16:40:00Z");
        w.h.cleaning.complete(&w.admin, task, task_version(&w, task)).unwrap();

        assert_eq!(column(&w, task, "status").as_deref(), Some("done"));
        assert_eq!(column(&w, task, "started_at").as_deref(), Some("2026-10-23T16:00:00Z"));
        assert_eq!(column(&w, task, "completed_at").as_deref(), Some("2026-10-23T16:40:00Z"));
        assert!(column(&w, task, "completed_by").is_some());
        assert_eq!(audits(&w, "cleaning_started"), 1);
        assert_eq!(audits(&w, "cleaning_completed"), 1);
    }

    #[test]
    fn complete_without_start_is_refused() {
        let w = world();
        let (_, task) = checked_out(&w);

        let result = w.h.cleaning.complete(&w.admin, task, task_version(&w, task));

        match result {
            Err(AppError::Validation(m)) => assert!(m.contains("Inicie a limpeza")),
            other => panic!("esperava Validation, veio {other:?}"),
        }
        assert_eq!(column(&w, task, "status").as_deref(), Some("awaiting"));
    }

    #[test]
    fn double_complete_is_conflict_and_keeps_first_user() {
        let w = world();
        let (_, task) = checked_out(&w);
        w.h.cleaning.start(&w.admin, task, 1).unwrap();
        w.h.cleaning.complete(&w.admin, task, 2).unwrap();
        let first = column(&w, task, "completed_at");

        set_now(&w, "2026-10-24T10:00:00Z");
        let again = w.h.cleaning.complete(&w.admin, task, 2);

        assert!(matches!(again, Err(AppError::Conflict)));
        assert!(matches!(w.h.cleaning.complete(&w.admin, task, 3), Err(AppError::Conflict)));
        assert!(matches!(w.h.cleaning.start(&w.admin, task, 3), Err(AppError::Conflict)));
        assert_eq!(column(&w, task, "completed_at"), first);
        assert_eq!(audits(&w, "cleaning_completed"), 1);
    }

    #[test]
    fn queue_orders_by_priority_then_age() {
        let w = world();
        let (_, task) = checked_out(&w);
        // Chegada às 14:00 do dia 23 (hoje = 23/10 12:00 local): em 2 h ⇒ urgente.
        let r2 = book(&w.h, &w.admin, &w.seed, "2026-10-23", "2026-10-25");
        assert!(r2 > 0);

        let queue = w.h.cleaning.queue(&w.admin).unwrap();

        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].task.id, task);
        assert_eq!(queue[0].priority, "urgent");
        assert_eq!(queue[0].suggested_priority, "urgent");
        assert_eq!(queue[0].next_arrival.as_deref(), Some("2026-10-23T14:00"));
    }

    #[test]
    fn queue_without_a_next_arrival_is_low() {
        let w = world();
        checked_out(&w);

        let queue = w.h.cleaning.queue(&w.admin).unwrap();

        assert_eq!(queue[0].priority, "low");
        assert!(queue[0].next_arrival.is_none());
    }

    #[test]
    fn late_departure_task_uses_the_real_time_and_urgent_priority_follows_the_arrival() {
        let w = world();
        let (_, stay) = paid_stay(&w);
        let next = book(&w.h, &w.admin, &w.seed, "2026-10-23", "2026-10-24");
        assert!(next > 0);
        // Saída 3 h atrasada: 23/10 15:00 local = 18:00Z.
        set_now(&w, "2026-10-23T18:00:00Z");
        let v = w.h.stays.get(&w.admin, stay).unwrap().stay.version;
        w.h.stays
            .checkout(&w.admin, CheckoutInput { stay_id: stay, expected_version: v, late_fee: LateFeeChoice::Waive { justification: "cortesia".into() }, balance_exception: None })
            .unwrap();
        let task = only_task(&w);

        assert_eq!(column(&w, task, "ready_since").as_deref(), Some("2026-10-23T18:00:00Z"));
        let queue = w.h.cleaning.queue(&w.admin).unwrap();
        assert_eq!(queue[0].priority, "urgent");
    }

    #[test]
    fn override_needs_a_justification_and_is_audited() {
        let w = world();
        let (_, task) = checked_out(&w);

        let blank = w.h.cleaning.set_priority(&w.admin, task, Some("high".into()), "   ".into(), 1);
        let unknown = w.h.cleaning.set_priority(&w.admin, task, Some("x".into()), "motivo".into(), 1);
        assert!(matches!(blank, Err(AppError::Validation(_))));
        assert!(matches!(unknown, Err(AppError::Validation(_))));
        assert_eq!(column(&w, task, "priority_override"), None);

        w.h.cleaning.set_priority(&w.admin, task, Some("high".into()), "hóspede pediu cedo".into(), 1).unwrap();

        assert_eq!(column(&w, task, "priority_override").as_deref(), Some("high"));
        assert_eq!(w.h.cleaning.queue(&w.admin).unwrap()[0].priority, "high");
        assert_eq!(audits(&w, "cleaning_priority_changed"), 1);
    }

    #[test]
    fn clearing_override_returns_to_the_suggestion() {
        let w = world();
        let (_, task) = checked_out(&w);
        w.h.cleaning.set_priority(&w.admin, task, Some("urgent".into()), "motivo".into(), 1).unwrap();

        w.h.cleaning.set_priority(&w.admin, task, None, "voltar ao automático".into(), 2).unwrap();

        let item = &w.h.cleaning.queue(&w.admin).unwrap()[0];
        assert_eq!(item.priority, item.suggested_priority);
        assert_eq!(item.priority, "low");
    }

    #[test]
    fn recepcao_can_start_and_complete() {
        let w = world();
        let (_, task) = checked_out(&w);
        let rec = recepcao_token(&w.h, &w.admin);

        w.h.cleaning.start(&rec, task, 1).unwrap();
        w.h.cleaning.complete(&rec, task, 2).unwrap();

        assert_eq!(column(&w, task, "status").as_deref(), Some("done"));
        let _ = Permission::ReleaseRoom; // nenhuma permissão extra é necessária
    }

    #[test]
    fn check_out_audit_is_in_the_same_transaction() {
        let w = world();
        checked_out(&w);

        // A auditoria do check-out e a tarefa existem juntas.
        let n: i64 = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_log WHERE module = 'stays' AND operation = 'check_out'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(count(&w, "cleaning_tasks"), 1);
    }

    #[test]
    fn an_unknown_task_and_a_stale_version_are_refused() {
        let w = world();
        let (_, task) = checked_out(&w);

        assert!(matches!(w.h.cleaning.start(&w.admin, 999, 1), Err(AppError::Validation(_))));
        assert!(matches!(w.h.cleaning.start(&w.admin, task, 9), Err(AppError::Conflict)));
    }
}
