//! Estados da ordem de manutenção (função pura, sem banco).

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Open,
    InProgress,
    AwaitingResolution,
    Completed,
}

impl OrderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OrderStatus::Open => "open",
            OrderStatus::InProgress => "in_progress",
            OrderStatus::AwaitingResolution => "awaiting_resolution",
            OrderStatus::Completed => "completed",
        }
    }

    pub fn parse(value: &str) -> Option<OrderStatus> {
        match value {
            "open" => Some(OrderStatus::Open),
            "in_progress" => Some(OrderStatus::InProgress),
            "awaiting_resolution" => Some(OrderStatus::AwaitingResolution),
            "completed" => Some(OrderStatus::Completed),
            _ => None,
        }
    }
}

/// Aberta → em andamento → aguardando resolução → concluída. Nada pula etapa nem volta.
pub fn transition(from: OrderStatus, to: OrderStatus) -> Result<(), AppError> {
    use OrderStatus::*;
    match (from, to) {
        (Open, InProgress) | (InProgress, AwaitingResolution) | (AwaitingResolution, Completed) => Ok(()),
        (Completed, _) => Err(AppError::Validation("A ordem de manutenção já foi concluída.".into())),
        _ => Err(AppError::Validation("Essa mudança de situação não é permitida para esta ordem de manutenção.".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::{transition, OrderStatus::*};

    #[test]
    fn only_the_forward_steps_are_allowed() {
        assert!(transition(Open, InProgress).is_ok());
        assert!(transition(InProgress, AwaitingResolution).is_ok());
        assert!(transition(AwaitingResolution, Completed).is_ok());
    }

    #[test]
    fn skipping_or_going_back_is_refused() {
        assert!(transition(Open, Completed).is_err());
        assert!(transition(Open, AwaitingResolution).is_err());
        assert!(transition(InProgress, Open).is_err());
        assert!(transition(Completed, InProgress).is_err());
        assert!(transition(Open, Open).is_err());
    }
}
