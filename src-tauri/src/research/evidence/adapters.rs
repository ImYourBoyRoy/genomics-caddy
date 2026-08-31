// ./src-tauri/src/research/evidence/adapters.rs
//! Live source adapters: PGS Catalog, Reactome, Open Targets, OLS4/EFO, PharmGKB/CPIC.

use super::cache::{
    fetch_json_cached, fetch_json_post_cached, opentargets_ttl, pgs_ttl, pharmgkb_ttl, reactome_ttl,
};
use super::ontology::{TraitOntologyMapping, map_trait_via_ols4};
use super::pgs_match::{PgsMatchSummary, compute_pgs_match_for_score};
use super::source_records;
use crate::research::http::{
    OLS4_SEMAPHORE, OPENTARGETS_SEMAPHORE, PGS_SEMAPHORE, PHARMGKB_SEMAPHORE, REACTOME_SEMAPHORE,
    acquire_adapter_permit,
};
use crate::research::util::{normalize_rsid, unix_now};
use serde_json::{Value, json};
use std::path::Path;

const PGS_BASE: &str = "https://www.pgscatalog.org/rest";
const REACTOME_BASE: &str = "https://reactome.org/ContentService";
const OPENTARGETS_GRAPHQL: &str = "https://api.platform.opentargets.org/api/v4/graphql";
const CLINPGX_BASE: &str = "https://api.clinpgx.org/v1";

#[derive(Debug, Clone, Default)]
pub struct SecondarySourceBundle {
    pub provenance: Value,
    pub narrative_lines: Vec<String>,
    pub ontology_mappings: Vec<TraitOntologyMapping>,
    pub pgs_matches: Vec<PgsMatchSummary>,
    pub pathway_names: Vec<String>,
    pub pgx_drugs: Vec<String>,
    pub open_targets_hits: u32,
}

pub async fn fetch_secondary_sources(
    db_path: &Path,
    rsid: &str,
    gene: Option<&str>,
    allele1: &str,
    allele2: &str,
    traits: &[String],
    sample_id: i64,
) -> SecondarySourceBundle {
    let rsid_norm = normalize_rsid(rsid).unwrap_or_else(|| rsid.to_uppercase());
    let primary_trait = traits.first().map(String::as_str);

    let pgs_fut = fetch_pgs_context(db_path, &rsid_norm, allele1, allele2, sample_id);
    let reactome_fut = fetch_reactome_context(db_path, gene);
    let ot_fut = fetch_open_targets_context(db_path, gene, primary_trait);
    let pgx_fut = fetch_pharmgkb_context(db_path, &rsid_norm, gene);
    let ols_fut = fetch_ols4_traits(db_path, traits);

    let (pgs, reactome, ot, pgx, ols) =
        tokio::join!(pgs_fut, reactome_fut, ot_fut, pgx_fut, ols_fut);

    let mut provenance = serde_json::Map::new();
    provenance.insert("pgs_catalog".into(), pgs.provenance);
    provenance.insert("reactome".into(), reactome.provenance);
    provenance.insert("open_targets".into(), ot.provenance);
    provenance.insert("pharmgkb".into(), pgx.provenance);
    provenance.insert("ols4".into(), ols.provenance);

    let mut narrative_lines = Vec::new();
    narrative_lines.extend(pgs.narrative);
    narrative_lines.extend(reactome.narrative);
    narrative_lines.extend(ot.narrative);
    narrative_lines.extend(pgx.narrative);

    SecondarySourceBundle {
        provenance: Value::Object(provenance),
        narrative_lines,
        ontology_mappings: ols.mappings,
        pgs_matches: pgs.matches,
        pathway_names: reactome.pathway_names,
        pgx_drugs: pgx.drugs,
        open_targets_hits: ot.hit_count,
    }
}

struct PgsSlice {
    provenance: Value,
    narrative: Vec<String>,
    matches: Vec<PgsMatchSummary>,
}

struct ReactomeSlice {
    provenance: Value,
    narrative: Vec<String>,
    pathway_names: Vec<String>,
}

struct OpenTargetsSlice {
    provenance: Value,
    narrative: Vec<String>,
    hit_count: u32,
}

struct PharmGkbSlice {
    provenance: Value,
    narrative: Vec<String>,
    drugs: Vec<String>,
}

struct OlsSlice {
    provenance: Value,
    mappings: Vec<TraitOntologyMapping>,
}

