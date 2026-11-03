//! Chave de recuperação e, mais adiante, recuperação em outro computador (spec §14, §16.2).

use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::password::verify_password;
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::recovery_key;
use crate::error::AppError;
use crate::db::keystore::KeyStore;
use crate::domain::secret::SecretKey;
use crate::repositories::user_repo;
use super::backup_service::{latest_schema_version, validate_file};
use std::path::Path;

/// Confere o arquivo de backup com a chave digitada e a versão do esquema. Devolve a chave em bytes.
fn check_backup_with_key(backup: &Path, recovery_key_text: &str) -> Result<Vec<u8>, AppError> {
    let key = recovery_key::parse(recovery_key_text)?;
    if !backup.is_absolute() || !backup.is_file() {
        return Err(AppError::Validation("O arquivo de backup não foi encontrado.".into()));
    }
    let secret = SecretKey::new(key.clone());
    let info = validate_file(backup, &secret).map_err(|e| match e {
        AppError::Validation(_) => AppError::Validation("A chave de recuperação não abre este backup, ou o arquivo está danificado.".into()),
        other => other,
    })?;
    if info.schema_version > latest_schema_version() {
        return Err(AppError::Validation("Este backup foi feito por uma versão mais nova do Boreal. Instale a versão mais nova antes de restaurar.".into()));
    }
    Ok(key)
}

/// Copia o backup para o lugar do banco (arquivo temporário + renomear).
fn place_database(backup: &Path, db_path: &Path) -> Result<(), AppError> {
    let fail = || AppError::Validation("Não foi possível gravar o banco de dados neste computador. Nada foi alterado.".into());
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| fail())?;
    }
    let temp = db_path.with_extension("db.restaurando");
    std::fs::copy(backup, &temp).map_err(|_| {
        let _ = std::fs::remove_file(&temp);
        fail()
    })?;
    std::fs::rename(&temp, db_path).map_err(|_| {
        let _ = std::fs::remove_file(&temp);
        fail()
    })
}

#[derive(Clone)]
pub struct RecoveryService {
    ctx: Ctx,
}

