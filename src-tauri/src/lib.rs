pub mod app;
pub mod audit;
pub mod auth;
pub mod commands;
pub mod db;
pub mod domain;
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
            commands::users::list_users,
            commands::users::set_user_role,
            commands::users::set_user_active,
            commands::settings::get_settings,
            commands::settings::get_session_policy,
            commands::settings::update_settings,
            commands::pousada::get_pousada,
            commands::pousada::get_pousada_logo,
            commands::pousada::update_pousada,
            commands::catalog::list_categories,
            commands::catalog::create_category,
            commands::catalog::update_category,
            commands::catalog::set_category_active,
            commands::catalog::delete_category,
            commands::catalog::list_rooms,
            commands::catalog::create_room,
            commands::catalog::update_room,
            commands::catalog::set_room_active,
            commands::catalog::delete_room,
            commands::guests::list_guests,
            commands::guests::get_guest,
            commands::guests::find_guest_duplicates,
            commands::guests::create_guest,
            commands::guests::update_guest,
            commands::guests::set_guest_active,
            commands::rates::list_rate_rules,
            commands::rates::create_rate_rule,
            commands::rates::update_rate_rule,
            commands::rates::set_rate_rule_active,
            commands::rates::delete_rate_rule,
            commands::rates::list_holidays,
            commands::rates::create_holiday,
            commands::rates::delete_holiday,
            commands::rates::quote_stay,
            commands::policy::get_cancellation_policy,
            commands::policy::update_cancellation_policy,
            commands::reservations::create_reservation,
            commands::reservations::get_reservation,
            commands::reservations::list_reservations,
            commands::reservations::check_room_availability,
            commands::reservations::preview_reservation_change,
            commands::reservations::apply_reservation_change,
            commands::reservations::preview_reservation_cancellation,
            commands::reservations::cancel_reservation,
            commands::reservations::mark_reservation_no_show,
            commands::reservations::list_reservations_needing_review,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
