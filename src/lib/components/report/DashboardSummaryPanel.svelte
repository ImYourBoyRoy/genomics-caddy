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
  import { getCompactSimpleMeaning, getLaypersonTranslation } from '../../utils/layperson';
  import Tooltip from '../common/Tooltip.svelte';

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
    topFindings: true,
    diet: true,
    cycleSupport: true,
    supplements: true,
    activity: true,
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

  function markerForFinding(finding: ActionablePlan['topFindings'][number]) {
    for (const section of report.sections || []) {
      const marker = section.markers.find((candidate) => candidate.link_id === finding.link_id);
      if (marker) return marker;
    }
    return undefined;
  }

  function findingMeaning(finding: ActionablePlan['topFindings'][number]): string {
    const marker = markerForFinding(finding);
    return marker
      ? getCompactSimpleMeaning(getLaypersonTranslation(marker))
      : 'This is a research association that may be useful to discuss in the right personal context.';
  }

  function findingTitle(finding: ActionablePlan['topFindings'][number]): string {
    const marker = markerForFinding(finding);
    return marker
      ? getLaypersonTranslation(marker).simpleImpact
      : 'Research finding';
  }

  function findingNextStep(finding: ActionablePlan['topFindings'][number]): string {
    if (finding.severity_class === 'confirmation_required' || finding.severity_class === 'high_risk') {
      return 'Consider clinical confirmation.';
    }
    if (finding.severity_class === 'moderate_risk' || finding.severity_class === 'low_risk') {
      return 'Compare with symptoms, history, and relevant labs.';
    }
    return 'Review the detailed finding for personal relevance.';
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
  <section class="action-queue summary-card card" aria-labelledby="action-queue-title">
    <div class="action-queue-header">
      <div>
        <span class="section-kicker">Start here</span>
        <h3 id="action-queue-title">Your next steps</h3>
        <p>Up to five prioritized follow-up prompts from this report.</p>
      </div>
      <span class="action-queue-count">{Math.min(plan.topFindings.length, 5)} shown</span>
    </div>

    {#if plan.topFindings.length > 0}
      <div class="action-queue-list">
        {#each plan.topFindings.slice(0, 5) as finding (finding.link_id || `${finding.rsid}:${finding.gene}`)}
          <article class="action-queue-item">
            <div class="action-queue-item-top">
              <div>
                <h4>{findingTitle(finding)}</h4>
                <span class="action-queue-context">{finding.section_name}</span>
              </div>
              <span class="severity-badge {finding.severity_class}">{getSeverityLabel(finding.severity_class)}</span>
            </div>
            <p>{findingMeaning(finding)}</p>
            <div class="action-queue-next"><strong>Next helpful step:</strong> {findingNextStep(finding)}</div>
            {#if onJumpToMarker}
              <button class="btn btn-xs btn-link jump-btn" type="button" onclick={() => onJumpToMarker?.(finding.link_id)}>
                View finding details →
              </button>
            {/if}
          </article>
        {/each}
      </div>
    {:else}
      <div class="action-queue-empty">
        <strong>No immediate genetic follow-up prompts were generated.</strong>
        <span>Use routine care and selected personal context to guide follow-up.</span>
      </div>
    {/if}
  </section>

  {#if plan.safetyNotes.length > 0}
    <details class="actionability-safety">
      <summary>🧭 How to use suggestions</summary>
      <ul>
        {#each plan.safetyNotes as note (note)}
          <li>{note}</li>
        {/each}
      </ul>
    </details>
  {/if}

  <div class="context-selector summary-card card" role="region" aria-labelledby="reproductive-context-label">
    <div class="context-selector-copy">
      <div class="context-selector-heading">
        <strong id="reproductive-context-label">Optional reproductive &amp; hormone context</strong>
        <Tooltip
          label="About context selection"
          description="This is self-reported context stored for this DNA profile. It is not inferred from genotype or chromosome calls and does not establish gender, anatomy, fertility, pregnancy, or hormone status."
        >
          <span class="context-info-icon" aria-hidden="true">ⓘ</span>
        </Tooltip>
      </div>
      <span>Optional self-reported context used to tailor guidance.</span>
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
    <div class="action-row">
      <!-- Panel 2: Dietary Guidance -->
      {#if plan.diet.favor.length > 0 || plan.diet.avoid.length > 0 || plan.foodSafety.explicitExclusions.length > 0 || plan.foodSafety.confirmedAllergies.length > 0 || plan.foodSafety.suspectedAllergies.length > 0 || plan.foodSafety.relevantRules.length > 0 || plan.foodSafety.suppressedSuggestions.length > 0}
        <div class="summary-card card" class:collapsed={collapsed.diet}>
          <button type="button" class="card-header" onclick={() => toggle('diet')} aria-expanded={!collapsed.diet} aria-controls="dietary-alignment-body">
            <span class="card-header-title" role="heading" aria-level="3">🥗 Dietary Alignment</span>
            <span class="chevron">{collapsed.diet ? '▶' : '▼'}</span>
          </button>
          {#if !collapsed.diet}
            <div class="card-body" id="dietary-alignment-body">
              <p class="section-hint">Conditional prompts, not permanent food rules.</p>
              {#if plan.foodSafety.explicitExclusions.length > 0 || plan.foodSafety.confirmedAllergies.length > 0 || plan.foodSafety.suspectedAllergies.length > 0}
                <div class="dietary-profile-safety" role="note">
                  <strong>🛡️ Explicit food context</strong>
                  <p>These entries came from the profile, not the DNA. They take priority over genetic optimization.</p>
                  {#if plan.foodSafety.explicitExclusions.length > 0}
                    <div class="dietary-profile-list">
                      <strong>Foods / ingredients to exclude</strong>
                      <span>{plan.foodSafety.explicitExclusions.join(' · ')}</span>
                    </div>
                  {/if}
                  {#if plan.foodSafety.confirmedAllergies.length > 0}
                    <div class="dietary-profile-list dietary-profile-allergy">
                      <strong>Confirmed food allergies — keep excluded</strong>
                      <span>{plan.foodSafety.confirmedAllergies.join(' · ')}</span>
                    </div>
                  {/if}
                  {#if plan.foodSafety.suspectedAllergies.length > 0}
                    <div class="dietary-profile-list">
                      <strong>Suspected reactions — not confirmed allergies</strong>
                      <span>{plan.foodSafety.suspectedAllergies.join(' · ')}</span>
                    </div>
                  {/if}
                  <ul class="guardrail-list">
                    {#each plan.foodSafety.notes as note (note)}<li>{note}</li>{/each}
                  </ul>
                </div>
              {/if}
              {#if plan.foodSafety.relevantRules.length > 0}
                <div class="dietary-resource-rules">
                  <strong>Resource-backed food checks</strong>
                  {#each plan.foodSafety.relevantRules as rule (rule.id)}
                    <div class="dietary-resource-rule">
                      <strong>{rule.label}</strong>
                      <p>{rule.recommendation}</p>
                      {#if rule.substitutions?.length}
                        <span><strong>Possible substitutions:</strong> {rule.substitutions.join('; ')}</span>
                      {/if}
                      {#if rule.confirm_with?.length}
                        <span><strong>Confirm with:</strong> {rule.confirm_with.join('; ')}</span>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
              {#if plan.foodSafety.suppressedSuggestions.length > 0}
                <div class="dietary-suppressed-suggestions" role="note">
                  <strong>🛡️ Suggestions withheld by explicit food constraints</strong>
                  <p>These genotype- or nutrient-linked food suggestions were removed from the “Lean Into / Favor” list because they conflict with an explicitly entered food exclusion or reaction.</p>
                  <ul>
                    {#each plan.foodSafety.suppressedSuggestions as item (item)}<li>{item}</li>{/each}
                  </ul>
                  {#if plan.foodSafety.conflictNotes.length > 0}
                    <ul class="guardrail-list">
                      {#each plan.foodSafety.conflictNotes as note (note)}<li>{note}</li>{/each}
                    </ul>
                  {/if}
                </div>
              {/if}
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
          <button type="button" class="card-header" onclick={() => toggle('supplements')} aria-expanded={!collapsed.supplements} aria-controls="supplements-body">
            <span class="card-header-title" role="heading" aria-level="3">💊 Supplements to Discuss</span>
            <span class="chevron">{collapsed.supplements ? '▶' : '▼'}</span>
          </button>
          {#if !collapsed.supplements}
            <div class="card-body" id="supplements-body">
              <p class="section-hint">Review interactions and health context before use.</p>
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
          <button type="button" class="card-header" onclick={() => toggle('cycleSupport')} aria-expanded={!collapsed.cycleSupport} aria-controls="cycle-support-body">
            <span class="card-header-title" role="heading" aria-level="3">⚕️ Reproductive &amp; Hormone Support</span>
            <span class="chevron">{collapsed.cycleSupport ? '▶' : '▼'}</span>
          </button>
          {#if !collapsed.cycleSupport}
            <div class="card-body" id="cycle-support-body">
              <p class="section-hint">Guidance for {selectedReproductiveContextLabel()}: timing, symptoms, products, and follow-up questions.</p>
              {#if plan.cycleSupport.diaryReview}
                <div class="cycle-diary-review" role="region" aria-labelledby="cycle-diary-review-title">
                  <h4 id="cycle-diary-review-title">📈 Observed diary review</h4>
                  <p>{plan.cycleSupport.diaryReview.entry_count} saved observation{plan.cycleSupport.diaryReview.entry_count === 1 ? '' : 's'} across {plan.cycleSupport.diaryReview.observed_date_count} dated day{plan.cycleSupport.diaryReview.observed_date_count === 1 ? '' : 's'}{#if plan.cycleSupport.diaryReview.first_observed_date && plan.cycleSupport.diaryReview.last_observed_date} ({plan.cycleSupport.diaryReview.first_observed_date} to {plan.cycleSupport.diaryReview.last_observed_date}{#if plan.cycleSupport.diaryReview.observation_span_days !== null}, {plan.cycleSupport.diaryReview.observation_span_days} day span{/if}).{/if}</p>
                  <div class="cycle-diary-review-metrics">
                    {#each plan.cycleSupport.diaryReview.metrics as metric (metric.id)}
                      <div class="cycle-diary-review-metric">
                        <strong>{metric.label}</strong>
                        <span>{metric.elevated_days} day{metric.elevated_days === 1 ? '' : 's'} at or above self-rated {metric.threshold} ({metric.recorded_days} recorded)</span>
                      </div>
                    {/each}
                  </div>
                  {#if plan.cycleSupport.diaryReview.cycle_day_observation_count > 0}
                    <p><strong>Cycle-day entries:</strong> {plan.cycleSupport.diaryReview.cycle_day_observation_count}. These are user-entered labels only; they do not establish ovulation, luteal phase, hormone levels, or a cause.</p>
                    <div class="cycle-diary-review-days" aria-label="Observed cycle-day comparisons">
                      {#each plan.cycleSupport.diaryReview.cycle_day_summary.slice(0, 12) as day (day.cycle_day)}
                        {#each day.metrics as metric (metric.metric_id)}
                          {#if metric.recorded_days > 0}
                            <span>
                              Day {day.cycle_day}: {metric.label} {metric.elevated_days}/{metric.recorded_days} recorded entries at or above {metric.threshold}
                              {#if metric.enough_observations}
                                ({metric.observed_share_percent}% observed share)
                              {:else}
                                (one recorded entry; collect at least {metric.minimum_observations} before comparing)
                              {/if}
                            </span>
                          {/if}
                        {/each}
                      {/each}
                    </div>
                  {/if}
                  {#if plan.cycleSupport.diaryReview.co_occurrence.mood_behavior_with_bleeding_days > 0 || plan.cycleSupport.diaryReview.co_occurrence.pain_headache_with_bleeding_days > 0}
                    <p><strong>Observed co-occurrence:</strong> mood/behavior impact and bleeding were both recorded on {plan.cycleSupport.diaryReview.co_occurrence.mood_behavior_with_bleeding_days} day{plan.cycleSupport.diaryReview.co_occurrence.mood_behavior_with_bleeding_days === 1 ? '' : 's'}; pain/headache impact and bleeding were both recorded on {plan.cycleSupport.diaryReview.co_occurrence.pain_headache_with_bleeding_days} day{plan.cycleSupport.diaryReview.co_occurrence.pain_headache_with_bleeding_days === 1 ? '' : 's'}. This is descriptive only, not evidence of a hormone cause.</p>
                  {/if}
                  <ul class="guardrail-list">
                    {#each plan.cycleSupport.diaryReview.notes as note (note)}<li>{note}</li>{/each}
                  </ul>
                </div>
              {/if}
              {#if plan.cycleSupport.relevantEvidenceLayers.length > 0}
                <div class="reproductive-evidence-layer" role="note">
                  <strong>🧬 How to read the DNA for this context</strong>
                  {#if plan.cycleSupport.dnaCoverage}
                    <p class="reproductive-coverage"><strong>Selected-context DNA coverage:</strong> {plan.cycleSupport.dnaCoverage.present_marker_count} of {plan.cycleSupport.dnaCoverage.tracked_marker_count} tracked markers have a raw call; {plan.cycleSupport.dnaCoverage.callable_marker_count} are verified for interpretation; {plan.cycleSupport.dnaCoverage.unknown_marker_count} are unknown or unavailable. Coverage is not a risk score.</p>
                  {/if}
                  {#each plan.cycleSupport.relevantEvidenceLayers as layer (layer.id)}
                    <div class="reproductive-evidence-item">
                      <h4>{layer.title}</h4>
                      <p>{layer.summary}</p>
                      <p><strong>Useful next step:</strong> {layer.next_step}</p>
                    </div>
                  {/each}
                </div>
              {/if}
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
        <button type="button" class="card-header" onclick={() => toggle('activity')} aria-expanded={!collapsed.activity} aria-controls="activity-body">
          <span class="card-header-title" role="heading" aria-level="3">🏃 Activity &amp; Recovery Guardrails</span>
          <span class="chevron">{collapsed.activity ? '▶' : '▼'}</span>
        </button>
        {#if !collapsed.activity}
          <div class="card-body" id="activity-body">
            <p class="section-hint">Planning prompts for training and recovery — not activity clearance.</p>
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
        <button type="button" class="card-header" onclick={() => toggle('medication')} aria-expanded={!collapsed.medication} aria-controls="medication-body">
          <span class="card-header-title" role="heading" aria-level="3">💊 Medication Safety &amp; Context</span>
          <span class="chevron">{collapsed.medication ? '▶' : '▼'}</span>
        </button>
        {#if !collapsed.medication}
          <div class="card-body" id="medication-body">
            <p class="section-hint">Bring current medications and past responses to a clinician or pharmacist.</p>
            {#if plan.pgxGuidance.relevantGenes.length > 0}
              <div class="pgx-readiness" role="note">
                <strong>🧪 PGx completeness check</strong>
                <p>{plan.pgxGuidance.policy.summary}</p>
                {#each plan.pgxGuidance.relevantGenes as gene (gene.id)}
                  <div class="pgx-readiness-item">
                    <strong>{gene.label}</strong>
                    <p>{gene.limitation}</p>
                    <p><strong>Useful next step:</strong> {gene.clinical_next_step}</p>
                  </div>
                {/each}
              </div>
            {/if}
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
        <button type="button" class="card-header" onclick={() => toggle('labTests')} aria-expanded={!collapsed.labTests} aria-controls="lab-followups-body">
          <span class="card-header-title" role="heading" aria-level="3">🔬 Lab &amp; screening follow-ups ({plan.labTests.length})</span>
          <span class="chevron">{collapsed.labTests ? '▶' : '▼'}</span>
        </button>
        {#if !collapsed.labTests}
          <div class="card-body lab-body" id="lab-followups-body">
            <p class="section-hint">Grouped by priority for clinician discussion.</p>
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
                                <span class="lab-chip-counselor" aria-label="Genetic counselor advised">🧑‍⚕️</span>
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

  .action-queue {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
    background: var(--surface-raised);
  }

  .action-queue-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .section-kicker {
    display: block;
    margin-bottom: 0.25rem;
    color: var(--accent);
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .action-queue-header h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1.15rem;
  }

  .action-queue-header p {
    max-width: 46rem;
    margin: 0.4rem 0 0;
    color: var(--text-secondary);
    font-size: 0.78rem;
    line-height: 1.45;
  }

  .action-queue-count {
    flex: 0 0 auto;
    padding: 0.35rem 0.55rem;
    border: 1px solid var(--border-color);
    border-radius: 999px;
    color: var(--text-secondary);
    font-size: 0.7rem;
    font-weight: 700;
  }

  .action-queue-list {
    display: grid;
    gap: 0.75rem;
    padding-top: 1rem;
  }

  .action-queue-item {
    padding: 0.9rem 1rem;
    border: 1px solid var(--border-color);
    border-left: 3px solid var(--accent);
    border-radius: 0.65rem;
    background: var(--surface-subtle);
  }

  .action-queue-item-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .action-queue-item h4 {
    margin: 0;
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .action-queue-context {
    display: block;
    margin-top: 0.2rem;
    color: var(--text-secondary);
    font-size: 0.7rem;
  }

  .action-queue-item > p {
    margin: 0.65rem 0;
    color: var(--text-primary);
    font-size: 0.8rem;
    line-height: 1.45;
  }

  .action-queue-next {
    padding: 0.55rem 0.65rem;
    border-radius: 0.45rem;
    background: var(--accent-soft);
    color: var(--text-secondary);
    font-size: 0.75rem;
    line-height: 1.4;
  }

  .action-queue-next strong {
    color: var(--text-primary);
  }

  .action-queue-item .jump-btn {
    margin-top: 0.55rem;
  }

  .action-queue-empty {
    display: grid;
    gap: 0.35rem;
    padding-top: 1rem;
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.45;
  }

  .action-queue-empty strong {
    color: var(--text-primary);
  }

  .actionability-safety {
    border: 1px solid var(--status-info-border);
    background: var(--status-info-bg);
    color: var(--text-secondary);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    font-size: 0.75rem;
    line-height: 1.45;
  }
  .actionability-safety summary {
    color: var(--status-info-text);
    cursor: pointer;
    font-weight: 700;
  }
  .actionability-safety ul {
    margin: 0.35rem 0 0;
    padding-left: 1.2rem;
  }
  .context-selector {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border: 1px solid var(--coverage-border);
    background: var(--coverage-bg);
  }

  .context-selector-copy {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
    font-size: 0.72rem;
    line-height: 1.4;
  }

  .context-selector-heading {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    min-width: 0;
  }

  .context-selector-heading strong {
    min-width: 0;
  }

  .context-info-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.1rem;
    min-height: 1.1rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .context-selector-copy strong {
    color: var(--coverage-text);
  }

  .context-selector-copy span {
    color: var(--text-secondary);
  }

  .personal-context-card {
    gap: 0.65rem;
    border-color: var(--status-success-border);
    background: var(--status-success-bg);
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
    color: var(--status-success-text);
  }

  .personal-context-grid span {
    color: var(--text-secondary);
    overflow-wrap: anywhere;
  }

  .context-selector select {
    min-width: min(320px, 42%);
    background: var(--surface-control);
    border: 1px solid var(--coverage-border);
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
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 0.55rem 0.65rem;
    background: var(--surface-control);
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
    color: var(--text-primary);
  }

  .lab-tier-count {
    font-size: 0.62rem;
    font-weight: 700;
    opacity: 0.65;
    padding: 0.05rem 0.35rem;
    border-radius: 999px;
    background: var(--surface-subtle);
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
    border: 1px solid var(--border-color);
    background: var(--surface-card);
    color: inherit;
    cursor: pointer;
    font: inherit;
  }

  .lab-chip-main:hover {
    border-color: var(--border-strong);
    background: var(--surface-subtle);
  }

  .lab-chip-counselor .lab-chip-main {
    border-color: var(--status-accent-soft-border);
    background: var(--status-accent-soft-bg);
  }

  .lab-chip-name {
    font-size: 0.68rem;
    font-weight: 600;
    color: var(--text-primary);
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
    background: var(--status-accent-bg);
    color: var(--status-accent-text);
  }

  .lab-chip-badge.badge-discuss {
    background: var(--status-warning-bg);
    color: var(--status-warning-text);
  }

  .lab-chip-badge.badge-optional {
    background: var(--status-info-bg);
    color: var(--status-info-text);
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
    background: var(--surface-card);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    overflow: hidden;
    height: fit-content;
    transition: all 0.2s ease;
    min-width: 0;
  }
  .summary-card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 4px 12px var(--shadow-subtle);
  }

  .card-header {
    display: flex;
    width: 100%;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background: var(--surface-subtle);
    border: 0;
    border-bottom: 1px solid var(--border-color);
    border-radius: 0;
    appearance: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    user-select: none;
  }
  .card-header:hover {
    background: var(--surface-subtle);
  }
  .card-header:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .card-header-title {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: var(--text-primary);
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
    background: var(--surface-subtle);
    border-left: 3px solid var(--border-strong);
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
    background: var(--coverage-bg);
    color: var(--coverage-text);
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
    background: var(--status-danger-bg);
    color: var(--status-danger-text);
  }
  .severity-badge.confirmation_required {
    background: var(--status-warning-bg);
    color: var(--status-warning-text);
  }
  .severity-badge.moderate_risk {
    background: var(--status-warning-bg);
    color: var(--status-warning-text);
  }
  .severity-badge.low_risk {
    background: var(--status-info-bg);
    color: var(--status-info-text);
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
    color: var(--status-info-text);
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

  .dietary-profile-safety,
  .dietary-resource-rules {
    margin-bottom: 0.85rem;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--status-success-border);
    border-radius: 6px;
    background: var(--status-success-bg);
    color: var(--text-secondary);
    font-size: 0.68rem;
    line-height: 1.4;
  }

  .dietary-profile-safety > p {
    margin: 0.2rem 0 0.5rem;
  }

  .dietary-profile-list {
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
    margin: 0.45rem 0;
  }

  .dietary-profile-list strong,
  .dietary-resource-rules > strong {
    color: var(--status-success-text);
  }

  .dietary-profile-allergy {
    color: var(--status-danger-text);
  }

  .dietary-profile-allergy strong {
    color: var(--status-danger-strong-text);
  }

  .dietary-resource-rule {
    margin-top: 0.55rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-color);
  }

  .dietary-resource-rule p {
    margin: 0.2rem 0;
  }

  .dietary-resource-rule > span {
    display: block;
    margin-top: 0.18rem;
  }

  .dietary-suppressed-suggestions {
    margin-bottom: 0.85rem;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--status-warning-border);
    border-radius: 6px;
    background: var(--status-warning-bg);
    color: var(--text-secondary);
    font-size: 0.68rem;
    line-height: 1.4;
  }

  .dietary-suppressed-suggestions p {
    margin: 0.2rem 0 0.45rem;
  }

  .dietary-suppressed-suggestions ul {
    margin: 0.35rem 0 0;
  }

  .dietary-suppressed-suggestions > strong {
    color: var(--status-warning-text);
  }

  .diet-column h4 {
    margin: 0 0 0.4rem 0;
    font-size: 0.75rem;
    font-weight: bold;
  }
  .diet-column.favor h4 { color: var(--status-success-text); }
  .diet-column.avoid h4 { color: var(--status-danger-strong-text); }
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
    border-top: 1px solid var(--border-color);
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
    background: var(--surface-subtle);
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    border-left: 2px solid var(--status-accent-soft-border);
  }
  .supp-name {
    font-size: 0.75rem;
    font-weight: bold;
    color: var(--status-accent-soft-text);
  }
  .supp-reason {
    font-size: 0.68rem;
    opacity: 0.75;
    margin-top: 0.1rem;
  }
  .supplement-safety {
    margin-top: 0.85rem;
    padding-top: 0.65rem;
    border-top: 1px solid var(--border-color);
  }
  .supplement-safety-rule {
    margin-top: 0.65rem;
    padding: 0.45rem 0.55rem;
    border-left: 2px solid var(--status-warning-border);
    background: var(--status-warning-bg);
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
    border-left: 2px solid var(--status-info-soft-border);
    background: var(--status-info-soft-bg);
    border-radius: 4px;
  }
  .cycle-support-domain {
    margin-top: 0.85rem;
    padding: 0.55rem 0.65rem;
    border-left: 2px solid var(--status-accent-soft-border);
    background: var(--status-accent-soft-bg);
    border-radius: 4px;
  }
  .cycle-diary-review {
    margin: 0.65rem 0 0.85rem;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--status-success-border);
    border-left: 3px solid var(--status-success-text);
    background: var(--status-success-bg);
    border-radius: 5px;
  }
  .cycle-diary-review h4 {
    margin: 0;
    font-size: 0.75rem;
    color: var(--status-success-text);
  }
  .cycle-diary-review > p,
  .cycle-diary-review-metric span,
  .cycle-diary-review-days span {
    font-size: 0.7rem;
    line-height: 1.45;
  }
  .cycle-diary-review > p {
    margin: 0.3rem 0 0;
  }
  .cycle-diary-review-metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
    gap: 0.45rem;
    margin-top: 0.6rem;
  }
  .cycle-diary-review-metric {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    padding: 0.4rem 0.5rem;
    border-radius: 4px;
    background: var(--surface-subtle);
  }
  .cycle-diary-review-metric strong {
    font-size: 0.7rem;
  }
  .cycle-diary-review-metric span,
  .cycle-diary-review-days span {
    color: var(--text-secondary);
  }
  .cycle-diary-review-days {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-top: 0.4rem;
  }
  .cycle-diary-review-days span {
    padding: 0.25rem 0.4rem;
    border: 1px solid var(--status-success-border);
    border-radius: 4px;
  }
  .reproductive-evidence-layer {
    margin: 0.65rem 0 0.85rem;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--status-info-soft-border);
    border-left: 3px solid var(--status-info-text);
    background: var(--status-info-soft-bg);
    border-radius: 5px;
  }
  .reproductive-evidence-item {
    margin-top: 0.65rem;
    padding-top: 0.55rem;
    border-top: 1px solid var(--border-color);
  }
  .reproductive-evidence-item h4 {
    margin: 0;
    font-size: 0.75rem;
    color: var(--status-info-soft-text);
  }
  .reproductive-evidence-item p {
    margin: 0.25rem 0 0;
    font-size: 0.7rem;
    line-height: 1.45;
  }
  .reproductive-coverage {
    margin: 0.35rem 0 0;
    font-size: 0.7rem;
    line-height: 1.45;
    color: var(--status-info-soft-text);
  }
  .pgx-readiness {
    margin: 0.65rem 0 0.85rem;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--status-info-soft-border);
    border-left: 3px solid var(--status-info-text);
    background: var(--status-info-soft-bg);
    border-radius: 5px;
  }
  .pgx-readiness > p,
  .pgx-readiness-item p {
    margin: 0.25rem 0 0;
    font-size: 0.7rem;
    line-height: 1.45;
  }
  .pgx-readiness-item {
    margin-top: 0.65rem;
    padding-top: 0.55rem;
    border-top: 1px solid var(--border-color);
  }
  .pgx-readiness-item > strong {
    font-size: 0.75rem;
    color: var(--status-info-soft-text);
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
    color: var(--status-info-soft-text);
  }
  .activity-stop-list {
    margin-top: 0.85rem;
    padding: 0.55rem 0.65rem;
    border: 1px solid var(--status-danger-border);
    border-radius: 4px;
    background: var(--status-danger-bg);
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
