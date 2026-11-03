//! Saúde do sistema (spec §15.3): espaço em disco e, nas próximas tarefas, integridade e relatório de abertura.

use rusqlite::Connection;

use super::Ctx;
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::error::AppError;
use crate::models::health::{DiskStatus, StartupReport};

const MAX_PROBLEMS: usize = 20;

/// Resultado de `PRAGMA integrity_check` e `PRAGMA foreign_key_check`.
pub struct IntegrityReport {
    pub ok: bool,
    pub problems: Vec<String>,
}

/// Confere o banco ao abrir. Qualquer erro de leitura conta como problema (página ilegível, por exemplo).
/// As descrições nunca trazem o conteúdo das linhas, só o tipo do problema e o nome da tabela.
pub fn check_database(conn: &Connection) -> IntegrityReport {
    let mut problems: Vec<String> = Vec::new();
    match conn.prepare("PRAGMA integrity_check").and_then(|mut stmt| stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()) {
        Ok(rows) => {
            if rows != ["ok"] {
                problems.extend(rows.into_iter().take(MAX_PROBLEMS).map(|row| format!("Verificação de integridade: {}", row.chars().take(120).collect::<String>())));
            }
        }
        Err(_) => problems.push("O arquivo do banco não pôde ser lido até o fim.".to_string()),
    }
    match conn.prepare("PRAGMA foreign_key_check").and_then(|mut stmt| stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()) {
        Ok(tables) => {
            for table in tables.into_iter().take(MAX_PROBLEMS.saturating_sub(problems.len())) {
                problems.push(format!("Registro sem vínculo na tabela {table}."));
            }
        }
        Err(_) => problems.push("Os vínculos entre tabelas não puderam ser conferidos.".to_string()),
    }
    problems.truncate(MAX_PROBLEMS);
    IntegrityReport { ok: problems.is_empty(), problems }
}

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
    startup: Option<StartupReport>,
}

impl HealthService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx, startup: None }
    }

    /// Guarda o relatório da abertura (feito por `initialize`).
    pub fn with_startup(mut self, report: StartupReport) -> Self {
        self.startup = Some(report);
        self
    }

    pub fn last_startup(&self, token: &str) -> Result<Option<StartupReport>, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        Ok(self.startup.clone())
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

    fn file_db(dir: &std::path::Path) -> (std::path::PathBuf, Connection) {
        use crate::db::keystore::MemoryKeyStore;
        let path = dir.join("boreal.db");
        let mut conn = crate::db::open_database(&path, &MemoryKeyStore::with_key(&crate::services::test_support::TEST_KEY)).unwrap();
        crate::migrations::run_migrations(&mut conn).unwrap();
        (path, conn)
    }

    #[test]
    fn a_healthy_database_passes_the_check() {
        let dir = tempfile::tempdir().unwrap();
        let (_, conn) = file_db(dir.path());

        let report = check_database(&conn);

        assert!(report.ok && report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn foreign_key_violations_are_listed_by_table_without_row_data() {
        let dir = tempfile::tempdir().unwrap();
        let (_, conn) = file_db(dir.path());
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "INSERT INTO rooms (code, category_id, max_capacity, description, notes, active, version, created_at, updated_at) VALUES ('SEGREDO-999', 4242, 2, '', '', 1, 1, 'a', 'a')",
            [],
        )
        .unwrap();

        let report = check_database(&conn);

        assert!(!report.ok);
        assert!(report.problems.iter().any(|p| p.contains("tabela rooms")), "{:?}", report.problems);
        assert!(report.problems.iter().all(|p| !p.contains("SEGREDO-999")), "sem dados das linhas");
    }

    #[test]
    fn a_corrupted_file_is_reported_as_a_problem_and_never_panics() {
        let dir = tempfile::tempdir().unwrap();
        let (path, conn) = file_db(dir.path());
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        drop(conn);
        let mut bytes = std::fs::read(&path).unwrap();
        let len = bytes.len();
        assert!(len > 20_000, "o banco migrado tem várias páginas");
        for b in &mut bytes[len / 2..len / 2 + 4096] {
            *b ^= 0xA5;
        }
        std::fs::write(&path, &bytes).unwrap();

        use crate::db::keystore::MemoryKeyStore;
        let reopened = crate::db::open_database(&path, &MemoryKeyStore::with_key(&crate::services::test_support::TEST_KEY));
        match reopened {
            Ok(conn) => {
                let report = check_database(&conn);
                assert!(!report.ok && !report.problems.is_empty());
                assert!(report.problems.len() <= MAX_PROBLEMS);
            }
            Err(e) => assert!(matches!(e, AppError::KeyInvalid), "{e:?}"),
        }
    }

    #[test]
    fn a_crash_in_the_middle_of_a_transaction_leaves_the_database_intact() {
        use crate::db::keystore::MemoryKeyStore;
        let dir = tempfile::tempdir().unwrap();
        let (path, conn) = file_db(dir.path());
        conn.execute_batch("CREATE TABLE crash_teste (v TEXT);").unwrap();
        conn.execute("INSERT INTO crash_teste VALUES ('confirmada')", []).unwrap();
        // Transação aberta e grande o bastante para o SQLite começar a gravar páginas no WAL antes do commit.
        conn.execute_batch("BEGIN;").unwrap();
        for i in 0..6000 {
            conn.execute("INSERT INTO crash_teste VALUES (?1)", [format!("{i}-{}", "x".repeat(2000))]).unwrap();
        }

        // "Queda de energia": copia os arquivos como estão em disco, sem fechar nem confirmar nada.
        let crashed = tempfile::tempdir().unwrap();
        let copy = crashed.path().join("boreal.db");
        std::fs::copy(&path, &copy).unwrap();
        let wal = dir.path().join("boreal.db-wal");
        if wal.exists() {
            std::fs::copy(&wal, crashed.path().join("boreal.db-wal")).unwrap();
        }
        let recovered = crate::db::open_database(&copy, &MemoryKeyStore::with_key(&crate::services::test_support::TEST_KEY)).unwrap();

        let report = check_database(&recovered);
        let rows: Vec<String> = recovered.prepare("SELECT v FROM crash_teste").unwrap().query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect();
        assert!(report.ok, "{:?}", report.problems);
        assert_eq!(rows, ["confirmada"], "só o que foi confirmado sobrevive; a transação interrompida some inteira");
        conn.execute_batch("ROLLBACK;").unwrap();
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
