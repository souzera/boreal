//! Motor de alertas persistentes (spec §20.2): sincroniza os achados dos detectores com a tabela,
//! resolve sozinho quando a causa some e nunca apaga nada.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use serde_json::json;

use super::alert_detectors;
use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::session::{require, SessionInfo};
use crate::domain::alert::{can_dismiss, AlertKind, Finding, Severity};
use crate::error::AppError;
use crate::models::alert::{AlertEventView, AlertQuery, AlertSummary, AlertView};
use crate::models::page::{clamp_paging, Page};
use crate::repositories::alert_repo::{self, AlertRow, Filters};
use crate::repositories::settings_repo;

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn view(row: AlertRow) -> AlertView {
    let kind = AlertKind::parse(&row.kind);
    let severity = Severity::parse(&row.severity);
    AlertView {
        id: row.id,
        kind_label: kind.map_or_else(|| row.kind.clone(), |k| k.label().to_string()),
        kind: row.kind,
        severity_label: severity.map_or_else(|| row.severity.clone(), |s| s.label().to_string()),
        severity: row.severity,
        title: row.title,
        detail: row.detail,
        link: row.link,
        status: row.status,
        first_seen_at: row.first_seen_at,
        last_seen_at: row.last_seen_at,
        resolved_at: row.resolved_at,
        resolved_by_name: row.resolved_by_name,
        resolution_note: row.resolution_note,
        version: row.version,
    }
}

#[derive(Clone)]
pub struct AlertService {
    ctx: Ctx,
}

