import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./Tooltip.svelte', import.meta.url), 'utf8');
const theme = readFileSync(new URL('../../styles/theme.css', import.meta.url), 'utf8');

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
    expect(source).not.toContain('tabindex="0"');
  });

  it('keeps interactive tooltip content out of tooltip role semantics', () => {
    expect(source).toContain("role={learnMoreHref ? 'dialog' : 'tooltip'}");
    expect(source).toContain("nextTrigger.setAttribute('aria-haspopup', 'dialog')");
  });

  it('ships a static tooltip fallback for native WebKit first-render reliability', () => {
    const fallback = theme.match(/\.tooltip-panel \{[\s\S]*?\n\}/)?.[0] ?? '';
    expect(fallback).toContain('position: fixed;');
    expect(fallback).toContain('display: flex;');
    expect(fallback).toContain('padding: 0.75rem 0.85rem;');
    expect(fallback).toContain('max-height: 24rem;');
    expect(fallback).toContain('max-height: min(24rem, calc(100vh - 1rem));');
    expect(fallback).toContain('background: var(--tooltip-bg);');
  });

  it('lets a second trigger activation dismiss hover- or focus-open content', () => {
    expect(source).toContain('function handleTriggerClick(event: MouseEvent)');
    expect(source).toContain('if (isClicked) {');
    expect(source).toContain('close();');
    expect(source).toContain('onclick={handleTriggerClick}');
  });

  it('keeps explanatory chips as inline notes instead of 44px buttons', () => {
    expect(source).toContain('class={`tooltip-trigger ${triggerClass}`}');
    expect(source).not.toContain('<button');
    expect(source).not.toContain('tabindex="0"');
    expect(source).not.toMatch(/\.tooltip-trigger \{[\s\S]*?min-height: 44px/);
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
    expect(source).toContain('function observePanelSize()');
    expect(source).toContain('panelResizeObserver = new ResizeObserver(() => {');
    expect(source).toContain('requestAnimationFrame(updatePosition);');
    expect(source).toContain("nextTrigger.setAttribute('aria-describedby', panelDescriptionId);");
    expect(source).toContain('{#if interactiveChildren}');
  });
});
