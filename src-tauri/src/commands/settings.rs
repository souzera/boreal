use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::settings::{SessionPolicy, SettingsView};
use crate::services::settings_service::UpdateSettings;

#[tauri::command]
pub fn get_settings(app: State<'_, App>, token: String) -> Result<SettingsView, AppError> {
    app.state()?.settings.get(&token)
}

#[tauri::command]
pub fn get_session_policy(app: State<'_, App>, token: String) -> Result<SessionPolicy, AppError> {
    app.state()?.settings.session_policy(&token)
}

#[tauri::command]
pub fn update_settings(
    app: State<'_, App>,
    token: String,
    input: UpdateSettings,
) -> Result<i64, AppError> {
    app.state()?.settings.update(&token, input)
}
