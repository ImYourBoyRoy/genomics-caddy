// ./src-tauri/src/research/gnomad/graphql.rs
use super::cache::{context_to_cache_write, hash_record, write_cache};
use super::types::{GnomadContext, GnomadLookupStatus, GnomadSourceMode};
use crate::research::http::HTTP_CLIENT;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::time::sleep;

static GRAPHQL_THROTTLE: Mutex<Option<Instant>> = Mutex::new(None);
const MIN_INTERVAL: Duration = Duration::from_secs(6);
const MAX_RETRIES: u32 = 4;

async fn throttle_graphql() {
    loop {
        let wait = {
            let mut guard = GRAPHQL_THROTTLE.lock().expect("graphql throttle lock");
            let now = Instant::now();
            if let Some(last) = *guard {
                let elapsed = now.duration_since(last);
                if elapsed < MIN_INTERVAL {
                    Some(MIN_INTERVAL - elapsed)
                } else {
                    *guard = Some(now);
                    None
                }
            } else {
                *guard = Some(now);
                None
            }
        };
        match wait {
            Some(d) => sleep(d).await,
            None => return,
        }
    }
}

async fn post_graphql(body: &serde_json::Value) -> Result<serde_json::Value, GnomadLookupStatus> {
    for attempt in 0..MAX_RETRIES {
        throttle_graphql().await;
        let res = HTTP_CLIENT
            .post("https://gnomad.broadinstitute.org/api")
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await;
        match res {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return resp.json().await.map_err(|_| GnomadLookupStatus::ParserError);
                }
                if status.as_u16() == 403
                    || status.as_u16() == 429
                    || status.is_server_error()
                {
                    let backoff = Duration::from_secs(2u64.pow(attempt.min(4)));
                    sleep(backoff).await;
                    continue;
                }
                return Err(GnomadLookupStatus::NetworkError);
            }
            Err(_) => {
                if attempt + 1 >= MAX_RETRIES {
                    return Err(GnomadLookupStatus::NetworkError);
                }
                sleep(Duration::from_secs(2u64.pow(attempt.min(4)))).await;
            }
        }
    }
    Err(GnomadLookupStatus::NetworkError)
}

pub async fn fetch_graphql_context(
    db_path: &Path,
    rsid: &str,
    release: &str,
    genotype: Option<&str>,
    chrom: Option<&str>,
    pos: Option<i64>,
    ref_allele: Option<&str>,
    alt_allele: Option<&str>,
) -> GnomadContext {
    let search_query = serde_json::json!({
        "query": "query($query: String!) { variant_search(query: $query, dataset: gnomad_r4) { variant_id rsids } }",
        "variables": { "query": rsid }
    });

    let val1 = match post_graphql(&search_query).await {
        Ok(v) => v,
        Err(status) => return GnomadContext::empty(status),
    };

    let hits = val1["data"]["variant_search"].as_array();
    let Some(hits) = hits else {
        return GnomadContext::empty(GnomadLookupStatus::ParserError);
    };
    if hits.is_empty() {
        return GnomadContext::empty(GnomadLookupStatus::NoRecordAtPosition);
    }

    let mut best: Option<GnomadContext> = None;
    for item in hits {
        let Some(var_id) = item["variant_id"].as_str() else {
            continue;
        };
        let rsids: Vec<String> = item["rsids"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let freq_query = serde_json::json!({
            "query": "query($varId: String!) { variant(variantId: $varId, dataset: gnomad_r4) {
                genome { ac an af homozygoteCount hemizygoteCount filters flags popmax { af population } }
                exome { ac an af homozygoteCount hemizygoteCount filters flags popmax { af population } }
            } }",
            "variables": { "varId": var_id }
        });

        let val2 = match post_graphql(&freq_query).await {
            Ok(v) => v,
            Err(status) => return GnomadContext::empty(status),
        };
        let variant = match val2["data"]["variant"].as_object() {
            Some(v) if !val2["data"]["variant"].is_null() => v,
            _ => continue,
        };

        let genome = variant.get("genome");
        let exome = variant.get("exome");
        let pick_af = |node: Option<&serde_json::Value>| {
            node.and_then(|n| n.get("af")).and_then(|v| v.as_f64())
        };
        let pick_i = |node: Option<&serde_json::Value>, key: &str| {
            node.and_then(|n| n.get(key)).and_then(|v| v.as_i64())
        };
        let pick_popmax = |node: Option<&serde_json::Value>| {
            node.and_then(|n| n.get("popmax"))
                .and_then(|p| p.get("af"))
                .and_then(|v| v.as_f64())
        };
        let pick_pop = |node: Option<&serde_json::Value>| {
            node.and_then(|n| n.get("popmax"))
                .and_then(|p| p.get("population"))
                .and_then(|v| v.as_str())
                .map(String::from)
        };

        let af_genomes = pick_af(genome);
        let af_exomes = pick_af(exome);
        let af = [af_genomes, af_exomes]
            .into_iter()
            .flatten()
            .filter(|v| *v > 0.0)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let ctx = GnomadContext {
            lookup_status: GnomadLookupStatus::GraphQlHit.as_str().to_string(),
            release: release.to_string(),
            dataset: "combined".to_string(),
            variant_id: Some(var_id.to_string()),
            rsids,
            ac: pick_i(genome, "ac").or_else(|| pick_i(exome, "ac")),
            an: pick_i(genome, "an").or_else(|| pick_i(exome, "an")),
            af,
            ac_exomes: pick_i(exome, "ac"),
            an_exomes: pick_i(exome, "an"),
            af_exomes,
            ac_genomes: pick_i(genome, "ac"),
            an_genomes: pick_i(genome, "an"),
            af_genomes,
            popmax: pick_popmax(genome).or_else(|| pick_popmax(exome)),
            popmax_population: pick_pop(genome).or_else(|| pick_pop(exome)),
            faf95_popmax: None,
            faf95_popmax_population: None,
            homozygote_count: pick_i(genome, "homozygoteCount")
                .or_else(|| pick_i(exome, "homozygoteCount")),
            hemizygote_count: pick_i(genome, "hemizygoteCount")
                .or_else(|| pick_i(exome, "hemizygoteCount")),
            filters: Vec::new(),
            flags: Vec::new(),
            population_frequencies: serde_json::json!({}),
            source_url: Some("https://gnomad.broadinstitute.org/api".into()),
            source_mode: GnomadSourceMode::GraphQlInteractive.as_str().to_string(),
            fetched_at: Some(unix_now()),
            warnings: vec!["GraphQL provides population context only.".into()],
            user_allele_match_status: Some("graphql_rsid_match".into()),
        };

        if ctx.af.is_some() {
            best = Some(ctx);
            break;
        }
        if best.is_none() {
            best = Some(ctx);
        }
    }

    let Some(ctx) = best else {
        return GnomadContext::empty(GnomadLookupStatus::NoRecordAtPosition);
    };

    if let (Some(ch), Some(p), Some(r), Some(a)) = (chrom, pos, ref_allele, alt_allele)
        && let Ok(conn) = crate::db::connect(db_path) {
            let mut input = context_to_cache_write(
                &ctx,
                GnomadSourceMode::GraphQlInteractive,
                ch,
                p,
                r,
                a,
                genotype.map(String::from),
                hash_record(&format!("{rsid}|graphql")),
            );
            input.lookup_status = GnomadLookupStatus::GraphQlHit;
            let _ = write_cache(&conn, input);
        }

    ctx
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
