use boreal_pms_lib::app::initialize;
use boreal_pms_lib::db::keystore::MemoryKeyStore;
use boreal_pms_lib::error::AppError;
use boreal_pms_lib::models::user::Role;
use boreal_pms_lib::services::user_service::NewUserInput;

const ADMIN_PASSWORD: &str = "Admin-Senha-1";

#[test]
fn startup_creates_db_and_runs_migrations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boreal.db");

    let app = initialize(&path, &MemoryKeyStore::empty()).unwrap();

    assert!(path.exists());
    assert!(app.schema_version().unwrap() >= 1);
    assert!(app.auth.needs_bootstrap().unwrap());
}

#[test]
fn app_info_reports_version_schema_and_last_migration() {
    let dir = tempfile::tempdir().unwrap();
    let app = initialize(&dir.path().join("boreal.db"), &MemoryKeyStore::empty()).unwrap();

    let info = app.app_info().unwrap();

    assert_eq!(info.app_version, env!("CARGO_PKG_VERSION"));
    assert!(info.schema_version >= 3);
    let at = info.last_migration_at.expect("há migração aplicada");
    assert!(at.ends_with('Z') && at.len() == 20, "formato RFC 3339 UTC: {at}");
}

#[test]
fn startup_with_missing_key_reports_error_and_leaves_file_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boreal.db");
    drop(initialize(&path, &MemoryKeyStore::empty()).unwrap());
    let before = std::fs::read(&path).unwrap();

    let result = initialize(&path, &MemoryKeyStore::empty());

    assert!(matches!(result, Err(AppError::KeyUnavailable)));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn full_flow() {
    let dir = tempfile::tempdir().unwrap();
    let app = initialize(&dir.path().join("boreal.db"), &MemoryKeyStore::empty()).unwrap();

    app.auth
        .bootstrap_admin("admin", "Administrador", ADMIN_PASSWORD)
        .unwrap();
    let admin = app.auth.login("admin", ADMIN_PASSWORD).unwrap();
    app.users
        .create_user(
            &admin.token,
            NewUserInput {
                username: "maria".into(),
                display_name: "Maria".into(),
                role: Role::Recepcao,
                initial_password: "Temp-Senha-123".into(),
            },
        )
        .unwrap();
    let maria = app.auth.login("maria", "Temp-Senha-123").unwrap();
    app.auth
        .change_own_password(&maria.token, "Temp-Senha-123", "Definitiva-456")
        .unwrap();

    let denied = app.users.create_user(
        &maria.token,
        NewUserInput {
            username: "joao".into(),
            display_name: "João".into(),
            role: Role::Recepcao,
            initial_password: "Temp-Senha-123".into(),
        },
    );
    assert!(matches!(denied, Err(AppError::Forbidden)));
}

#[test]
fn startup_checks_integrity_and_reports_it() {
    let dir = tempfile::tempdir().unwrap();
    let app = initialize(&dir.path().join("boreal.db"), &MemoryKeyStore::empty()).unwrap();
    app.auth.bootstrap_admin("admin", "Administrador", ADMIN_PASSWORD).unwrap();
    let admin = app.auth.login("admin", ADMIN_PASSWORD).unwrap();

    let report = app.health.last_startup(&admin.token).unwrap().expect("há relatório da abertura");

    assert!(report.integrity_ok && report.problems.is_empty());
    assert_eq!(report.app_version, env!("CARGO_PKG_VERSION"));
    assert!(report.schema_version >= 14);
}

#[test]
fn a_corrupted_database_does_not_open_normally_and_the_file_is_not_modified() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boreal.db");
    let keys = MemoryKeyStore::empty();
    drop(initialize(&path, &keys).unwrap());
    let mut bytes = std::fs::read(&path).unwrap();
    let len = bytes.len();
    for b in &mut bytes[len / 2..len / 2 + 4096] {
        *b ^= 0xA5;
    }
    std::fs::write(&path, &bytes).unwrap();
    let before = std::fs::read(&path).unwrap();

    let result = initialize(&path, &keys);

    assert!(matches!(result, Err(AppError::IntegrityProblem(_)) | Err(AppError::KeyInvalid)), "{:?}", result.err());
    assert_eq!(std::fs::read(&path).unwrap(), before, "o arquivo com problema não é alterado");
}

#[test]
fn the_app_state_can_be_replaced_after_a_recovery() {
    use boreal_pms_lib::app::App;
    let dir = tempfile::tempdir().unwrap();
    let app = App::new(Err(AppError::IntegrityProblem("x".into())));
    assert!(matches!(app.state(), Err(AppError::IntegrityProblem(_))));

    app.replace(initialize(&dir.path().join("boreal.db"), &MemoryKeyStore::empty()));

    assert!(app.state().unwrap().auth.needs_bootstrap().unwrap());
}
