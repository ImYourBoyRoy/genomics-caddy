import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./desktopContextMenu.ts', import.meta.url), 'utf8');

describe('desktop context menu', () => {
  it('uses a Tauri-native menu for the desktop app', () => {
    expect(source).toContain("import { Menu, MenuItem, PredefinedMenuItem } from '@tauri-apps/api/menu';");
    expect(source).toContain('event.preventDefault();');
    expect(source).toContain('new LogicalPosition(event.clientX, event.clientY)');
    expect(source).toContain("document.addEventListener('contextmenu', handler);");
    expect(source).toContain("document.documentElement.dataset.contextMenu = 'genomics-caddy';");
  });

  it('keeps the menu relevant to genomics work and out of browser chrome', () => {
    expect(source).toContain('Refresh current report');
    expect(source).toContain('Explore research findings');
    expect(source).toContain('Open AI consultation');
    expect(source).toContain('Import DNA export');
    expect(source).not.toContain('Inspect');
    expect(source).not.toContain('View page source');
    expect(source).not.toContain('Save Page As');
  });

  it('preserves useful editing actions inside editable controls', () => {
    expect(source).toContain("closest('input, textarea, [contenteditable=\"true\"]')");
    expect(source).toContain("item: 'Cut'");
    expect(source).toContain("item: 'Copy'");
    expect(source).toContain("item: 'Paste'");
    expect(source).toContain("item: 'SelectAll'");
  });

  it('removes the listener and QA marker during cleanup', () => {
    expect(source).toContain("document.removeEventListener('contextmenu', handler);");
    expect(source).toContain("delete document.documentElement.dataset.contextMenu;");
    expect(source).toContain('await menu.close();');
  });
});
