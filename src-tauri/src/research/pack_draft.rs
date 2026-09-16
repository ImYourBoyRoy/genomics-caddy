// ./src-tauri/src/research/pack_draft.rs
/*
Purpose: Export draft marker-pack JSON from vector research and merge into research_found only.
Helps grow a dedicated hypothesis pack without touching curated hardcoded packs.
*/

use crate::db::{self, VectorPromotedFinding};
use crate::research::types::QdrantHit;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Sole runtime merge target for vector research drafts.
pub const RESEARCH_FOUND_PACK_ID: &str = "research_found";

/// Curated / system pack ids that must never receive draft merges.
static PROTECTED_PACK_IDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "core",
        "pgx",
        "metabolic",
        "nutrients",
        "neuropsych",
        "sleep",
        "connective_tissue",
        "thyroid_autoimmune",
        "cardiovascular",
        "cancer_confirmation_only",
        "allergy_atopy_mast_cell",
        "digestive_gut_microbiome",
        "muscle_performance_recovery",
        "hormones_reproductive",
        "skin_hair_dermatology",
        "bone_growth_mineral_density",
        "kidney_fluid_electrolytes",
        "respiratory_airway",
        "immune_autoimmune_general",
        "pain_migraine_sensory",
        "dental_oral_health",
        "longevity_aging_resilience",
        "discovery_catalog",
        "actionability_guidance",
        "activity_guardrails",
        "callability_rules",
        "evidence_policy",
        "safety_guardrails",
        "lab_overlays",
        "phenotype_prompts",
        "prs_registry",
        "source_registry",
        "runtime_validation_summary",
        "user_diet_profile_schema",
        "meal_planning_rules",
        "food_requirement_prompts",
        "food_nutrient_matrix",
        "dietary_requirements",
        "diet_pattern_profiles",
        "supplement_safety",
        // Legacy draft target — keep protected so old UI cannot pollute it into curated space.
        "research_candidates",
    ])
});

#[derive(Debug, serde::Serialize)]
pub struct PackDraftExportResult {
    pub path: String,
    pub candidate_count: u32,
    pub already_in_packs: u32,
    pub from_promoted: u32,
    pub from_vector_browse: u32,
    pub message: String,
}

fn pack_rsids(data_dir: &Path) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let packs_dir = data_dir.join("marker-packs");
    if !packs_dir.is_dir() {
        return set;
    }
    let Ok(entries) = fs::read_dir(&packs_dir) else {
        return set;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        // Skip research_found itself so re-export can refresh candidates still under review.
        if path.file_stem().and_then(|s| s.to_str()) == Some(RESEARCH_FOUND_PACK_ID) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(val) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if let Some(markers) = val.get("markers").and_then(|m| m.as_array()) {
            for m in markers {
                if let Some(rsid) = m.get("rsid").and_then(|r| r.as_str()) {
                    set.insert(rsid.to_ascii_lowercase());
                }
            }
        }
    }
    set
}

fn stamp_accessed() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Stable stamp without a calendar crate (curated packs use YYYY-MM-DD when hand-authored).
    format!("unix:{secs}")
}

fn infer_effect_allele_hint(genotype: Option<&str>) -> String {
    let Some(g) = genotype.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    // Keep empty when we cannot infer — merge may fill "?" for research_found.
    let cleaned: String = g
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if cleaned.len() >= 2
        && cleaned.chars().all(|base| matches!(base, 'A' | 'C' | 'G' | 'T'))
        && cleaned.as_bytes()[0] == cleaned.as_bytes()[1]
    {
        cleaned.chars().next().unwrap_or('?').to_string()
    } else {
        String::new()
    }
}

