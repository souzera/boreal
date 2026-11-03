use serde::Deserialize;
use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::error::AppError;
use crate::models::settings::{AppSettings, SessionPolicy, SettingsView};
use crate::repositories::settings_repo;

#[derive(Debug, Deserialize)]
pub struct UpdateSettings {
    pub settings: AppSettings,
    pub expected_version: i64,
}

#[derive(Clone)]
pub struct SettingsService {
    ctx: Ctx,
}

impl SettingsService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    /// Qualquer usuário autenticado pode consultar as configurações.
    pub fn get(&self, token: &str) -> Result<SettingsView, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        Ok(SettingsView {
            settings: settings_repo::load(&conn)?,
            version: settings_repo::version(&conn)?,
        })
    }

    /// Política de sessão para a interface (tempo de inatividade).
    pub fn session_policy(&self, token: &str) -> Result<SessionPolicy, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        Ok(SessionPolicy {
            idle_lock_minutes: settings_repo::get_u32(&conn, "idle_lock_minutes")?,
        })
    }

    /// Grava as configurações (exige `ManageSettings`) e devolve a nova versão.
    pub fn update(&self, token: &str, input: UpdateSettings) -> Result<i64, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        input.settings.validate()?;

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        if settings_repo::version(&tx)? != input.expected_version {
            return Err(AppError::Conflict);
        }
        let before = settings_repo::load(&tx)?;
        settings_repo::save(&tx, &input.settings)?;
        let version = settings_repo::bump_version(&tx)?;
        audit::record(
            &tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "settings",
                operation: "update_settings",
                entity: "settings",
                entity_id: None,
                before: Some(json!(before)),
                after: Some(json!(input.settings)),
                justification: None,
            },
        )?;
        tx.commit()?;
        self.ctx.sessions.set_idle_lock(std::time::Duration::from_secs(
            u64::from(input.settings.idle_lock_minutes) * 60,
        ));
        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::UpdateSettings;
    use crate::error::AppError;
    use crate::models::settings::AppSettings;
    use crate::services::test_support::{admin_token, harness, recepcao_token, Harness};
    use std::time::Duration;

    type Mutation = Box<dyn Fn(&mut AppSettings)>;

    fn current(h: &Harness, token: &str) -> (AppSettings, i64) {
        let view = h.settings.get(token).unwrap();
        (view.settings, view.version)
    }

    fn update(h: &Harness, token: &str, settings: AppSettings, version: i64) -> Result<i64, AppError> {
        h.settings.update(
            token,
            UpdateSettings {
                settings,
                expected_version: version,
            },
        )
    }

    #[test]
    fn defaults_are_the_provisional_values() {
        let h = harness();
        let admin = admin_token(&h);

        let (s, version) = current(&h, &admin);

        assert_eq!(s.max_failed_attempts, 5);
        assert_eq!(s.lockout_minutes, 5);
        assert_eq!(s.idle_lock_minutes, 10);
        assert_eq!(s.default_check_in_time, "14:00");
        assert_eq!(s.default_check_out_time, "12:00");
        assert_eq!(s.late_checkout_tolerance_minutes, 30);
        assert_eq!(s.no_show_tolerance_minutes, 60);
        assert_eq!(s.cleaning_interval_minutes, 60);
        assert_eq!(s.deposit_percent_bp, 3000);
        assert_eq!(s.deposit_due_days_after_booking, 3);
        assert_eq!(s.deposit_due_days_before_arrival, 1);
        assert_eq!(s.late_checkout_fee_bp, 5000);
        assert_eq!(version, 1);
    }

    #[test]
    fn update_applies_idle_lock_immediately() {
        let h = harness();
        let admin = admin_token(&h);
        let (mut s, v) = current(&h, &admin);
        s.idle_lock_minutes = 1;

        update(&h, &admin, s, v).unwrap();
        h.clock.advance(Duration::from_secs(61));

        assert!(matches!(h.auth.session_status(&admin), Err(AppError::Locked)));
    }

    #[test]
    fn rejects_invalid_values() {
        let h = harness();
        let admin = admin_token(&h);
        let (base, v) = current(&h, &admin);

        let cases: Vec<Mutation> = vec![
            Box::new(|s| s.idle_lock_minutes = 0),
            Box::new(|s| s.max_failed_attempts = 0),
            Box::new(|s| s.lockout_minutes = 0),
            Box::new(|s| s.default_check_in_time = "25:00".into()),
            Box::new(|s| s.default_check_out_time = "9:7".into()),
            Box::new(|s| s.late_checkout_tolerance_minutes = 601),
            Box::new(|s| s.cleaning_interval_minutes = 601),
            Box::new(|s| s.deposit_percent_bp = 10_001),
            Box::new(|s| s.deposit_due_days_after_booking = 61),
            Box::new(|s| s.deposit_due_days_before_arrival = 61),
            Box::new(|s| s.late_checkout_fee_bp = 10_001),
        ];
        for mutate in cases {
            let mut s = base.clone();
            mutate(&mut s);
            assert!(
                matches!(update(&h, &admin, s, v), Err(AppError::Validation(_))),
                "valor inválido aceito"
            );
        }
    }

    #[test]
    fn update_requires_manage_settings() {
        let h = harness();
        let admin = admin_token(&h);
        let rec = recepcao_token(&h, &admin);
        let (s, v) = current(&h, &rec);

        assert!(matches!(update(&h, &rec, s, v), Err(AppError::Forbidden)));
    }

    #[test]
    fn update_is_audited_with_before_and_after() {
        let h = harness();
        let admin = admin_token(&h);
        let (mut s, v) = current(&h, &admin);
        s.idle_lock_minutes = 15;

        let new_version = update(&h, &admin, s, v).unwrap();

        assert_eq!(new_version, v + 1);
        let conn = h.db.lock().unwrap();
        let (before, after): (String, String) = conn
            .query_row(
                "SELECT before_json, after_json FROM audit_log WHERE operation = 'update_settings'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(before.contains("\"idle_lock_minutes\":10"));
        assert!(after.contains("\"idle_lock_minutes\":15"));
    }

    #[test]
    fn stale_version_conflicts() {
        let h = harness();
        let admin = admin_token(&h);
        let (s, v) = current(&h, &admin);
        update(&h, &admin, s.clone(), v).unwrap();

        assert!(matches!(update(&h, &admin, s, v), Err(AppError::Conflict)));
    }

    #[test]
    fn session_policy_is_available_to_recepcao() {
        let h = harness();
        let admin = admin_token(&h);
        let rec = recepcao_token(&h, &admin);

        assert_eq!(h.settings.session_policy(&rec).unwrap().idle_lock_minutes, 10);
        assert!(h.settings.get(&rec).is_ok());
    }
}
