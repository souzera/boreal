use chrono::{DateTime, Utc};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row, Transaction};

use crate::domain::alert::Finding;
use crate::error::AppError;
use crate::models::alert::AlertEventView;

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub struct AlertRow {
    pub id: i64,
    pub kind: String,
    pub subject_key: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub link: String,
    pub status: String,
    pub still_present: bool,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub resolved_at: Option<String>,
    pub resolved_by_name: Option<String>,
    pub resolution_note: Option<String>,
    pub version: i64,
}

const SELECT: &str = "SELECT a.id, a.kind, a.subject_key, a.severity, a.title, a.detail, a.link, a.status, a.still_present,
        a.first_seen_at, a.last_seen_at, a.resolved_at, u.display_name, a.resolution_note, a.version
     FROM alerts a LEFT JOIN users u ON u.id = a.resolved_by";

const SEVERITY_ORDER: &str = "CASE a.severity WHEN 'critical' THEN 0 WHEN 'high' THEN 1 ELSE 2 END";

fn map_row(r: &Row) -> rusqlite::Result<AlertRow> {
    Ok(AlertRow {
        id: r.get(0)?,
        kind: r.get(1)?,
        subject_key: r.get(2)?,
        severity: r.get(3)?,
        title: r.get(4)?,
        detail: r.get(5)?,
        link: r.get(6)?,
        status: r.get(7)?,
        still_present: r.get::<_, i64>(8)? == 1,
        first_seen_at: r.get(9)?,
        last_seen_at: r.get(10)?,
        resolved_at: r.get(11)?,
        resolved_by_name: r.get(12)?,
        resolution_note: r.get(13)?,
        version: r.get(14)?,
    })
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(",")
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<AlertRow>, AppError> {
    Ok(conn.query_row(&format!("{SELECT} WHERE a.id = ?1"), [id], map_row).optional()?)
}

/// Alertas que ainda "valem": ativos e dispensados cuja causa continua.
pub fn open_rows(conn: &Connection) -> Result<Vec<AlertRow>, AppError> {
    let mut stmt = conn.prepare(&format!("{SELECT} WHERE a.status = 'active' OR (a.status = 'dismissed' AND a.still_present = 1)"))?;
    let rows = stmt.query_map([], map_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn insert(tx: &Transaction, f: &Finding, severity: &str, now: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO alerts (kind, subject_key, severity, title, detail, link, status, first_seen_at, last_seen_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?7)",
        params![f.kind.as_str(), f.subject_key, severity, f.title, f.detail, f.link, stamp(now)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn touch(tx: &Transaction, id: i64, now: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute("UPDATE alerts SET last_seen_at = ?1 WHERE id = ?2", params![stamp(now), id])?;
    Ok(())
}

pub fn update_text(tx: &Transaction, id: i64, f: &Finding, severity: &str, now: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute(
        "UPDATE alerts SET title = ?1, detail = ?2, link = ?3, severity = ?4, last_seen_at = ?5, version = version + 1 WHERE id = ?6",
        params![f.title, f.detail, f.link, severity, stamp(now), id],
    )?;
    Ok(())
}

pub fn resolve(tx: &Transaction, id: i64, now: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute(
        "UPDATE alerts SET status = 'resolved', resolved_at = ?1, version = version + 1 WHERE id = ?2 AND status = 'active'",
        params![stamp(now), id],
    )?;
    Ok(())
}

pub fn clear_presence(tx: &Transaction, id: i64) -> Result<(), AppError> {
    tx.execute("UPDATE alerts SET still_present = 0 WHERE id = ?1", [id])?;
    Ok(())
}

pub fn dismiss(tx: &Transaction, id: i64, user_id: i64, note: &str, now: DateTime<Utc>, expected_version: i64) -> Result<(), AppError> {
    let changed = tx.execute(
        "UPDATE alerts SET status = 'dismissed', still_present = 1, resolved_at = ?1, resolved_by = ?2, resolution_note = ?3,
                version = version + 1
         WHERE id = ?4 AND version = ?5 AND status = 'active'",
        params![stamp(now), user_id, note, id, expected_version],
    )?;
    if changed == 0 {
        return Err(AppError::Conflict);
    }
    Ok(())
}

pub fn insert_event(tx: &Transaction, alert_id: i64, at: DateTime<Utc>, user_id: Option<i64>, event: &str, note: Option<&str>) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO alert_events (alert_id, at, user_id, event, note) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![alert_id, stamp(at), user_id, event, note],
    )?;
    Ok(())
}

pub struct Filters<'a> {
    pub statuses: &'a [&'a str],
    pub kind: Option<&'a str>,
    pub severity: Option<&'a str>,
    /// Tipos que a pessoa pode ver (os demais nunca aparecem).
    pub allowed_kinds: &'a [&'a str],
}

