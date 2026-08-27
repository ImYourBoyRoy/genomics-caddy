import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const card = readFileSync(new URL('./ResearchConnectionCard.svelte', import.meta.url), 'utf8');
const form = readFileSync(new URL('./ResearchConnectionEditForm.svelte', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles/components/research-connection-card.css', import.meta.url), 'utf8');

describe('research connection tooltip contract', () => {
  it('exposes connection status descriptions through the shared tooltip', () => {
    expect(card).toContain('import Tooltip from "../common/Tooltip.svelte";');
    expect(card).toContain('label="Qdrant status"');
    expect(card).toContain('label="Ollama status"');
    expect(card).not.toContain('title="Qdrant status"');
    expect(card).not.toContain('title="Ollama status"');
  });

  it('keeps the edit form status indicators accessible without native titles', () => {
    expect(form).toContain('import Tooltip from "../common/Tooltip.svelte";');
    expect(form).toContain('triggerClass="status-tooltip-trigger"');
    expect(form).not.toContain('title="Qdrant status"');
    expect(form).not.toContain('title="Ollama status"');
    expect(styles).toContain('.status-tooltip-trigger {');
  });
});
