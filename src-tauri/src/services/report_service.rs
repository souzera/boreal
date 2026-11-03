//! Relatórios (spec §12.3): um motor que devolve a mesma `ReportTable` para a tela, o CSV e o PDF.
//! Somente leitura. Relatórios financeiros exigem `ViewFinancialReports`.

use chrono::Duration;

use super::Ctx;
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::dates;
use crate::domain::pdf::{PdfColumn, PdfDoc};
use crate::domain::report::{br_date, cell_text, Cell, Column, ColumnKind, Filter, ReportKind, ReportTable};
use crate::error::AppError;
use crate::models::report::ReportQuery;
use crate::domain::payment::Method;
use crate::repositories::report_finance_repo as finance;
use crate::repositories::report_repo::{self, Rows, Scope, RESERVATION_STATUS_LABEL};

/// Linhas por relatório na tela, no CSV e no PDF.
pub const ROW_LIMIT: usize = 5_000;
const MAX_DAYS: i64 = 366;
const DEFAULT_DAYS: i64 = 30;

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn clean(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

fn total_label() -> Cell {
    Cell::text("Total")
}

fn sum_money(rows: &Rows, column: usize) -> i64 {
    rows.iter().map(|r| if let Some(Cell::Money(c)) = r.get(column) { *c } else { 0 }).sum()
}

fn sum_int(rows: &Rows, column: usize) -> i64 {
    rows.iter().map(|r| if let Some(Cell::Int(n)) = r.get(column) { *n } else { 0 }).sum()
}

#[derive(Clone)]
pub struct ReportService {
    ctx: Ctx,
}

impl ReportService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    pub fn run(&self, token: &str, query: ReportQuery) -> Result<ReportTable, AppError> {
        self.run_with_limit(token, query, ROW_LIMIT)
    }

    /// PDF do relatório: as mesmas linhas, colunas e totais da tela (`run`), com o nome da pousada e a data de emissão.
    pub fn pdf(&self, token: &str, query: ReportQuery) -> Result<Vec<u8>, AppError> {
        let table = self.run(token, query)?;
        let name = {
            let conn = self.ctx.conn()?;
            crate::repositories::pousada_repo::get(&conn)?.name
        };
        let issued = self.ctx.local_now().format("%d/%m/%Y %H:%M").to_string();
        Ok(table_pdf(&table, &name, &issued))
    }

    pub(crate) fn run_with_limit(&self, token: &str, query: ReportQuery, limit: usize) -> Result<ReportTable, AppError> {
        let info = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let kind = ReportKind::parse(&query.kind).ok_or_else(|| invalid("Relatório desconhecido."))?;
        // A permissão vem antes de qualquer validação: quem não pode não descobre nada sobre o relatório.
        if kind.financial() {
            require(&info, Permission::ViewFinancialReports)?;
        }

        let status = clean(&query.status);
        let text = clean(&query.text);
        let method = clean(&query.method);
        for (filter, present) in [
            (Filter::Status, status.is_some()),
            (Filter::Room, query.room_id.is_some()),
            (Filter::Method, method.is_some()),
            (Filter::Category, query.category_id.is_some()),
            (Filter::Text, text.is_some()),
        ] {
            if present && !kind.accepts(filter) {
                return Err(invalid(format!("Este relatório não usa o filtro de {}.", filter.label())));
            }
        }
        if !kind.uses_period() && (clean(&query.from).is_some() || clean(&query.to).is_some()) {
            return Err(invalid("Este relatório mostra a situação de agora e não usa o filtro de período."));
        }

        let today = self.ctx.today();
        let (from_date, to_date) = if kind.uses_period() {
            let to = match clean(&query.to) {
                Some(t) => dates::parse(t).ok_or_else(|| invalid("Data final inválida."))?,
                None => today,
            };
            let from = match clean(&query.from) {
                Some(f) => dates::parse(f).ok_or_else(|| invalid("Data inicial inválida."))?,
                None => to - Duration::days(DEFAULT_DAYS - 1),
            };
            if to < from {
                return Err(invalid("A data final não pode ser antes da inicial."));
            }
            if (to - from).num_days() + 1 > MAX_DAYS {
                return Err(invalid("O período pode ter no máximo 366 dias."));
            }
            (from, to)
        } else {
            (today, today)
        };

        let offset = self.ctx.utc.local_offset();
        let (start, end) = dates::utc_bounds(from_date, to_date, offset);
        let from = dates::format(from_date);
        let to = dates::format(to_date);
        let today_text = dates::format(today);
        let conn = self.ctx.conn()?;

        let mut filters = Vec::new();
        if kind.uses_period() {
            filters.push(format!("Período: {} a {}", br_date(&from), br_date(&to)));
        } else {
            filters.push(format!("Situação em {}", br_date(&dates::format(today))));
        }
        if let Some(s) = status {
            let label = status_label(kind, s)?;
            filters.push(format!("Situação: {label}"));
        }
        if let Some(room) = query.room_id {
            let code: Option<String> = conn
                .query_row("SELECT code FROM rooms WHERE id = ?1", [room], |r| r.get(0))
                .map(Some)
                .or_else(|e| if matches!(e, rusqlite::Error::QueryReturnedNoRows) { Ok(None) } else { Err(e) })?;
            filters.push(format!("Quarto: {}", code.ok_or_else(|| invalid("Quarto não encontrado."))?));
        }
        if let Some(m) = method {
            let parsed = Method::parse(m).ok_or_else(|| invalid("Forma de pagamento inválida."))?;
            filters.push(format!("Forma de pagamento: {}", parsed.label()));
        }
        if let Some(category) = query.category_id {
            let name: Option<String> = conn
                .query_row("SELECT name FROM expense_categories WHERE id = ?1", [category], |r| r.get(0))
                .map(Some)
                .or_else(|e| if matches!(e, rusqlite::Error::QueryReturnedNoRows) { Ok(None) } else { Err(e) })?;
            filters.push(format!("Categoria: {}", name.ok_or_else(|| invalid("Categoria não encontrada."))?));
        }
        if let Some(t) = text {
            filters.push(format!("Texto: {t}"));
        }

        let scope = Scope {
            from: &from,
            to: &to,
            start: &start,
            end: &end,
            offset_secs: offset.local_minus_utc(),
            status,
            room_id: query.room_id,
            text,
            method,
            category_id: query.category_id,
            today: &today_text,
            limit,
        };
        let (columns, mut rows, totals) = match kind {
            ReportKind::Occupancy => occupancy(&conn, &scope)?,
            ReportKind::Reservations => reservations(&conn, &scope)?,
            ReportKind::Stays => stays(&conn, &scope)?,
            ReportKind::Cancellations => cancellations(&conn, &scope)?,
            ReportKind::Receipts => receipts(&conn, &scope)?,
            ReportKind::Balances => balances(&conn, &scope)?,
            ReportKind::GuestExpenses => guest_expenses(&conn, &scope)?,
            ReportKind::PropertyExpenses => {
                filters.push(format!("Pago no período (data efetiva): {}", crate::domain::report::money_text(finance::paid_in_period(&conn, &scope)?)));
                property_expenses(&conn, &scope)?
            }
            ReportKind::Payables => payables(&conn, &scope)?,
            ReportKind::Closings => closings(&conn, &scope)?,
        };
        let truncated = rows.len() > limit;
        rows.truncate(limit);
        // Totais de linhas cortadas enganariam; quando trunca, o aviso substitui o rodapé.
        let totals = if truncated { None } else { totals(&rows) };
        Ok(ReportTable { kind: kind.as_str().into(), title: kind.label().into(), filters, columns, rows, totals, truncated })
    }
}

