use base64::Engine;
use serde_json::json;

use super::Ctx;
use crate::audit::{self, AuditEntry};
use crate::auth::permissions::Permission;
use crate::auth::session::require;
use crate::error::AppError;
pub use crate::models::pousada::{LogoChange, LogoData, PousadaView, UpdatePousada};
use crate::repositories::pousada_repo::{self, LogoWrite};

const MAX_LOGO_BYTES: usize = 512 * 1024;
const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const JPEG_SIGNATURE: &[u8] = &[0xFF, 0xD8, 0xFF];

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

/// Aceita só PNG/JPEG cuja assinatura de arquivo confere com o tipo informado e que caibam no limite.
fn decode_logo(mime: &str, encoded: &str) -> Result<Vec<u8>, AppError> {
    let signature = match mime {
        "image/png" => PNG_SIGNATURE,
        "image/jpeg" => JPEG_SIGNATURE,
        _ => return Err(invalid("O logotipo deve ser uma imagem PNG ou JPEG.")),
    };
    if encoded.len() > MAX_LOGO_BYTES * 2 {
        return Err(invalid("O logotipo deve ter no máximo 512 KB."));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| invalid("Não foi possível ler o arquivo do logotipo."))?;
    if bytes.len() > MAX_LOGO_BYTES {
        return Err(invalid("O logotipo deve ter no máximo 512 KB."));
    }
    if !bytes.starts_with(signature) {
        return Err(invalid("O arquivo não é uma imagem PNG ou JPEG válida."));
    }
    Ok(bytes)
}

fn clean(value: &str, max: usize, label: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.chars().count() > max {
        return Err(AppError::Validation(format!("{label}: no máximo {max} caracteres.")));
    }
    Ok(value.to_string())
}

#[derive(Clone)]
pub struct PousadaService {
    ctx: Ctx,
}

