<!-- ./src/lib/components/report/DashboardSummaryPanel.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import type { GeneratedReport, SeverityClass } from '../../types/genomics';
  import { deriveActionablePlan, type ActionablePlan, type LabTest } from '../../utils/actionabilityEngine';
  import cycleSupport from '../../marker-packs/cycle_support_guidance.json';
  import { saveReproductiveContext, selectedReproductiveContextOption } from '../../utils/reproductiveContext';
  import ReproductiveContextEditor from '../ai/ReproductiveContextEditor.svelte';
  import CycleDiaryEditor from '../ai/CycleDiaryEditor.svelte';
  import { EMPTY_PERSONAL_SAFETY_CONTEXT, type PersonalSafetyContext } from '../../utils/personalSafetyContext';
  import { populatedReproductiveIntake } from '../../utils/reproductiveIntake';

  interface Props {
    report: GeneratedReport;
    sampleId?: number;
    onJumpToMarker?: (linkId: string) => void;
    reproductiveContext?: string;
    personalSafetyContext?: PersonalSafetyContext;
  }

  let {
    report,
    sampleId,
    onJumpToMarker,
    reproductiveContext = $bindable(''),
    personalSafetyContext = $bindable({ ...EMPTY_PERSONAL_SAFETY_CONTEXT }),
  }: Props = $props();

  let plan = $derived<ActionablePlan>(deriveActionablePlan(report, { reproductiveContext, personalSafetyContext }));

  // Collapsible states with localStorage persistence
  let collapsed = $state({
    topFindings: false,
    diet: false,
    cycleSupport: false,
    supplements: false,
    activity: false,
    medication: true,
    labTests: true,
  });

  let expandedLabReasons = $state<Record<string, boolean>>({});

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

  function setReproductiveContext(event: Event) {
    reproductiveContext = (event.currentTarget as HTMLSelectElement).value;
    saveReproductiveContext(sampleId, reproductiveContext);
  }

  function selectedReproductiveContextLabel(): string {
    return selectedReproductiveContextOption(reproductiveContext)?.label || 'the selected context';
  }

  function getSeverityLabel(sc: SeverityClass): string {
    switch (sc) {
      case 'high_risk': return 'Stronger association';
      case 'moderate_risk': return 'Possible association';
      case 'low_risk': return 'Preliminary association';
      case 'confirmation_required': return 'Confirm clinically';
      default: return sc;
    }
  }

  function getTierBadgeClass(tier: LabTest['tier']): string {
    switch (tier) {
      case 'counselor': return 'badge-counselor';
      case 'discuss': return 'badge-discuss';
      case 'optional': return 'badge-optional';
    }
  }

  function getTierChipLabel(tier: LabTest['tier']): string {
    switch (tier) {
      case 'counselor': return 'Counselor';
      case 'discuss': return 'Discuss';
      case 'optional': return 'Optional';
    }
  }

  function labKey(test: LabTest): string {
    return `${test.tier}:${test.name}`;
  }

  function toggleLabReason(test: LabTest) {
    const key = labKey(test);
    expandedLabReasons = { ...expandedLabReasons, [key]: !expandedLabReasons[key] };
  }

  function labsByCategory(tests: LabTest[]): { category: string; tests: LabTest[] }[] {
    const map = new Map<string, LabTest[]>();
    for (const test of tests) {
      const list = map.get(test.category) || [];
      list.push(test);
      map.set(test.category, list);
    }
    return Array.from(map.entries())
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([category, categoryTests]) => ({ category, tests: categoryTests }));
  }
</script>

