// ./src-tauri/src/research/evidence/schema.rs
//! SQLite schema for normalized association evidence, source cache, and candidate expansion.

use rusqlite::{Connection, Result};

pub fn migrate_evidence_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS reference.source_records (
            source_record_id TEXT PRIMARY KEY,
            source_name TEXT NOT NULL,
            endpoint_family TEXT NOT NULL,
            source_entity_type TEXT,
            source_entity_id TEXT,
            rsid TEXT,
            gene_symbol TEXT,
            trait_name TEXT,
            trait_ontology_id TEXT,
            study_accession TEXT,
            pubmed_id TEXT,
            source_url TEXT,
            source_release TEXT,
            fetched_at INTEGER NOT NULL,
            raw_payload_hash TEXT NOT NULL,
            parsed_payload_hash TEXT,
            parser_version TEXT,
            schema_version_seen TEXT,
            license_note TEXT,
            record_quality_flags_json TEXT
        );
        CREATE INDEX IF NOT EXISTS reference.idx_source_records_rsid ON source_records(rsid);
        CREATE INDEX IF NOT EXISTS reference.idx_source_records_source ON source_records(source_name, rsid);

        CREATE TABLE IF NOT EXISTS reference.api_cache_entries (
            cache_id TEXT PRIMARY KEY,
            source_name TEXT NOT NULL,
            endpoint_family TEXT NOT NULL,
            request_method TEXT NOT NULL,
            request_url TEXT NOT NULL,
            normalized_cache_key TEXT NOT NULL UNIQUE,
            request_params_json TEXT,
            request_body_hash TEXT,
            response_status INTEGER,
            response_headers_json TEXT,
            response_content_type TEXT,
            response_body TEXT,
            raw_payload_hash TEXT,
            parsed_payload_hash TEXT,
            fetched_at INTEGER NOT NULL,
            expires_at INTEGER,
            source_release TEXT,
            etag TEXT,
            last_modified TEXT,
            error_count INTEGER DEFAULT 0,
            last_error TEXT,
            parser_version TEXT,
            schema_version_seen TEXT,
            cache_status TEXT NOT NULL DEFAULT 'fresh'
        );
        CREATE INDEX IF NOT EXISTS reference.idx_api_cache_key ON api_cache_entries(normalized_cache_key);
        CREATE INDEX IF NOT EXISTS reference.idx_api_cache_expires ON api_cache_entries(expires_at);

        CREATE TABLE IF NOT EXISTS association_facts (
            association_id TEXT PRIMARY KEY,
            sample_id INTEGER,
            rsid TEXT NOT NULL,
            genotype TEXT,
            canonical_variant_id TEXT,
            source_name TEXT NOT NULL,
            source_record_id TEXT,
            source_url TEXT,
            source_release TEXT,
            fetched_at INTEGER,
            raw_payload_hash TEXT,
            association_type TEXT NOT NULL,
            evidence_tier TEXT NOT NULL,
            trait_name_reported TEXT,
            trait_name_mapped TEXT,
            trait_ontology_id TEXT,
            trait_category TEXT,
            mapped_gene_symbol TEXT,
            reported_gene_symbols_json TEXT,
            ensembl_gene_ids_json TEXT,
            pubmed_id TEXT,
            study_accession TEXT,
            p_value REAL,
            p_value_mlog10 REAL,
            beta REAL,
            odds_ratio REAL,
            hazard_ratio REAL,
            confidence_interval_low REAL,
            confidence_interval_high REAL,
            effect_allele TEXT,
            other_allele TEXT,
            risk_allele TEXT,
            risk_allele_frequency REAL,
            effect_direction_raw TEXT,
            effect_direction_normalized TEXT,
            personal_effect_allele_dosage INTEGER,
            personal_direction TEXT,
            directionality_confidence REAL,
            sample_size INTEGER,
            ancestry_initial TEXT,
            ancestry_replication TEXT,
            cohort TEXT,
            association_strength_score REAL,
            personal_match_score REAL,
            clinical_actionability_score REAL,
            wellness_actionability_score REAL,
            data_quality_score REAL,
            novelty_score REAL,
            quality_flags_json TEXT,
            model_inferred INTEGER DEFAULT 0,
            created_at INTEGER,
            updated_at INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_assoc_facts_sample_rsid ON association_facts(sample_id, rsid);
        CREATE INDEX IF NOT EXISTS idx_assoc_facts_rsid ON association_facts(rsid);
        CREATE INDEX IF NOT EXISTS idx_assoc_facts_trait_cat ON association_facts(trait_category);

        CREATE TABLE IF NOT EXISTS candidate_marker_expansion (
            candidate_id TEXT PRIMARY KEY,
            rsid TEXT NOT NULL,
            gene_symbol TEXT,
            trait_name TEXT,
            trait_category TEXT,
            reason_surfaced TEXT,
            source_count INTEGER,
            association_count INTEGER,
            known_direction_count INTEGER,
            unknown_direction_count INTEGER,
            best_evidence_tier TEXT,
            best_source_name TEXT,
            best_p_value REAL,
            has_clinvar INTEGER,
            has_gwas INTEGER,
            has_pgs INTEGER,
            has_pharmgkb INTEGER,
            has_gtex INTEGER,
            has_pubmed INTEGER,
            association_strength_score REAL,
            clinical_actionability_score REAL,
            wellness_actionability_score REAL,
            data_quality_score REAL,
            novelty_score REAL,
            status TEXT NOT NULL DEFAULT 'unreviewed_dynamic',
            evidence_packet_id TEXT,
            created_at INTEGER,
            updated_at INTEGER,
            reviewed_at INTEGER,
            reviewer_note TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_candidate_rsid ON candidate_marker_expansion(rsid);
        CREATE INDEX IF NOT EXISTS idx_candidate_status ON candidate_marker_expansion(status);

        CREATE TABLE IF NOT EXISTS hypothesis_clusters (
            cluster_id TEXT PRIMARY KEY,
            sample_id INTEGER,
            cluster_title TEXT,
            cluster_type TEXT,
            trait_category TEXT,
            top_traits_json TEXT,
            top_genes_json TEXT,
            top_rsids_json TEXT,
            top_pathways_json TEXT,
            association_ids_json TEXT,
            source_count INTEGER,
            association_count INTEGER,
            known_direction_count INTEGER,
            unknown_direction_count INTEGER,
            conflict_count INTEGER,
            missing_field_count INTEGER,
            association_strength_score REAL,
            clinical_actionability_score REAL,
            wellness_actionability_score REAL,
            data_quality_score REAL,
            novelty_score REAL,
            verification_ideas_json TEXT,
            low_risk_actions_json TEXT,
            medical_escalation_boundary TEXT,
            evidence_packet_id TEXT,
            created_at INTEGER,
            updated_at INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_clusters_sample ON hypothesis_clusters(sample_id);

        CREATE TABLE IF NOT EXISTS pgs_match_cache (
            sample_id INTEGER NOT NULL,
            pgs_id TEXT NOT NULL,
            trait_reported TEXT,
            total_variants INTEGER,
            matched_variants INTEGER,
            missing_variants INTEGER,
            ambiguous_variants INTEGER,
            match_rate_pct REAL,
            updated_at INTEGER,
            PRIMARY KEY (sample_id, pgs_id)
        );

        CREATE TABLE IF NOT EXISTS vector_atlas_points (
            atlas_id TEXT PRIMARY KEY,
            sample_id INTEGER NOT NULL,
            rsid TEXT NOT NULL,
            qdrant_point_id TEXT,
            x REAL NOT NULL,
            y REAL NOT NULL,
            trait_category TEXT,
            gene_symbol TEXT,
            data_quality_score REAL,
            projection_method TEXT NOT NULL DEFAULT 'umap',
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_atlas_sample ON vector_atlas_points(sample_id);
        ",
    )?;
    Ok(())
}
