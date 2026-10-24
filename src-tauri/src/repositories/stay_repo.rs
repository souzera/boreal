use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, Connection, OptionalExtension, Row, Transaction};

use crate::domain::account;
use crate::error::AppError;
use crate::models::page::like_pattern;
use crate::models::stay::{ChargeLine, Occupant, PaymentLine, RefundLine, Stay, StayAccount};

pub fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn expect_one(changed: usize) -> Result<(), AppError> {
    if changed == 0 {
        Err(AppError::Conflict)
    } else {
        Ok(())
    }
}

const SELECT: &str = "SELECT s.id, s.code, s.reservation_id, s.status, r.room_id, rm.code, r.guest_id, g.full_name,
        r.check_in_date, r.check_out_date, r.expected_check_in_time, r.expected_check_out_time,
        s.checked_in_at, s.checked_out_at, s.early_check_in, s.late_tolerance_minutes, s.version
     FROM stays s
     JOIN reservations r ON r.id = s.reservation_id
     JOIN rooms rm ON rm.id = r.room_id
     JOIN guests g ON g.id = r.guest_id";

fn map_stay(r: &Row) -> rusqlite::Result<Stay> {
    Ok(Stay {
        id: r.get(0)?,
        code: r.get(1)?,
        reservation_id: r.get(2)?,
        status: r.get(3)?,
        room_id: r.get(4)?,
        room_code: r.get(5)?,
        guest_id: r.get(6)?,
        guest_name: r.get(7)?,
        check_in_date: r.get(8)?,
        check_out_date: r.get(9)?,
        expected_check_in_time: r.get(10)?,
        expected_check_out_time: r.get(11)?,
        checked_in_at: r.get(12)?,
        checked_out_at: r.get(13)?,
        early_check_in: r.get::<_, i64>(14)? == 1,
        late_tolerance_minutes: r.get(15)?,
        version: r.get(16)?,
    })
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<Stay>, AppError> {
    Ok(conn.query_row(&format!("{SELECT} WHERE s.id = ?1"), [id], map_stay).optional()?)
}

#[allow(clippy::too_many_arguments)] // espelha as colunas gravadas no check-in
pub fn insert_stay(
    tx: &Transaction,
    reservation_id: i64,
    code: &str,
    checked_in_by: i64,
    early_check_in: bool,
    exceeded_capacity: bool,
    at: DateTime<Utc>,
) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO stays (reservation_id, code, status, checked_in_at, checked_in_by, early_check_in, exceeded_capacity,
            created_at, updated_at)
         VALUES (?1, ?2, 'in_house', ?3, ?4, ?5, ?6, ?3, ?3)",
        params![reservation_id, code, stamp(at), checked_in_by, early_check_in, exceeded_capacity],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn insert_occupant(
    tx: &Transaction,
    stay_id: i64,
    full_name: &str,
    guest_id: Option<i64>,
    document: &str,
    is_holder: bool,
) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO stay_occupants (stay_id, full_name, guest_id, document, is_holder) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![stay_id, full_name, guest_id, document, is_holder],
    )?;
    Ok(())
}

