import { describe, expect, it } from 'vitest';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

describe('Windows portable zip', () => {
  it('keeps Data/.keep so the empty library folder survives zip', () => {
    const script = resolve(process.cwd(), 'scripts/package_windows_portable.mjs');
    const result = spawnSync(process.execPath, [script, '--self-test'], {
      encoding: 'utf8',
    });
    expect(result.status, result.stderr || result.stdout).toBe(0);
    expect(result.stdout).toContain('Data/.keep present');
  });
});
