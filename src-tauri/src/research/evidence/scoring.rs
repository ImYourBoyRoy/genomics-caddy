// ./src-tauri/src/research/evidence/scoring.rs
//! Separated association scores — never collapse into a single risk score.

use super::types::EvidenceLedgerRow;
use crate::research::util::is_current_enrichment_version;

pub fn pvalue_mlog10(p: Option<f64>) -> Option<f64> {
    p.filter(|v| *v > 0.0).map(|v| -v.log10())
}

pub fn association_strength_score(p_value: Option<f64>, source_tier_weight: f32) -> f32 {
    let mut score = 0.0f32;
    if let Some(p) = p_value {
        if p < 5e-8 {
            score += 0.55;
        } else if p < 1e-5 {
            score += 0.35;
        } else if p < 1e-3 {
            score += 0.15;
        }
    }
    score += source_tier_weight * 0.25;
    score.min(1.0)
}

pub fn personal_match_score(
    genotype: Option<&str>,
    effect_allele: Option<&str>,
    personal_dosage: Option<i32>,
) -> f32 {
    let gt = genotype.unwrap_or("");
    if gt.is_empty() || gt.contains("--") {
        return 0.0;
    }
    if personal_dosage.is_some() {
        return 0.85;
    }
    if effect_allele.is_some() {
        return 0.35;
    }
    0.5
}

pub fn directionality_confidence(
    effect_allele: Option<&str>,
    beta: Option<f64>,
    odds_ratio: Option<f64>,
    has_conflict: bool,
) -> f32 {
    if has_conflict {
        return 0.0;
    }
    if effect_allele.is_none() {
        return 0.0;
    }
    if beta.is_some() || odds_ratio.is_some() {
        return 0.85;
    }
    0.15
}

pub fn clinical_actionability_score(
    clinvar_sig: Option<&str>,
    has_clinvar: bool,
    conflict_count: u32,
) -> f32 {
    if conflict_count > 0 {
        return 0.1;
    }
    let Some(sig) = clinvar_sig else {
        return if has_clinvar { 0.2 } else { 0.0 };
    };
    let lower = sig.to_lowercase();
    let mut score = 0.0f32;
    if lower.contains("pathogenic") && !lower.contains("likely") {
        score += 0.7;
    } else if lower.contains("likely pathogenic") {
        score += 0.55;
    } else if lower.contains("risk factor") || lower.contains("association") {
        score += 0.25;
    }
    if lower.contains("conflicting") {
        score *= 0.2;
    }
    score.min(1.0)
}

pub fn wellness_actionability_score(
    trait_categories: &[String],
    data_quality: f32,
    has_gwas: bool,
) -> f32 {
    if !has_gwas {
        return 0.0;
    }
    let mut score = data_quality * 0.45;
    if trait_categories.iter().any(|c| {
        matches!(
            c.as_str(),
            "nutrients" | "metabolic" | "sleep" | "bone_density" | "connective_tissue"
        )
    }) {
        score += 0.25;
    }
    score.min(1.0)
}

pub fn data_quality_score(
    missing_fields: &[String],
    quality_flags: &[String],
    source_count: u32,
    stale: bool,
) -> f32 {
    let mut score = 1.0f32;
    score -= (missing_fields.len() as f32 * 0.06).min(0.45);
    if quality_flags.iter().any(|f| f.contains("conflict")) {
        score -= 0.2;
    }
    if quality_flags.iter().any(|f| f == "gwas_only_not_clinical") {
        score -= 0.05;
    }
    if source_count == 0 {
        score -= 0.35;
    }
    if stale {
        score -= 0.15;
    }
    score.clamp(0.0, 1.0)
}

pub fn novelty_score(in_curated_pack: bool, in_vector_promoted: bool) -> f32 {
    if in_curated_pack {
        0.1
    } else if in_vector_promoted {
        0.55
    } else {
        0.75
    }
}

pub fn compute_personal_direction(
    genotype: &str,
    effect_allele: Option<&str>,
    risk_allele: Option<&str>,
    beta: Option<f64>,
    odds_ratio: Option<f64>,
) -> (String, Option<i32>, Vec<String>) {
    let mut flags = Vec::new();
    let allele = effect_allele.or(risk_allele);
    let Some(ea) = allele.filter(|a| !a.is_empty() && *a != "?") else {
        flags.push("missing_effect_allele".into());
        return ("unknown".into(), None, flags);
    };
    let normalized_genotype = genotype.trim().to_uppercase();
    if normalized_genotype.is_empty() || normalized_genotype.contains("--") {
        flags.push("genotype_no_call".into());
        return ("unknown".into(), None, flags);
    }
    let parts: Vec<&str> = genotype.split('/').collect();
    if parts.len() != 2 {
        return ("unknown".into(), None, flags);
    }
    let has_known_base = parts.iter().any(|part| {
        part.trim()
            .chars()
            .any(|base| matches!(base.to_ascii_uppercase(), 'A' | 'C' | 'G' | 'T'))
    });
    let has_unknown_base = parts.iter().any(|part| {
        let token = part.trim().to_uppercase();
        token.is_empty()
            || token.contains('-')
            || token.contains('?')
            || token.contains('0')
            || token.contains('N')
    });
    if has_unknown_base {
        flags.push(if has_known_base {
            "genotype_ambiguous_call"
        } else {
            "genotype_no_call"
        }
        .into());
        return ("unknown".into(), None, flags);
    }
    let ea_upper = ea.to_uppercase();
    let dosage = parts
        .iter()
        .filter(|a| a.to_uppercase() == ea_upper)
        .count() as i32;
    if dosage == 0 {
        return ("no_personal_match".into(), Some(0), flags);
    }
    if beta.is_none() && odds_ratio.is_none() {
        flags.push("missing_effect_size".into());
        return (
            "possible_relevance_unknown_direction".into(),
            Some(dosage),
            flags,
        );
    }
    if let Some(b) = beta {
        if b > 0.0 {
            return ("increased_trait_value".into(), Some(dosage), flags);
        }
        if b < 0.0 {
            return ("decreased_trait_value".into(), Some(dosage), flags);
        }
    }
    if let Some(or) = odds_ratio {
        if or > 1.0 {
            return ("increased_odds".into(), Some(dosage), flags);
        }
        if or < 1.0 {
            return ("decreased_odds".into(), Some(dosage), flags);
        }
    }
    (
        "possible_relevance_unknown_direction".into(),
        Some(dosage),
        flags,
    )
}

