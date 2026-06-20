<!-- ./src/lib/components/agent/AgentQuickLaunch.svelte -->
<!--
Module Docstring:
Purpose: Component to render shortcuts for high-impact genomic findings detected in the user's report.
Responsibilities:
- Render a grid of shortcut buttons for user variants of interest.
- Notify the parent component when a variant is selected for agentic research.
Key Inputs: `quickFindings` list, `onLaunch` callback function.
Key Outputs: User click events triggering agentic research.
Operational Notes: Adheres to Svelte 5 runes and strictly stays under the 500-line limit.
-->
<script lang="ts">
  interface Props {
    quickFindings: {
      rsid: string;
      gene: string;
      user_genotype: string;
      severity_class: string;
    }[];
    onLaunch: (rsid: string, gene: string) => void;
  }

  let { quickFindings, onLaunch }: Props = $props();
</script>

<div class="setup-quick-launch card">
  <h3>🧬 Quick Launch Report Findings</h3>
  <p class="card-hint">
    Click any active marker found in your Trait Report to launch a complete agentic research investigation.
  </p>

  {#if quickFindings.length === 0}
    <div class="empty-quick-list">
      No active findings detected. Please import a genome first.
    </div>
  {:else}
    <div class="findings-grid">
      {#each quickFindings as f}
        <button class="finding-shortcut-btn" onclick={() => onLaunch(f.rsid, f.gene)}>
          <div class="shortcut-header">
            <span class="rsid font-mono">{f.rsid}</span>
            <span class="gene font-bold">{f.gene}</span>
          </div>
          <div class="shortcut-body">
            <span class="genotype font-mono">Genotype: {f.user_genotype}</span>
            <span class="severity-tag {f.severity_class || 'unknown'}">{(f.severity_class || 'unknown').replace("_", " ")}</span>
          </div>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .setup-quick-launch {
    display: flex;
    flex-direction: column;
  }
  .empty-quick-list {
    font-size: 0.8rem;
    color: var(--text-secondary);
    text-align: center;
    padding: 40px;
  }
  .findings-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
    max-height: 480px;
    overflow-y: auto;
    padding-right: 4px;
  }
  .finding-shortcut-btn {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 10px 12px;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .finding-shortcut-btn:hover {
    background: rgba(88, 80, 236, 0.05);
    border-color: var(--accent);
    transform: translateY(-2px);
  }
  .shortcut-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .shortcut-header .rsid {
    font-size: 0.8rem;
    color: var(--text-primary);
    font-weight: 600;
  }
  .shortcut-header .gene {
    font-size: 0.72rem;
    color: var(--accent);
  }
  .shortcut-body {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.7rem;
    color: var(--text-secondary);
  }
  .severity-tag {
    font-size: 0.6rem;
    padding: 1px 4px;
    border-radius: 3px;
    font-weight: 500;
  }
  .severity-tag.high_risk {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
  }
  .severity-tag.moderate_risk {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
  }
  .severity-tag.protective {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
  }
  .severity-tag.no_data {
    background: rgba(107, 114, 128, 0.15);
    color: #9ca3af;
  }
  .font-mono { font-family: monospace; }
  .font-bold { font-weight: 700; }
</style>
