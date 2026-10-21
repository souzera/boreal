use serde::{ser::SerializeStruct, Serialize, Serializer};

/// Erro único exposto pelo backend. As mensagens são em linguagem simples e
/// nunca carregam senhas ou dados de autenticação.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("A chave de criptografia do banco não está disponível neste computador. Os dados não foram alterados.")]
    KeyUnavailable,
    #[error("Erro no banco de dados: {0}")]
    Database(String),
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
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::KeyUnavailable => "key_unavailable",
            AppError::Database(_) => "database",
            AppError::Validation(_) => "validation",
            AppError::Unauthorized => "unauthorized",
            AppError::InvalidCredentials => "invalid_credentials",
            AppError::AccountLocked { .. } => "account_locked",
            AppError::Locked => "locked",
            AppError::Forbidden => "forbidden",
            AppError::Conflict => "conflict",
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let until = match self {
            AppError::AccountLocked { until } => Some(until),
            _ => None,
        };
        let mut s = serializer.serialize_struct("AppError", if until.is_some() { 3 } else { 2 })?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        if let Some(until) = until {
            s.serialize_field("until", until)?;
        }
        s.end()
    }
}
