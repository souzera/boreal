use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::error::AppError;
use crate::models::item::Item;

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn map_item(r: &Row) -> rusqlite::Result<Item> {
    Ok(Item {
        id: r.get(0)?,
        name: r.get(1)?,
        default_price_cents: r.get(2)?,
        active: r.get::<_, i64>(3)? == 1,
        version: r.get(4)?,
    })
}

const COLUMNS: &str = "id, name, default_price_cents, active, version";

pub fn list(conn: &Connection, active_only: bool) -> Result<Vec<Item>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM catalog_items WHERE (?1 = 0 OR active = 1) ORDER BY name_key"
    ))?;
    let rows = stmt.query_map([active_only], map_item)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<Item>, AppError> {
    Ok(conn
        .query_row(&format!("SELECT {COLUMNS} FROM catalog_items WHERE id = ?1"), [id], map_item)
        .optional()?)
}

pub fn insert(tx: &Transaction, name: &str, name_key: &str, cents: i64, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO catalog_items (name, name_key, default_price_cents, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![name, name_key, cents, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

fn expect_one(changed: usize) -> Result<(), AppError> {
    if changed == 0 {
        Err(AppError::Conflict)
    } else {
        Ok(())
    }
}

pub fn update(
    tx: &Transaction,
    id: i64,
    name: &str,
    name_key: &str,
    cents: i64,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE catalog_items SET name = ?1, name_key = ?2, default_price_cents = ?3, version = version + 1, updated_at = ?4
         WHERE id = ?5 AND version = ?6",
        params![name, name_key, cents, stamp(at), id, expected_version],
    )?)
}

pub fn set_active(tx: &Transaction, id: i64, active: bool, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE catalog_items SET active = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3 AND version = ?4",
        params![active, stamp(at), id, expected_version],
    )?)
}
