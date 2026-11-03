use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::refund::{RefundInput, RefundReceipt};

#[tauri::command]
pub fn record_refund(app: State<'_, App>, token: String, input: RefundInput) -> Result<RefundReceipt, AppError> {
    app.state()?.refunds.record(&token, input)
}
