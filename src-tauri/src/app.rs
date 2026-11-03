use std::path::Path;

use crate::db::keystore::KeyStore;
use crate::db::open_database;
use crate::domain::secret::SecretKey;
use crate::error::AppError;
use crate::models::health::StartupReport;
use crate::migrations::{current_version, last_applied_at, run_migrations};
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
        })
    }

    pub fn schema_version(&self) -> Result<u32, AppError> {
        let conn = self.ctx.conn()?;
        current_version(&conn)
    }
}

/// Abre o banco, aplica migrações pendentes e monta os serviços.
/// Nunca cria um banco novo por cima de um existente quando a chave falta.
pub fn initialize(db_path: &Path, keys: &dyn KeyStore) -> Result<AppState, AppError> {
    let mut conn = open_database(db_path, keys)?;
    // Verificação de integridade a cada abertura: banco com problema não abre normalmente (modo de recuperação).
    let integrity = check_database(&conn);
    if !integrity.ok {
        return Err(AppError::IntegrityProblem(integrity.problems.join(" | ")));
    }
    run_migrations(&mut conn)?;
    // O banco só abre com a chave; ela fica em memória (zerada ao sair) para backup e chave de recuperação.
    let key = keys.get()?.ok_or(AppError::KeyUnavailable)?;
    let data_dir = db_path.parent().map_or_else(|| std::path::PathBuf::from("."), std::path::Path::to_path_buf);
    let ctx = Ctx::production(conn, SecretKey::new(key), data_dir)?;
    let schema_version = {
        let conn = ctx.conn()?;
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
