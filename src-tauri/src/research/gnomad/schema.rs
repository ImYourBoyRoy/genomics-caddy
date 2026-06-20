// ./src-tauri/src/research/gnomad/schema.rs
use rusqlite::{Connection, Result};

fn ensure_column(conn: &Connection, table: &str, column: &str, ddl: &str) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    if !cols.iter().any(|c| c == column) {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {ddl}"), [])?;
    }
    Ok(())
}

pub fn migrate_gnomad_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS gnomad_variant_cache (
            cache_id TEXT PRIMARY KEY,
            release TEXT NOT NULL,
            source_mode TEXT NOT NULL,
            dataset TEXT NOT NULL,
            chrom TEXT NOT NULL,
            pos INTEGER NOT NULL,
            ref TEXT NOT NULL,
            alt TEXT NOT NULL,
            variant_id TEXT,
            rsids_json TEXT,
            genotype TEXT,
            user_allele_match_status TEXT,
            ac INTEGER,
            an INTEGER,
            af REAL,
            ac_exomes INTEGER,
            an_exomes INTEGER,
            af_exomes REAL,
            ac_genomes INTEGER,
            an_genomes INTEGER,
            af_genomes REAL,
            popmax REAL,
            popmax_population TEXT,
            faf95_popmax REAL,
            faf95_popmax_population TEXT,
            homozygote_count INTEGER,
            hemizygote_count INTEGER,
            filters_json TEXT,
            flags_json TEXT,
            info_json TEXT,
            source_url TEXT,
            source_file TEXT,
            source_index TEXT,
            fetched_at INTEGER NOT NULL,
            raw_record_hash TEXT,
            parser_version TEXT NOT NULL,
            lookup_status TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_gnomad_cache_coords
            ON gnomad_variant_cache(release, dataset, chrom, pos, ref, alt);
        CREATE INDEX IF NOT EXISTS idx_gnomad_cache_locus
            ON gnomad_variant_cache(release, pos);
        CREATE INDEX IF NOT EXISTS idx_gnomad_cache_rsid
            ON gnomad_variant_cache(release, rsids_json);

        CREATE TABLE IF NOT EXISTS gnomad_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            enabled INTEGER NOT NULL DEFAULT 1,
            source_mode TEXT NOT NULL DEFAULT 'remote_indexed_vcf_https',
            release TEXT NOT NULL DEFAULT '4.1',
            provider TEXT NOT NULL DEFAULT 'aws',
            local_vcf_dir TEXT,
            max_remote_concurrent_files INTEGER NOT NULL DEFAULT 2,
            max_queries_per_second REAL NOT NULL DEFAULT 4.0,
            graphql_enabled_for_sweep INTEGER NOT NULL DEFAULT 0,
            exome_template TEXT NOT NULL,
            genome_template TEXT NOT NULL
        );
        ",
    )?;

    ensure_column(conn, "gnomad_config", "graphql_fallback_enabled", "INTEGER NOT NULL DEFAULT 1")?;
    ensure_column(conn, "gnomad_config", "dataset_policy", "TEXT NOT NULL DEFAULT 'auto'")?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM gnomad_config WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    if count == 0 {
        let defaults = super::types::GnomadConfig::default();
        conn.execute(
            "INSERT INTO gnomad_config (
                id, enabled, source_mode, release, provider, local_vcf_dir,
                max_remote_concurrent_files, max_queries_per_second,
                graphql_enabled_for_sweep, graphql_fallback_enabled, dataset_policy,
                exome_template, genome_template
            ) VALUES (1, 1, ?, ?, ?, NULL, ?, ?, 0, 1, ?, ?, ?)",
            rusqlite::params![
                defaults.source_mode.as_str(),
                defaults.release,
                "aws",
                defaults.max_remote_concurrent_files as i64,
                defaults.max_queries_per_second,
                defaults.dataset_policy.as_str(),
                defaults.exome_template,
                defaults.genome_template,
            ],
        )?;
    }
    Ok(())
}
