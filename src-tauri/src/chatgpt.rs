// ./src-tauri/src/chatgpt.rs
/*
Purpose: "Continue with ChatGPT" for the Ask AI consultation chat, using the
open-source ChatGPT plan-usage flow (OAuth 2.0 authorization code + PKCE + OIDC
against auth.openai.com, then the public Responses API).
Responsibilities:
- Persist a per-install host id and register/sign in through the system browser
  with a 127.0.0.1 loopback callback (no client secret, no API key).
- Validate state, nonce, and the ID token (RS256 via OpenAI's JWKS) and require
  the `chatgpt.tokens.use.direct` scope before enabling inference.
- Keep access/refresh/ID tokens in the OS credential manager only; the webview
  never receives a token. Refresh is serialized and replaces the token set whole.
- List account models and stream Responses API output as the same
  `ollama-chunk:<id>` / `ollama-done:<id>` events the Ollama path emits.
Key Inputs: Tauri commands from `src/lib/api/chatgpt.ts`.
Key Outputs: `ChatGptStatus` (never tokens), streamed text chunks, usage counts.
Operational Notes:
- Requests always use `store=false`, `stream=true`, and omit fields the preview
  route rejects (temperature, max_output_tokens, previous_response_id, ...).
- Raw genotype calls in a prompt leave the device when this provider is chosen;
  the UI discloses that destination. OpenAI documents this as a preview.
*/

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::stream_control;

const ISSUER: &str = "https://auth.openai.com";
const AUTHORIZE_URL: &str = "https://auth.openai.com/api/accounts/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/api/accounts/oauth/token";
const REVOKE_URL: &str = "https://auth.openai.com/api/accounts/oauth/revoke";
const JWKS_URL: &str = "https://auth.openai.com/.well-known/jwks.json";
const RESOURCE: &str = "https://api.openai.com/v1";
const AGENT_NAME: &str = "Genomics Caddy";
const SCOPES: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
const SHARING_SCOPE: &str = "chatgpt.tokens.use.direct";
const PREFERRED_CALLBACK_PORT: u16 = 1455;
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);
const REFRESH_SKEW_SECS: i64 = 120;
pub const USAGE_SETTINGS_URL: &str = "https://chatgpt.com/#settings/Usage";

const KEYRING_SERVICE: &str = "GenomicsCaddy";
const KEYRING_CREDENTIALS: &str = "chatgpt_credentials";
const HOST_FILE: &str = "chatgpt-host.json";

static SIGN_IN_CANCEL: AtomicBool = AtomicBool::new(false);
static REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Credentials {
    email: Option<String>,
    name: Option<String>,
    subject: String,
    client_id: String,
    ext_agent_host_id: String,
    id_token: String,
    access_token: String,
    refresh_token: Option<String>,
    scopes: Vec<String>,
    /// Unix seconds when `access_token` stops being valid.
    expires_at: i64,
    saved_at: i64,
}

impl Credentials {
    fn sharing(&self) -> bool {
        self.scopes.iter().any(|s| s == SHARING_SCOPE)
    }
}

/// Safe-to-serialize connection summary. Never contains a token.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ChatGptStatus {
    pub connected: bool,
    /// True only when the user granted ChatGPT plan usage (`chatgpt.tokens.use.direct`).
    pub sharing: bool,
    pub email: Option<String>,
    pub name: Option<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatGptModel {
    pub slug: String,
    pub display_name: String,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn status_of(creds: Option<&Credentials>) -> ChatGptStatus {
    match creds {
        Some(c) => ChatGptStatus {
            connected: true,
            sharing: c.sharing(),
            email: c.email.clone(),
            name: c.name.clone(),
            expires_at: Some(c.expires_at),
        },
        None => ChatGptStatus::default(),
    }
}

// ── Credential storage (OS credential manager only) ─────────────────────────

fn load_credentials() -> Option<Credentials> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_CREDENTIALS).ok()?;
    let raw = entry.get_password().ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_credentials(creds: &Credentials) -> Result<(), String> {
    let raw = serde_json::to_string(creds).map_err(|e| e.to_string())?;
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_CREDENTIALS)
        .and_then(|entry| entry.set_password(&raw))
        .map_err(|e| {
            format!(
                "Could not store the ChatGPT connection in the OS credential manager: {e}. \
                 Tokens are never written to disk in plain text."
            )
        })
}

