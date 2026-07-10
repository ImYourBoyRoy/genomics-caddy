// ./src-tauri/src/db_crypto.rs
/*
Purpose: Legacy sealed-DB cleanup only (encryption removed).
Responsibilities:
- Discard leftover `user_genome.db.enc` files from older builds.
- Provide a plain SQLite open helper used by `db::connect`.
Operational Notes:
- Genome DBs are plaintext on disk. OS disk encryption (BitLocker/FileVault/LUKS) is the real at-rest layer.
- Source genotype files are also plaintext; app-level AES sealing added cross-OS breakage without meaningful security.
*/

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn sealed_path(db_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.enc", db_path.to_string_lossy()))
}

/// Remove or quarantine any leftover sealed DB from older app versions.
/// Never blocks launch — plaintext `user_genome.db` / per-sample DBs are authoritative.
pub fn discard_legacy_sealed_db(db_path: &Path) {
    let sealed = sealed_path(db_path);
    if !sealed.is_file() {
        return;
    }

    // Prefer delete when plaintext already exists or we intentionally abandoned sealing.
    if db_path.is_file() {
        match fs::remove_file(&sealed) {
            Ok(()) => eprintln!(
                "Removed obsolete sealed DB {} (plaintext DB is authoritative).",
                sealed.display()
            ),
            Err(e) => eprintln!(
                "Could not remove obsolete sealed DB {}: {e}",
                sealed.display()
            ),
        }
        return;
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let orphaned = PathBuf::from(format!("{}.orphaned_{ts}", sealed.to_string_lossy()));
    match fs::rename(&sealed, &orphaned) {
        Ok(()) => eprintln!(
            "Quarantined obsolete sealed DB (encryption retired) as {}.\n\
             Re-import your genotype file for a clean plaintext sample database.",
            orphaned.display()
        ),
        Err(e) => {
            eprintln!(
                "Could not quarantine obsolete sealed DB {}: {e}. Attempting delete.",
                sealed.display()
            );
            let _ = fs::remove_file(&sealed);
        }
    }
}

/// Open a plaintext SQLite database (legacy name kept for call-site stability).
pub fn open_encrypted(path: &Path) -> Result<rusqlite::Connection, rusqlite::Error> {
    discard_legacy_sealed_db(path);
    rusqlite::Connection::open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discards_sealed_when_plaintext_exists() {
        let dir = std::env::temp_dir().join(format!("db_plain_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        let db_path = dir.join("user_genome.db");
        let sealed = sealed_path(&db_path);
        fs::write(&db_path, b"sqlite").expect("plain");
        fs::write(&sealed, b"GCDB1garbage").expect("sealed");
        discard_legacy_sealed_db(&db_path);
        assert!(db_path.exists());
        assert!(!sealed.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn quarantines_sealed_when_no_plaintext() {
        let dir = std::env::temp_dir().join(format!("db_orphan_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        let db_path = dir.join("user_genome.db");
        let sealed = sealed_path(&db_path);
        fs::write(&sealed, b"GCDB1garbage").expect("sealed");
        discard_legacy_sealed_db(&db_path);
        assert!(!db_path.exists());
        assert!(!sealed.exists());
        let orphans: Vec<_> = fs::read_dir(&dir)
            .expect("read")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("orphaned_"))
            .collect();
        assert_eq!(orphans.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
