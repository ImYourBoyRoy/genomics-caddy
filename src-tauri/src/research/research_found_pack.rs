// ./src-tauri/src/research/research_found_pack.rs
/*
Purpose: Enrich, review, and manage the research_found marker pack.
Fills alleles/genes from vector GWAS → ClinVar → gnomAD → genotype heuristic;
supports per-marker update/delete and opt-in manifest enablement.
*/

use crate::db;
use crate::offline::lookup::lookup_clinvar_local;
use crate::research::pack_draft::{
    ensure_research_found_pack, RESEARCH_FOUND_PACK_ID,
};
use crate::research::util::normalize_rsid;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct AlleleFillResult {
    pub updated: u32,
    pub unchanged: u32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ResearchFoundPackView {
    pub name: String,
    pub markers: Vec<Value>,
    pub enabled_in_report: bool,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct ResearchFoundMarkerPatch {
    pub rsid: String,
    pub gene: Option<String>,
    pub effect_allele: Option<String>,
    pub effect_direction: Option<String>,
    pub evidence_tier: Option<String>,
    pub impact: Option<String>,
    pub interpretation: Option<String>,
    pub variant_name: Option<String>,
    pub clinical_confirmation_required: Option<bool>,
}

fn packs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("marker-packs")
}

fn pack_path(data_dir: &Path) -> PathBuf {
    packs_dir(data_dir).join(format!("{RESEARCH_FOUND_PACK_ID}.json"))
}

fn manifest_path(data_dir: &Path) -> PathBuf {
    packs_dir(data_dir).join("manifest.json")
}

fn load_pack(data_dir: &Path) -> Result<Value, String> {
    ensure_research_found_pack(data_dir)?;
    let path = pack_path(data_dir);
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn write_pack(data_dir: &Path, pack: &Value) -> Result<(), String> {
    let path = pack_path(data_dir);
    let pack_json = serde_json::to_string_pretty(pack).map_err(|e| e.to_string())?;
    crate::file_utils::atomic_write(&path, pack_json.as_bytes())
        .map_err(|e| format!("Failed to write research_found pack: {e}"))?;
    db::clear_marker_packs_registry();
    db::sync_marker_packs_registry(Some(data_dir));
    Ok(())
}

fn normalize_allele_token(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    // GWAS riskAlleleName often looks like "rs123-A" or "rs123-T".
    if let Some((_, allele)) = t.rsplit_once('-') {
        let a = allele.trim().to_ascii_uppercase();
        if matches!(a.as_str(), "A" | "C" | "G" | "T" | "I" | "D") || a.len() <= 8 {
            return a;
        }
    }
    let upper = t.to_ascii_uppercase();
    if upper.chars().all(|c| matches!(c, 'A' | 'C' | 'G' | 'T' | 'I' | 'D' | '-' | '.')) {
        return upper.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    }
    String::new()
}

fn allele_from_gwas_json(assocs: &Value) -> Option<(String, String)> {
    let arr = assocs.as_array()?;
    let mut best: Option<(f64, String, String)> = None;
    for a in arr {
        let Some(allele) = a
            .get("effect_allele")
            .and_then(|v| v.as_str())
            .map(normalize_allele_token)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let gene = a
            .get("mapped_gene")
            .or_else(|| a.get("gene"))
            .or_else(|| a.get("reported_gene"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let p = a
            .get("pvalue")
            .or_else(|| a.get("p_value"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        match &best {
            None => best = Some((p, allele, gene)),
            Some((bp, _, _)) if p < *bp => best = Some((p, allele, gene)),
            _ => {}
        }
    }
    best.map(|(_, a, g)| (a, g))
}

fn gnomad_alt_for_rsid(conn: &Connection, rsid: &str) -> Option<(String, String, f64)> {
    let key = normalize_rsid(rsid)?.to_ascii_lowercase();
    let sql = "SELECT ref, alt, af, rsids_json
               FROM reference.gnomad_variant_cache
               WHERE lookup_status IN ('found', 'remote_vcf_hit')";
    let mut stmt = conn.prepare(sql).ok()?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<f64>>(2).ok().flatten(),
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .ok()?;
    for row in rows.flatten() {
        let (ref_a, alt, af, Some(rsids_json)) = row else {
            continue;
        };
        let Ok(list) = serde_json::from_str::<Vec<String>>(&rsids_json) else {
            continue;
        };
        if list.iter().any(|r| r.eq_ignore_ascii_case(&key) || r.to_ascii_lowercase() == key) {
            let alt_n = normalize_allele_token(&alt);
            if !alt_n.is_empty() {
                return Some((ref_a, alt_n, af.unwrap_or(0.0)));
            }
        }
    }
    None
}

fn infer_homozygous_allele(genotype: Option<&str>) -> String {
    let Some(g) = genotype.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    let cleaned: String = g
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if cleaned.len() >= 2 && cleaned.as_bytes()[0] == cleaned.as_bytes()[1] {
        cleaned.chars().next().unwrap_or('?').to_string()
    } else {
        String::new()
    }
}

fn is_placeholder_gene(gene: &str) -> bool {
    gene.trim().is_empty() || gene.eq_ignore_ascii_case("UNKNOWN")
}

fn is_placeholder_allele(allele: &str) -> bool {
    allele.trim().is_empty() || allele == "?"
}

/// Enrich one marker Value in place. Returns true if changed.
pub fn fill_marker_fields(conn: &Connection, marker: &mut Value) -> bool {
    let Some(obj) = marker.as_object_mut() else {
        return false;
    };
    let rsid = obj
        .get("rsid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if rsid.is_empty() {
        return false;
    }

    let mut gene = obj
        .get("gene")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let mut allele = obj
        .get("effect_allele")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let mut provenance: Vec<String> = Vec::new();
    let mut changed = false;

    // 1) Nested GWAS on marker (if present from draft meta / prior enrichment stash)
    if let Some(assocs) = obj.get("gwas_associations") {
        if let Some((a, g)) = allele_from_gwas_json(assocs) {
            if is_placeholder_allele(&allele) {
                allele = a;
                provenance.push(format!("effect_allele from vector GWAS ({allele})"));
                changed = true;
            }
            if is_placeholder_gene(&gene) && !g.is_empty() {
                gene = g;
                provenance.push(format!("gene from vector GWAS ({gene})"));
                changed = true;
            }
        }
    }

    // 2) ClinVar local
    if is_placeholder_allele(&allele) || is_placeholder_gene(&gene) {
        if let Some(cv) = lookup_clinvar_local(conn, &rsid) {
            if is_placeholder_allele(&allele) {
                let a = normalize_allele_token(&cv.relevant_allele);
                if !a.is_empty() {
                    allele = a;
                    provenance.push(format!("effect_allele from ClinVar relevant_allele ({allele})"));
                    changed = true;
                }
            }
            if is_placeholder_gene(&gene) {
                if let Some(g) = cv.gene.filter(|g| !g.trim().is_empty()) {
                    gene = g;
                    provenance.push(format!("gene from ClinVar ({gene})"));
                    changed = true;
                }
            }
        }
    }

    // 3) gnomAD cache alt (weak — genomic alt, not necessarily effect)
    if is_placeholder_allele(&allele) {
        if let Some((_ref_a, alt, af)) = gnomad_alt_for_rsid(conn, &rsid) {
            allele = alt;
            provenance.push(format!(
                "effect_allele tentatively from gnomAD alt ({allele}, AF={af:.4}) — verify"
            ));
            changed = true;
        }
    }

    // 4) Genotype homozygous heuristic (from sources notes or top-level if any leftover)
    if is_placeholder_allele(&allele) {
        let geno_owned: Option<String> = obj
            .get("_genotype_hint")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                obj.get("sources")
                    .and_then(|s| s.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|s| s.get("notes"))
                    .and_then(|n| n.as_str())
                    .and_then(|n| {
                        n.split("genotype=")
                            .nth(1)
                            .map(|s| s.split(|c: char| c == ';' || c.is_whitespace()).next().unwrap_or("").to_string())
                    })
                    .filter(|s| !s.is_empty())
            });
        let hint = infer_homozygous_allele(geno_owned.as_deref());
        if !hint.is_empty() {
            allele = hint;
            provenance.push(format!("effect_allele from homozygous genotype hint ({allele})"));
            changed = true;
        }
    }

    if is_placeholder_gene(&gene) {
        gene = "UNKNOWN".into();
    }
    if is_placeholder_allele(&allele) {
        allele = "?".into();
    }

    if obj.get("gene").and_then(|v| v.as_str()) != Some(gene.as_str()) {
        obj.insert("gene".into(), json!(gene));
        changed = true;
    }
    if obj.get("effect_allele").and_then(|v| v.as_str()) != Some(allele.as_str()) {
        obj.insert("effect_allele".into(), json!(allele));
        changed = true;
    }

    // Provenance stays inside sources[].notes (MarkerSource allowlist).
    if !provenance.is_empty() {
        let note_extra = provenance.join("; ");
        if let Some(sources) = obj.get_mut("sources").and_then(|s| s.as_array_mut()) {
            if let Some(first) = sources.first_mut().and_then(|s| s.as_object_mut()) {
                let prev = first
                    .get("notes")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let merged = if prev.is_empty() {
                    note_extra
                } else if prev.contains(&note_extra) {
                    prev
                } else {
                    format!("{prev} | fill: {note_extra}")
                };
                first.insert("notes".into(), json!(merged));
                changed = true;
            } else {
                sources.push(json!({
                    "name": "research_found_allele_fill",
                    "url": "",
                    "accessed": null,
                    "evidence_type": "Local catalog enrichment",
                    "conflict_of_interest": "None",
                    "notes": note_extra
                }));
                changed = true;
            }
        } else {
            obj.insert(
                "sources".into(),
                json!([{
                    "name": "research_found_allele_fill",
                    "url": "",
                    "accessed": null,
                    "evidence_type": "Local catalog enrichment",
                    "conflict_of_interest": "None",
                    "notes": note_extra
                }]),
            );
            changed = true;
        }
    }

    obj.remove("gwas_associations");
    obj.remove("_genotype_hint");
    changed
}

/// Fill alleles/genes for all research_found markers using local catalogs.
pub fn fill_research_found_alleles(
    data_dir: &Path,
    registry_db: &Path,
) -> Result<AlleleFillResult, String> {
    ensure_research_found_pack(data_dir)?;
    let conn = db::connect(registry_db).map_err(|e| e.to_string())?;
    let mut pack = load_pack(data_dir)?;
    let markers = pack
        .get_mut("markers")
        .and_then(|m| m.as_array_mut())
        .ok_or_else(|| "research_found missing markers".to_string())?;

    let mut updated = 0u32;
    let mut unchanged = 0u32;
    for marker in markers.iter_mut() {
        if fill_marker_fields(&conn, marker) {
            updated += 1;
        } else {
            unchanged += 1;
        }
    }
    write_pack(data_dir, &pack)?;
    Ok(AlleleFillResult {
        updated,
        unchanged,
        message: format!(
            "Filled {updated} marker(s) from ClinVar/gnomAD/GWAS catalogs ({unchanged} unchanged)."
        ),
    })
}

pub fn get_research_found_pack(data_dir: &Path) -> Result<ResearchFoundPackView, String> {
    ensure_research_found_pack(data_dir)?;
    let pack = load_pack(data_dir)?;
    let enabled = research_found_enabled(data_dir)?;
    Ok(ResearchFoundPackView {
        name: pack
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Research Found (Vector Discoveries)")
            .to_string(),
        markers: pack
            .get("markers")
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default(),
        enabled_in_report: enabled,
        path: pack_path(data_dir).display().to_string(),
    })
}

pub fn research_found_enabled(data_dir: &Path) -> Result<bool, String> {
    let path = manifest_path(data_dir);
    if !path.is_file() {
        return Ok(false);
    }
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    Ok(manifest
        .get("packs")
        .and_then(|p| p.as_array())
        .and_then(|arr| {
            arr.iter().find(|p| p.get("id").and_then(|v| v.as_str()) == Some(RESEARCH_FOUND_PACK_ID))
        })
        .and_then(|p| p.get("default_enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

pub fn set_research_found_enabled(data_dir: &Path, enabled: bool) -> Result<bool, String> {
    ensure_research_found_pack(data_dir)?;
    let path = manifest_path(data_dir);
    let mut manifest: Value =
        serde_json::from_str(&fs::read_to_string(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let packs = manifest
        .get_mut("packs")
        .and_then(|p| p.as_array_mut())
        .ok_or_else(|| "manifest missing packs".to_string())?;
    let entry = packs
        .iter_mut()
        .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(RESEARCH_FOUND_PACK_ID));
    if let Some(e) = entry {
        if let Some(obj) = e.as_object_mut() {
            obj.insert("default_enabled".into(), json!(enabled));
        }
    } else {
        packs.push(json!({
            "id": RESEARCH_FOUND_PACK_ID,
            "label": "Research Found (Vector Discoveries)",
            "description": "Auto-grown from vector research drafts.",
            "default_enabled": enabled,
            "requires_clinical_confirmation": true
        }));
    }
    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    crate::file_utils::atomic_write(&path, manifest_json.as_bytes())?;
    db::clear_marker_packs_registry();
    db::sync_marker_packs_registry(Some(data_dir));
    Ok(enabled)
}

fn valid_direction(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "risk" => Some("risk"),
        "protective" => Some("protective"),
        "context_dependent" => Some("context_dependent"),
        "trait" => Some("trait"),
        "unknown" => Some("unknown"),
        "not_applicable" => Some("not_applicable"),
        "no_claim" => Some("no_claim"),
        _ => None,
    }
}

pub fn update_research_found_marker(
    data_dir: &Path,
    patch: ResearchFoundMarkerPatch,
) -> Result<ResearchFoundPackView, String> {
    let key = patch.rsid.trim().to_ascii_lowercase();
    if key.is_empty() {
        return Err("rsid required".into());
    }
    let mut pack = load_pack(data_dir)?;
    let markers = pack
        .get_mut("markers")
        .and_then(|m| m.as_array_mut())
        .ok_or_else(|| "missing markers".to_string())?;
    let mut found = false;
    for marker in markers.iter_mut() {
        let Some(obj) = marker.as_object_mut() else {
            continue;
        };
        let rsid = obj.get("rsid").and_then(|v| v.as_str()).unwrap_or("");
        if !rsid.eq_ignore_ascii_case(&key) {
            continue;
        }
        found = true;
        if let Some(g) = patch.gene {
            obj.insert("gene".into(), json!(g.trim()));
        }
        if let Some(a) = patch.effect_allele {
            obj.insert("effect_allele".into(), json!(a.trim().to_ascii_uppercase()));
        }
        if let Some(d) = patch.effect_direction {
            let dir = valid_direction(&d).ok_or_else(|| format!("Invalid effect_direction: {d}"))?;
            obj.insert("effect_direction".into(), json!(dir));
        }
        if let Some(t) = patch.evidence_tier {
            obj.insert("evidence_tier".into(), json!(t));
        }
        if let Some(i) = patch.impact {
            obj.insert("impact".into(), json!(i));
        }
        if let Some(i) = patch.interpretation {
            obj.insert("interpretation".into(), json!(i));
        }
        if let Some(v) = patch.variant_name {
            obj.insert("variant_name".into(), json!(v));
        }
        if let Some(c) = patch.clinical_confirmation_required {
            obj.insert("clinical_confirmation_required".into(), json!(c));
        }
        break;
    }
    if !found {
        return Err(format!("Marker {key} not in research_found"));
    }
    write_pack(data_dir, &pack)?;
    get_research_found_pack(data_dir)
}

pub fn delete_research_found_markers(
    data_dir: &Path,
    rsids: &[String],
) -> Result<ResearchFoundPackView, String> {
    let drop: std::collections::BTreeSet<String> = rsids
        .iter()
        .map(|r| r.trim().to_ascii_lowercase())
        .filter(|r| !r.is_empty())
        .collect();
    if drop.is_empty() {
        return Err("No rsids to delete".into());
    }
    let mut pack = load_pack(data_dir)?;
    let markers = pack
        .get_mut("markers")
        .and_then(|m| m.as_array_mut())
        .ok_or_else(|| "missing markers".to_string())?;
    markers.retain(|m| {
        let r = m
            .get("rsid")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        !drop.contains(&r)
    });
    write_pack(data_dir, &pack)?;
    get_research_found_pack(data_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_gwas_allele_tokens() {
        assert_eq!(normalize_allele_token("rs123-A"), "A");
        assert_eq!(normalize_allele_token("T"), "T");
        assert_eq!(normalize_allele_token(""), "");
    }
}
