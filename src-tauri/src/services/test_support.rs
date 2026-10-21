use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;

use super::auth_service::AuthService;
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
        users: UserService::new(ctx),
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
