use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::backup::{BackupFileView, BackupRunView, DestinationView};

/// Backup manual (administrador).
#[tauri::command]
pub fn run_backup(app: State<'_, App>, token: String) -> Result<BackupRunView, AppError> {
    app.state()?.backup.run(&token)
}

#[tauri::command]
pub fn list_backup_files(app: State<'_, App>, token: String) -> Result<Vec<BackupFileView>, AppError> {
    app.state()?.backup.list_files(&token)
}

#[tauri::command]
pub fn recent_backup_runs(app: State<'_, App>, token: String, limit: i64) -> Result<Vec<BackupRunView>, AppError> {
    app.state()?.backup.recent_runs(&token, limit)
}

#[tauri::command]
pub fn backup_destination(app: State<'_, App>, token: String) -> Result<DestinationView, AppError> {
    app.state()?.backup.destination(&token)
}

#[tauri::command]
pub fn set_backup_destination(app: State<'_, App>, token: String, dir: String) -> Result<DestinationView, AppError> {
    app.state()?.backup.set_destination(&token, &dir)
}
