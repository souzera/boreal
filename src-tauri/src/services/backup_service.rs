//! Backup criptografado e validado (spec §15.1): cópia do banco com a mesma chave, gravada num arquivo
//! parcial, conferida e só então renomeada; a retenção só apaga depois de um backup novo válido.
//! Falha de backup nunca bloqueia a recepção: vira registro no histórico e alerta.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags};
use serde_json::json;

use super::{format_utc, Ctx};
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::backup::{file_name, is_backup_name, parse_datetime, retain};
use crate::domain::secret::SecretKey;
use crate::error::AppError;
use crate::migrations::current_version;
use crate::models::backup::{BackupFileView, BackupRunView, DestinationView};
use crate::repositories::backup_repo::{self, NewRun, RunRow};
use crate::repositories::settings_repo;

const RETENTION_DAYS: usize = 7;
const SPACE_MARGIN: u64 = 64 * 1024 * 1024;

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupKind {
    Manual,
    Auto,
    PreMigration,
    PreRestore,
}

impl BackupKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackupKind::Manual => "manual",
            BackupKind::Auto => "auto",
            BackupKind::PreMigration => "pre_migration",
            BackupKind::PreRestore => "pre_restore",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BackupKind::Manual => "Manual",
            BackupKind::Auto => "Automático",
            BackupKind::PreMigration => "Antes de atualizar",
            BackupKind::PreRestore => "Antes de restaurar",
        }
    }

    pub fn parse(value: &str) -> Option<BackupKind> {
        [BackupKind::Manual, BackupKind::Auto, BackupKind::PreMigration, BackupKind::PreRestore].into_iter().find(|k| k.as_str() == value)
    }
}

/// O que a validação leu do arquivo de backup.
pub struct BackupInfo {
    pub schema_version: u32,
    pub audit_count: i64,
}

/// Abre um arquivo de backup só para leitura com a chave; chave errada ou arquivo que não é um banco cifrado
/// dão `KeyInvalid`.
pub(crate) fn open_with_key(path: &Path, key: &SecretKey) -> Result<Connection, AppError> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|_| AppError::KeyInvalid)?;
    conn.pragma_update(None, "key", format!("x'{}'", key.hex())).map_err(|_| AppError::KeyInvalid)?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0)).map_err(|_| AppError::KeyInvalid)?;
    Ok(conn)
}

/// Confere o arquivo: abre com a chave, `integrity_check` ok, `foreign_key_check` vazio.
pub(crate) fn validate_file(path: &Path, key: &SecretKey) -> Result<BackupInfo, AppError> {
    let conn = open_with_key(path, key).map_err(|_| invalid("O arquivo de backup não abre com a chave deste sistema."))?;
    let mut stmt = conn.prepare("PRAGMA integrity_check")?;
    let verdicts: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    if verdicts != ["ok"] {
        return Err(invalid("O backup não passou na verificação de integridade."));
    }
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    if stmt.query([])?.next()?.is_some() {
        return Err(invalid("O backup tem registros que não se conectam (verificação de vínculos)."));
    }
    drop(stmt);
    Ok(BackupInfo {
        schema_version: current_version(&conn)?,
        audit_count: conn.query_row("SELECT count(*) FROM audit_log", [], |r| r.get(0))?,
    })
}

struct Done {
    file_name: String,
    bytes: u64,
    schema_version: u32,
}

