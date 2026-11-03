use serde::{Deserialize, Serialize};

/// Pedido de relatório vindo da tela. Todos os filtros são opcionais; cada relatório aceita só alguns.
#[derive(Debug, Clone, Deserialize)]
pub struct ReportQuery {
    pub kind: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub room_id: Option<i64>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub category_id: Option<i64>,
    #[serde(default)]
    pub text: Option<String>,
}

impl ReportQuery {
    pub fn new(kind: &str) -> Self {
        Self { kind: kind.into(), from: None, to: None, status: None, room_id: None, method: None, category_id: None, text: None }
    }
}

/// Arquivo gravado por uma exportação.
#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    pub path: String,
    pub bytes: u64,
}
