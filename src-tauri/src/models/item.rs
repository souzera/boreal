use serde::{Deserialize, Serialize};

/// Item ou serviço do catálogo de lançamentos (sem controle de estoque).
#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub default_price_cents: i64,
    pub active: bool,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct NewItem {
    pub name: String,
    pub default_price_cents: i64,
}

#[derive(Debug, Deserialize)]
pub struct ItemUpdate {
    pub name: String,
    pub default_price_cents: i64,
}
