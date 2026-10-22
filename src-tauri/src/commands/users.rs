use tauri::State;

use crate::app::App;
use crate::auth::permissions::Permission;
use crate::error::AppError;

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
            Permission::from_str(name)
                .ok_or_else(|| AppError::Validation("Permissão desconhecida.".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    app.state()?
        .users
        .set_extra_permissions(&token, user_id, &parsed)
}
