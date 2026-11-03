//! Consultas dos relatórios (somente leitura). Cada função devolve linhas já como células; a ordem é a do relatório.

use rusqlite::{named_params, Connection};

use crate::domain::report::{br_date, Cell};
use crate::domain::text::fold;
use crate::error::AppError;
use crate::models::page::like_pattern;
use crate::repositories::stay_repo;

pub type Rows = Vec<Vec<Cell>>;

/// Limites e filtros comuns. `from`/`to` são datas de calendário; `start`/`end` são os limites UTC dos mesmos dias.
pub struct Scope<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub start: &'a str,
    pub end: &'a str,
    pub offset_secs: i32,
    pub status: Option<&'a str>,
    pub room_id: Option<i64>,
    pub text: Option<&'a str>,
    pub method: Option<&'a str>,
    pub category_id: Option<i64>,
    /// Hoje (data local), para a situação das contas.
    pub today: &'a str,
    /// Máximo de linhas; a consulta busca uma a mais para saber se truncou.
    pub limit: usize,
}

impl Scope<'_> {
    pub(crate) fn shift(&self) -> String {
        format!("{:+} seconds", self.offset_secs)
    }

    pub(crate) fn code_pattern(&self) -> Option<String> {
        self.text.map(str::trim).filter(|t| !t.is_empty()).map(like_pattern)
    }

    pub(crate) fn name_pattern(&self) -> Option<String> {
        self.text.map(str::trim).filter(|t| !t.is_empty()).map(|t| like_pattern(&fold(t)))
    }

    pub(crate) fn fetch(&self) -> i64 {
        i64::try_from(self.limit).unwrap_or(i64::MAX - 1) + 1
    }
}

fn date_cell(iso: &str) -> Cell {
    Cell::Text(br_date(iso))
}

pub const RESERVATION_STATUS_LABEL: [(&str, &str); 6] = [
    ("pending", "Pendente"),
    ("confirmed", "Confirmada"),
    ("checked_in", "Hospedado"),
    ("completed", "Concluída"),
    ("cancelled", "Cancelada"),
    ("no_show", "No-show"),
];

pub fn reservation_status_label(status: &str) -> String {
    RESERVATION_STATUS_LABEL.iter().find(|(k, _)| *k == status).map_or(status, |(_, v)| v).to_string()
}

