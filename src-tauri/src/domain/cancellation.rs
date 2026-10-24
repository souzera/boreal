//! Política de cancelamento e prazo do sinal (funções puras). Os valores vêm de configuração
//! provisória (Anexo A da spec), nunca de constantes.

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use super::money;
use crate::error::AppError;

/// Faixa da política: com `min_days_before` dias ou mais de antecedência, devolve `refund_bp`
/// (pontos-base, 10000 = 100%) do sinal pago.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tier {
    pub min_days_before: u32,
    pub refund_bp: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub refund_cents: i64,
    pub retained_cents: i64,
    /// Faixa aplicada (`None` no no-show, que não usa faixas).
    pub tier_min_days: Option<u32>,
}

const MAX_DAYS_BEFORE: u32 = 3650;

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

/// A política precisa cobrir qualquer antecedência (faixa de 0 dia), não repetir faixas e
/// nunca devolver mais quando a antecedência diminui.
pub fn validate_tiers(tiers: &[Tier]) -> Result<(), AppError> {
    if tiers.is_empty() {
        return Err(invalid("Informe ao menos uma faixa de cancelamento."));
    }
    let mut sorted = tiers.to_vec();
    sorted.sort_by_key(|t| t.min_days_before);
    if sorted[0].min_days_before != 0 {
        return Err(invalid("Inclua a faixa de 0 dia (cancelamento em cima da hora)."));
    }
    for window in sorted.windows(2) {
        if window[0].min_days_before == window[1].min_days_before {
            return Err(invalid("Não pode haver duas faixas com a mesma antecedência."));
        }
        if window[0].refund_bp > window[1].refund_bp {
            return Err(invalid("A devolução não pode aumentar quando a antecedência diminui."));
        }
    }
    if sorted.iter().any(|t| t.refund_bp > 10_000) {
        return Err(invalid("A devolução não pode passar de 100%."));
    }
    if sorted.iter().any(|t| t.min_days_before > MAX_DAYS_BEFORE) {
        return Err(invalid("Antecedência máxima de 3650 dias."));
    }
    Ok(())
}

/// Quanto do sinal pago é devolvido e quanto é retido. No-show retém tudo.
pub fn compute(tiers: &[Tier], days_before: i64, deposit_paid_cents: i64, no_show: bool) -> Result<Outcome, AppError> {
    if deposit_paid_cents < 0 {
        return Err(invalid("O sinal pago não pode ser negativo."));
    }
    if no_show {
        return Ok(Outcome { refund_cents: 0, retained_cents: deposit_paid_cents, tier_min_days: None });
    }
    let effective = days_before.max(0);
    let tier = tiers
        .iter()
        .filter(|t| i64::from(t.min_days_before) <= effective)
        .max_by_key(|t| t.min_days_before)
        .ok_or_else(|| invalid("A política de cancelamento não tem faixa para esta antecedência."))?;
    let refund_cents = money::percent_of(deposit_paid_cents, tier.refund_bp)?;
    Ok(Outcome {
        refund_cents,
        retained_cents: money::sub(deposit_paid_cents, refund_cents)?,
        tier_min_days: Some(tier.min_days_before),
    })
}

/// Vencimento do sinal: o que vier primeiro entre `reserva + after_days` e `chegada − before_days`,
/// nunca antes do dia da reserva.
pub fn deposit_due_date(booked_on: NaiveDate, check_in: NaiveDate, after_days: u32, before_days: u32) -> NaiveDate {
    let by_booking = booked_on + Duration::days(i64::from(after_days));
    let by_arrival = check_in - Duration::days(i64::from(before_days));
    by_booking.min(by_arrival).max(booked_on)
}

#[cfg(test)]
mod tests {
    use super::{compute, deposit_due_date, validate_tiers, Tier};
    use crate::error::AppError;
    use chrono::NaiveDate;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn default_tiers() -> Vec<Tier> {
        vec![
            Tier { min_days_before: 3, refund_bp: 5000 },
            Tier { min_days_before: 7, refund_bp: 10_000 },
            Tier { min_days_before: 0, refund_bp: 0 },
        ]
    }

