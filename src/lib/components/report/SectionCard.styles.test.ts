import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./SectionCard.svelte', import.meta.url), 'utf8');
const styleBlock = source.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? '';

describe('SectionCard semantic status styling', () => {
  it('keeps active finding and score surfaces on shared theme tokens', () => {
    expect(styleBlock).toContain('var(--status-danger-bg)');
    expect(styleBlock).toContain('var(--status-danger-strong-text)');
    expect(styleBlock).toContain('var(--status-danger-border)');
    expect(styleBlock).toContain('var(--shadow-subtle)');
  });

  it('does not contain raw color literals in the section-card style block', () => {
    expect(styleBlock).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('disables the section transition when reduced motion is requested', () => {
    expect(source).toContain("window.matchMedia('(prefers-reduced-motion: reduce)')");
    expect(source).toContain('mediaQuery.addEventListener');
    expect(source).toContain('duration: reduceMotion ? 0 : 200');
  });

  it('keeps Simple section status concise without duplicating DNA coverage', () => {
    expect(source).toContain('<span class="sec-score-descriptive">Association context</span>');
    expect(source).not.toContain('Association context · {callableCount}/{section.summary.total_markers} called');
  });

  it('compresses Simple section summaries and avoids repeating uncalled counts', () => {
    expect(source).toContain('class:simple-summary={viewMode === \'simple\'}');
    expect(source).toContain('<span class="summary-line">');
    expect(source).toContain("summaryParts.filter((part) => !part.endsWith('not tested'))");
    expect(source).toContain("simpleSummaryParts.filter((part) => !part.endsWith('not called'))");
    expect(source).toContain('{noDataCount} not called');
    expect(source).not.toContain('unknown, not negative');
  });

  it('derives header findings from the visible filtered marker list', () => {
    expect(source).toContain("section.markers.filter((marker) => marker.severity_class !== 'benign' && marker.severity_class !== 'no_data').length");
    expect(source).toContain('let showActiveCount = $derived(activeCount > 0 && activeCount < section.markers.length);');
    expect(source).toContain('{#if showActiveCount}');
    expect(source).toContain('{section.markers.length} shown');
    expect(source).not.toContain("section.markers.length === 1 ? 'variant' : 'variants'");
    expect(source).not.toContain('section.summary.active_marker_count ?? 0');
  });

  it('gives each section a stable anchor for the dashboard health-area index', () => {
    expect(source).toContain('let sectionAnchorId = $derived(`report-section-${section.name.toLowerCase()');
    expect(source).toContain('id={sectionAnchorId}');
    expect(styleBlock).toContain('scroll-margin-top: 1rem;');
  });

  it('keeps the section title as a heading outside the expandable button content', () => {
    expect(source).toContain('<h4 class="section-heading">');
    expect(source).toContain('<span class="section-heading-label">{section.name}</span>');
    expect(source).not.toContain('<button\n        type="button"\n        class="section-toggle"\n        onclick={toggleCollapse}\n        aria-expanded={!isCollapsed}\n        aria-controls={sectionBodyId}\n      >\n        <span class="collapse-icon" aria-hidden="true">\n          {isCollapsed ? \'▶\' : \'▼\'}\n        </span>\n        <h4>');
  });

  it('uses an explicit parent callback for reliable section collapse state', () => {
    expect(source).toContain('onCollapsedChange?: (collapsed: boolean) => void;');
    expect(source).toContain('onCollapsedChange(nextCollapsed);');
    expect(source).toContain('collapsed = nextCollapsed;');
  });

  it('keeps section toggles usable as touch controls', () => {
    expect(styleBlock).toContain('min-height: 44px;');
    expect(styleBlock).toContain('gap: var(--space-1);');
  });
});
