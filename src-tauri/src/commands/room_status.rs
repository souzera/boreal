use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::room_status::RoomStatus;

#[tauri::command]
pub fn list_room_status(app: State<'_, App>, token: String) -> Result<Vec<RoomStatus>, AppError> {
    app.state()?.room_status.list(&token)
}

#[tauri::command]
pub fn set_room_cleaning_interval(
    app: State<'_, App>,
    token: String,
    room_id: i64,
    minutes: Option<u32>,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.catalog.set_room_cleaning_interval(&token, room_id, minutes, expected_version)
}
