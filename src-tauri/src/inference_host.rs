// ./src-tauri/src/inference_host.rs
/*
Purpose: Dynamic inference-host and Ollama model capability discovery.
Responsibilities:
  - Probe local OS signals for GPU / unified-memory acceleration (no SKU hardcodes).
  - Enrich Ollama /api/tags models with load hints using /api/show, /api/ps, and host profile.
  - Prefer observed VRAM residency over marketing names.
How to run: Invoked via Tauri commands `probe_inference_host` and `discover_ollama_models`.
Key inputs: Optional Ollama base URL + bearer token; live OS probes.
Key outputs: InferenceHostProfile + Vec<OllamaModelInsight> JSON for Advanced Connections UI.
Operational notes: Remote Ollama URLs are treated as opaque accelerators; local probes still run for context.
*/

use crate::config;
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct InferenceHostProfile {
    pub platform: String,
    pub arch: String,
    pub is_local_url: bool,
    pub unified_memory: bool,
    pub accel_backends: Vec<String>,
    pub accel_bytes: Option<u64>,
    pub system_ram_bytes: Option<u64>,
    pub observed_vram_in_use_bytes: Option<u64>,
    pub posture: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OllamaModelInsight {
    pub name: String,
    pub role: String,
    pub size_bytes: Option<u64>,
    pub parameter_size: Option<String>,
    pub quantization: Option<String>,
    pub family: Option<String>,
    pub currently_loaded: bool,
    pub size_vram_bytes: Option<u64>,
    pub load_hint: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OllamaDiscoveryReport {
    pub host: InferenceHostProfile,
    pub models: Vec<OllamaModelInsight>,
    pub ollama_reachable: bool,
    pub error: Option<String>,
}

fn is_loopback_or_local_host(host: &str) -> bool {
    let h = host.trim().to_ascii_lowercase();
    h == "localhost"
        || h == "127.0.0.1"
        || h == "::1"
        || h == "0.0.0.0"
        || h.ends_with(".local")
}

fn url_is_local(url: &str) -> bool {
    url::Url::parse(url.trim())
        .ok()
        .and_then(|u| u.host_str().map(|h| is_loopback_or_local_host(h)))
        .unwrap_or(false)
}

fn read_to_string_lossy(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn parse_u64_digits(s: &str) -> Option<u64> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn system_ram_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let meminfo = read_to_string_lossy(Path::new("/proc/meminfo"))?;
        for line in meminfo.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                // kB
                let kb = parse_u64_digits(rest)?;
                return Some(kb.saturating_mul(1024));
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("sysctl").args(["-n", "hw.memsize"]).output().ok()?;
        if !out.status.success() {
            return None;
        }
        parse_u64_digits(&String::from_utf8_lossy(&out.stdout))
    }
    #[cfg(target_os = "windows")]
    {
        None
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

fn detect_apple_unified() -> (bool, Vec<String>) {
    let notes = Vec::new();
    #[cfg(target_os = "macos")]
    {
        let mut notes = Vec::new();
        let arch = std::env::consts::ARCH;
        if arch == "aarch64" {
            notes.push(
                "Apple Silicon detected (arm64 + macOS): GPU and CPU share unified memory — large models can use system RAM via Metal."
                    .into(),
            );
            return (true, notes);
        }
        notes.push("macOS on non-ARM: Metal may still accelerate, but memory is not Apple-unified.".into());
        return (false, notes);
    }
    (false, notes)
}

fn detect_linux_accel() -> (Vec<String>, Option<u64>, bool, Vec<String>) {
    let mut backends = Vec::new();
    let mut notes = Vec::new();
    let mut accel_bytes: Option<u64> = None;
    let mut unified = false;

    if Command::new("nvidia-smi")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        backends.push("cuda".into());
        if let Ok(out) = Command::new("nvidia-smi")
            .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
            .output()
        {
            let mut total_mib: u64 = 0;
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some(mib) = parse_u64_digits(line) {
                    total_mib = total_mib.saturating_add(mib);
                }
            }
            if total_mib > 0 {
                accel_bytes = Some(total_mib.saturating_mul(1024 * 1024));
                notes.push(format!(
                    "NVIDIA CUDA visible via nvidia-smi (~{:.1} GiB device memory).",
                    total_mib as f64 / 1024.0
                ));
            }
        }
    }

    if Path::new("/dev/kfd").exists() || Path::new("/opt/rocm").exists() {
        backends.push("rocm".into());
        notes.push("AMD ROCm / kfd device present — Ollama can use GPU kernels when the runner supports it.".into());
    }

    // DRM cards: collect vendor + reported VRAM without naming SKUs.
    let drm = Path::new("/sys/class/drm");
    if drm.is_dir() {
        if let Ok(entries) = std::fs::read_dir(drm) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("card") || name.contains('-') {
                    continue;
                }
                let vendor = read_to_string_lossy(&entry.path().join("device/vendor"));
                let vram = read_to_string_lossy(&entry.path().join("device/mem_info_vram_total"))
                    .and_then(|s| s.parse::<u64>().ok());
                if let Some(v) = vendor.as_deref() {
                    if v == "0x1002" {
                        if !backends.iter().any(|b| b == "amdgpu") {
                            backends.push("amdgpu".into());
                        }
                        notes.push("AMD GPU (amdgpu) exposed via DRM.".into());
                    } else if v == "0x10de" {
                        if !backends.iter().any(|b| b == "cuda") {
                            backends.push("cuda".into());
                        }
                    } else if v == "0x8086" {
                        backends.push("intel".into());
                        notes.push("Intel graphics present — may accelerate via oneAPI/Vulkan depending on Ollama build.".into());
                    }
                }
                if let Some(bytes) = vram {
                    accel_bytes = Some(accel_bytes.map(|a| a.max(bytes)).unwrap_or(bytes));
                }
            }
        }
    }

    // Unified-memory APU class: AMD CPU advertising AI / HX class + large DRM VRAM relative to RAM,
    // or CPU model containing "ryzen ai" (product line marker, not a fixed SKU list).
    if let Some(cpuinfo) = read_to_string_lossy(Path::new("/proc/cpuinfo")) {
        let lower = cpuinfo.to_ascii_lowercase();
        let amd = lower.contains("authenticamd") || lower.contains("vendor_id\t: authenticamd");
        let ai_class = lower.contains("ryzen ai")
            || lower.contains("ryzen 9 hx")
            || lower.contains("strix")
            || lower.contains("halo");
        if amd && ai_class {
            unified = true;
            notes.push(
                "CPU model string indicates an AMD AI / HX-class APU with chipset-tied graphics memory — treat system RAM as the practical model budget (unified posture)."
                    .into(),
            );
        }
    }

    if let (Some(vram), Some(ram)) = (accel_bytes, system_ram_bytes()) {
        // If reported device memory is a large fraction of system RAM, treat as unified/shared.
        if vram > ram / 3 {
            unified = true;
            notes.push(
                "Reported GPU memory is a large fraction of system RAM — likely shared/unified memory rather than a small discrete framebuffer."
                    .into(),
            );
        }
    }

    if backends.is_empty() {
        notes.push("No local GPU backend detected on this machine (CPU-only posture for local Ollama).".into());
    }

    (backends, accel_bytes, unified, notes)
}