/// Ocupação: uma linha por dia de `from` a `to`. Devolve também (quartos, soma das noites ocupadas) para a taxa média.
pub fn occupancy(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    let rooms: i64 = conn.query_row(
        "SELECT count(*) FROM rooms WHERE active = 1 AND (:room IS NULL OR id = :room)",
        named_params! { ":room": scope.room_id },
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT n.date, count(*) FROM reservation_nights n JOIN reservations r ON r.id = n.reservation_id
         WHERE r.status IN ('pending', 'confirmed', 'checked_in', 'completed') AND n.date BETWEEN :from AND :to
           AND (:room IS NULL OR r.room_id = :room)
         GROUP BY n.date",
    )?;
    let nights: std::collections::BTreeMap<String, i64> = stmt
        .query_map(named_params! { ":from": scope.from, ":to": scope.to, ":room": scope.room_id }, |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;

    let mut rows = Vec::new();
    let mut date = crate::domain::dates::parse(scope.from).ok_or_else(|| AppError::Validation("Data inicial inválida.".into()))?;
    let last = crate::domain::dates::parse(scope.to).ok_or_else(|| AppError::Validation("Data final inválida.".into()))?;
    while date <= last && rows.len() <= scope.limit {
        let iso = crate::domain::dates::format(date);
        let occupied = nights.get(&iso).copied().unwrap_or(0);
        let rate_bp = if rooms == 0 { 0 } else { (occupied * 20_000 + rooms) / (2 * rooms) };
        rows.push(vec![date_cell(&iso), Cell::Int(rooms), Cell::Int(occupied), Cell::Int((rooms - occupied).max(0)), Cell::Percent(rate_bp)]);
        date = date.succ_opt().ok_or_else(|| AppError::Validation("Período inválido.".into()))?;
    }
    Ok(rows)
}

/// Reservas com chegada no período.
pub fn reservations(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    let mut stmt = conn.prepare(
        "SELECT r.code, g.full_name, rm.code, r.check_in_date, r.check_out_date,
                CAST(julianday(r.check_out_date) - julianday(r.check_in_date) AS INTEGER), r.status, r.agreed_total_cents
         FROM reservations r JOIN rooms rm ON rm.id = r.room_id JOIN guests g ON g.id = r.guest_id
         WHERE r.check_in_date BETWEEN :from AND :to
           AND (:status IS NULL OR r.status = :status)
           AND (:room IS NULL OR r.room_id = :room)
           AND (:code IS NULL OR r.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\')
         ORDER BY r.check_in_date, r.id LIMIT :limit",
    )?;
    let rows = stmt
        .query_map(
            named_params! {
                ":from": scope.from, ":to": scope.to, ":status": scope.status, ":room": scope.room_id,
                ":code": scope.code_pattern(), ":name": scope.name_pattern(), ":limit": scope.fetch()
            },
            |r| {
                Ok(vec![
                    Cell::Text(r.get(0)?),
                    Cell::Text(r.get(1)?),
                    Cell::Text(r.get(2)?),
                    date_cell(&r.get::<_, String>(3)?),
                    date_cell(&r.get::<_, String>(4)?),
                    Cell::Int(r.get(5)?),
                    Cell::Text(reservation_status_label(&r.get::<_, String>(6)?)),
                    Cell::Money(r.get(7)?),
                ])
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Hospedagens com check-in no período (data local do check-in).
pub fn stays(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.code, g.full_name, rm.code, date(s.checked_in_at, :shift), r.check_out_date, s.status
         FROM stays s JOIN reservations r ON r.id = s.reservation_id
              JOIN rooms rm ON rm.id = r.room_id JOIN guests g ON g.id = r.guest_id
         WHERE s.checked_in_at >= :start AND s.checked_in_at < :end
           AND (:status IS NULL OR s.status = :status)
           AND (:room IS NULL OR r.room_id = :room)
           AND (:code IS NULL OR s.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\' OR rm.code LIKE :code ESCAPE '\\')
         ORDER BY s.checked_in_at, s.id LIMIT :limit",
    )?;
    let raw = stmt
        .query_map(
            named_params! {
                ":shift": scope.shift(), ":start": scope.start, ":end": scope.end, ":status": scope.status, ":room": scope.room_id,
                ":code": scope.code_pattern(), ":name": scope.name_pattern(), ":limit": scope.fetch()
            },
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                ))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    raw.into_iter()
        .map(|(id, code, guest, room, entered, leaves, status)| {
            Ok(vec![
                Cell::Text(code),
                Cell::Text(guest),
                Cell::Text(room),
                date_cell(&entered),
                date_cell(&leaves),
                Cell::text(if status == "in_house" { "Hospedado" } else { "Check-out feito" }),
                Cell::Money(stay_repo::balance(conn, id)?),
            ])
        })
        .collect()
}

/// Cancelamentos e no-shows com a data (local) do cancelamento ou da marcação no período.
pub fn cancellations(conn: &Connection, scope: &Scope) -> Result<Rows, AppError> {
    let mut stmt = conn.prepare(
        "SELECT r.code, g.full_name, rm.code, r.status, date(r.cancelled_at, :shift),
                COALESCE(r.cancellation_fee_cents, 0), r.cancellation_refund_cents
         FROM reservations r JOIN rooms rm ON rm.id = r.room_id JOIN guests g ON g.id = r.guest_id
         WHERE r.status IN ('cancelled', 'no_show') AND r.cancelled_at >= :start AND r.cancelled_at < :end
           AND (:status IS NULL OR r.status = :status)
           AND (:room IS NULL OR r.room_id = :room)
           AND (:code IS NULL OR r.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\')
         ORDER BY r.cancelled_at, r.id LIMIT :limit",
    )?;
    let rows = stmt
        .query_map(
            named_params! {
                ":shift": scope.shift(), ":start": scope.start, ":end": scope.end, ":status": scope.status, ":room": scope.room_id,
                ":code": scope.code_pattern(), ":name": scope.name_pattern(), ":limit": scope.fetch()
            },
            |r| {
                Ok(vec![
                    Cell::Text(r.get(0)?),
                    Cell::Text(r.get(1)?),
                    Cell::Text(r.get(2)?),
                    Cell::text(if r.get::<_, String>(3)? == "no_show" { "No-show" } else { "Cancelamento" }),
                    date_cell(&r.get::<_, String>(4)?),
                    Cell::Money(r.get(5)?),
                    Cell::Money(r.get(6)?),
                ])
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
