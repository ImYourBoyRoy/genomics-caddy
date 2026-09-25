import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ReportView.svelte', import.meta.url), 'utf8');
const dashboardSource = readFileSync(new URL('./DashboardSummaryPanel.svelte', import.meta.url), 'utf8');
const helpSource = readFileSync(new URL('../common/HelpCenter.svelte', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../../../routes/+page.svelte', import.meta.url), 'utf8');
const theme = readFileSync(new URL('../../styles/theme.css', import.meta.url), 'utf8');
const catalogWarningStyles = theme.match(/\.catalog-warnings-banner \{[\s\S]*?\n\}/)?.[0] ?? '';

describe('ReportView Help Guide surface', () => {
  it('gives report generation a focused, theme-aware loading surface', () => {
    const loading = readFileSync(new URL('./ReportLoadingState.svelte', import.meta.url), 'utf8');
    const loadingCss = readFileSync(
      new URL('../../styles/components/report-loading.css', import.meta.url),
      'utf8',
    );
    expect(source).toContain("import ReportLoadingState from './ReportLoadingState.svelte'");
    expect(source).toContain('<ReportLoadingState sampleName={selectedSample.name} />');
    expect(source).not.toContain('On-device · live');
    expect(source).not.toContain('Working now');
    expect(loading).toContain("Preparing {sampleName}'s report");
    expect(loading).toContain('Your DNA stays on this computer.');
    expect(loading).toContain('class="report-loading-state"');
    expect(loading).not.toContain('Queued');
    expect(loading).not.toContain('s elapsed');
    expect(loading).not.toContain('$effect');
    expect(loadingCss).toContain('var(--surface-subtle)');
    expect(loadingCss).toContain('var(--accent)');
    expect(loadingCss).toContain('report-loading-sweep');
    expect(loadingCss).toContain('prefers-reduced-motion: reduce');
    expect(loadingCss).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('keeps the general safety reminder in the application footer instead of repeating it in the Guide', () => {
    expect(source).not.toContain('Crucial Safety Information');
    expect(source).not.toContain('Never change medications, supplement dosages, or medical therapies based on this report alone.');
    expect(source).not.toContain('medical-grade clinical lab test');
  });

  it('provides a top-level Help hub before and after a profile is selected', () => {
    expect(pageSource).toContain('{ id: "help", label: "Help" }');
    expect(pageSource).toContain('onOpenHelp={() => activeTab = "help"}');
    expect(pageSource).toContain('onOpenHelp={() => selectTab("help")}');
    expect(helpSource).toContain('Getting started');
    expect(helpSource).toContain('Understanding a finding');
    expect(helpSource).toContain('Reference catalogs &amp; updates');
    expect(helpSource).toContain('Privacy &amp; using results safely');
  });

  it('keeps app releases distinct from reference-catalog updates and includes requested about links', () => {
    expect(helpSource).toContain('Check for app updates');
    expect(helpSource).toContain('Manage those separately under <strong>Data &amp; updates</strong>');
    expect(helpSource).toContain('https://imyourboyroy.com');
    expect(helpSource).toContain('https://github.com/imyourboyroy');
    expect(helpSource).toContain('https://venmo.com/itsyourboyroy');
    expect(helpSource).toContain('rel="noopener noreferrer"');
  });

  it('keeps catalog warnings readable across themes', () => {
    expect(source).toContain('<details class="catalog-warnings-banner" aria-label="Reference catalog status">');
    expect(source).toContain('<span>Reference data needs attention</span>');
    expect(source).toContain("{generatedReport.catalog_warnings.length === 1 ? 'detail' : 'details'}");
    expect(catalogWarningStyles).toContain('padding: 0 16px;');
    expect(catalogWarningStyles).toContain('var(--status-danger-bg)');
    expect(catalogWarningStyles).toContain('var(--status-danger-border)');
    expect(catalogWarningStyles).toContain('var(--status-danger-text)');
    expect(theme).toContain('.catalog-warnings-banner > summary {');
    expect(theme).toContain('.catalog-warnings-banner > summary:focus-visible {');
    expect(theme).toContain('.catalog-warnings-banner[open] > summary::before');
    expect(catalogWarningStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('uses the dashboard health-area directory as the single technical-report boundary', () => {
    const summaryIndex = source.indexOf('<DashboardSummaryPanel');
    const healthAreaIndex = dashboardSource.indexOf('Explore all health areas');
    const sectionsIndex = source.indexOf('<div class="sections-container">');

    expect(source).not.toContain('report-deep-dive-heading');
    expect(source).not.toContain('report-deep-dive-title');
    expect(healthAreaIndex).toBeGreaterThan(-1);
    expect(sectionsIndex).toBeGreaterThan(summaryIndex);
    expect(dashboardSource).toContain('Jump to a health area for complete findings, evidence, and technical details.');
  });

  it('labels the additional-marker filter according to what it reveals', () => {
    expect(source).toContain('Show benign &amp; uncalled');
    expect(source).toContain('aria-label="Show benign and uncalled markers"');
    expect(source).not.toContain('>\n        Undetected\n');
  });

  it('surfaces quick concern filters and report search outside the advanced disclosure', () => {
    expect(source).toContain('Find a gene, marker, or condition');
    expect(source).toContain('Higher concern');
    expect(source).toContain('Review first');
    expect(source).toContain('showClinicalExtraColumns={showClinicalExtraColumns}');
    expect(source).toContain('Clear filters');
  });

  it('expands a health-area target before scrolling to its findings', () => {
    expect(source).toContain('function handleJumpToSection(sectionName: string)');
    expect(source).toContain('setSectionCollapsed(sectionName, false);');
    expect(source).toContain('onJumpToSection={handleJumpToSection}');
  });

  it('places the common export strip before the report reading flow', () => {
    const headerIndex = source.indexOf('<ReportHeader');
    const exportIndex = source.indexOf('<ReportExportActions');
    const summaryIndex = source.indexOf('<DashboardSummaryPanel');

    expect(exportIndex).toBeGreaterThan(headerIndex);
    expect(exportIndex).toBeLessThan(summaryIndex);
    expect(source).toContain("import ReportExportActions from './ReportExportActions.svelte';");
    expect(source).toContain('onExportAiJson={exportAiReviewJson}');
    expect(source).not.toContain('<summary>Report actions</summary>');
    expect(source).not.toContain('Download location');
  });
});

describe('ReportView print preparation', () => {
  it('expands filtered sections for printing and restores the user collapse state', () => {
    expect(source).toContain('async function printReport()');
    expect(source).toContain('window.addEventListener(\'afterprint\', restore, { once: true });');
    expect(source).toContain('Object.fromEntries(filteredSections.map((section) => [section.name, false]))');
    expect(source).toContain('collapsedSections = previousCollapsedSections;');
    expect(source).toContain('onPrintReport={printReport}');
    expect(source).toContain('ReportExportActions');
  });
});
