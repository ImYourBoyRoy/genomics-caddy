import { describe, expect, it } from 'vitest';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

describe('macOS folder-layout DMG', () => {
  it('recovers Genomics Caddy.app from the updater tarball after Tauri cleans the live bundle', () => {
    const script = resolve(process.cwd(), 'scripts/package_macos_folder_dmg.sh');
    const result = spawnSync('bash', [script, '--self-test'], {
      encoding: 'utf8',
    });
    expect(result.status, result.stderr || result.stdout).toBe(0);
    expect(result.stdout).toContain('recovered .app from tar.gz');
  });
});
