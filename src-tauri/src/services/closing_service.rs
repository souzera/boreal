//! Fechamento diário de caixa (spec §11): esperado por modalidade, conferido, diferença e justificativa.

use serde_json::json;

use super::reservation_service::parse_date;
use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::domain::closing::{build_closing_lines, CountedLine, ExpectedLine};
use crate::domain::payment::Method;
use crate::domain::{dates, money};
use crate::error::AppError;
use crate::models::closing::{CloseInput, ClosingPreview, ClosingView, ExpectedLineView};
use crate::repositories::closing_repo;

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

#[derive(Clone)]
pub struct ClosingService {
    ctx: Ctx,
}

impl ClosingService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    /// Esperado por modalidade para a data local: recebido − devolvido. Débito e crédito ficam separados e o
    /// legado sem modalidade só aparece quando há valor.
    fn expected_for(&self, conn: &rusqlite::Connection, date: chrono::NaiveDate) -> Result<(Vec<ExpectedLineView>, i64), AppError> {
        let (start, end) = dates::utc_bounds(date, date, self.ctx.utc.local_offset());
        let received = closing_repo::received(conn, &start, &end)?;
        let refunded = closing_repo::refunded(conn, &start, &end)?;
        let mut lines = Vec::new();
        for method in Method::selectable().into_iter().chain([Method::Unspecified]) {
            let key = method.as_str();
            let (got, gave) = (received.get(key).copied().unwrap_or(0), refunded.get(key).copied().unwrap_or(0));
            if method == Method::Unspecified && got == 0 && gave == 0 {
                continue;
            }
            lines.push(ExpectedLineView {
                method: key.to_string(),
                label: method.label().to_string(),
                received_cents: got,
                refunded_cents: gave,
                expected_cents: money::sub(got, gave)?,
            });
        }
        Ok((lines, closing_repo::supplier_payments(conn, &start, &end)?))
    }

    /// Situação do dia: esperado calculado agora, fechamento gravado (se houver) e o que mudou depois dele.
    pub fn preview(&self, token: &str, date: &str) -> Result<ClosingPreview, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let day = parse_date(date, "Data")?;
        let conn = self.ctx.conn()?;
        let (expected, supplier_payments_cents) = self.expected_for(&conn, day)?;
        let closed = closing_repo::find_by_date(&conn, &dates::format(day))?;
        let after_closing_cents = match &closed {
            Some(view) => {
                let now = money::sum(expected.iter().map(|l| l.expected_cents))?;
                let stored = money::sum(view.lines.iter().map(|l| l.expected_cents))?;
                Some(money::sub(now, stored)?)
            }
            None => None,
        };
        Ok(ClosingPreview { date: dates::format(day), closed, expected, supplier_payments_cents, after_closing_cents })
    }

    /// Conclui o fechamento do dia. O esperado é recalculado aqui; diferença exige justificativa mas não impede,
    /// e nenhum recebimento é alterado.
    pub fn close(&self, token: &str, input: CloseInput) -> Result<ClosingView, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let day = parse_date(&input.date, "Data")?;
        if day > self.ctx.today() {
            return Err(invalid("Não é possível fechar um dia que ainda não aconteceu."));
        }
        let mut counted = Vec::new();
        for item in &input.counted {
            let method = Method::parse(&item.method).ok_or_else(|| invalid("Forma de pagamento inválida no fechamento."))?;
            if counted.iter().any(|c: &CountedLine| c.method == method) {
                return Err(invalid("Cada forma de pagamento só pode aparecer uma vez."));
            }
            counted.push(CountedLine { method, counted_cents: item.counted_cents, justification: item.justification.clone() });
        }

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let date = dates::format(day);
        if closing_repo::find_by_date(&tx, &date)?.is_some() {
            return Err(invalid("Esse dia já foi fechado."));
        }
        let (expected_views, _) = self.expected_for(&tx, day)?;
        let expected: Vec<ExpectedLine> = expected_views
            .iter()
            .filter_map(|e| Method::parse(&e.method).map(|method| ExpectedLine { method, expected_cents: e.expected_cents }))
            .collect();
        let lines = build_closing_lines(&expected, &counted)?;

        let at = self.ctx.utc.now_utc();
        let id = closing_repo::insert(&tx, &date, at, actor.user_id).map_err(|e| match e {
            AppError::Database(msg) if msg.contains("UNIQUE") => invalid("Esse dia já foi fechado."),
            other => other,
        })?;
        for line in &lines {
            closing_repo::insert_line(&tx, id, line)?;
        }
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "finance",
                operation: "close_cash",
                entity: "cash_closing",
                entity_id: Some(id),
                before: None,
                after: Some(json!({
                    "closing_date": date,
                    "lines": lines.iter().map(|l| json!({
                        "method": l.method.as_str(), "expected_cents": l.expected_cents, "counted_cents": l.counted_cents,
                        "difference_cents": l.difference_cents, "justification": l.justification,
                    })).collect::<Vec<_>>(),
                })),
                justification: None,
            },
        )?;
        let view = closing_repo::find_by_date(&tx, &date)?.ok_or_else(|| invalid("Fechamento não encontrado."))?;
        tx.commit()?;
        Ok(view)
    }

    /// Dias anteriores a hoje com movimento e sem fechamento (só avisos: nada fica bloqueado).
    pub fn pending_dates(&self, token: &str) -> Result<Vec<String>, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        let shift = format!("{:+} seconds", self.ctx.utc.local_offset().local_minus_utc());
        closing_repo::pending_dates(&conn, &shift, &dates::format(self.ctx.today()))
    }
}


