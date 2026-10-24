use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;

use super::auth_service::AuthService;
use super::catalog_service::CatalogService;
use super::guest_service::GuestService;
use super::rate_service::RateService;
use super::reservation_service::ReservationService;
use super::policy_service::PolicyService;
use super::pousada_service::PousadaService;
use super::settings_service::SettingsService;
use super::user_service::UserService;
use super::{Ctx, UtcClock};
use crate::auth::session::{Clock, SessionManager};
use crate::migrations::run_migrations;

pub struct FakeUtc(Mutex<DateTime<Utc>>);

impl FakeUtc {
    pub fn set(&self, at: DateTime<Utc>) {
        *self.0.lock().unwrap() = at;
    }
    pub fn advance(&self, by: chrono::Duration) {
        *self.0.lock().unwrap() += by;
    }
}

impl UtcClock for FakeUtc {
    fn now_utc(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }

    // Nos testes a pousada está sempre em UTC-03:00, qualquer que seja o fuso da máquina.
    fn local_offset(&self) -> chrono::FixedOffset {
        chrono::FixedOffset::west_opt(3 * 3600).unwrap()
    }
}

pub struct FakeClock(Mutex<Instant>);

impl FakeClock {
    pub fn advance(&self, by: Duration) {
        *self.0.lock().unwrap() += by;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap()
    }
}

pub struct Harness {
    pub auth: AuthService,
    pub users: UserService,
    pub settings: SettingsService,
    pub pousada: PousadaService,
    pub catalog: CatalogService,
    pub guests: GuestService,
    pub rates: RateService,
    pub policy: PolicyService,
    pub reservations: ReservationService,
    pub utc: Arc<FakeUtc>,
    pub clock: Arc<FakeClock>,
    pub db: Arc<Mutex<Connection>>,
}

pub const ADMIN_PASSWORD: &str = "Admin-Senha-1";

pub fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 21, 12, 0, 0).unwrap()
}

pub fn harness() -> Harness {
    let mut conn = Connection::open_in_memory().unwrap();
    run_migrations(&mut conn).unwrap();
    let db = Arc::new(Mutex::new(conn));
    let utc = Arc::new(FakeUtc(Mutex::new(t0())));
    let clock = Arc::new(FakeClock(Mutex::new(Instant::now())));
    let ctx = Ctx {
        db: db.clone(),
        sessions: Arc::new(SessionManager::new(Duration::from_secs(600))),
        clock: clock.clone(),
        utc: utc.clone(),
    };
    Harness {
        auth: AuthService::new(ctx.clone()),
        users: UserService::new(ctx.clone()),
        settings: SettingsService::new(ctx.clone()),
        pousada: PousadaService::new(ctx.clone()),
        catalog: CatalogService::new(ctx.clone()),
        guests: GuestService::new(ctx.clone()),
        rates: RateService::new(ctx.clone()),
        policy: PolicyService::new(ctx.clone()),
        reservations: ReservationService::new(ctx),
        utc,
        clock,
        db,
    }
}

/// Cria o administrador inicial (`admin`) e entra com ele; retorna o token.
pub fn admin_token(h: &Harness) -> String {
    h.auth
        .bootstrap_admin("admin", "Administrador", ADMIN_PASSWORD)
        .unwrap();
    h.auth.login("admin", ADMIN_PASSWORD).unwrap().token
}

/// Cria um usuário da recepção (`rec`) já com a senha definitiva e entra com ele.
pub fn recepcao_token(h: &Harness, admin: &str) -> String {
    use crate::models::user::Role;
    use crate::services::user_service::NewUserInput;
    h.users
        .create_user(
            admin,
            NewUserInput {
                username: "rec".into(),
                display_name: "Recepção".into(),
                role: Role::Recepcao,
                initial_password: "Temp-Senha-123".into(),
            },
        )
        .unwrap();
    let token = h.auth.login("rec", "Temp-Senha-123").unwrap().token;
    h.auth
        .change_own_password(&token, "Temp-Senha-123", "Definitiva-456")
        .unwrap();
    token
}
