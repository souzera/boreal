//! Tipos, prioridades e regras de dispensa dos alertas (funções puras).

use crate::auth::permissions::Permission;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Critical,
    High,
    Normal,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Normal => "normal",
        }
    }

    pub fn parse(value: &str) -> Option<Severity> {
        [Severity::Critical, Severity::High, Severity::Normal].into_iter().find(|s| s.as_str() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            Severity::Critical => "Crítico",
            Severity::High => "Alto",
            Severity::Normal => "Normal",
        }
    }

    /// Menor número = mais urgente.
    pub fn rank(self) -> u8 {
        match self {
            Severity::Critical => 0,
            Severity::High => 1,
            Severity::Normal => 2,
        }
    }
}

/// Tipos de alerta (spec §20.2). Os três últimos são emitidos pelo Marco 7 (backup, disco e atualização).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertKind {
    ArrivalToday,
    DepartureToday,
    ArrivalOverdue,
    DepositOverdue,
    DepartureLate,
    DepartureRisk,
    MaintenanceConflict,
    CleaningUrgent,
    PayableOverdue,
    CheckoutBalance,
    ClosingPending,
    BackupMissing,
    DiskLow,
    UpdateFailed,
}

impl AlertKind {
    pub const ALL: [AlertKind; 14] = [
        AlertKind::ArrivalToday,
        AlertKind::DepartureToday,
        AlertKind::ArrivalOverdue,
        AlertKind::DepositOverdue,
        AlertKind::DepartureLate,
        AlertKind::DepartureRisk,
        AlertKind::MaintenanceConflict,
        AlertKind::CleaningUrgent,
        AlertKind::PayableOverdue,
        AlertKind::CheckoutBalance,
        AlertKind::ClosingPending,
        AlertKind::BackupMissing,
        AlertKind::DiskLow,
        AlertKind::UpdateFailed,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AlertKind::ArrivalToday => "arrival_today",
            AlertKind::DepartureToday => "departure_today",
            AlertKind::ArrivalOverdue => "arrival_overdue",
            AlertKind::DepositOverdue => "deposit_overdue",
            AlertKind::DepartureLate => "departure_late",
            AlertKind::DepartureRisk => "departure_risk",
            AlertKind::MaintenanceConflict => "maintenance_conflict",
            AlertKind::CleaningUrgent => "cleaning_urgent",
            AlertKind::PayableOverdue => "payable_overdue",
            AlertKind::CheckoutBalance => "checkout_balance",
            AlertKind::ClosingPending => "closing_pending",
            AlertKind::BackupMissing => "backup_missing",
            AlertKind::DiskLow => "disk_low",
            AlertKind::UpdateFailed => "update_failed",
        }
    }

    pub fn parse(value: &str) -> Option<AlertKind> {
        AlertKind::ALL.into_iter().find(|k| k.as_str() == value)
    }

    pub fn severity(self) -> Severity {
        match self {
            AlertKind::DepartureRisk
            | AlertKind::MaintenanceConflict
            | AlertKind::BackupMissing
            | AlertKind::DiskLow
            | AlertKind::UpdateFailed => Severity::Critical,
            AlertKind::ArrivalOverdue
            | AlertKind::DepositOverdue
            | AlertKind::DepartureLate
            | AlertKind::CleaningUrgent
            | AlertKind::PayableOverdue
            | AlertKind::CheckoutBalance => Severity::High,
            AlertKind::ArrivalToday | AlertKind::DepartureToday | AlertKind::ClosingPending => Severity::Normal,
        }
    }

    /// Rótulo curto do tipo, em linguagem da pousada.
    pub fn label(self) -> &'static str {
        match self {
            AlertKind::ArrivalToday => "Chegada hoje",
            AlertKind::DepartureToday => "Saída hoje",
            AlertKind::ArrivalOverdue => "Chegada atrasada",
            AlertKind::DepositOverdue => "Sinal vencido",
            AlertKind::DepartureLate => "Saída atrasada",
            AlertKind::DepartureRisk => "Risco para a próxima reserva",
            AlertKind::MaintenanceConflict => "Reserva afetada por manutenção",
            AlertKind::CleaningUrgent => "Limpeza urgente",
            AlertKind::PayableOverdue => "Conta vencida",
            AlertKind::CheckoutBalance => "Saldo pendente na saída",
            AlertKind::ClosingPending => "Fechamento de caixa pendente",
            AlertKind::BackupMissing => "Sem backup válido",
            AlertKind::DiskLow => "Pouco espaço em disco",
            AlertKind::UpdateFailed => "Falha na atualização",
        }
    }

    /// Permissão exigida para enxergar alertas deste tipo (quem não a tem nem os vê nem os conta).
    pub fn requires_permission(self) -> Option<Permission> {
        match self {
            AlertKind::PayableOverdue => Some(Permission::ManagePayables),
            // Backup, disco e atualização são assunto do administrador: a recepção não consegue resolvê-los.
            k if k.is_system() => Some(Permission::ManageSettings),
            _ => None,
        }
    }

    /// Alertas de saúde do sistema (backup, disco e atualização).
    pub fn is_system(self) -> bool {
        matches!(self, AlertKind::BackupMissing | AlertKind::DiskLow | AlertKind::UpdateFailed)
    }

    /// Todos os tipos têm detector; os de sistema só rodam quando o monitoramento está ligado.
    pub fn managed_by_detectors(self) -> bool {
        true
    }
}

