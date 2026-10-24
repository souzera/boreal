use chrono::{DateTime, Timelike, Utc};
use rusqlite::Transaction;
use serde_json::json;

use super::rate_service::quote_in;
use super::reservation_service::{br, conflict_message, parse_date, parse_time, resolve_times};
use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::domain::availability::find_conflicts;
use crate::domain::dates;
use crate::domain::stay::can_check_in;
use crate::error::AppError;
use crate::models::page::{clamp_paging, Page};
use crate::models::reservation::{ConflictInfo, Reservation};
use crate::models::stay::{CheckInInput, OccupantInput, StayDetail, StayQuery, StaySummary, WalkInInput};
use crate::repositories::stay_repo::{self, NewCharge};
use crate::repositories::reservation_repo::NewRow;
use crate::repositories::{catalog_repo, guest_repo, reservation_repo, sequence_repo, settings_repo};

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

pub(crate) struct CleanOccupant {
    name: String,
    guest_id: Option<i64>,
    document: String,
}

const MAX_OCCUPANTS: usize = 20;

pub(crate) fn clean_occupants(list: &[OccupantInput]) -> Result<Vec<CleanOccupant>, AppError> {
    if list.is_empty() {
        return Err(invalid("Informe ao menos um ocupante."));
    }
    if list.len() > MAX_OCCUPANTS {
        return Err(invalid(format!("No máximo {MAX_OCCUPANTS} ocupantes por hospedagem.")));
    }
    list.iter()
        .map(|o| {
            let name = o.full_name.trim();
            if name.is_empty() {
                return Err(invalid("Informe o nome de todos os ocupantes."));
            }
            if name.chars().count() > 120 {
                return Err(invalid("Nome do ocupante: no máximo 120 caracteres."));
            }
            let document = o.document.trim();
            if document.chars().count() > 30 {
                return Err(invalid("Documento do ocupante: no máximo 30 caracteres."));
            }
            Ok(CleanOccupant { name: name.to_string(), guest_id: o.guest_id, document: document.to_string() })
        })
        .collect()
}

#[derive(Clone)]
pub struct StayService {
    ctx: Ctx,
}

