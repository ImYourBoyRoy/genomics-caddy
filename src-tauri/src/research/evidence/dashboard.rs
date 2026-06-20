// ./src-tauri/src/research/evidence/dashboard.rs
//! Quality/coverage dashboard and trait cluster summaries.

use super::source_records;
use super::store::association_fact_count;
use super::types::{QualityDashboard, SourceCoverageSummary, TraitClusterSummary};
use crate::research::qdrant::{count_qdrant_points, count_qdrant_points_with_filter};
use crate::research::types::QdrantConfig;
use crate::research::util::{is_current_enrichment_version, ENRICHMENT_VERSION};
use rusqlite::{params, Connection};
use std::path::Path;

pub async fn build_quality_dashboard(
    db_path: &Path,
    sample_id: i64,
    config: &QdrantConfig,
    enrichment_status: Option<String>,
) -> Result<QualityDashboard, String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;

    let total_genotypes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM genotypes WHERE sample_id = ?",
            params![sample_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let association_fact_count = association_fact_count(&conn, sample_id)? as u64;

    let variants_with_gwas: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT rsid) FROM association_facts WHERE sample_id = ? AND association_type = 'gwas_top_association'",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let known_direction: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM association_facts WHERE sample_id = ? AND personal_direction IN ('increased_trait_value','decreased_trait_value','increased_odds','decreased_odds')",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let unknown_direction: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM association_facts WHERE sample_id = ? AND (personal_direction IS NULL OR personal_direction IN ('unknown','possible_relevance_unknown_direction'))",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let missing_gene: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM association_facts WHERE sample_id = ? AND (mapped_gene_symbol IS NULL OR mapped_gene_symbol = '')",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let missing_effect_allele: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM association_facts WHERE sample_id = ? AND (effect_allele IS NULL OR effect_allele = '')",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let source_conflict: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM association_facts WHERE sample_id = ? AND quality_flags_json LIKE '%conflict%'",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let candidate_unreviewed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM candidate_marker_expansion WHERE status = 'unreviewed_dynamic'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let avg_dq: f32 = conn
        .query_row(
            "SELECT AVG(data_quality_score) FROM association_facts WHERE sample_id = ?",
            params![sample_id],
            |row| row.get::<_, Option<f64>>(0),
        )
        .ok()
        .flatten()
        .map(|v| v as f32)
        .unwrap_or(0.0);

    let variants_with_clinvar: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT rsid) FROM association_facts WHERE sample_id = ? AND association_type = 'clinvar_assertion'",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let missing_study_accession: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM association_facts WHERE sample_id = ? AND source_name = 'gwas_catalog' AND (study_accession IS NULL OR study_accession = '')",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let cache_stale: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM api_cache_entries WHERE cache_status = 'stale'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let source_record_total = source_records::source_record_count(&conn).unwrap_or(0);

    let variants_with_gtex: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT rsid) FROM candidate_marker_expansion WHERE has_gtex = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let variants_with_pubmed: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT rsid) FROM candidate_marker_expansion WHERE has_pubmed = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let variants_with_pgs: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT rsid) FROM candidate_marker_expansion WHERE has_pgs = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let variants_with_pharmgkb: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT rsid) FROM candidate_marker_expansion WHERE has_pharmgkb = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let variants_with_reactome: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT rsid) FROM association_facts WHERE sample_id = ? AND source_name = 'reactome'",
            params![sample_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let sample_vectors = count_qdrant_points(
        &config.url,
        config.api_key.as_deref(),
        &config.collection,
        Some(sample_id),
    )
    .await
    .ok();

    let stale_vector_count = count_qdrant_points_with_filter(
        &config.url,
        config.api_key.as_deref(),
        &config.collection,
        sample_id,
        serde_json::json!({ "key": "stale", "match": { "value": true } }),
    )
    .await
    .unwrap_or(0);

    let schema_mismatch_count = count_qdrant_points_with_filter(
        &config.url,
        config.api_key.as_deref(),
        &config.collection,
        sample_id,
        serde_json::json!({
            "must_not": [{
                "key": "schema_version",
                "match": { "value": super::types::EVIDENCE_SCHEMA_VERSION }
            }]
        }),
    )
    .await
    .unwrap_or(0);

    let source_coverage = SourceCoverageSummary {
        gwas: variants_with_gwas as u64,
        clinvar: variants_with_clinvar as u64,
        gtex: variants_with_gtex as u64,
        pubmed: variants_with_pubmed as u64,
        pgs: variants_with_pgs as u64,
        pharmgkb: variants_with_pharmgkb as u64,
        reactome: variants_with_reactome as u64,
    };

    Ok(QualityDashboard {
        sample_id,
        total_genotypes: total_genotypes as u64,
        vectorized_variants: sample_vectors,
        association_fact_count,
        variants_with_gwas: variants_with_gwas as u64,
        variants_with_clinvar: variants_with_clinvar as u64,
        known_direction_count: known_direction as u64,
        unknown_direction_count: unknown_direction as u64,
        missing_gene_count: missing_gene as u64,
        missing_effect_allele_count: missing_effect_allele as u64,
        missing_study_accession_count: missing_study_accession as u64,
        stale_vector_count,
        source_conflict_count: source_conflict as u64,
        schema_mismatch_count,
        schema_v1_count: sample_vectors.unwrap_or(0),
        enrichment_v41_count: if is_current_enrichment_version(Some(ENRICHMENT_VERSION)) {
            sample_vectors.unwrap_or(0).saturating_sub(stale_vector_count)
        } else {
            0
        },
        candidate_unreviewed_count: candidate_unreviewed as u64,
        avg_data_quality_score: avg_dq,
        source_record_count: source_record_total as u64,
        cache_stale_count: cache_stale as u64,
        variants_with_pgs: variants_with_pgs as u64,
        variants_with_gtex: variants_with_gtex as u64,
        variants_with_pubmed: variants_with_pubmed as u64,
        source_coverage,
        collection: config.collection.clone(),
        enrichment_status,
    })
}

