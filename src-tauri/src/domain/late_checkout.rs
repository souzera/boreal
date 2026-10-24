//! Saída tardia e risco de atraso para a próxima reserva (funções puras).

use chrono::{Duration, NaiveDateTime};

use super::money;
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LateAssessment {
    pub minutes_late: i64,
    /// Passou da tolerância e há cobrança configurada.
    pub applies: bool,
    /// Cobrança prevista (0 quando não se aplica). Nunca é uma diária.
    pub fee_cents: i64,
}

/// Atraso em relação à saída prevista. A cobrança prevista é um percentual (`fee_bp`) da última
/// diária, uma única vez; só existe quando o atraso passa da tolerância.
pub fn assess(
    expected_out: NaiveDateTime,
    now: NaiveDateTime,
    tolerance_minutes: u32,
    last_night_cents: i64,
    fee_bp: u32,
) -> Result<LateAssessment, AppError> {
    let minutes_late = (now - expected_out).num_minutes().max(0);
    let past_tolerance = fee_bp > 0 && minutes_late > i64::from(tolerance_minutes);
    let fee_cents = if past_tolerance { money::percent_of(last_night_cents, fee_bp)? } else { 0 };
    // Cobrança prevista de R$ 0,00 (por exemplo, sem diária vigente) não é cobrança: nada a decidir.
    Ok(LateAssessment { minutes_late, applies: fee_cents > 0, fee_cents })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartureRisk {
    pub minutes_late: i64,
    /// Quando o quarto poderia estar pronto: agora + intervalo de limpeza.
    pub ready_at: NaiveDateTime,
    pub overrun_minutes: i64,
}

/// Risco para a próxima chegada: só existe se a saída já atrasou e a limpeza, começando agora,
/// termina depois da entrada seguinte.
pub fn departure_risk(
    expected_out: NaiveDateTime,
    now: NaiveDateTime,
    cleaning_minutes: u32,
    next_start: NaiveDateTime,
) -> Option<DepartureRisk> {
    if now <= expected_out {
        return None;
    }
    let ready_at = now + Duration::minutes(i64::from(cleaning_minutes));
    if ready_at <= next_start {
        return None;
    }
    Some(DepartureRisk {
        minutes_late: (now - expected_out).num_minutes(),
        ready_at,
        overrun_minutes: (ready_at - next_start).num_minutes(),
    })
}

#[cfg(test)]
mod tests {
    use super::{assess, departure_risk};
    use chrono::NaiveDateTime;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn at(now: &str) -> super::LateAssessment {
        assess(dt("2026-10-23 12:00"), dt(now), 30, 20_000, 5_000).unwrap()
    }

    #[test]
    fn within_tolerance_has_no_fee() {
        let a = at("2026-10-23 12:30");
        assert_eq!((a.minutes_late, a.applies, a.fee_cents), (30, false, 0));
    }

    #[test]
    fn one_minute_past_tolerance_applies_half_of_last_night() {
        let a = at("2026-10-23 12:31");
        assert_eq!((a.minutes_late, a.applies, a.fee_cents), (31, true, 10_000));
    }

    #[test]
    fn early_departure_is_not_late() {
        let a = at("2026-10-23 11:00");
        assert_eq!((a.minutes_late, a.applies, a.fee_cents), (0, false, 0));
    }

    #[test]
    fn days_late_is_still_a_single_fee() {
        let a = at("2026-10-25 09:00");
        assert_eq!((a.minutes_late, a.applies, a.fee_cents), (2_700, true, 10_000));
    }

    #[test]
    fn zero_fee_percent_never_applies() {
        let a = assess(dt("2026-10-23 12:00"), dt("2026-10-23 15:00"), 30, 20_000, 0).unwrap();
        assert_eq!((a.minutes_late, a.applies, a.fee_cents), (180, false, 0));
    }

    #[test]
    fn zero_last_night_means_no_fee_to_apply() {
        let a = assess(dt("2026-10-23 12:00"), dt("2026-10-23 15:00"), 30, 0, 5_000).unwrap();
        assert_eq!((a.minutes_late, a.applies, a.fee_cents), (180, false, 0));
    }

    #[test]
    fn year_boundary() {
        let a = assess(dt("2026-12-31 12:00"), dt("2027-01-01 13:00"), 30, 20_000, 5_000).unwrap();
        assert_eq!((a.minutes_late, a.applies), (1_500, true));
    }

    #[test]
    fn fee_rounds_half_cent_up() {
        let a = assess(dt("2026-10-23 12:00"), dt("2026-10-23 13:00"), 30, 10_001, 5_000).unwrap();
        assert_eq!(a.fee_cents, 5_001);
    }

    #[test]
    fn risk_when_cleaning_does_not_fit_before_next_arrival() {
        let r = departure_risk(dt("2026-10-23 12:00"), dt("2026-10-23 13:30"), 60, dt("2026-10-23 14:00")).unwrap();
        assert_eq!((r.minutes_late, r.ready_at, r.overrun_minutes), (90, dt("2026-10-23 14:30"), 30));
    }

    #[test]
    fn no_risk_when_it_still_fits_or_not_late_yet() {
        assert!(departure_risk(dt("2026-10-23 12:00"), dt("2026-10-23 12:10"), 60, dt("2026-10-23 14:00")).is_none());
        assert!(departure_risk(dt("2026-10-23 12:00"), dt("2026-10-23 11:00"), 60, dt("2026-10-23 11:30")).is_none());
    }
}
