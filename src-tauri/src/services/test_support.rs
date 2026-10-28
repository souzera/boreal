use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;

use super::auth_service::AuthService;
use super::block_service::BlockService;
use super::maintenance_service::MaintenanceService;
use super::room_status_service::RoomStatusService;
use super::catalog_service::CatalogService;
use super::cleaning_service::CleaningService;
use super::guest_service::GuestService;
use super::item_service::ItemService;
use super::rate_service::RateService;
use super::reservation_service::ReservationService;
use super::room_map_service::RoomMapService;
use super::policy_service::PolicyService;
use super::pousada_service::PousadaService;
use super::settings_service::SettingsService;
use super::stay_service::StayService;
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
        items: ItemService::new(ctx.clone()),
        rates: RateService::new(ctx.clone()),
        policy: PolicyService::new(ctx.clone()),
        reservations: ReservationService::new(ctx.clone()),
        room_map: RoomMapService::new(ctx.clone()),
        stays: StayService::new(ctx.clone()),
        cleaning: CleaningService::new(ctx.clone()),
        blocks: BlockService::new(ctx.clone()),
        maintenance: MaintenanceService::new(ctx.clone()),
        room_status: RoomStatusService::new(ctx),
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

/// Categoria, quarto "101", hóspede e tarifa de todos os dias da semana, para testes que precisam de reservas.
pub struct Seed {
    pub category_id: i64,
    pub room_id: i64,
    pub guest_id: i64,
}

pub fn seed(h: &Harness, admin: &str, nightly_cents: i64, capacity: i64) -> Seed {
    use crate::models::catalog::{NewCategory, NewRoom};
    use crate::models::guest::GuestInput;
    use crate::models::rate::NewRateRule;
    use crate::services::guest_service::CreateGuestOutcome;

    let category_id = h
        .catalog
        .create_category(admin, NewCategory { name: "Standard".into(), description: String::new() })
        .unwrap();
    for day in 0..7 {
        h.rates
            .create_rule(
                admin,
                NewRateRule {
                    category_id,
                    rule_type: "weekday".into(),
                    weekday: Some(day),
                    start_date: None,
                    end_date: None,
                    date: None,
                    amount_cents: nightly_cents,
                    label: String::new(),
                },
            )
            .unwrap();
    }
    let room_id = h
        .catalog
        .create_room(
            admin,
            NewRoom {
                code: "101".into(),
                category_id,
                max_capacity: capacity,
                description: String::new(),
                notes: String::new(),
            },
        )
        .unwrap();
    let guest_id = match h
        .guests
        .create(
            admin,
            GuestInput {
                full_name: "Ana Souza".into(),
                phone: "(11) 99999-0000".into(),
                cpf: None,
                email: String::new(),
                address: String::new(),
            },
            true,
        )
        .unwrap()
    {
        CreateGuestOutcome::Created { id, .. } => id,
        other => panic!("{other:?}"),
    };
    Seed { category_id, room_id, guest_id }
}

/// Cria uma reserva pendente do hóspede do `Seed` no quarto "101" e devolve o id.
pub fn book(h: &Harness, admin: &str, seed: &Seed, from: &str, to: &str) -> i64 {
    use crate::models::reservation::NewReservation;
    h.reservations
        .create(
            admin,
            NewReservation {
                room_id: seed.room_id,
                guest_id: seed.guest_id,
                check_in_date: from.into(),
                check_out_date: to.into(),
                expected_check_in_time: None,
                expected_check_out_time: None,
                occupants: 1,
                notes: String::new(),
                exceed_capacity: false,
            },
        )
        .unwrap()
        .id
}

/// Hospedagem em 21→23/10 com a conta quitada e o check-out feito no horário previsto (23/10 12:00 locais).
/// O relógio fica em 23/10 15:00Z. Devolve (reserva, hospedagem).
pub fn checked_out_stay(h: &Harness, admin: &str, seed: &Seed) -> (i64, i64) {
    use crate::models::stay::{CheckInInput, CheckoutInput, LateFeeChoice, OccupantInput, PaymentInput};
    let set = |rfc3339: &str| h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    let reservation = book(h, admin, seed, "2026-10-21", "2026-10-23");
    set("2026-10-21T17:00:00Z");
    let version = h.reservations.get(admin, reservation).unwrap().reservation.version;
    let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(seed.guest_id), document: String::new() };
    let stay = h
        .stays
        .check_in(admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
        .unwrap();
    let balance = h.stays.get(admin, stay).unwrap().account.balance_cents;
    h.stays.record_payment(admin, stay, PaymentInput { amount_cents: balance, method: "cash".into(), note: String::new() }).unwrap();
    set("2026-10-23T15:00:00Z");
    let stay_version = h.stays.get(admin, stay).unwrap().stay.version;
    h.stays
        .checkout(admin, CheckoutInput { stay_id: stay, expected_version: stay_version, late_fee: LateFeeChoice::NotApplicable, balance_exception: None })
        .unwrap();
    (reservation, stay)
}
