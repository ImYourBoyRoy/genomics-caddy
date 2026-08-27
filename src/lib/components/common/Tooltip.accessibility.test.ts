import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./Tooltip.svelte', import.meta.url), 'utf8');

describe('Tooltip accessibility structure', () => {
  it('keeps the accessible name on the native trigger instead of a redundant wrapper group', () => {
    expect(source).not.toContain('role="group"');
    expect(source).toContain('role="presentation"');
    expect(source).toContain('class={`tooltip-trigger ${triggerClass}`}');
    expect(source).toContain('triggerClass?: string;');
    expect(source).toContain('aria-label={label}');
    expect(source).toContain('aria-describedby={isOpen ? panelDescriptionId : undefined}');
  });

  it('keeps interactive tooltip content out of tooltip role semantics', () => {
    expect(source).toContain("role={learnMoreHref ? 'dialog' : 'tooltip'}");
    expect(source).toContain("aria-haspopup={learnMoreHref ? 'dialog' : undefined}");
  });

  it('lets a second trigger activation dismiss hover- or focus-open content', () => {
    expect(source).toContain('function handleTriggerClick(event: MouseEvent)');
    expect(source).toContain('if (isClicked) {');
    expect(source).toContain('close();');
    expect(source).toContain('onclick={handleTriggerClick}');
  });
});
