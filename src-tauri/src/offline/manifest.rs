// ./src-tauri/src/offline/manifest.rs
//! Canonical offline asset catalog (Tiers 0–2) with URLs and laptop size budgets.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineAssetId {
    GwasCatalog,
    LiftoverChain,
    GnomadIndexManifest,
    ClinvarVariantSummary,
    PharmgkbClinicalVariants,
    PharmgkbGenes,
    ClingenGeneValidity,
    ManeSelectSummary,
    DbsnpMergedJson,
    DbsnpWithdrawnJson,
    Tier2VariantLocus,
}

impl OfflineAssetId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GwasCatalog => "gwas_catalog",
            Self::LiftoverChain => "liftover_chain",
            Self::GnomadIndexManifest => "gnomad_index_manifest",
            Self::ClinvarVariantSummary => "clinvar_variant_summary",
            Self::PharmgkbClinicalVariants => "pharmgkb_clinical_variants",
            Self::PharmgkbGenes => "pharmgkb_genes",
            Self::ClingenGeneValidity => "clingen_gene_validity",
            Self::ManeSelectSummary => "mane_select_summary",
            Self::DbsnpMergedJson => "dbsnp_merged_json",
            Self::DbsnpWithdrawnJson => "dbsnp_withdrawn_json",
            Self::Tier2VariantLocus => "tier2_variant_locus",
        }
    }

    pub fn from_str_id(s: &str) -> Option<Self> {
        match s {
            "gwas_catalog" => Some(Self::GwasCatalog),
            "liftover_chain" => Some(Self::LiftoverChain),
            "gnomad_index_manifest" => Some(Self::GnomadIndexManifest),
            "clinvar_variant_summary" => Some(Self::ClinvarVariantSummary),
            "pharmgkb_clinical_variants" => Some(Self::PharmgkbClinicalVariants),
            "pharmgkb_genes" => Some(Self::PharmgkbGenes),
            "clingen_gene_validity" => Some(Self::ClingenGeneValidity),
            "mane_select_summary" => Some(Self::ManeSelectSummary),
            "dbsnp_merged_json" => Some(Self::DbsnpMergedJson),
            "dbsnp_withdrawn_json" => Some(Self::DbsnpWithdrawnJson),
            "tier2_variant_locus" => Some(Self::Tier2VariantLocus),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    RemoteFile,
    /// Built locally from SQLite genotypes (no remote download).
    Derived,
    /// Uses existing app sync path (GWAS).
    GwasSync,
}

#[derive(Debug, Clone)]
pub struct OfflineAssetDef {
    pub id: OfflineAssetId,
    pub tier: u8,
    pub label: &'static str,
    pub url: Option<&'static str>,
    pub category: &'static str,
    pub filename: &'static str,
    /// Hard ceiling in bytes — download is aborted if exceeded. Set generously.
    pub max_bytes: u64,
    /// Human-readable expected size for UI display only.
    pub display_size: &'static str,
    pub kind: AssetKind,
}

pub fn all_assets() -> &'static [OfflineAssetDef] {
    &[
        OfflineAssetDef {
            id: OfflineAssetId::GwasCatalog,
            tier: 0,
            label: "GWAS Catalog (ontology)",
            url: Some(
                "https://ftp.ebi.ac.uk/pub/databases/gwas/releases/latest/gwas-catalog-associations_ontology-annotated-full.zip",
            ),
            category: "gwas",
            filename: "gwas-catalog-associations_ontology-annotated-full.zip",
            max_bytes: 900 * 1024 * 1024,
            display_size: "~67 MB",
            kind: AssetKind::GwasSync,
        },
        OfflineAssetDef {
            id: OfflineAssetId::LiftoverChain,
            tier: 0,
            label: "UCSC GRCh37\u{2192}GRCh38 chain",
            url: Some(
                "https://hgdownload.soe.ucsc.edu/goldenPath/hg19/liftOver/hg19ToHg38.over.chain.gz",
            ),
            category: "reference",
            filename: "GRCh37_to_GRCh38.chain.gz",
            max_bytes: 50 * 1024 * 1024,
            display_size: "~222 KB",
            kind: AssetKind::RemoteFile,
        },
        OfflineAssetDef {
            id: OfflineAssetId::GnomadIndexManifest,
            tier: 0,
            label: "gnomAD release manifest",
            url: None,
            category: "gnomad",
            filename: "release_manifest.json",
            max_bytes: 1024 * 1024,
            display_size: "<1 MB (derived)",
            kind: AssetKind::Derived,
        },
        OfflineAssetDef {
            id: OfflineAssetId::ClinvarVariantSummary,
            tier: 1,
            label: "ClinVar variant summary",
            url: Some(
                "https://ftp.ncbi.nlm.nih.gov/pub/clinvar/tab_delimited/variant_summary.txt.gz",
            ),
            category: "clinvar",
            filename: "variant_summary.txt.gz",
            // The compressed .gz file is typically 250–500 MB; set ceiling at 4 GB to never refuse.
            max_bytes: 4 * 1024 * 1024 * 1024,
            display_size: "~420 MB",
            kind: AssetKind::RemoteFile,
        },
        OfflineAssetDef {
            id: OfflineAssetId::PharmgkbClinicalVariants,
            tier: 1,
            label: "PharmGKB clinical variants",
            url: Some("https://api.pharmgkb.org/v1/download/file/data/clinicalVariants.zip"),
            category: "pharmgkb",
            filename: "clinicalVariants.zip",
            max_bytes: 200 * 1024 * 1024,
            display_size: "~73 KB",
            kind: AssetKind::RemoteFile,
        },
        OfflineAssetDef {
            id: OfflineAssetId::PharmgkbGenes,
            tier: 1,
            label: "PharmGKB genes",
            url: Some("https://api.pharmgkb.org/v1/download/file/data/genes.zip"),
            category: "pharmgkb",
            filename: "genes.zip",
            max_bytes: 100 * 1024 * 1024,
            display_size: "~2.8 MB",
            kind: AssetKind::RemoteFile,
        },
        OfflineAssetDef {
            id: OfflineAssetId::ClingenGeneValidity,
            tier: 1,
            label: "ClinGen gene validity",
            url: Some("https://search.clinicalgenome.org/kb/gene-validity/download"),
            category: "clingen",
            filename: "gene-validity.csv",
            max_bytes: 100 * 1024 * 1024,
            display_size: "~1.1 MB",
            kind: AssetKind::RemoteFile,
        },
        OfflineAssetDef {
            id: OfflineAssetId::ManeSelectSummary,
            tier: 1,
            label: "MANE Select summary (GRCh38)",
            url: Some(
                "https://ftp.ncbi.nlm.nih.gov/refseq/MANE/MANE_human/release_1.5/MANE.GRCh38.v1.5.summary.txt.gz",
            ),
            category: "mane",
            filename: "MANE.GRCh38.v1.5.summary.txt.gz",
            max_bytes: 50 * 1024 * 1024,
            display_size: "~1.1 MB",
            kind: AssetKind::RemoteFile,
        },
        OfflineAssetDef {
            id: OfflineAssetId::DbsnpMergedJson,
            tier: 2,
            label: "dbSNP merged rsIDs",
            url: Some("https://ftp.ncbi.nih.gov/snp/latest_release/JSON/refsnp-merged.json.bz2"),
            category: "dbsnp",
            filename: "refsnp-merged.json.bz2",
            // Compressed .bz2 file can be 1–3 GB; set ceiling at 8 GB to never refuse.
            max_bytes: 8 * 1024 * 1024 * 1024,
            display_size: "~776 MB",
            kind: AssetKind::RemoteFile,
        },
        OfflineAssetDef {
            id: OfflineAssetId::DbsnpWithdrawnJson,
            tier: 2,
            label: "dbSNP withdrawn rsIDs",
            url: Some("https://ftp.ncbi.nih.gov/snp/latest_release/JSON/refsnp-withdrawn.json.bz2"),
            category: "dbsnp",
            filename: "refsnp-withdrawn.json.bz2",
            max_bytes: 1024 * 1024 * 1024,
            display_size: "~70 MB",
            kind: AssetKind::RemoteFile,
        },
        OfflineAssetDef {
            id: OfflineAssetId::Tier2VariantLocus,
            tier: 2,
            label: "Variant locus index (from genotypes)",
            url: None,
            category: "tier2",
            filename: "variant_locus.sqlite",
            max_bytes: 0,
            display_size: "derived",
            kind: AssetKind::Derived,
        },
    ]
}

