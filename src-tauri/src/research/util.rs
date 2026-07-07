// ./src-tauri/src/research/util.rs
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::hash::{Hash, DefaultHasher, Hasher};
use std::path::Path;
pub(crate) fn string_to_u64(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

// ---------------------------------------------------------------------------
// Helper: current unix timestamp (seconds)
// ---------------------------------------------------------------------------

pub(crate) fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Normalize rsIDs to lowercase `rs123` form for consistent SQLite joins.
pub(crate) fn normalize_rsid(raw: &str) -> Option<String> {
    let token = raw.trim().split('-').next()?.trim();
    if token.len() < 3 {
        return None;
    }
    let upper = token.to_uppercase();
    if upper.starts_with("RS") && upper.len() > 2 {
        Some(format!("rs{}", &upper[2..]))
    } else if upper.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("rs{}", upper))
    } else {
        None
    }
}

pub(crate) fn normalize_gwas_reference_rsids(conn: &Connection) -> Result<usize, String> {
    let updated = conn
        .execute(
            "UPDATE reference.gwas_reference
             SET rsid = 'rs' || SUBSTR(UPPER(rsid), 3)
             WHERE UPPER(rsid) LIKE 'RS%'",
            [],
        )
        .map_err(|e| e.to_string())?;
    Ok(updated)
}

pub(crate) fn is_placeholder_gene(name: &str) -> bool {
    let n = name.trim();
    n.is_empty()
        || n.eq_ignore_ascii_case("unknown")
        || n.starts_with("LOC")
        || n.starts_with("LINC")
        || n.starts_with("RNU")
}

pub(crate) fn dedupe_preserve_order(items: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|s| {
            let key = s.trim().to_lowercase();
            !key.is_empty() && seen.insert(key)
        })
        .collect()
}

pub(crate) fn parse_gene_tokens(raw: &str) -> Vec<String> {
    raw.split([';', ',', '|', '/'])
        .flat_map(|part| part.split(" - "))
        .map(str::trim)
        .filter(|g| !is_placeholder_gene(g))
        .map(|g| g.to_string())
        .collect::<Vec<_>>()
        .into_iter()
        .fold(Vec::new(), |mut acc, gene| {
            if !acc.iter().any(|g| g.eq_ignore_ascii_case(&gene)) {
                acc.push(gene);
            }
            acc
        })
}

pub(crate) fn lookup_gene_from_db(db_path: &Path, rsid: &str) -> Option<String> {
    let rsid_key = normalize_rsid(rsid)?.to_uppercase();
    crate::db::with_cached_conn(db_path, |conn| {
        let from_evidence: Option<String> = conn
            .query_row(
                "SELECT gene FROM evidence_library WHERE UPPER(rsid) = ? AND gene IS NOT NULL AND TRIM(gene) != '' LIMIT 1",
                params![rsid_key],
                |row| row.get(0),
            )
            .ok();
        if let Some(ref g) = from_evidence
            && !is_placeholder_gene(g) {
                return Ok(Some(g.clone()));
            }
        let from_clinvar: Option<String> = conn.query_row(
            "SELECT gene FROM clinvar_reference WHERE UPPER(rsid) = ? AND gene IS NOT NULL AND TRIM(gene) != '' LIMIT 1",
            params![rsid_key],
            |row| row.get(0),
        )
        .ok()
        .filter(|g: &String| !is_placeholder_gene(g));
        Ok(from_clinvar)
    })
    .ok()
    .flatten()
}