#[derive(Clone)]
pub struct BackupService {
    ctx: Ctx,
    db_path: PathBuf,
    /// Só nos testes: força a validação a falhar.
    #[cfg(test)]
    pub(crate) force_invalid: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl BackupService {
    pub fn new(ctx: Ctx, db_path: PathBuf) -> Self {
        Self {
            ctx,
            db_path,
            #[cfg(test)]
            force_invalid: std::sync::Arc::default(),
        }
    }

    fn dest_dir(&self, conn: &Connection) -> Result<PathBuf, AppError> {
        let configured = settings_repo::get(conn, "backup_dir")?.unwrap_or_default();
        if configured.trim().is_empty() {
            Ok(self.db_path.parent().map_or_else(|| PathBuf::from("backups"), |p| p.join("backups")))
        } else {
            Ok(PathBuf::from(configured))
        }
    }

    pub fn destination(&self, token: &str) -> Result<DestinationView, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        let conn = self.ctx.conn()?;
        let configured = settings_repo::get(&conn, "backup_dir")?.unwrap_or_default();
        Ok(DestinationView { path: self.dest_dir(&conn)?.to_string_lossy().to_string(), is_default: configured.trim().is_empty() })
    }

    /// Define a pasta de destino (caminho completo) ou volta ao padrão com texto vazio.
    pub fn set_destination(&self, token: &str, dir: &str) -> Result<DestinationView, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        let dir = dir.trim();
        if !dir.is_empty() && !Path::new(dir).is_absolute() {
            return Err(invalid("Escolha a pasta no seletor para informar o caminho completo."));
        }
        {
            let mut conn = self.ctx.conn()?;
            let tx = conn.transaction()?;
            settings_repo::set(&tx, "backup_dir", dir)?;
            audit::record(
                &tx,
                &AuditEntry {
                    at: self.ctx.utc.now_utc(),
                    user_id: Some(actor.user_id),
                    module: "backup",
                    operation: "set_destination",
                    entity: "backup_settings",
                    entity_id: None,
                    before: None,
                    after: Some(json!({ "custom": !dir.is_empty() })),
                    justification: None,
                },
            )?;
            tx.commit()?;
        }
        self.destination(token)
    }

    /// Backup manual (administrador).
    pub fn run(&self, token: &str) -> Result<BackupRunView, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        self.execute(BackupKind::Manual)
    }

    /// Backup sem sessão: usado pelo fechamento de caixa, pela abertura do aplicativo e antes de atualizar.
    pub fn run_system(&self, kind: BackupKind) -> Result<BackupRunView, AppError> {
        self.execute(kind)
    }

    /// Último backup válido (instante UTC), se houver.
    pub fn last_valid_at(&self) -> Result<Option<String>, AppError> {
        let conn = self.ctx.conn()?;
        backup_repo::last_valid_at(&conn)
    }

    pub fn recent_runs(&self, token: &str, limit: i64) -> Result<Vec<BackupRunView>, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        let conn = self.ctx.conn()?;
        Ok(backup_repo::recent_runs(&conn, limit.clamp(1, 100))?.iter().map(run_view).collect())
    }

    /// Arquivos de backup na pasta de destino, do mais novo para o mais antigo.
    pub fn list_files(&self, token: &str) -> Result<Vec<BackupFileView>, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        let dir = {
            let conn = self.ctx.conn()?;
            self.dest_dir(&conn)?
        };
        let mut files: Vec<BackupFileView> = match std::fs::read_dir(&dir) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let at = parse_datetime(&name)?;
                    let bytes = e.metadata().ok()?.len();
                    Some(BackupFileView { file_name: name, at: at.format("%d/%m/%Y %H:%M").to_string(), bytes })
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        files.sort_by(|a, b| b.file_name.cmp(&a.file_name));
        Ok(files)
    }

    fn execute(&self, kind: BackupKind) -> Result<BackupRunView, AppError> {
        let started = format_utc(self.ctx.utc.now_utc());
        let outcome = self.try_backup();
        let finished = format_utc(self.ctx.utc.now_utc());
        let conn = self.ctx.conn()?;
        match outcome {
            Ok(done) => {
                let id = backup_repo::insert_run(
                    &conn,
                    &NewRun {
                        kind: kind.as_str(),
                        status: "ok",
                        started_at: &started,
                        finished_at: &finished,
                        file_name: Some(&done.file_name),
                        bytes: i64::try_from(done.bytes).unwrap_or(i64::MAX),
                        schema_version: Some(i64::from(done.schema_version)),
                        message: "",
                    },
                )?;
                Ok(BackupRunView {
                    id,
                    kind: kind.as_str().into(),
                    kind_label: kind.label().into(),
                    status: "ok".into(),
                    finished_at: finished,
                    file_name: Some(done.file_name),
                    bytes: i64::try_from(done.bytes).unwrap_or(i64::MAX),
                    message: String::new(),
                })
            }
            Err(error) => {
                // O histórico guarda só a frase amigável, nunca detalhe técnico, caminho ou chave.
                let message = match &error {
                    AppError::Validation(m) => m.clone(),
                    _ => "Não foi possível concluir o backup.".to_string(),
                };
                backup_repo::insert_run(
                    &conn,
                    &NewRun { kind: kind.as_str(), status: "failed", started_at: &started, finished_at: &finished, file_name: None, bytes: 0, schema_version: None, message: &message },
                )?;
                Err(error)
            }
        }
    }

    fn try_backup(&self) -> Result<Done, AppError> {
        let dir = {
            let conn = self.ctx.conn()?;
            self.dest_dir(&conn)?
        };
        // Tamanho do arquivo do banco (o SQLCipher devolve `PRAGMA page_size` como texto, então não o usamos).
        let size = std::fs::metadata(&self.db_path).map(|m| m.len()).unwrap_or(0);
        let unreachable = || invalid("A pasta de backup não está acessível.");

        std::fs::create_dir_all(&dir).map_err(|_| unreachable())?;
        let probe = dir.join(".boreal-teste");
        std::fs::write(&probe, b"ok").map_err(|_| unreachable())?;
        let _ = std::fs::remove_file(&probe);

        let free = self.ctx.disk.free_bytes(&dir).map_err(|_| unreachable())?;
        if free < size.saturating_mul(2).saturating_add(SPACE_MARGIN) {
            return Err(invalid("Não há espaço suficiente na pasta de backup."));
        }

        let name = file_name(self.ctx.local_now());
        let target = dir.join(&name);
        let partial = dir.join(format!("{name}.partial"));
        if target.exists() {
            return Err(invalid("Já existe um backup com este horário. Tente de novo em instantes."));
        }

        let schema = match self.copy_and_validate(&partial) {
            Ok(schema) => schema,
            Err(error) => {
                remove_partial(&partial);
                return Err(error);
            }
        };
        if std::fs::rename(&partial, &target).is_err() {
            remove_partial(&partial);
            return Err(invalid("Não foi possível concluir o arquivo de backup."));
        }
        remove_partial(&partial);

        // Só depois de um backup novo e válido a retenção apaga os antigos (e só arquivos do padrão).
        let names: Vec<String> = std::fs::read_dir(&dir)
            .map(|entries| entries.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| is_backup_name(n)).collect())
            .unwrap_or_default();
        for old in retain(&names, RETENTION_DAYS) {
            let _ = std::fs::remove_file(dir.join(old));
        }

        let bytes = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
        Ok(Done { file_name: name, bytes, schema_version: schema })
    }

    /// Copia o banco para `partial` (criptografado com a mesma chave) e confere o resultado.
    fn copy_and_validate(&self, partial: &Path) -> Result<u32, AppError> {
        let copy_failed = |_| invalid("Não foi possível gravar o backup na pasta escolhida.");
        let (schema, audit) = {
            let source = self.ctx.conn()?;
            let mut target = Connection::open(partial).map_err(copy_failed)?;
            target.pragma_update(None, "key", format!("x'{}'", self.ctx.key.hex())).map_err(copy_failed)?;
            {
                let backup = Backup::new(&source, &mut target).map_err(copy_failed)?;
                backup.run_to_completion(256, Duration::ZERO, None).map_err(copy_failed)?;
            }
            // O banco de origem usa WAL e a cópia herda o modo; o backup deve ser um arquivo único e autocontido.
            target.execute_batch("PRAGMA journal_mode = DELETE;").map_err(copy_failed)?;
            // Lidos na mesma trava da cópia: nenhuma gravação entra entre a cópia e a contagem.
            (current_version(&source)?, source.query_row("SELECT count(*) FROM audit_log", [], |r| r.get::<_, i64>(0))?)
        };
        #[cfg(test)]
        if self.force_invalid.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(invalid("O backup foi gravado, mas não passou na validação."));
        }
        let info = validate_file(partial, &self.ctx.key)?;
        if info.schema_version != schema || info.audit_count != audit {
            return Err(invalid("O backup foi gravado, mas não confere com o banco. Tente de novo."));
        }
        Ok(schema)
    }
}

