use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::page::Page;
use crate::models::stay::{
    AdjustmentInput, ChargeInput, CheckInInput, CheckoutInput, CheckoutPreview, PaymentInput, StayDetail, StayQuery, StaySummary,
    WalkInInput,
};

#[tauri::command]
pub fn check_in_reservation(app: State<'_, App>, token: String, input: CheckInInput) -> Result<i64, AppError> {
    app.state()?.stays.check_in(&token, input)
}

#[tauri::command]
pub fn walk_in(app: State<'_, App>, token: String, input: WalkInInput) -> Result<i64, AppError> {
    app.state()?.stays.walk_in(&token, input)
}

#[tauri::command]
pub fn add_stay_charge(app: State<'_, App>, token: String, stay_id: i64, input: ChargeInput) -> Result<i64, AppError> {
    app.state()?.stays.add_charge(&token, stay_id, input)
}

#[tauri::command]
pub fn add_stay_adjustment(app: State<'_, App>, token: String, stay_id: i64, input: AdjustmentInput) -> Result<i64, AppError> {
    app.state()?.stays.add_adjustment(&token, stay_id, input)
}

#[tauri::command]
pub fn reverse_stay_charge(
    app: State<'_, App>,
    token: String,
    stay_id: i64,
    charge_id: i64,
    justification: String,
    replacement: Option<ChargeInput>,
) -> Result<(), AppError> {
    app.state()?.stays.reverse_charge(&token, stay_id, charge_id, justification, replacement)
}

#[tauri::command]
pub fn record_stay_payment(app: State<'_, App>, token: String, stay_id: i64, input: PaymentInput) -> Result<i64, AppError> {
    app.state()?.stays.record_payment(&token, stay_id, input)
}

#[tauri::command]
pub fn preview_stay_checkout(app: State<'_, App>, token: String, stay_id: i64) -> Result<CheckoutPreview, AppError> {
    app.state()?.stays.preview_checkout(&token, stay_id)
}

#[tauri::command]
pub fn checkout_stay(app: State<'_, App>, token: String, input: CheckoutInput) -> Result<(), AppError> {
    app.state()?.stays.checkout(&token, input)
}

#[tauri::command]
pub fn set_stay_late_tolerance(
    app: State<'_, App>,
    token: String,
    stay_id: i64,
    minutes: Option<u32>,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.stays.set_late_tolerance(&token, stay_id, minutes, expected_version)
}

#[tauri::command]
pub fn get_stay(app: State<'_, App>, token: String, id: i64) -> Result<StayDetail, AppError> {
    app.state()?.stays.get(&token, id)
}

#[tauri::command]
pub fn list_stays(app: State<'_, App>, token: String, query: StayQuery) -> Result<Page<StaySummary>, AppError> {
    app.state()?.stays.list(&token, query)
}
