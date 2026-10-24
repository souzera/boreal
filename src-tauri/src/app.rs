use std::path::Path;

use crate::db::keystore::KeyStore;
use crate::db::open_database;
use crate::error::AppError;
use crate::migrations::{current_version, last_applied_at, run_migrations};
use crate::services::auth_service::AuthService;
use crate::services::catalog_service::CatalogService;
use crate::services::guest_service::GuestService;
use crate::services::pousada_service::PousadaService;
use crate::services::settings_service::SettingsService;
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
    run_migrations(&mut conn)?;
    let ctx = Ctx::production(conn)?;
    Ok(AppState {
        auth: AuthService::new(ctx.clone()),
        users: UserService::new(ctx.clone()),
        settings: SettingsService::new(ctx.clone()),
        pousada: PousadaService::new(ctx.clone()),
        catalog: CatalogService::new(ctx.clone()),
        guests: GuestService::new(ctx.clone()),
        ctx,
    })
}

/// Estado gerenciado pelo Tauri: o app abre mesmo se a inicialização falhar,
/// para a interface poder explicar o problema.
pub struct App(pub Result<AppState, AppError>);

impl App {
    pub fn state(&self) -> Result<&AppState, AppError> {
        self.0.as_ref().map_err(|e| e.clone())
    }
}
