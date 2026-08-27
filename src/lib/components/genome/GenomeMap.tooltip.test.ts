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
});
