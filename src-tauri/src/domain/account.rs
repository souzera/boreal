//! Saldo da conta da hospedagem (função pura).

use super::money;
use crate::error::AppError;

/// `saldo = lançamentos − recebido + devolvido`. Positivo: o hóspede deve; negativo: há crédito a devolver.
pub fn balance(charges_cents: i64, received_cents: i64, refunded_cents: i64) -> Result<i64, AppError> {
    money::add(money::sub(charges_cents, received_cents)?, refunded_cents)
}

#[cfg(test)]
mod tests {
    use super::balance;

    #[test]
    fn balance_is_charges_minus_received_plus_refunded() {
        assert_eq!(balance(40_000, 12_000, 0).unwrap(), 28_000);
        assert_eq!(balance(40_000, 52_000, 2_000).unwrap(), -10_000);
    }

    #[test]
    fn balance_overflow_is_refused() {
        assert!(balance(i64::MAX, -1, 0).is_err());
        assert!(balance(i64::MIN, 1, 0).is_err());
    }

    #[test]
    fn random_sequence_matches_sum() {
        let mut seed: u64 = 42;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as i64 % 50_000
        };
        let (mut charges, mut received, mut refunded) = (0i64, 0i64, 0i64);
        for i in 0..200 {
            let v = next() + 1;
            match i % 3 {
                0 => charges += v,
                1 => received += v,
                _ => refunded += v,
            }
            assert_eq!(balance(charges, received, refunded).unwrap(), charges - received + refunded);
        }
    }
}
