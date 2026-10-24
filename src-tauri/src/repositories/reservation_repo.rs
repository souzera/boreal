use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rusqlite::{named_params, params, Connection, OptionalExtension, Row, Transaction};

use crate::domain::availability::Booking;
use crate::domain::dates;
use crate::error::AppError;
use crate::models::page::like_pattern;
use crate::models::reservation::{ConflictInfo, Reservation, ReservationChange, ReservationNight, ReservationQuery};

/// Estados que bloqueiam o quarto (spec §4.1: reserva pendente também bloqueia).
pub const BLOCKING_STATUSES: [&str; 3] = ["pending", "confirmed", "checked_in"];

pub const ALL_STATUSES: [&str; 6] = ["pending", "confirmed", "checked_in", "completed", "cancelled", "no_show"];

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

const SELECT: &str = "SELECT r.id, r.code, r.room_id, rm.code, r.guest_id, g.full_name, r.check_in_date, r.check_out_date,
        r.expected_check_in_time, r.expected_check_out_time,
        CAST(julianday(r.check_out_date) - julianday(r.check_in_date) AS INTEGER),
        r.occupants, r.status, r.list_total_cents, r.discount_cents, r.discount_reason, r.agreed_total_cents,
        r.deposit_expected_cents, r.deposit_paid_cents, r.deposit_due_date,
        (r.status = 'pending' AND r.deposit_paid_cents < r.deposit_expected_cents AND r.deposit_due_date < :today),
        r.notes, r.cancelled_at, r.cancellation_fee_cents, r.version, r.created_at
     FROM reservations r
     JOIN rooms rm ON rm.id = r.room_id
     JOIN guests g ON g.id = r.guest_id";

fn map_reservation(r: &Row) -> rusqlite::Result<Reservation> {
    Ok(Reservation {
        id: r.get(0)?,
        code: r.get(1)?,
        room_id: r.get(2)?,
        room_code: r.get(3)?,
        guest_id: r.get(4)?,
        guest_name: r.get(5)?,
        check_in_date: r.get(6)?,
        check_out_date: r.get(7)?,
        expected_check_in_time: r.get(8)?,
        expected_check_out_time: r.get(9)?,
        nights: r.get(10)?,
        occupants: r.get(11)?,
        status: r.get(12)?,
        list_total_cents: r.get(13)?,
        discount_cents: r.get(14)?,
        discount_reason: r.get(15)?,
        agreed_total_cents: r.get(16)?,
        deposit_expected_cents: r.get(17)?,
        deposit_paid_cents: r.get(18)?,
        deposit_due_date: r.get(19)?,
        deposit_overdue: r.get::<_, i64>(20)? == 1,
        notes: r.get(21)?,
        cancelled_at: r.get(22)?,
        cancellation_fee_cents: r.get(23)?,
        version: r.get(24)?,
        created_at: r.get(25)?,
    })
}

pub struct NewRow<'a> {
    pub code: &'a str,
    pub room_id: i64,
    pub guest_id: i64,
    pub check_in: &'a str,
    pub check_out: &'a str,
    pub in_time: &'a str,
    pub out_time: &'a str,
    pub occupants: u32,
    pub list_total_cents: i64,
    pub agreed_total_cents: i64,
    pub deposit_expected_cents: i64,
    pub deposit_due_date: &'a str,
    pub notes: &'a str,
    pub created_by: i64,
}

pub fn insert(tx: &Transaction, r: &NewRow, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO reservations (code, room_id, guest_id, check_in_date, check_out_date, expected_check_in_time,
            expected_check_out_time, occupants, status, list_total_cents, agreed_total_cents, deposit_expected_cents,
            deposit_due_date, notes, created_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            r.code, r.room_id, r.guest_id, r.check_in, r.check_out, r.in_time, r.out_time, r.occupants,
            r.list_total_cents, r.agreed_total_cents, r.deposit_expected_cents, r.deposit_due_date, r.notes,
            r.created_by, stamp(at)
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn insert_night(tx: &Transaction, reservation_id: i64, date: &str, cents: i64, rule: &str) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO reservation_nights (reservation_id, date, amount_cents, rule) VALUES (?1, ?2, ?3, ?4)",
        params![reservation_id, date, cents, rule],
    )?;
    Ok(())
}

pub fn find(conn: &Connection, id: i64, today: NaiveDate) -> Result<Option<Reservation>, AppError> {
    Ok(conn
        .query_row(
            &format!("{SELECT} WHERE r.id = :id"),
            named_params! { ":today": dates::format(today), ":id": id },
            map_reservation,
        )
        .optional()?)
}

