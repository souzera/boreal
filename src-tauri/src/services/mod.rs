pub mod auth_service;
pub mod block_service;
pub mod catalog_service;
pub mod cleaning_service;
pub mod guest_service;
pub mod item_service;
pub mod policy_service;
pub mod pousada_service;
pub mod rate_service;
pub mod reservation_service;
pub mod room_map_service;
pub(crate) mod room_rules;
pub mod settings_service;
pub mod stay_service;
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

    /// Deslocamento do fuso local do computador (spec §19.1: exibição e "hoje" seguem o horário local).
    fn local_offset(&self) -> chrono::FixedOffset {
        *chrono::Local::now().offset()
    }
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
    /// Conexão com o banco. Se outra operação entrou em pânico segurando a conexão, devolve erro
    /// (mensagem genérica) em vez de derrubar o aplicativo.
    pub fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.db.lock().map_err(|_| {
            AppError::Database("a conexão com o banco ficou indisponível após uma falha".into())
        })
    }

    /// Data e hora atuais no fuso local.
    pub fn local_now(&self) -> chrono::NaiveDateTime {
        let offset = i64::from(self.utc.local_offset().local_minus_utc());
        (self.utc.now_utc() + chrono::Duration::seconds(offset)).naive_utc()
    }

    /// Data de hoje no fuso local.
    pub fn today(&self) -> chrono::NaiveDate {
        let offset = i64::from(self.utc.local_offset().local_minus_utc());
        (self.utc.now_utc() + chrono::Duration::seconds(offset)).date_naive()
    }

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

#[cfg(test)]
mod tests {
    use super::test_support::{admin_token, harness};
    use crate::error::AppError;
    use crate::models::guest::GuestQuery;

    #[test]
    fn poisoned_db_mutex_returns_error_not_panic() {
        let h = harness();
        let admin = admin_token(&h);
        let db = h.db.clone();
        let _ = std::thread::spawn(move || {
            let _guard = db.lock().unwrap();
            panic!("pânico proposital para envenenar o mutex");
        })
        .join();

        let result = h.guests.list(
            &admin,
            GuestQuery { text: None, active: None, page: 1, page_size: 10, sort: None, direction: None },
        );

        assert!(matches!(result, Err(AppError::Database(_))));
    }
}