async fn fetch_pgs_context(
    db_path: &Path,
    rsid: &str,
    allele1: &str,
    allele2: &str,
    sample_id: i64,
) -> PgsSlice {
    let _permit = acquire_adapter_permit(&PGS_SEMAPHORE).await;
    let url = format!("{}/variant/{}", PGS_BASE, rsid);
    let cache_key = format!("pgs_catalog|variant|{}", rsid);

    let body = match fetch_json_cached(
        db_path,
        "pgs_catalog",
        "variant",
        &cache_key,
        &url,
        pgs_ttl(),
        None,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return PgsSlice {
                provenance: json!({ "queried": true, "hit": false, "error": e }),
                narrative: vec![],
                matches: vec![],
            };
        }
    };

    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = source_records::record_generic_json(
            &conn,
            "pgs_catalog",
            "variant",
            Some(rsid),
            None,
            None,
            Some(&url),
            &body,
        );
    }

    let scores = body.as_array().cloned().unwrap_or_default();
    if scores.is_empty() {
        return PgsSlice {
            provenance: json!({ "queried": true, "hit": false, "scores": 0 }),
            narrative: vec![],
            matches: vec![],
        };
    }

    let mut matches = Vec::new();
    let mut narrative = Vec::new();
    for score_ref in scores.iter().take(3) {
        let pgs_id = score_ref["id"]
            .as_str()
            .or_else(|| score_ref["score_id"].as_str());
        let Some(pgs_id) = pgs_id else { continue };

        let score_url = format!("{}/score/{}", PGS_BASE, pgs_id);
        let score_key = format!("pgs_catalog|score|{}", pgs_id);
        let score_meta = fetch_json_cached(
            db_path,
            "pgs_catalog",
            "score",
            &score_key,
            &score_url,
            pgs_ttl(),
            None,
        )
        .await
        .unwrap_or(json!({}));

        if let Ok(conn) = crate::db::connect(db_path) {
            let trait_name = score_meta["trait_reported"].as_str();
            let _ = source_records::record_generic_json(
                &conn,
                "pgs_catalog",
                "score",
                Some(rsid),
                None,
                trait_name,
                Some(&score_url),
                &score_meta,
            );
        }

        if let Ok(summary) = compute_pgs_match_for_score(
            db_path,
            sample_id,
            pgs_id,
            rsid,
            allele1,
            allele2,
            &score_meta,
        )
        .await
        {
            narrative.push(format!(
                "PGS {} ({}): variant match={}, score match rate {:.0}% ({}/{} variants)",
                pgs_id,
                summary.trait_reported.as_deref().unwrap_or("unknown trait"),
                if summary.variant_allele_match {
                    "yes"
                } else {
                    "no"
                },
                summary.match_rate_pct,
                summary.matched_variants,
                summary.total_variants
            ));
            matches.push(summary);
        }
    }

    PgsSlice {
        provenance: json!({
            "queried": true,
            "hit": !scores.is_empty(),
            "scores": scores.len(),
            "match_summaries": matches.len(),
        }),
        narrative,
        matches,
    }
}

async fn fetch_reactome_context(db_path: &Path, gene: Option<&str>) -> ReactomeSlice {
    let _permit = acquire_adapter_permit(&REACTOME_SEMAPHORE).await;
    let Some(gene_symbol) = gene.filter(|g| !g.trim().is_empty()) else {
        return ReactomeSlice {
            provenance: json!({ "queried": false, "hit": false, "reason": "no_gene" }),
            narrative: vec![],
            pathway_names: vec![],
        };
    };

    let url = format!(
        "{}/data/gene/{}/pathways?species=9606",
        REACTOME_BASE, gene_symbol
    );
    let cache_key = format!("reactome|gene_pathways|{}", gene_symbol.to_uppercase());

    let body = match fetch_json_cached(
        db_path,
        "reactome",
        "gene_pathways",
        &cache_key,
        &url,
        reactome_ttl(),
        None,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return ReactomeSlice {
                provenance: json!({ "queried": true, "hit": false, "error": e }),
                narrative: vec![],
                pathway_names: vec![],
            };
        }
    };

    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = source_records::record_generic_json(
            &conn,
            "reactome",
            "gene_pathways",
            None,
            Some(gene_symbol),
            None,
            Some(&url),
            &body,
        );
    }

    let pathways: Vec<String> = body
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p["displayName"].as_str().map(String::from))
                .take(8)
                .collect()
        })
        .unwrap_or_default();

    let narrative = if pathways.is_empty() {
        vec![]
    } else {
        vec![format!(
            "Reactome pathways for {}: {}",
            gene_symbol,
            pathways.join("; ")
        )]
    };

    ReactomeSlice {
        provenance: json!({
            "queried": true,
            "hit": !pathways.is_empty(),
            "hits": pathways.len(),
            "gene": gene_symbol,
        }),
        narrative,
        pathway_names: pathways,
    }
}

