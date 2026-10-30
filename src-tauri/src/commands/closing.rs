use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::closing::{CloseInput, ClosingPreview, ClosingView};

#[tauri::command]
pub fn closing_preview(app: State<'_, App>, token: String, date: String) -> Result<ClosingPreview, AppError> {
    app.state()?.closing.preview(&token, &date)
}

#[tauri::command]
pub fn close_cash(app: State<'_, App>, token: String, input: CloseInput) -> Result<ClosingView, AppError> {
    app.state()?.closing.close(&token, input)
}

#[tauri::command]
pub fn closing_pending_dates(app: State<'_, App>, token: String) -> Result<Vec<String>, AppError> {
    app.state()?.closing.pending_dates(&token)
}
