use chrono::{NaiveDate, NaiveTime};
use serde_json::json;

use super::rate_service::quote_in;
use super::room_rules;
use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::availability::find_conflicts;
use crate::domain::{cancellation, dates, money};
use crate::error::AppError;
use crate::models::page::{clamp_paging, Page};
use crate::models::reservation::{
    AvailabilityResult, CancellationOverride, CancellationPreview, CancellationResult, ChangePreview, ChangeRequest,
    ConflictInfo, CreatedReservation, DiscountInput, NewReservation, NightChange, Reservation, ReservationDetail,
    ReservationQuery, ReviewItem,
};
use crate::repositories::reservation_repo::{self, ChangedFields, NewRow};
use crate::repositories::{block_repo, catalog_repo, guest_repo, policy_repo, sequence_repo, settings_repo};

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

/// `HH:MM` estrito (cinco caracteres).
pub(crate) fn parse_time(text: &str) -> Option<NaiveTime> {
    if text.len() != 5 {
        return None;
    }
    NaiveTime::parse_from_str(text, "%H:%M").ok()
}

pub(crate) fn parse_date(text: &str, label: &str) -> Result<NaiveDate, AppError> {
    dates::parse(text).ok_or_else(|| invalid(format!("{label} inválida. Use uma data válida.")))
}

pub(crate) fn br(date: &str) -> String {
    match dates::parse(date) {
        Some(d) => d.format("%d/%m/%Y").to_string(),
        None => date.to_string(),
    }
}

pub(crate) fn conflict_message(conflicts: &[ConflictInfo]) -> String {
    let list = conflicts
        .iter()
        .map(|c| format!("{} ({}, {} a {})", c.code, c.guest_name, br(&c.check_in_date), br(&c.check_out_date)))
        .collect::<Vec<_>>()
        .join("; ");
    format!("O quarto já está reservado nesse período: {list}.")
}

/// Horários previstos (os informados ou os padrão das configurações).
pub(crate) fn resolve_times(
    in_time: Option<&str>,
    out_time: Option<&str>,
    default_in: &str,
    default_out: &str,
) -> Result<(NaiveTime, NaiveTime), AppError> {
    let check_in = parse_time(in_time.unwrap_or(default_in)).ok_or_else(|| invalid("Horário de entrada inválido. Use HH:MM."))?;
    let check_out = parse_time(out_time.unwrap_or(default_out)).ok_or_else(|| invalid("Horário de saída inválido. Use HH:MM."))?;
    Ok((check_in, check_out))
}

#[derive(Clone)]
pub struct ReservationService {
    ctx: Ctx,
}

impl ReservationService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    /// Cria uma reserva **pendente** (já bloqueia o quarto). Tudo — tarifa, disponibilidade,
    /// código e gravação — acontece numa única transação (spec §2.3, §4.1).
    pub fn create(&self, token: &str, input: NewReservation) -> Result<CreatedReservation, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let today = self.ctx.today();
        let check_in = parse_date(&input.check_in_date, "Data de entrada")?;
        let check_out = parse_date(&input.check_out_date, "Data de saída")?;
        if check_out <= check_in {
            return Err(invalid("A saída deve ser depois da entrada."));
        }
        if check_in < today {
            return Err(invalid(
                "A data de entrada não pode estar no passado. Para quem já chegou sem reserva, use o registro de hospedagem sem reserva.",
            ));
        }
        if input.occupants == 0 {
            return Err(invalid("Informe a quantidade de pessoas."));
        }
        let notes = input.notes.trim().to_string();
        if notes.chars().count() > 1000 {
            return Err(invalid("Observações: no máximo 1000 caracteres."));
        }

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let settings = settings_repo::load(&tx)?;
        let (in_time, out_time) = resolve_times(
            input.expected_check_in_time.as_deref(),
            input.expected_check_out_time.as_deref(),
            &settings.default_check_in_time,
            &settings.default_check_out_time,
        )?;

        let room = catalog_repo::find_room(&tx, input.room_id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        if !room.active {
            return Err(invalid("O quarto está desativado e não pode receber reservas."));
        }
        let guest = guest_repo::find(&tx, input.guest_id)?.ok_or_else(|| invalid("Hóspede não encontrado."))?;
        if !guest.active {
            return Err(invalid("O cadastro do hóspede está desativado."));
        }
        if i64::from(input.occupants) > room.max_capacity {
            if !input.exceed_capacity {
                return Err(invalid(format!(
                    "O quarto {} comporta {} pessoa(s). Reduza a quantidade ou peça autorização para exceder a capacidade.",
                    room.code, room.max_capacity
                )));
            }
            require(&actor, Permission::ExceedCapacity)?;
        }

        let quote = quote_in(&tx, room.category_id, check_in, check_out)?;
        if !quote.missing_dates.is_empty() {
            let missing = quote.missing_dates.iter().map(|d| br(d)).collect::<Vec<_>>().join(", ");
            return Err(invalid(format!(
                "Não há tarifa cadastrada para: {missing}. Cadastre as tarifas antes de reservar essas datas."
            )));
        }

        room_rules::ensure_no_blocks(&tx, room.id, check_in, Some(check_out), false)?;
        let existing = reservation_repo::blocking(&tx, room.id, check_in, check_out, self.ctx.local_now())?;
        let bookings: Vec<_> = existing.iter().map(|(b, _)| b.clone()).collect();
        let clashes = find_conflicts(&bookings, check_in, in_time, check_out, out_time, room_rules::cleaning_minutes(&tx, room.id, &settings)?, None);
        if !clashes.is_empty() {
            let infos: Vec<ConflictInfo> =
                existing.into_iter().filter(|(b, _)| clashes.contains(&b.id)).map(|(_, info)| info).collect();
            return Err(invalid(conflict_message(&infos)));
        }

        let deposit_expected = money::percent_of(quote.total_cents, settings.deposit_percent_bp)?;
        let due = cancellation::deposit_due_date(
            today,
            check_in,
            settings.deposit_due_days_after_booking,
            settings.deposit_due_days_before_arrival,
        );
        let number = sequence_repo::next(&tx, "reservation")?;
        let code = sequence_repo::format_code("RES", number);
        let (check_in_s, check_out_s, due_s) = (dates::format(check_in), dates::format(check_out), dates::format(due));
        let (in_s, out_s) = (in_time.format("%H:%M").to_string(), out_time.format("%H:%M").to_string());

        let id = reservation_repo::insert(
            &tx,
            &NewRow {
                code: &code,
                room_id: room.id,
                guest_id: guest.id,
                check_in: &check_in_s,
                check_out: &check_out_s,
                in_time: &in_s,
                out_time: &out_s,
                occupants: input.occupants,
                list_total_cents: quote.total_cents,
                agreed_total_cents: quote.total_cents,
                deposit_expected_cents: deposit_expected,
                deposit_due_date: &due_s,
                notes: &notes,
                created_by: actor.user_id,
                status: "pending",
                origin: "booking",
            },
            self.ctx.utc.now_utc(),
        )?;
        for night in &quote.nights {
            reservation_repo::insert_night(&tx, id, &night.date, night.amount_cents, night.rule.as_str())?;
        }
        audit::record(
            &tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "reservations",
                operation: "create_reservation",
                entity: "reservation",
                entity_id: Some(id),
                before: None,
                after: Some(json!({
                    "code": code, "room": room.code, "guest": guest.full_name,
                    "check_in_date": check_in_s, "check_out_date": check_out_s,
                    "occupants": input.occupants, "agreed_total_cents": quote.total_cents,
                    "deposit_expected_cents": deposit_expected, "deposit_due_date": due_s,
                    "exceeded_capacity": i64::from(input.occupants) > room.max_capacity
                })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(CreatedReservation {
            id,
            code,
            agreed_total_cents: quote.total_cents,
            deposit_expected_cents: deposit_expected,
            deposit_due_date: due_s,
        })
    }

    pub fn get(&self, token: &str, id: i64) -> Result<ReservationDetail, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        let reservation =
            reservation_repo::find(&conn, id, self.ctx.today())?.ok_or_else(|| invalid("Reserva não encontrada."))?;
        Ok(ReservationDetail {
            nights: reservation_repo::nights(&conn, id)?,
            history: reservation_repo::changes(&conn, id)?,
            stay_id: reservation_repo::stay_id(&conn, id)?,
            reservation,
        })
    }

    pub fn list(&self, token: &str, query: ReservationQuery) -> Result<Page<Reservation>, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        for (value, label) in [(&query.from, "Data inicial"), (&query.to, "Data final")] {
            if let Some(v) = value {
                parse_date(v, label)?;
            }
        }
        let (page, page_size, offset) = clamp_paging(query.page, query.page_size);
        let conn = self.ctx.conn()?;
        let (items, total) = reservation_repo::list(&conn, &query, self.ctx.today(), i64::from(page_size), offset)?;
        Ok(Page { items, total, page, page_size })
    }

    /// Consulta de disponibilidade do quarto no período (não grava nada).
    pub fn check_availability(
        &self,
        token: &str,
        room_id: i64,
        check_in: &str,
        check_out: &str,
        ignore_reservation_id: Option<i64>,
    ) -> Result<AvailabilityResult, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let check_in = parse_date(check_in, "Data de entrada")?;
        let check_out = parse_date(check_out, "Data de saída")?;
        if check_out <= check_in {
            return Err(invalid("A saída deve ser depois da entrada."));
        }
        let conn = self.ctx.conn()?;
        let settings = settings_repo::load(&conn)?;
        let (in_time, out_time) =
            resolve_times(None, None, &settings.default_check_in_time, &settings.default_check_out_time)?;
        let existing = reservation_repo::blocking(&conn, room_id, check_in, check_out, self.ctx.local_now())?;
        let bookings: Vec<_> = existing.iter().map(|(b, _)| b.clone()).collect();
        let clashes = find_conflicts(
            &bookings,
            check_in,
            in_time,
            check_out,
            out_time,
            room_rules::cleaning_minutes(&conn, room_id, &settings)?,
            ignore_reservation_id,
        );
        let conflicts: Vec<ConflictInfo> =
            existing.into_iter().filter(|(b, _)| clashes.contains(&b.id)).map(|(_, info)| info).collect();
        let blocks = block_repo::overlapping(&conn, room_id, check_in, Some(check_out), false)?;
        let not_ready = if check_in <= self.ctx.today() {
            let offset = i64::from(self.ctx.utc.local_offset().local_minus_utc());
            let minutes = room_rules::cleaning_minutes(&conn, room_id, &settings)?;
            let code = catalog_repo::find_room(&conn, room_id)?.map(|r| r.code).unwrap_or_default();
            match room_rules::ensure_room_clean(&conn, room_id, &code, minutes, self.ctx.local_now(), offset) {
                Err(AppError::Validation(message)) => Some(message),
                Err(other) => return Err(other),
                Ok(()) => None,
            }
        } else {
            None
        };
        Ok(AvailabilityResult { available: conflicts.is_empty() && blocks.is_empty(), conflicts, blocks, not_ready })
    }
}

