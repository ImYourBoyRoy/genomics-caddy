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
});
