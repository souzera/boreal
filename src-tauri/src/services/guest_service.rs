use serde::Serialize;
use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::domain::{cpf, phone, text::fold};
use crate::error::AppError;
use crate::models::guest::{DuplicateMatch, Guest, GuestInput, GuestQuery};
use crate::models::page::{clamp_paging, Page};
use crate::repositories::guest_repo::{self, CleanGuest};
use crate::repositories::sequence_repo;

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

/// Resultado da criação: cadastrado, ou conferência de possíveis duplicidades (nada gravado).
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CreateGuestOutcome {
    Created { id: i64, code: String },
    Duplicates { matches: Vec<DuplicateMatch> },
}

/// Dados validados e normalizados.
struct Clean {
    full_name: String,
    phone: String,
    cpf: Option<String>,
    email: String,
    address: String,
}

impl Clean {
    fn as_repo(&self) -> CleanGuest<'_> {
        CleanGuest {
            full_name: &self.full_name,
            phone: &self.phone,
            cpf: self.cpf.as_deref(),
            email: &self.email,
            address: &self.address,
        }
    }
}

fn clean(input: &GuestInput) -> Result<Clean, AppError> {
    let full_name = input.full_name.split_whitespace().collect::<Vec<_>>().join(" ");
    if full_name.is_empty() {
        return Err(invalid("Informe o nome completo."));
    }
    if full_name.chars().count() > 120 {
        return Err(invalid("Nome: no máximo 120 caracteres."));
    }
    if input.phone.trim().is_empty() {
        return Err(invalid("Informe o telefone."));
    }
    let phone = phone::normalize(&input.phone)
        .ok_or_else(|| invalid("Telefone inválido. Informe o DDD e o número."))?;
    let cpf = match input.cpf.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => Some(cpf::normalize(c).ok_or_else(|| invalid("CPF inválido. Confira os números."))?),
        None => None,
    };
    let email = input.email.trim().to_string();
    if !email.is_empty() && (!email.contains('@') || email.chars().count() > 120 || email.contains(' ')) {
        return Err(invalid("E-mail inválido."));
    }
    let address = input.address.trim().to_string();
    if address.chars().count() > 255 {
        return Err(invalid("Endereço: no máximo 255 caracteres."));
    }
    Ok(Clean { full_name, phone, cpf, email, address })
}

/// Na auditoria só vão o nome e o CPF mascarado; contato e endereço ficam de fora.
fn audit_view(c: &Clean) -> serde_json::Value {
    json!({
        "full_name": c.full_name,
        "cpf": c.cpf.as_deref().map(cpf::mask),
        "has_email": !c.email.is_empty(),
        "has_address": !c.address.is_empty(),
    })
}

fn audit_view_of(g: &Guest) -> serde_json::Value {
    json!({
        "full_name": g.full_name,
        "cpf": g.cpf.as_deref().map(cpf::mask),
        "has_email": !g.email.is_empty(),
        "has_address": !g.address.is_empty(),
    })
}

#[derive(Clone)]
pub struct GuestService {
    ctx: Ctx,
}

impl GuestService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn session(&self, token: &str) -> Result<SessionInfo, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())
    }

    fn record(
        &self,
        tx: &rusqlite::Transaction,
        actor: &SessionInfo,
        operation: &'static str,
        entity_id: i64,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        audit::record(
            tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "guests",
                operation,
                entity: "guest",
                entity_id: Some(entity_id),
                before,
                after,
                justification: None,
            },
        )
    }

    pub fn find_possible_duplicates(&self, token: &str, input: &GuestInput) -> Result<Vec<DuplicateMatch>, AppError> {
        self.session(token)?;
        let c = clean(input)?;
        let conn = self.ctx.conn()?;
        guest_repo::find_possible_duplicates(&conn, &fold(&c.full_name), Some(&c.phone), c.cpf.as_deref())
    }

    /// Cria o hóspede. Com possíveis duplicidades e sem `confirm_duplicates`, nada é gravado
    /// e a conferência é devolvida para a recepção decidir.
    pub fn create(&self, token: &str, input: GuestInput, confirm_duplicates: bool) -> Result<CreateGuestOutcome, AppError> {
        let actor = self.session(token)?;
        let c = clean(&input)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        if !confirm_duplicates {
            let matches = guest_repo::find_possible_duplicates(&tx, &fold(&c.full_name), Some(&c.phone), c.cpf.as_deref())?;
            if !matches.is_empty() {
                return Ok(CreateGuestOutcome::Duplicates { matches });
            }
        }
        let number = sequence_repo::next(&tx, "guest")?;
        let code = sequence_repo::format_code("HOS", number);
        let id = guest_repo::insert(&tx, &code, &c.as_repo(), self.ctx.utc.now_utc())?;
        self.record(&tx, &actor, "create_guest", id, None, Some(audit_view(&c)))?;
        tx.commit()?;
        Ok(CreateGuestOutcome::Created { id, code })
    }

    pub fn get(&self, token: &str, id: i64) -> Result<Guest, AppError> {
        self.session(token)?;
        let conn = self.ctx.conn()?;
        guest_repo::find(&conn, id)?.ok_or_else(|| invalid("Hóspede não encontrado."))
    }

    pub fn update(&self, token: &str, id: i64, input: GuestInput, expected_version: i64) -> Result<(), AppError> {
        let actor = self.session(token)?;
        let c = clean(&input)?;
        let mut conn = self.ctx.conn()?;
        let before = guest_repo::find(&conn, id)?.ok_or_else(|| invalid("Hóspede não encontrado."))?;
        let tx = conn.transaction()?;
        guest_repo::update(&tx, id, &c.as_repo(), expected_version, self.ctx.utc.now_utc())?;
        self.record(&tx, &actor, "update_guest", id, Some(audit_view_of(&before)), Some(audit_view(&c)))?;
        tx.commit()?;
        Ok(())
    }

    /// Desativar não apaga o histórico (exige `ManageCatalog`).
    pub fn set_active(&self, token: &str, id: i64, active: bool) -> Result<(), AppError> {
        let actor = self.session(token)?;
        require(&actor, Permission::ManageCatalog)?;
        let mut conn = self.ctx.conn()?;
        let before = guest_repo::find(&conn, id)?.ok_or_else(|| invalid("Hóspede não encontrado."))?;
        let tx = conn.transaction()?;
        guest_repo::set_active(&tx, id, active, self.ctx.utc.now_utc())?;
        self.record(&tx, &actor, "set_guest_active", id, Some(json!({ "active": before.active })), Some(json!({ "active": active })))?;
        tx.commit()?;
        Ok(())
    }

    pub fn list(&self, token: &str, query: GuestQuery) -> Result<Page<Guest>, AppError> {
        self.session(token)?;
        let (page, page_size, offset) = clamp_paging(query.page, query.page_size);
        let conn = self.ctx.conn()?;
        let (items, total) = guest_repo::list(&conn, &query, i64::from(page_size), offset)?;
        Ok(Page { items, total, page, page_size })
    }
}

