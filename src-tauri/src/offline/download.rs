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
    /// Filename from Content-Disposition when the server provides one
    /// (e.g. ClinGen `Clingen-Gene-Disease-Summary-2026-07-11.csv`).
    pub content_filename: Option<String>,
}

/// Parse `attachment; filename=…` / `filename*=UTF-8''…` into a bare filename.
pub fn content_filename_from_headers(headers: &reqwest::header::HeaderMap) -> Option<String> {
    let raw = headers
        .get(reqwest::header::CONTENT_DISPOSITION)?
        .to_str()
        .ok()?;
    parse_content_disposition_filename(raw)
}

fn parse_content_disposition_filename(raw: &str) -> Option<String> {
    // Prefer RFC 5987 filename*=charset''value
    for part in raw.split(';') {
        let part = part.trim();
        if !part.to_ascii_lowercase().starts_with("filename*=") {
            continue;
        }
        let value = part.split_once('=')?.1.trim().trim_matches('"');
        let decoded = value
            .split_once("''")
            .map(|(_, rest)| percent_decode_simple(rest))
            .unwrap_or_else(|| value.to_string());
        if !decoded.is_empty() {
            return Some(decoded);
        }
    }
    for part in raw.split(';') {
        let part = part.trim();
        if !part.to_ascii_lowercase().starts_with("filename=")
            || part.to_ascii_lowercase().starts_with("filename*=")
        {
            continue;
        }
        let value = part.split_once('=')?.1.trim().trim_matches('"').trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn percent_decode_simple(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push(((h << 4) | l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn remote_head_from_headers(
    headers: &reqwest::header::HeaderMap,
    content_length: Option<u64>,
) -> RemoteHead {
    RemoteHead {
        content_length,
        etag: headers
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(String::from),
        last_modified: headers
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(String::from),
        content_filename: content_filename_from_headers(headers),
    }
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
    let mut head = remote_head_from_headers(response.headers(), content_length);
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
            // Prefer Content-Disposition from the ranged GET when HEAD omitted it.
            if head.content_filename.is_none() {
                head.content_filename = content_filename_from_headers(probe.headers());
            }
            if head.etag.is_none() {
                head.etag = probe
                    .headers()
                    .get(reqwest::header::ETAG)
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);
            }
            if head.last_modified.is_none() {
                head.last_modified = probe
                    .headers()
                    .get(reqwest::header::LAST_MODIFIED)
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);
            }
        }
    }
    head.content_length = content_length;
    Ok(head)
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

    // Prefer identity headers from the actual GET (ClinGen stamps LM on every request;
    // Content-Disposition filename is the stable daily export id).
    let mut head = head;
    let get_meta = remote_head_from_headers(response.headers(), response.content_length());
    if get_meta.content_filename.is_some() {
        head.content_filename = get_meta.content_filename;
    }
    if get_meta.etag.is_some() {
        head.etag = get_meta.etag;
    }
    if get_meta.last_modified.is_some() {
        head.last_modified = get_meta.last_modified;
    }
    if let Some(n) = get_meta.content_length.filter(|n| *n > 0) {
        head.content_length = Some(n);
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

/// True only when we have a strong remote identity signal that differs from
/// what we stored at last successful download.
///
/// Priority: Content-Disposition filename → ETag → Last-Modified.
/// Content-Length alone is **not** an update signal (FTP/CDN HEADs thrash).
///
/// ClinGen (and similar) stamp `Last-Modified` to request time on every HEAD
/// while keeping the same daily export bytes. When LM differs but remote and
/// stored Content-Length both match, treat as unchanged unless a disposition
/// filename or ETag proves otherwise.
pub fn remote_changed(
    head: &RemoteHead,
    stored_etag: Option<&str>,
    stored_last_modified: Option<&str>,
    stored_length: Option<i64>,
    stored_content_filename: Option<&str>,
) -> bool {
    let stored_etag = stored_etag.map(str::trim).filter(|s| !s.is_empty());
    let stored_lm = stored_last_modified.map(str::trim).filter(|s| !s.is_empty());
    let stored_name = stored_content_filename
        .map(str::trim)
        .filter(|s| !s.is_empty() && looks_like_download_filename(s));
    let remote_etag = head.etag.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let remote_lm = head
        .last_modified
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let remote_name = head
        .content_filename
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    // Stable export filename (ClinGen daily CSV) beats volatile Last-Modified.
    if let (Some(remote), Some(stored)) = (remote_name, stored_name) {
        return remote != stored;
    }

    // Prefer ETag when both sides have one.
    if let (Some(etag), Some(stored)) = (remote_etag, stored_etag) {
        return etag != stored;
    }

    // Fall back to Last-Modified only when the payload size also gives us a
    // comparable identity signal. A request-time LM header without a stable
    // filename, ETag, or size is not enough evidence to label a downloaded
    // resource as outdated.
    if let (Some(lm), Some(stored)) = (remote_lm, stored_lm) {
        if lm == stored {
            return false;
        }
        if let (Some(remote_len), Some(stored_len)) = (head.content_length, stored_length) {
            if remote_len > 0 && stored_len > 0 {
                return remote_len as i64 != stored_len;
            }
        }
    }

    // No comparable identity metadata → do not claim an update. A false
    // negative is safer than repeatedly asking the user to redownload a
    // resource whose server metadata is known to be unstable.
    false
}

fn looks_like_download_filename(s: &str) -> bool {
    // version_label historically stored Last-Modified HTTP dates; only treat
    // values that look like real attachment names as content identity.
    s.contains('.') && !s.contains(',') && !s.contains(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clingen_disposition_filename() {
        let name = parse_content_disposition_filename(
            "attachment; filename=Clingen-Gene-Disease-Summary-2026-07-11.csv",
        );
        assert_eq!(
            name.as_deref(),
            Some("Clingen-Gene-Disease-Summary-2026-07-11.csv")
        );
    }

    #[test]
    fn clingen_lm_thrash_same_length_is_not_update() {
        let head = RemoteHead {
            content_length: Some(1_112_373),
            etag: None,
            last_modified: Some("Sat, 11 Jul 2026 21:00:00 GMT".into()),
            content_filename: Some("Clingen-Gene-Disease-Summary-2026-07-11.csv".into()),
        };
        // Stored version_label is still an old LM string (pre-fix installs).
        assert!(!remote_changed(
            &head,
            None,
            Some("Sat, 11 Jul 2026 20:58:34 GMT"),
            Some(1_112_373),
            Some("Sat, 11 Jul 2026 20:58:34 GMT"),
        ));
    }

    #[test]
    fn clingen_filename_date_change_is_update() {
        let head = RemoteHead {
            content_length: Some(1_112_373),
            etag: None,
            last_modified: Some("Sun, 12 Jul 2026 01:00:00 GMT".into()),
            content_filename: Some("Clingen-Gene-Disease-Summary-2026-07-12.csv".into()),
        };
        assert!(remote_changed(
            &head,
            None,
            Some("Sat, 11 Jul 2026 20:58:34 GMT"),
            Some(1_112_373),
            Some("Clingen-Gene-Disease-Summary-2026-07-11.csv"),
        ));
    }

    #[test]
    fn last_modified_only_change_without_size_is_not_update() {
        let head = RemoteHead {
            content_length: None,
            etag: None,
            last_modified: Some("Sun, 12 Jul 2026 01:00:00 GMT".into()),
            content_filename: None,
        };
        assert!(!remote_changed(
            &head,
            None,
            Some("Sat, 11 Jul 2026 20:58:34 GMT"),
            None,
            None,
        ));
    }

    #[test]
    fn last_modified_with_changed_size_is_update() {
        let head = RemoteHead {
            content_length: Some(200),
            etag: None,
            last_modified: Some("Sun, 12 Jul 2026 01:00:00 GMT".into()),
            content_filename: None,
        };
        assert!(remote_changed(
            &head,
            None,
            Some("Sat, 11 Jul 2026 20:58:34 GMT"),
            Some(100),
            None,
        ));
    }

    #[test]
    fn etag_change_still_wins() {
        let head = RemoteHead {
            content_length: Some(100),
            etag: Some("\"b\"".into()),
            last_modified: None,
            content_filename: None,
        };
        assert!(remote_changed(&head, Some("\"a\""), None, Some(100), None));
        assert!(!remote_changed(&head, Some("\"b\""), None, Some(100), None));
    }
}
