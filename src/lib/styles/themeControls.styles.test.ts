import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./theme.css', import.meta.url), 'utf8');
const rawColor = /(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i;

function themeBlock(start: string, end: string): string {
  const startIndex = source.indexOf(start);
  const endIndex = source.indexOf(end, startIndex + start.length);
  if (startIndex < 0 || endIndex < 0) throw new Error(`Theme block not found: ${start}`);
  return source.slice(startIndex, endIndex);
}

function hexToken(block: string, name: string): string {
  const value = block.match(new RegExp(`${name}:\\s*(#[0-9a-f]{6})`, 'i'))?.[1];
  if (!value) throw new Error(`Theme token not found: ${name}`);
  return value;
}

function relativeLuminance(hex: string): number {
  const channels = hex.slice(1).match(/../g)?.map((channel) => Number.parseInt(channel, 16) / 255);
  if (!channels || channels.length !== 3) throw new Error(`Invalid color: ${hex}`);
  const linear = channels.map((channel) => channel <= 0.03928
    ? channel / 12.92
    : ((channel + 0.055) / 1.055) ** 2.4);
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
}

function contrastRatio(first: string, second: string): number {
  const firstLuminance = relativeLuminance(first);
  const secondLuminance = relativeLuminance(second);
  return (Math.max(firstLuminance, secondLuminance) + 0.05)
    / (Math.min(firstLuminance, secondLuminance) + 0.05);
}

const darkTheme = themeBlock(':root {', ':root[data-theme="system"]');
const lightTheme = themeBlock(':root[data-theme="light"]', '@media (prefers-color-scheme: light)');

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

  it('keeps accent button foregrounds at normal-text AA contrast in active and hover states', () => {
    for (const [name, block] of [['dark', darkTheme], ['light', lightTheme]] as const) {
      expect(
        contrastRatio(hexToken(block, '--accent'), hexToken(block, '--text-on-accent')),
        `${name} teal accent foreground`,
      ).toBeGreaterThanOrEqual(4.5);
      expect(
        contrastRatio(hexToken(block, '--accent-hover'), hexToken(block, '--text-on-accent')),
        `${name} teal hover foreground`,
      ).toBeGreaterThanOrEqual(4.5);
      expect(
        contrastRatio(hexToken(block, '--action-accent-bg'), hexToken(block, '--action-accent-text')),
        `${name} blue action foreground`,
      ).toBeGreaterThanOrEqual(4.5);
      expect(
        contrastRatio(hexToken(block, '--action-accent-hover-bg'), hexToken(block, '--action-accent-text')),
        `${name} blue hover foreground`,
      ).toBeGreaterThanOrEqual(4.5);
    }
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
