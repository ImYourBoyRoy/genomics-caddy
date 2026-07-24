<!-- ./src/lib/components/research/PackDraftPanel.svelte -->
<script lang="ts">
  /*
  Purpose: Export reviewable marker-pack drafts and merge into research_found only.
  Curated packs are never modified.
  */

  import {
    exportPackDraftFromVectors,
    listPackDraftExports,
    mergePackDraftIntoPack,
    reloadMarkerPacks,
  } from "../../api/tauri";
  import { markerPacksStore } from "../../utils/markerPacksState.svelte";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import "$lib/styles/components/vector-workbench.css";

  interface Props {
    sampleId: number;
    sampleName: string;
    ollamaUrl: string;
    actionsEnabled?: boolean;
    onLog?: (msg: string) => void;
  }

  let { sampleId, sampleName, ollamaUrl, actionsEnabled = true, onLog }: Props = $props();

  const TARGET_PACK = "research_found";

  let minScore = $state(0.35);
  let busy = $state(false);
  let listing = $state(false);
  let message = $state("");
  let error = $state("");
  let lastPath = $state("");
  let drafts = $state<string[]>([]);
  let selectedDraft = $state("");
  /** When on, skips markers still needing gene/allele review (UNKNOWN / ?). */
  let requireComplete = $state(false);
  let progressMessage = $state("");

  let locked = $derived(!actionsEnabled || busy);

  async function refreshList() {
    listing = true;
    try {
      drafts = await listPackDraftExports();
      if (!selectedDraft && drafts.length) selectedDraft = drafts[0];
    } catch {
      drafts = [];
    } finally {
      listing = false;
    }
  }

  async function exportDraft() {
    if (locked) return;
    busy = true;
    error = "";
    message = "";
    progressMessage = "Exporting pack draft from vectors…";
    try {
      const result = await exportPackDraftFromVectors(
        sampleId,
        sampleName,
        ollamaUrl,
        minScore
      );
      lastPath = result.path;
      selectedDraft = result.path;
      message = result.message;
      onLog?.(result.message + ` → ${result.path}`);
      await refreshList();
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      progressMessage = "";
    }
  }

  async function mergeDraft() {
    if (locked) return;
    const draft = selectedDraft || lastPath;
    if (!draft) {
      error = "Select or export a draft first.";
      return;
    }
    if (
      !confirm(
        `Merge draft markers into App/Data/marker-packs/${TARGET_PACK}.json?\n\nCurated packs (metabolic, sleep, etc.) are never modified. A .bak backup is written if research_found already exists.`
      )
    ) {
      return;
    }
    busy = true;
    error = "";
    message = "";
    progressMessage = "Merging draft into research_found…";
    try {
      await reloadMarkerPacks();
      const result = await mergePackDraftIntoPack(
        draft,
        TARGET_PACK,
        undefined,
        requireComplete
      );
      message =
        result.message + (result.backup_path ? ` Backup: ${result.backup_path}` : "");
      onLog?.(message);
      await markerPacksStore.reload();
      await refreshList();
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      progressMessage = "";
    }
  }

  $effect(() => {
    if (sampleId && actionsEnabled) void refreshList();
  });
</script>

<section class="vector-workbench-card" aria-labelledby="pack-draft-title">
  <header class="vector-workbench-header">
    <div>
      <h3 id="pack-draft-title">Pack enrichment drafts</h3>
      <p class="vector-workbench-lead">
        Grow <code>{TARGET_PACK}</code> from vector research. Format matches curated packs
        (<code>name</code> + <code>markers</code>). Existing hardcoded packs are protected and never
        altered — including runtime copies under <code>App/Data/marker-packs/</code>.
      </p>
    </div>
  </header>

  {#if !actionsEnabled}
    <div class="workbench-boot-banner" role="status">
      <ActivityPulse message="Waiting for connections…" accent="#5eead4" />
    </div>
  {/if}

  {#if busy || listing}
    <div class="workbench-boot-banner" role="status">
      <ActivityPulse
        message={progressMessage || (listing ? "Loading draft list…" : "Working…")}
        accent="#38bdf8"
      />
    </div>
  {/if}

  <div class="pack-draft-controls">
    <label class="field">
      <span>Min significance</span>
      <input type="number" min="0" max="1" step="0.05" bind:value={minScore} disabled={locked} />
    </label>
    <button type="button" class="btn btn-primary btn-sm" disabled={locked} onclick={exportDraft}>
      {busy ? "Working…" : "Export draft pack JSON"}
    </button>
    <button type="button" class="btn btn-secondary btn-sm" disabled={locked || listing} onclick={refreshList}>
      Refresh list
    </button>
  </div>

  <div class="pack-merge-block">
    <h4 class="subhead">Merge into {TARGET_PACK}</h4>
    <p class="vector-workbench-note">
      Multi-day / rate-limited sweeps are expected — export paginates the full index (up to 100k
      points). Hypothesis markers land only in <code>{TARGET_PACK}</code>
      (<code>default_enabled: false</code>).
    </p>
    <div class="pack-draft-controls">
      <label class="field grow">
        <span>Draft file</span>
        <select bind:value={selectedDraft} disabled={locked}>
          <option value="">— select draft —</option>
          {#each drafts as path (path)}
            <option value={path}>{path}</option>
          {/each}
        </select>
      </label>
      <label class="check-row compact">
        <input type="checkbox" bind:checked={requireComplete} disabled={locked} />
        <span>Skip incomplete (UNKNOWN gene / ? allele)</span>
      </label>
      <button type="button" class="btn btn-accent btn-sm" disabled={locked} onclick={mergeDraft}>
        Merge into {TARGET_PACK}
      </button>
    </div>
  </div>

  {#if message}
    <p class="vector-workbench-note">{message}</p>
  {/if}
  {#if lastPath}
    <p class="vector-workbench-path font-mono">{lastPath}</p>
  {/if}
  {#if error}
    <p class="vector-workbench-error" role="alert">{error}</p>
  {/if}

  <ol class="pack-draft-steps">
    <li>Run Sweep Scopes (days-long / rate-limited runs are fine).</li>
    <li>Review hits in Viewer / Vector map.</li>
    <li>Export a draft (promoted + full browsed index above the score floor).</li>
    <li>Merge into <code>{TARGET_PACK}</code> only — then verify alleles/tiers as needed.</li>
    <li>Enable the pack in AI/report context when you want discoveries included.</li>
  </ol>

  {#if drafts.length}
    <h4 class="subhead">Recent drafts</h4>
    <ul class="draft-list">
      {#each drafts.slice(0, 12) as path (path)}
        <li class="font-mono">{path}</li>
      {/each}
    </ul>
  {:else}
    <p class="vector-workbench-empty">No drafts under App/Data/exports/pack_drafts yet.</p>
  {/if}
</section>
