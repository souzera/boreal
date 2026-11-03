use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::room_map::RoomMap;

#[tauri::command]
pub fn room_map(app: State<'_, App>, token: String, from: String, days: u32) -> Result<RoomMap, AppError> {
    app.state()?.room_map.room_map(&token, &from, days)
}
