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

fn parse_mcp_auth_token(args: &[String]) -> Option<String> {
    for arg in args {
        if let Some(token) = arg.strip_prefix("--mcp-auth-token=") {
            let trimmed = token.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    std::env::var("GENOMICS_MCP_TOKEN")
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

fn main() {
    tauri_app_lib::config::init_env();
    let args: Vec<String> = std::env::args().collect();
    let mcp_write = args.iter().any(|arg| arg == "--mcp-write");
    if args.iter().any(|arg| arg == "--mcp") {
        let data_dir = tauri_app_lib::paths::resolve_data_dir_info().path;
        tauri_app_lib::paths::ensure_data_layout(&data_dir).ok();
        let db_path = tauri_app_lib::paths::db_path(&data_dir);
        let auth_token = parse_mcp_auth_token(&args);

        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Failed to start async runtime: {e}");
                std::process::exit(1);
            }
        };
        rt.block_on(async {
            tauri_app_lib::mcp::run_mcp_server(db_path, mcp_write, auth_token).await;
        });
    } else if let Some(headless) = tauri_app_lib::research::parse_headless_args(&args) {
        let data_dir = tauri_app_lib::paths::resolve_data_dir_info().path;
        tauri_app_lib::paths::ensure_data_layout(&data_dir).ok();

        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Failed to start async runtime: {e}");
                std::process::exit(1);
            }
        };
        let code = rt.block_on(async {
            match tauri_app_lib::research::run_headless_sweep(headless).await {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("Headless sweep failed: {e}");
                    1
                }
            }
        });
        std::process::exit(code);
    } else if args.iter().any(|a| a == "--headless-sweep") {
        tauri_app_lib::research::print_headless_usage();
        std::process::exit(2);
    } else {
        tauri_app_lib::run();
    }
}
