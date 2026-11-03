use rusqlite::{OptionalExtension, Transaction};

use crate::error::AppError;

/// Próximo número da sequência, consumido dentro da transação do chamador:
/// se a transação for desfeita, o número volta; se confirmada, nunca é reutilizado.
pub fn next(tx: &Transaction, name: &str) -> Result<i64, AppError> {
    tx.query_row(
        "UPDATE sequences SET next_value = next_value + 1 WHERE name = ?1 RETURNING next_value - 1",
        [name],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::Database(format!("sequência desconhecida: {name}")))
}

/// Código visível, por exemplo `HOS-000042`.
pub fn format_code(prefix: &str, n: i64) -> String {
    format!("{prefix}-{n:06}")
}

#[cfg(test)]
mod tests {
    use super::{format_code, next};
    use crate::migrations::run_migrations;
    use rusqlite::Connection;

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();
        conn
    }

    #[test]
    fn next_returns_consecutive_numbers() {
        let mut conn = db();
        let tx = conn.transaction().unwrap();

        assert_eq!(next(&tx, "guest").unwrap(), 1);
        assert_eq!(next(&tx, "guest").unwrap(), 2);
        assert_eq!(next(&tx, "reservation").unwrap(), 1);
    }

    #[test]
    fn rolled_back_transaction_does_not_consume_number() {
        let mut conn = db();
        let tx = conn.transaction().unwrap();
        assert_eq!(next(&tx, "guest").unwrap(), 1);
        tx.rollback().unwrap();

        let tx = conn.transaction().unwrap();
        assert_eq!(next(&tx, "guest").unwrap(), 1);
    }

    #[test]
    fn committed_numbers_are_never_reused() {
        let mut conn = db();
        let tx = conn.transaction().unwrap();
        assert_eq!(next(&tx, "payment").unwrap(), 1);
        tx.commit().unwrap();

        let tx = conn.transaction().unwrap();
        assert_eq!(next(&tx, "payment").unwrap(), 2);
    }

    #[test]
    fn unknown_sequence_fails() {
        let mut conn = db();
        let tx = conn.transaction().unwrap();

        assert!(next(&tx, "inexistente").is_err());
    }

    #[test]
    fn format_code_pads_six_digits() {
        assert_eq!(format_code("HOS", 42), "HOS-000042");
        assert_eq!(format_code("RES", 1234567), "RES-1234567");
    }
}