impl StayService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    pub(crate) fn touch(&self, token: &str) -> Result<SessionInfo, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())
    }

    /// Abre a hospedagem de uma reserva já existente: ocupantes, diárias congeladas, desconto e sinal
    /// viram lançamentos da conta. Chamado dentro da transação do check-in ou do walk-in.
    #[allow(clippy::too_many_arguments)] // cada parâmetro é uma decisão já validada pelo chamador
    pub(crate) fn open_stay(
        &self,
        tx: &Transaction,
        actor: &SessionInfo,
        reservation: &Reservation,
        occupants: &[CleanOccupant],
        early: bool,
        exceeded: bool,
        at: DateTime<Utc>,
    ) -> Result<i64, AppError> {
        let stay_id = stay_repo::insert_stay(tx, reservation.id, &reservation.code, actor.user_id, early, exceeded, at)?;
        for o in occupants {
            let is_holder = o.guest_id == Some(reservation.guest_id);
            stay_repo::insert_occupant(tx, stay_id, &o.name, o.guest_id, &o.document, is_holder)?;
        }
        for night in reservation_repo::nights(tx, reservation.id)? {
            let description = format!("Diária de {}", br(&night.date));
            stay_repo::insert_charge(
                tx,
                &NewCharge {
                    stay_id,
                    kind: "night",
                    description: &description,
                    amount_cents: night.amount_cents,
                    item_id: None,
                    night_date: Some(&night.date),
                    reverses_charge_id: None,
                    replaces_charge_id: None,
                    justification: None,
                    created_by: actor.user_id,
                },
                at,
            )?;
        }
        if reservation.discount_cents > 0 {
            stay_repo::insert_charge(
                tx,
                &NewCharge {
                    stay_id,
                    kind: "discount",
                    description: "Desconto concedido na reserva",
                    amount_cents: -reservation.discount_cents,
                    item_id: None,
                    night_date: None,
                    reverses_charge_id: None,
                    replaces_charge_id: None,
                    justification: reservation.discount_reason.as_deref(),
                    created_by: actor.user_id,
                },
                at,
            )?;
        }
        if reservation.deposit_paid_cents > 0 {
            let code = sequence_repo::format_code("PAG", sequence_repo::next(tx, "payment")?);
            stay_repo::insert_payment(
                tx,
                &code,
                reservation.id,
                stay_id,
                reservation.deposit_paid_cents,
                "prior_deposit",
                actor.user_id,
                "Sinal recebido antes do check-in",
                at,
            )?;
        }
        Ok(stay_id)
    }

    /// Check-in manual de uma reserva (spec §6.1). Revalida tudo na mesma transação que grava.
    pub fn check_in(&self, token: &str, input: CheckInInput) -> Result<i64, AppError> {
        let actor = self.touch(token)?;
        let occupants = clean_occupants(&input.occupants)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;

        let reservation = reservation_repo::find(&tx, input.reservation_id, self.ctx.today())?
            .ok_or_else(|| invalid("Reserva não encontrada."))?;
        if !can_check_in(&reservation.status) {
            return Err(invalid("Só reservas pendentes ou confirmadas aceitam check-in."));
        }
        if reservation.version != input.expected_version {
            return Err(AppError::Conflict);
        }
        if !occupants.iter().any(|o| o.guest_id == Some(reservation.guest_id)) {
            return Err(invalid("Inclua o titular da reserva entre os ocupantes."));
        }
        let room = catalog_repo::find_room(&tx, reservation.room_id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        if !room.active {
            return Err(invalid("O quarto está desativado e não pode receber hóspedes."));
        }
        let exceeded = occupants.len() as i64 > room.max_capacity;
        if exceeded {
            if !input.exceed_capacity {
                return Err(invalid(format!(
                    "O quarto {} comporta {} pessoa(s). Reduza os ocupantes ou peça autorização para exceder a capacidade.",
                    room.code, room.max_capacity
                )));
            }
            require(&actor, Permission::ExceedCapacity)?;
        }

        let check_in = parse_date(&reservation.check_in_date, "Data de entrada")?;
        let check_out = parse_date(&reservation.check_out_date, "Data de saída")?;
        let in_time = parse_time(&reservation.expected_check_in_time).ok_or_else(|| invalid("Horário de entrada inválido."))?;
        let out_time = parse_time(&reservation.expected_check_out_time).ok_or_else(|| invalid("Horário de saída inválido."))?;
        let now = self.ctx.local_now();
        let booked_start = check_in.and_time(in_time);
        let early = now < booked_start;
        if early {
            require(&actor, Permission::EarlyCheckIn)?;
        }

        // O quarto precisa estar livre desde o início real da ocupação (mais cedo entre o previsto e agora).
        let settings = settings_repo::load(&tx)?;
        let start = booked_start.min(now);
        let existing = reservation_repo::blocking(&tx, room.id, start.date(), check_out)?;
        let bookings: Vec<_> = existing.iter().map(|(b, _)| b.clone()).collect();
        let clashes = find_conflicts(
            &bookings,
            start.date(),
            start.time(),
            check_out,
            out_time,
            settings.cleaning_interval_minutes,
            Some(reservation.id),
        );
        if !clashes.is_empty() {
            let infos: Vec<ConflictInfo> =
                existing.into_iter().filter(|(b, _)| clashes.contains(&b.id)).map(|(_, info)| info).collect();
            return Err(invalid(conflict_message(&infos)));
        }

        let at = self.ctx.utc.now_utc();
        let stay_id = self.open_stay(&tx, &actor, &reservation, &occupants, early, exceeded, at)?;
        reservation_repo::set_status(&tx, reservation.id, "checked_in", input.expected_version, at)?;
        let before = json!({ "status": reservation.status });
        let after = json!({ "status": "checked_in", "stay_id": stay_id, "early_check_in": early });
        reservation_repo::insert_change(&tx, reservation.id, at, actor.user_id, "check_in", &before.to_string(), &after.to_string(), None, None, None)?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "stays",
                operation: "check_in",
                entity: "stay",
                entity_id: Some(stay_id),
                before: Some(before),
                after: Some(json!({
                    "code": reservation.code, "room": room.code, "occupants": occupants.len(),
                    "early_check_in": early, "exceeded_capacity": exceeded,
                    "opening_balance_cents": stay_repo::balance(&tx, stay_id)?
                })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(stay_id)
    }

    /// Hospedagem sem reserva (spec §4.4): mesmas verificações do check-in, com preço pela tabela de tarifas.
    /// Cria também a linha de `reservations` (origem `walk_in`), para o quarto continuar tendo uma só fonte de ocupação.
    pub fn walk_in(&self, token: &str, input: WalkInInput) -> Result<i64, AppError> {
        let actor = self.touch(token)?;
        let occupants = clean_occupants(&input.occupants)?;
        let today = self.ctx.today();
        let check_out = parse_date(&input.check_out_date, "Data de saída")?;
        if check_out <= today {
            return Err(invalid("A saída deve ser depois de hoje."));
        }

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let settings = settings_repo::load(&tx)?;
        let (_, out_time) = resolve_times(
            None,
            input.expected_check_out_time.as_deref(),
            &settings.default_check_in_time,
            &settings.default_check_out_time,
        )?;
        let now = self.ctx.local_now();
        let in_time = now.time().with_second(0).and_then(|t| t.with_nanosecond(0)).unwrap_or_else(|| now.time());

        let room = catalog_repo::find_room(&tx, input.room_id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        if !room.active {
            return Err(invalid("O quarto está desativado e não pode receber hóspedes."));
        }
        let guest = guest_repo::find(&tx, input.guest_id)?.ok_or_else(|| invalid("Hóspede não encontrado."))?;
        if !guest.active {
            return Err(invalid("O cadastro do hóspede está desativado."));
        }
        if !occupants.iter().any(|o| o.guest_id == Some(guest.id)) {
            return Err(invalid("Inclua o titular entre os ocupantes."));
        }
        let exceeded = occupants.len() as i64 > room.max_capacity;
        if exceeded {
            if !input.exceed_capacity {
                return Err(invalid(format!(
                    "O quarto {} comporta {} pessoa(s). Reduza os ocupantes ou peça autorização para exceder a capacidade.",
                    room.code, room.max_capacity
                )));
            }
            require(&actor, Permission::ExceedCapacity)?;
        }

        let quote = quote_in(&tx, room.category_id, today, check_out)?;
        if !quote.missing_dates.is_empty() {
            let missing = quote.missing_dates.iter().map(|d| br(d)).collect::<Vec<_>>().join(", ");
            return Err(invalid(format!(
                "Não há tarifa cadastrada para: {missing}. Cadastre as tarifas antes de hospedar nessas datas."
            )));
        }

        let existing = reservation_repo::blocking(&tx, room.id, today, check_out)?;
        let bookings: Vec<_> = existing.iter().map(|(b, _)| b.clone()).collect();
        let clashes = find_conflicts(&bookings, today, in_time, check_out, out_time, settings.cleaning_interval_minutes, None);
        if !clashes.is_empty() {
            let infos: Vec<ConflictInfo> =
                existing.into_iter().filter(|(b, _)| clashes.contains(&b.id)).map(|(_, info)| info).collect();
            return Err(invalid(conflict_message(&infos)));
        }

        let code = sequence_repo::format_code("HOS", sequence_repo::next(&tx, "stay")?);
        let (today_s, out_s) = (dates::format(today), dates::format(check_out));
        let (in_s, out_time_s) = (in_time.format("%H:%M").to_string(), out_time.format("%H:%M").to_string());
        let at = self.ctx.utc.now_utc();
        let reservation_id = reservation_repo::insert(
            &tx,
            &NewRow {
                code: &code,
                room_id: room.id,
                guest_id: guest.id,
                check_in: &today_s,
                check_out: &out_s,
                in_time: &in_s,
                out_time: &out_time_s,
                occupants: occupants.len() as u32,
                list_total_cents: quote.total_cents,
                agreed_total_cents: quote.total_cents,
                deposit_expected_cents: 0,
                deposit_due_date: &today_s,
                notes: "",
                created_by: actor.user_id,
                status: "checked_in",
                origin: "walk_in",
            },
            at,
        )?;
        for night in &quote.nights {
            reservation_repo::insert_night(&tx, reservation_id, &night.date, night.amount_cents, night.rule.as_str())?;
        }
        let reservation = reservation_repo::find(&tx, reservation_id, today)?
            .ok_or_else(|| AppError::Database("reserva recém-criada não encontrada".into()))?;
        let stay_id = self.open_stay(&tx, &actor, &reservation, &occupants, false, exceeded, at)?;
        let after = json!({ "status": "checked_in", "stay_id": stay_id, "origin": "walk_in" });
        reservation_repo::insert_change(&tx, reservation_id, at, actor.user_id, "walk_in", "{}", &after.to_string(), None, None, None)?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "stays",
                operation: "walk_in",
                entity: "stay",
                entity_id: Some(stay_id),
                before: None,
                after: Some(json!({
                    "code": code, "room": room.code, "guest": guest.full_name, "check_out_date": out_s,
                    "occupants": occupants.len(), "agreed_total_cents": quote.total_cents, "exceeded_capacity": exceeded
                })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(stay_id)
    }

    pub fn get(&self, token: &str, id: i64) -> Result<StayDetail, AppError> {
        self.touch(token)?;
        let conn = self.ctx.conn()?;
        let stay = stay_repo::find(&conn, id)?.ok_or_else(|| invalid("Hospedagem não encontrada."))?;
        Ok(StayDetail { occupants: stay_repo::occupants(&conn, id)?, account: stay_repo::account(&conn, id)?, stay })
    }

    pub fn list(&self, token: &str, query: StayQuery) -> Result<Page<StaySummary>, AppError> {
        self.touch(token)?;
        let status = match query.status.as_deref() {
            None | Some("in_house") => "in_house",
            Some("checked_out") => "checked_out",
            Some(_) => return Err(invalid("Situação desconhecida.")),
        };
        let (page, page_size, offset) = clamp_paging(query.page, query.page_size);
        let conn = self.ctx.conn()?;
        let (stays, total) = stay_repo::list(&conn, query.text.as_deref(), status, i64::from(page_size), offset)?;
        let items = stays
            .into_iter()
            .map(|stay| {
                let balance_cents = stay_repo::balance(&conn, stay.id)?;
                Ok(StaySummary { stay, balance_cents, minutes_late: 0, risk: None })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(Page { items, total, page, page_size })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::auth::permissions::Permission;
    use crate::error::AppError;
    use crate::models::stay::{CheckInInput, OccupantInput, StayQuery, WalkInInput};
    use crate::services::test_support::{admin_token, book, harness, recepcao_token, seed, Harness, Seed};

    // "Hoje" nos testes é 21/10/2026, 09:00 local (12:00Z; fuso -03:00).

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

    fn set_now(w: &World, rfc3339: &str) {
        w.h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    fn holder(w: &World) -> OccupantInput {
        OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() }
    }

    fn companion(name: &str) -> OccupantInput {
        OccupantInput { full_name: name.into(), guest_id: None, document: String::new() }
    }

    fn version(w: &World, reservation_id: i64) -> i64 {
        w.h.reservations.get(&w.admin, reservation_id).unwrap().reservation.version
    }

    fn input(w: &World, reservation_id: i64, occupants: Vec<OccupantInput>) -> CheckInInput {
        CheckInInput { reservation_id, expected_version: version(w, reservation_id), occupants, exceed_capacity: false }
    }

    /// Reserva 21→23/10 com check-in às 14:00 locais (sem antecipação).
    fn checked_in(w: &World) -> (i64, i64) {
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        set_now(w, "2026-10-21T17:00:00Z");
        let stay = w.h.stays.check_in(&w.admin, input(w, reservation, vec![holder(w)])).unwrap();
        (reservation, stay)
    }

    fn count(w: &World, table: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    fn login_with(w: &World, permissions: &[Permission]) -> String {
        if w.h.auth.login("rec", "Definitiva-456").is_err() {
            let _ = recepcao_token(&w.h, &w.admin);
        }
        let rec_id = w.h.auth.login("rec", "Definitiva-456").unwrap().user.id;
        w.h.users.set_extra_permissions(&w.admin, rec_id, permissions).unwrap();
        w.h.auth.login("rec", "Definitiva-456").unwrap().token
    }

    #[test]
    fn check_in_creates_stay_nights_and_marks_reservation() {
        let w = world();

        let (reservation, stay) = checked_in(&w);

        let detail = w.h.stays.get(&w.admin, stay).unwrap();
        assert_eq!(detail.stay.status, "in_house");
        assert_eq!(detail.stay.code, "RES-000001");
        assert!(!detail.stay.early_check_in);
        assert_eq!(detail.account.charges.iter().filter(|c| c.kind == "night").count(), 2);
        assert!(detail.account.charges.iter().filter(|c| c.kind == "night").all(|c| c.amount_cents == 20_000));
        assert_eq!(detail.account.charges_total_cents, 40_000);
        assert_eq!(detail.occupants.len(), 1);
        assert!(detail.occupants[0].is_holder);
        let res = w.h.reservations.get(&w.admin, reservation).unwrap();
        assert_eq!(res.reservation.status, "checked_in");
        assert_eq!(res.stay_id, Some(stay));
    }

    #[test]
    fn check_in_converts_discount_and_deposit() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let v = version(&w, reservation);
        w.h.reservations
            .apply_discount(
                &w.admin,
                reservation,
                crate::models::reservation::DiscountInput {
                    kind: "amount".into(),
                    amount_cents: Some(4_000),
                    percent_bp: None,
                    justification: "Cliente antigo".into(),
                },
                v,
            )
            .unwrap();
        let v = version(&w, reservation);
        w.h.reservations.record_deposit(&w.admin, reservation, 12_000, v).unwrap();
        set_now(&w, "2026-10-21T17:00:00Z");

        let stay = w.h.stays.check_in(&w.admin, input(&w, reservation, vec![holder(&w)])).unwrap();

        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        let kinds: Vec<_> = account.charges.iter().map(|c| (c.kind.as_str(), c.amount_cents)).collect();
        assert_eq!(kinds, [("night", 20_000), ("night", 20_000), ("discount", -4_000)]);
        assert_eq!(account.payments.len(), 1);
        assert_eq!((account.payments[0].method.as_str(), account.payments[0].amount_cents), ("prior_deposit", 12_000));
        assert_eq!(account.balance_cents, 24_000);
    }

    #[test]
    fn early_check_in_requires_permission() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let rec = login_with(&w, &[]);

        let refused = w.h.stays.check_in(&rec, input(&w, reservation, vec![holder(&w)]));

        assert!(matches!(refused, Err(AppError::Forbidden)));
        assert_eq!(count(&w, "stays"), 0);
        let rec = login_with(&w, &[Permission::EarlyCheckIn]);
        let stay = w.h.stays.check_in(&rec, input(&w, reservation, vec![holder(&w)])).unwrap();
        assert!(w.h.stays.get(&w.admin, stay).unwrap().stay.early_check_in);
    }

    #[test]
    fn early_check_in_blocked_while_room_still_occupied() {
        let w = world();
        let a = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-22");
        set_now(&w, "2026-10-21T17:00:00Z");
        w.h.stays.check_in(&w.admin, input(&w, a, vec![holder(&w)])).unwrap();
        let b = book(&w.h, &w.admin, &w.seed, "2026-10-22", "2026-10-24");
        set_now(&w, "2026-10-21T18:00:00Z");

        let result = w.h.stays.check_in(&w.admin, input(&w, b, vec![holder(&w)]));

        match result {
            Err(AppError::Validation(msg)) => assert!(msg.contains("RES-000001"), "{msg}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn capacity_excess_needs_flag_and_permission() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        set_now(&w, "2026-10-21T17:00:00Z");
        let three = vec![holder(&w), companion("Bia"), companion("Caio")];

        let without_flag = w.h.stays.check_in(&w.admin, input(&w, reservation, three.clone()));
        assert!(matches!(without_flag, Err(AppError::Validation(_))));

        let rec = login_with(&w, &[]);
        let mut with_flag = input(&w, reservation, three.clone());
        with_flag.exceed_capacity = true;
        assert!(matches!(w.h.stays.check_in(&rec, with_flag.clone()), Err(AppError::Forbidden)));

        let rec = login_with(&w, &[Permission::ExceedCapacity]);
        let stay = w.h.stays.check_in(&rec, with_flag).unwrap();
        assert_eq!(w.h.stays.get(&w.admin, stay).unwrap().occupants.len(), 3);
    }

    #[test]
    fn check_in_refuses_wrong_status() {
        for status in ["cancelled", "no_show", "checked_in", "completed"] {
            let w = world();
            let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
            set_now(&w, "2026-10-21T17:00:00Z");
            let request = input(&w, reservation, vec![holder(&w)]);
            w.h.db
                .lock()
                .unwrap()
                .execute("UPDATE reservations SET status = ?1 WHERE id = ?2", rusqlite::params![status, reservation])
                .unwrap();

            let result = w.h.stays.check_in(&w.admin, request);

            assert!(matches!(result, Err(AppError::Validation(_))), "{status}: {result:?}");
            assert_eq!(count(&w, "stays"), 0, "{status}");
        }
    }

    #[test]
    fn check_in_refuses_inactive_room() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        set_now(&w, "2026-10-21T17:00:00Z");
        w.h.db.lock().unwrap().execute("UPDATE rooms SET active = 0", []).unwrap();

        let result = w.h.stays.check_in(&w.admin, input(&w, reservation, vec![holder(&w)]));

        assert!(matches!(result, Err(AppError::Validation(_))));
        assert_eq!(count(&w, "stays"), 0);
    }

    #[test]
    fn double_submit_is_conflict_and_does_not_duplicate() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let v = version(&w, reservation);
        w.h.reservations.record_deposit(&w.admin, reservation, 12_000, v).unwrap();
        set_now(&w, "2026-10-21T17:00:00Z");
        let request = input(&w, reservation, vec![holder(&w)]);

        w.h.stays.check_in(&w.admin, request.clone()).unwrap();
        let second = w.h.stays.check_in(&w.admin, request);

        assert!(second.is_err());
        assert_eq!(count(&w, "stays"), 1);
        assert_eq!(count(&w, "charges"), 2);
        assert_eq!(count(&w, "payments"), 1);
    }

    #[test]
    fn invalid_input_writes_nothing() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        set_now(&w, "2026-10-21T17:00:00Z");
        let audit_before = count(&w, "audit_log");

        for occupants in [vec![], vec![companion("   ")], vec![companion("Bia")]] {
            let result = w.h.stays.check_in(&w.admin, input(&w, reservation, occupants));
            assert!(matches!(result, Err(AppError::Validation(_))), "{result:?}");
        }

        assert_eq!((count(&w, "stays"), count(&w, "charges"), count(&w, "payments")), (0, 0, 0));
        assert_eq!(count(&w, "audit_log"), audit_before);
    }

    #[test]
    fn check_in_is_audited_in_same_transaction() {
        let w = world();
        checked_in(&w);

        let n: i64 = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_log WHERE module = 'stays' AND operation = 'check_in'", [], |r| r.get(0))
            .unwrap();

        assert_eq!(n, 1);
    }

    #[test]
    fn late_arrival_day_after_is_allowed() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        set_now(&w, "2026-10-22T17:00:00Z");

        let stay = w.h.stays.check_in(&w.admin, input(&w, reservation, vec![holder(&w)]));

        assert!(stay.is_ok(), "{stay:?}");
    }

    #[test]
    fn list_in_house_returns_only_open_stays_with_balance() {
        let w = world();
        let (_, stay) = checked_in(&w);
        let other = book(&w.h, &w.admin, &w.seed, "2026-10-25", "2026-10-26");
        let _ = other;

        let page = w
            .h
            .stays
            .list(&w.admin, StayQuery { text: None, status: None, page: 1, page_size: 10 })
            .unwrap();

        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].stay.id, stay);
        assert_eq!(page.items[0].balance_cents, 40_000);
        let closed = w
            .h
            .stays
            .list(&w.admin, StayQuery { text: None, status: Some("checked_out".into()), page: 1, page_size: 10 })
            .unwrap();
        assert_eq!(closed.total, 0);
    }

    fn walk_in_input(w: &World, check_out: &str, occupants: Vec<OccupantInput>) -> WalkInInput {
        WalkInInput {
            room_id: w.seed.room_id,
            guest_id: w.seed.guest_id,
            check_out_date: check_out.into(),
            expected_check_out_time: None,
            occupants,
            exceed_capacity: false,
        }
    }

    #[test]
    fn walk_in_creates_reservation_and_stay_with_hos_code() {
        let w = world();

        let stay = w.h.stays.walk_in(&w.admin, walk_in_input(&w, "2026-10-23", vec![holder(&w)])).unwrap();

        let detail = w.h.stays.get(&w.admin, stay).unwrap();
        assert_eq!(detail.stay.code, "HOS-000001");
        assert_eq!(detail.stay.status, "in_house");
        assert_eq!(detail.account.charges.iter().filter(|c| c.kind == "night").count(), 2);
        assert_eq!(detail.account.balance_cents, 40_000);
        let res = w.h.reservations.get(&w.admin, detail.stay.reservation_id).unwrap().reservation;
        assert_eq!((res.origin.as_str(), res.status.as_str()), ("walk_in", "checked_in"));
        assert_eq!((res.check_in_date.as_str(), res.expected_check_in_time.as_str()), ("2026-10-21", "09:00"));
        assert_eq!(res.deposit_expected_cents, 0);
    }

    #[test]
    fn walk_in_refuses_occupied_room() {
        let w = world();
        checked_in(&w);

        let result = w.h.stays.walk_in(&w.admin, walk_in_input(&w, "2026-10-24", vec![holder(&w)]));

        match result {
            Err(AppError::Validation(msg)) => assert!(msg.contains("RES-000001"), "{msg}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(count(&w, "stays"), 1);
    }

    #[test]
    fn walk_in_refuses_missing_tariff_and_lists_the_dates() {
        let w = world();
        w.h.db.lock().unwrap().execute("DELETE FROM rate_rules", []).unwrap();

        let result = w.h.stays.walk_in(&w.admin, walk_in_input(&w, "2026-10-23", vec![holder(&w)]));

        match result {
            Err(AppError::Validation(msg)) => assert!(msg.contains("21/10/2026") && msg.contains("22/10/2026"), "{msg}"),
            other => panic!("{other:?}"),
        }
        assert_eq!((count(&w, "stays"), count(&w, "reservations")), (0, 0));
    }

    #[test]
    fn walk_in_respects_capacity_rules() {
        let w = world();
        let three = vec![holder(&w), companion("Bia"), companion("Caio")];
        assert!(matches!(
            w.h.stays.walk_in(&w.admin, walk_in_input(&w, "2026-10-23", three.clone())),
            Err(AppError::Validation(_))
        ));

        let rec = login_with(&w, &[]);
        let mut flagged = walk_in_input(&w, "2026-10-23", three);
        flagged.exceed_capacity = true;
        assert!(matches!(w.h.stays.walk_in(&rec, flagged.clone()), Err(AppError::Forbidden)));
        assert!(w.h.stays.walk_in(&w.admin, flagged).is_ok());
    }

    #[test]
    fn walk_in_with_past_or_same_day_checkout_is_refused() {
        let w = world();

        for out in ["2026-10-21", "2026-10-20"] {
            let result = w.h.stays.walk_in(&w.admin, walk_in_input(&w, out, vec![holder(&w)]));
            assert!(matches!(result, Err(AppError::Validation(_))), "{out}");
        }
        assert_eq!(count(&w, "stays"), 0);
    }

    #[test]
    fn walk_in_sequence_does_not_consume_reservation_numbers() {
        let w = world();
        w.h.stays.walk_in(&w.admin, walk_in_input(&w, "2026-10-23", vec![holder(&w)])).unwrap();

        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-25", "2026-10-26");

        let code = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.code;
        assert_eq!(code, "RES-000001");
    }
}
