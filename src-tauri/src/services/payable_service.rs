//! Despesas da pousada: categorias e contas a pagar (spec §10.2).

use serde_json::{json, Value};

use super::reservation_service::parse_date;
use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::domain::payable::payable_status;
use crate::domain::payment::Method;
use crate::domain::text::fold;
use crate::domain::{dates, money};
use crate::error::AppError;
use crate::models::page::{clamp_paging, Page};
use crate::models::payable::{
    CategoryTotal, ExpenseCategory, NewPayable, PayInput, PayableDetail, PayableQuery, PayableView, SummaryQuery,
};
use crate::repositories::payable_repo::{self, Filters, NewPayableRow, PayableRow};
use crate::repositories::sequence_repo;

/// Teto de uma conta ou pagamento (R$ 1.000.000,00).
const MAX_CENTS: i64 = 100_000_000;

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn clean(value: &str, max: usize, what: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(invalid(format!("Informe {what}.")));
    }
    if value.chars().count() > max {
        return Err(invalid(format!("{what}: no máximo {max} caracteres.")));
    }
    Ok(value.to_string())
}

/// Modalidade de pagamento de fornecedor: uma das cinco que a interface oferece.
fn method(value: &str) -> Result<Method, AppError> {
    Method::parse(value)
        .filter(|m| *m != Method::Unspecified)
        .ok_or_else(|| invalid("Escolha a forma de pagamento."))
}

fn friendly_category(error: AppError) -> AppError {
    match error {
        AppError::Database(msg) if msg.contains("UNIQUE") => invalid("Já existe uma categoria com este nome."),
        other => other,
    }
}

#[derive(Clone)]
pub struct PayableService {
    ctx: Ctx,
}

