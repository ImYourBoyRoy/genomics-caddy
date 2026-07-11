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

use crate::liftover::LiftoverEngine;
use crate::parser::SnpRecord;
use rusqlite::{Connection, Result, params};
use std::path::Path;

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
    let parent_dir = path.as_ref().parent().unwrap_or_else(|| Path::new("."));
    attach_public_databases(&conn, parent_dir)?;
    Ok(conn)
}

/// Opens an isolated sample database and attaches shared public reference databases.
pub fn connect_sample(data_dir: &Path, sample_id: i64) -> Result<Connection> {
    std::fs::create_dir_all(crate::paths::sample_dir(data_dir, sample_id))
        .map_err(|_| rusqlite::Error::InvalidPath(crate::paths::sample_dir(data_dir, sample_id)))?;
    let conn =
        crate::db_crypto::open_encrypted(&crate::paths::sample_db_path(data_dir, sample_id))?;
    conn.execute("PRAGMA foreign_keys = ON;", [])?;
    attach_public_databases(&conn, data_dir)?;
    ensure_sample_schema(&conn)?;
    // Existing samples created before variant_locus was in the sample schema
    // get an empty table on first open — backfill once from genotypes.
    maybe_backfill_variant_locus(&conn, sample_id)?;
    Ok(conn)
}

fn maybe_backfill_variant_locus(conn: &Connection, sample_id: i64) -> Result<()> {
    let locus_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM variant_locus WHERE sample_id = ?",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if locus_count > 0 {
        return Ok(());
    }
    let genotype_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM genotypes WHERE sample_id = ? AND position_grch38 IS NOT NULL AND position_grch38 > 0",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if genotype_count == 0 {
        return Ok(());
    }
    match crate::offline::tier2::build_variant_locus_for_sample(conn, sample_id) {
        Ok(n) => {
            if n > 0 {
                eprintln!(
                    "Backfilled variant_locus for sample {sample_id}: {n} rows"
                );
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: failed to backfill variant_locus for sample {sample_id}: {e}"
            );
        }
    }
    Ok(())
}

/// Opens a sample database when only the registry database path is available.
pub fn connect_sample_from_registry_path(
    registry_path: &Path,
    sample_id: i64,
) -> Result<Connection> {
    let data_dir = registry_path.parent().unwrap_or_else(|| Path::new("."));
    connect_sample(data_dir, sample_id)
}

/// Attach a sidecar DB only when the file already exists.
/// SQLite `ATTACH` creates empty files otherwise — we refuse that for catalog DBs
/// (clinvar/dbsnp) so they appear only after a real download/import.
fn try_attach_existing(conn: &Connection, path: &Path, schema: &str) -> bool {
    if !path.is_file() {
        return false;
    }
    let sql = format!(
        "ATTACH DATABASE '{}' AS {schema}",
        path.to_string_lossy().replace('\\', "/")
    );
    match conn.execute(&sql, []) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Warning: could not ATTACH {} as {schema}: {e}", path.display());
            false
        }
    }
}