/// Pesos das colunas no PDF: texto mais largo que número; números à direita.
fn pdf_column(column: &Column) -> PdfColumn {
    let (weight, right) = match column.kind {
        ColumnKind::Text => (30, false),
        ColumnKind::Date => (19, false),
        ColumnKind::Time => (11, false),
        ColumnKind::Int => (12, true),
        ColumnKind::Money => (20, true),
        ColumnKind::Percent => (15, true),
    };
    PdfColumn { label: column.label.clone(), weight, right }
}

/// Desenha uma `ReportTable` em PDF. Mais de 7 colunas usam página em paisagem.
pub fn table_pdf(table: &ReportTable, pousada: &str, issued_at: &str) -> Vec<u8> {
    let mut doc = PdfDoc::new(table.columns.len() > 7);
    doc.heading(pousada);
    doc.subheading(&table.title);
    for filter in &table.filters {
        doc.text(filter);
    }
    doc.text(&format!("Emitido em {issued_at}"));
    doc.spacer(6.0);
    let columns: Vec<PdfColumn> = table.columns.iter().map(pdf_column).collect();
    let rows: Vec<Vec<String>> = table.rows.iter().map(|r| r.iter().map(cell_text).collect()).collect();
    let totals: Option<Vec<String>> = table.totals.as_ref().map(|t| t.iter().map(cell_text).collect());
    doc.table(&columns, &rows, totals.as_deref());
    if table.truncated {
        doc.spacer(6.0);
        doc.text(crate::domain::csv::TRUNCATED_NOTE);
    }
    doc.finish(&format!("{pousada} · Boreal PMS"))
}

type Built = (Vec<Column>, Rows, Box<dyn Fn(&Rows) -> Option<Vec<Cell>>>);

fn status_label(kind: ReportKind, status: &str) -> Result<String, AppError> {
    let allowed: &[&str] = match kind {
        ReportKind::Reservations => &["pending", "confirmed", "checked_in", "completed", "cancelled", "no_show"],
        ReportKind::Stays => &["in_house", "checked_out"],
        ReportKind::Cancellations => &["cancelled", "no_show"],
        ReportKind::PropertyExpenses | ReportKind::Payables => &["pending", "partial", "paid", "overdue"],
        _ => &[],
    };
    if !allowed.contains(&status) {
        return Err(invalid("Situação inválida para este relatório."));
    }
    Ok(match status {
        "in_house" => "Hospedado".to_string(),
        "checked_out" => "Check-out feito".to_string(),
        "pending" | "partial" | "paid" | "overdue" if kind.financial() => finance::payable_status_label(status).to_string(),
        other => RESERVATION_STATUS_LABEL.iter().find(|(k, _)| *k == other).map_or(other, |(_, v)| v).to_string(),
    })
}

fn occupancy(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = report_repo::occupancy(conn, scope)?;
    Ok((
        vec![
            Column::new("date", "Data", ColumnKind::Date),
            Column::new("rooms", "Quartos", ColumnKind::Int),
            Column::new("occupied", "Ocupados", ColumnKind::Int),
            Column::new("free", "Livres", ColumnKind::Int),
            Column::new("rate", "Ocupação", ColumnKind::Percent),
        ],
        rows,
        Box::new(|rows| {
            let rooms = sum_int(rows, 1);
            let occupied = sum_int(rows, 2);
            let rate = if rooms == 0 { 0 } else { (occupied * 20_000 + rooms) / (2 * rooms) };
            Some(vec![Cell::text("Média do período"), Cell::Empty, Cell::Int(occupied), Cell::Empty, Cell::Percent(rate)])
        }),
    ))
}

