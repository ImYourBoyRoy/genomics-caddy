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
    let conn = Connection::open(path)?;
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

    // Seed evidence library if needed
    let _ = seed_evidence_library(&conn);

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

pub fn seed_evidence_library(conn: &Connection) -> std::result::Result<(), String> {
    let manifest_str = include_str!("../../src/lib/marker-packs/manifest.json");
    let manifest: Manifest = serde_json::from_str(manifest_str)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    for pack_info in manifest.packs {
        let pack_str = match pack_info.id.as_str() {
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
        };

        if let Some(p_str) = pack_str {
            let pack: Pack = serde_json::from_str(p_str)
                .map_err(|e| format!("Failed to parse pack {}: {}", pack_info.id, e))?;

            for m in pack.markers {
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM evidence_library WHERE rsid = ?)",
                        params![m.rsid],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);

                if !exists {
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
                                let _ = conn.execute(
                                    "INSERT OR IGNORE INTO evidence_library (rsid, gene, evidence_text, source_citation) VALUES (?, ?, ?, ?)",
                                    params![m.rsid, m.gene, text, citation],
                                );
                            }
                            continue;
                        }
                    }

                    let citation = format!("Genomics Caddy Pack: {}", pack.name);
                    let text = format!(
                        "Gene: {} | Marker: {} | Impact: {} | Interpretation: {}",
                        m.gene, m.rsid, m.impact, m.interpretation
                    );
                    let _ = conn.execute(
                        "INSERT OR IGNORE INTO evidence_library (rsid, gene, evidence_text, source_citation) VALUES (?, ?, ?, ?)",
                        params![m.rsid, m.gene, text, citation],
                    );
                }
            }
        }
    }
    Ok(())
}
