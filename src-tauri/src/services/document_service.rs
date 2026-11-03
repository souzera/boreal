//! Documentos internos (spec §17): confirmação de reserva, comprovante de pagamento, ficha de hospedagem e
//! demonstrativo final. Cada emissão grava um instantâneo imutável; o PDF sai sempre do instantâneo.

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::domain::payment::Method;
use crate::domain::pdf::{PdfColumn, PdfDoc};
use crate::domain::report::{br_date, money_text};
use crate::error::AppError;
use crate::models::document::{DocumentKind, DocumentView};
use crate::models::report::ExportResult;
use crate::repositories::document_repo::{self, DocumentRow};
use crate::repositories::{guest_repo, payment_repo, pousada_repo, reservation_repo, stay_repo};
use crate::services::report_service::write_export;

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

fn view(row: &DocumentRow, current: bool) -> DocumentView {
    let kind = DocumentKind::parse(&row.kind);
    DocumentView {
        id: row.id,
        kind: row.kind.clone(),
        kind_label: kind.map_or_else(|| row.kind.clone(), |k| k.label().to_string()),
        source_id: row.source_id,
        version: row.version,
        issued_at: row.issued_at.clone(),
        issued_by_name: row.issued_by_name.clone(),
        current,
    }
}

#[derive(Clone)]
pub struct DocumentService {
    ctx: Ctx,
}

