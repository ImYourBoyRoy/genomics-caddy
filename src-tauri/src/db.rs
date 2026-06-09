// ./src-tauri/src/db.rs
/*
Module Docstring:
Purpose: Database management (SQLite) for storing standard genomes and clinical reference data.
Responsibilities:
- Initialize SQLite tables for samples, genotypes, and variant annotations.
- Perform high-speed transactional batch insertions of parsed SNPs.
- Support multi-sample ingestion (AncestryDNA / 23andMe files).
- Query variants by rsID, chromosome range, or trait criteria.
Key Inputs: SQLite connection handles, SNP records, liftover engine references.
Key Outputs: Query results and transaction success status.
Operational Notes: Uses prepared statements and explicit transaction blocks for performance.
*/

use rusqlite::{params, Connection, Result};
use std::path::Path;
use crate::parser::SnpRecord;
use crate::liftover::LiftoverEngine;

#[derive(Debug, serde::Serialize)]
pub struct SampleInfo {
    pub id: i64,
    pub name: String,
    pub genetic_sex: String,
    pub imported_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct DbSnpRecord {
    pub sample_id: i64,
    pub rsid: String,
    pub chromosome: String,
    pub position_grch37: u64,
    pub position_grch38: Option<u64>,
    pub allele1: String,
    pub allele2: String,
}

/// Initializes the user genome database schema.
pub fn init_user_db<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let conn = Connection::open(path.as_ref())?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    // Create samples table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS samples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            genetic_sex TEXT DEFAULT 'Unknown',
            imported_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Add column if it's an older database schema (migration helper)
    let _ = conn.execute("ALTER TABLE samples ADD COLUMN genetic_sex TEXT DEFAULT 'Unknown'", []);

    // Create genotypes table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS genotypes (
            sample_id INTEGER,
            rsid TEXT NOT NULL,
            chromosome TEXT NOT NULL,
            position_grch37 INTEGER NOT NULL,
            position_grch38 INTEGER,
            allele1 TEXT NOT NULL,
            allele2 TEXT NOT NULL,
            PRIMARY KEY (sample_id, rsid),
            FOREIGN KEY(sample_id) REFERENCES samples(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes for fast lookup
    conn.execute("CREATE INDEX IF NOT EXISTS idx_genotypes_rsid ON genotypes(rsid)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_genotypes_coords ON genotypes(chromosome, position_grch38)", [])?;

    // Create evidence_library table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS evidence_library (
            rsid TEXT NOT NULL,
            gene TEXT NOT NULL,
            evidence_text TEXT NOT NULL,
            source_citation TEXT NOT NULL,
            embedding TEXT,
            PRIMARY KEY (rsid, source_citation)
        )",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_evidence_rsid ON evidence_library(rsid)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_evidence_gene ON evidence_library(gene)", [])?;

    // Create chat_sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_sessions (
            id TEXT PRIMARY KEY,
            sample_id INTEGER,
            title TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            selected_packs TEXT NOT NULL,
            only_active_findings INTEGER NOT NULL,
            temperature REAL NOT NULL,
            selected_model TEXT NOT NULL,
            max_tokens INTEGER,
            extended_thinking INTEGER,
            consultation_mode TEXT,
            FOREIGN KEY(sample_id) REFERENCES samples(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create chat_messages table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            images TEXT,
            safety_review TEXT,
            FOREIGN KEY(session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id)", [])?;

    // Seed evidence library if needed
    let app_data_dir = path.as_ref().parent();
    let _ = seed_evidence_library(&conn, app_data_dir);

    Ok(conn)
}

