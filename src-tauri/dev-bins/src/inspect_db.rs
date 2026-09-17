// ./src-tauri/dev-bins/src/inspect_db.rs
use std::path::PathBuf;

fn main() {
    println!("=== Genomics Caddy Database Inspection ===");

    tauri_app_lib::config::init_env();

    let info = tauri_app_lib::paths::resolve_data_dir_info();
    let data_dir = info.path;
    let db_path = tauri_app_lib::paths::db_path(&data_dir);

    println!("Resolved Data Dir: {}", data_dir.display());
    println!("Resolved DB Path: {}", db_path.display());

    let sealed = PathBuf::from(format!("{}.enc", db_path.to_string_lossy()));
    println!("Legacy sealed path exists: {}", sealed.exists());
    tauri_app_lib::db_crypto::discard_legacy_sealed_db(&db_path);

    {
        println!("Opening database...");
        let conn = match tauri_app_lib::db::open_user_db(&db_path) {
            Ok(c) => c,
            Err(e) => {
                println!("Failed to open DB: {e}");
                return;
            }
        };

        println!("Database opened successfully.");

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        println!("Tables: {tables:?}");

        match conn.query_row("SELECT COUNT(*) FROM samples", [], |r| r.get::<_, i64>(0)) {
            Ok(n) => println!("Sample count: {n}"),
            Err(e) => println!("Could not count samples: {e}"),
        }

        match conn.query_row("SELECT COUNT(*) FROM genotypes", [], |r| r.get::<_, i64>(0)) {
            Ok(n) => println!("Genotype rows: {n}"),
            Err(e) => println!("Could not count genotypes: {e}"),
        }
    }
}
