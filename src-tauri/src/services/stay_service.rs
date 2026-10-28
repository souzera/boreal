use chrono::{DateTime, Duration, NaiveDateTime, Timelike, Utc};
use rusqlite::{Connection, Transaction};
use serde_json::json;

use super::rate_service::quote_in;
use super::room_rules;
use super::reservation_service::{br, conflict_message, parse_date, parse_time, resolve_times};
use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::domain::availability::find_conflicts;
use crate::domain::stay::{transition, StayStatus};
use crate::domain::{dates, late_checkout, money};
use crate::domain::stay::can_check_in;
use crate::error::AppError;
use crate::models::page::{clamp_paging, Page};
use crate::models::reservation::{ConflictInfo, Reservation};
use crate::models::settings::AppSettings;
use crate::models::stay::{
    AdjustmentInput, ChargeInput, CheckInInput, CheckoutInput, CheckoutPreview, DepartureRiskView, LateFeeChoice, LateInfo,
    OccupantInput, PaymentInput, Stay, StayDetail, StayQuery, StaySummary, WalkInInput,
};
use crate::repositories::stay_repo::{self, NewCharge};
use crate::repositories::reservation_repo::NewRow;
use crate::repositories::{catalog_repo, cleaning_repo, guest_repo, item_repo, reservation_repo, sequence_repo, settings_repo};

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

pub(crate) struct CleanOccupant {
    name: String,
    guest_id: Option<i64>,
    document: String,
}

const MAX_OCCUPANTS: usize = 20;
/// Teto de um lançamento ou recebimento (R$ 1.000.000,00): barra erros de digitação e estouros.
const MAX_ENTRY_CENTS: i64 = 100_000_000;
const PAYMENT_METHODS: [&str; 5] = ["cash", "pix", "debit", "credit", "transfer"];

