import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./DashboardSummaryPanel.svelte', import.meta.url), 'utf8');
const styleBlock = source.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? '';

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
      '--surface-control',
      '--border-color',
      '--text-primary',
      '--text-secondary',
    ];

    for (const token of requiredTokens) {
      expect(styleBlock).toContain(`var(${token})`);
    }
  });

  it('keeps repeated guidance boundaries compact and available on demand', () => {
    expect(source).toContain("import Tooltip from '../common/Tooltip.svelte';");
    expect(source).toContain('Optional self-reported context used to tailor guidance.');
    expect(source).toContain('Conditional prompts, not permanent food rules.');
    expect(source).toContain('Review interactions and health context before use.');
    expect(source).toContain('Planning prompts for training and recovery — not activity clearance.');
    expect(source).toContain('Bring current medications and past responses to a clinician or pharmacist.');
    expect(source).toContain('Grouped by priority for clinician discussion.');
    expect(source).toContain('Up to five prioritized follow-up prompts from this report.');
    expect(source).toContain('{Math.min(plan.topFindings.length, 5)} shown');
    expect(source).not.toContain('A genotype match is not a permanent food restriction;');
    expect(source).not.toContain('This selection is self-reported, stored per DNA profile, and is never inferred');
    expect(source).not.toContain('Every supplement item is a discussion prompt, not a prescription.');
    expect(source).not.toContain('Genotype may provide weak context for training questions.');
    expect(source).not.toContain('Raw consumer DNA is not a complete clinical PGx result');
    expect(source).not.toContain('DNA cannot measure current hormones or diagnose a condition or medication response.');
    expect(source).not.toContain('Grouped by priority for clinician discussion; seek care promptly for acute symptoms.');
    expect(source).not.toContain('{Math.min(plan.topFindings.length, 5)} of 5');
  });

  it('keeps repeated action steps concise instead of repeating warning copy', () => {
    expect(source).toContain("return 'Consider clinical confirmation.'");
    expect(source).toContain("return 'Compare with symptoms, history, and relevant labs.'");
    expect(source).not.toContain('Ask a qualified clinician whether medical-grade confirmation');
  });

  it('keeps collapsible guidance titles as real headings outside button descendants', () => {
    expect(source).toContain('<h3 class="card-header-heading">');
    expect(source).toContain('<span class="card-header-title">🥗 Dietary Alignment</span>');
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
    expect(source).toContain('Health areas');
    expect(source).toContain('healthAreaSections');
    expect(source).toContain("href={'#' + sectionAnchorId(section.name)}");
    expect(source).toContain('onJumpToSection?.(section.name)');
    expect(source).toContain('{section.markers.length} markers');

    const index = styleBlock.match(/\.health-area-index \{[\s\S]*?\.actionability-safety/)?.[0] ?? '';
    expect(index).toContain('grid-template-columns: repeat(auto-fit');
    expect(index).toContain('min-height: 44px;');
    expect(index).toContain('overflow-wrap: anywhere;');
    expect(index).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('allows long guidance text to wrap inside narrow cards', () => {
    const cardBody = styleBlock.match(/\.card-body \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const notes = styleBlock.match(/\.diet-notes-pre \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const supplementItem = styleBlock.match(/\.supplement-item \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const supplementReason = styleBlock.match(/\.supp-reason \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(cardBody).toContain('min-width: 0;');
    expect(notes).toContain('overflow-wrap: anywhere;');
    expect(notes).toContain('word-break: break-word;');
    expect(supplementItem).toContain('box-sizing: border-box;');
    expect(supplementReason).toContain('display: block;');
    expect(supplementReason).toContain('overflow-wrap: anywhere;');
  });

  it('bounds the optional context selector and dashboard containers', () => {
    const dashboard = styleBlock.match(/\.dashboard-v2 \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const context = styleBlock.match(/\.context-selector \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const heading = styleBlock.match(/\.context-selector-heading \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const select = styleBlock.match(/\.context-selector select \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const grid = styleBlock.match(/\.grid-layout \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const summaryCard = styleBlock.match(/\.summary-card \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const labCard = styleBlock.match(/\.card-lab-followups \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(dashboard).toContain('box-sizing: border-box;');
    expect(context).toContain('max-width: 100%;');
    expect(heading).toContain('flex-wrap: wrap;');
    expect(select).toContain('box-sizing: border-box;');
    expect(select).toContain('max-width: 100%;');
    expect(select).toContain('text-overflow: ellipsis;');
    expect(grid).toContain('min-width: 0;');
    expect(summaryCard).toContain('box-sizing: border-box;');
    expect(labCard).toContain('max-width: 100%;');
  });

  it('keeps context selector labels short while retaining their stable IDs', () => {
    expect(source).toContain("const contextOptionLabels: Record<string, string>");
    expect(source).toContain("menstrual_cycle: 'Menstrual cycle / PMS'");
    expect(source).toContain("suspected_adenomyosis: 'Adenomyosis / heavy bleeding'");
    expect(source).toContain("<option value=\"\">Not specified</option>");
    expect(source).toContain('contextOptionLabel(option.id, option.label)');
  });
});
