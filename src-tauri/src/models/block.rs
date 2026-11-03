use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RoomBlock {
    pub id: i64,
    pub room_id: i64,
    pub room_code: String,
    pub kind: String,
    pub status: String,
    pub start_date: String,
    /// Exclusivo; ausente = até liberação manual.
    pub end_date: Option<String>,
    pub reason: String,
    pub maintenance_order_id: Option<i64>,
    pub released_at: Option<String>,
    pub release_justification: Option<String>,
    pub version: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewBlockInput {
    pub room_id: i64,
    pub start_date: String,
    pub end_date: Option<String>,
    pub reason: String,
}
