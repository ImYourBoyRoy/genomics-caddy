<!-- ./src/lib/components/research/ResearchJobCard.svelte -->
<script lang="ts">
  import type { ResearchJob } from "../../types/research";

  interface Props {
    job: ResearchJob | null;
    onclick?: () => void;
  }

  let { job = null, onclick }: Props = $props();

  let percentage = $derived(
    job && job.total_markers > 0
      ? Math.round((job.enriched_count / job.total_markers) * 100)
      : 0
  );
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="research-job-card" class:clickable={!!onclick} onclick={onclick}>
  <div class="card-content">
    {#if !job || job.status === 'idle'}
      <span class="status-icon">🔬</span>
      <span class="status-text">Research: Not Started</span>
    {:else if job.status === 'running'}
      <span class="pulse-dot"></span>
      <span class="status-text">
        Researching: <strong class="rsid">{job.current_rsid || '...'}</strong>
      </span>
      <span class="progress-stats">{job.enriched_count}/{job.total_markers} ({percentage}%)</span>
    {:else if job.status === 'paused'}
      <span class="status-icon">⏸</span>
      <span class="status-text">Paused</span>
      <span class="progress-stats">{job.enriched_count}/{job.total_markers} ({percentage}%)</span>
    {:else if job.status === 'complete'}
      <span class="status-icon text-success">✅</span>
      <span class="status-text font-semibold">Research Complete</span>
      <span class="progress-stats text-success">{job.enriched_count} SNPs</span>
    {:else if job.status === 'error'}
      <span class="status-icon text-danger">⚠️</span>
      <span class="status-text text-danger font-semibold">Research Error</span>
      <span class="progress-stats text-danger">{percentage}%</span>
    {/if}
  </div>
</div>

<style>
  .research-job-card {
    display: inline-flex;
    align-items: center;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    padding: 6px 14px;
    font-size: 0.72rem;
    color: var(--text-primary);
    transition: all 0.2s ease;
    user-select: none;
  }

  .research-job-card.clickable {
    cursor: pointer;
  }

  .research-job-card.clickable:hover {
    background: rgba(255, 255, 255, 0.06);
    border-color: var(--accent);
    box-shadow: 0 0 10px rgba(139, 92, 246, 0.15);
  }

  .card-content {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-icon {
    font-size: 0.8rem;
    display: flex;
    align-items: center;
  }

  .status-text {
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .rsid {
    color: var(--text-primary);
    font-family: monospace;
  }

  .progress-stats {
    font-family: monospace;
    font-size: 0.68rem;
    color: var(--accent);
    font-weight: 600;
    background: rgba(139, 92, 246, 0.15);
    padding: 1px 6px;
    border-radius: 10px;
    margin-left: 2px;
  }

  .text-success {
    color: #10b981 !important;
  }

  .text-danger {
    color: #ef4444 !important;
  }

  .font-semibold {
    font-weight: 600;
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    background-color: #f59e0b;
    border-radius: 50%;
    display: inline-block;
    box-shadow: 0 0 0 0 rgba(245, 158, 11, 0.7);
    animation: pulse 1.6s infinite;
  }

  @keyframes pulse {
    0% {
      transform: scale(0.9);
      box-shadow: 0 0 0 0 rgba(245, 158, 11, 0.7);
    }
    70% {
      transform: scale(1);
      box-shadow: 0 0 0 6px rgba(245, 158, 11, 0);
    }
    100% {
      transform: scale(0.9);
      box-shadow: 0 0 0 0 rgba(245, 158, 11, 0);
    }
  }
</style>
