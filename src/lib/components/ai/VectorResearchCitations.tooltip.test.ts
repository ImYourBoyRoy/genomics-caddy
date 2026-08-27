import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./VectorResearchCitations.svelte', import.meta.url), 'utf8');

describe('VectorResearchCitations tooltip contract', () => {
  it('uses shared tooltips for the query and similarity metadata', () => {
    expect(source).toContain('import Tooltip from "../common/Tooltip.svelte";');
    expect(source).toContain('<Tooltip label="Semantic query"');
    expect(source).toContain('<Tooltip label="Semantic similarity"');
    expect(source).not.toContain('title="Semantic query"');
    expect(source).not.toContain('title="Semantic similarity"');
  });
});
