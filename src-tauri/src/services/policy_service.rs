use serde::Serialize;
use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::cancellation::{validate_tiers, Tier};
use crate::error::AppError;
use crate::repositories::policy_repo;

#[derive(Debug, Serialize)]
pub struct PolicyView {
    /// Da maior para a menor antecedência.
    pub tiers: Vec<Tier>,
    pub version: i64,
}

/// Política de cancelamento (valores provisórios até a aprovação dos responsáveis).
#[derive(Clone)]
pub struct PolicyService {
    ctx: Ctx,
}

impl PolicyService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    pub fn get(&self, token: &str) -> Result<PolicyView, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        Ok(PolicyView { tiers: policy_repo::list_tiers(&conn)?, version: policy_repo::version(&conn)? })
    }

    /// Substitui as faixas (exige `ManageSettings`) e devolve a nova versão.
    pub fn replace(&self, token: &str, tiers: Vec<Tier>, expected_version: i64) -> Result<i64, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        validate_tiers(&tiers)?;

        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        if policy_repo::version(&tx)? != expected_version {
            return Err(AppError::Conflict);
        }
        let before = policy_repo::list_tiers(&tx)?;
        let version = policy_repo::replace_tiers(&tx, &tiers)?;
        audit::record(
            &tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "settings",
                operation: "update_cancellation_policy",
                entity: "cancellation_policy",
                entity_id: None,
                before: Some(json!(before)),
                after: Some(json!(tiers)),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::cancellation::Tier;
    use crate::error::AppError;
    use crate::services::test_support::{admin_token, harness, recepcao_token};

    fn tier(min_days_before: u32, refund_bp: u32) -> Tier {
        Tier { min_days_before, refund_bp }
    }

    #[test]
    fn default_policy_is_the_provisional_one_sorted_by_notice() {
        let h = harness();
        let admin = admin_token(&h);

        let view = h.policy.get(&admin).unwrap();

        assert_eq!(view.tiers, [tier(7, 10_000), tier(3, 5000), tier(0, 0)]);
        assert_eq!(view.version, 1);
    }

    #[test]
    fn replacing_the_policy_bumps_the_version_and_is_audited() {
        let h = harness();
        let admin = admin_token(&h);

        let version = h.policy.replace(&admin, vec![tier(0, 0), tier(15, 10_000), tier(5, 2500)], 1).unwrap();

        assert_eq!(version, 2);
        assert_eq!(h.policy.get(&admin).unwrap().tiers, [tier(15, 10_000), tier(5, 2500), tier(0, 0)]);
        let (before, after): (String, String) = h
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT before_json, after_json FROM audit_log WHERE operation = 'update_cancellation_policy'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(before.contains("10000") && after.contains("2500"));
    }

    #[test]
    fn invalid_policies_are_rejected_and_nothing_changes() {
        let h = harness();
        let admin = admin_token(&h);
        let invalid = vec![
            vec![],
            vec![tier(3, 5000)],
            vec![tier(0, 0), tier(0, 100)],
            vec![tier(0, 10_001)],
            vec![tier(7, 0), tier(0, 5000)],
        ];

        for tiers in invalid {
            assert!(matches!(h.policy.replace(&admin, tiers, 1), Err(AppError::Validation(_))));
        }
        assert_eq!(h.policy.get(&admin).unwrap().version, 1);
    }

    #[test]
    fn recepcao_reads_but_cannot_change() {
        let h = harness();
        let admin = admin_token(&h);
        let rec = recepcao_token(&h, &admin);

        assert!(h.policy.get(&rec).is_ok());
        assert!(matches!(h.policy.replace(&rec, vec![tier(0, 0)], 1), Err(AppError::Forbidden)));
    }

    #[test]
    fn stale_version_conflicts() {
        let h = harness();
        let admin = admin_token(&h);
        h.policy.replace(&admin, vec![tier(0, 0)], 1).unwrap();

        assert!(matches!(h.policy.replace(&admin, vec![tier(0, 100)], 1), Err(AppError::Conflict)));
    }
}
