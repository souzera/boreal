pub mod app;
pub mod audit;
pub mod auth;
pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod migrations;
pub mod models;
#[cfg(feature = "dev-seed")]
pub mod seed;
pub mod repositories;
pub mod services;

use tauri::Manager;

use crate::db::keystore::OsKeyStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|tauri_app| {
            let dir = tauri_app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let state = app::initialize(&dir.join("boreal.db"), &OsKeyStore::new());
            if let Err(e) = &state {
                eprintln!("falha ao iniciar: {e}");
            }
            if let Ok(state) = &state {
                // Se o último backup válido tem mais de 24 h, faz um agora, sem atrasar a abertura.
                state.backup.catch_up();
            }
            tauri_app.manage(app::App::new(state));
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
            commands::auth::my_permissions,
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
            commands::items::list_items,
            commands::items::create_item,
            commands::items::update_item,
            commands::items::set_item_active,
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
            commands::reservations::apply_reservation_discount,
            commands::reservations::confirm_reservation,
            commands::reservations::set_reservation_notes,
            commands::room_map::room_map,
            commands::cleaning::cleaning_queue,
            commands::cleaning::start_cleaning,
            commands::cleaning::complete_cleaning,
            commands::cleaning::set_cleaning_priority,
            commands::blocks::list_blocks,
            commands::blocks::create_block,
            commands::blocks::release_block,
            commands::maintenance::list_maintenance,
            commands::maintenance::get_maintenance,
            commands::maintenance::open_maintenance,
            commands::maintenance::apply_maintenance_unavailability,
            commands::maintenance::resolve_maintenance_conflict,
            commands::maintenance::start_maintenance,
            commands::maintenance::request_maintenance_resolution,
            commands::maintenance::complete_maintenance,
            commands::room_status::list_room_status,
            commands::room_status::set_room_cleaning_interval,
            commands::stays::check_in_reservation,
            commands::stays::walk_in,
            commands::stays::add_stay_charge,
            commands::stays::add_stay_adjustment,
            commands::stays::reverse_stay_charge,
            commands::payments::record_stay_payment,
            commands::payments::record_reservation_payment,
            commands::refunds::record_refund,
            commands::payables::list_payables,
            commands::payables::get_payable,
            commands::payables::create_payable,
            commands::payables::pay_payable,
            commands::payables::payables_summary,
            commands::payables::list_expense_categories,
            commands::payables::create_expense_category,
            commands::payables::rename_expense_category,
            commands::payables::set_expense_category_active,
            commands::closing::closing_preview,
            commands::closing::close_cash,
            commands::closing::closing_pending_dates,
            commands::alerts::refresh_alerts,
            commands::alerts::list_alerts,
            commands::alerts::alert_summary,
            commands::alerts::alert_history,
            commands::alerts::dismiss_alert,
            commands::search::global_search,
            commands::reception::reception_overview,
            commands::management::management_dashboard,
            commands::recovery::reveal_recovery_key,
            commands::recovery::restore_on_new_computer,
            commands::recovery::recover_corrupted,
            commands::backup::preview_restore,
            commands::backup::restore_backup,
            commands::health::disk_status,
            commands::health::last_startup_report,
            commands::backup::run_backup,
            commands::backup::list_backup_files,
            commands::backup::recent_backup_runs,
            commands::backup::backup_destination,
            commands::backup::set_backup_destination,
            commands::reports::run_report,
            commands::reports::export_report_csv,
            commands::reports::export_report_pdf,
            commands::reports::open_exported_file,
            commands::documents::issue_document,
            commands::documents::list_documents,
            commands::documents::save_document_pdf,
            commands::stays::preview_stay_checkout,
            commands::stays::checkout_stay,
            commands::stays::set_stay_late_tolerance,
            commands::stays::get_stay,
            commands::stays::list_stays,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
