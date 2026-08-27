import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const findings = readFileSync(new URL('./ResearchLiveFindings.svelte', import.meta.url), 'utf8');
const sources = readFileSync(new URL('../ai/evidence/EvidenceSourcesSidebar.svelte', import.meta.url), 'utf8');

describe('research surface tooltip contract', () => {
  it('uses the shared tooltip for live finding bucket context', () => {
    expect(findings).toContain('import Tooltip from "../common/Tooltip.svelte";');
    expect(findings).toContain('<Tooltip label={bucketLabel(finding.impact_bucket)}');
    expect(findings).not.toContain('title={bucketHint(finding.impact_bucket)}');
  });

  it('uses the shared tooltip to reveal truncated source names', () => {
    expect(sources).toContain('import Tooltip from "../../common/Tooltip.svelte";');
    expect(sources).toContain('<Tooltip label="Loaded source" description={src} triggerClass="source-tooltip-trigger">');
    expect(sources).not.toContain('title={src}');
  });
});
