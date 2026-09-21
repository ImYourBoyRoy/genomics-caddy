// ./src-tauri/src/agent_ui.rs
/*
Purpose: Agent/MCP bridge to inspect and drive the live Svelte UI.
Responsibilities:
- Emit agent-ui-request events to the webview and await responses.
- Serve a loopback-only HTTP control plane for external agents (GUI must be running).
Key Inputs: method name + optional JSON args; bind 127.0.0.1 with an ephemeral port.
Key Outputs: JSON result from window.__GENOMICS_CADDY_UI__; 0600 runtime endpoint file.
Operational Notes:
- Never bind 0.0.0.0 or advertise `localhost` (that often resolves to ::1).
- Enabled in debug builds by default; release requires GENOMICS_AGENT_UI=1.
- /ui paths require a session token (env or generated). Token lives in the endpoint file.
- Debug-only unauthenticated access requires the explicit GENOMICS_AGENT_UI_ALLOW_UNAUTHENTICATED=1 opt-in.
*/

use crate::agent_ui_endpoint::{
    self, AgentUiEndpoint, AGENT_UI_SERVICE, LOOPBACK_HOST,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener, Manager, PhysicalSize, Size};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

type PendingMap = Mutex<HashMap<String, oneshot::Sender<Result<Value, String>>>>;

static SESSION_TOKEN: OnceLock<Option<String>> = OnceLock::new();
static ENDPOINT_PATH: OnceLock<PathBuf> = OnceLock::new();
static BOUND_PORT: OnceLock<u16> = OnceLock::new();

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

