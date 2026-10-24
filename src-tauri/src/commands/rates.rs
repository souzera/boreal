use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::rate::{Holiday, NewRateRule, RateRule};

#[tauri::command]
pub fn list_rate_rules(app: State<'_, App>, token: String, category_id: Option<i64>) -> Result<Vec<RateRule>, AppError> {
    app.state()?.rates.list_rules(&token, category_id)
}

#[tauri::command]
pub fn create_rate_rule(app: State<'_, App>, token: String, input: NewRateRule) -> Result<i64, AppError> {
    app.state()?.rates.create_rule(&token, input)
}

#[tauri::command]
pub fn update_rate_rule(
    app: State<'_, App>,
    token: String,
    id: i64,
    input: NewRateRule,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.rates.update_rule(&token, id, input, expected_version)
}

#[tauri::command]
pub fn set_rate_rule_active(app: State<'_, App>, token: String, id: i64, active: bool) -> Result<(), AppError> {
    app.state()?.rates.set_rule_active(&token, id, active)
}

#[tauri::command]
pub fn delete_rate_rule(app: State<'_, App>, token: String, id: i64) -> Result<(), AppError> {
    app.state()?.rates.delete_rule(&token, id)
}

#[tauri::command]
pub fn list_holidays(app: State<'_, App>, token: String, year: Option<i32>) -> Result<Vec<Holiday>, AppError> {
    app.state()?.rates.list_holidays(&token, year)
}

#[tauri::command]
pub fn create_holiday(app: State<'_, App>, token: String, date: String, name: String) -> Result<i64, AppError> {
    app.state()?.rates.create_holiday(&token, &date, &name)
}

#[tauri::command]
pub fn delete_holiday(app: State<'_, App>, token: String, id: i64) -> Result<(), AppError> {
    app.state()?.rates.delete_holiday(&token, id)
}
