// ./src-tauri/src/research/sources.rs
use super::http::HTTP_CLIENT;
use super::tuning::acquire_gwas_permit;
use super::sweep_metrics::{record_phase_cache, SweepPhase};
use super::util::{
    canonical_gwas_association, extract_gene_from_gwas, is_placeholder_gene, normalize_api_gwas_association,
    normalize_rsid, parse_gene_tokens,
};
use rusqlite::params;
use std::path::Path;

pub(crate) fn load_local_gwas_associations(
    db_path: &Path,
    rsid: &str,
) -> Option<Vec<serde_json::Value>> {
    let rsid_norm = normalize_rsid(rsid)?;
    let conn = crate::db::connect(db_path).ok()?;
    let row: Result<(i64, String, String, String, String, Option<f64>, String), _> = conn
        .query_row(
            "SELECT association_count, top_trait, primary_gene, mapped_genes, reported_genes, best_pvalue, associations_json
             FROM gwas_reference WHERE rsid = ?",
            params![rsid_norm],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        );
    let (count, top_trait, primary_gene, mapped_genes, reported_genes, best_pvalue, associations_json) =
        row.ok()?;
    if count <= 0 {
        return None;
    }

    if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(&associations_json)
        && !parsed.is_empty() {
            return Some(parsed);
        }

    let traits: Vec<String> = top_trait
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let primary = if is_placeholder_gene(&primary_gene) {
        parse_gene_tokens(&mapped_genes)
            .into_iter()
            .next()
            .or_else(|| parse_gene_tokens(&reported_genes).into_iter().next())
    } else {
        Some(primary_gene)
    };
    let reported = parse_gene_tokens(&reported_genes);
    let mapped = parse_gene_tokens(&mapped_genes);
    let mapped_gene = primary
        .as_deref()
        .or_else(|| mapped.first().map(String::as_str));

    let trait_names: Vec<String> = if traits.len() > 1 {
        traits.into_iter().take(8).collect()
    } else if !top_trait.trim().is_empty() {
        vec![top_trait.trim().to_string()]
    } else {
        vec!["GWAS catalog association".to_string()]
    };

    Some(
        trait_names
            .iter()
            .map(|trait_name| {
                canonical_gwas_association(
                    trait_name,
                    best_pvalue.or(Some(1e-10)),
                    &if reported.is_empty() {
                        mapped.clone()
                    } else {
                        reported.clone()
                    },
                    mapped_gene,
                    None,
                    "gwas_catalog_local",
                )
            })
            .collect(),
    )
}

pub fn gwas_needs_api_supplement(db_path: &Path, rsid: &str) -> bool {
    if super::tuning::local_gwas_only_in_sweep() {
        return false;
    }
    let Some(local) = load_local_gwas_associations(db_path, rsid) else {
        return true;
    };
    let local_len = local.len();
    let lacked_genes = associations_lack_genes(&local);
    let weak_mapping = local.iter().all(|a| {
        a["mapped_gene"]
            .as_str()
            .filter(|s| !is_placeholder_gene(s))
            .is_none()
    });
    lacked_genes || local_len < 3 || weak_mapping
}

pub async fn resolve_gwas_associations(
    db_path: &Path,
    rsid: &str,
    strict: bool,
) -> (Vec<serde_json::Value>, serde_json::Value) {
    let local_only = super::tuning::local_gwas_only_in_sweep();
    if let Some(local) = load_local_gwas_associations(db_path, rsid) {
        let local_len = local.len();
        let lacked_genes = associations_lack_genes(&local);
        let weak_mapping = local.iter().all(|a| {
            a["mapped_gene"]
                .as_str()
                .filter(|s| !is_placeholder_gene(s))
                .is_none()
        });
        let needs_api_supplement = gwas_needs_api_supplement(db_path, rsid);
        if !needs_api_supplement {
            record_phase_cache(SweepPhase::Gwas, true);
        }
        let mut merged = local.clone();
        let mut api_hits = 0u64;
        if needs_api_supplement {
            let api = fetch_gwas_associations(db_path, rsid, false)
                .await
                .unwrap_or_default();
            api_hits = api.len() as u64;
            if !api.is_empty() {
                merged = merge_gwas_association_sets(local, api);
            }
        }
        let provenance = serde_json::json!({
            "local_gwas_reference": { "queried": true, "hits": local_len },
            "gwas_api": {
                "queried": needs_api_supplement,
                "hits": api_hits,
                "supplement": api_hits > 0,
                "reason": if lacked_genes { "missing_genes" } else if weak_mapping { "weak_gene_mapping" } else if local_len < 3 { "sparse_local_traits" } else { "none" },
            }
        });
        return (merged, provenance);
    }

    if local_only {
        record_phase_cache(SweepPhase::Gwas, true);
        let provenance = serde_json::json!({
            "local_gwas_reference": { "queried": true, "hits": 0 },
            "gwas_api": { "queried": false, "reason": "fast_sweep_local_only" }
        });
        return (Vec::new(), provenance);
    }

    let api = fetch_gwas_associations(db_path, rsid, strict)
        .await
        .unwrap_or_default();
    let normalized: Vec<serde_json::Value> = api
        .iter()
        .map(normalize_api_gwas_association)
        .collect();
    let provenance = serde_json::json!({
        "local_gwas_reference": { "queried": true, "hits": 0 },
        "gwas_api": { "queried": true, "hits": normalized.len() }
    });
    (normalized, provenance)
}

