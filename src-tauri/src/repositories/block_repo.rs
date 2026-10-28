use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::domain::dates;
use crate::error::AppError;
use crate::models::block::RoomBlock;

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn expect_one(changed: usize) -> Result<(), AppError> {
    if changed == 0 {
        Err(AppError::Conflict)
    } else {
        Ok(())
    }
}

const SELECT: &str = "SELECT b.id, b.room_id, rm.code, b.kind, b.status, b.start_date, b.end_date, b.reason,
        b.maintenance_order_id, b.released_at, b.release_justification, b.version
     FROM room_blocks b JOIN rooms rm ON rm.id = b.room_id";

fn map(r: &Row) -> rusqlite::Result<RoomBlock> {
    Ok(RoomBlock {
        id: r.get(0)?,
        room_id: r.get(1)?,
        room_code: r.get(2)?,
        kind: r.get(3)?,
        status: r.get(4)?,
        start_date: r.get(5)?,
        end_date: r.get(6)?,
        reason: r.get(7)?,
        maintenance_order_id: r.get(8)?,
        released_at: r.get(9)?,
        release_justification: r.get(10)?,
        version: r.get(11)?,
    })
}

pub struct NewBlock<'a> {
    pub room_id: i64,
    pub kind: &'a str,
    pub status: &'a str,
    pub start_date: &'a str,
    pub end_date: Option<&'a str>,
    pub reason: &'a str,
    pub maintenance_order_id: Option<i64>,
    pub created_by: i64,
}