/// `R$ 1.234,56`, para mensagens.
fn money_br(cents: i64) -> String {
    let abs = cents.unsigned_abs();
    let reais = (abs / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in reais.chars().enumerate() {
        if i > 0 && (reais.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    format!("{}R$ {grouped},{:02}", if cents < 0 { "-" } else { "" }, abs % 100)
}

fn clean_text(value: &str, max: usize, what: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(invalid(format!("Informe {what}.")));
    }
    if value.chars().count() > max {
        return Err(invalid(format!("{what}: no máximo {max} caracteres.")));
    }
    Ok(value.to_string())
}

struct ResolvedCharge {
    kind: &'static str,
    description: String,
    amount_cents: i64,
    item_id: Option<i64>,
}

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
        if now >= check_out.and_time(out_time) {
            return Err(invalid(
                "O horário de saída previsto desta reserva já passou. Altere as datas da reserva antes de fazer o check-in.",
            ));
        }
        let offset = i64::from(self.ctx.utc.local_offset().local_minus_utc());
        room_rules::ensure_room_clean(&tx, room.id, &room.code, room_rules::cleaning_minutes(&tx, room.id, &settings_repo::load(&tx)?)?, now, offset)?;
        room_rules::ensure_no_blocks(&tx, room.id, booked_start.min(now).date(), Some(check_out), true)?;
        let early = now < booked_start;
        if early {
            require(&actor, Permission::EarlyCheckIn)?;
        }

        // O quarto precisa estar livre desde o início real da ocupação (mais cedo entre o previsto e agora).
        let settings = settings_repo::load(&tx)?;
        let start = booked_start.min(now);
        let existing = reservation_repo::blocking(&tx, room.id, start.date(), check_out, now)?;
        let bookings: Vec<_> = existing.iter().map(|(b, _)| b.clone()).collect();
        let clashes = find_conflicts(
            &bookings,
            start.date(),
            start.time(),
            check_out,
            out_time,
            room_rules::cleaning_minutes(&tx, room.id, &settings)?,
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

        let offset = i64::from(self.ctx.utc.local_offset().local_minus_utc());
        room_rules::ensure_room_clean(&tx, room.id, &room.code, room_rules::cleaning_minutes(&tx, room.id, &settings)?, now, offset)?;
        room_rules::ensure_no_blocks(&tx, room.id, today, Some(check_out), false)?;
        let existing = reservation_repo::blocking(&tx, room.id, today, check_out, now)?;
        let bookings: Vec<_> = existing.iter().map(|(b, _)| b.clone()).collect();
        let clashes = find_conflicts(&bookings, today, in_time, check_out, out_time, room_rules::cleaning_minutes(&tx, room.id, &settings)?, None);
        if !clashes.is_empty() {
            let infos: Vec<ConflictInfo> =
                existing.into_iter().filter(|(b, _)| clashes.contains(&b.id)).map(|(_, info)| info).collect();
            return Err(invalid(conflict_message(&infos)));
        }

        let code = sequence_repo::format_code("AVU", sequence_repo::next(&tx, "stay")?);
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

    /// Hospedagem que ainda aceita lançamentos.
    fn open_row(tx: &Transaction, stay_id: i64) -> Result<Stay, AppError> {
        let stay = stay_repo::find(tx, stay_id)?.ok_or_else(|| invalid("Hospedagem não encontrada."))?;
        if stay.status != "in_house" {
            return Err(invalid("A hospedagem já foi encerrada. Não é possível mudar a conta."));
        }
        Ok(stay)
    }

    fn resolve_charge(tx: &Transaction, input: &ChargeInput) -> Result<ResolvedCharge, AppError> {
        let kind = match input.kind.as_str() {
            "consumption" => "consumption",
            "extra" => "extra",
            _ => return Err(invalid("Escolha se é consumo ou despesa extra.")),
        };
        let item = match input.item_id {
            Some(id) => {
                let item = item_repo::find(tx, id)?.ok_or_else(|| invalid("Item não encontrado."))?;
                if !item.active {
                    return Err(invalid("O item escolhido está desativado."));
                }
                Some(item)
            }
            None => None,
        };
        let description = match (&item, input.description.trim().is_empty()) {
            (Some(item), true) => item.name.clone(),
            _ => clean_text(&input.description, 200, "a descrição")?,
        };
        let amount_cents = match (&item, input.amount_cents) {
            (Some(item), 0) => item.default_price_cents,
            (_, cents) => cents,
        };
        if !(1..=MAX_ENTRY_CENTS).contains(&amount_cents) {
            return Err(invalid("Informe um valor maior que zero e de até R$ 1.000.000,00."));
        }
        Ok(ResolvedCharge { kind, description, amount_cents, item_id: input.item_id })
    }

    fn insert_resolved(
        tx: &Transaction,
        actor: &SessionInfo,
        stay_id: i64,
        charge: &ResolvedCharge,
        replaces: Option<i64>,
        at: DateTime<Utc>,
    ) -> Result<i64, AppError> {
        stay_repo::insert_charge(
            tx,
            &NewCharge {
                stay_id,
                kind: charge.kind,
                description: &charge.description,
                amount_cents: charge.amount_cents,
                item_id: charge.item_id,
                night_date: None,
                reverses_charge_id: None,
                replaces_charge_id: replaces,
                justification: None,
                created_by: actor.user_id,
            },
            at,
        )
    }

    #[allow(clippy::too_many_arguments)] // cada chamada descreve um evento de auditoria completo
    fn audit_ledger(
        &self,
        tx: &Transaction,
        actor: &SessionInfo,
        operation: &'static str,
        stay_id: i64,
        before_balance: i64,
        detail: serde_json::Value,
        justification: Option<String>,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        audit::record(
            tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "stays",
                operation,
                entity: "stay",
                entity_id: Some(stay_id),
                before: Some(json!({ "balance_cents": before_balance })),
                after: Some(json!({ "balance_cents": stay_repo::balance(tx, stay_id)?, "entry": detail })),
                justification,
            },
        )
    }

    /// Lança consumo ou despesa extra, avulso ou do catálogo (spec §6.2, §10.1).
    pub fn add_charge(&self, token: &str, stay_id: i64, input: ChargeInput) -> Result<i64, AppError> {
        let actor = self.touch(token)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        Self::open_row(&tx, stay_id)?;
        let charge = Self::resolve_charge(&tx, &input)?;
        let before = stay_repo::balance(&tx, stay_id)?;
        let at = self.ctx.utc.now_utc();
        let id = Self::insert_resolved(&tx, &actor, stay_id, &charge, None, at)?;
        stay_repo::bump(&tx, stay_id, at)?;
        let detail = json!({ "charge_id": id, "kind": charge.kind, "description": charge.description, "amount_cents": charge.amount_cents });
        self.audit_ledger(&tx, &actor, "add_charge", stay_id, before, detail, None, at)?;
        tx.commit()?;
        Ok(id)
    }

    /// Ajuste manual da conta; exige `ApplyDiscount` e justificativa (spec §5).
    pub fn add_adjustment(&self, token: &str, stay_id: i64, input: AdjustmentInput) -> Result<i64, AppError> {
        let actor = self.touch(token)?;
        require(&actor, Permission::ApplyDiscount)?;
        let description = clean_text(&input.description, 200, "a descrição")?;
        let justification = clean_text(&input.justification, 500, "a justificativa")?;
        if input.amount_cents == 0 || input.amount_cents.abs() > MAX_ENTRY_CENTS {
            return Err(invalid("Informe um valor diferente de zero e de até R$ 1.000.000,00."));
        }
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        Self::open_row(&tx, stay_id)?;
        let before = stay_repo::balance(&tx, stay_id)?;
        let at = self.ctx.utc.now_utc();
        let id = stay_repo::insert_charge(
            &tx,
            &NewCharge {
                stay_id,
                kind: "adjustment",
                description: &description,
                amount_cents: input.amount_cents,
                item_id: None,
                night_date: None,
                reverses_charge_id: None,
                replaces_charge_id: None,
                justification: Some(&justification),
                created_by: actor.user_id,
            },
            at,
        )?;
        stay_repo::bump(&tx, stay_id, at)?;
        let detail = json!({ "charge_id": id, "description": description, "amount_cents": input.amount_cents });
        self.audit_ledger(&tx, &actor, "add_adjustment", stay_id, before, detail, Some(justification), at)?;
        tx.commit()?;
        Ok(id)
    }

    /// Estorna um lançamento (o original permanece) e, se houver, grava o novo no lugar, tudo na mesma transação.
    pub fn reverse_charge(
        &self,
        token: &str,
        stay_id: i64,
        charge_id: i64,
        justification: String,
        replacement: Option<ChargeInput>,
    ) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        require(&actor, Permission::ReverseEntry)?;
        let justification = clean_text(&justification, 500, "a justificativa do estorno")?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        Self::open_row(&tx, stay_id)?;
        let (kind, description, amount) =
            stay_repo::find_charge(&tx, stay_id, charge_id)?.ok_or_else(|| invalid("Lançamento não encontrado nesta hospedagem."))?;
        if kind == "reversal" {
            return Err(invalid("Um estorno não pode ser estornado. Lance novamente o valor correto."));
        }
        if stay_repo::is_reversed(&tx, charge_id)? {
            return Err(invalid("Este lançamento já foi estornado."));
        }
        let replacement = replacement.as_ref().map(|r| Self::resolve_charge(&tx, r)).transpose()?;
        let before = stay_repo::balance(&tx, stay_id)?;
        let at = self.ctx.utc.now_utc();
        let reversal_description = format!("Estorno de: {description}");
        let reversal_id = stay_repo::insert_charge(
            &tx,
            &NewCharge {
                stay_id,
                kind: "reversal",
                description: &reversal_description,
                amount_cents: -amount,
                item_id: None,
                night_date: None,
                reverses_charge_id: Some(charge_id),
                replaces_charge_id: None,
                justification: Some(&justification),
                created_by: actor.user_id,
            },
            at,
        )
        .map_err(|e| match e {
            AppError::Database(msg) if msg.contains("UNIQUE") => invalid("Este lançamento já foi estornado."),
            other => other,
        })?;
        let relaunched = match &replacement {
            Some(charge) => Some(Self::insert_resolved(&tx, &actor, stay_id, charge, Some(charge_id), at)?),
            None => None,
        };
        stay_repo::bump(&tx, stay_id, at)?;
        let detail = json!({
            "reversed_charge_id": charge_id, "reversal_id": reversal_id, "reversed_amount_cents": amount,
            "relaunched_charge_id": relaunched, "relaunched_amount_cents": replacement.as_ref().map(|r| r.amount_cents)
        });
        self.audit_ledger(&tx, &actor, "reverse_charge", stay_id, before, detail, Some(justification), at)?;
        tx.commit()?;
        Ok(())
    }

    /// Recebimento provisório com uma modalidade (o módulo financeiro do Marco 5 o substitui).
    pub fn record_payment(&self, token: &str, stay_id: i64, input: PaymentInput) -> Result<i64, AppError> {
        let actor = self.touch(token)?;
        if !PAYMENT_METHODS.contains(&input.method.as_str()) {
            return Err(invalid("Escolha a forma de pagamento."));
        }
        if !(1..=MAX_ENTRY_CENTS).contains(&input.amount_cents) {
            return Err(invalid("Informe um valor maior que zero e de até R$ 1.000.000,00."));
        }
        let note = input.note.trim();
        if note.chars().count() > 200 {
            return Err(invalid("Observação: no máximo 200 caracteres."));
        }
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let stay = Self::open_row(&tx, stay_id)?;
        let before = stay_repo::balance(&tx, stay_id)?;
        let at = self.ctx.utc.now_utc();
        let code = sequence_repo::format_code("PAG", sequence_repo::next(&tx, "payment")?);
        let id = stay_repo::insert_payment(&tx, &code, stay.reservation_id, stay_id, input.amount_cents, &input.method, actor.user_id, note, at)?;
        stay_repo::bump(&tx, stay_id, at)?;
        let detail = json!({ "payment_id": id, "code": code, "method": input.method, "amount_cents": input.amount_cents });
        self.audit_ledger(&tx, &actor, "record_payment", stay_id, before, detail, None, at)?;
        tx.commit()?;
        Ok(id)
    }

    /// Atraso da saída (com a tolerância em vigor) e risco para a próxima reserva do mesmo quarto.
    fn assess_departure(
        conn: &Connection,
        stay: &Stay,
        settings: &AppSettings,
        now: NaiveDateTime,
    ) -> Result<(LateInfo, Option<DepartureRiskView>), AppError> {
        let out_date = parse_date(&stay.check_out_date, "Data de saída")?;
        let out_time = parse_time(&stay.expected_check_out_time).ok_or_else(|| invalid("Horário de saída inválido."))?;
        let expected_out = out_date.and_time(out_time);
        let tolerance = stay.late_tolerance_minutes.and_then(|m| u32::try_from(m).ok()).unwrap_or(settings.late_checkout_tolerance_minutes);
        let assessment = late_checkout::assess(
            expected_out,
            now,
            tolerance,
            stay_repo::last_night_cents(conn, stay.id)?,
            settings.late_checkout_fee_bp,
        )?;
        let late = LateInfo {
            minutes_late: assessment.minutes_late,
            tolerance_minutes: tolerance,
            applies: assessment.applies,
            fee_cents: assessment.fee_cents,
        };

        let mut risk = None;
        if assessment.minutes_late > 0 {
            let nearby = reservation_repo::blocking(conn, stay.room_id, expected_out.date(), expected_out.date() + Duration::days(60), now)?;
            let next = nearby
                .into_iter()
                .filter(|(b, _)| b.id != stay.reservation_id)
                .map(|(b, info)| (b.check_in.and_time(b.expected_in), info))
                .filter(|(start, _)| *start >= expected_out)
                .min_by_key(|(start, _)| *start);
            if let Some((next_start, info)) = next {
                risk = late_checkout::departure_risk(expected_out, now, room_rules::cleaning_minutes(conn, stay.room_id, settings)?, next_start).map(|r| {
                    DepartureRiskView {
                        next_reservation_id: info.reservation_id,
                        next_code: info.code,
                        next_start: next_start.format("%Y-%m-%dT%H:%M").to_string(),
                        ready_at: r.ready_at.format("%Y-%m-%dT%H:%M").to_string(),
                        overrun_minutes: r.overrun_minutes,
                    }
                });
            }
        }
        Ok((late, risk))
    }

    /// Demonstrativo do check-out: tudo o que será gravado, sem gravar nada.
    pub fn preview_checkout(&self, token: &str, stay_id: i64) -> Result<CheckoutPreview, AppError> {
        self.touch(token)?;
        let conn = self.ctx.conn()?;
        let stay = stay_repo::find(&conn, stay_id)?.ok_or_else(|| invalid("Hospedagem não encontrada."))?;
        if stay.status != "in_house" {
            return Err(invalid("A hospedagem já foi encerrada."));
        }
        let settings = settings_repo::load(&conn)?;
        let (late, risk) = Self::assess_departure(&conn, &stay, &settings, self.ctx.local_now())?;
        let account = stay_repo::account(&conn, stay_id)?;
        let balance_with_late_fee_cents = if late.applies { money::add(account.balance_cents, late.fee_cents)? } else { account.balance_cents };
        Ok(CheckoutPreview { balance_cents: account.balance_cents, balance_with_late_fee_cents, stay, account, late, risk })
    }

    /// Conclui o check-out (spec §6.3, §6.4). Nada é liquidado nem apagado: saldo em aberto fica registrado.
    pub fn checkout(&self, token: &str, input: CheckoutInput) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let stay = stay_repo::find(&tx, input.stay_id)?.ok_or_else(|| invalid("Hospedagem não encontrada."))?;
        if stay.version != input.expected_version {
            return Err(AppError::Conflict);
        }
        let from = StayStatus::parse(&stay.status).ok_or_else(|| AppError::Database("situação de hospedagem desconhecida".into()))?;
        transition(from, StayStatus::CheckedOut)?;

        let settings = settings_repo::load(&tx)?;
        let at = self.ctx.utc.now_utc();
        let (late, _) = Self::assess_departure(&tx, &stay, &settings, self.ctx.local_now())?;
        let before_balance = stay_repo::balance(&tx, stay.id)?;

        let (decision, waive_justification) = match (&input.late_fee, late.applies) {
            (LateFeeChoice::NotApplicable, false) => ("not_applicable", None),
            (LateFeeChoice::NotApplicable, true) => {
                return Err(invalid(format!(
                    "A saída passou da tolerância ({} min). Escolha cobrar a saída tardia ({}) ou dispensar a cobrança.",
                    late.minutes_late,
                    money_br(late.fee_cents)
                )));
            }
            (_, false) => return Err(invalid("Não há cobrança de saída tardia a decidir.")),
            (LateFeeChoice::Charge, true) => {
                let description = format!("Saída tardia ({} min após o horário previsto)", late.minutes_late);
                stay_repo::insert_charge(
                    &tx,
                    &NewCharge {
                        stay_id: stay.id,
                        kind: "late_fee",
                        description: &description,
                        amount_cents: late.fee_cents,
                        item_id: None,
                        night_date: None,
                        reverses_charge_id: None,
                        replaces_charge_id: None,
                        justification: None,
                        created_by: actor.user_id,
                    },
                    at,
                )?;
                ("charged", None)
            }
            (LateFeeChoice::Waive { justification }, true) => {
                require(&actor, Permission::WaiveLateFee)?;
                ("waived", Some(clean_text(justification, 500, "a justificativa para dispensar a cobrança")?))
            }
        };

        let balance = stay_repo::balance(&tx, stay.id)?;
        let mut balance_exception = None;
        if balance != 0 {
            let Some(exception) = &input.balance_exception else {
                return Err(invalid(if balance > 0 {
                    format!("A conta tem saldo em aberto de {}. Registre o pagamento ou conclua com saldo pendente (exige autorização).", money_br(balance))
                } else {
                    format!("O hóspede tem {} a devolver. Conclua com saldo pendente (exige autorização) ou acerte o valor antes.", money_br(-balance))
                }));
            };
            require(&actor, Permission::CheckoutWithBalance)?;
            balance_exception = Some(clean_text(&exception.justification, 500, "a justificativa do saldo em aberto")?);
        }

        let reservation = reservation_repo::find(&tx, stay.reservation_id, self.ctx.today())?
            .ok_or_else(|| invalid("Reserva da hospedagem não encontrada."))?;
        stay_repo::finish(
            &tx,
            stay.id,
            &stay_repo::Checkout {
                by: actor.user_id,
                late_minutes: late.minutes_late,
                late_fee_calculated_cents: late.fee_cents,
                late_fee_decision: decision,
                waive_justification: waive_justification.as_deref(),
                waived_by: waive_justification.as_ref().map(|_| actor.user_id),
                balance_cents: balance_exception.as_ref().map(|_| balance),
                balance_justification: balance_exception.as_deref(),
                balance_authorized_by: balance_exception.as_ref().map(|_| actor.user_id),
            },
            input.expected_version,
            at,
        )?;
        reservation_repo::set_status(&tx, reservation.id, "completed", reservation.version, at)?;
        // O quarto fica aguardando limpeza desde a hora efetiva da saída (spec §7).
        cleaning_repo::insert_awaiting(&tx, stay.room_id, stay.id, at, at)?;

        let before = json!({ "status": "in_house", "balance_cents": before_balance });
        let after = json!({
            "status": "checked_out", "balance_cents": balance, "late_minutes": late.minutes_late,
            "late_fee_calculated_cents": late.fee_cents, "late_fee_decision": decision,
            "balance_exception": balance_exception.is_some()
        });
        reservation_repo::insert_change(&tx, reservation.id, at, actor.user_id, "check_out", &before.to_string(), &after.to_string(), None, None, None)?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "stays",
                operation: "check_out",
                entity: "stay",
                entity_id: Some(stay.id),
                before: Some(before),
                after: Some(after),
                justification: balance_exception.or(waive_justification),
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Tolerância de saída tardia só desta hospedagem (`None` volta ao padrão das configurações).
    pub fn set_late_tolerance(&self, token: &str, stay_id: i64, minutes: Option<u32>, expected_version: i64) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        require(&actor, Permission::ChangeReservation)?;
        if minutes.is_some_and(|m| m > 600) {
            return Err(invalid("Tolerância de saída tardia: informe de 0 a 600 minutos."));
        }
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let stay = Self::open_row(&tx, stay_id)?;
        if stay.version != expected_version {
            return Err(AppError::Conflict);
        }
        let at = self.ctx.utc.now_utc();
        stay_repo::set_tolerance(&tx, stay_id, minutes.map(i64::from), expected_version, at)?;
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "stays",
                operation: "set_late_tolerance",
                entity: "stay",
                entity_id: Some(stay_id),
                before: Some(json!({ "late_tolerance_minutes": stay.late_tolerance_minutes })),
                after: Some(json!({ "late_tolerance_minutes": minutes })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(())
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
        let settings = settings_repo::load(&conn)?;
        let now = self.ctx.local_now();
        let items = stays
            .into_iter()
            .map(|stay| {
                let balance_cents = stay_repo::balance(&conn, stay.id)?;
                if stay.status != "in_house" {
                    return Ok(StaySummary { stay, balance_cents, minutes_late: 0, risk: None });
                }
                let (late, risk) = Self::assess_departure(&conn, &stay, &settings, now)?;
                Ok(StaySummary { stay, balance_cents, minutes_late: late.minutes_late, risk })
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
    use crate::models::stay::{
        AdjustmentInput, BalanceException, ChargeInput, CheckInInput, CheckoutInput, LateFeeChoice, OccupantInput, PaymentInput,
        StayQuery, WalkInInput,
    };
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
        assert_eq!(detail.stay.code, "AVU-000001");
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

    // ---- conta da hospedagem ----

    /// Reserva 21→23/10, desconto R$ 40,00, sinal R$ 120,00 e check-in às 14:00: saldo inicial R$ 240,00.
    fn stay_with_balance(w: &World) -> (i64, i64) {
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        let v = version(w, reservation);
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
        let v = version(w, reservation);
        w.h.reservations.record_deposit(&w.admin, reservation, 12_000, v).unwrap();
        set_now(w, "2026-10-21T17:00:00Z");
        let stay = w.h.stays.check_in(&w.admin, input(w, reservation, vec![holder(w)])).unwrap();
        (reservation, stay)
    }

    fn consumption(description: &str, cents: i64) -> ChargeInput {
        ChargeInput { kind: "consumption".into(), description: description.into(), amount_cents: cents, item_id: None }
    }

    fn balance(w: &World, stay: i64) -> i64 {
        w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents
    }

    fn last_charge_id(w: &World) -> i64 {
        w.h.db.lock().unwrap().query_row("SELECT max(id) FROM charges", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn add_charge_increases_balance_and_item_fills_defaults() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        w.h.stays.add_charge(&w.admin, stay, consumption("Água", 3_500)).unwrap();
        assert_eq!(balance(&w, stay), 27_500);

        let item = w.h.items.create(&w.admin, crate::models::item::NewItem { name: "Água mineral".into(), default_price_cents: 600 }).unwrap();
        w.h.stays
            .add_charge(&w.admin, stay, ChargeInput { kind: "extra".into(), description: String::new(), amount_cents: 0, item_id: Some(item) })
            .unwrap();

        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        let last = account.charges.last().unwrap();
        assert_eq!((last.kind.as_str(), last.description.as_str(), last.amount_cents), ("extra", "Água mineral", 600));
        assert_eq!(account.balance_cents, 28_100);
    }

    #[test]
    fn reverse_and_relaunch_preserves_original() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        w.h.stays.add_charge(&w.admin, stay, consumption("Água", 3_500)).unwrap();
        let original = last_charge_id(&w);

        w.h.stays
            .reverse_charge(&w.admin, stay, original, "Valor errado".into(), Some(consumption("Água", 3_000)))
            .unwrap();

        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        let tail: Vec<_> = account.charges.iter().rev().take(3).rev().map(|c| (c.kind.as_str(), c.amount_cents, c.reversed)).collect();
        assert_eq!(tail, [("consumption", 3_500, true), ("reversal", -3_500, false), ("consumption", 3_000, false)]);
        assert_eq!(account.balance_cents, 27_000);
    }

    #[test]
    fn reverse_twice_or_reverse_a_reversal_is_refused() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        w.h.stays.add_charge(&w.admin, stay, consumption("Água", 3_500)).unwrap();
        let original = last_charge_id(&w);
        w.h.stays.reverse_charge(&w.admin, stay, original, "Engano".into(), None).unwrap();
        let reversal = last_charge_id(&w);
        let (charges, balance_after) = (count(&w, "charges"), balance(&w, stay));

        let again = w.h.stays.reverse_charge(&w.admin, stay, original, "Engano".into(), None);
        let of_reversal = w.h.stays.reverse_charge(&w.admin, stay, reversal, "Engano".into(), None);

        assert!(matches!(again, Err(AppError::Validation(_))), "{again:?}");
        assert!(matches!(of_reversal, Err(AppError::Validation(_))), "{of_reversal:?}");
        assert_eq!((count(&w, "charges"), balance(&w, stay)), (charges, balance_after));
    }

    #[test]
    fn reverse_requires_reverse_entry_and_justification() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        w.h.stays.add_charge(&w.admin, stay, consumption("Água", 3_500)).unwrap();
        let original = last_charge_id(&w);
        let rec = login_with(&w, &[]);

        assert!(matches!(w.h.stays.reverse_charge(&rec, stay, original, "Engano".into(), None), Err(AppError::Forbidden)));
        assert!(matches!(w.h.stays.reverse_charge(&w.admin, stay, original, "   ".into(), None), Err(AppError::Validation(_))));
        let rec = login_with(&w, &[Permission::ReverseEntry]);
        assert!(w.h.stays.reverse_charge(&rec, stay, original, "Engano".into(), None).is_ok());
    }

    #[test]
    fn adjustment_requires_apply_discount() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        let rec = login_with(&w, &[]);
        let adjust = |cents: i64| AdjustmentInput { description: "Cortesia".into(), amount_cents: cents, justification: "Atraso no quarto".into() };

        assert!(matches!(w.h.stays.add_adjustment(&rec, stay, adjust(-1_000)), Err(AppError::Forbidden)));
        assert!(matches!(w.h.stays.add_adjustment(&w.admin, stay, adjust(0)), Err(AppError::Validation(_))));
        w.h.stays.add_adjustment(&w.admin, stay, adjust(-1_000)).unwrap();

        assert_eq!(balance(&w, stay), 23_000);
    }

    #[test]
    fn invalid_amounts_and_descriptions_write_nothing() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        let item = w.h.items.create(&w.admin, crate::models::item::NewItem { name: "Lavanderia".into(), default_price_cents: 2_500 }).unwrap();
        let version = w.h.items.list(&w.admin, false).unwrap()[0].version;
        w.h.items.set_active(&w.admin, item, false, version).unwrap();
        let (charges, audit) = (count(&w, "charges"), count(&w, "audit_log"));

        let bad = [
            consumption("Água", 0),
            consumption("Água", -1),
            consumption("Água", i64::MAX),
            consumption("   ", 500),
            ChargeInput { kind: "night".into(), description: "x".into(), amount_cents: 500, item_id: None },
            ChargeInput { kind: "extra".into(), description: "x".into(), amount_cents: 500, item_id: Some(9_999) },
            ChargeInput { kind: "extra".into(), description: String::new(), amount_cents: 0, item_id: Some(item) },
        ];
        for charge in bad {
            let result = w.h.stays.add_charge(&w.admin, stay, charge.clone());
            assert!(matches!(result, Err(AppError::Validation(_))), "{charge:?}: {result:?}");
        }

        assert_eq!((count(&w, "charges"), count(&w, "audit_log")), (charges, audit));
    }

    #[test]
    fn record_payment_reduces_balance_and_validates() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        let pay = |cents: i64, method: &str| PaymentInput { amount_cents: cents, method: method.into(), note: String::new() };

        w.h.stays.record_payment(&w.admin, stay, pay(10_000, "pix")).unwrap();

        assert_eq!(balance(&w, stay), 14_000);
        for bad in [pay(0, "pix"), pay(-5, "pix"), pay(100, "prior_deposit"), pay(100, "cheque"), pay(i64::MAX, "cash")] {
            assert!(matches!(w.h.stays.record_payment(&w.admin, stay, bad), Err(AppError::Validation(_))));
        }
        assert_eq!(balance(&w, stay), 14_000);
        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        assert_eq!(account.payments.last().unwrap().code, "PAG-000002");
    }

    #[test]
    fn closed_stay_rejects_new_entries() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        w.h.stays.add_charge(&w.admin, stay, consumption("Água", 3_500)).unwrap();
        let original = last_charge_id(&w);
        w.h.db.lock().unwrap().execute("UPDATE stays SET status = 'checked_out'", []).unwrap();

        assert!(w.h.stays.add_charge(&w.admin, stay, consumption("Café", 500)).is_err());
        assert!(w.h.stays.reverse_charge(&w.admin, stay, original, "Engano".into(), None).is_err());
        assert!(w.h.stays.record_payment(&w.admin, stay, PaymentInput { amount_cents: 100, method: "cash".into(), note: String::new() }).is_err());
        assert!(w.h.stays.get(&w.admin, stay).is_ok());
    }

    #[test]
    fn balance_equals_sum_of_ledger_after_mixed_operations() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        w.h.stays.add_charge(&w.admin, stay, consumption("Água", 3_500)).unwrap();
        let first = last_charge_id(&w);
        w.h.stays.add_charge(&w.admin, stay, consumption("Jantar", 8_000)).unwrap();
        w.h.stays.record_payment(&w.admin, stay, PaymentInput { amount_cents: 5_000, method: "cash".into(), note: String::new() }).unwrap();
        w.h.stays.reverse_charge(&w.admin, stay, first, "Engano".into(), Some(consumption("Água", 3_000))).unwrap();
        w.h.stays
            .add_adjustment(&w.admin, stay, AdjustmentInput { description: "Cortesia".into(), amount_cents: -500, justification: "Cortesia da casa".into() })
            .unwrap();
        w.h.stays.record_payment(&w.admin, stay, PaymentInput { amount_cents: 7_000, method: "debit".into(), note: "Maquininha".into() }).unwrap();
        w.h.stays.add_charge(&w.admin, stay, consumption("Lavanderia", 2_500)).unwrap();
        let last = last_charge_id(&w);
        w.h.stays.reverse_charge(&w.admin, stay, last, "Não foi feita".into(), None).unwrap();

        let from_sql: i64 = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT (SELECT SUM(amount_cents) FROM charges) - (SELECT SUM(amount_cents) FROM payments) + (SELECT COALESCE(SUM(amount_cents), 0) FROM refunds)",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(balance(&w, stay), from_sql);
        assert_eq!(from_sql, 24_000 + 3_000 + 8_000 - 500 - 5_000 - 7_000);
    }

    #[test]
    fn every_ledger_operation_is_audited() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        w.h.stays.add_charge(&w.admin, stay, consumption("Água", 3_500)).unwrap();
        let original = last_charge_id(&w);
        w.h.stays.record_payment(&w.admin, stay, PaymentInput { amount_cents: 1_000, method: "pix".into(), note: String::new() }).unwrap();
        w.h.stays
            .add_adjustment(&w.admin, stay, AdjustmentInput { description: "Cortesia".into(), amount_cents: -100, justification: "Cortesia".into() })
            .unwrap();
        w.h.stays.reverse_charge(&w.admin, stay, original, "Engano".into(), None).unwrap();

        let operations: Vec<String> = {
            let conn = w.h.db.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT operation FROM audit_log WHERE module = 'stays' AND operation <> 'check_in' ORDER BY id")
                .unwrap();
            stmt.query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect()
        };

        assert_eq!(operations, ["add_charge", "record_payment", "add_adjustment", "reverse_charge"]);
    }

    // ---- check-out, saída tardia e risco ----
    // Saída prevista: 23/10 12:00 locais (15:00Z), tolerância 30 min, taxa 50% da última diária (R$ 200,00).

    const OUT: &str = "2026-10-23T15:00:00Z";
    const OUT_PLUS_30: &str = "2026-10-23T15:30:00Z";
    const OUT_PLUS_45: &str = "2026-10-23T15:45:00Z";

    fn pay(w: &World, stay: i64, cents: i64) {
        w.h.stays
            .record_payment(&w.admin, stay, PaymentInput { amount_cents: cents, method: "cash".into(), note: String::new() })
            .unwrap();
    }

    fn stay_version(w: &World, stay: i64) -> i64 {
        w.h.stays.get(&w.admin, stay).unwrap().stay.version
    }

    fn checkout_input(w: &World, stay: i64, late_fee: LateFeeChoice, exception: Option<&str>) -> CheckoutInput {
        CheckoutInput {
            stay_id: stay,
            expected_version: stay_version(w, stay),
            late_fee,
            balance_exception: exception.map(|j| BalanceException { justification: j.into() }),
        }
    }

    fn stay_column(w: &World, stay: i64, column: &str) -> Option<String> {
        w.h.db
            .lock()
            .unwrap()
            .query_row(&format!("SELECT CAST({column} AS TEXT) FROM stays WHERE id = ?1"), [stay], |r| r.get(0))
            .unwrap()
    }

    fn count_kind(w: &World, kind: &str) -> i64 {
        w.h.db.lock().unwrap().query_row("SELECT count(*) FROM charges WHERE kind = ?1", [kind], |r| r.get(0)).unwrap()
    }

    #[test]
    fn checkout_with_zero_balance_completes_stay_and_reservation() {
        let w = world();
        let (reservation, stay) = stay_with_balance(&w);
        pay(&w, stay, 24_000);
        set_now(&w, OUT);

        w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::NotApplicable, None)).unwrap();

        let detail = w.h.stays.get(&w.admin, stay).unwrap();
        assert_eq!(detail.stay.status, "checked_out");
        assert!(detail.stay.checked_out_at.is_some());
        assert_eq!(w.h.reservations.get(&w.admin, reservation).unwrap().reservation.status, "completed");
        assert_eq!(detail.account.balance_cents, 0);
    }

    #[test]
    fn checkout_with_balance_needs_permission_justification_and_keeps_the_balance() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        set_now(&w, OUT);

        let none = w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::NotApplicable, None));
        match none {
            Err(AppError::Validation(msg)) => assert!(msg.contains("R$ 240,00"), "{msg}"),
            other => panic!("{other:?}"),
        }
        let rec = login_with(&w, &[]);
        let forbidden = w.h.stays.checkout(&rec, checkout_input(&w, stay, LateFeeChoice::NotApplicable, Some("Paga depois")));
        assert!(matches!(forbidden, Err(AppError::Forbidden)));
        let blank = w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::NotApplicable, Some("  ")));
        assert!(matches!(blank, Err(AppError::Validation(_))));
        let (charges, payments) = (count(&w, "charges"), count(&w, "payments"));

        let rec = login_with(&w, &[Permission::CheckoutWithBalance]);
        w.h.stays.checkout(&rec, checkout_input(&w, stay, LateFeeChoice::NotApplicable, Some("Paga depois, pelo Pix"))).unwrap();

        assert_eq!(stay_column(&w, stay, "balance_at_checkout_cents").as_deref(), Some("24000"));
        assert_eq!(stay_column(&w, stay, "balance_justification").as_deref(), Some("Paga depois, pelo Pix"));
        assert!(stay_column(&w, stay, "balance_authorized_by").is_some());
        assert_eq!((count(&w, "charges"), count(&w, "payments")), (charges, payments), "nada é liquidado nem apagado");
        assert_eq!(w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents, 24_000);
    }

    #[test]
    fn checkout_with_credit_requires_exception_and_says_so() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        pay(&w, stay, 30_000);
        set_now(&w, OUT);

        let result = w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::NotApplicable, None));

        match result {
            Err(AppError::Validation(msg)) => assert!(msg.contains("devolver") && msg.contains("R$ 60,00"), "{msg}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn late_checkout_within_tolerance_has_no_fee_and_after_it_requires_a_decision() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        pay(&w, stay, 24_000);
        set_now(&w, OUT_PLUS_30);
        let preview = w.h.stays.preview_checkout(&w.admin, stay).unwrap();
        assert_eq!((preview.late.minutes_late, preview.late.applies, preview.late.fee_cents), (30, false, 0));

        set_now(&w, OUT_PLUS_45);
        let preview = w.h.stays.preview_checkout(&w.admin, stay).unwrap();
        assert_eq!((preview.late.minutes_late, preview.late.applies, preview.late.fee_cents), (45, true, 10_000));
        assert_eq!((preview.balance_cents, preview.balance_with_late_fee_cents), (0, 10_000));
        let undecided = w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::NotApplicable, None));
        assert!(matches!(undecided, Err(AppError::Validation(_))), "{undecided:?}");
        assert_eq!(count_kind(&w, "late_fee"), 0);
    }

    #[test]
    fn charging_the_late_fee_records_it_and_never_creates_a_night() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        pay(&w, stay, 34_000);
        set_now(&w, OUT_PLUS_45);
        let nights_before = count_kind(&w, "night");

        w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::Charge, None)).unwrap();

        assert_eq!(count_kind(&w, "late_fee"), 1);
        assert_eq!(count_kind(&w, "night"), nights_before);
        assert_eq!(stay_column(&w, stay, "late_fee_decision").as_deref(), Some("charged"));
        assert_eq!(stay_column(&w, stay, "late_minutes").as_deref(), Some("45"));
        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        assert_eq!(account.charges.iter().find(|c| c.kind == "late_fee").unwrap().amount_cents, 10_000);
    }

    #[test]
    fn waiving_the_late_fee_needs_permission_and_keeps_the_history() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        pay(&w, stay, 24_000);
        set_now(&w, OUT_PLUS_45);
        let waive = |j: &str| LateFeeChoice::Waive { justification: j.into() };

        let rec = login_with(&w, &[]);
        assert!(matches!(w.h.stays.checkout(&rec, checkout_input(&w, stay, waive("Cortesia"), None)), Err(AppError::Forbidden)));
        assert!(matches!(w.h.stays.checkout(&w.admin, checkout_input(&w, stay, waive("  "), None)), Err(AppError::Validation(_))));
        w.h.stays.checkout(&w.admin, checkout_input(&w, stay, waive("Problema no chuveiro"), None)).unwrap();

        assert_eq!(count_kind(&w, "late_fee"), 0);
        assert_eq!(stay_column(&w, stay, "late_minutes").as_deref(), Some("45"));
        assert_eq!(stay_column(&w, stay, "late_fee_calculated_cents").as_deref(), Some("10000"));
        assert_eq!(stay_column(&w, stay, "late_fee_decision").as_deref(), Some("waived"));
        assert_eq!(stay_column(&w, stay, "late_fee_waive_justification").as_deref(), Some("Problema no chuveiro"));
    }

    #[test]
    fn choosing_a_late_fee_when_none_applies_is_refused() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        pay(&w, stay, 24_000);
        set_now(&w, OUT);

        let result = w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::Charge, None));

        assert!(matches!(result, Err(AppError::Validation(_))), "{result:?}");
        assert_eq!(w.h.stays.get(&w.admin, stay).unwrap().stay.status, "in_house");
    }

    #[test]
    fn per_stay_tolerance_overrides_global() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        set_now(&w, OUT_PLUS_45);
        let rec = login_with(&w, &[]);
        let v = stay_version(&w, stay);
        assert!(matches!(w.h.stays.set_late_tolerance(&rec, stay, Some(120), v), Err(AppError::Forbidden)));
        assert!(matches!(w.h.stays.set_late_tolerance(&w.admin, stay, Some(601), v), Err(AppError::Validation(_))));

        w.h.stays.set_late_tolerance(&w.admin, stay, Some(120), v).unwrap();

        let preview = w.h.stays.preview_checkout(&w.admin, stay).unwrap();
        assert_eq!((preview.late.tolerance_minutes, preview.late.applies), (120, false));
        let v = stay_version(&w, stay);
        w.h.stays.set_late_tolerance(&w.admin, stay, None, v).unwrap();
        assert_eq!(w.h.stays.preview_checkout(&w.admin, stay).unwrap().late.tolerance_minutes, 30);
    }

    #[test]
    fn very_late_checkout_charges_once() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        pay(&w, stay, 34_000);
        set_now(&w, "2026-10-25T12:00:00Z");

        let preview = w.h.stays.preview_checkout(&w.admin, stay).unwrap();
        assert_eq!((preview.late.minutes_late, preview.late.fee_cents), (2_700, 10_000));
        w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::Charge, None)).unwrap();

        assert_eq!(count_kind(&w, "late_fee"), 1);
        assert_eq!(count_kind(&w, "night"), 2);
    }

    #[test]
    fn double_checkout_is_conflict() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        pay(&w, stay, 24_000);
        set_now(&w, OUT);
        let request = checkout_input(&w, stay, LateFeeChoice::NotApplicable, None);

        w.h.stays.checkout(&w.admin, request.clone()).unwrap();
        let second = w.h.stays.checkout(&w.admin, request);

        assert!(matches!(second, Err(AppError::Conflict)), "{second:?}");
    }

    #[test]
    fn a_stale_version_after_a_new_charge_is_conflict() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        pay(&w, stay, 24_000);
        set_now(&w, OUT);
        let request = checkout_input(&w, stay, LateFeeChoice::NotApplicable, None);
        w.h.stays.add_charge(&w.admin, stay, consumption("Água", 500)).unwrap();

        let result = w.h.stays.checkout(&w.admin, request);

        assert!(matches!(result, Err(AppError::Conflict)), "{result:?}");
        assert_eq!(w.h.stays.get(&w.admin, stay).unwrap().stay.status, "in_house");
    }

    #[test]
    fn checkout_failure_rolls_back_everything() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        set_now(&w, OUT_PLUS_45);
        let rec = login_with(&w, &[]);

        let result = w.h.stays.checkout(&rec, checkout_input(&w, stay, LateFeeChoice::Charge, Some("Paga depois")));

        assert!(matches!(result, Err(AppError::Forbidden)), "{result:?}");
        assert_eq!(count_kind(&w, "late_fee"), 0);
        assert_eq!(w.h.stays.get(&w.admin, stay).unwrap().stay.status, "in_house");
    }

    #[test]
    fn late_departure_flags_risk_without_touching_the_next_reservation() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        let next = book(&w.h, &w.admin, &w.seed, "2026-10-23", "2026-10-24");
        set_now(&w, "2026-10-23T16:30:00Z");

        let preview = w.h.stays.preview_checkout(&w.admin, stay).unwrap();

        let risk = preview.risk.expect("risco esperado");
        assert_eq!((risk.next_reservation_id, risk.overrun_minutes), (next, 30));
        assert_eq!(w.h.reservations.get(&w.admin, next).unwrap().reservation.status, "pending");
        let listed = w.h.stays.list(&w.admin, StayQuery { text: None, status: None, page: 1, page_size: 10 }).unwrap();
        assert_eq!(listed.items[0].minutes_late, 90);
        assert!(listed.items[0].risk.is_some());
    }

    #[test]
    fn no_risk_without_a_next_reservation_or_before_the_departure_time() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        set_now(&w, "2026-10-23T16:30:00Z");
        assert!(w.h.stays.preview_checkout(&w.admin, stay).unwrap().risk.is_none());

        set_now(&w, "2026-10-21T18:00:00Z");
        let next = book(&w.h, &w.admin, &w.seed, "2026-10-23", "2026-10-24");
        let _ = next;
        set_now(&w, "2026-10-23T14:00:00Z");
        assert!(w.h.stays.preview_checkout(&w.admin, stay).unwrap().risk.is_none());
    }

    #[test]
    fn preview_does_not_write_and_matches_what_checkout_records() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        pay(&w, stay, 34_000);
        set_now(&w, OUT_PLUS_45);
        let (charges, version, audit) = (count(&w, "charges"), stay_version(&w, stay), count(&w, "audit_log"));

        let preview = w.h.stays.preview_checkout(&w.admin, stay).unwrap();

        assert_eq!((count(&w, "charges"), stay_version(&w, stay), count(&w, "audit_log")), (charges, version, audit));
        w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::Charge, None)).unwrap();
        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        assert_eq!(account.balance_cents, preview.balance_with_late_fee_cents);
        assert_eq!(account.charges.iter().find(|c| c.kind == "late_fee").unwrap().amount_cents, preview.late.fee_cents);
    }

    // ---- Marco 4: portão "quarto limpo" ----

    /// Hóspede sai às 12:00 de 23/10 com a conta quitada; devolve a tarefa de limpeza e uma reserva seguinte 23→24/10.
    fn checked_out_with_next(w: &World) -> (i64, i64) {
        let (_, stay) = stay_with_balance(w);
        pay(w, stay, 24_000);
        set_now(w, OUT);
        w.h.stays.checkout(&w.admin, checkout_input(w, stay, LateFeeChoice::NotApplicable, None)).unwrap();
        let next = book(&w.h, &w.admin, &w.seed, "2026-10-23", "2026-10-24");
        let task: i64 = w.h.db.lock().unwrap().query_row("SELECT id FROM cleaning_tasks", [], |r| r.get(0)).unwrap();
        (next, task)
    }

    fn clean_message(result: Result<i64, AppError>) -> String {
        match result {
            Err(AppError::Validation(msg)) => msg,
            other => panic!("esperava recusa por limpeza: {other:?}"),
        }
    }

    #[test]
    fn check_in_is_refused_while_cleaning_is_awaiting() {
        let w = world();
        let (next, _) = checked_out_with_next(&w);
        set_now(&w, "2026-10-23T17:00:00Z");

        let msg = clean_message(w.h.stays.check_in(&w.admin, input(&w, next, vec![holder(&w)])));

        assert!(msg.contains("aguardando limpeza"), "{msg}");
        assert_eq!(count(&w, "stays"), 1);
    }

    #[test]
    fn check_in_is_refused_while_cleaning_is_in_progress() {
        let w = world();
        let (next, task) = checked_out_with_next(&w);
        w.h.cleaning.start(&w.admin, task, 1).unwrap();
        set_now(&w, "2026-10-23T17:00:00Z");

        let msg = clean_message(w.h.stays.check_in(&w.admin, input(&w, next, vec![holder(&w)])));

        assert!(msg.contains("em limpeza"), "{msg}");
    }

    #[test]
    fn check_in_waits_for_the_interval_after_cleaning_is_done() {
        let w = world();
        let (next, task) = checked_out_with_next(&w);
        w.h.cleaning.start(&w.admin, task, 1).unwrap();
        set_now(&w, "2026-10-23T17:00:00Z"); // 14:00 locais
        w.h.cleaning.complete(&w.admin, task, 2).unwrap();

        set_now(&w, "2026-10-23T17:30:00Z");
        let msg = clean_message(w.h.stays.check_in(&w.admin, input(&w, next, vec![holder(&w)])));
        assert!(msg.contains("15:00") && msg.contains("23/10/2026"), "{msg}");
        assert_eq!(count(&w, "stays"), 1);

        set_now(&w, "2026-10-23T18:00:00Z"); // 15:00 locais = fim do intervalo
        assert!(w.h.stays.check_in(&w.admin, input(&w, next, vec![holder(&w)])).is_ok());
    }

    #[test]
    fn walk_in_is_refused_in_a_dirty_room() {
        let w = world();
        checked_out_with_next(&w);
        set_now(&w, "2026-10-23T17:00:00Z");

        let msg = clean_message(w.h.stays.walk_in(&w.admin, walk_in_input(&w, "2026-10-24", vec![holder(&w)])));

        assert!(msg.contains("aguardando limpeza"), "{msg}");
    }

    #[test]
    fn room_interval_overrides_the_default_for_the_clean_gate() {
        let w = world();
        let (next, task) = checked_out_with_next(&w);
        w.h.catalog.set_room_cleaning_interval(&w.admin, w.seed.room_id, Some(0), 1).unwrap();
        w.h.cleaning.start(&w.admin, task, 1).unwrap();
        set_now(&w, "2026-10-23T17:00:00Z");
        w.h.cleaning.complete(&w.admin, task, 2).unwrap();

        assert!(w.h.stays.check_in(&w.admin, input(&w, next, vec![holder(&w)])).is_ok());
    }

    fn block_from_sql(w: &World, status: &str, from: &str, to: Option<&str>) {
        w.h.db
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO room_blocks (room_id, kind, status, start_date, end_date, reason, created_by, created_at, updated_at)
                 VALUES (?1, 'maintenance', ?2, ?3, ?4, 'Troca do chuveiro', 1, '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z')",
                rusqlite::params![w.seed.room_id, status, from, to],
            )
            .unwrap();
    }

    #[test]
    fn check_in_and_walk_in_respect_an_effective_block_starting_today() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        block_from_sql(&w, "active", "2026-10-21", Some("2026-10-22"));
        set_now(&w, "2026-10-21T17:00:00Z");

        let msg = clean_message(w.h.stays.check_in(&w.admin, input(&w, reservation, vec![holder(&w)])));
        assert!(msg.contains("bloqueado") && msg.contains("Troca do chuveiro"), "{msg}");
        let msg = clean_message(w.h.stays.walk_in(&w.admin, walk_in_input(&w, "2026-10-22", vec![holder(&w)])));
        assert!(msg.contains("bloqueado"), "{msg}");
        assert_eq!(count(&w, "stays"), 0);
    }

    #[test]
    fn a_preventive_block_does_not_stop_the_check_in_of_an_existing_reservation() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        block_from_sql(&w, "preventive", "2026-10-21", Some("2026-10-23"));
        set_now(&w, "2026-10-21T17:00:00Z");

        assert!(w.h.stays.check_in(&w.admin, input(&w, reservation, vec![holder(&w)])).is_ok());
    }

    // ---- correções da revisão final ----

    fn first_booking_conflict(result: Result<i64, AppError>) -> String {
        match result {
            Err(AppError::Validation(msg)) => msg,
            other => panic!("esperava conflito de disponibilidade: {other:?}"),
        }
    }

    #[test]
    fn check_in_is_blocked_while_the_previous_guest_is_still_in_the_room() {
        let w = world();
        let (_, _stay) = stay_with_balance(&w); // RES-000001 em andamento, saída prevista 23/10 12:00
        let next = book(&w.h, &w.admin, &w.seed, "2026-10-23", "2026-10-24");
        set_now(&w, "2026-10-23T17:00:00Z"); // 14:00 locais: o anterior está atrasado e ainda no quarto

        let msg = first_booking_conflict(w.h.stays.check_in(&w.admin, input(&w, next, vec![holder(&w)])));

        assert!(msg.contains("RES-000001"), "{msg}");
        assert_eq!(count(&w, "stays"), 1);
    }

    #[test]
    fn walk_in_and_new_reservations_are_blocked_by_a_stay_that_is_days_overdue() {
        let w = world();
        stay_with_balance(&w);
        set_now(&w, "2026-10-25T15:00:00Z");

        let msg = first_booking_conflict(w.h.stays.walk_in(&w.admin, walk_in_input(&w, "2026-10-26", vec![holder(&w)])));
        assert!(msg.contains("RES-000001"), "{msg}");

        let created = w.h.reservations.create(
            &w.admin,
            crate::models::reservation::NewReservation {
                room_id: w.seed.room_id,
                guest_id: w.seed.guest_id,
                check_in_date: "2026-10-25".into(),
                check_out_date: "2026-10-26".into(),
                // antes de agora + limpeza (12:00 + 60 min): o hóspede anterior ainda está no quarto
                expected_check_in_time: Some("12:30".into()),
                expected_check_out_time: None,
                occupants: 1,
                notes: String::new(),
                exceed_capacity: false,
            },
        );
        assert!(matches!(created, Err(AppError::Validation(_))), "{created:?}");
    }

    #[test]
    fn check_in_after_the_booked_checkout_is_refused() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        set_now(&w, "2026-10-24T13:00:00Z");

        let msg = first_booking_conflict(w.h.stays.check_in(&w.admin, input(&w, reservation, vec![holder(&w)])));

        assert!(msg.contains("horário de saída previsto"), "{msg}");
        assert_eq!(count(&w, "stays"), 0);
    }

    #[test]
    fn a_zero_late_fee_is_not_applicable_and_checkout_is_not_blocked() {
        let w = world();
        let (_, stay) = stay_with_balance(&w);
        let night_ids: Vec<i64> = w.h.stays.get(&w.admin, stay).unwrap().account.charges.iter().filter(|c| c.kind == "night").map(|c| c.id).collect();
        for id in night_ids {
            w.h.stays.reverse_charge(&w.admin, stay, id, "Diárias lançadas por engano".into(), None).unwrap();
        }
        set_now(&w, OUT_PLUS_45);

        let preview = w.h.stays.preview_checkout(&w.admin, stay).unwrap();
        assert_eq!((preview.late.applies, preview.late.fee_cents), (false, 0));
        let result = w.h.stays.checkout(&w.admin, checkout_input(&w, stay, LateFeeChoice::NotApplicable, Some("Crédito a devolver")));

        assert!(result.is_ok(), "{result:?}");
    }
}
