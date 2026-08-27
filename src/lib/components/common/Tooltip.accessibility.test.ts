import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./Tooltip.svelte', import.meta.url), 'utf8');

describe('Tooltip accessibility structure', () => {
  it('keeps the accessible name on the native trigger instead of a redundant wrapper group', () => {
    expect(source).not.toContain('role="group"');
    expect(source).toContain('role="presentation"');
    expect(source).toContain('class={`tooltip-trigger ${triggerClass}`}');
    expect(source).toContain('triggerClass?: string;');
    expect(source).toContain('interactiveChildren?: boolean;');
    expect(source).toContain("interactiveClickBehavior?: InteractiveClickBehavior;");
    expect(source).toContain("interactiveClickBehavior = 'toggle'");
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

  it('does not pin a tooltip when the wrapped child is an action control', () => {
    expect(source).toContain("if (interactiveChildren && interactiveClickBehavior === 'dismiss') {");
    expect(source).toContain('close();');
    expect(source).toContain('interactiveClickBehavior = \'toggle\'');
  });

  it('closes only the open tooltip before a parent drawer handles Escape', () => {
    expect(source).toContain("if (event.key !== 'Escape' || !isOpen) return;");
    expect(source).toContain('event.preventDefault();');
    expect(source).toContain('event.stopImmediatePropagation();');
    expect(source).toContain('document.addEventListener(\'keydown\', handleKeydown, true);');
    expect(source).toContain('document.removeEventListener(\'keydown\', handleKeydown, true);');
  });

  it('can attach semantics to an existing interactive child without nesting a button', () => {
    expect(source).toContain('interactiveChildren = false');
    expect(source).toContain('class:interactive-children={interactiveChildren}');
    expect(source).toContain('hostElement.querySelector<HTMLElement>');
    expect(source).toContain('let triggerElement = $state<HTMLElement | undefined>(undefined);');
    expect(source).toContain('triggerElement = nextTrigger;');
    expect(source).toContain('if (isOpen) requestAnimationFrame(updatePosition);');
    expect(source).toContain("nextTrigger.setAttribute('aria-describedby', panelDescriptionId);");
    expect(source).toContain('{#if interactiveChildren}');
  });
});
