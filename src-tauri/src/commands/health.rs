use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::health::{DiskStatus, StartupReport};

#[tauri::command]
pub fn disk_status(app: State<'_, App>, token: String) -> Result<DiskStatus, AppError> {
    app.state()?.health.disk_status(&token)
}

#[tauri::command]
pub fn log_folder(app: State<'_, App>, token: String) -> Result<String, AppError> {
    app.state()?.health.log_folder(&token)
}

#[tauri::command]
pub fn last_startup_report(app: State<'_, App>, token: String) -> Result<Option<StartupReport>, AppError> {
    app.state()?.health.last_startup(&token)
}
