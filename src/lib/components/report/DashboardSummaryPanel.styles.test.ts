import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./DashboardSummaryPanel.svelte', import.meta.url), 'utf8');
const styleBlock = source.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? '';
const theme = readFileSync(new URL('../../styles/theme.css', import.meta.url), 'utf8');

describe('DashboardSummaryPanel semantic styling', () => {
  it('keeps report-panel colors in shared semantic tokens', () => {
    expect(styleBlock).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(styleBlock).not.toMatch(/\brgba?\(/i);
  });

  it('retains explicit semantic states for the primary guidance surfaces', () => {
    const requiredTokens = [
      '--status-warning-bg',
      '--status-info-bg',
      '--status-success-bg',
      '--status-danger-bg',
      '--status-accent-bg',
      '--surface-card',
      '--surface-subtle',
      '--border-color',
      '--text-primary',
      '--text-secondary',
    ];

    for (const token of requiredTokens) {
      expect(styleBlock).toContain(`var(${token})`);
    }
  });

  it('keeps repeated guidance boundaries compact and keeps personal context out of the report', () => {
    expect(source).toContain('Food choices linked to the pathways found in this profile.');
    expect(source).toContain('Options linked to the pathways found in this profile.');
    expect(source).toContain('DNA-informed starting points for training, recovery, and self-tracking.');
    expect(source).toContain('DNA-linked medication and treatment topics found in this report.');
    expect(source).toContain('Medication pathways');
    expect(source).toContain('plan.medicationPathways');
    expect(source).toContain('Grouped by priority');
    expect(source).not.toContain('Conditional prompts, not permanent food rules.');
    expect(source).not.toContain('Review interactions and health context before use.');
    expect(source).not.toContain('Planning prompts for training and recovery — not activity clearance.');
    expect(source).not.toContain('Compare this with current medications and past responses.');
    expect(source).not.toContain('Grouped by priority for clinician discussion.');
    expect(source).toContain('The most actionable DNA-linked signals in this report.');
    expect(source).toContain('{Math.min(plan.topFindings.length, 9)} shown');
    expect(source).not.toContain('A genotype match is not a permanent food restriction;');
    expect(source).not.toContain('This selection is self-reported, stored per DNA profile, and is never inferred');
    expect(source).not.toContain('Every supplement item is a discussion prompt, not a prescription.');
    expect(source).not.toContain('Genotype may provide weak context for training questions.');
    expect(source).not.toContain('Raw consumer DNA is not a complete clinical PGx result');
    expect(source).not.toContain('DNA cannot measure current hormones or diagnose a condition or medication response.');
    expect(source).not.toContain('Ask for / record');
    expect(source).not.toContain('Do not do from raw DNA');
    expect(source).not.toContain('medication-columns');
    expect(source).not.toContain('Grouped by priority for clinician discussion; seek care promptly for acute symptoms.');
    expect(source).not.toContain('{Math.min(plan.topFindings.length, 5)} of 5');
    expect(source).toContain("{#if presentationMode !== 'simple' && plan.safetyNotes.length > 0}");
    expect(source).toContain('<summary>Safety details</summary>');
    expect(source).not.toContain('Profile context only; kept separate from DNA findings.');
    expect(source).not.toContain('Optional reproductive & hormone context');
    expect(source).not.toContain('Optional cycle and hormone context');
    expect(source).not.toContain('personalSafetyContext');
    expect(source).not.toContain('CycleDiaryEditor');
    expect(source).not.toContain('ReproductiveContextEditor');
  });

  it('compacts repeated supplement review prefixes without changing the underlying plan', () => {
    expect(source).toContain('getCompactSupplementName');
    expect(source).toContain('getCompactSupplementReason');
    expect(source).toContain('{getCompactSupplementName(s.name)}');
    expect(source).toContain('{getCompactSupplementReason(s.reason)}');
  });

  it('keeps Simple dietary lists focused on authored food content', () => {
    expect(source).toContain('getCompactGuidanceText');
    expect(source).toContain('{#each plan.diet.favorItems as item (item.recommendation_id)}');
    expect(source).toContain('{#each plan.diet.avoidItems as item (item.recommendation_id)}');
    expect(source).toContain('{#each plan.supplements as s (`${s.name}:${s.reason}`)}');
    expect(source).toContain('{#each plan.supplementAvoid as s (s.name)}');
    expect(source).toContain('plan.supplementAvoid');
    expect(source).not.toContain('getCompactDietNotes');
    expect(source).not.toContain('diet-notes');
    expect(source).not.toContain('compact-notes');
    expect(source).toContain('<h4><span aria-hidden="true">👍</span> Food ideas</h4>');
    expect(source).toContain('<h4><span aria-hidden="true">👎</span> Foods to limit</h4>');
    expect(source).toContain('<h4>👍 Consider</h4>');
    expect(source).toContain('<h4>👎 Avoid / confirm first</h4>');
    expect(source).toContain('class="dietary-items"');
    expect(source).toContain('class="insight-tile insight-tile-favor dietary-item recommendation-row"');
    expect(source).toContain('class="insight-tile insight-tile-avoid dietary-item recommendation-row"');
    expect(source).toContain('class="recommendation-basis"');
    expect(source).toContain('recommendationBasis(item)');
    expect(source).toContain('linked ${markerCount === 1 ? \'marker\' : \'markers\'}');
    expect(source).toContain('recommendationGenes(item)');
    expect(source).toContain('class="recommendation-genes"');
    expect(source).toContain('recommendationGenes(s)');
    expect(source).toContain('Evidence details');
    expect(source).not.toContain('DNA-linked · {item.basis_genes.join');
    expect(source).not.toContain('Why this appears');
    expect(source).toContain('class="insight-tile-text dietary-item-text"');
    expect(source).toContain('<summary>Food safety checks ({plan.foodSafety.relevantRules.length})</summary>');
  });

  it('renders a compact DNA-linked allergy map without the generic exposure checklist', () => {
    expect(source).toContain('Allergy &amp; sensitivity map');
    expect(source).toContain('Matched pathways');
    expect(source).toContain('Medication safety routes');
    expect(source).toContain('allergySensitivityCatalog.display.dna_intro');
    expect(source).toContain('context.relevance');
    expect(source).toContain('context.matched_genes');
    expect(source).toContain('matched DNA {context.matched_marker_link_ids.length === 1 ?');
    expect(source).not.toContain('Exposure history');
    expect(source).not.toContain('plan.allergy.exposureChecklists');
    expect(source).not.toContain('allergySensitivityCatalog.display.exposure_intro');
    expect(source).not.toContain('allergy-next-line');
    expect(styleBlock).not.toContain('.allergy-map-grid');
    expect(styleBlock).not.toContain('.allergy-exposure-grid');
    expect(styleBlock).toContain('.insight-tile-relevance');
  });

  it('renders complete guidance lists without Show more controls', () => {
    expect(source).toContain('simpleFramework');
    expect(source).toContain('activity-framework');
    expect(source).toContain('activity-domain-grid');
    expect(source).toContain('Build around');
    expect(source).toContain('Watch for');
    expect(source).toContain('Verify when relevant');
    expect(source).toContain('Pause and get prompt care for');
    expect(styleBlock).toContain('grid-template-columns: repeat(2, minmax(0, 1fr));');
    expect(source).toContain('{#each plan.activity.principles as principle (principle)}');
    expect(source).toContain("function activityItems(domain: ActivityDomain, kind: ActivityListKey): string[]");
    expect(source).toContain("const compactKey = kind === 'favor'");
    expect(source).toContain("activityItems(domain, 'favor')");
    expect(source).toContain("activityItems(domain, 'confirm_with')");
    expect(source).not.toContain('Show 1 more');
    expect(source).not.toContain('guidance-more');
    expect(source).not.toContain('previewGuidance');
    expect(source).not.toContain('remainingGuidance');
    expect(styleBlock).toContain('.guidance-details > summary');
    expect(styleBlock).toContain('min-height: 32px;');
  });

  it('keeps repeated action steps concise instead of repeating warning copy', () => {
    expect(source).toContain('getSimpleFindingCopy');
    expect(source).toContain('return getSimpleFindingCopy(marker, translation).review_action;');
    expect(source).toContain('<strong class="action-queue-next-label">Next</strong>');
    expect(source).not.toContain('<strong>Next helpful step:</strong>');
    expect(source).not.toContain('Ask a qualified clinician whether medical-grade confirmation');
    expect(source).toContain('copy.signal, copy.why_it_matters');
    expect(source).toContain('data-priority={item.rank}');
  });

  it('keeps collapsible guidance titles as real headings outside button descendants', () => {
    expect(source).toContain('<h3 class="card-header-heading">');
    expect(source).toContain('<span class="card-header-title">🥗 Food ideas</span>');
    expect(source).not.toContain('class="card-header-title" role="heading" aria-level="3"');
    expect(styleBlock).toContain('.card-header-heading {');
  });

  it('bounds long guidance titles beside the collapse control', () => {
    const title = styleBlock.match(/\.card-header-title \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const chevron = styleBlock.match(/\.chevron \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(title).toContain('min-width: 0;');
    expect(title).toContain('overflow-wrap: anywhere;');
    expect(chevron).toContain('flex: 0 0 auto;');
  });

  it('provides a compact health-area index that targets report section headers', () => {
    expect(source).toContain('Explore all health areas');
    expect(source).toContain('healthAreaSections');
    expect(source).toContain('<details class="health-area-index summary-card card">');
    expect(source).toContain('<summary class="health-area-index-heading">');
    expect(source).toContain('<nav aria-labelledby="health-area-index-title">');
    expect(source).not.toContain('<details class="health-area-index summary-card card" open');
    expect(source).toContain("href={'#' + sectionAnchorId(section.name)}");
    expect(source).toContain('onJumpToSection?.(section.name)');
    expect(source).toContain('{section.markers.length} markers');
    expect(source).toContain('Jump to a health area for complete findings, evidence, and technical details.');
    expect(styleBlock).toContain('.health-area-index[open] > .health-area-index-heading::before');
    expect(theme).toContain('.main-content .dashboard-v2 .health-area-index[open] > .health-area-index-heading::before');

    const index = styleBlock.match(/\.health-area-index \{[\s\S]*?\.actionability-safety/)?.[0] ?? '';
    expect(index).toContain('grid-template-columns: repeat(auto-fit');
    expect(index).toContain('min-height: 44px;');
    expect(index).toContain('overflow-wrap: anywhere;');
    expect(index).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('scopes collapsed dashboard preferences per profile and keeps review/labs open', () => {
    expect(source).toContain('sampleId: number;');
    expect(source).toContain('let loadedCollapseProfileId = $state<number | null>(null);');
    expect(source).toContain('genomics_dashboard_collapsed_v3_${profileId}');
    expect(source).toContain('labTests: false');
    expect(source).toContain('topFindings: false');
    expect(source).toContain('const defaults = {');
    expect(source).toContain('loadedCollapseProfileId === sampleId');
    expect(source).not.toContain("localStorage.getItem('genomics_dashboard_collapsed')");
  });

  it('visually segregates marker association references from narrative copy', () => {
    expect(source).toContain('action-queue-marker-ref');
    expect(styleBlock).toContain('.action-queue-marker-ref');
    expect(styleBlock).toContain('var(--report-marker-ref-text)');
    expect(styleBlock).toContain('var(--report-marker-ref-bg)');
    expect(theme).toContain('--report-marker-ref-text: #e7c27d;');
    expect(source).toContain('lab-spotlight');
    expect(source).toContain('Suggested lab');
  });

  it('keeps personal-context controls out of the report surface', () => {
    expect(source).not.toContain('geneticSex?: string;');
    expect(source).not.toContain('Suggested for this profile');
    expect(source).not.toContain('Other contexts — select if relevant');
    expect(source).not.toContain('reproductiveContextOptionIsSuggestedForGeneticSex');
  });

  it('organizes secondary guidance into a navigable desktop reading flow', () => {
    expect(source).toContain('class="guidance-index" aria-labelledby="guidance-index-title"');
    expect(source).toContain('<h3 id="guidance-index-title">Use your results</h3>');
    expect(source).toContain('href="#allergy-guidance-group"');
    expect(source).toContain('href="#nutrition-guidance-group"');
    expect(source).toContain('href="#training-guidance-group"');
    expect(source).toContain('href="#clinical-guidance-group"');
    expect(source).toContain('id="allergy-guidance-group"');
    expect(source).toContain('id="nutrition-guidance-group"');
    expect(source).toContain('id="training-guidance-group"');
    expect(source).toContain('id="clinical-guidance-group"');
    expect(source).toContain('let hasAllergyGuidance = $derived(');
    expect(source).toContain('let hasNutritionGuidance = $derived(');
    expect(source).toContain('let hasTrainingGuidance = $derived(');
    expect(source).toContain('let hasClinicalGuidance = $derived(');
    expect(styleBlock).toContain('.guidance-index {');
    expect(styleBlock).toContain('grid-template-columns: repeat(4, minmax(0, 1fr));');
    expect(styleBlock).toContain('.guidance-flow {');
    expect(styleBlock).toContain('.guidance-group {');
    expect(styleBlock).toContain('scroll-margin-top:');
  });

  it('uses two priority-board columns at laptop widths to protect card readability', () => {
    expect(theme).toContain('@media screen and (min-width: 1201px) and (max-width: 1400px)');
    expect(theme).toContain('.main-content .dashboard-v2 .action-queue-list {\n    grid-template-columns: repeat(2, minmax(0, 1fr));');
  });

  it('uses a single priority-board column and wrapping guidance links on narrow desktop', () => {
    expect(theme).toContain('@media screen and (min-width: 721px) and (max-width: 1100px)');
    expect(theme).toContain('.main-content .dashboard-v2 .action-queue-list {\n    grid-template-columns: 1fr;');
    expect(theme).toContain('overflow-wrap: anywhere;');
  });

  it('uses the wide desktop surface for clinical follow-up without forcing a single long stack', () => {
    expect(source).toContain('class="grid-layout clinical-guidance-grid"');
    expect(styleBlock).toContain('.clinical-guidance-grid {');
    expect(styleBlock).toContain('grid-template-columns: minmax(20rem, 0.85fr) minmax(0, 1.15fr);');
    expect(styleBlock).toContain('.clinical-guidance-grid {\n      grid-template-columns: 1fr;');
  });

  it('progressively discloses lower-priority follow-ups and keeps the list visually quiet', () => {
    expect(source).toContain('{#each plan.labGroups as group, groupIndex (group.label)}');
    expect(source).toContain('<details class="lab-tier-block" open={groupIndex === 0}>');
    expect(source).toContain('aria-label={`${group.tests.length} follow-ups`}');
    expect(styleBlock).toContain('.lab-tier-toggle:focus-visible');
    expect(styleBlock).toContain('.lab-tier-content {');
    expect(styleBlock).toContain('border-bottom: 1px solid var(--border-color);');
    expect(styleBlock).toContain('background: transparent;');
    expect(styleBlock).not.toContain('border-radius: 0.55rem;\n    background: var(--surface-card);');
    expect(theme).toContain('.main-content .dashboard-v2 .lab-tier-block {\n  display: block;');
    expect(theme).toContain('.main-content .dashboard-v2 .lab-chip {\n  min-width: 0;\n  min-height: 0;');
    expect(theme).toContain('background: transparent;');
  });

  it('sends lab links to the lab list and keeps medication gene references below their titles', () => {
    expect(source).toContain('href="#lab-followups-card"');
    expect(source).toContain('id="lab-followups-card"');
    expect(source).toContain('class="medication-pathway-genes"');
    expect(source).not.toContain('href="#clinical-guidance-group">Full lab list');
    expect(theme).toContain('.main-content .dashboard-v2 .medication-pathway-heading {\n  display: grid;');
    expect(theme).toContain('.main-content .dashboard-v2 .lab-chip-grid {\n  display: grid;');
    expect(source).toContain('Genetic counselor advised');
    expect(source).not.toContain('🧑‍⚕️');
  });

  it('keeps medication guidance focused on matched pathways without a warning block', () => {
    expect(source).toContain('hasMedicationGuidance');
    expect(source).toContain('class="medication-pathways"');
    expect(source).toContain('class="medication-pathway"');
    expect(source).toContain('pathway.matchedMarkerLinkIds');
    expect(source).toContain('View matching DNA');
    expect(styleBlock).toContain('.medication-pathways {');
    expect(styleBlock).toContain('.medication-pathway {');
    expect(source).not.toContain('pgx-readiness');
    expect(source).not.toContain('Coverage details');
  });

  it('allows long guidance text to wrap inside narrow cards', () => {
    const cardBody = styleBlock.match(/\.card-body \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const dietaryProfile = styleBlock.match(/\.dietary-profile-list span \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const insightTile = styleBlock.match(/\.insight-tile \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const supplementItem = styleBlock.match(/\.supplement-item \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const supplementReason = styleBlock.match(/\.supp-reason \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const supplementSafetyDetails = styleBlock.match(/\.supplement-safety-details > summary \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(cardBody).toContain('min-width: 0;');
    expect(dietaryProfile).toContain('overflow-wrap: anywhere;');
    expect(dietaryProfile).toContain('word-break: break-word;');
    expect(styleBlock).toContain('.dietary-items {');
    expect(styleBlock).toContain('grid-template-columns: repeat(2, minmax(0, 1fr));');
    expect(styleBlock).toContain('border-left: 2px solid var(--status-success-border);');
    expect(styleBlock).toContain('.recommendation-row {');
    expect(styleBlock).toContain('grid-template-columns: auto minmax(0, 1fr);');
    expect(styleBlock).toContain('.recommendation-basis {');
    expect(styleBlock).toContain('word-break: break-word;');
    expect(insightTile).toContain('box-sizing: border-box;');
    expect(supplementReason).toContain('display: block;');
    expect(supplementReason).toContain('overflow-wrap: anywhere;');
    expect(supplementSafetyDetails).toContain('min-height: 44px;');
  });

  it('bounds the optional context selector and dashboard containers', () => {
    const dashboard = styleBlock.match(/\.dashboard-v2 \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const grid = styleBlock.match(/\.grid-layout \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const summaryCard = styleBlock.match(/\.summary-card \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const labCard = styleBlock.match(/\.card-lab-followups \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(dashboard).toContain('box-sizing: border-box;');
    expect(grid).toContain('min-width: 0;');
    expect(summaryCard).toContain('box-sizing: border-box;');
    expect(labCard).toContain('max-width: 100%;');
  });

  it('uses a full-width three-column priority queue on wide desktop', () => {
    const actionQueueCard = styleBlock.match(/\.action-queue \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const actionQueue = styleBlock.match(/\.action-queue-list \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(actionQueueCard).toContain('width: min(100%, var(--report-dashboard-surface-width));');
    expect(actionQueueCard).toContain('margin-inline: auto;');
    expect(actionQueue).toContain('width: 100%;');
    expect(actionQueue).toContain('margin-inline: auto;');
    expect(actionQueue).toContain('box-sizing: border-box;');
    expect(source).toContain('<div class="action-queue-list">');
    expect(actionQueue).toContain('grid-template-columns: repeat(3, minmax(0, 1fr));');
    expect(source).toContain('action-queue-column');
    expect(source).toContain('columnIndex * 3 + rowIndex + 1');
    expect(source).toContain('data-concern={priorityTone(item.finding)}');
    expect(styleBlock).toContain('.action-queue-item[data-concern="high"]');
    expect(styleBlock).toContain('.action-queue-item[data-concern="moderate"]');
    expect(styleBlock).toContain('.action-queue-item[data-concern="low"]');
    expect(source).not.toContain('action-queue-priority');
  });

  it('shows condition-level DNA evidence as a compact, count-based review surface', () => {
    const conditionGrid = styleBlock.match(/\.condition-evidence-grid \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(source).toContain('Potential health patterns');
    expect(source).toContain('conditionCountLabel(summary)');
    expect(source).toContain('Counts show how many curated indicators matched in this report.');
    expect(source).toContain('conditionDiagnosticCapabilityLabel');
    expect(conditionGrid).toContain('grid-template-columns: repeat(3, minmax(0, 1fr));');
    expect(source).toContain('.condition-evidence-grid {\n      grid-template-columns: repeat(2, minmax(0, 1fr));');
    expect(source).toContain('.condition-evidence-grid {\n      grid-template-columns: 1fr;');
  });

  it('keeps database-linked conditions discoverable without adding a wall of cards or chips', () => {
    expect(source).toContain('<details class="catalog-associations summary-card card">');
    expect(source).toContain('Catalog-linked conditions, traits &amp; responses');
    expect(source).toContain('plan.catalogAssociations.slice(0, 6)');
    expect(source).toContain('association.association_is');
    expect(source).toContain('association.association_scope');
    expect(source).toContain('Exact allele match not verified');
    expect(styleBlock).toContain('.catalog-associations-list {');
    expect(styleBlock).not.toContain('.catalog-associations-list {\n    display: grid;\n    grid-template-columns:');
  });

  it('surfaces symptom-led mental-health support while separating it from DNA evidence', () => {
    expect(source).toContain('hasNeuropsychSection');
    expect(source).toContain('catalogAssociationsForTopic(plan.catalogAssociations, \'neuropsych\')');
    expect(source).toContain('No psychiatric score calculated');
    expect(source).toContain('A dopamine-pathway SNP is not evidence of low dopamine');
    expect(source).toContain('childhood history and whether difficulties appear in more than one setting');
    expect(source).toContain('No link here confirms or rules out a condition.');
    expect(source).toContain('call or text 988');
    expect(styleBlock).toContain('.brain-mood-context-grid {');
    expect(styleBlock).toContain('grid-template-columns: 1fr;');
  });

  it('uses the available desktop report pane without exceeding the shared surface cap', () => {
    const grid = styleBlock.match(/\.grid-layout \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(grid).toContain('width: min(100%, var(--report-dashboard-surface-width));');
    expect(grid).toContain('margin-inline: auto;');
  });

  it('uses the shared desktop dashboard width token for secondary surfaces', () => {
    expect(theme).toContain('--report-dashboard-surface-width: 80rem;');
    const healthAreaIndex = styleBlock.match(/\.health-area-index \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(healthAreaIndex).toContain('width: min(100%, var(--report-dashboard-surface-width));');
    expect(healthAreaIndex).toContain('margin-inline: auto;');
    expect(healthAreaIndex).toContain('box-sizing: border-box;');
  });

  it('keeps the native fallback composition bounded and visibly grouped', () => {
    expect(theme).toContain('.main-content .report-header');
    expect(theme).toContain('.main-content .dashboard-v2 .action-queue-item');
    expect(theme).toContain('.main-content .dashboard-v2 .condition-evidence-item');
    expect(theme).toContain('.main-content .sections-container .marker-card');
    expect(theme).toContain('width: min(100%, var(--report-dashboard-surface-width));');
    expect(theme).toContain('@media screen and (max-width: 720px)');
  });

  it('gives food recommendations room and themes their native-renderer controls', () => {
    expect(source).toContain('grid-template-columns: minmax(0, 1fr);');
    expect(theme).toContain('.main-content .dashboard-v2 .nutrition-action-row');
    expect(theme).toContain('.main-content .dashboard-v2 .recommendation-row');
    expect(theme).toContain('.main-content .dashboard-v2 .recommendation-link');
    expect(theme).toContain('-webkit-appearance: none;');
    expect(theme).toContain('background: var(--status-info-soft-bg);');
  });

  it('leaves context labels and stable context IDs to the Context workspace', () => {
    expect(source).not.toContain("const contextOptionLabels: Record<string, string>");
    expect(source).not.toContain("menstrual_cycle: 'Menstrual cycle / PMS'");
    expect(source).not.toContain("suspected_adenomyosis: 'Adenomyosis / heavy bleeding'");
    expect(source).not.toContain('<option value="">Not specified</option>');
    expect(source).not.toContain('contextOptionLabel(option.id, option.label)');
  });

  it('keeps guidance card aria-controls targets present while collapsed', () => {
    for (const id of [
      'dietary-alignment-body',
      'supplements-body',
      'activity-body',
      'medication-body',
      'lab-followups-body',
    ]) {
      expect(source).toContain(`id="${id}" hidden aria-hidden="true"`);
    }
  });
});
