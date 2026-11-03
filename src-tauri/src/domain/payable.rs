//! Situação de uma conta a pagar (função pura).

use chrono::NaiveDate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayableStatus {
    Pending,
    Partial,
    Paid,
    Overdue,
}

impl PayableStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PayableStatus::Pending => "pending",
            PayableStatus::Partial => "partial",
            PayableStatus::Paid => "paid",
            PayableStatus::Overdue => "overdue",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PayableStatus::Pending => "Pendente",
            PayableStatus::Partial => "Parcial",
            PayableStatus::Paid => "Paga",
            PayableStatus::Overdue => "Vencida",
        }
    }
}

/// Situação calculada: vencida só depois do dia do vencimento e com saldo em aberto (spec §10.2).
pub fn payable_status(amount_cents: i64, paid_cents: i64, due: NaiveDate, today: NaiveDate) -> PayableStatus {
    if paid_cents >= amount_cents {
        PayableStatus::Paid
    } else if due < today {
        PayableStatus::Overdue
    } else if paid_cents > 0 {
        PayableStatus::Partial
    } else {
        PayableStatus::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::{payable_status, PayableStatus};
    use chrono::NaiveDate;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    #[test]
    fn due_today_is_not_overdue() {
        assert_eq!(payable_status(50_000, 0, d(10), d(10)), PayableStatus::Pending);
    }

    #[test]
    fn due_yesterday_unpaid_is_overdue() {
        assert_eq!(payable_status(50_000, 0, d(9), d(10)), PayableStatus::Overdue);
    }

    #[test]
    fn partially_paid_and_overdue_is_overdue() {
        assert_eq!(payable_status(50_000, 20_000, d(9), d(10)), PayableStatus::Overdue);
    }

    #[test]
    fn fully_paid_is_paid_even_if_late() {
        assert_eq!(payable_status(50_000, 50_000, d(1), d(10)), PayableStatus::Paid);
    }

    #[test]
    fn partial_before_due_is_partial() {
        assert_eq!(payable_status(50_000, 20_000, d(15), d(10)), PayableStatus::Partial);
        assert_eq!(PayableStatus::Partial.as_str(), "partial");
        assert_eq!(PayableStatus::Overdue.label(), "Vencida");
    }
}
