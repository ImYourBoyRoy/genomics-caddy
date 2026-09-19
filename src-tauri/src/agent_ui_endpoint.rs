// ./src-tauri/src/agent_ui_endpoint.rs
/*
Purpose: Discover and advertise the local agent-UI HTTP bridge securely.
How to use: bind 127.0.0.1 with an ephemeral port, write a 0600 endpoint file,
            and have local tools read that file instead of guessing localhost.
Inputs: optional GENOMICS_AGENT_UI_PORT; XDG_RUNTIME_DIR / LOCALAPPDATA.
Outputs: agent-ui.json (loopback URL + optional token). Never bind 0.0.0.0.
Notes: `localhost` is not used — it often resolves to ::1 while we listen on IPv4.
*/

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

pub const LOOPBACK_HOST: &str = "127.0.0.1";
pub const APP_RUNTIME_DIRNAME: &str = "com.dna.explorer";
pub const AGENT_UI_ENDPOINT_NAME: &str = "agent-ui.json";
pub const AGENT_UI_SERVICE: &str = "genomics-caddy-agent-ui";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentUiEndpoint {
    pub service: String,
    pub bind: String,
    pub port: u16,
    pub url: String,
    pub pid: u32,
    #[serde(default)]
    pub auth_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_header: Option<String>,
}

pub fn loopback_url(port: u16) -> String {
    format!("http://{LOOPBACK_HOST}:{port}")
}

pub fn is_loopback_hostname(host: &str) -> bool {
    let host = host.trim().trim_matches(|c| c == '[' || c == ']');
    host.eq_ignore_ascii_case("localhost") || host == LOOPBACK_HOST || host == "::1"
}

pub fn canonicalize_loopback_url(raw: &str) -> Result<String, String> {
    let parsed = Url::parse(raw).map_err(|e| format!("invalid URL: {e}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("agent UI URL must be http(s) on loopback".into());
    }
    let host = parsed.host_str().ok_or_else(|| "agent UI URL is missing a host".to_string())?;
    if host == "0.0.0.0" || host == "::" || !is_loopback_hostname(host) {
        return Err(format!(
            "refusing non-loopback agent UI URL host {host}; use 127.0.0.1"
        ));
    }
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "agent UI URL is missing a port".to_string())?;
    Ok(loopback_url(port))
}

pub fn runtime_dir_from(xdg_runtime_dir: Option<&str>, local_app_data: Option<&str>) -> PathBuf {
    if let Some(dir) = xdg_runtime_dir.map(str::trim).filter(|d| !d.is_empty()) {
        return PathBuf::from(dir).join(APP_RUNTIME_DIRNAME);
    }
    if let Some(dir) = local_app_data.map(str::trim).filter(|d| !d.is_empty()) {
        return PathBuf::from(dir).join(APP_RUNTIME_DIRNAME).join("run");
    }
    std::env::temp_dir().join(format!("{APP_RUNTIME_DIRNAME}-{}", current_user_token()))
}

pub fn runtime_dir() -> PathBuf {
    runtime_dir_from(
        std::env::var("XDG_RUNTIME_DIR").ok().as_deref(),
        std::env::var("LOCALAPPDATA").ok().as_deref(),
    )
}

pub fn endpoint_path() -> PathBuf {
    runtime_dir().join(AGENT_UI_ENDPOINT_NAME)
}

fn current_user_token() -> String {
    #[cfg(unix)]
    {
        // SAFETY: getuid has no preconditions.
        unsafe { libc::getuid() }.to_string()
    }
    #[cfg(not(unix))]
    {
        std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "user".into())
    }
}

pub fn preferred_bind_port(raw: Option<&str>) -> Option<u16> {
    let value = raw?.trim();
    if value.is_empty() || value == "0" {
        return None;
    }
    value.parse::<u16>().ok().filter(|port| *port != 0)
}

pub fn bind_loopback_std(preferred: Option<u16>) -> io::Result<(TcpListener, u16)> {
    let listener = TcpListener::bind((LOOPBACK_HOST, preferred.unwrap_or(0)))?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    Ok((listener, port))
}

pub fn generate_session_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("failed to generate agent UI token: {e}"))?;
    Ok(hex::encode(bytes))
}

