//! Segredo em memória: zera o conteúdo ao ser solto e nunca aparece em `Debug`.

use zeroize::Zeroize;

pub struct SecretKey(Vec<u8>);

impl SecretKey {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Chave em hexadecimal minúsculo, no formato que o SQLCipher espera (`x'…'`).
    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Zera o conteúdo agora (também acontece ao soltar).
    pub fn wipe(&mut self) {
        self.0.zeroize();
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.wipe();
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKey(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::SecretKey;

    #[test]
    fn debug_never_shows_the_bytes() {
        let key = SecretKey::new(vec![0xAB; 32]);

        let shown = format!("{key:?}");

        assert_eq!(shown, "SecretKey(***)");
        assert!(!shown.to_lowercase().contains("ab"));
    }

    #[test]
    fn wipe_zeroes_the_content() {
        let mut key = SecretKey::new(vec![7; 32]);

        key.wipe();

        assert!(key.as_bytes().iter().all(|b| *b == 0));
    }

    #[test]
    fn hex_is_lowercase_pairs() {
        assert_eq!(SecretKey::new(vec![0x0f, 0xa0]).hex(), "0fa0");
    }
}
