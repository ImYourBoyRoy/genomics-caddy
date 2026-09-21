#!/usr/bin/env node
// ./scripts/capture_readme_screenshots.mjs
/**
 * Purpose: Capture README gallery frames (dark + light) and a looping WebP.
 * How to run: start the desktop app with the synthetic example profile and a
 *             public-safe library path (no /home/username), then
 *             `node ./scripts/capture_readme_screenshots.mjs`
 * Inputs: live agent-UI endpoint (`pnpm run agent-ui:url`).
 * Outputs: `docs/images/readme-*-dark.png`, `readme-*-light.png`, and
 *          `readme-*.webp` (animated dark→light). Aggregate logs only.
 * Notes: Waits out the 2500ms theme veil before capture. Keeps the data
 *        sidebar visible (import, profiles, Data & updates). Aborts if
 *        home-directory or LAN-looking text is on screen. Never prints
 *        genotype values, profile names, or image payloads.
 */

import { mkdirSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as sleep } from "node:timers/promises";
import { resolveAgentUiEndpoint } from "./lib/agentUiEndpoint.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "docs/images");
const THEME_TRANSITION_MS = 3200;
const SIDEBAR_MS = 450;
const shots = [
  { file: "readme-report-simple", tab: "report", click: "Simple" },
  { file: "readme-chromosome-map", tab: "map", waitFor: "Genotyped SNPs", settleMs: 800 },
];

let endpoint;

async function ui(method, body) {
  const path = method === "snapshot" ? "/ui/snapshot" : `/ui/${method}`;
  const response = await fetch(`${endpoint.url}${path}`, {
    method: method === "snapshot" ? "GET" : "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      ...(endpoint.token ? { "X-Genomics-Agent-Ui-Token": endpoint.token } : {}),
    },
    body: method === "snapshot" ? undefined : JSON.stringify(body ?? {}),
  });
  const payload = await response.json();
  if (!response.ok) {
    const detail = typeof payload?.error === "string" ? payload.error : `HTTP ${response.status}`;
    throw new Error(`${method} ${detail}`);
  }
  return payload;
}

function summarizeSnapshot(snapshot) {
  return {
    hasReport: Boolean(snapshot?.hasReport),
    tab: snapshot?.activeTab ?? "",
    mode: snapshot?.layout?.activePresentationMode ?? null,
    sections: snapshot?.sectionCount ?? 0,
    cards: snapshot?.layout?.markerCardCount ?? 0,
  };
}

async function clickOptional(text) {
  const result = await ui("clickText", { text });
  return result?.ok !== false;
}

async function setSidebarVisible(visible) {
  await clickOptional(visible ? "Show data sidebar" : "Hide data sidebar");
  await sleep(SIDEBAR_MS);
}

async function applyTheme(shortLabel) {
  await setSidebarVisible(true);
  const clicked = await ui("clickText", { text: shortLabel });
  if (clicked?.ok === false) {
    throw new Error(`could not select ${shortLabel} theme`);
  }
  await sleep(THEME_TRANSITION_MS);
  await setSidebarVisible(true);
}

async function ensureDataUpdatesExpanded(wantOpen) {
  const sync = await ui("queryText", { text: "Sync All Missing" });
  const open = Number(sync?.count) > 0;
  if (wantOpen === open) return;
  const clicked = await ui("clickText", { text: "Data & updates" });
  if (clicked?.ok === false) {
    throw new Error("could not toggle Data & updates");
  }
  await sleep(500);
}

async function settleCatalogStatus() {
  await ensureDataUpdatesExpanded(true);
  await clickOptional("Retry");
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline) {
    const current = await ui("queryText", { text: "Current" });
    if (Number(current?.count) > 0) break;
    await sleep(1000);
  }
  await ensureDataUpdatesExpanded(false);
}

async function assertPublicSafe() {
  const needles = ["/home/", "Users\\", "192.168.", "10.0.", "172.16."];
  for (const needle of needles) {
    const result = await ui("queryText", { text: needle });
    if (Number(result?.count) > 0) {
      throw new Error(`public screenshot would leak on-screen text matching ${JSON.stringify(needle)}`);
    }
  }
}

async function capturePng(pngPath) {
  mkdirSync(outDir, { recursive: true });
  const payload = await ui("capture", {});
  const byteLen = Number(payload?.byte_len) || 0;
  const encoded = typeof payload?.png_base64 === "string" ? payload.png_base64 : "";
  if (!encoded || byteLen < 8_000) {
    throw new Error(`capture too small (${byteLen} bytes)`);
  }
  writeFileSync(pngPath, Buffer.from(encoded, "base64"));
  console.log(`captured ${byteLen} bytes -> ${pngPath.split("/").pop()}`);
}

function writeAnimatedWebp(base) {
  const helper = join(root, "scripts/lib/make_theme_webp.py");
  const result = spawnSync(
    "python3",
    [
      helper,
      "--dark",
      join(outDir, `${base}-dark.png`),
      "--light",
      join(outDir, `${base}-light.png`),
      "--out",
      join(outDir, `${base}.webp`),
    ],
    { encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error((result.stderr || result.stdout || "animated webp encode failed").trim());
  }
  process.stdout.write(result.stdout);
}

async function waitForReport() {
  const deadline = Date.now() + 180_000;
  while (Date.now() < deadline) {
    try {
      const snapshot = await ui("snapshot");
      const summary = summarizeSnapshot(snapshot);
      if (summary.hasReport && summary.sections > 0) return summary;
    } catch {
      // Window may still be booting.
    }
    await sleep(1000);
  }
  throw new Error("report did not become ready for screenshots");
}

async function waitForVisibleText(needle, timeoutMs = 30_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const result = await ui("queryText", { text: needle });
    if (Number(result?.count) > 0) return;
    await sleep(1000);
  }
  throw new Error(`screenshot wait expired before seeing ${JSON.stringify(needle)}`);
}

async function captureTheme(theme) {
  for (const shot of shots) {
    await ui("setTab", { tab: shot.tab });
    if (shot.click) {
      const clicked = await ui("clickText", { text: shot.click });
      if (clicked?.ok === false) {
        throw new Error(`could not open ${shot.click}`);
      }
    }
    if (shot.waitFor) await waitForVisibleText(shot.waitFor);
    await ensureDataUpdatesExpanded(false);
    await sleep(shot.settleMs ?? 700);
    const summary = summarizeSnapshot(await ui("snapshot"));
    console.log(`capture_readme_screenshots: ${shot.file} ${theme} tab=${summary.tab} mode=${summary.mode}`);
    await assertPublicSafe();
    await sleep(250);
    await capturePng(join(outDir, `${shot.file}-${theme}.png`));
  }
}

async function main() {
  endpoint = await resolveAgentUiEndpoint();
  try {
    await ui("resize", { width: 1600, height: 1000 });
  } catch (error) {
    console.log(`capture_readme_screenshots: resize skipped (${error instanceof Error ? error.message : error})`);
  }
  await sleep(400);
  const ready = await waitForReport();
  console.log(`capture_readme_screenshots: report ready sections=${ready.sections} cards=${ready.cards}`);

  await applyTheme("Dark");
  await settleCatalogStatus();
  await captureTheme("dark");
  await applyTheme("Light");
  await captureTheme("light");
  for (const shot of shots) {
    writeAnimatedWebp(shot.file);
  }
  console.log(`capture_readme_screenshots: wrote ${shots.length} animated WebP galleries under docs/images/`);
}

try {
  await main();
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
