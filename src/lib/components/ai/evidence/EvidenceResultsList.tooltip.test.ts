import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./EvidenceResultsList.svelte', import.meta.url), 'utf8');

describe('EvidenceResultsList tooltip contract', () => {
  it('uses the shared accessible tooltip for search-result metadata', () => {
    expect(source).toContain('import Tooltip from "../../common/Tooltip.svelte";');
    expect(source).toContain('<Tooltip label="Concept match"');
    expect(source).toContain('<Tooltip label="Text match"');
    expect(source).toContain('<Tooltip label="Vectorized"');
    expect(source).not.toContain('title="Calculated cosine similarity');
    expect(source).not.toContain('title="Exact text search hit');
    expect(source).not.toContain('title="Vector embedding generated');
  });
});
