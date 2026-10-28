use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct MaintenanceOrder {
    pub id: i64,
    pub code: String,
    pub room_id: i64,
    pub room_code: String,
    pub description: String,
    pub priority: String,
    pub status: String,
    pub planned_start: String,
    pub planned_end: Option<String>,
    pub actual_start: Option<String>,
    pub actual_end: Option<String>,
    pub responsible: String,
    pub notes: String,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MaintenanceConflict {
    pub id: i64,
    pub order_id: i64,
    pub reservation_id: i64,
    pub reservation_code: String,
    pub guest_name: String,
    pub check_in_date: String,
    pub check_out_date: String,
    pub reservation_status: String,
    pub status: String,
    pub resolution_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewOrderInput {
    pub room_id: i64,
    pub description: String,
    pub priority: String,
    pub planned_start: String,
    pub planned_end: Option<String>,
    #[serde(default)]
    pub responsible: String,
    #[serde(default)]
    pub notes: String,
}
