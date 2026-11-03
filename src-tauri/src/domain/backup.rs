//! Nome dos arquivos de backup e política de retenção (spec §15.1). Funções puras.

use std::collections::BTreeMap;

use chrono::{NaiveDate, NaiveDateTime};

const PREFIX: &str = "boreal-";
/// Cópias de segurança (antes de restaurar ou atualizar): fora da retenção diária, para nunca apagarem um backup.
const SAFETY_PREFIX: &str = "boreal-seguranca-";
const EXT: &str = ".bak";
/// `AAAA-MM-DD-HHMMSS`
const STAMP_LEN: usize = 17;

/// `boreal-2026-10-21-143005.bak` (hora local).
pub fn file_name(local: NaiveDateTime) -> String {
    format!("{PREFIX}{}{EXT}", local.format("%Y-%m-%d-%H%M%S"))
}

fn stamp(name: &str) -> Option<&str> {
    let middle = name.strip_prefix(PREFIX)?.strip_suffix(EXT)?;
    (middle.len() == STAMP_LEN).then_some(middle)
}

/// `boreal-seguranca-2026-10-21-143005.bak` (cópia do estado de antes de restaurar ou atualizar).
pub fn safety_file_name(local: NaiveDateTime) -> String {
    format!("{SAFETY_PREFIX}{}{EXT}", local.format("%Y-%m-%d-%H%M%S"))
}

fn safety_stamp(name: &str) -> Option<&str> {
    let middle = name.strip_prefix(SAFETY_PREFIX)?.strip_suffix(EXT)?;
    (middle.len() == STAMP_LEN).then_some(middle)
}

fn parse_stamp(text: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%d-%H%M%S").ok()
}

/// Só os backups diários criados pelo sistema (nome no padrão exato) entram na retenção.
pub fn is_backup_name(name: &str) -> bool {
    stamp(name).and_then(parse_stamp).is_some()
}

pub fn is_safety_name(name: &str) -> bool {
    safety_stamp(name).and_then(parse_stamp).is_some()
}

/// Data e hora do nome, de um backup diário ou de uma cópia de segurança.
pub fn parse_datetime(name: &str) -> Option<NaiveDateTime> {
    stamp(name).or_else(|| safety_stamp(name)).and_then(parse_stamp)
}

pub fn parse_date(name: &str) -> Option<NaiveDate> {
    parse_datetime(name).map(|d| d.date())
}

/// Cópias de segurança a apagar: guarda as `keep` mais novas.
pub fn retain_safety(names: &[String], keep: usize) -> Vec<String> {
    let mut safety: Vec<&String> = names.iter().filter(|n| is_safety_name(n)).collect();
    safety.sort();
    let drop = safety.len().saturating_sub(keep);
    safety.into_iter().take(drop).cloned().collect()
}

/// Quais arquivos apagar: mantém o mais novo de cada um dos `days` dias mais recentes que têm backup.
/// Nomes fora do padrão nunca entram na lista (a pasta pode ter outros arquivos).
pub fn retain(names: &[String], days: usize) -> Vec<String> {
    let mut by_day: BTreeMap<NaiveDate, Vec<&String>> = BTreeMap::new();
    for name in names.iter().filter(|n| is_backup_name(n)) {
        if let Some(day) = parse_date(name) {
            by_day.entry(day).or_default().push(name);
        }
    }
    let mut delete = Vec::new();
    for (index, (_, mut files)) in by_day.into_iter().rev().enumerate() {
        files.sort();
        let newest = files.pop();
        if index >= days {
            delete.extend(newest.cloned());
        }
        delete.extend(files.into_iter().cloned());
    }
    delete.sort();
    delete
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(date: &str, time: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&format!("{date} {time}"), "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn names_use_the_local_time_and_round_trip() {
        let name = file_name(at("2026-10-21", "14:30:05"));

        assert_eq!(name, "boreal-2026-10-21-143005.bak");
        assert_eq!(parse_datetime(&name), Some(at("2026-10-21", "14:30:05")));
        assert_eq!(parse_date(&name), NaiveDate::from_ymd_opt(2026, 10, 21));
    }

    #[test]
    fn only_exact_pattern_names_are_backups() {
        assert!(is_backup_name("boreal-2026-10-21-143005.bak"));
        for bad in [
            "boreal-2026-10-21-143005.bak.partial",
            "boreal-2026-10-21.bak",
            "boreal-2026-13-21-143005.bak",
            "outro-2026-10-21-143005.bak",
            "boreal-2026-10-21-143005.zip",
            "relatorio.pdf",
            "",
        ] {
            assert!(!is_backup_name(bad), "{bad}");
        }
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn safety_copies_have_their_own_names_and_never_count_as_daily_backups() {
        let safety = safety_file_name(at("2026-10-21", "14:30:05"));

        assert_eq!(safety, "boreal-seguranca-2026-10-21-143005.bak");
        assert!(is_safety_name(&safety) && !is_backup_name(&safety));
        assert!(!is_safety_name("boreal-2026-10-21-143005.bak"));
        assert_eq!(parse_datetime(&safety), Some(at("2026-10-21", "14:30:05")));
        let all = names(&["boreal-2026-10-21-090000.bak", &safety]);
        assert!(retain(&all, 7).is_empty(), "a cópia de segurança não entra na retenção diária");
    }

    #[test]
    fn only_the_five_newest_safety_copies_are_kept() {
        let files: Vec<String> = (1..=7).map(|d| format!("boreal-seguranca-2026-10-{d:02}-100000.bak")).chain(["boreal-2026-10-01-100000.bak".to_string()]).collect();

        let delete = retain_safety(&files, 5);

        assert_eq!(delete, ["boreal-seguranca-2026-10-01-100000.bak", "boreal-seguranca-2026-10-02-100000.bak"]);
    }

    #[test]
    fn retention_keeps_the_newest_of_each_of_the_last_seven_days() {
        let files = names(&[
            "boreal-2026-10-21-090000.bak",
            "boreal-2026-10-21-180500.bak",
            "boreal-2026-10-21-120000.bak",
        ]);

        let delete = retain(&files, 7);

        assert_eq!(delete, ["boreal-2026-10-21-090000.bak", "boreal-2026-10-21-120000.bak"]);
    }

    #[test]
    fn retention_drops_the_oldest_days_beyond_seven() {
        let files: Vec<String> = (1..=9).map(|d| format!("boreal-2026-10-{d:02}-180000.bak")).collect();

        let delete = retain(&files, 7);

        assert_eq!(delete, ["boreal-2026-10-01-180000.bak", "boreal-2026-10-02-180000.bak"]);
    }

    #[test]
    fn retention_ignores_other_files_and_does_not_count_missing_days() {
        let files = names(&["boreal-2026-10-01-180000.bak", "boreal-2026-10-20-180000.bak", "notas.txt", "boreal-2026-10-20-180000.bak.partial"]);

        let delete = retain(&files, 7);

        assert!(delete.is_empty(), "dois dias com backup cabem nos sete: {delete:?}");
    }
}
