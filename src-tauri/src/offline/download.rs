// ./src-tauri/src/offline/download.rs
//! HTTP download helpers — streaming to disk with progress events and atomic completion.

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteProbeMethod {
    Head,
    RangeGetFallback,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteProbeResult {
    pub head: RemoteHead,
    pub method: RemoteProbeMethod,
}

fn remote_probe_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        // Sync-time probe only — keep this short so a dead mirror cannot stall the UI.
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(3))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| e.to_string())
}

fn content_range_total(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let raw = headers
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())?;
    let total = raw.split_once('/')?.1.trim();
    if total == "*" {
        None
    } else {
        total.parse::<u64>().ok()
    }
}

async fn range_metadata_probe(client: &reqwest::Client, url: &str) -> Result<RemoteHead, String> {
    let response = client
        .get(url)
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()
        .await
        .map_err(|e| format!("Range metadata probe failed for {url}: {e}"))?;
    let status = response.status();
    if status != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(format!(
            "Range metadata probe for {url} returned HTTP {status} instead of 206 Partial Content"
        ));
    }
    // Do not accept a server that ignores Range and sends an unbounded partial
    // response. The body is intentionally not read: dropping the response
    // cancels the probe without downloading the resource.
    if response.content_length().is_some_and(|length| length > 1) {
        return Err(format!(
            "Range metadata probe for {url} returned more than one byte"
        ));
    }
    Ok(remote_head_from_headers(
        response.headers(),
        content_range_total(response.headers()),
    ))
}

fn merge_missing_remote_metadata(head: &mut RemoteHead, supplement: RemoteHead) {
    if head.content_length.is_none_or(|length| length == 0) {
        head.content_length = supplement.content_length;
    }
    if head.content_filename.is_none() {
        head.content_filename = supplement.content_filename;
    }
    if head.etag.is_none() {
        head.etag = supplement.etag;
    }
    if head.last_modified.is_none() {
        head.last_modified = supplement.last_modified;
    }
}

pub(crate) async fn probe_remote(url: &str) -> Result<RemoteProbeResult, String> {
    let client = remote_probe_client()?;
    let response = client
        .head(url)
        .send()
        .await
        .map_err(|e| format!("HEAD failed for {url}: {e}"))?;
    let status = response.status();

    if status == reqwest::StatusCode::METHOD_NOT_ALLOWED
        || status == reqwest::StatusCode::NOT_IMPLEMENTED
    {
        let head = range_metadata_probe(&client, url).await?;
        return Ok(RemoteProbeResult {
            head,
            method: RemoteProbeMethod::RangeGetFallback,
        });
    }
    if !status.is_success() {
        return Err(format!("HEAD {url} returned HTTP {status}"));
    }

    let mut head = remote_head_from_headers(response.headers(), response.content_length());
    // Some CDNs omit Content-Length or identity headers on HEAD. A bounded
    // ranged GET can supplement those fields, but it is never used when the
    // server ignores Range or returns a full response.
    if head.content_length.is_none_or(|length| length == 0)
        || head.content_filename.is_none()
        || head.etag.is_none()
        || head.last_modified.is_none()
    {
        if let Ok(supplement) = range_metadata_probe(&client, url).await {
            merge_missing_remote_metadata(&mut head, supplement);
        }
    }
    Ok(RemoteProbeResult {
        head,
        method: RemoteProbeMethod::Head,
    })
}

