<!-- ./src/lib/components/ai/settings/AdvancedSettingsSection.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    getAppPaths,
    purgeDatabaseCache,
  } from "../../../api/tauri";
  import type { AppPaths } from "../../../types/genomics";
  import QdrantResearchSettingsDetails from "./QdrantResearchSettingsDetails.svelte";
  import { saveOllamaUrl } from "../../../utils/ollamaSettings";
  import "$lib/styles/components/advanced-settings-section.css";

  interface McpServerConfig {
    name: string;
    url: string;
  }

  interface Props {
    ollamaUrl?: string;
    ollamaToken?: string;
  }

  let { 
    ollamaUrl = $bindable(""), 
    ollamaToken = $bindable("") 
  }: Props = $props();

  // ── State — declared BEFORE any $effect to guarantee correct init order ───
  let paths = $state<AppPaths | null>(null);
  let cacheTtlDays = $state<number>(
    typeof localStorage !== "undefined" && localStorage.getItem("genomics_cache_ttl_days")
      ? parseInt(localStorage.getItem("genomics_cache_ttl_days")!, 10) || 7
      : 7
  );
  let purgeStatus = $state<"idle" | "clearing" | "success" | "error">("idle");
  let purgeMessage = $state<string>("");
  let newMcpName = $state<string>("");
  let newMcpUrl = $state<string>("");
  let externalMcpServers = $state<McpServerConfig[]>(
    typeof localStorage !== "undefined" && localStorage.getItem("genomics_external_mcp_servers")
      ? (() => {
          try {
            return JSON.parse(localStorage.getItem("genomics_external_mcp_servers")!) || [];
          } catch {
            return [];
          }
        })()
      : []
  );

  // Qdrant / research settings live in QdrantResearchSettingsDetails.svelte

  // Search source toggles — defaults are "true, true, true, false, true"
  let searchPubMed = $state<boolean>(
    typeof localStorage !== "undefined" && localStorage.getItem("genomics_search_pubmed") !== null
      ? localStorage.getItem("genomics_search_pubmed") !== "false"
      : true
  );
  let searchClinVar = $state<boolean>(
    typeof localStorage !== "undefined" && localStorage.getItem("genomics_search_clinvar") !== null
      ? localStorage.getItem("genomics_search_clinvar") !== "false"
      : true
  );
  let searchDbSnp = $state<boolean>(
    typeof localStorage !== "undefined" && localStorage.getItem("genomics_search_dbsnp") !== null
      ? localStorage.getItem("genomics_search_dbsnp") !== "false"
      : true
  );
  let searchWeb = $state<boolean>(
    typeof localStorage !== "undefined" && localStorage.getItem("genomics_search_web") !== null
      ? localStorage.getItem("genomics_search_web") === "true"
      : false
  );
  let searchQdrant = $state<boolean>(
    typeof localStorage !== "undefined" && localStorage.getItem("genomics_search_qdrant") !== null
      ? localStorage.getItem("genomics_search_qdrant") !== "false"
      : true
  );

  function defaultDomains(): string[] {
    return ["wikipedia.org", "snpedia.com"];
  }

  let approvedDomains = $state<string[]>(
    typeof localStorage !== "undefined" && localStorage.getItem("genomics_search_approved_domains")
      ? (() => {
          try {
            return JSON.parse(localStorage.getItem("genomics_search_approved_domains")!) || defaultDomains();
          } catch {
            return defaultDomains();
          }
        })()
      : defaultDomains()
  );
  let newDomain = $state<string>("");

  // ── Persistence Effects ───────────────────────────────────────────────────

  // Single consolidated $effect for all search toggles — avoids separate subscriptions.
  $effect(() => {
    if (typeof localStorage === "undefined") return;
    localStorage.setItem("genomics_search_pubmed", String(searchPubMed));
    localStorage.setItem("genomics_search_clinvar", String(searchClinVar));
    localStorage.setItem("genomics_search_dbsnp", String(searchDbSnp));
    localStorage.setItem("genomics_search_web", String(searchWeb));
    localStorage.setItem("genomics_search_qdrant", String(searchQdrant));
  });

  $effect(() => {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("genomics_cache_ttl_days", String(cacheTtlDays));
    }
  });

  $effect(() => {
    if (typeof localStorage !== "undefined") {
      saveOllamaUrl(ollamaUrl);
    }
  });

  // ── Lifecycle ─────────────────────────────────────────────────────────────
  onMount(async () => {
    try {
      paths = await getAppPaths();
    } catch (e) {
      console.error("Failed to load app paths:", e);
    }

    if (typeof localStorage === "undefined") return;

    if (!localStorage.getItem("genomics_search_approved_domains")) {
      localStorage.setItem("genomics_search_approved_domains", JSON.stringify(approvedDomains));
    }
  });

  // ── Cache Actions ─────────────────────────────────────────────────────────
  async function handleClearCache() {
    purgeStatus = "clearing";
    purgeMessage = "";
    try {
      await purgeDatabaseCache();
      purgeStatus = "success";
      purgeMessage = "Cache cleared successfully!";
      setTimeout(() => { purgeStatus = "idle"; purgeMessage = ""; }, 3000);
    } catch (e: any) {
      purgeStatus = "error";
      purgeMessage = `Failed: ${e.message || String(e)}`;
    }
  }

  // ── MCP Server Actions ────────────────────────────────────────────────────
  function saveMcpServers() {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("genomics_external_mcp_servers", JSON.stringify(externalMcpServers));
    }
  }

  function addMcpServer() {
    if (!newMcpName.trim() || !newMcpUrl.trim()) return;
    externalMcpServers = [...externalMcpServers, { name: newMcpName.trim(), url: newMcpUrl.trim() }];
    newMcpName = "";
    newMcpUrl = "";
    saveMcpServers();
  }

  function removeMcpServer(index: number) {
    externalMcpServers = externalMcpServers.filter((_, idx) => idx !== index);
    saveMcpServers();
  }

  // ── Domain Actions ────────────────────────────────────────────────────────
  function addDomain() {
    if (!newDomain.trim()) return;
    const clean = newDomain.trim().toLowerCase();
    if (!approvedDomains.includes(clean)) {
      approvedDomains = [...approvedDomains, clean];
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("genomics_search_approved_domains", JSON.stringify(approvedDomains));
      }
    }
    newDomain = "";
  }

  function removeDomain(index: number) {
    approvedDomains = approvedDomains.filter((_, idx) => idx !== index);
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("genomics_search_approved_domains", JSON.stringify(approvedDomains));
    }
  }
