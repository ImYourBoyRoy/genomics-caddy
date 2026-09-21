// ./src-tauri/src/portable_update.rs
//! Replace a Windows portable copy in place instead of launching the NSIS installer.
//!
//! How to run: invoked from the signed-update banner when `get_update_channel`
//! returns `portable`. Downloads the signed `GenomicsCaddy-portable-windows.zip`,
//! verifies the Tauri minisign signature, extracts next to the current exe
//! (never touching `Data/`), then exits so a helper can swap the running file.
//!
//! Inputs: GitHub release tag matching the updater `check()` version; pubkey
//! from `tauri.conf.json`. Outputs: replaced `DNA-Tools.exe` and sibling DLLs.
//! Closing the window while an update is in progress is blocked so the process
//! cannot sit hidden behind a cancelled NSIS wizard.

use crate::library::{dir_is_writable, is_machine_install_dir};
use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use serde::Serialize;
use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

const PORTABLE_ZIP_NAME: &str = "GenomicsCaddy-portable-windows.zip";
const TAURI_CONF: &str = include_str!("../tauri.conf.json");
static UPDATE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    Installer,
    Portable,
}

#[derive(Clone, Serialize)]
struct PortableUpdateProgress {
    percentage: u32,
    status: String,
}

pub fn update_in_progress() -> bool {
    UPDATE_IN_PROGRESS.load(Ordering::SeqCst)
}

pub fn set_in_progress(active: bool) {
    UPDATE_IN_PROGRESS.store(active, Ordering::SeqCst);
}

pub fn has_uninstaller(exe_dir: &Path) -> bool {
    const NAMES: &[&str] = &[
        "uninstall.exe",
        "Uninstall.exe",
        "unins000.exe",
        "Unins000.exe",
    ];
    NAMES.iter().any(|name| exe_dir.join(name).is_file())
}

pub fn is_windows_portable_layout(exe_dir: &Path) -> bool {
    if is_machine_install_dir(exe_dir) {
        return false;
    }
    if has_uninstaller(exe_dir) {
        return false;
    }
    dir_is_writable(exe_dir)
}

pub fn current_update_channel() -> UpdateChannel {
    let layout_portable = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(is_windows_portable_layout))
        .unwrap_or(false);
    if cfg!(all(windows, not(debug_assertions))) && layout_portable {
        return UpdateChannel::Portable;
    }
    UpdateChannel::Installer
}

fn updater_pubkey() -> Result<String, String> {
    let conf: serde_json::Value =
        serde_json::from_str(TAURI_CONF).map_err(|error| format!("tauri.conf.json: {error}"))?;
    conf.pointer("/plugins/updater/pubkey")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "updater pubkey missing from tauri.conf.json".to_string())
}

fn github_download_origin() -> Result<String, String> {
    let conf: serde_json::Value =
        serde_json::from_str(TAURI_CONF).map_err(|error| format!("tauri.conf.json: {error}"))?;
    let endpoint = conf
        .pointer("/plugins/updater/endpoints/0")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "updater endpoint missing from tauri.conf.json".to_string())?;
    let marker = "/releases/";
    let idx = endpoint
        .find(marker)
        .ok_or_else(|| "updater endpoint is not a GitHub Releases URL".to_string())?;
    Ok(endpoint[..idx].trim_end_matches('/').to_string())
}

pub fn portable_zip_url(version: &str) -> Result<String, String> {
    let origin = github_download_origin()?;
    let tag = if version.trim().starts_with('v') {
        version.trim().to_string()
    } else {
        format!("v{}", version.trim())
    };
    if tag.len() < 2 {
        return Err("update version is empty".to_string());
    }
    Ok(format!(
        "{origin}/releases/download/{tag}/{PORTABLE_ZIP_NAME}"
    ))
}

pub fn should_extract_member(name: &str) -> bool {
    let normalized = name.replace('\\', "/");
    if normalized.contains("..") {
        return false;
    }
    let parts: Vec<&str> = normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    if parts.is_empty() {
        return false;
    }
    !parts.contains(&"Data")
}

fn decode_minisign_blob(raw: &str) -> Result<String, String> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .map_err(|error| format!("signature is not base64: {error}"))?;
    String::from_utf8(decoded).map_err(|_| "signature is not UTF-8".to_string())
}

