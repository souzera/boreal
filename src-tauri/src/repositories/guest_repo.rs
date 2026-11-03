use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::domain::text::fold;
use crate::error::AppError;
use crate::models::guest::{DuplicateMatch, DuplicateReason, Guest, GuestQuery};
use crate::models::page::like_pattern;

const COLUMNS: &str = "id, code, full_name, phone, cpf, email, address, active, version";

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn map_guest(r: &Row) -> rusqlite::Result<Guest> {
    Ok(Guest {
        id: r.get(0)?,
        code: r.get(1)?,
        full_name: r.get(2)?,
        phone: r.get(3)?,
        cpf: r.get(4)?,
        email: r.get(5)?,
        address: r.get(6)?,
        active: r.get::<_, i64>(7)? == 1,
        version: r.get(8)?,
    })
}

/// Dados já validados e normalizados, prontos para gravar.
pub struct CleanGuest<'a> {
    pub full_name: &'a str,
    pub phone: &'a str,
    pub cpf: Option<&'a str>,
    pub email: &'a str,
    pub address: &'a str,
}

pub fn insert(tx: &Transaction, code: &str, g: &CleanGuest, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO guests (code, full_name, name_key, phone, cpf, email, address, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![code, g.full_name, fold(g.full_name), g.phone, g.cpf, g.email, g.address, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<Guest>, AppError> {
    Ok(conn
        .query_row(&format!("SELECT {COLUMNS} FROM guests WHERE id = ?1"), [id], map_guest)
        .optional()?)
}

pub fn update(
    tx: &Transaction,
    id: i64,
    g: &CleanGuest,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    let changed = tx.execute(
        "UPDATE guests SET full_name = ?1, name_key = ?2, phone = ?3, cpf = ?4, email = ?5, address = ?6,
                version = version + 1, updated_at = ?7
         WHERE id = ?8 AND version = ?9",
        params![g.full_name, fold(g.full_name), g.phone, g.cpf, g.email, g.address, stamp(at), id, expected_version],
    )?;
    if changed == 0 {
        return Err(AppError::Conflict);
    }
    Ok(())
}

pub fn set_active(tx: &Transaction, id: i64, active: bool, at: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute(
        "UPDATE guests SET active = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3",
        params![active, stamp(at), id],
    )?;
    Ok(())
}

/// Cadastros que coincidem em nome (sem acento/caixa), telefone (dígitos) ou CPF.
pub fn find_possible_duplicates(
    conn: &Connection,
    name_key: &str,
    phone: Option<&str>,
    cpf: Option<&str>,
) -> Result<Vec<DuplicateMatch>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS}, (name_key = ?1), (phone = ?2), (?3 IS NOT NULL AND cpf = ?3)
         FROM guests
         WHERE name_key = ?1 OR phone = ?2 OR (?3 IS NOT NULL AND cpf = ?3)
         ORDER BY id"
    ))?;
    let rows = stmt
        .query_map(params![name_key, phone, cpf], |r| {
            let guest = map_guest(r)?;
            let mut reasons = Vec::new();
            if r.get::<_, i64>(9)? == 1 {
                reasons.push(DuplicateReason::Name);
            }
            if r.get::<_, i64>(10)? == 1 {
                reasons.push(DuplicateReason::Phone);
            }
            if r.get::<_, i64>(11)? == 1 {
                reasons.push(DuplicateReason::Cpf);
            }
            Ok(DuplicateMatch { guest, reasons })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Colunas aceitas na ordenação: nunca se interpola texto vindo do cliente.
fn sort_column(sort: Option<&str>) -> &'static str {
    match sort {
        Some("code") => "code",
        Some("phone") => "phone",
        Some("active") => "active",
        _ => "name_key",
    }
}

pub fn list(conn: &Connection, q: &GuestQuery, limit: i64, offset: i64) -> Result<(Vec<Guest>, u64), AppError> {
    let text = q.text.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let name_pattern = text.map(|t| like_pattern(&fold(t)));
    let code_pattern = text.map(like_pattern);
    let digits: String = text.map(|t| t.chars().filter(char::is_ascii_digit).collect()).unwrap_or_default();
    let digits_pattern = (!digits.is_empty()).then(|| format!("%{digits}%"));

    let where_clause = "(?1 IS NULL OR name_key LIKE ?1 ESCAPE '\\' OR code LIKE ?2 ESCAPE '\\'
                          OR (?3 IS NOT NULL AND (phone LIKE ?3 OR cpf LIKE ?3)))
                        AND (?4 IS NULL OR active = ?4)";
    let total: i64 = conn.query_row(
        &format!("SELECT count(*) FROM guests WHERE {where_clause}"),
        params![name_pattern, code_pattern, digits_pattern, q.active],
        |r| r.get(0),
    )?;
    let direction = if q.direction.as_deref() == Some("desc") { "DESC" } else { "ASC" };
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM guests WHERE {where_clause} ORDER BY {} {direction}, id LIMIT ?5 OFFSET ?6",
        sort_column(q.sort.as_deref())
    ))?;
    let rows = stmt
        .query_map(params![name_pattern, code_pattern, digits_pattern, q.active, limit, offset], map_guest)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rows, total.max(0) as u64))
}
