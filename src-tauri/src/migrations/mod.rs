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
    Migration {
        version: 3,
        sql: include_str!("sql/0003_pousada_settings.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("sql/0004_catalog.sql"),
    },
    Migration {
        version: 5,
        sql: include_str!("sql/0005_guests.sql"),
    },
    Migration {
        version: 6,
        sql: include_str!("sql/0006_rates.sql"),
    },
    Migration {
        version: 7,
        sql: include_str!("sql/0007_policies.sql"),
    },
    Migration {
        version: 8,
        sql: include_str!("sql/0008_reservations.sql"),
    },
    Migration {
        version: 9,
        sql: include_str!("sql/0009_stays.sql"),
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

/// Quando a última migração foi aplicada (RFC 3339 UTC), se houver.
pub fn last_applied_at(conn: &Connection) -> Result<Option<String>, AppError> {
    Ok(conn.query_row("SELECT MAX(applied_at) FROM schema_migrations", [], |r| r.get(0))?)
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

    fn migrated() -> Connection {
        let mut conn = memory();
        run_migrations(&mut conn).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn
    }

    /// Usuário, hóspede, categoria, quarto, reserva e hospedagem mínimos para exercitar as restrições.
    fn seed_stay(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO users (id, username, display_name, password_hash, role, active, must_change_password, created_at, updated_at)
               VALUES (1, 'u', 'U', 'x', 'admin', 1, 0, '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z');
             INSERT INTO room_categories (id, name, description, active, version, created_at, updated_at)
               VALUES (1, 'Standard', '', 1, 1, '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z');
             INSERT INTO rooms (id, code, category_id, max_capacity, description, notes, active, version, created_at, updated_at)
               VALUES (1, '101', 1, 2, '', '', 1, 1, '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z');
             INSERT INTO guests (id, code, full_name, name_key, phone, email, address, active, version, created_at, updated_at)
               VALUES (1, 'HOS-1', 'Ana', 'ana', '11999990000', '', '', 1, 1, '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z');
             INSERT INTO reservations (id, code, room_id, guest_id, check_in_date, check_out_date, expected_check_in_time,
                 expected_check_out_time, occupants, status, list_total_cents, agreed_total_cents, deposit_expected_cents,
                 deposit_due_date, created_by, created_at, updated_at)
               VALUES (1, 'RES-000001', 1, 1, '2026-10-21', '2026-10-23', '14:00', '12:00', 1, 'checked_in', 40000, 40000, 12000,
                 '2026-10-20', 1, '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z');
             INSERT INTO stays (id, reservation_id, code, status, checked_in_at, checked_in_by, early_check_in, exceeded_capacity,
                 created_at, updated_at)
               VALUES (1, 1, 'RES-000001', 'in_house', '2026-10-21T17:00:00Z', 1, 0, 0, '2026-10-21T17:00:00Z', '2026-10-21T17:00:00Z');",
        )
        .unwrap();
    }

    fn insert_charge(conn: &Connection, kind: &str, amount: i64, night: Option<&str>, reverses: Option<i64>) -> rusqlite::Result<usize> {
        conn.execute(
            "INSERT INTO charges (stay_id, kind, description, amount_cents, night_date, reverses_charge_id, created_by, created_at)
             VALUES (1, ?1, 'x', ?2, ?3, ?4, 1, '2026-10-21T17:00:00Z')",
            rusqlite::params![kind, amount, night, reverses],
        )
    }

    #[test]
    fn migration_0009_creates_stay_schema() {
        let conn = migrated();
        for table in ["stays", "stay_occupants", "catalog_items", "charges", "payments", "refunds"] {
            let n: i64 = conn
                .query_row("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1", [table], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 1, "tabela {table} ausente");
        }
        let latest: i64 = conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r.get(0)).unwrap();
        assert_eq!(latest, 9);
        let fee: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'late_checkout_fee_bp'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fee, "5000");
    }

    #[test]
    fn charges_refuse_zero_amount_duplicate_nights_and_double_reversal() {
        let conn = migrated();
        seed_stay(&conn);

        assert!(insert_charge(&conn, "consumption", 0, None, None).is_err());
        insert_charge(&conn, "night", 20_000, Some("2026-10-21"), None).unwrap();
        assert!(insert_charge(&conn, "night", 20_000, Some("2026-10-21"), None).is_err());
        insert_charge(&conn, "consumption", 3_500, None, None).unwrap();
        let original: i64 = conn.query_row("SELECT max(id) FROM charges", [], |r| r.get(0)).unwrap();
        insert_charge(&conn, "reversal", -3_500, None, Some(original)).unwrap();
        assert!(insert_charge(&conn, "reversal", -3_500, None, Some(original)).is_err());
    }
}
