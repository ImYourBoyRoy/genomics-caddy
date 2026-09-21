<!-- ./src/lib/components/agent/DiscoveredVariantsList.svelte -->
<!--
Module Docstring:
Purpose: List view and checklist categorization for discovered genomic variants.
Responsibilities:
- Group discovered variants by clinical/research classification tabs.
- Support bulk selection and individual variant inclusion toggles.
- Render ClinVar details, genes, genotypes, and audit notes.
Key Inputs: List of VariantEvidence, selected sample status.
Key Outputs: Bound checkbox selections.
Operational Notes: Stays under the 500-line limit. Scoped styles.
-->
<script lang="ts">
  import type { GenomeSample } from "../../types/genomics";
  import type { VariantEvidence } from "../../types/agent";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";

  type DiscoveredVariant = VariantEvidence & { checked?: boolean };

  interface Props {
    selectedSample: GenomeSample | null;
    discoveredVariants: DiscoveredVariant[];
    isScanningDb: boolean;
    scanProgressMsg: string;
  }

  let {
    selectedSample,
    discoveredVariants = $bindable([]),
    isScanningDb,
    scanProgressMsg
  }: Props = $props();

  let activeChecklistTab = $state<"clinical" | "research" | "modifiers" | "blocked" | "inactive">("clinical");

  let filteredVariants = $derived(
    discoveredVariants.filter(v => {
      const status = v.interpretation_status;
      if (activeChecklistTab === "clinical") return status === "active_clinical";
      if (activeChecklistTab === "research") return status === "active_research";
      if (activeChecklistTab === "modifiers") return status === "modifier_only" || status === "benign";
      if (activeChecklistTab === "blocked") return status === "blocked_unverified" || status === "conflicting";
      return status === "not_active_for_user" || status === "insufficient_evidence";
    })
  );

  function handleSelectAll(val: boolean) {
    for (const item of filteredVariants) {
      item.checked = val;
    }
    discoveredVariants = [...discoveredVariants];
  }
</script>

<div class="discovered-box">
  <div class="discovered-header">
    <span class="label-header">Discovered Genotypes ({discoveredVariants.length})</span>
    {#if discoveredVariants.length > 0}
      <div class="bulk-actions">
        <button type="button" class="text-btn" onclick={() => handleSelectAll(true)}>Select All</button>
        <span class="divider">|</span>
        <button type="button" class="text-btn" onclick={() => handleSelectAll(false)}>Clear</button>
      </div>
    {/if}
  </div>

  <!-- Clinical Grade Filter Tabs -->
  <div class="checklist-tabs">
    <button
      type="button"
      class="checklist-tab-btn"
      class:active={activeChecklistTab === "clinical"}
      onclick={() => activeChecklistTab = "clinical"}
    >
      🔴 Active Clinical
    </button>
    <button
      type="button"
      class="checklist-tab-btn"
      class:active={activeChecklistTab === "research"}
      onclick={() => activeChecklistTab = "research"}
    >
      🟡 Active Research
    </button>
    <button
      type="button"
      class="checklist-tab-btn"
      class:active={activeChecklistTab === "modifiers"}
      onclick={() => activeChecklistTab = "modifiers"}
    >
      🔵 Modifiers
    </button>
    <button
      type="button"
      class="checklist-tab-btn"
      class:active={activeChecklistTab === "blocked"}
      onclick={() => activeChecklistTab = "blocked"}
    >
      ⚠️ Blocked
    </button>
    <button
      type="button"
      class="checklist-tab-btn"
      class:active={activeChecklistTab === "inactive"}
      onclick={() => activeChecklistTab = "inactive"}
    >
      ⚪ Inactive
    </button>
  </div>

  <div class="discovered-list-container">
    {#if !selectedSample}
      <div class="no-variants-message font-mono">
        ⚠️ Select a genome profile from the dashboard first.
      </div>
    {:else if isScanningDb && discoveredVariants.length === 0}
      <div class="no-variants-message active-progress">
        <ActivityPulse message={scanProgressMsg} accent="var(--status-info-text)" maxWidth="100%" />
      </div>
    {:else if filteredVariants.length === 0}
      <div class="no-variants-message font-mono">
        ℹ️ No variants found under this category tab.
      </div>
    {:else}
      <div class="discovered-list">
        {#each filteredVariants as v}
          <label class="variant-item-label" class:selected={v.checked}>
            <input type="checkbox" bind:checked={v.checked} />
            <div class="variant-item-content">
              <div class="variant-meta">
                <span class="variant-rs font-mono">{v.rsid}</span>
                <span class="variant-gene font-bold">{v.gene || "Unknown"}</span>
                <span class="variant-genotype font-mono">({v.user_genotype || "No Data"})</span>
              </div>
              <div class="variant-impact" class:risk-text={v.checked}>
                {v.clinvar_clinical_significance || "No ClinVar significance"}
              </div>
              {#if v.clinvar_condition}
                <div class="variant-desc">{v.clinvar_condition}</div>
              {/if}
              {#if v.notes && v.notes.length > 0}
                <div class="variant-notes font-mono">{v.notes.join("; ")}</div>
              {/if}
            </div>
          </label>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .discovered-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 12px;
    background: rgba(0, 0, 0, 0.1);
  }
  .discovered-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .label-header {
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-weight: 500;
  }
  .bulk-actions {
    display: flex;
    gap: 6px;
    font-size: 0.7rem;
    align-items: center;
  }
  .text-btn {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    padding: 0;
  }
  .text-btn:hover {
    text-decoration: underline;
  }
  .divider {
    color: var(--border-color);
  }

  /* Checklist tabs */
  .checklist-tabs {
    display: flex;
    gap: 4px;
    overflow-x: auto;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    padding-bottom: 6px;
    margin-bottom: 4px;
  }
  .checklist-tab-btn {
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 0.65rem;
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.2s;
  }
  .checklist-tab-btn:hover {
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-primary);
  }
  .checklist-tab-btn.active {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
    font-weight: 600;
  }

  .discovered-list-container {
    max-height: 200px;
    overflow-y: auto;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    background: rgba(0, 0, 0, 0.2);
  }
  .no-variants-message {
    padding: 24px;
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-align: center;
  }
  .no-variants-message.active-progress {
    color: var(--accent);
  }
  .discovered-list {
    display: flex;
    flex-direction: column;
  }
  .variant-item-label {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
    cursor: pointer;
    transition: background-color 0.2s;
  }
  .variant-item-label:hover {
    background: rgba(255, 255, 255, 0.02);
  }
  .variant-item-label.selected {
    background: rgba(88, 80, 236, 0.04);
  }
  .variant-item-label input[type="checkbox"] {
    margin-top: 3px;
    cursor: pointer;
  }
  .variant-item-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .variant-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
  }
  .variant-rs {
    color: var(--text-primary);
    font-weight: 600;
  }
  .variant-gene {
    color: var(--accent);
  }
  .variant-genotype {
    color: var(--text-secondary);
  }
  .variant-impact {
    font-size: 0.68rem;
    color: var(--text-secondary);
  }
  .variant-impact.risk-text {
    color: #f87171;
  }
  .variant-desc {
    font-size: 0.62rem;
    color: var(--text-secondary);
  }
  .variant-notes {
    font-size: 0.55rem;
    color: var(--accent);
    opacity: 0.8;
  }

  .font-mono { font-family: monospace; }
  .font-bold { font-weight: 700; }
</style>
