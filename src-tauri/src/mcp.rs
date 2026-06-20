// ./src-tauri/src/mcp.rs
/*
Module Docstring:
Purpose: Model Context Protocol (MCP) server implementation for genomic database interaction.
Responsibilities:
- Run a stdin/stdout JSON-RPC 2.0 loop when launched in --mcp mode.
- Expose read-only genomic tools by default; mutating tools require `--mcp-write`.
- Safely query the local user genome SQLite database.
Key Inputs: Stdin JSON-RPC messages.
Key Outputs: Stdout JSON-RPC responses.
Operational Notes: Enables external LLMs to interact directly with the user's standardized genome.
*/

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use rusqlite::params;

const MCP_WRITE_TOOLS: &[&str] = &[
    "delete_chat_session",
    "update_candidate_marker_status",
    "backfill_evidence_payloads",
    "build_vector_atlas",
    "enable_named_vectors_collection",
];

fn mcp_tool_requires_write(name: &str) -> bool {
    MCP_WRITE_TOOLS.contains(&name)
}

/// Whether a tool appears in read-only `tools/list` responses.
pub fn mcp_tool_visible_read_only(name: &str) -> bool {
    !mcp_tool_requires_write(name)
}

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

/// Runs the MCP server loop (`allow_write` enables mutating tools).
/// When `auth_token` is set, each request must include `params._meta.authToken` matching the token.
pub async fn run_mcp_server(db_path: PathBuf, allow_write: bool, auth_token: Option<String>) {
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
                    let res = handle_request(req, &db_path, allow_write, &auth_token).await;
                    if !is_notification
                        && let Ok(res_str) = serde_json::to_string(&res) {
                            println!("{}", res_str);
                            let _ = io::stdout().flush();
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

    if let Err(e) = crate::db::seal(&db_path) {
        eprintln!("Failed to seal genome database at rest: {e}");
    }
}

fn verify_mcp_auth(params: &Value, expected: &Option<String>) -> Result<(), String> {
    let Some(expected_token) = expected else {
        return Ok(());
    };
    if expected_token.is_empty() {
        return Ok(());
    }
    let provided = params
        .get("_meta")
        .and_then(|m| m.get("authToken").or_else(|| m.get("auth_token")))
        .and_then(|v| v.as_str())
        .or_else(|| params.get("authToken").and_then(|v| v.as_str()));
    match provided {
        Some(token) if token == expected_token => Ok(()),
        _ => Err(
            "MCP authentication failed: set params._meta.authToken to match --mcp-auth-token or GENOMICS_MCP_TOKEN"
                .into(),
        ),
    }
}

async fn handle_request(
    req: JsonRpcRequest,
    db_path: &PathBuf,
    allow_write: bool,
    auth_token: &Option<String>,
) -> JsonRpcResponse {
    let id = req.id;
    if req.jsonrpc != "2.0" {
        return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(json!({ "code": -32600, "message": "Invalid Request: jsonrpc must be '2.0'" })),
            id,
        };
    }
    if req.method != "ping"
        && let Err(message) = verify_mcp_auth(&req.params, auth_token)
    {
        return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(json!({ "code": -32001, "message": message })),
            id,
        };
    }
    match req.method.as_str() {
        "ping" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({})),
            error: None,
            id,
        },
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
        "tools/list" => {
            let mut result = json!({
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
                        "name": "get_app_bootstrap",
                        "description": "Returns startup database stats after migrations: sample counts, genotype totals, GWAS/evidence counts, and paths.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "get_research_job_status",
                        "description": "Returns the persisted vector research enrichment job for a sample (idle/running/paused/complete/error).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer", "description": "The target sample ID" }
                            },
                            "required": ["sample_id"]
                        }
                    },
                    {
                        "name": "get_discovered_findings_summary",
                        "description": "Lists persisted Research Agent discoveries for a sample (rsID, gene, genotype, interpretation status).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer", "description": "The target sample ID" }
                            },
                            "required": ["sample_id"]
                        }
                    },
                    {
                        "name": "search_vector_associations",
                        "description": "Hybrid Qdrant semantic search returning structured EvidenceCard JSON with provenance, directionality status, and quality flags.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer" },
                                "query": { "type": "string" },
                                "ollama_url": { "type": "string" },
                                "trait_category": { "type": "string" },
                                "limit": { "type": "integer" }
                            },
                            "required": ["sample_id", "query", "ollama_url"]
                        }
                    },
                    {
                        "name": "explain_vector_match",
                        "description": "Explain why a Qdrant vector hit matched a query (shared traits/genes, boost reasons, missing metadata warnings).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": { "type": "string" },
                                "hit_payload": { "type": "object" },
                                "vector_score": { "type": "number" }
                            },
                            "required": ["hit_payload"]
                        }
                    },
                    {
                        "name": "backfill_evidence_payloads",
                        "description": "Normalize existing Qdrant payloads to evidence schema v1 without re-embedding when structured text is unchanged.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer" },
                                "limit": { "type": "integer" }
                            },
                            "required": ["sample_id"]
                        }
                    },
                    {
                        "name": "get_variant_evidence_card",
                        "description": "Returns a structured EvidenceCard for one rsID from the Qdrant enrichment payload.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer" },
                                "rsid": { "type": "string" }
                            },
                            "required": ["sample_id", "rsid"]
                        }
                    },
                    {
                        "name": "get_similar_associations",
                        "description": "Find similar vector associations by rsID using Qdrant recommend-by-point.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer" },
                                "rsid": { "type": "string" },
                                "similarity_mode": { "type": "string" },
                                "limit": { "type": "integer" }
                            },
                            "required": ["sample_id", "rsid"]
                        }
                    },
                    {
                        "name": "get_quality_dashboard",
                        "description": "Evidence coverage dashboard: vectorized counts, association facts, directionality gaps, conflicts.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer" }
                            },
                            "required": ["sample_id"]
                        }
                    },
                    {
                        "name": "build_vector_atlas",
                        "description": "Project sample Qdrant embeddings to 2D atlas coordinates (UMAP-style) and cache in SQLite.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer" },
                                "limit": { "type": "integer" }
                            },
                            "required": ["sample_id"]
                        }
                    },
                    {
                        "name": "enable_named_vectors_collection",
                        "description": "Add trait_dense, gene_mechanism_dense, evidence_dense, actionability_dense named vectors to the Qdrant collection.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "ollama_url": { "type": "string" }
                            },
                            "required": ["ollama_url"]
                        }
                    },
                    {
                        "name": "get_trait_clusters",
                        "description": "Build trait/gene association clusters from SQLite association_facts.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer" },
                                "trait_category": { "type": "string" },
                                "limit": { "type": "integer" }
                            },
                            "required": ["sample_id"]
                        }
                    },
                    {
                        "name": "export_evidence_packet",
                        "description": "Export structured evidence packet for frontier model review (facts, cards, similar hits, prohibited claims).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sample_id": { "type": "integer" },
                                "rsid": { "type": "string" },
                                "cluster_id": { "type": "string" }
                            },
                            "required": ["sample_id"]
                        }
                    },
                    {
                        "name": "list_candidate_markers",
                        "description": "List dynamic candidate marker expansions surfaced during enrichment sweeps.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "limit": { "type": "integer" }
                            }
                        }
                    },
                    {
                        "name": "update_candidate_marker_status",
                        "description": "Update candidate marker status (cannot promote to curated_core via MCP).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "candidate_id": { "type": "string" },
                                "status": { "type": "string" },
                                "reviewer_note": { "type": "string" }
                            },
                            "required": ["candidate_id", "status"]
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
            });
            if !allow_write
                && let Some(tools) = result.get_mut("tools").and_then(|v| v.as_array_mut()) {
                    tools.retain(|t| {
                        t.get("name")
                            .and_then(|n| n.as_str())
                            .map(mcp_tool_visible_read_only)
                            .unwrap_or(true)
                    });
                }
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(result),
                error: None,
                id,
            }
        }
        "tools/call" => {
            let name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));
            
            match execute_tool(name, arguments, db_path, allow_write).await {
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

async fn execute_tool(name: &str, args: Value, db_path: &PathBuf, allow_write: bool) -> Result<Value, String> {
    if mcp_tool_requires_write(name) && !allow_write {
        return Err(format!(
            "Tool '{}' requires --mcp-write (mutating MCP operation)",
            name
        ));
    }

    match name {
        "search_evidence" => return mcp_search_evidence(db_path, args, allow_write).await,
        "scan_ollama_models" | "show_ollama_model" | "get_active_ollama_models" => {
            return mcp_ollama_tool(name, args).await;
        }
        _ => {}
    }

    let conn = crate::db::open_user_db(db_path).map_err(|e| format!("DB connection error: {}", e))?;

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
            crate::config::validate_template_json(template_json)?;
            
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

            for pack_id in &pack_ids {
                crate::config::validate_pack_id(pack_id)?;
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
            for s in rows.flatten() {
                sources.push(s);
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
                    "has_embedding": embedding.as_ref().is_some_and(|e| !e.trim().is_empty()),
                }))
            }).map_err(|e| e.to_string())?;
            let mut results = Vec::new();
            for r in rows {
                results.push(r.map_err(|e| e.to_string())?);
            }
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
            let session = crate::db::get_chat_session_by_id(&conn, session_id)
                .map_err(|e| format!("Failed to query session: {}", e))?
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
        "get_app_bootstrap" => {
            let data_dir = db_path.parent().ok_or("Could not resolve data directory")?;
            let status = crate::db::get_bootstrap_status(&conn, data_dir)?;
            Ok(serde_json::to_value(status).map_err(|e| format!("Serialization error: {}", e))?)
        }
        "get_research_job_status" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let job = crate::research::get_research_job_from_db(db_path, sample_id);
            Ok(json!(job))
        }
        "get_discovered_findings_summary" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let findings = crate::db::get_discovered_findings_summary(&conn, sample_id)?;
            Ok(json!(findings))
        }
        "search_vector_associations" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let query = args.get("query").and_then(|v| v.as_str()).ok_or("Missing query")?;
            let ollama_url = args.get("ollama_url").and_then(|v| v.as_str()).ok_or("Missing ollama_url")?;
            let _validated_ollama = crate::config::validate_service_url(ollama_url)?;
            let cfg = crate::config::load_qdrant_config(&conn)?;
            let params = crate::research::evidence::HybridSearchParams {
                sample_id,
                query: query.to_string(),
                trait_category: args.get("trait_category").and_then(|v| v.as_str()).map(String::from),
                evidence_tier: None,
                has_direction: None,
                min_data_quality: Some(0.0),
                min_wellness_actionability: None,
                limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as u32,
            };
            let cards = crate::research::evidence::search::search_associations_hybrid(
                &params,
                ollama_url,
                &cfg,
                Some(db_path),
            )
            .await?;
            Ok(json!(cards))
        }
        "explain_vector_match" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let hit_payload = args
                .get("hit_payload")
                .cloned()
                .ok_or("Missing hit_payload")?;
            let vector_score = args
                .get("vector_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            let explanation = crate::research::evidence::search::explain_vector_match(
                query,
                &hit_payload,
                vector_score,
            );
            Ok(json!(explanation))
        }
        "get_variant_evidence_card" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let rsid = args.get("rsid").and_then(|v| v.as_str()).ok_or("Missing rsid")?;
            let cfg = crate::config::load_qdrant_config(&conn)?;
            let payload = crate::research::find_point_payload_by_rsid(
                &cfg.url,
                cfg.api_key.as_deref(),
                &cfg.collection,
                sample_id,
                rsid,
            )
            .await?;
            Ok(json!(payload.map(|(p, pid)| {
                crate::research::evidence::card::evidence_card_from_payload(&p, 1.0, None, Some(pid))
            })))
        }
        "get_similar_associations" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let rsid = args.get("rsid").and_then(|v| v.as_str()).ok_or("Missing rsid")?;
            let cfg = crate::config::load_qdrant_config(&conn)?;
            let params = crate::research::evidence::SimilarSearchParams {
                sample_id,
                rsid: Some(rsid.to_string()),
                qdrant_point_id: None,
                similarity_mode: args
                    .get("similarity_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("evidence_similarity")
                    .to_string(),
                limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(8) as u32,
                include_self: false,
            };
            let cards = crate::research::evidence::search::get_similar_associations(&params, &cfg).await?;
            Ok(json!(cards))
        }
        "get_quality_dashboard" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let cfg = crate::config::load_qdrant_config(&conn)?;
            let status = crate::research::get_research_job_from_db(db_path, sample_id).map(|j| j.status);
            let dash = crate::research::evidence::dashboard::build_quality_dashboard(
                db_path,
                sample_id,
                &cfg,
                status,
            )
            .await?;
            Ok(json!(dash))
        }
        "get_trait_clusters" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let trait_category = args.get("trait_category").and_then(|v| v.as_str());
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as u32;
            let clusters = crate::research::evidence::dashboard::build_trait_clusters(
                &conn,
                sample_id,
                trait_category,
                Some(0.0),
                limit,
            )?;
            Ok(json!(clusters))
        }
        "export_evidence_packet" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let cfg = crate::config::load_qdrant_config(&conn)?;
            let packet = crate::research::evidence::packet::export_evidence_packet(
                db_path,
                sample_id,
                args.get("rsid").and_then(|v| v.as_str()),
                args.get("cluster_id").and_then(|v| v.as_str()),
                None,
                &cfg,
            )
            .await?;
            Ok(json!(packet))
        }
        "list_candidate_markers" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as u32;
            let rows = crate::research::evidence::store::list_candidates(&conn, limit)?;
            Ok(json!(rows))
        }
        "update_candidate_marker_status" => {
            let candidate_id = args.get("candidate_id").and_then(|v| v.as_str()).ok_or("Missing candidate_id")?;
            let status = args.get("status").and_then(|v| v.as_str()).ok_or("Missing status")?;
            if status == "curated_core" {
                return Err("curated_core promotion requires manual review".into());
            }
            let note = args.get("reviewer_note").and_then(|v| v.as_str());
            crate::research::evidence::store::update_candidate_status(&conn, candidate_id, status, note)?;
            Ok(json!({ "status": "updated", "candidate_id": candidate_id }))
        }
        "backfill_evidence_payloads" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(500) as u32;
            let cfg = crate::config::load_qdrant_config(&conn)?;
            let result = crate::research::evidence::backfill::backfill_evidence_payloads(
                db_path,
                sample_id,
                &cfg,
                limit,
            )
            .await?;
            Ok(json!(result))
        }
        "build_vector_atlas" => {
            let sample_id = args.get("sample_id").and_then(|v| v.as_i64()).ok_or("Missing sample_id")?;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(1500) as u32;
            let cfg = crate::config::load_qdrant_config(&conn)?;
            let result = crate::research::evidence::atlas::build_vector_atlas(
                db_path,
                sample_id,
                &cfg,
                limit,
            )
            .await?;
            Ok(json!(result))
        }
        "enable_named_vectors_collection" => {
            let ollama_url = args
                .get("ollama_url")
                .and_then(|v| v.as_str())
                .unwrap_or("http://127.0.0.1:11434");
            let cfg = crate::config::load_qdrant_config(&conn)?;
            let vector = crate::research::embed_text("dimension probe", ollama_url, &cfg.embedding_model).await?;
            let dims = vector.len() as u32;
            crate::research::ensure_qdrant_collection_named(
                &cfg.url,
                cfg.api_key.as_deref(),
                &cfg.collection,
                dims,
            )
            .await?;
            conn.execute(
                "UPDATE qdrant_config SET named_vectors_enabled = 1 WHERE id = 1",
                [],
            )
            .map_err(|e| e.to_string())?;
            Ok(json!({
                "status": "enabled",
                "collection": cfg.collection,
                "dims": dims,
                "vectors": ["trait_dense", "gene_mechanism_dense", "evidence_dense", "actionability_dense"]
            }))
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

type EvidenceHit = (String, String, String, String, Option<String>);

async fn mcp_search_evidence(
    db_path: &Path,
    args: Value,
    allow_write: bool,
) -> Result<Value, String> {
    let query = args.get("query").and_then(|v| v.as_str()).ok_or("Missing query")?;
    let ollama_url = args.get("ollama_url").and_then(|v| v.as_str());
    let ollama_token = args.get("ollama_token").and_then(|v| v.as_str());
    let query_clean = query.trim().to_string();
    if query_clean.is_empty() {
        return Ok(json!(Value::Null));
    }

    let keyword_hits: Vec<EvidenceHit> = crate::db_runtime::with_connection(db_path.to_path_buf(), {
        let query_clean = query_clean.clone();
        move |conn| {
            let search_pattern = crate::config::sql_like_contains_pattern(&query_clean);
            let mut stmt = conn
                .prepare(
                    "SELECT rsid, gene, evidence_text, source_citation, embedding
                     FROM evidence_library
                     WHERE rsid LIKE ? OR gene LIKE ? OR LOWER(evidence_text) LIKE ?",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![search_pattern, search_pattern, search_pattern], |row| {
                    let embedding: Option<String> = row.get(4)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        embedding,
                    ))
                })
                .map_err(|e| e.to_string())?;
            let mut hits = Vec::new();
            for r in rows {
                hits.push(r.map_err(|e| e.to_string())?);
            }
            Ok(hits)
        }
    })
    .await?;

    if let Some(url) = ollama_url
        && !url.trim().is_empty() {
            let clean_url = crate::config::validate_service_url(url)?;
            if let Some(embed_model) = super::get_embedding_model(&clean_url, ollama_token).await
                && let Ok(query_embedding) =
                    super::fetch_embedding(&clean_url, ollama_token, &embed_model, &query_clean).await
                {
                    let mut new_embeddings = Vec::new();
                    for (rsid, _gene, text, citation, embedding_opt) in keyword_hits.iter().take(10) {
                        if embedding_opt.as_ref().is_none_or(|e| e.trim().is_empty())
                            && let Ok(emb) =
                                super::fetch_embedding(&clean_url, ollama_token, &embed_model, text).await
                            {
                                new_embeddings.push((rsid.clone(), citation.clone(), emb));
                            }
                    }

                    if allow_write && !new_embeddings.is_empty() {
                        let writes = new_embeddings.clone();
                        crate::db_runtime::with_connection(db_path.to_path_buf(), move |conn| {
                            for (rsid, citation, emb) in &writes {
                                if let Ok(emb_json) = serde_json::to_string(emb) {
                                    let _ = conn.execute(
                                        "UPDATE evidence_library SET embedding = ? WHERE rsid = ? AND source_citation = ?",
                                        params![emb_json, rsid, citation],
                                    );
                                }
                            }
                            Ok(())
                        })
                        .await?;
                    }

                    let candidates = crate::db_runtime::with_connection(db_path.to_path_buf(), move |conn| {
                        let mut stmt = conn.prepare(
                            "SELECT rsid, gene, evidence_text, source_citation, embedding FROM evidence_library WHERE embedding IS NOT NULL AND embedding != ''",
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
                            if let Ok(emb) = serde_json::from_str::<Vec<f32>>(&emb_str)
                                && emb.len() == query_embedding.len() {
                                    let mut dot_product = 0.0f32;
                                    let mut norm_a = 0.0f32;
                                    let mut norm_b = 0.0f32;
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
                        candidates.sort_by(|a, b| {
                            let sim_a = a.get("similarity").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let sim_b = b.get("similarity").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            sim_b.partial_cmp(&sim_a).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        Ok(candidates
                            .into_iter()
                            .filter(|c| c.get("similarity").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.35)
                            .take(10)
                            .collect::<Vec<Value>>())
                    })
                    .await?;
                    return Ok(json!(candidates));
                }
        }

    let results: Vec<Value> = keyword_hits
        .into_iter()
        .map(|(rsid, gene, text, citation, emb)| {
            json!({
                "rsid": rsid,
                "gene": gene,
                "evidence_text": text,
                "source_citation": citation,
                "has_embedding": emb.as_ref().is_some_and(|e| !e.trim().is_empty()),
                "similarity": Value::Null,
            })
        })
        .collect();
    Ok(json!(results))
}

async fn mcp_ollama_tool(name: &str, args: Value) -> Result<Value, String> {
    let url = args.get("url").and_then(|v| v.as_str()).ok_or("Missing url")?;
    let token = args.get("token").and_then(|v| v.as_str());
    let clean_url = crate::config::validate_service_url(url)?;
    let client = reqwest::Client::new();

    match name {
        "scan_ollama_models" => {
            let mut req = client.get(format!("{}/api/tags", clean_url));
            if let Some(t) = token
                && !t.trim().is_empty() {
                    req = req.header(
                        "Authorization",
                        if t.to_lowercase().starts_with("bearer ") {
                            t.to_string()
                        } else {
                            format!("Bearer {}", t)
                        },
                    );
                }
            let res = req.send().await.map_err(|e| format!("Connection error: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Ollama tags API returned HTTP error: {}", res.status()));
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
            Ok(json!(tags.models.into_iter().map(|m| m.name).collect::<Vec<_>>()))
        }
        "show_ollama_model" => {
            let model_name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing name")?;
            let mut req = client.post(format!("{}/api/show", clean_url));
            if let Some(t) = token
                && !t.trim().is_empty() {
                    req = req.header(
                        "Authorization",
                        if t.to_lowercase().starts_with("bearer ") {
                            t.to_string()
                        } else {
                            format!("Bearer {}", t)
                        },
                    );
                }
            let res = req
                .json(&serde_json::json!({ "name": model_name }))
                .send()
                .await
                .map_err(|e| format!("Connection error: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Ollama show API returned HTTP error: {}", res.status()));
            }
            Ok(res.json().await.map_err(|e| format!("Failed to parse response: {}", e))?)
        }
        "get_active_ollama_models" => {
            let mut req = client.get(format!("{}/api/ps", clean_url));
            if let Some(t) = token
                && !t.trim().is_empty() {
                    req = req.header(
                        "Authorization",
                        if t.to_lowercase().starts_with("bearer ") {
                            t.to_string()
                        } else {
                            format!("Bearer {}", t)
                        },
                    );
                }
            let res = req.send().await.map_err(|e| format!("Connection error: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Ollama ps API returned HTTP error: {}", res.status()));
            }
            Ok(res.json().await.map_err(|e| format!("Failed to parse response: {}", e))?)
        }
        _ => Err(format!("Unknown Ollama tool: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_tools_include_build_vector_atlas() {
        assert!(mcp_tool_requires_write("build_vector_atlas"));
        assert!(!mcp_tool_requires_write("search_evidence"));
        assert!(mcp_tool_requires_write("delete_chat_session"));
    }

    #[test]
    fn read_only_mode_hides_all_write_tools() {
        for tool in MCP_WRITE_TOOLS {
            assert!(
                !mcp_tool_visible_read_only(tool),
                "{tool} should not appear in read-only tools/list"
            );
        }
        assert!(mcp_tool_visible_read_only("list_samples"));
        assert!(mcp_tool_visible_read_only("search_evidence"));
    }

    #[test]
    fn mcp_auth_optional_when_unconfigured() {
        assert!(verify_mcp_auth(&json!({}), &None).is_ok());
    }

    #[test]
    fn mcp_auth_rejects_missing_token_when_configured() {
        let token = Some("secret-token".to_string());
        assert!(verify_mcp_auth(&json!({}), &token).is_err());
    }

    #[test]
    fn mcp_auth_accepts_meta_token() {
        let token = Some("secret-token".to_string());
        let params = json!({ "_meta": { "authToken": "secret-token" } });
        assert!(verify_mcp_auth(&params, &token).is_ok());
    }
}
