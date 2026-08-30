import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const overlaySource = readFileSync(resolve(process.cwd(), 'src/lib/components/common/bootstrap/BootstrapOverlay.svelte'), 'utf8');
const screenSource = readFileSync(resolve(process.cwd(), 'src/lib/components/common/AppBootstrapScreen.svelte'), 'utf8');

describe('desktop bootstrap screen', () => {
  it('keeps startup intentionally dark and independent from the selected report theme', () => {
    expect(overlaySource).toContain('color-scheme: dark;');
    expect(overlaySource).toContain('--bg-primary: #080b14;');
    expect(overlaySource).toContain('linear-gradient(135deg, #070a12 0%, #0d1424 54%, #0a1718 100%)');
    expect(overlaySource).not.toContain('var(--bg-primary));');
  });

  it('gives the startup details a focused panel instead of spreading them across the screen', () => {
    expect(screenSource).toContain('max-width: 640px;');
    expect(screenSource).toContain('border-radius: 20px;');
    expect(screenSource).toContain('background: rgba(8, 12, 24, 0.72);');
    expect(screenSource).toContain('backdrop-filter: blur(18px);');
  });
});
