import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./VectorAtlasPanel.svelte', import.meta.url), 'utf8');

describe('VectorAtlasPanel tooltip contract', () => {
  it('uses the shared tooltip for the named-vector action', () => {
    expect(source).toContain('import Tooltip from "../../common/Tooltip.svelte";');
    expect(source).toContain('<Tooltip interactiveChildren label="Named vectors"');
    expect(source).toContain('onclick={enableNamedVectors}');
    expect(source).not.toContain('title="Qdrant only — named multi-vectors"');
  });
});
