// ./src-tauri/src/research/evidence/packet.rs
//! Export structured evidence packets for MCP / frontier model review.

use super::card::evidence_card_from_payload;
use super::search::get_similar_associations;
use super::store::facts_for_rsid;
use super::types::{EvidencePacket, SimilarSearchParams};
use crate::research::qdrant::find_point_payload_by_rsid;
use crate::research::types::QdrantConfig;
use crate::research::util::unix_now;
use std::path::Path;

pub async fn export_evidence_packet(
    db_path: &Path,
    sample_id: i64,
    rsid: Option<&str>,
    cluster_id: Option<&str>,
    association_ids: Option<Vec<String>>,
    config: &QdrantConfig,
) -> Result<EvidencePacket, String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let packet_id = format!("pkt_{}", unix_now());
    let now = unix_now();

    let mut association_facts = Vec::new();
    let mut evidence_cards = Vec::new();
    let mut similar = Vec::new();
    let mut quality_flags = Vec::new();
    let mut missing_fields = Vec::new();
    let mut conflicts = Vec::new();
    let mut genotype = None;
    let mut source_records = Vec::new();

    if let Some(r) = rsid {
        association_facts = facts_for_rsid(&conn, sample_id, r)?;
        source_records =
            super::source_records::list_source_records_for_rsid(&conn, r, 24).unwrap_or_default();
        if let Some((payload, point_id)) = find_point_payload_by_rsid(
            &config.url,
            config.api_key.as_deref(),
            &config.collection,
            sample_id,
            r,
        )
        .await?
        {
            genotype = payload["genotype"].as_str().map(String::from);
            let card = evidence_card_from_payload(&payload, 1.0, None, Some(point_id));
            quality_flags.extend(card.quality_flags.clone());
            missing_fields.extend(card.missing_fields.clone());
            evidence_cards.push(card);
        }
        similar = get_similar_associations(
            &SimilarSearchParams {
                sample_id,
                rsid: Some(r.to_string()),
                qdrant_point_id: None,
                similarity_mode: "evidence_similarity".into(),
                limit: 8,
                include_self: false,
            },
            config,
        )
        .await
        .unwrap_or_default();
    }

    if let Some(ids) = association_ids {
        for id in ids {
            let fact: Option<serde_json::Value> = conn
                .query_row(
                    "SELECT rsid, source_name, trait_name_reported, personal_direction, quality_flags_json
                     FROM association_facts WHERE association_id = ? AND sample_id = ?",
                    rusqlite::params![id, sample_id],
                    |row| {
                        Ok(serde_json::json!({
                            "association_id": id,
                            "rsid": row.get::<_, String>(0)?,
                            "source_name": row.get::<_, String>(1)?,
                            "trait_name": row.get::<_, Option<String>>(2)?,
                            "personal_direction": row.get::<_, Option<String>>(3)?,
                            "quality_flags": row.get::<_, Option<String>>(4)?,
                        }))
                    },
                )
                .ok();
            if let Some(f) = fact {
                association_facts.push(f);
            }
        }
    }

    if let Some(cid) = cluster_id {
        let cluster: Option<(String, String, String)> = conn
            .query_row(
                "SELECT top_traits_json, top_rsids_json, verification_ideas_json
                 FROM hypothesis_clusters WHERE cluster_id = ? AND sample_id = ?",
                rusqlite::params![cid, sample_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();
        if let Some((traits_json, rsids_json, ideas_json)) = cluster {
            if let Ok(rsids) = serde_json::from_str::<Vec<String>>(&rsids_json) {
                for r in rsids.iter().take(5) {
                    let facts = facts_for_rsid(&conn, sample_id, r).unwrap_or_default();
                    association_facts.extend(facts);
                }
            }
            if let Ok(ideas) = serde_json::from_str::<Vec<String>>(&ideas_json) {
                quality_flags.extend(ideas);
            }
            let _ = traits_json;
        }
    }

    quality_flags.sort();
    quality_flags.dedup();
    missing_fields.sort();
    missing_fields.dedup();

    if quality_flags.iter().any(|f| f.contains("conflict")) {
        conflicts.push("Conflicting source interpretations detected".into());
    }

    let scores = serde_json::json!({
        "association_strength": evidence_cards.first().map(|c| c.association_strength_score),
        "data_quality": evidence_cards.first().map(|c| c.data_quality_score),
        "wellness_actionability": evidence_cards.first().map(|c| c.wellness_actionability_score),
        "clinical_actionability": evidence_cards.first().map(|c| c.clinical_actionability_score),
    });

    Ok(EvidencePacket {
        packet_id,
        generated_at: now,
        sample_id,
        rsid: rsid.map(String::from),
        cluster_id: cluster_id.map(String::from),
        genotype,
        association_facts,
        evidence_cards,
        similar_associations: similar,
        quality_flags: quality_flags.clone(),
        missing_fields,
        conflicts,
        scores,
        verification_ideas: super::scoring::default_verification_ideas(None),
        prohibited_claims: super::scoring::default_prohibited_claims(),
        research_questions: default_research_questions(rsid),
        source_records,
    })
}

fn default_research_questions(rsid: Option<&str>) -> Vec<String> {
    let mut qs = vec![
        "Which primary study accessions support this association?".into(),
        "Is effect allele orientation harmonized for this build?".into(),
        "What metadata is missing that blocks personal direction?".into(),
    ];
    if let Some(r) = rsid {
        qs.push(format!(
            "What free public sources should be queried next for {}?",
            r
        ));
    }
    qs
}
