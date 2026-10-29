use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct NewCategory {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategory {
    pub name: String,
    pub description: String,
}

impl From<NewCategory> for UpdateCategory {
    fn from(c: NewCategory) -> Self {
        Self { name: c.name, description: c.description }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Room {
    pub id: i64,
    pub code: String,
    pub category_id: i64,
    pub category_name: String,
    pub max_capacity: i64,
    pub description: String,
    pub notes: String,
    pub active: bool,
    /// Intervalo mínimo de limpeza próprio do quarto; ausente usa o padrão das configurações.
    pub cleaning_interval_minutes: Option<i64>,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct NewRoom {
    pub code: String,
    pub category_id: i64,
    pub max_capacity: i64,
    pub description: String,
    pub notes: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoom {
    pub code: String,
    pub category_id: i64,
    pub max_capacity: i64,
    pub description: String,
    pub notes: String,
}

impl From<NewRoom> for UpdateRoom {
    fn from(r: NewRoom) -> Self {
        Self {
            code: r.code,
            category_id: r.category_id,
            max_capacity: r.max_capacity,
            description: r.description,
            notes: r.notes,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RoomQuery {
    pub text: Option<String>,
    pub category_id: Option<i64>,
    pub active: Option<bool>,
    pub page: u32,
    pub page_size: u32,
    pub sort: Option<String>,
    /// `"asc"` (padrão) ou `"desc"`.
    pub direction: Option<String>,
}
