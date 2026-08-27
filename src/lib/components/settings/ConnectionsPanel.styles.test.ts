import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const styles = readFileSync(new URL('../../styles/components/connections-panel.css', import.meta.url), 'utf8');
const rawColor = /(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i;

describe('connections panel theme surface', () => {
  it('keeps the desktop connections workbench on shared semantic tokens', () => {
    expect(styles).not.toMatch(rawColor);
    expect(styles).toContain('background: var(--surface-control);');
    expect(styles).toContain('color: var(--status-success-text);');
    expect(styles).toContain('background: var(--status-warning-bg);');
    expect(styles).toContain('background: var(--status-danger-bg);');
  });

  it('keeps connection controls readable across the shared theme modes', () => {
    expect(styles).toContain('color: var(--text-primary);');
    expect(styles).toContain('border: 1px solid var(--border-strong);');
    expect(styles).toContain('background: var(--surface-raised);');
  });
});
