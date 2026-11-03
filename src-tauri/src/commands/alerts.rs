use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::alert::{AlertEventView, AlertQuery, AlertSummary, AlertView};
use crate::models::page::Page;

#[tauri::command]
pub fn refresh_alerts(app: State<'_, App>, token: String) -> Result<(), AppError> {
    app.state()?.alerts.refresh(&token)
}

#[tauri::command]
pub fn list_alerts(app: State<'_, App>, token: String, query: AlertQuery) -> Result<Page<AlertView>, AppError> {
    app.state()?.alerts.list(&token, query)
}

#[tauri::command]
pub fn alert_summary(app: State<'_, App>, token: String) -> Result<AlertSummary, AppError> {
    app.state()?.alerts.summary(&token)
}

#[tauri::command]
pub fn alert_history(app: State<'_, App>, token: String, alert_id: i64) -> Result<Vec<AlertEventView>, AppError> {
    app.state()?.alerts.history(&token, alert_id)
}

#[tauri::command]
pub fn dismiss_alert(app: State<'_, App>, token: String, alert_id: i64, note: String, expected_version: i64) -> Result<(), AppError> {
    app.state()?.alerts.dismiss(&token, alert_id, note, expected_version)
}
