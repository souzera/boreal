use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, Connection, OptionalExtension, Row, Transaction};

use crate::error::AppError;
use crate::models::payable::{CategoryTotal, ExpenseCategory, PayablePaymentLine};

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn expect_one(changed: usize) -> Result<(), AppError> {
    if changed == 0 {
        Err(AppError::Conflict)
    } else {
        Ok(())
    }
}

// ---- categorias ----

fn map_category(r: &Row) -> rusqlite::Result<ExpenseCategory> {
    Ok(ExpenseCategory { id: r.get(0)?, name: r.get(1)?, active: r.get::<_, i64>(2)? == 1, version: r.get(3)? })
}

pub fn list_categories(conn: &Connection, active_only: bool) -> Result<Vec<ExpenseCategory>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, active, version FROM expense_categories WHERE (?1 = 0 OR active = 1) ORDER BY name_key",
    )?;
    let rows = stmt.query_map([active_only], map_category)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn find_category(conn: &Connection, id: i64) -> Result<Option<ExpenseCategory>, AppError> {
    Ok(conn
        .query_row("SELECT id, name, active, version FROM expense_categories WHERE id = ?1", [id], map_category)
        .optional()?)
}

pub fn insert_category(tx: &Transaction, name: &str, key: &str, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO expense_categories (name, name_key, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![name, key, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn rename_category(tx: &Transaction, id: i64, name: &str, key: &str, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE expense_categories SET name = ?1, name_key = ?2, version = version + 1, updated_at = ?3 WHERE id = ?4 AND version = ?5",
        params![name, key, stamp(at), id, expected_version],
    )?)
}

pub fn set_category_active(tx: &Transaction, id: i64, active: bool, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE expense_categories SET active = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3 AND version = ?4",
        params![active, stamp(at), id, expected_version],
    )?)
}

// ---- contas a pagar ----

pub struct PayableRow {
    pub id: i64,
    pub code: String,
    pub supplier: String,
    pub description: String,
    pub amount_cents: i64,
    pub paid_cents: i64,
    pub competence_date: String,
    pub due_date: String,
    pub category_id: i64,
    pub category_name: String,
    pub notes: String,
    pub version: i64,
    pub created_at: String,
}

const FROM: &str = "FROM (SELECT p.*, (SELECT COALESCE(SUM(x.amount_cents), 0) FROM payable_payments x WHERE x.payable_id = p.id) AS paid
                          FROM payables p) p
     JOIN expense_categories c ON c.id = p.category_id";

const COLUMNS: &str = "SELECT p.id, p.code, p.supplier, p.description, p.amount_cents, p.paid, p.competence_date, p.due_date,
        p.category_id, c.name, p.notes, p.version, p.created_at";

const STATUS: &str = "CASE WHEN p.paid >= p.amount_cents THEN 'paid' WHEN p.due_date < :today THEN 'overdue'
                          WHEN p.paid > 0 THEN 'partial' ELSE 'pending' END";

fn map_row(r: &Row) -> rusqlite::Result<PayableRow> {
    Ok(PayableRow {
        id: r.get(0)?,
        code: r.get(1)?,
        supplier: r.get(2)?,
        description: r.get(3)?,
        amount_cents: r.get(4)?,
        paid_cents: r.get(5)?,
        competence_date: r.get(6)?,
        due_date: r.get(7)?,
        category_id: r.get(8)?,
        category_name: r.get(9)?,
        notes: r.get(10)?,
        version: r.get(11)?,
        created_at: r.get(12)?,
    })
}

pub struct NewPayableRow<'a> {
    pub code: &'a str,
    pub supplier: &'a str,
    pub description: &'a str,
    pub amount_cents: i64,
    pub competence_date: &'a str,
    pub due_date: &'a str,
    pub category_id: i64,
    pub notes: &'a str,
    pub created_by: i64,
}