pub fn probe_local_host(ollama_url: Option<&str>) -> InferenceHostProfile {
    let platform = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let is_local_url = ollama_url.map(url_is_local).unwrap_or(true);
    let mut notes = Vec::new();
    let mut accel_backends = Vec::new();
    let mut accel_bytes = None;
    let mut unified_memory = false;

    let (apple_unified, apple_notes) = detect_apple_unified();
    unified_memory |= apple_unified;
    notes.extend(apple_notes);
    if apple_unified {
        accel_backends.push("metal".into());
        accel_bytes = system_ram_bytes();
    }

    #[cfg(target_os = "linux")]
    {
        let (b, bytes, uni, n) = detect_linux_accel();
        for backend in b {
            if !accel_backends.contains(&backend) {
                accel_backends.push(backend);
            }
        }
        if let Some(b) = bytes {
            accel_bytes = Some(accel_bytes.map(|a| a.max(b)).unwrap_or(b));
        }
        unified_memory |= uni;
        notes.extend(n);
    }

    #[cfg(target_os = "windows")]
    {
        notes.push(
            "Windows host: GPU capacity is inferred from Ollama /api/ps when models are loaded (no SKU table)."
                .into(),
        );
    }

    if !is_local_url {
        notes.push(
            "Ollama URL points at a remote host — local GPU probes describe this laptop/desktop only. Acceleration on the remote box is observed live via /api/ps (size_vram > 0)."
                .into(),
        );
    }

    let posture = if !is_local_url {
        "remote_managed".to_string()
    } else if unified_memory {
        "unified_memory".to_string()
    } else if !accel_backends.is_empty() {
        "discrete_or_dedicated_gpu".to_string()
    } else {
        "cpu_only_local".to_string()
    };

    if posture == "cpu_only_local" {
        notes.push(
            "For genomics embeddings/chat, prefer a GPU or unified-memory host (Apple Silicon, AMD AI Max / HX APU class, or discrete NVIDIA/AMD). CPU-only local runs will be slow for large models."
                .into(),
        );
    }

    InferenceHostProfile {
        platform,
        arch,
        is_local_url,
        unified_memory,
        accel_backends,
        accel_bytes,
        system_ram_bytes: system_ram_bytes(),
        observed_vram_in_use_bytes: None,
        posture,
        notes,
    }
}

