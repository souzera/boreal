use tauri::State;

use crate::app::App;
use crate::error::AppError;
use crate::models::document::DocumentView;
use crate::models::report::ExportResult;

/// Emite (ou devolve a versão vigente de) um documento interno.
#[tauri::command]
pub fn issue_document(app: State<'_, App>, token: String, kind: String, source_id: i64) -> Result<DocumentView, AppError> {
    app.state()?.documents.issue(&token, &kind, source_id)
}

#[tauri::command]
pub fn list_documents(app: State<'_, App>, token: String, kind: String, source_id: i64) -> Result<Vec<DocumentView>, AppError> {
    app.state()?.documents.list(&token, &kind, source_id)
}

/// Grava o PDF de uma versão já emitida no caminho escolhido; falhar aqui não emite nada de novo.
#[tauri::command]
pub fn save_document_pdf(app: State<'_, App>, token: String, document_id: i64, path: String) -> Result<ExportResult, AppError> {
    app.state()?.documents.save_pdf(&token, document_id, &path)
}