fn reservations(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = report_repo::reservations(conn, scope)?;
    Ok((
        vec![
            Column::new("code", "Código", ColumnKind::Text),
            Column::new("guest", "Hóspede", ColumnKind::Text),
            Column::new("room", "Quarto", ColumnKind::Text),
            Column::new("check_in", "Entrada", ColumnKind::Date),
            Column::new("check_out", "Saída", ColumnKind::Date),
            Column::new("nights", "Noites", ColumnKind::Int),
            Column::new("status", "Situação", ColumnKind::Text),
            Column::new("agreed", "Valor combinado", ColumnKind::Money),
        ],
        rows,
        Box::new(|rows| {
            Some(vec![total_label(), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Int(sum_int(rows, 5)), Cell::Empty, Cell::Money(sum_money(rows, 7))])
        }),
    ))
}

fn stays(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = report_repo::stays(conn, scope)?;
    Ok((
        vec![
            Column::new("code", "Código", ColumnKind::Text),
            Column::new("guest", "Hóspede", ColumnKind::Text),
            Column::new("room", "Quarto", ColumnKind::Text),
            Column::new("check_in", "Entrada", ColumnKind::Date),
            Column::new("check_out", "Saída prevista", ColumnKind::Date),
            Column::new("status", "Situação", ColumnKind::Text),
            Column::new("balance", "Saldo", ColumnKind::Money),
        ],
        rows,
        Box::new(|rows| Some(vec![total_label(), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Money(sum_money(rows, 6))])),
    ))
}

fn cancellations(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = report_repo::cancellations(conn, scope)?;
    Ok((
        vec![
            Column::new("code", "Código", ColumnKind::Text),
            Column::new("guest", "Hóspede", ColumnKind::Text),
            Column::new("room", "Quarto", ColumnKind::Text),
            Column::new("type", "Tipo", ColumnKind::Text),
            Column::new("date", "Data", ColumnKind::Date),
            Column::new("retained", "Valor retido", ColumnKind::Money),
            Column::new("refund", "Devolução prevista", ColumnKind::Money),
        ],
        rows,
        Box::new(|rows| Some(vec![total_label(), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Money(sum_money(rows, 5)), Cell::Money(sum_money(rows, 6))])),
    ))
}

fn receipts(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = finance::receipts(conn, scope)?;
    Ok((
        vec![
            Column::new("date", "Data", ColumnKind::Date),
            Column::new("time", "Hora", ColumnKind::Time),
            Column::new("number", "Número", ColumnKind::Text),
            Column::new("type", "Tipo", ColumnKind::Text),
            Column::new("guest", "Hóspede", ColumnKind::Text),
            Column::new("reference", "Referência", ColumnKind::Text),
            Column::new("method", "Forma", ColumnKind::Text),
            Column::new("amount", "Valor", ColumnKind::Money),
        ],
        rows,
        Box::new(|rows| {
            Some(vec![Cell::text("Líquido (recebido menos devolvido)"), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Money(sum_money(rows, 7))])
        }),
    ))
}

fn balances(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = finance::balances(conn, scope)?;
    Ok((
        vec![
            Column::new("stay", "Hospedagem", ColumnKind::Text),
            Column::new("guest", "Hóspede", ColumnKind::Text),
            Column::new("room", "Quarto", ColumnKind::Text),
            Column::new("check_out", "Saída prevista", ColumnKind::Date),
            Column::new("to_receive", "Saldo a receber", ColumnKind::Money),
            Column::new("credit", "Crédito do hóspede", ColumnKind::Money),
        ],
        rows,
        Box::new(|rows| Some(vec![total_label(), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Money(sum_money(rows, 4)), Cell::Money(sum_money(rows, 5))])),
    ))
}

fn guest_expenses(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = finance::guest_expenses(conn, scope)?;
    Ok((
        vec![
            Column::new("date", "Data", ColumnKind::Date),
            Column::new("stay", "Hospedagem", ColumnKind::Text),
            Column::new("guest", "Hóspede", ColumnKind::Text),
            Column::new("room", "Quarto", ColumnKind::Text),
            Column::new("item", "Item", ColumnKind::Text),
            Column::new("type", "Tipo", ColumnKind::Text),
            Column::new("amount", "Valor", ColumnKind::Money),
        ],
        rows,
        Box::new(|rows| Some(vec![Cell::text("Líquido"), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Money(sum_money(rows, 6))])),
    ))
}

fn property_expenses(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = finance::property_expenses(conn, scope)?;
    Ok((
        vec![
            Column::new("competence", "Competência", ColumnKind::Date),
            Column::new("code", "Código", ColumnKind::Text),
            Column::new("supplier", "Fornecedor", ColumnKind::Text),
            Column::new("description", "Descrição", ColumnKind::Text),
            Column::new("category", "Categoria", ColumnKind::Text),
            Column::new("amount", "Valor", ColumnKind::Money),
            Column::new("paid", "Pago", ColumnKind::Money),
            Column::new("open", "Em aberto", ColumnKind::Money),
        ],
        rows,
        Box::new(|rows| {
            Some(vec![total_label(), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Money(sum_money(rows, 5)), Cell::Money(sum_money(rows, 6)), Cell::Money(sum_money(rows, 7))])
        }),
    ))
}

fn payables(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = finance::payables(conn, scope)?;
    Ok((
        vec![
            Column::new("due", "Vencimento", ColumnKind::Date),
            Column::new("code", "Código", ColumnKind::Text),
            Column::new("supplier", "Fornecedor", ColumnKind::Text),
            Column::new("category", "Categoria", ColumnKind::Text),
            Column::new("amount", "Valor", ColumnKind::Money),
            Column::new("paid", "Pago", ColumnKind::Money),
            Column::new("open", "Em aberto", ColumnKind::Money),
            Column::new("status", "Situação", ColumnKind::Text),
            Column::new("last_payment", "Último pagamento", ColumnKind::Date),
        ],
        rows,
        Box::new(|rows| {
            Some(vec![total_label(), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Money(sum_money(rows, 4)), Cell::Money(sum_money(rows, 5)), Cell::Money(sum_money(rows, 6)), Cell::Empty, Cell::Empty])
        }),
    ))
}

