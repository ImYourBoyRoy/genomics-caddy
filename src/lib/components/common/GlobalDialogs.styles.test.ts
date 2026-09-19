import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./GlobalDialogs.svelte', import.meta.url), 'utf8');
const styleBlock = readFileSync(
  new URL('../../styles/components/global-dialogs.css', import.meta.url),
  'utf8',
);

describe('GlobalDialogs interaction and theme contract', () => {
  it('exposes labelled alert-dialog semantics and typed controls', () => {
    expect(source).toContain('role="alertdialog"');
    expect(source).toContain('aria-modal="true"');
    expect(source).toContain('tabindex="-1"');
    expect(source).toContain('onkeydown={handleDialogKeydown}');
    expect(source).toContain("if (event.key === 'Escape')");
    expect(source).toContain('aria-labelledby="global-dialog-title"');
    expect(source).toContain('aria-describedby="global-dialog-message"');
    expect(source).toContain('id="global-dialog-title"');
    expect(source).toContain('id="global-dialog-message"');
    expect(source).toContain('aria-label="Close dialog"');
    expect(source).toContain('dialogStore.handleChoice');
    expect(source.match(/type="button"/g)?.length).toBe(5);
    expect(styleBlock).toContain('min-width: 44px');
    expect(styleBlock).toContain('min-height: 44px');
  });

  it('uses semantic theme tokens for dialog surfaces', () => {
    for (const token of [
      '--modal-backdrop-bg',
      '--surface-raised',
      '--surface-subtle',
      '--border-color',
      '--shadow-modal',
    ]) {
      expect(styleBlock).toContain(`var(${token})`);
    }
    expect(styleBlock).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