#[cfg(test)]
mod tests {
    use super::CreateGuestOutcome;
    use crate::error::AppError;
    use crate::models::guest::{DuplicateReason, GuestInput, GuestQuery};
    use crate::services::test_support::{admin_token, harness, recepcao_token, Harness};

    fn input(name: &str, phone: &str, cpf: Option<&str>) -> GuestInput {
        GuestInput {
            full_name: name.into(),
            phone: phone.into(),
            cpf: cpf.map(String::from),
            email: "hospede@example.com".into(),
            address: "Rua das Flores, 100".into(),
        }
    }

    fn query() -> GuestQuery {
        GuestQuery { text: None, active: None, page: 1, page_size: 20, sort: None, direction: None }
    }

    fn created(outcome: CreateGuestOutcome) -> (i64, String) {
        match outcome {
            CreateGuestOutcome::Created { id, code } => (id, code),
            other => panic!("esperava Created, veio {other:?}"),
        }
    }

    fn create(h: &Harness, token: &str, i: GuestInput) -> (i64, String) {
        created(h.guests.create(token, i, false).unwrap())
    }

    const CPF: &str = "529.982.247-25";

    #[test]
    fn name_and_phone_are_required_and_formats_are_validated() {
        let h = harness();
        let admin = admin_token(&h);
        let bad = [
            input("  ", "(11) 99999-0000", None),
            input("Ana Souza", "", None),
            input("Ana Souza", "123", None),
            input("Ana Souza", "(11) 99999-0000", Some("111.111.111-11")),
            GuestInput { email: "sem-arroba".into(), ..input("Ana Souza", "(11) 99999-0000", None) },
        ];

        for i in bad {
            assert!(matches!(h.guests.create(&admin, i, false), Err(AppError::Validation(_))));
        }
        assert_eq!(h.guests.list(&admin, query()).unwrap().total, 0);
    }

    #[test]
    fn optional_fields_may_be_empty_and_values_are_normalized() {
        let h = harness();
        let admin = admin_token(&h);
        let mut i = input("  Ana   Souza ", "+55 (11) 99999-0000", Some(CPF));
        i.email = String::new();
        i.address = String::new();

        let (id, code) = create(&h, &admin, i);

        assert_eq!(code, "HOS-000001");
        let guest = h.guests.get(&admin, id).unwrap();
        assert_eq!(guest.full_name, "Ana Souza");
        assert_eq!(guest.phone, "11999990000");
        assert_eq!(guest.cpf.as_deref(), Some("52998224725"));
        let (_, second) = create(&h, &admin, input("Bia Lima", "(11) 98888-0000", None));
        assert_eq!(second, "HOS-000002");
    }

