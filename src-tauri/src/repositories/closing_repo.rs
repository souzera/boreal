use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::domain::closing::ClosingLine;
use crate::domain::payment::Method;
use crate::error::AppError;
use crate::models::closing::{ClosingLineView, ClosingView};

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn by_method(conn: &Connection, sql: &str, start: &str, end: &str) -> Result<BTreeMap<String, i64>, AppError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map(params![start, end], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(rows)
}

/// Recebido por modalidade em `[start, end)` (instantes UTC).
pub fn received(conn: &Connection, start: &str, end: &str) -> Result<BTreeMap<String, i64>, AppError> {
    by_method(
        conn,
        "SELECT c.method, SUM(c.amount_cents) FROM payment_components c JOIN payments p ON p.id = c.payment_id
         WHERE p.received_at >= ?1 AND p.received_at < ?2 GROUP BY c.method",
        start,
        end,
    )
}

/// Devolvido por modalidade em `[start, end)`.
pub fn refunded(conn: &Connection, start: &str, end: &str) -> Result<BTreeMap<String, i64>, AppError> {
    by_method(
        conn,
        "SELECT c.method, SUM(c.amount_cents) FROM refund_components c JOIN refunds f ON f.id = c.refund_id
         WHERE f.refunded_at >= ?1 AND f.refunded_at < ?2 GROUP BY c.method",
        start,
        end,
    )
}

pub fn supplier_payments(conn: &Connection, start: &str, end: &str) -> Result<i64, AppError> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM payable_payments WHERE paid_at >= ?1 AND paid_at < ?2",
        params![start, end],
        |r| r.get(0),
    )?)
}

pub fn find_by_date(conn: &Connection, date: &str) -> Result<Option<ClosingView>, AppError> {
    let head = conn
        .query_row(
            "SELECT c.id, c.closing_date, c.closed_at, u.display_name FROM cash_closings c JOIN users u ON u.id = c.closed_by
             WHERE c.closing_date = ?1",
            [date],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)),
        )
        .optional()?;
    let Some((id, closing_date, closed_at, closed_by_name)) = head else { return Ok(None) };
    let mut stmt = conn.prepare(
        "SELECT method, expected_cents, counted_cents, difference_cents, justification FROM cash_closing_lines
         WHERE closing_id = ?1 ORDER BY id",
    )?;
    let lines = stmt
        .query_map([id], |r| {
            let method: String = r.get(0)?;
            Ok(ClosingLineView {
                label: Method::parse(&method).map_or_else(|| method.clone(), |m| m.label().to_string()),
                method,
                expected_cents: r.get(1)?,
                counted_cents: r.get(2)?,
                difference_cents: r.get(3)?,
                justification: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(ClosingView { id, closing_date, closed_at, closed_by_name, lines }))
}

pub fn insert(tx: &Transaction, date: &str, at: DateTime<Utc>, closed_by: i64) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO cash_closings (closing_date, closed_at, closed_by) VALUES (?1, ?2, ?3)",
        params![date, stamp(at), closed_by],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn insert_line(tx: &Transaction, closing_id: i64, l: &ClosingLine) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO cash_closing_lines (closing_id, method, expected_cents, counted_cents, difference_cents, justification)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![closing_id, l.method.as_str(), l.expected_cents, l.counted_cents, l.difference_cents, l.justification],
    )?;
    Ok(())
}

/// Datas locais anteriores a `today` com recebimento ou devolução e sem fechamento. `shift` é o deslocamento
/// do fuso no formato do SQLite (por exemplo `-10800 seconds`).
pub fn pending_dates(conn: &Connection, shift: &str, today: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT d FROM (
            SELECT date(received_at, ?1) AS d FROM payments
            UNION SELECT date(refunded_at, ?1) FROM refunds
         ) WHERE d < ?2 AND d NOT IN (SELECT closing_date FROM cash_closings) ORDER BY d",
    )?;
    let rows = stmt.query_map(params![shift, today], |r| r.get(0))?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
