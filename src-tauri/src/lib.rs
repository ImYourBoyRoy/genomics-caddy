// ./src-tauri/src/lib.rs
/*
Module Docstring:
Purpose: Tauri library entry point and command handler declarations.
Responsibilities:
- Declare all internal Rust modules.
- Expose Tauri command APIs for Svelte frontend to query samples, import genomes, run liftovers, and evaluate reports.
- Emits real-time import progress events to the frontend.
Key Inputs: Tauri app states and command parameters.
Key Outputs: JSON responses to the frontend.
Operational Notes: Manages SQLite database connection pools and executes requests.
*/

pub mod parser;
pub mod liftover;
pub mod db;
pub mod mcp;
pub mod report;

use std::path::PathBuf;
use tauri::{AppHandle, Manager, Emitter};
use db::{SampleInfo, DbSnpRecord};
use report::GeneratedReport;

#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    percentage: u32,
    status: String,
}

fn get_db_path(app: &AppHandle) -> PathBuf {
    let mut path = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    std::fs::create_dir_all(&path).ok();
    path.push("user_genome.db");
    path
}

fn get_chain_path(app: &AppHandle) -> PathBuf {
    let mut path = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    std::fs::create_dir_all(&path).ok();
    path.push("GRCh37_to_GRCh38.chain.gz");
    path
}

