use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Recepcao,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Recepcao => "recepcao",
        }
    }

    pub fn from_db(value: &str) -> Option<Role> {
        match value {
            "admin" => Some(Role::Admin),
            "recepcao" => Some(Role::Recepcao),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: Role,
    pub active: bool,
    pub must_change_password: bool,
    pub failed_attempts: u32,
    pub locked_until: Option<String>,
    pub version: i64,
}

pub struct NewUser {
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub role: Role,
    pub must_change_password: bool,
}
