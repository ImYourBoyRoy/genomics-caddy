import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(
  resolve(process.cwd(), 'src/lib/components/sidebar/LibraryPanel.svelte'),
  'utf8',
);
const styles = readFileSync(
  resolve(process.cwd(), 'src/lib/styles/components/library-panel.css'),
  'utf8',
);

describe('LibraryPanel sidebar control', () => {
  it('shows a truncated library path with Open and Change actions', () => {
    expect(source).toContain('aria-label="Library folder"');
    expect(source).toContain('status.truncated_path');
    expect(source).toContain('handleOpen');
    expect(source).toContain('handleChange');
    expect(source).toContain('dialogStore.choice');
    expect(source).toContain('Move library');
    expect(source).toContain('Use empty folder');
    expect(source).toContain('setLibraryDir(dest, id)');
    expect(source).not.toContain("Cancel starts fresh");
    expect(styles).toContain('grid-template-columns: repeat(2, minmax(0, 1fr));');
  });

  it('keeps erase and reset behind confirmations and a restart', () => {
    expect(source).toContain('Erase genomes, downloads, and local caches');
    expect(source).toContain('resetLibraryDir');
    expect(source).toContain("goodbye === 'relaunch'");
    expect(source).toContain('eraseLibraryData');
  });
});
