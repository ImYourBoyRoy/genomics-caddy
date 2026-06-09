// ./src-tauri/src/mcp.rs
/*
Module Docstring:
Purpose: Model Context Protocol (MCP) server implementation for genomic database interaction.
Responsibilities:
- Run a stdin/stdout JSON-RPC 2.0 loop when launched in --mcp mode.
- Expose 17 read-only and chat-export tools for local personal genomic exploration.
- Safely query the local user genome SQLite database.
Key Inputs: Stdin JSON-RPC messages.
Key Outputs: Stdout JSON-RPC responses.
Operational Notes: Enables external LLMs to interact directly with the user's standardized genome.
*/

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use rusqlite::params;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Value,
    id: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
}

/// Runs the MCP server loop.
pub async fn run_mcp_server(db_path: PathBuf) {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    // Check if DB exists
    if !db_path.exists() {
        eprintln!("Database does not exist yet. Please import a genome first.");
        return;
    }

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
                    let is_notification = req.id.is_none();
                    let res = handle_request(req, &db_path).await;
                    if !is_notification {
                        if let Ok(res_str) = serde_json::to_string(&res) {
                            println!("{}", res_str);
                            let _ = io::stdout().flush();
                        }
                    }
                } else {
                    let err_res = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(json!({ "code": -32700, "message": "Parse error" })),
                        id: None,
                    };
                    if let Ok(res_str) = serde_json::to_string(&err_res) {
                        println!("{}", res_str);
                        let _ = io::stdout().flush();
                    }
                }
            }
            Err(_) => break,
        }
    }
}

