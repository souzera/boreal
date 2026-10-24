use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::auth::permissions::{role_has, Permission, PermissionSet};
use crate::error::AppError;
use crate::models::user::Role;

/// Fonte de tempo monotônico, injetável nos testes.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SessionInfo {
    pub user_id: i64,
    pub role: Role,
    pub extra: PermissionSet,
}

struct Session {
    info: SessionInfo,
    last_activity: Instant,
    locked: bool,
    must_change_password: bool,
}

/// Sessões em memória. O bloqueio por inatividade usa relógio monotônico,
/// então mexer no relógio do sistema não o contorna.
pub struct SessionManager {
    sessions: Mutex<HashMap<String, Session>>,
    idle_lock: Mutex<Duration>,
}

impl SessionManager {
    // O estado das sessões é simples e se mantém consistente mesmo se uma thread falhar
    // segurando o trava; por isso um mutex envenenado é recuperado em vez de derrubar o app.
    fn sessions(&self) -> std::sync::MutexGuard<'_, HashMap<String, Session>> {
        self.sessions.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn idle_lock(&self) -> std::sync::MutexGuard<'_, Duration> {
        self.idle_lock.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn new(idle_lock: Duration) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            idle_lock: Mutex::new(idle_lock),
        }
    }

    pub fn set_idle_lock(&self, idle_lock: Duration) {
        *self.idle_lock() = idle_lock;
    }

    pub fn create(&self, user_id: i64, role: Role, now: Instant) -> String {
        self.create_with(user_id, role, PermissionSet::empty(), now)
    }

    pub fn create_with(&self, user_id: i64, role: Role, extra: PermissionSet, now: Instant) -> String {
        let mut bytes = [0u8; 32];
        rand::fill(&mut bytes[..]);
        let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        self.sessions().insert(
            token.clone(),
            Session {
                info: SessionInfo { user_id, role, extra },
                last_activity: now,
                locked: false,
                must_change_password: false,
            },
        );
        token
    }

    /// Registra atividade. Sessão ociosa além do limite passa a `Locked`;
    /// sessão com troca de senha pendente é recusada (`PasswordChangeRequired`).
    pub fn touch(&self, token: &str, now: Instant) -> Result<SessionInfo, AppError> {
        self.touch_inner(token, now, false)
    }

    /// Como `touch`, mas aceita a sessão com troca de senha pendente
    /// (só para trocar a senha, consultar o estado e sair).
    pub fn touch_allow_pending(&self, token: &str, now: Instant) -> Result<SessionInfo, AppError> {
        self.touch_inner(token, now, true)
    }

    fn touch_inner(&self, token: &str, now: Instant, allow_pending: bool) -> Result<SessionInfo, AppError> {
        let idle_lock = *self.idle_lock();
        let mut sessions = self.sessions();
        let session = sessions.get_mut(token).ok_or(AppError::Unauthorized)?;
        if session.locked || now.saturating_duration_since(session.last_activity) > idle_lock {
            session.locked = true;
            return Err(AppError::Locked);
        }
        session.last_activity = now;
        if session.must_change_password && !allow_pending {
            return Err(AppError::PasswordChangeRequired);
        }
        Ok(session.info)
    }

    pub fn set_must_change(&self, token: &str, value: bool) {
        if let Some(session) = self.sessions().get_mut(token) {
            session.must_change_password = value;
        }
    }

    /// Encerra todas as sessões de um usuário (ex.: ao desativá-lo).
    pub fn destroy_user(&self, user_id: i64) {
        self.sessions
            .lock()
            .unwrap()
            .retain(|_, s| s.info.user_id != user_id);
    }

    /// Encerra as sessões de um usuário, exceto a do token informado.
    pub fn destroy_user_except(&self, user_id: i64, keep_token: &str) {
        self.sessions
            .lock()
            .unwrap()
            .retain(|token, s| s.info.user_id != user_id || token == keep_token);
    }

    /// Consulta a sessão sem exigir que esteja desbloqueada (usado para reautenticar).
    pub fn peek(&self, token: &str) -> Result<SessionInfo, AppError> {
        let sessions = self.sessions();
        sessions
            .get(token)
            .map(|s| s.info)
            .ok_or(AppError::Unauthorized)
    }

    /// Só deve ser chamado depois de o serviço reautenticar o usuário.
    pub fn unlock(&self, token: &str, now: Instant) -> Result<SessionInfo, AppError> {
        let mut sessions = self.sessions();
        let session = sessions.get_mut(token).ok_or(AppError::Unauthorized)?;
        session.locked = false;
        session.last_activity = now;
        Ok(session.info)
    }

    pub fn destroy(&self, token: &str) {
        self.sessions().remove(token);
    }
}