fn clear_credentials() {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_CREDENTIALS) {
        let _ = entry.delete_credential();
    }
}

// ── Host id (stable per install, not a secret) ──────────────────────────────

#[derive(Serialize, Deserialize)]
struct HostFile {
    ext_agent_host_id: String,
}

fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).map_err(|e| format!("Secure random source unavailable: {e}"))?;
    Ok(buf)
}

fn random_token(len: usize) -> Result<String, String> {
    let bytes = random_bytes::<64>()?;
    Ok(URL_SAFE_NO_PAD.encode(&bytes[..len.min(64)]))
}

fn new_uuid_urn() -> Result<String, String> {
    let mut b = random_bytes::<16>()?;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = hex::encode(b);
    Ok(format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    ))
}

fn host_id() -> Result<String, String> {
    let path = crate::paths::resolve_data_dir().join(HOST_FILE);
    if let Ok(raw) = std::fs::read_to_string(&path)
        && let Ok(file) = serde_json::from_str::<HostFile>(&raw)
        && file.ext_agent_host_id.starts_with("urn:uuid:")
    {
        return Ok(file.ext_agent_host_id);
    }
    let id = new_uuid_urn()?;
    let body = serde_json::to_string(&HostFile {
        ext_agent_host_id: id.clone(),
    })
    .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new(".")))
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| format!("Could not save the ChatGPT host id: {e}"))?;
    Ok(id)
}

// ── OAuth helpers ───────────────────────────────────────────────────────────

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: Option<i64>,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct OAuthError {
    error: Option<String>,
    error_description: Option<String>,
}

fn is_terminal_refresh_error(code: &str) -> bool {
    matches!(
        code,
        "invalid_grant"
            | "invalid_refresh_token"
            | "token_expired"
            | "refresh_token_expired"
            | "refresh_token_invalidated"
            | "refresh_token_reused"
    )
}

async fn post_token(form: &[(&str, &str)]) -> Result<TokenResponse, (String, String)> {
    let client = http_client().map_err(|e| ("client".to_string(), e))?;
    let res = client
        .post(TOKEN_URL)
        .form(form)
        .send()
        .await
        .map_err(|e| ("network".to_string(), format!("Could not reach OpenAI: {e}")))?;
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    if status.is_success() {
        return serde_json::from_str::<TokenResponse>(&body)
            .map_err(|e| ("parse".to_string(), format!("Unexpected token response: {e}")));
    }
    let parsed: Option<OAuthError> = serde_json::from_str(&body).ok();
    let code = parsed
        .as_ref()
        .and_then(|p| p.error.clone())
        .unwrap_or_else(|| format!("http_{}", status.as_u16()));
    let detail = parsed
        .and_then(|p| p.error_description)
        .unwrap_or_else(|| format!("HTTP {status}"));
    Err((code, detail))
}

#[derive(Debug, Deserialize)]
struct IdClaims {
    sub: String,
    nonce: Option<String>,
    email: Option<String>,
    name: Option<String>,
}

async fn verify_id_token(
    id_token: &str,
    client_id: &str,
    expected_nonce: Option<&str>,
) -> Result<IdClaims, String> {
    use jsonwebtoken::jwk::JwkSet;
    use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};

    let header = decode_header(id_token).map_err(|e| format!("Unreadable ID token: {e}"))?;
    let kid = header
        .kid
        .ok_or_else(|| "ID token has no key id".to_string())?;
    let jwks: JwkSet = http_client()?
        .get(JWKS_URL)
        .send()
        .await
        .map_err(|e| format!("Could not fetch OpenAI signing keys: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Unreadable OpenAI signing keys: {e}"))?;
    let jwk = jwks
        .find(&kid)
        .ok_or_else(|| "ID token signed with an unknown key".to_string())?;
    let key = DecodingKey::from_jwk(jwk).map_err(|e| format!("Unusable signing key: {e}"))?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[client_id]);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    let claims = decode::<IdClaims>(id_token, &key, &validation)
        .map_err(|e| format!("ID token failed validation: {e}"))?
        .claims;
    if let Some(expected) = expected_nonce
        && claims.nonce.as_deref() != Some(expected)
    {
        return Err("ID token nonce did not match this sign-in attempt".into());
    }
    Ok(claims)
}

