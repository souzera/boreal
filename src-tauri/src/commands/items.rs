use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::item::{Item, ItemUpdate, NewItem};

#[tauri::command]
pub fn list_items(app: State<'_, App>, token: String, only_active: bool) -> Result<Vec<Item>, AppError> {
    app.state()?.items.list(&token, only_active)
}

#[tauri::command]
pub fn create_item(app: State<'_, App>, token: String, input: NewItem) -> Result<i64, AppError> {
    app.state()?.items.create(&token, input)
}

#[tauri::command]
pub fn update_item(
    app: State<'_, App>,
    token: String,
    id: i64,
    input: ItemUpdate,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.items.update(&token, id, input, expected_version)
}

#[tauri::command]
pub fn set_item_active(
    app: State<'_, App>,
    token: String,
    id: i64,
    active: bool,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.items.set_active(&token, id, active, expected_version)
}
