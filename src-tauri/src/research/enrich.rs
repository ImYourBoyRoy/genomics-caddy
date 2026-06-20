// ./src-tauri/src/research/enrich.rs
use super::embed::embed_text;
use super::tuning::skip_gnomad_in_sweep;
use super::markers::score_marker_significance;
use super::qdrant::upsert_to_qdrant;
use super::sources::{fetch_ensembl_vep_genes, fetch_gtex_eqtls_for_rsid, resolve_gwas_associations};
use super::gnomad::fetch_gnomad_for_enrichment;
use super::sweep_metrics::{record_phase, timed_async, SweepPhase};
use std::time::Instant;
use super::state::NCBI_SEMAPHORE;
use super::types::*;
use super::util::{
    best_gwas_pvalue, build_enrichment_narrative, build_enrichment_payload, clean_pubmed_abstract,
    collect_gene_candidates, extract_gwas_traits, lookup_variant_locus, resolve_gene_name,
};
use super::promote::maybe_promote_vector_finding;
use super::crossmap::{build_cross_map_context, lookup_discovery_catalog_gene};
use super::tuning::{
    prepare_concurrency, prepare_variant_timeout_secs, gnomad_remote_timeout_secs,
    enrichment_batch_timeout_secs,
};
use std::path::Path;
use std::time::Duration;
use tokio::task::JoinSet;