/// Create-and-attach for operational caches that the app itself owns
/// (reference registry + HTTP cache). Catalog downloads still create clinvar/dbsnp.
fn attach_or_create(conn: &Connection, path: &Path, schema: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            rusqlite::Error::InvalidParameterName(format!(
                "Failed to create DB directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    let sql = format!(
        "ATTACH DATABASE '{}' AS {schema}",
        path.to_string_lossy().replace('\\', "/")
    );
    conn.execute(&sql, [])?;
    Ok(())
}

/// Move catalog DBs that were incorrectly written under `raw_downloads/` into `App/Data/`.
fn migrate_misplaced_catalog_dbs(data_dir: &Path) {
    for name in [
        "clinvar.db",
        "dbsnp.db",
        "gwas.db",
        "pharmgkb.db",
        "clingen.db",
        "mane.db",
    ] {
        let correct = data_dir.join(name);
        let misplaced = data_dir.join("raw_downloads").join(name);
        if correct.is_file() || !misplaced.is_file() {
            continue;
        }
        match std::fs::rename(&misplaced, &correct) {
            Ok(()) => eprintln!(
                "Migrated misplaced {name} from raw_downloads/ → {}",
                correct.display()
            ),
            Err(e) => {
                if let Err(copy_err) = std::fs::copy(&misplaced, &correct) {
                    eprintln!("Failed to migrate {name}: rename={e}; copy={copy_err}");
                    continue;
                }
                let _ = std::fs::remove_file(&misplaced);
                eprintln!(
                    "Migrated misplaced {name} (copy) from raw_downloads/ → {}",
                    correct.display()
                );
            }
        }
        let staging = data_dir
            .join("raw_downloads")
            .join(name.replace(".db", "_staging.db"));
        let _ = std::fs::remove_file(staging);
    }
}

fn attach_public_databases(conn: &Connection, parent_dir: &Path) -> Result<()> {
    migrate_misplaced_catalog_dbs(parent_dir);

    // App-owned sidecars (may be created empty on first launch).
    attach_or_create(conn, &parent_dir.join("genomics_reference.db"), "reference")?;
    attach_or_create(conn, &parent_dir.join("api_cache.db"), "api_cache_db")?;

    // Catalog sidecars — attach when present under App/Data/.
    let has_clinvar = try_attach_existing(conn, &parent_dir.join("clinvar.db"), "clinvar");
    let has_dbsnp = try_attach_existing(conn, &parent_dir.join("dbsnp.db"), "dbsnp");
    let has_gwas = try_attach_existing(conn, &parent_dir.join("gwas.db"), "gwas");
    let has_pharmgkb = try_attach_existing(conn, &parent_dir.join("pharmgkb.db"), "pharmgkb");
    let has_clingen = try_attach_existing(conn, &parent_dir.join("clingen.db"), "clingen");
    let has_mane = try_attach_existing(conn, &parent_dir.join("mane.db"), "mane");

    // One-time: if dedicated DBs are missing but reference still holds the tables, split them out.
    migrate_reference_catalogs_to_sidecars(conn, parent_dir)?;

    // Re-probe after possible migration (schema may already be attached by the migrator).
    let schema_is = |name: &str| {
        conn.query_row(
            "SELECT 1 FROM pragma_database_list WHERE name = ?1",
            params![name],
            |_| Ok(true),
        )
        .unwrap_or(false)
    };
    let has_gwas = has_gwas
        || schema_is("gwas")
        || try_attach_existing(conn, &parent_dir.join("gwas.db"), "gwas");
    let has_pharmgkb = has_pharmgkb
        || schema_is("pharmgkb")
        || try_attach_existing(conn, &parent_dir.join("pharmgkb.db"), "pharmgkb");
    let has_clingen = has_clingen
        || schema_is("clingen")
        || try_attach_existing(conn, &parent_dir.join("clingen.db"), "clingen");
    let has_mane = has_mane
        || schema_is("mane")
        || try_attach_existing(conn, &parent_dir.join("mane.db"), "mane");

    // TEMP VIEWs for unqualified reads.
    let mut reference_views: Vec<(&str, &str)> = vec![
        ("api_cache", "SELECT * FROM api_cache_db.api_cache"),
        (
            "api_cache_entries",
            "SELECT * FROM api_cache_db.api_cache_entries",
        ),
        (
            "source_records",
            "SELECT * FROM api_cache_db.source_records",
        ),
        (
            "gnomad_variant_cache",
            "SELECT * FROM reference.gnomad_variant_cache",
        ),
        ("gnomad_config", "SELECT * FROM reference.gnomad_config"),
        (
            "offline_asset_registry",
            "SELECT * FROM reference.offline_asset_registry",
        ),
        (
            "evidence_library",
            "SELECT * FROM reference.evidence_library",
        ),
    ];

    if has_gwas {
        reference_views.push(("gwas_reference", "SELECT * FROM gwas.gwas_reference"));
    } else {
        reference_views.push(("gwas_reference", "SELECT * FROM reference.gwas_reference"));
    }
    if has_pharmgkb {
        reference_views.push((
            "pharmgkb_clinical_variants",
            "SELECT * FROM pharmgkb.pharmgkb_clinical_variants",
        ));
        reference_views.push(("pharmgkb_genes", "SELECT * FROM pharmgkb.pharmgkb_genes"));
    } else {
        reference_views.push((
            "pharmgkb_clinical_variants",
            "SELECT * FROM reference.pharmgkb_clinical_variants",
        ));
        reference_views.push((
            "pharmgkb_genes",
            "SELECT * FROM reference.pharmgkb_genes",
        ));
    }
    if has_clingen {
        reference_views.push((
            "clingen_gene_validity",
            "SELECT * FROM clingen.clingen_gene_validity",
        ));
    } else {
        reference_views.push((
            "clingen_gene_validity",
            "SELECT * FROM reference.clingen_gene_validity",
        ));
    }
    if has_mane {
        reference_views.push(("mane_transcripts", "SELECT * FROM mane.mane_transcripts"));
    } else {
        reference_views.push((
            "mane_transcripts",
            "SELECT * FROM reference.mane_transcripts",
        ));
    }
    if has_clinvar {
        reference_views.push((
            "clinvar_reference",
            "SELECT * FROM clinvar.clinvar_reference",
        ));
    }
    if has_dbsnp {
        reference_views.push(("rsid_aliases", "SELECT * FROM dbsnp.rsid_aliases"));
    }
    for (view_name, select_sql) in reference_views {
        let ddl = format!("CREATE TEMP VIEW IF NOT EXISTS {view_name} AS {select_sql}");
        if let Err(e) = conn.execute(&ddl, []) {
            eprintln!("Warning: could not create temp view {view_name} on connect: {e}");
        }
    }

    Ok(())
}

/// Split GWAS / PharmGKB / ClinGen / MANE out of genomics_reference.db into dedicated files.
fn migrate_reference_catalogs_to_sidecars(conn: &Connection, data_dir: &Path) -> Result<()> {
    // (schema, filename, source_table → dest_table pairs)
    type CatalogPlan = (&'static str, &'static str, &'static [(&'static str, &'static str)]);
    let plans: &[CatalogPlan] = &[
        (
            "gwas",
            "gwas.db",
            &[("gwas_reference", "gwas_reference")],
        ),
        (
            "pharmgkb",
            "pharmgkb.db",
            &[
                ("pharmgkb_clinical_variants", "pharmgkb_clinical_variants"),
                ("pharmgkb_genes", "pharmgkb_genes"),
            ],
        ),
        (
            "clingen",
            "clingen.db",
            &[("clingen_gene_validity", "clingen_gene_validity")],
        ),
        ("mane", "mane.db", &[("mane_transcripts", "mane_transcripts")]),
    ];

    for (schema, filename, tables) in plans {
        let dest = data_dir.join(filename);
        let already = conn
            .query_row(
                "SELECT 1 FROM pragma_database_list WHERE name = ?1",
                params![*schema],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if already || dest.is_file() {
            continue;
        }

        // Only split if reference still has rows for at least one table.
        let mut has_rows = false;
        for (src, _) in *tables {
            let n: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM reference.{src}"),
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if n > 0 {
                has_rows = true;
                break;
            }
        }
        if !has_rows {
            continue;
        }

        eprintln!("Splitting {schema} catalog into {}", dest.display());
        attach_or_create(conn, &dest, schema)?;
        ensure_catalog_schema_ddl(conn, schema)?;
        for (src, dst) in *tables {
            let sql = format!(
                "INSERT OR IGNORE INTO {schema}.{dst} SELECT * FROM reference.{src}"
            );
            if let Err(e) = conn.execute(&sql, []) {
                eprintln!("Warning: migrate {src} → {schema}.{dst}: {e}");
            } else {
                let _ = conn.execute(&format!("DELETE FROM reference.{src}"), []);
            }
        }
    }
    Ok(())
}

/// Ensure a catalog sidecar exists and is attached (called from offline import).
pub fn ensure_catalog_db_attached(conn: &Connection, data_dir: &Path, schema: &str) -> Result<()> {
    let filename = match schema {
        "clinvar" => "clinvar.db",
        "dbsnp" => "dbsnp.db",
        "gwas" => "gwas.db",
        "pharmgkb" => "pharmgkb.db",
        "clingen" => "clingen.db",
        "mane" => "mane.db",
        other => {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Unknown catalog schema: {other}"
            )));
        }
    };
    let path = data_dir.join(filename);
    let attached: bool = conn
        .query_row(
            "SELECT 1 FROM pragma_database_list WHERE name = ?1",
            params![schema],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !attached {
        attach_or_create(conn, &path, schema)?;
    }
    ensure_catalog_schema_ddl(conn, schema)?;
    Ok(())
}

/// Create tables/indexes inside an attached catalog schema.
pub fn ensure_catalog_schema_ddl(conn: &Connection, schema: &str) -> Result<()> {
    match schema {
        "gwas" => {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS gwas.gwas_reference (
                    rsid TEXT PRIMARY KEY,
                    association_count INTEGER NOT NULL DEFAULT 1,
                    top_trait TEXT NOT NULL DEFAULT '',
                    primary_gene TEXT NOT NULL DEFAULT '',
                    mapped_genes TEXT NOT NULL DEFAULT '',
                    reported_genes TEXT NOT NULL DEFAULT '',
                    best_pvalue REAL,
                    associations_json TEXT NOT NULL DEFAULT '[]'
                );
                CREATE INDEX IF NOT EXISTS gwas.idx_gwas_reference_rsid ON gwas_reference(rsid);
                ",
            )?;
        }
        "pharmgkb" => {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS pharmgkb.pharmgkb_clinical_variants (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    rsid TEXT NOT NULL,
                    gene TEXT,
                    drug TEXT,
                    phenotype TEXT,
                    evidence_level TEXT,
                    raw_json TEXT
                );
                CREATE INDEX IF NOT EXISTS pharmgkb.idx_pharmgkb_rsid ON pharmgkb_clinical_variants(rsid);
                CREATE TABLE IF NOT EXISTS pharmgkb.pharmgkb_genes (
                    pharmgkb_id TEXT PRIMARY KEY,
                    symbol TEXT NOT NULL,
                    name TEXT,
                    raw_json TEXT
                );
                CREATE INDEX IF NOT EXISTS pharmgkb.idx_pharmgkb_genes_symbol ON pharmgkb_genes(symbol);
                ",
            )?;
        }
        "clingen" => {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS clingen.clingen_gene_validity (
                    hgnc_id TEXT,
                    gene_symbol TEXT NOT NULL,
                    disease_label TEXT NOT NULL,
                    classification TEXT,
                    moi TEXT,
                    report_url TEXT,
                    PRIMARY KEY (gene_symbol, disease_label)
                );
                CREATE INDEX IF NOT EXISTS clingen.idx_clingen_gene ON clingen_gene_validity(gene_symbol);
                ",
            )?;
        }
        "mane" => {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS mane.mane_transcripts (
                    gene_symbol TEXT PRIMARY KEY,
                    ensembl_transcript TEXT,
                    refseq_transcript TEXT,
                    mane_status TEXT,
                    grch38_coordinates TEXT
                );
                ",
            )?;
        }
        "clinvar" | "dbsnp" => {
            // Created by their importers / migrate_offline_schema.
        }
        other => {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Unknown catalog schema DDL: {other}"
            )));
        }
    }
    Ok(())
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