pub fn asset_def(id: OfflineAssetId) -> Option<&'static OfflineAssetDef> {
    all_assets().iter().find(|a| a.id == id)
}

pub fn assets_for_tier(tier: u8) -> Vec<&'static OfflineAssetDef> {
    all_assets().iter().filter(|a| a.tier == tier).collect()
}

pub fn local_path(
    data_dir: &Path,
    custom_dir: Option<&Path>,
    def: &OfflineAssetDef,
) -> std::path::PathBuf {
    let base = custom_dir.unwrap_or(data_dir);
    match def.id {
        OfflineAssetId::GwasCatalog => base.join("references").join(def.filename),
        OfflineAssetId::LiftoverChain => base.join(def.filename),
        OfflineAssetId::GnomadIndexManifest => base.join("gnomad_indexes").join(def.filename),
        OfflineAssetId::Tier2VariantLocus => base.join("tier2").join("variant_locus.meta"),
        _ => base
            .join("raw_downloads")
            .join(def.category)
            .join(def.filename),
    }
}

pub fn tier_budget_bytes(tier: u8) -> u64 {
    match tier {
        0 => 2 * 1024 * 1024 * 1024,  // 2 GB
        1 => 6 * 1024 * 1024 * 1024, // 6 GB (ClinVar compressed can be 500MB+, but uncompressed is larger)
        2 => 15 * 1024 * 1024 * 1024, // 15 GB (dbSNP merged + withdrawn can be several GBs)
        _ => 0,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OfflineAssetStatus {
    pub asset_id: String,
    pub tier: u8,
    pub label: String,
    pub local_present: bool,
    pub local_bytes: u64,
    pub row_count: u64,
    pub synced_at: Option<i64>,
    pub update_available: bool,
    pub remote_content_length: Option<u64>,
    pub version_label: Option<String>,
    pub message: String,
    pub display_size: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OfflineTierStatus {
    pub tier: u8,
    pub assets: Vec<OfflineAssetStatus>,
    pub ready: bool,
    pub updates_available: u32,
}
