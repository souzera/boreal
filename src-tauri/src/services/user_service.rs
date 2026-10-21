use serde::Deserialize;
use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::password::{hash_password, validate_password_policy};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::error::AppError;
use crate::models::user::{NewUser, Role};
use crate::repositories::user_repo;

#[derive(Debug, Deserialize)]
pub struct NewUserInput {
    pub username: String,
    pub display_name: String,
    pub role: Role,
    pub initial_password: String,
}

#[derive(Clone)]
pub struct UserService {
    ctx: Ctx,
}

impl UserService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn authorize(&self, token: &str) -> Result<SessionInfo, AppError> {
        let info = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&info, Permission::ManageUsers)?;
        Ok(info)
    }

    /// Cria o usuário com senha temporária: a troca é exigida no primeiro acesso.
    pub fn create_user(&self, token: &str, input: NewUserInput) -> Result<i64, AppError> {
        let actor = self.authorize(token)?;
        let username = input.username.trim();
        let display_name = input.display_name.trim();
        if username.is_empty() || display_name.is_empty() {
            return Err(AppError::Validation(
                "Informe o usuário e o nome completo.".into(),
            ));
        }
        validate_password_policy(&input.initial_password)?;
        let hash = hash_password(&input.initial_password)?;

        let mut conn = self.ctx.db.lock().unwrap();
        let tx = conn.transaction()?;
        let id = user_repo::insert(
            &tx,
            &NewUser {
                username: username.into(),
                display_name: display_name.into(),
                password_hash: hash,
                role: input.role,
                must_change_password: true,
            },
        )
        .map_err(|e| match e {
            AppError::Database(msg) if msg.contains("UNIQUE") => {
                AppError::Validation("Já existe um usuário com este nome.".into())
            }
            other => other,
        })?;
        audit::record(
            &tx,
            &AuditEntry {
                user_id: Some(actor.user_id),
                module: "users",
                operation: "create_user",
                entity: "user",
                entity_id: Some(id),
                before: None,
                after: Some(json!({ "username": username, "role": input.role.as_str() })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(id)
    }

    /// Define senha temporária, exige troca no próximo acesso e encerra bloqueios da conta.
    pub fn reset_password(
        &self,
        token: &str,
        user_id: i64,
        temp_password: &str,
    ) -> Result<(), AppError> {
        let actor = self.authorize(token)?;
        validate_password_policy(temp_password)?;
        let hash = hash_password(temp_password)?;

        let mut conn = self.ctx.db.lock().unwrap();
        let user = user_repo::find(&conn, user_id)?
            .ok_or_else(|| AppError::Validation("Usuário não encontrado.".into()))?;
        let tx = conn.transaction()?;
        user_repo::update_password(&tx, user.id, &hash, true, user.version)?;
        user_repo::reset_attempts(&tx, user.id)?;
        audit::record(
            &tx,
            &AuditEntry {
                user_id: Some(actor.user_id),
                module: "users",
                operation: "reset_password",
                entity: "user",
                entity_id: Some(user.id),
                before: None,
                after: None,
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_active(&self, token: &str, user_id: i64, active: bool) -> Result<(), AppError> {
        let actor = self.authorize(token)?;
        if !active && actor.user_id == user_id {
            return Err(AppError::Validation(
                "Você não pode desativar o próprio usuário.".into(),
            ));
        }
        let mut conn = self.ctx.db.lock().unwrap();
        let user = user_repo::find(&conn, user_id)?
            .ok_or_else(|| AppError::Validation("Usuário não encontrado.".into()))?;
        let tx = conn.transaction()?;
        user_repo::set_active(&tx, user.id, active)?;
        audit::record(
            &tx,
            &AuditEntry {
                user_id: Some(actor.user_id),
                module: "users",
                operation: "set_active",
                entity: "user",
                entity_id: Some(user.id),
                before: Some(json!({ "active": user.active })),
                after: Some(json!({ "active": active })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::NewUserInput;
    use crate::error::AppError;
    use crate::models::user::Role;
    use crate::services::test_support::{admin_token, harness, ADMIN_PASSWORD};

    fn input(name: &str) -> NewUserInput {
        NewUserInput {
            username: name.into(),
            display_name: "Maria Recepção".into(),
            role: Role::Recepcao,
            initial_password: "Temp-Senha-123".into(),
        }
    }

    #[test]
    fn must_change_flag_flow() {
        let h = harness();
        let admin = admin_token(&h);
        h.users.create_user(&admin, input("maria")).unwrap();

        let first = h.auth.login("maria", "Temp-Senha-123").unwrap();
        assert!(first.must_change_password);
        h.auth
            .change_own_password(&first.token, "Temp-Senha-123", "Definitiva-456")
            .unwrap();

        assert!(
            !h.auth
                .login("maria", "Definitiva-456")
                .unwrap()
                .must_change_password
        );
    }

    #[test]
    fn recepcao_cannot_create_or_reset_users() {
        let h = harness();
        let admin = admin_token(&h);
        let maria_id = h.users.create_user(&admin, input("maria")).unwrap();
        let maria = h.auth.login("maria", "Temp-Senha-123").unwrap().token;

        assert!(matches!(
            h.users.create_user(&maria, input("joao")),
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            h.users.reset_password(&maria, maria_id, "Outra-Senha-789"),
            Err(AppError::Forbidden)
        ));
    }

    #[test]
    fn duplicate_username_is_a_validation_error() {
        let h = harness();
        let admin = admin_token(&h);
        h.users.create_user(&admin, input("maria")).unwrap();

        assert!(matches!(
            h.users.create_user(&admin, input("MARIA")),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn admin_reset_forces_change_and_clears_lockout() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.users.create_user(&admin, input("maria")).unwrap();
        for _ in 0..5 {
            let _ = h.auth.login("maria", "errada-errada");
        }
        assert!(matches!(
            h.auth.login("maria", "Temp-Senha-123"),
            Err(AppError::AccountLocked { .. })
        ));

        h.users.reset_password(&admin, id, "Reset-Senha-321").unwrap();

        assert!(
            h.auth
                .login("maria", "Reset-Senha-321")
                .unwrap()
                .must_change_password
        );
    }

    #[test]
    fn deactivated_user_cannot_login_and_admin_cannot_deactivate_self() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.users.create_user(&admin, input("maria")).unwrap();
        h.users.set_active(&admin, id, false).unwrap();

        assert!(matches!(
            h.auth.login("maria", "Temp-Senha-123"),
            Err(AppError::InvalidCredentials)
        ));
        let admin_id = h.auth.login("admin", ADMIN_PASSWORD).unwrap().user.id;
        assert!(h.users.set_active(&admin, admin_id, false).is_err());
    }

    #[test]
    fn audit_never_contains_password() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.users.create_user(&admin, input("maria")).unwrap();
        h.users.reset_password(&admin, id, "Reset-Senha-321").unwrap();
        h.auth
            .change_own_password(&admin, ADMIN_PASSWORD, "Nova-Admin-654")
            .unwrap();
        let _ = h.auth.login("admin", "errada-errada");

        let conn = h.db.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT COALESCE(before_json,'') || COALESCE(after_json,'') || COALESCE(justification,'') FROM audit_log",
            )
            .unwrap();
        let all: String = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for secret in [
            ADMIN_PASSWORD,
            "Temp-Senha-123",
            "Reset-Senha-321",
            "Nova-Admin-654",
            "errada-errada",
            "$argon2",
            "password_hash",
        ] {
            assert!(!all.contains(secret), "auditoria contém {secret}");
        }
        assert!(!all.is_empty());
    }

    #[test]
    fn user_creation_rolls_back_with_audit_failure() {
        let h = harness();
        let admin = admin_token(&h);
        h.db.lock()
            .unwrap()
            .execute_batch("DROP TABLE audit_log;")
            .unwrap();

        assert!(h.users.create_user(&admin, input("maria")).is_err());

        let users: i64 = h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(users, 1);
    }
}
