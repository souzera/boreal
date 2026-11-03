use crate::error::AppError;

fn overflow() -> AppError {
    AppError::Validation("Valor muito grande.".into())
}

/// Percentual de um valor em centavos. `basis_points` usa pontos-base (10000 = 100%).
/// Arredonda ao centavo mais próximo; meio centavo sobe (spec §19.2, decisão D-12).
pub fn percent_of(cents: i64, basis_points: u32) -> Result<i64, AppError> {
    if cents < 0 {
        return Err(AppError::Validation("O valor não pode ser negativo.".into()));
    }
    let numerator = i128::from(cents) * i128::from(basis_points);
    let mut result = numerator / 10_000;
    if numerator % 10_000 >= 5_000 {
        result += 1;
    }
    i64::try_from(result).map_err(|_| overflow())
}

pub fn add(a: i64, b: i64) -> Result<i64, AppError> {
    a.checked_add(b).ok_or_else(overflow)
}

pub fn sub(a: i64, b: i64) -> Result<i64, AppError> {
    a.checked_sub(b).ok_or_else(overflow)
}

pub fn sum(values: impl IntoIterator<Item = i64>) -> Result<i64, AppError> {
    values.into_iter().try_fold(0i64, add)
}

#[cfg(test)]
mod tests {
    use super::{add, percent_of, sub, sum};
    use crate::error::AppError;

    #[test]
    fn rounds_to_nearest_cent_half_up() {
        // 30% de R$ 1.003,35 = R$ 301,005 -> R$ 301,01
        assert_eq!(percent_of(100_335, 3000).unwrap(), 30_101);
        // 30% de R$ 1.003,34 = R$ 301,002 -> R$ 301,00
        assert_eq!(percent_of(100_334, 3000).unwrap(), 30_100);
        // meio centavo sobe
        assert_eq!(percent_of(5, 5000).unwrap(), 3);
        assert_eq!(percent_of(4, 5000).unwrap(), 2);
    }

    #[test]
    fn handles_edges() {
        assert_eq!(percent_of(0, 3000).unwrap(), 0);
        assert_eq!(percent_of(123_456, 0).unwrap(), 0);
        assert_eq!(percent_of(123_456, 10_000).unwrap(), 123_456);
        assert_eq!(percent_of(i64::MAX, 10_000).unwrap(), i64::MAX);
    }

    #[test]
    fn rejects_overflow_and_negative_amounts() {
        assert!(matches!(percent_of(i64::MAX, 20_000), Err(AppError::Validation(_))));
        assert!(matches!(percent_of(-1, 3000), Err(AppError::Validation(_))));
    }

    #[test]
    fn checked_arithmetic() {
        assert_eq!(add(100, 250).unwrap(), 350);
        assert_eq!(sub(100, 250).unwrap(), -150);
        assert_eq!(sum([100, 200, 300]).unwrap(), 600);
        assert!(matches!(add(i64::MAX, 1), Err(AppError::Validation(_))));
        assert!(matches!(sub(i64::MIN, 1), Err(AppError::Validation(_))));
        assert!(matches!(sum([i64::MAX, 1]), Err(AppError::Validation(_))));
    }
}
