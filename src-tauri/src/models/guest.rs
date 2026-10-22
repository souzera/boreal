use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct Guest {
    pub id: i64,
    pub code: String,
    pub full_name: String,
    pub phone: String,
    pub cpf: Option<String>,
    pub email: String,
    pub address: String,
    pub active: bool,
    pub version: i64,
}

/// Dados informados na tela; nome e telefone são obrigatórios.
#[derive(Debug, Deserialize)]
pub struct GuestInput {
    pub full_name: String,
    pub phone: String,
    pub cpf: Option<String>,
    pub email: String,
    pub address: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateReason {
    Name,
    Phone,
    Cpf,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateMatch {
    pub guest: Guest,
    pub reasons: Vec<DuplicateReason>,
}

#[derive(Debug, Deserialize)]
pub struct GuestQuery {
    pub text: Option<String>,
    pub active: Option<bool>,
    pub page: u32,
    pub page_size: u32,
    pub sort: Option<String>,
    /// `"asc"` (padrão) ou `"desc"`.
    pub direction: Option<String>,
}
