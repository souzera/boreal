//! Nome dos arquivos de backup e política de retenção (spec §15.1). Funções puras.

use std::collections::BTreeMap;

use chrono::{NaiveDate, NaiveDateTime};

const PREFIX: &str = "boreal-";
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

/// Só os arquivos criados pelo sistema (nome no padrão exato) são tratados como backups.
pub fn is_backup_name(name: &str) -> bool {
    parse_datetime(name).is_some()
}

pub fn parse_datetime(name: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(stamp(name)?, "%Y-%m-%d-%H%M%S").ok()
}

pub fn parse_date(name: &str) -> Option<NaiveDate> {
    parse_datetime(name).map(|d| d.date())
}

/// Quais arquivos apagar: mantém o mais novo de cada um dos `days` dias mais recentes que têm backup.
/// Nomes fora do padrão nunca entram na lista (a pasta pode ter outros arquivos).
pub fn retain(names: &[String], days: usize) -> Vec<String> {
    let mut by_day: BTreeMap<NaiveDate, Vec<&String>> = BTreeMap::new();
    for name in names {
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
