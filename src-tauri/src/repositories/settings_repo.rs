use rusqlite::{Connection, OptionalExtension};

use crate::error::AppError;

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

pub fn get_u32(conn: &Connection, key: &str) -> Result<u32, AppError> {
    get(conn, key)?
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| AppError::Database(format!("configuração inválida: {key}")))
}