impl PayableService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn payables_actor(&self, token: &str) -> Result<SessionInfo, AppError> {
        let info = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&info, Permission::ManagePayables)?;
        Ok(info)
    }

    fn catalog_actor(&self, token: &str) -> Result<SessionInfo, AppError> {
        let info = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&info, Permission::ManageCatalog)?;
        Ok(info)
    }

    fn view(&self, row: PayableRow) -> Result<PayableView, AppError> {
        let due = parse_date(&row.due_date, "Vencimento")?;
        let status = payable_status(row.amount_cents, row.paid_cents, due, self.ctx.today());
        Ok(PayableView {
            id: row.id,
            code: row.code,
            supplier: row.supplier,
            description: row.description,
            amount_cents: row.amount_cents,
            paid_cents: row.paid_cents,
            balance_cents: (row.amount_cents - row.paid_cents).max(0),
            status: status.as_str().to_string(),
            status_label: status.label().to_string(),
            competence_date: row.competence_date,
            due_date: row.due_date,
            category_id: row.category_id,
            category_name: row.category_name,
            notes: row.notes,
            version: row.version,
            created_at: row.created_at,
        })
    }

    #[allow(clippy::too_many_arguments)] // cada chamada descreve um evento de auditoria completo
    fn audit(
        &self,
        tx: &rusqlite::Transaction,
        actor: &SessionInfo,
        module: &'static str,
        operation: &'static str,
        entity: &'static str,
        id: i64,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), AppError> {
        audit::record(
            tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module,
                operation,
                entity,
                entity_id: Some(id),
                before,
                after,
                justification: None,
            },
        )
    }

    // ---- contas ----

    /// Registra uma conta a pagar; com `paid_now` já nasce paga (despesa à vista), na mesma transação.
    pub fn create(&self, token: &str, input: NewPayable) -> Result<i64, AppError> {
        let actor = self.payables_actor(token)?;
        let supplier = clean(&input.supplier, 120, "o fornecedor")?;
        let description = clean(&input.description, 200, "a descrição")?;
        if !(1..=MAX_CENTS).contains(&input.amount_cents) {
            return Err(invalid("Informe um valor maior que zero e de até R$ 1.000.000,00."));
        }
        let competence = parse_date(&input.competence_date, "Data da despesa")?;
        let due = parse_date(&input.due_date, "Vencimento")?;
        if due < competence {
            return Err(invalid("O vencimento não pode ser antes da data da despesa."));
        }
        let notes = input.notes.trim();
        if notes.chars().count() > 500 {
            return Err(invalid("Observações: no máximo 500 caracteres."));
        }
        let paid_now = input.paid_now.as_ref().map(|p| method(&p.method)).transpose()?;

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let category = payable_repo::find_category(&tx, input.category_id)?
            .filter(|c| c.active)
            .ok_or_else(|| invalid("Escolha uma categoria ativa."))?;
        let at = self.ctx.utc.now_utc();
        let code = sequence_repo::format_code("CAP", sequence_repo::next(&tx, "payable")?);
        let id = payable_repo::insert(
            &tx,
            &NewPayableRow {
                code: &code,
                supplier: &supplier,
                description: &description,
                amount_cents: input.amount_cents,
                competence_date: &dates::format(competence),
                due_date: &dates::format(due),
                category_id: category.id,
                notes,
                created_by: actor.user_id,
            },
            at,
        )?;
        if let Some(m) = paid_now {
            payable_repo::insert_payment(&tx, id, input.amount_cents, m.as_str(), actor.user_id, "Pago à vista", at)?;
        }
        self.audit(
            &tx,
            &actor,
            "payables",
            "create_payable",
            "payable",
            id,
            None,
            Some(json!({
                "code": code, "supplier": supplier, "amount_cents": input.amount_cents, "category": category.name,
                "competence_date": input.competence_date, "due_date": input.due_date,
                "paid_now_method": paid_now.map(Method::as_str),
            })),
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub fn list(&self, token: &str, query: PayableQuery) -> Result<Page<PayableView>, AppError> {
        self.payables_actor(token)?;
        if query.status.as_deref().is_some_and(|s| !["pending", "partial", "paid", "overdue"].contains(&s)) {
            return Err(invalid("Situação inválida."));
        }
        for (value, label) in [(&query.due_from, "Vencimento inicial"), (&query.due_to, "Vencimento final")] {
            if let Some(v) = value {
                parse_date(v, label)?;
            }
        }
        let (page, page_size, offset) = clamp_paging(query.page, query.page_size);
        let conn = self.ctx.conn()?;
        let filters = Filters {
            status: query.status.as_deref(),
            category_id: query.category_id,
            due_from: query.due_from.as_deref(),
            due_to: query.due_to.as_deref(),
        };
        let (rows, total) = payable_repo::list(&conn, &filters, &dates::format(self.ctx.today()), i64::from(page_size), offset)?;
        let items = rows.into_iter().map(|r| self.view(r)).collect::<Result<Vec<_>, _>>()?;
        Ok(Page { items, total, page, page_size })
    }

    pub fn get(&self, token: &str, id: i64) -> Result<PayableDetail, AppError> {
        self.payables_actor(token)?;
        let conn = self.ctx.conn()?;
        let row = payable_repo::find(&conn, id)?.ok_or_else(|| invalid("Conta não encontrada."))?;
        Ok(PayableDetail { payable: self.view(row)?, payments: payable_repo::payments(&conn, id)? })
    }

    /// Pagamento parcial ou total de uma conta; nunca passa do saldo.
    pub fn pay(&self, token: &str, id: i64, input: PayInput, expected_version: i64) -> Result<i64, AppError> {
        let actor = self.payables_actor(token)?;
        let m = method(&input.method)?;
        let note = input.note.trim();
        if note.chars().count() > 200 {
            return Err(invalid("Observação: no máximo 200 caracteres."));
        }
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let row = payable_repo::find(&tx, id)?.ok_or_else(|| invalid("Conta não encontrada."))?;
        if row.version != expected_version {
            return Err(AppError::Conflict);
        }
        let balance = money::sub(row.amount_cents, row.paid_cents)?;
        if balance <= 0 {
            return Err(invalid("Esta conta já está paga."));
        }
        if input.amount_cents < 1 || input.amount_cents > balance {
            return Err(invalid("Informe um valor maior que zero e que não passe do saldo da conta."));
        }
        let at = self.ctx.utc.now_utc();
        let payment_id = payable_repo::insert_payment(&tx, id, input.amount_cents, m.as_str(), actor.user_id, note, at)?;
        payable_repo::bump(&tx, id, expected_version, at)?;
        self.audit(
            &tx,
            &actor,
            "payables",
            "pay_payable",
            "payable",
            id,
            Some(json!({ "paid_cents": row.paid_cents })),
            Some(json!({
                "paid_cents": row.paid_cents + input.amount_cents,
                "payment": { "id": payment_id, "amount_cents": input.amount_cents, "method": m.as_str() },
            })),
        )?;
        tx.commit()?;
        Ok(payment_id)
    }

    /// Totais por categoria pela competência (data da despesa) ou pela data efetiva de cada pagamento.
    pub fn summary(&self, token: &str, query: SummaryQuery) -> Result<Vec<CategoryTotal>, AppError> {
        self.payables_actor(token)?;
        let from = parse_date(&query.from, "Data inicial")?;
        let to = parse_date(&query.to, "Data final")?;
        if to < from {
            return Err(invalid("A data final não pode ser antes da inicial."));
        }
        let conn = self.ctx.conn()?;
        match query.basis.as_str() {
            "competence" => payable_repo::summary_by_competence(&conn, &dates::format(from), &dates::format(to), query.category_id),
            "payment" => {
                let (start, end) = dates::utc_bounds(from, to, self.ctx.utc.local_offset());
                payable_repo::summary_by_payment(&conn, &start, &end, query.category_id)
            }
            _ => Err(invalid("Escolha ver por competência ou por data de pagamento.")),
        }
    }

    // ---- categorias ----

    pub fn list_categories(&self, token: &str, only_active: bool) -> Result<Vec<ExpenseCategory>, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        payable_repo::list_categories(&conn, only_active)
    }

    pub fn create_category(&self, token: &str, name: &str) -> Result<i64, AppError> {
        let actor = self.catalog_actor(token)?;
        let name = clean(name, 60, "o nome da categoria")?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let id = payable_repo::insert_category(&tx, &name, &fold(&name), self.ctx.utc.now_utc()).map_err(friendly_category)?;
        self.audit(&tx, &actor, "expense_categories", "create_category", "expense_category", id, None, Some(json!({ "name": name })))?;
        tx.commit()?;
        Ok(id)
    }

    pub fn rename_category(&self, token: &str, id: i64, name: &str, expected_version: i64) -> Result<(), AppError> {
        let actor = self.catalog_actor(token)?;
        let name = clean(name, 60, "o nome da categoria")?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let before = payable_repo::find_category(&tx, id)?.ok_or_else(|| invalid("Categoria não encontrada."))?;
        payable_repo::rename_category(&tx, id, &name, &fold(&name), expected_version, self.ctx.utc.now_utc()).map_err(friendly_category)?;
        self.audit(
            &tx,
            &actor,
            "expense_categories",
            "rename_category",
            "expense_category",
            id,
            Some(json!({ "name": before.name })),
            Some(json!({ "name": name })),
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Categorias com histórico nunca são apagadas: só desativadas.
    pub fn set_category_active(&self, token: &str, id: i64, active: bool, expected_version: i64) -> Result<(), AppError> {
        let actor = self.catalog_actor(token)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let before = payable_repo::find_category(&tx, id)?.ok_or_else(|| invalid("Categoria não encontrada."))?;
        payable_repo::set_category_active(&tx, id, active, expected_version, self.ctx.utc.now_utc())?;
        self.audit(
            &tx,
            &actor,
            "expense_categories",
            "set_category_active",
            "expense_category",
            id,
            Some(json!({ "active": before.active })),
            Some(json!({ "active": active })),
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
    use crate::models::payable::{NewPayable, PayInput, PayNow, PayableQuery, SummaryQuery};
    use crate::services::test_support::{admin_token, harness, recepcao_token, Harness};

    // "Hoje" nos testes é 21/10/2026, 09:00 local (12:00Z; fuso -03:00). Categoria 1 = Energia (semeada).

    struct World {
        h: Harness,
        admin: String,
    }

    fn world() -> World {
        let h = harness();
        let admin = admin_token(&h);
        World { h, admin }
    }

    fn set_now(w: &World, rfc3339: &str) {
        w.h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    fn bill(category_id: i64) -> NewPayable {
        NewPayable {
            supplier: "Companhia de Energia".into(),
            description: "Conta de outubro".into(),
            amount_cents: 50_000,
            competence_date: "2026-10-15".into(),
            due_date: "2026-11-10".into(),
            category_id,
            notes: String::new(),
            paid_now: None,
        }
    }

    fn pay(cents: i64, method: &str) -> PayInput {
        PayInput { amount_cents: cents, method: method.into(), note: String::new() }
    }

    fn count(w: &World, table: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    fn query() -> PayableQuery {
        PayableQuery { status: None, category_id: None, due_from: None, due_to: None, page: 1, page_size: 50 }
    }

    fn status(w: &World, id: i64) -> String {
        w.h.payables.get(&w.admin, id).unwrap().payable.status
    }

    fn rec_with(w: &World, permissions: &[Permission]) -> String {
        if w.h.auth.login("rec", "Definitiva-456").is_err() {
            let _ = recepcao_token(&w.h, &w.admin);
        }
        let id = w.h.auth.login("rec", "Definitiva-456").unwrap().user.id;
        w.h.users.set_extra_permissions(&w.admin, id, permissions).unwrap();
        w.h.auth.login("rec", "Definitiva-456").unwrap().token
    }

    #[test]
    fn create_payable_starts_pending_with_code_and_audit() {
        let w = world();

        let id = w.h.payables.create(&w.admin, bill(1)).unwrap();

        let detail = w.h.payables.get(&w.admin, id).unwrap();
        assert_eq!(detail.payable.code, "CAP-000001");
        assert_eq!((detail.payable.status.as_str(), detail.payable.status_label.as_str()), ("pending", "Pendente"));
        assert_eq!((detail.payable.amount_cents, detail.payable.paid_cents, detail.payable.balance_cents), (50_000, 0, 50_000));
        assert_eq!(detail.payable.category_name, "Energia");
        let audited: i64 = w.h.db.lock().unwrap().query_row("SELECT count(*) FROM audit_log WHERE operation = 'create_payable'", [], |r| r.get(0)).unwrap();
        assert_eq!(audited, 1);
    }

    #[test]
    fn requires_manage_payables_for_every_operation() {
        let w = world();
        let id = w.h.payables.create(&w.admin, bill(1)).unwrap();
        let rec = rec_with(&w, &[]);

        assert!(matches!(w.h.payables.create(&rec, bill(1)), Err(AppError::Forbidden)));
        assert!(matches!(w.h.payables.list(&rec, query()), Err(AppError::Forbidden)));
        assert!(matches!(w.h.payables.get(&rec, id), Err(AppError::Forbidden)));
        assert!(matches!(w.h.payables.pay(&rec, id, pay(100, "cash"), 1), Err(AppError::Forbidden)));
        assert!(w.h.payables.list_categories(&rec, true).is_ok(), "categorias podem ser lidas por quem está autenticado");

        let rec = rec_with(&w, &[Permission::ManagePayables]);
        assert!(w.h.payables.list(&rec, query()).is_ok());
        assert!(w.h.payables.pay(&rec, id, pay(100, "cash"), 1).is_ok());
    }

    #[test]
    fn partial_then_full_payment_moves_pending_partial_paid() {
        let w = world();
        let id = w.h.payables.create(&w.admin, bill(1)).unwrap();

        w.h.payables.pay(&w.admin, id, pay(20_000, "pix"), 1).unwrap();
        let partial = w.h.payables.get(&w.admin, id).unwrap();
        assert_eq!((partial.payable.status.as_str(), partial.payable.balance_cents), ("partial", 30_000));
        assert_eq!(partial.payments.len(), 1);

        w.h.payables.pay(&w.admin, id, pay(30_000, "cash"), 2).unwrap();
        assert_eq!(status(&w, id), "paid");

        assert!(matches!(w.h.payables.pay(&w.admin, id, pay(1, "cash"), 3), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "payable_payments"), 2);
    }

    #[test]
    fn payment_above_balance_or_invalid_is_refused_without_writing() {
        let w = world();
        let id = w.h.payables.create(&w.admin, bill(1)).unwrap();

        for (cents, method) in [(50_001, "pix"), (0, "pix"), (-1, "pix"), (100, "cheque"), (100, "unspecified")] {
            assert!(matches!(w.h.payables.pay(&w.admin, id, pay(cents, method), 1), Err(AppError::Validation(_))), "{cents} {method}");
        }

        assert_eq!(count(&w, "payable_payments"), 0);
        assert_eq!(status(&w, id), "pending");
    }

    #[test]
    fn stale_version_on_pay_is_conflict() {
        let w = world();
        let id = w.h.payables.create(&w.admin, bill(1)).unwrap();
        w.h.payables.pay(&w.admin, id, pay(1_000, "cash"), 1).unwrap();

        assert!(matches!(w.h.payables.pay(&w.admin, id, pay(1_000, "cash"), 1), Err(AppError::Conflict)));
        assert_eq!(count(&w, "payable_payments"), 1);
    }

    #[test]
    fn due_today_is_pending_and_due_yesterday_is_overdue() {
        let w = world();
        let mut today = bill(1);
        (today.competence_date, today.due_date) = ("2026-10-01".into(), "2026-10-21".into());
        let mut yesterday = bill(1);
        (yesterday.competence_date, yesterday.due_date) = ("2026-10-01".into(), "2026-10-20".into());
        let due_today = w.h.payables.create(&w.admin, today).unwrap();
        let due_yesterday = w.h.payables.create(&w.admin, yesterday).unwrap();

        assert_eq!((status(&w, due_today).as_str(), status(&w, due_yesterday).as_str()), ("pending", "overdue"));

        set_now(&w, "2026-10-22T12:00:00Z");
        assert_eq!(status(&w, due_today), "overdue");
        w.h.payables.pay(&w.admin, due_today, pay(50_000, "pix"), 1).unwrap();
        assert_eq!(status(&w, due_today), "paid");
        let overdue = PayableQuery { status: Some("overdue".into()), ..query() };
        let page = w.h.payables.list(&w.admin, overdue).unwrap();
        assert_eq!(page.items.iter().map(|p| p.id).collect::<Vec<_>>(), [due_yesterday]);
    }

    #[test]
    fn paid_now_records_payable_and_payment_atomically() {
        let w = world();
        let mut cash = bill(1);
        cash.paid_now = Some(PayNow { method: "pix".into() });

        let id = w.h.payables.create(&w.admin, cash).unwrap();

        let detail = w.h.payables.get(&w.admin, id).unwrap();
        assert_eq!((detail.payable.status.as_str(), detail.payments.len(), detail.payments[0].amount_cents), ("paid", 1, 50_000));

        let mut bad = bill(1);
        bad.paid_now = Some(PayNow { method: "cheque".into() });
        assert!(matches!(w.h.payables.create(&w.admin, bad), Err(AppError::Validation(_))));
        assert_eq!((count(&w, "payables"), count(&w, "payable_payments")), (1, 1));
        let next: i64 = w.h.db.lock().unwrap().query_row("SELECT next_value FROM sequences WHERE name = 'payable'", [], |r| r.get(0)).unwrap();
        assert_eq!(next, 2);
    }

    #[test]
    fn invalid_payable_fields_are_refused_without_writing() {
        let w = world();
        type Change = Box<dyn Fn(&mut NewPayable)>;
        let cases: Vec<Change> = vec![
            Box::new(|b| b.supplier = "  ".into()),
            Box::new(|b| b.supplier = "x".repeat(121)),
            Box::new(|b| b.description = String::new()),
            Box::new(|b| b.description = "x".repeat(201)),
            Box::new(|b| b.amount_cents = 0),
            Box::new(|b| b.amount_cents = 100_000_001),
            Box::new(|b| b.competence_date = "15/10/2026".into()),
            Box::new(|b| b.due_date = "2026-10-14".into()), // vence antes da competência
            Box::new(|b| b.category_id = 999),
            Box::new(|b| b.notes = "x".repeat(501)),
        ];

        for change in cases {
            let mut input = bill(1);
            change(&mut input);
            assert!(matches!(w.h.payables.create(&w.admin, input), Err(AppError::Validation(_))));
        }

        assert_eq!(count(&w, "payables"), 0);
    }

    #[test]
    fn inactive_category_rejects_new_payables_but_keeps_history() {
        let w = world();
        let id = w.h.payables.create(&w.admin, bill(1)).unwrap();
        let category = w.h.payables.list_categories(&w.admin, false).unwrap().into_iter().find(|c| c.id == 1).unwrap();

        w.h.payables.set_category_active(&w.admin, 1, false, category.version).unwrap();

        assert!(matches!(w.h.payables.create(&w.admin, bill(1)), Err(AppError::Validation(_))));
        assert_eq!(w.h.payables.get(&w.admin, id).unwrap().payable.category_name, "Energia");
        assert!(w.h.payables.list_categories(&w.admin, true).unwrap().iter().all(|c| c.id != 1));
        w.h.payables.set_category_active(&w.admin, 1, true, category.version + 1).unwrap();
        assert!(w.h.payables.create(&w.admin, bill(1)).is_ok());
    }

    #[test]
    fn category_names_are_unique_ignoring_accents_and_case() {
        let w = world();

        let id = w.h.payables.create_category(&w.admin, "Seguros").unwrap();

        assert!(matches!(w.h.payables.create_category(&w.admin, "SEGUROS"), Err(AppError::Validation(_))));
        assert!(matches!(w.h.payables.create_category(&w.admin, "agua"), Err(AppError::Validation(_))), "Água já existe");
        assert!(matches!(w.h.payables.create_category(&w.admin, "  "), Err(AppError::Validation(_))));
        w.h.payables.rename_category(&w.admin, id, "Seguros e taxas", 1).unwrap();
        assert!(matches!(w.h.payables.rename_category(&w.admin, id, "Energia", 2), Err(AppError::Validation(_))));
        assert!(matches!(w.h.payables.rename_category(&w.admin, id, "Outro", 1), Err(AppError::Conflict)));
    }

    #[test]
    fn category_changes_require_manage_catalog() {
        let w = world();
        let rec = rec_with(&w, &[Permission::ManagePayables]);

        assert!(matches!(w.h.payables.create_category(&rec, "Seguros"), Err(AppError::Forbidden)));
        assert!(matches!(w.h.payables.rename_category(&rec, 1, "X", 1), Err(AppError::Forbidden)));
        assert!(matches!(w.h.payables.set_category_active(&rec, 1, false, 1), Err(AppError::Forbidden)));
    }

    #[test]
    fn summary_by_competence_and_by_payment_differ() {
        let w = world();
        let id = w.h.payables.create(&w.admin, bill(1)).unwrap(); // competência 15/10, R$ 500,00
        w.h.payables.pay(&w.admin, id, pay(20_000, "pix"), 1).unwrap(); // 21/10
        set_now(&w, "2026-11-05T12:00:00Z");
        w.h.payables.pay(&w.admin, id, pay(30_000, "cash"), 2).unwrap(); // 05/11
        let total = |from: &str, to: &str, basis: &str| -> i64 {
            w.h.payables
                .summary(&w.admin, SummaryQuery { from: from.into(), to: to.into(), basis: basis.into(), category_id: None })
                .unwrap()
                .iter()
                .map(|c| c.total_cents)
                .sum()
        };

        assert_eq!(total("2026-10-01", "2026-10-31", "competence"), 50_000);
        assert_eq!(total("2026-11-01", "2026-11-30", "competence"), 0);
        assert_eq!(total("2026-10-01", "2026-10-31", "payment"), 20_000);
        assert_eq!(total("2026-11-01", "2026-11-30", "payment"), 30_000);
        let by_category = w.h.payables
            .summary(&w.admin, SummaryQuery { from: "2026-10-01".into(), to: "2026-11-30".into(), basis: "payment".into(), category_id: Some(1) })
            .unwrap();
        assert_eq!((by_category[0].category_name.as_str(), by_category[0].total_cents), ("Energia", 50_000));
        assert!(matches!(
            w.h.payables.summary(&w.admin, SummaryQuery { from: "2026-10-01".into(), to: "2026-10-31".into(), basis: "x".into(), category_id: None }),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn every_payable_operation_is_audited() {
        let w = world();
        let id = w.h.payables.create(&w.admin, bill(1)).unwrap();
        w.h.payables.pay(&w.admin, id, pay(1_000, "cash"), 1).unwrap();

        let operations: Vec<String> = {
            let conn = w.h.db.lock().unwrap();
            let mut stmt = conn.prepare("SELECT operation FROM audit_log WHERE module = 'payables' ORDER BY id").unwrap();
            stmt.query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect()
        };
        assert_eq!(operations, ["create_payable", "pay_payable"]);
    }
}