pub fn occupants(conn: &Connection, stay_id: i64) -> Result<Vec<Occupant>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, full_name, guest_id, document, is_holder FROM stay_occupants WHERE stay_id = ?1 ORDER BY is_holder DESC, id",
    )?;
    let rows = stmt
        .query_map([stay_id], |r| {
            Ok(Occupant {
                id: r.get(0)?,
                full_name: r.get(1)?,
                guest_id: r.get(2)?,
                document: r.get(3)?,
                is_holder: r.get::<_, i64>(4)? == 1,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub struct NewCharge<'a> {
    pub stay_id: i64,
    pub kind: &'a str,
    pub description: &'a str,
    pub amount_cents: i64,
    pub item_id: Option<i64>,
    pub night_date: Option<&'a str>,
    pub reverses_charge_id: Option<i64>,
    pub replaces_charge_id: Option<i64>,
    pub justification: Option<&'a str>,
    pub created_by: i64,
}

pub fn insert_charge(tx: &Transaction, c: &NewCharge, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO charges (stay_id, kind, description, amount_cents, item_id, night_date, reverses_charge_id,
            replaces_charge_id, justification, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            c.stay_id, c.kind, c.description, c.amount_cents, c.item_id, c.night_date, c.reverses_charge_id,
            c.replaces_charge_id, c.justification, c.created_by, stamp(at)
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)] // espelha as colunas gravadas no recebimento
pub fn insert_payment(
    tx: &Transaction,
    code: &str,
    reservation_id: i64,
    stay_id: i64,
    amount_cents: i64,
    method: &str,
    received_by: i64,
    note: &str,
    at: DateTime<Utc>,
) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO payments (code, reservation_id, stay_id, amount_cents, method, received_at, received_by, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![code, reservation_id, stay_id, amount_cents, method, stamp(at), received_by, note],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find_charge(conn: &Connection, stay_id: i64, charge_id: i64) -> Result<Option<(String, String, i64)>, AppError> {
    Ok(conn
        .query_row(
            "SELECT kind, description, amount_cents FROM charges WHERE id = ?1 AND stay_id = ?2",
            params![charge_id, stay_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?)
}

pub fn is_reversed(conn: &Connection, charge_id: i64) -> Result<bool, AppError> {
    Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM charges WHERE reverses_charge_id = ?1)", [charge_id], |r| r.get::<_, i64>(0))? == 1)
}

/// Marca que a conta mudou (quem tinha uma prévia de check-out precisa recarregar).
pub fn bump(tx: &Transaction, stay_id: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute("UPDATE stays SET version = version + 1, updated_at = ?1 WHERE id = ?2", params![stamp(at), stay_id])?;
    Ok(())
}

/// Valor da última diária que não foi estornada (base da cobrança de saída tardia).
pub fn last_night_cents(conn: &Connection, stay_id: i64) -> Result<i64, AppError> {
    Ok(conn
        .query_row(
            "SELECT amount_cents FROM charges c
             WHERE c.stay_id = ?1 AND c.kind = 'night' AND NOT EXISTS (SELECT 1 FROM charges x WHERE x.reverses_charge_id = c.id)
             ORDER BY c.night_date DESC LIMIT 1",
            [stay_id],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0))
}

pub fn set_tolerance(
    tx: &Transaction,
    stay_id: i64,
    minutes: Option<i64>,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE stays SET late_tolerance_minutes = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3 AND version = ?4",
        params![minutes, stamp(at), stay_id, expected_version],
    )?)
}

pub struct Checkout<'a> {
    pub by: i64,
    pub late_minutes: i64,
    pub late_fee_calculated_cents: i64,
    /// `charged`, `waived` ou `not_applicable`.
    pub late_fee_decision: &'a str,
    pub waive_justification: Option<&'a str>,
    pub waived_by: Option<i64>,
    pub balance_cents: Option<i64>,
    pub balance_justification: Option<&'a str>,
    pub balance_authorized_by: Option<i64>,
}

/// Encerra a hospedagem se a versão ainda for a consultada.
pub fn finish(tx: &Transaction, stay_id: i64, c: &Checkout, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE stays SET status = 'checked_out', checked_out_at = ?1, checked_out_by = ?2, late_minutes = ?3,
                late_fee_calculated_cents = ?4, late_fee_decision = ?5, late_fee_waive_justification = ?6,
                late_fee_waived_by = ?7, balance_at_checkout_cents = ?8, balance_justification = ?9,
                balance_authorized_by = ?10, version = version + 1, updated_at = ?1
         WHERE id = ?11 AND version = ?12",
        params![
            stamp(at), c.by, c.late_minutes, c.late_fee_calculated_cents, c.late_fee_decision, c.waive_justification,
            c.waived_by, c.balance_cents, c.balance_justification, c.balance_authorized_by, stay_id, expected_version
        ],
    )?)
}

pub fn touch(tx: &Transaction, stay_id: i64, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE stays SET version = version + 1, updated_at = ?1 WHERE id = ?2 AND version = ?3",
        params![stamp(at), stay_id, expected_version],
    )?)
}

fn sum(conn: &Connection, sql: &str, stay_id: i64) -> Result<i64, AppError> {
    Ok(conn.query_row(sql, [stay_id], |r| r.get(0))?)
}

/// Saldo da hospedagem: lançamentos − recebido + devolvido.
pub fn balance(conn: &Connection, stay_id: i64) -> Result<i64, AppError> {
    account::balance(
        sum(conn, "SELECT COALESCE(SUM(amount_cents), 0) FROM charges WHERE stay_id = ?1", stay_id)?,
        sum(conn, "SELECT COALESCE(SUM(amount_cents), 0) FROM payments WHERE stay_id = ?1", stay_id)?,
        sum(conn, "SELECT COALESCE(SUM(amount_cents), 0) FROM refunds WHERE stay_id = ?1", stay_id)?,
    )
}

