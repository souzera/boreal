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
