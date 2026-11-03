pub mod keystore;

use std::path::Path;

use rusqlite::Connection;

use crate::error::AppError;
use keystore::KeyStore;

const KEY_LEN: usize = 32;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Abre (ou cria) o banco criptografado.
///
/// Banco novo: gera uma chave aleatória e a guarda no `KeyStore`.
/// Banco existente sem chave disponível: falha com `KeyUnavailable`, sem tocar no arquivo.
pub fn open_database(path: &Path, keys: &dyn KeyStore) -> Result<Connection, AppError> {
    let key = match keys.get()? {
        Some(key) => key,
        None if path.exists() => return Err(AppError::KeyUnavailable),
        None => {
            let mut key = vec![0u8; KEY_LEN];
            rand::fill(&mut key[..]);
            keys.set(&key)?;
            key
        }
    };

    let conn = Connection::open(path)?;
    conn.pragma_update(None, "key", format!("x'{}'", hex(&key)))?;
    // Falha aqui se a chave estiver errada ou o arquivo não for um banco cifrado.
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))
        .map_err(|_| AppError::KeyInvalid)?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::keystore::{KeyStore, MemoryKeyStore};
    use super::open_database;
    use crate::error::AppError;

    const PLAIN_HEADER: &[u8] = b"SQLite format 3\0";

    #[test]
    fn creates_encrypted_db_and_persists_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("boreal.db");
        let keys = MemoryKeyStore::empty();

        let conn = open_database(&path, &keys).unwrap();
        conn.execute_batch("CREATE TABLE t(v TEXT); INSERT INTO t VALUES ('segredo');")
            .unwrap();
        drop(conn);

        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() >= 16);
        assert_ne!(&bytes[..16], PLAIN_HEADER);
        assert!(keys.get().unwrap().is_some());

        let conn = open_database(&path, &keys).unwrap();
        let v: String = conn.query_row("SELECT v FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(v, "segredo");
    }

    #[test]
    fn refuses_existing_db_without_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("boreal.db");
        drop(open_database(&path, &MemoryKeyStore::empty()).unwrap());
        let before = std::fs::read(&path).unwrap();

        let result = open_database(&path, &MemoryKeyStore::empty());

        assert!(matches!(result, Err(AppError::KeyUnavailable)));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn wrong_key_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("boreal.db");
        drop(open_database(&path, &MemoryKeyStore::empty()).unwrap());

        let result = open_database(&path, &MemoryKeyStore::with_key(&[7u8; 32]));

        assert!(matches!(result, Err(AppError::KeyInvalid)));
    }
}
