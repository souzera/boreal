use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::error::AppError;
use crate::models::catalog::{Category, Room, RoomQuery};
use crate::models::page::like_pattern;

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn conflict_if_unchanged(changed: usize) -> Result<(), AppError> {
    if changed == 0 {
        Err(AppError::Conflict)
    } else {
        Ok(())
    }
}

// ---- categorias ----

fn map_category(r: &Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        active: r.get::<_, i64>(3)? == 1,
        version: r.get(4)?,
    })
}

const CATEGORY_COLUMNS: &str = "id, name, description, active, version";

pub fn insert_category(tx: &Transaction, name: &str, description: &str, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO room_categories (name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![name, description, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find_category(conn: &Connection, id: i64) -> Result<Option<Category>, AppError> {
    Ok(conn
        .query_row(
            &format!("SELECT {CATEGORY_COLUMNS} FROM room_categories WHERE id = ?1"),
            [id],
            map_category,
        )
        .optional()?)
}

pub fn list_categories(conn: &Connection, active_only: bool) -> Result<Vec<Category>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {CATEGORY_COLUMNS} FROM room_categories WHERE (?1 = 0 OR active = 1) ORDER BY name COLLATE NOCASE"
    ))?;
    let rows = stmt
        .query_map([active_only], map_category)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn update_category(
    tx: &Transaction,
    id: i64,
    name: &str,
    description: &str,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    conflict_if_unchanged(tx.execute(
        "UPDATE room_categories SET name = ?1, description = ?2, version = version + 1, updated_at = ?3
         WHERE id = ?4 AND version = ?5",
        params![name, description, stamp(at), id, expected_version],
    )?)
}

pub fn set_category_active(tx: &Transaction, id: i64, active: bool, at: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute(
        "UPDATE room_categories SET active = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3",
        params![active, stamp(at), id],
    )?;
    Ok(())
}

pub fn delete_category(tx: &Transaction, id: i64) -> Result<(), AppError> {
    tx.execute("DELETE FROM room_categories WHERE id = ?1", [id])?;
    Ok(())
}

pub fn count_active_rooms_in_category(conn: &Connection, category_id: i64) -> Result<i64, AppError> {
    Ok(conn.query_row(
        "SELECT count(*) FROM rooms WHERE category_id = ?1 AND active = 1",
        [category_id],
        |r| r.get(0),
    )?)
}

// ---- quartos ----

const ROOM_SELECT: &str = "SELECT r.id, r.code, r.category_id, c.name, r.max_capacity, r.description, r.notes, r.active, r.version
     FROM rooms r JOIN room_categories c ON c.id = r.category_id";

fn map_room(r: &Row) -> rusqlite::Result<Room> {
    Ok(Room {
        id: r.get(0)?,
        code: r.get(1)?,
        category_id: r.get(2)?,
        category_name: r.get(3)?,
        max_capacity: r.get(4)?,
        description: r.get(5)?,
        notes: r.get(6)?,
        active: r.get::<_, i64>(7)? == 1,
        version: r.get(8)?,
    })
}

pub struct RoomFields<'a> {
    pub code: &'a str,
    pub category_id: i64,
    pub max_capacity: i64,
    pub description: &'a str,
    pub notes: &'a str,
}

pub fn insert_room(tx: &Transaction, f: &RoomFields, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO rooms (code, category_id, max_capacity, description, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![f.code, f.category_id, f.max_capacity, f.description, f.notes, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find_room(conn: &Connection, id: i64) -> Result<Option<Room>, AppError> {
    Ok(conn
        .query_row(&format!("{ROOM_SELECT} WHERE r.id = ?1"), [id], map_room)
        .optional()?)
}

pub fn update_room(
    tx: &Transaction,
    id: i64,
    f: &RoomFields,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    conflict_if_unchanged(tx.execute(
        "UPDATE rooms SET code = ?1, category_id = ?2, max_capacity = ?3, description = ?4, notes = ?5,
                version = version + 1, updated_at = ?6
         WHERE id = ?7 AND version = ?8",
        params![f.code, f.category_id, f.max_capacity, f.description, f.notes, stamp(at), id, expected_version],
    )?)
}

pub fn set_room_active(tx: &Transaction, id: i64, active: bool, at: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute(
        "UPDATE rooms SET active = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3",
        params![active, stamp(at), id],
    )?;
    Ok(())
}

pub fn delete_room(tx: &Transaction, id: i64) -> Result<(), AppError> {
    tx.execute("DELETE FROM rooms WHERE id = ?1", [id])?;
    Ok(())
}

/// Colunas aceitas na ordenação: nunca se interpola texto vindo do cliente.
fn room_sort_column(sort: Option<&str>) -> &'static str {
    match sort {
        Some("category_name") => "c.name COLLATE NOCASE",
        Some("max_capacity") => "r.max_capacity",
        Some("active") => "r.active",
        _ => "r.code COLLATE NOCASE",
    }
}

pub fn list_rooms(conn: &Connection, q: &RoomQuery, limit: i64, offset: i64) -> Result<(Vec<Room>, u64), AppError> {
    let pattern = q.text.as_deref().filter(|t| !t.trim().is_empty()).map(like_pattern);
    let where_clause = "(?1 IS NULL OR r.code LIKE ?1 ESCAPE '\\' OR r.description LIKE ?1 ESCAPE '\\')
                        AND (?2 IS NULL OR r.category_id = ?2) AND (?3 IS NULL OR r.active = ?3)";
    let total: i64 = conn.query_row(
        &format!("SELECT count(*) FROM rooms r JOIN room_categories c ON c.id = r.category_id WHERE {where_clause}"),
        params![pattern, q.category_id, q.active],
        |r| r.get(0),
    )?;
    let direction = if q.direction.as_deref() == Some("desc") { "DESC" } else { "ASC" };
    let mut stmt = conn.prepare(&format!(
        "{ROOM_SELECT} WHERE {where_clause} ORDER BY {} {direction}, r.id LIMIT ?4 OFFSET ?5",
        room_sort_column(q.sort.as_deref())
    ))?;
    let rows = stmt
        .query_map(params![pattern, q.category_id, q.active, limit, offset], map_room)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rows, total.max(0) as u64))
}