async fn handle_request(req: JsonRpcRequest, db_path: &PathBuf) -> JsonRpcResponse {
    let id = req.id;
    match req.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "tauri-genomics-mcp",
                    "version": "1.0.0"
                }
            })),
            error: None,
            id,
        },
        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({
                "tools": [
                    {
                        "name": "list_samples",
                        "description": "Lists all imported DNA samples in the local SQLite database.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "get_variants_by_rsid",
                        "description": "Queries specific genotypes for a list of rsIDs in a given sample.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer", "description": "The target sample ID" },
                                "rsids": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "List of rsIDs to query, e.g., ['rs1801133']"
                                }
                            },
                            "required": ["sample_id", "rsids"]
                        }
                    },
                    {
                        "name": "get_variants_in_region",
                        "description": "Queries all standard variants in a given chromosome region (GRCh38 coordinates).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer", "description": "The target sample ID" },
                                "chromosome": { "type": "string", "description": "Chromosome number or label, e.g. '1', 'X'" },
                                "start": { "type": "integer", "description": "Start base-pair position" },
                                "end": { "type": "integer", "description": "End base-pair position" }
                            },
                            "required": ["sample_id", "chromosome", "start", "end"]
                        }
                    },
                    {
                        "name": "generate_report",
                        "description": "Generates a full direction-aware trait report for a sample using the built-in marker database. Returns evaluated markers with severity classes, section summaries, and risk-direction-only signal scores.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer", "description": "The target sample ID" },
                                "template_json": { "type": "string", "description": "JSON string of the report template with sections and markers" }
                            },
                            "required": ["sample_id", "template_json"]
                        }
                    },
                    {
                        "name": "list_packs",
                        "description": "Lists all available predefined genomic marker packs/bundles and their descriptions.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "get_report_for_packs",
                        "description": "Generates an evaluated genomic report for specific pack IDs (or all if omitted). Can optionally filter to only return active variant findings (effect_count > 0).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer", "description": "The target sample ID" },
                                "pack_ids": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Predefined pack IDs to evaluate, e.g. ['core', 'pgx', 'nutrients']. If omitted/empty, evaluates all packs."
                                },
                                "only_active_findings": {
                                    "type": "boolean",
                                    "description": "If true, only returns evaluated markers with effect alleles detected (effect_count > 0). Defaults to false."
                                }
                            },
                            "required": ["sample_id"]
                        }
                    },
                    {
                        "name": "list_evidence_sources",
                        "description": "Lists all unique source citations in the local RAG evidence library.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "get_evidence_for_marker",
                        "description": "Queries the local evidence library for references and interpretation notes associated with a specific rsID.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "rsid": { "type": "string", "description": "The target rsID, e.g. 'rs4680'" }
                            },
                            "required": ["rsid"]
                        }
                    },
                    {
                        "name": "search_evidence",
                        "description": "Performs keyword and semantic vector search in the local evidence library.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": { "type": "string", "description": "The search term or query" },
                                "ollama_url": { "type": "string", "description": "Ollama server URL for semantic search embeddings" },
                                "ollama_token": { "type": "string", "description": "Authentication token for remote Ollama server" }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "get_chat_sessions",
                        "description": "Lists saved consultation chat sessions from the local SQLite database.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer", "description": "Filter sessions by sample ID (optional)" }
                            }
                        }
                    },
                    {
                        "name": "delete_chat_session",
                        "description": "Deletes a specific consultation chat session.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "session_id": { "type": "string", "description": "The session ID to delete" }
                            },
                            "required": ["session_id"]
                        }
                    },
                    {
                        "name": "export_chat_history",
                        "description": "Exports a saved chat session history in a clean, human-readable Markdown format.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "session_id": { "type": "string", "description": "The session ID to export" }
                            },
                            "required": ["session_id"]
                        }
                    },
                    {
                        "name": "get_app_paths",
                        "description": "Retrieves the local application directory paths (database and marker packs folders).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "check_chain_status",
                        "description": "Checks if the GRCh37-to-GRCh38 liftover chain alignment file is locally present.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "get_current_exe",
                        "description": "Returns the absolute path of the running Genomics Caddy executable.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "scan_ollama_models",
                        "description": "Queries a local or remote Ollama server to list all available LLM models.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": { "type": "string", "description": "Ollama server URL" },
                                "token": { "type": "string", "description": "Authentication token" }
                            },
                            "required": ["url"]
                        }
                    },
                    {
                        "name": "show_ollama_model",
                        "description": "Retrieves detailed configuration and parameters for a specific Ollama model.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": { "type": "string", "description": "Ollama server URL" },
                                "token": { "type": "string", "description": "Authentication token" },
                                "name": { "type": "string", "description": "The model tag name" }
                            },
                            "required": ["url", "name"]
                        }
                    },
                    {
                        "name": "get_active_ollama_models",
                        "description": "Queries a local or remote Ollama server to list currently loaded models and VRAM usage.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": { "type": "string", "description": "Ollama server URL" },
                                "token": { "type": "string", "description": "Authentication token" }
                            },
                            "required": ["url"]
                        }
                    }
                ]
            })),
            error: None,
            id,
        },
        "tools/call" => {
            let name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));
            
            match execute_tool(name, arguments, db_path).await {
                Ok(data) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&data).unwrap_or_default()
                            }
                        ]
                    })),
                    error: None,
                    id,
                },
                Err(err) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(json!({ "code": -32000, "message": err })),
                    id,
                },
            }
        }
        "notifications/initialized" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: None,
            id,
        },
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(json!({ "code": -32601, "message": "Method not found" })),
            id,
        },
    }
}

