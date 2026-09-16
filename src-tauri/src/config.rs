// ./src-tauri/src/config.rs
/*
Module Docstring:
Purpose: Application configuration, .env loading, and secure secret storage.
Responsibilities:
- Load optional `.env` from the directory beside the executable (and project root in dev).
- Store API keys and tokens in the OS credential manager (keyring), not plaintext SQLite.
- Merge SQLite settings with keyring/env fallbacks for Qdrant and NCBI credentials.
Key Inputs: `.env` file, SQLite qdrant_config row, keyring entries.
Key Outputs: Resolved QdrantConfig and Ollama token values for backend commands.
Operational Notes: Plaintext secrets in SQLite are migrated to keyring on first init.
*/

use crate::research::{
    EnrichmentSourcesConfig, QdrantConfig, QdrantConfigPublic, QdrantConfigUpdate,
    ResearchScopeConfig,
};
use rusqlite::{Connection, params};
use std::collections::HashSet;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

const KEYRING_SERVICE: &str = "GenomicsCaddy";
const SECRET_QDRANT_API_KEY: &str = "qdrant_api_key";
const SECRET_NCBI_API_KEY: &str = "ncbi_api_key";
const SECRET_OLLAMA_TOKEN: &str = "ollama_token";
/// Directories searched for `.env` (first existing file wins).
pub fn app_config_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        roots.push(parent.to_path_buf());
    }
    let project_root = crate::paths::resolve_project_root(None);
    roots.push(crate::paths::app_layout_dir(&project_root));
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR")
        && let Some(project_root) = PathBuf::from(manifest).parent()
    {
        roots.push(project_root.to_path_buf());
    }
    roots
}

/// Path where operators should place `.env` (executable directory, or project root in dev).
pub fn recommended_env_path() -> PathBuf {
    app_config_roots()
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".env")
}

/// Load `.env` from the first config root that contains one.
pub fn init_env() {
    for root in app_config_roots() {
        let env_path = root.join(".env");
        if env_path.exists() {
            let _ = dotenvy::from_path(&env_path);
            return;
        }
    }
}

fn env_var(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn get_keyring_secret(entry: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, entry)
        .ok()
        .and_then(|e| e.get_password().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn set_keyring_secret(entry: &str, value: Option<&str>) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, entry).map_err(|e| e.to_string())?;
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        Some(v) => entry.set_password(v).map_err(|e| e.to_string()),
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        },
    }
}

fn resolve_secret(keyring_entry: &str, env_key: &str) -> Option<String> {
    get_keyring_secret(keyring_entry).or_else(|| env_var(env_key))
}

