import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./VectorAtlasPanel.svelte', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../../styles/components/vector-workbench.css', import.meta.url), 'utf8');

describe('VectorAtlasPanel tooltip contract', () => {
  it('uses the shared tooltip for the named-vector action', () => {
    expect(source).toContain('import Tooltip from "../../common/Tooltip.svelte";');
    expect(source).toContain('<Tooltip interactiveChildren interactiveClickBehavior="dismiss" label="Named vectors"');
    expect(source).toContain('onclick={enableNamedVectors}');
    expect(source).not.toContain('title="Qdrant only — named multi-vectors"');
  });

  it('keeps SVG point details synchronized with keyboard focus', () => {
    expect(source).toContain('function handleAtlasPointKeydown(event: KeyboardEvent, rsid: string)');
    expect(source).toContain('if (event.key !== "Enter" && event.key !== " ") return;');
    expect(source).toContain('onfocus={() => (hoveredRsid = p.rsid)}');
    expect(source).toContain('onblur={() => {');
    expect(source).toContain('Point to or focus a point for details.');
    expect(styles).toContain('.atlas-dot:focus-visible');
    expect(styles).toContain('stroke: var(--focus-ring);');
  });
});
