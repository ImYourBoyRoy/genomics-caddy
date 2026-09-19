// ./src-tauri/src/library.rs
/*
Purpose: Resolve, display, and relocate the personal Genomics Caddy library folder.
Responsibilities:
- Pick a default Data folder from launch layout (portable vs per-user machine install).
- Honor GENOMICS_DATA_DIR and a per-user pointer file.
- Copy or empty a user-chosen folder, open it, or erase known library files.
How to run: covered by `pnpm run cargo:test`.
Key inputs: exe location, APPIMAGE, username, optional pointer JSON.
Key outputs: resolved Data path plus LibraryStatus for the sidebar.
Operational notes: Changing the folder writes a pointer and requires relaunch.
  Never delete a chosen parent folder wholesale — only known library leaves.
*/

use crate::file_utils;
use crate::paths::{self, DataDirMode, ResolvedDataDir};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

const POINTER_VERSION: u32 = 1;
const PRODUCT_DIR: &str = "Genomics Caddy";
const POINTER_FILE: &str = "data-location.json";
const UNINSTALL_HINT_FILE: &str = "uninstall-library.txt";

const LIBRARY_FILES: &[&str] = &[
    "user_genome.db",
    "user_genome.db.enc",
    "user_genome.db-wal",
    "user_genome.db-shm",
    "GRCh37_to_GRCh38.chain.gz",
    "GRCh38_to_GRCh37.chain.gz",
];

const LIBRARY_DIRS: &[&str] = &[
    "samples",
    "references",
    "marker-packs",
    "raw_downloads",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DataLocationFile {
    #[serde(default = "default_pointer_version")]
    version: u32,
    #[serde(default)]
    users: BTreeMap<String, String>,
    #[serde(default)]
    pending_delete: BTreeMap<String, String>,
}

fn default_pointer_version() -> u32 {
    POINTER_VERSION
}

#[derive(Debug, Clone)]
pub struct DataDirInputs {
    pub env_override: Option<String>,
    pub portable_root: PathBuf,
    pub username: String,
    pub per_user_home: PathBuf,
    pub adjacent_writable: bool,
    pub machine_install: bool,
    pub pointer_contents: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryStatus {
    pub data_dir: String,
    pub default_data_dir: String,
    pub mode: DataDirMode,
    pub is_custom: bool,
    pub writable: bool,
    pub pointer_path: String,
    pub truncated_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocateMode {
    Move,
    UseEmpty,
}

impl RelocateMode {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "move" => Ok(Self::Move),
            "use" | "empty" | "use_empty" => Ok(Self::UseEmpty),
            other => Err(format!("Unknown library relocate mode '{other}'")),
        }
    }
}

pub fn sanitize_username(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    if cleaned.chars().all(|c| c == '_') || cleaned.is_empty() {
        "default".to_string()
    } else {
        cleaned
    }
}

pub fn is_machine_install_dir(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("program files")
        || lower.contains(r"\applications\")
        || lower.contains("/applications/")
        || lower.ends_with("/applications")
        || lower.ends_with(r"\applications")
        || lower.contains("/usr/bin")
        || lower.contains("/usr/lib")
        || lower.contains("/usr/local")
        || lower.contains("/opt/")
        || lower.contains(r"\windowsapps\")
}

pub fn live_appimage(exe_dir: &Path, appimage: Option<&Path>, appdir: Option<&Path>) -> Option<PathBuf> {
    let appimage = appimage?;
    let appdir = appdir?;
    if exe_dir.starts_with(appdir) {
        Some(appimage.to_path_buf())
    } else {
        None
    }
}

pub fn portable_root_from_exe(exe_dir: &Path, appimage: Option<&Path>) -> PathBuf {
    if let Some(appimage) = appimage {
        if let Some(parent) = appimage.parent() {
            if !parent.as_os_str().is_empty() {
                return parent.to_path_buf();
            }
        }
    }

    let dir = exe_dir.to_path_buf();
    if dir
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("MacOS"))
    {
        if let Some(contents) = dir.parent() {
            if contents
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("Contents"))
            {
                if let Some(bundle) = contents.parent() {
                    if bundle
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
                    {
                        if let Some(parent) = bundle.parent() {
                            return parent.to_path_buf();
                        }
                    }
                }
            }
        }
    }

    dir
}

pub fn pointer_path_for(portable_root: &Path, adjacent_writable: bool, per_user_home: &Path) -> PathBuf {
    if adjacent_writable {
        portable_root.join(POINTER_FILE)
    } else {
        per_user_home.join(POINTER_FILE)
    }
}

pub fn default_data_dir(inputs: &DataDirInputs) -> PathBuf {
    if inputs.adjacent_writable {
        inputs.portable_root.join("Data")
    } else if inputs.machine_install {
        inputs.per_user_home.join("Data")
    } else {
        inputs.portable_root.join("Data")
    }
}

pub fn resolve_from_inputs(inputs: &DataDirInputs) -> ResolvedDataDir {
    if let Some(raw) = inputs.env_override.as_deref() {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return ResolvedDataDir {
                path: PathBuf::from(trimmed),
                mode: DataDirMode::EnvOverride,
            };
        }
    }

    let username = sanitize_username(&inputs.username);
    if let Some(custom) = pointer_path_for_user(inputs.pointer_contents.as_deref(), &username) {
        return ResolvedDataDir {
            path: custom,
            mode: DataDirMode::CustomPointer,
        };
    }

    if inputs.adjacent_writable {
        return ResolvedDataDir {
            path: inputs.portable_root.join("Data"),
            mode: DataDirMode::AppLayout,
        };
    }

    if inputs.machine_install {
        return ResolvedDataDir {
            path: inputs.per_user_home.join("Data"),
            mode: DataDirMode::PerUser,
        };
    }

    ResolvedDataDir {
        path: inputs.portable_root.join("Data"),
        mode: DataDirMode::Unwritable,
    }
}

