import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./WarningBlocks.svelte', import.meta.url), 'utf8');
const theme = readFileSync(new URL('../../styles/theme.css', import.meta.url), 'utf8');

describe('warning block density', () => {
  it('keeps detailed warning copy behind one compact disclosure', () => {
    expect(source).toContain('<details class="warning-details">');
    expect(source).toContain('Limits &amp; confirmation');
    expect(source).toContain('class="warning-summary-badge">Clinical review</span>');
    expect(source).not.toContain('⚠️ Raw DNA Limitation:');
    expect(source).not.toContain('⚠️ Clinical Warning (Does Not Claim):');
  });

  it('keeps warning surfaces on shared semantic status tokens', () => {
    const warningStyles = [
      /\.confirm-pathway \{[\s\S]*?\n\}/,
      /\.confirm-pathway \.sec-title \{[\s\S]*?\n\}/,
      /\.claim-warning \{[\s\S]*?\n\}/,
      /\.claim-warning \.sec-title \{[\s\S]*?\n\}/,
      /\.raw-limitation-warning \{[\s\S]*?\n\}/,
      /\.raw-limitation-warning \.sec-title \{[\s\S]*?\n\}/,
      /\.clinical-confirmation-warning \{[\s\S]*?\n\}/,
    ]
      .map((pattern) => theme.match(pattern)?.[0] ?? '')
      .join('\n');

    expect(warningStyles).toContain('var(--status-info-bg)');
    expect(warningStyles).toContain('var(--status-warning-bg)');
    expect(warningStyles).toContain('var(--status-danger-bg)');
    expect(warningStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
