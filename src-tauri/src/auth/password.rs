use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};

use crate::error::AppError;

const MIN_PASSWORD_CHARS: usize = 6;

/// Gera o hash Argon2id (formato PHC) com salt aleatório. A senha não é normalizada.
pub fn hash_password(plain: &str) -> Result<String, AppError> {
    Argon2::default()
        .hash_password(plain.as_bytes())
        .map(|h| h.to_string())
        .map_err(|_| AppError::Validation("Não foi possível processar a senha.".into()))
}

pub fn verify_password(plain: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Mínimo de 6 caracteres e não pode ser vazia ou só espaços. Sem trim nem normalização.
pub fn validate_password_policy(plain: &str) -> Result<(), AppError> {
    if plain.trim().is_empty() || plain.chars().count() < MIN_PASSWORD_CHARS {
        return Err(AppError::Validation(
            "A senha deve ter pelo menos 6 caracteres.".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{hash_password, validate_password_policy, verify_password};

    #[test]
    fn hash_verify_roundtrip_preserves_spaces_and_unicode() {
        let plain = "  sénha 🔑 ";
        let hash = hash_password(plain).unwrap();

        assert!(verify_password(plain, &hash));
        assert!(!verify_password("sénha 🔑", &hash));
        assert!(!verify_password("outra", &hash));
    }

    #[test]
    fn policy_rejects_short_empty_blank() {
        assert!(validate_password_policy("abc").is_err());
        assert!(validate_password_policy("").is_err());
        assert!(validate_password_policy("        ").is_err());
        assert!(validate_password_policy("12345").is_err());
        assert!(validate_password_policy("123456").is_ok());
        assert!(validate_password_policy("sénha🔑").is_ok());
    }
}
