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
    Migration {
        version: 10,
        sql: include_str!("sql/0010_cleaning_maintenance.sql"),
    },
    Migration {
        version: 11,
        sql: include_str!("sql/0011_finance.sql"),
    },
    Migration {
        version: 12,
        sql: include_str!("sql/0012_alerts.sql"),
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

    #[test]
    fn migration_0010_creates_cleaning_and_maintenance_tables() {
        let conn = migrated();
        seed_stay(&conn);
        let stamp = "'2026-10-23T12:00:00Z'";

        for table in ["cleaning_tasks", "room_blocks", "maintenance_orders", "maintenance_conflicts"] {
            let n: i64 = conn
                .query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1", [table], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 1, "{table}");
        }
        conn.execute("UPDATE rooms SET cleaning_interval_minutes = 90 WHERE id = 1", []).unwrap();
        assert!(conn.execute("UPDATE rooms SET cleaning_interval_minutes = 1441 WHERE id = 1", []).is_err());

        let task = format!(
            "INSERT INTO cleaning_tasks (room_id, stay_id, status, ready_since, created_at, updated_at) VALUES (1, 1, 'awaiting', {stamp}, {stamp}, {stamp})"
        );
        conn.execute(&task, []).unwrap();
        assert!(conn.execute(&task, []).is_err(), "uma tarefa por hospedagem");

        let block = |status: &str| {
            conn.execute(
                &format!(
                    "INSERT INTO room_blocks (room_id, kind, status, start_date, reason, created_by, created_at, updated_at)
                     VALUES (1, 'manual', '{status}', '2026-11-01', 'x', 1, {stamp}, {stamp})"
                ),
                [],
            )
        };
        assert!(block("x").is_err());
        block("active").unwrap();

        let next: i64 = conn.query_row("SELECT next_value FROM sequences WHERE name='maintenance'", [], |r| r.get(0)).unwrap();
        assert_eq!(next, 1);
        assert_eq!(super::current_version(&conn).unwrap(), 12);
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
        assert_eq!(latest, 12);
        let fee: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'late_checkout_fee_bp'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fee, "5000");
    }

    #[test]
    fn migration_0011_creates_finance_tables() {
        let conn = migrated();
        for table in [
            "payments", "payment_components", "refunds", "refund_components", "expense_categories", "payables",
            "payable_payments", "cash_closings", "cash_closing_lines",
        ] {
            let n: i64 = conn
                .query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1", [table], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 1, "{table}");
        }
        let has_method: i64 = conn
            .query_row("SELECT count(*) FROM pragma_table_info('payments') WHERE name = 'method'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(has_method, 0, "payments não guarda mais a modalidade");
        let next: i64 = conn.query_row("SELECT next_value FROM sequences WHERE name='payable'", [], |r| r.get(0)).unwrap();
        assert_eq!(next, 1);
        let categories: i64 = conn.query_row("SELECT count(*) FROM expense_categories WHERE active = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(categories, 9);
        assert_eq!(super::current_version(&conn).unwrap(), 12);
    }

    #[test]
    fn migration_0011_preserves_existing_payments_as_components() {
        let mut conn = memory();
        run_with(&mut conn, &super::MIGRATIONS[..10]).unwrap();
        seed_stay(&conn);
        conn.execute("UPDATE sequences SET next_value = 3 WHERE name = 'payment'", []).unwrap();
        let t = "'2026-10-22T12:00:00Z'";
        conn.execute_batch(&format!(
            "INSERT INTO payments (id, code, reservation_id, stay_id, amount_cents, method, received_at, received_by)
               VALUES (1, 'PAG-000001', 1, 1, 10000, 'pix', {t}, 1), (2, 'PAG-000002', 1, 1, 12000, 'prior_deposit', {t}, 1);
             INSERT INTO refunds (id, code, payment_id, stay_id, amount_cents, reason, refunded_at, refunded_by)
               VALUES (1, 'DEV-000001', 1, 1, 2500, 'ajuste', {t}, 1);
             INSERT INTO reservations (id, code, room_id, guest_id, check_in_date, check_out_date, expected_check_in_time,
                 expected_check_out_time, occupants, status, list_total_cents, agreed_total_cents, deposit_expected_cents,
                 deposit_paid_cents, deposit_due_date, cancellation_fee_cents, created_by, created_at, updated_at)
               VALUES (2, 'RES-000002', 1, 1, '2026-12-01', '2026-12-03', '14:00', '12:00', 1, 'cancelled', 40000, 40000, 12000,
                 12000, '2026-11-20', 6000, 1, {t}, {t});
             INSERT INTO reservations (id, code, room_id, guest_id, check_in_date, check_out_date, expected_check_in_time,
                 expected_check_out_time, occupants, status, list_total_cents, agreed_total_cents, deposit_expected_cents,
                 deposit_paid_cents, deposit_due_date, created_by, created_at, updated_at)
               VALUES (3, 'RES-000003', 1, 1, '2027-01-10', '2027-01-12', '14:00', '12:00', 1, 'confirmed', 40000, 40000, 12000,
                 12000, '2027-01-05', 1, {t}, {t});"
        ))
        .unwrap();

        run_migrations(&mut conn).unwrap();

        let rows = |sql: &str| -> Vec<(i64, String, i64)> {
            let mut stmt = conn.prepare(sql).unwrap();
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
            rows.map(Result::unwrap).collect()
        };
        assert_eq!(
            rows("SELECT payment_id, method, amount_cents FROM payment_components ORDER BY payment_id"),
            vec![
                (1, "pix".to_string(), 10_000),
                (2, "unspecified".to_string(), 12_000),
                (3, "unspecified".to_string(), 12_000),
                (4, "unspecified".to_string(), 12_000),
            ]
        );
        assert_eq!(
            rows("SELECT id, kind, amount_cents FROM payments ORDER BY id"),
            vec![
                (1, "stay".to_string(), 10_000),
                (2, "deposit".to_string(), 12_000),
                (3, "deposit".to_string(), 12_000),
                (4, "deposit".to_string(), 12_000),
            ]
        );
        assert_eq!(
            rows("SELECT refund_id, method, amount_cents FROM refund_components"),
            vec![(1, "unspecified".to_string(), 2_500)]
        );
        let reservation: i64 = conn.query_row("SELECT reservation_id FROM refunds WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(reservation, 1);
        let refund_due: i64 = conn.query_row("SELECT cancellation_refund_cents FROM reservations WHERE id = 2", [], |r| r.get(0)).unwrap();
        assert_eq!(refund_due, 6_000);
        // sinal antigo que só existia como valor na reserva (sem hospedagem) ganha um recebimento sem modalidade
        let legacy: (String, i64) = conn
            .query_row(
                "SELECT c.method, c.amount_cents FROM payments p JOIN payment_components c ON c.payment_id = p.id
                 WHERE p.kind = 'deposit' AND p.reservation_id = 3",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((legacy.0.as_str(), legacy.1), ("unspecified", 12_000));
        let next: i64 = conn.query_row("SELECT next_value FROM sequences WHERE name = 'payment'", [], |r| r.get(0)).unwrap();
        assert_eq!(next, 5, "a sequência avança para os recebimentos criados pela migração");
        let total: i64 = conn.query_row("SELECT count(*) FROM payments", [], |r| r.get(0)).unwrap();
        assert_eq!(total, 4);
    }

    #[test]
    fn payment_components_reject_duplicate_method_and_zero_amount() {
        let conn = migrated();
        seed_stay(&conn);
        conn.execute(
            "INSERT INTO payments (id, code, reservation_id, stay_id, kind, amount_cents, received_at, received_by)
             VALUES (1, 'PAG-000001', 1, 1, 'stay', 10000, '2026-10-22T12:00:00Z', 1)",
            [],
        )
        .unwrap();
        let component = |method: &str, cents: i64| {
            conn.execute(
                "INSERT INTO payment_components (payment_id, method, amount_cents) VALUES (1, ?1, ?2)",
                rusqlite::params![method, cents],
            )
        };

        component("pix", 6_000).unwrap();
        assert!(component("pix", 4_000).is_err(), "modalidade repetida");
        assert!(component("cash", 0).is_err(), "valor zero");
        assert!(component("cheque", 100).is_err(), "modalidade desconhecida");
        component("cash", 4_000).unwrap();
    }

    #[test]
    fn cash_closings_reject_second_closing_for_same_date() {
        let conn = migrated();
        seed_stay(&conn);
        let close = |date: &str| {
            conn.execute(
                "INSERT INTO cash_closings (closing_date, closed_at, closed_by) VALUES (?1, '2026-10-22T22:00:00Z', 1)",
                [date],
            )
        };

        close("2026-10-22").unwrap();

        assert!(close("2026-10-22").is_err());
        close("2026-10-23").unwrap();
    }

    #[test]
    fn migration_0012_creates_alert_tables() {
        let conn = migrated();
        for table in ["alerts", "alert_events"] {
            let n: i64 = conn.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1", [table], |r| r.get(0)).unwrap();
            assert_eq!(n, 1, "{table}");
        }
        assert_eq!(super::current_version(&conn).unwrap(), 12);
        let insert = |kind: &str, key: &str, status: &str| {
            conn.execute(
                "INSERT INTO alerts (kind, subject_key, severity, title, detail, link, status, first_seen_at, last_seen_at)
                 VALUES (?1, ?2, 'high', 't', 'd', '/x', ?3, '2026-10-31T12:00:00Z', '2026-10-31T12:00:00Z')",
                rusqlite::params![kind, key, status],
            )
        };

        insert("deposit_overdue", "reservation:1", "active").unwrap();
        assert!(insert("deposit_overdue", "reservation:1", "active").is_err(), "um alerta ativo por causa");
        insert("deposit_overdue", "reservation:1", "resolved").unwrap();
        insert("deposit_overdue", "reservation:1", "dismissed").unwrap();
        assert!(insert("deposit_overdue", "reservation:1", "dismissed").is_err(), "um dispensado por causa enquanto ela continua");
        assert!(insert("deposit_overdue", "reservation:2", "x").is_err(), "situação inválida");
        assert!(insert("tipo_novo", "reservation:2", "active").is_err(), "tipo desconhecido");
    }

    #[test]
    fn alert_history_cannot_be_deleted() {
        let conn = migrated();
        conn.execute(
            "INSERT INTO alerts (kind, subject_key, severity, title, detail, link, status, first_seen_at, last_seen_at)
             VALUES ('departure_risk', 'stay:1', 'critical', 't', 'd', '/x', 'active', 'a', 'a')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO alert_events (alert_id, at, event) VALUES (1, 'a', 'raised')", []).unwrap();

        let del_alert = conn.execute("DELETE FROM alerts", []).unwrap_err();
        let del_event = conn.execute("DELETE FROM alert_events", []).unwrap_err();

        assert!(del_alert.to_string().contains("imutável"));
        assert!(del_event.to_string().contains("imutável"));
        assert!(conn.execute("INSERT INTO alert_events (alert_id, at, event) VALUES (1, 'a', 'x')", []).is_err());
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