fn hit_to_draft_marker(hit: &QdrantHit) -> Value {
    let gene = hit
        .gene
        .clone()
        .filter(|g| !g.trim().is_empty())
        .unwrap_or_else(|| "UNKNOWN".into());
    let trait_name = hit
        .gwas_trait
        .clone()
        .or_else(|| hit.category.clone())
        .unwrap_or_else(|| "research_found".into());
    let interpretation = if hit.text.trim().is_empty() {
        format!(
            "Vector research candidate ({}) with significance {:.2}. Hypothesis-grade only — review sources and alleles before clinical use.",
            trait_name, hit.significance_score
        )
    } else {
        hit.text.chars().take(800).collect::<String>()
    };
    let genotype = hit.genotype.clone();
    let allele_hint = infer_effect_allele_hint(genotype.as_deref());
    let mut marker = json!({
        "rsid": hit.rsid,
        "gene": gene,
        "variant_name": trait_name,
        "effect_allele": allele_hint,
        "impact": trait_name,
        "evidence_tier": "C_biohacker_hypothesis",
        "interpretation": interpretation,
        "do_not_claim": ["clinical diagnosis", "treatment recommendation", "disease probability"],
        "confirm_with": ["literature review", "clinical correlation", "allele verification"],
        "effect_direction": "context_dependent",
        "raw_dna_limitation": "Auto-generated from vector research — not a curated pack marker.",
        "clinical_confirmation_required": true,
        "variant_type": "snp",
        "sources": [{
            "name": hit.source,
            "url": hit.pmid.as_ref().map(|p| format!("https://pubmed.ncbi.nlm.nih.gov/{p}/")).unwrap_or_default(),
            "accessed": stamp_accessed(),
            "evidence_type": "Vector research index",
            "conflict_of_interest": "Unknown",
            "notes": format!(
                "significance_score={}; pmid={}; genotype={}",
                hit.significance_score,
                hit.pmid.clone().unwrap_or_default(),
                genotype.clone().unwrap_or_default()
            )
        }],
        "_draft_meta": {
            "significance_score": hit.significance_score,
            "enrichment_version": hit.enrichment_version,
            "trait_categories": hit.trait_categories,
            "genotype": genotype,
            "pmid": hit.pmid,
            "origin": "vector_browse"
        }
    });
    if let Some(assocs) = &hit.gwas_associations {
        if let Some(obj) = marker.as_object_mut() {
            obj.insert("gwas_associations".into(), json!(assocs));
        }
    }
    if let Some(gt) = hit.genotype.as_ref() {
        if let Some(obj) = marker.as_object_mut() {
            obj.insert("_genotype_hint".into(), json!(gt));
        }
    }
    marker
}

fn promoted_to_draft_marker(row: &VectorPromotedFinding) -> Value {
    let gene = row
        .gene
        .clone()
        .filter(|g| !g.trim().is_empty())
        .unwrap_or_else(|| "UNKNOWN".into());
    let allele_hint = infer_effect_allele_hint(row.user_genotype.as_deref());
    let mut marker = json!({
        "rsid": row.rsid,
        "gene": gene,
        "variant_name": row.trait_summary.chars().take(80).collect::<String>(),
        "effect_allele": allele_hint,
        "impact": row.trait_summary,
        "evidence_tier": "C_biohacker_hypothesis",
        "interpretation": row.trait_summary,
        "do_not_claim": ["clinical diagnosis", "treatment recommendation", "disease probability"],
        "confirm_with": ["literature review", "clinical correlation", "allele verification"],
        "effect_direction": "context_dependent",
        "raw_dna_limitation": "Draft from vector-promoted finding — not curated.",
        "clinical_confirmation_required": true,
        "variant_type": "snp",
        "sources": [{
            "name": "vector_promoted_findings",
            "url": "",
            "accessed": stamp_accessed(),
            "evidence_type": "Vector promotion",
            "conflict_of_interest": "Unknown",
            "notes": format!(
                "significance_score={}; genotype={}",
                row.significance_score,
                row.user_genotype.clone().unwrap_or_default()
            )
        }],
        "_draft_meta": {
            "significance_score": row.significance_score,
            "gwas_best_pvalue": row.gwas_best_pvalue,
            "enrichment_version": row.enrichment_version,
            "trait_categories": row.trait_categories,
            "genotype": row.user_genotype,
            "origin": "vector_promoted"
        }
    });
    if let Some(gt) = &row.user_genotype {
        if let Some(obj) = marker.as_object_mut() {
            obj.insert("_genotype_hint".into(), json!(gt));
        }
    }
    marker
}

