<!-- ./src/lib/components/settings/ConnectionsPanel.svelte -->
<script lang="ts">
  /*
  Purpose: Advanced Connections hub for Ollama + vector DB (UI-saved wins over .env).
  Responsibilities: endpoints, localhost reset, missing-service probes, model pull/delete/update,
  version checks, experimental non-Qdrant health probes.
  */

  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    checkOllamaUpdate,
    checkQdrantUpdate,
    deleteOllamaModel,
    discoverOllamaModels,
    getOllamaServiceConfig,
    getQdrantConfig,
    probeInferenceHost,
    probeLocalhostServices,
    probeVectorProvider,
    pullOllamaModel,
    saveOllamaToken,
    saveQdrantConfig,
    testQdrantConnection,
    type InferenceHostProfile,
    type LocalhostServiceStatus,
    type OllamaModelInsight,
    type ServiceUpdateCheck,
  } from "../../api/tauri";
  import type { QdrantConfigPublic, QdrantConnectionStatus } from "../../types/research";
  import { DEFAULT_QDRANT_CONFIG, isEmbedModel } from "../../types/research";
  import {
    LOCAL_OLLAMA_URL,
    LOCAL_QDRANT_URL,
    OLLAMA_URL_PLACEHOLDER,
    persistOllamaUrl,
    resolveInitialOllamaUrl,
  } from "../../utils/ollamaSettings";
  import {
    connectionCheckTimeoutMs,
    withInvokeTimeout,
  } from "../../utils/researchConnection";
  import "$lib/styles/components/connections-panel.css";

  interface Props {
    ollamaUrl?: string;
    ollamaToken?: string;
    onOllamaUrlChange?: (url: string) => void;
    onOllamaTokenChange?: (token: string) => void;
  }

  let {
    ollamaUrl = $bindable(""),
    ollamaToken = $bindable(""),
    onOllamaUrlChange,
    onOllamaTokenChange,
  }: Props = $props();

  let config = $state<QdrantConfigPublic>({ ...DEFAULT_QDRANT_CONFIG });
  let qdrantUrl = $state("");
  let collection = $state("");
  let embedModel = $state("");
  let apiKey = $state("");
  let namedVectors = $state(false);
  let vectorProvider = $state<"qdrant" | "chroma" | "weaviate" | "milvus">("qdrant");

  let qdrantStatus = $state<QdrantConnectionStatus | null>(null);
  let ollamaStatus = $state<"untested" | "testing" | "live" | "dead">("untested");
  let host = $state<InferenceHostProfile | null>(null);
  let models = $state<OllamaModelInsight[]>([]);
  let localhost = $state<LocalhostServiceStatus | null>(null);
  let envOllamaHint = $state<string | null>(null);
  let busy = $state(false);
  let saveMsg = $state("");
  let errorMsg = $state("");
  let pullName = $state("");
  let pullProgress = $state("");
  let ollamaUpdate = $state<ServiceUpdateCheck | null>(null);
  let qdrantUpdate = $state<ServiceUpdateCheck | null>(null);
  let providerProbe = $state<string>("");
  let monitorTimer: ReturnType<typeof setInterval> | null = null;
  let unlistenPull: (() => void) | null = null;

  let embedModels = $derived(models.filter((m) => m.role === "embed" || isEmbedModel(m.name)));
  let needsSetup = $derived(!ollamaUrl.trim() && !qdrantUrl.trim());
  let researchReady = $derived(vectorProvider === "qdrant");
  let usingLocalhost = $derived(
    ollamaUrl.trim().includes("127.0.0.1") ||
      ollamaUrl.trim().includes("localhost") ||
      qdrantUrl.trim().includes("127.0.0.1") ||
      qdrantUrl.trim().includes("localhost")
  );
  let showLocalhostMissing = $derived(
    usingLocalhost &&
      !!localhost &&
      (!localhost.ollama_reachable || !localhost.qdrant_reachable)
  );

  function fmtGiB(bytes?: number | null): string {
    if (!bytes || bytes <= 0) return "—";
    return `${(bytes / 1e9).toFixed(1)} GiB`;
  }

  function postureLabel(p?: string): string {
    switch (p) {
      case "unified_memory":
        return "Unified memory";
      case "discrete_or_dedicated_gpu":
        return "Dedicated GPU";
      case "remote_managed":
        return "Remote host";
      case "cpu_only_local":
        return "Local CPU-only";
      default:
        return p || "Unknown";
    }
  }

  async function loadSaved() {
    try {
      const [cfg, url, svc] = await Promise.all([
        getQdrantConfig(),
        resolveInitialOllamaUrl(),
        getOllamaServiceConfig().catch(() => null),
      ]);
      config = cfg;
      qdrantUrl = cfg.url || "";
      collection = cfg.collection || "";
      embedModel = cfg.embedding_model || "";
      namedVectors = !!cfg.named_vectors_enabled;
      ollamaUrl = url;
      onOllamaUrlChange?.(url);
      envOllamaHint = svc?.env_url?.trim() || null;
    } catch (e: any) {
      errorMsg = e?.message || String(e);
    }
  }

  async function refreshLocalhost() {
    localhost = await probeLocalhostServices();
  }

  async function refreshOllamaDiscovery(explicit = false) {
    const url = ollamaUrl.trim();
    if (!url) {
      ollamaStatus = "dead";
      models = [];
      return;
    }
    ollamaStatus = "testing";
    try {
      const report = await withInvokeTimeout(
        discoverOllamaModels(url, ollamaToken || undefined),
        connectionCheckTimeoutMs(url, explicit),
        "Ollama discovery"
      );
      host = report.host;
      models = report.models;
      ollamaStatus = report.ollama_reachable ? "live" : "dead";
      if (!report.ollama_reachable) errorMsg = report.error || "Ollama unreachable";
      if (!embedModel && embedModels.length) embedModel = embedModels[0].name;
    } catch (e: any) {
      ollamaStatus = "dead";
      models = [];
      errorMsg = e?.message || String(e);
    }
  }

  async function refreshQdrant(explicit = false) {
    const url = qdrantUrl.trim();
    if (!url) {
      qdrantStatus = null;
      return;
    }
    if (vectorProvider !== "qdrant") {
      try {
        const probe = await probeVectorProvider(vectorProvider, url, apiKey.trim() || undefined);
        providerProbe = probe.note || (probe.reachable ? "Reachable" : "Unreachable");
        qdrantStatus = {
          success: !!probe.reachable,
          collection_exists: false,
          error: probe.reachable ? undefined : providerProbe,
        };
      } catch (e: any) {
        providerProbe = e?.message || String(e);
        qdrantStatus = { success: false, collection_exists: false, error: providerProbe };
      }
      return;
    }
    try {
      qdrantStatus = await withInvokeTimeout(
        testQdrantConnection(url, apiKey.trim() || undefined, collection.trim() || undefined),
        connectionCheckTimeoutMs(url, explicit),
        "Qdrant check"
      );
      providerProbe = "";
    } catch (e: any) {
      qdrantStatus = {
        success: false,
        collection_exists: false,
        error: e?.message || String(e),
      };
    }
  }

  async function runMonitor(explicit = false) {
    if (busy) return;
    busy = true;
    if (explicit) errorMsg = "";
    try {
      host = await probeInferenceHost(ollamaUrl.trim() || undefined);
      await Promise.all([refreshOllamaDiscovery(explicit), refreshQdrant(explicit), refreshLocalhost()]);
    } finally {
      busy = false;
    }
  }

  async function handleSave() {
    busy = true;
    saveMsg = "";
    errorMsg = "";
    try {
      if (!qdrantUrl.trim()) throw new Error("Set a vector DB URL before saving.");
      if (vectorProvider === "qdrant" && !collection.trim()) {
        throw new Error("Set a Qdrant collection name before saving.");
      }
      if (vectorProvider === "qdrant") {
        await saveQdrantConfig({
          url: qdrantUrl.trim(),
          collection: collection.trim(),
          embedding_model: embedModel.trim(),
          gwas_strict: config.gwas_strict,
          auto_start: config.auto_start,
          named_vectors_enabled: namedVectors,
          ...(apiKey.trim() ? { api_key: apiKey.trim() } : {}),
        });
      }
      await persistOllamaUrl(ollamaUrl.trim());
      await saveOllamaToken(ollamaToken.trim() || undefined);
      onOllamaUrlChange?.(ollamaUrl.trim());
      onOllamaTokenChange?.(ollamaToken.trim());
      apiKey = "";
      envOllamaHint = null;
      saveMsg = "Saved. UI settings now override .env until you clear them.";
      busy = false;
      await runMonitor(true);
    } catch (e: any) {
      errorMsg = e?.message || String(e);
      saveMsg = "";
      busy = false;
    }
  }

  async function resetToLocalhost() {
    ollamaUrl = LOCAL_OLLAMA_URL;
    qdrantUrl = LOCAL_QDRANT_URL;
    vectorProvider = "qdrant";
    await refreshLocalhost();
    if (localhost && (!localhost.ollama_reachable || !localhost.qdrant_reachable)) {
      errorMsg = [
        !localhost.ollama_reachable ? localhost.ollama_error : null,
        !localhost.qdrant_reachable ? localhost.qdrant_error : null,
        ...localhost.notes,
      ]
        .filter(Boolean)
        .join(" · ");
    } else {
      errorMsg = "";
      saveMsg = "Filled localhost URLs — click Save & Verify to persist.";
    }
  }

  async function adoptEnvOllama() {
    if (!envOllamaHint) return;
    ollamaUrl = envOllamaHint;
    await persistOllamaUrl(ollamaUrl);
    envOllamaHint = null;
    saveMsg = "Adopted OLLAMA_URL from .env and saved.";
    await runMonitor(true);
  }

  async function handlePull(name: string, asUpdate = false) {
    const model = name.trim();
    if (!model || !ollamaUrl.trim()) return;
    busy = true;
    pullProgress = asUpdate ? `Updating ${model}…` : `Pulling ${model}…`;
    errorMsg = "";
    try {
      await pullOllamaModel(ollamaUrl.trim(), model, ollamaToken || undefined);
      pullProgress = `${asUpdate ? "Updated" : "Installed"} ${model}`;
      pullName = "";
      busy = false;
      await refreshOllamaDiscovery(true);
    } catch (e: any) {
      errorMsg = e?.message || String(e);
      pullProgress = "";
      busy = false;
    }
  }

  async function handleDelete(name: string) {
    if (!confirm(`Delete model "${name}" from the Ollama server? This cannot be undone.`)) return;
    busy = true;
    try {
      await deleteOllamaModel(ollamaUrl.trim(), name, ollamaToken || undefined);
      await refreshOllamaDiscovery(true);
      saveMsg = `Deleted ${name}`;
    } catch (e: any) {
      errorMsg = e?.message || String(e);
    } finally {
      busy = false;
    }
  }

  async function handleCheckUpdates() {
    busy = true;
    errorMsg = "";
    try {
      const tasks: Promise<void>[] = [];
      if (ollamaUrl.trim()) {
        tasks.push(
          checkOllamaUpdate(ollamaUrl.trim(), ollamaToken || undefined).then((r) => {
            ollamaUpdate = r;
          })
        );
      }
      if (qdrantUrl.trim() && vectorProvider === "qdrant") {
        tasks.push(
          checkQdrantUpdate(qdrantUrl.trim(), apiKey.trim() || undefined).then((r) => {
            qdrantUpdate = r;
          })
        );
      }
      await Promise.all(tasks);
    } catch (e: any) {
      errorMsg = e?.message || String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void (async () => {
      unlistenPull = await listen<Record<string, unknown>>("ollama:pull_progress", (ev) => {
        const s = ev.payload?.status;
        const completed = ev.payload?.completed;
        const total = ev.payload?.total;
        if (typeof s === "string") {
          pullProgress =
            completed && total
              ? `${s} (${Math.round((Number(completed) / Number(total)) * 100)}%)`
              : String(s);
        }
      });
      await loadSaved();
      await runMonitor(false);
      monitorTimer = setInterval(() => void runMonitor(false), 30000);
    })();
  });

  onDestroy(() => {
    if (monitorTimer) clearInterval(monitorTimer);
    unlistenPull?.();
  });
