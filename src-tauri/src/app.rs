use std::path::Path;

use crate::db::keystore::KeyStore;
use crate::db::open_database;
use crate::domain::secret::SecretKey;
use crate::error::AppError;
use crate::models::health::StartupReport;
use crate::migrations::{current_version, last_applied_at, pending, run_with, version_or_zero, Migration, MIGRATIONS};
use crate::repositories::backup_repo;
use crate::services::backup_service::BackupKind;
use crate::services::alert_service::AlertService;
use crate::services::auth_service::AuthService;
use crate::services::backup_service::BackupService;
use crate::services::catalog_service::CatalogService;
use crate::services::guest_service::GuestService;
use crate::services::health_service::{check_database, HealthService};
use crate::services::payable_service::PayableService;
use crate::services::payment_service::PaymentService;
use crate::services::refund_service::RefundService;
use crate::services::item_service::ItemService;
use crate::services::policy_service::PolicyService;
use crate::services::pousada_service::PousadaService;
use crate::services::rate_service::RateService;
use crate::services::document_service::DocumentService;
use crate::services::management_service::ManagementService;
use crate::services::reception_service::ReceptionService;
use crate::services::recovery_service::RecoveryService;
use crate::services::report_service::ReportService;
use crate::services::reservation_service::ReservationService;
use crate::services::block_service::BlockService;
use crate::services::maintenance_service::MaintenanceService;
use crate::services::room_status_service::RoomStatusService;
use crate::services::cleaning_service::CleaningService;
use crate::services::closing_service::ClosingService;
use crate::services::room_map_service::RoomMapService;
use crate::services::search_service::SearchService;
use crate::services::settings_service::SettingsService;
use crate::services::stay_service::StayService;
use crate::services::user_service::UserService;
use crate::services::Ctx;

#[derive(Debug, serde::Serialize)]
pub struct AppInfo {
    pub app_version: &'static str,
    pub schema_version: u32,
    pub last_migration_at: Option<String>,
    /// Instante UTC do último backup válido.
    pub last_valid_backup_at: Option<String>,
}

/// Serviços prontos depois de abrir o banco criptografado e aplicar as migrações.
pub struct AppState {
    pub auth: AuthService,
    pub users: UserService,
    pub settings: SettingsService,
    pub pousada: PousadaService,
    pub catalog: CatalogService,
    pub guests: GuestService,
    pub items: ItemService,
    pub rates: RateService,
    pub policy: PolicyService,
    pub reservations: ReservationService,
    pub room_map: RoomMapService,
    pub stays: StayService,
    pub cleaning: CleaningService,
    pub blocks: BlockService,
    pub maintenance: MaintenanceService,
    pub room_status: RoomStatusService,
    pub payments: PaymentService,
    pub refunds: RefundService,
    pub payables: PayableService,
    pub closing: ClosingService,
    pub alerts: AlertService,
    pub search: SearchService,
    pub reception: ReceptionService,
    pub management: ManagementService,
    pub reports: ReportService,
    pub documents: DocumentService,
    pub recovery: RecoveryService,
    pub backup: BackupService,
    pub health: HealthService,
    ctx: Ctx,
}

impl AppState {
    pub fn app_info(&self) -> Result<AppInfo, AppError> {
        let conn = self.ctx.conn()?;
        Ok(AppInfo {
            app_version: env!("CARGO_PKG_VERSION"),
            schema_version: current_version(&conn)?,
            last_migration_at: last_applied_at(&conn)?,
            last_valid_backup_at: crate::repositories::backup_repo::last_valid_at(&conn).unwrap_or(None),
        })
    }

    pub fn schema_version(&self) -> Result<u32, AppError> {
        let conn = self.ctx.conn()?;
        current_version(&conn)
    }
}

/// Arquivo ao lado do banco que guarda a última tentativa de atualização que falhou (o banco, nesse caso, nem
/// sempre está disponível para registrar). Na próxima abertura bem-sucedida vira o evento `update_failed`.
const STATUS_FILE: &str = "startup-status.json";

fn write_failure(data_dir: &Path, at: &str, message: &str) {
    let body = serde_json::json!({ "failed_at": at, "message": message }).to_string();
    let _ = std::fs::write(data_dir.join(STATUS_FILE), body);
}

fn take_failure(data_dir: &Path) -> Option<(String, String)> {
    let path = data_dir.join(STATUS_FILE);
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    let _ = std::fs::remove_file(&path);
    Some((value.get("failed_at")?.as_str()?.to_string(), value.get("message")?.as_str()?.to_string()))
}

/// Abre o banco, aplica migrações pendentes e monta os serviços.
/// Nunca cria um banco novo por cima de um existente quando a chave falta.
pub fn initialize(db_path: &Path, keys: &dyn KeyStore) -> Result<AppState, AppError> {
    initialize_with(db_path, keys, MIGRATIONS)
}

