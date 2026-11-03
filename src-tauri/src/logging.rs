//! Logs técnicos (spec §15.3): um arquivo por dia em `logs/`, separado da auditoria de negócio.
//! Nunca recebem senhas, chaves, caminhos completos nem dados de hóspedes; falhar ao escrever nunca atrapalha
//! quem chamou. Guarda 14 dias.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{Duration, NaiveDate};

const KEEP_DAYS: i64 = 14;
const MAX_LINE: usize = 500;
const PREFIX: &str = "boreal-";
const EXT: &str = ".log";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn text(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

/// Remove o que não pode ir para o log: caminhos completos (fica só o nome do arquivo), chaves hexadecimais
/// (32 caracteres ou mais) e o valor depois de `senha=`, `password=` ou `chave=`.
pub fn scrub(message: &str) -> String {
    let mut out = Vec::new();
    let mut hide_next = false;
    for token in message.split_whitespace() {
        let lower = token.to_lowercase();
        if hide_next {
            hide_next = false;
            out.push("[oculto]".to_string());
            continue;
        }
        if let Some((key, _)) = token.split_once('=')
            && matches!(key.to_lowercase().as_str(), "senha" | "password" | "chave" | "key")
        {
            out.push(format!("{key}=[oculto]"));
            continue;
        }
        if matches!(lower.trim_end_matches(':'), "senha" | "password" | "chave") && lower.ends_with(':') {
            hide_next = true;
            out.push(token.to_string());
            continue;
        }
        let hex_run = token.chars().filter(char::is_ascii_hexdigit).count() >= 32 && token.chars().all(|c| c.is_ascii_hexdigit() || c == '-' || c == '\'' || c == 'x');
        if hex_run {
            out.push("[oculto]".to_string());
        } else if token.contains('\\') || token.starts_with('/') || (token.len() > 2 && token.as_bytes()[1] == b':' && token.contains('/')) {
            let name = token.trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == '.').rsplit(['\\', '/']).next().unwrap_or("");
            out.push(if name.is_empty() { "[caminho]".to_string() } else { name.to_string() });
        } else {
            out.push(token.to_string());
        }
    }
    out.join(" ")
}

fn log_date(name: &str) -> Option<NaiveDate> {
    let middle = name.strip_prefix(PREFIX)?.strip_suffix(EXT)?;
    NaiveDate::parse_from_str(middle, "%Y-%m-%d").ok()
}

/// Apaga os arquivos de log com mais de `keep_days` dias; qualquer outro arquivo da pasta é ignorado.
pub fn rotate(dir: &Path, today: NaiveDate, keep_days: i64) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(day) = log_date(&name)
            && today - day > Duration::days(keep_days)
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Escritor de logs. Sem pasta (`disabled`) não faz nada.
pub struct Logger {
    dir: Option<PathBuf>,
    last_rotation: Mutex<Option<NaiveDate>>,
}

