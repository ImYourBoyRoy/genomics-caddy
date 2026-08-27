import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ActionabilityMatrix.svelte', import.meta.url), 'utf8');

describe('ActionabilityMatrix tooltip contract', () => {
  it('uses the shared tooltip for plot points without nesting controls', () => {
    expect(source).toContain('import Tooltip from "../../common/Tooltip.svelte";');
    expect(source).toContain('<Tooltip interactiveChildren label="Actionability point"');
    expect(source).toContain('aria-label={pointDescription(p)}');
    expect(source).not.toContain('title="{p.rsid}');
    expect(source).toContain('.dot:focus-visible');
  });
});
