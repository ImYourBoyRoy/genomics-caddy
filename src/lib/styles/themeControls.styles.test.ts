import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./theme.css', import.meta.url), 'utf8');
const rawColor = /(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i;

function firstRule(selector: string): string {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return source.match(new RegExp(`${escapedSelector} \\{[\\s\\S]*?\\n\\}`))?.[0] ?? '';
}

describe('shared theme controls', () => {
  it('defines action and danger hover tokens for dark, light, and system-light themes', () => {
    expect(source.match(/--action-accent-bg:/g)).toHaveLength(3);
    expect(source.match(/--action-accent-hover-bg:/g)).toHaveLength(3);
    expect(source.match(/--action-accent-text:/g)).toHaveLength(3);
    expect(source.match(/--status-danger-hover-bg:/g)).toHaveLength(3);
  });

  it('keeps buttons and status pills on semantic color tokens', () => {
    const rules = [
      '.badge.success',
      '.badge.warning',
      '.pill',
      '.btn-primary',
      '.btn-secondary',
      '.btn-secondary:hover:not(:disabled)',
      '.btn-danger',
      '.btn-danger:hover:not(:disabled)',
      '.btn-accent',
      '.btn-accent:hover',
      '.tab-status-pill.running',
      '.tab-status-pill.paused',
      '.view-mode-btn.view-mode-active'
    ].map(firstRule);

    expect(rules.every((rule) => rule && !rawColor.test(rule))).toBe(true);
  });

  it('keeps navigation, dual explanations, settings, and hero surfaces theme-aware', () => {
    const rules = [
      '.tabs',
      '.tab-btn',
      '.tab-btn:hover',
      '.tab-btn.active',
      '.tabs-advanced-strip',
      '.tabs-advanced-chip',
      '.tabs-advanced-chip:hover',
      '.tabs-advanced-chip.active',
      '.layperson-box',
      '.clinical-box',
      '.settings-select',
      '.settings-select:focus',
      '.report-hero',
      '.report-kicker',
      '.report-hero-title',
      '.report-hero-lead',
      '.report-glyph',
      '.report-hero-cta'
    ].map(firstRule);

    expect(rules.every((rule) => rule && !rawColor.test(rule))).toBe(true);
  });

  it('uses semantic focus and hover states for tab controls', () => {
    expect(firstRule('.tab-btn:focus-visible')).toContain('var(--focus-ring)');
    expect(firstRule('.tabs-advanced-chip:focus-visible')).toContain('var(--focus-ring)');
    expect(firstRule('.tab-btn:hover')).toContain('var(--control-hover-bg)');
    expect(firstRule('.tabs-advanced-chip:hover')).toContain('var(--control-hover-bg)');
  });
});
