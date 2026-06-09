<!-- ./src/lib/components/agent/AgentReportView.svelte -->
<script lang="ts">
  import { formatMarkdown, parseThinking } from "../../utils/chatParser";

  interface Props {
    reportText: string;
    modelName: string;
    rsid: string;
    stats: {
      pubmedCount: number;
      trialsCount: number;
      drugsCount: number;
      clinvarMatch: boolean;
      localMatch: boolean;
    };
    validation: {
      medicalClaimingFree: boolean;
      disclaimerPresent: boolean;
      structureOk: boolean;
      auditComments: string;
      approved: boolean;
    } | null;
    onReset: () => void;
    onExport: () => void;
  }

  let { reportText, modelName, rsid, stats, validation, onReset, onExport }: Props = $props();

  let copied = $state(false);

  // Parse reasoning / thought block out of the report text if present
  let parsed = $derived(parseThinking(reportText));

  function copyToClipboard() {
    navigator.clipboard.writeText(parsed.response)
      .then(() => {
        copied = true;
        setTimeout(() => copied = false, 2000);
      });
  }
</script>

<div class="agent-report-view">
  <!-- Top Stat Cards -->
  <div class="stats-grid">
    <div class="stat-card">
      <span class="stat-icon">🧬</span>
      <div class="stat-info">
        <span class="stat-label">Variant Target</span>
        <span class="stat-val font-mono">{rsid}</span>
      </div>
    </div>
    <div class="stat-card">
      <span class="stat-icon">📚</span>
      <div class="stat-info">
        <span class="stat-label">PubMed Papers</span>
        <span class="stat-val font-mono">{stats.pubmedCount}</span>
      </div>
    </div>
    <div class="stat-card">
      <span class="stat-icon">🔬</span>
      <div class="stat-info">
        <span class="stat-label">Clinical Trials</span>
        <span class="stat-val font-mono">{stats.trialsCount}</span>
      </div>
    </div>
    <div class="stat-card">
      <span class="stat-icon">💊</span>
      <div class="stat-info">
        <span class="stat-label">ChEMBL Compounds</span>
        <span class="stat-val font-mono">{stats.drugsCount}</span>
      </div>
    </div>
  </div>

  <!-- Main Report Panel -->
  <div class="report-panel card">
    <div class="report-header">
      <h4>🕵️ Agent Analysis Report (Synthesized by {modelName})</h4>
      <div class="report-actions">
        <button class="btn btn-secondary btn-sm" onclick={copyToClipboard}>
          {copied ? "✓ Copied!" : "📋 Copy"}
        </button>
        <button class="btn btn-secondary btn-sm" onclick={onExport}>
          💾 Save Markdown
        </button>
        <button class="btn btn-primary btn-sm" onclick={onReset}>
          🔄 Research New Topic
        </button>
      </div>
    </div>

    <!-- Thinking monologue if present -->
    {#if parsed.thought}
      <details class="thought-details-block" open={true}>
        <summary class="thought-summary">
          <span>🧠 Agent Reasoning Stream</span>
        </summary>
        <div class="thought-body font-mono">
          {parsed.thought}
        </div>
      </details>
    {/if}

    <div class="report-body message-body">
      {@html formatMarkdown(parsed.response)}
    </div>
  </div>

  <!-- Quality Assurance Validation Audit Panel -->
  {#if validation}
    <div class="audit-panel card" class:approved={validation.approved}>
      <div class="audit-header">
        <span>🛡️ Personal QA Quality Audit Log</span>
        <span class="audit-badge" class:badge-success={validation.approved} class:badge-warning={!validation.approved}>
          {validation.approved ? "APPROVED & VERIFIED" : "WARNING - REVISIONS RECOMMENDED"}
        </span>
      </div>
      <div class="audit-checklist">
        <div class="checklist-item">
          <span class="check-icon">{validation.medicalClaimingFree ? "✅" : "❌"}</span>
          <span>No Unreasonable Medical/Diagnosing Claims Detected</span>
        </div>
        <div class="checklist-item">
          <span class="check-icon">{validation.disclaimerPresent ? "✅" : "❌"}</span>
          <span>Supportive Medical Disclaimer Included</span>
        </div>
        <div class="checklist-item">
          <span class="check-icon">{validation.structureOk ? "✅" : "❌"}</span>
          <span>Genomic Markdown Report Structure Validated</span>
        </div>
      </div>
      <div class="audit-comments font-mono">
        <strong>Remarks:</strong> {validation.auditComments}
      </div>
    </div>
  {/if}
</div>

<style>
  .agent-report-view {
    display: flex;
    flex-direction: column;
    gap: 20px;
    max-width: 900px;
    margin: 0 auto;
    padding-bottom: 40px;
  }
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 12px;
  }
  .stat-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .stat-icon {
    font-size: 1.5rem;
  }
  .stat-info {
    display: flex;
    flex-direction: column;
  }
  .stat-label {
    font-size: 0.68rem;
    color: var(--text-secondary);
  }
  .stat-val {
    font-size: 0.9rem;
    font-weight: 700;
    color: var(--text-primary);
  }
  .report-panel {
    background: rgba(255, 255, 255, 0.015);
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .report-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-color);
    padding-bottom: 12px;
  }
  .report-actions {
    display: flex;
    gap: 8px;
  }
  .btn-sm {
    padding: 4px 10px;
    font-size: 0.72rem;
  }
  .report-body {
    max-height: 500px;
    overflow-y: auto;
    padding: 4px 8px;
    border-radius: 6px;
    line-height: 1.6;
    color: #e5e7eb;
  }

  /* QA Audit Box Styles */
  .audit-panel {
    border: 1px solid rgba(239, 68, 68, 0.25);
    background: rgba(239, 68, 68, 0.02);
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: all 0.3s;
  }
  .audit-panel.approved {
    border-color: rgba(16, 185, 129, 0.25);
    background: rgba(16, 185, 129, 0.02);
  }
  .audit-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.76rem;
    font-weight: 700;
    color: var(--text-secondary);
  }
  .audit-badge {
    font-size: 0.65rem;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .badge-success {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }
  .badge-warning {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }
  .audit-checklist {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .checklist-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.75rem;
  }
  .check-icon {
    font-size: 0.85rem;
  }
  .audit-comments {
    padding: 8px 12px;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 4px;
    font-size: 0.72rem;
    color: var(--text-secondary);
    border-left: 2px solid var(--accent);
  }
  .font-mono { font-family: monospace; }
</style>
