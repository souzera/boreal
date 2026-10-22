pub mod auth_service;
pub mod catalog_service;
pub mod pousada_service;
pub mod settings_service;
pub mod user_service;

#[cfg(test)]
pub mod test_support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::auth::session::{Clock, SessionManager, SystemClock};
use crate::error::AppError;
use crate::repositories::settings_repo;

/// Relógio de parede em UTC (instantes gravados no banco), injetável nos testes.
pub trait UtcClock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
}

pub struct SystemUtc;

impl UtcClock for SystemUtc {
    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Dependências compartilhadas pelos serviços.
#[derive(Clone)]
pub struct Ctx {
    pub db: Arc<Mutex<Connection>>,
    pub sessions: Arc<SessionManager>,
    pub clock: Arc<dyn Clock>,
    pub utc: Arc<dyn UtcClock>,
}

impl Ctx {
    pub fn production(conn: Connection) -> Result<Self, AppError> {
        let idle_minutes = settings_repo::get_u32(&conn, "idle_lock_minutes")?;
        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            sessions: Arc::new(SessionManager::new(Duration::from_secs(
                u64::from(idle_minutes) * 60,
            ))),
            clock: Arc::new(SystemClock),
            utc: Arc::new(SystemUtc),
        })
    }
}

pub(crate) fn format_utc(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub(crate) fn parse_utc(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
