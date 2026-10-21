use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::auth::permissions::{role_has, Permission};
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
}

struct Session {
    info: SessionInfo,
    last_activity: Instant,
    locked: bool,
}

/// Sessões em memória. O bloqueio por inatividade usa relógio monotônico,
/// então mexer no relógio do sistema não o contorna.
pub struct SessionManager {
    sessions: Mutex<HashMap<String, Session>>,
    idle_lock: Mutex<Duration>,
}

impl SessionManager {
    pub fn new(idle_lock: Duration) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            idle_lock: Mutex::new(idle_lock),
        }
    }

    pub fn set_idle_lock(&self, idle_lock: Duration) {
        *self.idle_lock.lock().unwrap() = idle_lock;
    }

    pub fn create(&self, user_id: i64, role: Role, now: Instant) -> String {
        let mut bytes = [0u8; 32];
        rand::fill(&mut bytes[..]);
        let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        self.sessions.lock().unwrap().insert(
            token.clone(),
            Session {
                info: SessionInfo { user_id, role },
                last_activity: now,
                locked: false,
            },
        );
        token
    }

    /// Registra atividade. Sessão ociosa além do limite passa a `Locked`.
    pub fn touch(&self, token: &str, now: Instant) -> Result<SessionInfo, AppError> {
        let idle_lock = *self.idle_lock.lock().unwrap();
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(token).ok_or(AppError::Unauthorized)?;
        if session.locked || now.saturating_duration_since(session.last_activity) > idle_lock {
            session.locked = true;
            return Err(AppError::Locked);
        }
        session.last_activity = now;
        Ok(session.info)
    }

    /// Consulta a sessão sem exigir que esteja desbloqueada (usado para reautenticar).
    pub fn peek(&self, token: &str) -> Result<SessionInfo, AppError> {
        let sessions = self.sessions.lock().unwrap();
        sessions
            .get(token)
            .map(|s| s.info)
            .ok_or(AppError::Unauthorized)
    }

    /// Só deve ser chamado depois de o serviço reautenticar o usuário.
    pub fn unlock(&self, token: &str, now: Instant) -> Result<SessionInfo, AppError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(token).ok_or(AppError::Unauthorized)?;
        session.locked = false;
        session.last_activity = now;
        Ok(session.info)
    }

    pub fn destroy(&self, token: &str) {
        self.sessions.lock().unwrap().remove(token);
    }
}

pub fn require(info: &SessionInfo, permission: Permission) -> Result<(), AppError> {
    if role_has(info.role, permission) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

#[cfg(test)]
mod tests {
    use super::{require, SessionManager};
    use crate::auth::permissions::Permission;
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
    fn require_forbidden_for_recepcao() {
        let m = manager();
        let now = Instant::now();
        let admin = m.touch(&m.create(1, Role::Admin, now), now).unwrap();
        let rec = m.touch(&m.create(2, Role::Recepcao, now), now).unwrap();

        assert!(require(&admin, Permission::ManageUsers).is_ok());
        assert!(matches!(require(&rec, Permission::ManageUsers), Err(AppError::Forbidden)));
    }
}