pub async fn head_remote(url: &str) -> Result<RemoteHead, String> {
    Ok(probe_remote(url).await?.head)
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
/// `on_progress(bytes_written, total_bytes, resumed_bytes)` is called every ~250 ms.
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
    F: Fn(u64, u64, u64) + Send + Sync,
{
    // HEAD — fetch ETag/Last-Modified for update detection; size is advisory only.
    let head = head_remote(url).await?;
    let mut total_bytes = head.content_length.unwrap_or(0);

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
    let metadata_path = PathBuf::from(format!("{}.meta", part_path.display()));

    let mut initial_bytes = 0u64;
    let mut saved_metadata = None;
    if part_path.is_file() && metadata_path.is_file() {
        let file_bytes = std::fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0);
        let metadata = std::fs::read(&metadata_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<PartMetadata>(&bytes).ok());
        if let Some(metadata) = metadata.filter(|metadata| {
            file_bytes > 0
                && metadata.total_bytes > file_bytes
                && metadata.total_bytes == total_bytes
                && resume_identity_matches(&head, metadata)
        }) {
            initial_bytes = file_bytes;
            saved_metadata = Some(metadata);
        }
    }
    if initial_bytes == 0 {
        // A partial from an older app version, a changed remote object, or a
        // server without stable validators cannot be resumed safely.
        let _ = std::fs::remove_file(&part_path);
        let _ = std::fs::remove_file(&metadata_path);
    }

    let client = reqwest::Client::builder()
        // 4-hour timeout for multi-GB files (dbSNP) on slow connections.
        .timeout(Duration::from_secs(14_400))
        .build()
        .map_err(|e| e.to_string())?;

    let mut request = client.get(url);
    if initial_bytes > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={initial_bytes}-"));
        let metadata = saved_metadata
            .as_ref()
            .expect("resume bytes require matching metadata");
        let if_range = metadata
            .etag
            .as_deref()
            .filter(|etag| !etag.trim_start().starts_with("W/"))
            .or(metadata.last_modified.as_deref())
            .expect("resume metadata requires an HTTP validator");
        request = request.header(reqwest::header::IF_RANGE, if_range);
    }

    let mut response = request
        .send()
        .await
        .map_err(|e| format!("GET failed for {url}: {e}"))?;

    let mut is_partial = response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    if initial_bytes > 0 {
        let metadata = saved_metadata
            .as_ref()
            .expect("resume bytes require matching metadata");
        let response_head = remote_head_from_headers(response.headers(), response.content_length());
        let range = response_content_range(response.headers());
        let validator_matches = match (metadata.etag.as_deref(), response_head.etag.as_deref()) {
            (Some(expected), Some(actual)) if !expected.trim_start().starts_with("W/") => {
                expected == actual
            }
            _ => metadata.last_modified.as_deref().is_some_and(|expected| {
                response_head.last_modified.as_deref() == Some(expected)
            }),
        };
        let valid_range = is_partial
            && range.is_some_and(|(start, end, total)| {
                start == initial_bytes
                    && end >= start
                    && end.saturating_add(1) == total
                    && total == metadata.total_bytes
                    && response
                        .content_length()
                        .is_none_or(|length| length == end.saturating_sub(start).saturating_add(1))
            })
            && validator_matches;

        if !valid_range {
            // If-Range normally makes a changed object return 200. Be defensive
            // about broken range servers: throw away the stale part and retry
            // once from byte zero instead of concatenating unrelated versions.
            drop(response);
            let _ = std::fs::remove_file(&part_path);
            let _ = std::fs::remove_file(&metadata_path);
            initial_bytes = 0;
            response = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("GET retry failed for {url}: {e}"))?;
            is_partial = response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
        }
    }

    let status = response.status();
    if !status.is_success() {
        return Err(format!("GET {} returned HTTP {}", url, status));
    }
    if is_partial && initial_bytes == 0 {
        return Err(format!(
            "GET {url} returned HTTP 206 without a validated resume range"
        ));
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
    if let Some((_, _, range_total)) = response_content_range(response.headers()) {
        head.content_length = Some(range_total);
        total_bytes = range_total;
    } else if initial_bytes == 0
        && let Some(n) = get_meta.content_length.filter(|n| *n > 0)
    {
        head.content_length = Some(n);
        total_bytes = n;
    }

    let transfer_metadata = PartMetadata {
        etag: head.etag.clone(),
        last_modified: head.last_modified.clone(),
        total_bytes,
    };
    let encoded_metadata = serde_json::to_vec(&transfer_metadata)
        .map_err(|e| format!("Could not serialize download resume metadata: {e}"))?;
    std::fs::write(&metadata_path, encoded_metadata)
        .map_err(|e| format!("Could not save download resume metadata: {e}"))?;

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
    on_progress(bytes_written, total_bytes, initial_bytes);
    let mut stream = response.bytes_stream();
    let mut last_progress_emit = Instant::now();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Stream read error: {e}"))?;

        // Ceiling check enforced during streaming (handles servers that omit Content-Length).
        if max_bytes > 0 && bytes_written + chunk.len() as u64 > max_bytes {
            let _ = std::fs::remove_file(&part_path);
            let _ = std::fs::remove_file(&metadata_path);
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
            on_progress(bytes_written, total_bytes, initial_bytes);
            last_progress_emit = Instant::now();
        }
    }

    // Final flush + progress event.
    file.flush()
        .map_err(|e| format!("Final flush error: {e}"))?;
    drop(file);
    on_progress(bytes_written, total_bytes, initial_bytes);

    if total_bytes > 0 && bytes_written != total_bytes {
        return Err(format!(
            "Download ended at {} of {} bytes; the partial file is retained for a safe retry.",
            bytes_written, total_bytes
        ));
    }

    // Atomic rename — only now is the file visible at the final path.
    std::fs::rename(&part_path, dest)
        .map_err(|e| format!("Failed to finalize download (rename failed): {e}"))?;
    let _ = std::fs::remove_file(&metadata_path);

    // Verify MD5 checksum if MD5 file exists on server
    if let Err(e) = verify_remote_md5(url, dest).await {
        let _ = std::fs::remove_file(dest);
        return Err(format!("MD5 checksum verification failed: {e}"));
    }

    Ok((bytes_written, head))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PartMetadata {
    etag: Option<String>,
    last_modified: Option<String>,
    total_bytes: u64,
}

