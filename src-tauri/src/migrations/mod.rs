use rusqlite::Connection;

use crate::error::AppError;

pub struct Migration {
    pub version: u32,
    pub sql: &'static str,
}

/// Migrações embutidas, em ordem crescente de versão.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("sql/0001_foundation.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("sql/0002_sequences_permissions.sql"),
    },
];

/// Versão atual do esquema (0 se nenhuma migração foi aplicada).
pub fn current_version(conn: &Connection) -> Result<u32, AppError> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |r| r.get(0),
    )?)
}

/// Aplica as migrações pendentes e retorna a versão atual do esquema.
pub fn run_migrations(conn: &mut Connection) -> Result<u32, AppError> {
    run_with(conn, MIGRATIONS)
}

/// Cada migração roda em uma transação própria; falha reverte só aquela migração
/// e interrompe o processo.
pub fn run_with(conn: &mut Connection, migrations: &[Migration]) -> Result<u32, AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;
    let current: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |r| r.get(0),
    )?;

    let latest_known = migrations.iter().map(|m| m.version).max().unwrap_or(0);
    if current > latest_known {
        return Err(AppError::SchemaTooNew);
    }

    for m in migrations.iter().filter(|m| m.version > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(m.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
            [m.version],
        )?;
        tx.commit()?;
    }

    Ok(migrations
        .iter()
        .map(|m| m.version)
        .max()
        .unwrap_or(0)
        .max(current))
}

#[cfg(test)]
mod tests {
    use super::{run_migrations, run_with, Migration};
    use rusqlite::Connection;

    fn memory() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn applied_rows(conn: &Connection) -> i64 {
        conn.query_row("SELECT count(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn applies_all_and_is_idempotent() {
        let mut conn = memory();
        let first = run_migrations(&mut conn).unwrap();
        let rows = applied_rows(&conn);
        let second = run_migrations(&mut conn).unwrap();

        assert_eq!(first, second);
        assert_eq!(applied_rows(&conn), rows);
        let max_attempts: String = conn
            .query_row("SELECT value FROM settings WHERE key='max_failed_attempts'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(max_attempts, "5");
    }

    #[test]
    fn failed_migration_rolls_back() {
        let mut conn = memory();
        let migrations = [
            Migration { version: 1, sql: "CREATE TABLE a(x INTEGER);" },
            Migration {
                version: 2,
                sql: "CREATE TABLE b(x INTEGER); INSERT INTO tabela_inexistente VALUES (1);",
            },
        ];

        assert!(run_with(&mut conn, &migrations).is_err());

        let b_exists: i64 = conn
            .query_row("SELECT count(*) FROM sqlite_master WHERE name='b'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(b_exists, 0);
        assert_eq!(applied_rows(&conn), 1);
    }

    #[test]
    fn refuses_database_newer_than_app() {
        let mut conn = memory();
        let both = [
            Migration { version: 1, sql: "CREATE TABLE a(x INTEGER);" },
            Migration { version: 2, sql: "CREATE TABLE b(x INTEGER);" },
        ];
        run_with(&mut conn, &both).unwrap();

        let result = run_with(&mut conn, &both[..1]);

        assert!(matches!(result, Err(crate::error::AppError::SchemaTooNew)));
        assert_eq!(applied_rows(&conn), 2);
    }

    #[test]
    fn audit_log_is_immutable() {
        let mut conn = memory();
        run_migrations(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO audit_log (at, module, operation, entity) VALUES ('2026-01-01T00:00:00Z','m','o','e')",
            [],
        )
        .unwrap();

        let upd = conn.execute("UPDATE audit_log SET module='x'", []).unwrap_err();
        let del = conn.execute("DELETE FROM audit_log", []).unwrap_err();

        assert!(upd.to_string().contains("imutável"));
        assert!(del.to_string().contains("imutável"));
    }

    #[test]
    fn username_unique_case_insensitive() {
        let mut conn = memory();
        run_migrations(&mut conn).unwrap();
        let insert = |name: &str| {
            conn.execute(
                "INSERT INTO users (username, display_name, password_hash, role, created_at, updated_at)
                 VALUES (?1, 'x', 'h', 'admin', 't', 't')",
                [name],
            )
        };

        insert("Ana").unwrap();

        assert!(insert("ana").is_err());
    }
}