pub fn verify_portable_signature(bytes: &[u8], signature_file: &str, pubkey_b64: &str) -> Result<(), String> {
    let pubkey_text = decode_minisign_blob(pubkey_b64)?;
    let public_key =
        PublicKey::decode(&pubkey_text).map_err(|error| format!("updater pubkey: {error}"))?;
    let signature_text = decode_minisign_blob(signature_file)
        .unwrap_or_else(|_| signature_file.trim().to_string());
    let signature = Signature::decode(&signature_text)
        .map_err(|error| format!("portable zip signature: {error}"))?;
    public_key
        .verify(bytes, &signature, true)
        .map_err(|error| format!("portable zip failed signature check: {error}"))
}

fn emit_progress(app: &AppHandle, percentage: u32, status: &str) {
    let _ = app.emit(
        "portable-update-progress",
        PortableUpdateProgress {
            percentage,
            status: status.to_string(),
        },
    );
}

fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("GenomicsCaddy-portable-updater")
        .redirect(reqwest::redirect::Policy::limited(8))
        .build()
        .map_err(|error| format!("HTTP client: {error}"))?;
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("download failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("download failed: HTTP {}", response.status()));
    }
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|error| format!("download failed: {error}"))
}

fn extract_portable_zip(zip_bytes: &[u8], dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|error| format!("staging dir: {error}"))?;
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|error| format!("portable zip is unreadable: {error}"))?;
    let mut wrote = 0usize;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|error| format!("portable zip entry: {error}"))?;
        let name = file
            .name()
            .map_err(|error| format!("portable zip entry: {error}"))?
            .into_owned();
        if file.is_dir() || !should_extract_member(&name) {
            continue;
        }
        let relative = PathBuf::from(name.replace('\\', "/"));
        let out_path = dest.join(&relative);
        if !out_path.starts_with(dest) {
            return Err("portable zip contained an unsafe path".to_string());
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("extract dir: {error}"))?;
        }
        let mut out = File::create(&out_path).map_err(|error| format!("extract file: {error}"))?;
        copy(&mut file, &mut out).map_err(|error| format!("extract file: {error}"))?;
        wrote += 1;
    }
    if !dest.join("DNA-Tools.exe").is_file() {
        return Err("signed portable zip is missing DNA-Tools.exe".to_string());
    }
    if wrote == 0 {
        return Err("signed portable zip had no files to install".to_string());
    }
    Ok(())
}

#[cfg(windows)]
fn spawn_replacer(payload: &Path, dest: &Path, exe_name: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    let work = payload
        .parent()
        .ok_or_else(|| "portable update staging is missing".to_string())?;
    let script = work.join("apply.ps1");
    let pid = std::process::id();
    let script_body = r#"param($WaitPid, $Payload, $Dest, $ExeName)
while (Get-Process -Id $WaitPid -ErrorAction SilentlyContinue) { Start-Sleep -Seconds 1 }
Get-ChildItem -LiteralPath $Payload -File | ForEach-Object {
  Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $Dest $_.Name) -Force
}
Get-ChildItem -LiteralPath $Payload -Directory | Where-Object { $_.Name -ne 'Data' } | ForEach-Object {
  Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $Dest $_.Name) -Recurse -Force
}
Start-Process -FilePath (Join-Path $Dest $ExeName) -WorkingDirectory $Dest
Remove-Item -LiteralPath (Split-Path $Payload -Parent) -Recurse -Force -ErrorAction SilentlyContinue
"#;
    fs::write(&script, script_body).map_err(|error| format!("update helper: {error}"))?;
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script.to_string_lossy(),
            &pid.to_string(),
            &payload.to_string_lossy(),
            &dest.to_string_lossy(),
            exe_name,
        ])
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .map_err(|error| format!("could not start portable replace helper: {error}"))?;
    Ok(())
}

#[cfg(not(windows))]
fn spawn_replacer(_payload: &Path, _dest: &Path, _exe_name: &str) -> Result<(), String> {
    Err("in-place portable replace is only used on Windows zip copies".to_string())
}

fn apply_portable_update_inner(app: &AppHandle, version: &str) -> Result<(), String> {
    if current_update_channel() != UpdateChannel::Portable {
        return Err(
            "This copy is an installer layout. Use the signed NSIS update, not the portable zip."
                .to_string(),
        );
    }
    let exe = std::env::current_exe().map_err(|error| format!("current exe: {error}"))?;
    let dest = exe
        .parent()
        .ok_or_else(|| "current exe has no parent folder".to_string())?
        .to_path_buf();
    let exe_name = exe
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("DNA-Tools.exe")
        .to_string();
    let zip_url = portable_zip_url(version)?;
    let sig_url = format!("{zip_url}.sig");
    emit_progress(app, 1, "Downloading signed portable zip");
    let zip_bytes = http_get_bytes(&zip_url)?;
    emit_progress(app, 55, "Verifying signature");
    let signature = String::from_utf8(http_get_bytes(&sig_url)?)
        .map_err(|_| "portable zip signature is not text".to_string())?;
    let pubkey = updater_pubkey()?;
    verify_portable_signature(&zip_bytes, &signature, &pubkey)?;
    emit_progress(app, 70, "Unpacking beside this copy");
    let work = std::env::temp_dir().join(format!(
        "genomics-caddy-portable-update-{}",
        std::process::id()
    ));
    if work.exists() {
        fs::remove_dir_all(&work).map_err(|error| format!("clear staging: {error}"))?;
    }
    let payload = work.join("payload");
    extract_portable_zip(&zip_bytes, &payload)?;
    emit_progress(app, 90, "Scheduling replace after exit");
    spawn_replacer(&payload, &dest, &exe_name)?;
    emit_progress(app, 100, "Restarting this copy");
    Ok(())
}

