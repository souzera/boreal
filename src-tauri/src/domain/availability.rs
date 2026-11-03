//! Disponibilidade de um quarto (função pura, sem banco).
//!
//! Cada reserva ocupa o quarto de `check_in` + horário previsto de entrada até `check_out` +
//! horário previsto de saída. Entre uma saída e a próxima entrada precisa caber o intervalo de
//! limpeza. Isso cobre ao mesmo tempo a sobreposição de noites e a troca de hóspede no mesmo dia.

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};

/// Reserva que bloqueia o quarto (pendente, confirmada ou hospedada).
#[derive(Debug, Clone)]
pub struct Booking {
    pub id: i64,
    pub check_in: NaiveDate,
    pub check_out: NaiveDate,
    pub expected_in: NaiveTime,
    pub expected_out: NaiveTime,
}

impl Booking {
    fn start(&self) -> NaiveDateTime {
        self.check_in.and_time(self.expected_in)
    }

    fn end(&self) -> NaiveDateTime {
        self.check_out.and_time(self.expected_out)
    }
}

/// Ids das reservas existentes que conflitam com o período desejado, na ordem recebida.
/// `ignore_id` exclui a própria reserva quando ela está sendo alterada.
pub fn find_conflicts(
    existing: &[Booking],
    wanted_in: NaiveDate,
    wanted_in_time: NaiveTime,
    wanted_out: NaiveDate,
    wanted_out_time: NaiveTime,
    cleaning_minutes: u32,
    ignore_id: Option<i64>,
) -> Vec<i64> {
    let cleaning = Duration::minutes(i64::from(cleaning_minutes));
    let wanted_start = wanted_in.and_time(wanted_in_time);
    let wanted_end = wanted_out.and_time(wanted_out_time);

    existing
        .iter()
        .filter(|b| Some(b.id) != ignore_id)
        .filter(|b| {
            let wanted_before_existing = wanted_end + cleaning <= b.start();
            let wanted_after_existing = b.end() + cleaning <= wanted_start;
            !(wanted_before_existing || wanted_after_existing)
        })
        .map(|b| b.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{find_conflicts, Booking};
    use chrono::{NaiveDate, NaiveTime};

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn t(s: &str) -> NaiveTime {
        NaiveTime::parse_from_str(s, "%H:%M").unwrap()
    }

    fn booking(id: i64, from: &str, to: &str) -> Booking {
        Booking { id, check_in: d(from), check_out: d(to), expected_in: t("14:00"), expected_out: t("12:00") }
    }

    fn conflicts(existing: &[Booking], from: &str, in_time: &str, to: &str, out_time: &str, cleaning: u32) -> Vec<i64> {
        find_conflicts(existing, d(from), t(in_time), d(to), t(out_time), cleaning, None)
    }

    #[test]
    fn overlapping_nights_conflict() {
        let existing = [booking(1, "2026-10-10", "2026-10-13")];

        assert_eq!(conflicts(&existing, "2026-10-12", "14:00", "2026-10-14", "12:00", 60), [1]);
        assert_eq!(conflicts(&existing, "2026-10-08", "14:00", "2026-10-11", "12:00", 60), [1]);
    }

    #[test]
    fn adjacent_days_do_not_conflict() {
        let existing = [booking(1, "2026-10-10", "2026-10-13")];

        assert!(conflicts(&existing, "2026-10-14", "14:00", "2026-10-16", "12:00", 60).is_empty());
        assert!(conflicts(&existing, "2026-10-07", "14:00", "2026-10-09", "12:00", 60).is_empty());
    }

    #[test]
    fn same_day_turnover_is_allowed_when_cleaning_fits() {
        // sai 12:00 + 60 min de limpeza = 13:00 <= entrada 14:00
        let existing = [booking(1, "2026-10-10", "2026-10-13")];

        assert!(conflicts(&existing, "2026-10-13", "14:00", "2026-10-15", "12:00", 60).is_empty());
    }

    #[test]
    fn same_day_turnover_is_refused_when_cleaning_does_not_fit() {
        let existing = [booking(1, "2026-10-10", "2026-10-13")];

        assert_eq!(conflicts(&existing, "2026-10-13", "13:30", "2026-10-15", "12:00", 120), [1]);
        // exatamente no limite: 12:00 + 120 = 14:00
        assert!(conflicts(&existing, "2026-10-13", "14:00", "2026-10-15", "12:00", 120).is_empty());
        assert_eq!(conflicts(&existing, "2026-10-13", "13:59", "2026-10-15", "12:00", 120), [1]);
    }

    #[test]
    fn turnover_is_checked_in_both_directions() {
        // A nova reserva termina no dia em que a existente começa.
        let existing = [booking(1, "2026-10-13", "2026-10-16")];

        assert!(conflicts(&existing, "2026-10-10", "14:00", "2026-10-13", "12:00", 60).is_empty());
        // saída às 13:30 + 60 min = 14:30 > entrada existente 14:00
        assert_eq!(conflicts(&existing, "2026-10-10", "14:00", "2026-10-13", "13:30", 60), [1]);
    }

    #[test]
    fn one_night_booking_inside_another_conflicts() {
        let existing = [booking(1, "2026-10-10", "2026-10-15")];

        assert_eq!(conflicts(&existing, "2026-10-12", "14:00", "2026-10-13", "12:00", 60), [1]);
    }

    #[test]
    fn ignore_id_excludes_the_booking_being_changed() {
        let existing = [booking(1, "2026-10-10", "2026-10-13"), booking(2, "2026-10-20", "2026-10-22")];

        let found = find_conflicts(&existing, d("2026-10-11"), t("14:00"), d("2026-10-14"), t("12:00"), 60, Some(1));

        assert!(found.is_empty());
    }

    #[test]
    fn reports_every_conflicting_booking() {
        let existing = [booking(1, "2026-10-10", "2026-10-12"), booking(2, "2026-10-12", "2026-10-14"), booking(3, "2026-10-20", "2026-10-22")];

        assert_eq!(conflicts(&existing, "2026-10-11", "14:00", "2026-10-13", "12:00", 60), [1, 2]);
    }

    #[test]
    fn month_and_year_boundaries() {
        let existing = [booking(1, "2026-12-30", "2027-01-02")];

        assert_eq!(conflicts(&existing, "2027-01-01", "14:00", "2027-01-04", "12:00", 60), [1]);
        assert!(conflicts(&existing, "2027-01-02", "14:00", "2027-01-04", "12:00", 60).is_empty());
        assert!(conflicts(&existing, "2026-12-28", "14:00", "2026-12-30", "12:00", 60).is_empty());
    }

    #[test]
    fn leap_day_boundary() {
        let existing = [booking(1, "2028-02-27", "2028-02-29")];

        assert_eq!(conflicts(&existing, "2028-02-28", "14:00", "2028-03-01", "12:00", 60), [1]);
        assert!(conflicts(&existing, "2028-02-29", "14:00", "2028-03-02", "12:00", 60).is_empty());
    }
}
