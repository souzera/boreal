use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::payment::{PaymentInput, PaymentReceipt, PaymentTarget};

#[tauri::command]
pub fn record_stay_payment(app: State<'_, App>, token: String, stay_id: i64, input: PaymentInput) -> Result<PaymentReceipt, AppError> {
    app.state()?.payments.record(&token, PaymentTarget::Stay { stay_id }, input)
}

#[tauri::command]
pub fn record_reservation_payment(
    app: State<'_, App>,
    token: String,
    reservation_id: i64,
    expected_version: i64,
    input: PaymentInput,
) -> Result<PaymentReceipt, AppError> {
    app.state()?.payments.record(&token, PaymentTarget::Reservation { reservation_id, expected_version }, input)
}