#[tauri::command]
pub fn get_update_channel() -> UpdateChannel {
    current_update_channel()
}

#[tauri::command]
pub fn set_update_in_progress(active: bool) {
    set_in_progress(active);
}

#[tauri::command]
pub async fn apply_portable_update(app: AppHandle, version: String) -> Result<(), String> {
    if version.trim().is_empty() {
        return Err("update version is empty".to_string());
    }
    set_in_progress(true);
    let app_for_work = app.clone();
    let version_for_work = version.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        apply_portable_update_inner(&app_for_work, &version_for_work)
    })
    .await
    .map_err(|error| format!("portable update task: {error}"))?;
    if let Err(error) = result {
        set_in_progress(false);
        return Err(error);
    }
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dna_tools_portable_update_{label}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn debug_builds_use_the_installer_channel() {
        assert_eq!(current_update_channel(), UpdateChannel::Installer);
    }

    #[test]
    fn nsis_uninstaller_is_not_portable() {
        let dir = temp_dir("nsis");
        fs::write(dir.join("DNA-Tools.exe"), b"exe").unwrap();
        fs::write(dir.join("uninstall.exe"), b"un").unwrap();
        assert!(has_uninstaller(&dir));
        assert!(!is_windows_portable_layout(&dir));
    }

    #[test]
    fn zip_layout_without_uninstaller_is_portable_when_writable() {
        let dir = temp_dir("zip");
        fs::write(dir.join("DNA-Tools.exe"), b"exe").unwrap();
        fs::create_dir_all(dir.join("Data")).unwrap();
        fs::write(dir.join("Data").join(".keep"), b"").unwrap();
        assert!(!has_uninstaller(&dir));
        assert!(is_windows_portable_layout(&dir));
    }

    #[test]
    fn program_files_is_never_portable() {
        assert!(!is_windows_portable_layout(Path::new(
            r"C:\Program Files\Genomics Caddy"
        )));
    }

    #[test]
    fn extract_skips_data_and_zip_slip() {
        assert!(should_extract_member("DNA-Tools.exe"));
        assert!(should_extract_member("WebView2Loader.dll"));
        assert!(should_extract_member("resources/icon.png"));
        assert!(!should_extract_member("Data/.keep"));
        assert!(!should_extract_member("Data/profiles/x.db"));
        assert!(!should_extract_member("../DNA-Tools.exe"));
        assert!(!should_extract_member(""));
    }

    #[test]
    fn portable_zip_url_uses_github_tag() {
        let url = portable_zip_url("0.2.3").expect("url");
        assert!(url.ends_with("/releases/download/v0.2.3/GenomicsCaddy-portable-windows.zip"));
        assert!(url.starts_with("https://github.com/"));
        let tagged = portable_zip_url("v0.2.3").expect("tagged");
        assert_eq!(url, tagged);
    }

    #[test]
    fn extract_writes_exe_and_leaves_data_alone() {
        let dest = temp_dir("extract");
        fs::create_dir_all(dest.join("Data")).unwrap();
        fs::write(dest.join("Data").join("keep-me.txt"), b"user").unwrap();

        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            zip.start_file("DNA-Tools.exe", options).unwrap();
            zip.write_all(b"new-exe").unwrap();
            zip.start_file("Data/.keep", options).unwrap();
            zip.write_all(b"should-not-land").unwrap();
            zip.finish().unwrap();
        }

        extract_portable_zip(&buffer, &dest).expect("extract");
        assert_eq!(fs::read(dest.join("DNA-Tools.exe")).unwrap(), b"new-exe");
        assert_eq!(fs::read(dest.join("Data").join("keep-me.txt")).unwrap(), b"user");
        assert!(!dest.join("Data").join(".keep").exists());
    }
}
