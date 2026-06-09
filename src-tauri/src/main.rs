// ./src-tauri/src/main.rs
/*
Module Docstring:
Purpose: Entry point for the Tauri app.
Responsibilities:
- Inspect command-line arguments to detect `--mcp` flag.
- Run the MCP server in stdin/stdout mode if requested.
- Fallback to launching the standard Tauri application GUI.
Key Inputs: CLI args.
Key Outputs: Main app runner or stdin/stdout MCP execution loop.
Operational Notes: Enables unified CLI + GUI packaging.
*/

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--mcp") {
        // Run MCP server loop
        let mut db_path = if let Ok(appdata) = std::env::var("APPDATA") {
            std::path::PathBuf::from(appdata)
        } else if let Ok(home) = std::env::var("HOME") {
            let mut p = std::path::PathBuf::from(home);
            p.push(".config");
            p
        } else {
            std::path::PathBuf::from(".")
        };
        db_path.push("com.dna.explorer");
        db_path.push("user_genome.db");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tauri_app_lib::mcp::run_mcp_server(db_path).await;
        });
    } else {
        tauri_app_lib::run();
    }
}
