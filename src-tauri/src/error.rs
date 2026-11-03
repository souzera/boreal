use serde::{ser::SerializeStruct, Serialize, Serializer};

use crate::models::payment::DuplicateInfo;

/// Erro único exposto pelo backend. As mensagens são em linguagem simples e
/// nunca carregam senhas ou dados de autenticação.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("A chave de criptografia do banco não está disponível neste computador. Os dados não foram alterados.")]
    KeyUnavailable,
    #[error("A chave de criptografia guardada neste computador não abre o banco de dados. Os dados não foram alterados.")]
    KeyInvalid,
    #[error("O banco de dados é de uma versão mais nova que este aplicativo. Atualize o aplicativo; os dados não foram alterados.")]
    SchemaTooNew,
    #[error("É preciso trocar a senha temporária antes de continuar.")]
    PasswordChangeRequired,
    #[error("Os preços ou a disponibilidade mudaram desde a prévia. Revise a nova prévia antes de confirmar.")]
    StalePreview,
    #[error("Não foi possível concluir a operação. Tente novamente.")]
    Database(String),
    #[error("Sem espaço em disco. Libere espaço e tente de novo. Nada foi salvo.")]
    DiskFull,
    #[error("{0}")]
    Validation(String),
    #[error("É preciso entrar no sistema para continuar.")]
    Unauthorized,
    #[error("Usuário ou senha inválidos.")]
    InvalidCredentials,
    #[error("Conta bloqueada temporariamente por tentativas inválidas. Tente novamente mais tarde.")]
    AccountLocked { until: String },
    #[error("A sessão está bloqueada por inatividade. Informe a senha para continuar.")]
    Locked,
    #[error("Você não tem permissão para executar esta operação.")]
    Forbidden,
    #[error("O registro foi alterado por outra operação. Revise e tente novamente.")]
    Conflict,
    #[error("Possível recebimento duplicado: já existe o {} com o mesmo valor e as mesmas formas de pagamento no mesmo dia.", .0.code)]
    DuplicatePayment(Box<DuplicateInfo>),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::KeyUnavailable => "key_unavailable",
            AppError::KeyInvalid => "key_invalid",
            AppError::SchemaTooNew => "schema_too_new",
            AppError::PasswordChangeRequired => "password_change_required",
            AppError::StalePreview => "stale_preview",
            AppError::Database(_) => "database",
            AppError::DiskFull => "disk_full",
            AppError::Validation(_) => "validation",
            AppError::Unauthorized => "unauthorized",
            AppError::InvalidCredentials => "invalid_credentials",
            AppError::AccountLocked { .. } => "account_locked",
            AppError::Locked => "locked",
            AppError::Forbidden => "forbidden",
            AppError::Conflict => "conflict",
            AppError::DuplicatePayment(_) => "duplicate_payment",
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        // Disco cheio: o SQLite já desfez a transação inteira; só traduzimos para uma mensagem clara.
        if let rusqlite::Error::SqliteFailure(failure, _) = &e
            && failure.code == rusqlite::ErrorCode::DiskFull
        {
            return AppError::DiskFull;
        }
        AppError::Database(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let until = match self {
            AppError::AccountLocked { until } => Some(until),
            _ => None,
        };
        let duplicate = match self {
            AppError::DuplicatePayment(info) => Some(info),
            _ => None,
        };
        let mut s = serializer.serialize_struct("AppError", if until.is_some() || duplicate.is_some() { 3 } else { 2 })?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        if let Some(until) = until {
            s.serialize_field("until", until)?;
        }
        if let Some(info) = duplicate {
            s.serialize_field("details", info)?;
        }
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn sqlite_full_becomes_a_plain_disk_full_error_and_the_transaction_leaves_nothing() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('antes');").unwrap();
        let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap();
        conn.execute_batch(&format!("PRAGMA max_page_count = {};", pages + 2)).unwrap();

        let tx = conn.transaction().unwrap();
        let mut failure = None;
        for _ in 0..200 {
            if let Err(e) = tx.execute("INSERT INTO t VALUES (?1)", ["x".repeat(2_000)]) {
                failure = Some(AppError::from(e));
                break;
            }
        }
        drop(tx);

        let error = failure.expect("o banco devia encher");
        assert!(matches!(error, AppError::DiskFull), "{error:?}");
        assert_eq!(error.code(), "disk_full");
        assert!(error.to_string().contains("Nada foi salvo"));
        let count: i64 = conn.query_row("SELECT count(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "a transação inteira foi desfeita");
    }

    #[test]
    fn database_error_message_is_plain() {
        let e = AppError::Database("UNIQUE constraint failed: x.y".into());

        assert!(!e.to_string().contains("UNIQUE"));
        assert!(format!("{e:?}").contains("UNIQUE"));
    }
}
