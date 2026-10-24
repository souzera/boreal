use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::page::Page;
use crate::models::stay::{CheckInInput, StayDetail, StayQuery, StaySummary, WalkInInput};

#[tauri::command]
pub fn check_in_reservation(app: State<'_, App>, token: String, input: CheckInInput) -> Result<i64, AppError> {
    app.state()?.stays.check_in(&token, input)
}

#[tauri::command]
pub fn walk_in(app: State<'_, App>, token: String, input: WalkInInput) -> Result<i64, AppError> {
    app.state()?.stays.walk_in(&token, input)
}

#[tauri::command]
pub fn get_stay(app: State<'_, App>, token: String, id: i64) -> Result<StayDetail, AppError> {
    app.state()?.stays.get(&token, id)
}

#[tauri::command]
pub fn list_stays(app: State<'_, App>, token: String, query: StayQuery) -> Result<Page<StaySummary>, AppError> {
    app.state()?.stays.list(&token, query)
}