pub fn account(conn: &Connection, stay_id: i64) -> Result<StayAccount, AppError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.kind, c.description, c.amount_cents, c.night_date, c.justification, c.reverses_charge_id,
                EXISTS(SELECT 1 FROM charges x WHERE x.reverses_charge_id = c.id), u.display_name, c.created_at
         FROM charges c JOIN users u ON u.id = c.created_by
         WHERE c.stay_id = ?1 ORDER BY c.id",
    )?;
    let charges = stmt
        .query_map([stay_id], |r| {
            Ok(ChargeLine {
                id: r.get(0)?,
                kind: r.get(1)?,
                description: r.get(2)?,
                amount_cents: r.get(3)?,
                night_date: r.get(4)?,
                justification: r.get(5)?,
                reverses_charge_id: r.get(6)?,
                reversed: r.get::<_, i64>(7)? == 1,
                created_by_name: r.get(8)?,
                created_at: r.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT p.id, p.code, p.amount_cents, p.method, p.received_at, u.display_name
         FROM payments p JOIN users u ON u.id = p.received_by WHERE p.stay_id = ?1 ORDER BY p.id",
    )?;
    let payments = stmt
        .query_map([stay_id], |r| {
            Ok(PaymentLine {
                id: r.get(0)?,
                code: r.get(1)?,
                amount_cents: r.get(2)?,
                method: r.get(3)?,
                received_at: r.get(4)?,
                received_by_name: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT f.id, f.code, p.code, f.amount_cents, f.reason, f.refunded_at
         FROM refunds f JOIN payments p ON p.id = f.payment_id WHERE f.stay_id = ?1 ORDER BY f.id",
    )?;
    let refunds = stmt
        .query_map([stay_id], |r| {
            Ok(RefundLine {
                id: r.get(0)?,
                code: r.get(1)?,
                payment_code: r.get(2)?,
                amount_cents: r.get(3)?,
                reason: r.get(4)?,
                refunded_at: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let charges_total = sum(conn, "SELECT COALESCE(SUM(amount_cents), 0) FROM charges WHERE stay_id = ?1", stay_id)?;
    let received = sum(conn, "SELECT COALESCE(SUM(amount_cents), 0) FROM payments WHERE stay_id = ?1", stay_id)?;
    let refunded = sum(conn, "SELECT COALESCE(SUM(amount_cents), 0) FROM refunds WHERE stay_id = ?1", stay_id)?;
    Ok(StayAccount {
        balance_cents: account::balance(charges_total, received, refunded)?,
        charges,
        payments,
        refunds,
        charges_total_cents: charges_total,
        received_cents: received,
        refunded_cents: refunded,
    })
}

/// Hospedagens por situação (`in_house` ou `checked_out`), com busca por código, hóspede ou quarto.
pub fn list(conn: &Connection, text: Option<&str>, status: &str, limit: i64, offset: i64) -> Result<(Vec<Stay>, u64), AppError> {
    let text = text.map(str::trim).filter(|t| !t.is_empty());
    let code_pattern = text.map(like_pattern);
    let name_pattern = text.map(|t| like_pattern(&crate::domain::text::fold(t)));
    let where_clause = "s.status = :status
         AND (:code IS NULL OR s.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\' OR rm.code LIKE :code ESCAPE '\\')";
    let total: i64 = conn.query_row(
        &format!(
            "SELECT count(*) FROM stays s JOIN reservations r ON r.id = s.reservation_id
             JOIN rooms rm ON rm.id = r.room_id JOIN guests g ON g.id = r.guest_id WHERE {where_clause}"
        ),
        named_params! { ":status": status, ":code": code_pattern, ":name": name_pattern },
        |r| r.get(0),
    )?;
    let order = if status == "in_house" {
        "r.check_out_date, r.expected_check_out_time, s.id"
    } else {
        "s.checked_out_at DESC, s.id DESC"
    };
    let mut stmt = conn.prepare(&format!("{SELECT} WHERE {where_clause} ORDER BY {order} LIMIT :limit OFFSET :offset"))?;
    let rows = stmt
        .query_map(
            named_params! { ":status": status, ":code": code_pattern, ":name": name_pattern, ":limit": limit, ":offset": offset },
            map_stay,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rows, total.max(0) as u64))
}
