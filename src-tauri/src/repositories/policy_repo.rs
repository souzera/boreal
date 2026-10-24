use rusqlite::{params, Connection, Transaction};

use super::settings_repo;
use crate::domain::cancellation::Tier;
use crate::error::AppError;

const VERSION_KEY: &str = "cancellation_policy_version";

/// Faixas da política, da maior para a menor antecedência.
pub fn list_tiers(conn: &Connection) -> Result<Vec<Tier>, AppError> {
    let mut stmt =
        conn.prepare("SELECT min_days_before, refund_bp FROM cancellation_tiers ORDER BY min_days_before DESC")?;
    let rows = stmt
        .query_map([], |r| Ok(Tier { min_days_before: r.get(0)?, refund_bp: r.get(1)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn version(conn: &Connection) -> Result<i64, AppError> {
    Ok(i64::from(settings_repo::get_u32(conn, VERSION_KEY)?))
}

pub fn replace_tiers(tx: &Transaction, tiers: &[Tier]) -> Result<i64, AppError> {
    tx.execute("DELETE FROM cancellation_tiers", [])?;
    for t in tiers {
        tx.execute(
            "INSERT INTO cancellation_tiers (min_days_before, refund_bp) VALUES (?1, ?2)",
            params![t.min_days_before, t.refund_bp],
        )?;
    }
    let next = version(tx)? + 1;
    settings_repo::set(tx, VERSION_KEY, &next.to_string())?;
    Ok(next)
}