/// Apaga o arquivo parcial e qualquer auxiliar do SQLite deixado ao lado dele.
fn remove_partial(partial: &Path) {
    let name = partial.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let _ = std::fs::remove_file(partial);
    for suffix in ["-wal", "-shm", "-journal"] {
        let _ = std::fs::remove_file(partial.with_file_name(format!("{name}{suffix}")));
    }
}

fn run_view(r: &RunRow) -> BackupRunView {
    let kind = BackupKind::parse(&r.kind);
    BackupRunView {
        id: r.id,
        kind: r.kind.clone(),
        kind_label: kind.map_or_else(|| r.kind.clone(), |k| k.label().to_string()),
        status: r.status.clone(),
        finished_at: r.finished_at.clone(),
        file_name: r.file_name.clone(),
        bytes: r.bytes,
        message: r.message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::services::test_support::{disk_world, DiskWorld};

    const PLAIN_HEADER: &[u8] = b"SQLite format 3\0";

    fn service(w: &DiskWorld) -> BackupService {
        BackupService::new(w.ctx.clone(), w.db_path.clone())
    }

    fn backups_dir(w: &DiskWorld) -> PathBuf {
        w.db_path.parent().unwrap().join("backups")
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir).map(|e| e.map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
        names.sort();
        names
    }

    fn put_data(w: &DiskWorld) {
        let conn = w.ctx.conn().unwrap();
        for i in 1..=5 {
            conn.execute(
                "INSERT INTO guests (code, full_name, name_key, phone, created_at, updated_at) VALUES (?1, ?2, ?3, '11999990000', '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z')",
                rusqlite::params![format!("HOS-{i:06}"), format!("Hóspede {i}"), format!("hospede {i}")],
            )
            .unwrap();
        }
    }

    fn dump(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn.prepare(&format!("SELECT * FROM {table} ORDER BY 1")).unwrap();
        let columns = stmt.column_count();
        let rows: Vec<String> = stmt
            .query_map([], |r| Ok((0..columns).map(|i| format!("{:?}", r.get::<_, rusqlite::types::Value>(i).unwrap())).collect::<Vec<_>>().join("|")))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        rows
    }

    #[test]
    fn backup_creates_an_encrypted_valid_file_that_opens_only_with_the_key() {
        let w = disk_world();
        put_data(&w);

        let run = service(&w).run(&w.admin).unwrap();

        let file = backups_dir(&w).join(run.file_name.clone().unwrap());
        let bytes = std::fs::read(&file).unwrap();
        assert!(bytes.len() > 1024 && &bytes[..16] != PLAIN_HEADER, "o arquivo é criptografado");
        assert_eq!((run.status.as_str(), run.kind.as_str()), ("ok", "manual"));
        assert!(validate_file(&file, &w.ctx.key).is_ok());
        let wrong = SecretKey::new(vec![9; 32]);
        assert!(matches!(open_with_key(&file, &wrong), Err(AppError::KeyInvalid)));
        assert!(matches!(validate_file(&file, &wrong), Err(AppError::Validation(_))));
        assert_eq!(files(&backups_dir(&w)).len(), 1, "nenhum arquivo parcial sobra");
    }

    #[test]
    fn backup_holds_exactly_the_same_data_as_the_database() {
        let w = disk_world();
        put_data(&w);
        let run = service(&w).run(&w.admin).unwrap();
        let file = backups_dir(&w).join(run.file_name.unwrap());

        let copy = open_with_key(&file, &w.ctx.key).unwrap();
        let live = w.ctx.conn().unwrap();

        for table in ["guests", "settings", "audit_log", "schema_migrations", "room_categories", "users"] {
            assert_eq!(dump(&copy, table), dump(&live, table), "{table}");
        }
        assert_eq!(dump(&copy, "guests").len(), 5);
    }

    #[test]
    fn a_failed_validation_removes_the_partial_file_and_records_the_failure() {
        let w = disk_world();
        let svc = service(&w);
        svc.force_invalid.store(true, Ordering::SeqCst);

        let result = svc.run(&w.admin);

        assert!(matches!(&result, Err(AppError::Validation(m)) if m.contains("validação")), "{result:?}");
        assert!(files(&backups_dir(&w)).is_empty(), "nenhum arquivo (nem parcial) sobra");
        let runs = svc.recent_runs(&w.admin, 10).unwrap();
        assert_eq!((runs.len(), runs[0].status.as_str()), (1, "failed"));
        assert!(runs[0].message.contains("validação") && runs[0].file_name.is_none());
        assert!(svc.last_valid_at().unwrap().is_none());
    }

    fn old_files(dir: &Path, days: std::ops::RangeInclusive<u32>) {
        std::fs::create_dir_all(dir).unwrap();
        for d in days {
            std::fs::write(dir.join(format!("boreal-2026-10-{d:02}-180000.bak")), b"antigo").unwrap();
        }
    }

    #[test]
    fn retention_removes_old_files_only_after_a_new_valid_backup() {
        let w = disk_world();
        let dir = backups_dir(&w);
        old_files(&dir, 1..=8);
        let svc = service(&w);

        svc.force_invalid.store(true, Ordering::SeqCst);
        assert!(svc.run(&w.admin).is_err());
        assert_eq!(files(&dir).len(), 8, "backup novo inválido: nada é apagado");

        svc.force_invalid.store(false, Ordering::SeqCst);
        let run = svc.run(&w.admin).unwrap();

        let left = files(&dir);
        assert_eq!(left.len(), 7, "{left:?}");
        assert!(left.contains(&run.file_name.unwrap()));
        assert!(!left.contains(&"boreal-2026-10-01-180000.bak".to_string()) && !left.contains(&"boreal-2026-10-02-180000.bak".to_string()));
        assert!(left.contains(&"boreal-2026-10-03-180000.bak".to_string()));
    }

    #[test]
    fn retention_never_touches_files_outside_the_pattern() {
        let w = disk_world();
        let dir = backups_dir(&w);
        old_files(&dir, 1..=8);
        for other in ["notas.txt", "outro-2026-10-01-180000.bak", "boreal-2026-10-01-180000.bak.partial", "boreal-manual.bak"] {
            std::fs::write(dir.join(other), b"x").unwrap();
        }

        service(&w).run(&w.admin).unwrap();

        let left = files(&dir);
        for other in ["notas.txt", "outro-2026-10-01-180000.bak", "boreal-2026-10-01-180000.bak.partial", "boreal-manual.bak"] {
            assert!(left.contains(&other.to_string()), "{other} foi apagado: {left:?}");
        }
    }

    #[test]
    fn a_missing_folder_is_created_and_an_unusable_one_fails_with_a_friendly_message() {
        let w = disk_world();
        let svc = service(&w);
        let nested = w.dir.path().join("novo").join("backups");
        svc.set_destination(&w.admin, nested.to_str().unwrap()).unwrap();
        assert!(svc.run(&w.admin).is_ok());
        assert!(nested.is_dir());

        let blocker = w.dir.path().join("arquivo.txt");
        std::fs::write(&blocker, b"x").unwrap();
        svc.set_destination(&w.admin, blocker.join("sub").to_str().unwrap()).unwrap();
        let result = svc.run(&w.admin);

        assert!(matches!(&result, Err(AppError::Validation(m)) if m == "A pasta de backup não está acessível."), "{result:?}");
        let runs = svc.recent_runs(&w.admin, 10).unwrap();
        assert_eq!(runs[0].status, "failed");
        assert!(!runs[0].message.contains(w.dir.path().to_str().unwrap()), "a mensagem não mostra o caminho");
    }

    #[test]
    fn not_enough_space_fails_before_writing_anything() {
        let w = disk_world();
        w.disk.set(10);
        let svc = service(&w);

        let result = svc.run(&w.admin);

        assert!(matches!(&result, Err(AppError::Validation(m)) if m.contains("espaço suficiente")), "{result:?}");
        assert!(files(&backups_dir(&w)).is_empty());
    }

    #[test]
    fn reception_cannot_run_or_list_backups_but_the_system_can_back_up_without_a_session() {
        let w = disk_world();
        let svc = service(&w);

        assert!(matches!(svc.run(&w.reception), Err(AppError::Forbidden)));
        assert!(matches!(svc.list_files(&w.reception), Err(AppError::Forbidden)));
        assert!(matches!(svc.recent_runs(&w.reception, 5), Err(AppError::Forbidden)));
        assert!(matches!(svc.set_destination(&w.reception, ""), Err(AppError::Forbidden)));
        assert!(svc.run("invalido").is_err());
        let run = svc.run_system(BackupKind::Auto).unwrap();
        assert_eq!(run.kind, "auto");
        assert!(svc.last_valid_at().unwrap().is_some());
    }

    #[test]
    fn runs_are_listed_newest_first_and_the_history_is_immutable() {
        let w = disk_world();
        let svc = service(&w);
        svc.run(&w.admin).unwrap();
        w.utc.set(chrono::DateTime::parse_from_rfc3339("2026-10-22T12:00:00Z").unwrap().with_timezone(&chrono::Utc));
        svc.run_system(BackupKind::Auto).unwrap();

        let runs = svc.recent_runs(&w.admin, 10).unwrap();
        let listed = svc.list_files(&w.admin).unwrap();

        assert_eq!(runs.iter().map(|r| r.kind.as_str()).collect::<Vec<_>>(), ["auto", "manual"]);
        assert_eq!(listed.len(), 2);
        assert!(listed[0].at.starts_with("22/10/2026"), "{listed:?}");
        let conn = w.ctx.conn().unwrap();
        assert!(conn.execute("UPDATE backup_runs SET status = 'failed'", []).is_err());
        assert!(conn.execute("DELETE FROM backup_runs", []).is_err());
    }

    #[test]
    fn the_destination_defaults_next_to_the_database_and_changes_are_audited_without_the_path() {
        let w = disk_world();
        let svc = service(&w);

        let default = svc.destination(&w.admin).unwrap();
        assert!(default.is_default && default.path.ends_with("backups"), "{default:?}");
        let custom = w.dir.path().join("externo");
        let changed = svc.set_destination(&w.admin, custom.to_str().unwrap()).unwrap();
        assert!(!changed.is_default);
        assert!(matches!(svc.set_destination(&w.admin, "pasta/relativa"), Err(AppError::Validation(_))));
        assert!(svc.set_destination(&w.admin, "").unwrap().is_default);

        let conn = w.ctx.conn().unwrap();
        let after: String = conn.query_row("SELECT group_concat(after_json) FROM audit_log WHERE operation = 'set_destination'", [], |r| r.get(0)).unwrap();
        assert!(!after.contains("externo"), "{after}");
    }
}
