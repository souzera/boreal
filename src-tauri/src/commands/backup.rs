use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::backup::{BackupFileView, BackupRunView, DestinationView, RestorePreview, RestoreResult};

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

#[tauri::command]
pub fn preview_restore(app: State<'_, App>, token: String, path: String) -> Result<RestorePreview, AppError> {
    app.state()?.backup.preview_restore(&token, &path)
}

/// Restaura um backup (administrador, com a senha e a confirmação digitada). Encerra todas as sessões.
#[tauri::command]
pub fn restore_backup(app: State<'_, App>, token: String, path: String, confirmation: String, password: String) -> Result<RestoreResult, AppError> {
    app.state()?.backup.restore(&token, &path, &confirmation, &password)
}
