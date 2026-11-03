//! Busca global (spec §20.1): reservas, hóspedes, hospedagens e movimentos financeiros, respeitando permissões.

use super::Ctx;
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::text::fold;
use crate::error::AppError;
use crate::models::page::like_pattern;
use crate::models::search::SearchResults;
use crate::repositories::search_repo::{self, Terms};

/// Itens mostrados por grupo; o total real vem junto.
const PER_GROUP: i64 = 8;
const MIN_CHARS: usize = 2;
/// Menos dígitos que isso não casam telefone nem CPF (evita ruído com números curtos como o do quarto).
const MIN_DIGITS: usize = 4;

#[derive(Clone)]
pub struct SearchService {
    ctx: Ctx,
}

impl SearchService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    pub fn search(&self, token: &str, text: &str) -> Result<SearchResults, AppError> {
        let info = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let raw = text.trim();
        if raw.chars().count() < MIN_CHARS {
            return Err(AppError::Validation("Digite pelo menos 2 letras ou números.".into()));
        }
        let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
        let folded = fold(raw);
        let terms = Terms {
            raw,
            code: like_pattern(raw),
            name: like_pattern(&folded),
            digits: (digits.len() >= MIN_DIGITS).then(|| like_pattern(&digits)),
            folded,
        };
        let with_payables = require(&info, Permission::ManagePayables).is_ok();
        let conn = self.ctx.conn()?;
        Ok(SearchResults {
            query: raw.to_string(),
            guests: search_repo::guests(&conn, &terms, PER_GROUP)?,
            reservations: search_repo::reservations(&conn, &terms, PER_GROUP)?,
            stays: search_repo::stays(&conn, &terms, PER_GROUP)?,
            movements: search_repo::movements(&conn, &terms, with_payables, PER_GROUP)?,
        })
    }
}


#[cfg(test)]
mod tests {
    use std::time::Instant;

    use crate::auth::permissions::Permission;
    use crate::error::AppError;
    use crate::models::guest::GuestInput;
    use crate::models::payable::NewPayable;
    use crate::models::payment::{ComponentInput, PaymentInput, PaymentTarget};
    use crate::models::refund::RefundInput;
    use crate::models::stay::{CheckInInput, OccupantInput};
    use crate::services::guest_service::CreateGuestOutcome;
    use crate::services::test_support::{admin_token, book, harness, recepcao_token, seed, Harness, Seed};

    // Fuso -03:00. Um quarto (101) e um hóspede (Ana Souza, HOS-000001).

    struct World {
        h: Harness,
        admin: String,
        seed: Seed,
    }

    fn world() -> World {
        let h = harness();
        let admin = admin_token(&h);
        let seed = seed(&h, &admin, 20_000, 2);
        World { h, admin, seed }
    }

    fn guest(w: &World, name: &str, phone: &str, cpf: Option<&str>) -> i64 {
        let input = GuestInput { full_name: name.into(), phone: phone.into(), cpf: cpf.map(str::to_string), email: String::new(), address: String::new() };
        match w.h.guests.create(&w.admin, input, true).unwrap() {
            CreateGuestOutcome::Created { id, .. } => id,
            other => panic!("{other:?}"),
        }
    }

    fn rec_with(w: &World, permissions: &[Permission]) -> String {
        if w.h.auth.login("rec", "Definitiva-456").is_err() {
            let _ = recepcao_token(&w.h, &w.admin);
        }
        let id = w.h.auth.login("rec", "Definitiva-456").unwrap().user.id;
        w.h.users.set_extra_permissions(&w.admin, id, permissions).unwrap();
        w.h.auth.login("rec", "Definitiva-456").unwrap().token
    }

    fn parts(items: &[(&str, i64)]) -> Vec<ComponentInput> {
        items.iter().map(|(m, c)| ComponentInput { method: (*m).into(), amount_cents: *c }).collect()
    }

    fn names(group: &crate::models::search::Group<crate::models::search::GuestHit>) -> Vec<&str> {
        group.items.iter().map(|g| g.name.as_str()).collect()
    }

    #[test]
    fn finds_guest_by_name_without_accents_phone_and_masked_or_plain_cpf() {
        let w = world();
        guest(&w, "José Conceição", "(11) 98888-7777", Some("529.982.247-25"));

        for text in ["jose conceicao", "JOSÉ", "98888", "(11) 98888-7777", "529.982.247-25", "52998224725", "hos-000002"] {
            let results = w.h.search.search(&w.admin, text).unwrap();
            assert_eq!(names(&results.guests), ["José Conceição"], "{text}");
        }
        let hit = &w.h.search.search(&w.admin, "jose").unwrap().guests.items[0];
        assert_eq!(hit.cpf_masked.as_deref(), Some("***.***.***-25"));
        assert!(w.h.search.search(&w.admin, "52998224725").unwrap().guests.items.iter().all(|g| g.cpf_masked.as_deref() != Some("52998224725")));
    }

