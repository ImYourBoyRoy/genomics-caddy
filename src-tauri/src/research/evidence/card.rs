// ./src-tauri/src/research/evidence/card.rs
//! Build structured evidence cards from Qdrant payloads.

use super::scoring::{is_stale_vector, personal_direction_label};
use super::types::{EvidenceCard, EvidenceLedgerRow, MatchExplanation};
use crate::research::types::QdrantHit;
use crate::research::util::string_to_u64;
use serde_json::Value;

pub fn evidence_card_from_hit(
    hit: &QdrantHit,
    point_id: Option<u64>,
    explanation: Option<MatchExplanation>,
) -> EvidenceCard {
    let p = hit_to_payload(hit);
    evidence_card_from_payload(&p, hit.score, explanation, point_id)
}

pub fn evidence_card_from_payload(
    p: &Value,
    semantic_score: f32,
    explanation: Option<MatchExplanation>,
    point_id: Option<u64>,
) -> EvidenceCard {
    let rsid = p["rsid"].as_str().unwrap_or("").to_string();
    let personal_direction = p["personal_direction"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let has_direction = p["has_direction"].as_bool().unwrap_or(false);
    let quality_flags = json_str_array(p, "quality_flags");
    let missing_fields = json_str_array(p, "missing_fields");
    let source_names = json_str_array(p, "source_names");
    let source_names = if source_names.is_empty() {
        p.get("sources_used")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        source_names
    };

    let ledger: Vec<EvidenceLedgerRow> = p
        .get("evidence_ledger_json")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let verification_ideas = json_str_array(p, "verification_ideas");
    let verification_ideas = if verification_ideas.is_empty() {
        super::scoring::default_verification_ideas(p["trait_category"].as_str())
    } else {
        verification_ideas
    };

    let prohibited_claims = json_str_array(p, "prohibited_claims");
    let prohibited_claims = if prohibited_claims.is_empty() {
        super::scoring::default_prohibited_claims()
    } else {
        prohibited_claims
    };

    EvidenceCard {
        rsid: rsid.clone(),
        genotype: opt_str(p, "genotype"),
        chromosome: opt_str(p, "chromosome"),
        position_grch38: p
            .get("position_grch38")
            .or_else(|| p.get("position"))
            .and_then(|v| v.as_i64()),
        gene_symbol: opt_str(p, "gene_symbol").or_else(|| opt_str(p, "gene")),
        gene_confidence: opt_str(p, "gene_confidence"),
        primary_trait: opt_str(p, "trait_name")
            .or_else(|| opt_str(p, "primary_trait"))
            .or_else(|| opt_str(p, "gwas_trait")),
        trait_category: opt_str(p, "trait_category").or_else(|| {
            p.get("trait_categories")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .map(String::from)
        }),
        trait_categories: p
            .get("trait_categories")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        evidence_tier: opt_str(p, "evidence_tier"),
        association_type: opt_str(p, "association_type"),
        personal_direction: personal_direction.clone(),
        directionality_confidence: p["directionality_confidence"].as_f64().unwrap_or(0.0) as f32,
        directionality_label: p["directionality_label"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| personal_direction_label(&personal_direction)),
        has_direction,
        p_value_min: p["p_value_min"]
            .as_f64()
            .or_else(|| p["gwas_best_pvalue"].as_f64()),
        gwas_best_pvalue: p["gwas_best_pvalue"].as_f64(),
        association_strength_score: p["association_strength_score"].as_f64().unwrap_or(0.0) as f32,
        personal_match_score: p["personal_match_score"].as_f64().unwrap_or(0.5) as f32,
        clinical_actionability_score: p["clinical_actionability_score"].as_f64().unwrap_or(0.0)
            as f32,
        wellness_actionability_score: p["wellness_actionability_score"].as_f64().unwrap_or(0.0)
            as f32,
        data_quality_score: p["data_quality_score"].as_f64().unwrap_or(0.5) as f32,
        novelty_score: p["novelty_score"].as_f64().unwrap_or(0.5) as f32,
        source_names,
        primary_source: opt_str(p, "primary_source"),
        source_count: p["source_count"].as_u64().unwrap_or(0) as u32,
        conflict_count: p["conflict_count"].as_u64().unwrap_or(0) as u32,
        missing_field_count: p["missing_field_count"]
            .as_u64()
            .unwrap_or(missing_fields.len() as u64) as u32,
        quality_flags,
        missing_fields,
        has_gwas: p["has_gwas"].as_bool().unwrap_or(false),
        has_clinvar: p["has_clinvar"].as_bool().unwrap_or(false),
        has_pgs: p["has_pgs"].as_bool().unwrap_or(false),
        has_pharmgkb: p["has_pharmgkb"].as_bool().unwrap_or(false),
        has_reactome: p["has_reactome"].as_bool().unwrap_or(false),
        clinvar_significance: opt_str(p, "clinvar_significance"),
        gnomad_af: p["gnomad_af"].as_f64(),
        enrichment_version: opt_str(p, "enrichment_version"),
        schema_version: opt_str(p, "schema_version"),
        stale: p["stale"].as_bool().unwrap_or_else(|| {
            is_stale_vector(
                p["enrichment_version"].as_str(),
                p["schema_version"].as_str(),
            )
        }),
        semantic_score,
        synthesis: p["text"].as_str().unwrap_or("").chars().take(400).collect(),
        association_summary: p.get("association_summary").cloned(),
        gene_candidates: p.get("gene_candidates").and_then(|v| v.as_array()).cloned(),
        sources_provenance: p.get("sources_provenance").cloned(),
        cross_refs: p.get("cross_refs").cloned(),
        evidence_ledger: ledger,
        match_explanation: explanation,
        verification_ideas,
        prohibited_claims,
        qdrant_point_id: point_id.map(|id| id.to_string()).or_else(|| {
            Some(
                string_to_u64(&format!(
                    "{}_{}_{}",
                    p["sample_id"].as_i64().unwrap_or(0),
                    rsid,
                    opt_str(p, "genotype")
                        .unwrap_or_default()
                        .split('/')
                        .next()
                        .unwrap_or("")
                ))
                .to_string(),
            )
        }),
    }
}

fn hit_to_payload(hit: &QdrantHit) -> Value {
    serde_json::json!({
        "rsid": hit.rsid,
        "genotype": hit.genotype,
        "gene_symbol": hit.gene,
        "gene_confidence": hit.gene_confidence,
        "evidence_tier": hit.category,
        "text": hit.text,
        "gwas_traits": hit.gwas_trait,
        "gwas_trait": hit.gwas_trait,
        "gwas_associations": hit.gwas_associations,
        "gnomad_af": hit.gnomad_af,
        "clinvar_significance": hit.clinvar_significance,
        "chromosome": hit.chromosome,
        "position": hit.position,
        "gene_candidates": hit.gene_candidates,
        "sources_provenance": hit.sources_provenance,
        "cross_refs": hit.cross_refs,
        "trait_categories": hit.trait_categories,
        "association_summary": hit.association_summary,
        "enrichment_version": hit.enrichment_version,
        "significance_score": hit.significance_score,
    })
}

fn opt_str(p: &Value, key: &str) -> Option<String> {
    p.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn json_str_array(p: &Value, key: &str) -> Vec<String> {
    p.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