/// Ordem: abrir → verificar integridade → (migrações pendentes num banco com dados) backup obrigatório →
/// migrar → registrar o resultado. Falha de backup ou de migração não altera o banco e deixa rastro para o alerta.
pub(crate) fn initialize_with(db_path: &Path, keys: &dyn KeyStore, migrations: &[Migration]) -> Result<AppState, AppError> {
    let conn = open_database(db_path, keys)?;
    // Verificação de integridade a cada abertura: banco com problema não abre normalmente (modo de recuperação).
    let integrity = check_database(&conn);
    if !integrity.ok {
        return Err(AppError::IntegrityProblem(integrity.problems.join(" | ")));
    }
    // O banco só abre com a chave; ela fica em memória (zerada ao sair) para backup e chave de recuperação.
    let key = keys.get()?.ok_or(AppError::KeyUnavailable)?;
    let data_dir = db_path.parent().map_or_else(|| std::path::PathBuf::from("."), std::path::Path::to_path_buf);
    let ctx = Ctx::production(conn, SecretKey::new(key), data_dir.clone())?;
    let backup = BackupService::new(ctx.clone(), db_path.to_path_buf());

    let (before, pending_count) = {
        let conn = ctx.conn()?;
        (version_or_zero(&conn), pending(&conn, migrations))
    };
    let now_text = || crate::services::format_utc(ctx.utc.now_utc());
    let mut pre_migration = None;
    if pending_count > 0 && before > 0 {
        // Banco com dados e mudança de estrutura: sem backup válido antes, nada é migrado.
        match backup.run_system(BackupKind::PreMigration) {
            Ok(run) => pre_migration = Some(run),
            Err(_) => {
                write_failure(&data_dir, &now_text(), "Não foi possível fazer o backup antes de atualizar o banco.");
                return Err(AppError::UpdateBackupFailed);
            }
        }
    }
    {
        let mut conn = ctx.conn()?;
        if let Err(error) = run_with(&mut conn, migrations) {
            if matches!(error, AppError::SchemaTooNew) {
                return Err(error);
            }
            let step = version_or_zero(&conn) + 1;
            write_failure(&data_dir, &now_text(), &format!("A etapa {step} da atualização do banco falhou."));
            return Err(AppError::UpdateFailed(format!("etapa {step}")));
        }
    }

    // Registro do resultado (o banco já está na versão nova, então as tabelas existem).
    {
        let conn = ctx.conn()?;
        if let Some(run) = pre_migration.as_ref().filter(|r| r.id == 0) {
            // A cópia foi feita antes de a tabela de histórico existir: registra agora.
            let _ = backup_repo::insert_run(
                &conn,
                &backup_repo::NewRun {
                    kind: run.kind.as_str(),
                    status: "ok",
                    started_at: &run.finished_at,
                    finished_at: &run.finished_at,
                    file_name: run.file_name.as_deref(),
                    bytes: run.bytes,
                    schema_version: Some(i64::from(before)),
                    message: "",
                },
            );
        }
        if let Some((failed_at, message)) = take_failure(&data_dir) {
            let _ = backup_repo::insert_event(&conn, "update_failed", &failed_at, &message);
        }
        if pending_count > 0 {
            let _ = backup_repo::insert_event(&conn, "update_ok", &now_text(), "");
        }
    }
    let schema_version = {
        let conn = ctx.conn()?;
        // Com o banco já migrado, aplica o tempo de inatividade configurado.
        if let Ok(minutes) = crate::repositories::settings_repo::get_u32(&conn, "idle_lock_minutes") {
            ctx.sessions.set_idle_lock(std::time::Duration::from_secs(u64::from(minutes) * 60));
        }
        current_version(&conn)?
    };
    let report = StartupReport {
        checked_at: crate::services::format_utc(ctx.utc.now_utc()),
        integrity_ok: true,
        problems: Vec::new(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version,
    };
    Ok(AppState {
        auth: AuthService::new(ctx.clone()),
        users: UserService::new(ctx.clone()),
        settings: SettingsService::new(ctx.clone()),
        pousada: PousadaService::new(ctx.clone()),
        catalog: CatalogService::new(ctx.clone()),
        guests: GuestService::new(ctx.clone()),
        items: ItemService::new(ctx.clone()),
        rates: RateService::new(ctx.clone()),
        policy: PolicyService::new(ctx.clone()),
        reservations: ReservationService::new(ctx.clone()),
        room_map: RoomMapService::new(ctx.clone()),
        stays: StayService::new(ctx.clone()),
        cleaning: CleaningService::new(ctx.clone()),
        blocks: BlockService::new(ctx.clone()),
        maintenance: MaintenanceService::new(ctx.clone()),
        room_status: RoomStatusService::new(ctx.clone()),
        payments: PaymentService::new(ctx.clone()),
        refunds: RefundService::new(ctx.clone()),
        payables: PayableService::new(ctx.clone()),
        closing: ClosingService::new(ctx.clone()),
        alerts: AlertService::new(ctx.clone()),
        search: SearchService::new(ctx.clone()),
        reception: ReceptionService::new(ctx.clone()),
        management: ManagementService::new(ctx.clone()),
        reports: ReportService::new(ctx.clone()),
        documents: DocumentService::new(ctx.clone()),
        recovery: RecoveryService::new(ctx.clone()),
        backup: BackupService::new(ctx.clone(), db_path.to_path_buf()),
        health: HealthService::new(ctx.clone()).with_startup(report),
        ctx,
    })
}

/// Estado gerenciado pelo Tauri: o app abre mesmo se a inicialização falhar, para a interface poder explicar
/// o problema, e o estado pode ser **substituído** depois de uma recuperação (nova máquina, banco restaurado).
pub struct App(std::sync::RwLock<Result<std::sync::Arc<AppState>, AppError>>);

impl App {
    pub fn new(result: Result<AppState, AppError>) -> Self {
        Self(std::sync::RwLock::new(result.map(std::sync::Arc::new)))
    }

    pub fn state(&self) -> Result<std::sync::Arc<AppState>, AppError> {
        self.0.read().map_err(|_| AppError::Database("o estado do aplicativo ficou indisponível".into()))?.clone()
    }

    /// Troca o estado (por exemplo, depois de restaurar um backup em modo de recuperação).
    pub fn replace(&self, result: Result<AppState, AppError>) {
        if let Ok(mut guard) = self.0.write() {
            *guard = result.map(std::sync::Arc::new);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::db::keystore::{KeyStore, MemoryKeyStore};
    use crate::services::alert_service::AlertService;

    /// Banco já com dados, parado numa versão antiga do esquema.
    fn old_db(dir: &Path, keys: &dyn KeyStore, upto: usize) -> PathBuf {
        let path = dir.join("boreal.db");
        let mut conn = open_database(&path, keys).unwrap();
        run_with(&mut conn, &MIGRATIONS[..upto]).unwrap();
        conn.execute(
            "INSERT INTO users (id, username, display_name, password_hash, role, active, must_change_password, created_at, updated_at)
             VALUES (1, 'admin', 'Administrador', 'x', 'admin', 1, 0, '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z')",
            [],
        )
        .unwrap();
        path
    }

    fn schema_of(path: &Path, keys: &dyn KeyStore) -> u32 {
        let conn = open_database(path, keys).unwrap();
        current_version(&conn).unwrap()
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir).map(|e| e.map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
        names.sort();
        names
    }

    fn broken_migrations() -> Vec<Migration> {
        let mut all = MIGRATIONS.to_vec();
        all.push(Migration { version: 15, sql: "CREATE TABLE sera_desfeita (x INTEGER); INSERT INTO tabela_que_nao_existe VALUES (1);" });
        all
    }

    #[test]
    fn pending_migrations_with_data_make_a_backup_first() {
        let dir = tempfile::tempdir().unwrap();
        let keys = MemoryKeyStore::empty();
        let path = old_db(dir.path(), &keys, 12);

        let app = initialize_with(&path, &keys, MIGRATIONS).unwrap();

        assert_eq!(app.schema_version().unwrap(), 14);
        let backups = files(&dir.path().join("backups"));
        assert_eq!(backups.len(), 1, "{backups:?}");
        assert!(backups[0].starts_with("boreal-seguranca-"), "{backups:?}");
        let secret = SecretKey::new(keys.get().unwrap().unwrap());
        let info = crate::services::backup_service::validate_file(&dir.path().join("backups").join(&backups[0]), &secret).unwrap();
        assert_eq!(info.schema_version, 12, "a cópia guarda o banco como estava antes de migrar");
        let conn = app.ctx.conn().unwrap();
        let kinds: Vec<String> = conn.prepare("SELECT kind FROM backup_runs").unwrap().query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect();
        let events: Vec<String> = conn.prepare("SELECT kind FROM system_events").unwrap().query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect();
        assert_eq!(kinds, ["pre_migration"], "registrada depois, quando a tabela passou a existir");
        assert_eq!(events, ["update_ok"]);
    }

    #[test]
    fn a_fresh_database_migrates_without_a_backup() {
        let dir = tempfile::tempdir().unwrap();

        let app = initialize_with(&dir.path().join("boreal.db"), &MemoryKeyStore::empty(), MIGRATIONS).unwrap();

        assert_eq!(app.schema_version().unwrap(), 14);
        assert!(!dir.path().join("backups").exists(), "banco novo não tem o que proteger");
    }

    #[test]
    fn if_the_pre_migration_backup_fails_nothing_is_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let keys = MemoryKeyStore::empty();
        let path = old_db(dir.path(), &keys, 12);
        std::fs::write(dir.path().join("backups"), b"arquivo no lugar da pasta").unwrap();

        let result = initialize_with(&path, &keys, MIGRATIONS);

        assert!(matches!(result, Err(AppError::UpdateBackupFailed)), "{:?}", result.err());
        assert_eq!(schema_of(&path, &keys), 12, "o banco continua como estava");
        assert!(dir.path().join(STATUS_FILE).exists(), "a falha deixa rastro para o alerta");
    }

    #[test]
    fn a_failing_migration_leaves_the_previous_database_intact_and_writes_the_status_file() {
        let dir = tempfile::tempdir().unwrap();
        let keys = MemoryKeyStore::empty();
        let path = old_db(dir.path(), &keys, 14);

        let result = initialize_with(&path, &keys, &broken_migrations());

        assert!(matches!(&result, Err(AppError::UpdateFailed(m)) if m.contains("15")), "{:?}", result.err());
        assert_eq!(schema_of(&path, &keys), 14);
        let conn = open_database(&path, &keys).unwrap();
        let leftover: i64 = conn.query_row("SELECT count(*) FROM sqlite_master WHERE name = 'sera_desfeita'", [], |r| r.get(0)).unwrap();
        assert_eq!(leftover, 0, "a etapa que falhou foi desfeita por inteiro");
        let status = std::fs::read_to_string(dir.path().join(STATUS_FILE)).unwrap();
        assert!(status.contains("failed_at") && status.contains("etapa 15"), "{status}");
        assert_eq!(files(&dir.path().join("backups")).len(), 1, "o backup de antes da atualização foi feito");
    }

    #[test]
    fn the_next_successful_start_records_update_failed_and_the_alert_clears_after_a_later_update_ok() {
        let dir = tempfile::tempdir().unwrap();
        let keys = MemoryKeyStore::empty();
        let path = old_db(dir.path(), &keys, 14);
        assert!(initialize_with(&path, &keys, &broken_migrations()).is_err());

        let app = initialize_with(&path, &keys, MIGRATIONS).unwrap();

        assert!(!dir.path().join(STATUS_FILE).exists(), "o rastro foi consumido");
        let admin = app.ctx.sessions.create(1, crate::models::user::Role::Admin, app.ctx.clock.now());
        let alerts: &AlertService = &app.alerts;
        let kinds = |alerts: &AlertService| -> Vec<String> {
            alerts.refresh(&admin).unwrap();
            let query = crate::models::alert::AlertQuery { status: Some("active".into()), kind: None, severity: None, page: 1, page_size: 50 };
            let mut k: Vec<String> = alerts.list(&admin, query).unwrap().items.into_iter().map(|a| a.kind).collect();
            k.sort();
            k
        };
        assert_eq!(kinds(alerts), ["update_failed"]);

        {
            let conn = app.ctx.conn().unwrap();
            backup_repo::insert_event(&conn, "update_ok", "2999-01-01T00:00:00Z", "").unwrap();
        }
        assert!(kinds(alerts).is_empty(), "uma atualização concluída depois resolve o alerta");
    }

    #[test]
    fn a_newer_database_is_refused_without_a_backup_or_any_change() {
        let dir = tempfile::tempdir().unwrap();
        let keys = MemoryKeyStore::empty();
        let path = old_db(dir.path(), &keys, 14);
        {
            let conn = open_database(&path, &keys).unwrap();
            conn.execute("INSERT INTO schema_migrations (version, applied_at) VALUES (99, 'x')", []).unwrap();
        }

        let result = initialize_with(&path, &keys, MIGRATIONS);

        assert!(matches!(result, Err(AppError::SchemaTooNew)), "{:?}", result.err());
        assert_eq!(schema_of(&path, &keys), 99);
        assert!(!dir.path().join("backups").exists());
        assert!(AppError::SchemaTooNew.to_string().contains("versão mais nova"));
    }

    #[test]
    fn about_info_includes_the_last_valid_backup() {
        let dir = tempfile::tempdir().unwrap();
        let app = initialize_with(&dir.path().join("boreal.db"), &MemoryKeyStore::empty(), MIGRATIONS).unwrap();
        assert!(app.app_info().unwrap().last_valid_backup_at.is_none());

        app.backup.run_system(BackupKind::Manual).unwrap();

        let info = app.app_info().unwrap();
        assert!(info.last_valid_backup_at.is_some_and(|t| t.ends_with('Z')));
        assert_eq!(info.schema_version, 14);
    }
}
