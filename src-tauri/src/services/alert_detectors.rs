//! Detectores de alertas: cada um olha o estado atual e devolve achados; o motor (`AlertService::sync`)
//! cria, atualiza e resolve os alertas. Os detectores só leem, nunca alteram o sistema.

use chrono::{Duration, NaiveDate, NaiveDateTime};
use rusqlite::Connection;

use super::reservation_service::{br, parse_date, parse_time};
use super::stay_service::StayService;
use crate::domain::alert::{AlertKind, Finding};
use crate::error::AppError;
use crate::models::settings::AppSettings;
use crate::repositories::{reservation_repo, stay_repo};

fn finding(kind: AlertKind, subject_key: String, title: String, detail: String, link: String) -> Finding {
    Finding { kind, subject_key, title, detail, link }
}

/// Todos os achados do momento, de todos os detectores.
pub(crate) fn collect_all(conn: &Connection, today: NaiveDate, now_local: NaiveDateTime, settings: &AppSettings) -> Result<Vec<Finding>, AppError> {
    collect_stay_findings(conn, today, now_local, settings)
}

/// Chegadas, sinais e saídas: `arrival_today`, `arrival_overdue`, `deposit_overdue`, `departure_today`,
/// `departure_late` e `departure_risk`.
pub(crate) fn collect_stay_findings(
    conn: &Connection,
    today: NaiveDate,
    now_local: NaiveDateTime,
    settings: &AppSettings,
) -> Result<Vec<Finding>, AppError> {
    let mut out = Vec::new();

    for r in reservation_repo::review_candidates(conn, today)? {
        let date = parse_date(&r.check_in_date, "Data de entrada")?;
        let time = parse_time(&r.expected_check_in_time).ok_or_else(|| AppError::Database("horário previsto inválido na reserva".into()))?;
        let overdue = now_local >= date.and_time(time) + Duration::minutes(i64::from(settings.no_show_tolerance_minutes));
        let who = format!("{}, quarto {}", r.guest_name, r.room_code);
        let link = format!("/reservas/{}", r.id);
        let key = format!("reservation:{}", r.id);

        if r.deposit_overdue {
            out.push(finding(
                AlertKind::DepositOverdue,
                key.clone(),
                format!("Sinal vencido: {who}"),
                format!("O prazo era {}. Receba o sinal ou decida sobre a reserva.", br(&r.deposit_due_date)),
                link.clone(),
            ));
        }
        if overdue && date <= today {
            out.push(finding(
                AlertKind::ArrivalOverdue,
                key,
                format!("Chegada atrasada: {who}"),
                format!("Previsto para {} às {}. Faça o check-in ou marque no-show.", br(&r.check_in_date), r.expected_check_in_time),
                link,
            ));
        } else if date == today {
            out.push(finding(
                AlertKind::ArrivalToday,
                key,
                format!("Chegada hoje: {who}"),
                format!("Previsto às {}. Faça o check-in quando a pessoa chegar.", r.expected_check_in_time),
                link,
            ));
        }
    }

    for stay in stay_repo::list(conn, None, "in_house", 1000, 0)?.0 {
        let (late, risk) = StayService::assess_departure(conn, &stay, settings, now_local)?;
        let who = format!("{}, quarto {}", stay.guest_name, stay.room_code);
        let link = format!("/hospedagens/{}", stay.id);
        let key = format!("stay:{}", stay.id);

        if late.minutes_late > i64::from(late.tolerance_minutes) {
            out.push(finding(
                AlertKind::DepartureLate,
                key.clone(),
                format!("Saída atrasada: {who}"),
                format!(
                    "{} min depois do previsto ({} às {}); tolerância de {} min. Faça o check-out ou combine a saída tardia.",
                    late.minutes_late,
                    br(&stay.check_out_date),
                    stay.expected_check_out_time,
                    late.tolerance_minutes
                ),
                link.clone(),
            ));
        } else if parse_date(&stay.check_out_date, "Data de saída")? == today {
            out.push(finding(
                AlertKind::DepartureToday,
                key.clone(),
                format!("Saída hoje: {who}"),
                format!("Previsto às {}. Confira a conta antes do check-out.", stay.expected_check_out_time),
                link.clone(),
            ));
        }
        if let Some(risk) = risk {
            out.push(finding(
                AlertKind::DepartureRisk,
                key,
                format!("Risco para a próxima reserva: quarto {}", stay.room_code),
                format!("A saída atrasada pode atrasar a chegada da reserva {} (≈ {} min).", risk.next_code, risk.overrun_minutes),
                link,
            ));
        }
    }
    Ok(out)
}


