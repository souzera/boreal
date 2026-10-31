//! Dados fictícios para desenvolvimento e demonstração (feature `dev-seed`, nunca no build de produção).
//!
//! Tudo passa pelos serviços reais, então regras, auditoria e códigos ficam consistentes. Só roda em
//! banco **sem quartos**. Nomes, telefones e valores são inventados; nenhum dado pessoal real.

use crate::app::AppState;
use crate::error::AppError;

/// Quantos registros o seed criou.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SeedSummary {
    pub rooms: usize,
    pub guests: usize,
    pub reservations: usize,
    pub stays: usize,
    pub blocks: usize,
    pub maintenance_orders: usize,
    pub payables: usize,
}

fn date(offset_days: i64) -> String {
    (chrono::Local::now().date_naive() + chrono::Duration::days(offset_days)).format("%Y-%m-%d").to_string()
}

const GUESTS: [&str; 10] = [
    "Ana Souza", "Bruno Lima", "Carla Menezes", "Diego Alves", "Elisa Prado",
    "Fábio Nunes", "Gabriela Rocha", "Heitor Campos", "Inês Duarte", "João Teixeira",
];

pub fn run(app: &AppState, admin_username: &str, admin_password: &str, recepcao_password: Option<&str>) -> Result<SeedSummary, AppError> {
    use crate::models::catalog::{NewCategory, NewRoom, RoomQuery};
    use crate::models::guest::GuestInput;
    use crate::models::item::NewItem;
    use crate::models::maintenance::NewOrderInput;
    use crate::models::block::NewBlockInput;
    use crate::models::rate::NewRateRule;
    use crate::models::reservation::NewReservation;
    use crate::models::payable::{NewPayable, PayInput};
    use crate::models::payment::{ComponentInput, PaymentInput, PaymentTarget};
    use crate::models::refund::RefundInput;
    use crate::models::stay::{
        ChargeInput, CheckoutInput, LateFeeChoice, OccupantInput, WalkInInput,
    };
    use crate::models::user::Role;
    use crate::services::guest_service::CreateGuestOutcome;
    use crate::services::user_service::NewUserInput;

    if app.auth.needs_bootstrap()? {
        app.auth.bootstrap_admin(admin_username, "Administrador", admin_password)?;
    }
    let t = app.auth.login(admin_username, admin_password)?.token;

    let existing = app.catalog.list_rooms(
        &t,
        RoomQuery { text: None, category_id: None, active: None, page: 1, page_size: 1, sort: None, direction: None },
    )?;
    if existing.total > 0 {
        return Err(AppError::Validation("O banco já tem quartos; o seed só roda em banco vazio.".into()));
    }
    let mut summary = SeedSummary::default();

    if let Some(password) = recepcao_password {
        app.users.create_user(
            &t,
            NewUserInput { username: "recepcao".into(), display_name: "Recepção".into(), role: Role::Recepcao, initial_password: password.into() },
        )?;
    }

    // Categorias, quartos e tarifas.
    let mut rooms = std::collections::HashMap::new();
    for (name, description, price, prefix, capacity) in [
        ("Standard", "Quarto simples", 18_000, "10", 2),
        ("Luxo", "Quarto amplo com varanda", 28_000, "20", 3),
    ] {
        let category = app.catalog.create_category(&t, NewCategory { name: name.into(), description: description.into() })?;
        for day in 0..7 {
            app.rates.create_rule(
                &t,
                NewRateRule {
                    category_id: category,
                    rule_type: "weekday".into(),
                    weekday: Some(day),
                    start_date: None,
                    end_date: None,
                    date: None,
                    amount_cents: price,
                    label: String::new(),
                },
            )?;
        }
        for n in 1..=4 {
            let code = format!("{prefix}{n}");
            let id = app.catalog.create_room(
                &t,
                NewRoom { code: code.clone(), category_id: category, max_capacity: capacity, description: String::new(), notes: String::new() },
            )?;
            rooms.insert(code, id);
            summary.rooms += 1;
        }
    }

    // Itens do catálogo.
    let mut water = 0;
    for (name, price) in [("Água mineral", 600), ("Refrigerante", 800), ("Lavanderia (peça)", 1_500), ("Café da manhã extra", 3_500)] {
        let id = app.items.create(&t, NewItem { name: name.into(), default_price_cents: price })?;
        if water == 0 {
            water = id;
        }
    }

    // Hóspedes fictícios.
    let mut guests = Vec::new();
    for (i, name) in GUESTS.iter().enumerate() {
        let input = GuestInput {
            full_name: (*name).into(),
            phone: format!("(11) 99999-{:04}", i + 1),
            cpf: None,
            email: String::new(),
            address: String::new(),
        };
        match app.guests.create(&t, input, true)? {
            CreateGuestOutcome::Created { id, .. } => guests.push(id),
            other => return Err(AppError::Validation(format!("hóspede de demonstração não criado: {other:?}"))),
        }
        summary.guests += 1;
    }

    let holder = |guest: usize| OccupantInput { full_name: GUESTS[guest].into(), guest_id: Some(guests[guest]), document: String::new() };
    let walk_in = |app: &AppState, room: &str, guest: usize, nights: i64| -> Result<i64, AppError> {
        app.stays.walk_in(
            &t,
            WalkInInput {
                room_id: rooms[room],
                guest_id: guests[guest],
                check_out_date: date(nights),
                expected_check_out_time: None,
                occupants: vec![holder(guest)],
                exceed_capacity: false,
            },
        )
    };
    let book = |app: &AppState, room: &str, guest: usize, from: i64, to: i64| -> Result<i64, AppError> {
        Ok(app
            .reservations
            .create(
                &t,
                NewReservation {
                    room_id: rooms[room],
                    guest_id: guests[guest],
                    check_in_date: date(from),
                    check_out_date: date(to),
                    expected_check_in_time: None,
                    expected_check_out_time: None,
                    occupants: 1,
                    notes: String::new(),
                    exceed_capacity: false,
                },
            )?
            .id)
    };
    let pay_all_and_check_out = |app: &AppState, stay: i64| -> Result<(), AppError> {
        let detail = app.stays.get(&t, stay)?;
        app.payments.record(
            &t,
            PaymentTarget::Stay { stay_id: stay },
            PaymentInput {
                components: vec![ComponentInput { method: "pix".into(), amount_cents: detail.account.balance_cents }],
                note: String::new(),
                duplicate_release: None,
            },
        )?;
        let version = app.stays.get(&t, stay)?.stay.version;
        app.stays.checkout(
            &t,
            CheckoutInput { stay_id: stay, expected_version: version, late_fee: LateFeeChoice::NotApplicable, balance_exception: None },
        )
    };
    let confirm_with_deposit = |app: &AppState, reservation: i64| -> Result<(), AppError> {
        let detail = app.reservations.get(&t, reservation)?;
        app.payments
            .record(
                &t,
                PaymentTarget::Reservation { reservation_id: reservation, expected_version: detail.reservation.version },
                PaymentInput {
                    components: vec![ComponentInput { method: "pix".into(), amount_cents: detail.reservation.deposit_expected_cents }],
                    note: String::new(),
                    duplicate_release: None,
                },
            )
            .map(|_| ())
    };

    // 101: hóspede em casa, com consumo e saldo a receber.
    let stay = walk_in(app, "101", 0, 3)?;
    app.stays.add_charge(&t, stay, ChargeInput { kind: "consumption".into(), description: String::new(), amount_cents: 0, item_id: Some(water) })?;
    app.stays.add_charge(&t, stay, ChargeInput { kind: "consumption".into(), description: String::new(), amount_cents: 0, item_id: Some(water) })?;
    // Recebimento dividido (Pix + dinheiro) com uma devolução parcial; a conta segue com saldo a receber.
    let split = app.payments.record(
        &t,
        PaymentTarget::Stay { stay_id: stay },
        PaymentInput {
            components: vec![
                ComponentInput { method: "pix".into(), amount_cents: 6_000 },
                ComponentInput { method: "cash".into(), amount_cents: 4_000 },
            ],
            note: String::new(),
            duplicate_release: None,
        },
    )?;
    app.refunds.record(
        &t,
        RefundInput { payment_id: split.id, components: vec![ComponentInput { method: "cash".into(), amount_cents: 1_000 }], reason: "Cobrança em dobro".into() },
    )?;
    summary.stays += 1;

    // 102, 103 e 104: saíram hoje, cada um num ponto diferente da limpeza.
    for (room, guest) in [("102", 1), ("103", 2), ("104", 3)] {
        let stay = walk_in(app, room, guest, 1)?;
        pay_all_and_check_out(app, stay)?;
        summary.stays += 1;
    }
    let queue = app.cleaning.queue(&t)?;
    let task = |code: &str| queue.iter().find(|i| i.task.room_code == code).map(|i| (i.task.id, i.task.version));
    if let Some((id, version)) = task("103") {
        app.cleaning.start(&t, id, version)?;
    }
    if let Some((id, version)) = task("104") {
        app.cleaning.start(&t, id, version)?;
        app.cleaning.complete(&t, id, version + 1)?;
    }
    // 102 tem hóspede chegando amanhã: a limpeza sobe de prioridade.
    let arriving = book(app, "102", 4, 1, 3)?;
    confirm_with_deposit(app, arriving)?;
    summary.reservations += 1;

    // 204: reservas futuras, uma pendente e uma confirmada com sinal.
    book(app, "204", 5, 2, 4)?;
    let confirmed = book(app, "204", 6, 7, 9)?;
    confirm_with_deposit(app, confirmed)?;
    summary.reservations += 2;

    // 201: reserva confirmada e depois cancelada com antecedência: sobra uma devolução a fazer.
    let cancelled = book(app, "201", 6, 15, 17)?;
    confirm_with_deposit(app, cancelled)?;
    let version = app.reservations.get(&t, cancelled)?.reservation.version;
    app.reservations.cancel(&t, cancelled, version, None)?;
    summary.reservations += 1;

    // Contas a pagar: uma pendente, uma parcial e uma vencida.
    let category = |name: &str| -> Result<i64, AppError> {
        app.payables
            .list_categories(&t, true)?
            .into_iter()
            .find(|c| c.name == name)
            .map(|c| c.id)
            .ok_or_else(|| AppError::Validation(format!("categoria de demonstração ausente: {name}")))
    };
    let bill = |supplier: &str, description: &str, cents: i64, competence: i64, due: i64, category_id: i64| NewPayable {
        supplier: supplier.into(),
        description: description.into(),
        amount_cents: cents,
        competence_date: date(competence),
        due_date: date(due),
        category_id,
        notes: String::new(),
        paid_now: None,
    };
    app.payables.create(&t, bill("Companhia de Energia", "Conta do mês", 48_000, 0, 10, category("Energia")?))?;
    let partial = app.payables.create(&t, bill("Padaria Sol", "Pães e frios do mês", 15_000, -5, 5, category("Alimentos e bebidas")?))?;
    app.payables.pay(&t, partial, PayInput { amount_cents: 6_000, method: "pix".into(), note: String::new() }, 1)?;
    app.payables.create(&t, bill("Eletricista Silva", "Reparo na fiação", 32_000, -20, -3, category("Manutenção")?))?;
    summary.payables += 3;

    // 201: bloqueio manual no futuro.
    app.blocks.create(&t, NewBlockInput { room_id: rooms["201"], start_date: date(10), end_date: Some(date(13)), reason: "Pintura do corredor".into() })?;
    summary.blocks += 1;

    // 202: reserva confirmada que a manutenção afeta (bloqueio preventivo + conflito).
    let affected = book(app, "202", 7, 5, 7)?;
    confirm_with_deposit(app, affected)?;
    summary.reservations += 1;
    app.maintenance.open(
        &t,
        NewOrderInput {
            room_id: rooms["202"],
            description: "Trocar o chuveiro".into(),
            priority: "high".into(),
            planned_start: date(6),
            planned_end: Some(date(8)),
            responsible: "Seu Antônio".into(),
            notes: "Material já comprado".into(),
        },
    )?;
    summary.maintenance_orders += 1;

    // 203: manutenção em andamento hoje, sem reservas afetadas.
    let opened = app.maintenance.open(
        &t,
        NewOrderInput {
            room_id: rooms["203"],
            description: "Consertar o ar-condicionado".into(),
            priority: "urgent".into(),
            planned_start: date(0),
            planned_end: Some(date(2)),
            responsible: "Refrigeração Central".into(),
            notes: String::new(),
        },
    )?;
    app.maintenance.start(&t, opened.id, 1)?;
    summary.maintenance_orders += 1;

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::{run, SeedSummary};
    use crate::app::{initialize, AppState};
    use crate::db::keystore::MemoryKeyStore;
    use crate::error::AppError;
    use crate::models::catalog::RoomQuery;

    const ADMIN: &str = "Admin-Senha-1";

    fn fresh() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let app = initialize(&dir.path().join("boreal.db"), &MemoryKeyStore::empty()).unwrap();
        (dir, app)
    }

    fn states(app: &AppState) -> Vec<(String, String)> {
        let token = app.auth.login("admin", ADMIN).unwrap().token;
        app.room_status.list(&token).unwrap().into_iter().map(|s| (s.code, s.state)).collect()
    }

    #[test]
    fn seed_builds_the_complete_scenario() {
        let (_dir, app) = fresh();

        let summary = run(&app, "admin", ADMIN, Some("Recepcao-Demo-1")).unwrap();

        assert_eq!(summary.rooms, 8);
        assert_eq!(summary.guests, 10);
        assert_eq!((summary.stays, summary.reservations), (4, 5), "{summary:?}");
        assert_eq!((summary.blocks, summary.maintenance_orders, summary.payables), (1, 2, 3));
        let got: std::collections::HashMap<_, _> = states(&app).into_iter().collect();
        for (room, state) in [
            ("101", "occupied"),
            ("102", "awaiting_cleaning"),
            ("103", "cleaning"),
            ("104", "clean_waiting_interval"),
            ("201", "available"),
            ("202", "available"),
            ("203", "blocked"),
            ("204", "available"),
        ] {
            assert_eq!(got.get(room).map(String::as_str), Some(state), "quarto {room}");
        }
    }

    #[test]
    fn seed_leaves_work_in_every_queue() {
        let (_dir, app) = fresh();
        run(&app, "admin", ADMIN, None).unwrap();
        let token = app.auth.login("admin", ADMIN).unwrap().token;

        assert_eq!(app.cleaning.queue(&token).unwrap().len(), 2);
        assert_eq!(app.blocks.list(&token, false).unwrap().len(), 3); // manual + preventivo + manutenção
        let orders = app.maintenance.list(&token, true).unwrap();
        assert_eq!(orders.len(), 2);
        let with_conflict = orders.iter().filter(|o| !app.maintenance.get(&token, o.id).unwrap().conflicts.is_empty()).count();
        assert_eq!(with_conflict, 1);
        assert_eq!(app.stays.list(&token, crate::models::stay::StayQuery { status: Some("in_house".into()), text: None, page: 1, page_size: 20 }).unwrap().total, 1);
    }

    #[test]
    fn seed_creates_the_finance_scenario() {
        let (_dir, app) = fresh();
        run(&app, "admin", ADMIN, None).unwrap();
        let token = app.auth.login("admin", ADMIN).unwrap().token;

        let page = |status: &str| {
            app.payables
                .list(&token, crate::models::payable::PayableQuery { status: Some(status.into()), category_id: None, due_from: None, due_to: None, page: 1, page_size: 50 })
                .unwrap()
                .total
        };
        assert_eq!((page("pending"), page("partial"), page("overdue")), (1, 1, 1));

        let in_house = app.stays.list(&token, crate::models::stay::StayQuery { status: Some("in_house".into()), text: None, page: 1, page_size: 20 }).unwrap();
        let detail = app.stays.get(&token, in_house.items[0].stay.id).unwrap();
        let split = detail.account.payments.iter().find(|p| p.components.len() == 2).expect("recebimento dividido");
        assert_eq!(split.amount_cents, 10_000);
        assert_eq!(detail.account.refunds.len(), 1, "devolução parcial no recebimento dividido");
        assert!(detail.account.balance_cents > 0, "a conta continua com saldo a receber");

        let reservations = app.reservations.list(&token, crate::models::reservation::ReservationQuery {
            text: None, status: Some(vec!["cancelled".into()]), room_id: None, from: None, to: None, page: 1, page_size: 20, sort: None, direction: None,
        }).unwrap();
        assert_eq!(reservations.total, 1);
        assert!(reservations.items[0].refund_due_cents > 0, "cancelada com devolução a fazer");
        assert!(app.closing.pending_dates(&token).unwrap().is_empty(), "o seed só tem movimento de hoje");
    }

    #[test]
    fn seed_refuses_a_database_that_already_has_rooms() {
        let (_dir, app) = fresh();
        run(&app, "admin", ADMIN, None).unwrap();

        let again = run(&app, "admin", ADMIN, None);

        assert!(matches!(again, Err(AppError::Validation(m)) if m.contains("já tem quartos")));
        let token = app.auth.login("admin", ADMIN).unwrap().token;
        let rooms = app
            .catalog
            .list_rooms(&token, RoomQuery { text: None, category_id: None, active: None, page: 1, page_size: 50, sort: None, direction: None })
            .unwrap();
        assert_eq!(rooms.total, 8);
    }

    #[test]
    fn seed_works_on_a_database_whose_admin_already_exists() {
        let (_dir, app) = fresh();
        app.auth.bootstrap_admin("admin", "Administrador", ADMIN).unwrap();

        assert_eq!(run(&app, "admin", ADMIN, None).unwrap().rooms, 8);
        assert_eq!(SeedSummary::default().rooms, 0);
    }

    #[test]
    fn seed_runs_with_a_custom_admin_username() {
        let (_dir, app) = fresh();
        app.auth.bootstrap_admin("matheus", "Matheus", "senha-dev-1").unwrap();

        assert_eq!(run(&app, "matheus", "senha-dev-1", None).unwrap().rooms, 8);
        let token = app.auth.login("matheus", "senha-dev-1").unwrap().token;
        assert_eq!(app.room_status.list(&token).unwrap().len(), 8);
    }

    #[test]
    fn seed_with_the_wrong_admin_password_changes_nothing() {
        let (_dir, app) = fresh();
        app.auth.bootstrap_admin("admin", "Administrador", ADMIN).unwrap();

        assert!(run(&app, "admin", "senha-errada-1", None).is_err());
        let token = app.auth.login("admin", ADMIN).unwrap().token;
        assert!(app.room_status.list(&token).unwrap().is_empty());
    }
}
