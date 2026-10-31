use std::collections::BTreeSet;

use chrono::{DateTime, Duration, FixedOffset, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::domain::payment::{is_possible_duplicate, Component, Method, PaymentFacts};
use crate::error::AppError;
use crate::models::payment::{ComponentLine, DuplicateInfo, PaymentSummary};

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Liberação de um possível duplicado, gravada junto do recebimento.
pub struct DuplicateRef<'a> {
    pub of_payment_id: i64,
    pub justification: &'a str,
    pub released_by: i64,
}

pub struct NewPayment<'a> {
    pub code: &'a str,
    pub reservation_id: i64,
    pub stay_id: Option<i64>,
    /// `deposit` (sinal da reserva) ou `stay` (conta da hospedagem).
    pub kind: &'a str,
    pub amount_cents: i64,
    pub received_by: i64,
    pub note: &'a str,
    pub duplicate: Option<DuplicateRef<'a>>,
}

pub fn insert(tx: &Transaction, p: &NewPayment, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO payments (code, reservation_id, stay_id, kind, amount_cents, received_at, received_by, note,
            duplicate_of_payment_id, duplicate_justification, duplicate_released_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            p.code,
            p.reservation_id,
            p.stay_id,
            p.kind,
            p.amount_cents,
            stamp(at),
            p.received_by,
            p.note,
            p.duplicate.as_ref().map(|d| d.of_payment_id),
            p.duplicate.as_ref().map(|d| d.justification),
            p.duplicate.as_ref().map(|d| d.released_by),
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn insert_components(tx: &Transaction, payment_id: i64, components: &[Component]) -> Result<(), AppError> {
    for c in components {
        tx.execute(
            "INSERT INTO payment_components (payment_id, method, amount_cents) VALUES (?1, ?2, ?3)",
            params![payment_id, c.method.as_str(), c.amount_cents],
        )?;
    }
    Ok(())
}

/// Componentes de um recebimento, em ordem de modalidade.
pub fn components(conn: &Connection, payment_id: i64) -> Result<Vec<ComponentLine>, AppError> {
    let mut stmt =
        conn.prepare("SELECT method, amount_cents FROM payment_components WHERE payment_id = ?1 ORDER BY method")?;
    let rows = stmt
        .query_map([payment_id], |r| Ok(ComponentLine { method: r.get(0)?, amount_cents: r.get(1)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub struct SimilarPayment {
    pub id: i64,
    pub info: DuplicateInfo,
}

fn local_date(received_at: &str, offset: FixedOffset) -> Option<chrono::NaiveDate> {
    let at = DateTime::parse_from_rfc3339(received_at).ok()?.with_timezone(&Utc);
    Some((at + Duration::seconds(i64::from(offset.local_minus_utc()))).date_naive())
}

/// Recebimento da mesma reserva (e, portanto, da hospedagem dela) que seria duplicado de `candidate` (M5-6).
pub fn find_similar(
    conn: &Connection,
    reservation_id: i64,
    candidate: &PaymentFacts,
    offset: FixedOffset,
) -> Result<Option<SimilarPayment>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, code, received_at FROM payments WHERE reservation_id = ?1 AND amount_cents = ?2 ORDER BY id DESC",
    )?;
    let rows = stmt
        .query_map(params![reservation_id, candidate.total_cents], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, code, received_at) in rows {
        let lines = components(conn, id)?;
        let methods: BTreeSet<Method> = lines.iter().filter_map(|l| Method::parse(&l.method)).collect();
        let Some(date) = local_date(&received_at, offset) else { continue };
        let existing = PaymentFacts { total_cents: candidate.total_cents, methods, local_date: date };
        if is_possible_duplicate(candidate, &existing) {
            let methods = lines.into_iter().map(|l| l.method).collect();
            return Ok(Some(SimilarPayment {
                id,
                info: DuplicateInfo { code, received_at, total_cents: candidate.total_cents, methods },
            }));
        }
    }
    Ok(None)
}

/// Vincula à hospedagem os recebimentos da reserva ainda sem hospedagem (o sinal). Devolve quantos.
pub fn link_to_stay(tx: &Transaction, reservation_id: i64, stay_id: i64) -> Result<usize, AppError> {
    let linked = tx.execute(
        "UPDATE payments SET stay_id = ?1 WHERE reservation_id = ?2 AND stay_id IS NULL",
        params![stay_id, reservation_id],
    )?;
    // devoluções feitas antes do check-in acompanham o recebimento, senão sumiriam da conta
    tx.execute("UPDATE refunds SET stay_id = ?1 WHERE reservation_id = ?2 AND stay_id IS NULL", params![stay_id, reservation_id])?;
    Ok(linked)
}

pub fn released_duplicate(conn: &Connection, payment_id: i64) -> Result<bool, AppError> {
    Ok(conn.query_row(
        "SELECT duplicate_of_payment_id IS NOT NULL FROM payments WHERE id = ?1",
        [payment_id],
        |r| r.get::<_, i64>(0),
    )? == 1)
}

pub struct PaymentRow {
    pub id: i64,
    pub code: String,
    pub reservation_id: i64,
    pub stay_id: Option<i64>,
    pub amount_cents: i64,
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<PaymentRow>, AppError> {
    Ok(conn
        .query_row(
            "SELECT id, code, reservation_id, stay_id, amount_cents FROM payments WHERE id = ?1",
            [id],
            |r| Ok(PaymentRow { id: r.get(0)?, code: r.get(1)?, reservation_id: r.get(2)?, stay_id: r.get(3)?, amount_cents: r.get(4)? }),
        )
        .optional()?)
}

/// Recebimentos de uma reserva, do mais antigo ao mais novo, com devolvido e devolvível.
pub fn summaries_for_reservation(conn: &Connection, reservation_id: i64) -> Result<Vec<PaymentSummary>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.code, p.amount_cents, p.received_at, p.stay_id,
                (SELECT COALESCE(SUM(f.amount_cents), 0) FROM refunds f WHERE f.payment_id = p.id)
         FROM payments p WHERE p.reservation_id = ?1 ORDER BY p.id",
    )?;
    let rows = stmt
        .query_map([reservation_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, String>(3)?, r.get::<_, Option<i64>>(4)?, r.get::<_, i64>(5)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, code, amount_cents, received_at, stay_id, refunded_cents)| {
            Ok(PaymentSummary {
                id,
                code,
                amount_cents,
                refunded_cents,
                refundable_cents: (amount_cents - refunded_cents).max(0),
                components: components(conn, id)?,
                received_at,
                stay_id,
            })
        })
        .collect()
}
