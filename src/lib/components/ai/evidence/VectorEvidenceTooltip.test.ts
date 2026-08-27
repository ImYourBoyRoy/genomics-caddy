import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const vectorCard = readFileSync(new URL('./VectorEvidenceCard.svelte', import.meta.url), 'utf8');
const qdrantList = readFileSync(new URL('./QdrantResultsList.svelte', import.meta.url), 'utf8');

describe('advanced evidence tooltip contract', () => {
  it('uses the shared tooltip for vector-card score metadata', () => {
    expect(vectorCard).toContain('import Tooltip from "../../common/Tooltip.svelte";');
    expect(vectorCard).toContain('<Tooltip label="Vector similarity"');
    expect(vectorCard).toContain('<Tooltip label="Data quality"');
    expect(vectorCard).toContain('<Tooltip label="Direction unknown"');
    expect(vectorCard).not.toContain('title="Vector similarity"');
    expect(vectorCard).not.toContain('title="Data quality"');
    expect(vectorCard).not.toContain('title="Do not infer direction');
  });

  it('uses the shared tooltip for Qdrant result scores', () => {
    expect(qdrantList).toContain('import Tooltip from "../../common/Tooltip.svelte";');
    expect(qdrantList).toContain('<Tooltip label="Concept match"');
    expect(qdrantList).toContain('<Tooltip label="Research significance"');
    expect(qdrantList).not.toContain('title="Calculated cosine similarity score');
    expect(qdrantList).not.toContain('title="Significance score');
  });
});
