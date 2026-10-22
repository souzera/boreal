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
    ManageCatalog,
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
        Permission::ManageCatalog,
    ];

    /// Permissões que o administrador pode conceder individualmente à recepção.
    pub fn grantable() -> &'static [Permission] {
        &[
            Permission::ApplyDiscount,
            Permission::ExceedCapacity,
            Permission::ReverseEntry,
            Permission::CancellationException,
            Permission::ChangeReservation,
            Permission::ReleaseRoom,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Permission::ManageUsers => "manage_users",
            Permission::ViewFinancialReports => "view_financial_reports",
            Permission::ApplyDiscount => "apply_discount",
            Permission::ExceedCapacity => "exceed_capacity",
            Permission::ReverseEntry => "reverse_entry",
            Permission::CancellationException => "cancellation_exception",
            Permission::ChangeReservation => "change_reservation",
            Permission::ReleaseRoom => "release_room",
            Permission::RestoreBackup => "restore_backup",
            Permission::ManageSettings => "manage_settings",
            Permission::ManageCatalog => "manage_catalog",
        }
    }

    pub fn from_str(value: &str) -> Option<Permission> {
        Permission::ALL.iter().copied().find(|p| p.as_str() == value)
    }
}

/// Conjunto de permissões em máscara de bits (cabe em `Copy`, vai dentro da sessão).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PermissionSet(u32);

impl PermissionSet {
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn with(self, permission: Permission) -> Self {
        Self(self.0 | (1 << permission as u32))
    }

    pub fn contains(self, permission: Permission) -> bool {
        self.0 & (1 << permission as u32) != 0
    }
}

/// Administrador tem todas; recepção não tem nenhuma das sensíveis por padrão.
pub fn role_has(role: Role, _permission: Permission) -> bool {
    matches!(role, Role::Admin)
}

#[cfg(test)]
mod tests {
    use super::{role_has, Permission, PermissionSet};
    use crate::models::user::Role;

    #[test]
    fn grantable_excludes_admin_only_permissions() {
        let grantable = Permission::grantable();

        for p in [
            Permission::ManageUsers,
            Permission::ManageSettings,
            Permission::ManageCatalog,
            Permission::RestoreBackup,
            Permission::ViewFinancialReports,
        ] {
            assert!(!grantable.contains(&p), "{p:?} não pode ser concedida");
        }
        assert!(grantable.contains(&Permission::ApplyDiscount));
    }

    #[test]
    fn permission_set_roundtrip() {
        let set = PermissionSet::empty()
            .with(Permission::ApplyDiscount)
            .with(Permission::ReleaseRoom);

        assert!(set.contains(Permission::ApplyDiscount));
        assert!(set.contains(Permission::ReleaseRoom));
        assert!(!set.contains(Permission::ReverseEntry));
        for p in Permission::ALL {
            assert_eq!(Permission::from_str(p.as_str()), Some(*p));
        }
        assert_eq!(Permission::from_str("inexistente"), None);
    }

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
