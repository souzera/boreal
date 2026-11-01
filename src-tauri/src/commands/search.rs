use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::search::SearchResults;

#[tauri::command]
pub fn global_search(app: State<'_, App>, token: String, text: String) -> Result<SearchResults, AppError> {
    app.state()?.search.search(&token, &text)
}
