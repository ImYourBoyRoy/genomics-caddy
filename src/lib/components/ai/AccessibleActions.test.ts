import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const files = [
  'ChatMessages.svelte',
  'ChatSidebar.svelte',
  'ChatInput.svelte',
  'ChatWindow.svelte',
  'EvidenceLibraryPanel.svelte',
  'settings/ConnectionModelSection.svelte',
  'settings/AdvancedSettingsSection.svelte',
].map((file) => readFileSync(new URL(`./${file}`, import.meta.url), 'utf8'));

describe('AI surface action labels', () => {
  it('does not rely on native title attributes for interactive AI controls', () => {
    expect(files.join('\n')).not.toMatch(/<(?:button|a|input|select|textarea)\b[^>]*\s+title\s*=/i);
  });

  it('keeps icon-only actions keyboard and assistive-technology addressable', () => {
    const source = files.join('\n');
    for (const label of [
      'Edit question and branch chat',
      'Delete message',
      'Rename consultation',
      'Delete consultation',
      'Toggle history sidebar',
      'Refresh active models',
      'Remove domain',
      'Remove connection',
    ]) {
      expect(source).toContain(`aria-label="${label}"`);
    }
  });
});
