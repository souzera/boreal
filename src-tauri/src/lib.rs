pub mod app;
pub mod audit;
pub mod auth;
pub mod commands;
pub mod db;
pub mod error;
pub mod migrations;
pub mod models;
pub mod repositories;
pub mod services;

use tauri::Manager;

use crate::db::keystore::OsKeyStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|tauri_app| {
            let dir = tauri_app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let state = app::initialize(&dir.join("boreal.db"), &OsKeyStore::new());
            if let Err(e) = &state {
                eprintln!("falha ao iniciar: {e}");
            }
            tauri_app.manage(app::App(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::startup_status,
            commands::app::get_app_info,
            commands::auth::needs_bootstrap,
            commands::auth::bootstrap_admin,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::change_own_password,
            commands::auth::unlock_session,
            commands::auth::session_status,
            commands::auth::create_user,
            commands::auth::reset_password,
            commands::users::grantable_permissions,
            commands::users::set_extra_permissions,
            commands::settings::get_settings,
            commands::settings::get_session_policy,
            commands::settings::update_settings,
            commands::pousada::get_pousada,
            commands::pousada::get_pousada_logo,
            commands::pousada::update_pousada,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
