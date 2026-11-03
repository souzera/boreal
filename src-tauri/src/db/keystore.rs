use std::sync::Mutex;

use crate::error::AppError;

/// Guarda a chave de criptografia do banco fora dos arquivos do aplicativo.
pub trait KeyStore: Send + Sync {
    fn get(&self) -> Result<Option<Vec<u8>>, AppError>;
    fn set(&self, key: &[u8]) -> Result<(), AppError>;
}

/// Cofre de credenciais do sistema operacional.
pub struct OsKeyStore {
    service: String,
    user: String,
}

impl OsKeyStore {
    pub fn new() -> Self {
        Self {
            service: "br.com.boreal.pms".into(),
            user: "database-key".into(),
        }
    }

    fn entry(&self) -> Result<keyring::Entry, AppError> {
        keyring::Entry::new(&self.service, &self.user).map_err(|_| AppError::KeyUnavailable)
    }
}

impl Default for OsKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyStore for OsKeyStore {
    fn get(&self) -> Result<Option<Vec<u8>>, AppError> {
        match self.entry()?.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(AppError::KeyUnavailable),
        }
    }

    fn set(&self, key: &[u8]) -> Result<(), AppError> {
        self.entry()?
            .set_secret(key)
            .map_err(|_| AppError::KeyUnavailable)
    }
}

/// Cofre em memória, para testes.
#[derive(Default)]
pub struct MemoryKeyStore(Mutex<Option<Vec<u8>>>);

impl MemoryKeyStore {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_key(key: &[u8]) -> Self {
        Self(Mutex::new(Some(key.to_vec())))
    }
}

impl KeyStore for MemoryKeyStore {
    fn get(&self) -> Result<Option<Vec<u8>>, AppError> {
        Ok(self.0.lock().unwrap().clone())
    }

    fn set(&self, key: &[u8]) -> Result<(), AppError> {
        *self.0.lock().unwrap() = Some(key.to_vec());
        Ok(())
    }
}
