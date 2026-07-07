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

const DB_BOOTSTRAP_KEY: &str = "db_bootstrapped_v1";

#[derive(Debug, serde::Serialize, Clone)]
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

pub fn connect<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let conn = crate::db_crypto::open_encrypted(path.as_ref())?;
    conn.execute("PRAGMA foreign_keys = ON;", [])?;
    
    let ref_db_path = path.as_ref().parent()
        .map(|p| p.join("genomics_reference.db"))
        .unwrap_or_else(|| std::path::PathBuf::from("genomics_reference.db"));
        
    let attach_query = format!(
        "ATTACH DATABASE '{}' AS reference",
        ref_db_path.to_string_lossy().replace('\\', "/")
    );
    conn.execute(&attach_query, [])?;

    // SQLite does not support persistent cross-database VIEWs, but it fully supports
    // TEMP VIEWs referencing attached databases. Creating temporary views allows unqualified
    // queries like `SELECT * FROM gwas_reference` to transparently route to the attached
    // database `reference.gwas_reference`. Note: these are read-only (writes must use
    // the explicit `reference.` schema prefix).
    let reference_views: &[(&str, &str)] = &[
        ("gwas_reference",              "SELECT * FROM reference.gwas_reference"),
        ("clinvar_reference",           "SELECT * FROM reference.clinvar_reference"),
        ("api_cache",                   "SELECT * FROM reference.api_cache"),
        ("api_cache_entries",           "SELECT * FROM reference.api_cache_entries"),
        ("source_records",              "SELECT * FROM reference.source_records"),
        ("gnomad_variant_cache",        "SELECT * FROM reference.gnomad_variant_cache"),
        ("gnomad_config",               "SELECT * FROM reference.gnomad_config"),
        ("offline_asset_registry",      "SELECT * FROM reference.offline_asset_registry"),
        ("pharmgkb_clinical_variants",  "SELECT * FROM reference.pharmgkb_clinical_variants"),
        ("pharmgkb_genes",              "SELECT * FROM reference.pharmgkb_genes"),
        ("clingen_gene_validity",       "SELECT * FROM reference.clingen_gene_validity"),
        ("mane_transcripts",            "SELECT * FROM reference.mane_transcripts"),
        ("rsid_aliases",                "SELECT * FROM reference.rsid_aliases"),
        ("evidence_library",            "SELECT * FROM reference.evidence_library"),
    ];
    for (view_name, select_sql) in reference_views {
        let ddl = format!("CREATE TEMP VIEW IF NOT EXISTS {view_name} AS {select_sql}");
        if let Err(e) = conn.execute(&ddl, []) {
            eprintln!("Warning: could not create temp view {view_name} on connect: {e}");
        }
    }
 
    Ok(conn)
}

thread_local! {
    static CACHED_CONN: std::cell::RefCell<Option<(std::path::PathBuf, Connection)>> = const { std::cell::RefCell::new(None) };
}

/// Run a closure with a thread-cached SQLite connection to avoid expensive open/close and lock overhead in tight loops.
pub fn with_cached_conn<F, R, P: AsRef<Path>>(path: P, f: F) -> Result<R>
where
    F: FnOnce(&Connection) -> Result<R>,
{
    let path_ref = path.as_ref();
    CACHED_CONN.with(|cell| {
        let mut guard = cell.borrow_mut();
        if let Some((ref cached_path, ref conn)) = *guard
            && cached_path == path_ref
        {
            return f(conn);
        }
        let conn = connect(path_ref)?;
        let res = f(&conn);
        *guard = Some((path_ref.to_path_buf(), conn));
        res
    })
}