<div class="dashboard-v2">
  <div class="disclaimer-banner">
    <span class="warning-icon">⚠️</span>
    <p>
      <strong>Educational Information Only:</strong> This dashboard summarizes curated genetic associations from your raw genotype calls and local research registries. It is not medical advice, diagnosis, or a treatment plan. Always review these markers and any suggested testing with a qualified healthcare provider.
    </p>
  </div>

  {#if plan.safetyNotes.length > 0}
    <div class="actionability-safety" role="note">
      <strong>🧭 How to use actionability guidance</strong>
      <ul>
        {#each plan.safetyNotes as note (note)}
          <li>{note}</li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="context-selector summary-card card" role="region" aria-labelledby="reproductive-context-label">
    <div class="context-selector-copy">
      <strong id="reproductive-context-label">Optional reproductive &amp; hormone context</strong>
      <span>Choose only when relevant to the person. This selection is self-reported, stored per DNA profile, and is never inferred from genotype, chromosome calls, gender, anatomy, fertility, pregnancy, or hormone status.</span>
    </div>
    <select aria-labelledby="reproductive-context-label" value={reproductiveContext} onchange={setReproductiveContext}>
      <option value="">Not specified — keep context-specific guidance hidden</option>
      {#each cycleSupport.context_options.filter((option) => option.id !== 'none_or_unknown') as option (option.id)}
        <option value={option.id}>{option.label}</option>
      {/each}
    </select>
  </div>

  <ReproductiveContextEditor bind:personalSafetyContext sampleId={sampleId} reproductiveContext={reproductiveContext} />
  <CycleDiaryEditor bind:personalSafetyContext sampleId={sampleId} reproductiveContext={reproductiveContext} />

  {#if plan.personalContext.priorityNotes.length > 0}
    <div class="personal-context-card summary-card card" role="region" aria-labelledby="personal-context-label">
      <div class="context-selector-copy">
        <strong id="personal-context-label">🧾 Personal safety context applied</strong>
        <span>This information is self-reported for this DNA profile. It is not genetic evidence, and it is kept separate from the genotype interpretation.</span>
      </div>
      <ul class="guardrail-list personal-context-notes">
        {#each plan.personalContext.priorityNotes as note (note)}<li>{note}</li>{/each}
      </ul>
      <div class="personal-context-grid">
        {#if plan.personalContext.medications.length > 0}
          <div><strong>Medications</strong><span>{plan.personalContext.medications.join(' · ')}</span></div>
        {/if}
        {#if plan.personalContext.supplements.length > 0}
          <div><strong>Supplements</strong><span>{plan.personalContext.supplements.join(' · ')}</span></div>
        {/if}
        {#if plan.personalContext.allergies.length > 0}
          <div><strong>Allergies / intolerances</strong><span>{plan.personalContext.allergies.join(' · ')}</span></div>
        {/if}
        {#if plan.personalContext.symptoms.length > 0}
          <div><strong>Symptoms / timing</strong><span>{plan.personalContext.symptoms.join(' · ')}</span></div>
        {/if}
        {#if plan.personalContext.labObservations.length > 0}
          <div><strong>Recent labs / findings</strong><span>{plan.personalContext.labObservations.join(' · ')}</span></div>
        {/if}
        {#each populatedReproductiveIntake(plan.personalContext.reproductiveIntake) as item (item.field.id)}
          <div><strong>{item.field.label}</strong><span>{item.value}</span></div>
        {/each}
        {#if plan.personalContext.cycleDiary && plan.personalContext.cycleDiary.length > 0}
          <div><strong>Daily cycle diary</strong><span>{plan.personalContext.cycleDiary.length} self-reported observation{plan.personalContext.cycleDiary.length === 1 ? '' : 's'}</span></div>
        {/if}
      </div>
    </div>
  {/if}

  <div class="grid-layout">
    <!-- Panel 1: Top Concerns — full width, compact 2-col findings -->
    {#if plan.topFindings.length > 0}
      <div class="summary-card card card-top-findings" class:collapsed={collapsed.topFindings}>
        <div class="card-header" onclick={() => toggle('topFindings')} role="button" tabindex="0" onkeydown={e => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), toggle('topFindings'))}>
          <h3>⭐ Top {plan.topFindings.length} Areas of Concern</h3>
          <span class="chevron">{collapsed.topFindings ? '▶' : '▼'}</span>
        </div>
        {#if !collapsed.topFindings}
          <div class="card-body">
            <div class="findings-list">
              {#each plan.topFindings as f (f.link_id || `${f.rsid}:${f.gene}`)}
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

    <div class="action-row">
      <!-- Panel 2: Dietary Guidance -->
      {#if plan.diet.favor.length > 0 || plan.diet.avoid.length > 0}
        <div class="summary-card card" class:collapsed={collapsed.diet}>
          <div class="card-header" onclick={() => toggle('diet')} role="button" tabindex="0" onkeydown={e => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), toggle('diet'))}>
            <h3>🥗 Dietary Alignment</h3>
            <span class="chevron">{collapsed.diet ? '▶' : '▼'}</span>
          </div>
          {#if !collapsed.diet}
            <div class="card-body">
              <p class="section-hint">These are conditional discussion or short-trial prompts. A genotype match is not a permanent food restriction; use symptoms, labs, allergies, medications, and clinician guidance first.</p>
              <div class="diet-section">
                {#if plan.diet.favor.length > 0}
                  <div class="diet-column favor">
                    <h4>👍 Lean Into / Favor</h4>
                    <ul>
                      {#each plan.diet.favor as item (item)}
                        <li>{item}</li>
                      {/each}
                    </ul>
                  </div>
                {/if}

                {#if plan.diet.avoid.length > 0}
                  <div class="diet-column avoid">
                    <h4>👎 Limit / Avoid</h4>
                    <ul>
                      {#each plan.diet.avoid as item (item)}
                        <li>{item}</li>
                      {/each}
                    </ul>
                  </div>
                {/if}
              </div>
              {#if plan.diet.notes}
                <div class="diet-notes">
                  <strong>Notes:</strong>
                  <pre class="diet-notes-pre">{plan.diet.notes}</pre>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Panel 3: Supplements to Discuss -->
      {#if plan.supplements.length > 0 || plan.supplementSafety.relevantRules.length > 0}
        <div class="summary-card card" class:collapsed={collapsed.supplements}>
          <div class="card-header" onclick={() => toggle('supplements')} role="button" tabindex="0" onkeydown={e => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), toggle('supplements'))}>
            <h3>💊 Supplements to Discuss</h3>
            <span class="chevron">{collapsed.supplements ? '▶' : '▼'}</span>
          </div>
          {#if !collapsed.supplements}
            <div class="card-body">
              <p class="section-hint">Every supplement item is a discussion prompt, not a prescription. Check medications, pregnancy/lactation status, kidney/liver health, and labs before starting anything.</p>
              {#if plan.supplements.length > 0}
                <div class="supplements-list">
                  {#each plan.supplements as s (`${s.name}:${s.reason}`)}
                    <div class="supplement-item">
                      <span class="supp-name">{s.name}</span>
                      <span class="supp-reason">{s.reason}</span>
                    </div>
                  {/each}
                </div>
              {/if}
              {#if plan.supplementSafety.relevantRules.length > 0}
                <div class="supplement-safety">
                  <strong>Safety checks before any supplement</strong>
                  <ul class="guardrail-list">
                    {#each plan.supplementSafety.principles as principle (principle)}<li>{principle}</li>{/each}
                  </ul>
                  {#each plan.supplementSafety.relevantRules as rule (rule.id)}
                    <div class="supplement-safety-rule">
                      <strong>{rule.label}</strong>
                      {#if rule.avoid?.length}
                        <ul class="guardrail-list">
                          {#each rule.avoid as item (item)}<li>{item}</li>{/each}
                        </ul>
                      {/if}
                      {#if rule.confirm_with?.length}
                        <span class="supp-reason">Confirm with: {rule.confirm_with.join('; ')}</span>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Cycle/reproductive support is a phenotype and safety layer, not a diagnosis. -->
      {#if plan.cycleSupport.relevantDomains.length > 0}
        <div class="summary-card card card-cycle-support" class:collapsed={collapsed.cycleSupport}>
          <div class="card-header" onclick={() => toggle('cycleSupport')} role="button" tabindex="0" onkeydown={e => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), toggle('cycleSupport'))}>
            <h3>⚕️ Reproductive &amp; Hormone Support</h3>
            <span class="chevron">{collapsed.cycleSupport ? '▶' : '▼'}</span>
          </div>
          {#if !collapsed.cycleSupport}
            <div class="card-body">
              <p class="section-hint">Showing guidance for {selectedReproductiveContextLabel()}. This section organizes timing, medication, symptom, and clinical follow-up questions. DNA cannot measure current hormones or diagnose a condition or medication response.</p>
              <ul class="guardrail-list">
                {#each plan.cycleSupport.principles as principle (principle)}<li>{principle}</li>{/each}
              </ul>
              {#each plan.cycleSupport.relevantDomains as domain (domain.id)}
                <div class="cycle-support-domain">
                  <strong>{domain.title}</strong>
                  <p>{domain.context}</p>
                  <div class="activity-columns">
                    <div>
                      <h4>Ask / record</h4>
                      <ul class="guardrail-list">
                        {#each domain.questions as item (item)}<li>{item}</li>{/each}
                      </ul>
                    </div>
                    <div>
                      <h4>Support / confirm</h4>
                      <ul class="guardrail-list">
                        {#each domain.support_options as item (item)}<li>{item}</li>{/each}
                        {#each domain.confirm_with as item (item)}<li>Confirm with: {item}</li>{/each}
                      </ul>
                    </div>
                  </div>
                  {#if domain.red_flags?.length}
                    <p class="activity-stop-list"><strong>Escalate promptly for:</strong> {domain.red_flags.join('; ')}</p>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    {#if plan.activity.relevantDomains.length > 0}
      <div class="summary-card card" class:collapsed={collapsed.activity}>
        <div class="card-header" onclick={() => toggle('activity')} role="button" tabindex="0" onkeydown={e => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), toggle('activity'))}>
          <h3>🏃 Activity &amp; Recovery Guardrails</h3>
          <span class="chevron">{collapsed.activity ? '▶' : '▼'}</span>
        </div>
        {#if !collapsed.activity}
          <div class="card-body">
            <p class="section-hint">Genotype may provide weak context for training questions. It never clears high-intensity, contact, endurance, heat, altitude, or maximal-load activity.</p>
            <ul class="guardrail-list">
              {#each plan.activity.principles as principle (principle)}
                <li>{principle}</li>
              {/each}
            </ul>
            {#each plan.activity.relevantDomains as domain (domain.id)}
              <div class="activity-domain">
                <strong>{domain.id.replaceAll('_', ' ')}</strong>
                <span class="activity-context">{domain.context}</span>
                <div class="activity-columns">
                  <div>
                    <h4>Favor</h4>
                    <ul class="guardrail-list">
                      {#each domain.favor as item (item)}<li>{item}</li>{/each}
                    </ul>
                  </div>
                  <div>
                    <h4>Avoid / confirm</h4>
                    <ul class="guardrail-list">
                      {#each domain.avoid as item (item)}<li>{item}</li>{/each}
                      {#each domain.confirm_with as item (item)}<li>{item}</li>{/each}
                    </ul>
                  </div>
                </div>
              </div>
            {/each}
            <div class="activity-stop-list">
              <strong>Stop activity and seek appropriate care for:</strong>
              <ul class="guardrail-list">
                {#each plan.activity.stopAndEscalate as item (item)}<li>{item}</li>{/each}
              </ul>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    {#if plan.medication.rules.length > 0}
      <div class="summary-card card" class:collapsed={collapsed.medication}>
        <div class="card-header" onclick={() => toggle('medication')} role="button" tabindex="0" onkeydown={e => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), toggle('medication'))}>
          <h3>💊 Medication Safety &amp; Context</h3>
          <span class="chevron">{collapsed.medication ? '▶' : '▼'}</span>
        </div>
        {#if !collapsed.medication}
          <div class="card-body">
            <p class="section-hint">Record medication context before interpreting a marker. Raw consumer DNA is not a complete clinical PGx result and is never a reason to change a medication.</p>
            <div class="medication-columns">
              <div>
                <h4>Ask for / record</h4>
                <ul class="guardrail-list">
                  {#each plan.medication.askFor as item (item)}<li>{item}</li>{/each}
                </ul>
              </div>
              <div>
                <h4>Do not do from raw DNA</h4>
                <ul class="guardrail-list">
                  {#each plan.medication.rules as item (item)}<li>{item}</li>{/each}
                </ul>
              </div>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Labs: full-width, grouped & compact (collapsed by default) -->
    {#if plan.labTests.length > 0}
      <div class="summary-card card card-lab-followups" class:collapsed={collapsed.labTests}>
        <div class="card-header" onclick={() => toggle('labTests')} role="button" tabindex="0" onkeydown={e => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), toggle('labTests'))}>
          <h3>🔬 Lab & screening follow-ups ({plan.labTests.length})</h3>
          <span class="chevron">{collapsed.labTests ? '▶' : '▼'}</span>
        </div>
        {#if !collapsed.labTests}
          <div class="card-body lab-body">
            <p class="section-hint">Grouped by priority — bring to a clinician; none of these imply an emergency workup unless you have acute symptoms.</p>
            <div class="lab-tier-stack">
              {#each plan.labGroups as group (group.label)}
                <section class="lab-tier-block">
                  <div class="lab-tier-header">
                    <span class="lab-tier-title">{group.label}</span>
                    <span class="lab-tier-count">{group.tests.length}</span>
                  </div>
                  <p class="lab-tier-hint">{group.hint}</p>
                  {#each labsByCategory(group.tests) as { category, tests } (category)}
                    <div class="lab-category">
                      <div class="lab-category-label">{category}</div>
                      <div class="lab-chip-grid">
                        {#each tests as lt (labKey(lt))}
                          <div class="lab-chip" class:lab-chip-counselor={lt.tier === 'counselor'}>
                            <button
                              type="button"
                              class="lab-chip-main"
                              onclick={() => toggleLabReason(lt)}
                              aria-expanded={expandedLabReasons[labKey(lt)] ? 'true' : 'false'}
                            >
                              <span class="lab-chip-name">{lt.name}</span>
                              <span class="lab-chip-badge {getTierBadgeClass(lt.tier)}">{getTierChipLabel(lt.tier)}</span>
                              {#if lt.requires_counselor}
                                <span class="lab-chip-counselor" title="Genetic counselor advised">🧑‍⚕️</span>
                              {/if}
                            </button>
                            {#if expandedLabReasons[labKey(lt)]}
                              <p class="lab-chip-reason">{lt.reason}</p>
                            {/if}
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/each}
                </section>
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
  .actionability-safety {
    border: 1px solid rgba(96, 165, 250, 0.28);
    background: rgba(59, 130, 246, 0.08);
    color: var(--text-secondary);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    font-size: 0.75rem;
    line-height: 1.45;
  }
  .actionability-safety strong {
    color: #93c5fd;
  }
  .actionability-safety ul {
    margin: 0.35rem 0 0;
    padding-left: 1.2rem;
  }
  .warning-icon {
    font-size: 1.1rem;
  }

  .context-selector {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border: 1px solid rgba(167, 139, 250, 0.28);
    background: rgba(139, 92, 246, 0.08);
  }

  .context-selector-copy {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
    font-size: 0.72rem;
    line-height: 1.4;
  }

  .context-selector-copy strong {
    color: #ddd6fe;
  }

  .context-selector-copy span {
    color: var(--text-secondary);
  }

  .personal-context-card {
    gap: 0.65rem;
    border-color: rgba(52, 211, 153, 0.28);
    background: rgba(16, 185, 129, 0.06);
  }

  .personal-context-notes {
    margin: 0;
  }

  .personal-context-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0.6rem 1rem;
    font-size: 0.72rem;
  }

  .personal-context-grid div {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
  }

  .personal-context-grid strong {
    color: #a7f3d0;
  }

  .personal-context-grid span {
    color: var(--text-secondary);
    overflow-wrap: anywhere;
  }

  .context-selector select {
    min-width: min(320px, 42%);
    background: rgba(0, 0, 0, 0.28);
    border: 1px solid rgba(167, 139, 250, 0.35);
    border-radius: 5px;
    color: var(--text-primary);
    padding: 0.45rem 0.55rem;
    font: inherit;
  }

  .grid-layout {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    width: 100%;
  }

  .action-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
    align-items: start;
  }

  .card-lab-followups {
    width: 100%;
  }

  .card-cycle-support {
    grid-column: 1 / -1;
  }

  @media (max-width: 720px) {
    .context-selector {
      align-items: stretch;
      flex-direction: column;
    }

    .context-selector select {
      min-width: 0;
      width: 100%;
    }
  }

  .lab-body {
    padding-top: 0.65rem;
  }

  .lab-tier-stack {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .lab-tier-block {
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    padding: 0.55rem 0.65rem;
    background: rgba(0, 0, 0, 0.12);
  }

  .lab-tier-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .lab-tier-title {
    font-size: 0.74rem;
    font-weight: 700;
    color: #e2e8f0;
  }

  .lab-tier-count {
    font-size: 0.62rem;
    font-weight: 700;
    opacity: 0.65;
    padding: 0.05rem 0.35rem;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
  }

  .lab-tier-hint {
    margin: 0.2rem 0 0.45rem;
    font-size: 0.64rem;
    opacity: 0.62;
    line-height: 1.3;
  }

  .lab-category {
    margin-top: 0.35rem;
  }

  .lab-category-label {
    font-size: 0.62rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.55;
    margin-bottom: 0.25rem;
  }

  .lab-chip-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .lab-chip {
    min-width: 0;
    max-width: 100%;
  }

  .lab-chip-main {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    flex-wrap: wrap;
    width: 100%;
    text-align: left;
    padding: 0.28rem 0.45rem;
    border-radius: 5px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.03);
    color: inherit;
    cursor: pointer;
    font: inherit;
  }

  .lab-chip-main:hover {
    border-color: rgba(255, 255, 255, 0.16);
    background: rgba(255, 255, 255, 0.05);
  }

  .lab-chip-counselor .lab-chip-main {
    border-color: rgba(236, 72, 153, 0.25);
    background: rgba(236, 72, 153, 0.06);
  }

  .lab-chip-name {
    font-size: 0.68rem;
    font-weight: 600;
    color: #e2e8f0;
    line-height: 1.25;
  }

  .lab-chip-badge {
    font-size: 0.55rem;
    font-weight: 800;
    padding: 0.04rem 0.28rem;
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    white-space: nowrap;
  }

  .lab-chip-badge.badge-counselor {
    background: rgba(236, 72, 153, 0.18);
    color: #f9a8d4;
  }

  .lab-chip-badge.badge-discuss {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
  }

  .lab-chip-badge.badge-optional {
    background: rgba(96, 165, 250, 0.15);
    color: #93c5fd;
  }

  .lab-chip-counselor {
    font-size: 0.7rem;
    line-height: 1;
  }

  .lab-chip-reason {
    margin: 0.2rem 0 0 0.15rem;
    font-size: 0.62rem;
    opacity: 0.72;
    line-height: 1.3;
    max-width: 42rem;
  }

  .card-top-findings {
    width: 100%;
  }

  .summary-card {
    background: rgba(30, 41, 59, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    overflow: hidden;
    height: fit-content;
    transition: all 0.2s ease;
    min-width: 0;
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
    padding: 0.85rem 1rem;
  }

  .section-hint {
    margin: 0 0 0.75rem 0;
    font-size: 0.7rem;
    opacity: 0.65;
    font-style: italic;
  }

  .findings-list {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.55rem 0.85rem;
  }
  .finding-item {
    background: rgba(255, 255, 255, 0.02);
    border-left: 3px solid rgba(255, 255, 255, 0.1);
    padding: 0.4rem 0.65rem;
    border-radius: 0 4px 4px 0;
  }
  .finding-meta {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin-bottom: 0.2rem;
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
    line-height: 1.3;
    opacity: 0.9;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .jump-btn {
    margin-top: 0.2rem;
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

  .diet-section {
    display: grid;
    grid-template-columns: 1fr 1fr;
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
  .diet-notes-pre {
    white-space: pre-wrap;
    font-family: inherit;
    font-size: 0.75rem;
    margin-top: 0.25rem;
    opacity: 0.95;
  }

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
  .supplement-safety {
    margin-top: 0.85rem;
    padding-top: 0.65rem;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }
  .supplement-safety-rule {
    margin-top: 0.65rem;
    padding: 0.45rem 0.55rem;
    border-left: 2px solid #f59e0b;
    background: rgba(245, 158, 11, 0.04);
    border-radius: 4px;
    font-size: 0.72rem;
  }

  .guardrail-list {
    margin: 0.35rem 0 0;
    padding-left: 1.15rem;
    font-size: 0.72rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .activity-domain {
    margin-top: 0.85rem;
    padding: 0.55rem 0.65rem;
    border-left: 2px solid #38bdf8;
    background: rgba(56, 189, 248, 0.04);
    border-radius: 4px;
  }
  .cycle-support-domain {
    margin-top: 0.85rem;
    padding: 0.55rem 0.65rem;
    border-left: 2px solid #c084fc;
    background: rgba(192, 132, 252, 0.04);
    border-radius: 4px;
  }
  .activity-context {
    display: block;
    margin-top: 0.15rem;
    font-size: 0.68rem;
    opacity: 0.72;
  }
  .activity-columns,
  .medication-columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-top: 0.45rem;
  }
  .activity-columns h4,
  .medication-columns h4 {
    margin: 0;
    font-size: 0.72rem;
    color: #bae6fd;
  }
  .activity-stop-list {
    margin-top: 0.85rem;
    padding: 0.55rem 0.65rem;
    border: 1px solid rgba(248, 113, 113, 0.35);
    border-radius: 4px;
    background: rgba(248, 113, 113, 0.05);
  }

  @media (max-width: 1100px) {
    .action-row {
      grid-template-columns: 1fr;
    }
    .findings-list {
      grid-template-columns: 1fr;
    }
    .diet-section {
      grid-template-columns: 1fr;
    }
    .activity-columns,
    .medication-columns {
      grid-template-columns: 1fr;
    }
  }
</style>