/// Checkpoint WAL and drop cached handles on graceful shutdown (no encryption).
pub fn seal<P: AsRef<Path>>(path: P) -> Result<(), String> {
    clear_cached_conn();
    let path = path.as_ref();
    if path.is_file()
        && let Ok(conn) = Connection::open(path)
    {
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }
    crate::db_crypto::discard_legacy_sealed_db(path);
    Ok(())
}

fn table_exists_in_main(conn: &Connection, table_name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?",
        params![table_name],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

fn migrate_to_reference_db<F: Fn(&str)>(conn: &Connection, progress: F) -> Result<()> {
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
    let total = tables_to_migrate.len();
    for (idx, t) in tables_to_migrate.iter().enumerate() {
        if table_exists_in_main(conn, t) {
            progress(&format!(
                "Migrating reference database table: {} ({} of {})...",
                t,
                idx + 1,
                total
            ));
            let copy_sql = format!(
                "INSERT OR IGNORE INTO reference.{} SELECT * FROM main.{}",
                t, t
            );
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
        progress("Vacuuming user database to reclaim disk space...");
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
    if let Some(data_dir) = path.as_ref().parent() {
        migrate_legacy_sample_tables(&conn, data_dir)?;
    }

    emit_progress("Migrating reference schema mappings...");
    migrate_to_reference_db(&conn, emit_progress)?;

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
    let _ = conn.execute(
        "ALTER TABLE samples ADD COLUMN genetic_sex TEXT DEFAULT 'Unknown'",
        [],
    );

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
    conn.execute(
        "CREATE INDEX IF NOT EXISTS reference.idx_evidence_rsid ON evidence_library(rsid)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS reference.idx_evidence_gene ON evidence_library(gene)",
        [],
    )?;

    // Qdrant / research settings (secrets live in OS keyring — columns kept for legacy migration)
    // Network URLs default empty: users configure them under Advanced → Connections (or via .env).
    conn.execute(
        "CREATE TABLE IF NOT EXISTS qdrant_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            url TEXT NOT NULL DEFAULT '',
            api_key TEXT,
            collection TEXT NOT NULL DEFAULT '',
            embedding_model TEXT NOT NULL DEFAULT '',
            gwas_strict INTEGER NOT NULL DEFAULT 1,
            ncbi_api_key TEXT,
            auto_start INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )?;
    let _ = conn.execute(
        "ALTER TABLE qdrant_config ADD COLUMN ollama_url TEXT NOT NULL DEFAULT ''",
        [],
    );
    let default_url = std::env::var("QDRANT_URL").unwrap_or_default();
    let default_collection = std::env::var("QDRANT_COLLECTION").unwrap_or_default();
    let default_model = std::env::var("OLLAMA_EMBED_MODEL").unwrap_or_default();
    let default_ollama = std::env::var("GENOMICS_OLLAMA_URL")
        .or_else(|_| std::env::var("OLLAMA_URL"))
        .unwrap_or_default();
    conn.execute(
        "INSERT OR IGNORE INTO qdrant_config (id, url, collection, embedding_model, ollama_url) VALUES (1, ?, ?, ?, ?)",
        rusqlite::params![default_url, default_collection, default_model, default_ollama],
    )?;
    // Backfill ollama_url from env when the column is still empty.
    if !default_ollama.is_empty() {
        let _ = conn.execute(
            "UPDATE qdrant_config SET ollama_url = ?1 WHERE id = 1 AND (ollama_url IS NULL OR TRIM(ollama_url) = '')",
            rusqlite::params![default_ollama],
        );
    }
    conn.execute(
        "CREATE TABLE IF NOT EXISTS api_cache_db.api_cache (
            url TEXT PRIMARY KEY,
            response_json TEXT NOT NULL,
            fetched_at INTEGER NOT NULL
        )",
        [],
    )?;

    // ClinVar catalog schema only when clinvar.db was downloaded/attached.
    let clinvar_attached: bool = conn
        .query_row(
            "SELECT 1 FROM pragma_database_list WHERE name = 'clinvar'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if clinvar_attached {
        // Drop clinvar_reference if it's the old schema (lacking gene_symbol) or has old single PK
        let has_new_schema = conn
            .query_row(
                "SELECT 1 FROM clinvar.sqlite_master WHERE type='table' AND name='clinvar_reference' AND sql LIKE '%gene_symbol%'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);

        let has_old_pk = conn
            .query_row(
                "SELECT 1 FROM clinvar.sqlite_master WHERE type='table' AND name='clinvar_reference' AND sql LIKE '%PRIMARY KEY (rsid)%'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !has_new_schema || has_old_pk {
            let _ = conn.execute("DROP TABLE IF EXISTS clinvar.clinvar_reference", []);
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clinvar.clinvar_reference (
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
        conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS clinvar.idx_clinvar_reference_rsid ON clinvar_reference(rsid);
            CREATE INDEX IF NOT EXISTS clinvar.idx_clinvar_reference_gene ON clinvar_reference(gene_symbol);
            CREATE INDEX IF NOT EXISTS clinvar.idx_clinvar_reference_varid ON clinvar_reference(variation_id);
            CREATE INDEX IF NOT EXISTS clinvar.idx_clinvar_reference_coords
                ON clinvar_reference(assembly, chromosome, start);
            ",
        )?;
    }

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

    let _ = migrate_gwas_reference_columns(conn);

    let _ = migrate_qdrant_scope_columns(conn);
    let _ = crate::config::migrate_plaintext_secrets(conn);
    let _ = crate::research::evidence::migrate_evidence_schema(conn);
    let _ = crate::research::gnomad::migrate_gnomad_schema(conn);
    let _ = crate::offline::schema::migrate_offline_schema(conn);

    // Legacy Reference DB Data Migration Check
    let tables_to_migrate = vec![
        ("api_cache", "api_cache_db", "api_cache"),
        ("api_cache_entries", "api_cache_db", "api_cache_entries"),
        ("source_records", "api_cache_db", "source_records"),
        ("clinvar_reference", "clinvar", "clinvar_reference"),
        ("rsid_aliases", "dbsnp", "rsid_aliases"),
    ];

    let mut migrated_any = false;
    for (old_table, target_schema, target_table) in tables_to_migrate {
        // Check if old table exists in reference database
        let old_exists = conn
            .query_row(
                "SELECT 1 FROM reference.sqlite_master WHERE type='table' AND name = ?",
                [old_table],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if old_exists {
            // Check if old table has rows
            let old_rows: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM reference.{}", old_table),
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            if old_rows > 0 {
                // Check if target table is empty
                let target_rows: i64 = conn
                    .query_row(
                        &format!("SELECT COUNT(*) FROM {}.{}", target_schema, target_table),
                        [],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);

                if target_rows == 0 {
                    println!(
                        "Migrating {} rows from reference.{} to {}.{}",
                        old_rows, old_table, target_schema, target_table
                    );
                    let copy_sql = format!(
                        "INSERT INTO {}.{} SELECT * FROM reference.{}",
                        target_schema, target_table, old_table
                    );
                    if let Err(e) = conn.execute(&copy_sql, []) {
                        eprintln!("Warning: failed to copy table {}: {}", old_table, e);
                    } else {
                        migrated_any = true;
                    }
                }
            }

            // Drop the old table to clean up
            let _ = conn.execute(&format!("DROP TABLE IF EXISTS reference.{}", old_table), []);
        }
    }

    if migrated_any {
        println!("Vacuuming legacy reference database to reclaim space...");
        let _ = conn.execute("VACUUM reference", []);
    }

    // NOTE: We do not create persistent main views here because SQLite does not support
    // views referencing attached databases persistently. Read-only views are created
    // dynamically as TEMP VIEWs on every connection setup in connect().

    Ok(())
}

fn ensure_sample_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS genotypes (
            sample_id INTEGER NOT NULL,
            rsid TEXT NOT NULL,
            chromosome TEXT NOT NULL,
            position_grch37 INTEGER NOT NULL,
            position_grch38 INTEGER,
            allele1 TEXT NOT NULL,
            allele2 TEXT NOT NULL,
            PRIMARY KEY (sample_id, rsid)
        );
        CREATE INDEX IF NOT EXISTS idx_genotypes_rsid ON genotypes(rsid);
        CREATE INDEX IF NOT EXISTS idx_genotypes_coords ON genotypes(chromosome, position_grch38);
        CREATE TABLE IF NOT EXISTS variant_locus (
            variant_key TEXT PRIMARY KEY,
            assembly TEXT NOT NULL DEFAULT 'GRCh38',
            chrom TEXT NOT NULL,
            pos INTEGER NOT NULL,
            ref_allele TEXT,
            alt_allele TEXT,
            rsid TEXT,
            sample_id INTEGER,
            source TEXT NOT NULL DEFAULT 'genotype'
        );
        CREATE INDEX IF NOT EXISTS idx_variant_locus_rsid ON variant_locus(rsid);
        CREATE INDEX IF NOT EXISTS idx_variant_locus_coords ON variant_locus(assembly, chrom, pos);
        CREATE TABLE IF NOT EXISTS chat_sessions (
            id TEXT PRIMARY KEY,
            sample_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            selected_packs TEXT NOT NULL,
            only_active_findings INTEGER NOT NULL,
            temperature REAL NOT NULL,
            selected_model TEXT NOT NULL,
            max_tokens INTEGER,
            extended_thinking INTEGER,
            consultation_mode TEXT
        );
        CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            images TEXT,
            safety_review TEXT,
            FOREIGN KEY(session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id);
        CREATE TABLE IF NOT EXISTS research_jobs (
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
            scope_json TEXT,
            session_started_at INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_research_jobs_sample ON research_jobs(sample_id, started_at DESC);
        CREATE TABLE IF NOT EXISTS discovered_findings (
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
            PRIMARY KEY (sample_id, rsid)
        );
        CREATE TABLE IF NOT EXISTS vector_promoted_findings (
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
            PRIMARY KEY (sample_id, rsid)
        );
        CREATE INDEX IF NOT EXISTS idx_vector_promoted_sample ON vector_promoted_findings(sample_id);",
    )?;
    Ok(())
}

fn migrate_legacy_sample_tables(conn: &Connection, data_dir: &Path) -> Result<()> {
    if !table_exists_in_main(conn, "genotypes")
        || conn.query_row("SELECT COUNT(*) FROM genotypes", [], |row| {
            row.get::<_, i64>(0)
        })? == 0
    {
        return Ok(());
    }

    let mut samples = conn.prepare("SELECT id FROM samples")?;
    let ids = samples
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for sample_id in ids {
        let _sample = connect_sample(data_dir, sample_id)?;
        let target = crate::paths::sample_db_path(data_dir, sample_id);
        let escaped = target.to_string_lossy().replace('\'', "''");
        conn.execute(
            &format!("ATTACH DATABASE '{escaped}' AS sample_migration"),
            [],
        )?;
        let migration = (|| -> Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            for table in [
                "genotypes",
                "research_jobs",
                "discovered_findings",
                "vector_promoted_findings",
            ] {
                if table_exists_in_main(conn, table) {
                    conn.execute(
                        &format!("INSERT OR REPLACE INTO sample_migration.{table} SELECT * FROM main.{table} WHERE sample_id = ?"),
                        params![sample_id],
                    )?;
                }
            }
            if table_exists_in_main(conn, "chat_sessions") {
                conn.execute(
                    "INSERT OR REPLACE INTO sample_migration.chat_sessions
                     SELECT * FROM main.chat_sessions WHERE sample_id = ?",
                    params![sample_id],
                )?;
                if table_exists_in_main(conn, "chat_messages") {
                    conn.execute(
                        "INSERT OR REPLACE INTO sample_migration.chat_messages
                         SELECT * FROM main.chat_messages
                         WHERE session_id IN (SELECT id FROM main.chat_sessions WHERE sample_id = ?)",
                        params![sample_id],
                    )?;
                }
            }
            conn.execute_batch("COMMIT")
        })();
        if migration.is_err() {
            let _ = conn.execute_batch("ROLLBACK");
        }
        conn.execute("DETACH DATABASE sample_migration", [])?;
        migration?;
    }

    for table in [
        "chat_messages",
        "chat_sessions",
        "vector_promoted_findings",
        "discovered_findings",
        "research_jobs",
        "genotypes",
    ] {
        if table_exists_in_main(conn, table) {
            conn.execute(&format!("DROP TABLE main.{table}"), [])?;
        }
    }
    Ok(())
}

pub fn import_raw_genome<F: Fn(u32, &str)>(
    conn: &mut Connection,
    data_dir: &Path,
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

    // 2. Perform bulk insertion in the isolated sample database.
    let mut sample_conn = connect_sample(data_dir, sample_id)
        .map_err(|e| format!("Failed to initialize sample database: {e}"))?;
    let tx = sample_conn
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
                progress_callback(
                    percent,
                    &format!("Liftover & Ingesting SNPs: {}/{}...", i, total),
                );
            }

            let pos_grch38 = liftover_engine
                .and_then(|engine| engine.liftover(&record.chromosome, record.position));

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

/// Retrieves list of all imported samples from the registry database.
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
        let (id, name, genetic_sex, imported_at) = row_res?;
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
pub fn query_by_rsids(
    conn: &Connection,
    sample_id: i64,
    rsids: &[String],
) -> Result<Vec<DbSnpRecord>> {
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
        params![
            sample_id,
            chromosome,
            start as i64,
            end as i64,
            MAX_REGION_RESULTS as i64
        ],
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
        "SELECT chromosome, COUNT(*) FROM genotypes WHERE sample_id = ? GROUP BY chromosome",
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

/// Deletes the registry row and the complete private database directory for a sample.
pub fn delete_sample(conn: &Connection, data_dir: &Path, sample_id: i64) -> Result<()> {
    clear_cached_conn();
    let sample_dir = crate::paths::sample_dir(data_dir, sample_id);
    if sample_dir.exists() {
        std::fs::remove_dir_all(&sample_dir)
            .map_err(|_| rusqlite::Error::InvalidPath(sample_dir))?;
    }
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
        (
            "manifest.json",
            include_str!("../../src/lib/marker-packs/manifest.json"),
        ),
        (
            "core.json",
            include_str!("../../src/lib/marker-packs/core.json"),
        ),
        (
            "pgx.json",
            include_str!("../../src/lib/marker-packs/pgx.json"),
        ),
        (
            "metabolic.json",
            include_str!("../../src/lib/marker-packs/metabolic.json"),
        ),
        (
            "nutrients.json",
            include_str!("../../src/lib/marker-packs/nutrients.json"),
        ),
        (
            "neuropsych.json",
            include_str!("../../src/lib/marker-packs/neuropsych.json"),
        ),
        (
            "sleep.json",
            include_str!("../../src/lib/marker-packs/sleep.json"),
        ),
        (
            "connective_tissue.json",
            include_str!("../../src/lib/marker-packs/connective_tissue.json"),
        ),
        (
            "thyroid_autoimmune.json",
            include_str!("../../src/lib/marker-packs/thyroid_autoimmune.json"),
        ),
        (
            "cardiovascular.json",
            include_str!("../../src/lib/marker-packs/cardiovascular.json"),
        ),
        (
            "cancer_confirmation_only.json",
            include_str!("../../src/lib/marker-packs/cancer_confirmation_only.json"),
        ),
        (
            "allergy_atopy_mast_cell.json",
            include_str!("../../src/lib/marker-packs/allergy_atopy_mast_cell.json"),
        ),
        (
            "digestive_gut_microbiome.json",
            include_str!("../../src/lib/marker-packs/digestive_gut_microbiome.json"),
        ),
        (
            "muscle_performance_recovery.json",
            include_str!("../../src/lib/marker-packs/muscle_performance_recovery.json"),
        ),
        (
            "hormones_reproductive.json",
            include_str!("../../src/lib/marker-packs/hormones_reproductive.json"),
        ),
        (
            "skin_hair_dermatology.json",
            include_str!("../../src/lib/marker-packs/skin_hair_dermatology.json"),
        ),
        (
            "bone_growth_mineral_density.json",
            include_str!("../../src/lib/marker-packs/bone_growth_mineral_density.json"),
        ),
        (
            "kidney_fluid_electrolytes.json",
            include_str!("../../src/lib/marker-packs/kidney_fluid_electrolytes.json"),
        ),
        (
            "respiratory_airway.json",
            include_str!("../../src/lib/marker-packs/respiratory_airway.json"),
        ),
        (
            "immune_autoimmune_general.json",
            include_str!("../../src/lib/marker-packs/immune_autoimmune_general.json"),
        ),
        (
            "pain_migraine_sensory.json",
            include_str!("../../src/lib/marker-packs/pain_migraine_sensory.json"),
        ),
        (
            "dental_oral_health.json",
            include_str!("../../src/lib/marker-packs/dental_oral_health.json"),
        ),
        (
            "longevity_aging_resilience.json",
            include_str!("../../src/lib/marker-packs/longevity_aging_resilience.json"),
        ),
        (
            "discovery_catalog.json",
            include_str!("../../src/lib/marker-packs/discovery_catalog.json"),
        ),
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

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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
    REGISTRY.get_or_init(|| {
        Mutex::new(MarkerPackRegistry {
            packs: HashMap::new(),
            warnings: HashMap::new(),
        })
    })
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
    let p: ValidatePack =
        serde_json::from_str(content).map_err(|e| format!("JSON syntax error: {}", e))?;
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
            let mtime = metadata
                .modified()
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
                            reg.packs.insert(
                                "manifest".to_string(),
                                PackRegistryEntry { content, mtime },
                            );
                            reg.warnings.remove("manifest");
                        }
                        Err(e) => {
                            reg.warnings.insert(
                                "manifest".to_string(),
                                format!("Manifest validation failed: {}", e),
                            );
                        }
                    }
                }
            }
        }
    }

    // 2. Sync all pack JSONs listed in manifest
    let manifest_str = reg
        .packs
        .get("manifest")
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
                    let mtime = metadata
                        .modified()
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
                                    reg.packs.insert(
                                        pack.id.clone(),
                                        PackRegistryEntry { content, mtime },
                                    );
                                    reg.warnings.remove(&pack.id);
                                }
                                Err(e) => {
                                    reg.warnings.insert(
                                        pack.id.clone(),
                                        format!("Pack '{}' validation failed: {}", pack.id, e),
                                    );
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
        "neuropsych" => {
            Some(include_str!("../../src/lib/marker-packs/neuropsych.json").to_string())
        }
        "sleep" => Some(include_str!("../../src/lib/marker-packs/sleep.json").to_string()),
        "connective_tissue" => {
            Some(include_str!("../../src/lib/marker-packs/connective_tissue.json").to_string())
        }
        "thyroid_autoimmune" => {
            Some(include_str!("../../src/lib/marker-packs/thyroid_autoimmune.json").to_string())
        }
        "cardiovascular" => {
            Some(include_str!("../../src/lib/marker-packs/cardiovascular.json").to_string())
        }
        "cancer_confirmation_only" => Some(
            include_str!("../../src/lib/marker-packs/cancer_confirmation_only.json").to_string(),
        ),
        "allergy_atopy_mast_cell" => Some(
            include_str!("../../src/lib/marker-packs/allergy_atopy_mast_cell.json").to_string(),
        ),
        "digestive_gut_microbiome" => Some(
            include_str!("../../src/lib/marker-packs/digestive_gut_microbiome.json").to_string(),
        ),
        "muscle_performance_recovery" => Some(
            include_str!("../../src/lib/marker-packs/muscle_performance_recovery.json").to_string(),
        ),
        "hormones_reproductive" => {
            Some(include_str!("../../src/lib/marker-packs/hormones_reproductive.json").to_string())
        }
        "skin_hair_dermatology" => {
            Some(include_str!("../../src/lib/marker-packs/skin_hair_dermatology.json").to_string())
        }
        "bone_growth_mineral_density" => Some(
            include_str!("../../src/lib/marker-packs/bone_growth_mineral_density.json").to_string(),
        ),
        "kidney_fluid_electrolytes" => Some(
            include_str!("../../src/lib/marker-packs/kidney_fluid_electrolytes.json").to_string(),
        ),
        "respiratory_airway" => {
            Some(include_str!("../../src/lib/marker-packs/respiratory_airway.json").to_string())
        }
        "immune_autoimmune_general" => Some(
            include_str!("../../src/lib/marker-packs/immune_autoimmune_general.json").to_string(),
        ),
        "pain_migraine_sensory" => {
            Some(include_str!("../../src/lib/marker-packs/pain_migraine_sensory.json").to_string())
        }
        "dental_oral_health" => {
            Some(include_str!("../../src/lib/marker-packs/dental_oral_health.json").to_string())
        }
        "longevity_aging_resilience" => Some(
            include_str!("../../src/lib/marker-packs/longevity_aging_resilience.json").to_string(),
        ),
        "discovery_catalog" => {
            Some(include_str!("../../src/lib/marker-packs/discovery_catalog.json").to_string())
        }
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
        "ALTER TABLE qdrant_config ADD COLUMN vector_provider TEXT NOT NULL DEFAULT 'qdrant'",
        "ALTER TABLE qdrant_config ADD COLUMN namespace TEXT NOT NULL DEFAULT ''",
    ] {
        let _ = conn.execute(sql, []);
    }
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