fn pointer_path_for_user(contents: Option<&str>, username: &str) -> Option<PathBuf> {
    let raw = contents?.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed: DataLocationFile = serde_json::from_str(raw).ok()?;
    parsed
        .users
        .get(username)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn read_pointer(path: &Path) -> DataLocationFile {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_pointer(path: &Path, file: &DataLocationFile) -> Result<(), String> {
    let payload = serde_json::to_vec_pretty(file)
        .map_err(|error| format!("Could not serialize library pointer: {error}"))?;
    file_utils::atomic_write(path, &payload)
}

pub fn per_user_home() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .map(|value| PathBuf::from(value).join(PRODUCT_DIR))
            .unwrap_or_else(|| PathBuf::from("C:\\GenomicsCaddyData"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library/Application Support").join(PRODUCT_DIR))
            .unwrap_or_else(|| PathBuf::from("/tmp/GenomicsCaddyData"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(|value| PathBuf::from(value).join(PRODUCT_DIR))
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|home| PathBuf::from(home).join(".local/share").join(PRODUCT_DIR))
            })
            .unwrap_or_else(|| PathBuf::from("/tmp/genomics-caddy"))
    }
}

pub fn current_username() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "default".to_string())
}

fn dir_is_writable(path: &Path) -> bool {
    if path.exists() {
        let probe = path.join(".write_test");
        if fs::write(&probe, b"").is_ok() {
            let _ = fs::remove_file(probe);
            true
        } else {
            false
        }
    } else {
        fs::create_dir_all(path).is_ok()
    }
}