async fn fetch_open_targets_context(
    db_path: &Path,
    gene: Option<&str>,
    trait_name: Option<&str>,
) -> OpenTargetsSlice {
    let _permit = acquire_adapter_permit(&OPENTARGETS_SEMAPHORE).await;
    let Some(gene_symbol) = gene.filter(|g| !g.trim().is_empty()) else {
        return OpenTargetsSlice {
            provenance: json!({ "queried": false, "hit": false, "reason": "no_gene" }),
            narrative: vec![],
            hit_count: 0,
        };
    };

    let query = r#"
        query targetSearch($queryString: String!) {
          search(queryString: $queryString, entityNames: ["target"], page: { index: 0, size: 1 }) {
            hits { id name entity description
              object {
                ... on Target { id approvedSymbol biotype }
              }
            }
          }
        }
    "#;
    let variables = json!({ "queryString": gene_symbol });
    let cache_key = format!(
        "opentargets|target_search|{}",
        super::source_records::payload_hash(&format!(
            "{}|{}",
            gene_symbol,
            trait_name.unwrap_or("")
        ))
    );

    let body = match fetch_json_post_cached(
        db_path,
        "open_targets",
        "graphql",
        &cache_key,
        OPENTARGETS_GRAPHQL,
        query,
        &variables,
        opentargets_ttl(),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return OpenTargetsSlice {
                provenance: json!({ "queried": true, "hit": false, "error": e }),
                narrative: vec![],
                hit_count: 0,
            };
        }
    };

    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = source_records::record_generic_json(
            &conn,
            "open_targets",
            "graphql",
            None,
            Some(gene_symbol),
            trait_name,
            Some(OPENTARGETS_GRAPHQL),
            &body,
        );
    }

    let hits = body["data"]["search"]["hits"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0) as u32;

    let top_name = body["data"]["search"]["hits"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|h| h["name"].as_str())
        .unwrap_or(gene_symbol);

    let narrative = if hits > 0 {
        vec![format!(
            "Open Targets target context: {} (integrated disease/drug evidence available)",
            top_name
        )]
    } else {
        vec![]
    };

    OpenTargetsSlice {
        provenance: json!({
            "queried": true,
            "hit": hits > 0,
            "hits": hits,
            "gene": gene_symbol,
            "platform": "v4",
        }),
        narrative,
        hit_count: hits,
    }
}

