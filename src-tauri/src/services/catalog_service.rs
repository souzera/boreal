use serde_json::{json, Value};

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::{require, SessionInfo};
use crate::error::AppError;
use crate::models::catalog::{
    Category, NewCategory, NewRoom, Room, RoomQuery, UpdateCategory, UpdateRoom,
};
use crate::models::page::{clamp_paging, Page};
use crate::repositories::catalog_repo::{self, RoomFields};

const MAX_CAPACITY: i64 = 50;

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn text(value: &str, max: usize, label: &str, required: bool) -> Result<String, AppError> {
    let value = value.trim();
    if required && value.is_empty() {
        return Err(invalid(format!("Informe {label}.")));
    }
    if value.chars().count() > max {
        return Err(invalid(format!("{label}: no máximo {max} caracteres.")));
    }
    Ok(value.to_string())
}

/// Traduz violações de integridade do banco em mensagens para a recepção.
fn friendly(error: AppError, unique: &str, referenced: &str) -> AppError {
    match error {
        AppError::Database(msg) if msg.contains("UNIQUE") => invalid(unique),
        AppError::Database(msg) if msg.contains("FOREIGN KEY") => invalid(referenced),
        other => other,
    }
}

fn category_json(c: &Category) -> Value {
    json!({ "name": c.name, "description": c.description, "active": c.active })
}

fn room_json(r: &Room) -> Value {
    json!({
        "code": r.code, "category_id": r.category_id, "max_capacity": r.max_capacity,
        "description": r.description, "notes": r.notes, "active": r.active
    })
}

struct CleanRoom {
    code: String,
    category_id: i64,
    max_capacity: i64,
    description: String,
    notes: String,
}

fn clean_room(code: &str, category_id: i64, max_capacity: i64, description: &str, notes: &str) -> Result<CleanRoom, AppError> {
    if !(1..=MAX_CAPACITY).contains(&max_capacity) {
        return Err(invalid(format!("Capacidade máxima: informe de 1 a {MAX_CAPACITY} pessoas.")));
    }
    Ok(CleanRoom {
        code: text(code, 20, "o código do quarto", true)?,
        category_id,
        max_capacity,
        description: text(description, 255, "Descrição", false)?,
        notes: text(notes, 1000, "Observações", false)?,
    })
}

#[derive(Clone)]
pub struct CatalogService {
    ctx: Ctx,
}

