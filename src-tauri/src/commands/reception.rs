use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::reception::ReceptionOverview;

#[tauri::command]
pub fn reception_overview(app: State<'_, App>, token: String) -> Result<ReceptionOverview, AppError> {
    app.state()?.reception.overview(&token)
}