fn agent_ui_enabled() -> bool {
    if cfg!(debug_assertions) {
        return true;
    }
    matches!(
        std::env::var("GENOMICS_AGENT_UI")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn env_token() -> Option<String> {
    std::env::var("GENOMICS_AGENT_UI_TOKEN")
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

fn session_token() -> Option<String> {
    SESSION_TOKEN
        .get_or_init(|| {
            if let Some(token) = env_token() {
                return Some(token);
            }
            if allow_unauthenticated() {
                return None;
            }
            match agent_ui_endpoint::generate_session_token() {
                Ok(token) => Some(token),
                Err(error) => {
                    eprintln!("Agent UI token generation failed: {error}");
                    None
                }
            }
        })
        .clone()
}

pub fn cleanup_endpoint_file() {
    if let Some(path) = ENDPOINT_PATH.get() {
        agent_ui_endpoint::remove_endpoint_file(path);
    }
}

fn allow_unauthenticated() -> bool {
    cfg!(debug_assertions)
        && matches!(
            std::env::var("GENOMICS_AGENT_UI_ALLOW_UNAUTHENTICATED")
                .ok()
                .as_deref()
                .map(str::trim),
            Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
        )
}

fn extract_request_token(req: &str) -> Option<String> {
    for line in req.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if name == "authorization" {
            if let Some(token) = value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
            {
                return Some(token.trim().to_string());
            }
            return Some(value.to_string());
        }
        if name == "x-genomics-agent-ui-token" {
            return Some(value.to_string());
        }
    }
    None
}

fn authorize_ui(req: &str) -> Result<(), String> {
    if let Some(expected) = session_token() {
        return match extract_request_token(req) {
            Some(got) if got == expected => Ok(()),
            Some(_) => Err("Invalid agent UI token".into()),
            None => Err(
                "Agent UI token required (Authorization: Bearer … or X-Genomics-Agent-Ui-Token)"
                    .into(),
            ),
        };
    }
    if allow_unauthenticated() {
        return Ok(());
    }
    Err("Agent UI token is not configured; restart the app or set GENOMICS_AGENT_UI_TOKEN".into())
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

    match tokio::time::timeout(Duration::from_secs(30), rx).await {
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
    if !agent_ui_enabled() {
        eprintln!(
            "Agent UI HTTP bridge disabled (release build). Set GENOMICS_AGENT_UI=1 to enable."
        );
        return;
    }

    tauri::async_runtime::spawn(async move {
        let token = session_token();
        if token.is_none() && !allow_unauthenticated() {
            eprintln!("Agent UI HTTP bridge requires a token; set GENOMICS_AGENT_UI_TOKEN");
            return;
        }
        let preferred = agent_ui_endpoint::preferred_bind_port(
            std::env::var("GENOMICS_AGENT_UI_PORT").ok().as_deref(),
        );
        let (std_listener, port) = match agent_ui_endpoint::bind_loopback_std(preferred) {
            Ok(bound) => bound,
            Err(error) => {
                eprintln!("Agent UI HTTP bridge could not bind 127.0.0.1: {error}");
                return;
            }
        };
        let listener = match TcpListener::from_std(std_listener) {
            Ok(bound) => bound,
            Err(error) => {
                eprintln!("Agent UI HTTP bridge could not adopt loopback listener: {error}");
                return;
            }
        };
        let path = agent_ui_endpoint::endpoint_path();
        if let Ok(existing) = agent_ui_endpoint::read_endpoint_file(&path) {
            if existing.pid != std::process::id() && agent_ui_endpoint::port_is_live(existing.port) {
                eprintln!(
                    "Agent UI replacing a live loopback endpoint at {}",
                    existing.url
                );
            }
        }
        let endpoint = AgentUiEndpoint {
            service: AGENT_UI_SERVICE.to_string(),
            bind: LOOPBACK_HOST.to_string(),
            port,
            url: agent_ui_endpoint::loopback_url(port),
            pid: std::process::id(),
            auth_required: token.is_some(),
            token: token.clone(),
            token_header: Some("X-Genomics-Agent-Ui-Token".to_string()),
        };
        if let Err(error) = agent_ui_endpoint::write_endpoint_file(&path, &endpoint) {
            eprintln!("Agent UI endpoint file not written: {error}");
            return;
        }
        let _ = BOUND_PORT.set(port);
        let _ = ENDPOINT_PATH.set(path.clone());
        let url = agent_ui_endpoint::loopback_url(port);
        let auth_note = if token.is_some() {
            format!(" (token required for /ui/*; endpoint {})", path.display())
        } else {
            " (debug-only unauthenticated bypass enabled)".to_string()
        };
        eprintln!("Agent UI HTTP bridge listening on {url}{auth_note}");

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
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
                "service": AGENT_UI_SERVICE,
                "bind": LOOPBACK_HOST,
                "port": BOUND_PORT.get().copied().unwrap_or(0),
                "pid": std::process::id(),
                "windows": app.webview_windows().len(),
                "auth_required": session_token().is_some(),
            })
            .to_string(),
        );
    }

    if path.starts_with("/ui/") {
        if let Err(e) = authorize_ui(req) {
            return (401, json!({"ok": false, "error": e}).to_string());
        }
    }

    if method == "GET" && path == "/ui/snapshot" {
        return match invoke_ui(app, "snapshot", None).await {
            Ok(v) => (200, v.to_string()),
            Err(e) => (504, json!({"ok": false, "error": e}).to_string()),
        };
    }

    if method == "POST" && path == "/ui/capture" {
        return match crate::agent_ui_capture::capture_main_window_png(app).await {
            Ok(png) => {
                let png_base64 =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png);
                (
                    200,
                    json!({
                        "ok": true,
                        "mime": "image/png",
                        "byte_len": png.len(),
                        "png_base64": png_base64,
                    })
                    .to_string(),
                )
            }
            Err(error) => (501, json!({ "ok": false, "error": error }).to_string()),
        };
    }

    if method == "POST" && path == "/ui/resize" {
        let body = req.split("\r\n\r\n").nth(1).unwrap_or("{}").trim();
        let args: Value = serde_json::from_str(if body.is_empty() { "{}" } else { body })
            .unwrap_or_else(|_| json!({}));
        let Some(width) = args.get("width").and_then(Value::as_u64) else {
            return (400, json!({"ok": false, "error": "width must be a positive integer"}).to_string());
        };
        let Some(height) = args.get("height").and_then(Value::as_u64) else {
            return (400, json!({"ok": false, "error": "height must be a positive integer"}).to_string());
        };
        if !(640..=3_840).contains(&width) || !(480..=2_160).contains(&height) {
            return (
                400,
                json!({"ok": false, "error": "desktop QA size must be within 640..3840 by 480..2160"}).to_string(),
            );
        }
        let Some(window) = app.get_webview_window("main") else {
            return (504, json!({"ok": false, "error": "main Genomics Caddy window is unavailable"}).to_string());
        };
        let size = Size::Physical(PhysicalSize {
            width: width as u32,
            height: height as u32,
        });
        let result = window
            .unmaximize()
            .and_then(|()| window.set_size(size));
        return match result {
            Ok(()) => (200, json!({"ok": true, "width": width, "height": height}).to_string()),
            Err(error) => (500, json!({"ok": false, "error": format!("could not resize desktop QA window: {error}")}).to_string()),
        };
    }

    if method == "POST" && path.starts_with("/ui/") {
        let ui_method = path.trim_start_matches("/ui/");
        let body = req.split("\r\n\r\n").nth(1).unwrap_or("{}").trim();
        let args: Value = serde_json::from_str(if body.is_empty() { "{}" } else { body })
            .unwrap_or_else(|_| json!({}));
        let args = if ui_method == "setTab" {
            if let Some(tab) = args.get("tab").cloned() {
                tab
            } else if let Some(arg) = args.get("arg").cloned() {
                arg
            } else {
                args
            }
        } else if matches!(ui_method, "clickText" | "focusText" | "clickSection" | "queryText") {
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
                "POST /ui/focusText",
                "POST /ui/pressKey",
                "POST /ui/probeTooltips",
                "POST /ui/probeContrast",
                "POST /ui/clickSection",
                "POST /ui/queryText",
                "POST /ui/resize",
                "POST /ui/capture"
            ]
        })
        .to_string(),
    )
}
