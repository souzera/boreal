//! Documentos emitidos e as consultas auxiliares para montar o instantâneo de cada tipo.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::error::AppError;

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub struct DocumentRow {
    pub id: i64,
    pub kind: String,
    pub source_id: i64,
    pub version: i64,
    pub snapshot: String,
    pub issued_at: String,
    pub issued_by_name: String,
}

const SELECT: &str = "SELECT d.id, d.kind, d.source_id, d.version, d.snapshot, d.issued_at, u.display_name
                      FROM documents d JOIN users u ON u.id = d.issued_by";

fn map(r: &rusqlite::Row) -> rusqlite::Result<DocumentRow> {
    Ok(DocumentRow { id: r.get(0)?, kind: r.get(1)?, source_id: r.get(2)?, version: r.get(3)?, snapshot: r.get(4)?, issued_at: r.get(5)?, issued_by_name: r.get(6)? })
}

/// A versão mais nova de um documento, se já foi emitido.
pub fn latest(conn: &Connection, kind: &str, source_id: i64) -> Result<Option<DocumentRow>, AppError> {
    Ok(conn
        .query_row(&format!("{SELECT} WHERE d.kind = ?1 AND d.source_id = ?2 ORDER BY d.version DESC LIMIT 1"), params![kind, source_id], map)
        .optional()?)
}

/// Todas as versões, da mais nova para a mais antiga.
pub fn list(conn: &Connection, kind: &str, source_id: i64) -> Result<Vec<DocumentRow>, AppError> {
    let mut stmt = conn.prepare(&format!("{SELECT} WHERE d.kind = ?1 AND d.source_id = ?2 ORDER BY d.version DESC"))?;
    let rows = stmt.query_map(params![kind, source_id], map)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<DocumentRow>, AppError> {
    Ok(conn.query_row(&format!("{SELECT} WHERE d.id = ?1"), [id], map).optional()?)
}

pub fn insert(tx: &Transaction, kind: &str, source_id: i64, version: i64, snapshot: &str, at: DateTime<Utc>, issued_by: i64) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO documents (kind, source_id, version, snapshot, issued_at, issued_by) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![kind, source_id, version, snapshot, stamp(at), issued_by],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn category_name(conn: &Connection, room_id: i64) -> Result<String, AppError> {
    Ok(conn
        .query_row("SELECT c.name FROM rooms r JOIN room_categories c ON c.id = r.category_id WHERE r.id = ?1", [room_id], |r| r.get(0))
        .optional()?
        .unwrap_or_default())
}

/// Dados de um recebimento para o comprovante.
pub struct PaymentDoc {
    pub code: String,
    pub kind: String,
    pub received_at: String,
    pub received_by_name: String,
    pub guest_name: String,
    pub reservation_code: String,
    pub stay_code: Option<String>,
    pub room_code: String,
    pub amount_cents: i64,
}

pub fn payment(conn: &Connection, payment_id: i64) -> Result<Option<PaymentDoc>, AppError> {
    Ok(conn
        .query_row(
            "SELECT p.code, p.kind, p.received_at, u.display_name, g.full_name, r.code, s.code, rm.code, p.amount_cents
             FROM payments p JOIN users u ON u.id = p.received_by JOIN reservations r ON r.id = p.reservation_id
                  JOIN guests g ON g.id = r.guest_id JOIN rooms rm ON rm.id = r.room_id LEFT JOIN stays s ON s.id = p.stay_id
             WHERE p.id = ?1",
            [payment_id],
            |r| {
                Ok(PaymentDoc {
                    code: r.get(0)?,
                    kind: r.get(1)?,
                    received_at: r.get(2)?,
                    received_by_name: r.get(3)?,
                    guest_name: r.get(4)?,
                    reservation_code: r.get(5)?,
                    stay_code: r.get(6)?,
                    room_code: r.get(7)?,
                    amount_cents: r.get(8)?,
                })
            },
        )
        .optional()?)
}