pub fn require(info: &SessionInfo, permission: Permission) -> Result<(), AppError> {
    if role_has(info.role, permission) || info.extra.contains(permission) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

#[cfg(test)]
mod tests {
    use super::{require, SessionManager};
    use crate::auth::permissions::{Permission, PermissionSet};
    use crate::error::AppError;
    use crate::models::user::Role;
    use std::time::{Duration, Instant};

    fn manager() -> SessionManager {
        SessionManager::new(Duration::from_secs(600))
    }

    #[test]
    fn idle_session_becomes_locked_and_unlock_restores_it() {
        let m = manager();
        let t0 = Instant::now();
        let token = m.create(1, Role::Admin, t0);

        assert!(m.touch(&token, t0 + Duration::from_secs(599)).is_ok());
        let later = t0 + Duration::from_secs(599 + 601);
        assert!(matches!(m.touch(&token, later), Err(AppError::Locked)));
        assert!(matches!(m.touch(&token, later), Err(AppError::Locked)));

        m.unlock(&token, later).unwrap();
        assert_eq!(m.touch(&token, later).unwrap().user_id, 1);
    }

    #[test]
    fn poisoned_session_manager_still_answers() {
        let m = std::sync::Arc::new(manager());
        let now = Instant::now();
        let token = m.create(1, Role::Admin, now);
        let clone = m.clone();
        let _ = std::thread::spawn(move || {
            let _guard = clone.sessions.lock().unwrap();
            panic!("pânico proposital para envenenar o mutex");
        })
        .join();

        assert!(m.touch(&token, now).is_ok());
        assert!(m.create(2, Role::Recepcao, now).len() == 64);
    }

    #[test]
    fn unknown_token_unauthorized() {
        let m = manager();
        assert!(matches!(m.touch("nope", Instant::now()), Err(AppError::Unauthorized)));
        assert!(matches!(m.unlock("nope", Instant::now()), Err(AppError::Unauthorized)));
    }

    #[test]
    fn destroy_invalidates() {
        let m = manager();
        let now = Instant::now();
        let token = m.create(1, Role::Recepcao, now);
        m.destroy(&token);

        assert!(matches!(m.touch(&token, now), Err(AppError::Unauthorized)));
    }

    #[test]
    fn tokens_are_unique_and_long() {
        let m = manager();
        let now = Instant::now();
        let a = m.create(1, Role::Admin, now);
        let b = m.create(1, Role::Admin, now);

        assert_ne!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn pending_password_change_blocks_strict_touch() {
        let m = manager();
        let now = Instant::now();
        let token = m.create(1, Role::Admin, now);
        m.set_must_change(&token, true);

        assert!(matches!(m.touch(&token, now), Err(AppError::PasswordChangeRequired)));
        assert!(m.touch_allow_pending(&token, now).is_ok());
        m.set_must_change(&token, false);
        assert!(m.touch(&token, now).is_ok());
    }

    #[test]
    fn destroy_user_ends_all_sessions_of_that_user() {
        let m = manager();
        let now = Instant::now();
        let a1 = m.create(1, Role::Admin, now);
        let a2 = m.create(1, Role::Admin, now);
        let b = m.create(2, Role::Recepcao, now);

        m.destroy_user(1);

        assert!(matches!(m.touch(&a1, now), Err(AppError::Unauthorized)));
        assert!(matches!(m.touch(&a2, now), Err(AppError::Unauthorized)));
        assert!(m.touch(&b, now).is_ok());
    }

    #[test]
    fn extra_permission_allows_recepcao_operation() {
        let m = manager();
        let now = Instant::now();
        let extra = PermissionSet::empty().with(Permission::ApplyDiscount);
        let info = m.touch(&m.create_with(2, Role::Recepcao, extra, now), now).unwrap();

        assert!(require(&info, Permission::ApplyDiscount).is_ok());
        assert!(matches!(require(&info, Permission::ReleaseRoom), Err(AppError::Forbidden)));
        assert!(matches!(require(&info, Permission::ManageUsers), Err(AppError::Forbidden)));
    }

    #[test]
    fn require_forbidden_for_recepcao() {
        let m = manager();
        let now = Instant::now();
        let admin = m.touch(&m.create(1, Role::Admin, now), now).unwrap();
        let rec = m.touch(&m.create(2, Role::Recepcao, now), now).unwrap();

        assert!(require(&admin, Permission::ManageUsers).is_ok());
        assert!(matches!(require(&rec, Permission::ManageUsers), Err(AppError::Forbidden)));
    }
}
