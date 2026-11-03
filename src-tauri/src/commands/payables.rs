use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::page::Page;
use crate::models::payable::{
    CategoryTotal, ExpenseCategory, NewPayable, PayInput, PayableDetail, PayableQuery, PayableView, SummaryQuery,
};

#[tauri::command]
pub fn list_payables(app: State<'_, App>, token: String, query: PayableQuery) -> Result<Page<PayableView>, AppError> {
    app.state()?.payables.list(&token, query)
}

#[tauri::command]
pub fn get_payable(app: State<'_, App>, token: String, id: i64) -> Result<PayableDetail, AppError> {
    app.state()?.payables.get(&token, id)
}

#[tauri::command]
pub fn create_payable(app: State<'_, App>, token: String, input: NewPayable) -> Result<i64, AppError> {
    app.state()?.payables.create(&token, input)
}

#[tauri::command]
pub fn pay_payable(app: State<'_, App>, token: String, id: i64, input: PayInput, expected_version: i64) -> Result<i64, AppError> {
    app.state()?.payables.pay(&token, id, input, expected_version)
}

#[tauri::command]
pub fn payables_summary(app: State<'_, App>, token: String, query: SummaryQuery) -> Result<Vec<CategoryTotal>, AppError> {
    app.state()?.payables.summary(&token, query)
}

#[tauri::command]
pub fn list_expense_categories(app: State<'_, App>, token: String, only_active: bool) -> Result<Vec<ExpenseCategory>, AppError> {
    app.state()?.payables.list_categories(&token, only_active)
}

#[tauri::command]
pub fn create_expense_category(app: State<'_, App>, token: String, name: String) -> Result<i64, AppError> {
    app.state()?.payables.create_category(&token, &name)
}

#[tauri::command]
pub fn rename_expense_category(app: State<'_, App>, token: String, id: i64, name: String, expected_version: i64) -> Result<(), AppError> {
    app.state()?.payables.rename_category(&token, id, &name, expected_version)
}

#[tauri::command]
pub fn set_expense_category_active(
    app: State<'_, App>,
    token: String,
    id: i64,
    active: bool,
    expected_version: i64,
) -> Result<(), AppError> {
    app.state()?.payables.set_category_active(&token, id, active, expected_version)
}
