use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::error::AppError;
use crate::models::maintenance::{MaintenanceConflict, MaintenanceOrder};

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

const SELECT: &str = "SELECT o.id, o.code, o.room_id, rm.code, o.description, o.priority, o.status, o.planned_start, o.planned_end,
        o.actual_start, o.actual_end, o.responsible, o.notes, o.version
     FROM maintenance_orders o JOIN rooms rm ON rm.id = o.room_id";

fn map(r: &Row) -> rusqlite::Result<MaintenanceOrder> {
    Ok(MaintenanceOrder {
        id: r.get(0)?,
        code: r.get(1)?,
        room_id: r.get(2)?,
        room_code: r.get(3)?,
        description: r.get(4)?,
        priority: r.get(5)?,
        status: r.get(6)?,
        planned_start: r.get(7)?,
        planned_end: r.get(8)?,
        actual_start: r.get(9)?,
        actual_end: r.get(10)?,
        responsible: r.get(11)?,
        notes: r.get(12)?,
        version: r.get(13)?,
    })
}

pub struct NewOrder<'a> {
    pub code: &'a str,
    pub room_id: i64,
    pub description: &'a str,
    pub priority: &'a str,
    pub planned_start: &'a str,
    pub planned_end: Option<&'a str>,
    pub responsible: &'a str,
    pub notes: &'a str,
    pub created_by: i64,
}

pub fn insert_order(tx: &Transaction, o: &NewOrder, at: DateTime<Utc>) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO maintenance_orders (code, room_id, description, priority, planned_start, planned_end, responsible, notes,
            created_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![o.code, o.room_id, o.description, o.priority, o.planned_start, o.planned_end, o.responsible, o.notes, o.created_by, stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find_order(conn: &Connection, id: i64) -> Result<Option<MaintenanceOrder>, AppError> {
    Ok(conn.query_row(&format!("{SELECT} WHERE o.id = ?1"), [id], map).optional()?)
}

pub fn list(conn: &Connection, only_open: bool) -> Result<Vec<MaintenanceOrder>, AppError> {
    let filter = if only_open { "WHERE o.status <> 'completed'" } else { "" };
    let mut stmt = conn.prepare(&format!("{SELECT} {filter} ORDER BY o.id DESC"))?;
    let rows = stmt.query_map([], map)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Muda a situação; `actual_start`/`actual_end` são gravados quando informados.
pub fn set_status(
    tx: &Transaction,
    id: i64,
    status: &str,
    actual_start: Option<DateTime<Utc>>,
    actual_end: Option<DateTime<Utc>>,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE maintenance_orders SET status = ?1, actual_start = COALESCE(?2, actual_start), actual_end = COALESCE(?3, actual_end),
                version = version + 1, updated_at = ?4
         WHERE id = ?5 AND version = ?6",
        params![status, actual_start.map(stamp), actual_end.map(stamp), stamp(at), id, expected_version],
    )?)
}

pub fn insert_conflict(tx: &Transaction, order_id: i64, reservation_id: i64) -> Result<(), AppError> {
    tx.execute(
        "INSERT OR IGNORE INTO maintenance_conflicts (order_id, reservation_id) VALUES (?1, ?2)",
        params![order_id, reservation_id],
    )?;
    Ok(())
}

const SELECT_CONFLICT: &str = "SELECT c.id, c.order_id, c.reservation_id, r.code, g.full_name, r.check_in_date, r.check_out_date,
        r.status, c.status, c.resolution_note
     FROM maintenance_conflicts c JOIN reservations r ON r.id = c.reservation_id JOIN guests g ON g.id = r.guest_id";

fn map_conflict(r: &Row) -> rusqlite::Result<MaintenanceConflict> {
    Ok(MaintenanceConflict {
        id: r.get(0)?,
        order_id: r.get(1)?,
        reservation_id: r.get(2)?,
        reservation_code: r.get(3)?,
        guest_name: r.get(4)?,
        check_in_date: r.get(5)?,
        check_out_date: r.get(6)?,
        reservation_status: r.get(7)?,
        status: r.get(8)?,
        resolution_note: r.get(9)?,
    })
}

pub fn conflicts(conn: &Connection, order_id: i64) -> Result<Vec<MaintenanceConflict>, AppError> {
    let mut stmt = conn.prepare(&format!("{SELECT_CONFLICT} WHERE c.order_id = ?1 ORDER BY r.check_in_date, c.id"))?;
    let rows = stmt.query_map([order_id], map_conflict)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn open_conflicts(conn: &Connection, order_id: i64) -> Result<Vec<MaintenanceConflict>, AppError> {
    Ok(conflicts(conn, order_id)?.into_iter().filter(|c| c.status == "open").collect())
}

pub fn find_conflict(conn: &Connection, id: i64) -> Result<Option<MaintenanceConflict>, AppError> {
    Ok(conn.query_row(&format!("{SELECT_CONFLICT} WHERE c.id = ?1"), [id], map_conflict).optional()?)
}

/// `Conflict` se o conflito já estava resolvido.
pub fn resolve_conflict(tx: &Transaction, id: i64, user: i64, note: &str, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE maintenance_conflicts SET status = 'resolved', resolution_note = ?1, resolved_at = ?2, resolved_by = ?3
         WHERE id = ?4 AND status = 'open'",
        params![note, stamp(at), user, id],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::test_db::{db, reservation};
    use chrono::TimeZone;

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 1, 12, 0, 0).unwrap()
    }

    fn order(tx: &Transaction) -> i64 {
        insert_order(
            tx,
            &NewOrder {
                code: "MAN-000001",
                room_id: 1,
                description: "Trocar chuveiro",
                priority: "high",
                planned_start: "2026-11-10",
                planned_end: Some("2026-11-12"),
                responsible: "",
                notes: "",
                created_by: 1,
            },
            at(),
        )
        .unwrap()
    }

    #[test]
    fn open_conflicts_lists_only_open() {
        let mut conn = db();
        reservation(&conn, 1, 1, "2026-11-10", "2026-11-12", "confirmed");
        reservation(&conn, 2, 1, "2026-11-12", "2026-11-14", "confirmed");
        let tx = conn.transaction().unwrap();
        let id = order(&tx);
        insert_conflict(&tx, id, 1).unwrap();
        insert_conflict(&tx, id, 2).unwrap();
        insert_conflict(&tx, id, 2).unwrap(); // idempotente
        let first = conflicts(&tx, id).unwrap()[0].id;

        resolve_conflict(&tx, first, 1, "reserva cancelada", at()).unwrap();

        let open = open_conflicts(&tx, id).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].reservation_code, "RES-000002");
        assert!(matches!(resolve_conflict(&tx, first, 1, "x", at()), Err(AppError::Conflict)));
    }

    #[test]
    fn status_change_checks_version_and_keeps_actual_dates() {
        let mut conn = db();
        let tx = conn.transaction().unwrap();
        let id = order(&tx);

        set_status(&tx, id, "in_progress", Some(at()), None, 1, at()).unwrap();
        set_status(&tx, id, "awaiting_resolution", None, None, 2, at()).unwrap();

        let o = find_order(&tx, id).unwrap().unwrap();
        assert_eq!(o.actual_start.as_deref(), Some("2026-11-01T12:00:00Z"));
        assert!(matches!(set_status(&tx, id, "completed", None, Some(at()), 2, at()), Err(AppError::Conflict)));
        assert_eq!(list(&tx, true).unwrap().len(), 1);
    }
}