pub fn write_endpoint_file(path: &Path, endpoint: &AgentUiEndpoint) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(endpoint)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn read_endpoint_file(path: &Path) -> Result<AgentUiEndpoint, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("endpoint file unreadable: {e}"))?;
    let parsed: AgentUiEndpoint =
        serde_json::from_str(&raw).map_err(|e| format!("endpoint file is not JSON: {e}"))?;
    if parsed.service != AGENT_UI_SERVICE {
        return Err("endpoint file is not the Genomics Caddy agent UI".into());
    }
    if parsed.bind != LOOPBACK_HOST {
        return Err(format!("endpoint file bind {} is not 127.0.0.1", parsed.bind));
    }
    let url = canonicalize_loopback_url(&parsed.url)?;
    if loopback_url(parsed.port) != url {
        return Err("endpoint file URL does not match its port".into());
    }
    Ok(AgentUiEndpoint { url, ..parsed })
}

pub fn port_is_live(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

pub fn remove_endpoint_file(path: &Path) {
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener as StdListener;

    #[test]
    fn loopback_url_never_uses_localhost() {
        let url = loopback_url(48123);
        assert_eq!(url, "http://127.0.0.1:48123");
        assert!(!url.contains("localhost"));
    }

    #[test]
    fn canonicalize_rewrites_localhost_and_rejects_lan() {
        assert_eq!(
            canonicalize_loopback_url("http://localhost:1420").unwrap(),
            "http://127.0.0.1:1420"
        );
        assert_eq!(
            canonicalize_loopback_url("http://[::1]:48200/").unwrap(),
            "http://127.0.0.1:48200"
        );
        assert!(canonicalize_loopback_url("http://0.0.0.0:1420").is_err());
        assert!(canonicalize_loopback_url("http://192.168.1.21:17321").is_err());
    }

    #[test]
    fn runtime_dir_prefers_xdg() {
        let dir = runtime_dir_from(Some("/run/user/1000"), Some(r"C:\Users\x\AppData\Local"));
        assert_eq!(dir, PathBuf::from("/run/user/1000").join(APP_RUNTIME_DIRNAME));
    }

    #[test]
    fn preferred_port_treats_zero_as_ephemeral() {
        assert_eq!(preferred_bind_port(None), None);
        assert_eq!(preferred_bind_port(Some("")), None);
        assert_eq!(preferred_bind_port(Some("0")), None);
        assert_eq!(preferred_bind_port(Some("17321")), Some(17321));
    }

    #[test]
    fn bind_loopback_uses_ephemeral_port_on_ipv4() {
        let (listener, port) = bind_loopback_std(None).expect("bind loopback");
        assert!(port > 0);
        let addr = listener.local_addr().expect("local addr");
        assert_eq!(addr.ip().to_string(), LOOPBACK_HOST);
        drop(listener);
    }

    #[test]
    fn endpoint_file_roundtrip_is_user_only_and_loopback() {
        let dir = std::env::temp_dir().join(format!("gc-endpoint-test-{}", std::process::id()));
        let path = dir.join(AGENT_UI_ENDPOINT_NAME);
        let endpoint = AgentUiEndpoint {
            service: AGENT_UI_SERVICE.into(),
            bind: LOOPBACK_HOST.into(),
            port: 48201,
            url: loopback_url(48201),
            pid: std::process::id(),
            auth_required: true,
            token: Some("n".repeat(64)),
            token_header: Some("X-Genomics-Agent-Ui-Token".into()),
        };
        write_endpoint_file(&path, &endpoint).expect("write endpoint");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
            let dir_mode = fs::metadata(&dir).expect("dir meta").permissions().mode() & 0o777;
            assert_eq!(dir_mode, 0o700);
        }
        let parsed = read_endpoint_file(&path).expect("read endpoint");
        assert_eq!(parsed.url, "http://127.0.0.1:48201");
        assert_eq!(parsed.token.as_deref(), Some(endpoint.token.as_deref().unwrap()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_endpoint_file_rejects_wrong_service() {
        let dir = std::env::temp_dir().join(format!("gc-endpoint-bad-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join(AGENT_UI_ENDPOINT_NAME);
        fs::write(
            &path,
            r#"{"service":"nope","bind":"127.0.0.1","port":17321,"url":"http://127.0.0.1:17321","pid":1}"#,
        )
        .expect("write");
        assert!(read_endpoint_file(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn generate_session_token_is_64_hex_chars() {
        let token = generate_session_token().expect("token");
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn port_is_live_detects_bound_listener() {
        let listener = StdListener::bind((LOOPBACK_HOST, 0)).expect("bind");
        let port = listener.local_addr().expect("addr").port();
        assert!(port_is_live(port));
        drop(listener);
    }
}
