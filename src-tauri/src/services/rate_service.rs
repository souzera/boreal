use serde_json::{json, Value};

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::domain::pricing::{self, Quote, RuleInput, RuleKind};
use crate::domain::dates;
use crate::error::AppError;
use crate::models::rate::{Holiday, NewRateRule, RateRule};
use crate::repositories::{catalog_repo, rate_repo};
use rusqlite::Connection;

const MAX_AMOUNT_CENTS: i64 = 9_999_999; // R$ 99.999,99

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn rule_json(r: &RateRule) -> Value {
    json!({
        "category_id": r.category_id, "rule_type": r.rule_type, "weekday": r.weekday,
        "start_date": r.start_date, "end_date": r.end_date, "date": r.date,
        "amount_cents": r.amount_cents, "label": r.label, "active": r.active
    })
}

fn new_rule_json(r: &NewRateRule) -> Value {
    json!({
        "category_id": r.category_id, "rule_type": r.rule_type, "weekday": r.weekday,
        "start_date": r.start_date, "end_date": r.end_date, "date": r.date,
        "amount_cents": r.amount_cents, "label": r.label
    })
}

/// Confere o formato dos campos conforme o tipo da regra e normaliza o rótulo.
fn validate_shape(r: &NewRateRule) -> Result<NewRateRule, AppError> {
    if !(1..=MAX_AMOUNT_CENTS).contains(&r.amount_cents) {
        return Err(invalid("Informe um valor de diária entre R$ 0,01 e R$ 99.999,99."));
    }
    let label = r.label.trim().to_string();
    if label.chars().count() > 80 {
        return Err(invalid("Descrição: no máximo 80 caracteres."));
    }
    let mut clean = NewRateRule {
        category_id: r.category_id,
        rule_type: r.rule_type.clone(),
        weekday: None,
        start_date: None,
        end_date: None,
        date: None,
        amount_cents: r.amount_cents,
        label,
    };
    let no_extra = |present: bool, what: &str| {
        if present {
            Err(invalid(format!("Este tipo de tarifa não usa {what}.")))
        } else {
            Ok(())
        }
    };
    match r.rule_type.as_str() {
        "weekday" => {
            let day = r.weekday.ok_or_else(|| invalid("Escolha o dia da semana."))?;
            if day > 6 {
                return Err(invalid("Dia da semana inválido."));
            }
            no_extra(r.start_date.is_some() || r.end_date.is_some() || r.date.is_some(), "datas")?;
            clean.weekday = Some(day);
        }
        "season" => {
            let (Some(from), Some(to)) = (r.start_date.as_deref(), r.end_date.as_deref()) else {
                return Err(invalid("Informe o início e o fim da temporada."));
            };
            let (from_d, to_d) = (
                dates::parse(from).ok_or_else(|| invalid("Início da temporada inválido. Use uma data válida."))?,
                dates::parse(to).ok_or_else(|| invalid("Fim da temporada inválido. Use uma data válida."))?,
            );
            if from_d > to_d {
                return Err(invalid("O início da temporada deve ser anterior ou igual ao fim."));
            }
            no_extra(r.weekday.is_some() || r.date.is_some(), "dia da semana ou data única")?;
            clean.start_date = Some(from.to_string());
            clean.end_date = Some(to.to_string());
        }
        "holiday" => {
            no_extra(r.weekday.is_some() || r.start_date.is_some() || r.end_date.is_some() || r.date.is_some(), "datas (vale para todos os feriados cadastrados)")?;
        }
        "date_override" => {
            let date = r.date.as_deref().ok_or_else(|| invalid("Informe a data."))?;
            dates::parse(date).ok_or_else(|| invalid("Data inválida. Use uma data válida."))?;
            no_extra(r.weekday.is_some() || r.start_date.is_some() || r.end_date.is_some(), "dia da semana ou período")?;
            clean.date = Some(date.to_string());
        }
        _ => return Err(invalid("Tipo de tarifa desconhecido.")),
    }
    Ok(clean)
}

fn conflict_message(rule_type: &str) -> &'static str {
    match rule_type {
        "weekday" => "Já existe uma tarifa ativa para este dia da semana nesta categoria.",
        "season" => "Esta temporada se sobrepõe a outra temporada ativa da mesma categoria.",
        "holiday" => "Já existe uma tarifa de feriado ativa nesta categoria.",
        _ => "Já existe uma tarifa ativa para esta data nesta categoria.",
    }
}