impl AlertService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn touch(&self, token: &str) -> Result<SessionInfo, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())
    }

    /// Tipos que a pessoa pode ver: os sem exigência de permissão e os que ela tem.
    fn allowed_kinds(info: &SessionInfo) -> Vec<&'static str> {
        AlertKind::ALL
            .into_iter()
            .filter(|k| k.requires_permission().is_none_or(|p| require(info, p).is_ok()))
            .map(AlertKind::as_str)
            .collect()
    }

    /// Sincroniza os achados com a tabela, numa só transação: cria o que é novo, atualiza o que mudou,
    /// resolve (sozinho) os ativos do `scope` cuja causa sumiu e mantém dispensados enquanto a causa continua.
    /// Nada é apagado.
    #[cfg_attr(not(test), allow(dead_code))] // o Marco 7 o usa para backup, disco e atualização
    pub(crate) fn sync(&self, findings: Vec<Finding>, scope: &[AlertKind]) -> Result<(), AppError> {
        let now = self.ctx.utc.now_utc();
        let mut conn = self.ctx.conn()?;
        Self::sync_in(&mut conn, now, findings, scope)
    }

    fn sync_in(conn: &mut rusqlite::Connection, now: DateTime<Utc>, findings: Vec<Finding>, scope: &[AlertKind]) -> Result<(), AppError> {
        let scope: HashSet<AlertKind> = scope.iter().copied().collect();
        let tx = conn.transaction()?;
        let existing: HashMap<(String, String), AlertRow> =
            alert_repo::open_rows(&tx)?.into_iter().map(|r| ((r.kind.clone(), r.subject_key.clone()), r)).collect();

        let mut seen = HashSet::new();
        for finding in findings {
            let key = (finding.kind.as_str().to_string(), finding.subject_key.clone());
            // Um achado repetido na mesma rodada não pode criar dois alertas (nem derrubar a rodada).
            if !seen.insert(key.clone()) {
                continue;
            }
            let severity = finding.kind.severity().as_str();
            match existing.get(&key) {
                Some(row) if row.status == "active" => {
                    // Texto que muda sozinho com o tempo (minutos de atraso) atualiza a linha sem barulho:
                    // não mexe na versão (a dispensa em andamento continua valendo) nem enche o histórico.
                    let retitled = row.title != finding.title || row.link != finding.link || row.severity != severity;
                    if retitled || row.detail != finding.detail {
                        alert_repo::update_text(&tx, row.id, &finding, severity, now)?;
                        if retitled {
                            alert_repo::insert_event(&tx, row.id, now, None, "updated", None)?;
                        }
                    } else {
                        alert_repo::touch(&tx, row.id, now)?;
                    }
                }
                // Dispensado e a causa continua: não volta a aparecer.
                Some(_) => {}
                None => {
                    let id = alert_repo::insert(&tx, &finding, severity, now)?;
                    alert_repo::insert_event(&tx, id, now, None, "raised", None)?;
                }
            }
        }

        for (key, row) in &existing {
            let in_scope = AlertKind::parse(&row.kind).is_some_and(|k| scope.contains(&k));
            if !in_scope || seen.contains(key) {
                continue;
            }
            if row.status == "active" {
                alert_repo::resolve(&tx, row.id, now)?;
                alert_repo::insert_event(&tx, row.id, now, None, "resolved", None)?;
            } else {
                alert_repo::clear_presence(&tx, row.id)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Roda os detectores e sincroniza. Chamado ao abrir o painel e a cada minuto pela tela.
    pub fn refresh(&self, token: &str) -> Result<(), AppError> {
        self.touch(token)?;
        let managed: Vec<AlertKind> = AlertKind::ALL.into_iter().filter(|k| k.managed_by_detectors()).collect();
        let now = self.ctx.utc.now_utc();
        // Detectores e sincronização sob a mesma trava da conexão: duas rodadas nunca se misturam.
        let mut conn = self.ctx.conn()?;
        let settings = settings_repo::load(&conn)?;
        let findings = alert_detectors::collect_all(&conn, self.ctx.today(), self.ctx.local_now(), self.ctx.utc.local_offset().local_minus_utc(), &settings)?;
        Self::sync_in(&mut conn, now, findings, &managed)
    }

    pub fn list(&self, token: &str, query: AlertQuery) -> Result<Page<AlertView>, AppError> {
        let info = self.touch(token)?;
        let statuses: &[&str] = match query.status.as_deref() {
            None | Some("active") => &["active"],
            Some("history") => &["resolved", "dismissed"],
            Some(_) => return Err(invalid("Situação inválida.")),
        };
        if let Some(kind) = query.kind.as_deref() {
            AlertKind::parse(kind).ok_or_else(|| invalid("Tipo de alerta inválido."))?;
        }
        if let Some(severity) = query.severity.as_deref() {
            Severity::parse(severity).ok_or_else(|| invalid("Prioridade inválida."))?;
        }
        let allowed = Self::allowed_kinds(&info);
        let (page, page_size, offset) = clamp_paging(query.page, query.page_size);
        let conn = self.ctx.conn()?;
        let filters = Filters { statuses, kind: query.kind.as_deref(), severity: query.severity.as_deref(), allowed_kinds: &allowed };
        let (rows, total) = alert_repo::list(&conn, &filters, i64::from(page_size), offset)?;
        Ok(Page { items: rows.into_iter().map(view).collect(), total, page, page_size })
    }

    pub fn summary(&self, token: &str) -> Result<AlertSummary, AppError> {
        let info = self.touch(token)?;
        let allowed = Self::allowed_kinds(&info);
        let conn = self.ctx.conn()?;
        let (critical, high, normal) = alert_repo::counts(&conn, &allowed)?;
        let top = alert_repo::top(&conn, &allowed, 5)?.into_iter().map(view).collect();
        Ok(AlertSummary { critical, high, normal, total: critical + high + normal, top })
    }

    pub fn history(&self, token: &str, alert_id: i64) -> Result<Vec<AlertEventView>, AppError> {
        let info = self.touch(token)?;
        let conn = self.ctx.conn()?;
        self.visible(&conn, &info, alert_id)?;
        alert_repo::events(&conn, alert_id)
    }

    fn visible(&self, conn: &rusqlite::Connection, info: &SessionInfo, alert_id: i64) -> Result<AlertRow, AppError> {
        let row = alert_repo::find(conn, alert_id)?.ok_or_else(|| invalid("Alerta não encontrado."))?;
        if !Self::allowed_kinds(info).contains(&row.kind.as_str()) {
            return Err(invalid("Alerta não encontrado."));
        }
        Ok(row)
    }

    /// Dispensa um alerta não crítico com justificativa. Crítico só se resolve quando o problema é resolvido.
    pub fn dismiss(&self, token: &str, alert_id: i64, note: String, expected_version: i64) -> Result<(), AppError> {
        let actor = self.touch(token)?;
        let note = note.trim();
        if note.is_empty() {
            return Err(invalid("Informe por que o alerta está sendo dispensado."));
        }
        if note.chars().count() > 200 {
            return Err(invalid("Justificativa: no máximo 200 caracteres."));
        }
        let now = self.ctx.utc.now_utc();
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let row = self.visible(&tx, &actor, alert_id)?;
        if row.status != "active" {
            return Err(invalid("Este alerta já foi resolvido."));
        }
        if row.version != expected_version {
            return Err(AppError::Conflict);
        }
        if !can_dismiss(Severity::parse(&row.severity).unwrap_or(Severity::Critical)) {
            return Err(invalid("Alertas críticos só se resolvem quando o problema for resolvido; não podem ser dispensados."));
        }
        alert_repo::dismiss(&tx, alert_id, actor.user_id, note, now, expected_version)?;
        alert_repo::insert_event(&tx, alert_id, now, Some(actor.user_id), "dismissed", Some(note))?;
        audit::record(
            &tx,
            &AuditEntry {
                at: now,
                user_id: Some(actor.user_id),
                module: "alerts",
                operation: "dismiss_alert",
                entity: "alert",
                entity_id: Some(alert_id),
                before: Some(json!({ "status": "active" })),
                after: Some(json!({ "status": "dismissed", "kind": row.kind, "subject_key": row.subject_key })),
                justification: Some(note.to_string()),
            },
        )?;
        tx.commit()?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::auth::permissions::Permission;
    use crate::domain::alert::{AlertKind, Finding};
    use crate::error::AppError;
    use crate::models::alert::AlertQuery;
    use crate::services::test_support::{admin_token, harness, recepcao_token, Harness};

    struct World {
        h: Harness,
        admin: String,
    }

    fn world() -> World {
        let h = harness();
        let admin = admin_token(&h);
        World { h, admin }
    }

    fn at(w: &World, rfc3339: &str) {
        w.h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    fn finding(kind: AlertKind, key: &str, detail: &str) -> Finding {
        Finding { kind, subject_key: key.into(), title: format!("{} {key}", kind.label()), detail: detail.into(), link: format!("/x/{key}") }
    }

    fn managed() -> Vec<AlertKind> {
        AlertKind::ALL.into_iter().filter(|k| k.managed_by_detectors()).collect()
    }

    fn query(status: &str) -> AlertQuery {
        AlertQuery { status: Some(status.into()), kind: None, severity: None, page: 1, page_size: 50 }
    }

    fn count(w: &World, sql: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(sql, [], |r| r.get(0)).unwrap()
    }

    fn active_ids(w: &World) -> Vec<i64> {
        w.h.alerts.list(&w.admin, query("active")).unwrap().items.into_iter().map(|a| a.id).collect()
    }

    fn rec_with(w: &World, permissions: &[Permission]) -> String {
        if w.h.auth.login("rec", "Definitiva-456").is_err() {
            let _ = recepcao_token(&w.h, &w.admin);
        }
        let id = w.h.auth.login("rec", "Definitiva-456").unwrap().user.id;
        w.h.users.set_extra_permissions(&w.admin, id, permissions).unwrap();
        w.h.auth.login("rec", "Definitiva-456").unwrap().token
    }

    #[test]
    fn sync_creates_an_alert_with_a_raised_event() {
        let w = world();

        w.h.alerts.sync(vec![finding(AlertKind::DepositOverdue, "reservation:1", "Venceu em 20/10/2026")], &managed()).unwrap();

        let alerts = w.h.alerts.list(&w.admin, query("active")).unwrap().items;
        assert_eq!(alerts.len(), 1);
        assert_eq!((alerts[0].kind.as_str(), alerts[0].severity.as_str(), alerts[0].status.as_str()), ("deposit_overdue", "high", "active"));
        assert_eq!(alerts[0].kind_label, "Sinal vencido");
        assert_eq!(alerts[0].link, "/x/reservation:1");
        let events = w.h.alerts.history(&w.admin, alerts[0].id).unwrap();
        assert_eq!(events.iter().map(|e| e.event.as_str()).collect::<Vec<_>>(), ["raised"]);
    }

    #[test]
    fn sync_twice_with_the_same_finding_does_not_duplicate_alert_or_event() {
        let w = world();
        let f = || vec![finding(AlertKind::DepositOverdue, "reservation:1", "Venceu em 20/10/2026")];

        w.h.alerts.sync(f(), &managed()).unwrap();
        at(&w, "2026-10-21T13:00:00Z");
        w.h.alerts.sync(f(), &managed()).unwrap();

        assert_eq!(count(&w, "SELECT count(*) FROM alerts"), 1);
        assert_eq!(count(&w, "SELECT count(*) FROM alert_events"), 1);
        let last: String = w.h.db.lock().unwrap().query_row("SELECT last_seen_at FROM alerts", [], |r| r.get(0)).unwrap();
        assert_eq!(last, "2026-10-21T13:00:00Z", "a última vez em que a causa foi vista é atualizada");
    }

    #[test]
    fn detail_only_changes_update_the_row_without_event_or_version_bump() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::DepartureLate, "stay:1", "30 min")], &managed()).unwrap();
        let before = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);

        w.h.alerts.sync(vec![finding(AlertKind::DepartureLate, "stay:1", "31 min")], &managed()).unwrap();
        w.h.alerts.sync(vec![finding(AlertKind::DepartureLate, "stay:1", "32 min")], &managed()).unwrap();

        let after = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);
        assert_eq!((after.detail.as_str(), after.version), ("32 min", before.version));
        let events: Vec<String> = w.h.alerts.history(&w.admin, after.id).unwrap().into_iter().map(|e| e.event).collect();
        assert_eq!(events, ["raised"], "os minutos que passam não enchem o histórico");
    }

    #[test]
    fn a_new_title_updates_the_alert_and_logs_one_updated_event() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::CheckoutBalance, "stay:1", "x")], &managed()).unwrap();
        let renamed = || Finding { title: "Crédito do hóspede na saída: Ana".into(), ..finding(AlertKind::CheckoutBalance, "stay:1", "x") };

        w.h.alerts.sync(vec![renamed()], &managed()).unwrap();
        w.h.alerts.sync(vec![renamed()], &managed()).unwrap();

        let alert = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);
        assert_eq!(alert.title, "Crédito do hóspede na saída: Ana");
        let events: Vec<String> = w.h.alerts.history(&w.admin, alert.id).unwrap().into_iter().map(|e| e.event).collect();
        assert_eq!(events, ["raised", "updated"]);
    }

    #[test]
    fn dismissing_with_the_version_seen_still_works_after_a_text_only_refresh() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::DepartureLate, "stay:1", "40 min")], &managed()).unwrap();
        let seen = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);

        w.h.alerts.sync(vec![finding(AlertKind::DepartureLate, "stay:1", "41 min")], &managed()).unwrap();

        assert!(w.h.alerts.dismiss(&w.admin, seen.id, "Combinei a saída com o hóspede".into(), seen.version).is_ok());
    }

    #[test]
    fn duplicate_findings_in_one_sync_do_not_abort_it() {
        let w = world();
        let f = || finding(AlertKind::ArrivalToday, "reservation:1", "x");

        w.h.alerts.sync(vec![f(), f(), finding(AlertKind::DepositOverdue, "reservation:1", "y")], &managed()).unwrap();

        assert_eq!(active_ids(&w).len(), 2);
    }

    #[test]
    fn dismissing_an_alert_that_resolved_itself_says_so() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::ArrivalToday, "reservation:1", "x")], &managed()).unwrap();
        let seen = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);
        w.h.alerts.sync(vec![], &managed()).unwrap();

        let result = w.h.alerts.dismiss(&w.admin, seen.id, "Tarde demais".into(), seen.version);

        assert!(matches!(result, Err(AppError::Validation(m)) if m.contains("já foi resolvido")));
        assert_eq!(count(&w, "SELECT count(*) FROM alert_events WHERE event = 'dismissed'"), 0);
    }

    #[test]
    fn alert_resolves_itself_when_the_cause_disappears() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::DepositOverdue, "reservation:1", "x")], &managed()).unwrap();
        at(&w, "2026-10-21T15:00:00Z");

        w.h.alerts.sync(vec![], &managed()).unwrap();

        assert!(active_ids(&w).is_empty());
        let history = w.h.alerts.list(&w.admin, query("history")).unwrap().items;
        assert_eq!((history[0].status.as_str(), history[0].resolved_at.as_deref()), ("resolved", Some("2026-10-21T15:00:00Z")));
        let events: Vec<String> = w.h.alerts.history(&w.admin, history[0].id).unwrap().into_iter().map(|e| e.event).collect();
        assert_eq!(events, ["raised", "resolved"]);
    }

    #[test]
    fn cause_returning_creates_a_new_alert_and_keeps_the_old_one_as_history() {
        let w = world();
        let f = || vec![finding(AlertKind::DepositOverdue, "reservation:1", "x")];
        w.h.alerts.sync(f(), &managed()).unwrap();
        w.h.alerts.sync(vec![], &managed()).unwrap();

        w.h.alerts.sync(f(), &managed()).unwrap();

        assert_eq!(count(&w, "SELECT count(*) FROM alerts"), 2);
        assert_eq!(active_ids(&w).len(), 1);
        assert_eq!(w.h.alerts.list(&w.admin, query("history")).unwrap().items.len(), 1);
    }

    #[test]
    fn sync_never_touches_kinds_outside_its_scope() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::BackupMissing, "backup", "Nenhum backup nos últimos 7 dias")], &[]).unwrap();

        w.h.alerts.sync(vec![], &managed()).unwrap();

        assert_eq!(active_ids(&w).len(), 1, "alerta de outro módulo sobrevive ao refresh dos detectores");
    }

    #[test]
    fn critical_alert_cannot_be_dismissed() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::DepartureRisk, "stay:1", "x")], &managed()).unwrap();
        let alert = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);

        let result = w.h.alerts.dismiss(&w.admin, alert.id, "Já sei".into(), alert.version);

        assert!(matches!(result, Err(AppError::Validation(m)) if m.contains("críticos")));
        assert_eq!(active_ids(&w).len(), 1);
        assert_eq!(count(&w, "SELECT count(*) FROM alert_events WHERE event = 'dismissed'"), 0);
    }

    #[test]
    fn dismiss_requires_a_note_up_to_200_chars() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::ArrivalToday, "reservation:1", "x")], &managed()).unwrap();
        let alert = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);

        for note in ["", "   ", &"x".repeat(201)] {
            assert!(matches!(w.h.alerts.dismiss(&w.admin, alert.id, note.into(), alert.version), Err(AppError::Validation(_))), "{} chars", note.len());
        }

        assert_eq!(active_ids(&w).len(), 1);
        assert!(w.h.alerts.dismiss(&w.admin, alert.id, "x".repeat(200), alert.version).is_ok());
    }

    #[test]
    fn a_wrong_version_of_an_active_alert_is_a_conflict() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::ArrivalToday, "reservation:1", "x")], &managed()).unwrap();
        let alert = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);

        assert!(matches!(w.h.alerts.dismiss(&w.admin, alert.id, "x".into(), alert.version + 5), Err(AppError::Conflict)));
        assert_eq!(active_ids(&w).len(), 1);
    }

    #[test]
    fn dismissed_alert_does_not_come_back_while_the_cause_continues() {
        let w = world();
        let f = || vec![finding(AlertKind::ClosingPending, "closing:2026-10-20", "x")];
        w.h.alerts.sync(f(), &managed()).unwrap();
        let alert = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);
        w.h.alerts.dismiss(&w.admin, alert.id, "Vou fechar amanhã".into(), alert.version).unwrap();

        w.h.alerts.sync(f(), &managed()).unwrap();
        w.h.alerts.sync(f(), &managed()).unwrap();

        assert!(active_ids(&w).is_empty());
        let history = w.h.alerts.list(&w.admin, query("history")).unwrap().items;
        assert_eq!((history.len(), history[0].status.as_str(), history[0].resolution_note.as_deref()), (1, "dismissed", Some("Vou fechar amanhã")));
        assert_eq!(history[0].resolved_by_name.as_deref(), Some("Administrador"));
    }

    #[test]
    fn dismissed_then_cause_gone_then_back_creates_a_new_alert() {
        let w = world();
        let f = || vec![finding(AlertKind::ClosingPending, "closing:2026-10-20", "x")];
        w.h.alerts.sync(f(), &managed()).unwrap();
        let alert = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);
        w.h.alerts.dismiss(&w.admin, alert.id, "Depois".into(), alert.version).unwrap();
        w.h.alerts.sync(vec![], &managed()).unwrap();

        w.h.alerts.sync(f(), &managed()).unwrap();

        assert_eq!(active_ids(&w).len(), 1);
        assert_eq!(count(&w, "SELECT count(*) FROM alerts"), 2);
    }

    #[test]
    fn dismiss_is_audited_with_user_and_note_and_double_dismiss_is_refused() {
        let w = world();
        w.h.alerts.sync(vec![finding(AlertKind::ArrivalToday, "reservation:1", "x")], &managed()).unwrap();
        let alert = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);

        w.h.alerts.dismiss(&w.admin, alert.id, "Hóspede avisou que chega tarde".into(), alert.version).unwrap();
        let again = w.h.alerts.dismiss(&w.admin, alert.id, "Hóspede avisou que chega tarde".into(), alert.version);

        assert!(matches!(again, Err(AppError::Validation(m)) if m.contains("já foi resolvido")), "o segundo clique é recusado sem duplicar o evento");
        assert_eq!(count(&w, "SELECT count(*) FROM alert_events WHERE event = 'dismissed'"), 1);
        let (module, operation, why): (String, String, String) = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT module, operation, justification FROM audit_log WHERE operation = 'dismiss_alert'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap();
        assert_eq!((module.as_str(), operation.as_str(), why.as_str()), ("alerts", "dismiss_alert", "Hóspede avisou que chega tarde"));
    }

    #[test]
    fn no_path_deletes_alert_rows() {
        let w = world();
        let mut previous = (0, 0);
        let mut steps = 0;
        let mut check = |w: &World| {
            let now = (count(w, "SELECT count(*) FROM alerts"), count(w, "SELECT count(*) FROM alert_events"));
            assert!(now.0 >= previous.0 && now.1 >= previous.1, "passo {steps}: {previous:?} -> {now:?}");
            previous = now;
            steps += 1;
        };
        let critical = || vec![finding(AlertKind::MaintenanceConflict, "conflict:1", "x")];
        w.h.alerts.sync(critical(), &managed()).unwrap();
        check(&w);
        w.h.alerts.sync(critical(), &managed()).unwrap();
        check(&w);
        w.h.alerts.sync(vec![], &managed()).unwrap();
        check(&w);
        w.h.alerts.sync(critical(), &managed()).unwrap();
        check(&w);
        let alert = w.h.alerts.list(&w.admin, query("active")).unwrap().items.remove(0);
        let _ = w.h.alerts.dismiss(&w.admin, alert.id, "x".into(), alert.version);
        check(&w);
        assert_eq!(previous.0, 2);
    }

    #[test]
    fn payable_overdue_is_invisible_without_manage_payables() {
        let w = world();
        w.h.alerts
            .sync(
                vec![finding(AlertKind::PayableOverdue, "payable:1", "R$ 320,00"), finding(AlertKind::DepositOverdue, "reservation:1", "x")],
                &managed(),
            )
            .unwrap();
        let payable_id = w.h.alerts.list(&w.admin, query("active")).unwrap().items.iter().find(|a| a.kind == "payable_overdue").unwrap().id;

        let without = rec_with(&w, &[]);
        let list = w.h.alerts.list(&without, query("active")).unwrap();
        assert_eq!(list.items.iter().map(|a| a.kind.as_str()).collect::<Vec<_>>(), ["deposit_overdue"]);
        assert_eq!(list.total, 1);
        let summary = w.h.alerts.summary(&without).unwrap();
        assert_eq!((summary.total, summary.high), (1, 1));
        assert!(summary.top.iter().all(|a| a.kind != "payable_overdue"));
        assert!(matches!(w.h.alerts.history(&without, payable_id), Err(AppError::Validation(_))));

        let with = rec_with(&w, &[Permission::ManagePayables]);
        assert_eq!(w.h.alerts.list(&with, query("active")).unwrap().total, 2);
        assert_eq!(w.h.alerts.summary(&with).unwrap().total, 2);
    }

    #[test]
    fn summary_counts_by_severity_and_orders_critical_first() {
        let w = world();
        w.h.alerts
            .sync(
                vec![
                    finding(AlertKind::ArrivalToday, "reservation:1", "x"),
                    finding(AlertKind::DepositOverdue, "reservation:2", "x"),
                    finding(AlertKind::DepartureRisk, "stay:3", "x"),
                    finding(AlertKind::MaintenanceConflict, "conflict:4", "x"),
                ],
                &managed(),
            )
            .unwrap();

        let summary = w.h.alerts.summary(&w.admin).unwrap();

        assert_eq!((summary.critical, summary.high, summary.normal, summary.total), (2, 1, 1, 4));
        assert_eq!(summary.top.iter().map(|a| a.severity.as_str()).collect::<Vec<_>>(), ["critical", "critical", "high", "normal"]);
    }

    #[test]
    fn summary_top_is_limited_to_five() {
        let w = world();
        let many = (1..=8).map(|i| finding(AlertKind::ArrivalToday, &format!("reservation:{i}"), "x")).collect();
        w.h.alerts.sync(many, &managed()).unwrap();

        let summary = w.h.alerts.summary(&w.admin).unwrap();

        assert_eq!((summary.total, summary.top.len()), (8, 5));
    }

    #[test]
    fn list_filters_by_kind_and_severity_and_pages() {
        let w = world();
        let findings = (1..=5).map(|i| finding(AlertKind::ArrivalToday, &format!("reservation:{i}"), "x")).chain([finding(AlertKind::DepartureRisk, "stay:1", "x")]).collect();
        w.h.alerts.sync(findings, &managed()).unwrap();
        let q = |kind: Option<&str>, severity: Option<&str>, page, size| AlertQuery { status: None, kind: kind.map(str::to_string), severity: severity.map(str::to_string), page, page_size: size };

        assert_eq!(w.h.alerts.list(&w.admin, q(Some("arrival_today"), None, 1, 50)).unwrap().total, 5);
        assert_eq!(w.h.alerts.list(&w.admin, q(None, Some("critical"), 1, 50)).unwrap().total, 1);
        let page2 = w.h.alerts.list(&w.admin, q(None, None, 2, 4)).unwrap();
        assert_eq!((page2.total, page2.items.len()), (6, 2));
        assert!(matches!(w.h.alerts.list(&w.admin, q(Some("inexistente"), None, 1, 50)), Err(AppError::Validation(_))));
        assert!(matches!(w.h.alerts.list(&w.admin, q(None, Some("urgentissimo"), 1, 50)), Err(AppError::Validation(_))));
    }
}