impl PousadaService {
    pub fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    pub fn get(&self, token: &str) -> Result<PousadaView, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.db.lock().unwrap();
        pousada_repo::get(&conn)
    }

    pub fn get_logo(&self, token: &str) -> Result<Option<LogoData>, AppError> {
        self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        let conn = self.ctx.db.lock().unwrap();
        Ok(pousada_repo::get_logo(&conn)?.map(|(mime, bytes)| LogoData {
            mime,
            base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        }))
    }

    /// Atualiza os dados da pousada (exige `ManageSettings`) e devolve a nova versão.
    pub fn update(
        &self,
        token: &str,
        input: UpdatePousada,
        expected_version: i64,
    ) -> Result<i64, AppError> {
        let actor = self.ctx.sessions.touch(token, self.ctx.clock.now())?;
        require(&actor, Permission::ManageSettings)?;
        let name = clean(&input.name, 120, "Nome")?;
        if name.is_empty() {
            return Err(invalid("Informe o nome da pousada."));
        }
        let address = clean(&input.address, 255, "Endereço")?;
        let phone = clean(&input.phone, 30, "Telefone")?;
        let decoded = match &input.logo {
            LogoChange::Set { mime, base64 } => Some((mime.as_str(), decode_logo(mime, base64)?)),
            _ => None,
        };
        let logo = match (&input.logo, &decoded) {
            (LogoChange::Keep, _) => LogoWrite::Keep,
            (LogoChange::Remove, _) => LogoWrite::Remove,
            (LogoChange::Set { .. }, Some((mime, bytes))) => LogoWrite::Set { mime, bytes },
            (LogoChange::Set { .. }, None) => unreachable!("logo decodificado acima"),
        };

        let mut conn = self.ctx.db.lock().unwrap();
        let before = pousada_repo::get(&conn)?;
        let tx = conn.transaction()?;
        let version = pousada_repo::update(
            &tx,
            &name,
            &address,
            &phone,
            logo,
            expected_version,
            self.ctx.utc.now_utc(),
        )?;
        let logo_state = match &input.logo {
            LogoChange::Keep => "mantido",
            LogoChange::Remove => "removido",
            LogoChange::Set { .. } => "substituído",
        };
        audit::record(
            &tx,
            &AuditEntry {
                at: self.ctx.utc.now_utc(),
                user_id: Some(actor.user_id),
                module: "settings",
                operation: "update_pousada",
                entity: "pousada",
                entity_id: Some(1),
                before: Some(json!({ "name": before.name, "address": before.address, "phone": before.phone, "has_logo": before.has_logo })),
                after: Some(json!({ "name": name, "address": address, "phone": phone, "logo": logo_state })),
                justification: None,
            },
        )?;
        tx.commit()?;
        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::{LogoChange, UpdatePousada};
    use crate::error::AppError;
    use crate::services::test_support::{admin_token, harness, recepcao_token, Harness};
    use base64::Engine;

    const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    const JPEG_SIGNATURE: [u8; 3] = [0xFF, 0xD8, 0xFF];

    fn b64(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn png(extra: usize) -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend(std::iter::repeat_n(0u8, extra));
        bytes
    }

    fn input(name: &str, logo: LogoChange) -> UpdatePousada {
        UpdatePousada {
            name: name.into(),
            address: "Rua das Flores, 100".into(),
            phone: "(11) 3333-4444".into(),
            logo,
        }
    }

    fn save(h: &Harness, token: &str, data: UpdatePousada, version: i64) -> Result<i64, AppError> {
        h.pousada.update(token, data, version)
    }

    #[test]
    fn default_is_empty_and_readable_by_recepcao() {
        let h = harness();
        let admin = admin_token(&h);
        let rec = recepcao_token(&h, &admin);

        let view = h.pousada.get(&rec).unwrap();

        assert_eq!(view.name, "");
        assert!(!view.has_logo);
        assert_eq!(view.version, 1);
        assert!(h.pousada.get_logo(&rec).unwrap().is_none());
    }

    #[test]
    fn recepcao_cannot_update() {
        let h = harness();
        let admin = admin_token(&h);
        let rec = recepcao_token(&h, &admin);

        assert!(matches!(
            save(&h, &rec, input("Pousada Boreal", LogoChange::Keep), 1),
            Err(AppError::Forbidden)
        ));
    }

    #[test]
    fn update_roundtrip_bumps_version_and_is_audited() {
        let h = harness();
        let admin = admin_token(&h);

        let version = save(&h, &admin, input("  Pousada Boreal  ", LogoChange::Keep), 1).unwrap();

        assert_eq!(version, 2);
        let view = h.pousada.get(&admin).unwrap();
        assert_eq!(view.name, "Pousada Boreal");
        assert_eq!(view.address, "Rua das Flores, 100");
        assert_eq!(view.phone, "(11) 3333-4444");
        let audited: i64 = h
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM audit_log WHERE operation = 'update_pousada'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audited, 1);
    }

    #[test]
    fn name_is_required_and_version_must_match() {
        let h = harness();
        let admin = admin_token(&h);

        assert!(matches!(
            save(&h, &admin, input("   ", LogoChange::Keep), 1),
            Err(AppError::Validation(_))
        ));
        save(&h, &admin, input("Boreal", LogoChange::Keep), 1).unwrap();
        assert!(matches!(
            save(&h, &admin, input("Boreal 2", LogoChange::Keep), 1),
            Err(AppError::Conflict)
        ));
    }

    #[test]
    fn accepts_png_and_roundtrips_then_removes() {
        let h = harness();
        let admin = admin_token(&h);
        let bytes = png(100);

        let v = save(
            &h,
            &admin,
            input("Boreal", LogoChange::Set { mime: "image/png".into(), base64: b64(&bytes) }),
            1,
        )
        .unwrap();

        assert!(h.pousada.get(&admin).unwrap().has_logo);
        let logo = h.pousada.get_logo(&admin).unwrap().unwrap();
        assert_eq!(logo.mime, "image/png");
        assert_eq!(logo.base64, b64(&bytes));

        save(&h, &admin, input("Boreal", LogoChange::Keep), v).unwrap();
        assert!(h.pousada.get(&admin).unwrap().has_logo);
        let v = h.pousada.get(&admin).unwrap().version;
        save(&h, &admin, input("Boreal", LogoChange::Remove), v).unwrap();
        assert!(!h.pousada.get(&admin).unwrap().has_logo);
    }

    #[test]
    fn rejects_hostile_logos() {
        let h = harness();
        let admin = admin_token(&h);
        let set = |mime: &str, bytes: &[u8]| LogoChange::Set {
            mime: mime.into(),
            base64: b64(bytes),
        };

        let cases = vec![
            set("image/png", b"isto nao e uma imagem"),
            set("image/png", &[&JPEG_SIGNATURE[..], &[0u8; 10][..]].concat()),
            set("image/svg+xml", b"<svg/>"),
            set("image/png", &png(600 * 1024)),
            LogoChange::Set { mime: "image/png".into(), base64: "%%%não-é-base64%%%".into() },
        ];
        for logo in cases {
            assert!(
                matches!(save(&h, &admin, input("Boreal", logo), 1), Err(AppError::Validation(_))),
                "logo hostil aceito"
            );
        }
        assert!(!h.pousada.get(&admin).unwrap().has_logo);
    }

    #[test]
    fn debug_never_prints_logo_content() {
        let data = input("Boreal", LogoChange::Set { mime: "image/png".into(), base64: b64(&png(50)) });

        assert!(!format!("{data:?}").contains(&b64(&png(50))));
    }
}
