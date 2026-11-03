//! Agregações de leitura da visão gerencial. Nada aqui grava dados.

use std::collections::BTreeMap;

use rusqlite::{named_params, params, Connection};

use crate::error::AppError;
use crate::models::management::CategoryOpen;

/// Reservas que contam como ocupação ou movimento (as canceladas e no-show ficam de fora).
const ACTIVE: &str = "('pending', 'confirmed', 'checked_in', 'completed')";

pub fn active_rooms(conn: &Connection) -> Result<i64, AppError> {
    Ok(conn.query_row("SELECT count(*) FROM rooms WHERE active = 1", [], |r| r.get(0))?)
}

/// Noites ocupadas por data (`AAAA-MM-DD`) de `from` a `to` inclusive.
pub fn occupied_nights(conn: &Connection, from: &str, to: &str) -> Result<BTreeMap<String, i64>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT n.date, count(*) FROM reservation_nights n JOIN reservations r ON r.id = n.reservation_id
         WHERE r.status IN {ACTIVE} AND n.date BETWEEN ?1 AND ?2 GROUP BY n.date"
    ))?;
    let rows = stmt
        .query_map(params![from, to], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(rows)
}

pub fn arrivals(conn: &Connection, from: &str, to: &str) -> Result<i64, AppError> {
    Ok(conn.query_row(
        &format!("SELECT count(*) FROM reservations WHERE status IN {ACTIVE} AND check_in_date BETWEEN ?1 AND ?2"),
        params![from, to],
        |r| r.get(0),
    )?)
}

pub fn departures(conn: &Connection, from: &str, to: &str) -> Result<i64, AppError> {
    Ok(conn.query_row(
        &format!("SELECT count(*) FROM reservations WHERE status IN {ACTIVE} AND check_out_date BETWEEN ?1 AND ?2"),
        params![from, to],
        |r| r.get(0),
    )?)
}

pub fn in_house(conn: &Connection) -> Result<i64, AppError> {
    Ok(conn.query_row("SELECT count(*) FROM stays WHERE status = 'in_house'", [], |r| r.get(0))?)
}

pub fn in_house_ids(conn: &Connection) -> Result<Vec<i64>, AppError> {
    let mut stmt = conn.prepare("SELECT id FROM stays WHERE status = 'in_house' ORDER BY id")?;
    let rows = stmt.query_map([], |r| r.get(0))?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn daily(conn: &Connection, sql: &str, offset_secs: i32, start: &str, end: &str) -> Result<BTreeMap<String, i64>, AppError> {
    let shift = format!("{offset_secs:+} seconds");
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map(params![shift, start, end], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(rows)
}

/// Recebido por dia **local** em `[start, end)` (instantes UTC).
pub fn received_daily(conn: &Connection, offset_secs: i32, start: &str, end: &str) -> Result<BTreeMap<String, i64>, AppError> {
    daily(
        conn,
        "SELECT date(p.received_at, ?1), SUM(c.amount_cents) FROM payment_components c JOIN payments p ON p.id = c.payment_id
         WHERE p.received_at >= ?2 AND p.received_at < ?3 GROUP BY 1",
        offset_secs,
        start,
        end,
    )
}

/// Devolvido por dia **local** em `[start, end)`.
pub fn refunded_daily(conn: &Connection, offset_secs: i32, start: &str, end: &str) -> Result<BTreeMap<String, i64>, AppError> {
    daily(
        conn,
        "SELECT date(f.refunded_at, ?1), SUM(c.amount_cents) FROM refund_components c JOIN refunds f ON f.id = c.refund_id
         WHERE f.refunded_at >= ?2 AND f.refunded_at < ?3 GROUP BY 1",
        offset_secs,
        start,
        end,
    )
}

const OPEN: &str = "FROM (SELECT p.*, (SELECT COALESCE(SUM(x.amount_cents), 0) FROM payable_payments x WHERE x.payable_id = p.id) AS paid
                           FROM payables p) p
     JOIN expense_categories c ON c.id = p.category_id
     WHERE p.paid < p.amount_cents";

/// (quantidade, centavos) das contas em aberto vencidas (vencimento antes de `today`).
pub fn payables_overdue(conn: &Connection, today: &str) -> Result<(i64, i64), AppError> {
    Ok(conn.query_row(
        &format!("SELECT count(*), COALESCE(SUM(p.amount_cents - p.paid), 0) {OPEN} AND p.due_date < :today"),
        named_params! { ":today": today },
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?)
}

/// Saldo em aberto por categoria (só as que têm saldo).
pub fn payables_open_by_category(conn: &Connection) -> Result<Vec<CategoryOpen>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT c.name, SUM(p.amount_cents - p.paid) {OPEN} GROUP BY c.id ORDER BY c.name_key"
    ))?;
    let rows = stmt
        .query_map([], |r| Ok(CategoryOpen { category_name: r.get(0)?, open_cents: r.get(1)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