/// Clear the thread-local cached connection, dropping the SQLite connection handle.
/// This must be called before sealing or modifying the database file directly.
pub fn clear_cached_conn() {
    CACHED_CONN.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Seal the database file (encrypt at rest). Call on graceful shutdown.
pub fn seal<P: AsRef<Path>>(path: P) -> Result<(), String> {
    clear_cached_conn();
    let path = path.as_ref();
    if path.is_file() {
        // Checkpoint WAL into main file before sealing.
        if let Ok(conn) = Connection::open(path) {
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
    }
    crate::db_crypto::seal_encrypted(path)
}

fn table_exists_in_main(conn: &Connection, table_name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?",
        params![table_name],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

fn migrate_to_reference_db(conn: &Connection) -> Result<()> {
    let tables_to_migrate = [
        "api_cache",
        "clinvar_reference",
        "gwas_reference",
        "source_records",
        "api_cache_entries",
        "gnomad_variant_cache",
        "gnomad_config",
        "offline_asset_registry",
        "pharmgkb_clinical_variants",
        "pharmgkb_genes",
        "clingen_gene_validity",
        "mane_transcripts",
        "rsid_aliases",
        "evidence_library",
    ];

    let mut migrated_any = false;
    for t in &tables_to_migrate {
        if table_exists_in_main(conn, t) {
            println!("Migrating table {} to genomics_reference.db...", t);
            let copy_sql = format!("INSERT OR IGNORE INTO reference.{} SELECT * FROM main.{}", t, t);
            if let Err(e) = conn.execute(&copy_sql, []) {
                eprintln!("Failed to copy table {}: {}", t, e);
                return Err(e);
            }
            let drop_sql = format!("DROP TABLE main.{}", t);
            if let Err(e) = conn.execute(&drop_sql, []) {
                eprintln!("Failed to drop table {}: {}", t, e);
                return Err(e);
            }
            migrated_any = true;
        }
    }

    if migrated_any {
        println!("Vacuuming user_genome.db to reclaim disk space...");
        let _ = conn.execute("VACUUM", []);
    }

    Ok(())
}

pub fn open_user_db<P: AsRef<Path>>(path: P) -> Result<Connection> {
    open_user_db_with_progress(path, None)
}

pub fn open_user_db_with_progress<P: AsRef<Path>>(
    path: P,
    app: Option<&tauri::AppHandle>,
) -> Result<Connection> {
    let emit_progress = |msg: &str| {
        if let Some(handle) = app {
            use tauri::Emitter;
            let _ = handle.emit("bootstrap-progress", msg.to_string());
        }
    };

    emit_progress("Connecting to local database...");
    let conn = connect(path.as_ref())?;

    emit_progress("Applying schema migrations...");
    ensure_schema(&conn)?;

    emit_progress("Migrating reference schema mappings...");
    migrate_to_reference_db(&conn)?;

    if let Err(e) = crate::config::sync_qdrant_sqlite_from_env(&conn) {
        eprintln!("Could not sync Qdrant defaults from environment: {}", e);
    }

    let app_data_dir = path.as_ref().parent();
    if !is_db_bootstrapped(&conn)? {
        run_heavy_bootstrap_with_progress(&conn, app_data_dir, emit_progress)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        mark_db_bootstrapped(&conn)?;
    }

    Ok(conn)
}

/// Backward-compatible alias for `open_user_db`.
pub fn init_user_db<P: AsRef<Path>>(path: P) -> Result<Connection> {
    open_user_db(path)
}

fn ensure_app_metadata(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn is_db_bootstrapped(conn: &Connection) -> Result<bool> {
    ensure_app_metadata(conn)?;
    let value: Result<String, _> = conn.query_row(
        "SELECT value FROM app_metadata WHERE key = ?",
        params![DB_BOOTSTRAP_KEY],
        |row| row.get(0),
    );
    match value {
        Ok(v) => Ok(v == "1"),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e),
    }
}

fn mark_db_bootstrapped(conn: &Connection) -> Result<()> {
    ensure_app_metadata(conn)?;
    conn.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES (?, '1')",
        params![DB_BOOTSTRAP_KEY],
    )?;
    Ok(())
}

fn run_heavy_bootstrap_with_progress<F: Fn(&str)>(
    conn: &Connection,
    app_data_dir: Option<&Path>,
    progress: F,
) -> Result<(), String> {
    seed_evidence_library_with_progress(conn, app_data_dir, &progress)?;

    progress("Syncing GWAS Catalog references...");
    crate::research::references::seed_gwas_reference_fallback(conn, app_data_dir)
        .map_err(|e| e.to_string())?;

    progress("Normalizing rsIDs and optimizing database...");
    crate::research::references::normalize_gwas_reference_rsids_on_startup(conn)
        .map_err(|e| e.to_string())?;
    Ok(())
}



fn ensure_schema(conn: &Connection) -> Result<()> {
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
        "CREATE TABLE IF NOT EXISTS reference.evidence_library (
            rsid TEXT NOT NULL,
            gene TEXT NOT NULL,
            evidence_text TEXT NOT NULL,
            source_citation TEXT NOT NULL,
            embedding TEXT,
            PRIMARY KEY (rsid, source_citation)
        )",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS reference.idx_evidence_rsid ON evidence_library(rsid)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS reference.idx_evidence_gene ON evidence_library(gene)", [])?;

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

    // Qdrant / research settings (secrets live in OS keyring — columns kept for legacy migration)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS qdrant_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            url TEXT NOT NULL DEFAULT 'http://localhost:6333',
            api_key TEXT,
            collection TEXT NOT NULL DEFAULT 'genomics_evidence',
            embedding_model TEXT NOT NULL DEFAULT 'mxbai-embed-large',
            gwas_strict INTEGER NOT NULL DEFAULT 1,
            ncbi_api_key TEXT,
            auto_start INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )?;
    let default_url = std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());
    let default_collection = std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "genomics_evidence".to_string());
    let default_model = std::env::var("OLLAMA_EMBED_MODEL").unwrap_or_else(|_| "mxbai-embed-large".to_string());
    conn.execute(
        "INSERT OR IGNORE INTO qdrant_config (id, url, collection, embedding_model) VALUES (1, ?, ?, ?)",
        rusqlite::params![default_url, default_collection, default_model],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS research_jobs (
            job_id TEXT PRIMARY KEY,
            sample_id INTEGER NOT NULL,
            status TEXT NOT NULL,
            total_markers INTEGER NOT NULL DEFAULT 0,
            enriched_count INTEGER NOT NULL DEFAULT 0,
            priority_complete INTEGER NOT NULL DEFAULT 0,
            current_rsid TEXT,
            current_source TEXT,
            started_at INTEGER NOT NULL,
            last_updated INTEGER NOT NULL,
            error_message TEXT,
            FOREIGN KEY(sample_id) REFERENCES samples(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_research_jobs_sample ON research_jobs(sample_id, started_at DESC)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS reference.api_cache (
            url TEXT PRIMARY KEY,
            response_json TEXT NOT NULL,
            fetched_at INTEGER NOT NULL
        )",
        [],
    )?;

    // Drop clinvar_reference if it's the old schema (lacking gene_symbol) or has old single PK
    let has_new_schema = conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='clinvar_reference' AND sql LIKE '%gene_symbol%'",
        [],
        |_| Ok(true)
    ).unwrap_or(false);

    let has_old_pk = conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='clinvar_reference' AND sql LIKE '%PRIMARY KEY (rsid)%'",
        [],
        |_| Ok(true)
    ).unwrap_or(false);

    if !has_new_schema || has_old_pk {
        let _ = conn.execute("DROP TABLE IF EXISTS reference.clinvar_reference", []);
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS reference.clinvar_reference (
            rsid TEXT NOT NULL,
            allele_id TEXT NOT NULL DEFAULT '',
            variation_id TEXT NOT NULL DEFAULT '',
            type TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL DEFAULT '',
            gene_symbol TEXT NOT NULL DEFAULT '',
            clinical_significance TEXT NOT NULL DEFAULT '',
            clin_sig_simple TEXT NOT NULL DEFAULT '',
            last_evaluated TEXT NOT NULL DEFAULT '',
            rcv_accession TEXT NOT NULL DEFAULT '',
            phenotype_ids TEXT NOT NULL DEFAULT '',
            phenotype_list TEXT NOT NULL DEFAULT '',
            origin_simple TEXT NOT NULL DEFAULT '',
            assembly TEXT NOT NULL DEFAULT '',
            chromosome TEXT NOT NULL DEFAULT '',
            start INTEGER,
            stop INTEGER,
            review_status TEXT NOT NULL DEFAULT '',
            number_submitters INTEGER,
            position_vcf INTEGER,
            reference_allele_vcf TEXT NOT NULL DEFAULT '',
            alternate_allele_vcf TEXT NOT NULL DEFAULT '',
            scvs_for_aggregate_germline_classification TEXT NOT NULL DEFAULT '',
            -- Backward compatibility aliases:
            gene TEXT NOT NULL DEFAULT '',
            conditions TEXT NOT NULL DEFAULT '',
            relevant_allele TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (rsid, assembly, gene_symbol, variation_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS reference.gwas_reference (
            rsid TEXT PRIMARY KEY,
            association_count INTEGER NOT NULL DEFAULT 1,
            top_trait TEXT NOT NULL DEFAULT '',
            primary_gene TEXT NOT NULL DEFAULT '',
            mapped_genes TEXT NOT NULL DEFAULT '',
            reported_genes TEXT NOT NULL DEFAULT '',
            best_pvalue REAL,
            associations_json TEXT NOT NULL DEFAULT '[]'
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS reference.idx_gwas_reference_rsid ON gwas_reference(rsid)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS discovered_findings (
            sample_id INTEGER NOT NULL,
            rsid TEXT NOT NULL,
            gene TEXT,
            user_genotype TEXT,
            allele_match_status TEXT NOT NULL,
            orientation_status TEXT NOT NULL,
            clinvar_clinical_significance TEXT,
            clinvar_condition TEXT,
            notes TEXT,
            interpretation_status TEXT NOT NULL,
            PRIMARY KEY (sample_id, rsid),
            FOREIGN KEY(sample_id) REFERENCES samples(id) ON DELETE CASCADE
        )",
        [],
    )?;

    let _ = migrate_gwas_reference_columns(conn);

    let _ = migrate_qdrant_scope_columns(conn);
    let _ = migrate_research_job_scope_column(conn);
    let _ = migrate_vector_promoted_findings(conn);
    let _ = crate::config::migrate_plaintext_secrets(conn);
    let _ = crate::research::evidence::migrate_evidence_schema(conn);
    let _ = crate::research::gnomad::migrate_gnomad_schema(conn);
    let _ = crate::offline::schema::migrate_offline_schema(conn);
    // NOTE: We do not create persistent main views here because SQLite does not support
    // views referencing attached databases persistently. Read-only views are created
    // dynamically as TEMP VIEWs on every connection setup in connect().
 
    Ok(())
}

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
                record.rsid.to_lowercase(),
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

/// Queries specific variants by rsID (batched IN queries).
pub fn query_by_rsids(conn: &Connection, sample_id: i64, rsids: &[String]) -> Result<Vec<DbSnpRecord>> {
    if rsids.is_empty() {
        return Ok(Vec::new());
    }

    const CHUNK: usize = 100;
    let mut results = Vec::new();

    for chunk in rsids.chunks(CHUNK) {
        let placeholders: Vec<&str> = chunk.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2
             FROM genotypes WHERE sample_id = ? AND rsid IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut param_refs: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(1 + chunk.len());
        param_refs.push(&sample_id);
        for rsid in chunk {
            param_refs.push(rsid);
        }
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
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
        for row in rows {
            results.push(row?);
        }
    }
    Ok(results)
}

const MAX_REGION_WIDTH: u64 = 10_000_000;
const MAX_REGION_RESULTS: usize = 10_000;

/// Queries variants in a chromosome region (GRCh38 coordinates).
pub fn query_region(
    conn: &Connection,
    sample_id: i64,
    chromosome: &str,
    start: u64,
    end: u64,
) -> Result<Vec<DbSnpRecord>> {
    if end < start {
        return Err(rusqlite::Error::InvalidParameterName(
            "end must be >= start".into(),
        ));
    }
    if end - start > MAX_REGION_WIDTH {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "Region width exceeds maximum of {} base pairs",
            MAX_REGION_WIDTH
        )));
    }

    let mut stmt = conn.prepare(
        "SELECT sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2 
         FROM genotypes 
         WHERE sample_id = ? AND chromosome = ? AND position_grch38 >= ? AND position_grch38 <= ?
         ORDER BY position_grch38 ASC
         LIMIT ?",
    )?;

    let rows = stmt.query_map(
        params![sample_id, chromosome, start as i64, end as i64, MAX_REGION_RESULTS as i64],
        |row| {
            Ok(DbSnpRecord {
                sample_id: row.get(0)?,
                rsid: row.get(1)?,
                chromosome: row.get(2)?,
                position_grch37: row.get::<_, i64>(3)? as u64,
                position_grch38: row.get::<_, Option<i64>>(4)?.map(|p| p as u64),
                allele1: row.get(5)?,
                allele2: row.get(6)?,
            })
        },
    )?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Queries variant counts grouped by chromosome for a given sample.
