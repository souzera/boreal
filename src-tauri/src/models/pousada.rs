use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct PousadaView {
    pub name: String,
    pub address: String,
    pub phone: String,
    pub has_logo: bool,
    pub version: i64,
}

#[derive(Debug, Serialize)]
pub struct LogoData {
    pub mime: String,
    pub base64: String,
}

/// O conteúdo do logotipo nunca aparece em `Debug`.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum LogoChange {
    Keep,
    Remove,
    Set { mime: String, base64: String },
}

impl std::fmt::Debug for LogoChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogoChange::Keep => f.write_str("Keep"),
            LogoChange::Remove => f.write_str("Remove"),
            LogoChange::Set { mime, .. } => f
                .debug_struct("Set")
                .field("mime", mime)
                .field("base64", &"[oculto]")
                .finish(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePousada {
    pub name: String,
    pub address: String,
    pub phone: String,
    pub logo: LogoChange,
}
