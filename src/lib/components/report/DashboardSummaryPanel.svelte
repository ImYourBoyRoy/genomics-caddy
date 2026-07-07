<!-- ./src/lib/components/report/DashboardSummaryPanel.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import type { GeneratedReport, SeverityClass } from '../../types/genomics';
  import { deriveActionablePlan, type ActionablePlan, type TopFinding, type LabTest, type SupplementItem } from '../../utils/actionabilityEngine';

  interface Props {
    report: GeneratedReport;
    onJumpToMarker?: (linkId: string) => void;
  }

  let { report, onJumpToMarker }: Props = $props();

  let plan = $derived<ActionablePlan>(deriveActionablePlan(report));

  // Collapsible states with localStorage persistence
  let collapsed = $state({
    topFindings: false,
    diet: false,
    supplements: false,
    labTests: false,
  });

  onMount(() => {
    try {
      const stored = localStorage.getItem('genomics_dashboard_collapsed');
      if (stored) {
        collapsed = { ...collapsed, ...JSON.parse(stored) };
      }
    } catch (e) {
      console.warn('Failed to load dashboard collapsed state:', e);
    }
  });

  function toggle(section: keyof typeof collapsed) {
    collapsed = { ...collapsed, [section]: !collapsed[section] };
    try {
      localStorage.setItem('genomics_dashboard_collapsed', JSON.stringify(collapsed));
    } catch (e) {
      console.warn('Failed to save dashboard collapsed state:', e);
    }
  }

  function getSeverityLabel(sc: SeverityClass): string {
    switch (sc) {
      case 'high_risk': return 'High Risk';
      case 'moderate_risk': return 'Moderate Risk';
      case 'low_risk': return 'Risk (Preliminary)';
      case 'confirmation_required': return 'Verify Lab';
      default: return sc;
    }
  }

  function getUrgencyBadge(urgency: LabTest['urgency']): string {
    switch (urgency) {
      case 'urgent': return 'badge-urgent';
      case 'consider': return 'badge-consider';
      case 'routine': return 'badge-routine';
    }
  }
</script>

