import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const styles = readFileSync(new URL('../../styles/components/research-scope-section.css', import.meta.url), 'utf8');
const rawColor = /(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i;

describe('research scope workbench theme surface', () => {
  it('keeps the workbench on shared semantic theme tokens', () => {
    expect(styles).not.toMatch(rawColor);
    expect(styles).toContain('color: var(--report-scope-text);');
    expect(styles).toContain('background: var(--surface-control);');
    expect(styles).toContain('background: var(--status-info-soft-bg);');
    expect(styles).toContain('background: var(--status-warning-bg);');
    expect(styles).toContain('color: var(--status-success-text);');
  });

  it('keeps selected scope and source presets visibly distinct', () => {
    expect(styles).toContain('.scope-option:has(input:checked)');
    expect(styles).toContain('background: var(--report-scope-bg);');
    expect(styles).toContain('.preset-btn.active');
    expect(styles).toContain('color: var(--report-scope-text);');
  });
});
