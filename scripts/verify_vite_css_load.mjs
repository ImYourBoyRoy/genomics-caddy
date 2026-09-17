#!/usr/bin/env node
// ./scripts/verify_vite_css_load.mjs
/**
 * Purpose: Fail if Vite emits "failed to load virtual css module" for our Svelte components.
 * How to run: `pnpm run verify:css` (starts a temporary Vite dev server on :1420 — must be free).
 * Inputs: none.
 * Outputs: exit 0 on clean load; exit 1 with matching log lines on failure.
 */

import { spawn } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";
import { createWriteStream } from "node:fs";
import { readdir, readFile, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const logPath = path.join(root, "tmp-vite-css-verify.log");
const port = 1420;

async function listSvelteFiles(dir) {
  const out = [];
  const entries = await readdir(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) out.push(...(await listSvelteFiles(full)));
    else if (e.name.endsWith(".svelte")) out.push(full);
  }
  return out;
}

async function waitForServer(timeoutMs = 60000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/`);
      if (res.status) return;
    } catch {
      /* retry */
    }
    await sleep(250);
  }
  throw new Error(`Vite did not become ready on 127.0.0.1:${port}`);
}

async function main() {
  await rm(path.join(root, "node_modules/.vite"), { recursive: true, force: true });
  await rm(logPath, { force: true });

  const all = await listSvelteFiles(path.join(root, "src"));
  const withStyle = [];
  for (const abs of all) {
    const text = await readFile(abs, "utf8");
    if (text.includes("<style")) withStyle.push(path.relative(root, abs));
  }
  // Always include page shell + extracted-css panels.
  const components = [
    ...new Set([
      "src/routes/+page.svelte",
      "src/lib/components/sidebar/Sidebar.svelte",
      "src/lib/components/settings/ConnectionsPanel.svelte",
      "src/lib/components/ai/AiAssistantPanel.svelte",
      "src/lib/components/agent/AgentResearchPanel.svelte",
      "src/lib/components/search/VariantSearchPanel.svelte",
      ...withStyle,
    ]),
  ];

  const logStream = createWriteStream(logPath, { flags: "w" });
  const child = spawn("pnpm", ["run", "dev"], {
    cwd: root,
    env: {
      ...process.env,
      FORCE_COLOR: "0",
      // vite.config uses TAURI_DEV_HOST; bind IPv4 so CI fetch(127.0.0.1) works.
      TAURI_DEV_HOST: "127.0.0.1",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  child.stdout.pipe(logStream);
  child.stderr.pipe(logStream);

  let failed = false;
  try {
    await waitForServer();
    await fetch(`http://127.0.0.1:${port}/`);

    // Parallel load (reproduces Vite 8 virtual-CSS races when emitCss is external).
    await Promise.all(
      components.map(async (rel) => {
        const res = await fetch(`http://127.0.0.1:${port}/${rel}`);
        if (!res.ok) throw new Error(`HTTP ${res.status} for ${rel}`);
        const text = await res.text();
        const styleUrls = [...text.matchAll(/["']([^"']+\?svelte&type=style&lang\.css)["']/g)].map(
          (m) => m[1]
        );
        await Promise.all(
          styleUrls.map(async (u) => {
            const url = u.startsWith("http") ? u : `http://127.0.0.1:${port}${u}`;
            await fetch(url);
          })
        );
      })
    );
    await sleep(800);

    const log = await readFile(logPath, "utf8");
    const hits = log.split("\n").filter((line) => line.includes("failed to load virtual css module"));
    if (hits.length) {
      failed = true;
      console.error(`FAIL: ${hits.length} virtual CSS load warning(s):`);
      for (const line of hits.slice(0, 30)) console.error(`  ${line}`);
      if (hits.length > 30) console.error(`  … +${hits.length - 30} more`);
    } else {
      console.log(
        `PASS: no vite-plugin-svelte virtual CSS load failures (${components.length} modules warmed).`
      );
    }
  } finally {
    child.kill("SIGTERM");
    await sleep(400);
    if (!child.killed) child.kill("SIGKILL");
    logStream.end();
  }

  process.exit(failed ? 1 : 0);
}

main().catch(async (err) => {
  console.error(err);
  try {
    const log = await readFile(logPath, "utf8");
    const tail = log.trim().split("\n").slice(-80).join("\n");
    if (tail) {
      console.error("Vite log tail:");
      console.error(tail);
    }
  } catch {
    // The log file is optional diagnostic output.
  }
  process.exit(1);
});
