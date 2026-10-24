use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::error::AppError;
use crate::models::settings::AppSettings;

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

pub fn get_u32(conn: &Connection, key: &str) -> Result<u32, AppError> {
    get(conn, key)?
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| AppError::Database(format!("configuração inválida: {key}")))
}

fn get_text(conn: &Connection, key: &str) -> Result<String, AppError> {
    get(conn, key)?.ok_or_else(|| AppError::Database(format!("configuração ausente: {key}")))
}

pub fn set(tx: &Transaction, key: &str, value: &str) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn load(conn: &Connection) -> Result<AppSettings, AppError> {
    Ok(AppSettings {
        max_failed_attempts: get_u32(conn, "max_failed_attempts")?,
        lockout_minutes: get_u32(conn, "lockout_minutes")?,
        idle_lock_minutes: get_u32(conn, "idle_lock_minutes")?,
        default_check_in_time: get_text(conn, "default_check_in_time")?,
        default_check_out_time: get_text(conn, "default_check_out_time")?,
        late_checkout_tolerance_minutes: get_u32(conn, "late_checkout_tolerance_minutes")?,
        no_show_tolerance_minutes: get_u32(conn, "no_show_tolerance_minutes")?,
        cleaning_interval_minutes: get_u32(conn, "cleaning_interval_minutes")?,
        deposit_percent_bp: get_u32(conn, "deposit_percent_bp")?,
        deposit_due_days_after_booking: get_u32(conn, "deposit_due_days_after_booking")?,
        deposit_due_days_before_arrival: get_u32(conn, "deposit_due_days_before_arrival")?,
    })
}

pub fn version(conn: &Connection) -> Result<i64, AppError> {
    Ok(i64::from(get_u32(conn, "settings_version")?))
}

pub fn save(tx: &Transaction, s: &AppSettings) -> Result<(), AppError> {
    set(tx, "max_failed_attempts", &s.max_failed_attempts.to_string())?;
    set(tx, "lockout_minutes", &s.lockout_minutes.to_string())?;
    set(tx, "idle_lock_minutes", &s.idle_lock_minutes.to_string())?;
    set(tx, "default_check_in_time", &s.default_check_in_time)?;
    set(tx, "default_check_out_time", &s.default_check_out_time)?;
    set(tx, "late_checkout_tolerance_minutes", &s.late_checkout_tolerance_minutes.to_string())?;
    set(tx, "no_show_tolerance_minutes", &s.no_show_tolerance_minutes.to_string())?;
    set(tx, "cleaning_interval_minutes", &s.cleaning_interval_minutes.to_string())?;
    set(tx, "deposit_percent_bp", &s.deposit_percent_bp.to_string())?;
    set(tx, "deposit_due_days_after_booking", &s.deposit_due_days_after_booking.to_string())?;
    set(tx, "deposit_due_days_before_arrival", &s.deposit_due_days_before_arrival.to_string())?;
    Ok(())
}

pub fn bump_version(tx: &Transaction) -> Result<i64, AppError> {
    let next = version(tx)? + 1;
    set(tx, "settings_version", &next.to_string())?;
    Ok(next)
}