fn associations_lack_genes(assocs: &[serde_json::Value]) -> bool {
    extract_gene_from_gwas(assocs).is_none()
}

fn merge_gwas_association_sets(
    local: Vec<serde_json::Value>,
    api: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let mut out = api;
    for assoc in local {
        let trait_name = assoc["trait_name"]
            .as_str()
            .or_else(|| assoc["trait"]["trait"].as_str())
            .unwrap_or("");
        let duplicate = out.iter().any(|existing| {
            existing["trait_name"]
                .as_str()
                .or_else(|| existing["trait"]["trait"].as_str())
                .unwrap_or("")
                == trait_name
        });
        if !duplicate {
            out.push(assoc);
        }
    }
    out.truncate(16);
    out
}

#[derive(Debug, Clone)]
pub struct ExternalGeneAnnotation {
    pub genes: Vec<String>,
    pub chromosome: Option<String>,
    pub position: Option<i64>,
    pub source: &'static str,
}

pub async fn fetch_ensembl_vep_genes(db_path: &Path, rsid: &str) -> Option<ExternalGeneAnnotation> {
    let url = format!(
        "https://rest.ensembl.org/vep/human/id/{}?content-type=application/json",
        rsid.to_lowercase()
    );
    let cache_key = crate::research::evidence::cache::ensembl_vep_cache_key(rsid);
    let val = crate::research::evidence::cache::fetch_json_cached(
        db_path,
        "ensembl",
        "vep",
        &cache_key,
        &url,
        crate::research::evidence::cache::ensembl_ttl(),
        None,
    )
    .await
    .ok()?;

    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = crate::research::evidence::source_records::record_ensembl_vep(
            &conn,
            rsid,
            &url,
            &val,
        );
    }

    let entry = val.as_array()?.first()?;
    let mut genes: Vec<String> = Vec::new();
    if let Some(consequences) = entry["transcript_consequences"].as_array() {
        for tc in consequences {
            if let Some(symbol) = tc["gene_symbol"].as_str()
                && !is_placeholder_gene(symbol)
                    && !genes.iter().any(|g: &String| g.eq_ignore_ascii_case(symbol))
                {
                    genes.push(symbol.to_string());
                }
        }
    }
    let chromosome = entry["seq_region_name"]
        .as_str()
        .map(|c| c.to_string());
    let position = entry["start"].as_i64();

    if genes.is_empty() && chromosome.is_none() {
        return None;
    }

    Some(ExternalGeneAnnotation {
        genes: genes.into_iter().take(6).collect(),
        chromosome,
        position,
        source: "ensembl_vep",
    })
}

pub async fn fetch_gwas_associations(
    db_path: &Path,
    rsid: &str,
    strict: bool,
) -> Result<Vec<serde_json::Value>, String> {
    if super::tuning::local_gwas_only_in_sweep() {
        return Ok(Vec::new());
    }
    let _permit = acquire_gwas_permit().await;

    let url = format!(
        "https://www.ebi.ac.uk/gwas/rest/api/singleNucleotidePolymorphisms/{}/associations?projection=associationBySnp",
        rsid
    );
    let cache_key = crate::research::evidence::cache::gwas_cache_key(rsid);
    let val = match crate::research::evidence::cache::fetch_json_cached(
        db_path,
        "gwas_catalog",
        "associations",
        &cache_key,
        &url,
        crate::research::evidence::cache::gwas_ttl(),
        None,
    )
    .await
    {
        Ok(v) => v,
        Err(_) => return Ok(vec![]),
    };

    persist_gwas_source_records(db_path, rsid, &url, &val);

    let assocs = val["_embedded"]["associations"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let p_threshold = if strict { 5e-8_f64 } else { 1e-5_f64 };
    let filtered: Vec<serde_json::Value> = assocs
        .into_iter()
        .filter(|a| {
            a["pvalue"].as_f64().map(|p| p < p_threshold).unwrap_or(false)
        })
        .take(12)
        .collect();

    Ok(filtered)
}

fn persist_gwas_source_records(db_path: &Path, rsid: &str, url: &str, body: &serde_json::Value) {
    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = crate::research::evidence::source_records::record_gwas_api_response(
            &conn, rsid, url, body,
        );
    }
}

// ---------------------------------------------------------------------------
// 8. fetch_gnomad_frequency (legacy AF-only helper — delegates to indexed VCF path)
// ---------------------------------------------------------------------------

pub async fn fetch_gnomad_frequency(rsid: &str) -> Option<f64> {
    let _ = rsid;
    None
}

// ---------------------------------------------------------------------------
// 8b. fetch_gtex_gencode_id
// ---------------------------------------------------------------------------

