import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

const files = [
  'ResearchJobActionsPanel.svelte',
  'OfflineDataPanel.svelte',
  'ResearchScopeSection.svelte',
  'ResearchConnectionEditForm.svelte',
  'ResearchLiveLog.svelte',
];

describe('research standalone tooltip migrations', () => {
  it('uses the shared interactive-child tooltip for standalone control help', () => {
    for (const filename of files) {
      const source = readFileSync(new URL(`./${filename}`, import.meta.url), 'utf8');

      expect(source).toContain('Tooltip');
      expect(source).toContain('<Tooltip interactiveChildren');
      expect(source).not.toContain('title=');
    }
  });
});