impl CatalogService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    fn read(&self, token: &str) -> Result<SessionInfo, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())
    }

    fn write(&self, token: &str) -> Result<SessionInfo, AppError> {
        let info = self.read(token)?;
        require(&info, Permission::ManageCatalog)?;
        Ok(info)
    }

    #[allow(clippy::too_many_arguments)] // cada chamada descreve um evento de auditoria completo
    fn audit(
        &self,
        tx: &rusqlite::Transaction,
        actor: &SessionInfo,
        operation: &'static str,
        entity: &'static str,
        entity_id: i64,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), AppError> {
        audit::record(
            tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "catalog",
                operation,
                entity,
                entity_id: Some(entity_id),
                before,
                after,
                justification: None,
            },
        )
    }

    // ---- categorias ----

    pub fn list_categories(&self, token: &str, active_only: bool) -> Result<Vec<Category>, AppError> {
        self.read(token)?;
        let conn = self.ctx.conn()?;
        catalog_repo::list_categories(&conn, active_only)
    }

    pub fn create_category(&self, token: &str, input: NewCategory) -> Result<i64, AppError> {
        let actor = self.write(token)?;
        let name = text(&input.name, 80, "o nome da categoria", true)?;
        let description = text(&input.description, 255, "Descrição", false)?;
        let mut conn = self.ctx.conn()?;
        let tx = conn.transaction()?;
        let id = catalog_repo::insert_category(&tx, &name, &description, self.ctx.utc.now_utc())
            .map_err(|e| friendly(e, "Já existe uma categoria com este nome.", "Categoria em uso."))?;
        self.audit(&tx, &actor, "create_category", "room_category", id, None,
            Some(json!({ "name": name, "description": description })))?;
        tx.commit()?;
        Ok(id)
    }

    pub fn update_category(&self, token: &str, id: i64, input: UpdateCategory, expected_version: i64) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let name = text(&input.name, 80, "o nome da categoria", true)?;
        let description = text(&input.description, 255, "Descrição", false)?;
        let mut conn = self.ctx.conn()?;
        let before = catalog_repo::find_category(&conn, id)?.ok_or_else(|| invalid("Categoria não encontrada."))?;
        let tx = conn.transaction()?;
        catalog_repo::update_category(&tx, id, &name, &description, expected_version, self.ctx.utc.now_utc())
            .map_err(|e| friendly(e, "Já existe uma categoria com este nome.", "Categoria em uso."))?;
        self.audit(&tx, &actor, "update_category", "room_category", id, Some(category_json(&before)),
            Some(json!({ "name": name, "description": description, "active": before.active })))?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_category_active(&self, token: &str, id: i64, active: bool) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let mut conn = self.ctx.conn()?;
        let before = catalog_repo::find_category(&conn, id)?.ok_or_else(|| invalid("Categoria não encontrada."))?;
        if !active {
            let rooms = catalog_repo::count_active_rooms_in_category(&conn, id)?;
            if rooms > 0 {
                return Err(invalid(format!(
                    "Há {rooms} quarto(s) ativo(s) nesta categoria. Desative-os ou mude a categoria antes."
                )));
            }
        }
        let tx = conn.transaction()?;
        catalog_repo::set_category_active(&tx, id, active, self.ctx.utc.now_utc())?;
        self.audit(&tx, &actor, "set_category_active", "room_category", id,
            Some(json!({ "active": before.active })), Some(json!({ "active": active })))?;
        tx.commit()?;
        Ok(())
    }

    /// Exclusão física só sem histórico: com quartos ligados, o banco recusa e a pessoa é orientada a desativar.
    pub fn delete_category(&self, token: &str, id: i64) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let mut conn = self.ctx.conn()?;
        let before = catalog_repo::find_category(&conn, id)?.ok_or_else(|| invalid("Categoria não encontrada."))?;
        let tx = conn.transaction()?;
        catalog_repo::delete_category(&tx, id).map_err(|e| {
            friendly(e, "", "Esta categoria tem quartos ou histórico e não pode ser apagada. Desative-a em vez de apagar.")
        })?;
        self.audit(&tx, &actor, "delete_category", "room_category", id, Some(category_json(&before)), None)?;
        tx.commit()?;
        Ok(())
    }

    // ---- quartos ----

    pub fn list_rooms(&self, token: &str, query: RoomQuery) -> Result<Page<Room>, AppError> {
        self.read(token)?;
        let (page, page_size, offset) = clamp_paging(query.page, query.page_size);
        let conn = self.ctx.conn()?;
        let (items, total) = catalog_repo::list_rooms(&conn, &query, i64::from(page_size), offset)?;
        Ok(Page { items, total, page, page_size })
    }

    fn require_active_category(conn: &rusqlite::Connection, category_id: i64) -> Result<(), AppError> {
        match catalog_repo::find_category(conn, category_id)? {
            Some(c) if c.active => Ok(()),
            Some(_) => Err(invalid("A categoria escolhida está desativada.")),
            None => Err(invalid("Escolha a categoria do quarto.")),
        }
    }

    pub fn create_room(&self, token: &str, input: NewRoom) -> Result<i64, AppError> {
        let actor = self.write(token)?;
        let r = clean_room(&input.code, input.category_id, input.max_capacity, &input.description, &input.notes)?;
        let mut conn = self.ctx.conn()?;
        Self::require_active_category(&conn, r.category_id)?;
        let tx = conn.transaction()?;
        let id = catalog_repo::insert_room(
            &tx,
            &RoomFields { code: &r.code, category_id: r.category_id, max_capacity: r.max_capacity, description: &r.description, notes: &r.notes },
            self.ctx.utc.now_utc(),
        )
        .map_err(|e| friendly(e, "Já existe um quarto com este código.", "Categoria inválida."))?;
        let created = catalog_repo::find_room(&tx, id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        self.audit(&tx, &actor, "create_room", "room", id, None, Some(room_json(&created)))?;
        tx.commit()?;
        Ok(id)
    }

    pub fn update_room(&self, token: &str, id: i64, input: UpdateRoom, expected_version: i64) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let r = clean_room(&input.code, input.category_id, input.max_capacity, &input.description, &input.notes)?;
        let mut conn = self.ctx.conn()?;
        let before = catalog_repo::find_room(&conn, id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        if before.category_id != r.category_id {
            Self::require_active_category(&conn, r.category_id)?;
        }
        let tx = conn.transaction()?;
        catalog_repo::update_room(
            &tx,
            id,
            &RoomFields { code: &r.code, category_id: r.category_id, max_capacity: r.max_capacity, description: &r.description, notes: &r.notes },
            expected_version,
            self.ctx.utc.now_utc(),
        )
        .map_err(|e| friendly(e, "Já existe um quarto com este código.", "Categoria inválida."))?;
        let after = catalog_repo::find_room(&tx, id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        self.audit(&tx, &actor, "update_room", "room", id, Some(room_json(&before)), Some(room_json(&after)))?;
        tx.commit()?;
        Ok(())
    }

    /// Intervalo mínimo de limpeza do quarto (0 a 1440 minutos); `None` volta ao padrão das configurações.
    pub fn set_room_cleaning_interval(
        &self,
        token: &str,
        id: i64,
        minutes: Option<u32>,
        expected_version: i64,
    ) -> Result<(), AppError> {
        let actor = self.write(token)?;
        if minutes.is_some_and(|m| m > 1440) {
            return Err(invalid("Intervalo de limpeza: informe de 0 a 1440 minutos."));
        }
        let mut conn = self.ctx.conn()?;
        let before = catalog_repo::find_room(&conn, id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        let tx = conn.transaction()?;
        catalog_repo::set_room_cleaning_interval(&tx, id, minutes.map(i64::from), expected_version, self.ctx.utc.now_utc())?;
        self.audit(
            &tx,
            &actor,
            "set_room_cleaning_interval",
            "room",
            id,
            Some(json!({ "cleaning_interval_minutes": before.cleaning_interval_minutes })),
            Some(json!({ "cleaning_interval_minutes": minutes })),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_room_active(&self, token: &str, id: i64, active: bool) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let mut conn = self.ctx.conn()?;
        let before = catalog_repo::find_room(&conn, id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        if active {
            Self::require_active_category(&conn, before.category_id)?;
        }
        let tx = conn.transaction()?;
        catalog_repo::set_room_active(&tx, id, active, self.ctx.utc.now_utc())?;
        self.audit(&tx, &actor, "set_room_active", "room", id,
            Some(json!({ "active": before.active })), Some(json!({ "active": active })))?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_room(&self, token: &str, id: i64) -> Result<(), AppError> {
        let actor = self.write(token)?;
        let mut conn = self.ctx.conn()?;
        let before = catalog_repo::find_room(&conn, id)?.ok_or_else(|| invalid("Quarto não encontrado."))?;
        let tx = conn.transaction()?;
        catalog_repo::delete_room(&tx, id).map_err(|e| {
            friendly(e, "", "Este quarto tem histórico e não pode ser apagado. Desative-o em vez de apagar.")
        })?;
        self.audit(&tx, &actor, "delete_room", "room", id, Some(room_json(&before)), None)?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::models::catalog::{NewCategory, NewRoom, RoomQuery};
    use crate::services::test_support::{admin_token, harness, recepcao_token, Harness};

    fn category(name: &str) -> NewCategory {
        NewCategory { name: name.into(), description: "Quarto simples".into() }
    }

    fn room(code: &str, category_id: i64) -> NewRoom {
        NewRoom {
            code: code.into(),
            category_id,
            max_capacity: 2,
            description: "Vista para o jardim".into(),
            notes: String::new(),
        }
    }

    fn query() -> RoomQuery {
        RoomQuery { text: None, category_id: None, active: None, page: 1, page_size: 20, sort: None, direction: None }
    }

    fn setup() -> (Harness, String, i64) {
        let h = harness();
        let admin = admin_token(&h);
        let cat = h.catalog.create_category(&admin, category("Standard")).unwrap();
        (h, admin, cat)
    }

    fn audit_count(h: &Harness, operation: &str) -> i64 {
        h.db.lock()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_log WHERE operation = ?1", [operation], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn names_and_codes_are_unique_ignoring_case() {
        let (h, admin, cat) = setup();
        h.catalog.create_room(&admin, room("A1", cat)).unwrap();

        assert!(matches!(h.catalog.create_category(&admin, category("standard")), Err(AppError::Validation(_))));
        assert!(matches!(h.catalog.create_room(&admin, room("a1", cat)), Err(AppError::Validation(_))));
    }

    #[test]
    fn rejects_empty_names_and_invalid_capacity() {
        let (h, admin, cat) = setup();

        assert!(matches!(h.catalog.create_category(&admin, category("   ")), Err(AppError::Validation(_))));
        assert!(matches!(h.catalog.create_room(&admin, room("  ", cat)), Err(AppError::Validation(_))));
        for capacity in [0, 51] {
            let mut r = room("B1", cat);
            r.max_capacity = capacity;
            assert!(matches!(h.catalog.create_room(&admin, r), Err(AppError::Validation(_))));
        }
    }

    #[test]
    fn recepcao_reads_but_cannot_write() {
        let (h, admin, cat) = setup();
        let room_id = h.catalog.create_room(&admin, room("A1", cat)).unwrap();
        let rec = recepcao_token(&h, &admin);

        assert!(h.catalog.list_categories(&rec, false).is_ok());
        assert!(h.catalog.list_rooms(&rec, query()).is_ok());
        assert!(matches!(h.catalog.create_category(&rec, category("Luxo")), Err(AppError::Forbidden)));
        assert!(matches!(h.catalog.create_room(&rec, room("C1", cat)), Err(AppError::Forbidden)));
        assert!(matches!(h.catalog.set_room_active(&rec, room_id, false), Err(AppError::Forbidden)));
        assert!(matches!(h.catalog.delete_room(&rec, room_id), Err(AppError::Forbidden)));
        assert!(matches!(h.catalog.set_category_active(&rec, cat, false), Err(AppError::Forbidden)));
        assert!(matches!(h.catalog.delete_category(&rec, cat), Err(AppError::Forbidden)));
    }

    #[test]
    fn updates_conflict_on_stale_version() {
        let (h, admin, cat) = setup();
        let room_id = h.catalog.create_room(&admin, room("A1", cat)).unwrap();

        h.catalog.update_room(&admin, room_id, room("A1", cat).into(), 1).unwrap();
        assert!(matches!(
            h.catalog.update_room(&admin, room_id, room("A1", cat).into(), 1),
            Err(AppError::Conflict)
        ));
        h.catalog.update_category(&admin, cat, category("Standard").into(), 1).unwrap();
        assert!(matches!(
            h.catalog.update_category(&admin, cat, category("Standard").into(), 1),
            Err(AppError::Conflict)
        ));
    }

    #[test]
    fn category_with_active_rooms_cannot_be_deactivated() {
        let (h, admin, cat) = setup();
        let room_id = h.catalog.create_room(&admin, room("A1", cat)).unwrap();

        assert!(matches!(h.catalog.set_category_active(&admin, cat, false), Err(AppError::Validation(_))));
        h.catalog.set_room_active(&admin, room_id, false).unwrap();
        h.catalog.set_category_active(&admin, cat, false).unwrap();

        assert!(matches!(h.catalog.create_room(&admin, room("A2", cat)), Err(AppError::Validation(_))));
        assert!(h.catalog.list_categories(&admin, true).unwrap().is_empty());
        assert_eq!(h.catalog.list_categories(&admin, false).unwrap().len(), 1);
    }

    #[test]
    fn delete_fails_with_history_and_keeps_the_row() {
        let (h, admin, cat) = setup();
        let room_id = h.catalog.create_room(&admin, room("A1", cat)).unwrap();
        {
            let conn = h.db.lock().unwrap();
            conn.execute_batch(
                "CREATE TABLE room_refs_test (id INTEGER PRIMARY KEY, room_id INTEGER REFERENCES rooms(id));",
            )
            .unwrap();
            conn.execute("INSERT INTO room_refs_test (room_id) VALUES (?1)", [room_id]).unwrap();
        }

        assert!(matches!(h.catalog.delete_room(&admin, room_id), Err(AppError::Validation(_))));
        assert!(matches!(h.catalog.delete_category(&admin, cat), Err(AppError::Validation(_))));
        assert_eq!(h.catalog.list_rooms(&admin, query()).unwrap().total, 1);
    }

    #[test]
    fn unused_records_can_be_deleted_and_it_is_audited() {
        let (h, admin, cat) = setup();
        let room_id = h.catalog.create_room(&admin, room("A1", cat)).unwrap();

        h.catalog.delete_room(&admin, room_id).unwrap();
        h.catalog.delete_category(&admin, cat).unwrap();

        assert_eq!(h.catalog.list_rooms(&admin, query()).unwrap().total, 0);
        assert_eq!(audit_count(&h, "delete_room"), 1);
        assert_eq!(audit_count(&h, "delete_category"), 1);
    }

    #[test]
    fn list_rooms_filters_paginates_and_shows_category_name() {
        let (h, admin, cat) = setup();
        let luxo = h.catalog.create_category(&admin, category("Luxo")).unwrap();
        for (code, c) in [("101", cat), ("102", cat), ("201", luxo)] {
            h.catalog.create_room(&admin, room(code, c)).unwrap();
        }
        let inactive = h.catalog.create_room(&admin, room("999", cat)).unwrap();
        h.catalog.set_room_active(&admin, inactive, false).unwrap();

        let mut q = query();
        q.category_id = Some(luxo);
        let only_luxo = h.catalog.list_rooms(&admin, q).unwrap();
        assert_eq!((only_luxo.total, only_luxo.items[0].category_name.as_str()), (1, "Luxo"));

        let mut q = query();
        q.text = Some("10".into());
        q.active = Some(true);
        assert_eq!(h.catalog.list_rooms(&admin, q).unwrap().total, 2);

        let mut q = query();
        q.page_size = 2;
        q.page = 2;
        let page2 = h.catalog.list_rooms(&admin, q).unwrap();
        assert_eq!((page2.total, page2.items.len()), (4, 2));
    }

    #[test]
    fn room_interval_needs_manage_catalog_and_accepts_0_to_1440() {
        let (h, admin, cat) = setup();
        let rec = recepcao_token(&h, &admin);
        let room_id = h.catalog.create_room(&admin, room("A1", cat)).unwrap();

        assert!(matches!(h.catalog.set_room_cleaning_interval(&rec, room_id, Some(30), 1), Err(AppError::Forbidden)));
        assert!(matches!(h.catalog.set_room_cleaning_interval(&admin, room_id, Some(1441), 1), Err(AppError::Validation(_))));
        assert!(matches!(h.catalog.set_room_cleaning_interval(&admin, room_id, Some(30), 9), Err(AppError::Conflict)));
        assert!(matches!(h.catalog.set_room_cleaning_interval(&admin, 999, Some(30), 1), Err(AppError::Validation(_))));

        h.catalog.set_room_cleaning_interval(&admin, room_id, Some(0), 1).unwrap();
        assert_eq!(h.catalog.list_rooms(&admin, query()).unwrap().items[0].cleaning_interval_minutes, Some(0));
        h.catalog.set_room_cleaning_interval(&admin, room_id, None, 2).unwrap();
        assert_eq!(h.catalog.list_rooms(&admin, query()).unwrap().items[0].cleaning_interval_minutes, None);
    }

    #[test]
    fn writes_are_audited_with_before_and_after() {
        let (h, admin, cat) = setup();
        let room_id = h.catalog.create_room(&admin, room("A1", cat)).unwrap();
        let mut changed = room("A1", cat);
        changed.max_capacity = 4;
        h.catalog.update_room(&admin, room_id, changed.into(), 1).unwrap();
        h.catalog.set_room_active(&admin, room_id, false).unwrap();

        let conn = h.db.lock().unwrap();
        let (before, after): (String, String) = conn
            .query_row(
                "SELECT before_json, after_json FROM audit_log WHERE operation = 'update_room'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(before.contains("\"max_capacity\":2") && after.contains("\"max_capacity\":4"));
        drop(conn);
        assert_eq!(audit_count(&h, "create_room"), 1);
        assert_eq!(audit_count(&h, "set_room_active"), 1);
        assert_eq!(audit_count(&h, "create_category"), 1);
    }
}