pub(crate) async fn fetch_pubmed_abstracts(
    db_path: &Path,
    rsid: &str,
    ncbi_api_key: Option<&str>,
) -> Vec<String> {
    let _permit = NCBI_SEMAPHORE.acquire().await.ok();

    let mut search_url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term={}[SNP]&retmax=2&retmode=json",
        rsid
    );
    if let Some(key) = ncbi_api_key
        && !key.trim().is_empty() {
            search_url = format!("{}&api_key={}", search_url, key.trim());
        }

    let search_val = match crate::agent::fetch_api_cached(
        db_path,
        &search_url,
        ncbi_api_key,
        86400 * 7,
    )
    .await
    {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = crate::research::evidence::source_records::record_pubmed_search(
            &conn,
            rsid,
            &search_url,
            &search_val,
        );
    }

    let ids = match search_val["esearchresult"]["idlist"].as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<_>>(),
        None => return vec![],
    };

    if ids.is_empty() {
        return vec![];
    }

    let id_list = ids.join(",");
    let fetch_url = {
        let mut u = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi?db=pubmed&id={}&rettype=abstract&retmode=text",
            id_list
        );
        if let Some(key) = ncbi_api_key
            && !key.trim().is_empty() {
                u = format!("{}&api_key={}", u, key.trim());
            }
        u
    };

    let cache_key = format!("pubmed|efetch|{}", id_list);
    let text = match crate::research::evidence::cache::fetch_text_cached(
        db_path,
        "pubmed",
        "efetch",
        &cache_key,
        &fetch_url,
        crate::research::evidence::cache::pubmed_ttl(),
    )
    .await
    {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    text.split("PMID:")
        .skip(1)
        .take(2)
        .map(|chunk| {
            let trimmed = clean_pubmed_abstract(&format!("PMID:{}", chunk.trim()));
            if trimmed.is_empty() {
                String::new()
            } else {
                trimmed
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

pub(crate) struct PreparedEnrichment {
    pub(crate) rsid: String,
    pub(crate) point_id: String,
    pub(crate) full_text: String,
    pub(crate) payload: serde_json::Value,
    pub(crate) named_vectors: Option<std::collections::HashMap<String, Vec<f32>>>,
}

impl Clone for PreparedEnrichment {
    fn clone(&self) -> Self {
        Self {
            rsid: self.rsid.clone(),
            point_id: self.point_id.clone(),
            full_text: self.full_text.clone(),
            payload: self.payload.clone(),
            named_vectors: self.named_vectors.clone(),
        }
    }
}

pub(crate) async fn prepare_marker_enrichment(
    sample_id: i64,
    rsid: &str,
    gene: Option<&str>,
    allele1: &str,
    allele2: &str,
    clinvar_sig: Option<&str>,
    db_path: &Path,
    config: &QdrantConfig,
) -> Result<PreparedEnrichment, String> {
    let prefetch_gene = resolve_gene_name(db_path, rsid, gene, &[]).0;
    let gwas_started = Instant::now();
    let (gwas_assocs, mut sources_provenance) =
        resolve_gwas_associations(db_path, rsid, false).await;
    record_phase(SweepPhase::Gwas, gwas_started.elapsed());

    let mut live_clinvar_sig = clinvar_sig.filter(|s| !s.is_empty()).map(String::from);
    let mut ncbi_narrative_lines: Vec<String> = Vec::new();
    if live_clinvar_sig.is_none() && super::sources_config::enrichment_clinvar_live_enabled() {
        let clinvar_started = Instant::now();
        let live = super::evidence::ncbi_context::fetch_clinvar_live(
            db_path,
            rsid,
            config.ncbi_api_key.as_deref(),
        )
        .await;
        record_phase(SweepPhase::Clinvar, clinvar_started.elapsed());
        if let Some(obj) = sources_provenance.as_object_mut() {
            obj.insert("clinvar_eutils".into(), live.provenance);
        }
        if let Some(sig) = live.significance.filter(|s| !s.is_empty()) {
            live_clinvar_sig = Some(sig);
        }
        if let Some(line) = live.narrative {
            ncbi_narrative_lines.push(line);
        }
    }
    let effective_clinvar = live_clinvar_sig.as_deref();

    let needs_pubmed = super::sources_config::enrichment_pubmed_enabled()
        && (effective_clinvar.is_some() || !gwas_assocs.is_empty());
    let needs_gtex = super::sources_config::enrichment_gtex_enabled() && prefetch_gene.is_some();

    let data_dir = db_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| db_path.to_path_buf());
    let gnomad_fut = async {
        if skip_gnomad_in_sweep() {
            (None, super::gnomad::GnomadContext::empty(super::gnomad::GnomadLookupStatus::NotQueried))
        } else {
            let started = Instant::now();
            let result =
                fetch_gnomad_for_enrichment(db_path, &data_dir, sample_id, rsid, allele1, allele2)
                    .await;
            record_phase(SweepPhase::Gnomad, started.elapsed());
            result
        }
    };
    let pubmed_fut = async {
        if needs_pubmed {
            let started = Instant::now();
            let abstracts =
                fetch_pubmed_abstracts(db_path, rsid, config.ncbi_api_key.as_deref()).await;
            record_phase(SweepPhase::Pubmed, started.elapsed());
            abstracts
        } else {
            Vec::new()
        }
    };
    let gtex_fut = async {
        if needs_gtex {
            let started = Instant::now();
            let eqtls =
                fetch_gtex_eqtls_for_rsid(db_path, rsid, prefetch_gene.as_deref()).await;
            record_phase(SweepPhase::Gtex, started.elapsed());
            eqtls
        } else {
            None
        }
    };

    let (gnomad_result, abstracts, eqtls) = tokio::join!(gnomad_fut, pubmed_fut, gtex_fut);
    let (gnomad_af, gnomad_ctx) = gnomad_result;

    if let Some(obj) = sources_provenance.as_object_mut() {
        let skip_gnomad = skip_gnomad_in_sweep();
        obj.insert(
            "gnomad".into(),
            serde_json::json!({
                "queried": !skip_gnomad,
                "hit": gnomad_af.is_some(),
                "af": gnomad_af,
                "skipped": skip_gnomad,
                "lookup_status": gnomad_ctx.lookup_status,
                "source_mode": gnomad_ctx.source_mode,
                "release": gnomad_ctx.release,
            }),
        );
        obj.insert(
            "clinvar_local".into(),
            serde_json::json!({
                "queried": true,
                "hit": effective_clinvar.map(|s| !s.is_empty()).unwrap_or(false),
                "significance": effective_clinvar,
            }),
        );
        obj.insert(
            "pubmed".into(),
            serde_json::json!({
                "queried": needs_pubmed,
                "hits": abstracts.len(),
            }),
        );
        obj.insert(
            "gtex".into(),
            serde_json::json!({
                "queried": needs_gtex,
                "hits": eqtls.as_ref().map(|v| v.len()).unwrap_or(0),
            }),
        );
    }

    let gwas_strict_count = gwas_assocs
        .iter()
        .filter(|a| a["pvalue"].as_f64().map(|p| p < 5e-8).unwrap_or(false))
        .count() as u32;

    let has_pathogenic_clinvar = effective_clinvar
        .map(|s| {
            s.contains("Pathogenic")
                || s.contains("Likely pathogenic")
                || s.contains("Risk factor")
                || s.contains("Association")
        })
        .unwrap_or(false);

    let evidence_tier = if has_pathogenic_clinvar {
        "clinical"
    } else if gwas_strict_count > 0 || !gwas_assocs.is_empty() {
        "gwas_trait"
    } else if gnomad_af.map(|af| af >= 0.05).unwrap_or(false) {
        "population_common"
    } else {
        "sparse_annotation"
    };

    let (mut resolved_gene, gene_confidence_static) =
        resolve_gene_name(db_path, rsid, gene, &gwas_assocs);
    let mut gene_confidence = gene_confidence_static.to_string();
    let data_dir = db_path.parent();
    if resolved_gene.is_none()
        && let Some(catalog_gene) = lookup_discovery_catalog_gene(data_dir, rsid) {
            resolved_gene = Some(catalog_gene);
            gene_confidence = "discovery_catalog".to_string();
        }
    let locus = lookup_variant_locus(db_path, sample_id, rsid);
    let mut chromosome = locus.as_ref().map(|(c, _)| c.clone());
    let mut position = locus.map(|(_, p)| p);

    if super::sources_config::enrichment_vep_dbsnp_enabled()
        && (resolved_gene.is_none() || gene_confidence == "unknown" || gene_confidence == "gwas_reported")
    {
        let vep_started = Instant::now();
        if let Some(ensembl) = fetch_ensembl_vep_genes(db_path, rsid).await {
            record_phase(SweepPhase::Vep, vep_started.elapsed());
            if let Some(obj) = sources_provenance.as_object_mut() {
                obj.insert(
                    "ensembl_vep".into(),
                    serde_json::json!({
                        "queried": true,
                        "hits": ensembl.genes.len(),
                        "genes": ensembl.genes,
                        "source": ensembl.source,
                    }),
                );
            }
            if resolved_gene.is_none() {
                resolved_gene = ensembl.genes.first().cloned();
                if resolved_gene.is_some() {
                    gene_confidence = "ensembl_vep".to_string();
                }
            } else if (gene_confidence == "unknown" || gene_confidence == "gwas_reported")
                && let Some(symbol) = ensembl.genes.first() {
                    resolved_gene = Some(symbol.clone());
                    gene_confidence = "ensembl_vep".to_string();
                }
            if chromosome.is_none() {
                chromosome = ensembl.chromosome.clone();
            }
            if position.is_none() {
                position = ensembl.position;
            }
        } else if let Some(obj) = sources_provenance.as_object_mut() {
            record_phase(SweepPhase::Vep, vep_started.elapsed());
            obj.insert(
                "ensembl_vep".into(),
                serde_json::json!({ "queried": true, "hits": 0 }),
            );
        }
    }

    if super::sources_config::enrichment_vep_dbsnp_enabled() && (chromosome.is_none() || position.is_none()) {
        let vep_started = Instant::now();
        let dbsnp = super::evidence::ncbi_context::fetch_dbsnp_context(db_path, rsid).await;
        record_phase(SweepPhase::Vep, vep_started.elapsed());
        if let Some(obj) = sources_provenance.as_object_mut() {
            obj.insert("dbsnp".into(), dbsnp.provenance);
        }
        if chromosome.is_none() {
            chromosome = dbsnp.chromosome;
        }
        if position.is_none() {
            position = dbsnp.position;
        }
        if let Some(line) = dbsnp.narrative {
            ncbi_narrative_lines.push(line);
        }
    }

    let mut gene_candidates = collect_gene_candidates(db_path, rsid, gene, &gwas_assocs);
    if let Some(obj) = sources_provenance.as_object_mut()
        && let Some(ensembl) = obj.get("ensembl_vep").and_then(|v| v["genes"].as_array()) {
            for gene_val in ensembl {
                if let Some(symbol) = gene_val.as_str()
                    && !gene_candidates.iter().any(|c| {
                        c["symbol"]
                            .as_str()
                            .map(|s| s.eq_ignore_ascii_case(symbol))
                            .unwrap_or(false)
                    }) {
                        gene_candidates.push(serde_json::json!({
                            "symbol": symbol,
                            "confidence": "ensembl_vep",
                            "source": "ensembl_vep",
                        }));
                    }
            }
        }
    if resolved_gene.is_none()
        && let Some(candidate) = gene_candidates.first().and_then(|c| c["symbol"].as_str()) {
            resolved_gene = Some(candidate.to_string());
            gene_confidence = gene_candidates[0]["confidence"]
                .as_str()
                .unwrap_or("candidate")
                .to_string();
        }
    let mut eqtls = eqtls;
    if super::sources_config::enrichment_gtex_enabled() && eqtls.is_none()
        && let Some(ref resolved) = resolved_gene
            && prefetch_gene.is_none() {
                let gtex_started = Instant::now();
                eqtls = fetch_gtex_eqtls_for_rsid(db_path, rsid, Some(resolved.as_str())).await;
                record_phase(SweepPhase::Gtex, gtex_started.elapsed());
                if let Some(obj) = sources_provenance.as_object_mut() {
                    obj.insert(
                        "gtex".into(),
                        serde_json::json!({
                            "queried": true,
                            "hits": eqtls.as_ref().map(|v| v.len()).unwrap_or(0),
                            "resolved_after_gwas_gene_mapping": true,
                        }),
                    );
                }
            }
    let genotype = format!("{}/{}", allele1, allele2);
    let gwas_hit_count = gwas_assocs.len() as u32;
    let traits = extract_gwas_traits(&gwas_assocs);

    let facts_fresh = crate::db::connect(db_path)
        .map(|c| super::evidence::store::facts_fresh_for_rsid(&c, sample_id, rsid))
        .unwrap_or(false);

    let secondary = if !super::sources_config::enrichment_secondary_enabled() {
        super::evidence::adapters::SecondarySourceBundle {
            provenance: serde_json::json!({ "sweep_fast": true, "skipped_live_fetch": true }),
            ..Default::default()
        }
    } else if facts_fresh {
        super::evidence::adapters::SecondarySourceBundle {
            provenance: serde_json::json!({ "cached_facts": true, "skipped_live_fetch": true }),
            ..Default::default()
        }
    } else {
        let secondary_deadline =
            Duration::from_secs(super::tuning::secondary_sources_timeout_secs());
        match tokio::time::timeout(
            secondary_deadline,
            super::evidence::adapters::fetch_secondary_sources(
                db_path,
                rsid,
                resolved_gene.as_deref(),
                allele1,
                allele2,
                &traits,
                sample_id,
            ),
        )
        .await
        {
            Ok(bundle) => bundle,
            Err(_) => {
                eprintln!(
                    "[enrich] secondary sources timed out after {}s for {}",
                    secondary_deadline.as_secs(),
                    rsid
                );
                super::evidence::adapters::SecondarySourceBundle {
                    provenance: serde_json::json!({
                        "timeout_secs": secondary_deadline.as_secs(),
                        "skipped_live_fetch": true,
                    }),
                    ..Default::default()
                }
            }
        }
    };

    if let Some(obj) = sources_provenance.as_object_mut()
        && let Some(sec) = secondary.provenance.as_object() {
            for (k, v) in sec {
                obj.insert(k.clone(), v.clone());
            }
        }

    let crossmap = build_cross_map_context(
        data_dir,
        rsid,
        resolved_gene.as_deref(),
        &traits,
        &gwas_assocs,
    );
    if resolved_gene.is_none()
        && let Some(catalog_gene) = crossmap
            .discovery_catalog_match
            .as_ref()
            .and_then(|m| m["gene"].as_str())
            .filter(|g| !g.trim().is_empty())
        {
            resolved_gene = Some(catalog_gene.to_string());
            gene_confidence = "discovery_catalog".to_string();
        }

    let mut full_text = build_enrichment_narrative(
        rsid,
        &genotype,
        resolved_gene.as_deref(),
        gene_confidence.as_str(),
        &traits,
        &gwas_assocs,
        effective_clinvar,
        gnomad_af,
        &crossmap.trait_categories,
        chromosome.as_deref(),
        position,
    );
    if !crossmap.searchable_tags.is_empty() {
        full_text.push_str(&format!(
            "\nSearch tags: {}.",
            crossmap.searchable_tags.join(", ")
        ));
    }

    if let Some(eqs) = eqtls {
        let eqtl_summaries: Vec<String> = eqs
            .iter()
            .take(3)
            .map(|e| {
                let tissue = e["tissueSiteDetailId"].as_str().unwrap_or("Unknown");
                let nes = e["nes"].as_f64().unwrap_or(0.0);
                let p = e["pValue"].as_f64().unwrap_or(0.0);
                format!("{} (NES: {:.3}, p: {:.2e})", tissue, nes, p)
            })
            .collect();
        if !eqtl_summaries.is_empty() {
            full_text.push_str(&format!(
                "\nVariant eQTL Associations (GTEx): {}",
                eqtl_summaries.join("; ")
            ));
        }
    }

    for abstract_text in &abstracts {
        full_text.push('\n');
        full_text.push_str(abstract_text);
    }

    for line in &secondary.narrative_lines {
        full_text.push('\n');
        full_text.push_str(line);
    }
    for line in &ncbi_narrative_lines {
        full_text.push('\n');
        full_text.push_str(line);
    }

    let best_p = best_gwas_pvalue(&gwas_assocs);
    let score = score_marker_significance(effective_clinvar, gwas_strict_count, gnomad_af, best_p);
    let gwas_trait_str = traits.join("; ");
    let point_id = format!("{}_{}_{}", sample_id, rsid, allele1);
    let payload = build_enrichment_payload(
        sample_id,
        rsid,
        &resolved_gene,
        gene_confidence.as_str(),
        evidence_tier,
        &genotype,
        &full_text,
        &gwas_trait_str,
        gwas_hit_count,
        effective_clinvar,
        gnomad_af,
        score,
        &gwas_assocs,
        &gene_candidates,
        &sources_provenance,
        &crossmap,
        chromosome.as_deref(),
        position,
    );

    let mut payload_map = payload.as_object().cloned().unwrap_or_default();

    if let Some(mapping) = super::evidence::adapters::best_ontology_mapping(&secondary.ontology_mappings) {
        payload_map.insert("trait_name_mapped".into(), serde_json::json!(mapping.mapped_label));
        payload_map.insert("trait_ontology_id".into(), serde_json::json!(mapping.ontology_id));
        payload_map.insert(
            "trait_mapping_confidence".into(),
            serde_json::json!(mapping.confidence),
        );
    }
    if !secondary.pathway_names.is_empty() {
        payload_map.insert("pathway_names".into(), serde_json::json!(secondary.pathway_names));
    }
    if !secondary.pgs_matches.is_empty() {
        payload_map.insert("pgs_match_summaries".into(), serde_json::json!(secondary.pgs_matches));
    }
    super::evidence::named_vectors::extend_payload_named_vector_meta(
        &mut payload_map,
        super::evidence::named_vectors::named_vectors_enabled(config),
    );
    super::gnomad::apply_gnomad_to_payload(&mut payload_map, &gnomad_ctx);

    super::evidence::normalize::extend_payload_normalized(
        &mut payload_map,
        sample_id,
        rsid,
        &genotype,
        &resolved_gene,
        gene_confidence.as_str(),
        evidence_tier,
        effective_clinvar,
        gnomad_af,
        &gwas_assocs,
        &crossmap,
        chromosome.as_deref(),
        position,
        &sources_provenance,
        &config.embedding_model,
        &full_text,
    );
    let payload = serde_json::Value::Object(payload_map);

    let dq = payload["data_quality_score"].as_f64().unwrap_or(0.5) as f32;
    let assoc_strength = payload["association_strength_score"].as_f64().unwrap_or(0.0) as f32;
    let primary_trait = payload["trait_name"].as_str().map(String::from);
    let trait_cat = payload["trait_category"].as_str().map(String::from);
    let _ = super::evidence::store::persist_enrichment_evidence(
        db_path,
        sample_id,
        rsid,
        &genotype,
        resolved_gene.as_deref(),
        trait_cat.as_deref(),
        primary_trait.as_deref(),
        &gwas_assocs,
        effective_clinvar,
        dq,
        assoc_strength,
        &sources_provenance,
        &secondary,
    );

    Ok(PreparedEnrichment {
        rsid: rsid.to_string(),
        point_id,
        full_text,
        payload,
        named_vectors: None,
    })
}

pub async fn enrich_marker(
    sample_id: i64,
    rsid: &str,
    gene: Option<&str>,
    allele1: &str,
    allele2: &str,
    clinvar_sig: Option<&str>,
    db_path: &Path,
    ollama_url: &str,
    config: &QdrantConfig,
) -> Result<(), String> {
    let prepared = prepare_marker_enrichment(
        sample_id,
        rsid,
        gene,
        allele1,
        allele2,
        clinvar_sig,
        db_path,
        config,
    )
    .await?;
    let vector = embed_text(&prepared.full_text, ollama_url, &config.embedding_model).await?;

    if super::evidence::named_vectors::named_vectors_enabled(config) {
        let texts = super::evidence::named_vectors::build_named_vector_texts(
            prepared.payload.as_object().unwrap_or(&serde_json::Map::new()),
            &prepared.full_text,
        );
        if let Ok(named) =
            super::evidence::named_vectors::embed_named_vectors(&texts, ollama_url, &config.embedding_model)
                .await
        {
            let mut vectors = named;
            vectors.insert(String::new(), vector.clone());
            super::qdrant::upsert_points_batch_named(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                vec![(
                    super::util::string_to_u64(&prepared.point_id),
                    vectors,
                    prepared.payload.clone(),
                )],
            )
            .await?;
        } else {
            upsert_to_qdrant(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                &prepared.point_id,
                vector,
                prepared.payload.clone(),
            )
            .await?;
        }
    } else {
        upsert_to_qdrant(
            &config.url,
            config.api_key.as_deref(),
            &config.collection,
            &prepared.point_id,
            vector,
            prepared.payload.clone(),
        )
        .await?;
    }
    let _ = maybe_promote_vector_finding(db_path, sample_id, &prepared);
    Ok(())
}

pub(crate) fn is_fatal_enrichment_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    // Per-variant prepare pressure — skip variant and continue the sweep.
    if lower.contains("prepare slot") || lower.contains("prepare timed out") {
        return false;
    }
    if lower.contains("batch enrichment timed out") || lower.contains("batch timed out") {
        return false;
    }
    lower.contains("401 unauthorized")
        || lower.contains("connection refused")
        || lower.contains("connect error")
        || lower.contains("connection failed")
        || lower.contains("dns error")
        || lower.contains("ollama returned")
        || lower.contains("might not be installed")
}

pub(crate) async fn process_enrichment_batch(
    sample_id: i64,
    batch: &[&ScoredMarker],
    db_path: &Path,
    ollama_url: &str,
    config: &QdrantConfig,
) -> Vec<Result<PreparedEnrichment, String>> {
    let deadline = Duration::from_secs(enrichment_batch_timeout_secs(batch.len()));
    match tokio::time::timeout(
        deadline,
        process_enrichment_batch_inner(sample_id, batch, db_path, ollama_url, config),
    )
    .await
    {
        Ok(results) => results,
        Err(_) => {
            super::sweep_metrics::clear_batch_activity();
            eprintln!(
                "[enrich] batch timed out after {}s ({} variants)",
                deadline.as_secs(),
                batch.len()
            );
            batch
                .iter()
                .map(|m| {
                    Err(format!(
                        "Batch enrichment timed out after {}s at {}",
                        deadline.as_secs(),
                        m.rsid
                    ))
                })
                .collect()
        }
    }
}

async fn process_enrichment_batch_inner(
    sample_id: i64,
    batch: &[&ScoredMarker],
    db_path: &Path,
    _ollama_url: &str,
    config: &QdrantConfig,
) -> Vec<Result<PreparedEnrichment, String>> {
    if let Some(first) = batch.first() {
        super::sweep_metrics::set_batch_activity(
            if super::tuning::skip_gnomad_in_sweep() {
                "API prefetch"
            } else {
                "gnomAD prefetch"
            },
            &first.rsid,
            batch.len() as u32,
        );
    }
    if !super::tuning::skip_gnomad_in_sweep() {
        let rsids: Vec<String> = batch.iter().map(|m| m.rsid.clone()).collect();
        let data_dir = db_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| db_path.to_path_buf());
        let prefetch_timeout = Duration::from_secs(
            (gnomad_remote_timeout_secs() * rsids.len() as u64).clamp(120, 600),
        );
        let _ = tokio::time::timeout(
            prefetch_timeout,
            timed_async(
                SweepPhase::Gnomad,
                super::gnomad::prefetch_gnomad_batch(db_path, &data_dir, sample_id, &rsids),
            ),
        )
        .await;
    }

    super::sweep_metrics::set_batch_phase("API prefetch");
    super::debug_log::log(
        "enrich",
        format!("API prefetch starting ({} variants)", batch.len()),
    );
    super::prefetch::prefetch_enrichment_batch(db_path, sample_id, batch, config).await;

    super::sweep_metrics::set_batch_phase("prepare");
    super::debug_log::log(
        "enrich",
        format!("prepare starting ({} variants, concurrency={})", batch.len(), prepare_concurrency()),
    );

    let concurrency = prepare_concurrency().max(1);
    let mut indexed: Vec<(usize, Result<PreparedEnrichment, String>)> =
        Vec::with_capacity(batch.len());

    for wave_start in (0..batch.len()).step_by(concurrency) {
        let wave_end = (wave_start + concurrency).min(batch.len());
        let mut join_set = JoinSet::new();

        for (offset, marker) in batch[wave_start..wave_end].iter().enumerate() {
            let idx = wave_start + offset;
            let marker = (*marker).clone();
            let db_path = db_path.to_path_buf();
            let config = config.clone();
            join_set.spawn(async move {
                let rsid = marker.rsid.clone();
                let deadline = Duration::from_secs(prepare_variant_timeout_secs());
                let result = match tokio::time::timeout(
                    deadline,
                    prepare_marker_enrichment(
                        sample_id,
                        &marker.rsid,
                        marker.gene.as_deref(),
                        &marker.allele1,
                        &marker.allele2,
                        marker.clinvar_sig.as_deref(),
                        &db_path,
                        &config,
                    ),
                )
                .await
                {
                    Ok(r) => r,
                    Err(_) => Err(format!(
                        "Prepare timed out after {}s for {}",
                        deadline.as_secs(),
                        rsid
                    )),
                };
                (idx, result)
            });
        }

        while let Some(joined) = join_set.join_next().await {
            match joined {
                Ok((idx, result)) => {
                    if idx < batch.len() {
                        super::sweep_metrics::set_batch_current_rsid(&batch[idx].rsid);
                    }
                    super::sweep_metrics::bump_batch_prepared();
                    indexed.push((idx, result));
                }
                Err(e) => indexed.push((batch.len(), Err(format!("Task join error: {}", e)))),
            }
        }
    }

    super::sweep_metrics::clear_batch_activity();

    indexed.sort_by_key(|(idx, _)| *idx);
    indexed.into_iter().map(|(_, result)| result).collect()
}
