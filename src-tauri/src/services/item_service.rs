use serde_json::{json, Value};

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::domain::text::fold;
use crate::error::AppError;
use crate::models::item::{Item, ItemUpdate, NewItem};
use crate::repositories::item_repo;

/// Teto de preço de um item (R$ 100.000,00): evita valores absurdos por engano de digitação.
const MAX_PRICE_CENTS: i64 = 10_000_000;

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn clean(name: &str, cents: i64) -> Result<(String, String), AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(invalid("Informe o nome do item."));
    }
    if name.chars().count() > 80 {
        return Err(invalid("Nome do item: no máximo 80 caracteres."));
    }
    if !(1..=MAX_PRICE_CENTS).contains(&cents) {
        return Err(invalid("Informe um preço maior que zero e de até R$ 100.000,00."));
    }
    Ok((name.to_string(), fold(name)))
}

fn friendly(error: AppError) -> AppError {
    match error {
        AppError::Database(msg) if msg.contains("UNIQUE") => invalid("Já existe um item com este nome."),
        other => other,
    }
}

fn item_json(i: &Item) -> Value {
    json!({ "name": i.name, "default_price_cents": i.default_price_cents, "active": i.active })
}

#[derive(Clone)]
pub struct ItemService {
    ctx: Ctx,
}

impl ItemService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn write(&self, token: &str) -> Result<SessionInfo, AppError> {
        let info = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&info, Permission::ManageCatalog)?;
        Ok(info)
    }

    fn audit(
        &self,
        tx: &rusqlite::Transaction,
        actor: &SessionInfo,
        operation: &'static str,
        id: i64,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), AppError> {
        audit::record(
            tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "items",
                operation,
                entity: "catalog_item",
                entity_id: Some(id),
                before,
                after,
                justification: None,
            },
        )
    }

    pub fn list(&self, token: &str, only_active: bool) -> Result<Vec<Item>, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.conn()?;
        item_repo::list(&conn, only_active)
    }

    pub fn create(&self, token: &str, input: NewItem) -> Result<i64, AppError> {
        let actor = self.write(token)?;
        let (name, key) = clean(&input.name, input.default_price_cents)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let id = item_repo::insert(&tx, &name, &key, input.default_price_cents, self.ctx.utc.now_utc()).map_err(friendly)?;
        self.audit(
            &tx,
            &actor,
            "create_item",
            id,
            None,
            Some(json!({ "name": name, "default_price_cents": input.default_price_cents, "active": true })),
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub fn update(&self, token: &str, id: i64, input: ItemUpdate, expected_version: i64) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let (name, key) = clean(&input.name, input.default_price_cents)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let before = item_repo::find(&tx, id)?.ok_or_else(|| invalid("Item não encontrado."))?;
        item_repo::update(&tx, id, &name, &key, input.default_price_cents, expected_version, self.ctx.utc.now_utc())
            .map_err(friendly)?;
        self.audit(
            &tx,
            &actor,
            "update_item",
            id,
            Some(item_json(&before)),
            Some(json!({ "name": name, "default_price_cents": input.default_price_cents, "active": before.active })),
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Itens nunca são apagados (podem estar em lançamentos antigos); apenas desativados.
    pub fn set_active(&self, token: &str, id: i64, active: bool, expected_version: i64) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let before = item_repo::find(&tx, id)?.ok_or_else(|| invalid("Item não encontrado."))?;
        item_repo::set_active(&tx, id, active, expected_version, self.ctx.utc.now_utc())?;
        self.audit(&tx, &actor, "set_item_active", id, Some(json!({ "active": before.active })), Some(json!({ "active": active })))?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::models::item::{ItemUpdate, NewItem};
    use crate::services::test_support::{admin_token, harness, recepcao_token};

    fn new_item(name: &str, cents: i64) -> NewItem {
        NewItem { name: name.into(), default_price_cents: cents }
    }

    #[test]
    fn create_item_requires_manage_catalog() {
        let h = harness();
        let admin = admin_token(&h);
        let rec = recepcao_token(&h, &admin);

        assert!(matches!(h.items.create(&rec, new_item("Água mineral", 600)), Err(AppError::Forbidden)));
        assert!(h.items.create(&admin, new_item("Água mineral", 600)).is_ok());
        assert_eq!(h.items.list(&rec, false).unwrap().len(), 1, "recepção pode consultar");
    }

    #[test]
    fn item_name_is_unique_ignoring_case_and_accents_trimmed() {
        let h = harness();
        let admin = admin_token(&h);
        h.items.create(&admin, new_item("Água mineral", 600)).unwrap();

        let result = h.items.create(&admin, new_item("  agua MINERAL ", 700));

        assert!(matches!(result, Err(AppError::Validation(_))), "{result:?}");
        assert_eq!(h.items.list(&admin, false).unwrap().len(), 1);
    }

    #[test]
    fn name_and_price_are_validated() {
        let h = harness();
        let admin = admin_token(&h);

        for (name, cents) in [("Café", 0), ("Café", -1), ("   ", 500), ("Café", i64::MAX), ("Café", 10_000_001)] {
            let result = h.items.create(&admin, new_item(name, cents));
            assert!(matches!(result, Err(AppError::Validation(_))), "{name:?} {cents}");
        }
        assert!(h.items.list(&admin, false).unwrap().is_empty());
    }

    #[test]
    fn inactive_item_is_hidden_from_active_list_but_kept() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.items.create(&admin, new_item("Lavanderia", 2500)).unwrap();
        let version = h.items.list(&admin, false).unwrap()[0].version;

        h.items.set_active(&admin, id, false, version).unwrap();

        assert!(h.items.list(&admin, true).unwrap().is_empty());
        let all = h.items.list(&admin, false).unwrap();
        assert_eq!(all.len(), 1);
        assert!(!all[0].active);
    }

    #[test]
    fn stale_version_is_conflict() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.items.create(&admin, new_item("Café da manhã", 3000)).unwrap();
        let version = h.items.list(&admin, false).unwrap()[0].version;
        h.items
            .update(&admin, id, ItemUpdate { name: "Café da manhã".into(), default_price_cents: 3500 }, version)
            .unwrap();

        let stale = h.items.update(&admin, id, ItemUpdate { name: "Café".into(), default_price_cents: 4000 }, version);

        assert!(matches!(stale, Err(AppError::Conflict)));
        assert_eq!(h.items.list(&admin, false).unwrap()[0].default_price_cents, 3500);
    }

    #[test]
    fn item_changes_are_audited() {
        let h = harness();
        let admin = admin_token(&h);
        let id = h.items.create(&admin, new_item("Lavanderia", 2500)).unwrap();
        let version = h.items.list(&admin, false).unwrap()[0].version;
        h.items.set_active(&admin, id, false, version).unwrap();

        let n: i64 = h
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_log WHERE module = 'items'", [], |r| r.get(0))
            .unwrap();

        assert_eq!(n, 2);
    }
}