// ---- alteração de datas ou quarto (prévia + confirmação) ----

/// Resultado do cálculo de uma alteração, compartilhado pela prévia e pela confirmação.
struct ChangePlan {
    reservation: Reservation,
    old_room_code: String,
    new_room_id: i64,
    new_room_code: String,
    check_in: NaiveDate,
    check_out: NaiveDate,
    nights: Vec<NightChange>,
    list_total_cents: i64,
    agreed_total_cents: i64,
    deposit_expected_cents: i64,
    missing_dates: Vec<String>,
    conflicts: Vec<ConflictInfo>,
    problems: Vec<String>,
}

impl ChangePlan {
    fn feasible(&self) -> bool {
        self.problems.is_empty() && self.missing_dates.is_empty() && self.conflicts.is_empty()
    }

    /// Resume tudo o que o usuário viu na prévia; se qualquer coisa mudar, a confirmação é recusada.
    fn fingerprint(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.reservation.version.hash(&mut hasher);
        self.new_room_id.hash(&mut hasher);
        self.check_in.hash(&mut hasher);
        self.check_out.hash(&mut hasher);
        for night in &self.nights {
            (night.date.as_str(), night.amount_cents, night.kept).hash(&mut hasher);
        }
        (self.agreed_total_cents, self.deposit_expected_cents).hash(&mut hasher);
        for conflict in &self.conflicts {
            conflict.reservation_id.hash(&mut hasher);
        }
        self.missing_dates.hash(&mut hasher);
        self.problems.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    fn first_problem(&self) -> String {
        if let Some(p) = self.problems.first() {
            p.clone()
        } else if !self.conflicts.is_empty() {
            conflict_message(&self.conflicts)
        } else {
            let missing = self.missing_dates.iter().map(|d| br(d)).collect::<Vec<_>>().join(", ");
            format!("Não há tarifa cadastrada para: {missing}.")
        }
    }
}

fn plan_change(
    conn: &rusqlite::Connection,
    today: NaiveDate,
    now: chrono::NaiveDateTime,
    id: i64,
    request: &ChangeRequest,
) -> Result<ChangePlan, AppError> {
    let reservation = reservation_repo::find(conn, id, today)?.ok_or_else(|| invalid("Reserva não encontrada."))?;
    if !matches!(reservation.status.as_str(), "pending" | "confirmed") {
        return Err(invalid("Só reservas pendentes ou confirmadas podem ser alteradas."));
    }
    let old_room = catalog_repo::find_room(conn, reservation.room_id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
    let new_room = match request.room_id {
        Some(room_id) => catalog_repo::find_room(conn, room_id)?.ok_or_else(|| invalid("Quarto não encontrado."))?,
        None => old_room.clone(),
    };
    let old_in = parse_date(&reservation.check_in_date, "Data de entrada")?;
    let old_out = parse_date(&reservation.check_out_date, "Data de saída")?;
    let check_in = match &request.check_in_date {
        Some(d) => parse_date(d, "Data de entrada")?,
        None => old_in,
    };
    let check_out = match &request.check_out_date {
        Some(d) => parse_date(d, "Data de saída")?,
        None => old_out,
    };
    if check_out <= check_in {
        return Err(invalid("A saída deve ser depois da entrada."));
    }

    let mut problems = Vec::new();
    if new_room.id == old_room.id && check_in == old_in && check_out == old_out {
        problems.push("Nenhuma alteração informada.".to_string());
    }
    if !new_room.active {
        problems.push("O quarto escolhido está desativado.".to_string());
    }
    if check_in != old_in && check_in < today {
        problems.push("A nova data de entrada não pode estar no passado.".to_string());
    }
    if new_room.id != old_room.id && reservation.occupants > new_room.max_capacity {
        problems.push(format!(
            "O quarto {} comporta {} pessoa(s) e a reserva tem {}.",
            new_room.code, new_room.max_capacity, reservation.occupants
        ));
    }

    // Noites que continuam na mesma categoria mantêm o preço combinado; as demais usam a tabela vigente.
    let same_category = old_room.category_id == new_room.category_id;
    let old_nights = reservation_repo::nights(conn, id)?;
    let quote = quote_in(conn, new_room.category_id, check_in, check_out)?;
    let mut nights = Vec::new();
    let mut missing_dates = Vec::new();
    let mut day = check_in;
    while day < check_out {
        let date = dates::format(day);
        if let Some(old) = old_nights.iter().find(|n| n.date == date).filter(|_| same_category) {
            nights.push(NightChange { date, amount_cents: old.amount_cents, kept: true, rule: old.rule.clone() });
        } else if let Some(priced) = quote.nights.iter().find(|n| n.date == date) {
            nights.push(NightChange { date, amount_cents: priced.amount_cents, kept: false, rule: priced.rule.as_str().into() });
        } else {
            missing_dates.push(date);
        }
        day += chrono::Duration::days(1);
    }

    let list_total_cents = money::sum(nights.iter().map(|n| n.amount_cents))?;
    if reservation.discount_cents > list_total_cents {
        problems.push("O desconto já aplicado é maior que o novo valor da reserva. Revise o desconto antes.".to_string());
    }
    let agreed_total_cents = list_total_cents.saturating_sub(reservation.discount_cents);
    let settings = settings_repo::load(conn)?;
    let deposit_expected_cents = money::percent_of(agreed_total_cents, settings.deposit_percent_bp)?;

    let (in_time, out_time) = resolve_times(
        Some(&reservation.expected_check_in_time),
        Some(&reservation.expected_check_out_time),
        &settings.default_check_in_time,
        &settings.default_check_out_time,
    )?;
    let existing = reservation_repo::blocking(conn, new_room.id, check_in, check_out, now)?;
    let bookings: Vec<_> = existing.iter().map(|(b, _)| b.clone()).collect();
    let clashes = find_conflicts(&bookings, check_in, in_time, check_out, out_time, room_rules::cleaning_minutes(conn, new_room.id, &settings)?, Some(id));
    let conflicts: Vec<ConflictInfo> =
        existing.into_iter().filter(|(b, _)| clashes.contains(&b.id)).map(|(_, info)| info).collect();

    let blocks = block_repo::overlapping(conn, new_room.id, check_in, Some(check_out), false)?;
    if !blocks.is_empty() {
        problems.push(room_rules::blocks_message(&blocks));
    }

    Ok(ChangePlan {
        old_room_code: old_room.code.clone(),
        new_room_id: new_room.id,
        new_room_code: new_room.code.clone(),
        reservation,
        check_in,
        check_out,
        nights,
        list_total_cents,
        agreed_total_cents,
        deposit_expected_cents,
        missing_dates,
        conflicts,
        problems,
    })
}

impl ReservationService {
    /// Mostra o efeito de uma alteração **sem gravar nada**: preço anterior, novo e diferença.
    pub fn preview_change(&self, token: &str, id: i64, request: ChangeRequest) -> Result<ChangePreview, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ChangeReservation)?;
        let conn = self.ctx.conn()?;
        let plan = plan_change(&conn, self.ctx.today(), self.ctx.local_now(), id, &request)?;
        Ok(ChangePreview {
            feasible: plan.feasible(),
            fingerprint: plan.fingerprint(),
            price_before_cents: plan.reservation.agreed_total_cents,
            price_after_cents: plan.agreed_total_cents,
            difference_cents: plan.agreed_total_cents - plan.reservation.agreed_total_cents,
            new_deposit_expected_cents: plan.deposit_expected_cents,
            nights: plan.nights,
            missing_dates: plan.missing_dates,
            conflicts: plan.conflicts,
            problems: plan.problems,
        })
    }

