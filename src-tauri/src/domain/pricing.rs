//! Motor de preço (função pura, sem banco).
//!
//! Para cada noite aplica a primeira regra que existir, nesta ordem (spec §5):
//! exceção de data > feriado > temporada > dia da semana. A data de saída não gera diária.

use chrono::{Datelike, NaiveDate};
use serde::Serialize;

use super::dates;
use super::money;
use crate::error::AppError;

/// Limite de noites por cotação (evita cotações absurdas e laços longos).
pub const MAX_NIGHTS: i64 = 366;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    Weekday,
    Season,
    Holiday,
    DateOverride,
}

/// Regra de tarifa de **uma categoria**, já ativa e com datas convertidas.
#[derive(Debug, Clone)]
pub struct RuleInput {
    pub kind: RuleKind,
    /// 0 = domingo … 6 = sábado.
    pub weekday: Option<u8>,
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
    pub date: Option<NaiveDate>,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppliedRule {
    DateOverride,
    Holiday,
    Season,
    Weekday,
}

#[derive(Debug, Clone, Serialize)]
pub struct NightPrice {
    pub date: String,
    pub amount_cents: i64,
    pub rule: AppliedRule,
}

#[derive(Debug, Clone, Serialize)]
pub struct Quote {
    pub nights: Vec<NightPrice>,
    pub total_cents: i64,
    /// Noites sem tarifa. Se não estiver vazio, a reserva **não pode** ser criada.
    pub missing_dates: Vec<String>,
}

fn rate_for(rules: &[RuleInput], holidays: &[NaiveDate], day: NaiveDate) -> Option<(i64, AppliedRule)> {
    let weekday_sun0 = day.weekday().num_days_from_sunday() as u8;
    let pick = |f: &dyn Fn(&RuleInput) -> bool| rules.iter().find(|r| f(r)).map(|r| r.amount_cents);

    if let Some(cents) = pick(&|r| r.kind == RuleKind::DateOverride && r.date == Some(day)) {
        return Some((cents, AppliedRule::DateOverride));
    }
    if let Some(cents) = holidays
        .contains(&day)
        .then(|| pick(&|r| r.kind == RuleKind::Holiday))
        .flatten()
    {
        return Some((cents, AppliedRule::Holiday));
    }
    if let Some(cents) = pick(&|r| {
        r.kind == RuleKind::Season && r.start.is_some_and(|s| s <= day) && r.end.is_some_and(|e| day <= e)
    }) {
        return Some((cents, AppliedRule::Season));
    }
    pick(&|r| r.kind == RuleKind::Weekday && r.weekday == Some(weekday_sun0)).map(|c| (c, AppliedRule::Weekday))
}

/// Cota a estadia de `check_in` (inclusive) até `check_out` (exclusive).
pub fn price_stay(
    rules: &[RuleInput],
    holidays: &[NaiveDate],
    check_in: NaiveDate,
    check_out: NaiveDate,
) -> Result<Quote, AppError> {
    let nights = (check_out - check_in).num_days();
    if nights < 1 {
        return Err(AppError::Validation("A saída deve ser depois da entrada.".into()));
    }
    if nights > MAX_NIGHTS {
        return Err(AppError::Validation(format!("A estadia pode ter no máximo {MAX_NIGHTS} noites.")));
    }

    let mut priced = Vec::new();
    let mut missing = Vec::new();
    for offset in 0..nights {
        let day = check_in + chrono::Duration::days(offset);
        match rate_for(rules, holidays, day) {
            Some((amount_cents, rule)) => priced.push(NightPrice { date: dates::format(day), amount_cents, rule }),
            None => missing.push(dates::format(day)),
        }
    }
    let total_cents = money::sum(priced.iter().map(|n| n.amount_cents))?;
    Ok(Quote { nights: priced, total_cents, missing_dates: missing })
}

#[cfg(test)]
mod tests {
    use super::{price_stay, AppliedRule, RuleInput, RuleKind};
    use crate::error::AppError;
    use chrono::NaiveDate;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn weekday(day: u8, cents: i64) -> RuleInput {
        RuleInput { kind: RuleKind::Weekday, weekday: Some(day), start: None, end: None, date: None, amount_cents: cents }
    }

    fn season(from: &str, to: &str, cents: i64) -> RuleInput {
        RuleInput { kind: RuleKind::Season, weekday: None, start: Some(d(from)), end: Some(d(to)), date: None, amount_cents: cents }
    }

    fn holiday(cents: i64) -> RuleInput {
        RuleInput { kind: RuleKind::Holiday, weekday: None, start: None, end: None, date: None, amount_cents: cents }
    }

    fn override_on(date: &str, cents: i64) -> RuleInput {
        RuleInput { kind: RuleKind::DateOverride, weekday: None, start: None, end: None, date: Some(d(date)), amount_cents: cents }
    }

    /// Todos os dias da semana com o mesmo valor, para isolar a regra testada.
    fn every_weekday(cents: i64) -> Vec<RuleInput> {
        (0..7).map(|day| weekday(day, cents)).collect()
    }

