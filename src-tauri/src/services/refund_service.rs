//! Devoluções vinculadas ao recebimento original (spec §9.2).

use serde_json::json;

use super::payment_service::{parse_components, MAX_PAYMENT_CENTS};
use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::payment;
use crate::error::AppError;
use crate::models::refund::{RefundInput, RefundReceipt};
use crate::repositories::{payment_repo, refund_repo, reservation_repo, sequence_repo, stay_repo};

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

/// `R$ 1.234,56`, para mensagens.
fn money_br(cents: i64) -> String {
    let reais = (cents / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in reais.chars().enumerate() {
        if i > 0 && (reais.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    format!("R$ {grouped},{:02}", cents % 100)
}

#[derive(Clone)]
pub struct RefundService {
    ctx: Ctx,
}

impl RefundService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    /// Registra uma devolução (parcial ou total) de um recebimento, com motivo, usuário e instante.
    /// Não passa do que ainda resta do recebimento (M5-7: sem permissão extra; a auditoria guarda o motivo).
    pub fn record(&self, token: &str, input: RefundInput) -> Result<RefundReceipt, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let components = parse_components(&input.components)?;
        let total = payment::validate_components(&components, MAX_PAYMENT_CENTS)?;
        let reason = input.reason.trim();
        if reason.is_empty() {
            return Err(invalid("Informe o motivo da devolução."));
        }
        if reason.chars().count() > 200 {
            return Err(invalid("Motivo: no máximo 200 caracteres."));
        }
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let original = payment_repo::find(&tx, input.payment_id)?.ok_or_else(|| invalid("Recebimento não encontrado."))?;
        let refunded = refund_repo::refunded_for_payment(&tx, original.id)?;
        let refundable = payment::refundable_cents(original.amount_cents, refunded)?;
        if total > refundable {
            return Err(invalid(format!(
                "Só é possível devolver até {} deste recebimento ({}).",
                money_br(refundable),
                original.code
            )));
        }

        // Sinal de reserva cancelada ou no-show: devolver mais do que a política calculou é uma exceção de cancelamento.
        let reservation = reservation_repo::find(&tx, original.reservation_id, self.ctx.today())?
            .ok_or_else(|| invalid("Reserva do recebimento não encontrada."))?;
        if original.stay_id.is_none()
            && matches!(reservation.status.as_str(), "cancelled" | "no_show")
            && total > reservation.refund_due_cents
        {
            require(&actor, Permission::CancellationException)?;
        }

        let at = self.ctx.utc.now_utc();
        let code = sequence_repo::format_code("DEV", sequence_repo::next(&tx, "refund")?);
        let id = refund_repo::insert(
            &tx,
            &refund_repo::NewRefund {
                code: &code,
                payment_id: original.id,
                reservation_id: original.reservation_id,
                stay_id: original.stay_id,
                amount_cents: total,
                reason,
                refunded_by: actor.user_id,
            },
            at,
        )?;
        refund_repo::insert_components(&tx, id, &components)?;
        if let Some(stay_id) = original.stay_id {
            stay_repo::bump(&tx, stay_id, at)?;
        } else {
            reservation_repo::lower_deposit_paid(&tx, original.reservation_id, total, at)?;
        }
        audit::record(
            &tx,
            &AuditEntry {
                at,
                user_id: Some(actor.user_id),
                module: "finance",
                operation: "record_refund",
                entity: "payment",
                entity_id: Some(original.id),
                before: Some(json!({ "refunded_cents": refunded })),
                after: Some(json!({
                    "refunded_cents": refunded + total,
                    "entry": {
                        "refund_id": id, "code": code, "payment_code": original.code, "amount_cents": total,
                        "components": components.iter().map(|c| json!({ "method": c.method.as_str(), "amount_cents": c.amount_cents })).collect::<Vec<_>>(),
                    }
                })),
                justification: Some(reason.to_string()),
            },
        )?;
        tx.commit()?;
        Ok(RefundReceipt { id, code, total_cents: total })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::error::AppError;
    use crate::models::payment::{ComponentInput, PaymentInput, PaymentTarget};
    use crate::models::refund::RefundInput;
    use crate::models::reservation::CancellationOverride;
    use crate::models::stay::{CheckInInput, OccupantInput};
    use crate::services::test_support::{admin_token, book, harness, pay, seed, Harness, Seed};

    // "Hoje" nos testes é 21/10/2026, 09:00 local (12:00Z; fuso -03:00). Diária R$ 200,00.

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

    fn parts(items: &[(&str, i64)]) -> Vec<ComponentInput> {
        items.iter().map(|(m, c)| ComponentInput { method: (*m).into(), amount_cents: *c }).collect()
    }

    fn refund(w: &World, payment_id: i64, items: &[(&str, i64)], reason: &str) -> Result<crate::models::refund::RefundReceipt, AppError> {
        w.h.refunds.record(&w.admin, RefundInput { payment_id, components: parts(items), reason: reason.into() })
    }

    fn count(w: &World, table: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    /// Hospedagem em casa (conta R$ 400,00) com um recebimento de R$ 100,00 (Pix 60 + dinheiro 40).
    fn stay_with_payment(w: &World) -> (i64, i64) {
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        w.h.utc.set(DateTime::parse_from_rfc3339("2026-10-21T17:00:00Z").unwrap().with_timezone(&Utc));
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        let stay = w
            .h
            .stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
            .unwrap();
        let receipt = w
            .h
            .payments
            .record(
                &w.admin,
                PaymentTarget::Stay { stay_id: stay },
                PaymentInput { components: parts(&[("pix", 6_000), ("cash", 4_000)]), note: String::new(), duplicate_release: None },
            )
            .unwrap();
        (stay, receipt.id)
    }

    #[test]
    fn partial_then_full_refund_never_exceeds_the_payment() {
        let w = world();
        let (_, payment) = stay_with_payment(&w);

        assert_eq!(refund(&w, payment, &[("pix", 3_000)], "Cobrança em dobro").unwrap().code, "DEV-000001");
        assert_eq!(refund(&w, payment, &[("pix", 3_000), ("cash", 4_000)], "Resto").unwrap().code, "DEV-000002");

        assert!(matches!(refund(&w, payment, &[("cash", 1)], "Mais um centavo"), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "refunds"), 2);
    }

    #[test]
    fn refund_split_across_methods_must_sum_exactly() {
        let w = world();
        let (_, payment) = stay_with_payment(&w);

        assert!(refund(&w, payment, &[("pix", 2_000), ("cash", 1_000)], "Ajuste").is_ok());

        let before = (count(&w, "refunds"), count(&w, "refund_components"));
        for bad in [vec![], vec![("pix", 0)], vec![("pix", 100), ("pix", 100)], vec![("unspecified", 100)], vec![("cheque", 100)]] {
            assert!(matches!(refund(&w, payment, &bad, "Ajuste"), Err(AppError::Validation(_))), "{bad:?}");
        }
        assert_eq!((count(&w, "refunds"), count(&w, "refund_components")), before);
    }

    #[test]
    fn refund_requires_reason_and_existing_payment() {
        let w = world();
        let (_, payment) = stay_with_payment(&w);

        for reason in ["", "   ", &"x".repeat(201)] {
            assert!(matches!(refund(&w, payment, &[("pix", 100)], reason), Err(AppError::Validation(_))));
        }
        assert!(matches!(refund(&w, 999, &[("pix", 100)], "Motivo"), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "refunds"), 0);
    }

    #[test]
    fn refund_raises_the_stay_balance_by_the_refunded_amount() {
        let w = world();
        let (stay, payment) = stay_with_payment(&w);
        assert_eq!(w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents, 30_000);

        refund(&w, payment, &[("pix", 2_500)], "Desistiu de parte").unwrap();

        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        assert_eq!((account.balance_cents, account.refunded_cents), (32_500, 2_500));
        assert_eq!(account.refunds[0].components[0].method, "pix");
        assert_eq!(account.payments[0].refundable_cents, 7_500);
    }

    #[test]
    fn cancellation_exposes_pending_refund_and_it_shrinks_with_each_refund() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-11-10", "2026-11-13"); // total R$ 600,00; sinal R$ 180,00
        let receipt = w
            .h
            .payments
            .record(
                &w.admin,
                PaymentTarget::Reservation { reservation_id: id, expected_version: 1 },
                PaymentInput { components: parts(&[("pix", 12_000), ("cash", 6_000)]), note: String::new(), duplicate_release: None },
            )
            .unwrap();
        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;
        let (payments, refunds) = (count(&w, "payments"), count(&w, "refunds"));

        w.h.reservations.cancel(&w.admin, id, version, None).unwrap();

        assert_eq!((count(&w, "payments"), count(&w, "refunds")), (payments, refunds), "o cancelamento não mexe em recebimentos");
        let due = |w: &World| w.h.reservations.get(&w.admin, id).unwrap().reservation.refund_due_cents;
        assert_eq!(due(&w), 18_000);
        refund(&w, receipt.id, &[("pix", 5_000)], "Devolução do sinal").unwrap();
        assert_eq!(due(&w), 13_000);
        refund(&w, receipt.id, &[("pix", 7_000), ("cash", 6_000)], "Resto do sinal").unwrap();
        assert_eq!(due(&w), 0);
        assert!(matches!(refund(&w, receipt.id, &[("cash", 1)], "Extra"), Err(AppError::Validation(_))));
        let detail = w.h.reservations.get(&w.admin, id).unwrap();
        assert_eq!((detail.payments[0].refunded_cents, detail.payments[0].refundable_cents), (18_000, 0));
    }

    #[test]
    fn cancellation_with_exception_exposes_the_authorized_refund() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-10-22", "2026-10-24"); // 1 dia antes: política retém tudo
        w.h.payments
            .record(
                &w.admin,
                PaymentTarget::Reservation { reservation_id: id, expected_version: 1 },
                PaymentInput { components: parts(&[("pix", 12_000)]), note: String::new(), duplicate_release: None },
            )
            .unwrap();
        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;
        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.refund_due_cents, 0);

        w.h.reservations
            .cancel(&w.admin, id, version, Some(CancellationOverride { retained_cents: 2_000, justification: "Doença na família".into() }))
            .unwrap();

        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.refund_due_cents, 10_000);
    }

    #[test]
    fn refund_made_before_check_in_stays_on_the_stay_account() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23"); // R$ 400,00
        let deposit = w
            .h
            .payments
            .record(
                &w.admin,
                PaymentTarget::Reservation { reservation_id: reservation, expected_version: 1 },
                PaymentInput { components: parts(&[("pix", 12_000)]), note: String::new(), duplicate_release: None },
            )
            .unwrap();
        refund(&w, deposit.id, &[("pix", 5_000)], "Ajuste do sinal").unwrap();
        w.h.utc.set(DateTime::parse_from_rfc3339("2026-10-21T17:00:00Z").unwrap().with_timezone(&Utc));
        let version = w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version;
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };

        let stay = w
            .h
            .stays
            .check_in(&w.admin, CheckInInput { reservation_id: reservation, expected_version: version, occupants: vec![holder], exceed_capacity: false })
            .unwrap();

        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        assert_eq!((account.received_cents, account.refunded_cents), (12_000, 5_000));
        assert_eq!(account.balance_cents, 40_000 - 12_000 + 5_000);
        assert_eq!(account.refunds.len(), 1);
    }

    #[test]
    fn refunding_a_deposit_of_an_open_reservation_lowers_the_deposit_paid() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-11-10", "2026-11-13"); // total R$ 600,00; sinal R$ 180,00
        let deposit = w
            .h
            .payments
            .record(
                &w.admin,
                PaymentTarget::Reservation { reservation_id: id, expected_version: 1 },
                PaymentInput { components: parts(&[("pix", 18_000)]), note: String::new(), duplicate_release: None },
            )
            .unwrap();

        refund(&w, deposit.id, &[("pix", 5_000)], "Devolução parcial do sinal").unwrap();

        let r = w.h.reservations.get(&w.admin, id).unwrap().reservation;
        assert_eq!(r.deposit_paid_cents, 13_000);
        // o teto do sinal considera o que de fato ficou: 13.000 + 47.000 = 60.000
        w.h.payments
            .record(
                &w.admin,
                PaymentTarget::Reservation { reservation_id: id, expected_version: r.version },
                PaymentInput { components: parts(&[("cash", 47_000)]), note: String::new(), duplicate_release: None },
            )
            .unwrap();
        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.deposit_paid_cents, 60_000);
    }

    #[test]
    fn refund_before_cancellation_is_not_discounted_twice() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-11-10", "2026-11-13");
        let deposit = w
            .h
            .payments
            .record(
                &w.admin,
                PaymentTarget::Reservation { reservation_id: id, expected_version: 1 },
                PaymentInput { components: parts(&[("pix", 18_000)]), note: String::new(), duplicate_release: None },
            )
            .unwrap();
        refund(&w, deposit.id, &[("pix", 6_000)], "Devolução parcial").unwrap();
        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;

        w.h.reservations.cancel(&w.admin, id, version, None).unwrap();

        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.refund_due_cents, 12_000);
        refund(&w, deposit.id, &[("pix", 12_000)], "Resto do sinal").unwrap();
        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.refund_due_cents, 0);
    }

    #[test]
    fn refunding_more_than_the_cancellation_policy_needs_the_exception_permission() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-10-24", "2026-10-26"); // 3 dias antes: a política devolve 50%
        let deposit = w
            .h
            .payments
            .record(
                &w.admin,
                PaymentTarget::Reservation { reservation_id: id, expected_version: 1 },
                PaymentInput { components: parts(&[("pix", 12_000)]), note: String::new(), duplicate_release: None },
            )
            .unwrap();
        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;
        w.h.reservations.cancel(&w.admin, id, version, None).unwrap();
        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.refund_due_cents, 6_000);
        let _ = crate::services::test_support::recepcao_token(&w.h, &w.admin);
        let rec = w.h.auth.login("rec", "Definitiva-456").unwrap().token;
        let input = |cents| RefundInput { payment_id: deposit.id, components: parts(&[("pix", cents)]), reason: "Pediu tudo de volta".into() };

        assert!(matches!(w.h.refunds.record(&rec, input(7_000)), Err(AppError::Forbidden)), "acima do devido sem a permissão");
        assert!(w.h.refunds.record(&rec, input(6_000)).is_ok(), "o devido pela política é rotina da recepção");
        assert!(w.h.refunds.record(&w.admin, input(6_000)).is_ok(), "com CancellationException passa do devido");
        assert_eq!(count(&w, "refunds"), 2);
    }

    #[test]
    fn no_show_has_no_pending_refund() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        w.h.payments
            .record(
                &w.admin,
                PaymentTarget::Reservation { reservation_id: id, expected_version: 1 },
                PaymentInput { components: parts(&[("pix", 12_000)]), note: String::new(), duplicate_release: None },
            )
            .unwrap();
        w.h.utc.set(DateTime::parse_from_rfc3339("2026-10-21T20:00:00Z").unwrap().with_timezone(&Utc));
        let version = w.h.reservations.get(&w.admin, id).unwrap().reservation.version;

        w.h.reservations.mark_no_show(&w.admin, id, version).unwrap();

        assert_eq!(w.h.reservations.get(&w.admin, id).unwrap().reservation.refund_due_cents, 0);
    }

    #[test]
    fn refund_audit_records_user_reason_and_amount() {
        let w = world();
        let (_, payment) = stay_with_payment(&w);

        refund(&w, payment, &[("pix", 3_000)], "Cobrança em dobro").unwrap();

        let (user, after, why): (i64, String, String) = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT user_id, after_json, justification FROM audit_log WHERE operation = 'record_refund'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert!(user > 0 && after.contains("DEV-000001") && after.contains("3000"), "{after}");
        assert_eq!(why, "Cobrança em dobro");
    }

    #[test]
    fn double_click_cannot_refund_twice_beyond_the_balance() {
        let w = world();
        let (_, payment) = stay_with_payment(&w);

        assert!(refund(&w, payment, &[("pix", 6_000), ("cash", 4_000)], "Integral").is_ok());

        assert!(matches!(refund(&w, payment, &[("pix", 6_000), ("cash", 4_000)], "Integral"), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "refunds"), 1);
    }

    #[test]
    fn refund_of_a_payment_from_a_paid_stay_still_works_after_checkout() {
        let w = world();
        let (stay, payment) = stay_with_payment(&w);
        pay(&w.h, &w.admin, stay, 30_000, "cash").unwrap(); // quita a conta
        w.h.db.lock().unwrap().execute("UPDATE stays SET status = 'checked_out'", []).unwrap();

        assert!(refund(&w, payment, &[("pix", 1_000)], "Crédito devolvido").is_ok());
    }
}
