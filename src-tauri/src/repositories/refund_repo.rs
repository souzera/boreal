use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Transaction};

use crate::domain::payment::Component;
use crate::error::AppError;
use crate::models::payment::ComponentLine;

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub struct NewRefund<'a> {
    pub code: &'a str,
    pub payment_id: i64,
    pub reservation_id: i64,
    pub stay_id: Option<i64>,
    pub amount_cents: i64,
    pub reason: &'a str,
    pub refunded_by: i64,
}

pub fn insert(tx: &Transaction, r: &NewRefund, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO refunds (code, payment_id, reservation_id, stay_id, amount_cents, reason, refunded_at, refunded_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![r.code, r.payment_id, r.reservation_id, r.stay_id, r.amount_cents, r.reason, stamp(at), r.refunded_by],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn insert_components(tx: &Transaction, refund_id: i64, components: &[Component]) -> Result<(), AppError> {
    for c in components {
        tx.execute(
            "INSERT INTO refund_components (refund_id, method, amount_cents) VALUES (?1, ?2, ?3)",
            params![refund_id, c.method.as_str(), c.amount_cents],
        )?;
    }
    Ok(())
}

pub fn components(conn: &Connection, refund_id: i64) -> Result<Vec<ComponentLine>, AppError> {
    let mut stmt =
        conn.prepare("SELECT method, amount_cents FROM refund_components WHERE refund_id = ?1 ORDER BY method")?;
    let rows = stmt
        .query_map([refund_id], |r| Ok(ComponentLine { method: r.get(0)?, amount_cents: r.get(1)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn refunded_for_payment(conn: &Connection, payment_id: i64) -> Result<i64, AppError> {
    Ok(conn.query_row("SELECT COALESCE(SUM(amount_cents), 0) FROM refunds WHERE payment_id = ?1", [payment_id], |r| r.get(0))?)
}