fn classify_role(name: &str, family: Option<&str>) -> String {
    let blob = format!("{} {}", name, family.unwrap_or("")).to_ascii_lowercase();
    if blob.contains("embed")
        || blob.contains("nomic")
        || blob.contains("bge")
        || blob.contains("e5-")
        || blob.contains("gte-")
        || blob.contains("mxbai")
        || blob.contains("qwen3-embedding")
    {
        "embed".into()
    } else if blob.contains("rerank") || blob.contains("colbert") {
        "rerank".into()
    } else {
        "chat".into()
    }
}

fn parse_param_bytes(parameter_size: Option<&str>, size_bytes: Option<u64>) -> Option<u64> {
    if let Some(s) = size_bytes {
        return Some(s);
    }
    let raw = parameter_size?.to_ascii_lowercase();
    // e.g. "7B", "27.2B", "1.5B"
    let num: String = raw
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let n: f64 = num.parse().ok()?;
    if raw.contains('b') {
        Some((n * 1e9) as u64)
    } else if raw.contains('m') {
        Some((n * 1e6) as u64)
    } else {
        None
    }
}

fn load_hint_for(
    host: &InferenceHostProfile,
    size_bytes: Option<u64>,
    loaded_vram: Option<u64>,
    currently_loaded: bool,
) -> (String, String) {
    if currently_loaded {
        if loaded_vram.unwrap_or(0) > 0 {
            return (
                "on_gpu".into(),
                "Currently resident with size_vram > 0 — Ollama is using accelerator memory.".into(),
            );
        }
        return (
            "on_cpu".into(),
            "Currently loaded but size_vram is 0 — running in system RAM / CPU path.".into(),
        );
    }

    if !host.is_local_url {
        return (
            "remote_unknown".into(),
            "Remote Ollama — load a model once and watch /api/ps to confirm GPU residency on that machine.".into(),
        );
    }

    if host.posture == "cpu_only_local" {
        return (
            "likely_cpu".into(),
            "No local GPU/unified accelerator detected — this model would run on CPU here.".into(),
        );
    }

    let budget = if host.unified_memory {
        host.system_ram_bytes
            .or(host.accel_bytes)
            .map(|b| (b as f64 * 0.55) as u64)
    } else {
        host.accel_bytes.map(|b| (b as f64 * 0.85) as u64)
    };

    match (size_bytes, budget) {
        (Some(need), Some(have)) if need <= have => {
            if host.unified_memory {
                (
                    "fits_unified".into(),
                    format!(
                        "Estimated weight (~{:.1} GiB) fits unified/shared memory budget (~{:.1} GiB usable).",
                        need as f64 / 1e9,
                        have as f64 / 1e9
                    ),
                )
            } else {
                (
                    "likely_gpu".into(),
                    format!(
                        "Estimated weight (~{:.1} GiB) fits discovered accelerator memory (~{:.1} GiB).",
                        need as f64 / 1e9,
                        have as f64 / 1e9
                    ),
                )
            }
        }
        (Some(need), Some(have)) => (
            "may_spill_cpu".into(),
            format!(
                "Estimated weight (~{:.1} GiB) exceeds comfortable accelerator budget (~{:.1} GiB) — may spill to CPU or fail to load.",
                need as f64 / 1e9,
                have as f64 / 1e9
            ),
        ),
        _ => (
            "unknown".into(),
            "Insufficient size metadata — connect and load once to observe size_vram via telemetry.".into(),
        ),
    }
}

fn auth_header(token: Option<&str>) -> Option<String> {
    let t = token.map(str::trim).filter(|s| !s.is_empty())?;
    Some(if t.to_ascii_lowercase().starts_with("bearer ") {
        t.to_string()
    } else {
        format!("Bearer {t}")
    })
}