    #[test]
    fn finds_duplicates_by_name_phone_and_cpf_in_any_format() {
        let h = harness();
        let admin = admin_token(&h);
        create(&h, &admin, input("José da Silva", "(11) 99999-0000", Some(CPF)));

        let by_name = h.guests.find_possible_duplicates(&admin, &input("jose  DA silva", "(21) 97777-0000", None)).unwrap();
        let by_phone = h.guests.find_possible_duplicates(&admin, &input("Outra Pessoa", "11 99999 0000", None)).unwrap();
        let by_cpf = h.guests.find_possible_duplicates(&admin, &input("Outra Pessoa", "(21) 97777-0000", Some("52998224725"))).unwrap();
        let none = h.guests.find_possible_duplicates(&admin, &input("Maria Nova", "(21) 97777-0000", None)).unwrap();

        assert_eq!(by_name[0].reasons, vec![DuplicateReason::Name]);
        assert_eq!(by_phone[0].reasons, vec![DuplicateReason::Phone]);
        assert_eq!(by_cpf[0].reasons, vec![DuplicateReason::Cpf]);
        assert!(none.is_empty());
    }

    #[test]
    fn create_with_duplicates_stores_nothing_until_confirmed() {
        let h = harness();
        let admin = admin_token(&h);
        create(&h, &admin, input("José da Silva", "(11) 99999-0000", None));
        let again = input("José da Silva", "(11) 91111-2222", None);

        match h.guests.create(&admin, again, false).unwrap() {
            CreateGuestOutcome::Duplicates { matches } => assert_eq!(matches.len(), 1),
            other => panic!("esperava Duplicates, veio {other:?}"),
        }
        assert_eq!(h.guests.list(&admin, query()).unwrap().total, 1);

        let (_, code) = created(
            h.guests.create(&admin, input("José da Silva", "(11) 91111-2222", None), true).unwrap(),
        );
        assert_eq!(code, "HOS-000002");
        assert_eq!(h.guests.list(&admin, query()).unwrap().total, 2);
    }

    #[test]
    fn update_changes_data_and_conflicts_on_stale_version() {
        let h = harness();
        let admin = admin_token(&h);
        let (id, _) = create(&h, &admin, input("Ana Souza", "(11) 99999-0000", None));

        h.guests.update(&admin, id, input("Ana Souza Lima", "(11) 99999-0000", None), 1).unwrap();
        assert_eq!(h.guests.get(&admin, id).unwrap().full_name, "Ana Souza Lima");
        assert!(matches!(
            h.guests.update(&admin, id, input("Ana S.", "(11) 99999-0000", None), 1),
            Err(AppError::Conflict)
        ));
    }

    #[test]
    fn recepcao_creates_and_edits_but_only_admin_deactivates() {
        let h = harness();
        let admin = admin_token(&h);
        let rec = recepcao_token(&h, &admin);

        let (id, _) = create(&h, &rec, input("Ana Souza", "(11) 99999-0000", None));
        h.guests.update(&rec, id, input("Ana Souza Lima", "(11) 99999-0000", None), 1).unwrap();

        assert!(matches!(h.guests.set_active(&rec, id, false), Err(AppError::Forbidden)));
        h.guests.set_active(&admin, id, false).unwrap();
        assert!(!h.guests.get(&admin, id).unwrap().active);
        assert_eq!(h.guests.list(&admin, GuestQuery { active: Some(true), ..query() }).unwrap().total, 0);
    }

    #[test]
    fn search_ignores_accents_and_matches_phone_cpf_and_code() {
        let h = harness();
        let admin = admin_token(&h);
        create(&h, &admin, input("José da Silva", "(11) 99999-0000", Some(CPF)));
        create(&h, &admin, input("Bia Lima", "(21) 98888-7777", None));
        let find = |text: &str| h.guests.list(&admin, GuestQuery { text: Some(text.into()), ..query() }).unwrap();

        assert_eq!(find("JOSE").total, 1);
        assert_eq!(find("silva").items[0].full_name, "José da Silva");
        assert_eq!(find("98888").total, 1);
        assert_eq!(find("52998224725").total, 1);
        assert_eq!(find("529.982").total, 1);
        assert_eq!(find("HOS-000002").total, 1);
        assert_eq!(find("zzz").total, 0);
    }

    #[test]
    fn list_paginates() {
        let h = harness();
        let admin = admin_token(&h);
        for n in 0..5 {
            create(&h, &admin, input(&format!("Hóspede {n}"), &format!("(11) 9999{n}-0000"), None));
        }

        let page = h.guests.list(&admin, GuestQuery { page: 2, page_size: 2, ..query() }).unwrap();

        assert_eq!((page.total, page.items.len()), (5, 2));
    }

    #[test]
    fn audit_masks_cpf_and_omits_contact_details() {
        let h = harness();
        let admin = admin_token(&h);
        let (id, _) = create(&h, &admin, input("Ana Souza", "(11) 99999-0000", Some(CPF)));
        h.guests.update(&admin, id, input("Ana Souza Lima", "(11) 99999-0000", Some(CPF)), 1).unwrap();

        let conn = h.db.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT COALESCE(before_json,'') || COALESCE(after_json,'') FROM audit_log WHERE module = 'guests'")
            .unwrap();
        let all: String = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap().map(|r| r.unwrap()).collect();

        assert!(all.contains("***.***.***-25"));
        for secret in ["52998224725", "529.982.247-25", "hospede@example.com", "Rua das Flores", "99999-0000", "11999990000"] {
            assert!(!all.contains(secret), "auditoria contém {secret}");
        }
        assert!(all.contains("Ana Souza Lima"));
    }
}
