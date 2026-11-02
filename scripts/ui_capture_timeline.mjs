#!/usr/bin/env node
// ./scripts/ui_capture_timeline.mjs
/**
 * Purpose: Capture periodic PNG frames of the running Tauri window through the
 *          loopback agent UI bridge (POST /ui/capture) and optionally tile them
 *          into one labelled contact sheet for startup/animation diagnosis.
 * How to run: `node scripts/ui_capture_timeline.mjs [--frames 24] [--interval-ms 500]
 *             [--out /tmp/gc-timeline] [--sheet] [--cols 6]`
 * Inputs: live endpoint (agent-ui.json or GENOMICS_AGENT_UI_URL, loopback only) and
 *         the session token when one exists. Start the app first so the first frames
 *         catch boot (see .cursor/skills/tauri-ui-visual-capture/SKILL.md).
 * Outputs: <out>/frame-NNN.png (+ timeline.json with elapsed ms) and, with --sheet,
 *          <out>/contact-sheet.png (needs python3 + Pillow; skipped with a warning).
 * Notes: Captured pixels may show genotype-derived UI. Keep output in /tmp or another
 *        untracked folder and use only the bundled sample fixture.
 */

import { spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { resolveAgentUiEndpoint } from "./lib/agentUiEndpoint.mjs";

const { values } = parseArgs({
  options: {
    frames: { type: "string", default: "24" },
    "interval-ms": { type: "string", default: "500" },
    out: { type: "string", default: "/tmp/gc-timeline" },
    sheet: { type: "boolean", default: false },
    cols: { type: "string", default: "6" },
    "wait-ms": { type: "string", default: "30000" },
  },
});

const frames = Math.max(1, Number.parseInt(values.frames, 10) || 24);
const intervalMs = Math.max(50, Number.parseInt(values["interval-ms"], 10) || 500);
const waitMs = Math.max(0, Number.parseInt(values["wait-ms"], 10) || 0);
const outDir = resolve(values.out);

const sleep = (ms) => new Promise((done) => setTimeout(done, ms));

/** Poll until the bridge endpoint file exists so a just-launched app is caught early. */
async function waitForEndpoint() {
  const deadline = Date.now() + waitMs;
  for (;;) {
    try {
      return await resolveAgentUiEndpoint();
    } catch (error) {
      if (Date.now() >= deadline) throw error;
      await sleep(100);
    }
  }
}

async function capture(endpoint) {
  const response = await fetch(`${endpoint.url}/ui/capture`, {
    method: "POST",
    headers: endpoint.token ? { "X-Genomics-Agent-Ui-Token": endpoint.token } : {},
  });
  const body = await response.json();
  if (!response.ok || !body.ok) throw new Error(body.error || `HTTP ${response.status}`);
  return Buffer.from(body.png_base64, "base64");
}

await mkdir(outDir, { recursive: true });
const endpoint = await waitForEndpoint();
const started = Date.now();
const timeline = [];

for (let index = 0; index < frames; index += 1) {
  const tick = Date.now();
  const name = `frame-${String(index).padStart(3, "0")}.png`;
  try {
    await writeFile(join(outDir, name), await capture(endpoint));
    timeline.push({ file: name, elapsed_ms: tick - started });
  } catch (error) {
    // Early frames can fail while the webview is still booting; record and continue.
    timeline.push({ file: null, elapsed_ms: tick - started, error: String(error.message || error) });
  }
  const spent = Date.now() - tick;
  if (index < frames - 1) await sleep(Math.max(0, intervalMs - spent));
}

await writeFile(join(outDir, "timeline.json"), JSON.stringify(timeline, null, 2));
const ok = timeline.filter((entry) => entry.file).length;
console.log(`captured ${ok}/${frames} frames -> ${outDir}`);

if (values.sheet) {
  const python = `
import json, sys
from pathlib import Path
from PIL import Image, ImageDraw
out = Path(sys.argv[1]); cols = int(sys.argv[2])
rows = [r for r in json.loads((out / "timeline.json").read_text()) if r.get("file")]
if not rows: sys.exit("no frames to tile")
thumbs = []
for row in rows:
    img = Image.open(out / row["file"]).convert("RGB"); img.thumbnail((480, 300))
    ImageDraw.Draw(img).text((6, 4), f'{row["elapsed_ms"]/1000:.1f}s', fill=(255, 220, 0))
    thumbs.append(img)
w, h = thumbs[0].size
grid = Image.new("RGB", (w * cols, h * ((len(thumbs) + cols - 1) // cols)), (20, 20, 20))
for i, t in enumerate(thumbs): grid.paste(t, ((i % cols) * w, (i // cols) * h))
grid.save(out / "contact-sheet.png"); print(out / "contact-sheet.png")
`;
  const result = spawnSync("python3", ["-c", python, outDir, values.cols], { encoding: "utf8" });
  if (result.status === 0) console.log(`contact sheet: ${result.stdout.trim()}`);
  else console.warn(`contact sheet skipped: ${(result.stderr || "python3/Pillow unavailable").trim()}`);
}
