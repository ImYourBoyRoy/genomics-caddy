import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./agentUiBridge.ts', import.meta.url), 'utf8');

describe('agent UI bridge visibility boundary', () => {
  it('keeps closed disclosure summaries visible but excludes their hidden bodies', () => {
    expect(source).toContain("const closedDetails = element.closest('details:not([open])');");
    expect(source).toContain('if (closedDetails && !element.closest(\'summary\')) return false;');
  });
});
