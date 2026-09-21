// ./src-tauri/src/agent_ui_capture.rs
/*
Purpose: Capture the visible Genomics Caddy webview to PNG for local README/QA.
How it is used: POST /ui/capture on the loopback agent UI bridge.
Inputs: the live main WebviewWindow.
Outputs: PNG bytes. Never logs pixel payloads, profile names, or genotype values.
Notes: Linux uses WebKitGTK snapshot so Wayland compositor screenshot denial
does not block capture. Other OS builds return a clear unsupported error.
*/

use std::time::Duration;
use tauri::{AppHandle, Manager};

const CAPTURE_TIMEOUT: Duration = Duration::from_secs(20);

pub async fn capture_main_window_png(app: &AppHandle) -> Result<Vec<u8>, String> {
    capture_main_window_png_inner(app).await
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
async fn capture_main_window_png_inner(app: &AppHandle) -> Result<Vec<u8>, String> {
    use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main Genomics Caddy window is unavailable".to_string())?;
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<Vec<u8>, String>>();

    window
        .with_webview(move |platform| {
            let webview = platform.inner();
            webview.snapshot(
                SnapshotRegion::Visible,
                SnapshotOptions::NONE,
                None::<&webkit2gtk::gio::Cancellable>,
                move |result| {
                    let png = match result {
                        Ok(surface) => {
                            let mut bytes = Vec::new();
                            match surface.write_to_png(&mut bytes) {
                                Ok(()) if !bytes.is_empty() => Ok(bytes),
                                Ok(()) => Err("webview snapshot produced an empty PNG".to_string()),
                                Err(error) => Err(format!("could not encode webview snapshot: {error}")),
                            }
                        }
                        Err(error) => Err(format!("could not snapshot webview: {error}")),
                    };
                    let _ = tx.send(png);
                },
            );
        })
        .map_err(|error| format!("could not reach the webview for capture: {error}"))?;

    match tokio::time::timeout(CAPTURE_TIMEOUT, rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("webview capture dropped before a PNG was returned".to_string()),
        Err(_) => Err("webview capture timed out".to_string()),
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
async fn capture_main_window_png_inner(_app: &AppHandle) -> Result<Vec<u8>, String> {
    let _ = CAPTURE_TIMEOUT;
    Err("webview capture is implemented for Linux agent QA in this workspace".to_string())
}
