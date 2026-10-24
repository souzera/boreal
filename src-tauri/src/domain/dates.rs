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

#[cfg(test)]
mod tests {
    use super::{format, parse};
    use chrono::NaiveDate;

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
