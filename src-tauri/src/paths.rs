// ./src-tauri/src/paths.rs
/*
Module Docstring:
Purpose: Resolve portable application data paths under ./data (not AppData Roaming).
Responsibilities:
- Locate the data directory from GENOMICS_DATA_DIR, project root, or executable dir.
- Ensure expected subfolders exist (references, marker-packs).
Key Inputs: Environment variables, CARGO_MANIFEST_DIR, current executable path.
Key Outputs: Absolute PathBuf values for db, chain file, and reference downloads.
Operational Notes: All persistent user data should live under resolve_data_dir().
*/

use std::fs;
use std::path::{Path, PathBuf};

/// Root data directory for SQLite, liftover chain, and downloaded references.
pub fn resolve_data_dir() -> PathBuf {
    if let Ok(raw) = std::env::var("GENOMICS_DATA_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR")
        && let Some(project_root) = PathBuf::from(manifest).parent() {
            return project_root.join("data");
        }

    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent() {
            return parent.join("data");
        }

    PathBuf::from("./data")
}

pub fn ensure_data_layout(data_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(data_dir)?;
    fs::create_dir_all(references_dir(data_dir))?;
    fs::create_dir_all(marker_packs_dir(data_dir))?;
    Ok(())
}

pub fn db_path(data_dir: &Path) -> PathBuf {
    data_dir.join("user_genome.db")
}

pub fn chain_path(data_dir: &Path) -> PathBuf {
    data_dir.join("GRCh37_to_GRCh38.chain.gz")
}

pub fn references_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("references")
}

pub fn marker_packs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("marker-packs")
}

pub fn gwas_catalog_file(data_dir: &Path) -> PathBuf {
    references_dir(data_dir).join("gwas-catalog-associations_ontology-annotated.tsv")
}

pub fn gwas_catalog_gz_file(data_dir: &Path) -> PathBuf {
    references_dir(data_dir).join("gwas-catalog-associations_ontology-annotated.tsv.gz")
}

pub fn gwas_catalog_zip_file(data_dir: &Path) -> PathBuf {
    references_dir(data_dir).join("gwas-catalog-associations_ontology-annotated-full.zip")
}