pub fn build_trait_clusters(
    conn: &Connection,
    sample_id: i64,
    trait_category: Option<&str>,
    min_data_quality: Option<f32>,
    limit: u32,
) -> Result<Vec<TraitClusterSummary>, String> {
    let min_dq = min_data_quality.unwrap_or(0.0);
    let sql = if trait_category.is_some() {
        "SELECT trait_category, trait_name_reported, mapped_gene_symbol, rsid,
                personal_direction, data_quality_score, wellness_actionability_score,
                clinical_actionability_score
         FROM association_facts
         WHERE sample_id = ? AND trait_category = ? AND data_quality_score >= ?
         ORDER BY data_quality_score DESC
         LIMIT 500"
    } else {
        "SELECT trait_category, trait_name_reported, mapped_gene_symbol, rsid,
                personal_direction, data_quality_score, wellness_actionability_score,
                clinical_actionability_score
         FROM association_facts
         WHERE sample_id = ? AND data_quality_score >= ?
         ORDER BY data_quality_score DESC
         LIMIT 500"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows: Vec<(Option<String>, Option<String>, Option<String>, String, Option<String>, f32, f32, f32)> =
        if let Some(cat) = trait_category {
            stmt.query_map(params![sample_id, cat, min_dq], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get::<_, Option<f64>>(5)?.unwrap_or(0.0) as f32,
                    row.get::<_, Option<f64>>(6)?.unwrap_or(0.0) as f32,
                    row.get::<_, Option<f64>>(7)?.unwrap_or(0.0) as f32,
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
        } else {
            stmt.query_map(params![sample_id, min_dq], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get::<_, Option<f64>>(5)?.unwrap_or(0.0) as f32,
                    row.get::<_, Option<f64>>(6)?.unwrap_or(0.0) as f32,
                    row.get::<_, Option<f64>>(7)?.unwrap_or(0.0) as f32,
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
        };

    use std::collections::HashMap;
    let mut groups: HashMap<String, Vec<(Option<String>, Option<String>, String, Option<String>, f32, f32, f32)>> =
        HashMap::new();
    for (cat, trait_name, gene, rsid, direction, dq, wellness, clinical) in rows {
        let key = cat.clone().unwrap_or_else(|| "uncategorized".into());
        groups.entry(key).or_default().push((
            trait_name,
            gene,
            rsid,
            direction,
            dq,
            wellness,
            clinical,
        ));
    }

    let now = crate::research::util::unix_now();
    let mut summaries: Vec<TraitClusterSummary> = groups
        .into_iter()
        .take(limit as usize)
        .map(|(cat, items)| {
            let mut traits = Vec::new();
            let mut genes = Vec::new();
            let mut rsids = Vec::new();
            let mut known = 0u32;
            let mut unknown = 0u32;
            let mut dq_sum = 0.0f32;
            let mut wellness_sum = 0.0f32;
            let mut clinical_sum = 0.0f32;
            for (t, g, r, d, dq, w, c) in &items {
                if let Some(tn) = t
                    && !traits.contains(tn) {
                        traits.push(tn.clone());
                    }
                if let Some(gn) = g
                    && !genes.contains(gn) {
                        genes.push(gn.clone());
                    }
                if !rsids.contains(r) {
                    rsids.push(r.clone());
                }
                match d.as_deref() {
                    Some("increased_trait_value")
                    | Some("decreased_trait_value")
                    | Some("increased_odds")
                    | Some("decreased_odds") => known += 1,
                    _ => unknown += 1,
                }
                dq_sum += dq;
                wellness_sum += w;
                clinical_sum += c;
            }
            let n = items.len().max(1) as f32;
            let cluster_id = format!("cluster_{}_{}", cat.replace(' ', "_"), now);
            let low_risk_actions: Vec<String> = vec![
                "Review primary literature and study accession before acting.".to_string(),
                "Confirm effect allele orientation against your build.".to_string(),
            ];
            let escalation = if cat.contains("clinical") || clinical_sum / n > 0.4 {
                "Clinical significance may warrant licensed clinician review — do not self-diagnose from GWAS alone."
            } else {
                "Wellness/lifestyle context only — not diagnostic or prescribing guidance."
            };
            let _ = conn.execute(
                "INSERT OR REPLACE INTO hypothesis_clusters (
                    cluster_id, sample_id, cluster_title, cluster_type, trait_category,
                    top_traits_json, top_genes_json, top_rsids_json, association_count,
                    known_direction_count, unknown_direction_count, data_quality_score,
                    wellness_actionability_score, clinical_actionability_score,
                    verification_ideas_json, low_risk_actions_json, medical_escalation_boundary,
                    created_at, updated_at
                 ) VALUES (?, ?, ?, 'trait_category', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    cluster_id,
                    sample_id,
                    format!("{} cluster", cat),
                    cat,
                    serde_json::to_string(&traits).ok(),
                    serde_json::to_string(&genes).ok(),
                    serde_json::to_string(&rsids).ok(),
                    items.len() as i32,
                    known as i32,
                    unknown as i32,
                    dq_sum / n,
                    wellness_sum / n,
                    clinical_sum / n,
                    serde_json::to_string(&super::scoring::default_verification_ideas(Some(&cat))).ok(),
                    serde_json::to_string(&low_risk_actions).ok(),
                    escalation,
                    now,
                    now,
                ],
            );
            TraitClusterSummary {
                cluster_id,
                cluster_title: format!("{} associations", cat.replace('_', " ")),
                trait_category: Some(cat.clone()),
                top_traits: traits.into_iter().take(8).collect(),
                top_genes: genes.into_iter().take(8).collect(),
                top_rsids: rsids.into_iter().take(12).collect(),
                association_count: items.len() as u32,
                source_count: 1,
                known_direction_count: known,
                unknown_direction_count: unknown,
                data_quality_score: dq_sum / n,
                wellness_actionability_score: wellness_sum / n,
                clinical_actionability_score: clinical_sum / n,
                missing_field_count: unknown,
                verification_ideas: super::scoring::default_verification_ideas(Some(&cat)),
            }
        })
        .collect();

    summaries.sort_by(|a, b| {
        b.data_quality_score
            .partial_cmp(&a.data_quality_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(summaries)
}

pub fn list_actionability_points(
    conn: &Connection,
    sample_id: i64,
    limit: u32,
) -> Result<Vec<super::types::ActionabilityPoint>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT rsid, mapped_gene_symbol, trait_category,
                    MAX(clinical_actionability_score), MAX(wellness_actionability_score),
                    MAX(data_quality_score)
             FROM association_facts
             WHERE sample_id = ?
             GROUP BY rsid
             ORDER BY MAX(wellness_actionability_score) DESC, MAX(clinical_actionability_score) DESC
             LIMIT ?",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id, limit], |row| {
            Ok(super::types::ActionabilityPoint {
                rsid: row.get(0)?,
                gene_symbol: row.get(1)?,
                trait_category: row.get(2)?,
                clinical_actionability_score: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0) as f32,
                wellness_actionability_score: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0) as f32,
                data_quality_score: row.get::<_, Option<f64>>(5)?.unwrap_or(0.0) as f32,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn chromosome_trait_overlay(
    conn: &Connection,
    sample_id: i64,
) -> Result<Vec<super::types::ChromosomeTraitBand>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT g.chromosome, COALESCE(af.trait_category, 'unknown'), COUNT(DISTINCT af.rsid)
             FROM association_facts af
             JOIN genotypes g ON g.sample_id = af.sample_id AND g.rsid = af.rsid
             WHERE af.sample_id = ? AND g.chromosome IS NOT NULL AND g.chromosome != ''
             GROUP BY g.chromosome, COALESCE(af.trait_category, 'unknown')
             ORDER BY g.chromosome, COUNT(DISTINCT af.rsid) DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id], |row| {
            Ok(super::types::ChromosomeTraitBand {
                chromosome: row.get(0)?,
                trait_category: row.get(1)?,
                association_count: row.get::<_, i32>(2)? as u32,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn list_pathway_flow_rows(
    conn: &Connection,
    sample_id: i64,
    limit: u32,
) -> Result<Vec<super::types::PathwayFlowRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT mapped_gene_symbol, trait_name_reported, rsid
             FROM association_facts
             WHERE sample_id = ? AND source_name = 'reactome'
               AND mapped_gene_symbol IS NOT NULL AND trait_name_reported IS NOT NULL
             ORDER BY updated_at DESC
             LIMIT ?",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id, limit], |row| {
            let gene: String = row.get(0)?;
            let pathway: String = row.get(1)?;
            let rsid: String = row.get(2)?;
            Ok(super::types::PathwayFlowRow {
                gene_symbol: gene,
                pathway_name: pathway,
                trait_name: None,
                rsid: Some(rsid),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