    #[test]
    fn short_digit_runs_do_not_match_phone_or_cpf() {
        let w = world();
        guest(&w, "Carla Dias", "(11) 91234-5678", None);

        let results = w.h.search.search(&w.admin, "12").unwrap();

        assert!(results.guests.items.is_empty(), "dois dígitos não casam telefone");
    }

    #[test]
    fn finds_reservation_and_stay_by_shared_code_in_any_case_and_by_room() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        w.h.utc.set(chrono::DateTime::parse_from_rfc3339("2026-10-21T17:00:00Z").unwrap().with_timezone(&chrono::Utc));
        let stay = w.h.stays.check_in(&w.admin, CheckInInput { reservation_id: id, expected_version: version, occupants: vec![holder], exceed_capacity: false }).unwrap();

        for text in ["res-000001", "RES-000001", "RES-00000"] {
            let r = w.h.search.search(&w.admin, text).unwrap();
            assert_eq!((r.reservations.total, r.stays.total), (1, 1), "{text}");
            assert_eq!((r.reservations.items[0].id, r.stays.items[0].id), (id, stay));
        }
        let by_room = w.h.search.search(&w.admin, "101").unwrap();
        assert_eq!((by_room.reservations.total, by_room.stays.total), (1, 1));
        let by_name = w.h.search.search(&w.admin, "souza").unwrap();
        assert_eq!(by_name.reservations.items[0].guest_name, "Ana Souza");
    }

    #[test]
    fn finds_payment_and_refund_by_code_and_by_guest_name() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-11-10", "2026-11-13");
        let payment = w
            .h
            .payments
            .record(&w.admin, PaymentTarget::Reservation { reservation_id: id, expected_version: 1 }, PaymentInput { components: parts(&[("pix", 12_000)]), note: String::new(), duplicate_release: None })
            .unwrap();
        w.h.refunds.record(&w.admin, RefundInput { payment_id: payment.id, components: parts(&[("pix", 2_000)]), reason: "Ajuste".into() }).unwrap();

        let by_code = w.h.search.search(&w.admin, "pag-000001").unwrap();
        assert_eq!(by_code.movements.items.iter().map(|m| (m.kind.as_str(), m.code.as_str())).collect::<Vec<_>>(), [("payment", "PAG-000001")]);
        assert_eq!(by_code.movements.items[0].amount_cents, 12_000);
        assert_eq!(by_code.movements.items[0].link, format!("/reservas/{id}"));
        let refund = w.h.search.search(&w.admin, "DEV-000001").unwrap();
        assert_eq!(refund.movements.items[0].kind, "refund");
        let by_name = w.h.search.search(&w.admin, "ana souza").unwrap();
        assert_eq!(by_name.movements.total, 2, "recebimento e devolução do hóspede");
    }

    #[test]
    fn payable_results_need_manage_payables() {
        let w = world();
        w.h.payables
            .create(
                &w.admin,
                NewPayable {
                    supplier: "Padaria Sol".into(),
                    description: "Pães".into(),
                    amount_cents: 15_000,
                    competence_date: "2026-10-20".into(),
                    due_date: "2026-11-05".into(),
                    category_id: 1,
                    notes: String::new(),
                    paid_now: None,
                },
            )
            .unwrap();

        let admin = w.h.search.search(&w.admin, "padaria").unwrap();
        assert_eq!((admin.movements.total, admin.movements.items[0].kind.as_str(), admin.movements.items[0].code.as_str()), (1, "payable", "CAP-000001"));
        assert_eq!(w.h.search.search(&w.admin, "CAP-000001").unwrap().movements.total, 1);

        let without = rec_with(&w, &[]);
        assert_eq!(w.h.search.search(&without, "padaria").unwrap().movements.total, 0);
        assert_eq!(w.h.search.search(&without, "CAP-000001").unwrap().movements.total, 0);
        let with = rec_with(&w, &[Permission::ManagePayables]);
        assert_eq!(w.h.search.search(&with, "padaria").unwrap().movements.total, 1);
    }

    #[test]
    fn percent_underscore_and_backslash_are_literals() {
        let w = world(); // só existe "Ana Souza"

        for text in ["%%", "a_", "an\\", "%a", "_n"] {
            let r = w.h.search.search(&w.admin, text).unwrap();
            assert!(r.guests.items.is_empty() && r.reservations.items.is_empty(), "{text}");
        }
        guest(&w, "A_B Teste", "11999990001", None);
        assert_eq!(names(&w.h.search.search(&w.admin, "a_b").unwrap().guests), ["A_B Teste"]);
    }

    #[test]
    fn one_character_is_refused_and_two_are_accepted() {
        let w = world();

        for text in ["", " ", "a", "  x "] {
            assert!(matches!(w.h.search.search(&w.admin, text), Err(AppError::Validation(m)) if m.contains("pelo menos 2")), "{text:?}");
        }
        assert!(w.h.search.search(&w.admin, "an").is_ok());
    }

    #[test]
    fn results_are_limited_to_eight_with_the_full_total_and_exact_name_first() {
        let w = world();
        for i in 1..=12 {
            guest(&w, &format!("Cliente {i:02}"), &format!("1199999{i:04}"), None);
        }
        guest(&w, "Cliente", "11977770000", None);

        let r = w.h.search.search(&w.admin, "cliente").unwrap();

        assert_eq!((r.guests.items.len(), r.guests.total), (8, 13));
        assert_eq!(r.guests.items[0].name, "Cliente", "o nome exato vem primeiro");
    }

    #[test]
    fn search_requires_a_session() {
        let w = world();

        assert!(matches!(w.h.search.search("token-invalido", "ana"), Err(AppError::Unauthorized)));
    }

    #[test]
    fn search_over_five_years_of_data_stays_fast() {
        let w = world();
        {
            let mut conn = w.h.db.lock().unwrap();
            let tx = conn.transaction().unwrap();
            let t = "2026-10-21T12:00:00Z";
            for i in 1..=3000 {
                tx.execute(
                    "INSERT INTO guests (code, full_name, name_key, phone, cpf, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    rusqlite::params![format!("HOS-{:06}", 1000 + i), format!("Hóspede Teste {i}"), format!("hospede teste {i}"), format!("11{:09}", 900_000_000 + i), format!("{:011}", 10_000_000_000i64 + i), t],
                )
                .unwrap();
            }
            for room in 2..=8 {
                tx.execute(
                    "INSERT INTO rooms (code, category_id, max_capacity, description, notes, active, version, created_at, updated_at) VALUES (?1, ?2, 2, '', '', 1, 1, ?3, ?3)",
                    rusqlite::params![format!("{}", 100 + room), w.seed.category_id, t],
                )
                .unwrap();
            }
            let rooms: Vec<i64> = {
                let mut stmt = tx.prepare("SELECT id FROM rooms").unwrap();
                stmt.query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect()
            };
            let guests: Vec<i64> = {
                let mut stmt = tx.prepare("SELECT id FROM guests").unwrap();
                stmt.query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect()
            };
            let start = chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
            let mut n = 0i64;
            for day in 0..1825 {
                let date = start + chrono::Duration::days(day);
                let (d_in, d_out) = (date.format("%Y-%m-%d").to_string(), (date + chrono::Duration::days(1)).format("%Y-%m-%d").to_string());
                for (k, room) in rooms.iter().enumerate() {
                    n += 1;
                    let code = format!("RES-{:06}", 100 + n);
                    let guest = guests[(n as usize + k) % guests.len()];
                    tx.execute(
                        "INSERT INTO reservations (code, room_id, guest_id, check_in_date, check_out_date, expected_check_in_time, expected_check_out_time, occupants, status,
                            list_total_cents, agreed_total_cents, deposit_expected_cents, deposit_due_date, created_by, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, '14:00', '12:00', 1, 'completed', 20000, 20000, 6000, ?4, 1, ?6, ?6)",
                        rusqlite::params![code, room, guest, d_in, d_out, t],
                    )
                    .unwrap();
                    let reservation_id = tx.last_insert_rowid();
                    tx.execute(
                        "INSERT INTO stays (reservation_id, code, status, checked_in_at, checked_in_by, created_at, updated_at) VALUES (?1, ?2, 'checked_out', ?3, 1, ?3, ?3)",
                        rusqlite::params![reservation_id, code, t],
                    )
                    .unwrap();
                    let stay_id = tx.last_insert_rowid();
                    tx.execute(
                        "INSERT INTO payments (code, reservation_id, stay_id, kind, amount_cents, received_at, received_by) VALUES (?1, ?2, ?3, 'stay', 20000, ?4, 1)",
                        rusqlite::params![format!("PAG-{:06}", 100 + n), reservation_id, stay_id, t],
                    )
                    .unwrap();
                    let payment_id = tx.last_insert_rowid();
                    tx.execute("INSERT INTO payment_components (payment_id, method, amount_cents) VALUES (?1, 'pix', 20000)", [payment_id]).unwrap();
                }
            }
            tx.commit().unwrap();
        }
        let timed = |text: &str| {
            let started = Instant::now();
            let results = w.h.search.search(&w.admin, text).unwrap();
            (started.elapsed(), results)
        };

        let (by_name, r1) = timed("hospede teste 1500");
        let (by_code, r2) = timed("RES-007000");
        let (by_cpf, r3) = timed("10000001500");

        assert!(by_name.as_millis() < 500 && by_code.as_millis() < 500 && by_cpf.as_millis() < 500, "{by_name:?} {by_code:?} {by_cpf:?}");
        assert!(r1.guests.total >= 1 && r2.reservations.total >= 1 && r3.guests.total >= 1);
        assert!(r2.reservations.items.len() <= 8 && r1.reservations.items.len() <= 8);
    }
}