pub fn personal_direction_label(code: &str) -> String {
    match code {
        "increased_trait_value" => {
            "Possible increased trait value (requires effect size context)".into()
        }
        "decreased_trait_value" => {
            "Possible decreased trait value (requires effect size context)".into()
        }
        "increased_odds" => "Possible increased odds (association context only)".into(),
        "decreased_odds" => "Possible decreased odds (association context only)".into(),
        "possible_relevance_unknown_direction" => {
            "Association found; personal direction unknown".into()
        }
        "no_personal_match" => "Effect allele not present in your genotype".into(),
        "unknown" => "Direction unknown — insufficient structured effect data".into(),
        other => other.replace('_', " "),
    }
}

pub fn is_stale_vector(enrichment_version: Option<&str>, schema_version: Option<&str>) -> bool {
    !is_current_enrichment_version(enrichment_version)
        || schema_version
            .map(|v| v != super::types::EVIDENCE_SCHEMA_VERSION)
            .unwrap_or(true)
}

pub fn default_prohibited_claims() -> Vec<String> {
    vec![
        "Do not infer diagnosis from GWAS association alone.".into(),
        "Do not prescribe medications or supplements from SNP data.".into(),
        "Do not treat mapped_gene as causal_gene.".into(),
        "Do not infer effect direction from p-value alone.".into(),
        "Semantic similarity is not biological causality.".into(),
    ]
}

pub fn default_verification_ideas(trait_category: Option<&str>) -> Vec<String> {
    let mut ideas = vec![
        "Review primary GWAS study accession and ancestry match.".into(),
        "Confirm effect allele orientation before interpreting direction.".into(),
    ];
    if matches!(trait_category, Some("bone_density")) {
        ideas.push("Consider DEXA/bone density measurement for phenotype validation.".into());
    }
    if matches!(trait_category, Some("metabolic")) {
        ideas.push("Consider fasting glucose/HbA1c labs if clinically indicated.".into());
    }
    ideas
}

pub fn ledger_from_gwas_assoc(
    assoc: &serde_json::Value,
    genotype: &str,
    source_name: &str,
) -> EvidenceLedgerRow {
    let p_value = assoc["pvalue"].as_f64();
    let beta = assoc["beta"].as_f64();
    let odds_ratio = assoc["odds_ratio"].as_f64();
    let effect_allele = assoc["effect_allele"].as_str().map(String::from);
    let (personal_direction, dosage, mut flags) =
        compute_personal_direction(genotype, effect_allele.as_deref(), None, beta, odds_ratio);
    if p_value.is_some() && effect_allele.is_none() {
        flags.push("missing_effect_allele".into());
    }
    if p_value.is_some() && beta.is_none() && odds_ratio.is_none() {
        flags.push("missing_effect_size".into());
    }
    flags.push("gwas_only_not_clinical".into());
    EvidenceLedgerRow {
        source_name: source_name.to_string(),
        trait_name: assoc["trait_name"]
            .as_str()
            .or_else(|| assoc["trait"]["trait"].as_str())
            .map(String::from),
        mapped_gene: assoc["mapped_gene"].as_str().map(String::from),
        p_value,
        beta,
        odds_ratio,
        effect_allele,
        personal_dosage: dosage,
        personal_direction,
        study_accession: assoc["study_accession"].as_str().map(String::from),
        pubmed_id: assoc["pubmed_id"].as_str().map(String::from),
        ancestry: assoc["ancestry"].as_str().map(String::from),
        sample_size: assoc["sample_size"].as_i64().map(|v| v as i32),
        fetched_at: None,
        source_release: assoc["source_release"].as_str().map(String::from),
        raw_payload_hash: assoc["raw_payload_hash"].as_str().map(String::from),
        quality_flags: flags,
        evidence_tier: "curated_gwas".into(),
        association_type: "gwas_top_association".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::compute_personal_direction;

    #[test]
    fn unknown_bases_are_not_scored_as_personal_matches() {
        let (direction, dosage, flags) =
            compute_personal_direction("N/A", Some("A"), None, Some(0.4), None);
        assert_eq!(direction, "unknown");
        assert_eq!(dosage, None);
        assert!(flags.iter().any(|flag| flag == "genotype_ambiguous_call"));

        let (direction, dosage, flags) =
            compute_personal_direction("N/N", Some("A"), None, Some(0.4), None);
        assert_eq!(direction, "unknown");
        assert_eq!(dosage, None);
        assert!(flags.iter().any(|flag| flag == "genotype_no_call"));
    }

    #[test]
    fn complete_calls_retain_personal_direction_scoring() {
        let (direction, dosage, flags) =
            compute_personal_direction("A/A", Some("A"), None, Some(0.4), None);
        assert_eq!(direction, "increased_trait_value");
        assert_eq!(dosage, Some(2));
        assert!(flags.is_empty());
    }
}
