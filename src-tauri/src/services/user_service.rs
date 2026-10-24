use serde::Deserialize;
use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::password::{hash_password, validate_password_policy};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::error::AppError;
use crate::models::page::{clamp_paging, Page};
use crate::models::user::{NewUser, Role};
use crate::repositories::user_repo::{UserListFilter, UserRow};
use crate::repositories::{permission_repo, user_repo};

#[derive(Deserialize)]
pub struct NewUserInput {
    pub username: String,
    pub display_name: String,
    pub role: Role,
    pub initial_password: String,
}

#[derive(Debug, Deserialize)]
pub struct UserQuery {
    pub text: Option<String>,
    pub active: Option<bool>,
    pub page: u32,
    pub page_size: u32,
    pub sort: Option<String>,
    /// `"asc"` (padrão) ou `"desc"`.
    pub direction: Option<String>,
}

impl std::fmt::Debug for NewUserInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewUserInput")
            .field("username", &self.username)
            .field("display_name", &self.display_name)
            .field("role", &self.role)
            .field("initial_password", &"[oculto]")
            .finish()
    }
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

    /// Impede ficar sem nenhum administrador ativo.
    fn ensure_not_last_active_admin(conn: &rusqlite::Connection, user: &crate::models::user::User) -> Result<(), AppError> {
        if user.role == Role::Admin && user.active && user_repo::count_active_admins(conn)? <= 1 {
            return Err(AppError::Validation(
                "O sistema precisa de pelo menos um administrador ativo.".into(),
            ));
        }
        Ok(())
    }

    /// Lista usuários com busca, filtro e paginação (exige `ManageUsers`).
    pub fn list(&self, token: &str, query: UserQuery) -> Result<Page<UserRow>, AppError> {
        self.authorize(token)?;
        let (page, page_size, offset) = clamp_paging(query.page, query.page_size);
        let conn = self.ctx.conn()?;
        let (mut items, total) = user_repo::list(
            &conn,
            &UserListFilter {
                text: query.text.as_deref(),
                active: query.active,
                sort: query.sort.as_deref(),
                descending: query.direction.as_deref() == Some("desc"),
                limit: i64::from(page_size),
                offset,
            },
        )?;
        for row in &mut items {
            row.extra_permissions = permission_repo::list(&conn, row.id)?
                .iter()
                .map(|p| p.as_str().to_string())
                .collect();
        }
        Ok(Page { items, total, page, page_size })
    }

    /// Troca o perfil de um usuário. Não permite mudar o próprio perfil nem
    /// rebaixar o último administrador ativo.
    pub fn set_role(
        &self,
        token: &str,
        user_id: i64,
        role: Role,
        expected_version: i64,
    ) -> Result<(), AppError> {
        let actor = self.authorize(token)?;
        if actor.user_id == user_id {
            return Err(AppError::Validation("Você não pode alterar o próprio perfil.".into()));
        }
        let mut conn = self.ctx.conn()?;
        let user = user_repo::find(&conn, user_id)?
            .ok_or_else(|| AppError::Validation("Usuário não encontrado.".into()))?;
        if role != Role::Admin {
            Self::ensure_not_last_active_admin(&conn, &user)?;
        }
        let tx = conn.transaction()?;
        user_repo::set_role(&tx, user.id, role, expected_version, self.ctx.utc.now_utc())?;
        if role == Role::Admin {
            // Administradores têm todas as permissões; extras deixam de fazer sentido.
            permission_repo::replace(&tx, user.id, &[], actor.user_id, self.ctx.utc.now_utc())?;
        }
        audit::record(
            &tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "users",
                operation: "set_role",
                entity: "user",
                entity_id: Some(user.id),
                before: Some(json!({ "role": user.role.as_str() })),
                after: Some(json!({ "role": role.as_str() })),
                justification: None,
            },
        )?;
        tx.commit()?;
        self.ctx.sessions.destroy_user(user.id);
        Ok(())
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

        let mut conn = self.ctx.conn()?;
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
            self.ctx.utc.now_utc(),
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
                at: self.ctx.utc.now_utc(),
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
        expected_version: i64,
    ) -> Result<(), AppError> {
        let actor = self.authorize(token)?;
        validate_password_policy(temp_password)?;
        let hash = hash_password(temp_password)?;

        let mut conn = self.ctx.conn()?;
        let user = user_repo::find(&conn, user_id)?
            .ok_or_else(|| AppError::Validation("Usuário não encontrado.".into()))?;
        let tx = conn.transaction()?;
        user_repo::update_password(&tx, user.id, &hash, true, expected_version, self.ctx.utc.now_utc())?;
        user_repo::reset_attempts(&tx, user.id, self.ctx.utc.now_utc())?;
        audit::record(
            &tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
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
        self.ctx.sessions.destroy_user(user.id);
        Ok(())
    }

    /// Substitui as permissões extras de um usuário da recepção. Vale a partir do
    /// próximo login: as sessões abertas do usuário são encerradas.
    pub fn set_extra_permissions(
        &self,
        token: &str,
        user_id: i64,
        permissions: &[Permission],
    ) -> Result<(), AppError> {
        let actor = self.authorize(token)?;
        if let Some(p) = permissions
            .iter()
            .find(|p| !Permission::grantable().contains(p))
        {
            return Err(AppError::Validation(format!(
                "A permissão \"{}\" não pode ser concedida individualmente.",
                p.as_str()
            )));
        }
        let mut conn = self.ctx.conn()?;
        let user = user_repo::find(&conn, user_id)?
            .ok_or_else(|| AppError::Validation("Usuário não encontrado.".into()))?;
        if user.role == Role::Admin {
            return Err(AppError::Validation(
                "Administradores já têm todas as permissões.".into(),
            ));
        }
        let before = permission_repo::list(&conn, user_id)?;
        let tx = conn.transaction()?;
        permission_repo::replace(&tx, user_id, permissions, actor.user_id, self.ctx.utc.now_utc())?;
        let names = |list: &[Permission]| json!(list.iter().map(|p| p.as_str()).collect::<Vec<_>>());
        audit::record(
            &tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "users",
                operation: "set_extra_permissions",
                entity: "user",
                entity_id: Some(user_id),
                before: Some(names(&before)),
                after: Some(names(permissions)),
                justification: None,
            },
        )?;
        tx.commit()?;
        self.ctx.sessions.destroy_user(user_id);
        Ok(())
    }

    pub fn set_active(&self, token: &str, user_id: i64, active: bool) -> Result<(), AppError> {
        let actor = self.authorize(token)?;
        if !active && actor.user_id == user_id {
            return Err(AppError::Validation(
                "Você não pode desativar o próprio usuário.".into(),
            ));
        }
        let mut conn = self.ctx.conn()?;
        let user = user_repo::find(&conn, user_id)?
            .ok_or_else(|| AppError::Validation("Usuário não encontrado.".into()))?;
        if !active {
            Self::ensure_not_last_active_admin(&conn, &user)?;
        }
        let tx = conn.transaction()?;
        user_repo::set_active(&tx, user.id, active, self.ctx.utc.now_utc())?;
        audit::record(
            &tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
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
        if !active {
            self.ctx.sessions.destroy_user(user.id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{NewUserInput, UserQuery};
    use crate::auth::permissions::Permission;
    use crate::auth::session::require;
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
        h.auth
            .change_own_password(&maria, "Temp-Senha-123", "Definitiva-456")
            .unwrap();

        assert!(matches!(
            h.users.create_user(&maria, input("joao")),
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            h.users.reset_password(&maria, maria_id, "Outra-Senha-789", 1),
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

        h.users.reset_password(&admin, id, "Reset-Senha-321", 1).unwrap();

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
    fn temporary_password_session_cannot_manage_users_until_changed() {
        let h = harness();
        let admin = admin_token(&h);
        h.users
            .create_user(
                &admin,
                NewUserInput {
                    role: Role::Admin,
                    ..input("chefe2")
                },
            )
            .unwrap();
        let chefe = h.auth.login("chefe2", "Temp-Senha-123").unwrap();
        assert!(chefe.must_change_password);

        assert!(matches!(
            h.users.create_user(&chefe.token, input("joao")),
            Err(AppError::PasswordChangeRequired)
        ));
        h.auth
            .change_own_password(&chefe.token, "Temp-Senha-123", "Definitiva-456")
            .unwrap();
        assert!(h.users.create_user(&chefe.token, input("joao")).is_ok());
    }

    #[test]
    fn deactivating_user_ends_their_sessions() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.users.create_user(&admin, input("maria")).unwrap();
        let maria = h.auth.login("maria", "Temp-Senha-123").unwrap().token;
        assert!(h.auth.session_status(&maria).is_ok());

        h.users.set_active(&admin, id, false).unwrap();

        assert!(matches!(h.auth.session_status(&maria), Err(AppError::Unauthorized)));
    }

    #[test]
    fn locked_session_of_deactivated_user_cannot_unlock() {
        let h = harness();
        let admin = admin_token(&h);
        h.users.create_user(&admin, input("maria")).unwrap();
        let maria = h.auth.login("maria", "Temp-Senha-123").unwrap().token;
        h.clock.advance(std::time::Duration::from_secs(601));
        assert!(matches!(h.auth.session_status(&maria), Err(AppError::Locked)));
        h.db.lock()
            .unwrap()
            .execute("UPDATE users SET active = 0 WHERE username = 'maria'", [])
            .unwrap();

        assert!(matches!(
            h.auth.unlock_session(&maria, "Temp-Senha-123"),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn reset_password_rejects_stale_version_and_ends_user_sessions() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.users.create_user(&admin, input("maria")).unwrap();
        let maria = h.auth.login("maria", "Temp-Senha-123").unwrap().token;

        assert!(matches!(
            h.users.reset_password(&admin, id, "Reset-Senha-321", 99),
            Err(AppError::Conflict)
        ));
        assert!(h.auth.session_status(&maria).is_ok());

        h.users.reset_password(&admin, id, "Reset-Senha-321", 1).unwrap();
        assert!(matches!(h.auth.session_status(&maria), Err(AppError::Unauthorized)));
    }

    #[test]
    fn debug_output_never_contains_secrets() {
        let h = harness();
        let admin = admin_token(&h);
        let user_input = input("maria");
        let dbg = format!("{user_input:?}");
        assert!(!dbg.contains("Temp-Senha-123"));
        h.users.create_user(&admin, user_input).unwrap();

        let login = h.auth.login("maria", "Temp-Senha-123").unwrap();
        let dbg = format!("{login:?}");
        assert!(!dbg.contains(&login.token));
    }

    fn maria_with_password_changed(h: &crate::services::test_support::Harness, admin: &str) -> (i64, String) {
        let id = h.users.create_user(admin, input("maria")).unwrap();
        let token = h.auth.login("maria", "Temp-Senha-123").unwrap().token;
        h.auth
            .change_own_password(&token, "Temp-Senha-123", "Definitiva-456")
            .unwrap();
        (id, token)
    }

    #[test]
    fn extra_permissions_apply_on_next_login_and_revoking_ends_sessions() {
        let h = harness();
        let admin = admin_token(&h);
        let (id, old_token) = maria_with_password_changed(&h, &admin);

        h.users
            .set_extra_permissions(&admin, id, &[Permission::ApplyDiscount])
            .unwrap();
        assert!(matches!(h.auth.session_status(&old_token), Err(AppError::Unauthorized)));

        let token = h.auth.login("maria", "Definitiva-456").unwrap().token;
        let info = h.auth.session_status(&token).unwrap();
        assert!(require(&info, Permission::ApplyDiscount).is_ok());

        h.users.set_extra_permissions(&admin, id, &[]).unwrap();
        assert!(matches!(h.auth.session_status(&token), Err(AppError::Unauthorized)));
        let again = h.auth.login("maria", "Definitiva-456").unwrap().token;
        let info = h.auth.session_status(&again).unwrap();
        assert!(matches!(require(&info, Permission::ApplyDiscount), Err(AppError::Forbidden)));
    }

    #[test]
    fn cannot_grant_admin_only_permissions_or_to_admins() {
        let h = harness();
        let admin = admin_token(&h);
        let (id, _) = maria_with_password_changed(&h, &admin);
        let admin_id = h.auth.login("admin", ADMIN_PASSWORD).unwrap().user.id;

        assert!(matches!(
            h.users.set_extra_permissions(&admin, id, &[Permission::ManageUsers]),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            h.users.set_extra_permissions(&admin, admin_id, &[Permission::ApplyDiscount]),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn set_extra_permissions_requires_manage_users_and_is_audited() {
        let h = harness();
        let admin = admin_token(&h);
        let (id, maria) = maria_with_password_changed(&h, &admin);

        assert!(matches!(
            h.users.set_extra_permissions(&maria, id, &[Permission::ApplyDiscount]),
            Err(AppError::Forbidden)
        ));
        h.users
            .set_extra_permissions(&admin, id, &[Permission::ApplyDiscount, Permission::ReleaseRoom])
            .unwrap();

        let conn = h.db.lock().unwrap();
        let after: String = conn
            .query_row(
                "SELECT after_json FROM audit_log WHERE operation = 'set_extra_permissions'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(after.contains("apply_discount") && after.contains("release_room"));
    }

    fn query(text: Option<&str>, page: u32, page_size: u32) -> UserQuery {
        UserQuery {
            text: text.map(String::from),
            active: None,
            page,
            page_size,
            sort: None,
            direction: None,
        }
    }

    #[test]
    fn list_requires_manage_users() {
        let h = harness();
        let admin = admin_token(&h);
        let rec = crate::services::test_support::recepcao_token(&h, &admin);

        assert!(matches!(h.users.list(&rec, query(None, 1, 10)), Err(AppError::Forbidden)));
        assert!(h.users.list(&admin, query(None, 1, 10)).is_ok());
    }

    #[test]
    fn list_filters_paginates_and_shows_extra_permissions() {
        let h = harness();
        let admin = admin_token(&h);
        for name in ["ana", "bia", "caio"] {
            h.users.create_user(&admin, input(name)).unwrap();
        }
        let ana = h.users.list(&admin, query(Some("ANA"), 1, 10)).unwrap();
        assert_eq!(ana.total, 1);
        assert_eq!(ana.items[0].username, "ana");

        let first = h.users.list(&admin, query(None, 1, 2)).unwrap();
        let second = h.users.list(&admin, query(None, 2, 2)).unwrap();
        assert_eq!((first.total, first.items.len(), second.items.len()), (4, 2, 2));

        let id = ana.items[0].id;
        h.users.set_extra_permissions(&admin, id, &[Permission::ApplyDiscount]).unwrap();
        let again = h.users.list(&admin, query(Some("ana"), 1, 10)).unwrap();
        assert_eq!(again.items[0].extra_permissions, vec!["apply_discount".to_string()]);
    }

    #[test]
    fn list_ignores_unknown_sort_column() {
        let h = harness();
        let admin = admin_token(&h);
        let mut q = query(None, 1, 10);
        q.sort = Some("password_hash; DROP TABLE users".into());

        assert!(h.users.list(&admin, q).is_ok());
    }

    #[test]
    fn cannot_remove_last_active_admin() {
        let h = harness();
        let admin = admin_token(&h);
        let admin_id = h.auth.login("admin", ADMIN_PASSWORD).unwrap().user.id;
        let other_id = h
            .users
            .create_user(&admin, NewUserInput { role: Role::Admin, ..input("chefe2") })
            .unwrap();
        // O ator deixa de estar ativo no banco (cenário de corrida): o outro vira o último admin ativo.
        h.db.lock()
            .unwrap()
            .execute("UPDATE users SET active = 0 WHERE id = ?1", [admin_id])
            .unwrap();

        assert!(matches!(
            h.users.set_role(&admin, other_id, Role::Recepcao, 1),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(h.users.set_active(&admin, other_id, false), Err(AppError::Validation(_))));
    }

    #[test]
    fn admin_cannot_change_own_role() {
        let h = harness();
        let admin = admin_token(&h);
        let admin_id = h.auth.login("admin", ADMIN_PASSWORD).unwrap().user.id;

        assert!(matches!(
            h.users.set_role(&admin, admin_id, Role::Recepcao, 1),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn set_role_conflicts_on_stale_version_and_is_audited() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.users.create_user(&admin, input("maria")).unwrap();

        assert!(matches!(
            h.users.set_role(&admin, id, Role::Admin, 99),
            Err(AppError::Conflict)
        ));
        h.users.set_role(&admin, id, Role::Admin, 1).unwrap();

        let conn = h.db.lock().unwrap();
        let (before, after): (String, String) = conn
            .query_row(
                "SELECT before_json, after_json FROM audit_log WHERE operation = 'set_role'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(before.contains("recepcao") && after.contains("admin"));
    }

    #[test]
    fn audit_never_contains_password() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.users.create_user(&admin, input("maria")).unwrap();
        h.users.reset_password(&admin, id, "Reset-Senha-321", 1).unwrap();
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
