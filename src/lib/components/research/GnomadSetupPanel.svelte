<!-- ./src/lib/components/research/GnomadSetupPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import {
    getGnomadConfig,
    saveGnomadConfig,
    getGnomadReadiness,
    downloadGnomadIndexes,
    testGnomadSourceUrls,
    selectGnomadLocalDir,
    clearGnomadCache,
  } from "../../api/tauri";
  import type { GnomadConfig, GnomadReadinessStatus } from "../../types/research";
  import { DEFAULT_GNOMAD_CONFIG } from "../../types/research";

  interface Props {
    compact?: boolean;
    onLog?: (msg: string) => void;
    onReadyChange?: (ready: boolean, status: GnomadReadinessStatus | null) => void;
  }

  let { compact = false, onLog, onReadyChange }: Props = $props();

  let cfg = $state<GnomadConfig>({ ...DEFAULT_GNOMAD_CONFIG });
  let readiness = $state<GnomadReadinessStatus | null>(null);
  let loading = $state(false);
  let busy = $state<"idle" | "check" | "download" | "folder" | "cache">("idle");
  let message = $state("");

  const gnomadReady = $derived(!!readiness && (!readiness.enabled || readiness.ready));
  const showMissing = $derived(
    !!readiness && readiness.enabled && readiness.missing_items.length > 0 && readiness.missing_items.length <= 8
  );
  const indexPct = $derived(
    readiness && readiness.indexes_expected > 0
      ? Math.round((readiness.indexes_cached / readiness.indexes_expected) * 100)
      : 0
  );
  const needsSetup = $derived(!!readiness && readiness.enabled && !readiness.ready);

  function notifyReady() {
    onReadyChange?.(gnomadReady, readiness);
  }

  async function loadAll() {
    loading = true;
    try {
      cfg = await getGnomadConfig();
      readiness = await getGnomadReadiness();
      notifyReady();
    } catch (e) {
      readiness = null;
      message = String(e);
      onReadyChange?.(true, null);
    } finally {
      loading = false;
    }
  }

  async function persistConfig(patch: Partial<GnomadConfig>) {
    cfg = { ...cfg, ...patch };
    try {
      await saveGnomadConfig(cfg);
      await loadAll();
    } catch (e) {
      message = String(e);
    }
  }

  async function handleCheck() {
    busy = "check";
    message = "";
    try {
      await persistConfig({});
      const test = await testGnomadSourceUrls();
      message = test.smoke_test_message ?? (test.errors[0] ?? "URL check complete.");
      onLog?.(message);
    } catch (e) {
      message = String(e);
    } finally {
      busy = "idle";
    }
  }

  async function handleDownloadIndexes() {
    busy = "download";
    message = "Downloading tabix indexes (small files, one-time setup)…";
    onLog?.(message);
    const poll = setInterval(() => {
      void getGnomadReadiness()
        .then((s) => {
          readiness = s;
          notifyReady();
        })
        .catch(() => {});
    }, 1500);
    try {
      const result = await downloadGnomadIndexes();
      message = result.message;
      onLog?.(result.message);
      if (result.errors.length) {
        onLog?.(result.errors.slice(0, 3).join("; "));
      }
      await loadAll();
    } catch (e) {
      message = String(e);
    } finally {
      clearInterval(poll);
      busy = "idle";
    }
  }

  async function handlePrimaryAction() {
    const action = readiness?.primary_action ?? "";
    if (action.startsWith("Download")) {
      await handleDownloadIndexes();
    } else if (action === "Choose local folder") {
      await handlePickFolder();
    } else if (action === "Open gnomAD downloads") {
      openDownloads();
    } else if (action === "Test source URLs") {
      await handleCheck();
    } else if (action === "Use Remote indexed VCF") {
      useRemoteMode();
    }
  }

  async function handleQuickSetup() {
    if (cfg.source_mode !== "remote_indexed_vcf_https") {
      await persistConfig({ source_mode: "remote_indexed_vcf_https" });
    }
    if (!readiness?.remote_urls_ok) {
      await handleCheck();
      await loadAll();
      if (!readiness?.remote_urls_ok) return;
    }
    if (needsSetup && readiness?.primary_action?.startsWith("Download")) {
      await handleDownloadIndexes();
    }
  }

  async function handlePickFolder() {
    busy = "folder";
    try {
      const path = await selectGnomadLocalDir();
      if (path) {
        message = `Using folder: ${path}`;
        onLog?.(message);
        await loadAll();
      }
    } catch (e) {
      message = String(e);
    } finally {
      busy = "idle";
    }
  }

  async function handleClearCache() {
    busy = "cache";
    try {
      const n = await clearGnomadCache();
      message = `Cleared ${n} cached gnomAD rows.`;
      onLog?.(message);
    } catch (e) {
      message = String(e);
    } finally {
      busy = "idle";
    }
  }

  function openDownloads() {
    const url = readiness?.downloads_page_url ?? "https://gnomad.broadinstitute.org/downloads";
    void openUrl(url);
  }

  function useRemoteMode() {
    void persistConfig({ source_mode: "remote_indexed_vcf_https" });
  }

  onMount(() => {
    void loadAll();
  });
