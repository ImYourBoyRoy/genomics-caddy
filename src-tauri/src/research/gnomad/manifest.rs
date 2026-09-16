// ./src-tauri/src/research/gnomad/manifest.rs
//! Discover which gnomAD site VCF contigs exist for a release/provider (manifest-driven readiness).

use super::config::{index_urls, vcf_url};
use super::lookup::gnomad_cache_dir;
use super::types::{GnomadConfig, GnomadHttpsProvider};
use super::vcf_remote::head_url_ok;
use crate::research::http::HTTP_CLIENT;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const MANIFEST_TTL_SECS: i64 = 7 * 24 * 3600;
const RELEASE_DISCOVERY_TTL_SECS: i64 = 24 * 3600;

/// Candidate contigs to probe — includes M/MT so 404 becomes `unsupported`, not a hard failure.
pub fn manifest_candidate_contigs() -> &'static [&'static str] {
    &[
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        "17", "18", "19", "20", "21", "22", "X", "Y", "M",
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadReleaseManifest {
    pub release: String,
    pub provider: String,
    pub exome_template: String,
    pub genome_template: String,
    pub built_at: i64,
    pub exome_contigs: Vec<String>,
    pub genome_contigs: Vec<String>,
    pub unsupported_contigs: Vec<String>,
}

impl GnomadReleaseManifest {
    pub fn contig_available(&self, chrom: &str, dataset: &str) -> bool {
        let key = super::allele_norm::normalize_chrom_key(chrom);
        match dataset {
            "exomes" => self.exome_contigs.iter().any(|c| c == &key),
            "genomes" => self.genome_contigs.iter().any(|c| c == &key),
            _ => {
                self.exome_contigs.iter().any(|c| c == &key)
                    || self.genome_contigs.iter().any(|c| c == &key)
            }
        }
    }

    pub fn is_supported_chrom(&self, chrom: &str) -> bool {
        let key = super::allele_norm::normalize_chrom_key(chrom);
        self.exome_contigs.contains(&key) || self.genome_contigs.contains(&key)
    }

    pub fn expected_index_count(&self) -> u32 {
        (self.exome_contigs.len() + self.genome_contigs.len()) as u32
    }
}

pub fn manifest_path(data_dir: &Path) -> PathBuf {
    gnomad_cache_dir(data_dir).join("release_manifest.json")
}

fn release_discovery_path(data_dir: &Path) -> PathBuf {
    gnomad_cache_dir(data_dir).join("release_discovery.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseDiscoveryCache {
    provider: String,
    release: String,
    checked_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GnomadReleaseDiscovery {
    pub release: String,
    pub exome_template: String,
    pub genome_template: String,
}

fn provider_key(provider: GnomadHttpsProvider) -> &'static str {
    match provider {
        GnomadHttpsProvider::Aws => "aws",
        GnomadHttpsProvider::Google => "google",
    }
}

fn release_listing_url(provider: GnomadHttpsProvider) -> &'static str {
    match provider {
        GnomadHttpsProvider::Aws => {
            "https://gnomad-public-us-east-1.s3.amazonaws.com/?list-type=2&delimiter=%2F&prefix=release%2F"
        }
        GnomadHttpsProvider::Google => {
            "https://storage.googleapis.com/storage/v1/b/gcp-public-data--gnomad/o?prefix=release%2F&delimiter=%2F&maxResults=1000"
        }
    }
}

fn release_sort_key(release: &str) -> Option<Vec<u64>> {
    let normalized = release.trim().trim_start_matches('v');
    let parts: Vec<u64> = normalized
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    (parts.len() >= 2).then_some(parts)
}

fn parse_release_candidates(body: &str) -> Vec<String> {
    let mut found = BTreeSet::new();
    let mut cursor = 0;
    while let Some(relative) = body[cursor..].find("release/") {
        let start = cursor + relative + "release/".len();
        let tail = &body[start..];
        let Some(end) = tail.find('/') else { break };
        let release = &tail[..end];
        if release_sort_key(release).is_some() {
            found.insert(release.to_string());
        }
        cursor = start + end + 1;
    }
    found.into_iter().collect()
}

fn templates_for_release(release: &str) -> Option<(String, String)> {
    release_sort_key(release)?;
    let file_release = if release.starts_with('v') {
        release.to_string()
    } else {
        format!("v{release}")
    };
    Some((
        format!("release/{release}/vcf/exomes/gnomad.exomes.{file_release}.sites.chr{{chrom}}.vcf.bgz"),
        format!("release/{release}/vcf/genomes/gnomad.genomes.{file_release}.sites.chr{{chrom}}.vcf.bgz"),
    ))
}

fn load_release_discovery(data_dir: &Path, provider: GnomadHttpsProvider) -> Option<GnomadReleaseDiscovery> {
    let raw = std::fs::read_to_string(release_discovery_path(data_dir)).ok()?;
    let cached: ReleaseDiscoveryCache = serde_json::from_str(&raw).ok()?;
    if cached.provider != provider_key(provider)
        || unix_now().saturating_sub(cached.checked_at) >= RELEASE_DISCOVERY_TTL_SECS
    {
        return None;
    }
    let (exome_template, genome_template) = templates_for_release(&cached.release)?;
    Some(GnomadReleaseDiscovery {
        release: cached.release,
        exome_template,
        genome_template,
    })
}

fn save_release_discovery(
    data_dir: &Path,
    provider: GnomadHttpsProvider,
    release: &str,
) -> Result<(), String> {
    let dir = gnomad_cache_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let cache = ReleaseDiscoveryCache {
        provider: provider_key(provider).to_string(),
        release: release.to_string(),
        checked_at: unix_now(),
    };
    let json = serde_json::to_string_pretty(&cache).map_err(|e| e.to_string())?;
    crate::file_utils::atomic_write(&release_discovery_path(data_dir), json.as_bytes())
}

/// Discover the newest release whose public exome and genome site indexes are
/// actually available. The listing is authoritative for release names; the
/// chr22 index probes prevent a directory-only or incomplete release from
/// becoming the active source. Results are cached for one day.
pub async fn discover_latest_gnomad_release(
    cfg: &GnomadConfig,
    data_dir: &Path,
    force_refresh: bool,
) -> Option<GnomadReleaseDiscovery> {
    if !force_refresh
        && let Some(cached) = load_release_discovery(data_dir, cfg.provider)
    {
        return Some(cached);
    }

    let response = HTTP_CLIENT.get(release_listing_url(cfg.provider)).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().await.ok()?;
    let mut candidates: Vec<(String, Vec<u64>)> = parse_release_candidates(&body)
        .into_iter()
        .filter_map(|release| release_sort_key(&release).map(|key| (release, key)))
        .collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1));

    for (release, _) in candidates {
        let Some((exome_template, genome_template)) = templates_for_release(&release) else {
            continue;
        };
        let exome_vcf = format!("{}/{}", cfg.provider.base_url(), exome_template.replace("{chrom}", "22"));
        let genome_vcf = format!("{}/{}", cfg.provider.base_url(), genome_template.replace("{chrom}", "22"));
        let (exome_tbi, exome_csi) = index_urls(&exome_vcf);
        let (genome_tbi, genome_csi) = index_urls(&genome_vcf);
        let exome_ok = head_url_ok(&exome_tbi).await || head_url_ok(&exome_csi).await;
        let genome_ok = head_url_ok(&genome_tbi).await || head_url_ok(&genome_csi).await;
        if exome_ok && genome_ok {
            let _ = save_release_discovery(data_dir, cfg.provider, &release);
            return Some(GnomadReleaseDiscovery {
                release,
                exome_template,
                genome_template,
            });
        }
    }
    None
}

