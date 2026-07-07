// ./src-tauri/src/research/evidence/normalize.rs
//! Qdrant payload normalization and structured embedding text.

use super::scoring::{
    association_strength_score, clinical_actionability_score, compute_personal_direction,
    data_quality_score, default_prohibited_claims, default_verification_ideas,
    personal_direction_label, personal_match_score, pvalue_mlog10, wellness_actionability_score,
};
use super::types::{EvidenceLedgerRow, EVIDENCE_SCHEMA_VERSION};
use crate::research::crossmap::CrossMapContext;
use crate::research::util::{best_gwas_pvalue, ENRICHMENT_VERSION, unix_now};
use serde_json::{json, Map, Value};

#[allow(clippy::too_many_arguments)]
pub fn build_structured_embedding_text(
    rsid: &str,
    genotype: &str,
    chromosome: Option<&str>,
    position: Option<i64>,
    trait_name: Option<&str>,
    trait_category: Option<&str>,
    gene: Option<&str>,
    gene_confidence: &str,
    evidence_tier: &str,
    association_type: &str,
    p_value: Option<f64>,
    personal_direction: &str,
    data_quality: f32,
    wellness_actionability: f32,
    source_names: &[String],
    quality_flags: &[String],
    missing_fields: &[String],
) -> String {
    format!(
        "[VARIANT]\nrsid={rsid}\ngenotype={genotype}\nchromosome={chr}\nposition_grch38={pos}\n\n\
         [TRAIT]\nreported_trait={trait}\ntrait_category={cat}\n\n\
         [GENE_MECHANISM]\nmapped_gene={gene}\ngene_confidence={gene_confidence}\n\n\
         [EVIDENCE]\nevidence_tier={evidence_tier}\nassociation_type={association_type}\n\
         p_value={pval}\n\n\
         [ACTIONABILITY]\nwellness_actionability_score={wellness:.3}\n\
         personal_direction={personal_direction}\n\n\
         [QUALITY]\ndata_quality_score={dq:.3}\nquality_flags={flags}\n\
         missing_fields={missing}\n\n\
         [SOURCES]\nsource_names={sources}",
        chr = chromosome.unwrap_or("unknown"),
        pos = position.map(|p| p.to_string()).unwrap_or_else(|| "unknown".into()),
        trait = trait_name.unwrap_or("unknown"),
        cat = trait_category.unwrap_or("unknown"),
        gene = gene.unwrap_or("unknown"),
        pval = p_value.map(|p| format!("{:.2e}", p)).unwrap_or_else(|| "unknown".into()),
        wellness = wellness_actionability,
        dq = data_quality,
        flags = quality_flags.join(","),
        missing = missing_fields.join(","),
        sources = source_names.join(","),
    )
}

pub fn build_evidence_ledger(
    gwas_associations: &[Value],
    genotype: &str,
    clinvar_sig: Option<&str>,
) -> Vec<EvidenceLedgerRow> {
    let mut rows: Vec<EvidenceLedgerRow> = gwas_associations
        .iter()
        .map(|a| super::scoring::ledger_from_gwas_assoc(a, genotype, "gwas_catalog"))
        .collect();
    if let Some(sig) = clinvar_sig.filter(|s| !s.is_empty()) {
        rows.push(EvidenceLedgerRow {
            source_name: "clinvar".into(),
            trait_name: None,
            mapped_gene: None,
            p_value: None,
            beta: None,
            odds_ratio: None,
            effect_allele: None,
            personal_dosage: None,
            personal_direction: "unknown".into(),
            study_accession: None,
            pubmed_id: None,
            ancestry: None,
            sample_size: None,
            fetched_at: None,
            source_release: None,
            raw_payload_hash: None,
            quality_flags: if sig.to_lowercase().contains("conflict") {
                vec!["clinvar_conflicting_interpretations".into()]
            } else {
                vec!["not_medical_actionable".into()]
            },
            evidence_tier: "clinical_assertion".into(),
            association_type: "clinvar_assertion".into(),
        });
    }
    rows
}