async fn fetch_pharmgkb_context(db_path: &Path, rsid: &str, gene: Option<&str>) -> PharmGkbSlice {
    if let Ok(conn) = crate::db::connect(db_path) {
        let (drugs, annotation_count, hit) = crate::offline::lookup_pharmgkb_local(&conn, rsid);
        if hit {
            let narrative = if drugs.is_empty() {
                format!(
                    "PharmGKB (local): {} PGx annotations — context only",
                    annotation_count
                )
            } else {
                format!(
                    "PharmGKB (local, {} annotations): drugs {} — context only",
                    annotation_count,
                    drugs.join(", ")
                )
            };
            return PharmGkbSlice {
                provenance: json!({
                    "queried": true,
                    "hit": true,
                    "local": true,
                    "annotations": annotation_count,
                }),
                narrative: vec![narrative],
                drugs,
            };
        }
    }

    let _permit = acquire_adapter_permit(&PHARMGKB_SEMAPHORE).await;
    let url = format!(
        "{}/data/clinicalAnnotation?location.rsId={}",
        CLINPGX_BASE, rsid
    );
    let cache_key = format!("pharmgkb|clinical_annotation|{}", rsid);

    let mut drugs = Vec::new();
    let mut hit = false;
    let mut annotation_count = 0u32;

    if let Ok(body) = fetch_json_cached(
        db_path,
        "pharmgkb",
        "clinical_annotation",
        &cache_key,
        &url,
        pharmgkb_ttl(),
        None,
    )
    .await
    {
        if let Ok(conn) = crate::db::connect(db_path) {
            let _ = source_records::record_generic_json(
                &conn,
                "pharmgkb",
                "clinical_annotation",
                Some(rsid),
                gene,
                None,
                Some(&url),
                &body,
            );
        }
        if let Some(data) = body["data"].as_array() {
            annotation_count = data.len() as u32;
            hit = !data.is_empty();
            for ann in data.iter().take(5) {
                if let Some(chemicals) = ann["relatedChemicals"].as_array() {
                    for c in chemicals {
                        if let Some(name) = c["name"].as_str()
                            && !drugs.contains(&name.to_string())
                        {
                            drugs.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    // ClinPGx variant lookup as supplement
    let clinpgx_url = format!("{}/variant?name={}", CLINPGX_BASE, rsid);
    let clinpgx_key = format!("clinpgx|variant|{}", rsid);
    if let Ok(cpgx) = fetch_json_cached(
        db_path,
        "clinpgx",
        "variant",
        &clinpgx_key,
        &clinpgx_url,
        pharmgkb_ttl(),
        None,
    )
    .await
    {
        if let Ok(conn) = crate::db::connect(db_path) {
            let _ = source_records::record_generic_json(
                &conn,
                "clinpgx",
                "variant",
                Some(rsid),
                gene,
                None,
                Some(&clinpgx_url),
                &cpgx,
            );
        }
        if cpgx["data"].is_array() && !cpgx["data"].as_array().unwrap_or(&vec![]).is_empty() {
            hit = true;
        }
    }

    let narrative = if hit {
        let drug_str = if drugs.is_empty() {
            "PGx context only — review with clinician/pharmacist; diplotype unavailable from raw SNP data"
                .to_string()
        } else {
            format!(
                "PharmGKB PGx context ({} annotations): drugs {} — context only, not dosing guidance",
                annotation_count,
                drugs.join(", ")
            )
        };
        vec![drug_str]
    } else {
        vec![]
    };

    PharmGkbSlice {
        provenance: json!({
            "queried": true,
            "hit": hit,
            "annotations": annotation_count,
            "drugs": drugs.len(),
            "context_only": true,
            "diplotype_unavailable": true,
        }),
        narrative,
        drugs,
    }
}

async fn fetch_ols4_traits(db_path: &Path, traits: &[String]) -> OlsSlice {
    let _permit = acquire_adapter_permit(&OLS4_SEMAPHORE).await;
    let mut mappings = Vec::new();
    for trait_name in traits.iter().take(4).filter(|t| !t.trim().is_empty()) {
        if let Some(mapping) = map_trait_via_ols4(db_path, trait_name).await {
            mappings.push(mapping);
        }
    }

    OlsSlice {
        provenance: json!({
            "queried": !traits.is_empty(),
            "hit": !mappings.is_empty(),
            "mapped_traits": mappings.len(),
            "mappings": mappings.iter().map(|m| json!({
                "reported": m.reported_trait,
                "mapped": m.mapped_label,
                "ontology_id": m.ontology_id,
                "confidence": m.confidence,
            })).collect::<Vec<_>>(),
        }),
        mappings,
    }
}

pub fn best_ontology_mapping(mappings: &[TraitOntologyMapping]) -> Option<&TraitOntologyMapping> {
    mappings.iter().max_by(|a, b| {
        a.confidence
            .partial_cmp(&b.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

pub fn ledger_from_pgs_match(
    summary: &PgsMatchSummary,
    _genotype: &str,
) -> super::types::EvidenceLedgerRow {
    super::types::EvidenceLedgerRow {
        source_name: "pgs_catalog".into(),
        trait_name: summary.trait_reported.clone(),
        mapped_gene: None,
        p_value: None,
        beta: summary.effect_weight,
        odds_ratio: None,
        effect_allele: summary.effect_allele.clone(),
        personal_dosage: summary.personal_dosage,
        personal_direction: summary.personal_direction.clone(),
        study_accession: None,
        pubmed_id: None,
        ancestry: None,
        sample_size: Some(summary.total_variants as i32),
        fetched_at: Some(unix_now()),
        source_release: None,
        raw_payload_hash: Some(summary.pgs_id.clone()),
        quality_flags: vec!["polygenic_score".into(), "not_medical_actionable".into()],
        evidence_tier: "polygenic_score".into(),
        association_type: "pgs_weight".into(),
    }
}

pub fn ledger_from_pathway(
    gene: &str,
    pathway: &str,
    source_record_id: Option<&str>,
) -> super::types::EvidenceLedgerRow {
    super::types::EvidenceLedgerRow {
        source_name: "reactome".into(),
        trait_name: Some(pathway.to_string()),
        mapped_gene: Some(gene.to_string()),
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
        raw_payload_hash: source_record_id.map(String::from),
        quality_flags: vec!["pathway_context".into(), "not_medical_actionable".into()],
        evidence_tier: "pathway_context".into(),
        association_type: "pathway_mapping".into(),
    }
}

pub fn ledger_from_pgx(
    rsid: &str,
    gene: Option<&str>,
    drug: &str,
) -> super::types::EvidenceLedgerRow {
    super::types::EvidenceLedgerRow {
        source_name: "pharmgkb".into(),
        trait_name: Some(format!("drug: {}", drug)),
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
        raw_payload_hash: Some(rsid.to_string()),
        quality_flags: vec![
            "pharmacogenomic_context".into(),
            "not_medical_actionable".into(),
            "requires_human_review".into(),
        ],
        evidence_tier: "pharmacogenomic_guideline".into(),
        association_type: "pharmacogenomic_annotation".into(),
    }
}