/// Imports raw parsed genomic records into the user database, performing liftover in the process.
pub fn import_raw_genome<F: Fn(u32, &str)>(
    conn: &mut Connection,
    sample_name: &str,
    records: &[SnpRecord],
    liftover_engine: Option<&LiftoverEngine>,
    progress_callback: F,
) -> Result<i64, String> {
    // 1. Create/Retrieve sample ID
    conn.execute(
        "INSERT OR IGNORE INTO samples (name) VALUES (?)",
        params![sample_name],
    )
    .map_err(|e| format!("Failed to create sample: {}", e))?;

    let sample_id: i64 = conn
        .query_row(
            "SELECT id FROM samples WHERE name = ?",
            params![sample_name],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to retrieve sample ID: {}", e))?;

    // Determine genetic sex from Y chromosome density
    let mut y_call_count = 0;
    for record in records {
        if record.chromosome == "Y" {
            let genotype = format!("{}{}", record.allele1.trim(), record.allele2.trim());
            if genotype != "--" && genotype != "-" && genotype != "00" && !genotype.is_empty() {
                y_call_count += 1;
            }
        }
    }
    let genetic_sex = if y_call_count > 20 {
        "XY (Male)".to_string()
    } else {
        "XX (Female)".to_string()
    };

    conn.execute(
        "UPDATE samples SET genetic_sex = ? WHERE id = ?",
        params![genetic_sex, sample_id],
    )
    .map_err(|e| format!("Failed to update genetic sex: {}", e))?;

    // 2. Perform bulk insertion using a transaction
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    {
        let total = records.len();
        let mut stmt = tx
            .prepare(
                "INSERT OR REPLACE INTO genotypes 
                (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2) 
                VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        for (i, record) in records.iter().enumerate() {
            if i % 50_000 == 0 && i > 0 {
                let percent = 50 + ((i as f32 / total as f32) * 45.0) as u32;
                progress_callback(percent, &format!("Liftover & Ingesting SNPs: {}/{}...", i, total));
            }

            let pos_grch38 = liftover_engine.and_then(|engine| {
                engine.liftover(&record.chromosome, record.position)
            });

            stmt.execute(params![
                sample_id,
                record.rsid,
                record.chromosome,
                record.position as i64,
                pos_grch38.map(|p| p as i64),
                record.allele1,
                record.allele2,
            ])
            .map_err(|e| format!("Failed to insert record {}: {}", record.rsid, e))?;
        }
    }

    progress_callback(95, "Committing database transaction...");
    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    progress_callback(100, "Genotypes successfully imported.");
    Ok(sample_id)
}

/// Retrieves list of all imported samples, auto-repairing genetic sex determination if "Unknown".
pub fn get_samples(conn: &Connection) -> Result<Vec<SampleInfo>> {
    let mut stmt = conn.prepare("SELECT id, name, genetic_sex, datetime(imported_at, 'localtime') FROM samples ORDER BY id DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut list = Vec::new();
    for row_res in rows {
        let (id, name, mut genetic_sex, imported_at) = row_res?;
        if genetic_sex == "Unknown" {
            // Count non-missing Y chromosome genotypes for this sample
            let y_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM genotypes 
                 WHERE sample_id = ? AND chromosome = 'Y' 
                 AND allele1 != '-' AND allele1 != '0' AND allele1 != '?' AND allele1 != ''",
                params![id],
                |r| r.get(0),
            ).unwrap_or(0);
            
            let resolved_sex = if y_count > 20 {
                "XY (Male)".to_string()
            } else {
                "XX (Female)".to_string()
            };
            
            // Persist back to the samples table
            let _ = conn.execute(
                "UPDATE samples SET genetic_sex = ? WHERE id = ?",
                params![resolved_sex, id],
            );
            
            genetic_sex = resolved_sex;
        }

        list.push(SampleInfo {
            id,
            name,
            genetic_sex,
            imported_at,
        });
    }
    Ok(list)
}

/// Queries specific variants by rsID.
pub fn query_by_rsids(conn: &Connection, sample_id: i64, rsids: &[String]) -> Result<Vec<DbSnpRecord>> {
    let mut stmt = conn.prepare(
        "SELECT sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2 
         FROM genotypes WHERE sample_id = ? AND rsid = ?",
    )?;

    let mut results = Vec::new();
    for rsid in rsids {
        let mut rows = stmt.query(params![sample_id, rsid])?;
        while let Some(row) = rows.next()? {
            results.push(DbSnpRecord {
                sample_id: row.get(0)?,
                rsid: row.get(1)?,
                chromosome: row.get(2)?,
                position_grch37: row.get::<_, i64>(3)? as u64,
                position_grch38: row.get::<_, Option<i64>>(4)?.map(|p| p as u64),
                allele1: row.get(5)?,
                allele2: row.get(6)?,
            });
        }
    }
    Ok(results)
}

/// Queries variants in a chromosome region (GRCh38 coordinates).
pub fn query_region(
    conn: &Connection,
    sample_id: i64,
    chromosome: &str,
    start: u64,
    end: u64,
) -> Result<Vec<DbSnpRecord>> {
    let mut stmt = conn.prepare(
        "SELECT sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2 
         FROM genotypes 
         WHERE sample_id = ? AND chromosome = ? AND position_grch38 >= ? AND position_grch38 <= ?
         ORDER BY position_grch38 ASC",
    )?;

    let rows = stmt.query_map(params![sample_id, chromosome, start as i64, end as i64], |row| {
        Ok(DbSnpRecord {
            sample_id: row.get(0)?,
            rsid: row.get(1)?,
            chromosome: row.get(2)?,
            position_grch37: row.get::<_, i64>(3)? as u64,
            position_grch38: row.get::<_, Option<i64>>(4)?.map(|p| p as u64),
            allele1: row.get(5)?,
            allele2: row.get(6)?,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Deletes a sample and its genotypes from the database.
pub fn delete_sample(conn: &Connection, sample_id: i64) -> Result<()> {
    conn.execute("DELETE FROM samples WHERE id = ?", params![sample_id])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Predefined Evidence Seeding (RAG)
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct ManifestPack {
    id: String,
}

#[derive(Debug, serde::Deserialize)]
struct Manifest {
    packs: Vec<ManifestPack>,
}

#[derive(Debug, serde::Deserialize)]
struct PackSource {
    name: String,
    url: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct PackMarker {
    rsid: String,
    gene: String,
    interpretation: String,
    impact: String,
    sources: Option<Vec<PackSource>>,
}

#[derive(Debug, serde::Deserialize)]
struct Pack {
    name: String,
    markers: Vec<PackMarker>,
}

pub(crate) fn get_manifest_str(app_data_dir: Option<&Path>) -> String {
    if let Some(dir) = app_data_dir {
        let manifest_path = dir.join("marker-packs").join("manifest.json");
        if manifest_path.exists() {
            if let Ok(s) = std::fs::read_to_string(manifest_path) {
                return s;
            }
        }
    }
    include_str!("../../src/lib/marker-packs/manifest.json").to_string()
}

pub(crate) fn get_pack_str(app_data_dir: Option<&Path>, pack_id: &str) -> Option<String> {
    if let Some(dir) = app_data_dir {
        let pack_path = dir.join("marker-packs").join(format!("{}.json", pack_id));
        if pack_path.exists() {
            if let Ok(s) = std::fs::read_to_string(pack_path) {
                return Some(s);
            }
        }
    }
    match pack_id {
        "core" => Some(include_str!("../../src/lib/marker-packs/core.json").to_string()),
        "pgx" => Some(include_str!("../../src/lib/marker-packs/pgx.json").to_string()),
        "metabolic" => Some(include_str!("../../src/lib/marker-packs/metabolic.json").to_string()),
        "nutrients" => Some(include_str!("../../src/lib/marker-packs/nutrients.json").to_string()),
        "neuropsych" => Some(include_str!("../../src/lib/marker-packs/neuropsych.json").to_string()),
        "sleep" => Some(include_str!("../../src/lib/marker-packs/sleep.json").to_string()),
        "connective_tissue" => Some(include_str!("../../src/lib/marker-packs/connective_tissue.json").to_string()),
        "thyroid_autoimmune" => Some(include_str!("../../src/lib/marker-packs/thyroid_autoimmune.json").to_string()),
        "cardiovascular" => Some(include_str!("../../src/lib/marker-packs/cardiovascular.json").to_string()),
        "cancer_confirmation_only" => Some(include_str!("../../src/lib/marker-packs/cancer_confirmation_only.json").to_string()),
        _ => None,
    }
}

pub fn seed_evidence_library(conn: &Connection, app_data_dir: Option<&Path>) -> std::result::Result<(), String> {
    let manifest_str = get_manifest_str(app_data_dir);
    let manifest: Manifest = serde_json::from_str(&manifest_str)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    for pack_info in manifest.packs {
        let pack_str = get_pack_str(app_data_dir, &pack_info.id);

        if let Some(p_str) = pack_str {
            let pack: Pack = serde_json::from_str(&p_str)
                .map_err(|e| format!("Failed to parse pack {}: {}", pack_info.id, e))?;

            for m in pack.markers {
                if let Some(ref sources) = m.sources {
                    if !sources.is_empty() {
                        for src in sources {
                            let citation = format!(
                                "{} ({})",
                                src.name,
                                src.url.as_deref().unwrap_or("No URL")
                            );
                            let text = format!(
                                "Gene: {} | Marker: {} | Impact: {} | Interpretation: {} | Source Notes: {}",
                                m.gene,
                                m.rsid,
                                m.impact,
                                m.interpretation,
                                src.notes.as_deref().unwrap_or("N/A")
                            );

                            // Query existing evidence text by exact key
                            let existing_text: Option<String> = conn.query_row(
                                "SELECT evidence_text FROM evidence_library WHERE rsid = ? AND source_citation = ?",
                                params![m.rsid, citation],
                                |row| row.get(0),
                            ).ok();

                            match existing_text {
                                None => {
                                    // Not found, insert new
                                    let _ = conn.execute(
                                        "INSERT INTO evidence_library (rsid, gene, evidence_text, source_citation) VALUES (?, ?, ?, ?)",
                                        params![m.rsid, m.gene, text, citation],
                                    );
                                }
                                Some(old_text) if old_text != text => {
                                    // Text updated, reset embedding to force re-vectorization
                                    let _ = conn.execute(
                                        "UPDATE evidence_library SET evidence_text = ?, embedding = NULL WHERE rsid = ? AND source_citation = ?",
                                        params![text, m.rsid, citation],
                                    );
                                }
                                _ => {} // Identical, skip
                            }
                        }
                        continue;
                    }
                }

                // Default fallback source when no references are provided in the pack
                let citation = format!("Genomics Caddy Pack: {}", pack.name);
                let text = format!(
                    "Gene: {} | Marker: {} | Impact: {} | Interpretation: {}",
                    m.gene, m.rsid, m.impact, m.interpretation
                );
                
                let existing_text: Option<String> = conn.query_row(
                    "SELECT evidence_text FROM evidence_library WHERE rsid = ? AND source_citation = ?",
                    params![m.rsid, citation],
                    |row| row.get(0),
                ).ok();

                match existing_text {
                    None => {
                        let _ = conn.execute(
                            "INSERT INTO evidence_library (rsid, gene, evidence_text, source_citation) VALUES (?, ?, ?, ?)",
                            params![m.rsid, m.gene, text, citation],
                        );
                    }
                    Some(old_text) if old_text != text => {
                        let _ = conn.execute(
                            "UPDATE evidence_library SET evidence_text = ?, embedding = NULL WHERE rsid = ? AND source_citation = ?",
                            params![text, m.rsid, citation],
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SQLite Chat History Serialization & Persistence
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub images: Option<Vec<String>>,
    #[serde(rename = "safetyReview")]
    pub safety_review: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct DbChatSession {
    pub id: String,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub timestamp: i64,
    #[serde(rename = "sampleId")]
    pub sample_id: Option<i64>,
    #[serde(rename = "selectedPacks")]
    pub selected_packs: serde_json::Value,
    #[serde(rename = "onlyActiveFindings")]
    pub only_active_findings: bool,
    pub temperature: f64,
    #[serde(rename = "selectedModel")]
    pub selected_model: String,
    #[serde(rename = "maxTokens")]
    pub max_tokens: Option<i32>,
    #[serde(rename = "extendedThinking")]
    pub extended_thinking: Option<bool>,
    #[serde(rename = "consultationMode")]
    pub consultation_mode: Option<String>,
}

pub fn get_chat_sessions(conn: &Connection, sample_id_filter: Option<i64>) -> Result<Vec<DbChatSession>> {
    let mut stmt = if sample_id_filter.is_some() {
        conn.prepare(
            "SELECT id, sample_id, title, timestamp, selected_packs, only_active_findings, 
                    temperature, selected_model, max_tokens, extended_thinking, consultation_mode 
             FROM chat_sessions WHERE sample_id = ? ORDER BY timestamp DESC"
        )?
    } else {
        conn.prepare(
            "SELECT id, sample_id, title, timestamp, selected_packs, only_active_findings, 
                    temperature, selected_model, max_tokens, extended_thinking, consultation_mode 
             FROM chat_sessions ORDER BY timestamp DESC"
        )?
    };

    let mapper = |row: &rusqlite::Row<'_>| {
        let id: String = row.get(0)?;
        let sample_id: Option<i64> = row.get(1)?;
        let title: String = row.get(2)?;
        let timestamp: i64 = row.get(3)?;
        let selected_packs_str: String = row.get(4)?;
        let only_active_findings_int: i32 = row.get(5)?;
        let temperature: f64 = row.get(6)?;
        let selected_model: String = row.get(7)?;
        let max_tokens: Option<i32> = row.get(8)?;
        let extended_thinking_int: Option<i32> = row.get(9)?;
        let consultation_mode: Option<String> = row.get(10)?;

        let selected_packs = serde_json::from_str(&selected_packs_str).unwrap_or(serde_json::Value::Null);
        let only_active_findings = only_active_findings_int != 0;
        let extended_thinking = extended_thinking_int.map(|v| v != 0);

        Ok((id, sample_id, title, timestamp, selected_packs, only_active_findings, temperature, selected_model, max_tokens, extended_thinking, consultation_mode))
    };

    let rows = if let Some(sample_id) = sample_id_filter {
        stmt.query_map(params![sample_id], mapper)?
    } else {
        stmt.query_map([], mapper)?
    };

    let mut sessions = Vec::new();
    for r in rows {
        let (id, sample_id, title, timestamp, selected_packs, only_active_findings, temperature, selected_model, max_tokens, extended_thinking, consultation_mode) = r?;
        
        // Load messages for this session
        let mut msg_stmt = conn.prepare(
            "SELECT role, content, images, safety_review FROM chat_messages WHERE session_id = ? ORDER BY id ASC"
        )?;
        let msg_rows = msg_stmt.query_map(params![id], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            let images_str: Option<String> = row.get(2)?;
            let safety_review: Option<String> = row.get(3)?;

            let images = images_str.and_then(|s| serde_json::from_str(&s).ok());

            Ok(ChatMessage {
                role,
                content,
                images,
                safety_review,
            })
        })?;

        let mut messages = Vec::new();
        for mr in msg_rows {
            messages.push(mr?);
        }

        sessions.push(DbChatSession {
            id,
            title,
            messages,
            timestamp,
            sample_id,
            selected_packs,
            only_active_findings,
            temperature,
            selected_model,
            max_tokens,
            extended_thinking,
            consultation_mode,
        });
    }

    Ok(sessions)
}

pub fn save_chat_session(conn: &mut Connection, session: &DbChatSession) -> Result<()> {
    let tx = conn.transaction()?;

    let selected_packs_str = serde_json::to_string(&session.selected_packs).unwrap_or_else(|_| "{}".to_string());
    let only_active_findings_int = if session.only_active_findings { 1 } else { 0 };
    let extended_thinking_int = session.extended_thinking.map(|v| if v { 1 } else { 0 });

    tx.execute(
        "INSERT OR REPLACE INTO chat_sessions (id, sample_id, title, timestamp, selected_packs, 
                                               only_active_findings, temperature, selected_model, 
                                               max_tokens, extended_thinking, consultation_mode) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            session.id,
            session.sample_id,
            session.title,
            session.timestamp,
            selected_packs_str,
            only_active_findings_int,
            session.temperature,
            session.selected_model,
            session.max_tokens,
            extended_thinking_int,
            session.consultation_mode,
        ],
    )?;

    // Delete existing messages to prevent duplication
    tx.execute("DELETE FROM chat_messages WHERE session_id = ?", params![session.id])?;

    // Insert new messages
    let mut stmt = tx.prepare(
        "INSERT INTO chat_messages (session_id, role, content, images, safety_review) 
         VALUES (?, ?, ?, ?, ?)"
    )?;

    for msg in &session.messages {
        let images_str = msg.images.as_ref().map(|imgs| serde_json::to_string(imgs).unwrap_or_else(|_| "[]".to_string()));
        stmt.execute(params![
            session.id,
            msg.role,
            msg.content,
            images_str,
            msg.safety_review,
        ])?;
    }

    stmt.finalize()?;
    tx.commit()?;
    Ok(())
}

pub fn delete_chat_session(conn: &Connection, session_id: &str) -> Result<()> {
    conn.execute("DELETE FROM chat_sessions WHERE id = ?", params![session_id])?;
    Ok(())
}