</script>

<details class="settings-details-group">
  <summary class="settings-details-summary">⚙️ Advanced Settings</summary>
  <div class="settings-details-content">
    
    <!-- Cache TTL -->
    <div class="setting-row">
      <div class="row-header">
        <label for="cache-ttl-slider">Cache Lifetime (TTL)</label>
        <span class="value-badge">{cacheTtlDays} {cacheTtlDays === 1 ? 'day' : 'days'}</span>
      </div>
      <input 
        id="cache-ttl-slider"
        type="range" 
        min="1" 
        max="30" 
        bind:value={cacheTtlDays} 
        class="slider-input"
      />
      <span class="help-text">Controls offline caching of API records and evidence lookups.</span>
    </div>

    <!-- Purge Cache -->
    <div class="setting-row mt-2">
      <button 
        class="btn btn-secondary w-full"
        onclick={handleClearCache}
        disabled={purgeStatus === "clearing"}
      >
        {purgeStatus === "clearing" ? "🧹 Clearing Cache..." : "🧹 Clear Local Cache"}
      </button>
      {#if purgeMessage}
        <div class="status-msg {purgeStatus}">{purgeMessage}</div>
      {/if}
      <span class="help-text mt-1">Clears Ensembl APIs cache and RAG evidence library vector embeddings.</span>
    </div>

    <!-- Storage Paths -->
    <div class="setting-row mt-2">
      <span class="section-label">📂 App Data Paths</span>
      {#if paths}
        <div class="path-box">
          <span class="path-title">Data root ({paths.data_dir_mode.replaceAll("_", " ")}):</span>
          <code class="font-mono path-value" title={paths.data_dir}>{paths.data_dir}</code>
        </div>
        <div class="path-box mt-1">
          <span class="path-title">Database:</span>
          <code class="font-mono path-value" title={paths.db_path}>{paths.db_path}</code>
        </div>
        <div class="path-box mt-1">
          <span class="path-title">Offline downloads:</span>
          <code class="font-mono path-value" title={paths.raw_downloads_dir}>{paths.raw_downloads_dir}</code>
        </div>
        <div class="path-box mt-1">
          <span class="path-title">Chain File:</span>
          <code class="font-mono path-value" title={paths.chain_path}>{paths.chain_path}</code>
        </div>
        {#if paths.env_path}
          <div class="path-box mt-1">
            <span class="path-title">Local `.env`:</span>
            <code class="font-mono path-value" title={paths.env_path}>{paths.env_path}</code>
          </div>
          <span class="help-text mt-1">
            Copy <code>.env.example</code> to <code>App/.env</code> (or project root) for QDRANT_URL, QDRANT_API_KEY, NCBI_API_KEY, and OLLAMA_TOKEN.
            Persistent genome data lives in <code>App/Data/</code>. Legacy <code>data/</code> is still read when App/Data is empty.
            Secrets are stored in the OS keychain when saved from settings — never in git.
          </span>
        {/if}
      {:else}
        <span class="help-text">Loading storage paths...</span>
      {/if}
    </div>

    <!-- Search & Evidence Sources -->
    <div class="setting-row section-divider mt-2">
      <span class="section-label">🔍 Search &amp; Evidence Sources</span>
      
      <div class="toggle-list">
        <label class="toggle-item">
          <input type="checkbox" bind:checked={searchPubMed} />
          <span>PubMed (NCBI References)</span>
        </label>
        <label class="toggle-item">
          <input type="checkbox" bind:checked={searchClinVar} />
          <span>ClinVar (Clinical Significance)</span>
        </label>
        <label class="toggle-item">
          <input type="checkbox" bind:checked={searchDbSnp} />
          <span>dbSNP (Variant Coordinates)</span>
        </label>
        <label class="toggle-item">
          <input type="checkbox" bind:checked={searchWeb} />
          <span>Web Search (DuckDuckGo Lite)</span>
        </label>
        <label class="toggle-item">
          <input type="checkbox" bind:checked={searchQdrant} />
          <span>Qdrant Vector DB (Self-Hosted Knowledge)</span>
        </label>
      </div>

      {#if searchWeb}
        <div class="mcp-add-form domain-form mt-1">
          <span class="domain-list-label">Approved Web Domains</span>
          <div class="domain-input-row">
            <input 
              type="text" 
              placeholder="e.g. wikipedia.org" 
              bind:value={newDomain} 
              class="mcp-input domain-input" 
              onkeydown={(e) => { if (e.key === "Enter") { e.preventDefault(); addDomain(); } }}
            />
            <button class="btn btn-secondary btn-add-domain" onclick={addDomain} disabled={!newDomain.trim()}>
              ➕
            </button>
          </div>
          
          {#if approvedDomains.length > 0}
            <div class="domain-chips">
              {#each approvedDomains as domain, index}
                <span class="domain-chip">
                  {domain}
                  <button
                    onclick={() => removeDomain(index)}
                    class="domain-chip-remove"
                    aria-label="Remove domain"
                  >
                    ❌
                  </button>
                </span>
              {/each}
            </div>
          {:else}
            <span class="help-text help-text--warn">⚠️ No domains added! Web searches will query the entire internet.</span>
          {/if}
        </div>
      {/if}
    </div>

    <!-- External MCP Connections -->
    <div class="setting-row section-divider mt-2">
      <span class="section-label">🔌 External MCP Connections</span>
      
      <div class="mcp-add-form">
        <input type="text" placeholder="Server Name" bind:value={newMcpName} class="mcp-input" />
        <input type="text" placeholder="SSE URL (e.g. http://localhost:3000/sse)" bind:value={newMcpUrl} class="mcp-input mt-1" />
        <button class="btn btn-secondary w-full mt-1" onclick={addMcpServer} disabled={!newMcpName.trim() || !newMcpUrl.trim()}>
          ➕ Add Connection
        </button>
      </div>

      {#if externalMcpServers.length > 0}
        <div class="mcp-list mt-2">
          {#each externalMcpServers as server, index}
            <div class="mcp-item">
              <div class="mcp-info">
                <span class="mcp-name">{server.name}</span>
                <span class="mcp-url font-mono">{server.url}</span>
              </div>
              <button class="btn-remove" onclick={() => removeMcpServer(index)} aria-label="Remove connection">❌</button>
            </div>
          {/each}
        </div>
      {/if}

      <div class="mcp-preview-alert mt-1">
        💡 <strong>Preview Feature:</strong> Configured external MCP connections will allow the AI assistant to dynamically query third-party clinical services in a future release.
      </div>
    </div>

  </div>
</details>

<QdrantResearchSettingsDetails bind:ollamaUrl bind:ollamaToken />