    /// Grava a alteração depois da confirmação. Recalcula tudo na transação; se a prévia deixou de
    /// valer (tabela de preços, disponibilidade ou reserva mudaram), recusa e pede nova prévia.
    pub fn apply_change(
        &self,
        token: &str,
        id: i64,
        request: ChangeRequest,
        expected_version: i64,
        justification: String,
        fingerprint: &str,
    ) -> Result<(), AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ChangeReservation)?;
        let justification = justification.trim().to_string();
        if justification.is_empty() {
            return Err(invalid("Informe a justificativa da alteração."));
        }

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let plan = plan_change(&tx, self.ctx.today(), self.ctx.local_now(), id, &request)?;
        if plan.reservation.version != expected_version {
            return Err(AppError::Conflict);
        }
        if plan.fingerprint() != fingerprint {
            return Err(AppError::StalePreview);
        }
        if !plan.feasible() {
            return Err(invalid(plan.first_problem()));
        }

        let before = json!({
            "room": plan.old_room_code,
            "check_in_date": plan.reservation.check_in_date,
            "check_out_date": plan.reservation.check_out_date,
            "agreed_total_cents": plan.reservation.agreed_total_cents,
        });
        let after = json!({
            "room": plan.new_room_code,
            "check_in_date": dates::format(plan.check_in),
            "check_out_date": dates::format(plan.check_out),
            "agreed_total_cents": plan.agreed_total_cents,
        });
        let at = self.ctx.utc.now_utc();
        reservation_repo::apply_change(
            &tx,
            id,
            &ChangedFields {
                room_id: plan.new_room_id,
                check_in: &dates::format(plan.check_in),
                check_out: &dates::format(plan.check_out),
                list_total_cents: plan.list_total_cents,
                agreed_total_cents: plan.agreed_total_cents,
                deposit_expected_cents: plan.deposit_expected_cents,
            },
            expected_version,
            at,
        )?;
        let nights: Vec<_> = plan.nights.iter().map(|n| (n.date.clone(), n.amount_cents, n.rule.clone())).collect();
        reservation_repo::replace_nights(&tx, id, &nights)?;
        reservation_repo::insert_change(
            &tx,
            id,
            at,
            actor.user_id,
            "dates_room",
            &before.to_string(),
            &after.to_string(),
            Some(plan.reservation.agreed_total_cents),
            Some(plan.agreed_total_cents),
            Some(&justification),
        )?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "reservations",
                operation: "change_reservation",
                entity: "reservation",
                entity_id: Some(id),
                before: Some(before),
                after: Some(after),
                justification: Some(justification),
            },
        )?;
        tx.commit()?;
        Ok(())
    }
}

// ---- cancelamento, no-show e revisão ----

impl ReservationService {
    /// Mostra quanto do sinal seria devolvido e retido, **sem gravar nada**.
    pub fn preview_cancellation(&self, token: &str, id: i64) -> Result<CancellationPreview, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        let today = self.ctx.today();
        let reservation = reservation_repo::find(&conn, id, today)?.ok_or_else(|| invalid("Reserva não encontrada."))?;
        let check_in = parse_date(&reservation.check_in_date, "Data de entrada")?;
        let days_before = (check_in - today).num_days();
        let can_cancel = matches!(reservation.status.as_str(), "pending" | "confirmed");
        let tiers = policy_repo::list_tiers(&conn)?;
        let outcome = cancellation::compute(&tiers, days_before, reservation.deposit_paid_cents, false)?;
        Ok(CancellationPreview {
            days_before,
            deposit_paid_cents: reservation.deposit_paid_cents,
            refund_cents: outcome.refund_cents,
            retained_cents: outcome.retained_cents,
            tier_min_days: outcome.tier_min_days,
            can_cancel,
        })
    }

    /// Cancela a reserva preservando o registro. O valor padrão vem da política; uma exceção exige
    /// `CancellationException` e justificativa, e a auditoria guarda também o valor calculado.
    pub fn cancel(
        &self,
        token: &str,
        id: i64,
        expected_version: i64,
        exception: Option<CancellationOverride>,
    ) -> Result<CancellationResult, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let today = self.ctx.today();
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let reservation = reservation_repo::find(&tx, id, today)?.ok_or_else(|| invalid("Reserva não encontrada."))?;
        if !matches!(reservation.status.as_str(), "pending" | "confirmed") {
            return Err(invalid("Só reservas pendentes ou confirmadas podem ser canceladas."));
        }
        if reservation.version != expected_version {
            return Err(AppError::Conflict);
        }
        let check_in = parse_date(&reservation.check_in_date, "Data de entrada")?;
        let tiers = policy_repo::list_tiers(&tx)?;
        let calculated =
            cancellation::compute(&tiers, (check_in - today).num_days(), reservation.deposit_paid_cents, false)?;

        let (retained, justification) = match &exception {
            None => (calculated.retained_cents, None),
            Some(exception) => {
                require(&actor, Permission::CancellationException)?;
                let why = exception.justification.trim();
                if why.is_empty() {
                    return Err(invalid("Informe a justificativa da exceção."));
                }
                if exception.retained_cents < 0 || exception.retained_cents > reservation.deposit_paid_cents {
                    return Err(invalid("O valor retido deve estar entre zero e o sinal já pago."));
                }
                (exception.retained_cents, Some(why.to_string()))
            }
        };
        let refund = money::sub(reservation.deposit_paid_cents, retained)?;

        let at = self.ctx.utc.now_utc();
        reservation_repo::set_closed(&tx, id, "cancelled", retained, expected_version, at)?;
        let before = json!({ "status": reservation.status, "deposit_paid_cents": reservation.deposit_paid_cents });
        let after = json!({
            "status": "cancelled", "refund_cents": refund, "retained_cents": retained,
            "calculated_refund_cents": calculated.refund_cents, "calculated_retained_cents": calculated.retained_cents,
            "tier_min_days": calculated.tier_min_days, "overridden": exception.is_some()
        });
        reservation_repo::insert_change(
            &tx, id, at, actor.user_id, "cancellation", &before.to_string(), &after.to_string(), None, None, justification.as_deref(),
        )?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "reservations",
                operation: "cancel_reservation",
                entity: "reservation",
                entity_id: Some(id),
                before: Some(before),
                after: Some(after),
                justification,
            },
        )?;
        tx.commit()?;
        Ok(CancellationResult { refund_cents: refund, retained_cents: retained })
    }

    /// Marca no-show (decisão manual da recepção) depois do horário previsto de chegada mais a tolerância.
    /// O sinal fica retido e o quarto é liberado.
    pub fn mark_no_show(&self, token: &str, id: i64, expected_version: i64) -> Result<(), AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let today = self.ctx.today();
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let reservation = reservation_repo::find(&tx, id, today)?.ok_or_else(|| invalid("Reserva não encontrada."))?;
        if !matches!(reservation.status.as_str(), "pending" | "confirmed") {
            return Err(invalid("Só reservas pendentes ou confirmadas podem ser marcadas como no-show."));
        }
        if reservation.version != expected_version {
            return Err(AppError::Conflict);
        }
        let settings = settings_repo::load(&tx)?;
        if !self.arrival_overdue(&reservation, settings.no_show_tolerance_minutes)? {
            return Err(invalid(format!(
                "O no-show só pode ser marcado depois de {} minuto(s) do horário previsto de chegada.",
                settings.no_show_tolerance_minutes
            )));
        }

        let at = self.ctx.utc.now_utc();
        reservation_repo::set_closed(&tx, id, "no_show", reservation.deposit_paid_cents, expected_version, at)?;
        let before = json!({ "status": reservation.status });
        let after = json!({ "status": "no_show", "retained_cents": reservation.deposit_paid_cents });
        reservation_repo::insert_change(&tx, id, at, actor.user_id, "no_show", &before.to_string(), &after.to_string(), None, None, None)?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "reservations",
                operation: "mark_no_show",
                entity: "reservation",
                entity_id: Some(id),
                before: Some(before),
                after: Some(after),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    /// A chegada prevista (data + horário) mais a tolerância já passou?
    fn arrival_overdue(&self, reservation: &Reservation, tolerance_minutes: u32) -> Result<bool, AppError> {
        let date = parse_date(&reservation.check_in_date, "Data de entrada")?;
        let time = parse_time(&reservation.expected_check_in_time)
            .ok_or_else(|| AppError::Database("horário previsto inválido na reserva".into()))?;
        let deadline = date.and_time(time) + chrono::Duration::minutes(i64::from(tolerance_minutes));
        Ok(self.ctx.local_now() >= deadline)
    }

    /// Reservas que pedem uma decisão da recepção (nunca altera nada): chegada vencida (candidata a
    /// no-show) e sinal vencido.
    pub fn list_needing_review(&self, token: &str) -> Result<Vec<ReviewItem>, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        let settings = settings_repo::load(&conn)?;
        let mut items = Vec::new();
        for reservation in reservation_repo::review_candidates(&conn, self.ctx.today())? {
            let mut reasons = Vec::new();
            if self.arrival_overdue(&reservation, settings.no_show_tolerance_minutes)? {
                reasons.push("no_show_candidate".to_string());
            }
            if reservation.deposit_overdue {
                reasons.push("deposit_overdue".to_string());
            }
            if !reasons.is_empty() {
                items.push(ReviewItem { reservation, reasons });
            }
        }
        Ok(items)
    }
}

// ---- desconto, confirmação, sinal e observações ----

impl ReservationService {
    /// Carrega a reserva dentro da transação, exigindo estado e versão.
    fn load_open(
        &self,
        tx: &rusqlite::Transaction,
        id: i64,
        expected_version: i64,
        what: &str,
    ) -> Result<Reservation, AppError> {
        let reservation =
            reservation_repo::find(tx, id, self.ctx.today())?.ok_or_else(|| invalid("Reserva não encontrada."))?;
        if !matches!(reservation.status.as_str(), "pending" | "confirmed") {
            return Err(invalid(format!("Só reservas pendentes ou confirmadas aceitam {what}.")));
        }
        if reservation.version != expected_version {
            return Err(AppError::Conflict);
        }
        Ok(reservation)
    }

