import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ReportView.svelte', import.meta.url), 'utf8');
const theme = readFileSync(new URL('../../styles/theme.css', import.meta.url), 'utf8');
const interactiveTheme = theme.split('@media print')[0];
const modalSurfaceStyles = source.match(/\.help-backdrop[\s\S]*?\.help-body/)?.[0] ?? '';
const catalogWarningStyles = theme.match(/\.catalog-warnings-banner \{[\s\S]*?\n\}/)?.[0] ?? '';

describe('ReportView Help Guide surface', () => {
  it('keeps the general safety reminder in the application footer instead of repeating it in the Guide', () => {
    expect(source).not.toContain('Crucial Safety Information');
    expect(source).not.toContain('Never change medications, supplement dosages, or medical therapies based on this report alone.');
    expect(source).not.toContain('medical-grade clinical lab test');
  });

  it('uses semantic backdrop and shadow tokens', () => {
    expect(modalSurfaceStyles).toContain('var(--modal-backdrop-bg)');
    expect(modalSurfaceStyles).toContain('var(--shadow-modal)');
    expect(modalSurfaceStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('defines modal tokens for dark, light, and system-light themes', () => {
    expect(interactiveTheme.match(/--modal-backdrop-bg:/g)?.length).toBe(3);
    expect(interactiveTheme.match(/--shadow-modal:/g)?.length).toBe(3);
  });

  it('keeps catalog warnings readable across themes', () => {
    expect(catalogWarningStyles).toContain('var(--status-danger-bg)');
    expect(catalogWarningStyles).toContain('var(--status-danger-border)');
    expect(catalogWarningStyles).toContain('var(--status-danger-text)');
    expect(catalogWarningStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('labels the additional-marker filter according to what it reveals', () => {
    expect(source).toContain('Show benign &amp; uncalled');
    expect(source).toContain('aria-label="Show benign and uncalled markers"');
    expect(source).not.toContain('>\n        Undetected\n');
  });

  it('expands a health-area target before scrolling to its findings', () => {
    expect(source).toContain('function handleJumpToSection(sectionName: string)');
    expect(source).toContain('setSectionCollapsed(sectionName, false);');
    expect(source).toContain('onJumpToSection={handleJumpToSection}');
  });
});

describe('ReportView print preparation', () => {
  it('expands filtered sections for printing and restores the user collapse state', () => {
    expect(source).toContain('async function printReport()');
    expect(source).toContain('window.addEventListener(\'afterprint\', restore, { once: true });');
    expect(source).toContain('Object.fromEntries(filteredSections.map((section) => [section.name, false]))');
    expect(source).toContain('collapsedSections = previousCollapsedSections;');
    expect(source).toContain('onclick={printReport}');
    expect(source).toContain("isPreparingPrint ? 'Preparing PDF…' : 'Export PDF'");
  });
});