/// Write a reviewable draft pack JSON under `App/Data/exports/pack_drafts/`.
pub fn export_pack_draft(
    data_dir: &Path,
    sample_id: i64,
    sample_name: &str,
    promoted: &[VectorPromotedFinding],
    browsed: &[QdrantHit],
    min_score: f32,
) -> Result<PackDraftExportResult, String> {
    let existing = pack_rsids(data_dir);
    let mut seen = BTreeSet::new();
    let mut markers = Vec::new();
    let mut already_in_packs = 0u32;
    let mut from_promoted = 0u32;
    let mut from_browse = 0u32;

    for row in promoted {
        let key = row.rsid.to_ascii_lowercase();
        if !seen.insert(key.clone()) {
            continue;
        }
        if existing.contains(&key) {
            already_in_packs += 1;
            continue;
        }
        if row.significance_score < min_score {
            continue;
        }
        markers.push(promoted_to_draft_marker(row));
        from_promoted += 1;
    }

    for hit in browsed {
        if hit.rsid.trim().is_empty() {
            continue;
        }
        let key = hit.rsid.to_ascii_lowercase();
        if !seen.insert(key.clone()) {
            continue;
        }
        if existing.contains(&key) {
            already_in_packs += 1;
            continue;
        }
        if hit.significance_score < min_score {
            continue;
        }
        markers.push(hit_to_draft_marker(hit));
        from_browse += 1;
    }

    markers.sort_by(|a, b| {
        let sa = a
            .pointer("/_draft_meta/significance_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let sb = b
            .pointer("/_draft_meta/significance_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let exports = data_dir.join("exports").join("pack_drafts");
    fs::create_dir_all(&exports).map_err(|e| format!("Could not create pack_drafts dir: {e}"))?;
    let safe_name: String = sample_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let stamp = chrono_lite_stamp();
    let path = exports.join(format!("draft_from_vectors_{safe_name}_{sample_id}_{stamp}.json"));

    let body = json!({
        "name": format!("Draft for research_found — {sample_name}"),
        "markers": markers,
        "_export_meta": {
            "schema_version": "2.0.0",
            "draft": true,
            "intended_pack": RESEARCH_FOUND_PACK_ID,
            "sample_id": sample_id,
            "sample_name": sample_name,
            "min_significance_score": min_score,
            "note": "Merge only into research_found. Curated packs are protected. Strip _draft_meta / _export_meta before shipping to repo."
        }
    });

    let draft_json = serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?;
    crate::file_utils::atomic_write(&path, draft_json.as_bytes())
        .map_err(|e| format!("Failed to write draft pack: {e}"))?;

    let candidate_count = markers.len() as u32;
    Ok(PackDraftExportResult {
        path: path.display().to_string(),
        candidate_count,
        already_in_packs,
        from_promoted,
        from_vector_browse: from_browse,
        message: format!(
            "Wrote {candidate_count} draft markers ({from_promoted} promoted, {from_browse} vector; skipped {already_in_packs} already in curated packs)."
        ),
    })
}

fn chrono_lite_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

pub fn exports_pack_drafts_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("exports").join("pack_drafts")
}

pub fn list_pack_drafts(data_dir: &Path) -> Result<Vec<String>, String> {
    let dir = exports_pack_drafts_dir(data_dir);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<String> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .map(|p| p.display().to_string())
        .collect();
    paths.sort();
    paths.reverse();
    Ok(paths)
}

/// Convenience: load promoted rows for a sample from registry+sample DB.
pub fn load_promoted_for_sample(
    registry_db: &Path,
    sample_id: i64,
) -> Result<Vec<VectorPromotedFinding>, String> {
    let conn = db::connect_sample_from_registry_path(registry_db, sample_id)
        .map_err(|e| e.to_string())?;
    db::get_vector_promoted_findings(&conn, sample_id)
}

#[derive(Debug, serde::Serialize)]
pub struct PackMergeResult {
    pub target_pack_id: String,
    pub target_pack_path: String,
    pub added: u32,
    pub skipped_existing: u32,
    pub skipped_incomplete: u32,
    pub backup_path: Option<String>,
    pub message: String,
}

fn sanitize_pack_id(id: &str) -> Result<String, String> {
    let id = id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return Err("Target pack id is required.".into());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err("Pack id must be lowercase letters, digits, or underscore.".into());
    }
    Ok(id)
}

fn assert_merge_allowed(pack_id: &str) -> Result<(), String> {
    if pack_id == RESEARCH_FOUND_PACK_ID {
        return Ok(());
    }
    if PROTECTED_PACK_IDS.contains(pack_id) {
        return Err(format!(
            "Refusing to merge into protected pack '{pack_id}'. Vector drafts only merge into '{RESEARCH_FOUND_PACK_ID}'."
        ));
    }
    Err(format!(
        "Merge target must be '{RESEARCH_FOUND_PACK_ID}' (refused '{pack_id}'). Curated packs are never modified by research drafts."
    ))
}

fn normalize_effect_direction(raw: Option<&str>) -> &'static str {
    match raw.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "risk" => "risk",
        "protective" => "protective",
        "context_dependent" => "context_dependent",
        "trait" => "trait",
        "unknown" => "unknown",
        "not_applicable" => "not_applicable",
        "no_claim" => "no_claim",
        // Legacy draft value — invalid for report MarkerDefinition.
        "association" | "" => "context_dependent",
        _ => "context_dependent",
    }
}

