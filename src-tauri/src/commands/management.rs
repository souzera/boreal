use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::management::ManagementDashboard;

#[tauri::command]
pub fn management_dashboard(app: State<'_, App>, token: String, from: String, to: String) -> Result<ManagementDashboard, AppError> {
    app.state()?.management.dashboard(&token, &from, &to)
}
