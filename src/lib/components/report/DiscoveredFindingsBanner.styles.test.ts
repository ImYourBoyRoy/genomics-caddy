import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const source = readFileSync(
  resolve(process.cwd(), 'src/lib/components/report/DiscoveredFindingsBanner.svelte'),
  'utf8'
);
const bannerStyles = source.slice(source.indexOf('<style>'), source.indexOf('</style>'));

describe('DiscoveredFindingsBanner disclosure surface', () => {
  it('exposes a labelled disclosure relationship for the findings list', () => {
    expect(source).toContain('aria-expanded={expanded}');
    expect(source).toContain('aria-controls={expanded ? "discovered-findings-list" : undefined}');
    expect(source).toContain('id="discovered-findings-list"');
    expect(source).toContain('aria-label={expanded ? "Hide discovered findings" : "Show discovered findings"}');
  });

  it('uses semantic theme tokens instead of fixed banner colors', () => {
    expect(bannerStyles).toContain('var(--report-claim-bg)');
    expect(bannerStyles).toContain('var(--report-claim-border)');
    expect(bannerStyles).toContain('var(--border-color)');
    expect(bannerStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('keeps discovery identifiers and DNA calls behind technical details in Simple mode', () => {
    expect(source).toContain("presentationMode?: 'simple' | 'clinical' | 'compare';");
    expect(source).toContain("{#if presentationMode === 'simple'}");
    expect(source).toContain('<summary>Technical data</summary>');
    expect(source).toContain('<dt>DNA call</dt><dd>{item.user_genotype}</dd>');
  });

  it('wraps long technical discovery labels without displacing navigation controls', () => {
    expect(bannerStyles).toContain('.findings-list li {');
    expect(bannerStyles).toContain('flex-wrap: wrap;');
    expect(bannerStyles).toContain('min-width: 0;');
    expect(bannerStyles).toContain('overflow-wrap: anywhere;');
    expect(bannerStyles).toContain('.mini-nav {');
    expect(bannerStyles).toContain('flex: 0 0 auto;');
  });
});