fn scopes_from(response: &TokenResponse) -> Vec<String> {
    response
        .scope
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

// ── Loopback callback ───────────────────────────────────────────────────────

struct Callback {
    code: Option<String>,
    state: Option<String>,
    client_id: Option<String>,
    error: Option<String>,
}

const CALLBACK_PAGE: &str = "<!doctype html><meta charset=utf-8><title>Genomics Caddy</title>\
<body style=\"font:16px system-ui;background:#0b1117;color:#e6edf3;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center;max-width:26rem\"><h1 style=\"font-size:1.25rem\">Connected to Genomics Caddy</h1>\
<p style=\"color:#9aa7b4\">You can close this tab and return to the app.</p></div></body>";

async fn wait_for_callback(listener: TcpListener) -> Result<Callback, String> {
    let deadline = tokio::time::Instant::now() + SIGN_IN_TIMEOUT;
    loop {
        if SIGN_IN_CANCEL.load(Ordering::SeqCst) {
            return Err("Sign-in cancelled.".into());
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("Sign-in timed out. Start again when you are ready.".into());
        }
        let accept = tokio::time::timeout(
            remaining.min(Duration::from_millis(400)),
            listener.accept(),
        )
        .await;
        let Ok(Ok((mut socket, _))) = accept else {
            continue;
        };
        let mut buf = vec![0u8; 8192];
        let n = tokio::time::timeout(Duration::from_secs(5), socket.read(&mut buf))
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let target = request
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_string();
        let Ok(url) = url::Url::parse(&format!("http://127.0.0.1{target}")) else {
            continue;
        };
        if url.path() != "/auth/callback" {
            let _ = socket
                .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await;
            continue;
        }
        let get = |key: &str| {
            url.query_pairs()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.into_owned())
        };
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
            CALLBACK_PAGE.len(),
            CALLBACK_PAGE
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
        return Ok(Callback {
            code: get("code"),
            state: get("state"),
            client_id: get("client_id"),
            error: get("error"),
        });
    }
}

// ── Public commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn chatgpt_status() -> ChatGptStatus {
    status_of(load_credentials().as_ref())
}

#[tauri::command]
pub fn chatgpt_cancel_sign_in() {
    SIGN_IN_CANCEL.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn chatgpt_sign_in(app: AppHandle) -> Result<ChatGptStatus, String> {
    SIGN_IN_CANCEL.store(false, Ordering::SeqCst);
    let existing = load_credentials();
    let host = host_id()?;

    // Bind first so the redirect URI is known before the browser opens.
    let listener = match TcpListener::bind(("127.0.0.1", PREFERRED_CALLBACK_PORT)).await {
        Ok(l) => l,
        Err(_) => TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| format!("Could not open a local callback port: {e}"))?,
    };
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/auth/callback");

    let state = random_token(32)?;
    let nonce = random_token(32)?;
    let verifier = random_token(64)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    let reauth_client = existing.as_ref().map(|c| c.client_id.clone());
    let mut url = url::Url::parse(AUTHORIZE_URL).map_err(|e| e.to_string())?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair(
            "client_id",
            reauth_client.as_deref().unwrap_or("dynamic_agent_client"),
        );
        if reauth_client.is_none() {
            q.append_pair("agent_name_hint", AGENT_NAME);
        }
        q.append_pair("ext_agent_host_id", &host)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("scope", SCOPES)
            .append_pair("resource", RESOURCE)
            .append_pair("state", &state)
            .append_pair("nonce", &nonce)
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", &challenge);
        if let Some(c) = existing.as_ref() {
            q.append_pair("id_token_hint", &c.id_token);
            if let Some(email) = c.email.as_deref() {
                q.append_pair("login_hint", email);
            }
        }
    }

    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|e| format!("Could not open your browser: {e}"))?;

    let callback = wait_for_callback(listener).await?;
    if callback.state.as_deref() != Some(state.as_str()) {
        return Err("Sign-in response did not match this attempt (state mismatch).".into());
    }
    if let Some(err) = callback.error {
        return Err(if err == "access_denied" {
            "ChatGPT sign-in was declined. Nothing was connected.".to_string()
        } else {
            format!("ChatGPT sign-in failed: {err}")
        });
    }
    let code = callback
        .code
        .ok_or_else(|| "Sign-in response had no authorization code.".to_string())?;

    // A returning registration keeps its client id; a mismatch is rejected.
    let client_id = match (&reauth_client, callback.client_id) {
        (Some(saved), Some(returned)) if *saved != returned => {
            return Err("Sign-in returned a different client than the saved connection.".into());
        }
        (Some(saved), _) => saved.clone(),
        (None, Some(returned)) if returned != "dynamic_agent_client" => returned,
        (None, _) => return Err("Registration did not return a client id. Try again.".into()),
    };

    let token = post_token(&[
        ("grant_type", "authorization_code"),
        ("client_id", &client_id),
        ("code", &code),
        ("code_verifier", &verifier),
        ("redirect_uri", &redirect_uri),
        ("resource", RESOURCE),
    ])
    .await
    .map_err(|(code, detail)| format!("ChatGPT token exchange failed ({code}): {detail}"))?;

    let id_token = token
        .id_token
        .clone()
        .ok_or_else(|| "ChatGPT did not return a verifiable identity.".to_string())?;
    let claims = verify_id_token(&id_token, &client_id, Some(&nonce)).await?;
    if let Some(prev) = existing.as_ref()
        && prev.subject != claims.sub
    {
        return Err("Signed in as a different ChatGPT account than the saved connection. Disconnect first to switch accounts.".into());
    }

    let scopes = scopes_from(&token);
    let creds = Credentials {
        email: claims.email,
        name: claims.name,
        subject: claims.sub,
        client_id,
        ext_agent_host_id: host,
        id_token,
        access_token: token.access_token,
        refresh_token: token
            .refresh_token
            .or_else(|| existing.and_then(|c| c.refresh_token)),
        scopes,
        expires_at: now_secs() + token.expires_in.unwrap_or(3600),
        saved_at: now_secs(),
    };
    save_credentials(&creds)?;
    Ok(status_of(Some(&creds)))
}