pub fn gather_inputs(app: Option<&AppHandle>) -> DataDirInputs {
    let exe_dir = paths::executable_dir(app).unwrap_or_else(|| PathBuf::from("."));
    let appimage = live_appimage(
        &exe_dir,
        std::env::var_os("APPIMAGE").as_deref().map(Path::new),
        std::env::var_os("APPDIR").as_deref().map(Path::new),
    );
    let portable_root = portable_root_from_exe(&exe_dir, appimage.as_deref());
    let machine_install = is_machine_install_dir(&portable_root);
    let adjacent_writable = dir_is_writable(&portable_root);
    let per_user_home = per_user_home();
    let pointer_path = pointer_path_for(&portable_root, adjacent_writable, &per_user_home);
    let pointer_contents = fs::read_to_string(pointer_path).ok();
    DataDirInputs {
        env_override: std::env::var("GENOMICS_DATA_DIR").ok(),
        portable_root,
        username: current_username(),
        per_user_home,
        adjacent_writable,
        machine_install,
        pointer_contents,
    }
}

pub fn finish_library_startup(resolved: &Path, inputs: &DataDirInputs) {
    apply_pending_deletes(resolved, inputs);
    let _ = write_uninstall_hint(resolved, inputs);
}

fn write_uninstall_hint(data_dir: &Path, inputs: &DataDirInputs) -> Result<(), String> {
    let hint_path = pointer_path_for(&inputs.portable_root, inputs.adjacent_writable, &inputs.per_user_home)
        .with_file_name(UNINSTALL_HINT_FILE);
    let text = format!("{}\n", data_dir.display());
    #[cfg(windows)]
    {
        let mut encoded: Vec<u8> = Vec::new();
        for unit in text.encode_utf16() {
            encoded.extend_from_slice(&unit.to_le_bytes());
        }
        file_utils::atomic_write(&hint_path, &encoded)
    }
    #[cfg(not(windows))]
    {
        file_utils::atomic_write(&hint_path, text.as_bytes())
    }
}

fn apply_pending_deletes(resolved: &Path, inputs: &DataDirInputs) {
    let username = sanitize_username(&inputs.username);
    let pointer_path = pointer_path_for(&inputs.portable_root, inputs.adjacent_writable, &inputs.per_user_home);
    if !pointer_path.exists() {
        return;
    }
    let mut file = read_pointer(&pointer_path);
    let Some(old) = file.pending_delete.get(&username).cloned() else {
        return;
    };
    let old_path = PathBuf::from(old);
    if old_path == resolved {
        file.pending_delete.remove(&username);
        let _ = write_pointer(&pointer_path, &file);
        return;
    }
    if erase_library_leaves(&old_path).is_ok() {
        file.pending_delete.remove(&username);
        let _ = write_pointer(&pointer_path, &file);
    }
}

pub fn migrate_legacy_if_needed(resolved: &Path, inputs: &DataDirInputs, app: Option<&AppHandle>) {
    if paths::data_dir_has_content(resolved) {
        return;
    }
    for candidate in legacy_candidates(inputs, app) {
        if candidate == resolved {
            continue;
        }
        if !paths::data_dir_has_content(&candidate) {
            continue;
        }
        if copy_library(&candidate, resolved).is_ok() {
            break;
        }
    }
}

fn legacy_candidates(inputs: &DataDirInputs, app: Option<&AppHandle>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(handle) = app {
        if let Ok(dir) = handle.path().app_data_dir() {
            out.push(dir);
        }
    }
    out.push(inputs.per_user_home.clone());
    if let Some(roaming) = std::env::var_os("APPDATA") {
        out.push(PathBuf::from(roaming).join(PRODUCT_DIR));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        out.push(home.join(".local/share/genomics-caddy"));
        out.push(home.join(".local/share/com.dna.explorer"));
        out.push(home.join("Library/Application Support/com.dna.explorer"));
    }
    out
}

fn path_is_inside(child: &Path, parent: &Path) -> bool {
    child.starts_with(parent)
}

fn library_looks_occupied(dir: &Path) -> bool {
    paths::data_dir_has_content(dir)
}

