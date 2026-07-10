// ./src-tauri/src/research/gnomad/manifest.rs
//! Discover which gnomAD site VCF contigs exist for a release/provider (manifest-driven readiness).

use super::config::{index_urls, vcf_url};
use super::lookup::gnomad_cache_dir;
use super::types::GnomadConfig;
use super::vcf_remote::head_url_ok;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MANIFEST_TTL_SECS: i64 = 7 * 24 * 3600;

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

pub fn load_manifest(data_dir: &Path) -> Option<GnomadReleaseManifest> {
    let path = manifest_path(data_dir);
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_manifest(data_dir: &Path, manifest: &GnomadReleaseManifest) -> Result<(), String> {
    let dir = gnomad_cache_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    std::fs::write(manifest_path(data_dir), json).map_err(|e| e.to_string())
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

pub async fn get_or_build_manifest(
    cfg: &GnomadConfig,
    data_dir: &Path,
    force_refresh: bool,
) -> GnomadReleaseManifest {
    if !force_refresh
        && let Some(m) = load_manifest(data_dir)
        && manifest_matches_cfg(&m, cfg)
        && manifest_fresh(&m)
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
    }
}
