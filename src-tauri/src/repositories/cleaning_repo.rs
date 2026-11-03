use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::domain::cleaning::Priority;
use crate::error::AppError;
use crate::models::cleaning::CleaningTask;

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

const SELECT: &str = "SELECT c.id, c.room_id, rm.code, c.stay_id, c.status, c.ready_since, c.priority_override,
        c.priority_justification, c.started_at, c.started_by, c.completed_at, c.completed_by, c.version
     FROM cleaning_tasks c JOIN rooms rm ON rm.id = c.room_id";

fn map(r: &Row) -> rusqlite::Result<CleaningTask> {
    Ok(CleaningTask {
        id: r.get(0)?,
        room_id: r.get(1)?,
        room_code: r.get(2)?,
        stay_id: r.get(3)?,
        status: r.get(4)?,
        ready_since: r.get(5)?,
        priority_override: r.get(6)?,
        priority_justification: r.get(7)?,
        started_at: r.get(8)?,
        started_by: r.get(9)?,
        completed_at: r.get(10)?,
        completed_by: r.get(11)?,
        version: r.get(12)?,
    })
}

/// Uma tarefa por hospedagem: a segunda inserção da mesma hospedagem falha.
pub fn insert_awaiting(
    tx: &Transaction,
    room_id: i64,
    stay_id: i64,
    ready_since: DateTime<Utc>,
    at: DateTime<Utc>,
) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO cleaning_tasks (room_id, stay_id, status, ready_since, created_at, updated_at)
         VALUES (?1, ?2, 'awaiting', ?3, ?4, ?4)",
        params![room_id, stay_id, stamp(ready_since), stamp(at)],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<CleaningTask>, AppError> {
    Ok(conn.query_row(&format!("{SELECT} WHERE c.id = ?1"), [id], map).optional()?)
}

/// Tarefa ainda não concluída do quarto (aguardando ou em andamento).
pub fn open_for_room(conn: &Connection, room_id: i64) -> Result<Option<CleaningTask>, AppError> {
    Ok(conn
        .query_row(
            &format!("{SELECT} WHERE c.room_id = ?1 AND c.status <> 'done' ORDER BY c.id DESC LIMIT 1"),
            [room_id],
            map,
        )
        .optional()?)
}

pub fn last_done_for_room(conn: &Connection, room_id: i64) -> Result<Option<CleaningTask>, AppError> {
    Ok(conn
        .query_row(
            &format!("{SELECT} WHERE c.room_id = ?1 AND c.status = 'done' ORDER BY c.completed_at DESC, c.id DESC LIMIT 1"),
            [room_id],
            map,
        )
        .optional()?)
}

/// Tarefas abertas, na ordem de chegada à fila (a prioridade é aplicada pelo serviço).
pub fn queue(conn: &Connection) -> Result<Vec<CleaningTask>, AppError> {
    let mut stmt = conn.prepare(&format!("{SELECT} WHERE c.status <> 'done' ORDER BY c.ready_since, c.id"))?;
    let rows = stmt.query_map([], map)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// `awaiting → in_progress`; `Conflict` se a versão mudou ou a tarefa já não está aguardando.
pub fn start(tx: &Transaction, id: i64, user: i64, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE cleaning_tasks SET status = 'in_progress', started_at = ?1, started_by = ?2, version = version + 1, updated_at = ?1
         WHERE id = ?3 AND version = ?4 AND status = 'awaiting'",
        params![stamp(at), user, id, expected_version],
    )?)
}

/// `in_progress → done`; `Conflict` se a versão mudou ou a tarefa não está em andamento.
pub fn complete(tx: &Transaction, id: i64, user: i64, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE cleaning_tasks SET status = 'done', completed_at = ?1, completed_by = ?2, version = version + 1, updated_at = ?1
         WHERE id = ?3 AND version = ?4 AND status = 'in_progress'",
        params![stamp(at), user, id, expected_version],
    )?)
}