#[tauri::command]
pub async fn chatgpt_sign_out() -> Result<ChatGptStatus, String> {
    if let Some(creds) = load_credentials() {
        if let Some(refresh) = creds.refresh_token.as_deref() {
            let revoked = http_client()?
                .post(REVOKE_URL)
                .form(&[
                    ("token", refresh),
                    ("token_type_hint", "refresh_token"),
                    ("client_id", creds.client_id.as_str()),
                ])
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            if !revoked {
                crate::app_log::log("chatgpt", 
                    "ChatGPT remote revocation was not confirmed; local tokens were cleared.",
                );
            }
        }
    }
    clear_credentials();
    Ok(ChatGptStatus::default())
}

#[tauri::command]
pub fn chatgpt_open_usage(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(USAGE_SETTINGS_URL, None::<&str>)
        .map_err(|e| format!("Could not open ChatGPT usage settings: {e}"))
}

/// Return a usable access token, refreshing (serialized) when near expiry.
async fn fresh_credentials(force: bool) -> Result<Credentials, String> {
    let _guard = REFRESH_LOCK.lock().await;
    let creds = load_credentials().ok_or_else(|| "Not signed in with ChatGPT.".to_string())?;
    if !creds.sharing() {
        return Err("ChatGPT plan usage is not enabled. Reconnect and allow plan usage.".into());
    }
    if !force && creds.expires_at - now_secs() > REFRESH_SKEW_SECS {
        return Ok(creds);
    }
    let refresh = creds.refresh_token.clone().ok_or_else(|| {
        "ChatGPT session expired and cannot be renewed. Sign in again.".to_string()
    })?;
    match post_token(&[
        ("grant_type", "refresh_token"),
        ("client_id", &creds.client_id),
        ("refresh_token", &refresh),
        ("resource", RESOURCE),
    ])
    .await
    {
        Ok(token) => {
            let mut next = creds;
            let scopes = scopes_from(&token);
            next.access_token = token.access_token;
            next.refresh_token = token.refresh_token.or(Some(refresh));
            if let Some(id) = token.id_token {
                next.id_token = id;
            }
            if !scopes.is_empty() {
                next.scopes = scopes;
            }
            next.expires_at = now_secs() + token.expires_in.unwrap_or(3600);
            next.saved_at = now_secs();
            save_credentials(&next)?;
            Ok(next)
        }
        Err((code, _)) if is_terminal_refresh_error(&code) => {
            clear_credentials();
            Err(format!(
                "ChatGPT connection was disconnected or expired ({code}). Sign in again."
            ))
        }
        Err((_, detail)) => Err(format!("Could not renew the ChatGPT session: {detail}")),
    }
}

