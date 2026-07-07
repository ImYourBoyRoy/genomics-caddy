// ./src-tauri/src/paths.rs
/*
Module Docstring:
Purpose: Resolve portable application data paths for Genomics Caddy.
Responsibilities:
- Locate the data directory from GENOMICS_DATA_DIR, portable exe layout, dev project root, or OS app data.
- Ensure expected subfolders exist (references, marker-packs, raw_downloads).
Key Inputs: Environment variables, optional Tauri AppHandle, executable path.
Key Outputs: Absolute PathBuf values for db, chain file, and reference downloads.
Operational Notes:
- Persistent data lives at `<project-root>/App/Data` (portable layout).
- Legacy dev data at `<project-root>/data/` is used when App/Data is empty but data/ has content.
- Override anytime with GENOMICS_DATA_DIR.
- Built binaries are staged to `<project-root>/App/` by scripts/purge_and_build.ps1.
*/

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataDirMode {
    EnvOverride,
    AppLayout,
    LegacyProject,
}

#[derive(Debug, Clone)]
pub struct ResolvedDataDir {
    pub path: PathBuf,
    pub mode: DataDirMode,
}

static RESOLVED_DATA_DIR: OnceLock<ResolvedDataDir> = OnceLock::new();

/// Called once from Tauri setup so all later `resolve_data_dir()` calls agree with the running app.
pub fn initialize_data_dir(app: &AppHandle) -> PathBuf {
    let resolved = compute_data_dir(Some(app));
    let _ = RESOLVED_DATA_DIR.set(resolved.clone());
    ensure_data_layout(&resolved.path).ok();
    resolved.path.clone()
}

/// Root data directory for SQLite, liftover chain, and downloaded references.
pub fn resolve_data_dir() -> PathBuf {
    RESOLVED_DATA_DIR
        .get()
        .map(|r| r.path.clone())
        .unwrap_or_else(|| compute_data_dir(None).path)
}

pub fn resolve_data_dir_mode() -> DataDirMode {
    RESOLVED_DATA_DIR
        .get()
        .map(|r| r.mode)
        .unwrap_or_else(|| compute_data_dir(None).mode)
}

pub fn resolve_data_dir_info() -> ResolvedDataDir {
    RESOLVED_DATA_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| compute_data_dir(None))
}

fn compute_data_dir(app: Option<&AppHandle>) -> ResolvedDataDir {
    if let Ok(raw) = std::env::var("GENOMICS_DATA_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return ResolvedDataDir {
                path: PathBuf::from(trimmed),
                mode: DataDirMode::EnvOverride,
            };
        }
    }

    let project_root = resolve_project_root(app);
    let app_data = app_data_dir(&project_root);
    let legacy_data = project_root.join("data");

    if data_dir_has_content(&app_data) || !data_dir_has_content(&legacy_data) {
        return ResolvedDataDir {
            path: app_data,
            mode: DataDirMode::AppLayout,
        };
    }

    ResolvedDataDir {
        path: legacy_data,
        mode: DataDirMode::LegacyProject,
    }
}

/// Portable application folder (exe + sidecar files).
pub fn app_layout_dir(project_root: &Path) -> PathBuf {
    project_root.join("App")
}

/// Persistent genome/reference storage beside the staged application.
pub fn app_data_dir(project_root: &Path) -> PathBuf {
    app_layout_dir(project_root).join("Data")
}

pub fn resolve_project_root(app: Option<&AppHandle>) -> PathBuf {
    if let Ok(raw) = std::env::var("GENOMICS_APP_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    if let Some(exe_dir) = executable_dir(app) {
        if exe_dir
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("App"))
        {
            return exe_dir
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or(exe_dir);
        }

        let mut dir = exe_dir.clone();
        for _ in 0..8 {
            if dir.join("package.json").exists() || dir.join("App").join("Data").exists() {
                return dir;
            }
            if !dir.pop() {
                break;
            }
        }
    }

    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR")
        && let Some(project_root) = PathBuf::from(manifest).parent()
    {
        return project_root.to_path_buf();
    }

    PathBuf::from(".")
}

fn data_dir_has_content(dir: &Path) -> bool {
    dir.join("user_genome.db").exists()
        || dir.join("user_genome.db.enc").exists()
        || dir.join("references").exists()
        || dir.join("raw_downloads").exists()
        || dir.join("GRCh37_to_GRCh38.chain.gz").exists()
}

fn executable_dir(app: Option<&AppHandle>) -> Option<PathBuf> {
    if let Some(app) = app {
        app.path().executable_dir().ok()
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
    }
}

pub fn ensure_data_layout(data_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(data_dir)?;
    fs::create_dir_all(references_dir(data_dir))?;
    fs::create_dir_all(marker_packs_dir(data_dir))?;
    fs::create_dir_all(raw_downloads_dir(data_dir))?;
    for sub in ["clinvar", "pharmgkb", "clingen", "mane", "dbsnp", "gwas"] {
        fs::create_dir_all(raw_downloads_dir(data_dir).join(sub))?;
    }
    Ok(())
}

/// Downloaded source files before SQLite import.
pub fn raw_downloads_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("raw_downloads")
}

pub fn offline_asset_path(data_dir: &Path, category: &str, filename: &str) -> PathBuf {
    raw_downloads_dir(data_dir).join(category).join(filename)
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