fn to_input(r: &RateRule) -> Option<RuleInput> {
    let kind = match r.rule_type.as_str() {
        "weekday" => RuleKind::Weekday,
        "season" => RuleKind::Season,
        "holiday" => RuleKind::Holiday,
        "date_override" => RuleKind::DateOverride,
        _ => return None,
    };
    Some(RuleInput {
        kind,
        weekday: r.weekday,
        start: r.start_date.as_deref().and_then(dates::parse),
        end: r.end_date.as_deref().and_then(dates::parse),
        date: r.date.as_deref().and_then(dates::parse),
        amount_cents: r.amount_cents,
    })
}

/// Cotação da estadia pela tabela vigente da categoria. Usada pela cotação avulsa e pela
/// criação/alteração de reservas (dentro da transação de gravação).
pub(crate) fn quote_in(
    conn: &Connection,
    category_id: i64,
    check_in: chrono::NaiveDate,
    check_out: chrono::NaiveDate,
) -> Result<Quote, AppError> {
    if catalog_repo::find_category(conn, category_id)?.is_none() {
        return Err(invalid("Escolha a categoria do quarto."));
    }
    let rules: Vec<RuleInput> = rate_repo::active_rules(conn, category_id)?
        .iter()
        .filter_map(to_input)
        .collect();
    let holidays: Vec<chrono::NaiveDate> = rate_repo::list_holidays(conn, None)?
        .iter()
        .filter_map(|h| dates::parse(&h.date))
        .collect();
    pricing::price_stay(&rules, &holidays, check_in, check_out)
}

#[derive(Clone)]
pub struct RateService {
    ctx: Ctx,
}

