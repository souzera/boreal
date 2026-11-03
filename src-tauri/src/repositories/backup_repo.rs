//! Histórico de backups e eventos do sistema (tabelas imutáveis).

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppError;

pub struct NewRun<'a> {
    pub kind: &'a str,
    pub status: &'a str,
    pub started_at: &'a str,
    pub finished_at: &'a str,
    pub file_name: Option<&'a str>,
    pub bytes: i64,
    pub schema_version: Option<i64>,
    pub message: &'a str,
}

pub struct RunRow {
    pub id: i64,
    pub kind: String,
    pub status: String,
    pub finished_at: String,
    pub file_name: Option<String>,
    pub bytes: i64,
    pub message: String,
}

pub fn insert_run(conn: &Connection, run: &NewRun) -> Result<i64, AppError> {
    conn.execute(
        "INSERT INTO backup_runs (kind, status, started_at, finished_at, file_name, bytes, schema_version, message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![run.kind, run.status, run.started_at, run.finished_at, run.file_name, run.bytes, run.schema_version, run.message],
    )?;
    Ok(conn.last_insert_rowid())
}

const SELECT: &str = "SELECT id, kind, status, finished_at, file_name, bytes, message FROM backup_runs";

fn map(r: &rusqlite::Row) -> rusqlite::Result<RunRow> {
    Ok(RunRow { id: r.get(0)?, kind: r.get(1)?, status: r.get(2)?, finished_at: r.get(3)?, file_name: r.get(4)?, bytes: r.get(5)?, message: r.get(6)? })
}

/// Execuções mais recentes primeiro.
pub fn recent_runs(conn: &Connection, limit: i64) -> Result<Vec<RunRow>, AppError> {
    let mut stmt = conn.prepare(&format!("{SELECT} ORDER BY finished_at DESC, id DESC LIMIT ?1"))?;
    let rows = stmt.query_map([limit], map)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn last_run(conn: &Connection) -> Result<Option<RunRow>, AppError> {
    Ok(conn.query_row(&format!("{SELECT} ORDER BY finished_at DESC, id DESC LIMIT 1"), [], map).optional()?)
}

/// Instante (UTC) do último backup válido.
pub fn last_valid_at(conn: &Connection) -> Result<Option<String>, AppError> {
    Ok(conn.query_row("SELECT MAX(finished_at) FROM backup_runs WHERE status = 'ok'", [], |r| r.get(0))?)
}

pub fn insert_event(conn: &Connection, kind: &str, at: &str, message: &str) -> Result<(), AppError> {
    conn.execute("INSERT INTO system_events (kind, at, message) VALUES (?1, ?2, ?3)", params![kind, at, message])?;
    Ok(())
}

/// Instante e mensagem do evento mais recente de um tipo.
pub fn latest_event(conn: &Connection, kind: &str) -> Result<Option<(String, String)>, AppError> {
    Ok(conn
        .query_row("SELECT at, message FROM system_events WHERE kind = ?1 ORDER BY at DESC, id DESC LIMIT 1", [kind], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?)
}
