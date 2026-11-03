//! Prioridade sugerida da limpeza (função pura, sem banco).

use chrono::{Duration, NaiveDateTime};

/// Chegada a menos que isto do agora torna a limpeza urgente (valor provisório, Anexo A.4).
pub const URGENT_WITHIN_HOURS: i64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Urgent,
    High,
    Normal,
    Low,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Urgent => "urgent",
            Priority::High => "high",
            Priority::Normal => "normal",
            Priority::Low => "low",
        }
    }

    pub fn parse(value: &str) -> Option<Priority> {
        match value {
            "urgent" => Some(Priority::Urgent),
            "high" => Some(Priority::High),
            "normal" => Some(Priority::Normal),
            "low" => Some(Priority::Low),
            _ => None,
        }
    }

    /// Menor número = mais urgente (para ordenar a fila).
    pub fn rank(self) -> u8 {
        self as u8
    }
}

/// Prioridade pela proximidade da próxima chegada prevista no quarto.
pub fn suggest_priority(next_arrival: Option<NaiveDateTime>, now: NaiveDateTime) -> Priority {
    let Some(arrival) = next_arrival else {
        return Priority::Low;
    };
    if arrival <= now + Duration::hours(URGENT_WITHIN_HOURS) {
        Priority::Urgent
    } else if arrival.date() == now.date() {
        Priority::High
    } else {
        Priority::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::{suggest_priority, Priority};
    use chrono::NaiveDateTime;

    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    #[test]
    fn arrival_in_4h_is_urgent() {
        assert_eq!(suggest_priority(Some(at("2026-11-01 14:00")), at("2026-11-01 10:00")), Priority::Urgent);
        assert_eq!(suggest_priority(Some(at("2026-11-01 14:01")), at("2026-11-01 10:00")), Priority::High);
    }

    #[test]
    fn arrival_already_late_is_urgent() {
        assert_eq!(suggest_priority(Some(at("2026-11-01 14:00")), at("2026-11-01 16:00")), Priority::Urgent);
    }

    #[test]
    fn arrival_today_is_high() {
        assert_eq!(suggest_priority(Some(at("2026-11-01 20:00")), at("2026-11-01 08:00")), Priority::High);
    }

    #[test]
    fn arrival_tomorrow_across_midnight_is_normal() {
        assert_eq!(suggest_priority(Some(at("2026-11-02 00:30")), at("2026-11-01 23:30")), Priority::Urgent);
        assert_eq!(suggest_priority(Some(at("2026-11-02 14:00")), at("2026-11-01 23:30")), Priority::Normal);
    }

    #[test]
    fn no_arrival_is_low() {
        assert_eq!(suggest_priority(None, at("2026-11-01 10:00")), Priority::Low);
    }

    #[test]
    fn parse_round_trips() {
        for p in [Priority::Urgent, Priority::High, Priority::Normal, Priority::Low] {
            assert_eq!(Priority::parse(p.as_str()), Some(p));
        }
        assert_eq!(Priority::parse("x"), None);
        assert!(Priority::Urgent.rank() < Priority::Low.rank());
    }
}