pub(crate) fn lookup_gene_from_gwas_reference(
    db_path: &Path,
    rsid: &str,
) -> Option<(String, &'static str)> {
    let rsid_norm = normalize_rsid(rsid)?;
    crate::db::with_cached_conn(db_path, |conn| {
        let row: Result<(String, String, String), _> = conn.query_row(
            "SELECT primary_gene, mapped_genes, reported_genes FROM gwas_reference WHERE rsid = ?",
            params![rsid_norm],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        );
        let (primary, mapped, reported) = match row {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        if !is_placeholder_gene(&primary) {
            return Ok(Some((primary.trim().to_string(), "gwas_catalog")));
        }
        if let Some(gene) = parse_gene_tokens(&mapped).into_iter().next() {
            return Ok(Some((gene, "gwas_catalog")));
        }
        if let Some(gene) = parse_gene_tokens(&reported).into_iter().next() {
            return Ok(Some((gene, "gwas_reported")));
        }
        Ok(None)
    })
    .ok()
    .flatten()
}

pub(crate) fn collect_gene_candidates(
    db_path: &Path,
    rsid: &str,
    local_gene: Option<&str>,
    gwas_assocs: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let mut out: Vec<serde_json::Value> = Vec::new();
    let mut push = |symbol: &str, confidence: &str, source: &str| {
        if is_placeholder_gene(symbol) {
            return;
        }
        if out.iter().any(|v| {
            v["symbol"]
                .as_str()
                .map(|s| s.eq_ignore_ascii_case(symbol))
                .unwrap_or(false)
        }) {
            return;
        }
        out.push(serde_json::json!({
            "symbol": symbol,
            "confidence": confidence,
            "source": source,
        }));
    };

    if let Some(g) = local_gene {
        push(g, "curated", "marker_pack");
    }
    if let Some(g) = lookup_gene_from_db(db_path, rsid) {
        push(&g, "evidence_db", "evidence_library|clinvar_reference");
    }
    if let Some((g, confidence)) = lookup_gene_from_gwas_reference(db_path, rsid) {
        push(&g, confidence, "gwas_reference");
    }
    if let Some(g) = extract_gene_from_gwas(gwas_assocs) {
        push(&g, "gwas_mapped", "gwas_associations");
    }
    out
}

pub(crate) fn consider_gene_candidate(best: &mut Option<(i32, String)>, name: &str, priority: i32) {
    if is_placeholder_gene(name) {
        return;
    }
    let name = name.trim().to_string();
    if best.as_ref().map(|(p, _)| *p).unwrap_or(-1) < priority {
        *best = Some((priority, name));
    }
}

pub(crate) fn extract_gene_from_gwas(assocs: &[serde_json::Value]) -> Option<String> {
    let mut best: Option<(i32, String)> = None;
    for assoc in assocs {
        if let Some(mapped) = assoc["mapped_gene"].as_str() {
            consider_gene_candidate(&mut best, mapped, 9);
        }
        if let Some(reported) = assoc["reported_genes"].as_array() {
            for gene in reported {
                if let Some(name) = gene.as_str() {
                    consider_gene_candidate(&mut best, name, 7);
                }
            }
        }
        if let Some(loci) = assoc["loci"].as_array() {
            for locus in loci {
                if let Some(reported) = locus["authorReportedGenes"].as_array() {
                    for gene in reported {
                        if let Some(name) = gene["geneName"].as_str() {
                            consider_gene_candidate(&mut best, name, 10);
                        }
                    }
                }
            }
        }
        if let Some(contexts) = assoc["genomicContexts"].as_array() {
            for ctx in contexts {
                if let Some(name) = ctx["gene"]["geneName"].as_str() {
                    let priority = if ctx["isClosestGene"].as_bool() == Some(true) {
                        2
                    } else {
                        4
                    };
                    consider_gene_candidate(&mut best, name, priority);
                }
            }
        }
    }
    best.map(|(_, name)| name)
}

pub(crate) fn extract_gwas_traits(assocs: &[serde_json::Value]) -> Vec<String> {
    let raw: Vec<String> = assocs
        .iter()
        .filter_map(|a| {
            a["trait_name"]
                .as_str()
                .map(String::from)
                .or_else(|| {
                    a["trait"]["trait"]
                        .as_str()
                        .map(String::from)
                })
                .or_else(|| {
                    a["efoTraits"].as_array().and_then(|arr| {
                        arr.first()
                            .and_then(|t| t["trait"].as_str().map(String::from))
                    })
                })
        })
        .collect();
    dedupe_preserve_order(raw).into_iter().take(8).collect()
}

pub(crate) fn canonical_gwas_association(
    trait_name: &str,
    pvalue: Option<f64>,
    reported_genes: &[String],
    mapped_gene: Option<&str>,
    study_accession: Option<&str>,
    source: &str,
) -> serde_json::Value {
    serde_json::json!({
        "trait_name": trait_name,
        "trait": { "trait": trait_name },
        "pvalue": pvalue,
        "reported_genes": reported_genes,
        "mapped_gene": mapped_gene.filter(|g| !is_placeholder_gene(g)),
        "study_accession": study_accession.filter(|s| !s.is_empty()),
        "source": source,
    })
}

pub(crate) fn normalize_api_gwas_association(assoc: &serde_json::Value) -> serde_json::Value {
    let trait_name = assoc["trait"]["trait"]
        .as_str()
        .or_else(|| {
            assoc["efoTraits"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|t| t["trait"].as_str())
        })
        .unwrap_or("")
        .to_string();
    let pvalue = assoc["pvalue"].as_f64();
    let mapped_gene = extract_gene_from_gwas(std::slice::from_ref(assoc));
    let reported_genes = assoc["loci"]
        .as_array()
        .map(|loci| {
            loci.iter()
                .flat_map(|locus| {
                    locus["authorReportedGenes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|gene| gene["geneName"].as_str().map(String::from))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let study_accession = assoc["studyAccession"].as_str().map(String::from);
    let beta = assoc["beta"].as_f64();
    let odds_ratio = assoc["orPerCopyNum"]
        .as_f64()
        .or_else(|| assoc["oddsRatio"].as_f64());
    let effect_allele = assoc["riskAllele"]
        .as_str()
        .or_else(|| {
            assoc["loci"].as_array().and_then(|loci| {
                loci.iter().find_map(|locus| {
                    locus["strongestRiskAlleles"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|ra| ra["riskAlleleName"].as_str())
                })
            })
        })
        .map(String::from);
    let pubmed_id = assoc["pubmedId"]
        .as_str()
        .or_else(|| assoc["study"]["pubmedId"].as_str())
        .map(String::from);
    let ancestry = assoc["ancestryInitial"]
        .as_str()
        .or_else(|| assoc["ancestry"].as_str())
        .map(String::from);
    let sample_size = assoc["initialSampleSize"]
        .as_i64()
        .or_else(|| assoc["sampleSize"].as_i64());

    let mut obj = canonical_gwas_association(
        &trait_name,
        pvalue,
        &reported_genes,
        mapped_gene.as_deref(),
        study_accession.as_deref(),
        "gwas_api",
    );
    if let Some(map) = obj.as_object_mut() {
        if let Some(b) = beta {
            map.insert("beta".into(), serde_json::json!(b));
        }
        if let Some(or) = odds_ratio {
            map.insert("odds_ratio".into(), serde_json::json!(or));
        }
        if let Some(ea) = effect_allele {
            map.insert("effect_allele".into(), serde_json::json!(ea));
        }
        if let Some(pm) = pubmed_id {
            map.insert("pubmed_id".into(), serde_json::json!(pm));
        }
        if let Some(a) = ancestry {
            map.insert("ancestry".into(), serde_json::json!(a));
        }
        if let Some(n) = sample_size {
            map.insert("sample_size".into(), serde_json::json!(n));
        }
    }
    obj
}

pub(crate) fn clean_pubmed_abstract(text: &str) -> String {
    let mut cleaned = text.trim().to_string();
    for marker in [
        "Conflict of interest statement:",
        "Conflict of interest:",
        "COI statement:",
        "[Indexed for MEDLINE]",
        "Copyright",
        "©",
    ] {
        if let Some(idx) = cleaned.find(marker) {
            cleaned.truncate(idx);
        }
    }
    cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.len() > 400 {
        cleaned.truncate(400);
        cleaned.push('…');
    }
    cleaned
}

pub(crate) fn resolve_gene_name(
    db_path: &Path,
    rsid: &str,
    local_gene: Option<&str>,
    gwas_assocs: &[serde_json::Value],
) -> (Option<String>, &'static str) {
    if let Some(g) = local_gene
        && !is_placeholder_gene(g) {
            return (Some(g.trim().to_string()), "curated");
        }
    if let Some(g) = lookup_gene_from_db(db_path, rsid) {
        return (Some(g), "evidence_db");
    }
    if let Some((g, confidence)) = lookup_gene_from_gwas_reference(db_path, rsid) {
        return (Some(g), confidence);
    }
    if let Some(g) = extract_gene_from_gwas(gwas_assocs) {
        return (Some(g), "gwas_mapped");
    }
    (None, "unknown")
}

pub(crate) fn lookup_variant_locus(
    db_path: &Path,
    sample_id: i64,
    rsid: &str,
) -> Option<(String, i64)> {
    let rsid_key = normalize_rsid(rsid)?.to_lowercase();
    crate::db::with_cached_conn(db_path, |conn| {
        conn.query_row(
            "SELECT chromosome, position_grch38 FROM genotypes
             WHERE sample_id = ? AND LOWER(rsid) = ? LIMIT 1",
            params![sample_id, rsid_key],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
    })
    .ok()
}

pub(crate) fn best_gwas_pvalue(assocs: &[serde_json::Value]) -> Option<f64> {
    assocs
        .iter()
        .filter_map(|a| a["pvalue"].as_f64())
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_enrichment_narrative(
    rsid: &str,
    genotype: &str,
    gene: Option<&str>,
    gene_confidence: &str,
    traits: &[String],
    gwas_assocs: &[serde_json::Value],
    clinvar_sig: Option<&str>,
    gnomad_af: Option<f64>,
    trait_categories: &[String],
    chromosome: Option<&str>,
    position: Option<i64>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    let best_p = best_gwas_pvalue(gwas_assocs);
    let primary_trait = traits.first().map(String::as_str).unwrap_or("");

    if !primary_trait.is_empty() {
        let p_str = best_p
            .map(|p| format!(" (GWAS p={:.2e})", p))
            .unwrap_or_default();
        parts.push(format!(
            "Primary association: {}{}. Variant {} genotype {}.",
            primary_trait, p_str, rsid, genotype
        ));
    } else {
        parts.push(format!("Variant {} genotype {}.", rsid, genotype));
    }

    if let Some(g) = gene.filter(|s| !s.is_empty()) {
        parts.push(format!(
            "Gene: {} (confidence: {}).",
            g, gene_confidence
        ));
    } else if !primary_trait.is_empty() {
        parts.push(
            "Gene: not mapped in GWAS catalog for this locus; trait association is still valid at the population level."
                .to_string(),
        );
    }

    if !trait_categories.is_empty() {
        parts.push(format!(
            "Clinical domains: {}.",
            trait_categories.join(", ")
        ));
    }

    if let (Some(chr), Some(pos)) = (chromosome, position) {
        parts.push(format!("Locus: chr{}:{}", chr, pos));
    }

    if !traits.is_empty() && traits.len() > 1 {
        parts.push(format!("Additional GWAS traits: {}.", traits[1..].join("; ")));
    }

    for assoc in gwas_assocs.iter().take(4) {
        let trait_name = assoc["trait_name"]
            .as_str()
            .or_else(|| assoc["trait"]["trait"].as_str())
            .unwrap_or("");
        let mapped = assoc["mapped_gene"].as_str().unwrap_or("");
        let reported = assoc["reported_genes"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let pvalue = assoc["pvalue"].as_f64();
        if !trait_name.is_empty() {
            parts.push(format!(
                "GWAS detail: trait={}; mapped_gene={}; reported_genes={}; p={}",
                trait_name,
                if mapped.is_empty() { "n/a" } else { mapped },
                if reported.is_empty() { "n/a" } else { &reported },
                pvalue
                    .map(|p| format!("{:.2e}", p))
                    .unwrap_or_else(|| "n/a".to_string()),
            ));
        }
    }

    if let Some(sig) = clinvar_sig.filter(|s| !s.is_empty()) {
        parts.push(format!("ClinVar: {}.", sig));
    }

    if let Some(af) = gnomad_af {
        parts.push(format!("Population frequency (gnomAD): {:.6}.", af));
    }

    parts.join("\n")
}

pub const ENRICHMENT_VERSION: &str = "4.1";

/// True when a stored Qdrant payload is on the current enrichment pipeline.
pub(crate) fn is_current_enrichment_version(version: Option<&str>) -> bool {
    match version.map(str::trim).filter(|s| !s.is_empty()) {
        None => false,
        Some(v) => v == ENRICHMENT_VERSION || v.starts_with("4.1"),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_enrichment_payload(
    sample_id: i64,
    rsid: &str,
    gene: &Option<String>,
    gene_confidence: &str,
    evidence_tier: &str,
    genotype: &str,
    full_text: &str,
    gwas_traits: &str,
    gwas_hit_count: u32,
    clinvar_sig: Option<&str>,
    gnomad_af: Option<f64>,
    significance_score: f32,
    gwas_associations: &[serde_json::Value],
    gene_candidates: &[serde_json::Value],
    sources_provenance: &serde_json::Value,
    crossmap: &super::crossmap::CrossMapContext,
    chromosome: Option<&str>,
    position: Option<i64>,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert("sample_id".into(), serde_json::json!(sample_id));
    payload.insert("rsid".into(), serde_json::json!(rsid));
    payload.insert(
        "gene_symbol".into(),
        serde_json::json!(gene.as_deref().unwrap_or("")),
    );
    payload.insert(
        "gene".into(),
        serde_json::json!(gene.as_deref().unwrap_or("")),
    );
    payload.insert("gene_confidence".into(), serde_json::json!(gene_confidence));
    if !gene_candidates.is_empty() {
        payload.insert("gene_candidates".into(), serde_json::json!(gene_candidates));
        let symbols: Vec<String> = gene_candidates
            .iter()
            .filter_map(|c| c["symbol"].as_str().map(String::from))
            .collect();
        if !symbols.is_empty() {
            payload.insert("gene_symbols".into(), serde_json::json!(symbols));
        }
    }
    payload.insert("genotype".into(), serde_json::json!(genotype));
    payload.insert("evidence_tier".into(), serde_json::json!(evidence_tier));
    payload.insert("indexed_by".into(), serde_json::json!("vector_research"));
    payload.insert("enrichment_version".into(), serde_json::json!(ENRICHMENT_VERSION));
    payload.insert("sources_provenance".into(), sources_provenance.clone());
    payload.insert(
        "sources_used".into(),
        serde_json::json!(
            sources_provenance
                .as_object()
                .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        ),
    );
    payload.insert("text".into(), serde_json::json!(full_text));
    payload.insert("has_gwas".into(), serde_json::json!(gwas_hit_count > 0));
    payload.insert("gwas_hit_count".into(), serde_json::json!(gwas_hit_count));
    if !gwas_associations.is_empty() {
        payload.insert(
            "gwas_associations".into(),
            serde_json::json!(gwas_associations),
        );
    }
    if !gwas_traits.is_empty() {
        payload.insert("gwas_traits".into(), serde_json::json!(gwas_traits));
        payload.insert("gwas_trait".into(), serde_json::json!(gwas_traits));
        if let Some(primary) = gwas_traits.split(';').next().map(str::trim).filter(|s| !s.is_empty()) {
            payload.insert("primary_trait".into(), serde_json::json!(primary));
        }
    }
    if let Some(best_p) = best_gwas_pvalue(gwas_associations) {
        payload.insert("gwas_best_pvalue".into(), serde_json::json!(best_p));
    }
    payload.insert(
        "has_clinvar".into(),
        serde_json::json!(clinvar_sig.map(|s| !s.is_empty()).unwrap_or(false)),
    );
    if let Some(sig) = clinvar_sig.filter(|s| !s.is_empty()) {
        payload.insert("clinvar_significance".into(), serde_json::json!(sig));
    }
    if let Some(af) = gnomad_af {
        payload.insert("gnomad_af".into(), serde_json::json!(af));
        payload.insert(
            "population_common".into(),
            serde_json::json!(af >= 0.05),
        );
    }
    payload.insert(
        "significance_score".into(),
        serde_json::json!(significance_score),
    );
    payload.insert("last_enriched".into(), serde_json::json!(unix_now()));
    if let Some(chr) = chromosome.filter(|c| !c.is_empty()) {
        payload.insert("chromosome".into(), serde_json::json!(chr));
    }
    if let Some(pos) = position {
        payload.insert("position".into(), serde_json::json!(pos));
    }
    if !crossmap.trait_categories.is_empty() {
        payload.insert(
            "trait_categories".into(),
            serde_json::json!(crossmap.trait_categories),
        );
    }
    if !crossmap.consultation_modes.is_empty() {
        payload.insert(
            "consultation_modes".into(),
            serde_json::json!(crossmap.consultation_modes),
        );
    }
    if !crossmap.pack_refs.is_empty() {
        payload.insert("pack_refs".into(), serde_json::json!(crossmap.pack_refs));
    }
    if !crossmap.searchable_tags.is_empty() {
        payload.insert(
            "searchable_tags".into(),
            serde_json::json!(crossmap.searchable_tags),
        );
    }
    payload.insert(
        "association_summary".into(),
        crossmap.association_summary.clone(),
    );
    payload.insert(
        "cross_refs".into(),
        serde_json::json!({
            "rsid": rsid,
            "gene_symbol": gene.as_deref().unwrap_or(""),
            "genotype": genotype,
            "traits": gwas_traits.split(';').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>(),
            "trait_categories": crossmap.trait_categories,
            "consultation_modes": crossmap.consultation_modes,
            "pack_refs": crossmap.pack_refs,
            "discovery_catalog": crossmap.discovery_catalog_match,
            "association_summary": crossmap.association_summary,
            "searchable_tags": crossmap.searchable_tags,
            "enrichment_version": ENRICHMENT_VERSION,
            "has_clinvar": clinvar_sig.map(|s| !s.is_empty()).unwrap_or(false),
            "gnomad_af": gnomad_af,
            "chromosome": chromosome,
            "position": position,
        }),
    );
    serde_json::Value::Object(payload)
}