pub fn insert(tx: &Transaction, b: &NewBlock, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO room_blocks (room_id, kind, status, start_date, end_date, reason, maintenance_order_id, created_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![b.room_id, b.kind, b.status, b.start_date, b.end_date, b.reason, b.maintenance_order_id, b.created_by, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<RoomBlock>, AppError> {
    Ok(conn.query_row(&format!("{SELECT} WHERE b.id = ?1"), [id], map).optional()?)
}

/// Bloqueio ligado a uma ordem de manutenção, se houver (o mais recente não liberado).
pub fn for_order(conn: &Connection, order_id: i64) -> Result<Option<RoomBlock>, AppError> {
    Ok(conn
        .query_row(
            &format!("{SELECT} WHERE b.maintenance_order_id = ?1 AND b.status <> 'released' ORDER BY b.id DESC LIMIT 1"),
            [order_id],
            map,
        )
        .optional()?)
}

pub fn list(conn: &Connection, include_released: bool) -> Result<Vec<RoomBlock>, AppError> {
    let filter = if include_released { "" } else { "WHERE b.status <> 'released'" };
    let mut stmt = conn.prepare(&format!("{SELECT} {filter} ORDER BY b.start_date DESC, b.id DESC"))?;
    let rows = stmt.query_map([], map)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Bloqueios não liberados do quarto que tocam as noites `[start, end)` (`end` ausente = sem fim).
/// Inclui os preventivos: eles impedem novas reservas.
pub fn overlapping(conn: &Connection, room_id: i64, start: NaiveDate, end: Option<NaiveDate>) -> Result<Vec<RoomBlock>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT} WHERE b.room_id = ?1 AND b.status <> 'released'
           AND (b.end_date IS NULL OR b.end_date > ?2)
           AND (?3 IS NULL OR b.start_date < ?3)
         ORDER BY b.start_date, b.id"
    ))?;
    let rows = stmt
        .query_map(params![room_id, dates::format(start), end.map(dates::format)], map)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// O quarto está indisponível hoje por um bloqueio efetivo (ativo ou aguardando liberação)?
pub fn active_now(conn: &Connection, room_id: i64, today: NaiveDate) -> Result<bool, AppError> {
    let today = dates::format(today);
    let n: i64 = conn.query_row(
        "SELECT count(*) FROM room_blocks
         WHERE room_id = ?1 AND status IN ('active', 'awaiting_release')
           AND start_date <= ?2 AND (end_date IS NULL OR end_date > ?2)",
        params![room_id, today],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

pub fn set_status(tx: &Transaction, id: i64, status: &str, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE room_blocks SET status = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3 AND version = ?4",
        params![status, stamp(at), id, expected_version],
    )?)
}

pub fn release(
    tx: &Transaction,
    id: i64,
    user: i64,
    justification: &str,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE room_blocks SET status = 'released', released_at = ?1, released_by = ?2, release_justification = ?3,
                version = version + 1, updated_at = ?1
         WHERE id = ?4 AND version = ?5 AND status <> 'released'",
        params![stamp(at), user, justification, id, expected_version],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::test_db::db;
    use chrono::TimeZone;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 1, 12, 0, 0).unwrap()
    }

    fn add(conn: &mut Connection, status: &str, from: &str, to: Option<&str>) -> i64 {
        let tx = conn.transaction().unwrap();
        let id = insert(
            &tx,
            &NewBlock { room_id: 1, kind: "manual", status, start_date: from, end_date: to, reason: "pintura", maintenance_order_id: None, created_by: 1 },
            at(),
        )
        .unwrap();
        tx.commit().unwrap();
        id
    }

    #[test]
    fn overlapping_uses_half_open_periods() {
        let mut conn = db();
        add(&mut conn, "active", "2026-11-10", Some("2026-11-13"));

        assert!(overlapping(&conn, 1, d("2026-11-13"), Some(d("2026-11-15"))).unwrap().is_empty());
        assert!(overlapping(&conn, 1, d("2026-11-08"), Some(d("2026-11-10"))).unwrap().is_empty());
        assert_eq!(overlapping(&conn, 1, d("2026-11-12"), Some(d("2026-11-14"))).unwrap().len(), 1);
        assert!(overlapping(&conn, 2, d("2026-11-12"), Some(d("2026-11-14"))).unwrap().is_empty());
    }

    #[test]
    fn open_ended_block_touches_every_later_date() {
        let mut conn = db();
        add(&mut conn, "active", "2026-11-10", None);

        assert_eq!(overlapping(&conn, 1, d("2027-06-01"), Some(d("2027-06-03"))).unwrap().len(), 1);
        assert!(overlapping(&conn, 1, d("2026-11-01"), Some(d("2026-11-10"))).unwrap().is_empty());
        assert_eq!(overlapping(&conn, 1, d("2026-11-01"), None).unwrap().len(), 1);
    }

    #[test]
    fn released_blocks_are_ignored() {
        let mut conn = db();
        let id = add(&mut conn, "active", "2026-11-10", Some("2026-11-13"));
        let tx = conn.transaction().unwrap();
        release(&tx, id, 1, "obra terminou", 1, at()).unwrap();
        tx.commit().unwrap();

        assert!(overlapping(&conn, 1, d("2026-11-10"), Some(d("2026-11-13"))).unwrap().is_empty());
        assert!(!active_now(&conn, 1, d("2026-11-11")).unwrap());
        assert!(list(&conn, false).unwrap().is_empty());
        assert_eq!(list(&conn, true).unwrap().len(), 1);
    }

    #[test]
    fn active_now_ignores_preventive_and_future_blocks() {
        let mut conn = db();
        add(&mut conn, "preventive", "2026-11-10", Some("2026-11-13"));
        add(&mut conn, "active", "2026-11-20", None);

        assert!(!active_now(&conn, 1, d("2026-11-11")).unwrap());
        assert!(!active_now(&conn, 1, d("2026-11-19")).unwrap());
        assert!(active_now(&conn, 1, d("2026-11-20")).unwrap());
    }

    #[test]
    fn double_release_is_conflict() {
        let mut conn = db();
        let id = add(&mut conn, "awaiting_release", "2026-11-10", None);
        let tx = conn.transaction().unwrap();

        release(&tx, id, 1, "ok", 1, at()).unwrap();

        assert!(matches!(release(&tx, id, 1, "ok", 2, at()), Err(AppError::Conflict)));
    }
}
