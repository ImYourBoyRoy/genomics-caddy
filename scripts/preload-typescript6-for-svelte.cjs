// ./scripts/preload-typescript6-for-svelte.cjs
/*
Purpose: Compatibility shim so Svelte tooling can run while the project uses TypeScript 7.
Responsibilities:
  - Redirect `require("typescript")` from svelte-check / language-server / svelte2tsx
    to `@typescript/typescript6` (TS 6 API), which still exposes `typescript.sys`.
  - Leave all other consumers on the real TypeScript 7 package.
Why: TypeScript 7 does not yet ship a stable programmatic API for Svelte embedders.
     Update freely to TS 7 for `tsc`; this shim is a temporary bridge until upstream catches up.
How: NODE_OPTIONS='--require ./scripts/preload-typescript6-for-svelte.cjs' svelte-check …
*/

"use strict";

const Module = require("module");
const path = require("path");

const SVELTE_TOOLING =
  /(?:^|[/\\])(?:svelte-check|svelte-language-server|svelte2tsx|language-tools)(?:[/\\]|$)/;

const originalResolveFilename = Module._resolveFilename;

Module._resolveFilename = function patchedResolveFilename(request, parent, isMain, options) {
  if (
    (request === "typescript" || request.startsWith("typescript/")) &&
    parent?.filename &&
    SVELTE_TOOLING.test(parent.filename)
  ) {
    const mapped =
      request === "typescript"
        ? "@typescript/typescript6"
        : request.replace(/^typescript\//, "@typescript/typescript6/");
    try {
      return originalResolveFilename.call(this, mapped, parent, isMain, options);
    } catch {
      // Fall through to normal resolve if the TS6 compatibility package is missing.
    }
  }
  return originalResolveFilename.call(this, request, parent, isMain, options);
};

// Helpful once-per-process note for operators debugging check failures.
if (!process.env.GENOMICS_TS6_SHIM_QUIET) {
  const from = path.basename(__filename);
  // eslint-disable-next-line no-console
  console.error(`[${from}] Svelte tooling → @typescript/typescript6 (project typescript remains 7.x)`);
}
