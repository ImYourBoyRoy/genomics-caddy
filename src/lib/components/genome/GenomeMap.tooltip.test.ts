import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./GenomeMap.svelte', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles/components/genome-map.css', import.meta.url), 'utf8');

describe('chromosome map tooltip surfaces', () => {
  it('uses the shared tooltip for trait overlay help', () => {
    expect(source).toContain('import Tooltip from "../common/Tooltip.svelte";');
    expect(source).toContain('<Tooltip');
    expect(source).toContain('triggerClass="trait-band-trigger"');
    expect(source).not.toContain('title="{band.trait_category}: {band.association_count} associations"');
    expect(styles).toContain('.trait-band-trigger');
    expect(styles).toContain('min-width: 44px;');
    expect(styles).toContain('min-height: 44px;');
  });

  it('keeps chromosome track metadata accessible without a nested interactive wrapper', () => {
    expect(source).toContain('aria-label={`Chromosome ${chr}; ${formatMb(CHR_LENGTHS[chr])}; ${count.toLocaleString()} SNPs`}');
    expect(source).not.toContain('title={`Chr ${chr}');
  });

  it('uses the shared collision-aware tooltip for variant pins', () => {
    expect(source).toContain('interactiveChildren');
    expect(source).toContain('interactiveClickBehavior="dismiss"');
    expect(source).toContain('description={`Chromosome ${pin.chr} finding: ${severityLabel(pin.severity)}. Select to open this finding in the report.`}');
    expect(source).not.toContain('hoveredPin');
    expect(source).not.toContain('class="map-tooltip"');
    expect(styles).not.toContain('.map-tooltip');
  });

  it('keeps pin insight copy wrapping and does not wash the active chromosome row', () => {
    expect(styles).toContain('.map-pin-insight p {');
    expect(styles).toContain('overflow-wrap: anywhere;');
    expect(styles).toContain('white-space: normal;');
    expect(styles).toContain('background: var(--surface-subtle);');
    expect(styles).toContain('box-shadow: inset 3px 0 0 var(--accent-bright);');
    expect(styles).not.toContain('background: rgba(45, 212, 191, 0.06)');
    expect(styles).not.toContain('background: rgba(255, 255, 255, 0.015)');
  });
});