    /// Aplica (ou troca) o desconto da reserva. Sempre calculado sobre o valor de tabela das noites.
    pub fn apply_discount(
        &self,
        token: &str,
        id: i64,
        input: DiscountInput,
        expected_version: i64,
    ) -> Result<(), AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ApplyDiscount)?;
        let justification = input.justification.trim().to_string();
        if justification.is_empty() {
            return Err(invalid("Informe a justificativa do desconto."));
        }

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let reservation = self.load_open(&tx, id, expected_version, "desconto")?;
        let list_total = reservation.list_total_cents;
        let discount = match (input.kind.as_str(), input.amount_cents, input.percent_bp) {
            ("amount", Some(cents), None) => cents,
            ("percent", None, Some(bp)) if bp <= 10_000 => money::percent_of(list_total, bp)?,
            ("percent", None, Some(_)) => return Err(invalid("O percentual do desconto vai de 0% a 100%.")),
            _ => return Err(invalid("Informe o desconto em valor ou em percentual.")),
        };
        if discount <= 0 {
            return Err(invalid("Informe um desconto maior que zero."));
        }
        if discount > list_total {
            return Err(invalid("O desconto não pode ser maior que o valor da reserva."));
        }

        let settings = settings_repo::load(&tx)?;
        let agreed = list_total - discount;
        let deposit_expected = money::percent_of(agreed, settings.deposit_percent_bp)?;
        let at = self.ctx.utc.now_utc();
        reservation_repo::update_discount(&tx, id, discount, &justification, actor.user_id, agreed, deposit_expected, expected_version, at)?;
        let before = json!({
            "list_total_cents": list_total, "discount_cents": reservation.discount_cents,
            "agreed_total_cents": reservation.agreed_total_cents
        });
        let after = json!({
            "list_total_cents": list_total, "discount_cents": discount, "agreed_total_cents": agreed,
            "kind": input.kind
        });
        reservation_repo::insert_change(
            &tx, id, at, actor.user_id, "discount", &before.to_string(), &after.to_string(),
            Some(reservation.agreed_total_cents), Some(agreed), Some(&justification),
        )?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "reservations",
                operation: "apply_discount",
                entity: "reservation",
                entity_id: Some(id),
                before: Some(before),
                after: Some(after),
                justification: Some(justification),
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Confirma manualmente uma reserva pendente (D-06).
    pub fn confirm(&self, token: &str, id: i64, expected_version: i64) -> Result<(), AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let reservation = self.load_open(&tx, id, expected_version, "confirmação")?;
        if reservation.status != "pending" {
            return Err(invalid("A reserva já está confirmada."));
        }
        let at = self.ctx.utc.now_utc();
        reservation_repo::update_payment_state(&tx, id, reservation.deposit_paid_cents, "confirmed", expected_version, at)?;
        self.record_status_change(&tx, &actor, id, "confirm_reservation", "pending", "confirmed", at)?;
        tx.commit()?;
        Ok(())
    }

    /// Registra um valor de sinal recebido. (Provisório até o módulo de recebimentos, Marco 5.)
    /// Ao atingir o sinal esperado, a reserva pendente passa a confirmada.
    pub fn record_deposit(&self, token: &str, id: i64, amount_cents: i64, expected_version: i64) -> Result<(), AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        if amount_cents <= 0 {
            return Err(invalid("Informe um valor maior que zero."));
        }
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let reservation = self.load_open(&tx, id, expected_version, "registro de sinal")?;
        let paid = money::add(reservation.deposit_paid_cents, amount_cents)?;
        if paid > reservation.agreed_total_cents {
            return Err(invalid("O sinal recebido não pode passar do valor total da reserva."));
        }
        let new_status = if reservation.status == "pending" && paid >= reservation.deposit_expected_cents {
            "confirmed"
        } else {
            reservation.status.as_str()
        };
        let at = self.ctx.utc.now_utc();
        reservation_repo::update_payment_state(&tx, id, paid, new_status, expected_version, at)?;
        let before = json!({ "deposit_paid_cents": reservation.deposit_paid_cents, "status": reservation.status });
        let after = json!({ "deposit_paid_cents": paid, "status": new_status, "received_cents": amount_cents });
        reservation_repo::insert_change(&tx, id, at, actor.user_id, "deposit", &before.to_string(), &after.to_string(), None, None, None)?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "reservations",
                operation: "record_deposit",
                entity: "reservation",
                entity_id: Some(id),
                before: Some(before),
                after: Some(after),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_notes(&self, token: &str, id: i64, notes: &str, expected_version: i64) -> Result<(), AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let notes = notes.trim();
        if notes.chars().count() > 1000 {
            return Err(invalid("Observações: no máximo 1000 caracteres."));
        }
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let reservation = reservation_repo::find(&tx, id, self.ctx.today())?.ok_or_else(|| invalid("Reserva não encontrada."))?;
        if reservation.version != expected_version {
            return Err(AppError::Conflict);
        }
        let at = self.ctx.utc.now_utc();
        reservation_repo::update_notes(&tx, id, notes, expected_version, at)?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "reservations",
                operation: "update_reservation_notes",
                entity: "reservation",
                entity_id: Some(id),
                before: Some(json!({ "notes": reservation.notes })),
                after: Some(json!({ "notes": notes })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn record_status_change(
        &self,
        tx: &rusqlite::Transaction,
        actor: &crate::auth::session::SessionInfo,
        id: i64,
        operation: &'static str,
        from: &str,
        to: &str,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), AppError> {
        let (before, after) = (json!({ "status": from }), json!({ "status": to }));
        reservation_repo::insert_change(tx, id, at, actor.user_id, "status", &before.to_string(), &after.to_string(), None, None, None)?;
        audit::record(
            tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "reservations",
                operation,
                entity: "reservation",
                entity_id: Some(id),
                before: Some(before),
                after: Some(after),
                justification: None,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::permissions::Permission;
    use crate::error::AppError;
    use crate::models::catalog::{NewCategory, NewRoom};
    use crate::models::guest::GuestInput;
    use crate::models::rate::NewRateRule;
    use crate::models::reservation::{CancellationOverride, ChangeRequest, DiscountInput, NewReservation, ReservationQuery};
    use crate::services::guest_service::CreateGuestOutcome;
    use crate::services::test_support::{admin_token, harness, recepcao_token, Harness};
    use std::time::Duration;

    // "Hoje" nos testes é 21/10/2026 (relógio injetado, fuso -03:00).

    struct World {
        h: Harness,
        admin: String,
        category: i64,
        room: i64,
        guest: i64,
    }

    fn weekday_rule(category: i64, day: u8, cents: i64) -> NewRateRule {
        NewRateRule {
            category_id: category,
            rule_type: "weekday".into(),
            weekday: Some(day),
            start_date: None,
            end_date: None,
            date: None,
            amount_cents: cents,
            label: String::new(),
        }
    }

    fn world() -> World {
        let h = harness();
        let admin = admin_token(&h);
        let category = h
            .catalog
            .create_category(&admin, NewCategory { name: "Standard".into(), description: String::new() })
            .unwrap();
        for day in 0..7 {
            h.rates.create_rule(&admin, weekday_rule(category, day, 10_000)).unwrap();
        }
        let room = new_room(&h, &admin, category, "101", 2);
        let guest = new_guest(&h, &admin, "Ana Souza", "(11) 99999-0000");
        World { h, admin, category, room, guest }
    }

    fn new_room(h: &Harness, admin: &str, category: i64, code: &str, capacity: i64) -> i64 {
        h.catalog
            .create_room(
                admin,
                NewRoom { code: code.into(), category_id: category, max_capacity: capacity, description: String::new(), notes: String::new() },
            )
            .unwrap()
    }

    fn new_guest(h: &Harness, admin: &str, name: &str, phone: &str) -> i64 {
        match h
            .guests
            .create(
                admin,
                GuestInput { full_name: name.into(), phone: phone.into(), cpf: None, email: String::new(), address: String::new() },
                true,
            )
            .unwrap()
        {
            CreateGuestOutcome::Created { id, .. } => id,
            other => panic!("{other:?}"),
        }
    }

    fn booking(w: &World, from: &str, to: &str) -> NewReservation {
        NewReservation {
            room_id: w.room,
            guest_id: w.guest,
            check_in_date: from.into(),
            check_out_date: to.into(),
            expected_check_in_time: None,
            expected_check_out_time: None,
            occupants: 2,
            notes: String::new(),
            exceed_capacity: false,
        }
    }

    fn query() -> ReservationQuery {
        ReservationQuery { text: None, status: None, room_id: None, from: None, to: None, page: 1, page_size: 20, sort: None, direction: None }
    }

    fn set_status(w: &World, id: i64, status: &str) {
        w.h.db.lock().unwrap().execute("UPDATE reservations SET status = ?1 WHERE id = ?2", rusqlite::params![status, id]).unwrap();
    }

    #[test]
    fn creates_a_pending_reservation_with_frozen_nights_and_deposit() {
        let w = world();

        let created = w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-28")).unwrap();

        assert_eq!(created.code, "RES-000001");
        assert_eq!(created.agreed_total_cents, 30_000);
        assert_eq!(created.deposit_expected_cents, 9_000); // 30% provisório
        assert_eq!(created.deposit_due_date, "2026-10-24"); // min(hoje+3, chegada-1)
        let detail = w.h.reservations.get(&w.admin, created.id).unwrap();
        assert_eq!(detail.reservation.status, "pending");
        assert_eq!(detail.reservation.expected_check_in_time, "14:00");
        assert_eq!(detail.reservation.expected_check_out_time, "12:00");
        assert_eq!(detail.reservation.nights, 3);
        let nights: Vec<_> = detail.nights.iter().map(|n| (n.date.as_str(), n.amount_cents, n.rule.as_str())).collect();
        assert_eq!(nights, [("2026-10-25", 10_000, "weekday"), ("2026-10-26", 10_000, "weekday"), ("2026-10-27", 10_000, "weekday")]);
    }

    #[test]
    fn a_booked_reservation_has_origin_booking_and_no_stay() {
        let w = world();
        let created = w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-28")).unwrap();

        let detail = w.h.reservations.get(&w.admin, created.id).unwrap();

        assert_eq!(detail.reservation.origin, "booking");
        assert_eq!(detail.stay_id, None);
    }

    #[test]
    fn a_second_reservation_for_the_same_room_and_period_is_refused() {
        let w = world();
        w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-28")).unwrap();

        let result = w.h.reservations.create(&w.admin, booking(&w, "2026-10-27", "2026-10-30"));

        match result {
            Err(AppError::Validation(msg)) => assert!(msg.contains("RES-000001"), "{msg}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(w.h.reservations.list(&w.admin, query()).unwrap().total, 1);
    }

    #[test]
    fn concurrent_creation_has_exactly_one_winner() {
        let w = world();
        let results: Vec<_> = (0..2)
            .map(|_| {
                let service = w.h.reservations.clone();
                let token = w.admin.clone();
                let request = booking(&w, "2026-11-10", "2026-11-12");
                std::thread::spawn(move || service.create(&token, request).is_ok())
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect();

        assert_eq!(results.iter().filter(|ok| **ok).count(), 1);
        assert_eq!(w.h.reservations.list(&w.admin, query()).unwrap().total, 1);
    }

    #[test]
    fn cancelled_and_no_show_reservations_do_not_block_the_room() {
        let w = world();
        let first = w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-28")).unwrap();
        set_status(&w, first.id, "cancelled");
        let second = w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-28")).unwrap();
        set_status(&w, second.id, "no_show");

        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-26", "2026-10-27")).is_ok());
    }

    #[test]
    fn same_day_turnover_respects_the_cleaning_interval() {
        let w = world();
        w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")).unwrap(); // sai 12:00

        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-27", "2026-10-29")).is_ok()); // entra 14:00
        let mut too_early = booking(&w, "2026-11-05", "2026-11-07");
        w.h.reservations.create(&w.admin, booking(&w, "2026-11-03", "2026-11-05")).unwrap();
        too_early.expected_check_in_time = Some("12:30".into());
        assert!(matches!(w.h.reservations.create(&w.admin, too_early), Err(AppError::Validation(_))));
    }

    #[test]
    fn room_interval_override_changes_the_same_day_turnover() {
        let w = world();
        w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")).unwrap(); // sai 12:00
        w.h.catalog.set_room_cleaning_interval(&w.admin, w.room, Some(180), 1).unwrap();

        // Entrada 14:00 = 120 min depois da saída: cabe no padrão (60), não em 180.
        assert!(matches!(
            w.h.reservations.create(&w.admin, booking(&w, "2026-10-27", "2026-10-29")),
            Err(AppError::Validation(_))
        ));
        let free = w.h.reservations.check_availability(&w.admin, w.room, "2026-10-27", "2026-10-29", None).unwrap();
        assert!(!free.available);

        w.h.catalog.set_room_cleaning_interval(&w.admin, w.room, None, 2).unwrap();
        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-27", "2026-10-29")).is_ok());
    }

    #[test]
    fn missing_tariff_blocks_creation_and_lists_the_dates() {
        let w = world();
        let other_category = w
            .h
            .catalog
            .create_category(&w.admin, NewCategory { name: "Luxo".into(), description: String::new() })
            .unwrap();
        w.h.rates.create_rule(&w.admin, weekday_rule(other_category, 1, 20_000)).unwrap(); // só segundas
        let luxury = new_room(&w.h, &w.admin, other_category, "201", 3);
        let mut request = booking(&w, "2026-10-25", "2026-10-28");
        request.room_id = luxury;

        match w.h.reservations.create(&w.admin, request) {
            Err(AppError::Validation(msg)) => assert!(msg.contains("25/10/2026") && msg.contains("27/10/2026"), "{msg}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(w.h.reservations.list(&w.admin, query()).unwrap().total, 0);
    }

    #[test]
    fn invalid_dates_and_times_are_refused() {
        let w = world();
        let bad = [
            booking(&w, "2026-10-20", "2026-10-23"), // entrada no passado
            booking(&w, "2026-10-25", "2026-10-25"), // sem noites
            booking(&w, "2026-10-28", "2026-10-25"), // saída antes da entrada
            booking(&w, "25/10/2026", "2026-10-27"), // formato
        ];
        for request in bad {
            assert!(matches!(w.h.reservations.create(&w.admin, request), Err(AppError::Validation(_))));
        }
        let mut wrong_time = booking(&w, "2026-10-25", "2026-10-27");
        wrong_time.expected_check_in_time = Some("25:00".into());
        assert!(matches!(w.h.reservations.create(&w.admin, wrong_time), Err(AppError::Validation(_))));
        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-21", "2026-10-22")).is_ok()); // hoje vale
    }

    #[test]
    fn inactive_room_or_guest_is_refused_and_unknown_ids_too() {
        let w = world();
        w.h.catalog.set_room_active(&w.admin, w.room, false).unwrap();
        assert!(matches!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")), Err(AppError::Validation(_))));
        w.h.catalog.set_room_active(&w.admin, w.room, true).unwrap();
        w.h.guests.set_active(&w.admin, w.guest, false).unwrap();
        assert!(matches!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")), Err(AppError::Validation(_))));
        let mut unknown = booking(&w, "2026-10-25", "2026-10-27");
        unknown.room_id = 999;
        assert!(matches!(w.h.reservations.create(&w.admin, unknown), Err(AppError::Validation(_))));
    }

    #[test]
    fn exceeding_capacity_needs_the_permission() {
        let w = world();
        let rec = recepcao_token(&w.h, &w.admin);
        let mut over = booking(&w, "2026-10-25", "2026-10-27");
        over.occupants = 3;

        assert!(matches!(w.h.reservations.create(&w.admin, over.clone()), Err(AppError::Validation(_))));
        over.exceed_capacity = true;
        assert!(matches!(w.h.reservations.create(&rec, over.clone()), Err(AppError::Forbidden)));
        assert!(w.h.reservations.create(&w.admin, over.clone()).is_ok());

        // Recepção com a permissão extra concedida pelo administrador.
        let rec_id = w.h.auth.login("rec", "Definitiva-456").unwrap().user.id;
        w.h.users.set_extra_permissions(&w.admin, rec_id, &[Permission::ExceedCapacity]).unwrap();
        let rec = w.h.auth.login("rec", "Definitiva-456").unwrap().token;
        let mut other_period = over.clone();
        other_period.check_in_date = "2026-11-10".into();
        other_period.check_out_date = "2026-11-12".into();
        assert!(w.h.reservations.create(&rec, other_period).is_ok());
    }

    #[test]
    fn a_later_rate_change_does_not_alter_an_existing_reservation() {
        let w = world();
        let created = w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")).unwrap();
        let rule = w.h.rates.list_rules(&w.admin, Some(w.category)).unwrap().into_iter().next().unwrap();
        w.h.rates.update_rule(&w.admin, rule.id, weekday_rule(w.category, rule.weekday.unwrap(), 99_000), rule.version).unwrap();

        let detail = w.h.reservations.get(&w.admin, created.id).unwrap();

        assert_eq!(detail.reservation.agreed_total_cents, 20_000);
        assert!(detail.nights.iter().all(|n| n.amount_cents == 10_000));
    }

    #[test]
    fn a_refused_attempt_does_not_consume_a_code() {
        let w = world();
        w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")).unwrap();
        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-26", "2026-10-28")).is_err());

        let next = w.h.reservations.create(&w.admin, booking(&w, "2026-11-10", "2026-11-12")).unwrap();

        assert_eq!(next.code, "RES-000002");
    }

    #[test]
    fn creation_is_audited_with_the_final_values() {
        let w = world();
        let created = w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")).unwrap();

        let after: String = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT after_json FROM audit_log WHERE operation = 'create_reservation'", [], |r| r.get(0))
            .unwrap();
        assert!(after.contains(&created.code) && after.contains("20000") && after.contains("2026-10-25"));
    }

    #[test]
    fn recepcao_can_create_and_read() {
        let w = world();
        let rec = recepcao_token(&w.h, &w.admin);

        let created = w.h.reservations.create(&rec, booking(&w, "2026-10-25", "2026-10-27")).unwrap();

        assert!(w.h.reservations.get(&rec, created.id).is_ok());
        assert_eq!(w.h.reservations.list(&rec, query()).unwrap().total, 1);
    }

    #[test]
    fn list_filters_by_status_room_period_and_text() {
        let w = world();
        let second_room = new_room(&w.h, &w.admin, w.category, "102", 2);
        let other_guest = new_guest(&w.h, &w.admin, "Bruno Lima", "(21) 98888-7777");
        let a = w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")).unwrap();
        let mut b = booking(&w, "2026-11-10", "2026-11-13");
        b.room_id = second_room;
        b.guest_id = other_guest;
        let b = w.h.reservations.create(&w.admin, b).unwrap();
        set_status(&w, b.id, "confirmed");
        let count = |q: ReservationQuery| w.h.reservations.list(&w.admin, q).unwrap().total;

        assert_eq!(count(query()), 2);
        assert_eq!(count(ReservationQuery { status: Some(vec!["confirmed".into()]), ..query() }), 1);
        assert_eq!(count(ReservationQuery { status: Some(vec!["pending".into(), "confirmed".into()]), ..query() }), 2);
        assert_eq!(count(ReservationQuery { room_id: Some(second_room), ..query() }), 1);
        assert_eq!(count(ReservationQuery { from: Some("2026-11-01".into()), to: Some("2026-11-30".into()), ..query() }), 1);
        assert_eq!(count(ReservationQuery { text: Some("bruno".into()), ..query() }), 1);
        assert_eq!(count(ReservationQuery { text: Some(a.code.clone()), ..query() }), 1);
        assert_eq!(count(ReservationQuery { text: Some("zzz".into()), ..query() }), 0);
        let page = w.h.reservations.list(&w.admin, ReservationQuery { page_size: 1, page: 2, ..query() }).unwrap();
        assert_eq!((page.total, page.items.len()), (2, 1));
        assert_eq!(page.items[0].room_code, "102");
        assert_eq!(page.items[0].guest_name, "Bruno Lima");
    }

    #[test]
    fn list_flags_an_overdue_deposit_without_changing_the_reservation() {
        let w = world();
        let created = w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")).unwrap(); // vence 24/10
        w.h.utc.advance(chrono::Duration::days(4)); // 25/10
        w.h.clock.advance(Duration::from_secs(1));
        let admin = w.h.auth.login("admin", crate::services::test_support::ADMIN_PASSWORD).unwrap().token;

        let page = w.h.reservations.list(&admin, query()).unwrap();

        assert!(page.items[0].deposit_overdue);
        assert_eq!(page.items[0].status, "pending");
        assert_eq!(page.items[0].id, created.id);
    }

    #[test]
    fn check_availability_reports_the_conflicts() {
        let w = world();
        let existing = w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-28")).unwrap();

        let busy = w.h.reservations.check_availability(&w.admin, w.room, "2026-10-27", "2026-10-30", None).unwrap();
        let free = w.h.reservations.check_availability(&w.admin, w.room, "2026-10-28", "2026-10-30", None).unwrap();
        let itself = w.h.reservations.check_availability(&w.admin, w.room, "2026-10-27", "2026-10-30", Some(existing.id)).unwrap();

        assert!(!busy.available);
        assert_eq!(busy.conflicts[0].code, "RES-000001");
        assert_eq!(busy.conflicts[0].guest_name, "Ana Souza");
        assert!(free.available && free.conflicts.is_empty());
        assert!(itself.available);
    }

    // ---- bloqueios (Marco 4) ----

    fn block(w: &World, from: &str, to: Option<&str>) -> i64 {
        w.h.blocks
            .create(
                &w.admin,
                crate::models::block::NewBlockInput {
                    room_id: w.room,
                    start_date: from.into(),
                    end_date: to.map(String::from),
                    reason: "Pintura".into(),
                },
            )
            .unwrap()
    }

    #[test]
    fn reservation_over_a_block_is_refused() {
        let w = world();
        block(&w, "2026-10-25", Some("2026-10-28"));

        let result = w.h.reservations.create(&w.admin, booking(&w, "2026-10-24", "2026-10-26"));

        match result {
            Err(AppError::Validation(m)) => assert!(m.contains("bloqueado") && m.contains("Pintura"), "{m}"),
            other => panic!("esperava recusa: {other:?}"),
        }
        assert_eq!(counts(&w).0, 0);
    }

    #[test]
    fn reservation_ending_the_day_the_block_starts_is_allowed() {
        let w = world();
        block(&w, "2026-10-25", Some("2026-10-28"));

        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-23", "2026-10-25")).is_ok());
        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-28", "2026-10-29")).is_ok());
    }

    #[test]
    fn open_ended_block_blocks_every_future_date() {
        let w = world();
        block(&w, "2026-11-01", None);

        let result = w.h.reservations.create(&w.admin, booking(&w, "2027-06-01", "2027-06-03"));

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn released_block_stops_refusing_reservations() {
        let w = world();
        let id = block(&w, "2026-10-25", Some("2026-10-28"));
        w.h.blocks.release(&w.admin, id, "Obra terminou".into(), 1).unwrap();

        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")).is_ok());
    }

    #[test]
    fn change_into_a_blocked_period_is_unfeasible() {
        let w = world();
        let id = reservation(&w, "2026-10-22", "2026-10-24");
        block(&w, "2026-10-26", Some("2026-10-29"));

        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, Some("2026-10-27"), Some("2026-10-28"))).unwrap();

        assert!(!preview.feasible);
        assert!(preview.problems.iter().any(|p| p.contains("bloqueado")), "{:?}", preview.problems);
    }

    #[test]
    fn check_availability_reports_the_block() {
        let w = world();
        block(&w, "2026-10-25", Some("2026-10-28"));

        let busy = w.h.reservations.check_availability(&w.admin, w.room, "2026-10-27", "2026-10-30", None).unwrap();
        let free = w.h.reservations.check_availability(&w.admin, w.room, "2026-10-28", "2026-10-30", None).unwrap();

        assert!(!busy.available);
        assert_eq!(busy.blocks.len(), 1);
        assert_eq!(busy.blocks[0].reason, "Pintura");
        assert!(free.available && free.blocks.is_empty());
    }

    // ---- alteração de datas ou quarto ----

    fn change(room: Option<i64>, from: Option<&str>, to: Option<&str>) -> ChangeRequest {
        ChangeRequest { room_id: room, check_in_date: from.map(String::from), check_out_date: to.map(String::from) }
    }

    fn reprice_all(w: &World, category: i64, cents: i64) {
        for rule in w.h.rates.list_rules(&w.admin, Some(category)).unwrap() {
            w.h.rates.update_rule(&w.admin, rule.id, weekday_rule(category, rule.weekday.unwrap(), cents), rule.version).unwrap();
        }
    }

    fn counts(w: &World) -> (i64, i64, i64, i64) {
        let conn = w.h.db.lock().unwrap();
        let n = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
        (
            n("SELECT count(*) FROM reservations"),
            n("SELECT count(*) FROM reservation_nights"),
            n("SELECT count(*) FROM reservation_changes"),
            n("SELECT count(*) FROM audit_log"),
        )
    }

    fn reservation(w: &World, from: &str, to: &str) -> i64 {
        w.h.reservations.create(&w.admin, booking(w, from, to)).unwrap().id
    }

    #[test]
    fn preview_never_writes() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        let before = counts(&w);
        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;

        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-30"))).unwrap();

        assert!(preview.feasible);
        assert_eq!(counts(&w), before);
        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.version, version);
    }

    #[test]
    fn preview_shows_before_after_and_difference() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28"); // 3 noites = R$ 300,00

        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-29"))).unwrap();

        assert_eq!((preview.price_before_cents, preview.price_after_cents, preview.difference_cents), (30_000, 40_000, 10_000));
        let kept: Vec<_> = preview.nights.iter().map(|n| (n.date.as_str(), n.kept)).collect();
        assert_eq!(kept, [("2026-10-25", true), ("2026-10-26", true), ("2026-10-27", true), ("2026-10-28", false)]);
        assert_eq!(preview.new_deposit_expected_cents, 12_000);
    }

    #[test]
    fn unchanged_nights_keep_the_agreed_price_and_new_nights_use_the_current_table() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        reprice_all(&w, w.category, 20_000);

        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-29"))).unwrap();
        w.h.reservations
            .apply_change(&w.admin, id, change(None, None, Some("2026-10-29")), 1, "Mais uma noite".into(), &preview.fingerprint)
            .unwrap();

        let detail = w.h.reservations.get(&w.admin, id).unwrap();
        let amounts: Vec<_> = detail.nights.iter().map(|n| n.amount_cents).collect();
        assert_eq!(amounts, [10_000, 10_000, 10_000, 20_000]);
        assert_eq!(detail.reservation.agreed_total_cents, 50_000);
        assert_eq!(detail.reservation.deposit_expected_cents, 15_000);
        assert_eq!(detail.reservation.version, 2);
    }

    #[test]
    fn shortening_the_stay_drops_the_removed_nights() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");

        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, Some("2026-10-26"), None)).unwrap();
        w.h.reservations
            .apply_change(&w.admin, id, change(None, Some("2026-10-26"), None), 1, "Chega um dia depois".into(), &preview.fingerprint)
            .unwrap();

        let detail = w.h.reservations.get(&w.admin, id).unwrap();
        assert_eq!(detail.nights.len(), 2);
        assert_eq!(detail.reservation.agreed_total_cents, 20_000);
        assert_eq!(detail.reservation.check_in_date, "2026-10-26");
    }

    #[test]
    fn changing_category_reprices_every_night() {
        let w = world();
        let luxo = w.h.catalog.create_category(&w.admin, NewCategory { name: "Luxo".into(), description: String::new() }).unwrap();
        for day in 0..7 {
            w.h.rates.create_rule(&w.admin, weekday_rule(luxo, day, 25_000)).unwrap();
        }
        let luxury_room = new_room(&w.h, &w.admin, luxo, "201", 3);
        let id = reservation(&w, "2026-10-25", "2026-10-28");

        let preview = w.h.reservations.preview_change(&w.admin, id, change(Some(luxury_room), None, None)).unwrap();
        w.h.reservations
            .apply_change(&w.admin, id, change(Some(luxury_room), None, None), 1, "Upgrade para o Luxo".into(), &preview.fingerprint)
            .unwrap();

        let detail = w.h.reservations.get(&w.admin, id).unwrap();
        assert!(preview.nights.iter().all(|n| !n.kept));
        assert_eq!(detail.reservation.agreed_total_cents, 75_000);
        assert_eq!(detail.reservation.room_code, "201");
    }

    #[test]
    fn same_category_room_swap_keeps_the_prices() {
        let w = world();
        let other = new_room(&w.h, &w.admin, w.category, "102", 2);
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        reprice_all(&w, w.category, 50_000);

        let preview = w.h.reservations.preview_change(&w.admin, id, change(Some(other), None, None)).unwrap();

        assert!(preview.nights.iter().all(|n| n.kept));
        assert_eq!(preview.difference_cents, 0);
    }

    #[test]
    fn apply_needs_the_permission_and_a_justification() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        let rec = recepcao_token(&w.h, &w.admin);
        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-29"))).unwrap();

        assert!(matches!(w.h.reservations.preview_change(&rec, id, change(None, None, Some("2026-10-29"))), Err(AppError::Forbidden)));
        assert!(matches!(
            w.h.reservations.apply_change(&rec, id, change(None, None, Some("2026-10-29")), 1, "x".into(), &preview.fingerprint),
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            w.h.reservations.apply_change(&w.admin, id, change(None, None, Some("2026-10-29")), 1, "   ".into(), &preview.fingerprint),
            Err(AppError::Validation(_))
        ));
        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.version, 1);
    }

    #[test]
    fn recepcao_with_the_extra_permission_can_change() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        let _ = recepcao_token(&w.h, &w.admin);
        let rec_id = w.h.auth.login("rec", "Definitiva-456").unwrap().user.id;
        w.h.users.set_extra_permissions(&w.admin, rec_id, &[Permission::ChangeReservation]).unwrap();
        let rec = w.h.auth.login("rec", "Definitiva-456").unwrap().token;

        let preview = w.h.reservations.preview_change(&rec, id, change(None, None, Some("2026-10-29"))).unwrap();

        assert!(w
            .h
            .reservations
            .apply_change(&rec, id, change(None, None, Some("2026-10-29")), 1, "Pedido do hóspede".into(), &preview.fingerprint)
            .is_ok());
    }

    #[test]
    fn a_stale_preview_is_refused() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-29"))).unwrap();
        reprice_all(&w, w.category, 30_000); // a tabela mudou depois da prévia

        let result = w.h.reservations.apply_change(
            &w.admin,
            id,
            change(None, None, Some("2026-10-29")),
            1,
            "Mais uma noite".into(),
            &preview.fingerprint,
        );

        assert!(matches!(result, Err(AppError::StalePreview)));
        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.version, 1);
    }

    #[test]
    fn a_stale_reservation_version_conflicts() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-29"))).unwrap();

        let result = w.h.reservations.apply_change(&w.admin, id, change(None, None, Some("2026-10-29")), 7, "x".into(), &preview.fingerprint);

        assert!(matches!(result, Err(AppError::Conflict)));
    }

    #[test]
    fn a_clash_with_another_reservation_makes_the_change_unfeasible() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        reservation(&w, "2026-10-29", "2026-10-31");

        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-30"))).unwrap();

        assert!(!preview.feasible);
        assert_eq!(preview.conflicts[0].code, "RES-000002");
        assert!(matches!(
            w.h.reservations.apply_change(&w.admin, id, change(None, None, Some("2026-10-30")), 1, "Mais noites".into(), &preview.fingerprint),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn missing_tariff_for_new_nights_is_reported() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        for rule in w.h.rates.list_rules(&w.admin, Some(w.category)).unwrap() {
            if rule.weekday == Some(3) {
                w.h.rates.delete_rule(&w.admin, rule.id).unwrap(); // quarta-feira 28/10 fica sem tarifa
            }
        }

        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-29"))).unwrap();

        assert!(!preview.feasible);
        assert_eq!(preview.missing_dates, ["2026-10-28"]);
    }

    #[test]
    fn nothing_to_change_and_invalid_requests_are_not_feasible() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        let small = new_room(&w.h, &w.admin, w.category, "103", 1);

        let nothing = w.h.reservations.preview_change(&w.admin, id, change(None, None, None)).unwrap();
        let backwards = w.h.reservations.preview_change(&w.admin, id, change(None, Some("2026-10-30"), Some("2026-10-29")));
        let too_small = w.h.reservations.preview_change(&w.admin, id, change(Some(small), None, None)).unwrap();
        let past = w.h.reservations.preview_change(&w.admin, id, change(None, Some("2026-10-10"), None)).unwrap();

        assert!(!nothing.feasible && !nothing.problems.is_empty());
        assert!(matches!(backwards, Err(AppError::Validation(_))));
        assert!(!too_small.feasible);
        assert!(!past.feasible);
    }

    #[test]
    fn only_pending_or_confirmed_reservations_can_change() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        for status in ["checked_in", "completed", "cancelled", "no_show"] {
            set_status(&w, id, status);
            assert!(
                matches!(
                    w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-29"))),
                    Err(AppError::Validation(_))
                ),
                "{status}"
            );
        }
        set_status(&w, id, "confirmed");
        assert!(w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-29"))).is_ok());
    }

    #[test]
    fn history_and_audit_record_values_user_time_and_justification() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        let preview = w.h.reservations.preview_change(&w.admin, id, change(None, None, Some("2026-10-29"))).unwrap();
        w.h.reservations
            .apply_change(&w.admin, id, change(None, None, Some("2026-10-29")), 1, "Hóspede pediu mais uma noite".into(), &preview.fingerprint)
            .unwrap();

        let history = w.h.reservations.get(&w.admin, id).unwrap().history;

        assert_eq!(history.len(), 1);
        let entry = &history[0];
        assert_eq!(entry.changed_by_name, "Administrador");
        assert_eq!(entry.changed_at, "2026-10-21T12:00:00Z");
        assert_eq!((entry.price_before_cents, entry.price_after_cents), (Some(30_000), Some(40_000)));
        assert_eq!(entry.justification.as_deref(), Some("Hóspede pediu mais uma noite"));
        assert!(entry.before_json.contains("2026-10-28") && entry.after_json.contains("2026-10-29"));
        let audited: String = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT justification FROM audit_log WHERE operation = 'change_reservation'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(audited, "Hóspede pediu mais uma noite");
    }

    // ---- cancelamento, no-show e revisão ----

    fn pay_deposit(w: &World, id: i64, cents: i64) {
        w.h.db
            .lock()
            .unwrap()
            .execute("UPDATE reservations SET deposit_paid_cents = ?1 WHERE id = ?2", rusqlite::params![cents, id])
            .unwrap();
    }

    fn version(w: &World, id: i64) -> i64 {
        w.h.reservations.get(&w.admin, id).unwrap().reservation.version
    }

    #[test]
    fn cancellation_preview_uses_the_policy_tiers_and_never_writes() {
        let w = world();
        let far = reservation(&w, "2026-11-10", "2026-11-12"); // 20 dias
        let mid = reservation(&w, "2026-10-25", "2026-10-27"); // 4 dias
        let near = reservation(&w, "2026-10-22", "2026-10-24"); // 1 dia
        for id in [far, mid, near] {
            pay_deposit(&w, id, 9_000);
        }
        let before = counts(&w);

        let p_far = w.h.reservations.preview_cancellation(&w.admin, far).unwrap();
        let p_mid = w.h.reservations.preview_cancellation(&w.admin, mid).unwrap();
        let p_near = w.h.reservations.preview_cancellation(&w.admin, near).unwrap();

        assert_eq!((p_far.days_before, p_far.refund_cents, p_far.retained_cents, p_far.tier_min_days), (20, 9_000, 0, Some(7)));
        assert_eq!((p_mid.days_before, p_mid.refund_cents, p_mid.retained_cents, p_mid.tier_min_days), (4, 4_500, 4_500, Some(3)));
        assert_eq!((p_near.days_before, p_near.refund_cents, p_near.retained_cents, p_near.tier_min_days), (1, 0, 9_000, Some(0)));
        assert!(p_far.can_cancel);
        assert_eq!(counts(&w), before);
    }

    #[test]
    fn cancel_defaults_to_the_calculated_values_and_keeps_the_record() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-27");
        pay_deposit(&w, id, 9_000);

        let result = w.h.reservations.cancel(&w.admin, id, 1, None).unwrap();

        assert_eq!((result.refund_cents, result.retained_cents), (4_500, 4_500));
        let detail = w.h.reservations.get(&w.admin, id).unwrap();
        assert_eq!(detail.reservation.status, "cancelled");
        assert_eq!(detail.reservation.cancellation_fee_cents, Some(4_500));
        assert_eq!(detail.reservation.cancelled_at.as_deref(), Some("2026-10-21T12:00:00Z"));
        assert_eq!(detail.nights.len(), 2);
        assert_eq!(detail.history.last().unwrap().kind, "cancellation");
        // o quarto fica livre depois de gravado
        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-25", "2026-10-27")).is_ok());
    }

    #[test]
    fn an_exception_needs_the_permission_a_justification_and_valid_bounds() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-27");
        pay_deposit(&w, id, 9_000);
        let rec = recepcao_token(&w.h, &w.admin);
        let exception = |retained: i64, why: &str| Some(CancellationOverride { retained_cents: retained, justification: why.into() });

        assert!(matches!(w.h.reservations.cancel(&rec, id, 1, exception(0, "Motivo de saúde")), Err(AppError::Forbidden)));
        assert!(matches!(w.h.reservations.cancel(&w.admin, id, 1, exception(0, "  ")), Err(AppError::Validation(_))));
        assert!(matches!(w.h.reservations.cancel(&w.admin, id, 1, exception(9_001, "x")), Err(AppError::Validation(_))));
        assert!(matches!(w.h.reservations.cancel(&w.admin, id, 1, exception(-1, "x")), Err(AppError::Validation(_))));
        assert_eq!(version(&w, id), 1);

        let result = w.h.reservations.cancel(&w.admin, id, 1, exception(0, "Motivo de saúde")).unwrap();

        assert_eq!((result.refund_cents, result.retained_cents), (9_000, 0));
        let (after, why): (String, String) = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT after_json, justification FROM audit_log WHERE operation = 'cancel_reservation'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(after.contains("\"calculated_retained_cents\":4500") && after.contains("\"overridden\":true"));
        assert_eq!(why, "Motivo de saúde");
    }

    #[test]
    fn only_pending_or_confirmed_reservations_can_be_cancelled() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-27");
        for status in ["checked_in", "completed", "cancelled", "no_show"] {
            set_status(&w, id, status);
            assert!(!w.h.reservations.preview_cancellation(&w.admin, id).unwrap().can_cancel, "{status}");
            assert!(matches!(w.h.reservations.cancel(&w.admin, id, 1, None), Err(AppError::Validation(_))), "{status}");
        }
    }

    #[test]
    fn cancel_with_a_stale_version_conflicts() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-27");

        assert!(matches!(w.h.reservations.cancel(&w.admin, id, 9, None), Err(AppError::Conflict)));
        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.status, "pending");
    }

    #[test]
    fn no_show_is_refused_before_the_tolerance_and_allowed_after() {
        let w = world();
        let today = reservation(&w, "2026-10-21", "2026-10-22"); // chegada hoje, 14:00 + 60 min
        let future = reservation(&w, "2026-10-30", "2026-10-31");
        pay_deposit(&w, today, 9_000);

        assert!(matches!(w.h.reservations.mark_no_show(&w.admin, today, 1), Err(AppError::Validation(_)))); // 09:00 local
        w.h.utc.advance(chrono::Duration::hours(5)); // 14:00Z = 11:00 local ... ainda cedo
        assert!(matches!(w.h.reservations.mark_no_show(&w.admin, today, 1), Err(AppError::Validation(_))));
        w.h.utc.advance(chrono::Duration::hours(5)); // 19:00Z = 16:00 local
        assert!(matches!(w.h.reservations.mark_no_show(&w.admin, future, 1), Err(AppError::Validation(_))));

        w.h.reservations.mark_no_show(&w.admin, today, 1).unwrap();

        let detail = w.h.reservations.get(&w.admin, today).unwrap();
        assert_eq!(detail.reservation.status, "no_show");
        assert_eq!(detail.reservation.cancellation_fee_cents, Some(9_000));
        assert_eq!(detail.history.last().unwrap().kind, "no_show");
        // o no-show libera o quarto
        assert!(w.h.reservations.create(&w.admin, booking(&w, "2026-10-21", "2026-10-22")).is_ok());
    }

    #[test]
    fn the_review_list_flags_candidates_and_overdue_deposits_without_changing_anything() {
        let w = world();
        let arriving = reservation(&w, "2026-10-22", "2026-10-24"); // vence o sinal em 22/10
        let calm = reservation(&w, "2026-11-20", "2026-11-22");
        pay_deposit(&w, calm, 100_000);
        w.h.utc.advance(chrono::Duration::days(1) + chrono::Duration::hours(10)); // 22/10, 16:00 local
        let before = counts(&w);

        let items = w.h.reservations.list_needing_review(&w.admin).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].reservation.id, arriving);
        assert_eq!(items[0].reasons, ["no_show_candidate", "deposit_overdue"]);
        assert_eq!(w.h.reservations.get(&w.admin, arriving).unwrap().reservation.status, "pending");
        assert_eq!(counts(&w), before);
    }

    // ---- desconto, confirmação, sinal e observações ----

    fn amount_discount(cents: i64, why: &str) -> DiscountInput {
        DiscountInput { kind: "amount".into(), amount_cents: Some(cents), percent_bp: None, justification: why.into() }
    }

    fn percent_discount(bp: u32, why: &str) -> DiscountInput {
        DiscountInput { kind: "percent".into(), amount_cents: None, percent_bp: Some(bp), justification: why.into() }
    }

    #[test]
    fn a_discount_needs_the_permission_and_a_justification() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        let rec = recepcao_token(&w.h, &w.admin);

        assert!(matches!(w.h.reservations.apply_discount(&rec, id, amount_discount(1_000, "x"), 1), Err(AppError::Forbidden)));
        assert!(matches!(w.h.reservations.apply_discount(&w.admin, id, amount_discount(1_000, "  "), 1), Err(AppError::Validation(_))));
        assert_eq!(version(&w, id), 1);
    }

    #[test]
    fn a_percent_discount_rounds_half_up_once_on_the_total() {
        let w = world();
        reprice_all(&w, w.category, 10_035);
        let id = reservation(&w, "2026-10-25", "2026-10-26"); // 1 noite = R$ 100,35

        w.h.reservations.apply_discount(&w.admin, id, percent_discount(5_000, "Cliente antigo"), 1).unwrap();

        let r = w.h.reservations.get(&w.admin, id).unwrap().reservation;
        assert_eq!(r.discount_cents, 5_018); // 50,175 -> 50,18
        assert_eq!(r.agreed_total_cents, 5_017);
        assert_eq!(r.deposit_expected_cents, 1_505); // 30% de 50,17 = 15,051
    }

    #[test]
    fn a_discount_is_bounded_by_the_total_and_must_be_positive() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28"); // R$ 300,00

        for bad in [amount_discount(30_001, "x"), amount_discount(0, "x"), amount_discount(-5, "x"), percent_discount(10_001, "x"), percent_discount(0, "x")] {
            assert!(matches!(w.h.reservations.apply_discount(&w.admin, id, bad, 1), Err(AppError::Validation(_))));
        }
        let malformed = DiscountInput { kind: "amount".into(), amount_cents: None, percent_bp: Some(100), justification: "x".into() };
        assert!(matches!(w.h.reservations.apply_discount(&w.admin, id, malformed, 1), Err(AppError::Validation(_))));
        assert!(w.h.reservations.apply_discount(&w.admin, id, amount_discount(30_000, "Cortesia"), 1).is_ok());
        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.agreed_total_cents, 0);
    }

    #[test]
    fn a_discount_records_original_and_final_values_and_replaces_the_previous_one() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");
        w.h.reservations.apply_discount(&w.admin, id, amount_discount(3_000, "Primeiro desconto"), 1).unwrap();
        w.h.reservations.apply_discount(&w.admin, id, percent_discount(1_000, "Desconto de 10%"), 2).unwrap();

        let detail = w.h.reservations.get(&w.admin, id).unwrap();

        assert_eq!(detail.reservation.list_total_cents, 30_000);
        assert_eq!((detail.reservation.discount_cents, detail.reservation.agreed_total_cents), (3_000, 27_000));
        assert_eq!(detail.reservation.discount_reason.as_deref(), Some("Desconto de 10%"));
        let last = detail.history.last().unwrap();
        assert_eq!((last.kind.as_str(), last.price_before_cents, last.price_after_cents), ("discount", Some(27_000), Some(27_000)));
        let (before, after, why): (String, String, String) = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT before_json, after_json, justification FROM audit_log WHERE operation = 'apply_discount' ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert!(before.contains("\"list_total_cents\":30000") && after.contains("\"agreed_total_cents\":27000"));
        assert_eq!(why, "Desconto de 10%");
    }

    #[test]
    fn a_discount_is_refused_for_closed_reservations_and_stale_versions() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");

        assert!(matches!(w.h.reservations.apply_discount(&w.admin, id, amount_discount(100, "x"), 9), Err(AppError::Conflict)));
        set_status(&w, id, "cancelled");
        assert!(matches!(w.h.reservations.apply_discount(&w.admin, id, amount_discount(100, "x"), 1), Err(AppError::Validation(_))));
    }

    #[test]
    fn manual_confirmation_moves_pending_to_confirmed() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");

        assert!(matches!(w.h.reservations.confirm(&w.admin, id, 9), Err(AppError::Conflict)));
        w.h.reservations.confirm(&w.admin, id, 1).unwrap();

        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.status, "confirmed");
        assert!(matches!(w.h.reservations.confirm(&w.admin, id, 2), Err(AppError::Validation(_))));
        for status in ["checked_in", "completed", "cancelled", "no_show"] {
            set_status(&w, id, status);
            assert!(matches!(w.h.reservations.confirm(&w.admin, id, 2), Err(AppError::Validation(_))), "{status}");
        }
    }

    #[test]
    fn notes_are_updated_with_a_version_check_and_a_limit() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28");

        w.h.reservations.set_notes(&w.admin, id, "  Chegada tarde, após as 22h  ", 1).unwrap();

        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.notes, "Chegada tarde, após as 22h");
        assert!(matches!(w.h.reservations.set_notes(&w.admin, id, "x", 1), Err(AppError::Conflict)));
        assert!(matches!(w.h.reservations.set_notes(&w.admin, id, &"a".repeat(1001), 2), Err(AppError::Validation(_))));
    }

    #[test]
    fn recording_the_deposit_confirms_the_reservation_when_the_expected_amount_arrives() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28"); // sinal esperado R$ 90,00

        w.h.reservations.record_deposit(&w.admin, id, 4_000, 1).unwrap();
        let partial = w.h.reservations.get(&w.admin, id).unwrap().reservation;
        assert_eq!((partial.status.as_str(), partial.deposit_paid_cents), ("pending", 4_000));

        w.h.reservations.record_deposit(&w.admin, id, 5_000, 2).unwrap();
        let full = w.h.reservations.get(&w.admin, id).unwrap().reservation;
        assert_eq!((full.status.as_str(), full.deposit_paid_cents), ("confirmed", 9_000));
        assert!(!full.deposit_overdue);
    }

    #[test]
    fn deposit_amounts_are_validated() {
        let w = world();
        let id = reservation(&w, "2026-10-25", "2026-10-28"); // total R$ 300,00

        assert!(matches!(w.h.reservations.record_deposit(&w.admin, id, 0, 1), Err(AppError::Validation(_))));
        assert!(matches!(w.h.reservations.record_deposit(&w.admin, id, -100, 1), Err(AppError::Validation(_))));
        assert!(matches!(w.h.reservations.record_deposit(&w.admin, id, 30_001, 1), Err(AppError::Validation(_))));
        assert!(matches!(w.h.reservations.record_deposit(&w.admin, id, 1_000, 9), Err(AppError::Conflict)));
        set_status(&w, id, "cancelled");
        assert!(matches!(w.h.reservations.record_deposit(&w.admin, id, 1_000, 1), Err(AppError::Validation(_))));
    }
}
