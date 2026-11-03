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

use super::block::RoomBlock;
use super::reservation::ConflictInfo;

/// Resultado de abrir uma ordem: reservas afetadas exigem resolução antes da indisponibilidade efetiva.
#[derive(Debug, Clone, Serialize)]
pub struct OpenedOrder {
    pub id: i64,
    pub code: String,
    pub conflicts: Vec<ConflictInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    /// Conflitos que ainda impedem a indisponibilidade efetiva.
    pub still_open: Vec<MaintenanceConflict>,
    /// O quarto já está indisponível de fato (bloqueio efetivo).
    pub effective: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderDetail {
    pub order: MaintenanceOrder,
    pub conflicts: Vec<MaintenanceConflict>,
    pub block: Option<RoomBlock>,
}