pub async fn discover_ollama_models(
    url: &str,
    token: Option<&str>,
) -> Result<OllamaDiscoveryReport, String> {
    let clean = config::validate_service_url(url)?;
    let mut host = probe_local_host(Some(&clean));
    let client = reqwest::Client::new();
    let auth = auth_header(token);

    let mut tags_req = client.get(format!("{clean}/api/tags"));
    let mut ps_req = client.get(format!("{clean}/api/ps"));
    if let Some(ref a) = auth {
        tags_req = tags_req.header("Authorization", a);
        ps_req = ps_req.header("Authorization", a);
    }

    let tags_res = tags_req.send().await.map_err(|e| format!("Ollama tags: {e}"))?;
    if !tags_res.status().is_success() {
        return Ok(OllamaDiscoveryReport {
            host,
            models: vec![],
            ollama_reachable: false,
            error: Some(format!("Ollama /api/tags HTTP {}", tags_res.status())),
        });
    }

    let tags_json: serde_json::Value = tags_res
        .json()
        .await
        .map_err(|e| format!("parse tags: {e}"))?;

    let mut loaded: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut observed_vram = 0u64;
    if let Ok(ps_res) = ps_req.send().await
        && ps_res.status().is_success()
        && let Ok(ps_json) = ps_res.json::<serde_json::Value>().await
    {
        if let Some(arr) = ps_json.get("models").and_then(|m| m.as_array()) {
            for m in arr {
                let name = m
                    .get("name")
                    .or_else(|| m.get("model"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let vram = m.get("size_vram").and_then(|v| v.as_u64()).unwrap_or(0);
                observed_vram = observed_vram.saturating_add(vram);
                if !name.is_empty() {
                    loaded.insert(name, vram);
                }
            }
        }
    }
    host.observed_vram_in_use_bytes = if observed_vram > 0 {
        Some(observed_vram)
    } else {
        None
    };
    if observed_vram > 0 && host.accel_bytes.is_none() && !host.is_local_url {
        // Bootstrap remote capacity estimate from peak observed residency.
        host.accel_bytes = Some(observed_vram);
        host.notes.push(
            "Remote accelerator budget estimated from currently loaded size_vram (grows as you load models)."
                .into(),
        );
    }

    let tag_models = tags_json
        .get("models")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    let mut insights = Vec::new();
    for m in tag_models {
        let name = m
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let size_bytes = m.get("size").and_then(|v| v.as_u64());
        let mut parameter_size = m
            .get("details")
            .and_then(|d| d.get("parameter_size"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let mut quantization = m
            .get("details")
            .and_then(|d| d.get("quantization_level"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let mut family = m
            .get("details")
            .and_then(|d| d.get("family"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Best-effort /api/show enrichment (skip on failure).
        let mut show_req = client.post(format!("{clean}/api/show")).json(&serde_json::json!({
            "name": name
        }));
        if let Some(ref a) = auth {
            show_req = show_req.header("Authorization", a);
        }
        if let Ok(show_res) = show_req.send().await
            && show_res.status().is_success()
            && let Ok(show) = show_res.json::<serde_json::Value>().await
        {
            if parameter_size.is_none() {
                parameter_size = show
                    .get("details")
                    .and_then(|d| d.get("parameter_size"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
            if quantization.is_none() {
                quantization = show
                    .get("details")
                    .and_then(|d| d.get("quantization_level"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
            if family.is_none() {
                family = show
                    .get("details")
                    .and_then(|d| d.get("family"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        show.get("model_info")
                            .and_then(|mi| mi.as_object())
                            .and_then(|obj| {
                                obj.keys()
                                    .find(|k| k.ends_with(".general.architecture"))
                                    .and_then(|k| obj.get(k))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            })
                    });
            }
        }

        let role = classify_role(&name, family.as_deref());
        let weight = parse_param_bytes(parameter_size.as_deref(), size_bytes);
        let currently_loaded = loaded.contains_key(&name);
        let size_vram_bytes = loaded.get(&name).copied();
        let (load_hint, rationale) =
            load_hint_for(&host, weight.or(size_bytes), size_vram_bytes, currently_loaded);

        insights.push(OllamaModelInsight {
            name,
            role,
            size_bytes,
            parameter_size,
            quantization,
            family,
            currently_loaded,
            size_vram_bytes,
            load_hint,
            rationale,
        });
    }

    // Prefer GPU-resident / likely_gpu / fits_unified first within role groups.
    insights.sort_by(|a, b| {
        let rank = |h: &str| match h {
            "on_gpu" => 0,
            "fits_unified" | "likely_gpu" => 1,
            "remote_unknown" | "unknown" => 2,
            "may_spill_cpu" => 3,
            "on_cpu" | "likely_cpu" => 4,
            _ => 5,
        };
        rank(&a.load_hint)
            .cmp(&rank(&b.load_hint))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(OllamaDiscoveryReport {
        host,
        models: insights,
        ollama_reachable: true,
        error: None,
    })
}
