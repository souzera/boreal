use serde::Serialize;
use tauri::State;

use crate::app::App;
use crate::error::AppError;

#[derive(Serialize)]
pub struct AppInfo {
    pub app_version: &'static str,
    pub schema_version: u32,
}

/// `Ok` se o banco abriu; senão devolve o erro de inicialização (ex.: chave indisponível).
#[tauri::command]
pub fn startup_status(app: State<'_, App>) -> Result<(), AppError> {
    app.state().map(|_| ())
}

#[tauri::command]
pub fn get_app_info(app: State<'_, App>) -> Result<AppInfo, AppError> {
    Ok(AppInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        schema_version: app.state()?.schema_version()?,
    })
}