impl DocumentService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn local_text(&self, utc: &str) -> String {
        let offset = i64::from(self.ctx.utc.local_offset().local_minus_utc());
        DateTime::parse_from_rfc3339(utc)
            .map(|d| (d.with_timezone(&Utc) + Duration::seconds(offset)).format("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_else(|_| utc.to_string())
    }

    /// Emite o documento. Se nada mudou desde a última versão, devolve essa mesma versão (nada é gravado);
    /// se a origem mudou, grava a versão seguinte sem tocar nas anteriores. Auditoria na mesma transação.
    pub fn issue(&self, token: &str, kind: &str, source_id: i64) -> Result<DocumentView, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let kind = DocumentKind::parse(kind).ok_or_else(|| invalid("Tipo de documento desconhecido."))?;
        let at = self.ctx.utc.now_utc();
        let mut conn = self.ctx.conn()?;
        let snapshot = serde_json::to_string(&self.build(&conn, kind, source_id)?).map_err(|_| invalid("Não foi possível montar o documento."))?;

        let tx = conn.transaction()?;
        let last = document_repo::latest(&tx, kind.as_str(), source_id)?;
        if let Some(last) = last.as_ref().filter(|l| l.snapshot == snapshot) {
            return Ok(view(last, true));
        }
        let version = last.map_or(1, |l| l.version + 1);
        let id = document_repo::insert(&tx, kind.as_str(), source_id, version, &snapshot, at, actor.user_id)?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "documents",
                operation: "issue_document",
                entity: "document",
                entity_id: Some(id),
                before: None,
                after: Some(json!({ "kind": kind.as_str(), "source_id": source_id, "version": version })),
                justification: None,
            },
        )?;
        let row = document_repo::find(&tx, id)?.ok_or_else(|| invalid("Documento não encontrado."))?;
        tx.commit()?;
        Ok(view(&row, true))
    }

    /// Versões emitidas, da mais nova para a mais antiga.
    pub fn list(&self, token: &str, kind: &str, source_id: i64) -> Result<Vec<DocumentView>, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let kind = DocumentKind::parse(kind).ok_or_else(|| invalid("Tipo de documento desconhecido."))?;
        let conn = self.ctx.conn()?;
        let rows = document_repo::list(&conn, kind.as_str(), source_id)?;
        Ok(rows.iter().enumerate().map(|(i, r)| view(r, i == 0)).collect())
    }

    /// PDF de uma versão emitida: sai só do instantâneo gravado, nunca dos dados atuais.
    pub fn pdf(&self, token: &str, document_id: i64) -> Result<Vec<u8>, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        let row = document_repo::find(&conn, document_id)?.ok_or_else(|| invalid("Documento não encontrado."))?;
        let kind = DocumentKind::parse(&row.kind).ok_or_else(|| invalid("Tipo de documento desconhecido."))?;
        let snapshot: Value = serde_json::from_str(&row.snapshot).map_err(|_| invalid("O documento gravado está ilegível."))?;
        Ok(render_pdf(kind, row.version, &self.local_text(&row.issued_at), &row.issued_by_name, &snapshot))
    }

    /// Grava o PDF de uma versão no caminho escolhido. Falhar aqui não emite nem repete nada: o documento já existe.
    pub fn save_pdf(&self, token: &str, document_id: i64, path: &str) -> Result<ExportResult, AppError> {
        let bytes = self.pdf(token, document_id)?;
        write_export(path, "pdf", &bytes)
    }

    fn pousada(conn: &rusqlite::Connection) -> Result<Value, AppError> {
        let p = pousada_repo::get(conn)?;
        Ok(json!({ "name": p.name, "address": p.address, "phone": p.phone }))
    }

    fn build(&self, conn: &rusqlite::Connection, kind: DocumentKind, source_id: i64) -> Result<Value, AppError> {
        match kind {
            DocumentKind::ReservationConfirmation => self.confirmation(conn, source_id),
            DocumentKind::PaymentReceipt => self.receipt(conn, source_id),
            DocumentKind::StayRecord => self.stay_record(conn, source_id),
            DocumentKind::FinalStatement => self.final_statement(conn, source_id),
        }
    }

    fn confirmation(&self, conn: &rusqlite::Connection, id: i64) -> Result<Value, AppError> {
        let r = reservation_repo::find(conn, id, self.ctx.today())?.ok_or_else(|| invalid("Reserva não encontrada."))?;
        if !matches!(r.status.as_str(), "pending" | "confirmed" | "checked_in" | "completed") {
            return Err(invalid("A confirmação só pode ser emitida para uma reserva ativa."));
        }
        let guest = guest_repo::find(conn, r.guest_id)?;
        let nights: Vec<Value> = reservation_repo::nights(conn, id)?.into_iter().map(|n| json!({ "date": n.date, "amount_cents": n.amount_cents })).collect();
        Ok(json!({
            "pousada": Self::pousada(conn)?,
            "code": r.code,
            "guest_name": r.guest_name,
            "guest_phone": guest.map(|g| g.phone).unwrap_or_default(),
            "room_code": r.room_code,
            "category": document_repo::category_name(conn, r.room_id)?,
            "check_in_date": r.check_in_date,
            "check_out_date": r.check_out_date,
            "check_in_time": r.expected_check_in_time,
            "check_out_time": r.expected_check_out_time,
            "occupants": r.occupants,
            "nights": nights,
            "list_total_cents": r.list_total_cents,
            "discount_cents": r.discount_cents,
            "agreed_total_cents": r.agreed_total_cents,
            "deposit_expected_cents": r.deposit_expected_cents,
            "deposit_paid_cents": r.deposit_paid_cents,
            "deposit_due_date": r.deposit_due_date,
        }))
    }

    fn receipt(&self, conn: &rusqlite::Connection, payment_id: i64) -> Result<Value, AppError> {
        let p = document_repo::payment(conn, payment_id)?.ok_or_else(|| invalid("Recebimento não encontrado."))?;
        let components: Vec<Value> = payment_repo::components(conn, payment_id)?
            .into_iter()
            .map(|c| json!({ "method": c.method, "amount_cents": c.amount_cents }))
            .collect();
        Ok(json!({
            "pousada": Self::pousada(conn)?,
            "code": p.code,
            "kind": p.kind,
            "received_at": self.local_text(&p.received_at),
            "received_by": p.received_by_name,
            "guest_name": p.guest_name,
            "room_code": p.room_code,
            "reservation_code": p.reservation_code,
            "stay_code": p.stay_code,
            "components": components,
            "total_cents": p.amount_cents,
        }))
    }

    fn stay_record(&self, conn: &rusqlite::Connection, stay_id: i64) -> Result<Value, AppError> {
        let s = stay_repo::find(conn, stay_id)?.ok_or_else(|| invalid("Hospedagem não encontrada."))?;
        let room_id: i64 = conn.query_row("SELECT room_id FROM reservations WHERE id = ?1", [s.reservation_id], |r| r.get(0))?;
        let occupants: Vec<Value> = stay_repo::occupants(conn, stay_id)?
            .into_iter()
            .map(|o| json!({ "name": o.full_name, "document": o.document, "holder": o.is_holder }))
            .collect();
        Ok(json!({
            "pousada": Self::pousada(conn)?,
            "code": s.code,
            "status": s.status,
            "guest_name": s.guest_name,
            "room_code": s.room_code,
            "category": document_repo::category_name(conn, room_id)?,
            "check_in_date": s.check_in_date,
            "check_out_date": s.check_out_date,
            "check_in_time": s.expected_check_in_time,
            "check_out_time": s.expected_check_out_time,
            "checked_in_at": self.local_text(&s.checked_in_at),
            "occupants": occupants,
        }))
    }

    fn final_statement(&self, conn: &rusqlite::Connection, stay_id: i64) -> Result<Value, AppError> {
        let s = stay_repo::find(conn, stay_id)?.ok_or_else(|| invalid("Hospedagem não encontrada."))?;
        if s.status != "checked_out" {
            return Err(invalid("O demonstrativo final só pode ser emitido depois do check-out."));
        }
        let account = stay_repo::account(conn, stay_id)?;
        let charges: Vec<Value> = account
            .charges
            .iter()
            .map(|c| json!({ "kind": c.kind, "description": c.description, "amount_cents": c.amount_cents, "date": c.night_date.clone().unwrap_or_else(|| self.local_text(&c.created_at)) }))
            .collect();
        let payments: Vec<Value> = account
            .payments
            .iter()
            .map(|p| json!({ "code": p.code, "received_at": self.local_text(&p.received_at), "amount_cents": p.amount_cents, "components": p.components.iter().map(|c| json!({ "method": c.method, "amount_cents": c.amount_cents })).collect::<Vec<_>>() }))
            .collect();
        let refunds: Vec<Value> = account
            .refunds
            .iter()
            .map(|r| json!({ "code": r.code, "refunded_at": self.local_text(&r.refunded_at), "amount_cents": r.amount_cents }))
            .collect();
        Ok(json!({
            "pousada": Self::pousada(conn)?,
            "code": s.code,
            "guest_name": s.guest_name,
            "room_code": s.room_code,
            "check_in_date": s.check_in_date,
            "check_out_date": s.check_out_date,
            "checked_out_at": s.checked_out_at.as_deref().map(|t| self.local_text(t)).unwrap_or_default(),
            "charges": charges,
            "payments": payments,
            "refunds": refunds,
            "charges_total_cents": account.charges_total_cents,
            "received_cents": account.received_cents,
            "refunded_cents": account.refunded_cents,
            "balance_cents": account.balance_cents,
        }))
    }
}

