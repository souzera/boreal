use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::catalog::{
    Category, NewCategory, NewRoom, Room, RoomQuery, UpdateCategory, UpdateRoom,
};
use crate::models::page::Page;

#[tauri::command]
pub fn list_categories(app: State<'_, App>, token: String, active_only: bool) -> Result<Vec<Category>, AppError> {
    app.state()?.catalog.list_categories(&token, active_only)
}

#[tauri::command]
pub fn create_category(app: State<'_, App>, token: String, input: NewCategory) -> Result<i64, AppError> {
    app.state()?.catalog.create_category(&token, input)
}

#[tauri::command]
pub fn update_category(
    app: State<'_, App>,
    token: String,
    id: i64,
    input: UpdateCategory,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.catalog.update_category(&token, id, input, expected_version)
}

#[tauri::command]
pub fn set_category_active(app: State<'_, App>, token: String, id: i64, active: bool) -> Result<(), AppError> {
    app.state()?.catalog.set_category_active(&token, id, active)
}

#[tauri::command]
pub fn delete_category(app: State<'_, App>, token: String, id: i64) -> Result<(), AppError> {
    app.state()?.catalog.delete_category(&token, id)
}

#[tauri::command]
pub fn list_rooms(app: State<'_, App>, token: String, query: RoomQuery) -> Result<Page<Room>, AppError> {
    app.state()?.catalog.list_rooms(&token, query)
}

#[tauri::command]
pub fn create_room(app: State<'_, App>, token: String, input: NewRoom) -> Result<i64, AppError> {
    app.state()?.catalog.create_room(&token, input)
}

#[tauri::command]
pub fn update_room(
    app: State<'_, App>,
    token: String,
    id: i64,
    input: UpdateRoom,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.catalog.update_room(&token, id, input, expected_version)
}

#[tauri::command]
pub fn set_room_active(app: State<'_, App>, token: String, id: i64, active: bool) -> Result<(), AppError> {
    app.state()?.catalog.set_room_active(&token, id, active)
}

#[tauri::command]
pub fn delete_room(app: State<'_, App>, token: String, id: i64) -> Result<(), AppError> {
    app.state()?.catalog.delete_room(&token, id)
}