pub fn get_chromosome_counts(
    conn: &Connection,
    sample_id: i64,
) -> Result<std::collections::HashMap<String, i64>> {
    let mut stmt = conn.prepare(
        "SELECT chromosome, COUNT(*) FROM genotypes WHERE sample_id = ? GROUP BY chromosome"
    )?;
    
    let rows = stmt.query_map(params![sample_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    
    let mut results = std::collections::HashMap::new();
    for r in rows {
        let (chr, count) = r?;
        results.insert(chr, count);
    }
    Ok(results)
}

/// Deletes a sample and its genotypes from the database.
pub fn delete_sample(conn: &Connection, sample_id: i64) -> Result<()> {
    conn.execute("DELETE FROM genotypes WHERE sample_id = ?", params![sample_id])?;
    conn.execute("DELETE FROM chat_sessions WHERE sample_id = ?", params![sample_id])?;
    conn.execute("DELETE FROM research_jobs WHERE sample_id = ?", params![sample_id])?;
    conn.execute("DELETE FROM discovered_findings WHERE sample_id = ?", params![sample_id])?;
    conn.execute("DELETE FROM vector_promoted_findings WHERE sample_id = ?", params![sample_id])?;
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

pub fn dump_default_marker_packs_if_missing(data_dir: &Path) {
    let packs_dir = data_dir.join("marker-packs");
    if !packs_dir.exists() {
        let _ = std::fs::create_dir_all(&packs_dir);
    }
    
    let embedded_packs: &[(&str, &str)] = &[
        ("manifest.json", include_str!("../../src/lib/marker-packs/manifest.json")),
        ("core.json", include_str!("../../src/lib/marker-packs/core.json")),
        ("pgx.json", include_str!("../../src/lib/marker-packs/pgx.json")),
        ("metabolic.json", include_str!("../../src/lib/marker-packs/metabolic.json")),
        ("nutrients.json", include_str!("../../src/lib/marker-packs/nutrients.json")),
        ("neuropsych.json", include_str!("../../src/lib/marker-packs/neuropsych.json")),
        ("sleep.json", include_str!("../../src/lib/marker-packs/sleep.json")),
        ("connective_tissue.json", include_str!("../../src/lib/marker-packs/connective_tissue.json")),
        ("thyroid_autoimmune.json", include_str!("../../src/lib/marker-packs/thyroid_autoimmune.json")),
        ("cardiovascular.json", include_str!("../../src/lib/marker-packs/cardiovascular.json")),
        ("cancer_confirmation_only.json", include_str!("../../src/lib/marker-packs/cancer_confirmation_only.json")),
        ("allergy_atopy_mast_cell.json", include_str!("../../src/lib/marker-packs/allergy_atopy_mast_cell.json")),
        ("digestive_gut_microbiome.json", include_str!("../../src/lib/marker-packs/digestive_gut_microbiome.json")),
        ("muscle_performance_recovery.json", include_str!("../../src/lib/marker-packs/muscle_performance_recovery.json")),
        ("hormones_reproductive.json", include_str!("../../src/lib/marker-packs/hormones_reproductive.json")),
        ("skin_hair_dermatology.json", include_str!("../../src/lib/marker-packs/skin_hair_dermatology.json")),
        ("bone_growth_mineral_density.json", include_str!("../../src/lib/marker-packs/bone_growth_mineral_density.json")),
        ("kidney_fluid_electrolytes.json", include_str!("../../src/lib/marker-packs/kidney_fluid_electrolytes.json")),
        ("respiratory_airway.json", include_str!("../../src/lib/marker-packs/respiratory_airway.json")),
        ("immune_autoimmune_general.json", include_str!("../../src/lib/marker-packs/immune_autoimmune_general.json")),
        ("pain_migraine_sensory.json", include_str!("../../src/lib/marker-packs/pain_migraine_sensory.json")),
        ("dental_oral_health.json", include_str!("../../src/lib/marker-packs/dental_oral_health.json")),
        ("longevity_aging_resilience.json", include_str!("../../src/lib/marker-packs/longevity_aging_resilience.json")),
        ("discovery_catalog.json", include_str!("../../src/lib/marker-packs/discovery_catalog.json")),
    ];

    for (filename, content) in embedded_packs {
        let dest = packs_dir.join(filename);
        if !dest.exists() {
            if let Err(e) = std::fs::write(&dest, content) {
                eprintln!("Failed to dump default pack file {}: {}", filename, e);
            }
        }
    }
}

use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;

struct PackRegistryEntry {
    content: String,
    mtime: u64,
}

struct MarkerPackRegistry {
    packs: HashMap<String, PackRegistryEntry>,
    warnings: HashMap<String, String>,
}

static REGISTRY: OnceLock<Mutex<MarkerPackRegistry>> = OnceLock::new();

fn get_registry() -> &'static Mutex<MarkerPackRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(MarkerPackRegistry {
        packs: HashMap::new(),
        warnings: HashMap::new(),
    }))
}