pub fn seed_evidence_library(
    conn: &Connection,
    app_data_dir: Option<&Path>,
) -> std::result::Result<(), String> {
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

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to start seed transaction: {}", e))?;

    let total_packs = manifest.packs.len();
    for (idx, pack_info) in manifest.packs.iter().enumerate() {
        progress(&format!(
            "Seeding evidence library: {} ({} of {})...",
            pack_info.id,
            idx + 1,
            total_packs
        ));
        let pack_str = get_pack_str(app_data_dir, &pack_info.id);

        if let Some(p_str) = pack_str {
            let pack: Pack = serde_json::from_str(&p_str)
                .map_err(|e| format!("Failed to parse pack {}: {}", pack_info.id, e))?;

            for m in pack.markers {
                if let Some(ref sources) = m.sources
                    && !sources.is_empty()
                {
                    for src in sources {
                        let citation =
                            format!("{} ({})", src.name, src.url.as_deref().unwrap_or("No URL"));
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

    tx.commit()
        .map_err(|e| format!("Failed to commit seed transaction: {}", e))?;
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
fn map_chat_session_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(
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

    let selected_packs =
        serde_json::from_str(&selected_packs_str).unwrap_or(serde_json::Value::Null);
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

pub fn get_chat_session_by_id(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<DbChatSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, sample_id, title, timestamp, selected_packs, only_active_findings, 
                temperature, selected_model, max_tokens, extended_thinking, consultation_mode 
         FROM chat_sessions WHERE id = ?",
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

pub fn get_chat_sessions(
    conn: &Connection,
    sample_id_filter: Option<i64>,
) -> Result<Vec<DbChatSession>> {
    let mut stmt = if sample_id_filter.is_some() {
        conn.prepare(
            "SELECT id, sample_id, title, timestamp, selected_packs, only_active_findings, 
                    temperature, selected_model, max_tokens, extended_thinking, consultation_mode 
             FROM chat_sessions WHERE sample_id = ? ORDER BY timestamp DESC",
        )?
    } else {
        conn.prepare(
            "SELECT id, sample_id, title, timestamp, selected_packs, only_active_findings, 
                    temperature, selected_model, max_tokens, extended_thinking, consultation_mode 
             FROM chat_sessions ORDER BY timestamp DESC",
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

    let selected_packs_str =
        serde_json::to_string(&session.selected_packs).unwrap_or_else(|_| "{}".to_string());
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
    tx.execute(
        "DELETE FROM chat_messages WHERE session_id = ?",
        params![session.id],
    )?;

    // Insert new messages
    let mut stmt = tx.prepare(
        "INSERT INTO chat_messages (session_id, role, content, images, safety_review) 
         VALUES (?, ?, ?, ?, ?)",
    )?;

    for msg in &session.messages {
        let images_str = msg
            .images
            .as_ref()
            .map(|imgs| serde_json::to_string(imgs).unwrap_or_else(|_| "[]".to_string()));
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

pub fn delete_chat_session(conn: &Connection, session_id: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM chat_sessions WHERE id = ?",
        params![session_id],
    )
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
pub fn get_bootstrap_status(
    conn: &Connection,
    data_dir: &Path,
) -> Result<AppBootstrapStatus, String> {
    let count_query = |sql: &str| -> u64 {
        conn.query_row(sql, [], |row| row.get::<_, i64>(0))
            .unwrap_or(0) as u64
    };

    let samples = get_samples(conn).map_err(|e| e.to_string())?;
    let chain_path = crate::paths::chain_path(data_dir);
    let sample_table_count = |table: &str| {
        samples
            .iter()
            .filter_map(|sample| connect_sample(data_dir, sample.id).ok())
            .map(|sample_conn| {
                sample_conn
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap_or(0) as u64
            })
            .sum()
    };

    Ok(AppBootstrapStatus {
        data_dir: data_dir.to_string_lossy().to_string(),
        db_path: crate::paths::db_path(data_dir)
            .to_string_lossy()
            .to_string(),
        chain_path: chain_path.to_string_lossy().to_string(),
        chain_present: chain_path.exists(),
        env_path: crate::config::recommended_env_path()
            .to_string_lossy()
            .to_string(),
        sample_count: samples.len() as u64,
        genotype_count: sample_table_count("genotypes"),
        discovered_findings_count: sample_table_count("discovered_findings"),
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
            let trait_categories: Vec<String> =
                serde_json::from_str(&categories_json).unwrap_or_default();
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
