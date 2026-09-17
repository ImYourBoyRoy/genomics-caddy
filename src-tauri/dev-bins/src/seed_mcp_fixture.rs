// ./src-tauri/dev-bins/src/seed_mcp_fixture.rs
//! Create a disposable, empty-sample database for MCP runtime QA.
//!
//! This helper is intentionally limited to synthetic metadata. It never reads
//! a user genome and is only used by the opt-in `audit:mcp:write` command.

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let data_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: seed_mcp_fixture <data-directory>")?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|error| format!("create fixture directory: {error}"))?;
    tauri_app_lib::paths::ensure_data_layout(&data_dir)
        .map_err(|error| format!("create fixture data layout: {error}"))?;

    let db_path = tauri_app_lib::paths::db_path(&data_dir);
    let conn = tauri_app_lib::db::open_user_db(&db_path)
        .map_err(|error| format!("bootstrap fixture database: {error}"))?;
    conn.execute(
        "INSERT INTO samples (name, genetic_sex) VALUES ('mcp-runtime-fixture', 'Unknown')",
        [],
    )
    .map_err(|error| format!("insert fixture sample: {error}"))?;
    let sample_id = conn.last_insert_rowid();
    drop(conn);

    tauri_app_lib::db::connect_sample(&data_dir, sample_id)
        .map(|_| ())
        .map_err(|error| format!("create fixture sample database: {error}"))
}
