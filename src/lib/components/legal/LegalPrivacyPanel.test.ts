import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./LegalPrivacyPanel.svelte', import.meta.url), 'utf8');

describe('Legal & Privacy surface', () => {
  it('keeps product/legal copy separate from report findings', () => {
    expect(source).toContain('id="legal-page-title"');
    expect(source).toContain('Local by design');
    expect(source).toContain('Sharing an export');
    expect(source).toContain('Use and responsibility');
    expect(source).toContain('Profile data stays scoped to this sample.');
    expect(source).not.toContain('do_not_claim');
    expect(source).not.toContain('raw_dna_limitation');
  });

  it('uses semantic theme tokens and a desktop-friendly two-column surface', () => {
    expect(source).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(source).not.toMatch(/\brgba?\(/i);
    expect(source).toContain('grid-template-columns: repeat(2, minmax(0, 1fr));');
    expect(source).toContain('var(--surface-card)');
    expect(source).toContain('var(--text-secondary)');
  });
});
