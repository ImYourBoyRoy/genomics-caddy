// ./src-tauri/src/offline/schema.rs
/*
Purpose: Offline/reference SQLite schema migrations that are safe when catalog DBs are absent.
*/

use rusqlite::{Connection, Result};

pub fn schema_attached(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM pragma_database_list WHERE name = ?1",
        rusqlite::params![name],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

fn ensure_column(
    conn: &Connection,
    schema: &str,
    table: &str,
    column: &str,
    ddl: &str,
) -> Result<()> {
    if !schema.is_empty() && !schema_attached(conn, schema) {
        return Ok(());
    }
    let pragma_sql = if schema.is_empty() {
        format!("PRAGMA table_info({table})")
    } else {
        format!("PRAGMA {schema}.table_info({table})")
    };
    let mut stmt = conn.prepare(&pragma_sql)?;
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    if !cols.iter().any(|c| c == column) {
        let alter_sql = if schema.is_empty() {
            format!("ALTER TABLE {table} ADD COLUMN {column} {ddl}")
        } else {
            format!("ALTER TABLE {schema}.{table} ADD COLUMN {column} {ddl}")
        };
        conn.execute(&alter_sql, [])?;
    }
    Ok(())
}

pub fn migrate_offline_schema(conn: &Connection) -> Result<()> {
    // Always-available schemas (attached on every connect).
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS reference.offline_asset_registry (
            asset_id TEXT PRIMARY KEY,
            tier INTEGER NOT NULL,
            local_path TEXT NOT NULL DEFAULT '',
            source_url TEXT NOT NULL DEFAULT '',
            remote_etag TEXT,
            remote_last_modified TEXT,
            remote_content_length INTEGER,
            local_sha256 TEXT,
            local_bytes INTEGER NOT NULL DEFAULT 0,
            row_count INTEGER NOT NULL DEFAULT 0,
            version_label TEXT,
            synced_at INTEGER NOT NULL DEFAULT 0,
            update_available INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS reference.pharmgkb_clinical_variants (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rsid TEXT NOT NULL,
            gene TEXT,
            drug TEXT,
            phenotype TEXT,
            evidence_level TEXT,
            raw_json TEXT
        );
        CREATE INDEX IF NOT EXISTS reference.idx_pharmgkb_rsid ON pharmgkb_clinical_variants(rsid);

        CREATE TABLE IF NOT EXISTS reference.pharmgkb_genes (
            pharmgkb_id TEXT PRIMARY KEY,
            symbol TEXT NOT NULL,
            name TEXT,
            raw_json TEXT
        );
        CREATE INDEX IF NOT EXISTS reference.idx_pharmgkb_genes_symbol ON pharmgkb_genes(symbol);

        CREATE TABLE IF NOT EXISTS reference.clingen_gene_validity (
            hgnc_id TEXT,
            gene_symbol TEXT NOT NULL,
            disease_label TEXT NOT NULL,
            classification TEXT,
            moi TEXT,
            report_url TEXT,
            PRIMARY KEY (gene_symbol, disease_label)
        );

        CREATE TABLE IF NOT EXISTS reference.mane_transcripts (
            gene_symbol TEXT PRIMARY KEY,
            ensembl_transcript TEXT,
            refseq_transcript TEXT,
            mane_status TEXT,
            grch38_coordinates TEXT
        );

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
        ",
    )?;

    // Catalog sidecars exist only after download/import — skip DDL until attached.
    if schema_attached(conn, "dbsnp") {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS dbsnp.rsid_aliases (
                rsid TEXT PRIMARY KEY,
                merged_into TEXT,
                withdrawn INTEGER NOT NULL DEFAULT 0,
                source TEXT NOT NULL DEFAULT 'dbsnp',
                raw_refsnp_id TEXT
            );
            CREATE TABLE IF NOT EXISTS dbsnp.refsnp_meta (
                refsnp_id TEXT PRIMARY KEY,
                create_date TEXT,
                last_update_date TEXT,
                last_update_build_id TEXT,
                citation_count INTEGER NOT NULL DEFAULT 0,
                citations_json TEXT NOT NULL DEFAULT '[]',
                mane_select_ids_json TEXT NOT NULL DEFAULT '[]'
            );
            CREATE INDEX IF NOT EXISTS dbsnp.idx_rsid_aliases_merged_into ON rsid_aliases(merged_into);
            CREATE INDEX IF NOT EXISTS dbsnp.idx_rsid_aliases_withdrawn ON rsid_aliases(withdrawn);
            CREATE INDEX IF NOT EXISTS dbsnp.idx_rsid_aliases_source ON rsid_aliases(source);
            ",
        )?;
        ensure_column(conn, "dbsnp", "rsid_aliases", "raw_refsnp_id", "TEXT")?;
    }

    if schema_attached(conn, "clinvar") {
        ensure_column(
            conn,
            "clinvar",
            "clinvar_reference",
            "review_status",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        ensure_column(conn, "clinvar", "clinvar_reference", "variation_id", "TEXT")?;
        ensure_column(
            conn,
            "clinvar",
            "clinvar_reference",
            "last_evaluated",
            "TEXT",
        )?;
        ensure_column(
            conn,
            "clinvar",
            "clinvar_reference",
            "assembly",
            "TEXT NOT NULL DEFAULT 'GRCh38'",
        )?;
    }

    Ok(())
}

pub fn table_count(conn: &Connection, table: &str) -> u64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get::<_, i64>(0)
    })
    .unwrap_or(0)
    .max(0) as u64
}
