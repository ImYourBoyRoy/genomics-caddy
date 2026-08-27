import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ResearchJobProgressPanel.svelte', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles/components/research-job-controls.css', import.meta.url), 'utf8');

describe('research job progress tooltip contract', () => {
  it('keeps adaptive tuning explanation available through the shared tooltip', () => {
    expect(source).toContain('import Tooltip from "../common/Tooltip.svelte";');
    expect(source).toContain('<Tooltip');
    expect(source).toContain('label="Adaptive sweep tuning"');
    expect(source).not.toContain('title="Batch size and prepare parallelism adapt');
    expect(styles).toContain('.tuning-banner .tuning-help-trigger');
  });
});