#[tauri::command]
async fn select_file() -> Result<Option<String>, String> {
    let file = rfd::FileDialog::new()
        .add_filter("Genomic Data", &["txt", "zip"])
        .pick_file();
    Ok(file.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
async fn save_report_json(content: String, default_filename: String) -> Result<bool, String> {
    let file = rfd::FileDialog::new()
        .set_file_name(&default_filename)
        .add_filter("JSON Report", &["json"])
        .save_file();

    if let Some(path) = file {
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
fn get_app_paths(app: AppHandle) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app);
    let chain_path = get_chain_path(&app);
    Ok(serde_json::json!({
        "db_path": db_path.to_string_lossy().to_string(),
        "chain_path": chain_path.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
async fn import_genome(app: AppHandle, file_path: String, sample_name: String) -> Result<i64, String> {
    // Emit initial status
    app.emit("import-progress", ProgressPayload {
        percentage: 5,
        status: "Loading file...".to_string(),
    }).ok();

    // 1. Parse raw DNA file (either txt or zip)
    let app_clone = app.clone();
    let records = parser::parse_dna_file(&file_path, move |status| {
        app_clone.emit("import-progress", ProgressPayload {
            percentage: 15,
            status: status.to_string(),
        }).ok();
    })?;

    app.emit("import-progress", ProgressPayload {
        percentage: 45,
        status: "Initializing liftover database...".to_string(),
    }).ok();

    // 2. Resolve liftover engine if chain file exists
    let chain_path = get_chain_path(&app);
    let liftover_engine = if chain_path.exists() {
        match liftover::LiftoverEngine::new(&chain_path) {
            Ok(engine) => Some(engine),
            Err(e) => {
                eprintln!("Failed to initialize liftover engine: {}", e);
                None
            }
        }
    } else {
        println!("No liftover chain found at {:?}. Importing without GRCh38 coordinates.", chain_path);
        None
    };

    // 3. Import into database
    let db_path = get_db_path(&app);
    let mut conn = db::init_user_db(&db_path).map_err(|e| e.to_string())?;
    
    let app_clone = app.clone();
    let sample_id = db::import_raw_genome(
        &mut conn, 
        &sample_name, 
        &records, 
        liftover_engine.as_ref(),
        move |pct, status| {
            app_clone.emit("import-progress", ProgressPayload {
                percentage: pct,
                status: status.to_string(),
            }).ok();
        }
    )?;

    Ok(sample_id)
}

#[tauri::command]
async fn get_samples(app: AppHandle) -> Result<Vec<SampleInfo>, String> {
    let db_path = get_db_path(&app);
    let conn = db::init_user_db(&db_path).map_err(|e| e.to_string())?;
    let samples = db::get_samples(&conn).map_err(|e| e.to_string())?;
    Ok(samples)
}

#[tauri::command]
async fn query_rsids(app: AppHandle, sample_id: i64, rsids: Vec<String>) -> Result<Vec<DbSnpRecord>, String> {
    let db_path = get_db_path(&app);
    let conn = db::init_user_db(&db_path).map_err(|e| e.to_string())?;
    let records = db::query_by_rsids(&conn, sample_id, &rsids).map_err(|e| e.to_string())?;
    Ok(records)
}

#[tauri::command]
async fn query_region(
    app: AppHandle,
    sample_id: i64,
    chromosome: String,
    start: u64,
    end: u64,
) -> Result<Vec<DbSnpRecord>, String> {
    let db_path = get_db_path(&app);
    let conn = db::init_user_db(&db_path).map_err(|e| e.to_string())?;
    let records = db::query_region(&conn, sample_id, &chromosome, start, end).map_err(|e| e.to_string())?;
    Ok(records)
}

#[tauri::command]
async fn generate_report(app: AppHandle, sample_id: i64, template_json: String) -> Result<GeneratedReport, String> {
    let db_path = get_db_path(&app);
    let conn = db::init_user_db(&db_path).map_err(|e| e.to_string())?;
    
    let template: report::ReportTemplate = serde_json::from_str(&template_json)
        .map_err(|e| format!("Failed to parse report template JSON: {}", e))?;

    let report = report::generate_report(&conn, sample_id, &template)?;
    Ok(report)
}

#[tauri::command]
async fn delete_sample(app: AppHandle, sample_id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let conn = db::init_user_db(&db_path).map_err(|e| e.to_string())?;
    db::delete_sample(&conn, sample_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn check_chain_status(app: AppHandle) -> Result<bool, String> {
    let chain_path = get_chain_path(&app);
    Ok(chain_path.exists())
}

#[tauri::command]
async fn download_chain_file(app: AppHandle) -> Result<(), String> {
    let chain_path = get_chain_path(&app);
    let url = "https://ftp.ensembl.org/pub/assembly_mapping/homo_sapiens/GRCh37_to_GRCh38.chain.gz";

    let response = reqwest::get(url).await.map_err(|e| format!("Failed to download chain file: {}", e))?;
    let mut file = std::fs::File::create(&chain_path).map_err(|e| format!("Failed to create file: {}", e))?;
    
    let content = response.bytes().await.map_err(|e| format!("Failed to read download content: {}", e))?;
    std::io::copy(&mut &*content, &mut file).map_err(|e| format!("Failed to write to file: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn scan_ollama_models(url: String, token: Option<String>) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let clean_url = url.trim().trim_end_matches('/');
    let mut req = client.get(format!("{}/api/tags", clean_url));
    
    if let Some(t) = token {
        if !t.trim().is_empty() {
            let t_val = t.trim();
            req = req.header("Authorization", if t_val.to_lowercase().starts_with("bearer ") { t_val.to_string() } else { format!("Bearer {}", t_val) });
        }
    }
    
    let res = req.send().await.map_err(|e| format!("Connection error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }
    
    #[derive(serde::Deserialize)]
    struct OllamaModel {
        name: String,
    }
    #[derive(serde::Deserialize)]
    struct OllamaTagsResponse {
        models: Vec<OllamaModel>,
    }
    
    let tags: OllamaTagsResponse = res.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(tags.models.into_iter().map(|m| m.name).collect())
}

#[tauri::command]
async fn show_ollama_model(
    url: String,
    token: Option<String>,
    name: String,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let clean_url = url.trim().trim_end_matches('/');
    let mut req = client.post(format!("{}/api/show", clean_url));
    
    if let Some(t) = token {
        if !t.trim().is_empty() {
            let t_val = t.trim();
            req = req.header("Authorization", if t_val.to_lowercase().starts_with("bearer ") { t_val.to_string() } else { format!("Bearer {}", t_val) });
        }
    }
    
    let payload = serde_json::json!({
        "name": name
    });
    
    let res = req.json(&payload).send().await.map_err(|e| format!("Connection error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }
    
    let details: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(details)
}

#[tauri::command]
async fn stream_ollama_chat(
    app: AppHandle,
    url: String,
    token: Option<String>,
    model: String,
    messages: Vec<serde_json::Value>,
    temperature: Option<f64>,
    num_predict: Option<u32>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let clean_url = url.trim().trim_end_matches('/');
    let mut req = client.post(format!("{}/api/chat", clean_url));
    
    if let Some(t) = token {
        if !t.trim().is_empty() {
            let t_val = t.trim();
            req = req.header("Authorization", if t_val.to_lowercase().starts_with("bearer ") { t_val.to_string() } else { format!("Bearer {}", t_val) });
        }
    }
    
    let payload = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "options": {
            "temperature": temperature.unwrap_or(0.0),
            "num_predict": num_predict.unwrap_or(2048)
        }
    });
    
    let mut res = req.json(&payload).send().await.map_err(|e| format!("Connection error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }
    
    let mut prompt_eval_count = None;
    let mut eval_count = None;
    
    let mut buffer = String::new();
    while let Some(chunk) = res.chunk().await.map_err(|e| format!("Stream error: {}", e))? {
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);
        
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();
            
            if line.is_empty() {
                continue;
            }
            
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(content) = val.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                    app.emit("ollama-chunk", content).ok();
                }
                if let Some(pec) = val.get("prompt_eval_count").and_then(|v| v.as_u64()) {
                    prompt_eval_count = Some(pec);
                }
                if let Some(ec) = val.get("eval_count").and_then(|v| v.as_u64()) {
                    eval_count = Some(ec);
                }
            }
        }
    }
    
    if !buffer.trim().is_empty() {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(buffer.trim()) {
            if let Some(content) = val.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                app.emit("ollama-chunk", content).ok();
            }
            if let Some(pec) = val.get("prompt_eval_count").and_then(|v| v.as_u64()) {
                prompt_eval_count = Some(pec);
            }
            if let Some(ec) = val.get("eval_count").and_then(|v| v.as_u64()) {
                eval_count = Some(ec);
            }
        }
    }
    
    #[derive(serde::Serialize, Clone)]
    struct DonePayload {
        prompt_eval_count: Option<u64>,
        eval_count: Option<u64>,
    }
    
    app.emit("ollama-done", DonePayload {
        prompt_eval_count,
        eval_count,
    }).ok();
    Ok(())
}

#[tauri::command]
fn get_current_exe() -> Result<String, String> {
    let p = std::env::current_exe()
        .map_err(|e| e.to_string())?;
    Ok(p.to_string_lossy().to_string().replace('\\', "/"))
}

#[tauri::command]
fn get_mcp_tools() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!([
        {
            "name": "list_samples",
            "description": "Lists all imported DNA samples in the local SQLite database.",
            "params": []
        },
        {
            "name": "get_variants_by_rsid",
            "description": "Queries specific genotypes for a list of rsIDs in a given sample.",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" },
                { "name": "rsids", "type": "array of strings", "required": true, "description": "List of rsIDs to query, e.g. ['rs1801133']" }
            ]
        },
        {
            "name": "get_variants_in_region",
            "description": "Queries all standard variants in a given chromosome region (GRCh38 coordinates).",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" },
                { "name": "chromosome", "type": "string", "required": true, "description": "Chromosome number or label (e.g. '1', 'X')" },
                { "name": "start", "type": "integer", "required": true, "description": "Start base-pair position" },
                { "name": "end", "type": "integer", "required": true, "description": "End base-pair position" }
            ]
        },
        {
            "name": "generate_report",
            "description": "Generates a full direction-aware trait report for a sample using the built-in marker database. Returns evaluated markers with severity classes, section summaries, and risk-direction-only signal scores.",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" },
                { "name": "template_json", "type": "string", "required": true, "description": "JSON string of the report template with sections and markers" }
            ]
        },
        {
            "name": "list_packs",
            "description": "Lists all available predefined genomic marker packs/bundles and their descriptions.",
            "params": []
        },
        {
            "name": "get_report_for_packs",
            "description": "Generates an evaluated genomic report for specific pack IDs (or all if omitted). Can optionally filter to only return active variant findings (effect_count > 0).",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" },
                { "name": "pack_ids", "type": "array of strings", "required": false, "description": "Predefined pack IDs to evaluate, e.g. ['core', 'pgx', 'nutrients']. If omitted, evaluates all." },
                { "name": "only_active_findings", "type": "boolean", "required": false, "description": "If true, only returns evaluated markers with effect alleles detected (effect_count > 0). Defaults to false." }
            ]
        }
    ]))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            select_file,
            save_report_json,
            get_app_paths,
            import_genome,
            get_samples,
            query_rsids,
            query_region,
            generate_report,
            delete_sample,
            check_chain_status,
            download_chain_file,
            scan_ollama_models,
            stream_ollama_chat,
            show_ollama_model,
            get_current_exe,
            get_mcp_tools
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
