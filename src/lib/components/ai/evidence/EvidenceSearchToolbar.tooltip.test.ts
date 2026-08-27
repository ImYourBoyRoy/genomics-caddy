import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./EvidenceSearchToolbar.svelte', import.meta.url), 'utf8');

describe('EvidenceSearchToolbar tooltip contract', () => {
  it('keeps trait query hints accessible on the existing buttons', () => {
    expect(source).toContain('import Tooltip from "../../common/Tooltip.svelte";');
    expect(source).toContain('<Tooltip interactiveChildren interactiveClickBehavior="dismiss" label={cat.label} description={cat.queryHint}');
    expect(source).not.toContain('title={cat.queryHint}');
    expect(source).toContain('onclick={() => onBrowseTrait(cat.id)}');
  });
});