pub async fn fetch_gtex_gencode_id_cached(db_path: &Path, gene_symbol: &str) -> Option<String> {
    let url = format!(
        "https://gtexportal.org/api/v2/reference/gene?geneId={}&gencodeVersion=v39",
        gene_symbol
    );
    let cache_key = format!("gtex|gencode|{}", gene_symbol.to_uppercase());
    let val = crate::research::evidence::cache::fetch_json_cached(
        db_path,
        "gtex",
        "gencode",
        &cache_key,
        &url,
        crate::research::evidence::cache::gtex_ttl(),
        None,
    )
    .await
    .ok()?;

    let data = val.get("data")?.as_array()?;
    if data.is_empty() {
        return None;
    }

    let mut gencode_id = None;
    for d in data {
        if d["geneSymbol"].as_str().map(|s| s.to_uppercase()) == Some(gene_symbol.to_uppercase()) {
            gencode_id = d["gencodeId"].as_str().map(String::from);
            break;
        }
    }

    if gencode_id.is_none() {
        gencode_id = data[0]["gencodeId"].as_str().map(String::from);
    }

    gencode_id
}

pub async fn fetch_gtex_gencode_id(gene_symbol: &str) -> Option<String> {
    fetch_gtex_gencode_id_uncached(gene_symbol).await
}

async fn fetch_gtex_gencode_id_uncached(gene_symbol: &str) -> Option<String> {
    let client = &*HTTP_CLIENT;

    let url = format!(
        "https://gtexportal.org/api/v2/reference/gene?geneId={}&gencodeVersion=v39",
        gene_symbol
    );

    let res = client.get(&url).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let val: serde_json::Value = res.json().await.ok()?;
    let data = val.get("data")?.as_array()?;
    if data.is_empty() {
        return None;
    }

    let mut gencode_id = None;
    for d in data {
        if d["geneSymbol"].as_str().map(|s| s.to_uppercase()) == Some(gene_symbol.to_uppercase()) {
            gencode_id = d["gencodeId"].as_str().map(String::from);
            break;
        }
    }

    if gencode_id.is_none() {
        gencode_id = data[0]["gencodeId"].as_str().map(String::from);
    }

    gencode_id
}

// ---------------------------------------------------------------------------
// 8c. fetch_gtex_median_expression
// ---------------------------------------------------------------------------

pub async fn fetch_gtex_median_expression(gencode_id: &str) -> Option<Vec<(String, f64)>> {
    let client = &*HTTP_CLIENT;

    let url = format!(
        "https://gtexportal.org/api/v2/expression/medianGeneExpression?gencodeId={}&datasetId=gtex_v10",
        gencode_id
    );

    let res = client.get(&url).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let val: serde_json::Value = res.json().await.ok()?;
    let data = val["data"].as_array()?;

    let mut tissue_exprs = Vec::new();
    for item in data {
        let tissue = item["tissueSiteDetailId"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();
        let median = item["median"].as_f64().unwrap_or(0.0);
        tissue_exprs.push((tissue, median));
    }

    tissue_exprs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Some(tissue_exprs)
}

// ---------------------------------------------------------------------------
// 8d. fetch_gtex_eqtls_for_rsid (gene-level API, filtered to matching rsID)
// GTEx v2 has no singleTissueEqtlByVariant endpoint; gene query + snpId filter works.
// ---------------------------------------------------------------------------

pub async fn fetch_gtex_eqtls_for_rsid(
    db_path: &Path,
    rsid: &str,
    gene: Option<&str>,
) -> Option<Vec<serde_json::Value>> {
    let gene = gene.filter(|g| !is_placeholder_gene(g))?;
    let gencode_id = fetch_gtex_gencode_id_cached(db_path, gene).await?;
    let rsid_upper = rsid.trim().to_uppercase();
    let mut matches: Vec<serde_json::Value> = Vec::new();

    for page in 0..4u32 {
        let url = format!(
            "https://gtexportal.org/api/v2/association/singleTissueEqtl?gencodeId={}&datasetId=gtex_v10&page={}&itemsPerPage=250",
            gencode_id, page
        );
        let cache_key = format!("gtex|eqtl|{}|p{}", gencode_id, page);
        let val = match crate::research::evidence::cache::fetch_json_cached(
            db_path,
            "gtex",
            "single_tissue_eqtl",
            &cache_key,
            &url,
            crate::research::evidence::cache::gtex_ttl(),
            None,
        )
        .await
        {
            Ok(v) => v,
            Err(_) => break,
        };

        let data = match val["data"].as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => break,
        };

        for item in data {
            let snp = item["snpId"]
                .as_str()
                .map(|s| s.trim().to_uppercase())
                .unwrap_or_default();
            if snp == rsid_upper {
                matches.push(item.clone());
            }
        }

        if !matches.is_empty() || data.len() < 250 {
            break;
        }
    }

    if matches.is_empty() {
        return None;
    }

    matches.sort_by(|a, b| {
        let pa = a["pValue"].as_f64().unwrap_or(f64::MAX);
        let pb = b["pValue"].as_f64().unwrap_or(f64::MAX);
        pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
    });
    matches.truncate(3);
    Some(matches)
}
