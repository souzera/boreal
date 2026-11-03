use chrono::NaiveDate;

/// Data de calendário no formato exato `YYYY-MM-DD`; qualquer outra forma é recusada.
pub fn parse(text: &str) -> Option<NaiveDate> {
    if text.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

pub fn format(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Limites UTC (`[início, fim)`, RFC 3339) dos dias locais de `from` a `to` inclusive, para filtrar instantes gravados.
pub fn utc_bounds(from: NaiveDate, to: NaiveDate, offset: chrono::FixedOffset) -> (String, String) {
    let shift = chrono::Duration::seconds(i64::from(offset.local_minus_utc()));
    let start = from.and_hms_opt(0, 0, 0).unwrap_or_default() - shift;
    let end = to.succ_opt().unwrap_or(to).and_hms_opt(0, 0, 0).unwrap_or_default() - shift;
    let fmt = |t: chrono::NaiveDateTime| t.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    (fmt(start), fmt(end))
}

#[cfg(test)]
mod tests {
    use super::{format, parse, utc_bounds};
    use chrono::{FixedOffset, NaiveDate};

    #[test]
    fn utc_bounds_shift_local_days_by_the_offset() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
        let west3 = FixedOffset::west_opt(3 * 3600).unwrap();
        // 22/10 00:00 local (-03:00) = 22/10 03:00Z; o dia termina às 03:00Z de 23/10 (exclusivo)
        assert_eq!(utc_bounds(day, day, west3), ("2026-10-22T03:00:00Z".to_string(), "2026-10-23T03:00:00Z".to_string()));
        let next = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();
        assert_eq!(utc_bounds(day, next, west3).1, "2026-10-24T03:00:00Z");
    }

    #[test]
    fn parses_strict_iso_dates() {
        assert_eq!(parse("2026-10-24"), NaiveDate::from_ymd_opt(2026, 10, 24));
        assert_eq!(parse("2028-02-29"), NaiveDate::from_ymd_opt(2028, 2, 29));
    }

    #[test]
    fn rejects_invalid_or_loosely_formatted_dates() {
        for bad in ["2026-2-9", "24/10/2026", "2026-13-01", "2027-02-29", "2026-10-24T00:00:00Z", "", " 2026-10-24"] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn formats_back_to_iso() {
        assert_eq!(format(NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()), "2026-01-05");
    }
}
