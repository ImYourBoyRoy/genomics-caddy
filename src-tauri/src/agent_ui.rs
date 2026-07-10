// ./src-tauri/src/agent_ui.rs
/*
Purpose: Agent/MCP bridge to inspect and drive the live Svelte UI.
Responsibilities:
- Emit agent-ui-request events to the webview and await responses.
- Serve a localhost-only HTTP control plane for external agents (GUI must be running).
Key Inputs: method name + optional JSON args; HTTP on 127.0.0.1:17321.
Key Outputs: JSON result from window.__GENOMICS_CADDY_UI__.
Operational Notes: Bind is loopback-only. Requires the frontend bridge in +page.svelte.
*/

use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

type PendingMap = Mutex<HashMap<String, oneshot::Sender<Result<Value, String>>>>;

const AGENT_UI_PORT: u16 = 17321;

fn pending() -> &'static PendingMap {
    static PENDING: OnceLock<PendingMap> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_request_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!(
        "ui-{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn ensure_response_listener(app: &AppHandle) {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    let app_handle = app.clone();
    app_handle.listen("agent-ui-response", move |event| {
        let Ok(payload) = serde_json::from_str::<Value>(event.payload()) else {
            return;
        };
        let Some(id) = payload.get("id").and_then(|v| v.as_str()) else {
            return;
        };
        let result = if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(payload.get("result").cloned().unwrap_or(Value::Null))
        } else {
            Err(payload
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("UI bridge error")
                .to_string())
        };
        if let Ok(mut map) = pending().lock() {
            if let Some(tx) = map.remove(id) {
                let _ = tx.send(result);
            }
        }
    });
}

/// Invoke a frontend UI bridge method and wait for the JSON result.
pub async fn invoke_ui(
    app: &AppHandle,
    method: &str,
    args: Option<Value>,
) -> Result<Value, String> {
    ensure_response_listener(app);

    // Ensure at least one webview window exists.
    if app.webview_windows().is_empty() {
        return Err("No Genomics Caddy window is open".to_string());
    }

    let id = next_request_id();
    let (tx, rx) = oneshot::channel();
    {
        let mut map = pending()
            .lock()
            .map_err(|_| "UI bridge lock poisoned".to_string())?;
        map.insert(id.clone(), tx);
    }

    app.emit(
        "agent-ui-request",
        json!({
            "id": id,
            "method": method,
            "args": args.unwrap_or(Value::Null),
        }),
    )
    .map_err(|e| format!("Failed to emit agent-ui-request: {e}"))?;

    match tokio::time::timeout(Duration::from_secs(5), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("UI bridge response channel closed".to_string()),
        Err(_) => {
            if let Ok(mut map) = pending().lock() {
                map.remove(&id);
            }
            Err(
                "Timed out waiting for UI bridge (is the Genomics Caddy window open and loaded?)"
                    .to_string(),
            )
        }
    }
}

#[tauri::command]
pub async fn agent_ui_invoke(
    app: AppHandle,
    method: String,
    args: Option<Value>,
) -> Result<Value, String> {
    invoke_ui(&app, &method, args).await
}

/// Start loopback HTTP control plane: GET /health, GET /ui/snapshot, POST /ui/{method}.
pub fn start_http_bridge(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let addr = format!("127.0.0.1:{AGENT_UI_PORT}");
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Agent UI HTTP bridge not started on {addr}: {e}");
                return;
            }
        };
        eprintln!("Agent UI HTTP bridge listening on http://{addr}");

        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                continue;
            };
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let mut buf = vec![0u8; 16_384];
                let n = match socket.read(&mut buf).await {
                    Ok(0) | Err(_) => return,
                    Ok(n) => n,
                };
                let req = String::from_utf8_lossy(&buf[..n]);
                let (status, body) = handle_http_request(&app, &req).await;
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
}

async fn handle_http_request(app: &AppHandle, req: &str) -> (u16, String) {
    let first = req.lines().next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    if method == "OPTIONS" {
        return (204, String::new());
    }

    if method == "GET" && path == "/health" {
        return (
            200,
            json!({
                "ok": true,
                "service": "genomics-caddy-agent-ui",
                "port": AGENT_UI_PORT,
                "windows": app.webview_windows().len(),
            })
            .to_string(),
        );
    }

    if method == "GET" && path == "/ui/snapshot" {
        return match invoke_ui(app, "snapshot", None).await {
            Ok(v) => (200, v.to_string()),
            Err(e) => (504, json!({"ok": false, "error": e}).to_string()),
        };
    }

    if method == "POST" && path.starts_with("/ui/") {
        let ui_method = path.trim_start_matches("/ui/");
        let body = req.split("\r\n\r\n").nth(1).unwrap_or("{}").trim();
        let args: Value = serde_json::from_str(if body.is_empty() { "{}" } else { body })
            .unwrap_or_else(|_| json!({}));
        // Convenience: POST /ui/setTab {"tab":"map"} or {"arg":"map"}
        let args = if ui_method == "setTab" {
            if let Some(tab) = args.get("tab").cloned() {
                tab
            } else if let Some(arg) = args.get("arg").cloned() {
                arg
            } else {
                args
            }
        } else if matches!(ui_method, "clickText" | "queryText") {
            if let Some(text) = args.get("text").cloned() {
                text
            } else if let Some(arg) = args.get("arg").cloned() {
                arg
            } else {
                args
            }
        } else {
            args
        };

        return match invoke_ui(app, ui_method, Some(args)).await {
            Ok(v) => (200, v.to_string()),
            Err(e) => (504, json!({"ok": false, "error": e}).to_string()),
        };
    }

    (
        404,
        json!({
            "ok": false,
            "error": "not found",
            "endpoints": [
                "GET /health",
                "GET /ui/snapshot",
                "POST /ui/setTab",
                "POST /ui/expandLabs",
                "POST /ui/clickText",
                "POST /ui/queryText"
            ]
        })
        .to_string(),
    )
}