fn copy_library(src: &Path, dest: &Path) -> Result<(), String> {
    if src == dest {
        return Ok(());
    }
    fs::create_dir_all(dest).map_err(|error| format!("Could not create {}: {error}", dest.display()))?;
    for name in LIBRARY_FILES {
        let from = src.join(name);
        if from.is_file() {
            let to = dest.join(name);
            fs::copy(&from, &to).map_err(|error| format!("Could not copy {}: {error}", from.display()))?;
        }
    }
    for name in LIBRARY_DIRS {
        let from = src.join(name);
        if from.is_dir() {
            copy_dir_recursive(&from, &dest.join(name))?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|error| format!("Could not create {}: {error}", dest.display()))?;
    for entry in fs::read_dir(src).map_err(|error| format!("Could not read {}: {error}", src.display()))? {
        let entry = entry.map_err(|error| format!("Could not read directory entry: {error}"))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not inspect {}: {error}", from.display()))?;
        if file_type.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if file_type.is_file() {
            fs::copy(&from, &to).map_err(|error| format!("Could not copy {}: {error}", from.display()))?;
        }
    }
    Ok(())
}

fn erase_library_leaves(dir: &Path) -> Result<(), String> {
    for name in LIBRARY_FILES {
        let path = dir.join(name);
        if path.exists() {
            fs::remove_file(&path).map_err(|error| format!("Could not remove {}: {error}", path.display()))?;
        }
    }
    for name in LIBRARY_DIRS {
        let path = dir.join(name);
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .map_err(|error| format!("Could not remove {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

fn webview_cache_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Some(base) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(base).join("com.dna.explorer"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            dirs.push(home.join("Library/Caches/com.dna.explorer"));
            dirs.push(home.join("Library/WebKit/com.dna.explorer"));
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
        if let Some(base) = base {
            dirs.push(base.join("com.dna.explorer"));
        }
    }
    dirs
}

fn truncate_path(path: &Path) -> String {
    let display = path.to_string_lossy();
    if display.chars().count() <= 42 {
        return display.into_owned();
    }
    let mut chars = display.chars();
    let start: String = chars.by_ref().take(12).collect();
    let rest: String = chars.collect();
    let tail: String = rest.chars().rev().take(24).collect::<String>().chars().rev().collect();
    format!("{start}…{tail}")
}

pub fn status_from_resolved(resolved: &ResolvedDataDir, inputs: &DataDirInputs) -> LibraryStatus {
    let default_dir = default_data_dir(inputs);
    let pointer = pointer_path_for(&inputs.portable_root, inputs.adjacent_writable, &inputs.per_user_home);
    LibraryStatus {
        truncated_path: truncate_path(&resolved.path),
        data_dir: resolved.path.to_string_lossy().into_owned(),
        default_data_dir: default_dir.to_string_lossy().into_owned(),
        mode: resolved.mode,
        is_custom: resolved.mode == DataDirMode::CustomPointer,
        writable: dir_is_writable(&resolved.path),
        pointer_path: pointer.to_string_lossy().into_owned(),
    }
}

pub fn relocate_library(current: &Path, dest: &Path, mode: RelocateMode, inputs: &DataDirInputs) -> Result<PathBuf, String> {
    if !dest.exists() {
        fs::create_dir_all(dest)
            .map_err(|error| format!("Could not create {}: {error}", dest.display()))?;
    }
    let dest = dest
        .canonicalize()
        .map_err(|error| format!("Could not resolve {}: {error}", dest.display()))?;
    if path_is_inside(&dest, current) && dest != current {
        return Err("Choose a folder outside the current library, not a folder inside it.".to_string());
    }
    if !dir_is_writable(&dest) {
        return Err(format!("That folder is not writable: {}", dest.display()));
    }
    match mode {
        RelocateMode::UseEmpty => {
            if library_looks_occupied(&dest) && dest != current {
                return Err("That folder already has a Genomics Caddy library. Pick an empty folder or choose Move.".to_string());
            }
        }
        RelocateMode::Move => {
            copy_library(current, &dest)?;
        }
    }
    let username = sanitize_username(&inputs.username);
    let pointer_path = pointer_path_for(&inputs.portable_root, inputs.adjacent_writable, &inputs.per_user_home);
    let mut file = read_pointer(&pointer_path);
    file.version = POINTER_VERSION;
    file.users
        .insert(username.clone(), dest.to_string_lossy().into_owned());
    if mode == RelocateMode::Move && current != dest {
        file.pending_delete
            .insert(username.clone(), current.to_string_lossy().into_owned());
    }
    write_pointer(&pointer_path, &file)?;
    write_uninstall_hint(&dest, inputs)?;
    if mode == RelocateMode::Move && current != dest && erase_library_leaves(current).is_ok() {
        file.pending_delete.remove(&username);
        write_pointer(&pointer_path, &file)?;
    }
    Ok(dest)
}

pub fn reset_pointer(inputs: &DataDirInputs) -> Result<(), String> {
    let username = sanitize_username(&inputs.username);
    let pointer_path = pointer_path_for(&inputs.portable_root, inputs.adjacent_writable, &inputs.per_user_home);
    if pointer_path.exists() {
        let mut file = read_pointer(&pointer_path);
        file.users.remove(&username);
        file.pending_delete.remove(&username);
        if file.users.is_empty() && file.pending_delete.is_empty() {
            let _ = fs::remove_file(&pointer_path);
        } else {
            write_pointer(&pointer_path, &file)?;
        }
    }
    write_uninstall_hint(&default_data_dir(inputs), inputs)
}

#[tauri::command]
pub fn get_library_status() -> Result<LibraryStatus, String> {
    let inputs = gather_inputs(None);
    let resolved = paths::resolve_data_dir_info();
    Ok(status_from_resolved(&resolved, &inputs))
}

#[tauri::command]
pub fn open_library_dir(app: AppHandle) -> Result<(), String> {
    let path = paths::resolve_data_dir();
    fs::create_dir_all(&path).map_err(|error| format!("Could not open {}: {error}", path.display()))?;
    app.opener()
        .open_path(path.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|error| format!("Could not reveal the library folder: {error}"))
}

#[tauri::command]
pub fn set_library_dir(dest: String, mode: String) -> Result<LibraryStatus, String> {
    let relocate = RelocateMode::parse(&mode)?;
    let inputs = gather_inputs(None);
    let current = paths::resolve_data_dir();
    relocate_library(&current, Path::new(&dest), relocate, &inputs)?;
    let resolved = ResolvedDataDir {
        path: PathBuf::from(&dest),
        mode: DataDirMode::CustomPointer,
    };
    Ok(status_from_resolved(&resolved, &inputs))
}

#[tauri::command]
pub fn reset_library_dir() -> Result<LibraryStatus, String> {
    let inputs = gather_inputs(None);
    reset_pointer(&inputs)?;
    let resolved = resolve_from_inputs(&DataDirInputs {
        pointer_contents: None,
        env_override: None,
        ..inputs.clone()
    });
    Ok(status_from_resolved(&resolved, &inputs))
}

#[tauri::command]
pub fn erase_library_data() -> Result<(), String> {
    let inputs = gather_inputs(None);
    let data_dir = paths::resolve_data_dir();
    erase_library_leaves(&data_dir)?;
    reset_pointer(&inputs)?;
    for cache_root in webview_cache_dirs() {
        for leaf in ["WebKitCache", "CacheStorage", "GPUCache"] {
            let path = cache_root.join(leaf);
            if path.is_dir() {
                let _ = fs::remove_dir_all(&path);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dna_tools_library_{label}_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn base_inputs(root: PathBuf) -> DataDirInputs {
        DataDirInputs {
            env_override: None,
            portable_root: root.clone(),
            username: "Ada Lovelace".to_string(),
            per_user_home: root.join("per-user"),
            adjacent_writable: true,
            machine_install: false,
            pointer_contents: None,
        }
    }

    #[test]
    fn env_override_wins_over_pointer_and_defaults() {
        let root = temp_dir("env");
        let mut inputs = base_inputs(root);
        inputs.env_override = Some("/tmp/explicit-data".to_string());
        inputs.pointer_contents = Some(r#"{"version":1,"users":{"Ada_Lovelace":"/tmp/pointer"}}"#.into());
        let resolved = resolve_from_inputs(&inputs);
        assert_eq!(resolved.path, PathBuf::from("/tmp/explicit-data"));
        assert_eq!(resolved.mode, DataDirMode::EnvOverride);
    }

    #[test]
    fn pointer_uses_sanitized_username() {
        let root = temp_dir("pointer");
        let mut inputs = base_inputs(root);
        inputs.pointer_contents = Some(
            r#"{"version":1,"users":{"Ada_Lovelace":"/media/usb/Genomes"}}"#.into(),
        );
        let resolved = resolve_from_inputs(&inputs);
        assert_eq!(resolved.path, PathBuf::from("/media/usb/Genomes"));
        assert_eq!(resolved.mode, DataDirMode::CustomPointer);
        assert_eq!(sanitize_username("Ada Lovelace"), "Ada_Lovelace");
    }

    #[test]
    fn portable_writable_layout_uses_adjacent_data() {
        let root = temp_dir("portable");
        let inputs = base_inputs(root.clone());
        let resolved = resolve_from_inputs(&inputs);
        assert_eq!(resolved.path, root.join("Data"));
        assert_eq!(resolved.mode, DataDirMode::AppLayout);
    }

    #[test]
    fn machine_install_uses_per_user_data() {
        let root = temp_dir("machine");
        let mut inputs = base_inputs(root);
        inputs.adjacent_writable = false;
        inputs.machine_install = true;
        let resolved = resolve_from_inputs(&inputs);
        assert_eq!(resolved.path, inputs.per_user_home.join("Data"));
        assert_eq!(resolved.mode, DataDirMode::PerUser);
    }

    #[test]
    fn unwritable_portable_stays_adjacent_instead_of_relocating() {
        let root = temp_dir("usb");
        let mut inputs = base_inputs(root.clone());
        inputs.adjacent_writable = false;
        inputs.machine_install = false;
        let resolved = resolve_from_inputs(&inputs);
        assert_eq!(resolved.path, root.join("Data"));
        assert_eq!(resolved.mode, DataDirMode::Unwritable);
    }

    #[test]
    fn appimage_env_is_ignored_when_the_exe_is_not_inside_the_mount() {
        let exe = PathBuf::from("/home/ada/Apps");
        let image = PathBuf::from("/home/ada/.local/bin/GenomicsCaddy.AppImage");
        let mount = PathBuf::from("/tmp/.mount_GenomiXXXX");
        assert_eq!(live_appimage(&exe, Some(&image), Some(&mount)), None);
        assert_eq!(
            live_appimage(&mount.join("usr/bin"), Some(&image), Some(&mount)),
            Some(image)
        );
    }

    #[test]
    fn appimage_portable_root_is_the_image_parent() {
        let image = PathBuf::from("/media/usb/GenomicsCaddy.AppImage");
        let exe = PathBuf::from("/tmp/.mount_GenomiXXXX/usr/bin");
        assert_eq!(
            portable_root_from_exe(&exe, Some(&image)),
            PathBuf::from("/media/usb")
        );
    }

    #[test]
    fn macos_app_portable_root_is_beside_the_bundle() {
        let exe = PathBuf::from("/Users/ada/Apps/Genomics Caddy.app/Contents/MacOS");
        assert_eq!(
            portable_root_from_exe(&exe, None),
            PathBuf::from("/Users/ada/Apps")
        );
        assert!(is_machine_install_dir(Path::new("/Applications")));
        assert!(is_machine_install_dir(Path::new(r"C:\Program Files\Genomics Caddy")));
        assert!(!is_machine_install_dir(Path::new("/media/usb/Genomics Caddy")));
    }

    #[test]
    fn pointer_file_is_per_user_when_install_dir_is_not_writable() {
        let portable = PathBuf::from(r"C:\Program Files\Genomics Caddy");
        let home = PathBuf::from(r"C:\Users\ada\AppData\Local\Genomics Caddy");
        assert_eq!(
            pointer_path_for(&portable, false, &home),
            home.join("data-location.json")
        );
        assert_eq!(
            pointer_path_for(&PathBuf::from("/media/usb"), true, &home),
            PathBuf::from("/media/usb/data-location.json")
        );
    }

    #[test]
    fn relocate_refuses_a_nested_destination() {
        let root = temp_dir("nested");
        let current = root.join("Data");
        fs::create_dir_all(&current).expect("current");
        let dest = current.join("nested");
        fs::create_dir_all(&dest).expect("nested dest");
        let inputs = base_inputs(root);
        let error = relocate_library(&current, &dest, RelocateMode::UseEmpty, &inputs)
            .expect_err("nested dest");
        assert!(error.contains("outside the current library"));
    }

    #[test]
    fn relocate_move_copies_known_leaves_and_writes_pointer() {
        let root = temp_dir("move");
        let current = root.join("Data");
        let dest = root.join("alt");
        fs::create_dir_all(current.join("samples")).expect("samples");
        fs::write(current.join("user_genome.db"), b"sqlite").expect("db");
        fs::write(current.join("samples/1.db"), b"sample").expect("sample db");
        let inputs = base_inputs(root.clone());
        let moved = relocate_library(&current, &dest, RelocateMode::Move, &inputs).expect("move");
        assert!(moved.join("user_genome.db").is_file());
        assert!(moved.join("samples/1.db").is_file());
        assert!(
            !current.join("user_genome.db").is_file(),
            "Move must delete the source library after a successful copy"
        );
        assert!(!current.join("samples/1.db").is_file());
        let pointer = fs::read_to_string(root.join("data-location.json")).expect("pointer");
        assert!(pointer.contains("Ada_Lovelace"));
        assert!(pointer.contains("alt"));
        let parsed: DataLocationFile =
            serde_json::from_str(&pointer).expect("pointer json");
        assert!(
            !parsed.pending_delete.contains_key("Ada_Lovelace"),
            "successful Move must clear pending_delete"
        );
        let hint = fs::read_to_string(root.join("uninstall-library.txt")).expect("uninstall hint");
        assert!(hint.contains("alt"));
    }

    #[test]
    fn relocate_use_empty_leaves_the_source_library() {
        let root = temp_dir("use-empty");
        let current = root.join("Data");
        let dest = root.join("fresh");
        fs::create_dir_all(&current).expect("current");
        fs::write(current.join("user_genome.db"), b"sqlite").expect("db");
        fs::create_dir_all(&dest).expect("dest");
        let inputs = base_inputs(root);
        relocate_library(&current, &dest, RelocateMode::UseEmpty, &inputs).expect("use empty");
        assert!(current.join("user_genome.db").is_file());
        assert!(!dest.join("user_genome.db").is_file());
    }

    #[test]
    fn pending_delete_erases_abandoned_source_on_startup() {
        let root = temp_dir("pending");
        let old = root.join("Data");
        let current = root.join("alt");
        fs::create_dir_all(&old).expect("old");
        fs::create_dir_all(&current).expect("current");
        fs::write(old.join("user_genome.db"), b"sqlite").expect("db");
        let pointer = DataLocationFile {
            version: 1,
            users: BTreeMap::from([(
                "Ada_Lovelace".to_string(),
                current.to_string_lossy().into_owned(),
            )]),
            pending_delete: BTreeMap::from([(
                "Ada_Lovelace".to_string(),
                old.to_string_lossy().into_owned(),
            )]),
        };
        write_pointer(&root.join("data-location.json"), &pointer).expect("pointer");
        let inputs = base_inputs(root);
        apply_pending_deletes(&current, &inputs);
        assert!(!old.join("user_genome.db").is_file());
    }
}
