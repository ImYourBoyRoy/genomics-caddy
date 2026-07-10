// ./src-tauri/src/research/evidence/corpus.rs
//! Corpus-level summaries and filter-only browse (no user query required).

use super::card::evidence_card_from_payload;
use super::dashboard::{build_quality_dashboard, build_trait_clusters, list_actionability_points};
use super::types::{
    ActionabilityPoint, BrowseAssociationsParams, EvidenceCard, EvidenceCorpusSummary,
    QualityDashboard, TraitBucketSummary,
};
use crate::research::qdrant::find_point_payload_by_rsid;
use crate::research::types::QdrantConfig;
use rusqlite::{Connection, params};
use std::path::Path;

fn trait_buckets(conn: &Connection, sample_id: i64) -> Result<Vec<TraitBucketSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT COALESCE(trait_category, 'uncategorized'), COUNT(DISTINCT rsid)
             FROM association_facts
             WHERE sample_id = ?
             GROUP BY COALESCE(trait_category, 'uncategorized')
             ORDER BY COUNT(DISTINCT rsid) DESC
             LIMIT 16",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id], |row| {
            Ok(TraitBucketSummary {
                trait_category: row.get(0)?,
                variant_count: row.get::<_, i64>(1)? as u64,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn build_index_brief(
    dashboard: &QualityDashboard,
    buckets: &[TraitBucketSummary],
    top: &[ActionabilityPoint],
) -> String {
    let indexed = dashboard.vectorized_variants.unwrap_or(0).to_string();
    let facts = dashboard.association_fact_count;
    let avg_dq = dashboard.avg_data_quality_score;
    let known = dashboard.known_direction_count;
    let unknown = dashboard.unknown_direction_count;

    let top_cats: String = buckets
        .iter()
        .take(4)
        .map(|b| {
            format!(
                "{} ({})",
                b.trait_category.replace('_', " "),
                b.variant_count
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let top_rsids: String = top
        .iter()
        .take(5)
        .map(|p| p.rsid.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "Vector index: {indexed} enriched variants in Qdrant ({facts} association facts, avg data quality {avg_dq:.2}). \
         Direction: {known} known, {unknown} unknown. \
         Top trait categories: {top_cats}. \
         Highest-actionability rsIDs: {top_rsids}. \
         Browse presets below or search semantically — consultation uses the same hybrid cards."
    )
}

fn suggested_questions(buckets: &[TraitBucketSummary], top: &[ActionabilityPoint]) -> Vec<String> {
    let mut out = vec![
        "What are my highest wellness-actionability variants and what do they mean?".into(),
        "Summarize unknown-direction variants I should investigate first.".into(),
        "Which indexed findings overlap my curated trait report?".into(),
    ];
    if let Some(b) = buckets.first() {
        out.push(format!(
            "Explain my top {} associations from the vector index.",
            b.trait_category.replace('_', " ")
        ));
    }
    if let Some(p) = top.first()
        && let Some(g) = &p.gene_symbol
    {
        out.push(format!(
            "What does {} ({}) suggest for my genotype?",
            g, p.rsid
        ));
    }
    out.truncate(6);
    out
}

pub async fn build_evidence_corpus_summary(
    db_path: &Path,
    sample_id: i64,
    config: &QdrantConfig,
    enrichment_enriched: Option<u64>,
    enrichment_total: Option<u64>,
) -> Result<EvidenceCorpusSummary, String> {
    let enrichment_status = {
        let db_path2 = db_path.to_path_buf();
        tauri::async_runtime::spawn_blocking(move || {
            super::super::get_research_job_from_db(&db_path2, sample_id).map(|j| j.status)
        })
        .await
        .ok()
        .flatten()
    };

    let dashboard = build_quality_dashboard(db_path, sample_id, config, enrichment_status).await?;

    let (buckets, top_actionable, top_clusters) = tauri::async_runtime::spawn_blocking({
        let db_path = db_path.to_path_buf();
        move || {
            let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
            let buckets = trait_buckets(&conn, sample_id)?;
            let top_actionable = list_actionability_points(&conn, sample_id, 24)?;
            let top_clusters = build_trait_clusters(&conn, sample_id, None, Some(0.25), 6)?;
            Ok::<_, String>((buckets, top_actionable, top_clusters))
        }
    })
    .await
    .map_err(|e| format!("Corpus summary worker failed: {}", e))??;

    let index_brief = build_index_brief(&dashboard, &buckets, &top_actionable);
    let suggested_questions = suggested_questions(&buckets, &top_actionable);

    let enrichment_progress_pct = match (enrichment_enriched, enrichment_total) {
        (Some(e), Some(t)) if t > 0 => Some((e as f32 / t as f32) * 100.0),
        _ => None,
    };

    Ok(EvidenceCorpusSummary {
        sample_id,
        dashboard,
        trait_buckets: buckets,
        top_actionable,
        top_clusters,
        index_brief,
        suggested_questions,
        enrichment_progress_pct,
    })
}

fn list_rsids_for_browse(
    conn: &Connection,
    sample_id: i64,
    preset: &str,
    trait_category: Option<&str>,
    limit: u32,
) -> Result<Vec<String>, String> {
    let limit = limit as i64;
    let rsids: Vec<String> = match preset {
        "clinical" => {
            if let Some(cat) = trait_category {
                let mut stmt = conn
                    .prepare(
                        "SELECT rsid FROM (
                        SELECT rsid, MAX(clinical_actionability_score) AS score
                        FROM association_facts
                        WHERE sample_id = ? AND trait_category = ?
                        GROUP BY rsid
                        HAVING score >= 0.25
                        ORDER BY score DESC
                        LIMIT ?
                     )",
                    )
                    .map_err(|e| e.to_string())?;
                stmt.query_map(params![sample_id, cat, limit], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect()
            } else {
                let mut stmt = conn
                    .prepare(
                        "SELECT rsid FROM (
                        SELECT rsid, MAX(clinical_actionability_score) AS score
                        FROM association_facts
                        WHERE sample_id = ?
                        GROUP BY rsid
                        HAVING score >= 0.25
                        ORDER BY score DESC
                        LIMIT ?
                     )",
                    )
                    .map_err(|e| e.to_string())?;
                stmt.query_map(params![sample_id, limit], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect()
            }
        }
        "gwas" => {
            if let Some(cat) = trait_category {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT rsid FROM association_facts
                     WHERE sample_id = ? AND association_type = 'gwas_top_association' AND trait_category = ?
                     ORDER BY data_quality_score DESC
                     LIMIT ?",
                ).map_err(|e| e.to_string())?;
                stmt.query_map(params![sample_id, cat, limit], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect()
            } else {
                let mut stmt = conn
                    .prepare(
                        "SELECT DISTINCT rsid FROM association_facts
                     WHERE sample_id = ? AND association_type = 'gwas_top_association'
                     ORDER BY data_quality_score DESC
                     LIMIT ?",
                    )
                    .map_err(|e| e.to_string())?;
                stmt.query_map(params![sample_id, limit], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect()
            }
        }
        "unknown" => {
            if let Some(cat) = trait_category {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT rsid FROM association_facts
                     WHERE sample_id = ? AND trait_category = ?
                       AND (personal_direction IS NULL OR personal_direction IN ('unknown','possible_relevance_unknown_direction'))
                     ORDER BY data_quality_score DESC
                     LIMIT ?",
                ).map_err(|e| e.to_string())?;
                stmt.query_map(params![sample_id, cat, limit], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect()
            } else {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT rsid FROM association_facts
                     WHERE sample_id = ?
                       AND (personal_direction IS NULL OR personal_direction IN ('unknown','possible_relevance_unknown_direction'))
                     ORDER BY data_quality_score DESC
                     LIMIT ?",
                ).map_err(|e| e.to_string())?;
                stmt.query_map(params![sample_id, limit], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect()
            }
        }
        _ => {
            if let Some(cat) = trait_category {
                let mut stmt = conn
                    .prepare(
                        "SELECT rsid FROM (
                        SELECT rsid, MAX(wellness_actionability_score) AS score
                        FROM association_facts
                        WHERE sample_id = ? AND trait_category = ?
                        GROUP BY rsid
                        HAVING score >= 0.25
                        ORDER BY score DESC
                        LIMIT ?
                     )",
                    )
                    .map_err(|e| e.to_string())?;
                stmt.query_map(params![sample_id, cat, limit], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect()
            } else {
                let mut stmt = conn
                    .prepare(
                        "SELECT rsid FROM (
                        SELECT rsid, MAX(wellness_actionability_score) AS score
                        FROM association_facts
                        WHERE sample_id = ?
                        GROUP BY rsid
                        HAVING score >= 0.25
                        ORDER BY score DESC
                        LIMIT ?
                     )",
                    )
                    .map_err(|e| e.to_string())?;
                stmt.query_map(params![sample_id, limit], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect()
            }
        }
    };
    Ok(rsids)
}

pub async fn browse_associations(
    params: &BrowseAssociationsParams,
    config: &QdrantConfig,
    db_path: &Path,
) -> Result<Vec<EvidenceCard>, String> {
    let limit = params.limit.clamp(5, 100);
    let sample_id = params.sample_id;
    let preset = params.preset.clone();
    let trait_cat = params.trait_category.clone();

    let rsids: Vec<String> = {
        let db_path = db_path.to_path_buf();
        tauri::async_runtime::spawn_blocking(move || {
            let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
            list_rsids_for_browse(&conn, sample_id, &preset, trait_cat.as_deref(), limit)
        })
        .await
        .map_err(|e| format!("Browse worker failed: {}", e))??
    };

    let mut cards = Vec::new();
    for rsid in rsids {
        if let Some((payload, point_id)) = find_point_payload_by_rsid(
            &config.url,
            config.api_key.as_deref(),
            &config.collection,
            params.sample_id,
            &rsid,
        )
        .await?
        {
            let score = payload["wellness_actionability_score"]
                .as_f64()
                .or_else(|| payload["data_quality_score"].as_f64())
                .unwrap_or(0.5) as f32;
            cards.push(evidence_card_from_payload(
                &payload,
                score,
                None,
                Some(point_id),
            ));
        }
    }

    cards.sort_by(|a, b| {
        b.wellness_actionability_score
            .partial_cmp(&a.wellness_actionability_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(cards)
}
