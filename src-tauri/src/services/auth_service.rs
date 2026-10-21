use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use rusqlite::Transaction;
use serde::Serialize;
use serde_json::json;

use super::{format_utc, parse_utc, Ctx};
use crate::audit::{self, AuditEntry};
use crate::auth::password::{hash_password, validate_password_policy, verify_password};
use crate::auth::session::SessionInfo;
use crate::error::AppError;
use crate::models::user::{NewUser, Role, User};
use crate::repositories::{settings_repo, user_repo};

/// Hash usado para igualar o tempo de resposta quando o usuário não existe.
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password("senha-ficticia-para-igualar-tempo").unwrap_or_default());

#[derive(Debug, Serialize)]
pub struct LoginResult {
    pub token: String,
    pub user: User,
    pub must_change_password: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionStatus {
    pub state: &'static str,
    pub user: Option<User>,
}

#[derive(Clone)]
pub struct AuthService {
    ctx: Ctx,
}

impl AuthService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    pub fn needs_bootstrap(&self) -> Result<bool, AppError> {
        let conn = self.ctx.db.lock().unwrap();
        Ok(user_repo::count(&conn)? == 0)
    }

    /// Cria o primeiro administrador. Só funciona enquanto não houver usuários.
    pub fn bootstrap_admin(
        &self,
        username: &str,
        display_name: &str,
        password: &str,
    ) -> Result<(), AppError> {
        let username = username.trim();
        let display_name = display_name.trim();
        if username.is_empty() || display_name.is_empty() {
            return Err(AppError::Validation(
                "Informe o usuário e o nome do administrador.".into(),
            ));
        }
        validate_password_policy(password)?;
        let hash = hash_password(password)?;

        let mut conn = self.ctx.db.lock().unwrap();
        let tx = conn.transaction()?;
        if user_repo::count(&tx)? > 0 {
            return Err(AppError::Validation("O sistema já foi configurado.".into()));
        }
        let id = user_repo::insert(
            &tx,
            &NewUser {
                username: username.into(),
                display_name: display_name.into(),
                password_hash: hash,
                role: Role::Admin,
                must_change_password: false,
            },
        )?;
        audit::record(
            &tx,
            &AuditEntry {
                user_id: Some(id),
                module: "users",
                operation: "bootstrap_admin",
                entity: "user",
                entity_id: Some(id),
                before: None,
                after: Some(json!({ "username": username, "role": "admin" })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn login(&self, username: &str, password: &str) -> Result<LoginResult, AppError> {
        let mut conn = self.ctx.db.lock().unwrap();
        let Some((user, hash)) = user_repo::find_by_username(&conn, username.trim())? else {
            // Usuário inexistente: mesmo custo de verificação, nenhum estado gravado.
            verify_password(password, &DUMMY_HASH);
            return Err(AppError::InvalidCredentials);
        };
        let now = self.ctx.utc.now_utc();
        let tx = conn.transaction()?;

        let mut attempts = user.failed_attempts;
        if let Some(until) = user.locked_until.as_deref().and_then(parse_utc) {
            if until > now {
                return Err(AppError::AccountLocked {
                    until: format_utc(until),
                });
            }
            user_repo::reset_attempts(&tx, user.id)?;
            attempts = 0;
        }

        let password_ok = verify_password(password, &hash);
        if !user.active {
            return Err(AppError::InvalidCredentials);
        }
        if !password_ok {
            register_failure(&tx, &user, attempts, now)?;
            tx.commit()?;
            return Err(AppError::InvalidCredentials);
        }

        if attempts > 0 || user.locked_until.is_some() {
            user_repo::reset_attempts(&tx, user.id)?;
        }
        audit::record(
            &tx,
            &AuditEntry {
                user_id: Some(user.id),
                module: "auth",
                operation: "login",
                entity: "user",
                entity_id: Some(user.id),
                before: None,
                after: Some(json!({ "username": user.username })),
                justification: None,
            },
        )?;
        tx.commit()?;

        let token = self
            .ctx
            .sessions
            .create(user.id, user.role, self.ctx.clock.now());
        Ok(LoginResult {
            token,
            must_change_password: user.must_change_password,
            user,
        })
    }

    pub fn logout(&self, token: &str) {
        self.ctx.sessions.destroy(token);
    }

    /// Valida a sessão e registra atividade; `Locked` se estiver bloqueada por inatividade.
    pub fn session_status(&self, token: &str) -> Result<SessionInfo, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())
    }

    /// Estado da sessão para a interface: ativa ou bloqueada por inatividade.
    pub fn session_state(&self, token: &str) -> Result<SessionStatus, AppError> {
        let (state, info) = match self.session_status(token) {
            Ok(info) => ("active", info),
            Err(AppError::Locked) => ("locked", self.ctx.sessions.peek(token)?),
            Err(other) => return Err(other),
        };
        let conn = self.ctx.db.lock().unwrap();
        let user = user_repo::find(&conn, info.user_id)?;
        Ok(SessionStatus { state, user })
    }

    pub fn change_own_password(
        &self,
        token: &str,
        current: &str,
        new: &str,
    ) -> Result<(), AppError> {
        let info = self.session_status(token)?;
        validate_password_policy(new)?;
        let new_hash = hash_password(new)?;

        let mut conn = self.ctx.db.lock().unwrap();
        let (user, hash) =
            user_repo::find_with_hash(&conn, info.user_id)?.ok_or(AppError::Unauthorized)?;
        if !verify_password(current, &hash) {
            return Err(AppError::Validation("A senha atual está incorreta.".into()));
        }
        let tx = conn.transaction()?;
        user_repo::update_password(&tx, user.id, &new_hash, false, user.version)?;
        audit::record(
            &tx,
            &AuditEntry {
                user_id: Some(user.id),
                module: "auth",
                operation: "password_changed",
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

    /// Reautentica e desbloqueia uma sessão bloqueada por inatividade.
    pub fn unlock_session(&self, token: &str, password: &str) -> Result<(), AppError> {
        let info = self.ctx.sessions.peek(token)?;
        let now = self.ctx.utc.now_utc();
        let mut conn = self.ctx.db.lock().unwrap();
        let (user, hash) =
            user_repo::find_with_hash(&conn, info.user_id)?.ok_or(AppError::Unauthorized)?;
        let tx = conn.transaction()?;

        if let Some(until) = user
            .locked_until
            .as_deref()
            .and_then(parse_utc)
            .filter(|until| *until > now)
        {
            return Err(AppError::AccountLocked {
                until: format_utc(until),
            });
        }
        if !verify_password(password, &hash) {
            register_failure(&tx, &user, user.failed_attempts, now)?;
            tx.commit()?;
            return Err(AppError::InvalidCredentials);
        }
        if user.failed_attempts > 0 {
            user_repo::reset_attempts(&tx, user.id)?;
        }
        audit::record(
            &tx,
            &AuditEntry {
                user_id: Some(user.id),
                module: "auth",
                operation: "session_unlock",
                entity: "user",
                entity_id: Some(user.id),
                before: None,
                after: None,
                justification: None,
            },
        )?;
        tx.commit()?;
        self.ctx.sessions.unlock(token, self.ctx.clock.now())?;
        Ok(())
    }
}

/// Conta a tentativa inválida e bloqueia a conta ao atingir o limite configurado.
fn register_failure(
    tx: &Transaction,
    user: &User,
    attempts: u32,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let max = settings_repo::get_u32(tx, "max_failed_attempts")?;
    let minutes = settings_repo::get_u32(tx, "lockout_minutes")?;
    let locks = attempts + 1 >= max;
    let until = locks.then(|| format_utc(now + chrono::Duration::minutes(i64::from(minutes))));
    user_repo::record_failed_attempt(tx, user.id, until.as_deref())?;
    audit::record(
        tx,
        &AuditEntry {
            user_id: Some(user.id),
            module: "auth",
            operation: if locks { "login_lockout" } else { "login_failed" },
            entity: "user",
            entity_id: Some(user.id),
            before: None,
            after: until.map(|u| json!({ "locked_until": u })),
            justification: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::services::test_support::{admin_token, harness, t0, Harness, ADMIN_PASSWORD};
    use std::time::Duration;

    fn audit_ops(h: &Harness) -> Vec<String> {
        let conn = h.db.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT operation FROM audit_log ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn bootstrap_only_once() {
        let h = harness();
        assert!(h.auth.needs_bootstrap().unwrap());
        h.auth
            .bootstrap_admin("admin", "Administrador", ADMIN_PASSWORD)
            .unwrap();

        assert!(!h.auth.needs_bootstrap().unwrap());
        assert!(h
            .auth
            .bootstrap_admin("outro", "Outro", ADMIN_PASSWORD)
            .is_err());
    }

    #[test]
    fn login_success_is_audited() {
        let h = harness();
        admin_token(&h);

        assert!(audit_ops(&h).contains(&"login".to_string()));
    }

    #[test]
    fn wrong_password_and_unknown_user_return_same_error() {
        let h = harness();
        admin_token(&h);

        let wrong = h.auth.login("admin", "errada-errada").unwrap_err();
        let unknown = h.auth.login("fantasma", "errada-errada").unwrap_err();

        assert!(matches!(wrong, AppError::InvalidCredentials));
        assert!(matches!(unknown, AppError::InvalidCredentials));
        assert_eq!(wrong.to_string(), unknown.to_string());
    }

    #[test]
    fn lockout_after_max_attempts_survives_clock_rewind_and_expires() {
        let h = harness();
        admin_token(&h);
        for _ in 0..5 {
            assert!(matches!(
                h.auth.login("admin", "errada-errada"),
                Err(AppError::InvalidCredentials)
            ));
        }

        assert!(matches!(
            h.auth.login("admin", ADMIN_PASSWORD),
            Err(AppError::AccountLocked { .. })
        ));
        assert!(audit_ops(&h).contains(&"login_lockout".to_string()));

        h.utc.advance(chrono::Duration::hours(-1));
        assert!(matches!(
            h.auth.login("admin", ADMIN_PASSWORD),
            Err(AppError::AccountLocked { .. })
        ));

        h.utc.set(t0() + chrono::Duration::minutes(6));
        assert!(h.auth.login("admin", ADMIN_PASSWORD).is_ok());
        // contador zerado: uma falha depois do desbloqueio não bloqueia de novo
        assert!(matches!(
            h.auth.login("admin", "errada-errada"),
            Err(AppError::InvalidCredentials)
        ));
        assert!(h.auth.login("admin", ADMIN_PASSWORD).is_ok());
    }

    #[test]
    fn unknown_user_does_not_create_lock_state() {
        let h = harness();
        admin_token(&h);
        for _ in 0..10 {
            let _ = h.auth.login("fantasma", "errada-errada");
        }

        let conn = h.db.lock().unwrap();
        let users: i64 = conn
            .query_row("SELECT count(*) FROM users", [], |r| r.get(0))
            .unwrap();
        let locked: i64 = conn
            .query_row(
                "SELECT count(*) FROM users WHERE locked_until IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!((users, locked), (1, 0));
    }

    #[test]
    fn change_own_password_flow() {
        let h = harness();
        let token = admin_token(&h);

        assert!(h
            .auth
            .change_own_password(&token, "senha-atual-errada", "Nova-Senha-22")
            .is_err());
        assert!(h
            .auth
            .change_own_password(&token, ADMIN_PASSWORD, "curta")
            .is_err());
        h.auth
            .change_own_password(&token, ADMIN_PASSWORD, "Nova-Senha-22")
            .unwrap();

        assert!(h.auth.login("admin", ADMIN_PASSWORD).is_err());
        assert!(h.auth.login("admin", "Nova-Senha-22").is_ok());
    }

    #[test]
    fn unlock_session_requires_correct_password() {
        let h = harness();
        let token = admin_token(&h);
        h.clock.advance(Duration::from_secs(601));
        assert!(matches!(h.auth.session_status(&token), Err(AppError::Locked)));

        assert!(matches!(
            h.auth.unlock_session(&token, "errada-errada"),
            Err(AppError::InvalidCredentials)
        ));
        assert!(matches!(h.auth.session_status(&token), Err(AppError::Locked)));
        h.auth.unlock_session(&token, ADMIN_PASSWORD).unwrap();
        assert!(h.auth.session_status(&token).is_ok());
    }

    #[test]
    fn session_state_reports_active_then_locked() {
        let h = harness();
        let token = admin_token(&h);

        let active = h.auth.session_state(&token).unwrap();
        assert_eq!(active.state, "active");
        assert_eq!(active.user.unwrap().username, "admin");

        h.clock.advance(Duration::from_secs(601));
        let locked = h.auth.session_state(&token).unwrap();
        assert_eq!(locked.state, "locked");
        assert!(locked.user.is_some());

        assert!(matches!(
            h.auth.session_state("token-invalido"),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn logout_invalidates_session() {
        let h = harness();
        let token = admin_token(&h);
        h.auth.logout(&token);

        assert!(matches!(
            h.auth.session_status(&token),
            Err(AppError::Unauthorized)
        ));
    }
}