<div class="dashboard-v2">
  <div class="disclaimer-banner">
    <span class="warning-icon">⚠️</span>
    <p>
      <strong>Educational Information Only:</strong> This dashboard evaluates genetic risk factors based on raw genotype calls and local research registries. It is not medical advice, diagnosis, or a treatment plan. Always review these markers and any suggested testing with a qualified healthcare provider.
    </p>
  </div>

  <div class="grid-layout">
    <!-- Panel 1: Top Concerns / Findings -->
    {#if plan.topFindings.length > 0}
      <div class="summary-card card" class:collapsed={collapsed.topFindings}>
        <div class="card-header" onclick={() => toggle('topFindings')} role="button" tabindex="0" onkeydown={e => e.key === 'Enter' && toggle('topFindings')}>
          <h3>⭐ Key Areas of Concern ({plan.topFindings.length})</h3>
          <span class="chevron">{collapsed.topFindings ? '▶' : '▼'}</span>
        </div>
        {#if !collapsed.topFindings}
          <div class="card-body">
            <div class="findings-list">
              {#each plan.topFindings as f}
                <div class="finding-item">
                  <div class="finding-meta">
                    <span class="gene-badge">{f.gene}</span>
                    <span class="rsid">{f.rsid}</span>
                    <span class="severity-badge {f.severity_class}">{getSeverityLabel(f.severity_class)}</span>
                  </div>
                  <p class="finding-desc">{f.interpretation}</p>
                  {#if onJumpToMarker}
                    <button class="btn btn-xs btn-link jump-btn" onclick={() => onJumpToMarker?.(f.link_id)}>
                      🔍 View Details (in {f.section_name})
                    </button>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Panel 2: Dietary Guidance -->
    {#if plan.diet.favor.length > 0 || plan.diet.avoid.length > 0}
      <div class="summary-card card" class:collapsed={collapsed.diet}>
        <div class="card-header" onclick={() => toggle('diet')} role="button" tabindex="0" onkeydown={e => e.key === 'Enter' && toggle('diet')}>
          <h3>🥗 Dietary Alignment</h3>
          <span class="chevron">{collapsed.diet ? '▶' : '▼'}</span>
        </div>
        {#if !collapsed.diet}
          <div class="card-body">
            <div class="diet-section">
              {#if plan.diet.favor.length > 0}
                <div class="diet-column favor">
                  <h4>👍 Lean Into / Favor</h4>
                  <ul>
                    {#each plan.diet.favor as item}
                      <li>{item}</li>
                    {/each}
                  </ul>
                </div>
              {/if}

              {#if plan.diet.avoid.length > 0}
                <div class="diet-column avoid">
                  <h4>👎 Limit / Avoid</h4>
                  <ul>
                    {#each plan.diet.avoid as item}
                      <li>{item}</li>
                    {/each}
                  </ul>
                </div>
              {/if}
            </div>
            {#if plan.diet.notes}
              <div class="diet-notes">
                <strong>Notes:</strong>
                <pre style="white-space: pre-wrap; font-family: inherit; font-size: 0.75rem; margin-top: 0.25rem; opacity: 0.95;">{plan.diet.notes}</pre>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Panel 3: Supplements to Discuss -->
    {#if plan.supplements.length > 0}
      <div class="summary-card card" class:collapsed={collapsed.supplements}>
        <div class="card-header" onclick={() => toggle('supplements')} role="button" tabindex="0" onkeydown={e => e.key === 'Enter' && toggle('supplements')}>
          <h3>💊 Supplements to Discuss</h3>
          <span class="chevron">{collapsed.supplements ? '▶' : '▼'}</span>
        </div>
        {#if !collapsed.supplements}
          <div class="card-body">
            <p class="section-hint">Consult your doctor before starting any supplementation, especially if taking medications.</p>
            <div class="supplements-list">
              {#each plan.supplements as s}
                <div class="supplement-item">
                  <span class="supp-name">{s.name}</span>
                  <span class="supp-reason">{s.reason}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Panel 4: Suggested Lab Testing -->
    {#if plan.labTests.length > 0}
      <div class="summary-card card" class:collapsed={collapsed.labTests}>
        <div class="card-header" onclick={() => toggle('labTests')} role="button" tabindex="0" onkeydown={e => e.key === 'Enter' && toggle('labTests')}>
          <h3>🔬 Suggested Lab Panels</h3>
          <span class="chevron">{collapsed.labTests ? '▶' : '▼'}</span>
        </div>
        {#if !collapsed.labTests}
          <div class="card-body">
            <p class="section-hint">Bring these suggestions to your physician to request clinical bloodwork or specialist consults.</p>
            <div class="lab-list">
              {#each plan.labTests as lt}
                <div class="lab-item" class:urgent-row={lt.urgency === 'urgent'}>
                  <div class="lab-header-row">
                    <span class="lab-name">{lt.name}</span>
                    <span class="lab-badge {getUrgencyBadge(lt.urgency)}">
                      {lt.urgency.toUpperCase()}
                    </span>
                    {#if lt.requires_counselor}
                      <span class="lab-counselor-badge">🧑‍⚕️ Counselor Advised</span>
                    {/if}
                  </div>
                  <span class="lab-reason">{lt.reason}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .dashboard-v2 {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1.5rem;
    width: 100%;
  }

  .disclaimer-banner {
    display: flex;
    gap: 0.75rem;
    background: rgba(245, 158, 11, 0.08);
    border: 1px solid rgba(245, 158, 11, 0.25);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    font-size: 0.75rem;
    line-height: 1.4;
    color: #f59e0b;
  }
  .disclaimer-banner p {
    margin: 0;
  }
  .warning-icon {
    font-size: 1.1rem;
  }

  .grid-layout {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
    gap: 1rem;
  }

  .summary-card {
    background: rgba(30, 41, 59, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    overflow: hidden;
    height: fit-content;
    transition: all 0.2s ease;
  }
  .summary-card:hover {
    border-color: rgba(255, 255, 255, 0.12);
    box-shadow: 0 4px 12px rgba(0,0,0,0.15);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background: rgba(255, 255, 255, 0.02);
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    cursor: pointer;
    user-select: none;
  }
  .card-header h3 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: #f1f5f9;
  }
  .chevron {
    font-size: 0.75rem;
    opacity: 0.6;
  }

  .card-body {
    padding: 1rem;
  }

  .section-hint {
    margin: 0 0 0.75rem 0;
    font-size: 0.7rem;
    opacity: 0.65;
    font-style: italic;
  }

  /* Key Areas of Concern list */
  .findings-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .finding-item {
    background: rgba(255, 255, 255, 0.02);
    border-left: 3px solid rgba(255, 255, 255, 0.1);
    padding: 0.5rem 0.75rem;
    border-radius: 0 4px 4px 0;
  }
  .finding-meta {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin-bottom: 0.25rem;
  }
  .gene-badge {
    background: rgba(99, 102, 241, 0.15);
    color: #818cf8;
    font-size: 0.68rem;
    font-weight: 700;
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
  }
  .rsid {
    font-size: 0.7rem;
    opacity: 0.75;
    font-family: monospace;
  }
  .severity-badge {
    font-size: 0.6rem;
    font-weight: 700;
    padding: 0.08rem 0.3rem;
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .severity-badge.high_risk {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
  }
  .severity-badge.confirmation_required {
    background: rgba(236, 72, 153, 0.15);
    color: #ec4899;
  }
  .severity-badge.moderate_risk {
    background: rgba(245, 158, 11, 0.15);
    color: #f59e0b;
  }
  .severity-badge.low_risk {
    background: rgba(96, 165, 250, 0.15);
    color: #60a5fa;
  }
  .finding-desc {
    margin: 0;
    font-size: 0.72rem;
    line-height: 1.35;
    opacity: 0.9;
  }
  .jump-btn {
    margin-top: 0.3rem;
    padding: 0;
    font-size: 0.65rem;
    opacity: 0.7;
    background: none;
    border: none;
    color: #60a5fa;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
  }
  .jump-btn:hover {
    opacity: 1;
    text-decoration: underline;
  }

  /* Dietary Alignment */
  .diet-section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .diet-column h4 {
    margin: 0 0 0.4rem 0;
    font-size: 0.75rem;
    font-weight: bold;
  }
  .diet-column.favor h4 { color: #4ade80; }
  .diet-column.avoid h4 { color: #f87171; }
  .diet-column ul {
    margin: 0;
    padding-left: 1.1rem;
    font-size: 0.72rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .diet-notes {
    margin-top: 0.75rem;
    padding-top: 0.5rem;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.7rem;
    opacity: 0.8;
  }

  /* Supplements List */
  .supplements-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .supplement-item {
    display: flex;
    flex-direction: column;
    background: rgba(255, 255, 255, 0.015);
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    border-left: 2px solid #a855f7;
  }
  .supp-name {
    font-size: 0.75rem;
    font-weight: bold;
    color: #c084fc;
  }
  .supp-reason {
    font-size: 0.68rem;
    opacity: 0.75;
    margin-top: 0.1rem;
  }

  /* Lab Tests list */
  .lab-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .lab-item {
    display: flex;
    flex-direction: column;
    background: rgba(255, 255, 255, 0.015);
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    border-left: 2px solid rgba(255, 255, 255, 0.1);
  }
  .lab-item.urgent-row {
    border-left-color: #ef4444;
  }
  .lab-header-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .lab-name {
    font-size: 0.75rem;
    font-weight: bold;
    color: #e2e8f0;
  }
  .lab-badge {
    font-size: 0.58rem;
    font-weight: 800;
    padding: 0.05rem 0.25rem;
    border-radius: 2px;
  }
  .lab-badge.badge-urgent {
    background: rgba(239, 68, 68, 0.18);
    color: #f87171;
  }
  .lab-badge.badge-consider {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
  }
  .lab-badge.badge-routine {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
  }
  .lab-counselor-badge {
    background: rgba(236, 72, 153, 0.15);
    color: #f472b6;
    font-size: 0.58rem;
    font-weight: 700;
    padding: 0.05rem 0.25rem;
    border-radius: 2px;
  }
  .lab-reason {
    font-size: 0.68rem;
    opacity: 0.75;
    margin-top: 0.1rem;
  }
</style>
