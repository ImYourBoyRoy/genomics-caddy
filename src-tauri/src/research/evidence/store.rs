// ./src-tauri/src/research/evidence/store.rs
//! Persist association facts and candidate markers from enrichment.

use super::adapters::{self, SecondarySourceBundle};
use super::ontology::TraitOntologyMapping;
use super::pgs_match::store_pgs_match_cache;
use super::scoring::{
    clinical_actionability_score, ledger_from_gwas_assoc, novelty_score, pvalue_mlog10,
    wellness_actionability_score,
};
use super::source_records;
use super::types::EvidenceLedgerRow;
use crate::research::promote::is_curated_marker_rsid;
use crate::research::util::unix_now;
use rusqlite::{Connection, params};
use serde_json::json;
use std::path::Path;

const FACTS_FRESH_SECS: i64 = 30 * 24 * 3600;

fn trait_mapping_for_row(
    reported: Option<&str>,
    mappings: &[TraitOntologyMapping],
) -> (Option<String>, Option<String>) {
    let reported = reported.unwrap_or("").trim();
    if !reported.is_empty()
        && let Some(m) = mappings
            .iter()
            .find(|m| m.reported_trait.eq_ignore_ascii_case(reported))
    {
        return (Some(m.mapped_label.clone()), Some(m.ontology_id.clone()));
    }
    if let Some(best) = adapters::best_ontology_mapping(mappings) {
        return (
            Some(best.mapped_label.clone()),
            Some(best.ontology_id.clone()),
        );
    }
    (
        if reported.is_empty() {
            None
        } else {
            Some(reported.to_string())
        },
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn persist_enrichment_evidence(
    db_path: &Path,
    sample_id: i64,
    rsid: &str,
    genotype: &str,
    gene: Option<&str>,
    trait_category: Option<&str>,
    trait_name: Option<&str>,
    gwas_associations: &[serde_json::Value],
    clinvar_sig: Option<&str>,
    data_quality_score: f32,
    association_strength: f32,
    sources_provenance: &serde_json::Value,
    secondary: &SecondarySourceBundle,
) -> Result<(), String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    if facts_fresh_for_rsid(&conn, sample_id, rsid) {
        return Ok(());
    }
    let now = unix_now();
    let in_curated = is_curated_marker_rsid(&conn, rsid);

    let mut ledger: Vec<EvidenceLedgerRow> = gwas_associations
        .iter()
        .map(|a| ledger_from_gwas_assoc(a, genotype, "gwas_catalog"))
        .collect();

    if let Some(sig) = clinvar_sig.filter(|s| !s.is_empty()) {
        if let Ok(sr_id) = source_records::record_clinvar_local(&conn, rsid, sig, gene) {
            ledger.push(clinvar_ledger_row(sig, &sr_id));
        } else {
            ledger.push(clinvar_ledger_row(sig, ""));
        }
    }

    for pgs in &secondary.pgs_matches {
        ledger.push(adapters::ledger_from_pgs_match(pgs, genotype));
        let _ = store_pgs_match_cache(&conn, sample_id, pgs);
    }
    if let Some(gene_sym) = gene {
        for pathway in secondary.pathway_names.iter().take(5) {
            ledger.push(adapters::ledger_from_pathway(gene_sym, pathway, None));
        }
        for drug in secondary.pgx_drugs.iter().take(5) {
            ledger.push(adapters::ledger_from_pgx(rsid, Some(gene_sym), drug));
        }
    }
    for mapping in &secondary.ontology_mappings {
        ledger.push(EvidenceLedgerRow {
            source_name: "ols4".into(),
            trait_name: Some(mapping.reported_trait.clone()),
            mapped_gene: gene.map(String::from),
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
            fetched_at: Some(unix_now()),
            source_release: None,
            raw_payload_hash: Some(mapping.ontology_id.clone()),
            quality_flags: vec!["trait_ontology_mapped".into()],
            evidence_tier: "literature_context".into(),
            association_type: "gene_trait_context".into(),
        });
    }

    let known_direction = ledger
        .iter()
        .filter(|r| {
            matches!(
                r.personal_direction.as_str(),
                "increased_trait_value"
                    | "decreased_trait_value"
                    | "increased_odds"
                    | "decreased_odds"
            )
        })
        .count() as i32;
    let unknown_direction = ledger.len() as i32 - known_direction;

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
    let has_clinvar = clinvar_sig.map(|s| !s.is_empty()).unwrap_or(false);
    let has_pgs = sources_provenance
        .get("pgs_catalog")
        .and_then(|v| v.get("hit"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || !secondary.pgs_matches.is_empty();
    let has_pharmgkb = sources_provenance
        .get("pharmgkb")
        .and_then(|v| v.get("hit"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_reactome = sources_provenance
        .get("reactome")
        .and_then(|v| v.get("hit"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || !secondary.pathway_names.is_empty();
    let best_p = gwas_associations
        .iter()
        .filter_map(|a| a["pvalue"].as_f64())
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let conflict_count = count_direction_conflicts(&ledger);

    conn.execute(
        "DELETE FROM association_facts WHERE sample_id = ? AND rsid = ?",
        params![sample_id, rsid],
    )
    .map_err(|e| e.to_string())?;

    for (idx, ledger_row) in ledger.iter().enumerate() {
        let assoc_id = format!("assoc_{}_{}_{}", rsid, now, idx);
        let flags_json =
            serde_json::to_string(&ledger_row.quality_flags).unwrap_or_else(|_| "[]".into());
        let dir_conf = if ledger_row.personal_direction == "unknown" {
            0.0
        } else if ledger_row.beta.is_some() || ledger_row.odds_ratio.is_some() {
            0.85
        } else {
            0.15
        };
        let wellness = wellness_actionability_score(
            &trait_category
                .map(|c| vec![c.to_string()])
                .unwrap_or_default(),
            data_quality_score,
            !gwas_associations.is_empty(),
        );
        let clinical = clinical_actionability_score(clinvar_sig, has_clinvar, conflict_count);

        let (trait_mapped, trait_ontology_id) = trait_mapping_for_row(
            ledger_row.trait_name.as_deref(),
            &secondary.ontology_mappings,
        );

        conn.execute(
            "INSERT INTO association_facts (
                association_id, sample_id, rsid, genotype, source_name, source_record_id,
                association_type, evidence_tier, trait_name_reported, trait_name_mapped,
                trait_ontology_id, trait_category, mapped_gene_symbol, reported_gene_symbols_json,
                pubmed_id, study_accession, p_value, p_value_mlog10, beta, odds_ratio,
                effect_allele, personal_effect_allele_dosage, personal_direction,
                directionality_confidence, sample_size, ancestry_initial,
                association_strength_score, personal_match_score,
                clinical_actionability_score, wellness_actionability_score,
                data_quality_score, quality_flags_json, model_inferred, fetched_at,
                created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)",
            params![
                assoc_id,
                sample_id,
                rsid,
                genotype,
                ledger_row.source_name,
                ledger_row.raw_payload_hash.as_deref().filter(|s| !s.is_empty()),
                ledger_row.association_type,
                ledger_row.evidence_tier,
                ledger_row.trait_name,
                trait_mapped,
                trait_ontology_id,
                trait_category,
                ledger_row.mapped_gene,
                None::<String>,
                ledger_row.pubmed_id,
                ledger_row.study_accession,
                ledger_row.p_value,
                pvalue_mlog10(ledger_row.p_value),
                ledger_row.beta,
                ledger_row.odds_ratio,
                ledger_row.effect_allele,
                ledger_row.personal_dosage,
                ledger_row.personal_direction,
                dir_conf,
                ledger_row.sample_size,
                ledger_row.ancestry,
                association_strength,
                if ledger_row.personal_dosage.is_some() {
                    0.85
                } else {
                    0.5
                },
                clinical,
                wellness,
                data_quality_score,
                flags_json,
                now,
                now,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    if !in_curated && (!gwas_associations.is_empty() || has_clinvar) {
        upsert_candidate(
            &conn,
            rsid,
            gene,
            trait_name,
            trait_category,
            ledger.len() as i32,
            known_direction,
            unknown_direction,
            best_p,
            data_quality_score,
            association_strength,
            novelty_score(in_curated, false),
            has_clinvar,
            !gwas_associations.is_empty(),
            has_gtex,
            has_pubmed,
            has_pgs,
            has_pharmgkb,
            has_reactome,
        )?;
    }

    let _ = conflict_count;
    Ok(())
}

fn clinvar_ledger_row(sig: &str, source_record_id: &str) -> EvidenceLedgerRow {
    let conflicting = sig.to_lowercase().contains("conflict");
    EvidenceLedgerRow {
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
        fetched_at: Some(unix_now()),
        source_release: None,
        raw_payload_hash: if source_record_id.is_empty() {
            None
        } else {
            Some(source_record_id.to_string())
        },
        quality_flags: if conflicting {
            vec![
                "clinvar_conflicting_interpretations".into(),
                "source_conflict".into(),
            ]
        } else {
            vec!["not_medical_actionable".into()]
        },
        evidence_tier: "clinical_assertion".into(),
        association_type: "clinvar_assertion".into(),
    }
}

fn count_direction_conflicts(ledger: &[EvidenceLedgerRow]) -> u32 {
    use std::collections::HashMap;
    let mut by_trait: HashMap<String, Vec<&str>> = HashMap::new();
    for row in ledger {
        if let Some(trait_name) = row.trait_name.as_deref() {
            by_trait
                .entry(trait_name.to_lowercase())
                .or_default()
                .push(row.personal_direction.as_str());
        }
    }
    let mut conflicts = 0u32;
    for dirs in by_trait.values() {
        let has_inc = dirs.iter().any(|d| d.starts_with("increased"));
        let has_dec = dirs.iter().any(|d| d.starts_with("decreased"));
        if has_inc && has_dec {
            conflicts += 1;
        }
    }
    conflicts
}

#[allow(clippy::too_many_arguments)]
fn upsert_candidate(
    conn: &Connection,
    rsid: &str,
    gene: Option<&str>,
    trait_name: Option<&str>,
    trait_category: Option<&str>,
    association_count: i32,
    known_direction_count: i32,
    unknown_direction_count: i32,
    best_p_value: Option<f64>,
    data_quality: f32,
    assoc_strength: f32,
    novelty: f32,
    has_clinvar: bool,
    has_gwas: bool,
    has_gtex: bool,
    has_pubmed: bool,
    has_pgs: bool,
    has_pharmgkb: bool,
    has_reactome: bool,
) -> Result<(), String> {
    let existing: Option<(String, String)> = conn
        .query_row(
            "SELECT candidate_id, status FROM candidate_marker_expansion WHERE rsid = ? AND status != 'rejected'",
            params![rsid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    let now = unix_now();
    let source_count = (has_gwas as i32)
        + (has_clinvar as i32)
        + (has_gtex as i32)
        + (has_pubmed as i32)
        + (has_pgs as i32)
        + (has_pharmgkb as i32)
        + (has_reactome as i32);
    let best_tier = if has_clinvar {
        "clinical_assertion"
    } else if has_gwas {
        "curated_gwas"
    } else {
        "literature_context"
    };

    if let Some((id, status)) = existing {
        if status == "rejected" {
            return Ok(());
        }
        conn.execute(
            "UPDATE candidate_marker_expansion SET
                gene_symbol = COALESCE(?, gene_symbol),
                trait_name = COALESCE(?, trait_name),
                trait_category = COALESCE(?, trait_category),
                association_count = ?,
                source_count = ?,
                known_direction_count = ?,
                unknown_direction_count = ?,
                best_evidence_tier = ?,
                best_p_value = ?,
                data_quality_score = ?,
                association_strength_score = ?,
                novelty_score = ?,
                has_clinvar = ?,
                has_gwas = ?,
                has_gtex = ?,
                has_pubmed = ?,
                has_pgs = ?,
                has_pharmgkb = ?,
                updated_at = ?
             WHERE candidate_id = ?",
            params![
                gene,
                trait_name,
                trait_category,
                association_count,
                source_count,
                known_direction_count,
                unknown_direction_count,
                best_tier,
                best_p_value,
                data_quality,
                assoc_strength,
                novelty,
                has_clinvar as i32,
                has_gwas as i32,
                has_gtex as i32,
                has_pubmed as i32,
                has_pgs as i32,
                has_pharmgkb as i32,
                now,
                id,
            ],
        )
        .map_err(|e| e.to_string())?;
    } else {
        let candidate_id = format!("cand_{}_{}", rsid, now);
        conn.execute(
            "INSERT INTO candidate_marker_expansion (
                candidate_id, rsid, gene_symbol, trait_name, trait_category,
                reason_surfaced, source_count, association_count,
                known_direction_count, unknown_direction_count,
                best_evidence_tier, best_p_value,
                data_quality_score, association_strength_score, novelty_score,
                has_clinvar, has_gwas, has_gtex, has_pubmed, has_pgs, has_pharmgkb,
                status, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'unreviewed_dynamic', ?, ?)",
            params![
                candidate_id,
                rsid,
                gene,
                trait_name,
                trait_category,
                "vector_enrichment_v4.1",
                source_count,
                association_count,
                known_direction_count,
                unknown_direction_count,
                best_tier,
                best_p_value,
                data_quality,
                assoc_strength,
                novelty,
                has_clinvar as i32,
                has_gwas as i32,
                has_gtex as i32,
                has_pubmed as i32,
                has_pgs as i32,
                has_pharmgkb as i32,
                now,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn list_candidates(
    conn: &Connection,
    limit: u32,
) -> Result<Vec<super::types::CandidateMarkerRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT candidate_id, rsid, gene_symbol, trait_name, trait_category, status,
                    data_quality_score, association_strength_score, reason_surfaced, best_evidence_tier,
                    source_count, known_direction_count, best_p_value, has_gwas, has_clinvar, has_gtex, has_pubmed,
                    has_pgs, has_pharmgkb
             FROM candidate_marker_expansion
             WHERE status NOT IN ('rejected', 'deprecated')
             ORDER BY data_quality_score DESC, association_strength_score DESC
             LIMIT ?",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(super::types::CandidateMarkerRow {
                candidate_id: row.get(0)?,
                rsid: row.get(1)?,
                gene_symbol: row.get(2)?,
                trait_name: row.get(3)?,
                trait_category: row.get(4)?,
                status: row.get(5)?,
                data_quality_score: row.get(6)?,
                association_strength_score: row.get(7)?,
                reason_surfaced: row.get(8)?,
                best_evidence_tier: row.get(9)?,
                source_count: row.get(10)?,
                known_direction_count: row.get(11)?,
                best_p_value: row.get(12)?,
                has_gwas: row.get::<_, Option<i32>>(13)?.unwrap_or(0) != 0,
                has_clinvar: row.get::<_, Option<i32>>(14)?.unwrap_or(0) != 0,
                has_gtex: row.get::<_, Option<i32>>(15)?.unwrap_or(0) != 0,
                has_pubmed: row.get::<_, Option<i32>>(16)?.unwrap_or(0) != 0,
                has_pgs: row.get::<_, Option<i32>>(17)?.unwrap_or(0) != 0,
                has_pharmgkb: row.get::<_, Option<i32>>(18)?.unwrap_or(0) != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn update_candidate_status(
    conn: &Connection,
    candidate_id: &str,
    status: &str,
    reviewer_note: Option<&str>,
) -> Result<(), String> {
    crate::config::validate_candidate_status(status)?;
    if status == "curated_lite" {
        let row: (i32, i32) = conn
            .query_row(
                "SELECT COALESCE(has_gwas, 0), COALESCE(association_count, 0)
                 FROM candidate_marker_expansion WHERE candidate_id = ?",
                params![candidate_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        if row.0 == 0 && row.1 == 0 {
            return Err(
                "curated_lite promotion requires cached GWAS or association evidence on this candidate"
                    .into(),
            );
        }
    }

    let now = unix_now();
    conn.execute(
        "UPDATE candidate_marker_expansion SET status = ?, reviewer_note = ?, reviewed_at = ?, updated_at = ? WHERE candidate_id = ?",
        params![status, reviewer_note, now, now, candidate_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn rejected_rsids(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT rsid FROM candidate_marker_expansion WHERE status = 'rejected'")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<String>, _>>()
        .map_err(|e| e.to_string())
}

pub fn association_fact_count(conn: &Connection, sample_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM association_facts WHERE sample_id = ?",
        params![sample_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn facts_fresh_for_rsid(conn: &Connection, sample_id: i64, rsid: &str) -> bool {
    conn.query_row(
        "SELECT MAX(updated_at) FROM association_facts WHERE sample_id = ? AND rsid = ?",
        params![sample_id, rsid],
        |row| row.get::<_, Option<i64>>(0),
    )
    .ok()
    .flatten()
    .map(|ts| unix_now() - ts < FACTS_FRESH_SECS)
    .unwrap_or(false)
}

pub fn facts_for_rsid(
    conn: &Connection,
    sample_id: i64,
    rsid: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT association_id, source_name, source_record_id, trait_name_reported,
                    mapped_gene_symbol, p_value, beta, odds_ratio, effect_allele,
                    personal_direction, evidence_tier, study_accession, pubmed_id,
                    quality_flags_json, directionality_confidence
             FROM association_facts WHERE sample_id = ? AND rsid = ?",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id, rsid], |row| {
            Ok(json!({
                "association_id": row.get::<_, String>(0)?,
                "source_name": row.get::<_, String>(1)?,
                "source_record_id": row.get::<_, Option<String>>(2)?,
                "trait_name": row.get::<_, Option<String>>(3)?,
                "mapped_gene": row.get::<_, Option<String>>(4)?,
                "p_value": row.get::<_, Option<f64>>(5)?,
                "beta": row.get::<_, Option<f64>>(6)?,
                "odds_ratio": row.get::<_, Option<f64>>(7)?,
                "effect_allele": row.get::<_, Option<String>>(8)?,
                "personal_direction": row.get::<_, Option<String>>(9)?,
                "evidence_tier": row.get::<_, String>(10)?,
                "study_accession": row.get::<_, Option<String>>(11)?,
                "pubmed_id": row.get::<_, Option<String>>(12)?,
                "quality_flags": row.get::<_, Option<String>>(13)?,
                "directionality_confidence": row.get::<_, Option<f64>>(14)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