pub fn set_priority(
    tx: &Transaction,
    id: i64,
    priority: Option<Priority>,
    justification: &str,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    expect_one(tx.execute(
        "UPDATE cleaning_tasks SET priority_override = ?1, priority_justification = ?2, version = version + 1, updated_at = ?3
         WHERE id = ?4 AND version = ?5 AND status <> 'done'",
        params![priority.map(Priority::as_str), priority.map(|_| justification), stamp(at), id, expected_version],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::test_db::{db, reservation, stay};
    use chrono::TimeZone;

    fn t(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 1, h, 0, 0).unwrap()
    }

    fn two_stays(conn: &Connection) {
        reservation(conn, 1, 1, "2026-10-30", "2026-11-01", "completed");
        stay(conn, 1);
        reservation(conn, 2, 2, "2026-10-30", "2026-11-01", "completed");
        stay(conn, 2);
    }

    #[test]
    fn insert_awaiting_twice_for_same_stay_fails() {
        let mut conn = db();
        two_stays(&conn);
        let tx = conn.transaction().unwrap();

        insert_awaiting(&tx, 1, 1, t(12), t(12)).unwrap();

        assert!(insert_awaiting(&tx, 1, 1, t(12), t(12)).is_err());
    }

    #[test]
    fn complete_with_stale_version_is_conflict() {
        let mut conn = db();
        two_stays(&conn);
        let tx = conn.transaction().unwrap();
        let id = insert_awaiting(&tx, 1, 1, t(12), t(12)).unwrap();
        start(&tx, id, 1, 1, t(13)).unwrap();

        assert!(matches!(complete(&tx, id, 1, 1, t(14)), Err(AppError::Conflict)));
        complete(&tx, id, 1, 2, t(14)).unwrap();
        assert!(matches!(complete(&tx, id, 1, 3, t(15)), Err(AppError::Conflict)));
        let task = find(&tx, id).unwrap().unwrap();
        assert_eq!(task.status, "done");
        assert_eq!(task.completed_at.as_deref(), Some("2026-11-01T14:00:00Z"));
    }

    #[test]
    fn complete_requires_in_progress() {
        let mut conn = db();
        two_stays(&conn);
        let tx = conn.transaction().unwrap();
        let id = insert_awaiting(&tx, 1, 1, t(12), t(12)).unwrap();

        assert!(matches!(complete(&tx, id, 1, 1, t(14)), Err(AppError::Conflict)));
    }

    #[test]
    fn queue_orders_open_tasks_only() {
        let mut conn = db();
        two_stays(&conn);
        let tx = conn.transaction().unwrap();
        let first = insert_awaiting(&tx, 1, 1, t(13), t(13)).unwrap();
        let second = insert_awaiting(&tx, 2, 2, t(11), t(11)).unwrap();
        start(&tx, first, 1, 1, t(14)).unwrap();
        complete(&tx, first, 1, 2, t(15)).unwrap();

        let ids: Vec<i64> = queue(&tx).unwrap().into_iter().map(|c| c.id).collect();

        assert_eq!(ids, vec![second]);
        assert!(open_for_room(&tx, 1).unwrap().is_none());
        assert_eq!(last_done_for_room(&tx, 1).unwrap().unwrap().id, first);
        assert_eq!(open_for_room(&tx, 2).unwrap().unwrap().id, second);
    }

    #[test]
    fn priority_override_can_be_set_and_cleared() {
        let mut conn = db();
        two_stays(&conn);
        let tx = conn.transaction().unwrap();
        let id = insert_awaiting(&tx, 1, 1, t(12), t(12)).unwrap();

        set_priority(&tx, id, Some(Priority::Urgent), "hóspede chega cedo", 1, t(12)).unwrap();
        let task = find(&tx, id).unwrap().unwrap();
        assert_eq!(task.priority_override.as_deref(), Some("urgent"));
        set_priority(&tx, id, None, "", 2, t(12)).unwrap();
        assert!(find(&tx, id).unwrap().unwrap().priority_override.is_none());
    }
}
