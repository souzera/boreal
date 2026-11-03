use tauri::State;

use crate::app::App;
use crate::error::AppError;

/// Mostra a chave de recuperação ao administrador, que digita a senha de novo.
#[tauri::command]
pub fn reveal_recovery_key(app: State<'_, App>, token: String, password: String) -> Result<String, AppError> {
    app.state()?.recovery.reveal(&token, &password)
}