fn closings(conn: &rusqlite::Connection, scope: &Scope) -> Result<Built, AppError> {
    let rows = finance::closings(conn, scope)?;
    Ok((
        vec![
            Column::new("day", "Dia", ColumnKind::Date),
            Column::new("closed_at", "Fechado em", ColumnKind::Text),
            Column::new("closed_by", "Por", ColumnKind::Text),
            Column::new("method", "Forma", ColumnKind::Text),
            Column::new("expected", "Esperado", ColumnKind::Money),
            Column::new("counted", "Contado", ColumnKind::Money),
            Column::new("difference", "Diferença", ColumnKind::Money),
            Column::new("justification", "Justificativa", ColumnKind::Text),
        ],
        rows,
        Box::new(|rows| {
            Some(vec![total_label(), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Money(sum_money(rows, 4)), Cell::Money(sum_money(rows, 5)), Cell::Money(sum_money(rows, 6)), Cell::Empty])
        }),
    ))
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::models::catalog::NewRoom;
    use crate::models::reservation::NewReservation;
    use crate::models::stay::{CheckInInput, OccupantInput};
    use crate::services::test_support::{admin_token, book, harness, recepcao_token, seed, Harness, Seed};

    // Fuso -03:00. "Hoje" é 21/10/2026, 09:00 local. Diária R$ 200,00; quarto 101.

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
            .create_room(&w.admin, NewRoom { code: code.into(), category_id: w.seed.category_id, max_capacity: 2, description: String::new(), notes: String::new() })
            .unwrap()
    }

    fn book_in(w: &World, room_id: i64, guest_id: i64, from: &str, to: &str) -> i64 {
        w.h.reservations
            .create(
                &w.admin,
                NewReservation {
                    room_id,
                    guest_id,
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

    fn query(kind: &str, from: &str, to: &str) -> ReportQuery {
        ReportQuery { from: Some(from.into()), to: Some(to.into()), ..ReportQuery::new(kind) }
    }

    fn run(w: &World, q: ReportQuery) -> ReportTable {
        w.h.reports.run(&w.admin, q).unwrap()
    }

    fn texts(row: &[Cell]) -> Vec<String> {
        row.iter().map(crate::domain::report::cell_text).collect()
    }

    #[test]
    fn reception_runs_operational_reports_and_is_forbidden_from_every_financial_one() {
        let w = world();
        let reception = recepcao_token(&w.h, &w.admin);

        for kind in ["occupancy", "reservations", "stays", "cancellations"] {
            assert!(w.h.reports.run(&reception, ReportQuery::new(kind)).is_ok(), "{kind}");
        }
        for kind in ["receipts", "balances", "guest_expenses", "property_expenses", "payables", "closings"] {
            let result = w.h.reports.run(&reception, ReportQuery::new(kind));
            assert!(matches!(result, Err(AppError::Forbidden)), "{kind}: {result:?}");
        }
        // A permissão vem antes da validação: filtro inválido também dá Forbidden.
        let bad = ReportQuery { from: Some("lixo".into()), ..ReportQuery::new("receipts") };
        assert!(matches!(w.h.reports.run(&reception, bad), Err(AppError::Forbidden)));
    }

    #[test]
    fn invalid_periods_and_unknown_reports_are_refused() {
        let w = world();
        for (from, to) in [("2026-10-22", "2026-10-21"), ("2026-01-01", "2027-01-02"), ("2026-13-01", "2026-13-02")] {
            let result = w.h.reports.run(&w.admin, query("occupancy", from, to));
            assert!(matches!(result, Err(AppError::Validation(_))), "{from} a {to}: {result:?}");
        }
        assert!(w.h.reports.run(&w.admin, query("occupancy", "2026-01-01", "2027-01-01")).is_ok(), "366 dias cabem");
        assert!(matches!(w.h.reports.run(&w.admin, ReportQuery::new("nada")), Err(AppError::Validation(_))));
    }

    #[test]
    fn unsupported_filter_is_refused_instead_of_ignored() {
        let w = world();
        let q = ReportQuery { method: Some("pix".into()), ..ReportQuery::new("occupancy") };

        let error = w.h.reports.run(&w.admin, q).unwrap_err();

        assert!(matches!(&error, AppError::Validation(m) if m.contains("forma de pagamento")), "{error:?}");
        let bad_status = ReportQuery { status: Some("in_house".into()), ..ReportQuery::new("reservations") };
        assert!(matches!(w.h.reports.run(&w.admin, bad_status), Err(AppError::Validation(_))));
    }

    #[test]
    fn default_period_is_the_last_30_days() {
        let w = world();

        let table = run(&w, ReportQuery::new("occupancy"));

        assert_eq!(table.rows.len(), 30);
        assert_eq!(texts(&table.rows[0])[0], "22/09/2026");
        assert_eq!(texts(&table.rows[29])[0], "21/10/2026");
        assert_eq!(table.filters, ["Período: 22/09/2026 a 21/10/2026"]);
    }

    #[test]
    fn occupancy_has_one_row_per_day_and_matches_the_dashboard() {
        let w = world();
        book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        room(&w, "102");

        let table = run(&w, query("occupancy", "2026-10-21", "2026-10-24"));
        let dash = w.h.management.dashboard(&w.admin, "2026-10-21", "2026-10-24").unwrap();

        assert_eq!(table.rows.len(), 4);
        assert_eq!(texts(&table.rows[0]), ["21/10/2026", "2", "1", "1", "50,0%"]);
        assert_eq!(texts(&table.rows[2]), ["23/10/2026", "2", "0", "2", "0,0%"]);
        for (row, day) in table.rows.iter().zip(&dash.occupancy) {
            assert_eq!(row[2], Cell::Int(day.occupied));
            assert_eq!(row[4], Cell::Percent(day.rate_bp));
        }
        assert_eq!(table.totals.as_ref().unwrap()[4], Cell::Percent(2_500), "2 noites de 8");
    }

    #[test]
    fn occupancy_can_be_limited_to_one_room() {
        let w = world();
        let other = room(&w, "102");
        book_in(&w, other, w.seed.guest_id, "2026-10-21", "2026-10-22");

        let only_101 = run(&w, ReportQuery { room_id: Some(w.seed.room_id), ..query("occupancy", "2026-10-21", "2026-10-21") });
        let only_102 = run(&w, ReportQuery { room_id: Some(other), ..query("occupancy", "2026-10-21", "2026-10-21") });

        assert_eq!(texts(&only_101.rows[0]), ["21/10/2026", "1", "0", "1", "0,0%"]);
        assert_eq!(texts(&only_102.rows[0]), ["21/10/2026", "1", "1", "0", "100,0%"]);
        assert!(only_102.filters.contains(&"Quarto: 102".to_string()));
    }

    #[test]
    fn reservations_use_the_arrival_date() {
        let w = world();
        let room2 = room(&w, "102");
        book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        book_in(&w, room2, w.seed.guest_id, "2026-10-22", "2026-10-24");

        let one_day = run(&w, query("reservations", "2026-10-21", "2026-10-21"));
        let two_days = run(&w, query("reservations", "2026-10-21", "2026-10-22"));

        assert_eq!(one_day.rows.len(), 1);
        assert_eq!(texts(&one_day.rows[0])[3], "21/10/2026");
        assert_eq!(two_days.rows.len(), 2);
        let totals = two_days.totals.unwrap();
        assert_eq!(totals[5], Cell::Int(4), "noites somadas");
        assert_eq!(totals[7], Cell::Money(80_000));
    }

    #[test]
    fn reservations_combine_status_room_and_text_filters() {
        let w = world();
        let room2 = room(&w, "102");
        let room3 = room(&w, "103");
        let other_guest = match w
            .h
            .guests
            .create(
                &w.admin,
                crate::models::guest::GuestInput { full_name: "Bruno Lima".into(), phone: "(11) 98888-0000".into(), cpf: None, email: String::new(), address: String::new() },
                true,
            )
            .unwrap()
        {
            crate::services::guest_service::CreateGuestOutcome::Created { id, .. } => id,
            other => panic!("{other:?}"),
        };
        book_in(&w, w.seed.room_id, w.seed.guest_id, "2026-10-21", "2026-10-22"); // Ana, 101
        let bruno_pending = book_in(&w, room2, other_guest, "2026-10-21", "2026-10-22"); // Bruno, 102
        let bruno_other = book_in(&w, room3, other_guest, "2026-10-21", "2026-10-22"); // Bruno, 103
        let version = w.h.reservations.get(&w.admin, bruno_other).unwrap().reservation.version;
        w.h.reservations.confirm(&w.admin, bruno_other, version).unwrap();

        let q = ReportQuery {
            status: Some("pending".into()),
            room_id: Some(room2),
            text: Some("bruno".into()),
            ..query("reservations", "2026-10-21", "2026-10-21")
        };
        let table = run(&w, q);

        assert_eq!(table.rows.len(), 1);
        assert_eq!(texts(&table.rows[0])[0], w.h.reservations.get(&w.admin, bruno_pending).unwrap().reservation.code);
        assert_eq!(table.filters, ["Período: 21/10/2026 a 21/10/2026", "Situação: Pendente", "Quarto: 102", "Texto: bruno"]);
    }

    #[test]
    fn stays_list_balance_and_status_by_local_checkin_day() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        at(&w, "2026-10-22T02:30:00Z"); // 23:30 de 21/10 no horário local
        let stay = check_in(&w, reservation);

        let on_21 = run(&w, query("stays", "2026-10-21", "2026-10-21"));
        let on_22 = run(&w, query("stays", "2026-10-22", "2026-10-22"));

        assert_eq!(on_21.rows.len(), 1, "o check-in das 23:30 locais é do dia 21");
        assert!(on_22.rows.is_empty());
        let balance = w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents;
        assert_eq!(on_21.rows[0][6], Cell::Money(balance));
        assert_eq!(texts(&on_21.rows[0])[5], "Hospedado");
        let finished = ReportQuery { status: Some("checked_out".into()), ..query("stays", "2026-10-21", "2026-10-21") };
        assert!(run(&w, finished).rows.is_empty());
    }

    #[test]
    fn cancellations_list_cancelled_and_no_show_with_retained_amount() {
        let w = world();
        let room2 = room(&w, "102");
        let cancelled = book(&w.h, &w.admin, &w.seed, "2026-10-25", "2026-10-27");
        let version = w.h.reservations.get(&w.admin, cancelled).unwrap().reservation.version;
        w.h.reservations.cancel(&w.admin, cancelled, version, None).unwrap();
        let no_show = book_in(&w, room2, w.seed.guest_id, "2026-10-21", "2026-10-22");
        at(&w, "2026-10-21T20:00:00Z"); // 17:00 locais: passou 14:00 + 60 min
        let version = w.h.reservations.get(&w.admin, no_show).unwrap().reservation.version;
        w.h.reservations.mark_no_show(&w.admin, no_show, version).unwrap();

        let table = run(&w, query("cancellations", "2026-10-21", "2026-10-21"));
        let only_no_show = run(&w, ReportQuery { status: Some("no_show".into()), ..query("cancellations", "2026-10-21", "2026-10-21") });

        assert_eq!(table.rows.len(), 2);
        let kinds: Vec<String> = table.rows.iter().map(|r| texts(r)[3].clone()).collect();
        assert!(kinds.contains(&"Cancelamento".to_string()) && kinds.contains(&"No-show".to_string()), "{kinds:?}");
        assert_eq!(only_no_show.rows.len(), 1);
        assert_eq!(texts(&only_no_show.rows[0])[3], "No-show");
        assert!(table.totals.is_some());
    }

    #[test]
    fn rows_over_the_limit_are_truncated_with_a_flag_and_no_totals() {
        let w = world();
        for n in 0..5 {
            let id = room(&w, &format!("R{n}"));
            book_in(&w, id, w.seed.guest_id, "2026-10-21", "2026-10-22");
        }

        let table = w.h.reports.run_with_limit(&w.admin, query("reservations", "2026-10-21", "2026-10-21"), 3).unwrap();
        let full = w.h.reports.run_with_limit(&w.admin, query("reservations", "2026-10-21", "2026-10-21"), 5).unwrap();

        assert_eq!(table.rows.len(), 3);
        assert!(table.truncated && table.totals.is_none());
        assert_eq!(full.rows.len(), 5);
        assert!(!full.truncated && full.totals.is_some());
    }

    #[test]
    fn requires_a_session() {
        let w = world();

        assert!(w.h.reports.run("invalido", ReportQuery::new("occupancy")).is_err());
    }
}

#[cfg(test)]
mod pdf_tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::domain::pdf::extract_text;
    use crate::services::test_support::{admin_token, book, harness, recepcao_token, seed};

    fn at(h: &crate::services::test_support::Harness, rfc3339: &str) {
        h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    #[test]
    fn pdf_contains_the_title_filters_and_every_row_of_the_screen() {
        let h = harness();
        let admin = admin_token(&h);
        let seed = seed(&h, &admin, 20_000, 2);
        book(&h, &admin, &seed, "2026-10-21", "2026-10-23");
        let query = ReportQuery { from: Some("2026-10-21".into()), to: Some("2026-10-21".into()), ..ReportQuery::new("reservations") };

        let table = h.reports.run(&admin, query.clone()).unwrap();
        let pdf = h.reports.pdf(&admin, query).unwrap();

        let texts = extract_text(&pdf);
        assert!(texts.contains(&"Reservas".to_string()), "{texts:?}");
        assert!(texts.contains(&"Período: 21/10/2026 a 21/10/2026".to_string()));
        assert!(texts.iter().any(|t| t.starts_with("Emitido em 21/10/2026 09:00")), "{texts:?}");
        for row in &table.rows {
            for cell in row {
                let text = cell_text(cell);
                if !text.is_empty() {
                    assert!(texts.contains(&text), "falta {text:?} no PDF");
                }
            }
        }
        let totals = table.totals.unwrap();
        assert!(texts.contains(&cell_text(&totals[7])), "o total da tela também está no PDF");
        assert!(String::from_utf8_lossy(&pdf).contains("/MediaBox [0 0 842 595]"), "8 colunas usam paisagem");
    }

    #[test]
    fn pdf_of_a_financial_report_requires_the_permission_and_writes_nothing_else() {
        let h = harness();
        let admin = admin_token(&h);
        let reception = recepcao_token(&h, &admin);
        at(&h, "2026-10-21T12:00:00Z");

        let result = h.reports.pdf(&reception, ReportQuery::new("receipts"));

        assert!(matches!(result, Err(AppError::Forbidden)), "{result:?}");
        assert!(h.reports.pdf(&admin, ReportQuery::new("receipts")).is_ok());
    }

    #[test]
    fn narrow_reports_stay_in_portrait_and_empty_ones_say_so() {
        let h = harness();
        let admin = admin_token(&h);

        let pdf = h.reports.pdf(&admin, ReportQuery::new("balances")).unwrap();

        assert!(String::from_utf8_lossy(&pdf).contains("/MediaBox [0 0 595 842]"));
        assert!(extract_text(&pdf).contains(&"Nenhum registro no período.".to_string()));
    }
}

#[cfg(test)]
mod finance_tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::models::closing::{CloseInput, CountedInput};
    use crate::models::payable::{NewPayable, PayInput};
    use crate::models::payment::{ComponentInput, PaymentInput, PaymentTarget};
    use crate::models::refund::RefundInput;
    use crate::models::stay::{ChargeInput, CheckInInput, OccupantInput};
    use crate::services::test_support::{admin_token, book, harness, pay, seed, Harness, Seed};

    // Fuso -03:00. "Hoje" é 21/10/2026, 09:00 local. Diária R$ 200,00; quarto 101.

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

    fn stay_in_house(w: &World) -> i64 {
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        at(w, "2026-10-21T17:00:00Z");
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        w.h.stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
            .unwrap()
    }

    fn query(kind: &str, from: &str, to: &str) -> ReportQuery {
        ReportQuery { from: Some(from.into()), to: Some(to.into()), ..ReportQuery::new(kind) }
    }

    fn run(w: &World, q: ReportQuery) -> ReportTable {
        w.h.reports.run(&w.admin, q).unwrap()
    }

    fn texts(row: &[Cell]) -> Vec<String> {
        row.iter().map(crate::domain::report::cell_text).collect()
    }

    fn category(w: &World, name: &str) -> i64 {
        w.h.payables.create_category(&w.admin, name).unwrap()
    }

    fn bill(category_id: i64, cents: i64, competence: &str, due: &str) -> NewPayable {
        NewPayable {
            supplier: "Fornecedor".into(),
            description: "Conta".into(),
            amount_cents: cents,
            competence_date: competence.into(),
            due_date: due.into(),
            category_id,
            notes: String::new(),
            paid_now: None,
        }
    }

    #[test]
    fn every_financial_report_runs_for_the_administrator_on_an_empty_database() {
        let w = world();

        for kind in ["receipts", "balances", "guest_expenses", "property_expenses", "payables", "closings"] {
            let table = run(&w, ReportQuery::new(kind));
            assert!(table.rows.is_empty(), "{kind}");
            assert_eq!(table.kind, kind);
            assert!(!table.columns.is_empty());
        }
    }

    #[test]
    fn receipts_have_one_row_per_component_and_split_debit_credit() {
        let w = world();
        let stay = stay_in_house(&w);
        w.h.payments
            .record(
                &w.admin,
                PaymentTarget::Stay { stay_id: stay },
                PaymentInput {
                    components: vec![
                        ComponentInput { method: "debit".into(), amount_cents: 6_000 },
                        ComponentInput { method: "credit".into(), amount_cents: 4_000 },
                    ],
                    note: String::new(),
                    duplicate_release: None,
                },
            )
            .unwrap();

        let table = run(&w, query("receipts", "2026-10-21", "2026-10-21"));

        assert_eq!(table.rows.len(), 2);
        let methods: Vec<String> = table.rows.iter().map(|r| texts(r)[6].clone()).collect();
        assert!(methods.contains(&"Cartão de débito".to_string()) && methods.contains(&"Cartão de crédito".to_string()), "{methods:?}");
        assert_eq!(table.totals.unwrap()[7], Cell::Money(10_000));
        let only_debit = run(&w, ReportQuery { method: Some("debit".into()), ..query("receipts", "2026-10-21", "2026-10-21") });
        assert_eq!(only_debit.rows.len(), 1);
        assert_eq!(only_debit.rows[0][7], Cell::Money(6_000));
    }

    #[test]
    fn receipts_use_the_local_day() {
        let w = world();
        let stay = stay_in_house(&w);
        at(&w, "2026-10-22T02:30:00Z"); // 23:30 de 21/10 no horário local
        pay(&w.h, &w.admin, stay, 10_000, "pix").unwrap();

        let on_21 = run(&w, query("receipts", "2026-10-21", "2026-10-21"));
        let on_22 = run(&w, query("receipts", "2026-10-22", "2026-10-22"));

        assert_eq!(on_21.rows.len(), 1);
        assert_eq!(texts(&on_21.rows[0])[..2], ["21/10/2026", "23:30"]);
        assert!(on_22.rows.is_empty());
    }

    #[test]
    fn receipts_net_total_matches_closing_and_dashboard_and_a_refund_is_its_own_negative_row() {
        let w = world();
        let stay = stay_in_house(&w);
        let receipt = pay(&w.h, &w.admin, stay, 10_000, "cash").unwrap();
        pay(&w.h, &w.admin, stay, 4_000, "pix").unwrap();
        w.h.refunds
            .record(
                &w.admin,
                RefundInput { payment_id: receipt.id, components: vec![ComponentInput { method: "cash".into(), amount_cents: 2_500 }], reason: "Ajuste".into() },
            )
            .unwrap();

        let table = run(&w, query("receipts", "2026-10-21", "2026-10-21"));
        let net = match table.totals.as_ref().unwrap()[7] {
            Cell::Money(c) => c,
            _ => panic!("líquido"),
        };
        let preview = w.h.closing.preview(&w.admin, "2026-10-21").unwrap();
        let closing_net: i64 = preview.expected.iter().map(|e| e.expected_cents).sum();
        let dash = w.h.management.dashboard(&w.admin, "2026-10-21", "2026-10-21").unwrap();

        assert_eq!(net, 11_500);
        assert_eq!(net, closing_net, "bate com o fechamento de caixa");
        assert_eq!(net, dash.receipts_daily[0].net_cents, "bate com a visão gerencial");
        let refund_row = table.rows.iter().find(|r| texts(r)[3] == "Devolução").unwrap();
        assert_eq!(refund_row[7], Cell::Money(-2_500));
    }

    #[test]
    fn balances_match_the_stay_account_and_split_credit() {
        let w = world();
        let stay = stay_in_house(&w);
        let balance = w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents;

        let owing = run(&w, ReportQuery::new("balances"));
        assert_eq!(owing.rows.len(), 1);
        assert_eq!(owing.rows[0][4], Cell::Money(balance));
        assert_eq!(owing.rows[0][5], Cell::Money(0));

        pay(&w.h, &w.admin, stay, balance + 1_500, "pix").unwrap();
        let credit = run(&w, ReportQuery::new("balances"));
        assert_eq!((&credit.rows[0][4], &credit.rows[0][5]), (&Cell::Money(0), &Cell::Money(1_500)));
        assert_eq!(credit.totals.unwrap()[5], Cell::Money(1_500));
    }

    #[test]
    fn paid_up_stays_are_not_listed_in_balances() {
        let w = world();
        let stay = stay_in_house(&w);
        let balance = w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents;
        pay(&w.h, &w.admin, stay, balance, "cash").unwrap();

        assert!(run(&w, ReportQuery::new("balances")).rows.is_empty());
    }

    #[test]
    fn guest_expenses_include_reversals_and_net_zero_for_reversed_items() {
        let w = world();
        let stay = stay_in_house(&w);
        w.h.stays.add_charge(&w.admin, stay, ChargeInput { kind: "consumption".into(), description: "Água".into(), amount_cents: 500, item_id: None }).unwrap();
        w.h.stays.add_charge(&w.admin, stay, ChargeInput { kind: "extra".into(), description: "Lavanderia".into(), amount_cents: 3_000, item_id: None }).unwrap();
        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        let water = account.charges.iter().find(|c| c.description == "Água").unwrap().id;
        w.h.stays.reverse_charge(&w.admin, stay, water, "Lançado por engano".into(), None).unwrap();

        let table = run(&w, query("guest_expenses", "2026-10-21", "2026-10-21"));

        let kinds: Vec<String> = table.rows.iter().map(|r| texts(r)[5].clone()).collect();
        assert_eq!(kinds, ["Consumo", "Despesa extra", "Estorno"]);
        assert_eq!(table.totals.unwrap()[6], Cell::Money(3_000), "água estornada soma zero; sobra a lavanderia");
        assert!(table.rows.iter().all(|r| texts(r)[0] == "21/10/2026"));
    }

    #[test]
    fn property_expenses_use_competence_and_show_paid_in_period_separately() {
        let w = world();
        let energy = category(&w, "Energia do mês");
        let id = w.h.payables.create(&w.admin, bill(energy, 50_000, "2026-10-15", "2026-11-10")).unwrap();
        at(&w, "2026-11-05T15:00:00Z");
        let version = w.h.payables.get(&w.admin, id).unwrap().payable.version;
        w.h.payables.pay(&w.admin, id, PayInput { amount_cents: 20_000, method: "pix".into(), note: String::new() }, version).unwrap();

        let october = run(&w, query("property_expenses", "2026-10-01", "2026-10-31"));
        let november = run(&w, query("property_expenses", "2026-11-01", "2026-11-30"));

        assert_eq!(october.rows.len(), 1, "a competência é de outubro");
        assert_eq!(texts(&october.rows[0])[5..], ["R$ 500,00", "R$ 200,00", "R$ 300,00"]);
        assert!(october.filters.contains(&"Pago no período (data efetiva): R$ 0,00".to_string()), "{:?}", october.filters);
        assert!(november.rows.is_empty());
        assert!(november.filters.contains(&"Pago no período (data efetiva): R$ 200,00".to_string()), "{:?}", november.filters);
    }

    #[test]
    fn payables_filter_by_status_and_category_combined_and_use_the_due_date() {
        let w = world();
        let energy = category(&w, "Energia do mês");
        let water = category(&w, "Água do mês");
        w.h.payables.create(&w.admin, bill(energy, 10_000, "2026-10-01", "2026-10-10")).unwrap(); // vencida
        w.h.payables.create(&w.admin, bill(water, 20_000, "2026-10-01", "2026-10-10")).unwrap(); // vencida, outra categoria
        w.h.payables.create(&w.admin, bill(energy, 30_000, "2026-10-01", "2026-10-30")).unwrap(); // pendente
        let paid = w.h.payables.create(&w.admin, bill(energy, 5_000, "2026-10-01", "2026-10-12")).unwrap();
        let version = w.h.payables.get(&w.admin, paid).unwrap().payable.version;
        w.h.payables.pay(&w.admin, paid, PayInput { amount_cents: 5_000, method: "cash".into(), note: String::new() }, version).unwrap();

        let overdue_energy = run(
            &w,
            ReportQuery { status: Some("overdue".into()), category_id: Some(energy), ..query("payables", "2026-10-01", "2026-10-31") },
        );
        let all = run(&w, query("payables", "2026-10-01", "2026-10-31"));

        assert_eq!(overdue_energy.rows.len(), 1);
        assert_eq!(texts(&overdue_energy.rows[0])[7], "Vencida");
        assert_eq!(overdue_energy.filters.last().unwrap(), "Categoria: Energia do mês");
        assert_eq!(all.rows.len(), 4);
        assert_eq!(texts(&all.rows[1])[0], "10/10/2026", "ordenado pelo vencimento");
        let paid_row = all.rows.iter().find(|r| texts(r)[7] == "Paga").unwrap();
        assert_eq!(paid_row[8], Cell::text("21/10/2026"), "último pagamento");
        assert_eq!(all.totals.unwrap()[6], Cell::Money(60_000), "em aberto soma só o que falta");
    }

    #[test]
    fn closings_list_difference_and_justification() {
        let w = world();
        let stay = stay_in_house(&w);
        pay(&w.h, &w.admin, stay, 10_000, "cash").unwrap();
        let counted: Vec<CountedInput> = [("cash", 9_000, Some("Troco devolvido")), ("pix", 0, None), ("debit", 0, None), ("credit", 0, None), ("transfer", 0, None)]
            .into_iter()
            .map(|(m, c, j)| CountedInput { method: m.into(), counted_cents: c, justification: j.map(str::to_string) })
            .collect();
        w.h.closing.close(&w.admin, CloseInput { date: "2026-10-21".into(), counted }).unwrap();

        let table = run(&w, query("closings", "2026-10-21", "2026-10-21"));
        let cash_only = run(&w, ReportQuery { method: Some("cash".into()), ..query("closings", "2026-10-21", "2026-10-21") });

        assert_eq!(table.rows.len(), 5);
        let cash = cash_only.rows[0].clone();
        assert_eq!(texts(&cash)[3], "Dinheiro");
        assert_eq!((&cash[4], &cash[5], &cash[6]), (&Cell::Money(10_000), &Cell::Money(9_000), &Cell::Money(-1_000)));
        assert_eq!(texts(&cash)[7], "Troco devolvido");
        assert_eq!(table.totals.unwrap()[6], Cell::Money(-1_000));
        assert_eq!(cash_only.rows.len(), 1);
    }

    #[test]
    fn period_of_one_day_returns_only_that_day() {
        let w = world();
        let stay = stay_in_house(&w);
        pay(&w.h, &w.admin, stay, 1_000, "pix").unwrap();
        at(&w, "2026-10-22T15:00:00Z");
        pay(&w.h, &w.admin, stay, 2_000, "pix").unwrap();

        let table = run(&w, query("receipts", "2026-10-22", "2026-10-22"));

        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][7], Cell::Money(2_000));
    }

    #[test]
    fn invalid_method_and_category_filters_are_refused() {
        let w = world();

        let bad_method = ReportQuery { method: Some("cheque".into()), ..ReportQuery::new("receipts") };
        let bad_category = ReportQuery { category_id: Some(9_999), ..ReportQuery::new("payables") };

        assert!(matches!(w.h.reports.run(&w.admin, bad_method), Err(AppError::Validation(_))));
        assert!(matches!(w.h.reports.run(&w.admin, bad_category), Err(AppError::Validation(_))));
    }
}
