use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::error::AppError;
use crate::models::rate::{Holiday, NewRateRule, RateRule};

const COLUMNS: &str =
    "id, category_id, rule_type, weekday, start_date, end_date, date, amount_cents, label, active, version";

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn map_rule(r: &Row) -> rusqlite::Result<RateRule> {
    Ok(RateRule {
        id: r.get(0)?,
        category_id: r.get(1)?,
        rule_type: r.get(2)?,
        weekday: r.get(3)?,
        start_date: r.get(4)?,
        end_date: r.get(5)?,
        date: r.get(6)?,
        amount_cents: r.get(7)?,
        label: r.get(8)?,
        active: r.get::<_, i64>(9)? == 1,
        version: r.get(10)?,
    })
}

pub fn insert_rule(tx: &Transaction, r: &NewRateRule, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO rate_rules (category_id, rule_type, weekday, start_date, end_date, date, amount_cents, label, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![r.category_id, r.rule_type, r.weekday, r.start_date, r.end_date, r.date, r.amount_cents, r.label, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find_rule(conn: &Connection, id: i64) -> Result<Option<RateRule>, AppError> {
    Ok(conn
        .query_row(&format!("SELECT {COLUMNS} FROM rate_rules WHERE id = ?1"), [id], map_rule)
        .optional()?)
}

pub fn update_rule(
    tx: &Transaction,
    id: i64,
    r: &NewRateRule,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    let changed = tx.execute(
        "UPDATE rate_rules SET category_id = ?1, rule_type = ?2, weekday = ?3, start_date = ?4, end_date = ?5,
                date = ?6, amount_cents = ?7, label = ?8, version = version + 1, updated_at = ?9
         WHERE id = ?10 AND version = ?11",
        params![r.category_id, r.rule_type, r.weekday, r.start_date, r.end_date, r.date, r.amount_cents, r.label, stamp(at), id, expected_version],
    )?;
    if changed == 0 {
        return Err(AppError::Conflict);
    }
    Ok(())
}

pub fn set_rule_active(tx: &Transaction, id: i64, active: bool, at: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute(
        "UPDATE rate_rules SET active = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3",
        params![active, stamp(at), id],
    )?;
    Ok(())
}

pub fn delete_rule(tx: &Transaction, id: i64) -> Result<(), AppError> {
    tx.execute("DELETE FROM rate_rules WHERE id = ?1", [id])?;
    Ok(())
}

pub fn list_rules(conn: &Connection, category_id: Option<i64>) -> Result<Vec<RateRule>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM rate_rules WHERE (?1 IS NULL OR category_id = ?1)
         ORDER BY category_id, CASE rule_type WHEN 'weekday' THEN 1 WHEN 'season' THEN 2 WHEN 'holiday' THEN 3 ELSE 4 END,
                  weekday, start_date, date, id"
    ))?;
    let rows = stmt.query_map([category_id], map_rule)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Regras **ativas** da categoria, para o motor de preço.
pub fn active_rules(conn: &Connection, category_id: i64) -> Result<Vec<RateRule>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM rate_rules WHERE category_id = ?1 AND active = 1 ORDER BY id"
    ))?;
    let rows = stmt.query_map([category_id], map_rule)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Outra regra ativa da mesma categoria que conflita com a candidata (mesmo dia da semana,
/// mesma data, feriado ou temporada sobreposta). `exclude_id` ignora a própria regra.
pub fn find_conflicting_active(
    conn: &Connection,
    r: &NewRateRule,
    exclude_id: Option<i64>,
) -> Result<Option<i64>, AppError> {
    let sql = match r.rule_type.as_str() {
        "weekday" => "SELECT id FROM rate_rules WHERE active = 1 AND category_id = ?1 AND rule_type = 'weekday' AND weekday = ?2 AND id <> ?3 LIMIT 1",
        "holiday" => "SELECT id FROM rate_rules WHERE active = 1 AND category_id = ?1 AND rule_type = 'holiday' AND ?2 IS NULL AND id <> ?3 LIMIT 1",
        "date_override" => "SELECT id FROM rate_rules WHERE active = 1 AND category_id = ?1 AND rule_type = 'date_override' AND date = ?2 AND id <> ?3 LIMIT 1",
        _ => "SELECT id FROM rate_rules WHERE active = 1 AND category_id = ?1 AND rule_type = 'season' AND start_date <= ?2 AND end_date >= ?4 AND id <> ?3 LIMIT 1",
    };
    let exclude = exclude_id.unwrap_or(-1);
    let found = match r.rule_type.as_str() {
        "weekday" => conn.query_row(sql, params![r.category_id, r.weekday, exclude], |row| row.get(0)).optional()?,
        "date_override" => conn.query_row(sql, params![r.category_id, r.date, exclude], |row| row.get(0)).optional()?,
        "holiday" => conn.query_row(sql, params![r.category_id, Option::<i64>::None, exclude], |row| row.get(0)).optional()?,
        _ => conn
            .query_row(sql, params![r.category_id, r.end_date, exclude, r.start_date], |row| row.get(0))
            .optional()?,
    };
    Ok(found)
}

// ---- feriados ----

fn map_holiday(r: &Row) -> rusqlite::Result<Holiday> {
    Ok(Holiday { id: r.get(0)?, date: r.get(1)?, name: r.get(2)? })
}

pub fn list_holidays(conn: &Connection, year: Option<i32>) -> Result<Vec<Holiday>, AppError> {
    let prefix = year.map(|y| format!("{y:04}-"));
    let mut stmt = conn.prepare(
        "SELECT id, date, name FROM holidays WHERE (?1 IS NULL OR date LIKE ?1 || '%') ORDER BY date",
    )?;
    let rows = stmt.query_map([prefix], map_holiday)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn find_holiday(conn: &Connection, id: i64) -> Result<Option<Holiday>, AppError> {
    Ok(conn
        .query_row("SELECT id, date, name FROM holidays WHERE id = ?1", [id], map_holiday)
        .optional()?)
}

pub fn insert_holiday(tx: &Transaction, date: &str, name: &str, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO holidays (date, name, created_at) VALUES (?1, ?2, ?3)",
        params![date, name, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn delete_holiday(tx: &Transaction, id: i64) -> Result<(), AppError> {
    tx.execute("DELETE FROM holidays WHERE id = ?1", [id])?;
    Ok(())
}