// ---- PDF a partir do instantâneo ----

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn cents(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn method_text(method: &str) -> String {
    Method::parse(method).map_or_else(|| method.to_string(), |m| m.label().to_string())
}

fn when(date: &str, time: &str) -> String {
    format!("{} às {time}", br_date(date))
}

fn col(label: &str, weight: u32, right: bool) -> PdfColumn {
    PdfColumn { label: label.into(), weight, right }
}

fn balance_words(balance: i64) -> String {
    match balance {
        b if b > 0 => format!("Saldo a receber {}", money_text(b)),
        b if b < 0 => format!("Crédito do hóspede {}", money_text(-b)),
        _ => "Quitado".to_string(),
    }
}

fn render_pdf(kind: DocumentKind, version: i64, issued_at: &str, issued_by: &str, s: &Value) -> Vec<u8> {
    let pousada = s.get("pousada").cloned().unwrap_or(Value::Null);
    let name = text(&pousada, "name");
    let mut doc = PdfDoc::new(false);
    doc.heading(&name);
    for line in [text(&pousada, "address"), text(&pousada, "phone")] {
        if !line.is_empty() {
            doc.text(&line);
        }
    }
    doc.spacer(8.0);
    doc.subheading(kind.label());
    doc.text(&format!("Versão {version} · emitida em {issued_at} por {issued_by}"));
    doc.spacer(8.0);

    match kind {
        DocumentKind::ReservationConfirmation => {
            doc.field("Reserva", &text(s, "code"));
            doc.field("Hóspede", &text(s, "guest_name"));
            if !text(s, "guest_phone").is_empty() {
                doc.field("Telefone", &text(s, "guest_phone"));
            }
            doc.field("Quarto", &format!("{} · {}", text(s, "room_code"), text(s, "category")));
            doc.field("Entrada", &when(&text(s, "check_in_date"), &text(s, "check_in_time")));
            doc.field("Saída", &when(&text(s, "check_out_date"), &text(s, "check_out_time")));
            doc.field("Ocupantes", &cents(s, "occupants").to_string());
            doc.spacer(8.0);
            doc.subheading("Diárias");
            let nights: Vec<Vec<String>> = s
                .get("nights")
                .and_then(Value::as_array)
                .map(|a| a.iter().map(|n| vec![br_date(&text(n, "date")), money_text(cents(n, "amount_cents"))]).collect())
                .unwrap_or_default();
            doc.table(&[col("Data", 30, false), col("Valor", 20, true)], &nights, Some(&["Soma das diárias".to_string(), money_text(cents(s, "list_total_cents"))]));
            doc.spacer(8.0);
            if cents(s, "discount_cents") > 0 {
                doc.field("Desconto", &money_text(cents(s, "discount_cents")));
            }
            doc.field("Valor combinado", &money_text(cents(s, "agreed_total_cents")));
            doc.field("Sinal esperado", &money_text(cents(s, "deposit_expected_cents")));
            doc.field("Sinal recebido", &money_text(cents(s, "deposit_paid_cents")));
            doc.field("Prazo do sinal", &br_date(&text(s, "deposit_due_date")));
            doc.spacer(8.0);
            doc.text("Em caso de cancelamento, vale a política da pousada em vigor na data da reserva.");
        }
        DocumentKind::PaymentReceipt => {
            doc.field("Número", &text(s, "code"));
            doc.field("Data", &text(s, "received_at"));
            doc.field("Recebido por", &text(s, "received_by"));
            doc.field("Hóspede", &text(s, "guest_name"));
            doc.field("Quarto", &text(s, "room_code"));
            let reference = match s.get("stay_code").and_then(Value::as_str) {
                Some(stay) => format!("Hospedagem {stay}"),
                None => format!("Sinal da reserva {}", text(s, "reservation_code")),
            };
            doc.field("Referência", &reference);
            doc.spacer(8.0);
            let rows: Vec<Vec<String>> = s
                .get("components")
                .and_then(Value::as_array)
                .map(|a| a.iter().map(|c| vec![method_text(&text(c, "method")), money_text(cents(c, "amount_cents"))]).collect())
                .unwrap_or_default();
            doc.table(&[col("Forma de pagamento", 30, false), col("Valor", 20, true)], &rows, Some(&["Total recebido".to_string(), money_text(cents(s, "total_cents"))]));
        }
        DocumentKind::StayRecord => {
            doc.field("Hospedagem", &text(s, "code"));
            doc.field("Hóspede", &text(s, "guest_name"));
            doc.field("Quarto", &format!("{} · {}", text(s, "room_code"), text(s, "category")));
            doc.field("Entrada", &when(&text(s, "check_in_date"), &text(s, "check_in_time")));
            doc.field("Saída prevista", &when(&text(s, "check_out_date"), &text(s, "check_out_time")));
            doc.field("Check-in feito em", &text(s, "checked_in_at"));
            doc.spacer(8.0);
            doc.subheading("Ocupantes");
            let rows: Vec<Vec<String>> = s
                .get("occupants")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .map(|o| {
                            let doc_text = text(o, "document");
                            vec![text(o, "name"), if doc_text.is_empty() { "Não informado".into() } else { doc_text }, if o.get("holder").and_then(Value::as_bool).unwrap_or(false) { "Titular".into() } else { String::new() }]
                        })
                        .collect()
                })
                .unwrap_or_default();
            doc.table(&[col("Nome", 30, false), col("Documento", 20, false), col("", 10, false)], &rows, None);
            doc.signature_line("Assinatura do hóspede");
        }
        DocumentKind::FinalStatement => {
            doc.field("Hospedagem", &text(s, "code"));
            doc.field("Hóspede", &text(s, "guest_name"));
            doc.field("Quarto", &text(s, "room_code"));
            doc.field("Período", &format!("{} a {}", br_date(&text(s, "check_in_date")), br_date(&text(s, "check_out_date"))));
            doc.field("Check-out feito em", &text(s, "checked_out_at"));
            doc.spacer(8.0);
            doc.subheading("Lançamentos");
            let charges: Vec<Vec<String>> = s
                .get("charges")
                .and_then(Value::as_array)
                .map(|a| a.iter().map(|c| vec![br_date(&text(c, "date")), text(c, "description"), money_text(cents(c, "amount_cents"))]).collect())
                .unwrap_or_default();
            doc.table(&[col("Data", 18, false), col("Descrição", 40, false), col("Valor", 18, true)], &charges, Some(&["Total lançado".to_string(), String::new(), money_text(cents(s, "charges_total_cents"))]));
            doc.spacer(8.0);
            doc.subheading("Recebimentos");
            let payments: Vec<Vec<String>> = s
                .get("payments")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .map(|p| {
                            let methods: Vec<String> = p
                                .get("components")
                                .and_then(Value::as_array)
                                .map(|cs| cs.iter().map(|c| format!("{} {}", method_text(&text(c, "method")), money_text(cents(c, "amount_cents")))).collect())
                                .unwrap_or_default();
                            vec![text(p, "code"), text(p, "received_at"), methods.join(" + "), money_text(cents(p, "amount_cents"))]
                        })
                        .collect()
                })
                .unwrap_or_default();
            doc.table(&[col("Número", 16, false), col("Data", 18, false), col("Formas", 34, false), col("Valor", 16, true)], &payments, None);
            if s.get("refunds").and_then(Value::as_array).is_some_and(|r| !r.is_empty()) {
                doc.spacer(8.0);
                doc.subheading("Devoluções");
                let refunds: Vec<Vec<String>> = s
                    .get("refunds")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().map(|r| vec![text(r, "code"), text(r, "refunded_at"), money_text(cents(r, "amount_cents"))]).collect())
                    .unwrap_or_default();
                doc.table(&[col("Número", 20, false), col("Data", 24, false), col("Valor", 16, true)], &refunds, None);
            }
            doc.spacer(8.0);
            doc.field("Recebido", &money_text(cents(s, "received_cents")));
            doc.field("Devolvido", &money_text(cents(s, "refunded_cents")));
            doc.bold_text(&format!("Situação final: {}", balance_words(cents(s, "balance_cents"))));
        }
    }
    doc.finish(&format!("{name} · Boreal PMS"))
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::domain::pdf::extract_text;
    use crate::models::payment::{ComponentInput, PaymentInput, PaymentTarget};
    use crate::models::stay::{CheckInInput, OccupantInput};
    use crate::services::test_support::{admin_token, book, checked_out_stay, deposit, harness, recepcao_token, seed, Harness, Seed};

    // Fuso -03:00. "Hoje" é 21/10/2026, 09:00 local. Diária R$ 200,00; quarto 101; hóspede Ana Souza.

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

    fn count(w: &World, sql: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(sql, [], |r| r.get(0)).unwrap()
    }

    fn at(w: &World, rfc3339: &str) {
        w.h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    fn pdf_text(w: &World, id: i64) -> Vec<String> {
        extract_text(&w.h.documents.pdf(&w.admin, id).unwrap())
    }

    fn in_house(w: &World) -> (i64, i64) {
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        at(w, "2026-10-21T17:00:00Z");
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: "RG 12.345.678-9".into() };
        let stay = w
            .h
            .stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
            .unwrap();
        (reservation, stay)
    }

    #[test]
    fn issue_creates_version_1_and_audits_in_the_same_transaction() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");

        let doc = w.h.documents.issue(&w.admin, "reservation_confirmation", reservation).unwrap();

        assert_eq!((doc.version, doc.current, doc.kind_label.as_str()), (1, true, "Confirmação de reserva"));
        assert_eq!(count(&w, "SELECT count(*) FROM documents"), 1);
        assert_eq!(count(&w, "SELECT count(*) FROM audit_log WHERE module = 'documents' AND operation = 'issue_document'"), 1);
        let after: String = w.h.db.lock().unwrap().query_row("SELECT after_json FROM audit_log WHERE operation = 'issue_document'", [], |r| r.get(0)).unwrap();
        assert!(after.contains("reservation_confirmation") && !after.contains("Ana"), "a auditoria não guarda o conteúdo: {after}");
    }

    #[test]
    fn issue_again_without_changes_returns_the_same_version_and_writes_nothing() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let first = w.h.documents.issue(&w.admin, "reservation_confirmation", reservation).unwrap();

        let again = w.h.documents.issue(&w.admin, "reservation_confirmation", reservation).unwrap();

        assert_eq!((again.id, again.version), (first.id, 1));
        assert_eq!(count(&w, "SELECT count(*) FROM documents"), 1);
        assert_eq!(count(&w, "SELECT count(*) FROM audit_log WHERE operation = 'issue_document'"), 1);
    }

    #[test]
    fn issue_after_the_reservation_changed_creates_version_2_and_keeps_version_1_intact() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let v1 = w.h.documents.issue(&w.admin, "reservation_confirmation", reservation).unwrap();
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        deposit(&w.h, &w.admin, reservation, 5_000, version).unwrap();

        let v2 = w.h.documents.issue(&w.admin, "reservation_confirmation", reservation).unwrap();

        assert_eq!((v2.version, v2.current), (2, true));
        assert_ne!(v1.id, v2.id);
        let old = pdf_text(&w, v1.id);
        let new = pdf_text(&w, v2.id);
        let paid = |t: &[String]| t.iter().position(|x| x == "Sinal recebido:").map(|i| t[i + 1].clone());
        assert_eq!(paid(&old).as_deref(), Some("R$ 0,00"), "a versão 1 continua mostrando o que foi emitido");
        assert_eq!(paid(&new).as_deref(), Some("R$ 50,00"));
        let listed = w.h.documents.list(&w.admin, "reservation_confirmation", reservation).unwrap();
        assert_eq!(listed.iter().map(|d| (d.version, d.current)).collect::<Vec<_>>(), [(2, true), (1, false)]);
    }

    #[test]
    fn pdf_uses_the_snapshot_even_if_the_pousada_name_changed_later() {
        let w = world();
        w.h.db.lock().unwrap().execute("UPDATE pousada SET name = 'Pousada do Vale'", []).unwrap();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let doc = w.h.documents.issue(&w.admin, "reservation_confirmation", reservation).unwrap();
        w.h.db.lock().unwrap().execute("UPDATE pousada SET name = 'Outro Nome'", []).unwrap();

        let texts = pdf_text(&w, doc.id);

        assert!(texts.contains(&"Pousada do Vale".to_string()), "{texts:?}");
        assert!(!texts.contains(&"Outro Nome".to_string()));
    }

    #[test]
    fn documents_cannot_be_updated_or_deleted() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        w.h.documents.issue(&w.admin, "reservation_confirmation", reservation).unwrap();
        let conn = w.h.db.lock().unwrap();

        assert!(conn.execute("UPDATE documents SET snapshot = '{}'", []).is_err());
        assert!(conn.execute("DELETE FROM documents", []).is_err());
    }

    #[test]
    fn conditions_and_unknown_sources_are_refused_with_plain_messages() {
        let w = world();
        let (_, stay) = in_house(&w);
        let cancelled = {
            let id = book(&w.h, &w.admin, &w.seed, "2026-10-25", "2026-10-27");
            let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;
            w.h.reservations.cancel(&w.admin, id, version, None).unwrap();
            id
        };

        let statement = w.h.documents.issue(&w.admin, "final_statement", stay).unwrap_err();
        let confirmation = w.h.documents.issue(&w.admin, "reservation_confirmation", cancelled).unwrap_err();
        let missing = w.h.documents.issue(&w.admin, "stay_record", 9_999).unwrap_err();
        let unknown_kind = w.h.documents.issue(&w.admin, "contrato", 1).unwrap_err();
        let receipt = w.h.documents.issue(&w.admin, "payment_receipt", 9_999).unwrap_err();

        assert!(matches!(&statement, AppError::Validation(m) if m.contains("depois do check-out")), "{statement:?}");
        assert!(matches!(&confirmation, AppError::Validation(m) if m.contains("reserva ativa")), "{confirmation:?}");
        for e in [&missing, &unknown_kind, &receipt] {
            assert!(matches!(e, AppError::Validation(_)), "{e:?}");
        }
        assert_eq!(count(&w, "SELECT count(*) FROM documents"), 0);
    }

    #[test]
    fn payment_receipt_lists_each_method_and_the_total() {
        let w = world();
        let (_, stay) = in_house(&w);
        let receipt = w
            .h
            .payments
            .record(
                &w.admin,
                PaymentTarget::Stay { stay_id: stay },
                PaymentInput {
                    components: vec![ComponentInput { method: "debit".into(), amount_cents: 6_000 }, ComponentInput { method: "pix".into(), amount_cents: 4_000 }],
                    note: String::new(),
                    duplicate_release: None,
                },
            )
            .unwrap();

        let doc = w.h.documents.issue(&w.admin, "payment_receipt", receipt.id).unwrap();
        let texts = pdf_text(&w, doc.id);

        assert!(texts.contains(&receipt.code), "{texts:?}");
        assert!(texts.contains(&"Cartão de débito".to_string()) && texts.contains(&"R$ 60,00".to_string()));
        assert!(texts.contains(&"Pix".to_string()) && texts.contains(&"R$ 40,00".to_string()));
        assert!(texts.contains(&"Total recebido".to_string()) && texts.contains(&"R$ 100,00".to_string()));
        assert!(texts.iter().any(|t| t.starts_with("Hospedagem ")), "referência à hospedagem");
    }

    #[test]
    fn stay_record_lists_the_occupants_with_their_documents() {
        let w = world();
        let (_, stay) = in_house(&w);

        let doc = w.h.documents.issue(&w.admin, "stay_record", stay).unwrap();
        let texts = pdf_text(&w, doc.id);

        assert!(texts.contains(&"Ficha de hospedagem".to_string()));
        assert!(texts.contains(&"Ana Souza".to_string()) && texts.contains(&"RG 12.345.678-9".to_string()) && texts.contains(&"Titular".to_string()));
        assert!(texts.contains(&"Assinatura do hóspede".to_string()));
        assert!(texts.iter().any(|t| t == "21/10/2026 às 14:00"), "{texts:?}");
    }

    #[test]
    fn statement_lists_charges_payments_refunds_and_final_balance() {
        let w = world();
        let (_, stay) = checked_out_stay(&w.h, &w.admin, &w.seed);
        let account = w.h.stays.get(&w.admin, stay).unwrap().account;

        let doc = w.h.documents.issue(&w.admin, "final_statement", stay).unwrap();
        let texts = pdf_text(&w, doc.id);

        assert!(texts.contains(&money_text(account.charges_total_cents)), "total lançado");
        assert!(texts.contains(&money_text(account.received_cents)));
        assert!(texts.contains(&"Situação final: Quitado".to_string()), "{texts:?}");
        for charge in &account.charges {
            assert!(texts.contains(&charge.description), "falta {}", charge.description);
        }
        for payment in &account.payments {
            assert!(texts.contains(&payment.code));
        }
    }

    #[test]
    fn snapshot_has_no_cpf_email_or_internal_notes() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let doc = w.h.documents.issue(&w.admin, "reservation_confirmation", reservation).unwrap();

        let snapshot: String = w.h.db.lock().unwrap().query_row("SELECT snapshot FROM documents WHERE id = ?1", [doc.id], |r| r.get(0)).unwrap();

        for forbidden in ["cpf", "email", "notes", "password"] {
            assert!(!snapshot.contains(forbidden), "{forbidden} em {snapshot}");
        }
    }

    #[test]
    fn requires_a_session_and_any_logged_in_user_can_issue() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let reception = recepcao_token(&w.h, &w.admin);

        assert!(w.h.documents.issue("invalido", "reservation_confirmation", reservation).is_err());
        assert!(w.h.documents.list("invalido", "reservation_confirmation", reservation).is_err());
        assert!(w.h.documents.issue(&reception, "reservation_confirmation", reservation).is_ok());
    }
}
