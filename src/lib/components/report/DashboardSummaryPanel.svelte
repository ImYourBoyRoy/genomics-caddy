<!-- ./src/lib/components/report/DashboardSummaryPanel.svelte -->
<script lang="ts">
import type { GeneratedReport } from '../../types/genomics';
import allergySensitivityCatalog from '../../marker-packs/allergy_sensitivity_catalog.json';
import { buildLabRequestListText, deriveActionablePlan, type ActionablePlan, type LabTest } from '../../utils/actionabilityEngine';
import { getCompactGuidanceText, getCompactSupplementName, getCompactSupplementReason, getLaypersonTranslation, getSimpleFindingCopy } from '../../utils/layperson';
import type { PresentationMode } from '../../utils/presentationPreferences';

  interface Props {
    report: GeneratedReport;
    sampleId: number;
    onJumpToMarker?: (linkId: string) => void;
    onJumpToMarkers?: (linkIds: string[]) => void;
    onJumpToSection?: (sectionName: string) => void;
    presentationMode?: PresentationMode;
  }

  let {
    report,
    sampleId,
    onJumpToMarker,
    onJumpToMarkers,
    onJumpToSection,
    presentationMode = 'simple',
  }: Props = $props();

  let plan = $derived<ActionablePlan>(deriveActionablePlan(report));

  // Fresh profile defaults are collapsed. Deliberate expansion is retained per profile.
  let collapsed = $state({
    topFindings: true,
    allergy: true,
    diet: true,
    supplements: true,
    activity: true,
    medication: true,
    labTests: true,
  });
  let loadedCollapseProfileId = $state<number | null>(null);

  let expandedLabReasons = $state<Record<string, boolean>>({});
  let labRequestCopied = $state(false);

  function dashboardCollapseStorageKey(profileId: number): string {
    // Version the key so the old global preference cannot reopen a dense dashboard.
    return `genomics_dashboard_collapsed_v2_${profileId}`;
  }

  $effect(() => {
    if (!sampleId || loadedCollapseProfileId === sampleId) return;
    const defaults = {
      topFindings: true,
      allergy: true,
      diet: true,
      supplements: true,
      activity: true,
      medication: true,
      labTests: true,
    };
    let next = defaults;
    try {
      const stored = localStorage.getItem(dashboardCollapseStorageKey(sampleId));
      if (stored) {
        const parsed = JSON.parse(stored) as Partial<typeof defaults>;
        next = { ...defaults, ...parsed };
      }
    } catch {
      next = defaults;
    }
    loadedCollapseProfileId = sampleId;
    collapsed = next;
  });

  function toggle(section: keyof typeof collapsed) {
    collapsed = { ...collapsed, [section]: !collapsed[section] };
    try {
      if (sampleId) {
        localStorage.setItem(dashboardCollapseStorageKey(sampleId), JSON.stringify(collapsed));
      }
    } catch {
      // The in-memory dashboard remains usable when localStorage is unavailable.
    }
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
      ? getSimpleFindingCopy(marker, getLaypersonTranslation(marker)).why_it_matters
      : 'A research finding is available to review in the right personal context.';
  }

  function findingTitle(finding: ActionablePlan['topFindings'][number]): string {
    const marker = markerForFinding(finding);
    return marker
      ? getSimpleFindingCopy(marker, getLaypersonTranslation(marker)).plain_title
      : 'Research finding';
  }

  function findingNextStep(finding: ActionablePlan['topFindings'][number]): string {
    const marker = markerForFinding(finding);
    if (!marker) return 'Review the detailed finding for personal relevance.';
    const translation = getLaypersonTranslation(marker);
    return getSimpleFindingCopy(marker, translation).review_action;
  }

  function priorityTone(finding: ActionablePlan['topFindings'][number]): 'high' | 'moderate' | 'low' {
    if (finding.priority_tone === 'danger') return 'high';
    if (finding.priority_tone === 'warning') return 'moderate';
    return 'low';
  }

  function activityDomainLabel(domain: { id: string; label?: string }): string {
    return domain.label || domain.id.replaceAll('_', ' ');
  }

  type ActivityDomain = ActionablePlan['activity']['relevantDomains'][number];
  type ActivityListKey = 'favor' | 'avoid' | 'confirm_with';

  function activityItems(domain: ActivityDomain, kind: ActivityListKey): string[] {
    const compactKey = kind === 'favor'
      ? 'simple_favor'
      : kind === 'avoid'
        ? 'simple_watch'
        : 'simple_verify';
    const compactItems = presentationMode === 'simple' ? domain[compactKey] : undefined;
    return [...(compactItems?.length ? compactItems : domain[kind])];
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

  async function copyLabRequestList() {
    const text = buildLabRequestListText(plan.labTests);
    try {
      await navigator.clipboard.writeText(text);
      labRequestCopied = true;
      window.setTimeout(() => { labRequestCopied = false; }, 1800);
    } catch {
      labRequestCopied = false;
    }
  }

  let labRequestListText = $derived(buildLabRequestListText(plan.labTests));

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

  function sectionAnchorId(sectionName: string): string {
    return `report-section-${sectionName.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  }

  let healthAreaSections = $derived((report.sections || []).filter((section) => section.markers.length > 0));
  let hasAllergyGuidance = $derived(
    plan.allergy.dnaContexts.length > 0
      || plan.allergy.medicationSafety.length > 0,
  );
  let hasNutritionGuidance = $derived(
    plan.diet.favor.length > 0
      || plan.diet.avoid.length > 0
      || plan.foodSafety.explicitExclusions.length > 0
      || plan.foodSafety.confirmedAllergies.length > 0
      || plan.foodSafety.suspectedAllergies.length > 0
      || plan.foodSafety.relevantRules.length > 0
      || plan.foodSafety.suppressedSuggestions.length > 0
      || plan.supplements.length > 0
      || plan.supplementAvoid.length > 0
      || plan.supplementSafety.relevantRules.length > 0,
  );
  let hasTrainingGuidance = $derived(plan.activity.relevantDomains.length > 0);
  let hasMedicationGuidance = $derived(
    plan.medicationPathways.length > 0,
  );
  let hasClinicalGuidance = $derived(hasMedicationGuidance || plan.labTests.length > 0);
  let priorityColumns = $derived(
    Array.from({ length: Math.min(3, Math.ceil(Math.min(plan.topFindings.length, 9) / 3)) }, (_, columnIndex) =>
      plan.topFindings.slice(columnIndex * 3, columnIndex * 3 + 3).map((finding, rowIndex) => ({
        finding,
        rank: columnIndex * 3 + rowIndex + 1,
      })),
    ),
  );
</script>

<div class="dashboard-v2">
  <section class="action-queue summary-card card" aria-labelledby="action-queue-title">
    <div class="action-queue-header">
      <div>
        <span class="section-kicker">Start here</span>
        <h3 id="action-queue-title">Priority findings</h3>
        <p>DNA signals with the clearest reason to review them first.</p>
      </div>
      <span class="action-queue-count">{Math.min(plan.topFindings.length, 9)} shown</span>
    </div>

    {#if plan.topFindings.length > 0}
      <div class="action-queue-list">
        {#each priorityColumns as column, columnIndex}
          <div class="action-queue-column" data-column={columnIndex + 1}>
            {#each column as item (item.finding.link_id || `${item.finding.rsid}:${item.finding.gene}`)}
              <article class="action-queue-item" data-priority={item.rank} data-concern={priorityTone(item.finding)}>
                <span class="action-queue-rank" aria-label={`Priority ${item.rank}`}>{item.rank}</span>
                <div class="action-queue-item-content">
                  <div class="action-queue-item-top">
                    <div>
                      <h4>{findingTitle(item.finding)}</h4>
                      <span class="action-queue-context">{item.finding.section_name}</span>
                    </div>
                    <span class="action-queue-concern {priorityTone(item.finding)}">
                      <span class="concern-dot" aria-hidden="true"></span>
                      {item.finding.priority_reason}
                    </span>
                  </div>
                  <div class="action-queue-signal-row">
                    <span class="action-queue-signal">DNA-linked</span>
                    {#if item.finding.related_marker_count && item.finding.related_marker_count > 1}
                      <span>{item.finding.related_marker_count} related markers</span>
                    {/if}
                  </div>
                  <p>{findingMeaning(item.finding)}</p>
                  <div class="action-queue-next"><strong class="action-queue-next-label">Next</strong> {findingNextStep(item.finding)}</div>
                  {#if onJumpToMarker}
                    <button class="btn btn-xs btn-link jump-btn" type="button" onclick={() => onJumpToMarker?.(item.finding.link_id)}>
                      View DNA finding →
                    </button>
                  {/if}
                </div>
              </article>
            {/each}
          </div>
        {/each}
      </div>
    {:else}
      <div class="action-queue-empty">
        <strong>No immediate genetic follow-up prompts were generated.</strong>
        <span>Use routine care and selected personal context to guide follow-up.</span>
      </div>
    {/if}
  </section>

  {#if presentationMode !== 'simple' && plan.safetyNotes.length > 0}
    <details class="actionability-safety">
      <summary>🧭 How to use suggestions</summary>
      <ul>
        {#each plan.safetyNotes as note (note)}
          <li>{note}</li>
        {/each}
      </ul>
    </details>
  {/if}

  {#if hasAllergyGuidance || hasNutritionGuidance || hasTrainingGuidance || hasClinicalGuidance}
    <nav class="guidance-index" aria-labelledby="guidance-index-title">
      <div class="guidance-index-heading">
        <div>
          <span class="section-kicker">Personalized guidance</span>
          <h3 id="guidance-index-title">Use your results</h3>
        </div>
        <span>Jump to a section</span>
      </div>
      <div class="guidance-index-links">
        {#if hasAllergyGuidance}
          <a href="#allergy-guidance-group">Allergy &amp; sensitivity <span>→</span></a>
        {/if}
        {#if hasNutritionGuidance}
          <a href="#nutrition-guidance-group">Food &amp; supplements <span>→</span></a>
        {/if}
        {#if hasTrainingGuidance}
          <a href="#training-guidance-group">Training &amp; recovery <span>→</span></a>
        {/if}
        {#if hasClinicalGuidance}
          <a href="#clinical-guidance-group">Medication &amp; labs <span>→</span></a>
        {/if}
      </div>
    </nav>
  {/if}

  <div class="guidance-flow">
    {#if hasAllergyGuidance}
    <section id="allergy-guidance-group" class="guidance-group" aria-labelledby="allergy-guidance-group-title">
      <div class="guidance-group-heading">
        <div>
          <span class="section-kicker">Protect &amp; understand</span>
          <h3 id="allergy-guidance-group-title">Allergy &amp; sensitivity</h3>
          <p>DNA-linked allergy, sensitivity, and medication pathways found in this report.</p>
        </div>
      </div>

    <section class="summary-card card allergy-card" class:collapsed={collapsed.allergy} aria-labelledby="allergy-map-title">
      <h3 class="card-header-heading">
        <button type="button" class="card-header" onclick={() => toggle('allergy')} aria-expanded={!collapsed.allergy} aria-controls="allergy-map-body">
          <span class="card-header-title">🧬 Allergy &amp; sensitivity map</span>
          <span class="chevron">{collapsed.allergy ? '▶' : '▼'}</span>
        </button>
      </h3>
      {#if collapsed.allergy}
        <div id="allergy-map-body" hidden aria-hidden="true"></div>
      {:else}
        <div class="card-body allergy-map-body" id="allergy-map-body">
          <div class="allergy-map-intro">
            <div>
              <h3 id="allergy-map-title">Allergy &amp; sensitivity signals</h3>
              <p>{allergySensitivityCatalog.display.dna_intro}</p>
            </div>
            {#if plan.allergy.hasDnaSignal}
              <span class="allergy-signal-count">{plan.allergy.matchedMarkerCount} DNA-linked markers</span>
            {/if}
          </div>

          <section class="allergy-dna-section" aria-labelledby="allergy-dna-title">
              <div class="allergy-section-heading">
                <div>
                  <h4 id="allergy-dna-title">Matched pathways</h4>
                  <span>What the matched variants point toward</span>
                </div>
                <span class="allergy-section-count">{plan.allergy.dnaContexts.length}</span>
              </div>
              {#if plan.allergy.dnaContexts.length > 0}
                <div class="allergy-context-grid">
                  {#each plan.allergy.dnaContexts as context (context.id)}
                    <article class="insight-tile insight-tile-info allergy-context-card">
                      <div class="insight-tile-heading">
                        <div>
                          <h5 class="insight-tile-title">{context.label}</h5>
                            <span class="insight-tile-eyebrow">{context.signal_label}</span>
                        </div>
                        <span class="insight-tile-count">{context.matched_marker_count} DNA</span>
                      </div>
                      <p class="insight-tile-copy">{context.summary}</p>
                      <div class="insight-tile-relevance"><strong>Relevant when</strong> {context.relevance}</div>
                      <div class="insight-tile-examples"><strong>Examples</strong> {context.examples.join(' · ')}</div>
                      <div class="insight-tile-footer">
                        <span class="insight-tile-meta">Genes: {context.matched_genes.join(' · ')}</span>
                        {#if (onJumpToMarkers || onJumpToMarker) && context.matched_marker_link_ids.length > 0}
                          <button class="btn btn-xs btn-link insight-tile-link" type="button" onclick={() => onJumpToMarkers?.(context.matched_marker_link_ids) ?? onJumpToMarker?.(context.matched_marker_link_ids[0])}>
                            View {context.matched_marker_link_ids.length} matched DNA {context.matched_marker_link_ids.length === 1 ? 'finding' : 'findings'} →
                          </button>
                        {/if}
                      </div>
                    </article>
                  {/each}
                </div>
              {:else}
                <p class="allergy-empty">No active allergy-pathway finding was matched in this report.</p>
              {/if}

              {#if plan.allergy.medicationSafety.length > 0}
                <div class="allergy-medication-section">
                  <div class="allergy-section-heading">
                    <div>
                      <h4>Medication safety routes</h4>
                      <span>Only shown for a matching drug-specific marker</span>
                    </div>
                    <span class="allergy-section-count">{plan.allergy.medicationSafety.length}</span>
                  </div>
                  <div class="allergy-medication-grid">
                    {#each plan.allergy.medicationSafety as route (route.id)}
                      <article class="insight-tile insight-tile-alert allergy-medication-card" data-warning-kind="actionable_safety">
                        <div class="insight-tile-heading">
                          <div>
                            <h5 class="insight-tile-title">{route.label}</h5>
                            <span class="insight-tile-eyebrow">{route.signal_label}</span>
                          </div>
                          <span class="insight-tile-count">{route.matched_marker_count} DNA</span>
                        </div>
                        <div class="insight-tile-tags">{route.medications.join(' · ')}</div>
                        <p class="insight-tile-copy">{route.summary}</p>
                        <div class="insight-tile-relevance"><strong>Relevant when</strong> {route.relevance}</div>
                        <div class="insight-tile-footer">
                          <span class="insight-tile-meta">Genes: {route.matched_genes.join(' · ')}</span>
                          {#if (onJumpToMarkers || onJumpToMarker) && route.matched_marker_link_ids.length > 0}
                            <button class="btn btn-xs btn-link insight-tile-link" type="button" onclick={() => onJumpToMarkers?.(route.matched_marker_link_ids) ?? onJumpToMarker?.(route.matched_marker_link_ids[0])}>
                              View {route.matched_marker_link_ids.length} matched DNA {route.matched_marker_link_ids.length === 1 ? 'finding' : 'findings'} →
                            </button>
                          {/if}
                        </div>
                      </article>
                    {/each}
                  </div>
                </div>
              {/if}
            <p class="allergy-map-footer">{allergySensitivityCatalog.display.compact_footer}</p>
          </section>
        </div>
      {/if}
    </section>

    </section>
    {/if}

    {#if hasNutritionGuidance}
    <section id="nutrition-guidance-group" class="guidance-group" aria-labelledby="nutrition-guidance-group-title">
      <div class="guidance-group-heading">
        <div>
          <span class="section-kicker">Build your baseline</span>
          <h3 id="nutrition-guidance-group-title">Food &amp; supplements</h3>
          <p>Concrete food ideas and DNA-linked supplement pathways.</p>
        </div>
      </div>

  <div class="grid-layout">
    <div class="action-row">
      <!-- Panel 2: Dietary Guidance -->
      {#if plan.diet.favor.length > 0 || plan.diet.avoid.length > 0 || plan.foodSafety.explicitExclusions.length > 0 || plan.foodSafety.confirmedAllergies.length > 0 || plan.foodSafety.suspectedAllergies.length > 0 || plan.foodSafety.relevantRules.length > 0 || plan.foodSafety.suppressedSuggestions.length > 0}
        <div class="summary-card card" class:collapsed={collapsed.diet}>
          <h3 class="card-header-heading">
            <button type="button" class="card-header" onclick={() => toggle('diet')} aria-expanded={!collapsed.diet} aria-controls="dietary-alignment-body">
              <span class="card-header-title">🥗 Dietary Alignment</span>
              <span class="chevron">{collapsed.diet ? '▶' : '▼'}</span>
            </button>
          </h3>
          {#if collapsed.diet}
            <div id="dietary-alignment-body" hidden aria-hidden="true"></div>
          {:else}
            <div class="card-body" id="dietary-alignment-body">
              <p class="section-hint">DNA-linked food ideas</p>
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
                <details class="guidance-details dietary-resource-rules">
                  <summary>Food safety checks ({plan.foodSafety.relevantRules.length})</summary>
                  <div class="guidance-details-body">
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
                </details>
              {/if}
              {#if plan.foodSafety.suppressedSuggestions.length > 0}
                <details class="guidance-details dietary-suppressed-suggestions" role="note">
                  <summary>Suggestions withheld by food constraints ({plan.foodSafety.suppressedSuggestions.length})</summary>
                  <div class="guidance-details-body">
                    <p>Suggestions that conflict with an explicitly entered food exclusion or reaction.</p>
                    <ul>
                      {#each plan.foodSafety.suppressedSuggestions as item (item)}<li>{item}</li>{/each}
                    </ul>
                    {#if plan.foodSafety.conflictNotes.length > 0}
                      <ul class="guardrail-list">
                        {#each plan.foodSafety.conflictNotes as note (note)}<li>{note}</li>{/each}
                      </ul>
                    {/if}
                  </div>
                </details>
              {/if}
              <div class="diet-section">
                {#if plan.diet.favor.length > 0}
                  <div class="diet-column favor">
                    <div class="diet-column-heading">
                      <h4><span aria-hidden="true">👍</span> Food ideas</h4>
                      <span class="diet-column-count">{plan.diet.favorItems.length}</span>
                    </div>
                    <div class="dietary-items">
                      {#each plan.diet.favorItems as item (item.recommendation_id)}
                        <article class="insight-tile insight-tile-favor dietary-item">
                          <span class="insight-tile-indicator" aria-hidden="true">＋</span>
                          <span class="insight-tile-content">
                            <span class="insight-tile-text dietary-item-text">{getCompactGuidanceText(item.name)}</span>
                            <span class="insight-tile-meta">DNA-linked · {item.basis_genes.join(' / ')} · {item.evidence_level}</span>
                            <details class="recommendation-provenance">
                              <summary>Why this appears</summary>
                              <span>{item.why_it_appears} · Topic: {item.basis_topic_ids.join(' / ')}</span>
                            </details>
                          </span>
                        </article>
                      {/each}
                    </div>
                  </div>
                {/if}

                {#if plan.diet.avoid.length > 0}
                  <div class="diet-column avoid">
                    <div class="diet-column-heading">
                      <h4><span aria-hidden="true">👎</span> Foods to limit</h4>
                      <span class="diet-column-count">{plan.diet.avoidItems.length}</span>
                    </div>
                    <div class="dietary-items">
                      {#each plan.diet.avoidItems as item (item.recommendation_id)}
                        <article class="insight-tile insight-tile-avoid dietary-item">
                          <span class="insight-tile-indicator" aria-hidden="true">!</span>
                          <span class="insight-tile-content">
                            <span class="insight-tile-text dietary-item-text">{getCompactGuidanceText(item.name)}</span>
                            <span class="insight-tile-meta">DNA-linked · {item.basis_genes.join(' / ')} · {item.evidence_level}</span>
                            <details class="recommendation-provenance">
                              <summary>Why this appears</summary>
                              <span>{item.why_it_appears} · Topic: {item.basis_topic_ids.join(' / ')}</span>
                            </details>
                          </span>
                        </article>
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Panel 3: Supplements -->
      {#if plan.supplements.length > 0 || plan.supplementAvoid.length > 0 || plan.supplementSafety.relevantRules.length > 0}
        <div class="summary-card card" class:collapsed={collapsed.supplements}>
          <h3 class="card-header-heading">
            <button type="button" class="card-header" onclick={() => toggle('supplements')} aria-expanded={!collapsed.supplements} aria-controls="supplements-body">
              <span class="card-header-title">💊 Supplements</span>
              <span class="chevron">{collapsed.supplements ? '▶' : '▼'}</span>
            </button>
          </h3>
          {#if collapsed.supplements}
            <div id="supplements-body" hidden aria-hidden="true"></div>
          {:else}
            <div class="card-body" id="supplements-body">
              <p class="section-hint">DNA-linked options to consider</p>
              <div class="supplement-columns">
                {#if plan.supplements.length > 0}
                  <div class="supplement-column">
                    <h4>👍 Consider</h4>
                    <div class="supplements-list">
                      {#each plan.supplements as s (`${s.name}:${s.reason}`)}
                        <article class="insight-tile insight-tile-consider supplement-item">
                          <span class="insight-tile-indicator" aria-hidden="true">＋</span>
                          <span class="insight-tile-content">
                            <span class="supp-name">{getCompactSupplementName(s.name)}</span>
                            <span class="supp-reason">{getCompactSupplementReason(s.reason)}</span>
                            <span class="insight-tile-meta">DNA-linked · {s.basis_genes.join(' / ')} · {s.evidence_level}</span>
                            <details class="recommendation-provenance">
                              <summary>Why this appears</summary>
                              <span>{s.why_it_appears} · Topic: {s.basis_topic_ids.join(' / ')}</span>
                            </details>
                          </span>
                        </article>
                      {/each}
                    </div>
                  </div>
                {/if}
                {#if plan.supplementAvoid.length > 0}
                  <div class="supplement-column avoid">
                    <h4>👎 Avoid / confirm first</h4>
                    <div class="supplements-list">
                      {#each plan.supplementAvoid as s (s.name)}
                        <article class="insight-tile insight-tile-avoid supplement-item">
                          <span class="insight-tile-indicator" aria-hidden="true">!</span>
                          <span class="insight-tile-content">
                            <span class="supp-name">{s.name}</span>
                            {#if s.reason}<span class="supp-reason">{s.reason}</span>{/if}
                            <span class="insight-tile-meta">DNA-linked · {s.basis_genes.join(' / ')} · {s.evidence_level}</span>
                            <details class="recommendation-provenance">
                              <summary>Why this appears</summary>
                              <span>{s.why_it_appears} · Topic: {s.basis_topic_ids.join(' / ')}</span>
                            </details>
                          </span>
                        </article>
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>
              {#if plan.supplementSafety.relevantRules.length > 0}
                <details class="supplement-safety-details">
                  <summary>Safety details</summary>
                  <div class="supplement-safety">
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
                </details>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

    </div>
  </div>
    </section>
    {/if}

    {#if hasTrainingGuidance}
    <section id="training-guidance-group" class="guidance-group" aria-labelledby="training-guidance-group-title">
      <div class="guidance-group-heading">
        <div>
          <span class="section-kicker">Move with feedback</span>
          <h3 id="training-guidance-group-title">Training &amp; recovery</h3>
          <p>Use your current ability, symptoms, and recovery signals to guide progression.</p>
        </div>
      </div>

    <div class="grid-layout">
    {#if plan.activity.relevantDomains.length > 0}
      <div class="summary-card card" class:collapsed={collapsed.activity}>
        <h3 class="card-header-heading">
          <button type="button" class="card-header" onclick={() => toggle('activity')} aria-expanded={!collapsed.activity} aria-controls="activity-body">
            <span class="card-header-title">🏃 Training &amp; Recovery</span>
            <span class="chevron">{collapsed.activity ? '▶' : '▼'}</span>
          </button>
        </h3>
        {#if collapsed.activity}
          <div id="activity-body" hidden aria-hidden="true"></div>
        {:else}
          <div class="card-body" id="activity-body">
            <p class="section-hint">DNA-informed starting points for training, recovery, and self-tracking.</p>
            {#if presentationMode === 'simple'}
              <div class="activity-framework" aria-label="Training framework">
                {#each plan.activity.simpleFramework as item (item.label)}
                  <div class="activity-framework-item">
                    <span>{item.label}</span>
                    <p>{item.text}</p>
                  </div>
                {/each}
              </div>
            {:else}
              <ul class="guardrail-list">
                {#each plan.activity.principles as principle (principle)}
                  <li>{principle}</li>
                {/each}
              </ul>
            {/if}
            <div class:activity-domain-grid={presentationMode === 'simple'}>
              {#each plan.activity.relevantDomains as domain (domain.id)}
                <div class="activity-domain">
                  <div class="activity-domain-heading">
                    <strong>{activityDomainLabel(domain)}</strong>
                    <span class="activity-domain-label">
                      {domain.matched_marker_ids?.length ? 'DNA-linked' : 'Context-linked'}
                    </span>
                  </div>
                  <span class="activity-context">{domain.context}</span>
                  {#if domain.matched_marker_ids?.length}
                    <span class="activity-provenance">
                      {domain.matched_marker_ids.length} DNA {domain.matched_marker_ids.length === 1 ? 'finding' : 'findings'}
                      {#if domain.matched_genes?.length} · {domain.matched_genes.join(' / ')}{/if}
                    </span>
                  {/if}
                  <div class="activity-columns">
                    <div class="activity-column-build">
                      <h4>{presentationMode === 'simple' ? 'Build around' : 'Favor'}</h4>
                      <ul class="guardrail-list">
                        {#each activityItems(domain, 'favor') as item (item)}<li>{item}</li>{/each}
                      </ul>
                    </div>
                    <div class="activity-column-watch">
                      <h4>{presentationMode === 'simple' ? 'Watch for' : 'Avoid'}</h4>
                      <ul class="guardrail-list">
                        {#each activityItems(domain, 'avoid') as item (item)}<li>{item}</li>{/each}
                      </ul>
                      <h4 class="activity-verify-heading">{presentationMode === 'simple' ? 'Verify when relevant' : 'Confirm with'}</h4>
                      <ul class="guardrail-list">
                        {#each activityItems(domain, 'confirm_with') as item (item)}<li>{item}</li>{/each}
                      </ul>
                    </div>
                  </div>
                  {#if plan.activity.recommendationItems.some((item) => item.basis_topic_ids.includes(domain.id))}
                    <details class="recommendation-provenance activity-provenance-details">
                      <summary>Why these prompts appear</summary>
                      <div class="activity-provenance-list">
                        {#each plan.activity.recommendationItems.filter((item) => item.basis_topic_ids.includes(domain.id)) as item (item.recommendation_id)}
                          <span><strong>{item.kind}:</strong> {item.name} — {item.why_it_appears} · Basis: {item.basis_marker_ids.join(' / ')}</span>
                        {/each}
                      </div>
                    </details>
                  {/if}
                </div>
              {/each}
            </div>
            {#if presentationMode === 'simple'}
              <div class="activity-stop-list activity-stop-compact">
                <strong>Pause and get prompt care for</strong>
                <span>{plan.activity.stopAndEscalate.join(' · ')}</span>
              </div>
            {:else}
              <div class="activity-stop-list">
                <strong>Stop activity and seek appropriate care for:</strong>
                <ul class="guardrail-list">
                  {#each plan.activity.stopAndEscalate as item (item)}<li>{item}</li>{/each}
                </ul>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
    </div>
    </section>
    {/if}

    {#if hasClinicalGuidance}
    <section id="clinical-guidance-group" class="guidance-group" aria-labelledby="clinical-guidance-group-title">
      <div class="guidance-group-heading">
        <div>
          <span class="section-kicker">Clarify with care</span>
          <h3 id="clinical-guidance-group-title">Medication &amp; clinical follow-up</h3>
          <p>DNA-linked medication topics and focused labs that can clarify a genetic finding.</p>
        </div>
      </div>

    <div class="grid-layout clinical-guidance-grid">
    {#if hasMedicationGuidance}
      <div class="summary-card card" class:collapsed={collapsed.medication}>
        <h3 class="card-header-heading">
          <button type="button" class="card-header" onclick={() => toggle('medication')} aria-expanded={!collapsed.medication} aria-controls="medication-body">
            <span class="card-header-title">💊 Medication pathways</span>
            <span class="chevron">{collapsed.medication ? '▶' : '▼'}</span>
          </button>
        </h3>
        {#if collapsed.medication}
          <div id="medication-body" hidden aria-hidden="true"></div>
        {:else}
          <div class="card-body" id="medication-body">
            <p class="section-hint">DNA-linked medication and treatment topics found in this report.</p>
            {#if plan.medicationPathways.length > 0}
              <div class="medication-pathways">
                {#each plan.medicationPathways as pathway (pathway.id)}
                  <article class="medication-pathway" data-warning-kind="clinical_review">
                    <div class="medication-pathway-heading">
                      <strong>{pathway.label}</strong>
                      <span>{pathway.genes.join(' · ')} · {pathway.matchedMarkerCount} DNA {pathway.matchedMarkerCount === 1 ? 'marker' : 'markers'}</span>
                    </div>
                    <p>{pathway.detail}</p>
                    <div class="medication-pathway-footer">
                      <span>{pathway.matchedMarkerCount} matched DNA {pathway.matchedMarkerCount === 1 ? 'finding' : 'findings'}</span>
                      {#if onJumpToMarker && pathway.matchedMarkerLinkIds.length > 0}
                        <button class="btn btn-xs btn-link" type="button" onclick={() => onJumpToMarker?.(pathway.matchedMarkerLinkIds[0])}>
                          View matching DNA →
                        </button>
                      {/if}
                    </div>
                  </article>
                {/each}
              </div>
            {:else}
              <p class="medication-empty">PGx pathway coverage is available for review in the detailed findings.</p>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Labs: full-width, grouped & compact (collapsed by default) -->
    {#if plan.labTests.length > 0}
      <div class="summary-card card card-lab-followups" class:collapsed={collapsed.labTests}>
        <h3 class="card-header-heading">
          <button type="button" class="card-header" onclick={() => toggle('labTests')} aria-expanded={!collapsed.labTests} aria-controls="lab-followups-body">
            <span class="card-header-title">🔬 Lab &amp; screening follow-ups ({plan.labTests.length})</span>
            <span class="chevron">{collapsed.labTests ? '▶' : '▼'}</span>
          </button>
        </h3>
        {#if collapsed.labTests}
          <div id="lab-followups-body" hidden aria-hidden="true"></div>
        {:else}
          <div class="card-body lab-body" id="lab-followups-body">
            <p class="section-hint">Grouped by priority</p>
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
                              <p class="lab-chip-purpose">{lt.purpose}</p>
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
            <details class="lab-request-details">
              <summary>Clinician request list</summary>
              <div class="lab-request-content">
                <p>Copy these DNA-linked follow-ups by clinical question.</p>
                <pre>{labRequestListText}</pre>
                <button class="btn btn-xs btn-secondary" type="button" onclick={copyLabRequestList}>
                  {labRequestCopied ? 'Copied' : 'Copy request list'}
                </button>
              </div>
            </details>
          </div>
        {/if}
      </div>
    {/if}
  </div>
    </section>
    {/if}
  </div>

  {#if healthAreaSections.length > 0}
    <nav class="health-area-index summary-card card" aria-labelledby="health-area-index-title">
      <div class="health-area-index-heading">
        <div>
          <span class="section-kicker">Deep dive</span>
          <h3 id="health-area-index-title">Explore all health areas</h3>
          <p>Jump to a health area for complete findings, evidence, and technical details.</p>
        </div>
        <span class="health-area-index-count">{healthAreaSections.length} areas</span>
      </div>
      <ul class="health-area-index-list">
        {#each healthAreaSections as section (section.name)}
          <li>
            <a href={'#' + sectionAnchorId(section.name)} onclick={() => onJumpToSection?.(section.name)}>
              <span>{section.name}</span>
              <span class="health-area-index-markers">{section.markers.length} markers</span>
            </a>
          </li>
        {/each}
      </ul>
    </nav>
  {/if}
</div>

<style>
  .dashboard-v2 {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1.5rem;
    width: 100%;
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
  }

  .guidance-flow {
    display: flex;
    flex-direction: column;
    gap: 1.75rem;
    width: 100%;
    min-width: 0;
  }

  .guidance-group {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    width: min(100%, var(--report-dashboard-surface-width));
    margin-inline: auto;
    min-width: 0;
    scroll-margin-top: 1.5rem;
  }

  .guidance-group-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding: 0 0.25rem;
  }

  .guidance-group-heading h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
    line-height: 1.25;
  }

  .guidance-group-heading p {
    max-width: 48rem;
    margin: 0.25rem 0 0;
    color: var(--text-secondary);
    font-size: 0.76rem;
    line-height: 1.4;
  }

  .guidance-index {
    display: grid;
    grid-template-columns: minmax(12rem, 0.7fr) minmax(0, 2fr);
    align-items: center;
    gap: 1rem;
    width: min(100%, var(--report-dashboard-surface-width));
    margin: 0 auto;
    padding: 0.7rem 0.85rem;
    border-top: 1px solid var(--border-color);
    border-bottom: 1px solid var(--border-color);
    box-sizing: border-box;
  }

  .guidance-index-heading {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
    min-width: 0;
  }

  .guidance-index-heading h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 0.9rem;
    line-height: 1.25;
  }

  .guidance-index-heading > span {
    flex: 0 0 auto;
    color: var(--text-muted);
    font-size: 0.68rem;
    white-space: nowrap;
  }

  .guidance-index-links {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.45rem;
    min-width: 0;
  }

  .guidance-index-links a {
    display: flex;
    min-width: 0;
    min-height: 2.35rem;
    align-items: center;
    justify-content: space-between;
    gap: 0.35rem;
    padding: 0.45rem 0.55rem;
    border: 1px solid var(--border-color);
    border-radius: 0.45rem;
    background: var(--surface-subtle);
    color: var(--text-primary);
    font-size: 0.68rem;
    font-weight: 700;
    line-height: 1.25;
    text-decoration: none;
  }

  .guidance-index-links a:hover,
  .guidance-index-links a:focus-visible {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .guidance-index-links a:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .guidance-index-links a:first-child {
    border-left: 2px solid var(--status-info-border);
  }

  .guidance-index-links a:nth-child(2) {
    border-left: 2px solid var(--status-success-border);
  }

  .guidance-index-links a:nth-child(3) {
    border-left: 2px solid var(--status-accent-soft-border);
  }

  .guidance-index-links a:nth-child(4) {
    border-left: 2px solid var(--status-warning-border);
  }

  .guidance-index-links a > span {
    flex: 0 0 auto;
    color: var(--accent);
    font-size: 0.8rem;
  }

  .action-queue {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
    background: var(--surface-raised);
    width: min(100%, var(--report-dashboard-surface-width));
    margin-inline: auto;
  }

  .action-queue-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding-bottom: 0.75rem;
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
    margin: 0.3rem 0 0;
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
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.65rem;
    width: 100%;
    margin-inline: auto;
    padding-top: 0.75rem;
    box-sizing: border-box;
  }

  .action-queue-column {
    display: grid;
    align-content: start;
    gap: 0.65rem;
    min-width: 0;
  }

  .action-queue-item {
    display: grid;
    grid-template-columns: 2.25rem minmax(0, 1fr);
    align-items: start;
    gap: 0.7rem;
    padding: 0.75rem 0.85rem;
    border: 1px solid var(--border-color);
    border-left: 3px solid var(--accent);
    border-radius: 0.65rem;
    background: var(--surface-subtle);
  }

  .action-queue-item[data-concern="high"] {
    border-left-color: var(--status-danger-border);
  }

  .action-queue-item[data-concern="moderate"] {
    border-left-color: var(--status-warning-border);
  }

  .action-queue-item[data-concern="low"] {
    border-left-color: var(--status-caution-border);
  }

  .action-queue-rank {
    display: inline-flex;
    width: 2.1rem;
    height: 2.1rem;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--status-accent-soft-border);
    border-radius: 50%;
    background: var(--status-accent-bg);
    color: var(--status-accent-soft-text);
    font-size: 0.85rem;
    font-weight: 800;
    line-height: 1;
  }

  .action-queue-item-content {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    min-width: 0;
  }

  .action-queue-item-top {
    grid-column: 1 / -1;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .action-queue-concern {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 0.3rem;
    color: var(--text-secondary);
    font-size: 0.62rem;
    font-weight: 700;
    line-height: 1.2;
    white-space: nowrap;
  }

  .action-queue-concern.high { color: var(--status-danger-strong-text); }
  .action-queue-concern.moderate { color: var(--status-warning-text); }
  .action-queue-concern.low { color: var(--status-caution-text); }

  .concern-dot {
    display: inline-block;
    width: 0.48rem;
    height: 0.48rem;
    border-radius: 50%;
    background: currentColor;
  }

  .action-queue-item h4 {
    margin: 0;
    color: var(--text-primary);
    font-size: 0.9rem;
    line-height: 1.3;
  }

  .action-queue-context {
    display: block;
    margin-top: 0.2rem;
    color: var(--text-secondary);
    font-size: 0.7rem;
  }

  .action-queue-item-content > p {
    max-width: none;
    margin: 0.35rem 0 0;
    color: var(--text-primary);
    font-size: 0.76rem;
    line-height: 1.4;
  }

  .action-queue-next {
    align-self: stretch;
    max-width: none;
    margin-top: 0.2rem;
    padding: 0.45rem 0.55rem;
    border-left: 2px solid var(--accent);
    border-radius: 0 0.4rem 0.4rem 0;
    background: var(--surface-card);
    color: var(--text-primary);
    font-size: 0.7rem;
    line-height: 1.35;
  }

  .action-queue-next strong {
    margin-right: 0.25rem;
    color: var(--text-primary);
  }

  .action-queue-item .jump-btn {
    margin-top: 0.1rem;
    padding-inline: 0;
    grid-column: 2;
    justify-self: start;
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

  .health-area-index {
    padding: 1rem;
    width: min(100%, var(--report-dashboard-surface-width));
    margin-inline: auto;
    box-sizing: border-box;
  }

  .health-area-index-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid var(--border-color);
  }

  .health-area-index-heading h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .health-area-index-count {
    flex: 0 0 auto;
    color: var(--text-secondary);
    font-size: 0.7rem;
    font-weight: 700;
  }

  .health-area-index-list {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 15rem), 1fr));
    gap: 0.5rem;
    margin: 0.75rem 0 0;
    padding: 0;
    list-style: none;
  }

  .health-area-index-list a {
    display: flex;
    min-width: 0;
    min-height: 44px;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.65rem;
    box-sizing: border-box;
    padding: 0.55rem 0.65rem;
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    background: var(--surface-subtle);
    color: var(--text-primary);
    font-size: 0.75rem;
    line-height: 1.35;
    text-decoration: none;
  }

  .health-area-index-list a:hover,
  .health-area-index-list a:focus-visible {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .health-area-index-list a:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .health-area-index-list a > span:first-child {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .health-area-index-markers {
    flex: 0 0 auto;
    color: var(--text-secondary);
    font-size: 0.65rem;
    white-space: nowrap;
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

  .allergy-card {
    width: min(100%, var(--report-dashboard-surface-width));
    margin-inline: auto;
    border-color: color-mix(in srgb, var(--status-info-border) 70%, var(--border-color));
    background: var(--surface-raised);
  }

  .allergy-map-body {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .allergy-map-intro,
  .allergy-section-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
  }

  .allergy-map-intro {
    padding-bottom: 0.85rem;
    border-bottom: 1px solid var(--border-color);
  }

  .allergy-map-intro h3,
  .allergy-section-heading h4 {
    margin: 0;
    color: var(--text-primary);
  }

  .allergy-map-intro h3 {
    font-size: 1rem;
  }

  .allergy-map-intro p,
  .allergy-section-heading span,
  .allergy-map-footer,
  .allergy-empty {
    margin: 0.3rem 0 0;
    color: var(--text-secondary);
    font-size: 0.74rem;
    line-height: 1.45;
  }

  .allergy-signal-count,
  .allergy-section-count {
    flex: 0 0 auto;
    padding: 0.3rem 0.5rem;
    border: 1px solid var(--status-info-border);
    border-radius: 999px;
    color: var(--status-info-text);
    font-size: 0.66rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .allergy-dna-section {
    min-width: 0;
  }

  .allergy-section-heading {
    margin-bottom: 0.6rem;
  }

  .allergy-section-heading h4 {
    font-size: 0.86rem;
  }

  .allergy-context-grid,
  .allergy-medication-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.6rem;
  }

  .insight-tile {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 0.45rem;
    box-sizing: border-box;
    padding: 0.72rem 0.78rem;
    border: 1px solid var(--border-color);
    border-radius: 0.55rem;
    background: var(--surface-subtle);
  }

  .insight-tile-info { border-left: 3px solid var(--status-info-border); }
  .insight-tile-consider,
  .insight-tile-favor { border-left: 3px solid var(--status-success-border); }
  .insight-tile-review { border-left: 3px solid var(--status-warning-border); }
  .insight-tile-alert { border-left: 3px solid var(--status-danger-border); }
  .insight-tile-avoid { border-left: 3px solid var(--status-danger-border); }

  .insight-tile-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.65rem;
  }

  .insight-tile-title {
    min-width: 0;
    margin: 0;
    color: var(--text-primary);
    font-size: 0.8rem;
    line-height: 1.3;
    overflow-wrap: anywhere;
  }

  .insight-tile-eyebrow,
  .insight-tile-meta {
    color: var(--text-muted);
    font-size: 0.62rem;
    line-height: 1.3;
  }

  .insight-tile-eyebrow {
    display: block;
    margin-top: 0.12rem;
    color: var(--status-info-text);
    font-weight: 700;
  }

  .insight-tile-count {
    flex: 0 0 auto;
    padding: 0.16rem 0.34rem;
    border: 1px solid var(--border-color);
    border-radius: 999px;
    color: var(--text-muted);
    font-size: 0.58rem;
    font-weight: 800;
    line-height: 1.2;
    white-space: nowrap;
  }

  .insight-tile-copy,
  .insight-tile-examples,
  .insight-tile-tags,
  .insight-tile-relevance {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.68rem;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .insight-tile-examples strong { color: var(--text-primary); }

  .insight-tile-relevance {
    padding: 0.42rem 0.5rem;
    border-radius: 0.38rem;
    background: var(--surface-raised);
    color: var(--text-secondary);
  }

  .insight-tile-relevance strong { color: var(--text-primary); }

  .insight-tile-tags {
    color: var(--status-warning-text);
    font-weight: 700;
  }

  .insight-tile-footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.35rem;
    padding-top: 0.4rem;
    border-top: 1px solid var(--border-color);
  }

  .insight-tile-link {
    margin: 0;
    padding: 0;
  }

  .insight-tile-indicator {
    display: inline-flex;
    flex: 0 0 auto;
    width: 1.05rem;
    height: 1.05rem;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--status-success-border);
    border-radius: 50%;
    background: var(--status-success-bg);
    color: var(--status-success-text);
    font-size: 0.72rem;
    font-weight: 800;
    line-height: 1;
  }

  .insight-tile-avoid .insight-tile-indicator {
    border-color: var(--status-danger-border);
    background: var(--status-danger-bg);
    color: var(--status-danger-strong-text);
  }

  .insight-tile-text {
    min-width: 0;
    color: var(--text-primary);
    font-size: 0.68rem;
    line-height: 1.3;
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .insight-tile-content {
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
    min-width: 0;
  }

  .insight-tile-meta {
    color: var(--text-muted);
    font-size: 0.58rem;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  .recommendation-provenance {
    margin-top: 0.22rem;
    color: var(--text-muted);
    font-size: 0.58rem;
    line-height: 1.3;
  }

  .recommendation-provenance summary {
    display: inline-block;
    min-height: 1.35rem;
    color: var(--status-info-text);
    cursor: pointer;
    font-weight: 700;
  }

  .recommendation-provenance summary:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: 3px;
  }

  .recommendation-provenance > span,
  .activity-provenance-list {
    display: block;
    margin-top: 0.18rem;
    overflow-wrap: anywhere;
  }

  .activity-provenance-details {
    margin-top: 0.55rem;
    padding-top: 0.35rem;
    border-top: 1px solid var(--border-color);
  }

  .activity-provenance-list {
    display: grid;
    gap: 0.2rem;
  }

  .allergy-medication-section {
    margin-top: 1rem;
    padding-top: 0.9rem;
    border-top: 1px solid var(--border-color);
  }

  .allergy-map-footer {
    padding-top: 0.7rem;
    border-top: 1px solid var(--border-color);
    font-style: italic;
  }

  @media (max-width: 760px) {
    .allergy-context-grid,
    .allergy-medication-grid {
      grid-template-columns: 1fr;
    }
  }

  .grid-layout {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    width: min(100%, var(--report-dashboard-surface-width));
    margin-inline: auto;
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
  }

  .clinical-guidance-grid {
    display: grid;
    grid-template-columns: minmax(20rem, 0.85fr) minmax(0, 1.15fr);
    align-items: start;
  }

  .action-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
    align-items: start;
  }

  .card-lab-followups {
    width: 100%;
    max-width: 100%;
    box-sizing: border-box;
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

  .lab-chip-purpose {
    margin: 0.45rem 0 0 0.15rem;
    color: var(--text-primary);
    font-size: 0.68rem;
    line-height: 1.35;
    max-width: 42rem;
  }

  .lab-request-details {
    margin-top: 0.85rem;
    border-top: 1px solid var(--border-color);
    padding-top: 0.75rem;
  }

  .lab-request-details > summary {
    cursor: pointer;
    color: var(--accent-primary);
    font-size: 0.78rem;
    font-weight: 700;
  }

  .lab-request-content {
    display: grid;
    gap: 0.55rem;
    margin-top: 0.65rem;
    max-width: 52rem;
  }

  .lab-request-content p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.74rem;
  }

  .lab-request-content pre {
    max-height: 14rem;
    overflow: auto;
    margin: 0;
    padding: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 0.55rem;
    background: var(--surface-subtle);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.74rem;
    line-height: 1.45;
    white-space: pre-wrap;
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
    box-sizing: border-box;
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
  .card-header-heading {
    margin: 0;
  }
  .card-header-heading .card-header {
    width: 100%;
  }
  .card-header:hover {
    background: var(--surface-subtle);
  }
  .card-header:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .card-header-title {
    min-width: 0;
    margin: 0;
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: var(--text-primary);
    overflow-wrap: anywhere;
  }
  .chevron {
    flex: 0 0 auto;
    font-size: 0.75rem;
    opacity: 0.6;
  }

  .card-body {
    padding: 0.85rem 1rem;
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
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
    align-items: start;
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

  .dietary-profile-list span {
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .dietary-profile-list strong,
  .dietary-resource-rules > summary {
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

  .dietary-suppressed-suggestions > summary {
    color: var(--status-warning-text);
  }

  .diet-column {
    min-width: 0;
  }

  .diet-column-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.45rem;
  }

  .diet-column h4 {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 0.3rem;
    margin: 0;
    font-size: 0.75rem;
    font-weight: 800;
    line-height: 1.25;
  }

  .diet-column.favor h4 { color: var(--status-success-text); }
  .diet-column.avoid h4 { color: var(--status-danger-strong-text); }

  .diet-column-count {
    flex: 0 0 auto;
    min-width: 1.35rem;
    padding: 0.12rem 0.35rem;
    border: 1px solid var(--border-color);
    border-radius: 999px;
    color: var(--text-muted);
    font-size: 0.6rem;
    font-weight: 800;
    line-height: 1.2;
    text-align: center;
  }

  .dietary-items {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.45rem;
    min-width: 0;
  }

  .diet-column.avoid .dietary-items {
    grid-template-columns: 1fr;
  }

  .dietary-item {
    align-items: flex-start;
    flex-direction: row;
  }
  .guidance-details {
    margin-top: 0.55rem;
    min-width: 0;
  }
  .guidance-details > summary {
    display: inline-flex;
    min-height: 32px;
    align-items: center;
    color: var(--status-info-text);
    cursor: pointer;
    font-size: 0.68rem;
    font-weight: 700;
  }
  .guidance-details > summary:hover,
  .guidance-details > summary:focus-visible {
    color: var(--text-primary);
  }
  .guidance-details > summary:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: 3px;
  }
  .guidance-details-body {
    min-width: 0;
  }
  .supplements-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-width: 0;
    max-width: 100%;
  }
  .supplement-columns {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.9rem;
    min-width: 0;
  }
  .supplement-column {
    min-width: 0;
  }
  .supplement-column h4 {
    margin: 0 0 0.45rem;
    color: var(--status-success-text);
    font-size: 0.75rem;
  }
  .supplement-column.avoid h4 {
    color: var(--status-danger-strong-text);
  }
  .supplement-item {
    flex-direction: row;
    align-items: flex-start;
  }

  .supp-name {
    color: var(--status-accent-soft-text);
    font-size: 0.75rem;
    font-weight: 800;
    line-height: 1.3;
    overflow-wrap: anywhere;
  }

  .supplement-column.avoid .supp-name { color: var(--status-danger-strong-text); }

  .supp-reason {
    display: block;
    min-width: 0;
    max-width: 100%;
    margin-top: 0.1rem;
    color: var(--text-secondary);
    font-size: 0.68rem;
    line-height: 1.35;
    overflow-wrap: anywhere;
    word-break: break-word;
  }
  .supplement-safety {
    margin-top: 0.85rem;
    padding-top: 0.65rem;
    border-top: 1px solid var(--border-color);
  }
  .supplement-safety-details {
    margin-top: 0.75rem;
    border-top: 1px solid var(--border-color);
    padding-top: 0.45rem;
  }
  .supplement-safety-details > summary {
    min-height: 44px;
    display: flex;
    align-items: center;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.72rem;
    font-weight: 700;
  }
  .supplement-safety-details[open] > summary {
    color: var(--text-primary);
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

  .activity-framework {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.55rem;
    margin-bottom: 0.75rem;
  }

  .activity-framework-item {
    min-width: 0;
    padding: 0.55rem 0.65rem;
    border: 1px solid var(--status-info-soft-border);
    border-top: 2px solid var(--status-info-text);
    border-radius: 0.45rem;
    background: var(--status-info-soft-bg);
  }

  .activity-framework-item > span {
    color: var(--status-info-soft-text);
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .activity-framework-item p {
    margin: 0.25rem 0 0;
    color: var(--text-primary);
    font-size: 0.72rem;
    line-height: 1.35;
  }

  .activity-domain-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.65rem;
  }

  .activity-domain {
    min-width: 0;
    margin-top: 0.65rem;
    padding: 0.55rem 0.65rem;
    border-left: 2px solid var(--status-info-soft-border);
    background: var(--status-info-soft-bg);
    border-radius: 4px;
  }

  .activity-domain-grid .activity-domain {
    margin-top: 0;
  }

  .activity-domain-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .activity-domain-heading strong {
    min-width: 0;
    color: var(--text-primary);
    font-size: 0.78rem;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  .activity-domain-label {
    flex: 0 0 auto;
    color: var(--text-muted);
    font-size: 0.6rem;
    white-space: nowrap;
  }
  .medication-pathways {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.6rem;
    margin-top: 0.65rem;
  }
  .medication-pathway {
    min-width: 0;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--status-info-soft-border);
    border-left: 3px solid var(--status-info-text);
    border-radius: 4px;
    background: var(--surface-subtle);
  }
  .medication-pathway-heading {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.25rem 0.55rem;
  }
  .medication-pathway-heading strong {
    min-width: 0;
    color: var(--text-primary);
    font-size: 0.78rem;
    overflow-wrap: anywhere;
  }
  .medication-pathway-heading span {
    color: var(--text-muted);
    font-size: 0.62rem;
  }
  .medication-pathway p,
  .medication-empty {
    margin: 0.3rem 0 0;
    color: var(--text-secondary);
    font-size: 0.7rem;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .medication-pathway-footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.35rem;
    margin-top: 0.5rem;
    padding-top: 0.45rem;
    border-top: 1px solid var(--border-color);
    color: var(--text-muted);
    font-size: 0.62rem;
  }

  .medication-pathway-footer .btn {
    margin: 0;
    padding: 0;
  }
  .activity-context {
    display: block;
    margin-top: 0.15rem;
    font-size: 0.68rem;
    opacity: 0.72;
  }
  .activity-provenance {
    display: block;
    margin-top: 0.18rem;
    color: var(--status-info-soft-text);
    font-size: 0.62rem;
    letter-spacing: 0.02em;
  }
  .activity-columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-top: 0.45rem;
  }
  .activity-columns h4 {
    margin: 0;
    font-size: 0.72rem;
    color: var(--status-info-soft-text);
  }

  .activity-column-build,
  .activity-column-watch {
    min-width: 0;
  }

  .activity-column-watch {
    padding-left: 0.7rem;
    border-left: 1px solid var(--border-color);
  }

  .activity-columns .activity-verify-heading {
    margin-top: 0.65rem;
  }

  .activity-stop-compact {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: baseline;
    gap: 0.65rem;
  }

  .activity-stop-compact > span {
    color: var(--text-secondary);
    font-size: 0.68rem;
    line-height: 1.4;
  }
  .activity-stop-list {
    margin-top: 0.85rem;
    padding: 0.55rem 0.65rem;
    border: 1px solid var(--status-danger-border);
    border-radius: 4px;
    background: var(--status-danger-bg);
  }

  @media (max-width: 1100px) {
    .guidance-index {
      grid-template-columns: 1fr;
    }

    .guidance-index-links {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .clinical-guidance-grid {
      grid-template-columns: 1fr;
    }

    .action-queue-list {
      grid-template-columns: minmax(0, 1fr);
    }

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
    .medication-pathways {
      grid-template-columns: 1fr;
    }

    .activity-domain-grid {
      grid-template-columns: 1fr;
    }

    .activity-column-watch {
      padding-left: 0;
      border-left: 0;
    }

    .activity-stop-compact {
      grid-template-columns: 1fr;
      gap: 0.25rem;
    }
  }

  @media (min-width: 1101px) and (max-width: 1400px) {
    .guidance-index-links {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .action-queue-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 760px) {
    .guidance-index-links {
      grid-template-columns: 1fr;
    }

    .dietary-items {
      grid-template-columns: 1fr;
    }
  }
</style>