#[allow(clippy::too_many_arguments)]
pub fn extend_payload_normalized(
    payload: &mut Map<String, Value>,
    sample_id: i64,
    rsid: &str,
    genotype: &str,
    gene: &Option<String>,
    gene_confidence: &str,
    evidence_tier: &str,
    clinvar_sig: Option<&str>,
    gnomad_af: Option<f64>,
    gwas_associations: &[Value],
    crossmap: &CrossMapContext,
    chromosome: Option<&str>,
    position: Option<i64>,
    sources_provenance: &Value,
    embedding_model: &str,
    existing_text: &str,
) {
    let ledger = build_evidence_ledger(gwas_associations, genotype, clinvar_sig);
    let best_p = best_gwas_pvalue(gwas_associations);
    let gwas_trait_fallback = payload
        .get("gwas_traits")
        .and_then(|v| v.as_str())
        .and_then(|s| s.split(';').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let primary_trait = crossmap
        .association_summary
        .get("top")
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v["trait"].as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or(gwas_trait_fallback);

    let trait_category = crossmap.trait_categories.first().map(String::as_str);
    let source_names: Vec<String> = sources_provenance
        .as_object()
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();
    let primary_source = source_names.first().cloned();

    let mut missing_fields = Vec::new();
    if gene.is_none() {
        missing_fields.push("gene_symbol".into());
    }
    if primary_trait.is_none() {
        missing_fields.push("trait_name".into());
    }
    if best_p.is_none() {
        missing_fields.push("p_value".into());
    }
    if chromosome.is_none() {
        missing_fields.push("chromosome".into());
    }
    if position.is_none() {
        missing_fields.push("position_grch38".into());
    }

    let mut quality_flags: Vec<String> = ledger
        .iter()
        .flat_map(|r| r.quality_flags.clone())
        .collect();
    quality_flags.sort();
    quality_flags.dedup();
    if clinvar_sig.map(|s| s.to_lowercase().contains("conflict")).unwrap_or(false) {
        quality_flags.push("source_conflict".into());
    }
    quality_flags.push("gwas_only_not_clinical".into());

    let conflict_count = ledger
        .iter()
        .flat_map(|r| r.quality_flags.iter())
        .filter(|f| f.contains("conflict"))
        .count() as u32;

    let best_direction_row = ledger.iter().find(|r| {
        matches!(
            r.personal_direction.as_str(),
            "increased_trait_value"
                | "decreased_trait_value"
                | "increased_odds"
                | "decreased_odds"
        )
    });
    let (personal_direction, personal_dosage, _) = if let Some(row) = best_direction_row {
        (
            row.personal_direction.clone(),
            row.personal_dosage,
            row.quality_flags.clone(),
        )
    } else {
        compute_personal_direction(genotype, None, None, None, None)
    };
    let has_direction = matches!(
        personal_direction.as_str(),
        "increased_trait_value"
            | "decreased_trait_value"
            | "increased_odds"
            | "decreased_odds"
    );
    let has_effect_allele = ledger.iter().any(|r| r.effect_allele.is_some());
    let has_effect_size = ledger.iter().any(|r| r.beta.is_some() || r.odds_ratio.is_some());

    let has_gwas = !gwas_associations.is_empty();
    let has_clinvar = clinvar_sig.map(|s| !s.is_empty()).unwrap_or(false);
    let has_gtex = sources_provenance
        .get("gtex")
        .and_then(|v| v.get("hits"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        > 0;
    let has_pubmed = sources_provenance
        .get("pubmed")
        .and_then(|v| v.get("hits"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        > 0;
    let has_pgs = sources_provenance
        .get("pgs_catalog")
        .and_then(|v| v.get("hit"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_pharmgkb = sources_provenance
        .get("pharmgkb")
        .and_then(|v| v.get("hit"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_reactome = sources_provenance
        .get("reactome")
        .and_then(|v| v.get("hit"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let assoc_strength = association_strength_score(best_p, 0.8);
    let dq = data_quality_score(
        &missing_fields,
        &quality_flags,
        source_names.len() as u32,
        false,
    );
    let wellness = wellness_actionability_score(
        &crossmap.trait_categories,
        dq,
        !gwas_associations.is_empty(),
    );
    let clinical = clinical_actionability_score(clinvar_sig, has_clinvar, conflict_count);
    let personal_match = personal_match_score(
        Some(genotype),
        best_direction_row.and_then(|r| r.effect_allele.as_deref()),
        personal_dosage,
    );

    let structured_text = build_structured_embedding_text(
        rsid,
        genotype,
        chromosome,
        position,
        primary_trait.as_deref(),
        trait_category,
        gene.as_deref(),
        gene_confidence,
        evidence_tier,
        if gwas_associations.is_empty() {
            "literature_context"
        } else {
            "gwas_top_association"
        },
        best_p,
        &personal_direction,
        dq,
        wellness,
        &source_names,
        &quality_flags,
        &missing_fields,
    );

    let full_text = if existing_text.contains("[VARIANT]") {
        existing_text.to_string()
    } else {
        format!("{}\n\n{}", structured_text, existing_text)
    };

    payload.insert("schema_version".into(), json!(EVIDENCE_SCHEMA_VERSION));
    payload.insert("entity_type".into(), json!("variant_association"));
    payload.insert("trait_name".into(), json!(primary_trait));
    if payload.get("trait_ontology_id").is_none() {
        payload.insert("trait_ontology_id".into(), json!(null));
    }
    if payload.get("trait_name_mapped").is_none() {
        payload.insert("trait_name_mapped".into(), json!(primary_trait));
    }
    payload.insert(
        "trait_category".into(),
        json!(trait_category.unwrap_or("unknown")),
    );
    payload.insert("source_names".into(), json!(source_names));
    payload.insert("primary_source".into(), json!(primary_source));
    payload.insert("association_type".into(), json!("gwas_top_association"));
    payload.insert("p_value_min".into(), json!(best_p));
    payload.insert(
        "p_value_mlog10_max".into(),
        json!(pvalue_mlog10(best_p)),
    );
    payload.insert("has_effect_allele".into(), json!(has_effect_allele));
    payload.insert("has_effect_size".into(), json!(has_effect_size));
    payload.insert("has_direction".into(), json!(has_direction));
    payload.insert("has_gwas".into(), json!(has_gwas));
    payload.insert("has_clinvar".into(), json!(has_clinvar));
    payload.insert("has_gtex".into(), json!(has_gtex));
    payload.insert("has_pubmed".into(), json!(has_pubmed));
    payload.insert("has_pgs".into(), json!(has_pgs));
    payload.insert("has_pharmgkb".into(), json!(has_pharmgkb));
    payload.insert("has_reactome".into(), json!(has_reactome));
    payload.insert("personal_genotype_matched".into(), json!(true));
    payload.insert("personal_direction".into(), json!(personal_direction));
    payload.insert(
        "directionality_confidence".into(),
        json!(if has_direction { 0.85 } else { 0.0 }),
    );
    payload.insert("association_strength_score".into(), json!(assoc_strength));
    payload.insert("personal_match_score".into(), json!(personal_match));
    payload.insert(
        "clinical_actionability_score".into(),
        json!(clinical),
    );
    payload.insert(
        "wellness_actionability_score".into(),
        json!(wellness),
    );
    payload.insert("data_quality_score".into(), json!(dq));
    payload.insert("novelty_score".into(), json!(0.5));
    payload.insert("source_count".into(), json!(source_names.len()));
    payload.insert("conflict_count".into(), json!(conflict_count));
    payload.insert(
        "missing_field_count".into(),
        json!(missing_fields.len()),
    );
    payload.insert("quality_flags".into(), json!(quality_flags));
    payload.insert("missing_fields".into(), json!(missing_fields));
    payload.insert("embedding_model".into(), json!(embedding_model));
    payload.insert("embedding_version".into(), json!(ENRICHMENT_VERSION));
    payload.insert("last_vectorized".into(), json!(unix_now()));
    payload.insert("stale".into(), json!(false));
    payload.insert(
        "verification_ideas".into(),
        json!(default_verification_ideas(trait_category)),
    );
    payload.insert(
        "prohibited_claims".into(),
        json!(default_prohibited_claims()),
    );
    payload.insert("text".into(), json!(full_text));
    payload.insert(
        "directionality_label".into(),
        json!(personal_direction_label(&personal_direction)),
    );
    payload.insert("evidence_ledger_json".into(), json!(ledger));
    payload.insert("sample_id".into(), json!(sample_id));
    if let Some(af) = gnomad_af {
        payload.insert("gnomad_af".into(), json!(af));
    }
    let raw_hash = super::source_records::payload_hash(
        &serde_json::to_string(payload).unwrap_or_default(),
    );
    payload.insert("raw_payload_hash".into(), json!(raw_hash));
}