</script>

<div class="gnomad-panel" class:compact>
  <div class="panel-head">
    <div>
      <strong>gnomAD population frequency</strong>
      <p class="help">
        Not clinical significance. Remote mode downloads manifest-matched tabix indexes only (not full gnomAD).
        Best for sweeps under ~10k variants/session — use Local mode for larger runs.
      </p>
    </div>
    {#if loading}
      <ActivityPulse message="Checking setup…" accent="#38bdf8" maxWidth="140px" />
    {:else if readiness}
      <span class:status-pill={true} class:ok={gnomadReady} class:warn={!gnomadReady && readiness.enabled}>
        {#if !readiness.enabled}
          Off
        {:else if gnomadReady}
          Ready
        {:else}
          Needs setup
        {/if}
      </span>
    {/if}
  </div>

  {#if readiness}
    <p class="summary">{readiness.summary}</p>
    {#if readiness.enabled && readiness.indexes_expected > 0}
      <p class="meta">
        Indexes cached: {readiness.indexes_cached}/{readiness.indexes_expected}
        · Release {readiness.release} · {readiness.provider.toUpperCase()}
        · Manifest: {readiness.manifest_exome_contigs.length} exome + {readiness.manifest_genome_contigs.length} genome contigs
      </p>
      {#if readiness.unsupported_contigs.length}
        <p class="meta unsupported">
          Unsupported contigs (skipped): {readiness.unsupported_contigs.join(", ")}
        </p>
      {/if}
      <div class="index-progress" aria-label="Tabix index download progress">
        <div class="index-progress-fill" style="width: {indexPct}%"></div>
      </div>
    {/if}
  {/if}

  <div class="controls">
    <label class="toggle">
      <input
        type="checkbox"
        checked={cfg.enabled}
        onchange={(e) => persistConfig({ enabled: (e.currentTarget as HTMLInputElement).checked })}
      />
      Enable gnomAD during enrichment
    </label>

    <div class="grid-2">
      <label>
        Mode
        <select
          value={cfg.source_mode}
          onchange={(e) =>
            persistConfig({
              source_mode: (e.currentTarget as HTMLSelectElement).value as GnomadConfig["source_mode"],
            })}
        >
          <option value="remote_indexed_vcf_https">Remote indexed VCF (recommended)</option>
          <option value="local_indexed_vcf">Local indexed VCF (large sweeps)</option>
          <option value="graphql_interactive">GraphQL only (single variants)</option>
        </select>
      </label>
      <label>
        Dataset policy
        <select
          value={cfg.dataset_policy}
          onchange={(e) =>
            persistConfig({
              dataset_policy: (e.currentTarget as HTMLSelectElement).value as GnomadConfig["dataset_policy"],
            })}
        >
          <option value="auto">Auto — exomes first, genomes if miss</option>
          <option value="combined">Combined — always both</option>
          <option value="exomes_only">Exomes only</option>
          <option value="genomes_only">Genomes only</option>
        </select>
      </label>
    </div>

    <div class="grid-2">
      <label>
        Provider
        <select
          value={cfg.provider}
          onchange={(e) =>
            persistConfig({
              provider: (e.currentTarget as HTMLSelectElement).value as GnomadConfig["provider"],
            })}
        >
          <option value="aws">AWS HTTPS</option>
          <option value="google">Google HTTPS</option>
        </select>
      </label>
      <label class="toggle inline-toggle">
        <input
          type="checkbox"
          checked={cfg.graphql_fallback_enabled}
          onchange={(e) =>
            persistConfig({ graphql_fallback_enabled: (e.currentTarget as HTMLInputElement).checked })}
        />
        GraphQL fallback for priority variants
      </label>
    </div>
  </div>

  <div class="actions">
    {#if needsSetup && cfg.source_mode === "remote_indexed_vcf_https"}
      <button
        type="button"
        class="btn btn-primary btn-sm"
        disabled={busy !== "idle"}
        onclick={handleQuickSetup}
      >
        {busy === "download" ? "Setting up…" : "One-click setup (recommended)"}
      </button>
    {/if}

    {#if readiness?.primary_action}
      <button
        type="button"
        class="btn btn-sm"
        class:btn-primary={!needsSetup || cfg.source_mode !== "remote_indexed_vcf_https"}
        class:btn-secondary={needsSetup && cfg.source_mode === "remote_indexed_vcf_https"}
        disabled={busy !== "idle"}
        onclick={handlePrimaryAction}
      >
        {#if busy === "download" && readiness.primary_action.startsWith("Download")}
          Downloading… ({readiness.indexes_cached}/{readiness.indexes_expected})
        {:else if busy === "folder" && readiness.primary_action === "Choose local folder"}
          Opening folder picker…
        {:else if busy === "check" && readiness.primary_action === "Test source URLs"}
          Testing URLs…
        {:else}
          {readiness.primary_action}
        {/if}
      </button>
    {/if}

    {#if cfg.source_mode === "local_indexed_vcf"}
      <button type="button" class="btn btn-secondary btn-sm" disabled={busy !== "idle"} onclick={handlePickFolder}>
        {busy === "folder" ? "Opening…" : "Choose local folder"}
      </button>
      <button type="button" class="btn btn-secondary btn-sm" onclick={openDownloads}>Open gnomAD downloads</button>
    {/if}

    {#if readiness && !readiness.remote_urls_ok}
      <button type="button" class="btn btn-secondary btn-sm" onclick={() => persistConfig({ provider: cfg.provider === "aws" ? "google" : "aws" })}>
        Try {cfg.provider === "aws" ? "Google" : "AWS"} provider
      </button>
    {/if}

    <button type="button" class="btn btn-secondary btn-sm" disabled={busy !== "idle"} onclick={() => loadAll()}>
      Refresh status
    </button>
    <button type="button" class="btn btn-secondary btn-sm" disabled={busy !== "idle"} onclick={handleClearCache}>
      Clear result cache
    </button>
  </div>

  {#if showMissing && readiness}
    <ul class="missing-list">
      {#each readiness.missing_items as item}
        <li>{item.label}</li>
      {/each}
    </ul>
  {:else if readiness && readiness.missing_items.length > 8}
    <p class="help">{readiness.missing_items.length} items missing — use the primary action button above.</p>
  {/if}

  {#if message}
    <p class="message">{message}</p>
  {/if}
</div>

<style>
  .gnomad-panel {
    border: 1px solid rgba(56, 189, 248, 0.2);
    background: rgba(56, 189, 248, 0.05);
    border-radius: 10px;
    padding: 14px;
    margin-top: 12px;
  }
  .gnomad-panel.compact {
    padding: 10px;
  }
  .panel-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: flex-start;
    margin-bottom: 8px;
  }
  .help,
  .summary,
  .meta,
  .message {
    font-size: 0.68rem;
    color: var(--text-secondary);
    margin: 0 0 6px;
  }
  .status-pill {
    font-size: 0.65rem;
    font-weight: 700;
    padding: 4px 8px;
    border-radius: 999px;
    background: rgba(251, 113, 133, 0.15);
    color: #fb7185;
    white-space: nowrap;
  }
  .status-pill.ok {
    background: rgba(52, 211, 153, 0.15);
    color: #34d399;
  }
  .controls {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 10px 0;
  }
  .toggle {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 0.72rem;
  }
  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.68rem;
    color: var(--text-secondary);
  }
  select {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 4px;
    padding: 6px;
    font-size: 0.72rem;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .missing-list {
    margin: 8px 0 0;
    padding-left: 18px;
    font-size: 0.65rem;
    color: var(--text-secondary);
    max-height: 120px;
    overflow: auto;
  }
  .index-progress {
    height: 5px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    overflow: hidden;
    margin: 6px 0 4px;
  }
  .index-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #38bdf8, #34d399);
    transition: width 0.35s ease;
  }
  .unsupported {
    color: #fb923c;
  }
  .inline-toggle {
    flex-direction: row;
    align-items: center;
    justify-content: flex-start;
    margin-top: 18px;
  }
</style>