impl RateService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn read(&self, token: &str) -> Result<SessionInfo, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())
    }

    fn write(&self, token: &str) -> Result<SessionInfo, AppError> {
        let info = self.read(token)?;
        require(&info, Permission::ManageCatalog)?;
        Ok(info)
    }

    #[allow(clippy::too_many_arguments)] // cada chamada descreve um evento de auditoria completo
    fn record(
        &self,
        tx: &rusqlite::Transaction,
        actor: &SessionInfo,
        operation: &'static str,
        entity: &'static str,
        entity_id: i64,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), AppError> {
        audit::record(
            tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "rates",
                operation,
                entity,
                entity_id: Some(entity_id),
                before,
                after,
                justification: None,
            },
        )
    }

    /// Simula o preço de uma estadia (qualquer usuário). Datas sem tarifa vêm em `missing_dates`.
    pub fn quote(&self, token: &str, category_id: i64, check_in: &str, check_out: &str) -> Result<Quote, AppError> {
        self.read(token)?;
        let check_in = dates::parse(check_in).ok_or_else(|| invalid("Data de entrada inválida."))?;
        let check_out = dates::parse(check_out).ok_or_else(|| invalid("Data de saída inválida."))?;
        let conn = self.ctx.conn()?;
        quote_in(&conn, category_id, check_in, check_out)
    }

    pub fn list_rules(&self, token: &str, category_id: Option<i64>) -> Result<Vec<RateRule>, AppError> {
        self.read(token)?;
        let conn = self.ctx.conn()?;
        rate_repo::list_rules(&conn, category_id)
    }

    pub fn create_rule(&self, token: &str, input: NewRateRule) -> Result<i64, AppError> {
        let actor = self.write(token)?;
        let rule = validate_shape(&input)?;
        let mut conn = self.ctx.conn()?;
        if catalog_repo::find_category(&conn, rule.category_id)?.is_none() {
            return Err(invalid("Escolha a categoria da tarifa."));
        }
        if rate_repo::find_conflicting_active(&conn, &rule, None)?.is_some() {
            return Err(invalid(conflict_message(&rule.rule_type)));
        }
        let tx = conn.transaction()?;
        let id = rate_repo::insert_rule(&tx, &rule, self.ctx.utc.now_utc())?;
        self.record(&tx, &actor, "create_rate_rule", "rate_rule", id, None, Some(new_rule_json(&rule)))?;
        tx.commit()?;
        Ok(id)
    }

    pub fn update_rule(&self, token: &str, id: i64, input: NewRateRule, expected_version: i64) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let rule = validate_shape(&input)?;
        let mut conn = self.ctx.conn()?;
        let before = rate_repo::find_rule(&conn, id)?.ok_or_else(|| invalid("Tarifa não encontrada."))?;
        if catalog_repo::find_category(&conn, rule.category_id)?.is_none() {
            return Err(invalid("Escolha a categoria da tarifa."));
        }
        if before.active && rate_repo::find_conflicting_active(&conn, &rule, Some(id))?.is_some() {
            return Err(invalid(conflict_message(&rule.rule_type)));
        }
        let tx = conn.transaction()?;
        rate_repo::update_rule(&tx, id, &rule, expected_version, self.ctx.utc.now_utc())?;
        self.record(&tx, &actor, "update_rate_rule", "rate_rule", id, Some(rule_json(&before)), Some(new_rule_json(&rule)))?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_rule_active(&self, token: &str, id: i64, active: bool) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let mut conn = self.ctx.conn()?;
        let before = rate_repo::find_rule(&conn, id)?.ok_or_else(|| invalid("Tarifa não encontrada."))?;
        if active && !before.active {
            let candidate = NewRateRule {
                category_id: before.category_id,
                rule_type: before.rule_type.clone(),
                weekday: before.weekday,
                start_date: before.start_date.clone(),
                end_date: before.end_date.clone(),
                date: before.date.clone(),
                amount_cents: before.amount_cents,
                label: before.label.clone(),
            };
            if rate_repo::find_conflicting_active(&conn, &candidate, Some(id))?.is_some() {
                return Err(invalid(conflict_message(&before.rule_type)));
            }
        }
        let tx = conn.transaction()?;
        rate_repo::set_rule_active(&tx, id, active, self.ctx.utc.now_utc())?;
        self.record(&tx, &actor, "set_rate_rule_active", "rate_rule", id, Some(json!({ "active": before.active })), Some(json!({ "active": active })))?;
        tx.commit()?;
        Ok(())
    }

    /// Tarifas não têm dependentes (as reservas congelam o preço de cada noite), então podem ser apagadas.
    pub fn delete_rule(&self, token: &str, id: i64) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let mut conn = self.ctx.conn()?;
        let before = rate_repo::find_rule(&conn, id)?.ok_or_else(|| invalid("Tarifa não encontrada."))?;
        let tx = conn.transaction()?;
        rate_repo::delete_rule(&tx, id)?;
        self.record(&tx, &actor, "delete_rate_rule", "rate_rule", id, Some(rule_json(&before)), None)?;
        tx.commit()?;
        Ok(())
    }

    // ---- feriados ----

    pub fn list_holidays(&self, token: &str, year: Option<i32>) -> Result<Vec<Holiday>, AppError> {
        self.read(token)?;
        let conn = self.ctx.conn()?;
        rate_repo::list_holidays(&conn, year)
    }

    pub fn create_holiday(&self, token: &str, date: &str, name: &str) -> Result<i64, AppError> {
        let actor = self.write(token)?;
        dates::parse(date).ok_or_else(|| invalid("Data inválida. Use uma data válida."))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(invalid("Informe o nome do feriado."));
        }
        if name.chars().count() > 80 {
            return Err(invalid("Nome: no máximo 80 caracteres."));
        }
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let id = rate_repo::insert_holiday(&tx, date, name, self.ctx.utc.now_utc()).map_err(|e| match e {
            AppError::Database(msg) if msg.contains("UNIQUE") => invalid("Já existe um feriado cadastrado nesta data."),
            other => other,
        })?;
        self.record(&tx, &actor, "create_holiday", "holiday", id, None, Some(json!({ "date": date, "name": name })))?;
        tx.commit()?;
        Ok(id)
    }

    pub fn delete_holiday(&self, token: &str, id: i64) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let mut conn = self.ctx.conn()?;
        let before = rate_repo::find_holiday(&conn, id)?.ok_or_else(|| invalid("Feriado não encontrado."))?;
        let tx = conn.transaction()?;
        rate_repo::delete_holiday(&tx, id)?;
        self.record(&tx, &actor, "delete_holiday", "holiday", id, Some(json!({ "date": before.date, "name": before.name })), None)?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::models::catalog::NewCategory;
    use crate::models::rate::NewRateRule;
    use crate::services::test_support::{admin_token, harness, recepcao_token, Harness};

    fn setup() -> (Harness, String, i64) {
        let h = harness();
        let admin = admin_token(&h);
        let cat = h
            .catalog
            .create_category(&admin, NewCategory { name: "Standard".into(), description: String::new() })
            .unwrap();
        (h, admin, cat)
    }

    fn weekday(cat: i64, day: u8, cents: i64) -> NewRateRule {
        NewRateRule {
            category_id: cat,
            rule_type: "weekday".into(),
            weekday: Some(day),
            start_date: None,
            end_date: None,
            date: None,
            amount_cents: cents,
            label: String::new(),
        }
    }

    fn season(cat: i64, from: &str, to: &str, cents: i64) -> NewRateRule {
        NewRateRule {
            category_id: cat,
            rule_type: "season".into(),
            weekday: None,
            start_date: Some(from.into()),
            end_date: Some(to.into()),
            date: None,
            amount_cents: cents,
            label: "Alta temporada".into(),
        }
    }

    fn holiday_rate(cat: i64, cents: i64) -> NewRateRule {
        NewRateRule {
            category_id: cat,
            rule_type: "holiday".into(),
            weekday: None,
            start_date: None,
            end_date: None,
            date: None,
            amount_cents: cents,
            label: "Feriados".into(),
        }
    }

    fn override_on(cat: i64, date: &str, cents: i64) -> NewRateRule {
        NewRateRule {
            category_id: cat,
            rule_type: "date_override".into(),
            weekday: None,
            start_date: None,
            end_date: None,
            date: Some(date.into()),
            amount_cents: cents,
            label: "Evento".into(),
        }
    }

    fn audit_count(h: &Harness, op: &str) -> i64 {
        h.db.lock()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_log WHERE operation = ?1", [op], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn weekday_rule_is_unique_per_category_and_day() {
        let (h, admin, cat) = setup();
        let other = h
            .catalog
            .create_category(&admin, NewCategory { name: "Luxo".into(), description: String::new() })
            .unwrap();
        h.rates.create_rule(&admin, weekday(cat, 1, 18000)).unwrap();

        assert!(matches!(h.rates.create_rule(&admin, weekday(cat, 1, 19000)), Err(AppError::Validation(_))));
        assert!(h.rates.create_rule(&admin, weekday(cat, 2, 18000)).is_ok());
        assert!(h.rates.create_rule(&admin, weekday(other, 1, 28000)).is_ok());
    }

    #[test]
    fn season_requires_ordered_dates_and_rejects_overlap() {
        let (h, admin, cat) = setup();
        h.rates.create_rule(&admin, season(cat, "2026-12-15", "2027-02-28", 27000)).unwrap();

        assert!(matches!(
            h.rates.create_rule(&admin, season(cat, "2027-02-01", "2027-03-31", 25000)),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            h.rates.create_rule(&admin, season(cat, "2027-03-10", "2027-03-01", 25000)),
            Err(AppError::Validation(_))
        ));
        assert!(h.rates.create_rule(&admin, season(cat, "2027-03-01", "2027-03-31", 25000)).is_ok());
        assert!(h.rates.create_rule(&admin, season(cat, "2026-12-01", "2026-12-14", 22000)).is_ok());
    }

    #[test]
    fn fields_must_match_the_rule_type() {
        let (h, admin, cat) = setup();
        let bad_weekday = weekday(cat, 7, 18000);
        let mut weekday_with_dates = weekday(cat, 1, 18000);
        weekday_with_dates.start_date = Some("2026-01-01".into());
        let mut season_without_end = season(cat, "2026-12-15", "2027-02-28", 27000);
        season_without_end.end_date = None;
        let invalid_date = override_on(cat, "31/12/2026", 30000);
        let mut unknown_type = weekday(cat, 1, 18000);
        unknown_type.rule_type = "promocao".into();
        let mut holiday_with_date = holiday_rate(cat, 25000);
        holiday_with_date.date = Some("2026-11-02".into());

        for rule in [bad_weekday, weekday_with_dates, season_without_end, invalid_date, unknown_type, holiday_with_date] {
            assert!(matches!(h.rates.create_rule(&admin, rule), Err(AppError::Validation(_))));
        }
    }

    #[test]
    fn amount_must_be_positive_and_bounded() {
        let (h, admin, cat) = setup();

        for cents in [0, -100, 10_000_000] {
            assert!(matches!(h.rates.create_rule(&admin, weekday(cat, 1, cents)), Err(AppError::Validation(_))));
        }
        assert!(h.rates.create_rule(&admin, weekday(cat, 1, 9_999_999)).is_ok());
    }

    #[test]
    fn holiday_rate_and_date_override_are_unique() {
        let (h, admin, cat) = setup();
        h.rates.create_rule(&admin, holiday_rate(cat, 25000)).unwrap();
        h.rates.create_rule(&admin, override_on(cat, "2026-11-07", 32000)).unwrap();

        assert!(matches!(h.rates.create_rule(&admin, holiday_rate(cat, 26000)), Err(AppError::Validation(_))));
        assert!(matches!(
            h.rates.create_rule(&admin, override_on(cat, "2026-11-07", 33000)),
            Err(AppError::Validation(_))
        ));
        assert!(h.rates.create_rule(&admin, override_on(cat, "2026-11-08", 33000)).is_ok());
    }

    #[test]
    fn rule_requires_an_existing_category() {
        let (h, admin, _) = setup();

        assert!(matches!(h.rates.create_rule(&admin, weekday(999, 1, 18000)), Err(AppError::Validation(_))));
    }

    #[test]
    fn recepcao_reads_but_cannot_write() {
        let (h, admin, cat) = setup();
        let id = h.rates.create_rule(&admin, weekday(cat, 1, 18000)).unwrap();
        let rec = recepcao_token(&h, &admin);

        assert_eq!(h.rates.list_rules(&rec, Some(cat)).unwrap().len(), 1);
        assert!(h.rates.list_holidays(&rec, None).is_ok());
        assert!(matches!(h.rates.create_rule(&rec, weekday(cat, 2, 18000)), Err(AppError::Forbidden)));
        assert!(matches!(h.rates.update_rule(&rec, id, weekday(cat, 1, 19000), 1), Err(AppError::Forbidden)));
        assert!(matches!(h.rates.set_rule_active(&rec, id, false), Err(AppError::Forbidden)));
        assert!(matches!(h.rates.delete_rule(&rec, id), Err(AppError::Forbidden)));
        assert!(matches!(h.rates.create_holiday(&rec, "2026-11-02", "Finados"), Err(AppError::Forbidden)));
    }

    #[test]
    fn update_changes_value_and_conflicts_on_stale_version() {
        let (h, admin, cat) = setup();
        let id = h.rates.create_rule(&admin, weekday(cat, 1, 18000)).unwrap();

        h.rates.update_rule(&admin, id, weekday(cat, 1, 19500), 1).unwrap();

        assert_eq!(h.rates.list_rules(&admin, Some(cat)).unwrap()[0].amount_cents, 19500);
        assert!(matches!(h.rates.update_rule(&admin, id, weekday(cat, 1, 20000), 1), Err(AppError::Conflict)));
    }

    #[test]
    fn update_does_not_collide_with_itself_but_with_others() {
        let (h, admin, cat) = setup();
        let s1 = h.rates.create_rule(&admin, season(cat, "2026-12-15", "2027-02-28", 27000)).unwrap();
        h.rates.create_rule(&admin, season(cat, "2027-03-01", "2027-03-31", 25000)).unwrap();

        assert!(h.rates.update_rule(&admin, s1, season(cat, "2026-12-20", "2027-02-28", 27000), 1).is_ok());
        assert!(matches!(
            h.rates.update_rule(&admin, s1, season(cat, "2026-12-20", "2027-03-05", 27000), 2),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn inactive_rules_do_not_block_and_reactivation_is_revalidated() {
        let (h, admin, cat) = setup();
        let old = h.rates.create_rule(&admin, weekday(cat, 1, 18000)).unwrap();
        h.rates.set_rule_active(&admin, old, false).unwrap();
        h.rates.create_rule(&admin, weekday(cat, 1, 20000)).unwrap();

        assert!(matches!(h.rates.set_rule_active(&admin, old, true), Err(AppError::Validation(_))));
    }

    #[test]
    fn holidays_have_unique_valid_dates_and_filter_by_year() {
        let (h, admin, _) = setup();
        h.rates.create_holiday(&admin, "2026-11-02", "Finados").unwrap();
        h.rates.create_holiday(&admin, "2027-01-01", "Confraternização Universal").unwrap();

        assert!(matches!(h.rates.create_holiday(&admin, "2026-11-02", "Outro"), Err(AppError::Validation(_))));
        assert!(matches!(h.rates.create_holiday(&admin, "02/11/2026", "Finados"), Err(AppError::Validation(_))));
        assert!(matches!(h.rates.create_holiday(&admin, "2026-11-03", "  "), Err(AppError::Validation(_))));
        assert_eq!(h.rates.list_holidays(&admin, Some(2026)).unwrap().len(), 1);
        assert_eq!(h.rates.list_holidays(&admin, None).unwrap().len(), 2);
        let id = h.rates.list_holidays(&admin, Some(2026)).unwrap()[0].id;
        h.rates.delete_holiday(&admin, id).unwrap();
        assert_eq!(h.rates.list_holidays(&admin, None).unwrap().len(), 1);
    }

    #[test]
    fn deleting_a_rule_is_allowed_and_everything_is_audited() {
        let (h, admin, cat) = setup();
        let id = h.rates.create_rule(&admin, weekday(cat, 1, 18000)).unwrap();
        h.rates.update_rule(&admin, id, weekday(cat, 1, 19000), 1).unwrap();
        h.rates.set_rule_active(&admin, id, false).unwrap();
        h.rates.delete_rule(&admin, id).unwrap();
        h.rates.create_holiday(&admin, "2026-11-02", "Finados").unwrap();

        assert!(h.rates.list_rules(&admin, Some(cat)).unwrap().is_empty());
        for op in ["create_rate_rule", "update_rate_rule", "set_rate_rule_active", "delete_rate_rule", "create_holiday"] {
            assert_eq!(audit_count(&h, op), 1, "{op}");
        }
    }

    fn all_weekdays(cat: i64, cents: i64) -> Vec<NewRateRule> {
        (0..7).map(|day| weekday(cat, day, cents)).collect()
    }

    #[test]
    fn quote_uses_only_the_rules_of_the_category() {
        let (h, admin, cat) = setup();
        let luxo = h
            .catalog
            .create_category(&admin, NewCategory { name: "Luxo".into(), description: String::new() })
            .unwrap();
        for r in all_weekdays(cat, 10_000).into_iter().chain(all_weekdays(luxo, 20_000)) {
            h.rates.create_rule(&admin, r).unwrap();
        }

        let standard = h.rates.quote(&admin, cat, "2026-10-26", "2026-10-28").unwrap();
        let deluxe = h.rates.quote(&admin, luxo, "2026-10-26", "2026-10-28").unwrap();

        assert_eq!((standard.total_cents, deluxe.total_cents), (20_000, 40_000));
    }

    #[test]
    fn quote_applies_the_holiday_calendar_and_ignores_inactive_rules() {
        let (h, admin, cat) = setup();
        for r in all_weekdays(cat, 10_000) {
            h.rates.create_rule(&admin, r).unwrap();
        }
        h.rates.create_rule(&admin, holiday_rate(cat, 30_000)).unwrap();
        let promo = h.rates.create_rule(&admin, override_on(cat, "2026-11-03", 50_000)).unwrap();
        h.rates.create_holiday(&admin, "2026-11-02", "Finados").unwrap();
        h.rates.set_rule_active(&admin, promo, false).unwrap();

        let q = h.rates.quote(&admin, cat, "2026-11-01", "2026-11-04").unwrap();

        let amounts: Vec<_> = q.nights.iter().map(|n| n.amount_cents).collect();
        assert_eq!(amounts, [10_000, 30_000, 10_000]);
    }

    #[test]
    fn quote_reports_missing_dates_and_validates_input() {
        let (h, admin, cat) = setup();
        h.rates.create_rule(&admin, weekday(cat, 1, 10_000)).unwrap();

        let q = h.rates.quote(&admin, cat, "2026-10-26", "2026-10-28").unwrap();
        assert_eq!(q.missing_dates, ["2026-10-27"]);
        assert!(matches!(h.rates.quote(&admin, cat, "26/10/2026", "2026-10-28"), Err(AppError::Validation(_))));
        assert!(matches!(h.rates.quote(&admin, cat, "2026-10-28", "2026-10-26"), Err(AppError::Validation(_))));
        assert!(matches!(h.rates.quote(&admin, 999, "2026-10-26", "2026-10-28"), Err(AppError::Validation(_))));
    }

    #[test]
    fn quote_is_available_to_recepcao() {
        let (h, admin, cat) = setup();
        for r in all_weekdays(cat, 10_000) {
            h.rates.create_rule(&admin, r).unwrap();
        }
        let rec = recepcao_token(&h, &admin);

        assert_eq!(h.rates.quote(&rec, cat, "2026-10-26", "2026-10-27").unwrap().total_cents, 10_000);
    }
}
