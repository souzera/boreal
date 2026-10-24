use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::page::Page;
use crate::models::reservation::{
    AvailabilityResult, CreatedReservation, NewReservation, Reservation, ReservationDetail, ReservationQuery,
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