    #[test]
    fn priority_date_override_beats_holiday_beats_season_beats_weekday() {
        // 02/11/2026 é segunda-feira, feriado, está numa temporada e tem exceção de data.
        let holidays = [d("2026-11-02")];
        let mut rules = every_weekday(10_000);
        rules.push(season("2026-10-01", "2026-12-31", 20_000));
        rules.push(holiday(30_000));
        rules.push(override_on("2026-11-02", 40_000));

        let night = |rules: &[RuleInput]| {
            let q = price_stay(rules, &holidays, d("2026-11-02"), d("2026-11-03")).unwrap();
            (q.nights[0].amount_cents, q.nights[0].rule)
        };

        assert_eq!(night(&rules), (40_000, AppliedRule::DateOverride));
        rules.pop();
        assert_eq!(night(&rules), (30_000, AppliedRule::Holiday));
        rules.pop();
        assert_eq!(night(&rules), (20_000, AppliedRule::Season));
        rules.pop();
        assert_eq!(night(&rules), (10_000, AppliedRule::Weekday));
    }

    #[test]
    fn holiday_rate_applies_only_to_dates_in_the_calendar() {
        let mut rules = every_weekday(10_000);
        rules.push(holiday(30_000));

        let q = price_stay(&rules, &[d("2026-11-15")], d("2026-11-14"), d("2026-11-17")).unwrap();

        let by_date: Vec<_> = q.nights.iter().map(|n| (n.date.as_str(), n.amount_cents)).collect();
        assert_eq!(by_date, [("2026-11-14", 10_000), ("2026-11-15", 30_000), ("2026-11-16", 10_000)]);
    }

    #[test]
    fn checkout_night_is_not_charged() {
        let q = price_stay(&every_weekday(10_000), &[], d("2026-10-10"), d("2026-10-13")).unwrap();

        let dates: Vec<_> = q.nights.iter().map(|n| n.date.as_str()).collect();
        assert_eq!(dates, ["2026-10-10", "2026-10-11", "2026-10-12"]);
        assert_eq!(q.total_cents, 30_000);
    }

    #[test]
    fn season_crossing_the_year_applies_on_both_sides() {
        let mut rules = every_weekday(10_000);
        rules.push(season("2026-12-15", "2027-02-28", 27_000));

        let q = price_stay(&rules, &[], d("2026-12-28"), d("2027-01-03")).unwrap();

        assert_eq!(q.nights.len(), 6);
        assert!(q.nights.iter().all(|n| n.rule == AppliedRule::Season && n.amount_cents == 27_000));
    }

    #[test]
    fn season_edges_are_inclusive() {
        let mut rules = every_weekday(10_000);
        rules.push(season("2026-12-15", "2026-12-16", 27_000));

        let q = price_stay(&rules, &[], d("2026-12-14"), d("2026-12-18")).unwrap();

        let amounts: Vec<_> = q.nights.iter().map(|n| n.amount_cents).collect();
        assert_eq!(amounts, [10_000, 27_000, 27_000, 10_000]);
    }

    #[test]
    fn weekend_differs_from_weekdays() {
        // 22/10/2026 quinta, 23 sexta, 24 sábado, 25 domingo.
        let rules = vec![weekday(0, 10_000), weekday(4, 10_000), weekday(5, 15_000), weekday(6, 15_000)];

        let q = price_stay(&rules, &[], d("2026-10-22"), d("2026-10-26")).unwrap();

        let amounts: Vec<_> = q.nights.iter().map(|n| n.amount_cents).collect();
        assert_eq!(amounts, [10_000, 15_000, 15_000, 10_000]);
    }

    #[test]
    fn missing_tariff_lists_the_dates_and_never_charges_zero() {
        // Só há tarifa de segunda-feira (1); 26/10/2026 é segunda.
        let q = price_stay(&[weekday(1, 10_000)], &[], d("2026-10-25"), d("2026-10-28")).unwrap();

        assert_eq!(q.missing_dates, ["2026-10-25", "2026-10-27"]);
        assert_eq!(q.nights.len(), 1);
        assert_eq!(q.nights[0].date, "2026-10-26");
        assert!(q.nights.iter().all(|n| n.amount_cents > 0));
        assert_eq!(q.total_cents, 10_000);
    }

    #[test]
    fn leap_day_is_a_valid_night() {
        let q = price_stay(&every_weekday(10_000), &[], d("2028-02-28"), d("2028-03-01")).unwrap();

        let dates: Vec<_> = q.nights.iter().map(|n| n.date.as_str()).collect();
        assert_eq!(dates, ["2028-02-28", "2028-02-29"]);
    }

    #[test]
    fn invalid_ranges_are_rejected() {
        let rules = every_weekday(10_000);

        for (a, b) in [("2026-10-10", "2026-10-10"), ("2026-10-10", "2026-10-09")] {
            assert!(matches!(price_stay(&rules, &[], d(a), d(b)), Err(AppError::Validation(_))));
        }
        assert!(matches!(
            price_stay(&rules, &[], d("2026-01-01"), d("2027-01-03")),
            Err(AppError::Validation(_))
        ));
        assert!(price_stay(&rules, &[], d("2026-01-01"), d("2027-01-02")).is_ok());
    }

    #[test]
    fn total_is_the_sum_of_the_nights_and_overflow_is_refused() {
        let q = price_stay(&every_weekday(12_345), &[], d("2026-10-10"), d("2026-10-14")).unwrap();
        assert_eq!(q.total_cents, 4 * 12_345);

        let huge = every_weekday(i64::MAX / 2);
        assert!(matches!(price_stay(&huge, &[], d("2026-10-10"), d("2026-10-14")), Err(AppError::Validation(_))));
    }
}
