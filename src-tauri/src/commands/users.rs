use tauri::State;

use crate::app::App;
use crate::auth::permissions::Permission;
use crate::error::AppError;
use crate::models::page::Page;
use crate::models::user::Role;
use crate::repositories::user_repo::UserRow;
use crate::services::user_service::UserQuery;

/// Permissões que podem ser concedidas individualmente a um usuário da recepção.
#[tauri::command]
pub fn grantable_permissions() -> Vec<&'static str> {
    Permission::grantable().iter().map(|p| p.as_str()).collect()
}

#[tauri::command]
pub fn set_extra_permissions(
    app: State<'_, App>,
    token: String,
    user_id: i64,
    permissions: Vec<String>,
) -> Result<(), AppError> {
    let parsed = permissions
        .iter()
        .map(|name| {
            Permission::parse(name)
                .ok_or_else(|| AppError::Validation("Permissão desconhecida.".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    app.state()?
        .users
        .set_extra_permissions(&token, user_id, &parsed)
}

#[tauri::command]
pub fn list_users(
    app: State<'_, App>,
    token: String,
    query: UserQuery,
) -> Result<Page<UserRow>, AppError> {
    app.state()?.users.list(&token, query)
}

#[tauri::command]
pub fn set_user_role(
    app: State<'_, App>,
    token: String,
    user_id: i64,
    role: Role,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.users.set_role(&token, user_id, role, expected_version)
}

#[tauri::command]
pub fn set_user_active(
    app: State<'_, App>,
    token: String,
    user_id: i64,
    active: bool,
) -> Result<(), AppError> {
    app.state()?.users.set_active(&token, user_id, active)
}
