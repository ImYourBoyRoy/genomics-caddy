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
    expect(source).not.toContain('A genotype match is not a permanent food restriction;');
    expect(source).not.toContain('This selection is self-reported, stored per DNA profile, and is never inferred');
    expect(source).not.toContain('Every supplement item is a discussion prompt, not a prescription.');
    expect(source).not.toContain('Genotype may provide weak context for training questions.');
    expect(source).not.toContain('Raw consumer DNA is not a complete clinical PGx result');
    expect(source).not.toContain('DNA cannot measure current hormones or diagnose a condition or medication response.');
    expect(source).not.toContain('Grouped by priority for clinician discussion; seek care promptly for acute symptoms.');
  });
});
