use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::services::auth_service::{LoginResult, SessionStatus};
use crate::services::user_service::NewUserInput;

#[tauri::command]
pub fn needs_bootstrap(app: State<'_, App>) -> Result<bool, AppError> {
    app.state()?.auth.needs_bootstrap()
}

#[tauri::command]
pub fn bootstrap_admin(
    app: State<'_, App>,
    username: String,
    display_name: String,
    password: String,
) -> Result<(), AppError> {
    app.state()?
        .auth
        .bootstrap_admin(&username, &display_name, &password)
}

#[tauri::command]
pub fn login(
    app: State<'_, App>,
    username: String,
    password: String,
) -> Result<LoginResult, AppError> {
    app.state()?.auth.login(&username, &password)
}

#[tauri::command]
pub fn logout(app: State<'_, App>, token: String) -> Result<(), AppError> {
    app.state()?.auth.logout(&token);
    Ok(())
}

#[tauri::command]
pub fn change_own_password(
    app: State<'_, App>,
    token: String,
    current: String,
    new: String,
) -> Result<(), AppError> {
    app.state()?.auth.change_own_password(&token, &current, &new)
}

#[tauri::command]
pub fn unlock_session(
    app: State<'_, App>,
    token: String,
    password: String,
) -> Result<(), AppError> {
    app.state()?.auth.unlock_session(&token, &password)
}

#[tauri::command]
pub fn session_status(app: State<'_, App>, token: String) -> Result<SessionStatus, AppError> {
    app.state()?.auth.session_state(&token)
}

#[tauri::command]
pub fn create_user(
    app: State<'_, App>,
    token: String,
    input: NewUserInput,
) -> Result<i64, AppError> {
    app.state()?.users.create_user(&token, input)
}

#[tauri::command]
pub fn reset_password(
    app: State<'_, App>,
    token: String,
    user_id: i64,
    temp_password: String,
) -> Result<(), AppError> {
    app.state()?
        .users
        .reset_password(&token, user_id, &temp_password)
}
