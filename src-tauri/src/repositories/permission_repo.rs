use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Transaction};

use crate::auth::permissions::Permission;
use crate::error::AppError;

pub fn list(conn: &Connection, user_id: i64) -> Result<Vec<Permission>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT permission FROM user_permissions WHERE user_id = ?1 ORDER BY permission",
    )?;
    let names = stmt
        .query_map([user_id], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(names.iter().filter_map(|n| Permission::from_str(n)).collect())
}

/// Substitui o conjunto de permissões extras do usuário.
pub fn replace(
    tx: &Transaction,
    user_id: i64,
    permissions: &[Permission],
    granted_by: i64,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    tx.execute("DELETE FROM user_permissions WHERE user_id = ?1", [user_id])?;
    let granted_at = at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    for p in permissions {
        tx.execute(
            "INSERT INTO user_permissions (user_id, permission, granted_by, granted_at) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, p.as_str(), granted_by, granted_at],
        )?;
    }
    Ok(())
}
