use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::cleaning::CleaningItem;

#[tauri::command]
pub fn cleaning_queue(app: State<'_, App>, token: String) -> Result<Vec<CleaningItem>, AppError> {
    app.state()?.cleaning.queue(&token)
}

#[tauri::command]
pub fn start_cleaning(app: State<'_, App>, token: String, task_id: i64, expected_version: i64) -> Result<(), AppError> {
    app.state()?.cleaning.start(&token, task_id, expected_version)
}

#[tauri::command]
pub fn complete_cleaning(app: State<'_, App>, token: String, task_id: i64, expected_version: i64) -> Result<(), AppError> {
    app.state()?.cleaning.complete(&token, task_id, expected_version)
}

#[tauri::command]
pub fn set_cleaning_priority(
    app: State<'_, App>,
    token: String,
    task_id: i64,
    priority: Option<String>,
    justification: String,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.cleaning.set_priority(&token, task_id, priority, justification, expected_version)
}