/// Shape a marker to match curated pack MarkerDefinition (deny_unknown_fields safe).
fn clean_marker_for_pack(mut marker: Value) -> Value {
    let genotype_from_meta = marker
        .pointer("/_draft_meta/genotype")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            marker
                .get("_genotype_hint")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        });
    let gwas = marker.get("gwas_associations").cloned();
    if let Some(obj) = marker.as_object_mut() {
        obj.remove("_draft_meta");
        obj.remove("draft");
        obj.remove("genotype");
        obj.remove("schema_version");
        obj.remove("_export_meta");

        let dir = normalize_effect_direction(obj.get("effect_direction").and_then(|v| v.as_str()));
        obj.insert("effect_direction".into(), json!(dir));

        let gene = obj
            .get("gene")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if gene.is_empty() {
            obj.insert("gene".into(), json!("UNKNOWN"));
        }

        let allele = obj
            .get("effect_allele")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if allele.is_empty() {
            let hint = infer_effect_allele_hint(genotype_from_meta.as_deref());
            obj.insert(
                "effect_allele".into(),
                json!(if hint.is_empty() { "?" } else { hint.as_str() }),
            );
        }

        obj.entry("clinical_confirmation_required")
            .or_insert(json!(true));
        obj.entry("variant_type").or_insert(json!("snp"));
        obj.entry("do_not_claim").or_insert(json!([
            "clinical diagnosis",
            "treatment recommendation",
            "disease probability"
        ]));
        obj.entry("confirm_with").or_insert(json!([
            "literature review",
            "clinical correlation",
            "allele verification"
        ]));
        obj.entry("evidence_tier")
            .or_insert(json!("C_biohacker_hypothesis"));
        obj.entry("raw_dna_limitation").or_insert(json!(
            "Auto-generated from vector research — verify alleles before acting."
        ));

        if let Some(g) = gwas {
            obj.insert("gwas_associations".into(), g);
        }
        if let Some(gt) = genotype_from_meta {
            obj.insert("_genotype_hint".into(), json!(gt));
        }

        if let Some(sources) = obj.get_mut("sources").and_then(|s| s.as_array_mut()) {
            for src in sources {
                if let Some(s) = src.as_object_mut() {
                    if s.get("accessed").is_none_or(Value::is_null) {
                        s.insert("accessed".into(), json!(stamp_accessed()));
                    }
                }
            }
        }
    }
    marker
}