#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::error::AppError;
    use crate::models::alert::{AlertQuery, AlertView};
    use crate::models::reservation::NewReservation;
    use crate::models::stay::{CheckInInput, CheckoutInput, LateFeeChoice, OccupantInput};
    use crate::services::test_support::{admin_token, deposit, harness, pay, seed, Harness, Seed};

    // Fuso -03:00. Diária R$ 200,00; um quarto (101). Check-in padrão 14:00, saída 12:00; tolerâncias 60 min (no-show) e 30 min (saída).

    struct World {
        h: Harness,
        admin: String,
        seed: Seed,
    }

    fn world() -> World {
        let h = harness();
        let admin = admin_token(&h);
        let seed = seed(&h, &admin, 20_000, 2);
        World { h, admin, seed }
    }

    fn at(w: &World, rfc3339: &str) {
        w.h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    fn book(w: &World, from: &str, to: &str, in_time: Option<&str>) -> i64 {
        w.h.reservations
            .create(
                &w.admin,
                NewReservation {
                    room_id: w.seed.room_id,
                    guest_id: w.seed.guest_id,
                    check_in_date: from.into(),
                    check_out_date: to.into(),
                    expected_check_in_time: in_time.map(str::to_string),
                    expected_check_out_time: None,
                    occupants: 1,
                    notes: String::new(),
                    exceed_capacity: false,
                },
            )
            .unwrap()
            .id
    }

    fn check_in(w: &World, reservation: i64) -> i64 {
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        w.h.stays.check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false }).unwrap()
    }

    fn check_out(w: &World, stay: i64) -> Result<(), AppError> {
        let version = w.h.stays.get(&w.admin, stay).unwrap().stay.version;
        w.h.stays.checkout(&w.admin, CheckoutInput { stay_id: stay, expected_version: version, late_fee: LateFeeChoice::NotApplicable, balance_exception: None })
    }

    fn active(w: &World) -> Vec<AlertView> {
        w.h.alerts.refresh(&w.admin).unwrap();
        w.h.alerts.list(&w.admin, AlertQuery { status: Some("active".into()), kind: None, severity: None, page: 1, page_size: 100 }).unwrap().items
    }

    fn kinds(alerts: &[AlertView]) -> Vec<&str> {
        let mut k: Vec<&str> = alerts.iter().map(|a| a.kind.as_str()).collect();
        k.sort_unstable();
        k
    }

    /// Os tipos de chegada e saída, sem o sinal vencido (uma reserva criada na véspera já nasce com o sinal vencendo).
    fn movement_kinds(alerts: &[AlertView]) -> Vec<&str> {
        kinds(alerts).into_iter().filter(|k| *k != "deposit_overdue").collect()
    }

    fn count(w: &World, sql: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn reservation_arriving_today_without_check_in_raises_arrival_today() {
        let w = world();
        let id = book(&w, "2026-10-21", "2026-10-23", None);

        let alerts = active(&w);

        assert_eq!(kinds(&alerts), ["arrival_today"]);
        assert!(alerts[0].title.contains("Ana Souza") && alerts[0].title.contains("101"), "{}", alerts[0].title);
        assert!(alerts[0].detail.contains("14:00"), "{}", alerts[0].detail);
        assert_eq!(alerts[0].link, format!("/reservas/{id}"));

        at(&w, "2026-10-21T17:00:00Z");
        check_in(&w, id);
        assert!(!kinds(&active(&w)).contains(&"arrival_today"), "some depois do check-in");
    }

    #[test]
    fn cancelled_reservation_does_not_raise_arrival() {
        let w = world();
        let id = book(&w, "2026-10-21", "2026-10-23", None);
        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;
        w.h.reservations.cancel(&w.admin, id, version, None).unwrap();

        assert!(kinds(&active(&w)).is_empty());
    }

    #[test]
    fn stay_leaving_today_raises_departure_today_and_clears_on_check_out() {
        let w = world();
        let id = book(&w, "2026-10-21", "2026-10-23", None);
        at(&w, "2026-10-21T17:00:00Z");
        let stay = check_in(&w, id);
        let balance = w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents;
        pay(&w.h, &w.admin, stay, balance, "pix").unwrap();
        at(&w, "2026-10-23T12:00:00Z"); // 09:00 do dia da saída

        let alerts = active(&w);

        assert_eq!(kinds(&alerts), ["departure_today"]);
        assert_eq!(alerts[0].link, format!("/hospedagens/{stay}"));
        assert!(alerts[0].detail.contains("12:00"));
        check_out(&w, stay).unwrap();
        assert!(kinds(&active(&w)).is_empty());
    }

    #[test]
    fn arrival_at_23_30_local_is_still_today_even_though_it_is_next_day_in_utc() {
        let w = world();
        book(&w, "2026-10-22", "2026-10-24", Some("23:55"));
        at(&w, "2026-10-23T02:30:00Z"); // 22/10 23:30 local

        assert_eq!(movement_kinds(&active(&w)), ["arrival_today"]);
    }

    #[test]
    fn next_day_without_check_in_turns_arrival_today_into_arrival_overdue() {
        let w = world();
        let id = book(&w, "2026-10-22", "2026-10-24", Some("23:55"));
        at(&w, "2026-10-23T02:30:00Z");
        assert_eq!(movement_kinds(&active(&w)), ["arrival_today"]);

        at(&w, "2026-10-23T05:00:00Z"); // 23/10 02:00 local: depois das 23:55 + 60 min de tolerância

        let alerts = active(&w);
        assert_eq!(movement_kinds(&alerts), ["arrival_overdue"]);
        assert_eq!(alerts.iter().find(|a| a.kind == "arrival_overdue").unwrap().link, format!("/reservas/{id}"));
        let history = w.h.alerts.list(&w.admin, AlertQuery { status: Some("history".into()), kind: Some("arrival_today".into()), severity: None, page: 1, page_size: 10 }).unwrap();
        assert_eq!((history.total, history.items[0].status.as_str()), (1, "resolved"));
    }

    #[test]
    fn unpaid_deposit_past_due_raises_deposit_overdue_and_clears_when_paid() {
        let w = world();
        let id = book(&w, "2026-10-26", "2026-10-28", None); // sinal vence em 24/10
        assert!(!kinds(&active(&w)).contains(&"deposit_overdue"));

        at(&w, "2026-10-25T12:00:00Z");
        let alerts = active(&w);
        assert!(kinds(&alerts).contains(&"deposit_overdue"), "{:?}", kinds(&alerts));
        let alert = alerts.iter().find(|a| a.kind == "deposit_overdue").unwrap();
        assert!(alert.detail.contains("24/10/2026"), "{}", alert.detail);

        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;
        deposit(&w.h, &w.admin, id, 12_000, version).unwrap();
        assert!(!kinds(&active(&w)).contains(&"deposit_overdue"));
    }

    #[test]
    fn stay_past_departure_tolerance_raises_departure_late_with_the_minutes() {
        let w = world();
        let id = book(&w, "2026-10-21", "2026-10-23", None);
        at(&w, "2026-10-21T17:00:00Z");
        check_in(&w, id);

        at(&w, "2026-10-23T15:20:00Z"); // 12:20 local: dentro da tolerância de 30 min
        assert_eq!(kinds(&active(&w)), ["departure_today"]);

        at(&w, "2026-10-23T16:00:00Z"); // 13:00 local: 60 min depois
        let alerts = active(&w);
        assert_eq!(kinds(&alerts), ["departure_late"]);
        assert!(alerts[0].detail.contains("60 min"), "{}", alerts[0].detail);
    }

    #[test]
    fn late_departure_that_delays_the_next_reservation_raises_a_critical_departure_risk() {
        let w = world();
        let id = book(&w, "2026-10-21", "2026-10-23", None);
        book(&w, "2026-10-23", "2026-10-25", None); // chega às 14:00, no mesmo quarto
        at(&w, "2026-10-21T17:00:00Z");
        let stay = check_in(&w, id);

        at(&w, "2026-10-23T17:30:00Z"); // 14:30 local, 150 min depois do previsto

        let alerts = active(&w);
        let risk = alerts.iter().find(|a| a.kind == "departure_risk").expect("risco");
        assert_eq!((risk.severity.as_str(), risk.link.as_str()), ("critical", format!("/hospedagens/{stay}").as_str()));
        assert!(risk.detail.contains("RES-"), "{}", risk.detail);
        assert!(alerts.iter().any(|a| a.kind == "departure_late"));
    }

    #[test]
    fn refresh_is_idempotent() {
        let w = world();
        book(&w, "2026-10-21", "2026-10-23", None);
        active(&w);
        let before = (count(&w, "SELECT count(*) FROM alerts"), count(&w, "SELECT count(*) FROM alert_events"));

        active(&w);
        active(&w);

        assert_eq!((count(&w, "SELECT count(*) FROM alerts"), count(&w, "SELECT count(*) FROM alert_events")), before);
    }
}