impl Logger {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir: Some(dir), last_rotation: Mutex::new(None) }
    }

    pub fn disabled() -> Self {
        Self { dir: None, last_rotation: Mutex::new(None) }
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// Uma linha por evento: `2026-10-21T18:05:00-03:00 INFO backup.ok mensagem`. Nunca falha.
    pub fn event(&self, level: Level, name: &str, message: &str) {
        let Some(dir) = &self.dir else { return };
        let now = chrono::Local::now();
        let _ = std::fs::create_dir_all(dir);
        {
            let mut last = self.last_rotation.lock().unwrap_or_else(|e| e.into_inner());
            if *last != Some(now.date_naive()) {
                rotate(dir, now.date_naive(), KEEP_DAYS);
                *last = Some(now.date_naive());
            }
        }
        let mut line = format!("{} {} {} {}", now.format("%Y-%m-%dT%H:%M:%S%:z"), level.text(), name, scrub(message).replace(['\r', '\n'], " "));
        if line.chars().count() > MAX_LINE {
            line = line.chars().take(MAX_LINE).collect();
        }
        let path = dir.join(format!("{PREFIX}{}{EXT}", now.format("%Y-%m-%d")));
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }

    pub fn info(&self, name: &str, message: &str) {
        self.event(Level::Info, name, message);
    }

    pub fn warn(&self, name: &str, message: &str) {
        self.event(Level::Warn, name, message);
    }

    pub fn error(&self, name: &str, message: &str) {
        self.event(Level::Error, name, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(dir: &Path) -> Vec<String> {
        let mut all = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
            all.extend(std::fs::read_to_string(entry.path()).unwrap().lines().map(str::to_string));
        }
        all
    }

    #[test]
    fn writes_one_line_per_event_in_the_daily_file() {
        let dir = tempfile::tempdir().unwrap();
        let log = Logger::new(dir.path().join("logs"));

        log.info("backup.ok", "manual boreal-2026-10-21-143005.bak 120 KB");
        log.warn("backup.falhou", "A pasta de backup não está acessível.");

        let files: Vec<String> = std::fs::read_dir(dir.path().join("logs")).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
        assert_eq!(files.len(), 1);
        assert!(files[0].starts_with("boreal-") && files[0].ends_with(".log"), "{files:?}");
        let written = lines(&dir.path().join("logs"));
        assert_eq!(written.len(), 2);
        assert!(written[0].contains(" INFO backup.ok manual boreal-2026-10-21-143005.bak 120 KB"), "{}", written[0]);
        assert!(written[1].contains(" WARN backup.falhou "), "{}", written[1]);
        let stamp = &written[0][..25];
        assert!(chrono::DateTime::parse_from_rfc3339(stamp).is_ok(), "hora local com fuso: {stamp}");
    }

    #[test]
    fn long_messages_are_cut_and_never_span_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log = Logger::new(dir.path().to_path_buf());

        log.error("x", &format!("linha um\nlinha dois {}", "a".repeat(2_000)));

        let written = lines(dir.path());
        assert_eq!(written.len(), 1);
        assert!(written[0].chars().count() <= MAX_LINE && written[0].contains("linha um linha dois"));
    }

    #[test]
    fn rotation_keeps_the_last_14_days_and_ignores_other_files() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["boreal-2026-10-01.log", "boreal-2026-10-07.log", "boreal-2026-10-08.log", "boreal-2026-10-21.log", "notas.txt", "boreal-manual.log", "boreal-2026-10-01.log.zip"] {
            std::fs::write(dir.path().join(name), "x").unwrap();
        }

        rotate(dir.path(), NaiveDate::from_ymd_opt(2026, 10, 21).unwrap(), 14);

        let mut left: Vec<String> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
        left.sort();
        // 01/10 tem 20 dias e sai; 07/10 tem exatamente 14 e fica; o que não é log do sistema nunca é tocado.
        assert_eq!(left, ["boreal-2026-10-01.log.zip", "boreal-2026-10-07.log", "boreal-2026-10-08.log", "boreal-2026-10-21.log", "boreal-manual.log", "notas.txt"]);
    }

    #[test]
    fn scrub_removes_full_paths_hex_keys_and_passwords() {
        let key = "2a".repeat(32);
        let scrubbed = scrub(&format!(
            "gravado em C:\\Users\\matheus\\AppData\\boreal\\backups\\boreal-2026-10-21-143005.bak e /home/ana/dados/boreal.db com chave x'{key}' senha=Admin-Senha-1 password: segredo fim"
        ));

        assert!(scrubbed.contains("boreal-2026-10-21-143005.bak") && scrubbed.contains("boreal.db"), "{scrubbed}");
        for forbidden in ["matheus", "AppData", "/home", &key, "Admin-Senha-1", "segredo"] {
            assert!(!scrubbed.contains(forbidden), "{forbidden} em {scrubbed}");
        }
        assert!(scrubbed.ends_with("fim"));
    }

    #[test]
    fn dates_with_slashes_are_not_mistaken_for_paths() {
        assert_eq!(scrub("Backup de 21/10/2026 14:30 concluído"), "Backup de 21/10/2026 14:30 concluído");
    }

    #[test]
    fn a_log_failure_never_breaks_the_caller() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("logs");
        std::fs::write(&blocker, b"arquivo no lugar da pasta").unwrap();
        let log = Logger::new(blocker);

        log.error("x", "isto não pode derrubar ninguém");
        Logger::disabled().info("x", "sem pasta, sem efeito");
    }
}
