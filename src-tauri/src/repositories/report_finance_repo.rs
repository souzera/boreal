//! Consultas dos relatórios financeiros (somente leitura; o serviço exige `ViewFinancialReports`).

use rusqlite::{named_params, Connection};

use crate::domain::payment::Method;
use crate::domain::report::{br_date, Cell};
use crate::error::AppError;
use crate::repositories::report_repo::{Rows, Scope};
use crate::repositories::stay_repo;

fn date_cell(iso: &str) -> Cell {
    Cell::Text(br_date(iso))
}

fn method_label(method: &str) -> String {
    Method::parse(method).map_or_else(|| method.to_string(), |m| m.label().to_string())
}

/// Recebimentos e devoluções no período (dia **local**), uma linha por componente (débito e crédito separados).
/// Devolução sai com valor negativo, então a soma da coluna é o líquido.
pub fn receipts(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM (
            SELECT date(p.received_at, :shift) AS d, strftime('%H:%M', p.received_at, :shift) AS t, p.code AS code,
                   'Recebimento' AS kind, g.full_name AS guest, COALESCE(s.code, r.code) AS ref, c.method AS method,
                   c.amount_cents AS amount, p.received_at AS at, p.id AS id
              FROM payment_components c JOIN payments p ON p.id = c.payment_id
                   JOIN reservations r ON r.id = p.reservation_id JOIN guests g ON g.id = r.guest_id
                   LEFT JOIN stays s ON s.id = p.stay_id
             WHERE p.received_at >= :start AND p.received_at < :end
               AND (:method IS NULL OR c.method = :method) AND (:room IS NULL OR r.room_id = :room)
               AND (:code IS NULL OR p.code LIKE :code ESCAPE '\\' OR r.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\')
            UNION ALL
            SELECT date(f.refunded_at, :shift), strftime('%H:%M', f.refunded_at, :shift), f.code,
                   'Devolução', g.full_name, COALESCE(s.code, r.code), c.method,
                   -c.amount_cents, f.refunded_at, f.id
              FROM refund_components c JOIN refunds f ON f.id = c.refund_id
                   JOIN reservations r ON r.id = f.reservation_id JOIN guests g ON g.id = r.guest_id
                   LEFT JOIN stays s ON s.id = f.stay_id
             WHERE f.refunded_at >= :start AND f.refunded_at < :end
               AND (:method IS NULL OR c.method = :method) AND (:room IS NULL OR r.room_id = :room)
               AND (:code IS NULL OR f.code LIKE :code ESCAPE '\\' OR r.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\')
         ) ORDER BY at, kind, id, method LIMIT :limit",
    )?;
    let rows = stmt
        .query_map(
            named_params! {
                ":shift": scope.shift(), ":start": scope.start, ":end": scope.end, ":method": scope.method, ":room": scope.room_id,
                ":code": scope.code_pattern(), ":name": scope.name_pattern(), ":limit": scope.fetch()
            },
            |r| {
                Ok(vec![
                    date_cell(&r.get::<_, String>(0)?),
                    Cell::Text(r.get(1)?),
                    Cell::Text(r.get(2)?),
                    Cell::Text(r.get(3)?),
                    Cell::Text(r.get(4)?),
                    Cell::Text(r.get(5)?),
                    Cell::Text(method_label(&r.get::<_, String>(6)?)),
                    Cell::Money(r.get(7)?),
                ])
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Hospedagens em andamento com saldo diferente de zero (situação de agora).
pub fn balances(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.code, g.full_name, rm.code, r.check_out_date
         FROM stays s JOIN reservations r ON r.id = s.reservation_id JOIN rooms rm ON rm.id = r.room_id JOIN guests g ON g.id = r.guest_id
         WHERE s.status = 'in_house' AND (:room IS NULL OR r.room_id = :room)
           AND (:code IS NULL OR s.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\' OR rm.code LIKE :code ESCAPE '\\')
         ORDER BY r.check_out_date, s.id",
    )?;
    let raw = stmt
        .query_map(
            named_params! { ":room": scope.room_id, ":code": scope.code_pattern(), ":name": scope.name_pattern() },
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?)),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    for (id, code, guest, room, leaves) in raw {
        let balance = stay_repo::balance(conn, id)?;
        if balance == 0 {
            continue;
        }
        rows.push(vec![
            Cell::Text(code),
            Cell::Text(guest),
            Cell::Text(room),
            date_cell(&leaves),
            Cell::Money(balance.max(0)),
            Cell::Money((-balance).max(0)),
        ]);
    }
    Ok(rows)
}

/// Consumos e despesas extras lançados no período (dia local do lançamento), com os estornos correspondentes.
pub fn guest_expenses(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    let mut stmt = conn.prepare(
        "SELECT date(c.created_at, :shift), s.code, g.full_name, rm.code, c.description,
                CASE WHEN c.kind = 'reversal' THEN 'Estorno' WHEN c.kind = 'consumption' THEN 'Consumo' ELSE 'Despesa extra' END,
                c.amount_cents
         FROM charges c JOIN stays s ON s.id = c.stay_id JOIN reservations r ON r.id = s.reservation_id
              JOIN rooms rm ON rm.id = r.room_id JOIN guests g ON g.id = r.guest_id
              LEFT JOIN charges orig ON orig.id = c.reverses_charge_id
         WHERE c.created_at >= :start AND c.created_at < :end
           AND (c.kind IN ('consumption', 'extra') OR (c.kind = 'reversal' AND orig.kind IN ('consumption', 'extra')))
           AND (:room IS NULL OR r.room_id = :room)
           AND (:code IS NULL OR s.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\' OR c.description LIKE :code ESCAPE '\\')
         ORDER BY c.created_at, c.id LIMIT :limit",
    )?;
    let rows = stmt
        .query_map(
            named_params! {
                ":shift": scope.shift(), ":start": scope.start, ":end": scope.end, ":room": scope.room_id,
                ":code": scope.code_pattern(), ":name": scope.name_pattern(), ":limit": scope.fetch()
            },
            |r| {
                Ok(vec![
                    date_cell(&r.get::<_, String>(0)?),
                    Cell::Text(r.get(1)?),
                    Cell::Text(r.get(2)?),
                    Cell::Text(r.get(3)?),
                    Cell::Text(r.get(4)?),
                    Cell::Text(r.get(5)?),
                    Cell::Money(r.get(6)?),
                ])
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

const PAYABLE_FROM: &str = "FROM (SELECT p.*, (SELECT COALESCE(SUM(x.amount_cents), 0) FROM payable_payments x WHERE x.payable_id = p.id) AS paid,
                                    (SELECT MAX(x.paid_at) FROM payable_payments x WHERE x.payable_id = p.id) AS last_paid_at
                             FROM payables p) p
     JOIN expense_categories c ON c.id = p.category_id";

const PAYABLE_STATUS: &str = "CASE WHEN p.paid >= p.amount_cents THEN 'paid' WHEN p.due_date < :today THEN 'overdue'
                                   WHEN p.paid > 0 THEN 'partial' ELSE 'pending' END";

pub fn payable_status_label(status: &str) -> &'static str {
    match status {
        "paid" => "Paga",
        "overdue" => "Vencida",
        "partial" => "Parcial",
        _ => "Pendente",
    }
}

/// Despesas da pousada por **competência** no período.
pub fn property_expenses(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    payables_rows(conn, scope, "p.competence_date", false)
}

/// Contas a pagar por **vencimento** no período.
pub fn payables(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    payables_rows(conn, scope, "p.due_date", true)
}

fn payables_rows(conn: &Connection, scope: &Scope, date_column: &str, by_due: bool) -> Result<Rows, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT p.competence_date, p.due_date, p.code, p.supplier, p.description, c.name, p.amount_cents, p.paid,
                {PAYABLE_STATUS}, date(p.last_paid_at, :shift)
         {PAYABLE_FROM}
         WHERE {date_column} BETWEEN :from AND :to
           AND (:status IS NULL OR {PAYABLE_STATUS} = :status)
           AND (:category IS NULL OR p.category_id = :category)
           AND (:code IS NULL OR p.code LIKE :code ESCAPE '\\' OR p.supplier LIKE :code ESCAPE '\\' OR p.description LIKE :code ESCAPE '\\')
         ORDER BY {date_column}, p.id LIMIT :limit"
    ))?;
    let rows = stmt
        .query_map(
            named_params! {
                ":today": scope.today, ":shift": scope.shift(), ":from": scope.from, ":to": scope.to, ":status": scope.status,
                ":category": scope.category_id, ":code": scope.code_pattern(), ":limit": scope.fetch()
            },
            |r| {
                let amount: i64 = r.get(6)?;
                let paid: i64 = r.get(7)?;
                let status: String = r.get(8)?;
                let last: Option<String> = r.get(9)?;
                if by_due {
                    Ok(vec![
                        date_cell(&r.get::<_, String>(1)?),
                        Cell::Text(r.get(2)?),
                        Cell::Text(r.get(3)?),
                        Cell::Text(r.get(5)?),
                        Cell::Money(amount),
                        Cell::Money(paid),
                        Cell::Money((amount - paid).max(0)),
                        Cell::text(payable_status_label(&status)),
                        last.map_or(Cell::Empty, |d| date_cell(&d)),
                    ])
                } else {
                    Ok(vec![
                        date_cell(&r.get::<_, String>(0)?),
                        Cell::Text(r.get(2)?),
                        Cell::Text(r.get(3)?),
                        Cell::Text(r.get(4)?),
                        Cell::Text(r.get(5)?),
                        Cell::Money(amount),
                        Cell::Money(paid),
                        Cell::Money((amount - paid).max(0)),
                    ])
                }
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Total pago a fornecedores no período, pela data **efetiva** do pagamento (dia local), com os mesmos filtros de
/// categoria, situação e texto da lista de despesas.
pub fn paid_in_period(conn: &Connection, scope: &Scope) -> Result<i64, AppError> {
    Ok(conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(x.amount_cents), 0)
             FROM payable_payments x
             JOIN (SELECT p.*, (SELECT COALESCE(SUM(y.amount_cents), 0) FROM payable_payments y WHERE y.payable_id = p.id) AS paid FROM payables p) p
               ON p.id = x.payable_id
             WHERE x.paid_at >= :start AND x.paid_at < :end
               AND (:category IS NULL OR p.category_id = :category)
               AND (:status IS NULL OR {PAYABLE_STATUS} = :status)
               AND (:code IS NULL OR p.code LIKE :code ESCAPE '\\' OR p.supplier LIKE :code ESCAPE '\\' OR p.description LIKE :code ESCAPE '\\')"
        ),
        named_params! {
            ":start": scope.start, ":end": scope.end, ":category": scope.category_id, ":status": scope.status,
            ":today": scope.today, ":code": scope.code_pattern()
        },
        |r| r.get(0),
    )?)
}

/// Fechamentos de caixa dos dias do período, uma linha por dia e forma de pagamento.
pub fn closings(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    let mut stmt = conn.prepare(
        "SELECT k.closing_date, strftime('%d/%m/%Y %H:%M', k.closed_at, :shift), u.display_name, l.method,
                l.expected_cents, l.counted_cents, l.difference_cents, COALESCE(l.justification, '')
         FROM cash_closings k JOIN cash_closing_lines l ON l.closing_id = k.id JOIN users u ON u.id = k.closed_by
         WHERE k.closing_date BETWEEN :from AND :to AND (:method IS NULL OR l.method = :method)
         ORDER BY k.closing_date, l.id LIMIT :limit",
    )?;
    let rows = stmt
        .query_map(
            named_params! { ":shift": scope.shift(), ":from": scope.from, ":to": scope.to, ":method": scope.method, ":limit": scope.fetch() },
            |r| {
                Ok(vec![
                    date_cell(&r.get::<_, String>(0)?),
                    Cell::Text(r.get(1)?),
                    Cell::Text(r.get(2)?),
                    Cell::Text(method_label(&r.get::<_, String>(3)?)),
                    Cell::Money(r.get(4)?),
                    Cell::Money(r.get(5)?),
                    Cell::Money(r.get(6)?),
                    Cell::Text(r.get(7)?),
                ])
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
