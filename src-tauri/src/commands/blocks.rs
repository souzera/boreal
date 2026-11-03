use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::block::{NewBlockInput, RoomBlock};

#[tauri::command]
pub fn list_blocks(app: State<'_, App>, token: String, include_released: bool) -> Result<Vec<RoomBlock>, AppError> {
    app.state()?.blocks.list(&token, include_released)
}

#[tauri::command]
pub fn create_block(app: State<'_, App>, token: String, input: NewBlockInput) -> Result<i64, AppError> {
    app.state()?.blocks.create(&token, input)
}

#[tauri::command]
pub fn release_block(
    app: State<'_, App>,
    token: String,
    block_id: i64,
    justification: String,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.blocks.release(&token, block_id, justification, expected_version)
}
