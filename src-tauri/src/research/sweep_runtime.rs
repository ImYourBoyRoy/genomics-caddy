// ./src-tauri/src/research/sweep_runtime.rs
//! Progress emission for desktop (Tauri events) and headless (NDJSON stdout / Docker logs).

use super::types::{ResearchFindingPreview, ResearchProgress};
use tauri::Emitter;

#[derive(Clone)]
pub enum SweepProgressSink {
    Desktop(tauri::AppHandle),
    Headless,
}

impl SweepProgressSink {
    pub fn emit_progress(&self, payload: ResearchProgress) {
        match self {
            SweepProgressSink::Desktop(app) => {
                if let Err(e) = app.emit("research:progress", &payload) {
                    eprintln!("[sweep] research:progress emit failed: {}", e);
                }
            }
            SweepProgressSink::Headless => {
                if let Ok(line) = serde_json::to_string(&payload) {
                    println!("GENOMICS_PROGRESS {line}");
                }
            }
        }
    }

    pub fn emit_finding(&self, preview: ResearchFindingPreview) {
        match self {
            SweepProgressSink::Desktop(app) => {
                if let Err(e) = app.emit("research:finding", &preview) {
                    eprintln!("[sweep] research:finding emit failed: {}", e);
                }
            }
            SweepProgressSink::Headless => {
                if let Ok(line) = serde_json::to_string(&preview) {
                    println!("GENOMICS_FINDING {line}");
                }
            }
        }
    }

    pub fn debug_log(&self, tag: &str, message: impl Into<String>) {
        let msg = message.into();
        match self {
            SweepProgressSink::Desktop(app) => {
                super::debug_log::log_emit(Some(app), tag, msg);
            }
            SweepProgressSink::Headless => {
                eprintln!("[{tag}] {msg}");
            }
        }
    }

    pub fn as_app(&self) -> Option<&tauri::AppHandle> {
        match self {
            SweepProgressSink::Desktop(app) => Some(app),
            SweepProgressSink::Headless => None,
        }
    }
}
