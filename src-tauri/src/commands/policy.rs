use tauri::State;

use crate::app::App;
use crate::domain::cancellation::Tier;
use crate::error::AppError;
use crate::services::policy_service::PolicyView;

#[tauri::command]
pub fn get_cancellation_policy(app: State<'_, App>, token: String) -> Result<PolicyView, AppError> {
    app.state()?.policy.get(&token)
}

#[tauri::command]
pub fn update_cancellation_policy(
    app: State<'_, App>,
    token: String,
    tiers: Vec<Tier>,
    expected_version: i64,
) -> Result<i64, AppError> {
    app.state()?.policy.replace(&token, tiers, expected_version)
}
