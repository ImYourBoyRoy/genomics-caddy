import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const stylesDir = new URL('./', import.meta.url).pathname;
const motion = readFileSync(join(stylesDir, 'motion.css'), 'utf8');

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) return cssFiles(path);
    return path.endsWith('.css') ? [path] : [];
  });
}

function keyframeBody(source: string, name: string): string {
  const start = source.indexOf(`@keyframes ${name}`);
  expect(start, `${name} is defined`).toBeGreaterThan(-1);
  const open = source.indexOf('{', start);
  let depth = 0;
  for (let i = open; i < source.length; i += 1) {
    if (source[i] === '{') depth += 1;
    if (source[i] === '}' && --depth === 0) return source.slice(open, i + 1);
  }
  throw new Error(`${name} is not closed`);
}

describe('shared motion keyframes', () => {
  it('keeps looping keyframes compositor-only', () => {
    for (const name of ['gc-slide-indeterminate', 'gc-pulse-dot']) {
      const body = keyframeBody(motion, name);
      expect(body).not.toMatch(/(box-shadow|filter|background|width|height|top|left|margin)\s*:/);
      expect(body).toMatch(/transform\s*:/);
    }
  });

  it('defines each global keyframe name once across stylesheets', () => {
    const owners = new Map<string, string[]>();
    for (const file of cssFiles(join(stylesDir))) {
      const css = readFileSync(file, 'utf8');
      for (const match of css.matchAll(/@keyframes\s+([\w-]+)/g)) {
        owners.set(match[1], [...(owners.get(match[1]) ?? []), file]);
      }
    }
    const duplicates = [...owners].filter(([, files]) => new Set(files).size > 1);
    // A repeated global name silently lets whichever sheet loads last win.
    expect(duplicates.map(([name]) => name)).toEqual([]);
  });
});