fn marker_incomplete(marker: &Value) -> bool {
    let allele = marker
        .get("effect_allele")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let gene = marker
        .get("gene")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    allele.is_empty() || gene.is_empty() || gene.eq_ignore_ascii_case("UNKNOWN") || allele == "?"
}

fn empty_research_found_pack() -> Value {
    json!({
        "name": "Research Found (Vector Discoveries)",
        "markers": []
    })
}

/// Ensure `research_found.json` + manifest entry exist under App/Data (never overwrites curated files).
pub fn ensure_research_found_pack(data_dir: &Path) -> Result<(), String> {
    let packs_dir = data_dir.join("marker-packs");
    fs::create_dir_all(&packs_dir).map_err(|e| e.to_string())?;
    let pack_path = packs_dir.join(format!("{RESEARCH_FOUND_PACK_ID}.json"));
    if !pack_path.is_file() {
        let pack_json = serde_json::to_string_pretty(&empty_research_found_pack())
            .map_err(|e| e.to_string())?;
        crate::file_utils::atomic_write(&pack_path, pack_json.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    ensure_manifest_entry(
        data_dir,
        RESEARCH_FOUND_PACK_ID,
        Some("Research Found (Vector Discoveries)"),
    )?;
    Ok(())
}

/// Merge draft markers into App/Data/marker-packs/research_found.json only.
/// Never modifies repo `src/lib/marker-packs` or other curated runtime packs.
pub fn merge_draft_into_pack_with_db(
    data_dir: &Path,
    registry_db: Option<&Path>,
    draft_path: &str,
    target_pack_id: &str,
    only_rsids: Option<&[String]>,
    require_complete: bool,
) -> Result<PackMergeResult, String> {
    let mut pack_id = sanitize_pack_id(target_pack_id)?;
    // Alias legacy UI target → research_found.
    if pack_id == "research_candidates" {
        pack_id = RESEARCH_FOUND_PACK_ID.to_string();
    }
    assert_merge_allowed(&pack_id)?;
    ensure_research_found_pack(data_dir)?;

    let draft_file = PathBuf::from(draft_path);
    if !draft_file.is_file() {
        return Err(format!("Draft not found: {draft_path}"));
    }
    let drafts_dir = exports_pack_drafts_dir(data_dir)
        .canonicalize()
        .unwrap_or_else(|_| exports_pack_drafts_dir(data_dir));
    let draft_canon = draft_file
        .canonicalize()
        .map_err(|e| format!("Invalid draft path: {e}"))?;
    if !draft_canon.starts_with(&drafts_dir) {
        return Err("Draft must live under App/Data/exports/pack_drafts/.".into());
    }

    let draft_text = fs::read_to_string(&draft_canon).map_err(|e| e.to_string())?;
    let draft: Value = serde_json::from_str(&draft_text).map_err(|e| e.to_string())?;
    let draft_markers = draft
        .get("markers")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    let only: Option<BTreeSet<String>> = only_rsids.map(|rs| {
        rs.iter()
            .map(|r| r.trim().to_ascii_lowercase())
            .filter(|r| !r.is_empty())
            .collect()
    });

    let packs_dir = data_dir.join("marker-packs");
    let pack_path = packs_dir.join(format!("{pack_id}.json"));

    let mut pack: Value = if pack_path.is_file() {
        let text = fs::read_to_string(&pack_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())?
    } else {
        empty_research_found_pack()
    };

    // Keep curated top-level shape: name + markers only.
    if let Some(obj) = pack.as_object_mut() {
        obj.remove("schema_version");
        obj.remove("draft");
        obj.remove("generated_from");
        obj.remove("_export_meta");
        obj.entry("name")
            .or_insert(json!("Research Found (Vector Discoveries)"));
        if !obj.contains_key("markers") {
            obj.insert("markers".into(), json!([]));
        }
    }

    let existing: BTreeSet<String> = pack
        .get("markers")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("rsid").and_then(|r| r.as_str()))
                .map(|r| r.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default();

    let mut added = 0u32;
    let mut skipped_existing = 0u32;
    let mut skipped_incomplete = 0u32;
    let markers = pack
        .get_mut("markers")
        .and_then(|m| m.as_array_mut())
        .ok_or_else(|| "Target pack missing markers array.".to_string())?;

    for raw in draft_markers {
        let Some(rsid) = raw.get("rsid").and_then(|v| v.as_str()) else {
            continue;
        };
        let key = rsid.to_ascii_lowercase();
        if let Some(ref filter) = only {
            if !filter.contains(&key) {
                continue;
            }
        }
        if existing.contains(&key) {
            skipped_existing += 1;
            continue;
        }
        let cleaned = clean_marker_for_pack(raw);
        let mut cleaned = cleaned;
        if let Some(db_path) = registry_db {
            if let Ok(conn) = db::connect(db_path) {
                let _ = super::research_found_pack::fill_marker_fields(&conn, &mut cleaned);
            }
        }
        // research_found always fills gene/allele placeholders; require_complete skips UNKNOWN/? .
        if require_complete && marker_incomplete(&cleaned) {
            skipped_incomplete += 1;
            continue;
        }
        markers.push(cleaned);
        added += 1;
    }

    let backup_path = if pack_path.is_file() {
        let bak = packs_dir.join(format!("{pack_id}.json.bak"));
        fs::copy(&pack_path, &bak).map_err(|e| format!("Backup failed: {e}"))?;
        Some(bak.display().to_string())
    } else {
        None
    };

    let pack_json = serde_json::to_string_pretty(&pack).map_err(|e| e.to_string())?;
    crate::file_utils::atomic_write(&pack_path, pack_json.as_bytes())
        .map_err(|e| format!("Failed to write pack: {e}"))?;

    ensure_manifest_entry(
        data_dir,
        &pack_id,
        pack.get("name").and_then(|v| v.as_str()),
    )?;

    db::clear_marker_packs_registry();
    db::sync_marker_packs_registry(Some(data_dir));

    Ok(PackMergeResult {
        target_pack_id: pack_id.clone(),
        target_pack_path: pack_path.display().to_string(),
        added,
        skipped_existing,
        skipped_incomplete,
        backup_path,
        message: format!(
            "Merged {added} marker(s) into {pack_id}.json (skipped {skipped_existing} existing, {skipped_incomplete} incomplete). Curated packs were not modified."
        ),
    })
}

fn ensure_manifest_entry(data_dir: &Path, pack_id: &str, name: Option<&str>) -> Result<(), String> {
    let packs_dir = data_dir.join("marker-packs");
    let manifest_path = packs_dir.join("manifest.json");
    let mut manifest: Value = if manifest_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        json!({ "packs": [] })
    };
    let packs = manifest
        .get_mut("packs")
        .and_then(|p| p.as_array_mut())
        .ok_or_else(|| "manifest.json missing packs array".to_string())?;
    let exists = packs
        .iter()
        .any(|p| p.get("id").and_then(|v| v.as_str()) == Some(pack_id));
    if !exists {
        packs.push(json!({
            "id": pack_id,
            "label": name.unwrap_or("Research Found (Vector Discoveries)"),
            "description": "Auto-grown from vector research drafts. Hypothesis-grade only — review alleles and evidence before treating as curated.",
            "default_enabled": false,
            "requires_clinical_confirmation": true
        }));
        let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
        crate::file_utils::atomic_write(&manifest_path, manifest_json.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn list_runtime_pack_ids(data_dir: &Path) -> Result<Vec<String>, String> {
    let _ = ensure_research_found_pack(data_dir);
    Ok(vec![RESEARCH_FOUND_PACK_ID.to_string()])
}
