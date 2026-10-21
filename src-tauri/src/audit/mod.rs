use rusqlite::Transaction;
use serde_json::Value;

use crate::error::AppError;

const SENSITIVE_KEYS: &[&str] = &["password", "password_hash", "new_password"];

pub struct AuditEntry {
    pub user_id: Option<i64>,
    pub module: &'static str,
    pub operation: &'static str,
    pub entity: &'static str,
    pub entity_id: Option<i64>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub justification: Option<String>,
}

/// Remove, em qualquer nível, as chaves que carregam senhas.
pub fn scrub(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(k, _)| !SENSITIVE_KEYS.contains(&k.as_str()))
                .map(|(k, v)| (k, scrub(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(scrub).collect()),
        other => other,
    }
}

/// Grava o evento na mesma transação da operação auditada.
pub fn record(tx: &Transaction, entry: &AuditEntry) -> Result<(), AppError> {
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let json = |v: &Option<Value>| v.clone().map(|v| scrub(v).to_string());
    tx.execute(
        "INSERT INTO audit_log
            (user_id, at, module, operation, entity, entity_id, before_json, after_json, justification)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            entry.user_id,
            at,
            entry.module,
            entry.operation,
            entry.entity,
            entry.entity_id,
            json(&entry.before),
            json(&entry.after),
            entry.justification,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{record, scrub, AuditEntry};
    use crate::migrations::run_migrations;
    use rusqlite::Connection;
    use serde_json::json;

    fn entry() -> AuditEntry {
        AuditEntry {
            user_id: None,
            module: "auth",
            operation: "login",
            entity: "user",
            entity_id: Some(1),
            before: None,
            after: Some(json!({"name": "Ana", "password_hash": "x"})),
            justification: None,
        }
    }

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();
        conn
    }

    fn count(conn: &Connection) -> i64 {
        conn.query_row("SELECT count(*) FROM audit_log", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn record_commits_with_transaction() {
        let mut conn = db();
        let tx = conn.transaction().unwrap();
        record(&tx, &entry()).unwrap();
        tx.commit().unwrap();

        assert_eq!(count(&conn), 1);
        let after: String = conn
            .query_row("SELECT after_json FROM audit_log", [], |r| r.get(0))
            .unwrap();
        assert!(!after.contains("password_hash"));
        let at: String = conn.query_row("SELECT at FROM audit_log", [], |r| r.get(0)).unwrap();
        assert!(at.ends_with('Z'));
    }

    #[test]
    fn record_rolled_back_with_transaction() {
        let mut conn = db();
        let tx = conn.transaction().unwrap();
        record(&tx, &entry()).unwrap();
        tx.rollback().unwrap();

        assert_eq!(count(&conn), 0);
    }

    #[test]
    fn scrub_removes_password_keys() {
        let value = json!({"password_hash": "x", "name": "a", "nested": {"new_password": "y", "ok": 1}, "list": [{"password": "z"}]});

        assert_eq!(
            scrub(value),
            json!({"name": "a", "nested": {"ok": 1}, "list": [{}]})
        );
    }
}
