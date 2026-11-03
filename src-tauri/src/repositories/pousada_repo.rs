use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::error::AppError;
use crate::models::pousada::PousadaView;

pub fn get(conn: &Connection) -> Result<PousadaView, AppError> {
    Ok(conn.query_row(
        "SELECT name, address, phone, logo IS NOT NULL, version FROM pousada WHERE id = 1",
        [],
        |r| {
            Ok(PousadaView {
                name: r.get(0)?,
                address: r.get(1)?,
                phone: r.get(2)?,
                has_logo: r.get(3)?,
                version: r.get(4)?,
            })
        },
    )?)
}

pub fn get_logo(conn: &Connection) -> Result<Option<(String, Vec<u8>)>, AppError> {
    Ok(conn
        .query_row(
            "SELECT logo_mime, logo FROM pousada WHERE id = 1 AND logo IS NOT NULL",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

pub enum LogoWrite<'a> {
    Keep,
    Remove,
    Set { mime: &'a str, bytes: &'a [u8] },
}

/// Atualiza os dados se a versão ainda for a consultada; senão, `Conflict`. Devolve a nova versão.
pub fn update(
    tx: &Transaction,
    name: &str,
    address: &str,
    phone: &str,
    logo: LogoWrite,
    expected_version: i64,
    at: DateTime<Utc>,
) -> Result<i64, AppError> {
    let stamp = at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let changed = match logo {
        LogoWrite::Keep => tx.execute(
            "UPDATE pousada SET name = ?1, address = ?2, phone = ?3, version = version + 1, updated_at = ?4
             WHERE id = 1 AND version = ?5",
            params![name, address, phone, stamp, expected_version],
        )?,
        LogoWrite::Remove => tx.execute(
            "UPDATE pousada SET name = ?1, address = ?2, phone = ?3, logo = NULL, logo_mime = NULL,
                    version = version + 1, updated_at = ?4
             WHERE id = 1 AND version = ?5",
            params![name, address, phone, stamp, expected_version],
        )?,
        LogoWrite::Set { mime, bytes } => tx.execute(
            "UPDATE pousada SET name = ?1, address = ?2, phone = ?3, logo = ?6, logo_mime = ?7,
                    version = version + 1, updated_at = ?4
             WHERE id = 1 AND version = ?5",
            params![name, address, phone, stamp, expected_version, bytes, mime],
        )?,
    };
    if changed == 0 {
        return Err(AppError::Conflict);
    }
    Ok(expected_version + 1)
}