pub fn insert(tx: &Transaction, p: &NewPayableRow, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO payables (code, supplier, description, amount_cents, competence_date, due_date, category_id, notes,
            created_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![p.code, p.supplier, p.description, p.amount_cents, p.competence_date, p.due_date, p.category_id, p.notes, p.created_by, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<PayableRow>, AppError> {
    Ok(conn.query_row(&format!("{COLUMNS} {FROM} WHERE p.id = ?1"), [id], map_row).optional()?)
}

pub struct Filters<'a> {
    pub status: Option<&'a str>,
    pub category_id: Option<i64>,
    pub due_from: Option<&'a str>,
    pub due_to: Option<&'a str>,
}

pub fn list(conn: &Connection, f: &Filters, today: &str, limit: i64, offset: i64) -> Result<(Vec<PayableRow>, u64), AppError> {
    let where_clause = format!(
        "({STATUS} = :status OR :status IS NULL) AND (p.category_id = :category OR :category IS NULL)
         AND (p.due_date >= :from OR :from IS NULL) AND (p.due_date <= :to OR :to IS NULL)"
    );
    let total: i64 = conn.query_row(
        &format!("SELECT count(*) {FROM} WHERE {where_clause}"),
        named_params! { ":today": today, ":status": f.status, ":category": f.category_id, ":from": f.due_from, ":to": f.due_to },
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "{COLUMNS} {FROM} WHERE {where_clause} ORDER BY p.due_date, p.id LIMIT :limit OFFSET :offset"
    ))?;
    let rows = stmt
        .query_map(
            named_params! {
                ":today": today, ":status": f.status, ":category": f.category_id, ":from": f.due_from, ":to": f.due_to,
                ":limit": limit, ":offset": offset
            },
            map_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rows, total.max(0) as u64))
}

pub fn bump(tx: &Transaction, id: i64, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE payables SET version = version + 1, updated_at = ?1 WHERE id = ?2 AND version = ?3",
        params![stamp(at), id, expected_version],
    )?)
}

pub fn insert_payment(
    tx: &Transaction,
    payable_id: i64,
    amount_cents: i64,
    method: &str,
    paid_by: i64,
    note: &str,
    at: DateTime<Utc>,
) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO payable_payments (payable_id, amount_cents, method, paid_at, paid_by, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![payable_id, amount_cents, method, stamp(at), paid_by, note],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn payments(conn: &Connection, payable_id: i64) -> Result<Vec<PayablePaymentLine>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT x.id, x.amount_cents, x.method, x.paid_at, u.display_name, x.note
         FROM payable_payments x JOIN users u ON u.id = x.paid_by WHERE x.payable_id = ?1 ORDER BY x.id",
    )?;
    let rows = stmt
        .query_map([payable_id], |r| {
            Ok(PayablePaymentLine {
                id: r.get(0)?,
                amount_cents: r.get(1)?,
                method: r.get(2)?,
                paid_at: r.get(3)?,
                paid_by_name: r.get(4)?,
                note: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Totais por categoria das despesas com competência em `from..=to` (datas de calendário).
pub fn summary_by_competence(conn: &Connection, from: &str, to: &str, category_id: Option<i64>) -> Result<Vec<CategoryTotal>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, SUM(p.amount_cents)
         FROM payables p JOIN expense_categories c ON c.id = p.category_id
         WHERE p.competence_date BETWEEN ?1 AND ?2 AND (?3 IS NULL OR p.category_id = ?3)
         GROUP BY c.id ORDER BY c.name_key",
    )?;
    let rows = stmt
        .query_map(params![from, to, category_id], |r| {
            Ok(CategoryTotal { category_id: r.get(0)?, category_name: r.get(1)?, total_cents: r.get(2)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Totais por categoria dos pagamentos feitos em `[start_utc, end_utc)`, cada um pelo seu valor e instante.
pub fn summary_by_payment(conn: &Connection, start_utc: &str, end_utc: &str, category_id: Option<i64>) -> Result<Vec<CategoryTotal>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, SUM(x.amount_cents)
         FROM payable_payments x JOIN payables p ON p.id = x.payable_id JOIN expense_categories c ON c.id = p.category_id
         WHERE x.paid_at >= ?1 AND x.paid_at < ?2 AND (?3 IS NULL OR p.category_id = ?3)
         GROUP BY c.id ORDER BY c.name_key",
    )?;
    let rows = stmt
        .query_map(params![start_utc, end_utc, category_id], |r| {
            Ok(CategoryTotal { category_id: r.get(0)?, category_name: r.get(1)?, total_cents: r.get(2)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
