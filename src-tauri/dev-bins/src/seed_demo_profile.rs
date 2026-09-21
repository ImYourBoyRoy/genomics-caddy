// ./src-tauri/dev-bins/src/seed_demo_profile.rs
//! Replace local imported profiles with the committed synthetic demo fixture.
//!
//! How to run:
//! `cargo run -p genomics-caddy-dev-bins --bin seed_demo_profile -- [data-dir]`
//!
//! Inputs: `src-tauri/testdata/synthetic_demo_ancestry_grch37.csv` and the
//! resolved Genomics Caddy data directory (arg, `GENOMICS_DATA_DIR`, or the
//! portable default). Outputs: one profile named `Example (synthetic)`.
//! Deletes existing sample registry rows and `samples/<id>/` trees only.
//! Never prints genotype values or previous profile names.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const PROFILE_NAME: &str = "Example (synthetic)";

fn repo_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("testdata")
        .join("synthetic_demo_ancestry_grch37.csv")
}

fn list_sample_ids(conn: &rusqlite::Connection) -> Result<Vec<i64>, String> {
    let mut stmt = conn
        .prepare("SELECT id FROM samples ORDER BY id")
        .map_err(|error| format!("list profiles: {error}"))?;
    let ids = stmt
        .query_map([], |row| row.get(0))
        .map_err(|error| format!("list profiles: {error}"))?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|error| format!("list profiles: {error}"))?;
    Ok(ids)
}

fn remove_stray_sample_files(data_dir: &Path) -> Result<(), String> {
    if let Ok(entries) = fs::read_dir(data_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("sample_") && name.ends_with(".db") {
                fs::remove_file(entry.path()).map_err(|error| format!("remove stray sample db: {error}"))?;
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), String> {
    tauri_app_lib::config::init_env();
    let data_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| tauri_app_lib::paths::resolve_data_dir_info().path);
    let fixture = repo_fixture();
    if !fixture.is_file() {
        return Err(format!(
            "missing {}; run node ./scripts/generate_synthetic_demo_fixture.mjs",
            fixture.display()
        ));
    }

    tauri_app_lib::paths::ensure_data_layout(&data_dir)
        .map_err(|error| format!("data layout: {error}"))?;
    let db_path = tauri_app_lib::paths::db_path(&data_dir);
    let mut conn = tauri_app_lib::db::open_user_db(&db_path)
        .map_err(|error| format!("open registry: {error}"))?;

    let existing = list_sample_ids(&conn)?;
    let removed = existing.len();
    for sample_id in existing {
        tauri_app_lib::db::delete_sample(&mut conn, &data_dir, sample_id)
            .map_err(|error| format!("delete profile {sample_id}: {error}"))?;
    }
    remove_stray_sample_files(&data_dir)?;

    let records = tauri_app_lib::parser::parse_dna_file(&fixture, |_| {})
        .map_err(|error| format!("parse synthetic fixture: {error}"))?;
    let record_count = records.len();
    if record_count < 200 {
        return Err(format!("synthetic fixture is too small ({record_count} rows)"));
    }

    let chain_path = tauri_app_lib::offline::liftover_chain_path(&data_dir, &db_path);
    let liftover = if chain_path.exists() {
        match tauri_app_lib::liftover::LiftoverEngine::new(&chain_path) {
            Ok(engine) => Some(engine),
            Err(error) => {
                eprintln!("seed_demo_profile: liftover unavailable ({error}); importing source coordinates");
                None
            }
        }
    } else {
        None
    };

    let sample_id = tauri_app_lib::db::import_raw_genome(
        &mut conn,
        &data_dir,
        PROFILE_NAME,
        &records,
        liftover.as_ref(),
        None,
        |percent, _status| {
            if percent == 0 || percent == 100 || percent % 25 == 0 {
                eprintln!("seed_demo_profile: {percent}%");
            }
        },
    )?;

    println!(
        "seed_demo_profile: removed {removed} profile(s); imported {record_count} synthetic rows as sample_id={sample_id}"
    );
    Ok(())
}