    #[test]
    fn picks_the_tier_by_days_before_arrival() {
        let tiers = default_tiers();
        // Sinal de R$ 450,05: 50% = 22.502,5 centavos -> sobe para 22.503.
        let paid = 45_005;
        let cases = [
            (30, 45_005, 0, Some(7)),
            (8, 45_005, 0, Some(7)),
            (7, 45_005, 0, Some(7)),
            (6, 22_503, 22_502, Some(3)),
            (3, 22_503, 22_502, Some(3)),
            (2, 0, 45_005, Some(0)),
            (0, 0, 45_005, Some(0)),
            (-1, 0, 45_005, Some(0)),
        ];

        for (days, refund, retained, tier) in cases {
            let out = compute(&tiers, days, paid, false).unwrap();
            assert_eq!((out.refund_cents, out.retained_cents, out.tier_min_days), (refund, retained, tier), "{days} dias");
        }
    }

    #[test]
    fn no_show_retains_everything() {
        let out = compute(&default_tiers(), 30, 45_005, true).unwrap();

        assert_eq!((out.refund_cents, out.retained_cents, out.tier_min_days), (0, 45_005, None));
    }

    #[test]
    fn zero_deposit_means_nothing_to_refund_or_retain() {
        let out = compute(&default_tiers(), 10, 0, false).unwrap();

        assert_eq!((out.refund_cents, out.retained_cents), (0, 0));
    }

    #[test]
    fn refund_plus_retained_always_equals_the_deposit() {
        for paid in [1, 99, 100, 12_345, 99_999_999] {
            for days in [0, 3, 7] {
                let out = compute(&default_tiers(), days, paid, false).unwrap();
                assert_eq!(out.refund_cents + out.retained_cents, paid);
            }
        }
    }

    #[test]
    fn accepts_a_valid_policy_and_rejects_inconsistent_ones() {
        assert!(validate_tiers(&default_tiers()).is_ok());
        assert!(validate_tiers(&[Tier { min_days_before: 0, refund_bp: 0 }]).is_ok());

        let invalid: Vec<Vec<Tier>> = vec![
            vec![],
            // sem a faixa de 0 dias
            vec![Tier { min_days_before: 3, refund_bp: 5000 }],
            // faixas duplicadas
            vec![Tier { min_days_before: 0, refund_bp: 0 }, Tier { min_days_before: 0, refund_bp: 100 }],
            // devolução maior que 100%
            vec![Tier { min_days_before: 0, refund_bp: 10_001 }],
            // devolução cresce quando a antecedência diminui
            vec![Tier { min_days_before: 7, refund_bp: 0 }, Tier { min_days_before: 0, refund_bp: 5000 }],
            // antecedência absurda
            vec![Tier { min_days_before: 0, refund_bp: 0 }, Tier { min_days_before: 4000, refund_bp: 0 }],
        ];
        for tiers in invalid {
            assert!(matches!(validate_tiers(&tiers), Err(AppError::Validation(_))), "{tiers:?}");
        }
    }

    #[test]
    fn deposit_due_date_is_the_earlier_of_the_two_rules_but_never_before_booking() {
        // reserva feita 2 dias antes da chegada: vence 1 dia antes da chegada
        assert_eq!(deposit_due_date(d("2026-10-20"), d("2026-10-22"), 3, 1), d("2026-10-21"));
        // reserva feita com muita antecedência: vence 3 dias depois
        assert_eq!(deposit_due_date(d("2026-10-01"), d("2026-12-01"), 3, 1), d("2026-10-04"));
        // chegada amanhã: vence hoje
        assert_eq!(deposit_due_date(d("2026-10-22"), d("2026-10-23"), 3, 1), d("2026-10-22"));
        // chegada hoje: nunca antes do dia da reserva
        assert_eq!(deposit_due_date(d("2026-10-22"), d("2026-10-22"), 3, 1), d("2026-10-22"));
        // virada de ano
        assert_eq!(deposit_due_date(d("2026-12-30"), d("2027-02-01"), 3, 1), d("2027-01-02"));
    }
}
