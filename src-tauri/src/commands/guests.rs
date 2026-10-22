use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::guest::{DuplicateMatch, Guest, GuestInput, GuestQuery};
use crate::models::page::Page;
use crate::services::guest_service::CreateGuestOutcome;

#[tauri::command]
pub fn list_guests(app: State<'_, App>, token: String, query: GuestQuery) -> Result<Page<Guest>, AppError> {
    app.state()?.guests.list(&token, query)
}

#[tauri::command]
pub fn get_guest(app: State<'_, App>, token: String, id: i64) -> Result<Guest, AppError> {
    app.state()?.guests.get(&token, id)
}

#[tauri::command]
pub fn find_guest_duplicates(
    app: State<'_, App>,
    token: String,
    input: GuestInput,
) -> Result<Vec<DuplicateMatch>, AppError> {
    app.state()?.guests.find_possible_duplicates(&token, &input)
}

#[tauri::command]
pub fn create_guest(
    app: State<'_, App>,
    token: String,
    input: GuestInput,
    confirm_duplicates: bool,
) -> Result<CreateGuestOutcome, AppError> {
    app.state()?.guests.create(&token, input, confirm_duplicates)
}

#[tauri::command]
pub fn update_guest(
    app: State<'_, App>,
    token: String,
    id: i64,
    input: GuestInput,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.guests.update(&token, id, input, expected_version)
}

#[tauri::command]
pub fn set_guest_active(app: State<'_, App>, token: String, id: i64, active: bool) -> Result<(), AppError> {
    app.state()?.guests.set_active(&token, id, active)
}
