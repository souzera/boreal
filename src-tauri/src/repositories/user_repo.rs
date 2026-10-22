use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::error::AppError;
use crate::models::user::{NewUser, Role, User};

const COLUMNS: &str = "id, username, display_name, role, active, must_change_password,                        failed_attempts, locked_until, version, password_hash";

fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn map_user(row: &Row) -> rusqlite::Result<(User, String)> {
    let role: String = row.get(3)?;
    let user = User {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        role: Role::from_db(&role).ok_or(rusqlite::Error::InvalidQuery)?,
        active: row.get::<_, i64>(4)? == 1,
        must_change_password: row.get::<_, i64>(5)? == 1,
        failed_attempts: row.get(6)?,
        locked_until: row.get(7)?,
        version: row.get(8)?,
    };
    Ok((user, row.get(9)?))
}

pub fn insert(tx: &Transaction, user: &NewUser, at: DateTime<Utc>) -> Result<i64, AppError> {
    let at = stamp(at);
    tx.execute(
        "INSERT INTO users (username, display_name, password_hash, role, must_change_password, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            user.username,
            user.display_name,
            user.password_hash,
            user.role.as_str(),
            user.must_change_password,
            at
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn find_by_username(conn: &Connection, username: &str) -> Result<Option<(User, String)>, AppError> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM users WHERE username = ?1"),
            [username],
            map_user,
        )
        .optional()?)
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<User>, AppError> {
    Ok(conn
        .query_row(&format!("SELECT {COLUMNS} FROM users WHERE id = ?1"), [id], map_user)
        .optional()?
        .map(|(user, _)| user))
}

pub fn find_with_hash(conn: &Connection, id: i64) -> Result<Option<(User, String)>, AppError> {
    Ok(conn
        .query_row(&format!("SELECT {COLUMNS} FROM users WHERE id = ?1"), [id], map_user)
        .optional()?)
}

pub fn set_active(tx: &Transaction, id: i64, active: bool, at: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute(
        "UPDATE users SET active = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3",
        params![active, stamp(at), id],
    )?;
    Ok(())
}

/// Troca a senha se a versão do registro ainda for a consultada; senão, `Conflict`.
pub fn update_password(
    tx: &Transaction,
    id: i64,
    hash: &str,
    must_change: bool,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    let changed = tx.execute(
        "UPDATE users SET password_hash = ?1, must_change_password = ?2, version = version + 1, updated_at = ?3
         WHERE id = ?4 AND version = ?5",
        params![hash, must_change, stamp(at), id, expected_version],
    )?;
    if changed == 0 {
        return Err(AppError::Conflict);
    }
    Ok(())
}

pub fn record_failed_attempt(
    tx: &Transaction,
    id: i64,
    locked_until: Option<&str>,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    tx.execute(
        "UPDATE users SET failed_attempts = failed_attempts + 1,
                          locked_until = COALESCE(?1, locked_until), updated_at = ?2
         WHERE id = ?3",
        params![locked_until, stamp(at), id],
    )?;
    Ok(())
}

pub fn reset_attempts(tx: &Transaction, id: i64, at: DateTime<Utc>) -> Result<(), AppError> {
    tx.execute(
        "UPDATE users SET failed_attempts = 0, locked_until = NULL, updated_at = ?1 WHERE id = ?2",
        params![stamp(at), id],
    )?;
    Ok(())
}

pub fn count(conn: &Connection) -> Result<i64, AppError> {
    Ok(conn.query_row("SELECT count(*) FROM users", [], |r| r.get(0))?)
}

/// Linha de listagem de usuários (sem hash nem dados de sessão).
#[derive(Debug, serde::Serialize)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: Role,
    pub active: bool,
    pub must_change_password: bool,
    pub version: i64,
    pub extra_permissions: Vec<String>,
}

pub struct UserListFilter<'a> {
    pub text: Option<&'a str>,
    pub active: Option<bool>,
    pub sort: Option<&'a str>,
    pub descending: bool,
    pub limit: i64,
    pub offset: i64,
}

/// Colunas aceitas na ordenação: nunca se interpola texto vindo do cliente.
fn sort_column(sort: Option<&str>) -> &'static str {
    match sort {
        Some("username") => "username COLLATE NOCASE",
        Some("role") => "role",
        Some("active") => "active",
        _ => "display_name COLLATE NOCASE",
    }
}