fn resume_identity_matches(head: &RemoteHead, saved: &PartMetadata) -> bool {
    if let (Some(current), Some(previous)) = (head.etag.as_deref(), saved.etag.as_deref()) {
        if !current.trim_start().starts_with("W/") && current == previous {
            return true;
        }
    }
    head.last_modified
        .as_deref()
        .is_some_and(|current| saved.last_modified.as_deref() == Some(current))
}

fn response_content_range(headers: &reqwest::header::HeaderMap) -> Option<(u64, u64, u64)> {
    let value = headers
        .get(reqwest::header::CONTENT_RANGE)?
        .to_str()
        .ok()?
        .strip_prefix("bytes ")?;
    let (bounds, total) = value.split_once('/')?;
    let (start, end) = bounds.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?, total.parse().ok()?))
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
/// Priority: ETag → Content-Disposition filename → Last-Modified.
/// Content-Length alone is **not** an update signal (FTP/CDN HEADs thrash).
///
/// ClinGen (and similar) stamp `Last-Modified` to request time on every HEAD
/// while keeping the same daily export bytes. When LM differs but remote and
/// stored Content-Length both match, treat as unchanged unless a disposition
/// filename or ETag proves otherwise. A matching ETag is authoritative even
/// when a server varies its Content-Disposition filename between HEAD and GET.
pub fn remote_changed(
    head: &RemoteHead,
    stored_etag: Option<&str>,
    stored_last_modified: Option<&str>,
    stored_length: Option<i64>,
    stored_content_filename: Option<&str>,
) -> bool {
    let stored_etag = stored_etag.map(str::trim).filter(|s| !s.is_empty());
    let stored_lm = stored_last_modified
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let stored_name = stored_content_filename
        .map(str::trim)
        .filter(|s| !s.is_empty() && looks_like_download_filename(s));
    let remote_etag = head
        .etag
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
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

    // Prefer ETag when both sides have one.
    if let (Some(etag), Some(stored)) = (remote_etag, stored_etag) {
        return etag != stored;
    }

    // A changed stable export filename is the next-best identity signal when
    // no comparable ETag pair exists. Some servers vary this header between
    // HEAD and GET, so it must not override a matching ETag above.
    if let (Some(remote), Some(stored)) = (remote_name, stored_name) {
        if remote != stored {
            return true;
        }
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
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    fn fixture_directory(label: &str) -> std::path::PathBuf {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "dna_tools_{label}_{}_{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn read_request(stream: &mut TcpStream) -> Option<String> {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let count = stream.read(&mut chunk).ok()?;
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..count]);
            if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
            if bytes.len() > 16 * 1024 {
                return None;
            }
        }
        String::from_utf8(bytes).ok()
    }

    fn write_fixture_response(stream: &mut TcpStream, status: &str, body: &[u8], headers: &str) {
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write fixture response headers");
        if !body.is_empty() {
            stream.write_all(body).expect("write fixture response body");
        }
    }

    fn write_fixture_head_response(stream: &mut TcpStream, headers: &str) {
        let response =
            format!("HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n{headers}\r\n");
        stream
            .write_all(response.as_bytes())
            .expect("write fixture HEAD response");
    }

    fn spawn_retry_fixture_server(listener: TcpListener) -> thread::JoinHandle<()> {
        listener
            .set_nonblocking(true)
            .expect("set fixture listener nonblocking");
        thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            let mut get_attempts = 0u8;
            let mut requests = 0u8;
            while requests < 7 && std::time::Instant::now() < deadline {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(_) => break,
                };
                requests += 1;
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("set fixture read timeout");
                let Some(request) = read_request(&mut stream) else {
                    continue;
                };
                let mut request_parts = request.split_whitespace();
                let method = request_parts.next().unwrap_or_default();
                let path = request_parts.next().unwrap_or_default();
                let has_range_probe = request
                    .lines()
                    .any(|line| line.to_ascii_lowercase().starts_with("range:"));
                if path.ends_with(".md5") {
                    write_fixture_response(&mut stream, "404 Not Found", &[], "");
                } else if method == "HEAD" {
                    write_fixture_head_response(
                        &mut stream,
                        "ETag: \"fixture-v1\"\r\nContent-Disposition: attachment; filename=fixture.bin\r\n",
                    );
                } else if method == "GET" && has_range_probe {
                    write_fixture_response(
                        &mut stream,
                        "206 Partial Content",
                        b"h",
                        "Content-Range: bytes 0-0/5\r\nETag: \"fixture-v1\"\r\nContent-Disposition: attachment; filename=fixture.bin\r\n",
                    );
                } else if method == "GET" {
                    get_attempts += 1;
                    if get_attempts == 1 {
                        write_fixture_response(&mut stream, "503 Service Unavailable", &[], "");
                    } else {
                        write_fixture_response(
                            &mut stream,
                            "200 OK",
                            b"hello",
                            "ETag: \"fixture-v1\"\r\nContent-Disposition: attachment; filename=fixture.bin\r\n",
                        );
                    }
                } else {
                    write_fixture_response(&mut stream, "405 Method Not Allowed", &[], "");
                }
            }
        })
    }

    fn spawn_resume_fixture_server(listener: TcpListener) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            for _ in 0..4 {
                let (mut stream, _) = listener.accept().expect("accept resume fixture request");
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("set resume fixture read timeout");
                let request = read_request(&mut stream).expect("read resume fixture request");
                let method = request.split_whitespace().next().unwrap_or_default();
                let path = request.split_whitespace().nth(1).unwrap_or_default();
                if path.ends_with(".md5") {
                    write_fixture_response(&mut stream, "404 Not Found", &[], "");
                } else if method == "HEAD" {
                    write_fixture_head_response(
                        &mut stream,
                        "ETag: \"fixture-v1\"\r\nLast-Modified: Sat, 11 Jul 2026 21:00:00 GMT\r\n",
                    );
                } else if request.lines().any(|line| {
                    line.to_ascii_lowercase().starts_with("range: bytes=0-0")
                }) {
                    write_fixture_response(
                        &mut stream,
                        "206 Partial Content",
                        b"h",
                        "Content-Range: bytes 0-0/5\r\nETag: \"fixture-v1\"\r\nLast-Modified: Sat, 11 Jul 2026 21:00:00 GMT\r\nContent-Disposition: attachment; filename=fixture.bin\r\n",
                    );
                } else {
                    let resume_headers: Vec<&str> = request
                        .lines()
                        .map(str::trim)
                        .filter(|line| {
                            let line = line.to_ascii_lowercase();
                            line.starts_with("range:") || line.starts_with("if-range:")
                        })
                        .collect();
                    assert!(
                        resume_headers
                            .iter()
                            .any(|line| line.eq_ignore_ascii_case("Range: bytes=3-")),
                        "expected resume Range header, got {resume_headers:?}"
                    );
                    assert!(
                        resume_headers
                            .iter()
                            .any(|line| line.eq_ignore_ascii_case("If-Range: \"fixture-v1\"")),
                        "expected resume If-Range header, got {resume_headers:?}"
                    );
                    write_fixture_response(
                        &mut stream,
                        "206 Partial Content",
                        b"lo",
                        "Content-Range: bytes 3-4/5\r\nETag: \"fixture-v1\"\r\nLast-Modified: Sat, 11 Jul 2026 21:00:00 GMT\r\n",
                    );
                }
            }
        })
    }

    fn spawn_head_fallback_fixture_server(
        listener: TcpListener,
        head_status: &'static str,
        honors_range: bool,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("accept fallback fixture request");
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("set fallback fixture read timeout");
                let request = read_request(&mut stream).expect("read fallback fixture request");
                let method = request.split_whitespace().next().unwrap_or_default();
                if method == "HEAD" {
                    write_fixture_response(&mut stream, head_status, &[], "");
                } else if honors_range {
                    write_fixture_response(
                        &mut stream,
                        "206 Partial Content",
                        b"h",
                        "Content-Range: bytes 0-0/5\r\nETag: \"fixture-v1\"\r\nContent-Disposition: attachment; filename=fixture.bin\r\n",
                    );
                } else {
                    write_fixture_response(
                        &mut stream,
                        "200 OK",
                        b"hello",
                        "ETag: \"fixture-v1\"\r\nContent-Disposition: attachment; filename=fixture.bin\r\n",
                    );
                }
            }
        })
    }

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

    #[test]
    fn matching_etag_suppresses_filename_variation_after_download() {
        let head = RemoteHead {
            content_length: Some(100),
            etag: Some("\"same-payload\"".into()),
            last_modified: Some("Sun, 12 Jul 2026 01:00:00 GMT".into()),
            content_filename: Some("catalog-latest.csv".into()),
        };
        // HEAD and GET can expose different attachment names even though the
        // downloaded payload is unchanged. The strong ETag must prevent a
        // perpetual update badge in that case.
        assert!(!remote_changed(
            &head,
            Some("\"same-payload\""),
            Some("Sat, 11 Jul 2026 20:58:34 GMT"),
            Some(100),
            Some("catalog-2026-07-11.csv"),
        ));
    }

    #[test]
    fn etag_change_is_detected_when_export_filename_is_unchanged() {
        let head = RemoteHead {
            content_length: Some(100),
            etag: Some("\"b\"".into()),
            last_modified: Some("Sun, 12 Jul 2026 01:00:00 GMT".into()),
            content_filename: Some("catalog-2026-07-11.csv".into()),
        };
        assert!(remote_changed(
            &head,
            Some("\"a\""),
            Some("Sat, 11 Jul 2026 20:58:34 GMT"),
            Some(100),
            Some("catalog-2026-07-11.csv"),
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn head_method_rejection_uses_bounded_range_metadata_fallback() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind HEAD fallback fixture");
        let address = listener
            .local_addr()
            .expect("read HEAD fallback fixture address");
        let server = spawn_head_fallback_fixture_server(listener, "405 Method Not Allowed", true);
        let url = format!("http://{address}/fixture.bin");

        let result = probe_remote(&url).await.expect("range fallback probe");
        assert_eq!(result.method, RemoteProbeMethod::RangeGetFallback);
        assert_eq!(result.head.content_length, Some(5));
        assert_eq!(result.head.etag.as_deref(), Some("\"fixture-v1\""));
        assert_eq!(result.head.content_filename.as_deref(), Some("fixture.bin"));

        server.join().expect("join HEAD fallback fixture");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn head_fallback_rejects_servers_that_ignore_range() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ignored-range fixture");
        let address = listener
            .local_addr()
            .expect("read ignored-range fixture address");
        let server = spawn_head_fallback_fixture_server(listener, "501 Not Implemented", false);
        let url = format!("http://{address}/fixture.bin");

        let error = probe_remote(&url)
            .await
            .expect_err("full-body response must not be accepted as metadata");
        assert!(error.contains("instead of 206 Partial Content"));

        server.join().expect("join ignored-range fixture");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn failed_download_preserves_known_good_file_and_retry_recovers_from_fixture() {
        let directory = fixture_directory("download_recovery");
        std::fs::create_dir_all(&directory).expect("create fixture directory");
        let destination = directory.join("fixture.bin");
        let part_path = directory.join("fixture.bin.part");
        std::fs::write(&destination, b"known-good").expect("write known-good file");

        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind fixture server");
        let address = listener.local_addr().expect("read fixture address");
        let server = spawn_retry_fixture_server(listener);
        let url = format!("http://{address}/fixture.bin");

        let first_attempt = download_to_path(&url, &destination, 1024, |_, _, _| {}).await;
        assert!(first_attempt.is_err());
        assert_eq!(
            std::fs::read(&destination).expect("read preserved file"),
            b"known-good"
        );
        assert!(!part_path.exists());

        let second_attempt = download_to_path(&url, &destination, 1024, |_, _, _| {})
            .await
            .expect("retry fixture download");
        assert_eq!(second_attempt.0, 5);
        assert_eq!(
            std::fs::read(&destination).expect("read recovered file"),
            b"hello"
        );
        assert!(!part_path.exists());

        server.join().expect("join fixture server");
        std::fs::remove_dir_all(&directory).expect("remove fixture directory");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn validated_partial_download_resumes_and_reports_retained_bytes() {
        let directory = fixture_directory("download_resume");
        std::fs::create_dir_all(&directory).expect("create resume fixture directory");
        let destination = directory.join("fixture.bin");
        let part_path = directory.join("fixture.bin.part");
        let metadata_path = directory.join("fixture.bin.part.meta");
        std::fs::write(&part_path, b"hel").expect("write retained partial payload");
        std::fs::write(
            &metadata_path,
            serde_json::to_vec(&PartMetadata {
                etag: Some("\"fixture-v1\"".into()),
                last_modified: Some("Sat, 11 Jul 2026 21:00:00 GMT".into()),
                total_bytes: 5,
            })
            .expect("serialize resume metadata"),
        )
        .expect("write resume metadata");

        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind resume fixture server");
        let address = listener.local_addr().expect("read resume fixture address");
        let server = spawn_resume_fixture_server(listener);
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded = events.clone();
        let (bytes, _) = download_to_path(
            &format!("http://{address}/fixture.bin"),
            &destination,
            1024,
            move |done, total, resumed| {
                recorded
                    .lock()
                    .expect("lock progress events")
                    .push((done, total, resumed));
            },
        )
        .await
        .expect("resume validated partial payload");

        assert_eq!(bytes, 5);
        assert_eq!(std::fs::read(&destination).expect("read final payload"), b"hello");
        assert!(!part_path.exists());
        assert!(!metadata_path.exists());
        let events = events.lock().expect("lock recorded events");
        assert!(events.iter().any(|event| *event == (3, 5, 3)));
        assert!(events.iter().any(|event| *event == (5, 5, 3)));

        server.join().expect("join resume fixture server");
        std::fs::remove_dir_all(&directory).expect("remove resume fixture directory");
    }
}
