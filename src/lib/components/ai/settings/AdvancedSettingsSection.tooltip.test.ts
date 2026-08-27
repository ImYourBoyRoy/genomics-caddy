import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./AdvancedSettingsSection.svelte', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../../styles/components/advanced-settings-section.css', import.meta.url), 'utf8');

describe('advanced settings path tooltip coverage', () => {
  it('uses the shared tooltip for every optional local storage path', () => {
    expect(source).toContain('import Tooltip from "../../common/Tooltip.svelte";');
    expect(source.match(/<Tooltip interactiveChildren/g)?.length).toBe(5);
    expect(source.match(/<button type="button" class="path-tooltip-trigger">/g)?.length).toBe(5);
    expect(source).not.toContain('title={paths.');
  });

  it('keeps path tooltip anchors keyboard-visible', () => {
    const focusRule = styles.match(/\.path-tooltip-trigger:focus-visible \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(focusRule).toContain('outline: 2px solid var(--focus-ring);');
    expect(focusRule).toContain('outline-offset: 2px;');
    expect(styles.match(/\.path-tooltip-trigger \{[\s\S]*?\n  \}/)?.[0] ?? '').toContain('min-height: 44px;');
  });
});
