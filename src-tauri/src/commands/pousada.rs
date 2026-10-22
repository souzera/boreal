use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::pousada::{LogoData, PousadaView, UpdatePousada};

#[tauri::command]
pub fn get_pousada(app: State<'_, App>, token: String) -> Result<PousadaView, AppError> {
    app.state()?.pousada.get(&token)
}

#[tauri::command]
pub fn get_pousada_logo(app: State<'_, App>, token: String) -> Result<Option<LogoData>, AppError> {
    app.state()?.pousada.get_logo(&token)
}

#[tauri::command]
pub fn update_pousada(
    app: State<'_, App>,
    token: String,
    input: UpdatePousada,
    expected_version: i64,
) -> Result<i64, AppError> {
    app.state()?.pousada.update(&token, input, expected_version)
}
