//! Recebimentos e devoluções: modalidades, divisão em componentes e duplicidade (funções puras).

use std::collections::BTreeSet;

use chrono::NaiveDate;

use super::money;
use crate::error::AppError;

/// Modalidade de recebimento ou devolução (spec §9.1). `Unspecified` só existe em dados anteriores ao Marco 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Method {
    Cash,
    Pix,
    Debit,
    Credit,
    Transfer,
    Unspecified,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Cash => "cash",
            Method::Pix => "pix",
            Method::Debit => "debit",
            Method::Credit => "credit",
            Method::Transfer => "transfer",
            Method::Unspecified => "unspecified",
        }
    }

    pub fn parse(value: &str) -> Option<Method> {
        Method::selectable().into_iter().chain([Method::Unspecified]).find(|m| m.as_str() == value)
    }

    /// As cinco modalidades que a interface oferece.
    pub fn selectable() -> [Method; 5] {
        [Method::Cash, Method::Pix, Method::Debit, Method::Credit, Method::Transfer]
    }

    pub fn label(self) -> &'static str {
        match self {
            Method::Cash => "Dinheiro",
            Method::Pix => "Pix",
            Method::Debit => "Cartão de débito",
            Method::Credit => "Cartão de crédito",
            Method::Transfer => "Transferência bancária",
            Method::Unspecified => "Sem modalidade",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component {
    pub method: Method,
    pub amount_cents: i64,
}

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

/// Confere a divisão de um recebimento ou devolução e devolve o total (soma exata dos componentes).
pub fn validate_components(items: &[Component], max_total_cents: i64) -> Result<i64, AppError> {
    if items.is_empty() {
        return Err(invalid("Informe ao menos uma forma de pagamento."));
    }
    let mut seen = BTreeSet::new();
    for item in items {
        if item.method == Method::Unspecified {
            return Err(invalid("Escolha uma forma de pagamento válida."));
        }
        if item.amount_cents <= 0 {
            return Err(invalid("Cada forma de pagamento precisa de um valor maior que zero."));
        }
        if !seen.insert(item.method) {
            return Err(invalid("Cada forma de pagamento só pode aparecer uma vez."));
        }
    }
    let total = money::sum(items.iter().map(|i| i.amount_cents))?;
    if total > max_total_cents {
        return Err(invalid("O valor total passa do limite de R$ 1.000.000,00."));
    }
    Ok(total)
}

/// O que identifica um possível recebimento duplicado dentro da mesma hospedagem ou reserva (M5-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentFacts {
    pub total_cents: i64,
    pub methods: BTreeSet<Method>,
    pub local_date: NaiveDate,
}

pub fn is_possible_duplicate(candidate: &PaymentFacts, existing: &PaymentFacts) -> bool {
    candidate == existing
}

/// Quanto ainda pode ser devolvido de um recebimento.
pub fn refundable_cents(payment_total: i64, already_refunded: i64) -> Result<i64, AppError> {
    let left = money::sub(payment_total, already_refunded)?;
    if left < 0 {
        return Err(invalid("As devoluções passam do valor recebido."));
    }
    Ok(left)
}

#[cfg(test)]
mod tests {
    use super::{is_possible_duplicate, refundable_cents, validate_components, Component, Method, PaymentFacts};
    use crate::error::AppError;
    use chrono::NaiveDate;
    use std::collections::BTreeSet;

    const LIMIT: i64 = 100_000_000;

    fn c(method: Method, cents: i64) -> Component {
        Component { method, amount_cents: cents }
    }

    fn facts(total: i64, methods: &[Method], day: u32) -> PaymentFacts {
        PaymentFacts {
            total_cents: total,
            methods: methods.iter().copied().collect::<BTreeSet<_>>(),
            local_date: NaiveDate::from_ymd_opt(2026, 10, day).unwrap(),
        }
    }

    #[test]
    fn components_must_sum_exactly() {
        assert_eq!(validate_components(&[c(Method::Pix, 6_000), c(Method::Cash, 4_000)], LIMIT).unwrap(), 10_000);
    }

    #[test]
    fn rejects_empty_zero_negative_repeated_and_unspecified() {
        for bad in [
            vec![],
            vec![c(Method::Pix, 0)],
            vec![c(Method::Pix, -100)],
            vec![c(Method::Pix, 100), c(Method::Pix, 200)],
            vec![c(Method::Unspecified, 100)],
        ] {
            assert!(matches!(validate_components(&bad, LIMIT), Err(AppError::Validation(_))), "{bad:?}");
        }
    }

    #[test]
    fn rejects_total_above_limit() {
        assert!(validate_components(&[c(Method::Cash, LIMIT)], LIMIT).is_ok());
        assert!(matches!(validate_components(&[c(Method::Cash, LIMIT + 1)], LIMIT), Err(AppError::Validation(_))));
        assert!(validate_components(&[c(Method::Cash, i64::MAX), c(Method::Pix, 1)], LIMIT).is_err());
    }

    #[test]
    fn debit_and_credit_are_distinct_methods() {
        assert_eq!(validate_components(&[c(Method::Debit, 1_000), c(Method::Credit, 2_000)], LIMIT).unwrap(), 3_000);
        assert_ne!(Method::Debit.as_str(), Method::Credit.as_str());
        assert_eq!(Method::parse("debit"), Some(Method::Debit));
        assert_eq!(Method::parse("cheque"), None);
        assert_eq!(Method::Credit.label(), "Cartão de crédito");
    }

    #[test]
    fn duplicate_needs_same_total_methods_and_local_date() {
        let base = facts(10_000, &[Method::Pix], 22);
        assert!(is_possible_duplicate(&base, &facts(10_000, &[Method::Pix], 22)));
        assert!(!is_possible_duplicate(&base, &facts(10_000, &[Method::Cash], 22)));
        assert!(!is_possible_duplicate(&base, &facts(10_000, &[Method::Pix, Method::Cash], 22)));
        assert!(!is_possible_duplicate(&base, &facts(10_001, &[Method::Pix], 22)));
        assert!(!is_possible_duplicate(&base, &facts(10_000, &[Method::Pix], 23)));
    }

    #[test]
    fn refundable_is_payment_minus_refunded_and_never_negative() {
        assert_eq!(refundable_cents(10_000, 3_000).unwrap(), 7_000);
        assert_eq!(refundable_cents(10_000, 10_000).unwrap(), 0);
        assert!(refundable_cents(10_000, 10_001).is_err());
    }
}