pub fn load_manifest(data_dir: &Path) -> Option<GnomadReleaseManifest> {
    let path = manifest_path(data_dir);
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_manifest(data_dir: &Path, manifest: &GnomadReleaseManifest) -> Result<(), String> {
    let dir = gnomad_cache_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    crate::file_utils::atomic_write(&manifest_path(data_dir), json.as_bytes())
}

fn manifest_matches_cfg(manifest: &GnomadReleaseManifest, cfg: &GnomadConfig) -> bool {
    manifest.release == cfg.release
        && manifest.provider
            == match cfg.provider {
                super::types::GnomadHttpsProvider::Aws => "aws",
                super::types::GnomadHttpsProvider::Google => "google",
            }
        && manifest.exome_template == cfg.exome_template
        && manifest.genome_template == cfg.genome_template
}

fn manifest_fresh(manifest: &GnomadReleaseManifest) -> bool {
    let now = unix_now();
    now.saturating_sub(manifest.built_at) < MANIFEST_TTL_SECS
}

/// A manifest with no usable dataset contigs is an incomplete probe, not a
/// valid declaration that the public release has no site VCFs. This matters
/// after a transient network failure because the manifest is otherwise cached
/// for a week and readiness would incorrectly report success with zero files.
pub fn manifest_has_usable_contigs(manifest: &GnomadReleaseManifest) -> bool {
    manifest.expected_index_count() > 0
}

pub async fn get_or_build_manifest(
    cfg: &GnomadConfig,
    data_dir: &Path,
    force_refresh: bool,
) -> GnomadReleaseManifest {
    if !force_refresh
        && let Some(m) = load_manifest(data_dir)
        && manifest_matches_cfg(&m, cfg)
        && manifest_fresh(&m)
        && manifest_has_usable_contigs(&m)
    {
        return m;
    }
    let manifest = build_manifest(cfg).await;
    let _ = save_manifest(data_dir, &manifest);
    manifest
}

pub async fn build_manifest(cfg: &GnomadConfig) -> GnomadReleaseManifest {
    let mut exome_contigs = Vec::new();
    let mut genome_contigs = Vec::new();
    let mut unsupported_contigs = Vec::new();

    for chrom in manifest_candidate_contigs() {
        let exome_vcf = vcf_url(cfg, &cfg.exome_template, chrom);
        let (exome_tbi, exome_csi) = index_urls(&exome_vcf);
        let exome_ok = head_url_ok(&exome_tbi).await || head_url_ok(&exome_csi).await;

        let genome_vcf = vcf_url(cfg, &cfg.genome_template, chrom);
        let (genome_tbi, genome_csi) = index_urls(&genome_vcf);
        let genome_ok = head_url_ok(&genome_tbi).await || head_url_ok(&genome_csi).await;

        if exome_ok {
            exome_contigs.push(chrom.to_string());
        }
        if genome_ok {
            genome_contigs.push(chrom.to_string());
        }
        if !exome_ok && !genome_ok {
            unsupported_contigs.push(chrom.to_string());
        }
    }

    GnomadReleaseManifest {
        release: cfg.release.clone(),
        provider: match cfg.provider {
            super::types::GnomadHttpsProvider::Aws => "aws".into(),
            super::types::GnomadHttpsProvider::Google => "google".into(),
        },
        exome_template: cfg.exome_template.clone(),
        genome_template: cfg.genome_template.clone(),
        built_at: unix_now(),
        exome_contigs,
        genome_contigs,
        unsupported_contigs,
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_index_count_sums_datasets() {
        let m = GnomadReleaseManifest {
            release: "4.1".into(),
            provider: "aws".into(),
            exome_template: String::new(),
            genome_template: String::new(),
            built_at: 0,
            exome_contigs: vec!["1".into(), "2".into()],
            genome_contigs: vec!["1".into()],
            unsupported_contigs: vec!["M".into()],
        };
        assert_eq!(m.expected_index_count(), 3);
        assert!(!m.is_supported_chrom("M"));
        assert!(m.is_supported_chrom("chr1"));
        assert!(manifest_has_usable_contigs(&m));
    }

    #[test]
    fn empty_manifest_is_not_treated_as_a_successful_probe() {
        let m = GnomadReleaseManifest {
            release: "4.1".into(),
            provider: "aws".into(),
            exome_template: String::new(),
            genome_template: String::new(),
            built_at: unix_now(),
            exome_contigs: Vec::new(),
            genome_contigs: Vec::new(),
            unsupported_contigs: manifest_candidate_contigs()
                .iter()
                .map(|c| (*c).to_string())
                .collect(),
        };
        assert!(!manifest_has_usable_contigs(&m));
    }

    #[test]
    fn release_discovery_parses_and_orders_public_release_prefixes() {
        let listing = r#"
          <CommonPrefixes><Prefix>release/4.1/</Prefix></CommonPrefixes>
          <CommonPrefixes><Prefix>release/4.1.1/</Prefix></CommonPrefixes>
          <CommonPrefixes><Prefix>release/3.1.3/</Prefix></CommonPrefixes>
        "#;
        let mut candidates: Vec<(String, Vec<u64>)> = parse_release_candidates(listing)
            .into_iter()
            .filter_map(|release| release_sort_key(&release).map(|key| (release, key)))
            .collect();
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        assert_eq!(candidates.first().map(|(release, _)| release.as_str()), Some("4.1.1"));
    }

    #[test]
    fn release_discovery_derives_templates_without_pinning_a_release() {
        let (exomes, genomes) = templates_for_release("4.1.1").expect("release");
        assert!(exomes.contains("release/4.1.1/"));
        assert!(exomes.contains("gnomad.exomes.v4.1.1"));
        assert!(genomes.contains("gnomad.genomes.v4.1.1"));
    }
}
