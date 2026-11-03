use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct AlertView {
    pub id: i64,
    pub kind: String,
    pub kind_label: String,
    /// `critical`, `high` ou `normal`.
    pub severity: String,
    pub severity_label: String,
    pub title: String,
    pub detail: String,
    /// Rota da tela do registro relacionado.
    pub link: String,
    /// `active`, `resolved` (a causa sumiu) ou `dismissed` (dispensado por alguém).
    pub status: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub resolved_at: Option<String>,
    pub resolved_by_name: Option<String>,
    pub resolution_note: Option<String>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertEventView {
    pub id: i64,
    pub at: String,
    pub user_name: Option<String>,
    /// `raised`, `updated`, `resolved` ou `dismissed`.
    pub event: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertSummary {
    pub critical: i64,
    pub high: i64,
    pub normal: i64,
    pub total: i64,
    /// Os alertas ativos mais urgentes (até 5).
    pub top: Vec<AlertView>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AlertQuery {
    /// `active` (padrão) ou `history` (resolvidos e dispensados).
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub page_size: u32,
}
