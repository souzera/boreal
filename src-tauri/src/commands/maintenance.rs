use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::maintenance::{ApplyResult, MaintenanceOrder, NewOrderInput, OpenedOrder, OrderDetail};

#[tauri::command]
pub fn list_maintenance(app: State<'_, App>, token: String, only_open: bool) -> Result<Vec<MaintenanceOrder>, AppError> {
    app.state()?.maintenance.list(&token, only_open)
}

#[tauri::command]
pub fn get_maintenance(app: State<'_, App>, token: String, id: i64) -> Result<OrderDetail, AppError> {
    app.state()?.maintenance.get(&token, id)
}

#[tauri::command]
pub fn open_maintenance(app: State<'_, App>, token: String, input: NewOrderInput) -> Result<OpenedOrder, AppError> {
    app.state()?.maintenance.open(&token, input)
}

#[tauri::command]
pub fn apply_maintenance_unavailability(
    app: State<'_, App>,
    token: String,
    order_id: i64,
    expected_version: i64,
) -> Result<ApplyResult, AppError> {
    app.state()?.maintenance.apply_unavailability(&token, order_id, expected_version)
}

#[tauri::command]
pub fn resolve_maintenance_conflict(app: State<'_, App>, token: String, conflict_id: i64, note: String) -> Result<(), AppError> {
    app.state()?.maintenance.resolve_conflict_manually(&token, conflict_id, note)
}

#[tauri::command]
pub fn start_maintenance(app: State<'_, App>, token: String, order_id: i64, expected_version: i64) -> Result<(), AppError> {
    app.state()?.maintenance.start(&token, order_id, expected_version)
}

#[tauri::command]
pub fn request_maintenance_resolution(app: State<'_, App>, token: String, order_id: i64, expected_version: i64) -> Result<(), AppError> {
    app.state()?.maintenance.request_resolution(&token, order_id, expected_version)
}

#[tauri::command]
pub fn complete_maintenance(app: State<'_, App>, token: String, order_id: i64, expected_version: i64) -> Result<(), AppError> {
    app.state()?.maintenance.complete(&token, order_id, expected_version)
}