pub fn get_warnings() -> HashMap<String, String> {
    let reg = get_registry().lock().unwrap();
    reg.warnings.clone()
}

pub fn clear_marker_packs_registry() {
    let mut reg = get_registry().lock().unwrap();
    reg.packs.clear();
    reg.warnings.clear();
}

fn validate_pack_json(content: &str) -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct ValidateMarker {
        rsid: String,
        gene: String,
    }
    #[derive(serde::Deserialize)]
    struct ValidatePack {
        name: String,
        markers: Vec<ValidateMarker>,
    }
    let p: ValidatePack = serde_json::from_str(content)
        .map_err(|e| format!("JSON syntax error: {}", e))?;
    if p.name.is_empty() {
        return Err("Pack name cannot be empty".to_string());
    }
    for m in &p.markers {
        if m.rsid.is_empty() || m.gene.is_empty() {
            return Err("rsID and gene are required for all markers".to_string());
        }
    }
    Ok(())
}

pub fn sync_marker_packs_registry(app_data_dir: Option<&Path>) {
    let dir = match app_data_dir {
        Some(d) => d.join("marker-packs"),
        None => return,
    };
    if !dir.exists() {
        return;
    }

    let registry_mutex = get_registry();
    let mut reg = registry_mutex.lock().unwrap();

    // 1. Sync manifest.json
    let manifest_path = dir.join("manifest.json");
    if manifest_path.exists() {
        if let Ok(metadata) = manifest_path.metadata() {
            let mtime = metadata.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            
            let current_entry = reg.packs.get("manifest");
            let needs_update = current_entry.map(|e| e.mtime != mtime).unwrap_or(true);
            if needs_update {
                if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                    #[derive(serde::Deserialize)]
                    #[allow(dead_code)]
                    struct ValidateManifest {
                        packs: Vec<serde_json::Value>,
                    }
                    match serde_json::from_str::<ValidateManifest>(&content) {
                        Ok(_) => {
                            reg.packs.insert("manifest".to_string(), PackRegistryEntry {
                                content,
                                mtime,
                            });
                            reg.warnings.remove("manifest");
                        }
                        Err(e) => {
                            reg.warnings.insert("manifest".to_string(), format!("Manifest validation failed: {}", e));
                        }
                    }
                }
            }
        }
    }

    // 2. Sync all pack JSONs listed in manifest
    let manifest_str = reg.packs.get("manifest")
        .map(|e| e.content.clone())
        .unwrap_or_else(|| include_str!("../../src/lib/marker-packs/manifest.json").to_string());
    
    #[derive(serde::Deserialize)]
    struct ManifestInfo {
        id: String,
    }
    #[derive(serde::Deserialize)]
    struct ManifestPacks {
        packs: Vec<ManifestInfo>,
    }
    
    if let Ok(manifest_data) = serde_json::from_str::<ManifestPacks>(&manifest_str) {
        for pack in manifest_data.packs {
            let filename = format!("{}.json", pack.id);
            let pack_path = dir.join(&filename);
            if pack_path.exists() {
                if let Ok(metadata) = pack_path.metadata() {
                    let mtime = metadata.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    
                    let current_entry = reg.packs.get(&pack.id);
                    let needs_update = current_entry.map(|e| e.mtime != mtime).unwrap_or(true);
                    if needs_update {
                        if let Ok(content) = std::fs::read_to_string(&pack_path) {
                            match validate_pack_json(&content) {
                                Ok(_) => {
                                    reg.packs.insert(pack.id.clone(), PackRegistryEntry {
                                        content,
                                        mtime,
                                    });
                                    reg.warnings.remove(&pack.id);
                                }
                                Err(e) => {
                                    reg.warnings.insert(pack.id.clone(), format!("Pack '{}' validation failed: {}", pack.id, e));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn get_manifest_str(app_data_dir: Option<&Path>) -> String {
    sync_marker_packs_registry(app_data_dir);
    let reg = get_registry().lock().unwrap();
    if let Some(entry) = reg.packs.get("manifest") {
        return entry.content.clone();
    }
    include_str!("../../src/lib/marker-packs/manifest.json").to_string()
}

pub fn get_pack_str(app_data_dir: Option<&Path>, pack_id: &str) -> Option<String> {
    if let Err(e) = crate::config::validate_pack_id(pack_id) {
        eprintln!("Rejected pack_id: {}", e);
        return None;
    }
    sync_marker_packs_registry(app_data_dir);
    let reg = get_registry().lock().unwrap();
    if let Some(entry) = reg.packs.get(pack_id) {
        return Some(entry.content.clone());
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
        "allergy_atopy_mast_cell" => Some(include_str!("../../src/lib/marker-packs/allergy_atopy_mast_cell.json").to_string()),
        "digestive_gut_microbiome" => Some(include_str!("../../src/lib/marker-packs/digestive_gut_microbiome.json").to_string()),
        "muscle_performance_recovery" => Some(include_str!("../../src/lib/marker-packs/muscle_performance_recovery.json").to_string()),
        "hormones_reproductive" => Some(include_str!("../../src/lib/marker-packs/hormones_reproductive.json").to_string()),
        "skin_hair_dermatology" => Some(include_str!("../../src/lib/marker-packs/skin_hair_dermatology.json").to_string()),
        "bone_growth_mineral_density" => Some(include_str!("../../src/lib/marker-packs/bone_growth_mineral_density.json").to_string()),
        "kidney_fluid_electrolytes" => Some(include_str!("../../src/lib/marker-packs/kidney_fluid_electrolytes.json").to_string()),
        "respiratory_airway" => Some(include_str!("../../src/lib/marker-packs/respiratory_airway.json").to_string()),
        "immune_autoimmune_general" => Some(include_str!("../../src/lib/marker-packs/immune_autoimmune_general.json").to_string()),
        "pain_migraine_sensory" => Some(include_str!("../../src/lib/marker-packs/pain_migraine_sensory.json").to_string()),
        "dental_oral_health" => Some(include_str!("../../src/lib/marker-packs/dental_oral_health.json").to_string()),
        "longevity_aging_resilience" => Some(include_str!("../../src/lib/marker-packs/longevity_aging_resilience.json").to_string()),
        "discovery_catalog" => Some(include_str!("../../src/lib/marker-packs/discovery_catalog.json").to_string()),
        _ => None,
    }
}

fn migrate_qdrant_scope_columns(conn: &Connection) -> Result<(), rusqlite::Error> {
    for sql in [
        "ALTER TABLE qdrant_config ADD COLUMN scope_curated INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE qdrant_config ADD COLUMN scope_agent INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE qdrant_config ADD COLUMN scope_gwas INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE qdrant_config ADD COLUMN scope_non_ref INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE qdrant_config ADD COLUMN scope_non_ref_limit INTEGER NOT NULL DEFAULT 5000",
        "ALTER TABLE qdrant_config ADD COLUMN scope_gwas_limit INTEGER NOT NULL DEFAULT 10000",
        "ALTER TABLE qdrant_config ADD COLUMN named_vectors_enabled INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE qdrant_config ADD COLUMN scope_sweep_fast INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE qdrant_config ADD COLUMN scope_sources_json TEXT",
    ] {
        let _ = conn.execute(sql, []);
    }
    Ok(())
}

fn migrate_research_job_scope_column(conn: &Connection) -> Result<(), rusqlite::Error> {
    let _ = conn.execute("ALTER TABLE research_jobs ADD COLUMN scope_json TEXT", []);
    let _ = conn.execute(
        "ALTER TABLE research_jobs ADD COLUMN session_started_at INTEGER",
        [],
    );
    Ok(())
}

fn migrate_gwas_reference_columns(conn: &Connection) -> Result<(), rusqlite::Error> {
    for sql in [
        "ALTER TABLE gwas_reference ADD COLUMN primary_gene TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE gwas_reference ADD COLUMN mapped_genes TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE gwas_reference ADD COLUMN reported_genes TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE gwas_reference ADD COLUMN best_pvalue REAL",
        "ALTER TABLE gwas_reference ADD COLUMN associations_json TEXT NOT NULL DEFAULT '[]'",
    ] {
        let _ = conn.execute(sql, []);
    }
    Ok(())
}

fn migrate_vector_promoted_findings(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS vector_promoted_findings (
            sample_id INTEGER NOT NULL,
            rsid TEXT NOT NULL,
            gene TEXT,
            user_genotype TEXT,
            trait_summary TEXT NOT NULL DEFAULT '',
            trait_categories TEXT NOT NULL DEFAULT '[]',
            significance_score REAL NOT NULL DEFAULT 0,
            gwas_best_pvalue REAL,
            enrichment_version TEXT NOT NULL DEFAULT '4',
            promoted_at INTEGER NOT NULL,
            PRIMARY KEY (sample_id, rsid),
            FOREIGN KEY(sample_id) REFERENCES samples(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_vector_promoted_sample ON vector_promoted_findings(sample_id)",
        [],
    )?;
    Ok(())
}

pub fn seed_evidence_library(conn: &Connection, app_data_dir: Option<&Path>) -> std::result::Result<(), String> {
    seed_evidence_library_with_progress(conn, app_data_dir, |_| {})
}

pub fn seed_evidence_library_with_progress<F: Fn(&str)>(
    conn: &Connection,
    app_data_dir: Option<&Path>,
    progress: F,
) -> std::result::Result<(), String> {
    let manifest_str = get_manifest_str(app_data_dir);
    let manifest: Manifest = serde_json::from_str(&manifest_str)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    let tx = conn.unchecked_transaction().map_err(|e| format!("Failed to start seed transaction: {}", e))?;

    for pack_info in manifest.packs {
        progress(&format!("Seeding evidence library: {}...", pack_info.id));
        let pack_str = get_pack_str(app_data_dir, &pack_info.id);

        if let Some(p_str) = pack_str {
            let pack: Pack = serde_json::from_str(&p_str)
                .map_err(|e| format!("Failed to parse pack {}: {}", pack_info.id, e))?;

            for m in pack.markers {
                if let Some(ref sources) = m.sources
                    && !sources.is_empty() {
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

                            // Query existing evidence text by exact key using transaction
                            let existing_text: Option<String> = tx.query_row(
                                "SELECT evidence_text FROM evidence_library WHERE rsid = ? AND source_citation = ?",
                                params![m.rsid, citation],
                                |row| row.get(0),
                            ).ok();

                            match existing_text {
                                None => {
                                    // Not found, insert new
                                    let _ = tx.execute(
                                        "INSERT INTO reference.evidence_library (rsid, gene, evidence_text, source_citation) VALUES (?, ?, ?, ?)",
                                        params![m.rsid, m.gene, text, citation],
                                    );
                                }
                                Some(old_text) if old_text != text => {
                                    // Text updated, reset embedding to force re-vectorization
                                    let _ = tx.execute(
                                        "UPDATE reference.evidence_library SET evidence_text = ?, embedding = NULL WHERE rsid = ? AND source_citation = ?",
                                        params![text, m.rsid, citation],
                                    );
                                }
                                _ => {} // Identical, skip
                            }
                        }
                        continue;
                    }

                // Default fallback source when no references are provided in the pack
                let citation = format!("Genomics Caddy Pack: {}", pack.name);
                let text = format!(
                    "Gene: {} | Marker: {} | Impact: {} | Interpretation: {}",
                    m.gene, m.rsid, m.impact, m.interpretation
                );
                
                let existing_text: Option<String> = tx.query_row(
                    "SELECT evidence_text FROM evidence_library WHERE rsid = ? AND source_citation = ?",
                    params![m.rsid, citation],
                    |row| row.get(0),
                ).ok();

                match existing_text {
                    None => {
                        let _ = tx.execute(
                            "INSERT INTO reference.evidence_library (rsid, gene, evidence_text, source_citation) VALUES (?, ?, ?, ?)",
                            params![m.rsid, m.gene, text, citation],
                        );
                    }
                    Some(old_text) if old_text != text => {
                        let _ = tx.execute(
                            "UPDATE reference.evidence_library SET evidence_text = ?, embedding = NULL WHERE rsid = ? AND source_citation = ?",
                            params![text, m.rsid, citation],
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    tx.commit().map_err(|e| format!("Failed to commit seed transaction: {}", e))?;
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

fn load_chat_messages(conn: &Connection, session_id: &str) -> Result<Vec<ChatMessage>> {
    let mut msg_stmt = conn.prepare(
        "SELECT role, content, images, safety_review FROM chat_messages WHERE session_id = ? ORDER BY id ASC"
    )?;
    let msg_rows = msg_stmt.query_map(params![session_id], |row| {
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
    Ok(messages)
}

#[allow(clippy::type_complexity)]
fn map_chat_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(
    String,
    Option<i64>,
    String,
    i64,
    serde_json::Value,
    bool,
    f64,
    String,
    Option<i32>,
    Option<bool>,
    Option<String>,
)> {
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

    Ok((
        id,
        sample_id,
        title,
        timestamp,
        selected_packs,
        only_active_findings,
        temperature,
        selected_model,
        max_tokens,
        extended_thinking,
        consultation_mode,
    ))
}

pub fn get_chat_session_by_id(conn: &Connection, session_id: &str) -> Result<Option<DbChatSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, sample_id, title, timestamp, selected_packs, only_active_findings, 
                temperature, selected_model, max_tokens, extended_thinking, consultation_mode 
         FROM chat_sessions WHERE id = ?"
    )?;
    let mut rows = stmt.query_map(params![session_id], map_chat_session_row)?;
    let Some(row) = rows.next() else {
        return Ok(None);
    };
    let (
        id,
        sample_id,
        title,
        timestamp,
        selected_packs,
        only_active_findings,
        temperature,
        selected_model,
        max_tokens,
        extended_thinking,
        consultation_mode,
    ) = row?;
    let messages = load_chat_messages(conn, &id)?;
    Ok(Some(DbChatSession {
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
    }))
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

    let mapper = map_chat_session_row;

    let rows = if let Some(sample_id) = sample_id_filter {
        stmt.query_map(params![sample_id], mapper)?
    } else {
        stmt.query_map([], mapper)?
    };

    let mut sessions = Vec::new();
    for r in rows {
        let (
            id,
            sample_id,
            title,
            timestamp,
            selected_packs,
            only_active_findings,
            temperature,
            selected_model,
            max_tokens,
            extended_thinking,
            consultation_mode,
        ) = r?;

        let messages = load_chat_messages(conn, &id)?;

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

#[derive(Debug, serde::Serialize, Clone)]
pub struct AppBootstrapStatus {
    pub data_dir: String,
    pub db_path: String,
    pub chain_path: String,
    pub chain_present: bool,
    pub env_path: String,
    pub sample_count: u64,
    pub genotype_count: u64,
    pub discovered_findings_count: u64,
    pub gwas_reference_count: u64,
    pub evidence_library_count: u64,
    pub samples: Vec<SampleInfo>,
}

/// Collect database stats after schema init/migrations for the startup splash screen.
pub fn get_bootstrap_status(conn: &Connection, data_dir: &Path) -> Result<AppBootstrapStatus, String> {
    let count_query = |sql: &str| -> u64 {
        conn.query_row(sql, [], |row| row.get::<_, i64>(0))
            .unwrap_or(0) as u64
    };

    let samples = get_samples(conn).map_err(|e| e.to_string())?;
    let chain_path = crate::paths::chain_path(data_dir);

    Ok(AppBootstrapStatus {
        data_dir: data_dir.to_string_lossy().to_string(),
        db_path: crate::paths::db_path(data_dir).to_string_lossy().to_string(),
        chain_path: chain_path.to_string_lossy().to_string(),
        chain_present: chain_path.exists(),
        env_path: crate::config::recommended_env_path()
            .to_string_lossy()
            .to_string(),
        sample_count: samples.len() as u64,
        genotype_count: count_query("SELECT COUNT(*) FROM genotypes"),
        discovered_findings_count: count_query("SELECT COUNT(*) FROM discovered_findings"),
        gwas_reference_count: count_query("SELECT COUNT(*) FROM gwas_reference"),
        evidence_library_count: count_query("SELECT COUNT(*) FROM evidence_library"),
        samples,
    })
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct DiscoveredFindingSummary {
    pub rsid: String,
    pub gene: Option<String>,
    pub user_genotype: Option<String>,
    pub interpretation_status: String,
    pub clinvar_clinical_significance: Option<String>,
}

pub fn get_discovered_findings_summary(
    conn: &Connection,
    sample_id: i64,
) -> Result<Vec<DiscoveredFindingSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT rsid, gene, user_genotype, interpretation_status, clinvar_clinical_significance
             FROM discovered_findings WHERE sample_id = ? ORDER BY rsid",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id], |row| {
            Ok(DiscoveredFindingSummary {
                rsid: row.get(0)?,
                gene: row.get(1)?,
                user_genotype: row.get(2)?,
                interpretation_status: row.get(3)?,
                clinvar_clinical_significance: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct VectorPromotedFinding {
    pub rsid: String,
    pub gene: Option<String>,
    pub user_genotype: Option<String>,
    pub trait_summary: String,
    pub trait_categories: Vec<String>,
    pub significance_score: f32,
    pub gwas_best_pvalue: Option<f64>,
    pub enrichment_version: String,
    pub promoted_at: i64,
}

pub fn get_vector_promoted_findings(
    conn: &Connection,
    sample_id: i64,
) -> Result<Vec<VectorPromotedFinding>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT rsid, gene, user_genotype, trait_summary, trait_categories,
                    significance_score, gwas_best_pvalue, enrichment_version, promoted_at
             FROM vector_promoted_findings WHERE sample_id = ?
             ORDER BY significance_score DESC, rsid",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id], |row| {
            let categories_json: String = row.get(4)?;
            let trait_categories: Vec<String> = serde_json::from_str(&categories_json).unwrap_or_default();
            Ok(VectorPromotedFinding {
                rsid: row.get(0)?,
                gene: row.get(1)?,
                user_genotype: row.get(2)?,
                trait_summary: row.get(3)?,
                trait_categories,
                significance_score: row.get(5)?,
                gwas_best_pvalue: row.get(6)?,
                enrichment_version: row.get(7)?,
                promoted_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
