use crate::models::user::Role;

/// Operações sensíveis que exigem permissão específica (spec §13.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ManageUsers,
    ViewFinancialReports,
    ApplyDiscount,
    ExceedCapacity,
    ReverseEntry,
    CancellationException,
    ChangeReservation,
    ReleaseRoom,
    RestoreBackup,
    ManageSettings,
}

impl Permission {
    pub const ALL: &'static [Permission] = &[
        Permission::ManageUsers,
        Permission::ViewFinancialReports,
        Permission::ApplyDiscount,
        Permission::ExceedCapacity,
        Permission::ReverseEntry,
        Permission::CancellationException,
        Permission::ChangeReservation,
        Permission::ReleaseRoom,
        Permission::RestoreBackup,
        Permission::ManageSettings,
    ];
}

/// Administrador tem todas; recepção não tem nenhuma das sensíveis por padrão.
pub fn role_has(role: Role, _permission: Permission) -> bool {
    matches!(role, Role::Admin)
}

#[cfg(test)]
mod tests {
    use super::{role_has, Permission};
    use crate::models::user::Role;

    #[test]
    fn admin_has_all() {
        for p in Permission::ALL {
            assert!(role_has(Role::Admin, *p), "admin sem {p:?}");
        }
    }

    #[test]
    fn recepcao_lacks_sensitive_permissions() {
        for p in Permission::ALL {
            assert!(!role_has(Role::Recepcao, *p), "recepcao com {p:?}");
        }
    }
}
