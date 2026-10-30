//! Recebimentos divididos por modalidade: conta da hospedagem e sinal da reserva (spec §9).

use std::collections::BTreeSet;

use chrono::Duration;
use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::domain::money;
use crate::domain::payment::{self, Component, Method, PaymentFacts};
use crate::error::AppError;
use crate::models::payment::{ComponentInput, PaymentInput, PaymentReceipt, PaymentTarget};
use crate::repositories::payment_repo::{self, DuplicateRef, NewPayment};
use crate::repositories::{reservation_repo, sequence_repo, stay_repo};

/// Teto de um recebimento (R$ 1.000.000,00): barra erros de digitação e estouros.
pub(crate) const MAX_PAYMENT_CENTS: i64 = 100_000_000;

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

pub(crate) fn parse_components(items: &[ComponentInput]) -> Result<Vec<Component>, AppError> {
    items
        .iter()
        .map(|i| {
            let method = Method::parse(&i.method).ok_or_else(|| invalid("Escolha uma forma de pagamento válida."))?;
            Ok(Component { method, amount_cents: i.amount_cents })
        })
        .collect()
}

#[derive(Clone)]
pub struct PaymentService {
    ctx: Ctx,
}

impl PaymentService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    /// Grava um recebimento (hospedagem ou sinal) com todos os componentes, o número, a atualização do
    /// saldo ou da reserva e a auditoria na mesma transação. Possível duplicado só passa com liberação.
    pub fn record(&self, token: &str, target: PaymentTarget, input: PaymentInput) -> Result<PaymentReceipt, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let components = parse_components(&input.components)?;
        let total = payment::validate_components(&components, MAX_PAYMENT_CENTS)?;
        let note = input.note.trim();
        if note.chars().count() > 200 {
            return Err(invalid("Observação: no máximo 200 caracteres."));
        }
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let at = self.ctx.utc.now_utc();
        let offset = self.ctx.utc.local_offset();

        // Alvo: quem recebe, com as validações próprias de cada caso.
        let (reservation_id, stay_id, reservation) = match target {
            PaymentTarget::Stay { stay_id } => {
                let stay = stay_repo::find(&tx, stay_id)?.ok_or_else(|| invalid("Hospedagem não encontrada."))?;
                if stay.status != "in_house" {
                    return Err(invalid("A hospedagem já foi encerrada. Não é possível mudar a conta."));
                }
                (stay.reservation_id, Some(stay_id), None)
            }
            PaymentTarget::Reservation { reservation_id, expected_version } => {
                let reservation = reservation_repo::find(&tx, reservation_id, self.ctx.today())?
                    .ok_or_else(|| invalid("Reserva não encontrada."))?;
                if !matches!(reservation.status.as_str(), "pending" | "confirmed") {
                    return Err(invalid("Só reservas pendentes ou confirmadas aceitam registro de sinal."));
                }
                if reservation.version != expected_version {
                    return Err(AppError::Conflict);
                }
                if money::add(reservation.deposit_paid_cents, total)? > reservation.agreed_total_cents {
                    return Err(invalid("O sinal recebido não pode passar do valor total da reserva."));
                }
                (reservation_id, None, Some(reservation))
            }
        };

        // Duplicidade (M5-6): bloqueia, e só uma liberação autorizada e justificada deixa passar.
        let candidate = PaymentFacts {
            total_cents: total,
            methods: components.iter().map(|c| c.method).collect::<BTreeSet<_>>(),
            local_date: (at + Duration::seconds(i64::from(offset.local_minus_utc()))).date_naive(),
        };
        let mut released: Option<(i64, String)> = None;
        if let Some(similar) = payment_repo::find_similar(&tx, reservation_id, &candidate, offset)? {
            let Some(release) = &input.duplicate_release else {
                return Err(AppError::DuplicatePayment(Box::new(similar.info)));
            };
            require(&actor, Permission::ReleaseDuplicatePayment)?;
            let why = release.justification.trim();
            if why.is_empty() {
                return Err(invalid("Informe a justificativa para liberar o recebimento duplicado."));
            }
            if why.chars().count() > 200 {
                return Err(invalid("Justificativa: no máximo 200 caracteres."));
            }
            released = Some((similar.id, why.to_string()));
        }

        let before_balance = match stay_id {
            Some(id) => Some(stay_repo::balance(&tx, id)?),
            None => None,
        };
        let code = sequence_repo::format_code("PAG", sequence_repo::next(&tx, "payment")?);
        let id = payment_repo::insert(
            &tx,
            &NewPayment {
                code: &code,
                reservation_id,
                stay_id,
                kind: if stay_id.is_some() { "stay" } else { "deposit" },
                amount_cents: total,
                received_by: actor.user_id,
                note,
                duplicate: released.as_ref().map(|(of, why)| DuplicateRef {
                    of_payment_id: *of,
                    justification: why,
                    released_by: actor.user_id,
                }),
            },
            at,
        )?;
        payment_repo::insert_components(&tx, id, &components)?;

        let entry = json!({
            "payment_id": id, "code": code, "amount_cents": total,
            "components": components.iter().map(|c| json!({ "method": c.method.as_str(), "amount_cents": c.amount_cents })).collect::<Vec<_>>(),
            "duplicate_of_payment_id": released.as_ref().map(|(of, _)| of),
        });
        let justification = released.map(|(_, why)| why);

        if let (Some(stay_id), Some(before)) = (stay_id, before_balance) {
            stay_repo::bump(&tx, stay_id, at)?;
            audit::record(
                &tx,
                &AuditEntry {
                    at,
                    user_id: Some(actor.user_id),
                    module: "stays",
                    operation: "record_payment",
                    entity: "stay",
                    entity_id: Some(stay_id),
                    before: Some(json!({ "balance_cents": before })),
                    after: Some(json!({ "balance_cents": stay_repo::balance(&tx, stay_id)?, "entry": entry })),
                    justification,
                },
            )?;
        } else if let Some(reservation) = reservation {
            let paid = money::add(reservation.deposit_paid_cents, total)?;
            let new_status = if reservation.status == "pending" && paid >= reservation.deposit_expected_cents {
                "confirmed"
            } else {
                reservation.status.as_str()
            };
            reservation_repo::update_payment_state(&tx, reservation.id, paid, new_status, reservation.version, at)?;
            let before = json!({ "deposit_paid_cents": reservation.deposit_paid_cents, "status": reservation.status });
            let after = json!({ "deposit_paid_cents": paid, "status": new_status, "entry": entry });
            reservation_repo::insert_change(
                &tx, reservation.id, at, actor.user_id, "deposit", &before.to_string(), &after.to_string(), None, None, justification.as_deref(),
            )?;
            audit::record(
                &tx,
                &AuditEntry {
                    at,
                    user_id: Some(actor.user_id),
                    module: "reservations",
                    operation: "record_deposit",
                    entity: "reservation",
                    entity_id: Some(reservation.id),
                    before: Some(before),
                    after: Some(after),
                    justification,
                },
            )?;
        }
        tx.commit()?;
        Ok(PaymentReceipt { id, code, total_cents: total })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::auth::permissions::Permission;
    use crate::error::AppError;
    use crate::models::payment::{ComponentInput, DuplicateRelease, PaymentInput, PaymentTarget};
    use crate::models::stay::{CheckInInput, OccupantInput};
    use crate::services::test_support::{admin_token, book, deposit, harness, pay, recepcao_token, seed, Harness, Seed};

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

    fn set_now(w: &World, rfc3339: &str) {
        w.h.utc.set(DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc));
    }

    fn version(w: &World, reservation: i64) -> i64 {
        w.h.reservations.get(&w.admin, reservation).unwrap().reservation.version
    }

    fn check_in(w: &World, reservation: i64) -> Result<i64, AppError> {
        set_now(w, "2026-10-21T17:00:00Z");
        let holder = OccupantInput { full_name: "Ana Souza".into(), guest_id: Some(w.seed.guest_id), document: String::new() };
        w.h.stays.check_in(
            &w.admin,
            CheckInInput { reservation_id: reservation, expected_version: version(w, reservation), occupants: vec![holder], exceed_capacity: false },
        )
    }

    /// Hospedagem 21→23/10 em casa; conta de R$ 400,00.
    fn stay(w: &World) -> i64 {
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23");
        check_in(w, reservation).unwrap()
    }

    fn count(w: &World, table: &str) -> i64 {
        w.h.db.lock().unwrap().query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    fn balance(w: &World, stay: i64) -> i64 {
        w.h.stays.get(&w.admin, stay).unwrap().account.balance_cents
    }

    fn split(parts: &[(&str, i64)]) -> PaymentInput {
        PaymentInput {
            components: parts.iter().map(|(m, c)| ComponentInput { method: (*m).into(), amount_cents: *c }).collect(),
            note: String::new(),
            duplicate_release: None,
        }
    }

    fn login_with(w: &World, permissions: &[Permission]) -> String {
        if w.h.auth.login("rec", "Definitiva-456").is_err() {
            let _ = recepcao_token(&w.h, &w.admin);
        }
        let rec_id = w.h.auth.login("rec", "Definitiva-456").unwrap().user.id;
        w.h.users.set_extra_permissions(&w.admin, rec_id, permissions).unwrap();
        w.h.auth.login("rec", "Definitiva-456").unwrap().token
    }

    #[test]
    fn split_payment_records_components_and_reduces_balance() {
        let w = world();
        let stay = stay(&w);

        let receipt = w.h.payments.record(&w.admin, PaymentTarget::Stay { stay_id: stay }, split(&[("pix", 6_000), ("cash", 4_000)])).unwrap();

        assert_eq!((receipt.code.as_str(), receipt.total_cents), ("PAG-000001", 10_000));
        assert_eq!(balance(&w, stay), 30_000);
        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        let line = &account.payments[0];
        assert_eq!(line.amount_cents, 10_000);
        let parts: Vec<(&str, i64)> = line.components.iter().map(|c| (c.method.as_str(), c.amount_cents)).collect();
        assert_eq!(parts, [("cash", 4_000), ("pix", 6_000)]);
        assert_eq!(count(&w, "payment_components"), 2);
    }

    #[test]
    fn components_not_matching_are_refused_without_writing() {
        let w = world();
        let stay = stay(&w);
        let bad = [
            split(&[]),
            split(&[("pix", 0)]),
            split(&[("pix", -5)]),
            split(&[("pix", 100), ("pix", 200)]),
            split(&[("prior_deposit", 100)]),
            split(&[("unspecified", 100)]),
            split(&[("cheque", 100)]),
            split(&[("cash", 100_000_001)]),
            split(&[("cash", i64::MAX), ("pix", 1)]),
        ];

        for input in bad {
            let result = w.h.payments.record(&w.admin, PaymentTarget::Stay { stay_id: stay }, input);
            assert!(matches!(result, Err(AppError::Validation(_))), "{result:?}");
        }

        assert_eq!((count(&w, "payments"), count(&w, "payment_components")), (0, 0));
        assert_eq!(balance(&w, stay), 40_000);
        let next: i64 = w.h.db.lock().unwrap().query_row("SELECT next_value FROM sequences WHERE name = 'payment'", [], |r| r.get(0)).unwrap();
        assert_eq!(next, 1, "a sequência não foi consumida");
    }

    #[test]
    fn same_stay_same_total_methods_and_day_is_blocked_as_duplicate() {
        let w = world();
        let stay = stay(&w);
        pay(&w.h, &w.admin, stay, 10_000, "pix").unwrap();

        let second = pay(&w.h, &w.admin, stay, 10_000, "pix");

        match second {
            Err(AppError::DuplicatePayment(info)) => {
                assert_eq!((info.code.as_str(), info.total_cents), ("PAG-000001", 10_000));
                assert_eq!(info.methods, ["pix"]);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(count(&w, "payments"), 1);
        assert_eq!(balance(&w, stay), 30_000);
        // outra modalidade, outro valor ou outro dia não é duplicidade
        pay(&w.h, &w.admin, stay, 10_000, "cash").unwrap();
        pay(&w.h, &w.admin, stay, 10_001, "pix").unwrap();
        set_now(&w, "2026-10-22T12:00:00Z");
        pay(&w.h, &w.admin, stay, 10_000, "pix").unwrap();
        // a recusa não consumiu número de recebimento
        assert_eq!(count(&w, "payments"), 4);
        let last: String = w.h.db.lock().unwrap().query_row("SELECT code FROM payments ORDER BY id DESC LIMIT 1", [], |r| r.get(0)).unwrap();
        assert_eq!(last, "PAG-000004");
    }

    #[test]
    fn duplicate_release_requires_permission_and_justification() {
        let w = world();
        let stay = stay(&w);
        pay(&w.h, &w.admin, stay, 10_000, "pix").unwrap();
        let release = |why: &str| {
            let mut input = split(&[("pix", 10_000)]);
            input.duplicate_release = Some(DuplicateRelease { justification: why.into() });
            input
        };
        let target = PaymentTarget::Stay { stay_id: stay };

        let rec = login_with(&w, &[]);
        assert!(matches!(w.h.payments.record(&rec, target, release("Dois hóspedes")), Err(AppError::Forbidden)));
        assert!(matches!(w.h.payments.record(&w.admin, target, release("   ")), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "payments"), 1);

        let receipt = w.h.payments.record(&w.admin, target, release("Dois hóspedes pagaram o mesmo valor")).unwrap();

        assert_eq!(receipt.code, "PAG-000002");
        let (of, why, by): (i64, String, i64) = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT duplicate_of_payment_id, duplicate_justification, duplicate_released_by FROM payments WHERE code = 'PAG-000002'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((of, why.as_str()), (1, "Dois hóspedes pagaram o mesmo valor"));
        assert!(by > 0);
        let audited: String = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT justification FROM audit_log WHERE operation = 'record_payment' ORDER BY id DESC LIMIT 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(audited, "Dois hóspedes pagaram o mesmo valor");
        let rec_with = login_with(&w, &[Permission::ReleaseDuplicatePayment]);
        assert!(w.h.payments.record(&rec_with, target, release("Outro")).is_ok(), "com a permissão concedida");
    }

    #[test]
    fn deposit_on_reservation_confirms_it_and_keeps_deposit_paid_in_sync() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-10-25", "2026-10-28"); // total R$ 600,00; sinal esperado R$ 180,00
        let target = |v| PaymentTarget::Reservation { reservation_id: id, expected_version: v };

        w.h.payments.record(&w.admin, target(1), split(&[("pix", 10_000)])).unwrap();
        let partial = w.h.reservations.get(&w.admin, id).unwrap().reservation;
        assert_eq!((partial.status.as_str(), partial.deposit_paid_cents), ("pending", 10_000));

        w.h.payments.record(&w.admin, target(2), split(&[("cash", 8_000)])).unwrap();
        let full = w.h.reservations.get(&w.admin, id).unwrap().reservation;
        assert_eq!((full.status.as_str(), full.deposit_paid_cents), ("confirmed", 18_000));
        let kinds: i64 = w.h.db.lock().unwrap().query_row("SELECT count(*) FROM payments WHERE kind = 'deposit' AND stay_id IS NULL", [], |r| r.get(0)).unwrap();
        assert_eq!(kinds, 2);

        assert!(matches!(w.h.payments.record(&w.admin, target(9), split(&[("cash", 1_000)])), Err(AppError::Conflict)));
        assert!(matches!(w.h.payments.record(&w.admin, target(3), split(&[("cash", 42_001)])), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "payments"), 2);
    }

    #[test]
    fn deposit_on_cancelled_or_closed_reservation_is_refused() {
        let w = world();
        let id = book(&w.h, &w.admin, &w.seed, "2026-10-25", "2026-10-28");
        w.h.db.lock().unwrap().execute("UPDATE reservations SET status = 'cancelled'", []).unwrap();

        assert!(matches!(deposit(&w.h, &w.admin, id, 1_000, 1), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "payments"), 0);
    }

    #[test]
    fn check_in_links_deposit_payments_instead_of_copying_them() {
        let w = world();
        let reservation = book(&w.h, &w.admin, &w.seed, "2026-10-21", "2026-10-23"); // sinal esperado R$ 120,00
        w.h.payments
            .record(&w.admin, PaymentTarget::Reservation { reservation_id: reservation, expected_version: 1 }, split(&[("pix", 7_000), ("cash", 5_000)]))
            .unwrap();
        assert_eq!(count(&w, "payments"), 1);

        let stay = check_in(&w, reservation).unwrap();

        assert_eq!(count(&w, "payments"), 1, "o recebimento foi vinculado, não copiado");
        let account = w.h.stays.get(&w.admin, stay).unwrap().account;
        assert_eq!(account.payments.len(), 1);
        assert_eq!(account.payments[0].code, "PAG-000001");
        assert_eq!(account.payments[0].components.len(), 2);
        assert_eq!(account.balance_cents, 40_000 - 12_000);
        let legacy: i64 = w.h.db.lock().unwrap().query_row("SELECT count(*) FROM payment_components WHERE method = 'unspecified'", [], |r| r.get(0)).unwrap();
        assert_eq!(legacy, 0);
        // check-in repetido é recusado e não revincula nem duplica
        let again = CheckInInput { reservation_id: reservation, expected_version: 1, occupants: vec![], exceed_capacity: false };
        assert!(w.h.stays.check_in(&w.admin, again).is_err());
        assert_eq!(count(&w, "payments"), 1);
    }

    #[test]
    fn payment_on_checked_out_stay_is_refused() {
        let w = world();
        let stay = stay(&w);
        w.h.db.lock().unwrap().execute("UPDATE stays SET status = 'checked_out'", []).unwrap();

        assert!(matches!(pay(&w.h, &w.admin, stay, 100, "cash"), Err(AppError::Validation(_))));
        assert_eq!(count(&w, "payments"), 0);
    }

    #[test]
    fn every_payment_is_audited_with_its_components() {
        let w = world();
        let stay = stay(&w);

        w.h.payments.record(&w.admin, PaymentTarget::Stay { stay_id: stay }, split(&[("debit", 3_000), ("credit", 2_000)])).unwrap();

        let after: String = w
            .h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT after_json FROM audit_log WHERE module = 'stays' AND operation = 'record_payment'", [], |r| r.get(0))
            .unwrap();
        assert!(after.contains("\"debit\"") && after.contains("\"credit\"") && after.contains("PAG-000001"), "{after}");
    }
}
