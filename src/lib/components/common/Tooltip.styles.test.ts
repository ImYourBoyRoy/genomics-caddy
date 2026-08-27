import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const tooltip = readFileSync(new URL('./Tooltip.svelte', import.meta.url), 'utf8');
const theme = readFileSync(new URL('../../styles/theme.css', import.meta.url), 'utf8');

describe('Tooltip theme surface', () => {
  it('uses the shared tooltip shadow token', () => {
    expect(tooltip).toContain('box-shadow: 0 0.75rem 2rem var(--shadow-tooltip);');
    expect(tooltip).not.toMatch(/box-shadow:[^;]*(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('defines tooltip shadow values for dark, light, and system-light themes', () => {
    expect(theme.match(/--shadow-tooltip:/g)?.length).toBe(3);
  });
});
