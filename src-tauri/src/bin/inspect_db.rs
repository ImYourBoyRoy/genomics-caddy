// ./src-tauri/src/bin/inspect_db.rs
use std::path::PathBuf;

fn main() {
    println!("=== Genomics Caddy Database Inspection ===");
    
    // Initialize env variables
    tauri_app_lib::config::init_env();
    
    let info = tauri_app_lib::paths::resolve_data_dir_info();
    let data_dir = info.path;
    let db_path = tauri_app_lib::paths::db_path(&data_dir);
    
    println!("Resolved Data Dir: {}", data_dir.display());
    println!("Resolved DB Path: {}", db_path.display());
    
    let sealed = PathBuf::from(format!("{}.enc", db_path.to_string_lossy()));
    println!("Sealed Path Exists: {}", sealed.exists());
    if sealed.exists()
        && let Ok(meta) = std::fs::metadata(&sealed) {
            #[allow(clippy::cast_precision_loss)]
            let gb_size = meta.len() as f64 / 1_073_741_824.0;
            println!("Sealed File Size: {} bytes ({:.2} GB)", meta.len(), gb_size);
        }
    
    println!("Decrypting database...");
    let start_decrypt = std::time::Instant::now();
    match tauri_app_lib::db_crypto::ensure_decrypted(&db_path) {
        Ok(()) => println!("Successfully decrypted in {:?}", start_decrypt.elapsed()),
        Err(e) => {
            println!("Decryption failed: {e}");
            return;
        }
    }
    
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
        
        // Get tables
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'").unwrap();
        let tables: Vec<String> = stmt.query_map([], |row| row.get(0)).unwrap().map(|r| r.unwrap()).collect();
        
        println!("\nTables:");
        for t in tables {
            let count: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |row| row.get(0)).unwrap_or(-1);
            println!("- {t}: {count} rows");
        }
        
        // Inspect samples
        println!("\nSamples:");
        let mut stmt = conn.prepare("SELECT id, name, genetic_sex FROM samples").unwrap();
        let samples_list: Vec<(i64, String, String)> = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).unwrap().map(|r| r.unwrap()).collect();
        for (id, name, sex) in &samples_list {
            println!("- ID: {id}, Name: {name}, Sex: {sex}");
        }

        if !samples_list.is_empty() {
            // Build report template
            println!("\nBuilding report template...");
            let manifest_str = tauri_app_lib::db::get_manifest_str(Some(&data_dir));
            let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();
            
            let mut sections = Vec::new();
            if let Some(packs_array) = manifest.get("packs").and_then(|v| v.as_array()) {
                for pack_info in packs_array {
                    if let Some(pack_id) = pack_info.get("id").and_then(|v| v.as_str())
                        && let Some(pack_str) = tauri_app_lib::db::get_pack_str(Some(&data_dir), pack_id) {
                            let pack_json: serde_json::Value = serde_json::from_str(&pack_str).unwrap();
                            if let Some(markers_arr) = pack_json.get("markers").and_then(|v| v.as_array()) {
                                let markers: Vec<tauri_app_lib::report::MarkerDefinition> = serde_json::from_value(serde_json::Value::Array(markers_arr.clone())).unwrap();
                                sections.push(tauri_app_lib::report::SectionDefinition {
                                    name: pack_json.get("name").and_then(|v| v.as_str()).unwrap_or(pack_id).to_string(),
                                    markers,
                                });
                            }
                    }
                }
            }
            
            let template = tauri_app_lib::report::ReportTemplate {
                title: "DNA Analysis & Biohacker Profile Report".to_string(),
                description: "Personal genomic profile matching candidate markers across multiple health systems.".to_string(),
                sections,
            };
            
            println!("Generating report for Sample ID {}...", samples_list[0].0);
            let start_report = std::time::Instant::now();
            match tauri_app_lib::report::generate_report(&conn, samples_list[0].0, &template) {
                Ok(report) => {
                    println!("Report generated successfully in {:?}", start_report.elapsed());
                    println!("Overall Signal Score: {:.2}%", report.overall_signal_score);
                    println!("Number of sections: {}", report.sections.len());
                }
                Err(e) => {
                    println!("Report generation failed: {e}");
                }
            }
        }
    }
    
    // Seal database on exit
    println!("Sealing database...");
    tauri_app_lib::db::seal(&db_path).unwrap();
    println!("Database sealed successfully.");
}
