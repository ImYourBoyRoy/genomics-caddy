// ./src-tauri/src/db_runtime.rs
/*
Purpose: Run SQLite work on blocking threads so the Tauri webview stays responsive.
*/

use std::path::PathBuf;
use tauri::async_runtime;

pub async fn with_connection<F, T>(db_path: PathBuf, f: F) -> Result<T, String>
where
    F: FnOnce(&rusqlite::Connection) -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    async_runtime::spawn_blocking(move || {
        let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
        f(&conn)
    })
    .await
    .map_err(|e| format!("Database worker failed: {}", e))?
}

pub async fn with_connection_mut<F, T>(db_path: PathBuf, f: F) -> Result<T, String>
where
    F: FnOnce(&mut rusqlite::Connection) -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    async_runtime::spawn_blocking(move || {
        let mut conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
        f(&mut conn)
    })
    .await
    .map_err(|e| format!("Database worker failed: {}", e))?
}
