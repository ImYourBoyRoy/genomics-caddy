// ./src-tauri/src/stream_control.rs
/*
Purpose: Cooperative cancellation and namespaced events for Ollama streaming.
*/

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static OLLAMA_STREAM_CANCEL: AtomicBool = AtomicBool::new(false);
static ACTIVE_STREAM_ID: Mutex<Option<String>> = Mutex::new(None);

pub fn validate_stream_id(stream_id: &str) -> Result<(), String> {
    let id = stream_id.trim();
    if id.is_empty() || id.len() > 64 {
        return Err("Invalid stream_id length (1–64 characters required)".into());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("stream_id may only contain ASCII letters, digits, and hyphens".into());
    }
    Ok(())
}

pub fn ollama_chunk_event(stream_id: &str) -> String {
    format!("ollama-chunk:{stream_id}")
}

pub fn ollama_done_event(stream_id: &str) -> String {
    format!("ollama-done:{stream_id}")
}

pub fn reset_ollama_stream(stream_id: &str) {
    validate_stream_id(stream_id).ok();
    OLLAMA_STREAM_CANCEL.store(false, Ordering::SeqCst);
    if let Ok(mut guard) = ACTIVE_STREAM_ID.lock() {
        *guard = Some(stream_id.to_string());
    }
}

pub fn request_ollama_stream_cancel() {
    OLLAMA_STREAM_CANCEL.store(true, Ordering::SeqCst);
}

pub fn is_ollama_stream_cancelled() -> bool {
    OLLAMA_STREAM_CANCEL.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub fn active_stream_id() -> Option<String> {
    ACTIVE_STREAM_ID.lock().ok()?.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_id_rejects_empty_and_unsafe_chars() {
        assert!(validate_stream_id("").is_err());
        assert!(validate_stream_id("../evil").is_err());
        assert!(validate_stream_id("stream:1").is_err());
        assert!(validate_stream_id("abc-123-XYZ").is_ok());
    }

    #[test]
    fn event_names_are_namespaced() {
        assert_eq!(ollama_chunk_event("abc"), "ollama-chunk:abc");
        assert_eq!(ollama_done_event("abc"), "ollama-done:abc");
    }
}
