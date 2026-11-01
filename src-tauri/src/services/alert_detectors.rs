//! Detectores de alertas: cada um olha o estado atual e devolve achados; o motor (`AlertService::sync`)
//! cria, atualiza e resolve os alertas. Os detectores só leem, nunca alteram o sistema.

use chrono::{Duration, NaiveDate, NaiveDateTime};
use rusqlite::Connection;

use super::reservation_service::{br, parse_date, parse_time};
use super::stay_service::StayService;
use crate::domain::alert::{AlertKind, Finding};
use crate::domain::cleaning::{suggest_priority, Priority};
use crate::domain::dates;
use crate::error::AppError;
use crate::models::settings::AppSettings;
use crate::repositories::payable_repo::{self, Filters as PayableFilters};
use crate::repositories::{cleaning_repo, closing_repo, maintenance_repo, reservation_repo, stay_repo};

fn finding(kind: AlertKind, subject_key: String, title: String, detail: String, link: String) -> Finding {
    Finding { kind, subject_key, title, detail, link }
}

/// `R$ 1.234,56`, para os textos dos alertas.
fn money_br(cents: i64) -> String {
    let abs = cents.unsigned_abs();
    let reais = (abs / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in reais.chars().enumerate() {
        if i > 0 && (reais.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    format!("R$ {grouped},{:02}", abs % 100)
}

/// Todos os achados do momento, de todos os detectores. `offset_seconds` é o deslocamento do fuso local.
pub(crate) fn collect_all(
    conn: &Connection,
    today: NaiveDate,
    now_local: NaiveDateTime,
    offset_seconds: i32,
    settings: &AppSettings,
) -> Result<Vec<Finding>, AppError> {
    let mut all = collect_stay_findings(conn, today, now_local, settings)?;
    all.extend(collect_room_findings(conn, now_local)?);
    all.extend(collect_finance_findings(conn, today, offset_seconds)?);
    Ok(all)
}

/// Conflitos de manutenção abertos (`maintenance_conflict`, crítico) e limpezas urgentes (`cleaning_urgent`).
pub(crate) fn collect_room_findings(conn: &Connection, now_local: NaiveDateTime) -> Result<Vec<Finding>, AppError> {
    let mut out = Vec::new();
    for c in maintenance_repo::all_open_conflicts(conn)? {
        out.push(finding(
            AlertKind::MaintenanceConflict,
            format!("conflict:{}", c.id),
            format!("Reserva afetada por manutenção: {} ({})", c.guest_name, c.reservation_code),
            format!(
                "A reserva de {} a {} cai no período de uma manutenção. Remaneje, cancele ou registre a resolução.",
                br(&c.check_in_date),
                br(&c.check_out_date)
            ),
            format!("/manutencao/{}", c.order_id),
        ));
    }
    for task in cleaning_repo::queue(conn)? {
        let arrival = reservation_repo::next_arrival(conn, task.room_id)?;
        let suggested = suggest_priority(arrival, now_local);
        let priority = task.priority_override.as_deref().and_then(Priority::parse).unwrap_or(suggested);
        if priority == Priority::Urgent {
            let detail = match arrival {
                Some(a) => format!("Há chegada prevista para {}. Conclua a limpeza para liberar o quarto.", a.format("%d/%m/%Y às %H:%M")),
                None => "A limpeza foi marcada como urgente. Conclua para liberar o quarto.".to_string(),
            };
            out.push(finding(
                AlertKind::CleaningUrgent,
                format!("cleaning:{}", task.id),
                format!("Limpeza urgente: quarto {}", task.room_code),
                detail,
                "/limpeza".to_string(),
            ));
        }
    }
    Ok(out)
}

/// Contas vencidas (`payable_overdue`), saldos na saída (`checkout_balance`) e dias sem fechamento (`closing_pending`).
pub(crate) fn collect_finance_findings(conn: &Connection, today: NaiveDate, offset_seconds: i32) -> Result<Vec<Finding>, AppError> {
    let mut out = Vec::new();
    let today_text = dates::format(today);

    let filters = PayableFilters { status: Some("overdue"), category_id: None, due_from: None, due_to: None };
    for p in payable_repo::list(conn, &filters, &today_text, 1000, 0)?.0 {
        out.push(finding(
            AlertKind::PayableOverdue,
            format!("payable:{}", p.id),
            format!("Conta vencida: {}", p.supplier),
            format!("{}: saldo de {} vencido em {}.", p.description, money_br(p.amount_cents - p.paid_cents), br(&p.due_date)),
            "/contas-a-pagar".to_string(),
        ));
    }

    for stay in stay_repo::list(conn, None, "in_house", 1000, 0)?.0 {
        if parse_date(&stay.check_out_date, "Data de saída")? > today {
            continue;
        }
        let balance = stay_repo::balance(conn, stay.id)?;
        if balance != 0 {
            let situation = if balance > 0 { format!("Saldo a receber {}", money_br(balance)) } else { format!("Crédito do hóspede {}", money_br(-balance)) };
            let title = if balance > 0 { "Saldo pendente na saída" } else { "Crédito do hóspede na saída" };
            out.push(finding(
                AlertKind::CheckoutBalance,
                format!("stay:{}", stay.id),
                format!("{title}: {}, quarto {}", stay.guest_name, stay.room_code),
                format!("{situation}. Resolva a conta antes do check-out."),
                format!("/hospedagens/{}", stay.id),
            ));
        }
    }

    let shift = format!("{offset_seconds:+} seconds");
    for date in closing_repo::pending_dates(conn, &shift, &today_text)? {
        out.push(finding(
            AlertKind::ClosingPending,
            format!("closing:{date}"),
            format!("Fechamento de caixa pendente: {}", br(&date)),
            "Houve recebimentos ou devoluções nesse dia e o caixa não foi fechado. Nada fica bloqueado.".to_string(),
            format!("/fechamento/{date}"),
        ));
    }
    Ok(out)
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
        } else if date <= today {
            // Também depois da meia-noite, até acabar a tolerância: o alerta não pode sumir no intervalo.
            out.push(finding(
                AlertKind::ArrivalToday,
                key,
                format!("Chegada hoje: {who}"),
                format!("Previsto para {} às {}. Faça o check-in quando a pessoa chegar.", br(&r.check_in_date), r.expected_check_in_time),
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
    use crate::domain::alert::{AlertKind, Finding};
    use crate::models::alert::{AlertQuery, AlertView};
    use crate::models::maintenance::NewOrderInput;
    use crate::models::payable::{NewPayable, PayInput};
    use crate::models::reservation::NewReservation;
    use crate::models::stay::{CheckInInput, CheckoutInput, LateFeeChoice, OccupantInput};
    use crate::services::test_support::{admin_token, checked_out_stay, deposit, harness, pay, seed, Harness, Seed};

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

    /// Só os tipos de chegada e saída: sinal vencido, saldo na saída e fechamento pendente têm testes próprios
    /// e aparecem com razão em vários cenários (reserva criada na véspera, conta ainda por pagar, dia anterior sem fechamento).
    fn movement_kinds(alerts: &[AlertView]) -> Vec<&str> {
        kinds(alerts).into_iter().filter(|k| !matches!(*k, "deposit_overdue" | "checkout_balance" | "closing_pending")).collect()
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

        assert_eq!(movement_kinds(&alerts), ["departure_today"]);
        let alert = alerts.iter().find(|a| a.kind == "departure_today").unwrap();
        assert_eq!(alert.link, format!("/hospedagens/{stay}"));
        assert!(alert.detail.contains("12:00"));
        check_out(&w, stay).unwrap();
        assert!(!kinds(&active(&w)).contains(&"departure_today"));
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
    fn late_arrival_alert_survives_midnight_until_the_tolerance_ends() {
        let w = world();
        book(&w, "2026-10-22", "2026-10-24", Some("23:55"));
        at(&w, "2026-10-23T02:30:00Z"); // 22/10 23:30 local
        assert_eq!(movement_kinds(&active(&w)), ["arrival_today"]);

        at(&w, "2026-10-23T03:10:00Z"); // 23/10 00:10 local: virou o dia, mas a tolerância (até 00:55) não acabou

        let alerts = active(&w);
        assert_eq!(movement_kinds(&alerts), ["arrival_today"]);
        assert!(alerts.iter().find(|a| a.kind == "arrival_today").unwrap().detail.contains("22/10/2026"));
        let history = w.h.alerts.list(&w.admin, AlertQuery { status: Some("history".into()), kind: Some("arrival_today".into()), severity: None, page: 1, page_size: 10 }).unwrap();
        assert_eq!(history.total, 0, "não pode ir para o histórico como resolvido");

        at(&w, "2026-10-23T04:00:00Z"); // 01:00 local: passou da tolerância
        assert_eq!(movement_kinds(&active(&w)), ["arrival_overdue"]);
    }

    #[test]
    fn a_credit_balance_is_worded_as_credit_not_as_a_pending_balance() {
        let w = world();
        let id = book(&w, "2026-10-21", "2026-10-23", None);
        at(&w, "2026-10-21T17:00:00Z");
        let stay = check_in(&w, id);
        pay(&w.h, &w.admin, stay, 50_000, "pix").unwrap(); // R$ 100,00 a mais que a conta
        at(&w, "2026-10-23T12:00:00Z");

        let alerts = active(&w);

        let alert = alerts.iter().find(|a| a.kind == "checkout_balance").expect("crédito na saída");
        assert!(alert.title.starts_with("Crédito do hóspede na saída"), "{}", alert.title);
        assert!(alert.detail.contains("R$ 100,00"), "{}", alert.detail);
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
        assert_eq!(movement_kinds(&active(&w)), ["departure_today"]);

        at(&w, "2026-10-23T16:00:00Z"); // 13:00 local: 60 min depois
        let alerts = active(&w);
        assert_eq!(movement_kinds(&alerts), ["departure_late"]);
        let late = alerts.iter().find(|a| a.kind == "departure_late").unwrap();
        assert!(late.detail.contains("60 min"), "{}", late.detail);
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

    #[test]
    fn open_maintenance_conflict_raises_a_critical_alert_and_resolves_when_it_is_resolved() {
        let w = world();
        book(&w, "2026-10-25", "2026-10-28", None);
        let opened = w
            .h
            .maintenance
            .open(
                &w.admin,
                NewOrderInput {
                    room_id: w.seed.room_id,
                    description: "Trocar o chuveiro".into(),
                    priority: "high".into(),
                    planned_start: "2026-10-26".into(),
                    planned_end: Some("2026-10-27".into()),
                    responsible: String::new(),
                    notes: String::new(),
                },
            )
            .unwrap();

        let alerts = active(&w);

        let alert = alerts.iter().find(|a| a.kind == "maintenance_conflict").expect("conflito");
        assert_eq!((alert.severity.as_str(), alert.link.as_str()), ("critical", format!("/manutencao/{}", opened.id).as_str()));
        assert!(alert.title.contains("Ana Souza"), "{}", alert.title);
        let conflict_id = w.h.maintenance.get(&w.admin, opened.id).unwrap().conflicts[0].id;
        w.h.maintenance.resolve_conflict_manually(&w.admin, conflict_id, "Reserva remanejada para outro quarto".into()).unwrap();
        assert!(!kinds(&active(&w)).contains(&"maintenance_conflict"));
    }

    #[test]
    fn urgent_cleaning_raises_cleaning_urgent_and_clears_when_the_cleaning_is_done() {
        let w = world();
        let (_, stay) = checked_out_stay(&w.h, &w.admin, &w.seed); // saiu às 12:00 locais de 23/10
        let _ = stay;
        assert!(!kinds(&active(&w)).contains(&"cleaning_urgent"), "sem chegada próxima a prioridade é baixa");

        book(&w, "2026-10-23", "2026-10-25", None); // chega às 14:00, em menos de 4 horas
        let alerts = active(&w);

        let alert = alerts.iter().find(|a| a.kind == "cleaning_urgent").expect("limpeza urgente");
        assert_eq!(alert.link, "/limpeza");
        assert!(alert.title.contains("101"), "{}", alert.title);
        let queue = w.h.cleaning.queue(&w.admin).unwrap();
        w.h.cleaning.start(&w.admin, queue[0].task.id, queue[0].task.version).unwrap();
        w.h.cleaning.complete(&w.admin, queue[0].task.id, queue[0].task.version + 1).unwrap();
        assert!(!kinds(&active(&w)).contains(&"cleaning_urgent"));
    }

    #[test]
    fn overdue_payable_raises_payable_overdue_and_resolves_when_paid() {
        let w = world();
        let bill = |due: &str| NewPayable {
            supplier: "Companhia de Energia".into(),
            description: "Conta de outubro".into(),
            amount_cents: 32_000,
            competence_date: "2026-10-01".into(),
            due_date: due.into(),
            category_id: 1,
            notes: String::new(),
            paid_now: None,
        };
        w.h.payables.create(&w.admin, bill("2026-10-21")).unwrap(); // vence hoje: ainda não está vencida
        assert!(!kinds(&active(&w)).contains(&"payable_overdue"));

        let overdue = w.h.payables.create(&w.admin, bill("2026-10-20")).unwrap();
        let alerts = active(&w);

        let alert = alerts.iter().find(|a| a.kind == "payable_overdue").expect("conta vencida");
        assert_eq!(alert.link, "/contas-a-pagar");
        assert!(alert.detail.contains("R$ 320,00") && alert.detail.contains("20/10/2026"), "{}", alert.detail);
        w.h.payables.pay(&w.admin, overdue, PayInput { amount_cents: 32_000, method: "pix".into(), note: String::new() }, 1).unwrap();
        assert!(!kinds(&active(&w)).contains(&"payable_overdue"));
    }

    #[test]
    fn in_house_stay_leaving_today_with_balance_raises_checkout_balance_until_paid() {
        let w = world();
        let id = book(&w, "2026-10-21", "2026-10-23", None);
        at(&w, "2026-10-21T17:00:00Z");
        let stay = check_in(&w, id);
        at(&w, "2026-10-22T12:00:00Z");
        assert!(!kinds(&active(&w)).contains(&"checkout_balance"), "sai só amanhã");

        at(&w, "2026-10-23T12:00:00Z");
        let alerts = active(&w);

        let alert = alerts.iter().find(|a| a.kind == "checkout_balance").expect("saldo pendente");
        assert!(alert.detail.contains("R$ 400,00"), "{}", alert.detail);
        assert_eq!(alert.link, format!("/hospedagens/{stay}"));
        pay(&w.h, &w.admin, stay, 40_000, "pix").unwrap();
        let after = active(&w);
        assert!(!kinds(&after).contains(&"checkout_balance"));
        assert!(kinds(&after).contains(&"departure_today"), "a saída continua anunciada");
    }

    #[test]
    fn day_with_movements_and_no_closing_raises_closing_pending_until_closed() {
        let w = world();
        let id = book(&w, "2026-10-25", "2026-10-27", None);
        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;
        deposit(&w.h, &w.admin, id, 12_000, version).unwrap(); // recebimento em 21/10
        assert!(!kinds(&active(&w)).contains(&"closing_pending"), "hoje nunca é pendência");

        at(&w, "2026-10-22T12:00:00Z");
        let alerts = active(&w);

        let alert = alerts.iter().find(|a| a.kind == "closing_pending").expect("fechamento pendente");
        assert_eq!(alert.link, "/fechamento/2026-10-21");
        assert!(alert.title.contains("21/10/2026"), "{}", alert.title);
        let counted = ["cash", "pix", "debit", "credit", "transfer"]
            .into_iter()
            .map(|m| crate::models::closing::CountedInput { method: m.into(), counted_cents: if m == "cash" { 12_000 } else { 0 }, justification: None })
            .collect();
        w.h.closing.close(&w.admin, crate::models::closing::CloseInput { date: "2026-10-21".into(), counted }).unwrap();
        assert!(!kinds(&active(&w)).contains(&"closing_pending"));
    }

    #[test]
    fn refresh_runs_all_detectors_and_keeps_alerts_from_other_modules() {
        let w = world();
        let external = Finding { kind: AlertKind::BackupMissing, subject_key: "backup".into(), title: "Sem backup válido".into(), detail: "Nenhum backup nos últimos 7 dias".into(), link: "/configuracoes".into() };
        w.h.alerts.sync(vec![external], &[]).unwrap();
        book(&w, "2026-10-21", "2026-10-23", None);

        let alerts = active(&w);

        assert!(kinds(&alerts).contains(&"backup_missing"));
        assert!(kinds(&alerts).contains(&"arrival_today"));
    }
}
