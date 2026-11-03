//! Estados da hospedagem (função pura, sem banco).

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StayStatus {
    InHouse,
    CheckedOut,
}

impl StayStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            StayStatus::InHouse => "in_house",
            StayStatus::CheckedOut => "checked_out",
        }
    }

    pub fn parse(value: &str) -> Option<StayStatus> {
        match value {
            "in_house" => Some(StayStatus::InHouse),
            "checked_out" => Some(StayStatus::CheckedOut),
            _ => None,
        }
    }
}

/// Única transição válida: hospedada → encerrada.
pub fn transition(from: StayStatus, to: StayStatus) -> Result<(), AppError> {
    match (from, to) {
        (StayStatus::InHouse, StayStatus::CheckedOut) => Ok(()),
        (StayStatus::CheckedOut, _) => Err(AppError::Validation("A hospedagem já foi encerrada.".into())),
        _ => Err(AppError::Validation("Mudança de situação da hospedagem não permitida.".into())),
    }
}

/// Só reservas pendentes ou confirmadas podem receber check-in.
pub fn can_check_in(reservation_status: &str) -> bool {
    matches!(reservation_status, "pending" | "confirmed")
}

#[cfg(test)]
mod tests {
    use super::{can_check_in, transition, StayStatus};
    use crate::error::AppError;

    #[test]
    fn in_house_to_checked_out_is_valid() {
        assert!(transition(StayStatus::InHouse, StayStatus::CheckedOut).is_ok());
    }

    #[test]
    fn checked_out_cannot_go_back() {
        assert!(matches!(transition(StayStatus::CheckedOut, StayStatus::InHouse), Err(AppError::Validation(_))));
    }

    #[test]
    fn same_state_transition_is_refused() {
        assert!(transition(StayStatus::InHouse, StayStatus::InHouse).is_err());
        assert!(transition(StayStatus::CheckedOut, StayStatus::CheckedOut).is_err());
    }

    #[test]
    fn status_text_roundtrip() {
        for s in [StayStatus::InHouse, StayStatus::CheckedOut] {
            assert_eq!(StayStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(StayStatus::parse("x"), None);
    }

    #[test]
    fn only_pending_and_confirmed_can_check_in() {
        assert!(can_check_in("pending") && can_check_in("confirmed"));
        for s in ["checked_in", "completed", "cancelled", "no_show"] {
            assert!(!can_check_in(s), "{s}");
        }
    }
}