/// Move legacy plaintext secrets from SQLite into the OS keyring and clear DB columns.
pub fn migrate_plaintext_secrets(conn: &Connection) -> Result<(), String> {
    let row: Result<(Option<String>, Option<String>), rusqlite::Error> = conn.query_row(
        "SELECT api_key, ncbi_api_key FROM qdrant_config WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    let Ok((qdrant_key, ncbi_key)) = row else {
        return Ok(());
    };

    let mut migrated = false;
    if let Some(ref k) = qdrant_key
        && !k.trim().is_empty()
    {
        set_keyring_secret(SECRET_QDRANT_API_KEY, Some(k))?;
        migrated = true;
    }
    if let Some(ref k) = ncbi_key
        && !k.trim().is_empty()
    {
        set_keyring_secret(SECRET_NCBI_API_KEY, Some(k))?;
        migrated = true;
    }

    if migrated {
        conn.execute(
            "UPDATE qdrant_config SET api_key = NULL, ncbi_api_key = NULL WHERE id = 1",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Load sweep scope toggles from SQLite (defaults if columns missing).
pub fn load_research_scope(conn: &Connection) -> Result<ResearchScopeConfig, String> {
    Ok(conn
        .query_row(
            "SELECT scope_curated, scope_agent, scope_gwas, scope_non_ref, scope_non_ref_limit, scope_gwas_limit, scope_sweep_fast, scope_sources_json
         FROM qdrant_config WHERE id = 1",
            [],
            |row| {
                let sweep_fast = row.get::<_, i32>(6).unwrap_or(0) != 0;
                let sources_json: Option<String> = row.get(7)?;
                let enrichment_sources = sources_json
                    .as_deref()
                    .and_then(|json| serde_json::from_str::<EnrichmentSourcesConfig>(json).ok())
                    .unwrap_or_else(|| EnrichmentSourcesConfig::from_sweep_fast(sweep_fast));
                Ok(ResearchScopeConfig {
                    curated: row.get::<_, i32>(0)? != 0,
                    agent_discoveries: row.get::<_, i32>(1)? != 0,
                    gwas_discovery: row.get::<_, i32>(2)? != 0,
                    non_reference: row.get::<_, i32>(3)? != 0,
                    non_reference_limit: row.get::<_, i32>(4)? as u32,
                    gwas_discovery_limit: row.get::<_, i32>(5)? as u32,
                    sweep_fast,
                    enrichment_sources,
                })
            },
        )
        .unwrap_or_default())
}

pub fn save_research_scope(conn: &Connection, scope: &ResearchScopeConfig) -> Result<(), String> {
    let sweep_fast = scope.sweep_fast || scope.enrichment_sources.is_fast_index();
    let sources_json = serde_json::to_string(&scope.enrichment_sources)
        .map_err(|e| format!("Failed to serialize enrichment sources: {}", e))?;
    conn.execute(
        "UPDATE qdrant_config SET
            scope_curated = ?1,
            scope_agent = ?2,
            scope_gwas = ?3,
            scope_non_ref = ?4,
            scope_non_ref_limit = ?5,
            scope_gwas_limit = ?6,
            scope_sweep_fast = ?7,
            scope_sources_json = ?8
         WHERE id = 1",
        params![
            scope.curated as i32,
            scope.agent_discoveries as i32,
            scope.gwas_discovery as i32,
            scope.non_reference as i32,
            scope.non_reference_limit as i32,
            scope.gwas_discovery_limit as i32,
            sweep_fast as i32,
            sources_json,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Seed empty SQLite connection fields from `.env` only — never overwrite UI-saved values.
pub fn sync_qdrant_sqlite_from_env(conn: &Connection) -> Result<(), String> {
    let (url, collection, ollama_url): (String, String, String) = conn
        .query_row(
            "SELECT url, collection, COALESCE(ollama_url, '') FROM qdrant_config WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;

    let mut next_url = url.clone();
    let mut next_collection = collection.clone();
    let mut next_ollama = ollama_url.clone();

    if next_url.trim().is_empty() {
        if let Some(env) = env_var("QDRANT_URL") {
            next_url = env;
        }
    }
    if next_collection.trim().is_empty() {
        if let Some(env) = env_var("QDRANT_COLLECTION") {
            next_collection = env;
        }
    }
    if next_ollama.trim().is_empty() {
        if let Some(env) = ollama_url_from_env() {
            next_ollama = env;
        }
    }

    if next_url != url || next_collection != collection || next_ollama != ollama_url {
        let _ = conn.execute(
            "ALTER TABLE qdrant_config ADD COLUMN ollama_url TEXT NOT NULL DEFAULT ''",
            [],
        );
        conn.execute(
            "UPDATE qdrant_config SET url = ?1, collection = ?2, ollama_url = ?3 WHERE id = 1",
            params![
                next_url.trim(),
                next_collection.trim(),
                next_ollama.trim()
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Load merged Qdrant settings for the UI (secrets masked).
pub fn load_qdrant_config_for_ui(conn: &Connection) -> Result<QdrantConfigPublic, String> {
    let full = load_qdrant_config(conn)?;
    let research_scope = load_research_scope(conn)?;
    Ok(QdrantConfigPublic {
        url: full.url,
        collection: full.collection,
        embedding_model: full.embedding_model,
        gwas_strict: full.gwas_strict,
        auto_start: full.auto_start,
        api_key_set: full.api_key.is_some(),
        ncbi_api_key_set: full.ncbi_api_key.is_some(),
        research_scope,
        named_vectors_enabled: full.named_vectors_enabled,
        vector_provider: full.vector_provider,
        namespace: full.namespace,
    })
}

/// Load merged Qdrant settings: non-secrets from SQLite, secrets from keyring then `.env`.
pub fn load_qdrant_config(conn: &Connection) -> Result<QdrantConfig, String> {
    let (
        url,
        collection,
        embedding_model,
        gwas_strict,
        auto_start,
        named_vectors_enabled,
        vector_provider,
        namespace,
    ): (String, String, String, bool, bool, bool, String, String) = conn
        .query_row(
            "SELECT url, collection, embedding_model, gwas_strict, auto_start,
                    COALESCE(named_vectors_enabled, 0),
                    COALESCE(vector_provider, 'qdrant'),
                    COALESCE(namespace, '')
             FROM qdrant_config WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, i32>(3)? != 0,
                    row.get::<_, i32>(4)? != 0,
                    row.get::<_, i32>(5)? != 0,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    // UI-saved SQLite values win. `.env` only fills blanks (bootstrap).
    let url = if url.trim().is_empty() {
        env_var("QDRANT_URL").unwrap_or(url)
    } else {
        url
    };

    let collection = if collection.trim().is_empty() {
        env_var("QDRANT_COLLECTION").unwrap_or(collection)
    } else {
        collection
    };

    let vector_provider = {
        let p = vector_provider.trim().to_ascii_lowercase();
        if p.is_empty() {
            "qdrant".into()
        } else {
            p
        }
    };

    Ok(QdrantConfig {
        url,
        api_key: resolve_secret(SECRET_QDRANT_API_KEY, "QDRANT_API_KEY")
            .or_else(|| resolve_secret(SECRET_QDRANT_API_KEY, "PINECONE_API_KEY")),
        collection,
        embedding_model,
        gwas_strict,
        ncbi_api_key: resolve_secret(SECRET_NCBI_API_KEY, "NCBI_API_KEY"),
        auto_start,
        named_vectors_enabled,
        vector_provider,
        namespace,
    })
}

/// Persist Qdrant settings; secrets go to keyring only (never plaintext in SQLite).
/// Secret fields in `update` are optional: `None` leaves the existing keyring value unchanged.
pub fn save_qdrant_config(conn: &Connection, update: &QdrantConfigUpdate) -> Result<(), String> {
    if let Some(ref key) = update.api_key {
        set_keyring_secret(
            SECRET_QDRANT_API_KEY,
            if key.trim().is_empty() {
                None
            } else {
                Some(key.trim())
            },
        )?;
    }
    if let Some(ref key) = update.ncbi_api_key {
        set_keyring_secret(
            SECRET_NCBI_API_KEY,
            if key.trim().is_empty() {
                None
            } else {
                Some(key.trim())
            },
        )?;
    }

    let named_vectors = update.named_vectors_enabled.unwrap_or_else(|| {
        conn.query_row(
            "SELECT COALESCE(named_vectors_enabled, 0) FROM qdrant_config WHERE id = 1",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(false)
    });

    let provider = if let Some(raw) = update.vector_provider.as_deref() {
        let p = raw.trim().to_ascii_lowercase();
        if p.is_empty() {
            conn.query_row(
                "SELECT COALESCE(vector_provider, 'qdrant') FROM qdrant_config WHERE id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "qdrant".into())
        } else {
            p
        }
    } else {
        conn.query_row(
            "SELECT COALESCE(vector_provider, 'qdrant') FROM qdrant_config WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "qdrant".into())
    };
    let allowed = ["qdrant", "pinecone", "chroma", "weaviate"];
    if !allowed.contains(&provider.as_str()) {
        return Err(format!(
            "Unsupported vector provider '{provider}'. Use one of: {}",
            allowed.join(", ")
        ));
    }

    let namespace = if let Some(ns) = update.namespace.as_deref() {
        ns.trim().to_string()
    } else {
        conn.query_row(
            "SELECT COALESCE(namespace, '') FROM qdrant_config WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_default()
    };

    let qdrant_url = validate_service_url(&update.url)?;

    // Ensure columns exist on older DBs.
    let _ = conn.execute(
        "ALTER TABLE qdrant_config ADD COLUMN vector_provider TEXT NOT NULL DEFAULT 'qdrant'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE qdrant_config ADD COLUMN namespace TEXT NOT NULL DEFAULT ''",
        [],
    );

    conn.execute(
        "UPDATE qdrant_config SET url=?1, api_key=NULL, collection=?2, embedding_model=?3, gwas_strict=?4, ncbi_api_key=NULL, auto_start=?5, named_vectors_enabled=?6, vector_provider=?7, namespace=?8 WHERE id=1",
        params![
            qdrant_url,
            update.collection.trim(),
            update.embedding_model.trim(),
            update.gwas_strict as i32,
            update.auto_start as i32,
            named_vectors as i32,
            provider,
            namespace,
        ],
    )
    .map_err(|e| e.to_string())?;

    if let Some(ref scope) = update.research_scope {
        save_research_scope(conn, scope)?;
    }

    Ok(())
}

/// Prefer keyring / `.env` token; fall back to an explicit frontend override when provided.
pub fn resolve_ollama_token(frontend_token: Option<String>) -> Option<String> {
    frontend_token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .or_else(get_ollama_token)
}

pub fn get_ollama_token() -> Option<String> {
    resolve_secret(SECRET_OLLAMA_TOKEN, "OLLAMA_TOKEN")
}

pub fn save_ollama_token(token: Option<&str>) -> Result<(), String> {
    set_keyring_secret(SECRET_OLLAMA_TOKEN, token)
}

/// Ollama base URL: SQLite (UI-saved) → `.env` bootstrap → empty.
pub fn resolve_ollama_service_url() -> String {
    String::new()
}

pub fn resolve_ollama_service_url_with_db(conn: &Connection) -> String {
    let stored = conn
        .query_row(
            "SELECT COALESCE(ollama_url, '') FROM qdrant_config WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_default()
        .trim()
        .to_string();
    if !stored.is_empty() {
        return stored;
    }
    ollama_url_from_env().unwrap_or_default()
}

pub fn ollama_url_from_env() -> Option<String> {
    env_var("GENOMICS_OLLAMA_URL").or_else(|| env_var("OLLAMA_URL"))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OllamaServiceConfig {
    pub url: String,
    pub from_env: bool,
    pub token_set: bool,
    pub configured: bool,
    /// Present when `.env` differs from the active (SQLite) URL — UI can offer adopt/keep.
    pub env_url: Option<String>,
}

pub fn load_ollama_service_config() -> OllamaServiceConfig {
    let env = ollama_url_from_env();
    let url = env.clone().unwrap_or_default();
    OllamaServiceConfig {
        configured: !url.trim().is_empty(),
        from_env: env.is_some(),
        token_set: get_ollama_token().is_some(),
        env_url: env,
        url,
    }
}

pub fn load_ollama_service_config_with_db(conn: &Connection) -> OllamaServiceConfig {
    let env = ollama_url_from_env();
    let url = resolve_ollama_service_url_with_db(conn);
    let from_env = !url.trim().is_empty()
        && env
            .as_ref()
            .is_some_and(|e| e.trim() == url.trim())
        && conn
            .query_row(
                "SELECT COALESCE(ollama_url, '') FROM qdrant_config WHERE id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .map(|s| s.trim().is_empty())
            .unwrap_or(true);
    let env_url = env.filter(|e| e.trim() != url.trim());
    OllamaServiceConfig {
        configured: !url.trim().is_empty(),
        from_env,
        token_set: get_ollama_token().is_some(),
        env_url,
        url,
    }
}

pub fn save_ollama_url(conn: &Connection, url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    let stored = if trimmed.is_empty() {
        String::new()
    } else {
        validate_service_url(trimmed)?
    };
    // Ensure column exists on older DBs.
    let _ = conn.execute(
        "ALTER TABLE qdrant_config ADD COLUMN ollama_url TEXT NOT NULL DEFAULT ''",
        [],
    );
    conn.execute(
        "UPDATE qdrant_config SET ollama_url = ?1 WHERE id = 1",
        params![stored],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
/// Approved external API hosts for `fetch_external_api` (SSRF protection).
pub fn validate_external_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(format!("Unsupported URL scheme: {}", other)),
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "URL must include a host".to_string())?;

    if host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1" {
        return Err("Localhost URLs are not allowed".to_string());
    }

    if let Ok(ip) = host.parse::<IpAddr>()
        && is_private_or_loopback(ip)
    {
        return Err("Private or link-local network addresses are not allowed".to_string());
    }

    const ALLOWED_HOST_SUFFIXES: &[&str] = &[
        "ncbi.nlm.nih.gov",
        "api.ncbi.nlm.nih.gov",
        "ensembl.org",
        "ebi.ac.uk",
        "clinicaltrials.gov",
        "duckduckgo.com",
        "broadinstitute.org",
        "gtexportal.org",
        "pgscatalog.org",
        "www.pgscatalog.org",
        "reactome.org",
        "opentargets.org",
        "platform.opentargets.org",
        "api.platform.opentargets.org",
        "pharmgkb.org",
        "clinpgx.org",
        "api.clinpgx.org",
    ];

    let allowed = ALLOWED_HOST_SUFFIXES.iter().any(|suffix| {
        host.eq_ignore_ascii_case(suffix)
            || host.to_ascii_lowercase().ends_with(&format!(".{suffix}"))
    });

    if !allowed {
        return Err(format!(
            "Host '{}' is not in the approved external API allowlist",
            host
        ));
    }

    Ok(())
}

fn is_private_or_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.octets()[0] == 169 && v4.octets()[1] == 254 // link-local
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified() || is_ipv6_ula(v6) || is_ipv6_link_local(v6)
        }
    }
}

fn is_ipv6_ula(ip: std::net::Ipv6Addr) -> bool {
    (ip.octets()[0] & 0xfe) == 0xfc
}

fn is_ipv6_link_local(ip: std::net::Ipv6Addr) -> bool {
    (ip.octets()[0] == 0xfe) && ((ip.octets()[1] & 0xc0) == 0x80)
}

static ALLOWED_IMPORT_PATHS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn register_import_path(path: &str) {
    let canonical = std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string());
    if let Ok(mut guard) = ALLOWED_IMPORT_PATHS.lock() {
        guard.insert(canonical);
    }
}

pub fn validate_import_path(path: &str) -> Result<(), String> {
    let canonical =
        std::fs::canonicalize(path).map_err(|e| format!("Invalid import path: {}", e))?;
    if !canonical.is_file() {
        return Err("Import path must be an existing file".into());
    }
    validate_import_file_size(&canonical)?;
    let ext_ok = canonical.extension().is_some_and(|e| {
        let e = e.to_string_lossy().to_lowercase();
        e == "txt" || e == "csv" || e == "tsv" || e == "zip"
    });
    if !ext_ok {
        return Err("Import file must be .txt, .csv, .tsv, or .zip".into());
    }
    let path_str = canonical.to_string_lossy().to_string();
    let guard = ALLOWED_IMPORT_PATHS
        .lock()
        .map_err(|_| "Import path registry unavailable".to_string())?;
    if !guard.contains(&path_str) {
        return Err("Import path was not selected through the file picker in this session".into());
    }
    Ok(())
}

/// Maximum plain-text or ZIP import file size (512 MiB).
pub const MAX_IMPORT_FILE_BYTES: u64 = 512 * 1024 * 1024;

pub fn validate_import_file_size(path: &std::path::Path) -> Result<(), String> {
    let bytes = std::fs::metadata(path)
        .map_err(|e| format!("Cannot read import file metadata: {}", e))?
        .len();
    if bytes > MAX_IMPORT_FILE_BYTES {
        return Err(format!(
            "Import file exceeds maximum size of {} MiB",
            MAX_IMPORT_FILE_BYTES / (1024 * 1024)
        ));
    }
    Ok(())
}

/// Maximum report template JSON payload (16 MiB — covers all 22+ marker packs combined).
pub const MAX_TEMPLATE_JSON_BYTES: usize = 16 * 1024 * 1024;

pub fn validate_template_json(template_json: &str) -> Result<(), String> {
    if template_json.len() > MAX_TEMPLATE_JSON_BYTES {
        return Err(format!(
            "Report template JSON exceeds maximum size of {} MiB",
            MAX_TEMPLATE_JSON_BYTES / (1024 * 1024)
        ));
    }
    Ok(())
}

static ALLOWED_EXPORT_PATHS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Register a user-selected export destination from a save dialog in this session.
pub fn register_export_path(path: &str) {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return;
    }
    if let Ok(mut guard) = ALLOWED_EXPORT_PATHS.lock() {
        guard.insert(trimmed.to_string());
        if let Ok(canonical) = std::fs::canonicalize(trimmed) {
            guard.insert(canonical.to_string_lossy().to_string());
        } else if let Some(parent) = std::path::Path::new(trimmed).parent()
            && let Ok(parent_canon) = std::fs::canonicalize(parent)
            && let Some(name) = std::path::Path::new(trimmed).file_name()
        {
            guard.insert(parent_canon.join(name).to_string_lossy().to_string());
        }
    }
}

fn normalize_export_path(path: &str) -> Result<String, String> {
    let input = std::path::Path::new(path.trim());
    if input.as_os_str().is_empty() {
        return Err("Export path is empty".into());
    }
    if let Ok(canonical) = std::fs::canonicalize(input) {
        return Ok(canonical.to_string_lossy().to_string());
    }
    let parent = input
        .parent()
        .ok_or_else(|| "Export path must include a parent directory".to_string())?;
    let parent_canon =
        std::fs::canonicalize(parent).map_err(|e| format!("Invalid export path parent: {}", e))?;
    let file_name = input
        .file_name()
        .ok_or_else(|| "Export path must include a file name".to_string())?;
    Ok(parent_canon.join(file_name).to_string_lossy().to_string())
}

/// Export paths must be chosen through a save dialog in this session.
pub fn validate_export_path(path: &str) -> Result<(), String> {
    validate_registered_export_path(path, "json")
}

/// Validate a save-dialog-selected path for a specific export extension.
pub fn validate_registered_export_path(path: &str, required_extension: &str) -> Result<(), String> {
    let normalized = normalize_export_path(path)?;
    let required_extension = required_extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if required_extension.is_empty()
        || !required_extension
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return Err("Invalid export extension".into());
    }
    let ext_ok = std::path::Path::new(&normalized)
        .extension()
        .map(|e| {
            let e = e.to_string_lossy().to_lowercase();
            e == required_extension
        })
        .unwrap_or(false);
    if !ext_ok {
        return Err(format!(
            "Export file must use a .{required_extension} extension"
        ));
    }
    let guard = ALLOWED_EXPORT_PATHS
        .lock()
        .map_err(|_| "Export path registry unavailable".to_string())?;
    let raw = path.trim();
    if guard.contains(&normalized) || guard.contains(raw) {
        return Ok(());
    }
    Err("Export path was not selected through the save dialog in this session".into())
}

pub fn validate_pack_id(pack_id: &str) -> Result<(), String> {
    if pack_id.is_empty() || pack_id.len() > 64 {
        return Err("Invalid pack_id length".into());
    }
    if !pack_id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(format!(
            "Invalid pack_id '{}': use [a-z0-9_]+ only",
            pack_id
        ));
    }
    Ok(())
}

pub fn resolve_pack_path(app_data_dir: &Path, pack_id: &str) -> Result<PathBuf, String> {
    validate_pack_id(pack_id)?;
    let base = app_data_dir
        .join("marker-packs")
        .canonicalize()
        .map_err(|e| format!("marker-packs directory unavailable: {}", e))?;
    let pack_path = base.join(format!("{}.json", pack_id));
    let canonical = pack_path
        .canonicalize()
        .map_err(|_| format!("Unknown pack ID: {}", pack_id))?;
    if !canonical.starts_with(&base) {
        return Err("Pack path escapes marker-packs directory".into());
    }
    Ok(canonical)
}

pub const CANDIDATE_STATUSES: &[&str] = &["rejected", "candidate", "curated_lite"];

pub fn validate_candidate_status(status: &str) -> Result<(), String> {
    if CANDIDATE_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(format!(
            "Invalid candidate status '{}'. Allowed: {:?}",
            status, CANDIDATE_STATUSES
        ))
    }
}

/// User-configured Ollama / Qdrant endpoints (localhost and LAN allowed; cloud metadata blocked).
/// Escape `%`, `_`, and `\` so user input is treated literally in SQL LIKE patterns.
pub fn sanitize_sql_like_pattern(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    for c in input.chars() {
        match c {
            '%' | '_' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            other => out.push(other),
        }
    }
    out
}

/// Build a case-insensitive substring LIKE pattern with wildcards escaped.
pub fn sql_like_contains_pattern(input: &str) -> String {
    format!(
        "%{}%",
        sanitize_sql_like_pattern(&input.trim().to_lowercase())
    )
}

pub fn validate_service_url(url: &str) -> Result<String, String> {
    let parsed = url::Url::parse(url.trim()).map_err(|e| format!("Invalid URL: {}", e))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(format!("Unsupported URL scheme: {}", other)),
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL must include a host".to_string())?;
    if host.eq_ignore_ascii_case("metadata.google.internal") || host == "169.254.169.254" {
        return Err("Metadata endpoints are not allowed".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>()
        && ip.is_unspecified()
    {
        return Err("Unspecified network addresses are not allowed".into());
    }
    Ok(parsed
        .origin()
        .ascii_serialization()
        .trim_end_matches('/')
        .to_string())
}

pub fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        let url = attempt.url().as_str();
        match validate_external_url(url) {
            Ok(()) => attempt.follow(),
            Err(e) => attempt.error(e),
        }
    })
}

/// Load persisted research sweep debug logging preference (`app_metadata`).
pub fn load_research_debug_log(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT value FROM app_metadata WHERE key = ?",
        params![crate::research::debug_log::METADATA_KEY],
        |row| row.get::<_, String>(0),
    )
    .map(|v| v == "1")
    .unwrap_or(false)
}

pub fn save_research_debug_log(conn: &Connection, enabled: bool) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES (?, ?)",
        params![
            crate::research::debug_log::METADATA_KEY,
            if enabled { "1" } else { "0" }
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Apply env override, then SQLite preference.
pub fn install_research_debug_logging(conn: &Connection) {
    crate::research::debug_log::refresh_from_env();
    if load_research_debug_log(conn) {
        crate::research::debug_log::set_enabled(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn sql_like_pattern_escapes_wildcards() {
        assert_eq!(sanitize_sql_like_pattern("rs123"), "rs123");
        assert_eq!(sanitize_sql_like_pattern("100%"), "100\\%");
        assert_eq!(sanitize_sql_like_pattern("a_b"), "a\\_b");
        assert_eq!(sql_like_contains_pattern("RS%"), "%rs\\%%");
    }

    #[test]
    fn validate_service_url_blocks_metadata_hosts() {
        assert!(validate_service_url("http://127.0.0.1:11434").is_ok());
        assert!(validate_service_url("http://169.254.169.254/latest/meta-data").is_err());
        assert!(validate_service_url("http://metadata.google.internal").is_err());
    }

    #[test]
    fn validate_pack_id_rejects_traversal() {
        assert!(validate_pack_id("mood").is_ok());
        assert!(validate_pack_id("../etc/passwd").is_err());
        assert!(validate_pack_id("Bad-ID").is_err());
    }

    #[test]
    fn export_path_requires_save_dialog_registration() {
        let dir =
            std::env::temp_dir().join(format!("dna_tools_export_test_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let export_file = dir.join("findings.json");
        let export_str = export_file.to_string_lossy().to_string();

        assert!(validate_export_path(&export_str).is_err());
        register_export_path(&export_str);
        assert!(validate_export_path(&export_str).is_ok());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_path_extension_is_a_contract() {
        let dir = std::env::temp_dir().join(format!(
            "dna_tools_export_extension_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        let export_file = dir.join("findings.zip");
        let export_str = export_file.to_string_lossy().to_string();

        register_export_path(&export_str);
        assert!(validate_registered_export_path(&export_str, "zip").is_ok());
        assert!(validate_export_path(&export_str).is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_import_file_size_accepts_small_file() {
        let dir =
            std::env::temp_dir().join(format!("dna_tools_import_size_test_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let import_file = dir.join("sample.txt");
        fs::write(
            &import_file,
            b"rsid\tchromosome\tposition\tallele1\tallele2",
        )
        .expect("write");
        assert!(validate_import_file_size(&import_file).is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn template_json_size_limit() {
        assert!(validate_template_json("{}").is_ok());
        let huge = " ".repeat(MAX_TEMPLATE_JSON_BYTES + 1);
        assert!(validate_template_json(&huge).is_err());
    }

    #[test]
    fn import_path_registry_accepts_registered_file() {
        let dir =
            std::env::temp_dir().join(format!("dna_tools_import_test_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let import_file = dir.join("sample.txt");
        fs::write(&import_file, "test").expect("write sample");
        let import_str = import_file.to_string_lossy().to_string();

        register_import_path(&import_str);
        assert!(validate_import_path(&import_str).is_ok());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn registered_csv_and_tsv_import_paths_are_accepted() {
        let dir = std::env::temp_dir().join(format!("dna_tools_import_delimited_test_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        for extension in ["csv", "tsv"] {
            let import_file = dir.join(format!("sample.{extension}"));
            fs::write(&import_file, "rsid,chromosome,position,genotype\n").expect("write");
            let import_str = import_file.to_string_lossy().to_string();
            register_import_path(&import_str);
            assert!(validate_import_path(&import_str).is_ok());
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_pack_path_stays_under_marker_packs() {
        let base = std::env::temp_dir().join(format!("dna_tools_pack_test_{}", std::process::id()));
        let packs = base.join("marker-packs");
        fs::create_dir_all(&packs).expect("packs dir");
        fs::write(packs.join("mood.json"), "{}").expect("pack file");

        assert!(resolve_pack_path(&base, "mood").is_ok());
        assert!(resolve_pack_path(&base, "../outside").is_err());

        let _ = fs::remove_dir_all(&base);
    }
}
