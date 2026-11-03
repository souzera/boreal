//! Estado operacional do quarto, calculado (função pura, sem banco).
//!
//! O estado nunca é gravado e o decurso do tempo nunca conclui uma limpeza nem libera um bloqueio:
//! só o intervalo mínimo *depois* de uma limpeza confirmada passa sozinho (spec §7).

use chrono::{Duration, NaiveDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleaningStatus {
    Awaiting,
    InProgress,
    Done,
}

impl CleaningStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CleaningStatus::Awaiting => "awaiting",
            CleaningStatus::InProgress => "in_progress",
            CleaningStatus::Done => "done",
        }
    }

    pub fn parse(value: &str) -> Option<CleaningStatus> {
        match value {
            "awaiting" => Some(CleaningStatus::Awaiting),
            "in_progress" => Some(CleaningStatus::InProgress),
            "done" => Some(CleaningStatus::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleaningFact {
    pub status: CleaningStatus,
    pub completed_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Copy)]
pub struct RoomFacts {
    pub in_house: bool,
    /// Tarefa de limpeza mais recente do quarto (aberta, ou a última concluída).
    pub cleaning: Option<CleaningFact>,
    pub blocked_now: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomState {
    Occupied,
    AwaitingCleaning,
    Cleaning,
    CleanWaitingInterval { free_at: NaiveDateTime },
    Available,
    Blocked,
}

impl RoomState {
    pub fn as_str(self) -> &'static str {
        match self {
            RoomState::Occupied => "occupied",
            RoomState::AwaitingCleaning => "awaiting_cleaning",
            RoomState::Cleaning => "cleaning",
            RoomState::CleanWaitingInterval { .. } => "clean_waiting_interval",
            RoomState::Available => "available",
            RoomState::Blocked => "blocked",
        }
    }

    pub fn label_pt(self) -> &'static str {
        match self {
            RoomState::Occupied => "Ocupado",
            RoomState::AwaitingCleaning => "Aguardando limpeza",
            RoomState::Cleaning => "Em limpeza",
            RoomState::CleanWaitingInterval { .. } => "Limpo — aguardando intervalo",
            RoomState::Available => "Disponível",
            RoomState::Blocked => "Em manutenção ou bloqueado",
        }
    }
}

pub fn room_state(facts: &RoomFacts, interval_minutes: u32, now: NaiveDateTime) -> RoomState {
    if facts.in_house {
        return RoomState::Occupied;
    }
    if facts.blocked_now {
        return RoomState::Blocked;
    }
    match facts.cleaning {
        Some(CleaningFact { status: CleaningStatus::Awaiting, .. }) => RoomState::AwaitingCleaning,
        Some(CleaningFact { status: CleaningStatus::InProgress, .. }) => RoomState::Cleaning,
        Some(CleaningFact { status: CleaningStatus::Done, completed_at: Some(done) }) => {
            let free_at = done + Duration::minutes(i64::from(interval_minutes));
            if free_at > now {
                RoomState::CleanWaitingInterval { free_at }
            } else {
                RoomState::Available
            }
        }
        _ => RoomState::Available,
    }
}

#[cfg(test)]
mod tests {
    use super::{room_state, CleaningFact, CleaningStatus, RoomFacts, RoomState};
    use chrono::NaiveDateTime;

    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn facts(in_house: bool, status: Option<CleaningStatus>, done: Option<&str>, blocked_now: bool) -> RoomFacts {
        RoomFacts {
            in_house,
            cleaning: status.map(|status| CleaningFact { status, completed_at: done.map(at) }),
            blocked_now,
        }
    }

    #[test]
    fn occupied_wins_over_everything_else() {
        let f = facts(true, Some(CleaningStatus::Awaiting), None, true);
        assert_eq!(room_state(&f, 60, at("2026-11-01 10:00")), RoomState::Occupied);
    }

    #[test]
    fn blocked_wins_over_cleaning() {
        let f = facts(false, Some(CleaningStatus::Awaiting), None, true);
        assert_eq!(room_state(&f, 60, at("2026-11-01 10:00")), RoomState::Blocked);
    }

    #[test]
    fn awaiting_and_in_progress_map_to_their_states() {
        let now = at("2026-11-01 10:00");
        assert_eq!(room_state(&facts(false, Some(CleaningStatus::Awaiting), None, false), 60, now), RoomState::AwaitingCleaning);
        assert_eq!(room_state(&facts(false, Some(CleaningStatus::InProgress), None, false), 60, now), RoomState::Cleaning);
    }

    #[test]
    fn done_cleaning_waits_for_the_interval() {
        let f = facts(false, Some(CleaningStatus::Done), Some("2026-11-01 10:00"), false);
        assert_eq!(room_state(&f, 60, at("2026-11-01 10:59")), RoomState::CleanWaitingInterval { free_at: at("2026-11-01 11:00") });
        assert_eq!(room_state(&f, 60, at("2026-11-01 11:00")), RoomState::Available);
    }

    #[test]
    fn no_cleaning_history_is_available() {
        assert_eq!(room_state(&facts(false, None, None, false), 60, at("2026-11-01 10:00")), RoomState::Available);
    }

    #[test]
    fn time_alone_never_clears_awaiting_cleaning() {
        let f = facts(false, Some(CleaningStatus::Awaiting), None, false);
        assert_eq!(room_state(&f, 60, at("2026-12-01 10:00")), RoomState::AwaitingCleaning);
    }

    #[test]
    fn interval_zero_is_available_right_after_cleaning() {
        let f = facts(false, Some(CleaningStatus::Done), Some("2026-11-01 10:00"), false);
        assert_eq!(room_state(&f, 0, at("2026-11-01 10:00")), RoomState::Available);
    }

    #[test]
    fn labels_are_distinct() {
        let all = [
            RoomState::Occupied,
            RoomState::AwaitingCleaning,
            RoomState::Cleaning,
            RoomState::CleanWaitingInterval { free_at: at("2026-11-01 10:00") },
            RoomState::Available,
            RoomState::Blocked,
        ];
        let labels: std::collections::HashSet<_> = all.iter().map(|s| s.label_pt()).collect();
        assert_eq!(labels.len(), 6);
    }
}
