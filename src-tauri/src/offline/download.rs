// ./src-tauri/src/offline/download.rs
//! HTTP download helpers — streaming to disk with progress events and atomic completion.

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct RemoteHead {
    pub content_length: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

pub async fn head_remote(url: &str) -> Result<RemoteHead, String> {
    let client = reqwest::Client::builder()
        // Sync-time probe only — keep this short so a dead mirror cannot stall the UI.
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(3))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .head(url)
        .send()
        .await
        .map_err(|e| format!("HEAD failed for {url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("HEAD {} returned HTTP {}", url, response.status()));
    }
    let mut content_length = response.content_length();
    // Some CDNs omit Content-Length on HEAD; try a 1-byte ranged GET as fallback.
    if content_length.is_none() || content_length == Some(0) {
        if let Ok(probe) = client
            .get(url)
            .header("Range", "bytes=0-0")
            .send()
            .await
        {
            if let Some(cr) = probe.headers().get(reqwest::header::CONTENT_RANGE)
                && let Ok(s) = cr.to_str()
                && let Some(total) = s.split('/').nth(1)
                && let Ok(n) = total.trim().parse::<u64>()
                && n > 0
            {
                content_length = Some(n);
            } else if let Some(n) = probe.content_length().filter(|n| *n > 0) {
                content_length = Some(n);
            }
        }
    }
    Ok(RemoteHead {
        content_length,
        etag: response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(String::from),
        last_modified: response
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(String::from),
    })
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// If a sibling `.md5` or `.sha256` file exists next to `path`, verify the payload matches.
/// Missing sidecars are OK (many NCBI/EBI mirrors omit them); a present mismatch fails hard.
pub fn verify_local_hash_sidecar(path: &Path) -> Result<(), String> {
    use std::io::Read;
    use std::path::PathBuf;

    let sha_sidecar: PathBuf = PathBuf::from(format!("{}.sha256", path.display()));
    let md5_sidecar: PathBuf = PathBuf::from(format!("{}.md5", path.display()));

    if sha_sidecar.is_file() {
        let text = std::fs::read_to_string(&sha_sidecar).map_err(|e| e.to_string())?;
        let expected = text
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if expected.len() == 64 {
            let actual = sha256_file(path)?;
            if actual != expected {
                return Err(format!(
                    "SHA-256 mismatch for {}: local {actual} != expected {expected}",
                    path.display()
                ));
            }
        }
        return Ok(());
    }

    if md5_sidecar.is_file() {
        let text = std::fs::read_to_string(&md5_sidecar).map_err(|e| e.to_string())?;
        let expected = text
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if expected.len() == 32 {
            let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut context = md5::Context::new();
            let mut buf = [0u8; 65536];
            loop {
                let n = file.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                context.consume(&buf[..n]);
            }
            let local_hash = format!("{:x}", context.finalize());
            if local_hash != expected {
                return Err(format!(
                    "MD5 mismatch for {}: local {local_hash} != expected {expected}",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

/// Download `url` to `dest`, streaming in chunks.
///
/// Writes to a `.part` temp file during download and atomically renames to
/// `dest` on success — a partial or failed download never leaves a corrupt file.
///
/// `on_progress(bytes_written, total_bytes)` is called every ~250 ms.
/// `total_bytes` is 0 when the server did not send Content-Length.
///
/// `max_bytes` is an absolute ceiling (enforced during streaming, not from
/// Content-Length alone). Pass 0 to disable.
pub async fn download_to_path<F>(
    url: &str,
    dest: &Path,
    max_bytes: u64,
    on_progress: F,
) -> Result<(u64, RemoteHead), String>
where
    F: Fn(u64, u64) + Send + Sync,
{
    // HEAD — fetch ETag/Last-Modified for update detection; size is advisory only.
    let head = head_remote(url).await?;
    let total_bytes = head.content_length.unwrap_or(0);

    // Ensure destination parent exists.
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
    }

    // Write to a .part temp file so the dest path is never partially written.
    let part_path = {
        let mut p = dest.to_path_buf();
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{e}.part"))
            .unwrap_or_else(|| "part".into());
        p.set_extension(ext);
        p
    };

    let mut initial_bytes = 0u64;
    if part_path.is_file() {
        if let Ok(m) = std::fs::metadata(&part_path) {
            initial_bytes = m.len();
        }
    }

    let client = reqwest::Client::builder()
        // 4-hour timeout for multi-GB files (dbSNP) on slow connections.
        .timeout(Duration::from_secs(14_400))
        .build()
        .map_err(|e| e.to_string())?;

    let mut req_builder = client.get(url);
    if initial_bytes > 0 && initial_bytes < total_bytes {
        req_builder = req_builder.header("Range", format!("bytes={}-", initial_bytes));
    } else {
        initial_bytes = 0; // reset if invalid or full
    }

    let response = req_builder
        .send()
        .await
        .map_err(|e| format!("GET failed for {url}: {e}"))?;

    let status = response.status();
    let is_partial = status == reqwest::StatusCode::PARTIAL_CONTENT;

    if !status.is_success() {
        return Err(format!("GET {} returned HTTP {}", url, status));
    }

    let mut file = if is_partial && initial_bytes > 0 {
        std::fs::OpenOptions::new()
            .append(true)
            .open(&part_path)
            .map_err(|e| {
                format!(
                    "Failed to open temp file for append {}: {e}",
                    part_path.display()
                )
            })?
    } else {
        std::fs::File::create(&part_path)
            .map_err(|e| format!("Failed to create temp file {}: {e}", part_path.display()))?
    };

    let mut bytes_written: u64 = if is_partial { initial_bytes } else { 0 };
    let mut stream = response.bytes_stream();
    let mut last_progress_emit = Instant::now();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Stream read error: {e}"))?;

        // Ceiling check enforced during streaming (handles servers that omit Content-Length).
        if max_bytes > 0 && bytes_written + chunk.len() as u64 > max_bytes {
            let _ = std::fs::remove_file(&part_path);
            return Err(format!(
                "Download aborted: received >{}MB which exceeds the configured {}MB ceiling for {}",
                bytes_written / 1024 / 1024,
                max_bytes / 1024 / 1024,
                url,
            ));
        }

        file.write_all(&chunk)
            .map_err(|e| format!("Disk write error: {e}"))?;
        bytes_written += chunk.len() as u64;

        // Emit progress at most every 250 ms.
        if last_progress_emit.elapsed() >= Duration::from_millis(250) {
            on_progress(bytes_written, total_bytes);
            last_progress_emit = Instant::now();
        }
    }

    // Final flush + progress event.
    file.flush()
        .map_err(|e| format!("Final flush error: {e}"))?;
    drop(file);
    on_progress(bytes_written, total_bytes);

    // Atomic rename — only now is the file visible at the final path.
    std::fs::rename(&part_path, dest)
        .map_err(|e| format!("Failed to finalize download (rename failed): {e}"))?;

    // Verify MD5 checksum if MD5 file exists on server
    if let Err(e) = verify_remote_md5(url, dest).await {
        let _ = std::fs::remove_file(dest);
        return Err(format!("MD5 checksum verification failed: {e}"));
    }

    Ok((bytes_written, head))
}

pub async fn verify_remote_md5(url: &str, file_path: &Path) -> Result<(), String> {
    let md5_url = format!("{}.md5", url);
    let client = reqwest::Client::new();
    let res = client.get(&md5_url).send().await;
    if let Ok(response) = res {
        if response.status().is_success() {
            if let Ok(expected_hash_text) = response.text().await {
                let expected_hash = expected_hash_text
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_lowercase();
                if expected_hash.len() == 32 {
                    // Compute local file MD5 using md5::Context
                    use std::io::Read;
                    let mut file = std::fs::File::open(file_path).map_err(|e| e.to_string())?;
                    let mut context = md5::Context::new();
                    let mut buf = [0u8; 65536];
                    loop {
                        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
                        if n == 0 {
                            break;
                        }
                        context.consume(&buf[..n]);
                    }
                    let digest = context.finalize();
                    let local_hash = format!("{:x}", digest);
                    if local_hash != expected_hash {
                        return Err(format!(
                            "MD5 mismatch: local {} != expected {}",
                            local_hash, expected_hash
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// True only when we have a strong remote identity signal (ETag or Last-Modified)
/// that differs from what we stored at last successful download.
///
/// Content-Length alone is **not** treated as an update — FTP/CDN HEADs often
/// omit or vary length, which previously caused false "Update" badges.
pub fn remote_changed(
    head: &RemoteHead,
    stored_etag: Option<&str>,
    stored_last_modified: Option<&str>,
    _stored_length: Option<i64>,
) -> bool {
    let stored_etag = stored_etag.map(str::trim).filter(|s| !s.is_empty());
    let stored_lm = stored_last_modified.map(str::trim).filter(|s| !s.is_empty());
    let remote_etag = head.etag.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let remote_lm = head
        .last_modified
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    // Prefer ETag when both sides have one.
    if let (Some(etag), Some(stored)) = (remote_etag, stored_etag) {
        return etag != stored;
    }
    // Fall back to Last-Modified when both sides have one.
    if let (Some(lm), Some(stored)) = (remote_lm, stored_lm) {
        return lm != stored;
    }
    // No comparable identity metadata → do not claim an update.
    false
}