pub fn nights(conn: &Connection, reservation_id: i64) -> Result<Vec<ReservationNight>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT date, amount_cents, rule FROM reservation_nights WHERE reservation_id = ?1 ORDER BY date",
    )?;
    let rows = stmt
        .query_map([reservation_id], |r| {
            Ok(ReservationNight { date: r.get(0)?, amount_cents: r.get(1)?, rule: r.get(2)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn changes(conn: &Connection, reservation_id: i64) -> Result<Vec<ReservationChange>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.changed_at, c.changed_by, u.display_name, c.kind, c.before_json, c.after_json,
                c.price_before_cents, c.price_after_cents, c.justification
         FROM reservation_changes c JOIN users u ON u.id = c.changed_by
         WHERE c.reservation_id = ?1 ORDER BY c.id",
    )?;
    let rows = stmt
        .query_map([reservation_id], |r| {
            Ok(ReservationChange {
                id: r.get(0)?,
                changed_at: r.get(1)?,
                changed_by: r.get(2)?,
                changed_by_name: r.get(3)?,
                kind: r.get(4)?,
                before_json: r.get(5)?,
                after_json: r.get(6)?,
                price_before_cents: r.get(7)?,
                price_after_cents: r.get(8)?,
                justification: r.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Colunas aceitas na ordenação: nunca se interpola texto vindo do cliente.
fn sort_column(sort: Option<&str>) -> &'static str {
    match sort {
        Some("code") => "r.code",
        Some("guest_name") => "g.name_key",
        Some("room_code") => "rm.code COLLATE NOCASE",
        Some("status") => "r.status",
        Some("agreed_total_cents") => "r.agreed_total_cents",
        Some("created_at") => "r.created_at",
        _ => "r.check_in_date",
    }
}

pub fn list(
    conn: &Connection,
    q: &ReservationQuery,
    today: NaiveDate,
    limit: i64,
    offset: i64,
) -> Result<(Vec<Reservation>, u64), AppError> {
    let text = q.text.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let code_pattern = text.map(like_pattern);
    let name_pattern = text.map(|t| like_pattern(&crate::domain::text::fold(t)));

    // Só valores da lista fixa entram na consulta (nunca texto livre).
    let statuses: Vec<&str> = q
        .status
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|s| ALL_STATUSES.iter().copied().find(|known| known == s))
        .collect();
    let status_clause = if statuses.is_empty() {
        String::new()
    } else {
        format!(" AND r.status IN ({})", statuses.iter().map(|s| format!("'{s}'")).collect::<Vec<_>>().join(","))
    };
    let where_clause = format!(
        "(:code IS NULL OR r.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\')
         AND (:room IS NULL OR r.room_id = :room)
         AND (:from IS NULL OR r.check_out_date > :from)
         AND (:to IS NULL OR r.check_in_date <= :to){status_clause}"
    );
    let from = q.from.as_deref().and_then(dates::parse).map(dates::format);
    let to = q.to.as_deref().and_then(dates::parse).map(dates::format);
    let today = dates::format(today);

    let total: i64 = conn.query_row(
        &format!(
            "SELECT count(*) FROM reservations r JOIN rooms rm ON rm.id = r.room_id JOIN guests g ON g.id = r.guest_id WHERE {where_clause}"
        ),
        named_params! { ":code": code_pattern, ":name": name_pattern, ":room": q.room_id, ":from": from, ":to": to },
        |r| r.get(0),
    )?;
    let direction = if q.direction.as_deref() == Some("desc") { "DESC" } else { "ASC" };
    let mut stmt = conn.prepare(&format!(
        "{SELECT} WHERE {where_clause} ORDER BY {} {direction}, r.id LIMIT :limit OFFSET :offset",
        sort_column(q.sort.as_deref())
    ))?;
    let rows = stmt
        .query_map(
            named_params! {
                ":today": today, ":code": code_pattern, ":name": name_pattern, ":room": q.room_id,
                ":from": from, ":to": to, ":limit": limit, ":offset": offset
            },
            map_reservation,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rows, total.max(0) as u64))
}

/// Reservas que bloqueiam o quarto e tocam o período `from..=to` (com folga de um dia para a troca no mesmo dia).
pub fn blocking(
    conn: &Connection,
    room_id: i64,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(Booking, ConflictInfo)>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.code, g.full_name, r.status, r.check_in_date, r.check_out_date,
                r.expected_check_in_time, r.expected_check_out_time
         FROM reservations r JOIN guests g ON g.id = r.guest_id
         WHERE r.room_id = ?1 AND r.status IN ('pending', 'confirmed', 'checked_in')
           AND r.check_out_date >= ?2 AND r.check_in_date <= ?3
         ORDER BY r.check_in_date, r.id",
    )?;
    let raw = stmt
        .query_map(
            params![room_id, dates::format(from - chrono::Duration::days(1)), dates::format(to + chrono::Duration::days(1))],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                ))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;

    let mut out = Vec::new();
    for (id, code, guest_name, status, check_in, check_out, in_time, out_time) in raw {
        let (Some(ci), Some(co), Ok(ti), Ok(to_)) = (
            dates::parse(&check_in),
            dates::parse(&check_out),
            NaiveTime::parse_from_str(&in_time, "%H:%M"),
            NaiveTime::parse_from_str(&out_time, "%H:%M"),
        ) else {
            return Err(AppError::Database(format!("reserva {code} com datas ou horários inválidos")));
        };
        out.push((
            Booking { id, check_in: ci, check_out: co, expected_in: ti, expected_out: to_ },
            ConflictInfo { reservation_id: id, code, guest_name, check_in_date: check_in, check_out_date: check_out, status },
        ));
    }
    Ok(out)
}

pub struct ChangedFields<'a> {
    pub room_id: i64,
    pub check_in: &'a str,
    pub check_out: &'a str,
    pub list_total_cents: i64,
    pub agreed_total_cents: i64,
    pub deposit_expected_cents: i64,
}

/// Aplica a alteração se a versão ainda for a consultada; senão, `Conflict`.
pub fn apply_change(
    tx: &Transaction,
    id: i64,
    f: &ChangedFields,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    let changed = tx.execute(
        "UPDATE reservations SET room_id = ?1, check_in_date = ?2, check_out_date = ?3, list_total_cents = ?4,
                agreed_total_cents = ?5, deposit_expected_cents = ?6, version = version + 1, updated_at = ?7
         WHERE id = ?8 AND version = ?9",
        params![f.room_id, f.check_in, f.check_out, f.list_total_cents, f.agreed_total_cents, f.deposit_expected_cents, stamp(at), id, expected_version],
    )?;
    if changed == 0 {
        return Err(AppError::Conflict);
    }
    Ok(())
}

pub fn replace_nights(tx: &Transaction, reservation_id: i64, nights: &[(String, i64, String)]) -> Result<(), AppError> {
    tx.execute("DELETE FROM reservation_nights WHERE reservation_id = ?1", [reservation_id])?;
    for (date, cents, rule) in nights {
        insert_night(tx, reservation_id, date, *cents, rule)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // espelha as colunas do histórico
pub fn insert_change(
    tx: &Transaction,
    reservation_id: i64,
    at: DateTime<Utc>,
    changed_by: i64,
    kind: &str,
    before_json: &str,
    after_json: &str,
    price_before: Option<i64>,
    price_after: Option<i64>,
    justification: Option<&str>,
) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO reservation_changes (reservation_id, changed_at, changed_by, kind, before_json, after_json,
            price_before_cents, price_after_cents, justification)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![reservation_id, stamp(at), changed_by, kind, before_json, after_json, price_before, price_after, justification],
    )?;
    Ok(())
}

/// Encerra a reserva (`cancelled` ou `no_show`) registrando o valor retido.
pub fn set_closed(
    tx: &Transaction,
    id: i64,
    status: &str,
    retained_cents: i64,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    let changed = tx.execute(
        "UPDATE reservations SET status = ?1, cancelled_at = ?2, cancellation_fee_cents = ?3,
                version = version + 1, updated_at = ?2
         WHERE id = ?4 AND version = ?5",
        params![status, stamp(at), retained_cents, id, expected_version],
    )?;
    if changed == 0 {
        return Err(AppError::Conflict);
    }
    Ok(())
}

/// Reservas pendentes/confirmadas cuja chegada é hoje ou já passou, ou cujo sinal está vencido.
pub fn review_candidates(conn: &Connection, today: NaiveDate) -> Result<Vec<Reservation>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT} WHERE r.status IN ('pending', 'confirmed')
           AND (r.check_in_date <= :today
                OR (r.deposit_paid_cents < r.deposit_expected_cents AND r.deposit_due_date < :today))
         ORDER BY r.check_in_date, r.id"
    ))?;
    let rows = stmt
        .query_map(named_params! { ":today": dates::format(today) }, map_reservation)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
