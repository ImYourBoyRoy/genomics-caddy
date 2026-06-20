// ./src-tauri/src/db_crypto.rs
/*
Purpose: Encrypt the genome SQLite file at rest when the app is not running.
Responsibilities:
- Decrypt `user_genome.db.enc` to a working `user_genome.db` on open.
- Seal (encrypt + remove plaintext) on graceful app exit.
Key Inputs: Database path, 256-bit key from OS keyring.
Operational Notes: While the app is running the working DB is plaintext (standard SQLite). At rest only the `.enc` blob remains.
*/

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use std::fs;
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 5] = b"GCDB1";
const NONCE_LEN: usize = 12;

fn sealed_path(db_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.enc", db_path.to_string_lossy()))
}

fn load_key() -> Result<[u8; 32], String> {
    let hex = crate::config::get_or_create_db_encryption_key()?;
    let bytes = hex::decode(hex).map_err(|e| format!("Invalid DB encryption key: {e}"))?;
    if bytes.len() != 32 {
        return Err("DB encryption key must be 32 bytes".into());
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn encrypt_bytes(plain: &[u8]) -> Result<Vec<u8>, String> {
    let key = load_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce_bytes).map_err(|e| format!("Nonce generation failed: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted = cipher
        .encrypt(nonce, plain)
        .map_err(|e| format!("DB encryption failed: {e}"))?;
    let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + encrypted.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&encrypted);
    Ok(out)
}

fn decrypt_bytes(blob: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < MAGIC.len() + NONCE_LEN + 16 {
        return Err("Encrypted DB blob is too short".into());
    }
    if &blob[..MAGIC.len()] != MAGIC {
        return Err("Encrypted DB blob has invalid header".into());
    }
    let key = load_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(&blob[MAGIC.len()..MAGIC.len() + NONCE_LEN]);
    let encrypted = &blob[MAGIC.len() + NONCE_LEN..];
    cipher
        .decrypt(nonce, encrypted)
        .map_err(|e| format!("DB decryption failed: {e}"))
}

/// Ensure a working plaintext SQLite file exists (decrypt sealed copy if needed).
pub fn ensure_decrypted(db_path: &Path) -> Result<(), String> {
    if db_path.is_file() {
        return Ok(());
    }
    let sealed = sealed_path(db_path);
    if !sealed.is_file() {
        return Ok(());
    }
    let blob = fs::read(&sealed).map_err(|e| format!("Failed to read sealed DB: {e}"))?;
    let plain = decrypt_bytes(&blob)?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create DB directory: {e}"))?;
    }
    fs::write(db_path, plain).map_err(|e| format!("Failed to write decrypted DB: {e}"))?;
    Ok(())
}

/// Encrypt the working DB and remove plaintext sidecars (WAL/SHM).
pub fn seal_encrypted(db_path: &Path) -> Result<(), String> {
    if !db_path.is_file() {
        return Ok(());
    }
    let plain = fs::read(db_path).map_err(|e| format!("Failed to read DB for sealing: {e}"))?;
    let sealed = encrypt_bytes(&plain)?;
    fs::write(sealed_path(db_path), sealed).map_err(|e| format!("Failed to write sealed DB: {e}"))?;
    let _ = fs::remove_file(db_path);
    let _ = fs::remove_file(format!("{}-wal", db_path.to_string_lossy()));
    let _ = fs::remove_file(format!("{}-shm", db_path.to_string_lossy()));
    Ok(())
}

/// Open hook used by `db::connect` — decrypt sealed file when needed.
pub fn open_encrypted(path: &Path) -> Result<rusqlite::Connection, rusqlite::Error> {
    ensure_decrypted(path).map_err(rusqlite::Error::InvalidParameterName)?;
    rusqlite::Connection::open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn seal_and_decrypt_roundtrip() {
        let dir = std::env::temp_dir().join(format!("db_seal_test_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("dir");
        let db_path = dir.join("user_genome.db");
        fs::write(&db_path, b"sqlite-bytes-test").expect("write");

        seal_encrypted(&db_path).expect("seal");
        assert!(!db_path.exists());
        assert!(sealed_path(&db_path).exists());

        ensure_decrypted(&db_path).expect("decrypt");
        let content = fs::read(&db_path).expect("read back");
        assert_eq!(content, b"sqlite-bytes-test");

        let _ = fs::remove_dir_all(&dir);
    }
}
