// ./src-tauri/src/mcp.rs
/*
Module Docstring:
Purpose: Model Context Protocol (MCP) server implementation for genomic database interaction.
Responsibilities:
- Run a stdin/stdout JSON-RPC 2.0 loop when launched in --mcp mode.
- Expose tools: `list_samples`, `get_variants_by_rsid`, and `get_variants_in_region`.
- Safely query the local user genome SQLite database.
Key Inputs: Stdin JSON-RPC messages.
Key Outputs: Stdout JSON-RPC responses.
Operational Notes: Enables external LLMs to interact directly with the user's standardized genome.
*/

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

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
pub fn run_mcp_server(db_path: PathBuf) {
    let input = io::stdin();
    let mut reader = input.lock();
    let mut line = String::new();

    // Check if DB exists
    if !db_path.exists() {
        eprintln!("Database does not exist yet. Please import a genome first.");
        return;
    }

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
                    let res = handle_request(req, &db_path);
                    if let Ok(res_str) = serde_json::to_string(&res) {
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
}

fn handle_request(req: JsonRpcRequest, db_path: &PathBuf) -> JsonRpcResponse {
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
                    }
                ]
            })),
            error: None,
            id,
        },
        "tools/call" => {
            let name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));
            
            match execute_tool(name, arguments, db_path) {
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

fn execute_tool(name: &str, args: Value, db_path: &PathBuf) -> Result<Value, String> {
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
            let manifest: Value = serde_json::from_str(get_embedded_manifest())
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
                let manifest: Manifest = serde_json::from_str(get_embedded_manifest())
                    .map_err(|e| format!("Failed to parse manifest: {}", e))?;
                for pack in manifest.packs {
                    pack_ids.push(pack.id);
                }
            }

            // 2. Parse individual packs and construct a ReportTemplate
            let mut sections = Vec::new();
            for pack_id in &pack_ids {
                if let Some(pack_str) = get_embedded_pack(pack_id) {
                    #[derive(Debug, Deserialize)]
                    struct PackContent {
                        name: String,
                        markers: Vec<crate::report::MarkerDefinition>,
                    }
                    let pack: PackContent = serde_json::from_str(pack_str)
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
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

pub fn get_embedded_manifest() -> &'static str {
    include_str!("../../src/lib/marker-packs/manifest.json")
}

pub fn get_embedded_pack(pack_id: &str) -> Option<&'static str> {
    match pack_id {
        "core" => Some(include_str!("../../src/lib/marker-packs/core.json")),
        "pgx" => Some(include_str!("../../src/lib/marker-packs/pgx.json")),
        "metabolic" => Some(include_str!("../../src/lib/marker-packs/metabolic.json")),
        "nutrients" => Some(include_str!("../../src/lib/marker-packs/nutrients.json")),
        "neuropsych" => Some(include_str!("../../src/lib/marker-packs/neuropsych.json")),
        "sleep" => Some(include_str!("../../src/lib/marker-packs/sleep.json")),
        "connective_tissue" => Some(include_str!("../../src/lib/marker-packs/connective_tissue.json")),
        "thyroid_autoimmune" => Some(include_str!("../../src/lib/marker-packs/thyroid_autoimmune.json")),
        "cardiovascular" => Some(include_str!("../../src/lib/marker-packs/cardiovascular.json")),
        "cancer_confirmation_only" => Some(include_str!("../../src/lib/marker-packs/cancer_confirmation_only.json")),
        _ => None,
    }
}