pub fn list(conn: &Connection, f: &Filters, limit: i64, offset: i64) -> Result<(Vec<AlertRow>, u64), AppError> {
    use rusqlite::types::Value;
    let where_clause = format!(
        "a.status IN ({}) AND a.kind IN ({}) AND (? IS NULL OR a.kind = ?) AND (? IS NULL OR a.severity = ?)",
        placeholders(f.statuses.len()),
        placeholders(f.allowed_kinds.len())
    );
    let optional = |v: Option<&str>| v.map_or(Value::Null, |s| Value::Text(s.to_string()));
    // Ordem dos marcadores: situações, tipos permitidos, tipo (duas vezes), prioridade (duas vezes).
    let filter_params = || -> Vec<Value> {
        let mut all: Vec<Value> = f.statuses.iter().map(|s| Value::Text((*s).to_string())).collect();
        all.extend(f.allowed_kinds.iter().map(|s| Value::Text((*s).to_string())));
        all.extend([optional(f.kind), optional(f.kind), optional(f.severity), optional(f.severity)]);
        all
    };
    let total: i64 = conn.query_row(
        &format!("SELECT count(*) FROM alerts a WHERE {where_clause}"),
        params_from_iter(filter_params()),
        |r| r.get(0),
    )?;
    let order = if f.statuses.contains(&"active") { format!("{SEVERITY_ORDER}, a.first_seen_at, a.id") } else { "a.resolved_at DESC, a.id DESC".to_string() };
    let mut stmt = conn.prepare(&format!("{SELECT} WHERE {where_clause} ORDER BY {order} LIMIT ? OFFSET ?"))?;
    let mut all = filter_params();
    all.extend([Value::Integer(limit), Value::Integer(offset)]);
    let rows = stmt.query_map(params_from_iter(all), map_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok((rows, total.max(0) as u64))
}

/// Contagem de alertas ativos por prioridade, só dos tipos permitidos.
pub fn counts(conn: &Connection, allowed_kinds: &[&str]) -> Result<(i64, i64, i64), AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT a.severity, count(*) FROM alerts a WHERE a.status = 'active' AND a.kind IN ({}) GROUP BY a.severity",
        placeholders(allowed_kinds.len())
    ))?;
    let rows = stmt
        .query_map(params_from_iter(allowed_kinds.iter()), |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    let get = |s: &str| rows.iter().find(|(k, _)| k == s).map_or(0, |(_, n)| *n);
    Ok((get("critical"), get("high"), get("normal")))
}

pub fn top(conn: &Connection, allowed_kinds: &[&str], n: i64) -> Result<Vec<AlertRow>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT} WHERE a.status = 'active' AND a.kind IN ({}) ORDER BY {SEVERITY_ORDER}, a.first_seen_at, a.id LIMIT {n}",
        placeholders(allowed_kinds.len())
    ))?;
    let rows = stmt.query_map(params_from_iter(allowed_kinds.iter()), map_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn events(conn: &Connection, alert_id: i64) -> Result<Vec<AlertEventView>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.at, u.display_name, e.event, e.note FROM alert_events e LEFT JOIN users u ON u.id = e.user_id
         WHERE e.alert_id = ?1 ORDER BY e.id",
    )?;
    let rows = stmt
        .query_map([alert_id], |r| Ok(AlertEventView { id: r.get(0)?, at: r.get(1)?, user_name: r.get(2)?, event: r.get(3)?, note: r.get(4)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