impl RecoveryService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    /// Troca de computador: sem banco aqui, usa um backup e a chave de recuperação para criar o banco. A chave só
    /// vai para o cofre do Windows depois que o backup abriu com ela e o arquivo foi colocado no lugar.
    pub fn restore_on_new_computer(backup: &Path, recovery_key_text: &str, db_path: &Path, keys: &dyn KeyStore) -> Result<(), AppError> {
        if db_path.exists() {
            return Err(AppError::Validation("Já existe um banco de dados neste computador. Use a restauração dentro do sistema.".into()));
        }
        let key = check_backup_with_key(backup, recovery_key_text)?;
        place_database(backup, db_path)?;
        if keys.set(&key).is_err() {
            let _ = std::fs::remove_file(db_path);
            return Err(AppError::KeyUnavailable);
        }
        Ok(())
    }

    /// Banco com problema de integridade: troca o arquivo pelo backup, guardando o antigo ao lado
    /// (`boreal-com-problema-…`). Exige a chave de recuperação e a confirmação digitada.
    pub fn recover_corrupted(backup: &Path, recovery_key_text: &str, confirmation: &str, db_path: &Path, keys: &dyn KeyStore, now_local: chrono::NaiveDateTime) -> Result<(), AppError> {
        if confirmation.trim().to_uppercase() != "RESTAURAR" {
            return Err(AppError::Validation("Digite RESTAURAR para confirmar.".into()));
        }
        let key = check_backup_with_key(backup, recovery_key_text)?;
        if let Some(stored) = keys.get()?
            && stored != key
        {
            return Err(AppError::Validation("A chave de recuperação não é a deste computador.".into()));
        }
        let parent = db_path.parent().unwrap_or_else(|| Path::new("."));
        let saved = parent.join(format!("boreal-com-problema-{}.db", now_local.format("%Y-%m-%d-%H%M%S")));
        if db_path.exists() {
            std::fs::rename(db_path, &saved).map_err(|_| AppError::Validation("Não foi possível guardar uma cópia do banco com problema. Nada foi alterado.".into()))?;
        }
        for suffix in ["-wal", "-shm"] {
            let name = db_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let _ = std::fs::remove_file(db_path.with_file_name(format!("{name}{suffix}")));
        }
        if let Err(error) = place_database(backup, db_path) {
            let _ = std::fs::rename(&saved, db_path);
            return Err(error);
        }
        if keys.get()?.is_none() {
            keys.set(&key)?;
        }
        Ok(())
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
        self.ctx.log.info("chave.exibida", "o administrador consultou a chave de recuperação");
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

#[cfg(test)]
mod new_computer_tests {
    use std::path::PathBuf;

    use super::*;
    use crate::db::keystore::{KeyStore, MemoryKeyStore};
    use crate::services::backup_service::BackupService;
    use crate::services::test_support::{disk_world, DiskWorld, TEST_KEY};

    fn backup_of(w: &DiskWorld) -> PathBuf {
        {
            let conn = w.ctx.conn().unwrap();
            conn.execute(
                "INSERT INTO guests (code, full_name, name_key, phone, created_at, updated_at) VALUES ('HOS-000001', 'Ana Souza', 'ana souza', '11999990000', '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z')",
                [],
            )
            .unwrap();
        }
        let run = BackupService::new(w.ctx.clone(), w.db_path.clone()).run(&w.admin).unwrap();
        w.db_path.parent().unwrap().join("backups").join(run.file_name.unwrap())
    }

    fn key_text() -> String {
        recovery_key::format(&TEST_KEY)
    }

    fn local(hour: u32) -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 22).unwrap().and_hms_opt(hour, 0, 0).unwrap()
    }

    #[test]
    fn new_computer_restore_end_to_end() {
        let w = disk_world();
        let backup = backup_of(&w);
        let elsewhere = tempfile::tempdir().unwrap();
        let new_db = elsewhere.path().join("dados").join("boreal.db");
        let keys = MemoryKeyStore::empty();

        RecoveryService::restore_on_new_computer(&backup, &key_text(), &new_db, &keys).unwrap();

        assert_eq!(keys.get().unwrap().unwrap(), TEST_KEY.to_vec(), "a chave foi para o cofre do novo computador");
        let conn = crate::db::open_database(&new_db, &keys).unwrap();
        let name: String = conn.query_row("SELECT full_name FROM guests", [], |r| r.get(0)).unwrap();
        assert_eq!(name, "Ana Souza");
        let app = crate::app::initialize(&new_db, &keys);
        assert!(app.is_ok(), "o aplicativo abre com o banco restaurado");
    }

    #[test]
    fn a_wrong_key_creates_nothing_and_stores_nothing() {
        let w = disk_world();
        let backup = backup_of(&w);
        let elsewhere = tempfile::tempdir().unwrap();
        let new_db = elsewhere.path().join("boreal.db");
        let keys = MemoryKeyStore::empty();
        let other = recovery_key::format(&[9u8; 32]);

        let wrong = RecoveryService::restore_on_new_computer(&backup, &other, &new_db, &keys);
        let typo = RecoveryService::restore_on_new_computer(&backup, "ABCD", &new_db, &keys);

        assert!(matches!(&wrong, Err(AppError::Validation(m)) if m.contains("não abre este backup")), "{wrong:?}");
        assert!(matches!(&typo, Err(AppError::Validation(m)) if m.contains("incompleta")), "{typo:?}");
        assert!(!new_db.exists() && keys.get().unwrap().is_none());
    }

    #[test]
    fn new_computer_restore_refuses_when_a_database_already_exists() {
        let w = disk_world();
        let backup = backup_of(&w);
        let keys = MemoryKeyStore::with_key(&TEST_KEY);
        let before = std::fs::read(&w.db_path).unwrap();

        let result = RecoveryService::restore_on_new_computer(&backup, &key_text(), &w.db_path, &keys);

        assert!(matches!(&result, Err(AppError::Validation(m)) if m.contains("Já existe um banco")), "{result:?}");
        assert_eq!(std::fs::read(&w.db_path).unwrap(), before);
    }

    #[test]
    fn a_corrupted_database_is_recovered_with_the_key_and_the_typed_confirmation_keeping_the_bad_file() {
        let w = disk_world();
        let backup = backup_of(&w);
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("boreal.db");
        std::fs::write(&db, vec![3u8; 16_384]).unwrap();
        let keys = MemoryKeyStore::with_key(&TEST_KEY);

        let refused = RecoveryService::recover_corrupted(&backup, &key_text(), "talvez", &db, &keys, local(10));
        assert!(matches!(refused, Err(AppError::Validation(_))));
        assert_eq!(std::fs::read(&db).unwrap(), vec![3u8; 16_384], "sem confirmação nada muda");

        RecoveryService::recover_corrupted(&backup, &key_text(), "RESTAURAR", &db, &keys, local(10)).unwrap();

        assert!(dir.path().join("boreal-com-problema-2026-10-22-100000.db").exists(), "o banco com problema fica guardado");
        let conn = crate::db::open_database(&db, &keys).unwrap();
        let guests: i64 = conn.query_row("SELECT count(*) FROM guests", [], |r| r.get(0)).unwrap();
        assert_eq!(guests, 1);
    }

    #[test]
    fn recovery_refuses_a_key_that_differs_from_the_one_in_this_computer() {
        let w = disk_world();
        let backup = backup_of(&w);
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("boreal.db");
        std::fs::write(&db, b"x").unwrap();
        let keys = MemoryKeyStore::with_key(&[5u8; 32]);

        let result = RecoveryService::recover_corrupted(&backup, &key_text(), "RESTAURAR", &db, &keys, local(10));

        assert!(matches!(&result, Err(AppError::Validation(m)) if m.contains("não é a deste computador")), "{result:?}");
        assert_eq!(std::fs::read(&db).unwrap(), b"x");
    }
}
