// ./src-tauri/src/research/sources_config.rs
//! Per-source enrichment toggles for vector research sweeps (gnomAD, PubMed, etc.).

use super::types::{EnrichmentSourcesConfig, ResearchScopeConfig};
use serde_json::Value;
use std::sync::{OnceLock, RwLock};

static ACTIVE_SOURCES: OnceLock<RwLock<EnrichmentSourcesConfig>> = OnceLock::new();

fn store() -> &'static RwLock<EnrichmentSourcesConfig> {
    ACTIVE_SOURCES.get_or_init(|| RwLock::new(EnrichmentSourcesConfig::full()))
}

pub fn install_enrichment_sources(scope: &ResearchScopeConfig) {
    let effective = effective_sources_from_scope(scope);
    if let Ok(mut guard) = store().write() {
        *guard = effective;
    }
}

pub fn clear_enrichment_sources() {
    if let Ok(mut guard) = store().write() {
        *guard = EnrichmentSourcesConfig::full();
    }
}

pub fn active_enrichment_sources() -> EnrichmentSourcesConfig {
    store().read().ok().map(|g| g.clone()).unwrap_or_default()
}

pub fn effective_sources_from_scope(scope: &ResearchScopeConfig) -> EnrichmentSourcesConfig {
    if scope.sweep_fast {
        return EnrichmentSourcesConfig::fast_index();
    }
    scope.enrichment_sources.clone()
}

pub fn enrichment_fast_index_mode() -> bool {
    active_enrichment_sources().is_fast_index()
}

pub fn enrichment_gnomad_enabled() -> bool {
    active_enrichment_sources().gnomad
}

pub fn enrichment_clinvar_live_enabled() -> bool {
    active_enrichment_sources().clinvar_live
}

pub fn enrichment_pubmed_enabled() -> bool {
    active_enrichment_sources().pubmed
}

pub fn enrichment_gtex_enabled() -> bool {
    active_enrichment_sources().gtex
}

pub fn enrichment_vep_dbsnp_enabled() -> bool {
    active_enrichment_sources().vep_dbsnp
}

pub fn enrichment_secondary_enabled() -> bool {
    active_enrichment_sources().secondary
}

pub fn enrichment_gwas_api_supplement() -> bool {
    active_enrichment_sources().gwas_api_supplement
}

pub fn enrichment_supplement_missing() -> bool {
    active_enrichment_sources().supplement_missing
}

/// True when indexed payload still lacks data for any enabled source.
pub fn payload_needs_source_supplement(
    provenance: &Value,
    sources: &EnrichmentSourcesConfig,
) -> bool {
    let obj = provenance.as_object();
    if sources.gnomad && source_missing_or_skipped(obj, "gnomad") {
        return true;
    }
    if sources.clinvar_live && source_missing_or_skipped(obj, "clinvar_eutils") {
        return true;
    }
    if sources.pubmed && source_missing_or_skipped(obj, "pubmed") {
        return true;
    }
    if sources.gtex && source_missing_or_skipped(obj, "gtex") {
        return true;
    }
    if sources.vep_dbsnp
        && (source_missing_or_skipped(obj, "ensembl_vep")
            || source_missing_or_skipped(obj, "dbsnp"))
    {
        return true;
    }
    if sources.secondary {
        for key in [
            "pgs_catalog",
            "reactome",
            "open_targets",
            "pharmgkb",
            "ols4",
            "secondary_sources",
        ] {
            if source_missing_or_skipped(obj, key) {
                return true;
            }
        }
    }
    false
}

fn source_missing_or_skipped(obj: Option<&serde_json::Map<String, Value>>, key: &str) -> bool {
    match obj.and_then(|o| o.get(key)) {
        None => true,
        Some(v) => {
            if v.get("skipped").and_then(|s| s.as_bool()).unwrap_or(false) {
                return true;
            }
            if v.get("queried").and_then(|q| q.as_bool()) == Some(false) {
                return true;
            }
            if v.get("queried").and_then(|q| q.as_bool()) == Some(true) {
                return false;
            }
            true
        }
    }
}
