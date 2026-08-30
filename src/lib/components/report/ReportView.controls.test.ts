import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./ReportView.svelte', import.meta.url), 'utf8');
const theme = readFileSync(new URL('../../styles/theme.css', import.meta.url), 'utf8');

describe('Simple-first report controls', () => {
  it('keeps advanced filters in a disclosure that starts closed in every reading mode', () => {
    expect(source).toContain('<div class="report-controls no-print">');
    expect(source).toContain('let reportFiltersOpen = $state(false);');
    expect(source).toContain('<details class="report-filter-details" bind:open={reportFiltersOpen}>');
    expect(source).toContain('<summary>Filters &amp; ordering</summary>');
  });

  it('keeps the reading-mode group available outside the advanced filter disclosure', () => {
    const detailsStart = source.indexOf('<details class="report-filter-details"');
    const detailsEnd = source.indexOf('</details>', detailsStart);
    const filterDetails = source.slice(detailsStart, detailsEnd);
    const modeGroupIndex = source.indexOf('<div class="mode-group"');

    expect(filterDetails).toContain('bind:checked={showBenign}');
    expect(filterDetails).toContain('bind:value={tierFilter}');
    expect(filterDetails).toContain('bind:value={sortBy}');
    expect(filterDetails).not.toContain('class="mode-group"');
    expect(modeGroupIndex).toBeLessThan(detailsStart);
  });

  it('starts the report reading flow with mode selection, then optional filters', () => {
    const modeGroupIndex = source.indexOf('<div class="mode-group"');
    const filterDetailsIndex = source.indexOf('<details class="report-filter-details"');
    expect(modeGroupIndex).toBeGreaterThan(-1);
    expect(filterDetailsIndex).toBeGreaterThan(modeGroupIndex);
    expect(theme).toContain('.mode-group {\n  margin-left: 0;\n}');
  });

  it('gives the disclosure a keyboard-sized target and preserves narrow mode controls', () => {
    const filterSummary = theme.match(/\.report-filter-details > summary \{[\s\S]*?\n\}/)?.[0] ?? '';
    const narrow = theme.match(/@media \(max-width: 720px\) \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(filterSummary).toContain('min-height: 44px;');
    expect(filterSummary).toContain('var(--surface-subtle)');
    expect(theme).toContain('.report-filter-details[open] .filter-bar');
    expect(narrow).toContain('.report-controls');
    expect(narrow).toContain('.report-controls .view-mode-buttons');
    expect(narrow).toContain('.report-controls .view-mode-btn');
  });

  it('routes section collapse changes through an explicit parent updater', () => {
    expect(source).toContain('function setSectionCollapsed(sectionName: string, isCollapsed: boolean)');
    expect(source).toContain('collapsed={collapsedSections[section.name]}');
    expect(source).toContain('onCollapsedChange={(isCollapsed) => setSectionCollapsed(section.name, isCollapsed)}');
    expect(source).not.toContain('bind:collapsed={collapsedSections[section.name]}');
  });

  it('owns and scopes section-collapse persistence per profile', () => {
    expect(source).toContain('function sectionCollapseStorageKey(sampleId: number, sectionName: string)');
    expect(source).toContain('return `section-collapsed-v2-${sampleId}-${sectionName}`;');
    expect(source).toContain('let loadedCollapseProfileId = $state<number | null>(null);');
    expect(source).toContain('const profileChanged = loadedCollapseProfileId !== profileId;');
    expect(source).toContain('persistSectionCollapsed(sectionName, isCollapsed);');
    expect(source).not.toContain('section-collapsed-${sec.name}');
  });

  it('keeps the Simple landing focused on the profile and action queue', () => {
    const reportHeaderIndex = source.indexOf('<ReportHeader');
    const summaryIndex = source.indexOf('<DashboardSummaryPanel');
    const heroIndex = source.indexOf('<header class="report-hero no-print">');

    expect(source).toContain('{#if presentationMode !== \'simple\'}\n    <header class="report-hero no-print">');
    expect(reportHeaderIndex).toBeGreaterThan(-1);
    expect(summaryIndex).toBeGreaterThan(reportHeaderIndex);
    expect(heroIndex).toBeGreaterThan(summaryIndex);
  });

  it('explains how to reveal Clinical tables when report sections are collapsed', () => {
    expect(source).toContain("let clinicalSectionsExpanded = $derived(");
    expect(source).toContain('Clinical view is ready.');
    expect(source).toContain('Expand a health area below, or use “Expand all” under Filters &amp; ordering');
    expect(source).toContain('class="clinical-empty-hint" role="status"');
  });

  it('shows Clinical data provenance once at the report level', () => {
    expect(source).toContain("let clinicalProvenanceExpanded = $state(false);");
    expect(source).toContain('{#if presentationMode === \'clinical\'}\n    <details class="clinical-provenance" bind:open={clinicalProvenanceExpanded}>');
    expect(source).toContain('<summary>About the data in Clinical view</summary>');
    expect(source).toContain('<strong>DNA array</strong> Genotype calls shown in the tables.');
    expect(source).toContain('<strong>Personal context</strong> Symptoms, medications, and goals are not DNA findings.');
    expect(source).toContain('const previousClinicalProvenanceExpanded = clinicalProvenanceExpanded;');
    expect(source).toContain('if (presentationMode === \'clinical\') clinicalProvenanceExpanded = true;');
  });
});
