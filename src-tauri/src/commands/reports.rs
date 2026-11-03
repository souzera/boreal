use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::app::App;
use crate::domain::report::ReportTable;
use crate::error::AppError;
use crate::models::report::{ExportResult, ReportQuery};

#[tauri::command]
pub fn run_report(app: State<'_, App>, token: String, query: ReportQuery) -> Result<ReportTable, AppError> {
    app.state()?.reports.run(&token, query)
}

#[tauri::command]
pub fn export_report_csv(app: State<'_, App>, token: String, query: ReportQuery, delimiter: String, path: String) -> Result<ExportResult, AppError> {
    app.state()?.reports.export_csv(&token, query, &delimiter, &path)
}

#[tauri::command]
pub fn export_report_pdf(app: State<'_, App>, token: String, query: ReportQuery, path: String) -> Result<ExportResult, AppError> {
    app.state()?.reports.export_pdf(&token, query, &path)
}

/// Abre um arquivo exportado (`.pdf` ou `.csv`) no leitor padrão; de lá a pessoa imprime.
#[tauri::command]
pub fn open_exported_file(handle: AppHandle, app: State<'_, App>, token: String, path: String) -> Result<(), AppError> {
    app.state()?.reports.check_openable(&token, &path)?;
    handle
        .opener()
        .open_path(&path, None::<&str>)
        .map_err(|_| AppError::Validation("Não foi possível abrir o arquivo. Procure-o na pasta onde foi salvo.".into()))
}
