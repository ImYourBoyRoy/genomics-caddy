// ./src-tauri/src/research/http.rs
/*
Shared HTTP clients and outbound rate limits for the research enrichment pipeline.
Reuses connection pools instead of building a new reqwest::Client per variant.
Outbound requests validate hosts against a research allowlist and follow redirect policy.
*/

use super::tuning;
use crate::config;
use reqwest::Client;
use std::net::IpAddr;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Semaphore;

const RESEARCH_ALLOWED_HOST_SUFFIXES: &[&str] = &[
    "ncbi.nlm.nih.gov",
    "ensembl.org",
    "ebi.ac.uk",
    "broadinstitute.org",
    "gtexportal.org",
    "pgscatalog.org",
    "reactome.org",
    "opentargets.org",
    "pharmgkb.org",
    "clinpgx.org",
    "amazonaws.com",
    "googleapis.com",
    "duckduckgo.com",
];

fn is_private_or_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || (v4.octets()[0] == 169 && v4.octets()[1] == 254)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.octets()[0] & 0xfe) == 0xfc
                || (v6.octets()[0] == 0xfe && (v6.octets()[1] & 0xc0) == 0x80)
        }
    }
}

/// Validate URLs used by the research pipeline (hardcoded adapters + gnomAD HTTPS).
pub fn validate_research_outbound_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url.trim()).map_err(|e| format!("Invalid URL: {e}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(format!("Unsupported URL scheme: {other}")),
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL must include a host".to_string())?;
    if host.eq_ignore_ascii_case("metadata.google.internal") || host == "169.254.169.254" {
        return Err("Metadata endpoints are not allowed".into());
    }
    if host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1" {
        return Err("Localhost URLs are not allowed for research outbound requests".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>()
        && is_private_or_loopback(ip)
    {
        return Err("Private or link-local network addresses are not allowed".into());
    }
    let host_lower = host.to_ascii_lowercase();
    let allowed = RESEARCH_ALLOWED_HOST_SUFFIXES
        .iter()
        .any(|suffix| host_lower == *suffix || host_lower.ends_with(&format!(".{suffix}")));
    if !allowed {
        return Err(format!(
            "Host '{host}' is not in the research outbound allowlist"
        ));
    }
    Ok(())
}

pub fn ensure_research_outbound_url(url: &str) -> Result<(), String> {
    validate_research_outbound_url(url)
}

pub static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(32)
        .tcp_keepalive(Duration::from_secs(60))
        .redirect(config::redirect_policy())
        .build()
        .expect("Failed to build shared HTTP client")
});

pub static HEALTH_CHECK_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(4))
        .connect_timeout(Duration::from_secs(2))
        .pool_max_idle_per_host(4)
        .redirect(config::redirect_policy())
        .build()
        .expect("Failed to build health-check HTTP client")
});

pub static EMBED_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(180))
        .connect_timeout(Duration::from_secs(15))
        .pool_max_idle_per_host(16)
        .build()
        .expect("Failed to build shared embed HTTP client")
});

/// Shared client for Qdrant REST (search, upsert, scroll). Reuses connection pool across requests.
pub static QDRANT_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(24)
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .expect("Failed to build Qdrant HTTP client")
});

/// Sync HTTP for `spawn_blocking` paths (gnomAD tabix). Never call `Handle::block_on` from blocking threads.
pub static BLOCKING_HTTP_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(8)
        .redirect(config::redirect_policy())
        .build()
        .expect("Failed to build blocking HTTP client")
});

fn adapter_concurrency(env: &str, default: usize) -> usize {
    std::env::var(env)
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(default)
}

pub static PGS_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(adapter_concurrency("GENOMICS_PGS_CONCURRENCY", 4)));

pub static REACTOME_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(adapter_concurrency("GENOMICS_REACTOME_CONCURRENCY", 4)));

pub static OPENTARGETS_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(adapter_concurrency("GENOMICS_OPENTARGETS_CONCURRENCY", 4)));

pub static PHARMGKB_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(adapter_concurrency("GENOMICS_PHARMGKB_CONCURRENCY", 3)));

pub static OLS4_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(adapter_concurrency("GENOMICS_OLS4_CONCURRENCY", 4)));

pub fn enrich_batch_size() -> usize {
    tuning::enrich_batch_size()
}

pub fn sweep_fast_mode() -> bool {
    tuning::sweep_fast_enabled(false)
}

/// Acquire an adapter semaphore with a hard wait cap (prevents infinite stall after prefetch timeout).
pub async fn acquire_adapter_permit(sem: &Semaphore) -> Option<tokio::sync::SemaphorePermit<'_>> {
    let wait = Duration::from_secs(tuning::adapter_semaphore_wait_secs());
    match tokio::time::timeout(wait, sem.acquire()).await {
        Ok(Ok(permit)) => Some(permit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_url_allows_known_adapters() {
        assert!(
            validate_research_outbound_url("https://rest.ensembl.org/vep/human/id/rs123").is_ok()
        );
        assert!(validate_research_outbound_url(
            "https://gnomad-public-us-east-1.s3.amazonaws.com/release/4.1/vcf/exomes/gnomad.exomes.v4.1.sites.chr22.vcf.bgz"
        )
        .is_ok());
        assert!(validate_research_outbound_url("https://gnomad.broadinstitute.org/api").is_ok());
    }

    #[test]
    fn research_url_blocks_metadata_and_localhost() {
        assert!(validate_research_outbound_url("http://127.0.0.1:8080/").is_err());
        assert!(validate_research_outbound_url("http://169.254.169.254/latest/meta-data").is_err());
        assert!(validate_research_outbound_url("https://evil.example.com/data").is_err());
    }
}