</script>

<section class="connections-panel" aria-labelledby="connections-title">
  <header class="connections-hero">
    <div>
      <p class="connections-kicker">Advanced · Connections</p>
      <h3 id="connections-title">Ollama &amp; vector database</h3>
      <p class="connections-lead">
        Saved settings in the app win over <code>.env</code>. Use localhost reset for a local stack, or point at LAN / Tailscale hosts.
      </p>
    </div>
    <div class="connections-actions hero-actions">
      <button type="button" class="btn btn-secondary btn-sm" onclick={resetToLocalhost} disabled={busy}>
        Reset to localhost
      </button>
      <button type="button" class="btn btn-secondary btn-sm" onclick={() => runMonitor(true)} disabled={busy}>
        Re-scan
      </button>
      <button type="button" class="btn btn-primary btn-sm" onclick={handleSave} disabled={busy}>
        Save &amp; Verify
      </button>
    </div>
  </header>

  {#if needsSetup}
    <div class="connections-empty">
      No URLs saved yet. Enter endpoints below, or <strong>Reset to localhost</strong> then save.
    </div>
  {/if}

  {#if envOllamaHint}
    <div class="connections-banner warn">
      <span><code>.env</code> has a different Ollama URL: <code>{envOllamaHint}</code> (not applied — your saved setting wins).</span>
      <button type="button" class="btn btn-secondary btn-xs" onclick={adoptEnvOllama}>Adopt .env</button>
    </div>
  {/if}

  {#if showLocalhostMissing}
    <div class="connections-banner warn">
      <div>
        <strong>Localhost check:</strong>
        {#if localhost && !localhost.ollama_reachable}Ollama missing on {LOCAL_OLLAMA_URL}. {/if}
        {#if localhost && !localhost.qdrant_reachable}Qdrant missing on {LOCAL_QDRANT_URL}. {/if}
        Point Connections at a remote host, or install/start the missing service.
      </div>
    </div>
  {/if}

  {#if errorMsg}
    <div class="connections-alert" role="alert">{errorMsg}</div>
  {/if}

  <div class="conn-status-row">
    <span class="conn-pill {ollamaStatus}">Ollama · {ollamaStatus}</span>
    <span class="conn-pill {qdrantStatus?.success ? 'live' : qdrantStatus ? 'dead' : 'untested'}">
      {vectorProvider} · {qdrantStatus?.success ? "live" : qdrantStatus ? "dead" : "untested"}
    </span>
    {#if saveMsg}<span class="conn-pill live">{saveMsg}</span>{/if}
    {#if pullProgress}<span class="conn-pill testing">{pullProgress}</span>{/if}
  </div>

  <div class="connections-grid">
    <div class="connections-card">
      <h4>Endpoints</h4>
      <div class="field">
        <label for="conn-provider">Vector provider</label>
        <select id="conn-provider" bind:value={vectorProvider}>
          <option value="qdrant">Qdrant (full research support)</option>
          <option value="chroma">Chroma (health probe only)</option>
          <option value="weaviate">Weaviate (health probe only)</option>
          <option value="milvus">Milvus (experimental)</option>
        </select>
        {#if !researchReady}
          <p class="field-hint">Research sweeps still require Qdrant. Other providers are for connectivity checks while we expand support.</p>
        {/if}
      </div>
      <div class="field">
        <label for="conn-ollama-url">Ollama URL</label>
        <input id="conn-ollama-url" type="url" bind:value={ollamaUrl} placeholder={OLLAMA_URL_PLACEHOLDER} autocomplete="off" />
      </div>
      <div class="field">
        <label for="conn-ollama-token">Ollama token (optional)</label>
        <input id="conn-ollama-token" type="password" bind:value={ollamaToken} placeholder="Bearer …" />
      </div>
      <div class="field">
        <label for="conn-qdrant-url">{vectorProvider === "qdrant" ? "Qdrant URL" : `${vectorProvider} URL`}</label>
        <input id="conn-qdrant-url" type="url" bind:value={qdrantUrl} placeholder="e.g. http://127.0.0.1:6333" autocomplete="off" />
      </div>
      <div class="field">
        <label for="conn-qdrant-key">API key (optional)</label>
        <input id="conn-qdrant-key" type="password" bind:value={apiKey} placeholder={config.api_key_set ? "•••• (keep existing)" : "API key"} />
      </div>
      {#if vectorProvider === "qdrant"}
        <div class="field">
          <label for="conn-collection">Collection</label>
          <input id="conn-collection" type="text" bind:value={collection} placeholder="collection_name" list="conn-collections" />
          <datalist id="conn-collections">
            {#each qdrantStatus?.collections ?? [] as name}
              <option value={name}>{name}</option>
            {/each}
          </datalist>
        </div>
        <div class="field">
          <label for="conn-embed">Embedding model</label>
          <select id="conn-embed" bind:value={embedModel}>
            {#if embedModel && !embedModels.some((m) => m.name === embedModel)}
              <option value={embedModel}>{embedModel}</option>
            {/if}
            {#each embedModels as m}
              <option value={m.name}>{m.name} · {m.load_hint}</option>
            {/each}
          </select>
        </div>
        <label class="check-row">
          <input type="checkbox" bind:checked={namedVectors} />
          <span>Enable named vectors</span>
        </label>
      {/if}
      {#if providerProbe}
        <p class="field-hint">{providerProbe}</p>
      {/if}
    </div>

    <div class="connections-card">
      <h4>Host posture</h4>
      {#if host}
        <div class="host-meta">
          <span class="host-chip">{host.platform}/{host.arch}</span>
          <span class="host-chip">{postureLabel(host.posture)}</span>
          {#each host.accel_backends as b}
            <span class="host-chip">{b}</span>
          {/each}
          <span class="host-chip">accel {fmtGiB(host.accel_bytes)}</span>
          <span class="host-chip">VRAM in use {fmtGiB(host.observed_vram_in_use_bytes)}</span>
        </div>
        <ul class="host-notes">
          {#each host.notes.slice(0, 5) as note}
            <li>{note}</li>
          {/each}
        </ul>
      {:else}
        <p class="connections-lead">Probe pending…</p>
      {/if}

      <h4 class="subhead">Service versions</h4>
      <div class="connections-actions">
        <button type="button" class="btn btn-secondary btn-sm" onclick={handleCheckUpdates} disabled={busy}>
          Check for updates
        </button>
      </div>
      {#if ollamaUpdate}
        <div class="update-box">
          <strong>
            Ollama {ollamaUpdate.installed_version || "?"}
            {#if ollamaUpdate.latest_version}
              · latest {ollamaUpdate.latest_version}
              {#if ollamaUpdate.update_available} (update available){/if}
            {/if}
          </strong>
          <ul class="host-notes">
            {#each ollamaUpdate.notes as n}<li>{n}</li>{/each}
          </ul>
        </div>
      {/if}
      {#if qdrantUpdate}
        <div class="update-box">
          <strong>
            Qdrant {qdrantUpdate.installed_version || "?"}
            {#if qdrantUpdate.latest_version}
              · latest {qdrantUpdate.latest_version}
              {#if qdrantUpdate.update_available} (update available){/if}
            {/if}
          </strong>
          <ul class="host-notes">
            {#each qdrantUpdate.notes as n}<li>{n}</li>{/each}
          </ul>
        </div>
      {/if}
    </div>
  </div>

  <div class="connections-card">
    <h4>Ollama models</h4>
    <div class="pull-row">
      <input
        type="text"
        bind:value={pullName}
        placeholder="Model to install (e.g. mxbai-embed-large, qwen2.5:14b)"
        aria-label="Model name to install"
      />
      <button
        type="button"
        class="btn btn-accent btn-sm"
        disabled={busy || !pullName.trim() || !ollamaUrl.trim()}
        onclick={() => handlePull(pullName, false)}
      >
        Install
      </button>
    </div>
    <p class="field-hint">Install uses <code>POST /api/pull</code>. Update re-pulls the same tag. Delete uses <code>/api/delete</code>.</p>

    {#if models.length === 0}
      <div class="connections-empty">Connect Ollama and scan to list models.</div>
    {:else}
      <div class="model-table-wrap">
        <table class="model-table">
          <thead>
            <tr>
              <th>Model</th>
              <th>Role</th>
              <th>Hint</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each models as m (m.name)}
              <tr>
                <td class="font-mono">{m.name}{#if m.currently_loaded} · loaded{/if}</td>
                <td>{m.role}</td>
                <td class="hint-{m.load_hint}">{m.load_hint}</td>
                <td class="model-actions">
                  <button type="button" class="btn btn-link btn-xs" disabled={busy} onclick={() => handlePull(m.name, true)}>Update</button>
                  <button type="button" class="btn btn-link btn-xs danger" disabled={busy} onclick={() => handleDelete(m.name)}>Remove</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</section>
