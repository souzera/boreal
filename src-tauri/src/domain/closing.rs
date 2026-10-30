//! Linhas do fechamento diário de caixa (função pura).

use super::payment::Method;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedLine {
    pub method: Method,
    pub expected_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountedLine {
    pub method: Method,
    pub counted_cents: i64,
    pub justification: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosingLine {
    pub method: Method,
    pub expected_cents: i64,
    pub counted_cents: i64,
    pub difference_cents: i64,
    pub justification: Option<String>,
}

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

/// Monta as linhas do fechamento. Exige o valor conferido de cada modalidade esperada; diferença exige
/// justificativa, mas nunca impede o fechamento (spec §11).
pub fn build_closing_lines(expected: &[ExpectedLine], counted: &[CountedLine]) -> Result<Vec<ClosingLine>, AppError> {
    let mut lines = Vec::new();
    for method in Method::selectable().into_iter().chain([Method::Unspecified]) {
        let expected_cents = expected.iter().find(|e| e.method == method).map_or(0, |e| e.expected_cents);
        let entry = counted.iter().find(|c| c.method == method);
        if method == Method::Unspecified && expected_cents == 0 && entry.is_none() {
            continue;
        }
        let entry = entry.ok_or_else(|| invalid("Informe o valor conferido de todas as formas de pagamento."))?;
        if entry.counted_cents < 0 {
            return Err(invalid("O valor conferido não pode ser negativo."));
        }
        let difference_cents = super::money::sub(entry.counted_cents, expected_cents)?;
        let justification = entry.justification.as_deref().map(str::trim).filter(|j| !j.is_empty()).map(str::to_string);
        if difference_cents != 0 && justification.is_none() {
            return Err(invalid("Explique a diferença encontrada em cada forma de pagamento."));
        }
        lines.push(ClosingLine { method, expected_cents, counted_cents: entry.counted_cents, difference_cents, justification });
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::{build_closing_lines, CountedLine, ExpectedLine};
    use crate::domain::payment::Method;
    use crate::error::AppError;

    fn expected(cash: i64) -> Vec<ExpectedLine> {
        Method::selectable()
            .into_iter()
            .map(|m| ExpectedLine { method: m, expected_cents: if m == Method::Cash { cash } else { 0 } })
            .collect()
    }

    fn counted(cash: i64, justification: Option<&str>) -> Vec<CountedLine> {
        Method::selectable()
            .into_iter()
            .map(|m| CountedLine {
                method: m,
                counted_cents: if m == Method::Cash { cash } else { 0 },
                justification: if m == Method::Cash { justification.map(str::to_string) } else { None },
            })
            .collect()
    }

    #[test]
    fn difference_is_counted_minus_expected() {
        let lines = build_closing_lines(&expected(10_000), &counted(9_500, Some("Troco devolvido"))).unwrap();
        let cash = lines.iter().find(|l| l.method == Method::Cash).unwrap();
        assert_eq!((cash.expected_cents, cash.counted_cents, cash.difference_cents), (10_000, 9_500, -500));
        assert_eq!(cash.justification.as_deref(), Some("Troco devolvido"));
    }

    #[test]
    fn difference_without_justification_is_refused() {
        for why in [None, Some("   ")] {
            assert!(matches!(build_closing_lines(&expected(10_000), &counted(9_500, why)), Err(AppError::Validation(_))));
        }
    }

    #[test]
    fn difference_with_justification_is_accepted_and_matching_needs_none() {
        assert!(build_closing_lines(&expected(10_000), &counted(10_000, None)).is_ok());
        assert!(build_closing_lines(&expected(10_000), &counted(10_500, Some("Sobra"))).is_ok());
    }

    #[test]
    fn missing_method_is_refused() {
        let mut short = counted(10_000, None);
        short.retain(|l| l.method != Method::Pix);
        assert!(matches!(build_closing_lines(&expected(10_000), &short), Err(AppError::Validation(_))));
    }

    #[test]
    fn unspecified_line_required_only_when_expected_is_not_zero() {
        let mut with_legacy = expected(0);
        with_legacy.push(ExpectedLine { method: Method::Unspecified, expected_cents: 12_000 });
        assert!(build_closing_lines(&with_legacy, &counted(0, None)).is_err());
        let mut all = counted(0, None);
        all.push(CountedLine { method: Method::Unspecified, counted_cents: 12_000, justification: None });
        assert_eq!(build_closing_lines(&with_legacy, &all).unwrap().len(), 6);
        assert_eq!(build_closing_lines(&expected(0), &counted(0, None)).unwrap().len(), 5);
    }

    #[test]
    fn negative_counted_is_refused() {
        assert!(matches!(build_closing_lines(&expected(0), &counted(-1, Some("x"))), Err(AppError::Validation(_))));
    }
}
