//! Saúde do sistema (spec §15.3): espaço em disco e, nas próximas tarefas, integridade e relatório de abertura.

use rusqlite::Connection;

use super::Ctx;
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::error::AppError;
use crate::models::health::DiskStatus;

const ONE_GIB: u64 = 1024 * 1024 * 1024;

/// Tamanho do banco em bytes (`page_count * page_size`; o SQLCipher devolve o tamanho da página como texto).
pub(crate) fn database_bytes(conn: &Connection) -> u64 {
    let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap_or(0);
    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |r| r.get::<_, rusqlite::types::Value>(0))
        .ok()
        .and_then(|v| match v {
            rusqlite::types::Value::Integer(n) => Some(n),
            rusqlite::types::Value::Text(t) => t.trim().parse().ok(),
            _ => None,
        })
        .unwrap_or(4096);
    u64::try_from(pages.saturating_mul(page_size)).unwrap_or(0)
}

/// O alerta de pouco espaço dispara abaixo do maior entre 1 GB e três vezes o tamanho do banco.
pub fn disk_threshold(db_bytes: u64) -> u64 {
    ONE_GIB.max(db_bytes.saturating_mul(3))
}

/// `1,2 GB`, `340 MB`, `12 KB`.
pub fn size_text(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= KB * KB * KB {
        format!("{:.1} GB", b / (KB * KB * KB)).replace('.', ",")
    } else if b >= KB * KB {
        format!("{:.0} MB", b / (KB * KB))
    } else {
        format!("{:.0} KB", (b / KB).max(1.0))
    }
}

#[derive(Clone)]
pub struct HealthService {
    ctx: Ctx,
}

impl HealthService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    pub fn disk_status(&self, token: &str) -> Result<DiskStatus, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        let db_bytes = database_bytes(&*self.ctx.conn()?);
        let free_bytes = self.ctx.disk.free_bytes(&self.ctx.data_dir)?;
        let threshold_bytes = disk_threshold(db_bytes);
        Ok(DiskStatus { free_bytes, db_bytes, threshold_bytes, low: free_bytes < threshold_bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_support::disk_world;

    #[test]
    fn threshold_is_one_gib_or_three_times_the_database() {
        assert_eq!(disk_threshold(10 * 1024 * 1024), ONE_GIB);
        assert_eq!(disk_threshold(600 * 1024 * 1024), 1800 * 1024 * 1024);
    }

    #[test]
    fn sizes_read_in_plain_units() {
        assert_eq!(size_text(1536 * 1024 * 1024), "1,5 GB");
        assert_eq!(size_text(340 * 1024 * 1024), "340 MB");
        assert_eq!(size_text(100), "1 KB");
    }

    #[test]
    fn disk_status_uses_the_probe_and_is_admin_only() {
        let w = disk_world();
        let health = HealthService::new(w.ctx.clone());

        w.disk.set(5 * ONE_GIB);
        let ok = health.disk_status(&w.admin).unwrap();
        w.disk.set(ONE_GIB / 2);
        let low = health.disk_status(&w.admin).unwrap();

        assert!(!ok.low && low.low);
        assert_eq!((ok.free_bytes, low.free_bytes), (5 * ONE_GIB, ONE_GIB / 2));
        assert!(ok.db_bytes > 0 && ok.threshold_bytes >= ONE_GIB);
        assert!(matches!(health.disk_status(&w.reception), Err(AppError::Forbidden)));
    }
}
