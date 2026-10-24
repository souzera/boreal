use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::page::Page;
use crate::models::reservation::{
    AvailabilityResult, CancellationOverride, CancellationPreview, CancellationResult, ChangePreview, ChangeRequest, CreatedReservation, DiscountInput, NewReservation, Reservation, ReservationDetail,
    ReservationQuery, ReviewItem,
};

#[tauri::command]
pub fn create_reservation(
    app: State<'_, App>,
    token: String,
    input: NewReservation,
) -> Result<CreatedReservation, AppError> {
    app.state()?.reservations.create(&token, input)
}

#[tauri::command]
pub fn get_reservation(app: State<'_, App>, token: String, id: i64) -> Result<ReservationDetail, AppError> {
    app.state()?.reservations.get(&token, id)
}

#[tauri::command]
pub fn list_reservations(
    app: State<'_, App>,
    token: String,
    query: ReservationQuery,
) -> Result<Page<Reservation>, AppError> {
    app.state()?.reservations.list(&token, query)
}

#[tauri::command]
pub fn check_room_availability(
    app: State<'_, App>,
    token: String,
    room_id: i64,
    check_in: String,
    check_out: String,
    ignore_reservation_id: Option<i64>,
) -> Result<AvailabilityResult, AppError> {
    app.state()?
        .reservations
        .check_availability(&token, room_id, &check_in, &check_out, ignore_reservation_id)
}

#[tauri::command]
pub fn preview_reservation_change(
    app: State<'_, App>,
    token: String,
    id: i64,
    request: ChangeRequest,
) -> Result<ChangePreview, AppError> {
    app.state()?.reservations.preview_change(&token, id, request)
}

#[tauri::command]
pub fn apply_reservation_change(
    app: State<'_, App>,
    token: String,
    id: i64,
    request: ChangeRequest,
    expected_version: i64,
    justification: String,
    fingerprint: String,
) -> Result<(), AppError> {
    app.state()?
        .reservations
        .apply_change(&token, id, request, expected_version, justification, &fingerprint)
}

#[tauri::command]
pub fn preview_reservation_cancellation(
    app: State<'_, App>,
    token: String,
    id: i64,
) -> Result<CancellationPreview, AppError> {
    app.state()?.reservations.preview_cancellation(&token, id)
}

#[tauri::command]
pub fn cancel_reservation(
    app: State<'_, App>,
    token: String,
    id: i64,
    expected_version: i64,
    exception: Option<CancellationOverride>,
) -> Result<CancellationResult, AppError> {
    app.state()?.reservations.cancel(&token, id, expected_version, exception)
}

#[tauri::command]
pub fn mark_reservation_no_show(
    app: State<'_, App>,
    token: String,
    id: i64,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.reservations.mark_no_show(&token, id, expected_version)
}

#[tauri::command]
pub fn list_reservations_needing_review(app: State<'_, App>, token: String) -> Result<Vec<ReviewItem>, AppError> {
    app.state()?.reservations.list_needing_review(&token)
}

#[tauri::command]
pub fn apply_reservation_discount(
    app: State<'_, App>,
    token: String,
    id: i64,
    input: DiscountInput,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.reservations.apply_discount(&token, id, input, expected_version)
}

#[tauri::command]
pub fn confirm_reservation(app: State<'_, App>, token: String, id: i64, expected_version: i64) -> Result<(), AppError> {
    app.state()?.reservations.confirm(&token, id, expected_version)
}

#[tauri::command]
pub fn record_reservation_deposit(
    app: State<'_, App>,
    token: String,
    id: i64,
    amount_cents: i64,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.reservations.record_deposit(&token, id, amount_cents, expected_version)
}

#[tauri::command]
pub fn set_reservation_notes(
    app: State<'_, App>,
    token: String,
    id: i64,
    notes: String,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.reservations.set_notes(&token, id, &notes, expected_version)
}
