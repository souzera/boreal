use tauri::State;

use crate::app::{App, AppInfo};
use crate::error::AppError;

/// `Ok` se o banco abriu; senão devolve o erro de inicialização (ex.: chave indisponível).
#[tauri::command]
pub fn startup_status(app: State<'_, App>) -> Result<(), AppError> {
    app.state().map(|_| ())
}

#[tauri::command]
pub fn get_app_info(app: State<'_, App>) -> Result<AppInfo, AppError> {
    app.state()?.app_info()
}