async fn execute_tool(name: &str, args: Value, db_path: &PathBuf) -> Result<Value, String> {
    let conn = crate::db::init_user_db(db_path).map_err(|e| format!("DB connection error: {}", e))?;

    match name {
        "list_samples" => {
            let samples = crate::db::get_samples(&conn)
                .map_err(|e| format!("Failed to read samples: {}", e))?;
            Ok(json!(samples))
        }
        "get_variants_by_rsid" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let rsids_val = args.get("rsids").and_then(|v| v.as_array()).ok_or("Missing rsids array")?;
            
            let mut rsids = Vec::new();
            for val in rsids_val {
                if let Some(s) = val.as_str() {
                    rsids.push(s.to_string());
                }
            }

            let results = crate::db::query_by_rsids(&conn, sample_id, &rsids)
                .map_err(|e| format!("Query failed: {}", e))?;
            Ok(json!(results))
        }
        "get_variants_in_region" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let chromosome = args.get("chromosome").and_then(|v| v.as_str()).ok_or("Missing chromosome")?;
            let start = args.get("start").and_then(|v| v.as_u64()).ok_or("Missing start position")?;
            let end = args.get("end").and_then(|v| v.as_u64()).ok_or("Missing end position")?;

            let results = crate::db::query_region(&conn, sample_id, chromosome, start, end)
                .map_err(|e| format!("Query failed: {}", e))?;
            Ok(json!(results))
        }
        "generate_report" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let template_json = args.get("template_json").and_then(|v| v.as_str()).ok_or("Missing template_json")?;
            
            let template: crate::report::ReportTemplate = serde_json::from_str(template_json)
                .map_err(|e| format!("Failed to parse template: {}", e))?;
            
            let report = crate::report::generate_report(&conn, sample_id, &template)?;
            Ok(serde_json::to_value(report).map_err(|e| format!("Serialization error: {}", e))?)
        }
        "list_packs" => {
            let app_data_dir = db_path.parent();
            let manifest_str = crate::db::get_manifest_str(app_data_dir);
            let manifest: Value = serde_json::from_str(&manifest_str)
                .map_err(|e| format!("Failed to parse manifest: {}", e))?;
            Ok(manifest)
        }
        "get_report_for_packs" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let pack_ids_val = args.get("pack_ids").and_then(|v| v.as_array());
            let only_active_findings = args.get("only_active_findings").and_then(|v| v.as_bool()).unwrap_or(false);

            // 1. Resolve pack IDs
            let mut pack_ids = Vec::new();
            if let Some(arr) = pack_ids_val {
                for val in arr {
                    if let Some(s) = val.as_str() {
                        pack_ids.push(s.to_string());
                    }
                }
            }

            let app_data_dir = db_path.parent();
            if pack_ids.is_empty() {
                // Load all pack IDs from manifest
                #[derive(Debug, Deserialize)]
                struct ManifestPack {
                    id: String,
                }
                #[derive(Debug, Deserialize)]
                struct Manifest {
                    packs: Vec<ManifestPack>,
                }
                let manifest_str = crate::db::get_manifest_str(app_data_dir);
                let manifest: Manifest = serde_json::from_str(&manifest_str)
                    .map_err(|e| format!("Failed to parse manifest: {}", e))?;
                for pack in manifest.packs {
                    pack_ids.push(pack.id);
                }
            }

            // 2. Parse individual packs and construct a ReportTemplate
            let mut sections = Vec::new();
            for pack_id in &pack_ids {
                if let Some(pack_str) = crate::db::get_pack_str(app_data_dir, pack_id) {
                    #[derive(Debug, Deserialize)]
                    struct PackContent {
                        name: String,
                        markers: Vec<crate::report::MarkerDefinition>,
                    }
                    let pack: PackContent = serde_json::from_str(&pack_str)
                        .map_err(|e| format!("Failed to parse pack {}: {}", pack_id, e))?;
                    
                    sections.push(crate::report::SectionDefinition {
                        name: pack.name,
                        markers: pack.markers,
                    });
                } else {
                    return Err(format!("Unknown pack ID: {}", pack_id));
                }
            }

            let template = crate::report::ReportTemplate {
                title: "DNA Analysis & Biohacker Profile Report".to_string(),
                description: "Personal genomic profile matching candidate markers across multiple health systems.".to_string(),
                sections,
            };

            // 3. Generate the report
            let mut report = crate::report::generate_report(&conn, sample_id, &template)?;

            // 4. Optionally filter to only active findings
            if only_active_findings {
                for section in &mut report.sections {
                    section.markers.retain(|m| m.effect_count > 0 && m.severity_class != "no_data");
                }
                // Filter out sections that are now empty
                report.sections.retain(|section| !section.markers.is_empty());
            }

            Ok(serde_json::to_value(report).map_err(|e| format!("Serialization error: {}", e))?)
        }
        "list_evidence_sources" => {
            let mut stmt = conn.prepare("SELECT DISTINCT source_citation FROM evidence_library ORDER BY source_citation ASC")
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            let mut sources = Vec::new();
            for r in rows {
                if let Ok(s) = r {
                    sources.push(s);
                }
            }
            Ok(json!(sources))
        }
        "get_evidence_for_marker" => {
            let rsid = args.get("rsid").and_then(|v| v.as_str()).ok_or("Missing rsid")?;
            let mut stmt = conn.prepare("SELECT rsid, gene, evidence_text, source_citation, embedding FROM evidence_library WHERE rsid = ?")
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![rsid], |row| {
                let embedding: Option<String> = row.get(4)?;
                Ok(json!({
                    "rsid": row.get::<_, String>(0)?,
                    "gene": row.get::<_, String>(1)?,
                    "evidence_text": row.get::<_, String>(2)?,
                    "source_citation": row.get::<_, String>(3)?,
                    "has_embedding": embedding.is_some() && !embedding.unwrap().trim().is_empty(),
                }))
            }).map_err(|e| e.to_string())?;
            let mut results = Vec::new();
            for r in rows {
                results.push(r.map_err(|e| e.to_string())?);
            }
            Ok(json!(results))
        }
        "search_evidence" => {
            let query = args.get("query").and_then(|v| v.as_str()).ok_or("Missing query")?;
            let ollama_url = args.get("ollama_url").and_then(|v| v.as_str());
            let ollama_token = args.get("ollama_token").and_then(|v| v.as_str());

            let query_clean = query.trim().to_string();
            if query_clean.is_empty() {
                return Ok(json!(Value::Null));
            }

            // 1. Keyword search
            let keyword_hits = {
                let search_pattern = format!("%{}%", query_clean.to_lowercase());
                let mut stmt = conn.prepare(
                    "SELECT rsid, gene, evidence_text, source_citation, embedding 
                     FROM evidence_library 
                     WHERE rsid LIKE ? OR gene LIKE ? OR LOWER(evidence_text) LIKE ?"
                ).map_err(|e| e.to_string())?;
                
                let rows = stmt.query_map(params![search_pattern, search_pattern, search_pattern], |row| {
                    let embedding: Option<String> = row.get(4)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        embedding,
                    ))
                }).map_err(|e| e.to_string())?;

                let mut hits = Vec::new();
                for r in rows {
                    hits.push(r.map_err(|e| e.to_string())?);
                }
                hits
            };

            // 2. Semantic search if URL is provided
            if let Some(url) = ollama_url {
                if !url.trim().is_empty() {
                    let clean_url = url.trim().trim_end_matches('/').to_string();
                    if let Some(embed_model) = super::get_embedding_model(&clean_url, ollama_token).await {
                        if let Ok(query_embedding) = super::fetch_embedding(&clean_url, ollama_token, &embed_model, &query_clean).await {
                            // Generate embeddings on-demand for keyword hits
                            let mut new_embeddings = Vec::new();
                            for (rsid, _gene, text, citation, embedding_opt) in keyword_hits.iter().take(10) {
                                if embedding_opt.is_none() || embedding_opt.as_ref().unwrap().trim().is_empty() {
                                    if let Ok(emb) = super::fetch_embedding(&clean_url, ollama_token, &embed_model, text).await {
                                        new_embeddings.push((rsid.clone(), citation.clone(), emb));
                                    }
                                }
                            }

                            // Save new embeddings back to database
                            if !new_embeddings.is_empty() {
                                for (rsid, citation, emb) in &new_embeddings {
                                    if let Ok(emb_json) = serde_json::to_string(emb) {
                                        let _ = conn.execute(
                                            "UPDATE evidence_library SET embedding = ? WHERE rsid = ? AND source_citation = ?",
                                            params![emb_json, rsid, citation],
                                        );
                                    }
                                }
                            }

                            // Run cosine similarity matching across ALL records that have embeddings
                            let mut stmt = conn.prepare(
                                "SELECT rsid, gene, evidence_text, source_citation, embedding FROM evidence_library WHERE embedding IS NOT NULL AND embedding != ''"
                            ).map_err(|e| e.to_string())?;
                            
                            let rows = stmt.query_map([], |row| {
                                Ok((
                                    row.get::<_, String>(0)?,
                                    row.get::<_, String>(1)?,
                                    row.get::<_, String>(2)?,
                                    row.get::<_, String>(3)?,
                                    row.get::<_, String>(4)?,
                                ))
                            }).map_err(|e| e.to_string())?;

                            let mut candidates = Vec::new();
                            for r in rows {
                                let (rsid, gene, text, citation, emb_str) = r.map_err(|e| e.to_string())?;
                                if let Ok(emb) = serde_json::from_str::<Vec<f32>>(&emb_str) {
                                    // Calculate cosine similarity
                                    if emb.len() == query_embedding.len() {
                                        let mut dot_product = 0.0;
                                        let mut norm_a = 0.0;
                                        let mut norm_b = 0.0;
                                        for i in 0..emb.len() {
                                            dot_product += emb[i] * query_embedding[i];
                                            norm_a += emb[i] * emb[i];
                                            norm_b += query_embedding[i] * query_embedding[i];
                                        }
                                        let similarity = if norm_a > 0.0 && norm_b > 0.0 {
                                            dot_product / (norm_a.sqrt() * norm_b.sqrt())
                                        } else {
                                            0.0
                                        };
                                        candidates.push(json!({
                                            "rsid": rsid,
                                            "gene": gene,
                                            "evidence_text": text,
                                            "source_citation": citation,
                                            "has_embedding": true,
                                            "similarity": similarity,
                                        }));
                                    }
                                }
                            }

                            // Sort by similarity descending
                            candidates.sort_by(|a, b| {
                                let sim_a = a.get("similarity").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let sim_b = b.get("similarity").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                sim_b.partial_cmp(&sim_a).unwrap_or(std::cmp::Ordering::Equal)
                            });

                            // Filter to similarity > 0.35 and limit to 10
                            let filtered: Vec<Value> = candidates.into_iter()
                                .filter(|c| c.get("similarity").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.35)
                                .take(10)
                                .collect();

                            return Ok(json!(filtered));
                        }
                    }
                }
            }

            // Fallback to keyword hits if no semantic search was executed
            let results: Vec<Value> = keyword_hits.into_iter().map(|(rsid, gene, text, citation, emb)| {
                json!({
                    "rsid": rsid,
                    "gene": gene,
                    "evidence_text": text,
                    "source_citation": citation,
                    "has_embedding": emb.is_some() && !emb.unwrap().trim().is_empty(),
                    "similarity": Value::Null,
                })
            }).collect();

            Ok(json!(results))
        }
        "get_chat_sessions" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64());
            let sessions = crate::db::get_chat_sessions(&conn, sample_id)
                .map_err(|e| format!("Failed to read chat sessions: {}", e))?;
            Ok(json!(sessions))
        }
        "delete_chat_session" => {
            let session_id = args.get("session_id").and_then(|v| v.as_str()).ok_or("Missing session_id")?;
            crate::db::delete_chat_session(&conn, session_id)
                .map_err(|e| format!("Failed to delete chat session: {}", e))?;
            Ok(json!({ "status": "deleted", "session_id": session_id }))
        }
        "export_chat_history" => {
            let session_id = args.get("session_id").and_then(|v| v.as_str()).ok_or("Missing session_id")?;
            let sessions = crate::db::get_chat_sessions(&conn, None)
                .map_err(|e| format!("Failed to query sessions: {}", e))?;
            let session = sessions.iter().find(|s| s.id == session_id)
                .ok_or_else(|| format!("Session not found: {}", session_id))?;
            
            let mut md = format!("# Chat Export: {}\n\n", session.title);
            md.push_str(&format!("* **Timestamp:** {} (Epoch ms)\n", session.timestamp));
            md.push_str(&format!("* **Model:** {}\n", session.selected_model));
            md.push_str(&format!("* **Temperature:** {}\n\n", session.temperature));
            md.push_str("---\n\n");

            for msg in &session.messages {
                md.push_str(&format!("### {}\n\n", match msg.role.as_str() {
                    "user" => "👤 User",
                    "assistant" => "🤖 Genomics Assistant",
                    "system" => "⚙️ System",
                    _ => &msg.role
                }));
                md.push_str(&msg.content);
                md.push_str("\n\n");
                if let Some(ref safety) = msg.safety_review {
                    md.push_str(&format!("> 🛡️ **Safety Review Notes:**\n> {}\n\n", safety.replace('\n', "\n> ")));
                }
            }
            Ok(json!({ "markdown": md }))
        }
        "get_app_paths" => {
            let app_data_dir = db_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            Ok(json!({
                "db_path": db_path.to_string_lossy().to_string(),
                "app_data_dir": app_data_dir,
            }))
        }
        "check_chain_status" => {
            let app_data_dir = db_path.parent().ok_or("Could not resolve app data dir")?;
            let chain_path = app_data_dir.join("GRCh37_to_GRCh38.chain.gz");
            Ok(json!({
                "chain_file_exists": chain_path.exists(),
                "chain_file_path": chain_path.to_string_lossy().to_string()
            }))
        }
        "get_current_exe" => {
            let p = std::env::current_exe().map_err(|e| e.to_string())?;
            Ok(json!({ "executable_path": p.to_string_lossy().to_string().replace('\\', "/") }))
        }
        "scan_ollama_models" => {
            let url = args.get("url").and_then(|v| v.as_str()).ok_or("Missing url")?;
            let token = args.get("token").and_then(|v| v.as_str());
            
            let client = reqwest::Client::new();
            let clean_url = url.trim().trim_end_matches('/');
            let mut req = client.get(format!("{}/api/tags", clean_url));
            
            if let Some(t) = token {
                if !t.trim().is_empty() {
                    req = req.header("Authorization", if t.to_lowercase().starts_with("bearer ") { t.to_string() } else { format!("Bearer {}", t) });
                }
            }
            
            let res = req.send().await.map_err(|e| format!("Connection error: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Ollama tags API returned HTTP error: {}", res.status()));
            }
            
            #[derive(serde::Deserialize)]
            struct OllamaModel { name: String }
            #[derive(serde::Deserialize)]
            struct OllamaTagsResponse { models: Vec<OllamaModel> }
            
            let tags: OllamaTagsResponse = res.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;
            let models: Vec<String> = tags.models.into_iter().map(|m| m.name).collect();
            Ok(json!(models))
        }
        "show_ollama_model" => {
            let url = args.get("url").and_then(|v| v.as_str()).ok_or("Missing url")?;
            let token = args.get("token").and_then(|v| v.as_str());
            let name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing name")?;
            
            let client = reqwest::Client::new();
            let clean_url = url.trim().trim_end_matches('/');
            let mut req = client.post(format!("{}/api/show", clean_url));
            
            if let Some(t) = token {
                if !t.trim().is_empty() {
                    req = req.header("Authorization", if t.to_lowercase().starts_with("bearer ") { t.to_string() } else { format!("Bearer {}", t) });
                }
            }
            
            let payload = serde_json::json!({ "name": name });
            let res = req.json(&payload).send().await.map_err(|e| format!("Connection error: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Ollama show API returned HTTP error: {}", res.status()));
            }
            
            let details: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;
            Ok(details)
        }
        "get_active_ollama_models" => {
            let url = args.get("url").and_then(|v| v.as_str()).ok_or("Missing url")?;
            let token = args.get("token").and_then(|v| v.as_str());
            
            let client = reqwest::Client::new();
            let clean_url = url.trim().trim_end_matches('/');
            let mut req = client.get(format!("{}/api/ps", clean_url));
            
            if let Some(t) = token {
                if !t.trim().is_empty() {
                    req = req.header("Authorization", if t.to_lowercase().starts_with("bearer ") { t.to_string() } else { format!("Bearer {}", t) });
                }
            }
            
            let res = req.send().await.map_err(|e| format!("Connection error: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Ollama ps API returned HTTP error: {}", res.status()));
            }
            
            let info: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;
            Ok(info)
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}