#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::error::AppError;
    use crate::models::closing::{CloseInput, CountedInput};
    use crate::models::payable::{NewPayable, PayInput};
    use crate::models::payment::{ComponentInput, PaymentInput, PaymentTarget};
    use crate::models::refund::RefundInput;
    use crate::services::test_support::{admin_token, book, harness, seed, Harness, Seed};

    // Fuso -03:00. A reserva de teste tem 10 noites (R$ 2.000,00), o que comporta vários sinais.

    struct World {
        h: Harness,
        admin: String,
        reservation: i64,
    }

    fn world() -> World {
        let h = harness();
        let admin = admin_token(&h);
        let seed: Seed = seed(&h, &admin, 20_000, 2);
        let reservation = book(&h, &admin, &seed, "2026-11-10", "2026-11-20");
        World { h, admin, reservation }
    }

    fn at(w: &World, rfc3339: &str) {
        w.h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    fn parts(items: &[(&str, i64)]) -> Vec<ComponentInput> {
        items.iter().map(|(m, c)| ComponentInput { method: (*m).into(), amount_cents: *c }).collect()
    }

    /// Recebe um sinal dividido na reserva de teste e devolve o id do recebimento.
    fn receive(w: &World, items: &[(&str, i64)]) -> i64 {
        let version = w.h.reservations.get(&w.admin, w.reservation).unwrap().reservation.version;
        w.h.payments
            .record(
                &w.admin,
                PaymentTarget::Reservation { reservation_id: w.reservation, expected_version: version },
                PaymentInput { components: parts(items), note: String::new(), duplicate_release: None },
            )
            .unwrap()
            .id
    }

    fn refund(w: &World, payment_id: i64, items: &[(&str, i64)]) {
        w.h.refunds.record(&w.admin, RefundInput { payment_id, components: parts(items), reason: "Ajuste".into() }).unwrap();
    }

    fn counted(cash: i64, pix: i64, debit: i64, credit: i64, transfer: i64, why: Option<&str>) -> Vec<CountedInput> {
        [("cash", cash), ("pix", pix), ("debit", debit), ("credit", credit), ("transfer", transfer)]
            .into_iter()
            .map(|(m, c)| CountedInput { method: m.into(), counted_cents: c, justification: why.map(str::to_string) })
            .collect()
    }

    fn close(w: &World, date: &str, lines: Vec<CountedInput>) -> Result<crate::models::closing::ClosingView, AppError> {
        w.h.closing.close(&w.admin, CloseInput { date: date.into(), counted: lines })
    }

    fn expected(w: &World, date: &str) -> Vec<(String, i64)> {
        w.h.closing.preview(&w.admin, date).unwrap().expected.into_iter().map(|e| (e.method, e.expected_cents)).collect()
    }

    fn snapshot(w: &World) -> Vec<(i64, i64, String)> {
        let conn = w.h.db.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, amount_cents, received_at FROM payments ORDER BY id").unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().map(Result::unwrap).collect()
    }

    fn count(w: &World, table: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn expected_is_received_minus_refunded_per_method_with_debit_and_credit_apart() {
        let w = world();
        at(&w, "2026-10-21T15:00:00Z");
        let first = receive(&w, &[("pix", 10_000)]);
        receive(&w, &[("debit", 5_000)]);
        receive(&w, &[("credit", 3_000)]);
        refund(&w, first, &[("pix", 2_000)]);

        let by_method = expected(&w, "2026-10-21");

        assert_eq!(
            by_method,
            [("cash", 0), ("pix", 8_000), ("debit", 5_000), ("credit", 3_000), ("transfer", 0)].map(|(m, c)| (m.to_string(), c))
        );
        let preview = w.h.closing.preview(&w.admin, "2026-10-21").unwrap();
        let pix = preview.expected.iter().find(|e| e.method == "pix").unwrap();
        assert_eq!((pix.received_cents, pix.refunded_cents, pix.label.as_str()), (10_000, 2_000, "Pix"));
    }

    #[test]
    fn closing_with_difference_and_justification_completes_and_stores_everything() {
        let w = world();
        at(&w, "2026-10-21T15:00:00Z");
        receive(&w, &[("cash", 10_000)]);
        let before = snapshot(&w);

        let view = close(&w, "2026-10-21", counted(9_500, 0, 0, 0, 0, None).into_iter().map(|mut c| {
            if c.method == "cash" {
                c.justification = Some("Troco devolvido".into());
            }
            c
        }).collect()).unwrap();

        let cash = view.lines.iter().find(|l| l.method == "cash").unwrap();
        assert_eq!((cash.expected_cents, cash.counted_cents, cash.difference_cents), (10_000, 9_500, -500));
        assert_eq!(cash.justification.as_deref(), Some("Troco devolvido"));
        assert_eq!((view.closing_date.as_str(), view.closed_by_name.as_str()), ("2026-10-21", "Administrador"));
        assert_eq!(snapshot(&w), before, "o fechamento não altera recebimentos");
        let audited: i64 = w.h.db.lock().unwrap().query_row("SELECT count(*) FROM audit_log WHERE operation = 'close_cash'", [], |r| r.get(0)).unwrap();
        assert_eq!(audited, 1);
        assert!(w.h.closing.preview(&w.admin, "2026-10-21").unwrap().closed.is_some());
    }

    #[test]
    fn invalid_closings_are_refused_without_writing() {
        let w = world();
        at(&w, "2026-10-21T15:00:00Z");
        receive(&w, &[("cash", 10_000)]);
        let mut missing = counted(10_000, 0, 0, 0, 0, None);
        missing.retain(|c| c.method != "pix");
        let mut repeated = counted(10_000, 0, 0, 0, 0, None);
        repeated.push(CountedInput { method: "pix".into(), counted_cents: 0, justification: None });
        let mut unknown = counted(10_000, 0, 0, 0, 0, None);
        unknown[0].method = "cheque".into();

        for (label, lines) in [
            ("diferença sem justificativa", counted(9_500, 0, 0, 0, 0, None)),
            ("modalidade faltando", missing),
            ("modalidade repetida", repeated),
            ("modalidade desconhecida", unknown),
        ] {
            assert!(matches!(close(&w, "2026-10-21", lines), Err(AppError::Validation(_))), "{label}");
        }

        assert_eq!((count(&w, "cash_closings"), count(&w, "cash_closing_lines")), (0, 0));
    }

    #[test]
    fn closing_twice_for_the_same_date_is_refused() {
        let w = world();
        at(&w, "2026-10-21T15:00:00Z");

        close(&w, "2026-10-21", counted(0, 0, 0, 0, 0, None)).unwrap();

        assert!(matches!(close(&w, "2026-10-21", counted(0, 0, 0, 0, 0, None)), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "cash_closings"), 1);
    }

    #[test]
    fn future_date_is_refused() {
        let w = world();
        at(&w, "2026-10-21T15:00:00Z");

        assert!(matches!(close(&w, "2026-10-22", counted(0, 0, 0, 0, 0, None)), Err(AppError::Validation(_))));
        assert!(matches!(close(&w, "22/10/2026", counted(0, 0, 0, 0, 0, None)), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "cash_closings"), 0);
    }

    #[test]
    fn past_date_can_be_closed_late_without_moving_movements() {
        let w = world();
        at(&w, "2026-10-20T15:00:00Z");
        receive(&w, &[("cash", 7_000)]);
        at(&w, "2026-10-21T15:00:00Z");
        receive(&w, &[("cash", 4_000)]);

        let view = close(&w, "2026-10-20", counted(7_000, 0, 0, 0, 0, None)).unwrap();

        assert_eq!(view.lines.iter().find(|l| l.method == "cash").unwrap().expected_cents, 7_000);
        assert_eq!(expected(&w, "2026-10-21")[0], ("cash".to_string(), 4_000));
    }

    #[test]
    fn local_midnight_boundary_uses_the_local_date() {
        let w = world();
        at(&w, "2026-10-22T02:30:00Z"); // 21/10 23:30 local
        receive(&w, &[("cash", 10_000)]);
        at(&w, "2026-10-22T03:10:00Z"); // 22/10 00:10 local
        receive(&w, &[("cash", 2_000)]);
        at(&w, "2026-10-22T15:00:00Z");

        assert_eq!(expected(&w, "2026-10-21")[0], ("cash".to_string(), 10_000));
        assert_eq!(expected(&w, "2026-10-22")[0], ("cash".to_string(), 2_000));
    }

    #[test]
    fn pending_closing_never_blocks_other_operations() {
        let w = world();
        at(&w, "2026-10-20T15:00:00Z");
        let old = receive(&w, &[("pix", 7_000)]);
        at(&w, "2026-10-21T15:00:00Z");

        assert_eq!(w.h.closing.pending_dates(&w.admin).unwrap(), ["2026-10-20"]);
        let today = receive(&w, &[("pix", 3_000)]);
        refund(&w, today, &[("pix", 1_000)]);
        refund(&w, old, &[("pix", 500)]);

        assert_eq!(w.h.closing.pending_dates(&w.admin).unwrap(), ["2026-10-20"], "o dia de hoje não é pendência");
        close(&w, "2026-10-20", counted(0, 7_000, 0, 0, 0, None)).unwrap();
        assert!(w.h.closing.pending_dates(&w.admin).unwrap().is_empty());
    }

    #[test]
    fn movements_after_closing_are_reported_but_not_added() {
        let w = world();
        at(&w, "2026-10-21T15:00:00Z");
        receive(&w, &[("cash", 10_000)]);
        close(&w, "2026-10-21", counted(10_000, 0, 0, 0, 0, None)).unwrap();

        at(&w, "2026-10-21T20:00:00Z");
        receive(&w, &[("cash", 2_500)]);

        let preview = w.h.closing.preview(&w.admin, "2026-10-21").unwrap();
        assert_eq!(preview.after_closing_cents, Some(2_500));
        let stored = preview.closed.unwrap().lines.into_iter().find(|l| l.method == "cash").unwrap();
        assert_eq!(stored.expected_cents, 10_000, "o fechamento gravado não muda");
        assert_eq!(w.h.closing.preview(&w.admin, "2026-10-20").unwrap().after_closing_cents, None);
    }

    #[test]
    fn supplier_payments_are_listed_apart_and_not_subtracted() {
        let w = world();
        at(&w, "2026-10-21T15:00:00Z");
        receive(&w, &[("cash", 10_000)]);
        let payable = w
            .h
            .payables
            .create(
                &w.admin,
                NewPayable {
                    supplier: "Fornecedor".into(),
                    description: "Gás".into(),
                    amount_cents: 3_000,
                    competence_date: "2026-10-21".into(),
                    due_date: "2026-10-30".into(),
                    category_id: 1,
                    notes: String::new(),
                    paid_now: None,
                },
            )
            .unwrap();
        w.h.payables.pay(&w.admin, payable, PayInput { amount_cents: 3_000, method: "cash".into(), note: String::new() }, 1).unwrap();

        let preview = w.h.closing.preview(&w.admin, "2026-10-21").unwrap();

        assert_eq!(preview.supplier_payments_cents, 3_000);
        assert_eq!(preview.expected[0].expected_cents, 10_000);
    }

    #[test]
    fn a_day_with_more_refunds_than_receipts_can_be_counted_with_a_negative_amount() {
        let w = world();
        at(&w, "2026-10-20T15:00:00Z");
        let old = receive(&w, &[("cash", 10_000)]);
        at(&w, "2026-10-21T15:00:00Z");
        refund(&w, old, &[("cash", 10_000)]);
        assert_eq!(expected(&w, "2026-10-21")[0], ("cash".to_string(), -10_000));

        let view = close(&w, "2026-10-21", counted(-10_000, 0, 0, 0, 0, None)).unwrap();

        let cash = view.lines.iter().find(|l| l.method == "cash").unwrap();
        assert_eq!((cash.expected_cents, cash.counted_cents, cash.difference_cents), (-10_000, -10_000, 0));
    }

    #[test]
    fn legacy_unspecified_movements_get_their_own_line() {
        let w = world();
        at(&w, "2026-10-21T15:00:00Z");
        receive(&w, &[("cash", 12_000)]);
        w.h.db.lock().unwrap().execute("UPDATE payment_components SET method = 'unspecified'", []).unwrap();

        let methods: Vec<String> = w.h.closing.preview(&w.admin, "2026-10-21").unwrap().expected.into_iter().map(|e| e.method).collect();
        assert_eq!(methods.last().map(String::as_str), Some("unspecified"));
        assert_eq!(methods.len(), 6);

        assert!(matches!(close(&w, "2026-10-21", counted(0, 0, 0, 0, 0, None)), Err(AppError::Validation(_))), "falta o conferido do legado");
        let mut all = counted(0, 0, 0, 0, 0, None);
        all.push(CountedInput { method: "unspecified".into(), counted_cents: 12_000, justification: None });
        assert_eq!(close(&w, "2026-10-21", all).unwrap().lines.len(), 6);
    }
}
