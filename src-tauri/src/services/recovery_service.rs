//! Chave de recuperação e, mais adiante, recuperação em outro computador (spec §14, §16.2).

use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::password::verify_password;
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::recovery_key;
use crate::error::AppError;
use crate::repositories::user_repo;

#[derive(Clone)]
pub struct RecoveryService {
    ctx: Ctx,
}

impl RecoveryService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    /// Mostra a chave de recuperação ao administrador, que precisa digitar a própria senha de novo.
    /// Cada exibição é auditada; a chave nunca vai para a auditoria nem para erros.
    pub fn reveal(&self, token: &str, password: &str) -> Result<String, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        let mut conn = self.ctx.conn()?;
        let (_, hash) = user_repo::find_with_hash(&conn, actor.user_id)?.ok_or(AppError::Unauthorized)?;
        if !verify_password(password, &hash) {
            return Err(AppError::Validation("A senha está incorreta.".into()));
        }
        let tx = conn.transaction()?;
        audit::record(
            &tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "settings",
                operation: "reveal_recovery_key",
                entity: "recovery_key",
                entity_id: None,
                before: None,
                after: Some(json!({ "shown": true })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(recovery_key::format(self.ctx.key.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recovery_key;
    use crate::services::test_support::{admin_token, harness, recepcao_token, ADMIN_PASSWORD, TEST_KEY};

    #[test]
    fn the_administrator_with_the_right_password_gets_the_key_and_it_is_audited_without_the_key() {
        let h = harness();
        let admin = admin_token(&h);

        let shown = h.recovery.reveal(&admin, ADMIN_PASSWORD).unwrap();

        assert_eq!(recovery_key::parse(&shown).unwrap(), TEST_KEY);
        let conn = h.db.lock().unwrap();
        let (count, after): (i64, String) = conn
            .query_row("SELECT count(*), COALESCE(MAX(after_json), '') FROM audit_log WHERE operation = 'reveal_recovery_key'", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(count, 1);
        assert!(!after.contains(&shown) && !after.to_uppercase().contains("2A2A2A2A"), "a auditoria não guarda a chave: {after}");
    }

    #[test]
    fn a_wrong_password_shows_nothing_and_audits_nothing() {
        let h = harness();
        let admin = admin_token(&h);

        let result = h.recovery.reveal(&admin, "senha-errada");

        assert!(matches!(&result, Err(AppError::Validation(m)) if m.contains("senha está incorreta")), "{result:?}");
        let conn = h.db.lock().unwrap();
        let count: i64 = conn.query_row("SELECT count(*) FROM audit_log WHERE operation = 'reveal_recovery_key'", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn reception_is_forbidden_and_a_missing_session_is_refused() {
        let h = harness();
        let admin = admin_token(&h);
        let reception = recepcao_token(&h, &admin);

        assert!(matches!(h.recovery.reveal(&reception, "qualquer"), Err(AppError::Forbidden)));
        assert!(h.recovery.reveal("invalido", ADMIN_PASSWORD).is_err());
    }
}
