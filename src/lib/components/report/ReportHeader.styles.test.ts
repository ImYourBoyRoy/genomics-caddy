import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(process.cwd(), 'src/lib/components/report/ReportHeader.svelte'), 'utf8');
const badgeStyles = source.slice(source.indexOf('.overall-summary-badge'), source.indexOf('.technical-score-details'));

describe('ReportHeader summary badge', () => {
  it('uses semantic theme tokens for association context', () => {
    expect(badgeStyles).toContain('var(--status-accent-bg)');
    expect(badgeStyles).toContain('var(--status-accent-border)');
    expect(badgeStyles).toContain('var(--status-accent-text)');
  });

  it('does not contain raw color literals in the badge style', () => {
    expect(badgeStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