/// Um problema encontrado por um detector; o motor o transforma (ou atualiza) em alerta persistente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: AlertKind,
    /// Identifica a causa dentro do tipo (por exemplo `reservation:12`).
    pub subject_key: String,
    pub title: String,
    pub detail: String,
    /// Rota da tela do registro relacionado.
    pub link: String,
}

/// Alertas críticos só se resolvem quando o problema é resolvido (spec §20.2).
pub fn can_dismiss(severity: Severity) -> bool {
    severity != Severity::Critical
}

#[cfg(test)]
mod tests {
    use super::{can_dismiss, AlertKind, Severity};
    use crate::auth::permissions::Permission;

    const ALL: [AlertKind; 14] = [
        AlertKind::ArrivalToday,
        AlertKind::DepartureToday,
        AlertKind::ArrivalOverdue,
        AlertKind::DepositOverdue,
        AlertKind::DepartureLate,
        AlertKind::DepartureRisk,
        AlertKind::MaintenanceConflict,
        AlertKind::CleaningUrgent,
        AlertKind::PayableOverdue,
        AlertKind::CheckoutBalance,
        AlertKind::ClosingPending,
        AlertKind::BackupMissing,
        AlertKind::DiskLow,
        AlertKind::UpdateFailed,
    ];

    #[test]
    fn critical_kinds_cannot_be_dismissed() {
        for kind in [AlertKind::DepartureRisk, AlertKind::MaintenanceConflict, AlertKind::BackupMissing, AlertKind::DiskLow, AlertKind::UpdateFailed] {
            assert_eq!(kind.severity(), Severity::Critical, "{kind:?}");
            assert!(!can_dismiss(kind.severity()), "{kind:?}");
        }
    }

    #[test]
    fn other_kinds_can_be_dismissed() {
        for kind in ALL.into_iter().filter(|k| k.severity() != Severity::Critical) {
            assert!(can_dismiss(kind.severity()), "{kind:?}");
        }
    }

    #[test]
    fn severities_follow_the_plan() {
        assert_eq!(AlertKind::ArrivalToday.severity(), Severity::Normal);
        assert_eq!(AlertKind::DepartureToday.severity(), Severity::Normal);
        assert_eq!(AlertKind::ClosingPending.severity(), Severity::Normal);
        for kind in [AlertKind::ArrivalOverdue, AlertKind::DepositOverdue, AlertKind::DepartureLate, AlertKind::CleaningUrgent, AlertKind::PayableOverdue, AlertKind::CheckoutBalance] {
            assert_eq!(kind.severity(), Severity::High, "{kind:?}");
        }
    }

    #[test]
    fn kinds_roundtrip_through_str() {
        for kind in ALL {
            assert_eq!(AlertKind::parse(kind.as_str()), Some(kind));
            assert!(!kind.label().is_empty());
        }
        assert_eq!(AlertKind::DepartureRisk.as_str(), "departure_risk");
        assert_eq!(AlertKind::parse("inexistente"), None);
    }

    #[test]
    fn payables_need_their_permission_and_system_alerts_are_admin_only() {
        for kind in ALL {
            let expected = if kind == AlertKind::PayableOverdue {
                Some(Permission::ManagePayables)
            } else if kind.is_system() {
                Some(Permission::ManageSettings)
            } else {
                None
            };
            assert_eq!(kind.requires_permission(), expected, "{kind:?}");
        }
    }

    #[test]
    fn every_kind_has_a_detector_and_three_are_system_kinds() {
        for kind in ALL {
            let system = matches!(kind, AlertKind::BackupMissing | AlertKind::DiskLow | AlertKind::UpdateFailed);
            assert!(kind.managed_by_detectors(), "{kind:?}");
            assert_eq!(kind.is_system(), system, "{kind:?}");
        }
    }

    #[test]
    fn severity_ranks_critical_first() {
        assert!(Severity::Critical.rank() < Severity::High.rank());
        assert!(Severity::High.rank() < Severity::Normal.rank());
        assert_eq!(Severity::parse("high"), Some(Severity::High));
        assert_eq!(Severity::Critical.label(), "Crítico");
    }
}
