use std::path::Path;

use tauri::{AppHandle, Manager, State};

use crate::app::{initialize, App};
use crate::db::keystore::OsKeyStore;
use crate::error::AppError;
use crate::services::recovery_service::RecoveryService;

fn database_path(handle: &AppHandle) -> Result<std::path::PathBuf, AppError> {
    let dir = handle.path().app_data_dir().map_err(|_| AppError::Database("pasta de dados indisponível".into()))?;
    Ok(dir.join("boreal.db"))
}

/// Mostra a chave de recuperação ao administrador, que digita a senha de novo.
#[tauri::command]
pub fn reveal_recovery_key(app: State<'_, App>, token: String, password: String) -> Result<String, AppError> {
    app.state()?.recovery.reveal(&token, &password)
}

/// Troca de computador (sem sessão: ainda não existe banco aqui): backup + chave de recuperação.
#[tauri::command]
pub fn restore_on_new_computer(handle: AppHandle, app: State<'_, App>, backup_path: String, recovery_key: String) -> Result<(), AppError> {
    let db = database_path(&handle)?;
    let keys = OsKeyStore::new();
    RecoveryService::restore_on_new_computer(Path::new(&backup_path), &recovery_key, &db, &keys)?;
    app.replace(initialize(&db, &keys));
    app.state().map(|_| ())
}

/// Perda da credencial do Windows: a chave de recuperação reabre o banco que já existe neste computador.
#[tauri::command]
pub fn unlock_with_recovery_key(handle: AppHandle, app: State<'_, App>, recovery_key: String) -> Result<(), AppError> {
    let db = database_path(&handle)?;
    let keys = OsKeyStore::new();
    RecoveryService::unlock_with_recovery_key(&db, &recovery_key, &keys)?;
    app.replace(initialize(&db, &keys));
    app.state().map(|_| ())
}

/// Modo de recuperação (banco com problema): backup + chave de recuperação + confirmação digitada.
#[tauri::command]
pub fn recover_corrupted(handle: AppHandle, app: State<'_, App>, backup_path: String, recovery_key: String, confirmation: String) -> Result<(), AppError> {
    let db = database_path(&handle)?;
    let keys = OsKeyStore::new();
    let now = chrono::Local::now().naive_local();
    RecoveryService::recover_corrupted(Path::new(&backup_path), &recovery_key, &confirmation, &db, &keys, now)?;
    app.replace(initialize(&db, &keys));
    app.state().map(|_| ())
}
