import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ActivityPulse.svelte', import.meta.url), 'utf8');
const errorStyles = source.match(/\.activity-pulse\.error \.pulse-inner[\s\S]*?<\/style>/)?.[0] ?? '';

describe('ActivityPulse error surface', () => {
  it('uses semantic danger tokens for error text and surfaces', () => {
    expect(errorStyles).toContain('var(--status-danger-strong-text)');
    expect(errorStyles).toContain('var(--status-danger-border)');
    expect(errorStyles).toContain('var(--status-danger-bg)');
  });

  it('does not retain fixed error colors in the guarded error block', () => {
    expect(errorStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('keeps a full-width loading pulse inside its parent box', () => {
    const baseStyles = source.match(/\.activity-pulse \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(baseStyles).toContain('box-sizing: border-box;');
    expect(baseStyles).toContain('min-width: 0;');
  });
});
