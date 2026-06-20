// ./src-tauri/src/research/debug_log.rs
//! Optional verbose logging for the vector research sweep pipeline.
//!
//! Enable via:
//! - Environment: `DNA_RESEARCH_DEBUG=1` (or `true`/`yes`/`on`)
//! - UI toggle (persisted in `app_metadata.research_debug_log`)
//!
//! When enabled, detailed phase logs go to stderr and `research:debug` events for the Live Feed.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Optional app handle for UI debug feed (set for the duration of a sweep loop).
static EMIT_APP: std::sync::Mutex<Option<tauri::AppHandle>> = std::sync::Mutex::new(None);

pub fn set_emit_app(app: Option<tauri::AppHandle>) {
    if let Ok(mut guard) = EMIT_APP.lock() {
        *guard = app;
    }
}

pub const METADATA_KEY: &str = "research_debug_log";
const ENV_KEY: &str = "DNA_RESEARCH_DEBUG";

#[derive(Clone, Serialize)]
pub struct ResearchDebugEvent {
    pub tag: String,
    pub message: String,
    pub ts: i64,
}

fn env_truthy(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "on"
        })
        .unwrap_or(false)
}

pub fn refresh_from_env() {
    if env_truthy(ENV_KEY) {
        ENABLED.store(true, Ordering::SeqCst);
    }
}

pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

/// Terminal + UI debug line (uses sweep `AppHandle` when set).
pub fn log(tag: &str, message: impl AsRef<str>) {
    if !is_enabled() {
        return;
    }
    let message = message.as_ref();
    eprintln!("[{}] {}", tag, message);
    if let Ok(guard) = EMIT_APP.lock()
        && let Some(app) = guard.as_ref() {
            let _ = app.emit(
                "research:debug",
                ResearchDebugEvent {
                    tag: tag.to_string(),
                    message: message.to_string(),
                    ts: crate::research::util::unix_now(),
                },
            );
        }
}

/// Terminal + optional UI event when `AppHandle` is available.
pub fn log_emit(app: Option<&AppHandle>, tag: &str, message: impl Into<String>) {
    if !is_enabled() {
        return;
    }
    let message = message.into();
    eprintln!("[{}] {}", tag, message);
    if let Some(app) = app {
        let _ = app.emit(
            "research:debug",
            ResearchDebugEvent {
                tag: tag.to_string(),
                message,
                ts: crate::research::util::unix_now(),
            },
        );
    }
}