#[tauri::command]
pub async fn chatgpt_list_models() -> Result<Vec<ChatGptModel>, String> {
    let creds = fresh_credentials(false).await?;
    let res = http_client()?
        .get(format!("{RESOURCE}/models"))
        .bearer_auth(&creds.access_token)
        .send()
        .await
        .map_err(|e| format!("Could not reach OpenAI: {e}"))?;
    let status = res.status();
    let body: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
    if !status.is_success() {
        return Err(describe_http_error(status.as_u16(), &body));
    }
    let models = body
        .get("models")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(models
        .iter()
        .filter(|m| m.get("visibility").and_then(|v| v.as_str()) == Some("list"))
        .filter_map(|m| {
            let slug = m.get("slug")?.as_str()?.to_string();
            let display_name = m
                .get("display_name")
                .and_then(|v| v.as_str())
                .unwrap_or(&slug)
                .to_string();
            Some(ChatGptModel { slug, display_name })
        })
        .collect())
}

fn describe_http_error(status: u16, body: &serde_json::Value) -> String {
    let code = body
        .pointer("/error/code")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let detail = body
        .pointer("/error/message")
        .or_else(|| body.get("detail"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    describe_error_code(status, code, detail)
}

fn describe_error_code(status: u16, code: &str, detail: &str) -> String {
    let friendly = match code {
        "subscription_sharing_usage_limit_exceeded" => Some(
            "ChatGPT usage limit reached. Open ChatGPT Settings → Usage to review your plan or this app's limit.",
        ),
        "subscription_sharing_usage_unavailable" | "subscription_sharing_user_unavailable" => Some(
            "ChatGPT usage could not be checked right now. Your connection is intact; try again shortly.",
        ),
        "subscription_sharing_user_not_eligible" => Some(
            "ChatGPT plan usage is not available for this account, workspace, or policy.",
        ),
        "subscription_sharing_invalid_user" => {
            Some("ChatGPT could not validate this account. Disconnect and sign in again.")
        }
        _ => None,
    };
    if let Some(text) = friendly {
        return text.to_string();
    }
    let mut message = match status {
        401 => "ChatGPT did not accept the connection (401). Sign in again.".to_string(),
        403 => "ChatGPT blocked this request (403), for example by region or policy.".to_string(),
        503 => "ChatGPT direct routing is temporarily unavailable (503). Try again shortly.".to_string(),
        s => format!("ChatGPT returned HTTP {s}."),
    };
    if !code.is_empty() {
        message.push_str(&format!(" [{code}]"));
    }
    if !detail.is_empty() {
        let short: String = detail.chars().take(240).collect();
        message.push_str(&format!(" {short}"));
    }
    message
}

/// OpenAI Responses API hard cap on the `instructions` field.
const INSTRUCTIONS_MAX_CHARS: usize = 1_048_576;
const JSON_CONTEXT_MARKER: &str = "[JSON CONTEXT]";

/// Split a Genomics Caddy system prompt so genomic JSON is not stuffed into
/// `instructions` (1MB cap). Rules stay in instructions; `[JSON CONTEXT]…`
/// becomes a leading reference user message in `input`.
fn split_system_for_responses(system: &str) -> (String, Option<String>) {
    if let Some(idx) = system.find(JSON_CONTEXT_MARKER) {
        let instructions = system[..idx].trim_end().to_string();
        let context = system[idx..].trim().to_string();
        (
            instructions,
            if context.is_empty() {
                None
            } else {
                Some(context)
            },
        )
    } else {
        (system.to_string(), None)
    }
}

fn clamp_instructions(instructions: String) -> Result<String, String> {
    let len = instructions.chars().count();
    if len <= INSTRUCTIONS_MAX_CHARS {
        return Ok(instructions);
    }
    Err(format!(
        "ChatGPT instructions are {len} characters; OpenAI allows at most {INSTRUCTIONS_MAX_CHARS}. \
Switch Genomic Data Context away from Developer raw JSON / Full selected packs, \
or reduce selected packs, then try again."
    ))
}

/// Convert Ollama-style chat messages into a Responses API body fragment.
fn build_input(
    messages: &[serde_json::Value],
) -> Result<(String, Vec<serde_json::Value>), String> {
    let mut instructions: Vec<String> = Vec::new();
    let mut context_blocks: Vec<String> = Vec::new();
    let mut input: Vec<serde_json::Value> = Vec::new();
    for m in messages {
        let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("user");
        let text = m
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string();
        if role == "system" {
            if text.trim().is_empty() {
                continue;
            }
            let (instr, ctx) = split_system_for_responses(&text);
            if !instr.trim().is_empty() {
                instructions.push(instr);
            }
            if let Some(ctx) = ctx {
                context_blocks.push(ctx);
            }
            continue;
        }
        let images: Vec<&str> = m
            .get("images")
            .and_then(|i| i.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        if role == "user" && !images.is_empty() {
            let mut parts = vec![serde_json::json!({ "type": "input_text", "text": text })];
            for img in images {
                let url = if img.starts_with("data:") {
                    img.to_string()
                } else {
                    format!("data:image/png;base64,{img}")
                };
                parts.push(serde_json::json!({ "type": "input_image", "image_url": url }));
            }
            input.push(serde_json::json!({ "role": "user", "content": parts }));
        } else {
            let role = if role == "assistant" { "assistant" } else { "user" };
            input.push(serde_json::json!({ "role": role, "content": text }));
        }
    }

    let mut final_input = Vec::with_capacity(context_blocks.len() + input.len());
    for ctx in context_blocks {
        final_input.push(serde_json::json!({
            "role": "user",
            "content": format!(
                "Reference genomic context for this consultation (not a user question):\n\n{ctx}"
            ),
        }));
    }
    final_input.extend(input);

    let joined = clamp_instructions(instructions.join("\n\n"))?;
    Ok((joined, final_input))
}

#[tauri::command]
pub async fn stream_chatgpt_chat(
    app: AppHandle,
    stream_id: String,
    model: String,
    messages: Vec<serde_json::Value>,
) -> Result<(), String> {
    stream_control::validate_stream_id(&stream_id)?;
    stream_control::reset_ollama_stream(&stream_id);
    let chunk_event = stream_control::ollama_chunk_event(&stream_id);
    let done_event = stream_control::ollama_done_event(&stream_id);

    let (instructions, input) = build_input(&messages)?;
    let mut body = serde_json::json!({
        "model": model,
        "input": input,
        "store": false,
        "stream": true,
    });
    if !instructions.is_empty() {
        body["instructions"] = serde_json::Value::String(instructions);
    }

    let client = http_client()?;
    let mut creds = fresh_credentials(false).await?;
    let mut res = None;
    for attempt in 0..2 {
        let r = client
            .post(format!("{RESOURCE}/responses"))
            .bearer_auth(&creds.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Could not reach OpenAI: {e}"))?;
        if r.status().as_u16() == 401 && attempt == 0 {
            creds = fresh_credentials(true).await?;
            continue;
        }
        res = Some(r);
        break;
    }
    let res = res.ok_or_else(|| "ChatGPT did not accept the connection.".to_string())?;
    if !res.status().is_success() {
        let status = res.status().as_u16();
        let value: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        return Err(describe_http_error(status, &value));
    }

    let mut stream = res.bytes_stream();
    let mut buffer = String::new();
    let mut completed = false;
    let mut prompt_tokens: Option<u64> = None;
    let mut response_tokens: Option<u64> = None;

    while let Some(chunk) = stream.next().await {
        if stream_control::is_ollama_stream_cancelled() {
            app.emit(&done_event, serde_json::json!({ "cancelled": true })).ok();
            return Ok(());
        }
        let chunk = chunk.map_err(|e| format!("Stream error: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buffer.find("\n\n") {
            let block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();
            let data: String = block
                .lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .map(|l| l.trim_start())
                .collect::<Vec<_>>()
                .join("\n");
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let Ok(event) = serde_json::from_str::<serde_json::Value>(&data) else {
                continue;
            };
            match event.get("type").and_then(|t| t.as_str()).unwrap_or_default() {
                "response.output_text.delta" => {
                    if let Some(delta) = event.get("delta").and_then(|d| d.as_str())
                        && !delta.is_empty()
                    {
                        app.emit(&chunk_event, delta).ok();
                    }
                }
                "response.completed" => {
                    completed = true;
                    prompt_tokens = event
                        .pointer("/response/usage/input_tokens")
                        .and_then(|v| v.as_u64());
                    response_tokens = event
                        .pointer("/response/usage/output_tokens")
                        .and_then(|v| v.as_u64());
                }
                "response.failed" | "error" => {
                    let code = event
                        .pointer("/response/error/code")
                        .or_else(|| event.pointer("/error/code"))
                        .or_else(|| event.get("code"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown_error");
                    let detail = event
                        .pointer("/response/error/message")
                        .or_else(|| event.pointer("/error/message"))
                        .or_else(|| event.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    return Err(describe_error_code(0, code, detail));
                }
                "response.incomplete" => {
                    let reason = event
                        .pointer("/response/incomplete_details/reason")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    return Err(format!("ChatGPT stopped early ({reason})."));
                }
                _ => {}
            }
        }
    }

    if !completed {
        return Err("ChatGPT stream ended before the response completed.".into());
    }
    app.emit(
        &done_event,
        serde_json::json!({
            "prompt_eval_count": prompt_tokens,
            "eval_count": response_tokens,
        }),
    )
    .ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_urn_is_v4_shaped() {
        let id = new_uuid_urn().unwrap();
        assert!(id.starts_with("urn:uuid:"));
        let raw = &id["urn:uuid:".len()..];
        assert_eq!(raw.len(), 36);
        assert_eq!(&raw[14..15], "4");
    }

    #[test]
    fn pkce_challenge_matches_rfc7636_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn system_messages_become_instructions_and_images_become_parts() {
        let messages = vec![
            serde_json::json!({ "role": "system", "content": "be careful" }),
            serde_json::json!({ "role": "user", "content": "hi", "images": ["AAAA"] }),
            serde_json::json!({ "role": "assistant", "content": "hello" }),
        ];
        let (instructions, input) = build_input(&messages).unwrap();
        assert_eq!(instructions, "be careful");
        assert_eq!(input.len(), 2);
        assert_eq!(input[0]["content"][1]["type"], "input_image");
        assert_eq!(input[1]["role"], "assistant");
    }

    #[test]
    fn json_context_moves_out_of_instructions_into_input() {
        let system = "[SYSTEM INSTRUCTIONS]\nrules here\n\n[JSON CONTEXT]\n{\"sections\":[]}";
        let messages = vec![
            serde_json::json!({ "role": "system", "content": system }),
            serde_json::json!({ "role": "user", "content": "explain MTHFR" }),
        ];
        let (instructions, input) = build_input(&messages).unwrap();
        assert!(instructions.contains("rules here"));
        assert!(!instructions.contains("[JSON CONTEXT]"));
        assert!(instructions.chars().count() < 1_000);
        assert_eq!(input.len(), 2);
        assert!(input[0]["content"]
            .as_str()
            .unwrap()
            .contains("[JSON CONTEXT]"));
        assert_eq!(input[1]["content"], "explain MTHFR");
    }

    #[test]
    fn oversized_instructions_without_json_marker_fail_closed() {
        let huge = "x".repeat(INSTRUCTIONS_MAX_CHARS + 10);
        let messages = vec![serde_json::json!({ "role": "system", "content": huge })];
        let err = build_input(&messages).unwrap_err();
        assert!(err.contains("OpenAI allows at most"));
    }

    #[test]
    fn usage_limit_error_points_to_usage_settings() {
        let msg = describe_error_code(429, "subscription_sharing_usage_limit_exceeded", "");
        assert!(msg.contains("Usage"));
    }

    #[test]
    fn refresh_errors_are_classified_terminal() {
        assert!(is_terminal_refresh_error("invalid_grant"));
        assert!(is_terminal_refresh_error("refresh_token_reused"));
        assert!(!is_terminal_refresh_error("network"));
    }

    #[test]
    fn tokens_never_serialize_into_status() {
        let creds = Credentials {
            email: Some("a@b.c".into()),
            name: None,
            subject: "sub".into(),
            client_id: "oaiapp_x".into(),
            ext_agent_host_id: "urn:uuid:x".into(),
            id_token: "SECRET_ID".into(),
            access_token: "SECRET_ACCESS".into(),
            refresh_token: Some("SECRET_REFRESH".into()),
            scopes: vec![SHARING_SCOPE.into()],
            expires_at: 1,
            saved_at: 1,
        };
        let json = serde_json::to_string(&status_of(Some(&creds))).unwrap();
        assert!(!json.contains("SECRET"));
        assert!(json.contains("\"sharing\":true"));
    }
}