pub fn list(conn: &Connection, filter: &UserListFilter) -> Result<(Vec<UserRow>, u64), AppError> {
    let pattern = filter.text.filter(|t| !t.trim().is_empty()).map(crate::models::page::like_pattern);
    let direction = if filter.descending { "DESC" } else { "ASC" };
    let where_clause = "(?1 IS NULL OR username LIKE ?1 ESCAPE '\\' OR display_name LIKE ?1 ESCAPE '\\')
                        AND (?2 IS NULL OR active = ?2)";
    let total: i64 = conn.query_row(
        &format!("SELECT count(*) FROM users WHERE {where_clause}"),
        params![pattern, filter.active],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT id, username, display_name, role, active, must_change_password, version
         FROM users WHERE {where_clause}
         ORDER BY {} {direction}, id LIMIT ?3 OFFSET ?4",
        sort_column(filter.sort)
    ))?;
    let rows = stmt
        .query_map(params![pattern, filter.active, filter.limit, filter.offset], |r| {
            let role: String = r.get(3)?;
            Ok(UserRow {
                id: r.get(0)?,
                username: r.get(1)?,
                display_name: r.get(2)?,
                role: Role::from_db(&role).ok_or(rusqlite::Error::InvalidQuery)?,
                active: r.get::<_, i64>(4)? == 1,
                must_change_password: r.get::<_, i64>(5)? == 1,
                version: r.get(6)?,
                extra_permissions: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rows, total.max(0) as u64))
}

pub fn count_active_admins(conn: &Connection) -> Result<i64, AppError> {
    Ok(conn.query_row(
        "SELECT count(*) FROM users WHERE role = 'admin' AND active = 1",
        [],
        |r| r.get(0),
    )?)
}

/// Troca o perfil se a versão ainda for a consultada; senão, `Conflict`.
pub fn set_role(
    tx: &Transaction,
    id: i64,
    role: Role,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    let changed = tx.execute(
        "UPDATE users SET role = ?1, version = version + 1, updated_at = ?2 WHERE id = ?3 AND version = ?4",
        params![role.as_str(), stamp(at), id, expected_version],
    )?;
    if changed == 0 {
        return Err(AppError::Conflict);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{count, find, find_by_username, insert, update_password};
    use crate::error::AppError;
    use crate::migrations::run_migrations;
    use crate::models::user::{NewUser, Role};
    use rusqlite::Connection;

    fn at() -> chrono::DateTime<chrono::Utc> {
        use chrono::TimeZone;
        chrono::Utc.with_ymd_and_hms(2026, 10, 21, 12, 0, 0).unwrap()
    }

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();
        conn
    }

    fn new_user(name: &str) -> NewUser {
        NewUser {
            username: name.into(),
            display_name: "Ana Souza".into(),
            password_hash: "hash0".into(),
            role: Role::Recepcao,
            must_change_password: true,
        }
    }

    #[test]
    fn find_by_username_is_case_insensitive() {
        let mut conn = db();
        let tx = conn.transaction().unwrap();
        let id = insert(&tx, &new_user("Ana"), at()).unwrap();
        tx.commit().unwrap();

        let (user, hash) = find_by_username(&conn, "ANA").unwrap().unwrap();

        assert_eq!(user.id, id);
        assert_eq!(hash, "hash0");
        assert_eq!(user.role, Role::Recepcao);
        assert!(user.must_change_password);
        assert_eq!(count(&conn).unwrap(), 1);
        assert!(find(&conn, id).unwrap().is_some());
        assert!(find_by_username(&conn, "nobody").unwrap().is_none());
    }

    #[test]
    fn update_password_conflicts_on_stale_version() {
        let mut conn = db();
        let tx = conn.transaction().unwrap();
        let id = insert(&tx, &new_user("ana"), at()).unwrap();
        update_password(&tx, id, "hash1", false, 1, at()).unwrap();
        let stale = update_password(&tx, id, "hash2", false, 1, at());
        tx.commit().unwrap();

        assert!(matches!(stale, Err(AppError::Conflict)));
        let (user, hash) = find_by_username(&conn, "ana").unwrap().unwrap();
        assert_eq!(hash, "hash1");
        assert_eq!(user.version, 2);
        assert!(!user.must_change_password);
    }
}
