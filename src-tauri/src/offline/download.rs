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
        .timeout(Duration::from_secs(60))
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
    Ok(RemoteHead {
        content_length: response.content_length(),
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

    let client = reqwest::Client::builder()
        // 4-hour timeout for multi-GB files (dbSNP) on slow connections.
        .timeout(Duration::from_secs(14_400))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("GET failed for {url}: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("GET {} returned HTTP {}", url, response.status()));
    }

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

    let mut file = std::fs::File::create(&part_path)
        .map_err(|e| format!("Failed to create temp file {}: {e}", part_path.display()))?;

    let mut stream = response.bytes_stream();
    let mut bytes_written: u64 = 0;
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
    file.flush().map_err(|e| format!("Final flush error: {e}"))?;
    drop(file);
    on_progress(bytes_written, total_bytes);

    // Atomic rename — only now is the file visible at the final path.
    std::fs::rename(&part_path, dest)
        .map_err(|e| format!("Failed to finalize download (rename failed): {e}"))?;

    Ok((bytes_written, head))
}

pub fn remote_changed(
    head: &RemoteHead,
    stored_etag: Option<&str>,
    stored_last_modified: Option<&str>,
    stored_length: Option<i64>,
) -> bool {
    if let (Some(etag), Some(stored)) = (head.etag.as_deref(), stored_etag) {
        if !stored.is_empty() && etag != stored {
            return true;
        }
    }
    if let (Some(lm), Some(stored)) = (head.last_modified.as_deref(), stored_last_modified) {
        if !stored.is_empty() && lm != stored {
            return true;
        }
    }
    if let (Some(len), Some(stored)) = (head.content_length, stored_length) {
        if stored > 0 && len as i64 != stored {
            return true;
        }
    }
    false
}
