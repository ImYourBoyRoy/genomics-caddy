#!/usr/bin/env node
/*
Purpose: Exercise the desktop Tauri update path against a disposable fixture.
How to run: `pnpm run audit:tauri-update`.
Outputs: Aggregate phase, status, and report-readiness results only.
Privacy: Uses a synthetic sample and the public liftover chain; never prints
sample names, genotype calls, local paths, or report text.
*/

import { access, copyFile, mkdir, mkdtemp, rm } from "node:fs/promises";
import { execFile, spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { promisify } from "node:util";
import { setTimeout as sleep } from "node:timers/promises";

const projectRoot = resolve(new URL("..", import.meta.url).pathname);
import { resolveAgentUiEndpoint } from "./lib/agentUiEndpoint.mjs";

let baseUrl = "";
let bridgeToken = "";
const timeoutMs = parsePositiveInteger(process.env.GENOMICS_TAURI_UPDATE_TIMEOUT_MS, 120_000);
const pollMs = 10;
const execFileAsync = promisify(execFile);
const seedExecutable = resolve(projectRoot, "src-tauri/target/debug/seed_mcp_fixture");
const liftoverSource = resolve(projectRoot, "App/Data/GRCh37_to_GRCh38.chain.gz");
const liftoverUrl = "https://hgdownload.soe.ucsc.edu/goldenPath/hg19/liftOver/hg19ToHg38.over.chain.gz";

function parsePositiveInteger(value, fallback) {
  const parsed = Number.parseInt(value || "", 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

async function request(path, options = {}) {
  const response = await fetch(`${baseUrl}${path}`, {
    ...options,
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(bridgeToken ? { "X-Genomics-Agent-Ui-Token": bridgeToken } : {}),
      ...(options.headers || {}),
    },
  });
  const body = await response.text();
  let payload;
  try {
    payload = body ? JSON.parse(body) : null;
  } catch {
    throw new Error(`Bridge returned non-JSON data for ${path}`);
  }
  if (!response.ok) throw new Error(`Bridge request failed for ${path} (HTTP ${response.status})`);
  return payload;
}

async function waitFor(probe, label) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const result = await probe();
    if (result) return result;
    await sleep(pollMs);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function queryText(text) {
  return request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text }),
  });
}

async function clickText(text) {
  const result = await request("/ui/clickText", {
    method: "POST",
    body: JSON.stringify({ text }),
  });
  assert(result?.ok === true, `Could not activate ${text}`);
}

async function prepareFixture() {
  const fixtureDir = await mkdtemp(join(tmpdir(), "genomics-caddy-tauri-update-"));
  try {
    await access(seedExecutable);
    await access(liftoverSource);
    await execFileAsync(seedExecutable, [fixtureDir], {
      cwd: projectRoot,
      timeout: timeoutMs,
      maxBuffer: 1_000_000,
    });

    await copyFile(liftoverSource, join(fixtureDir, "GRCh37_to_GRCh38.chain.gz"));
    const referenceDb = join(fixtureDir, "genomics_reference.db");
    const seedSql = `
      INSERT OR REPLACE INTO offline_asset_registry (
        asset_id, tier, local_path, source_url, remote_etag, remote_last_modified,
        remote_content_length, local_bytes, row_count, version_label, synced_at, update_available
      ) VALUES (
        'liftover_chain', 0, 'GRCh37_to_GRCh38.chain.gz', '${liftoverUrl}',
        '"stale-fixture-etag"', 'Tue, 01 Jan 2019 00:00:00 GMT',
        227698, 285250, 0, 'GRCh37_to_GRCh38.chain.gz', 1, 0
      );
    `;
    await execFileAsync("sqlite3", [referenceDb, seedSql], {
      cwd: projectRoot,
      timeout: 10_000,
      maxBuffer: 1_000_000,
    });
    return fixtureDir;
  } catch (error) {
    await rm(fixtureDir, { recursive: true, force: true });
    throw error;
  }
}

async function stopProcess(child) {
  if (!child?.pid) return;
  try {
    process.kill(-child.pid, "SIGINT");
  } catch {
    try { child.kill("SIGINT"); } catch { /* already stopped */ }
  }
  await Promise.race([
    new Promise((resolveExit) => child.once("close", resolveExit)),
    sleep(3_000),
  ]);
  if (child.exitCode === null) {
    try { process.kill(-child.pid, "SIGTERM"); } catch { /* already stopped */ }
  }
}

async function assertNoUpdateCopy() {
  for (const text of ["newer remote", "Update available", "updates available"]) {
    const result = await queryText(text);
    assert(result?.ok === true && result.count === 0, `Update copy remained visible after sync: ${text}`);
  }
}

async function ensureBridge() {
  if (baseUrl) return true;
  try {
    const endpoint = await resolveAgentUiEndpoint();
    baseUrl = endpoint.url;
    bridgeToken = endpoint.token || "";
    return true;
  } catch {
    return false;
  }
}

async function waitForReport() {
  return waitFor(
    async () => {
      try {
        if (!(await ensureBridge())) return null;
        const snapshot = await request("/ui/snapshot");
        return snapshot?.hasReport === true && snapshot?.sample ? snapshot : null;
      } catch {
        return null;
      }
    },
    "the disposable Tauri report",
  );
}

async function waitForAvailable() {
  let panelOpened = false;
  let nextRetryAt = 0;
  return waitFor(
    async () => {
      try {
        const snapshot = await request("/ui/snapshot");
        if (snapshot?.resourceUpdatePhase === "available") return snapshot;
        if (snapshot?.resourceUpdatePhase === "error" && Date.now() >= nextRetryAt) {
          if (!panelOpened) {
            await clickText("Data & updates");
            panelOpened = true;
          }
          const retry = await queryText("Retry");
          if (retry?.count > 0) {
            await clickText("Retry");
            nextRetryAt = Date.now() + 1_000;
          }
        }
        return null;
      } catch {
        return null;
      }
    },
    "the disposable liftover update state",
  );
}

async function waitForBridgeStopped() {
  const started = Date.now();
  while (Date.now() - started < 10_000) {
    try {
      await request("/ui/snapshot");
    } catch {
      return;
    }
    await sleep(100);
  }
  throw new Error("Tauri fixture process did not stop cleanly");
}

async function waitForPhase(phase, label) {
  return waitFor(
    async () => {
      const snapshot = await request("/ui/snapshot");
      return snapshot?.resourceUpdatePhase === phase ? snapshot : null;
    },
    label,
  );
}

async function runDesktopUpdateFlow(simulateFailure) {
  const fixtureDir = await prepareFixture();
  let child;
  const phases = new Set();
  try {
    child = spawn("pnpm", ["run", "tauri:dev"], {
      cwd: projectRoot,
      env: { ...process.env, GENOMICS_DATA_DIR: fixtureDir },
      stdio: ["ignore", "ignore", "ignore"],
      detached: true,
    });
    baseUrl = "";
    bridgeToken = "";
    await waitForReport();
    const available = await waitForAvailable();
    phases.add(available.resourceUpdatePhase);
    assert(available.resourceStatus !== "error", "Fixture resource probe failed before update action");

    const openUpdatePanel = await queryText("Force re-download existing files");
    if (openUpdatePanel?.count === 0) await clickText("Data & updates");
    await waitFor(
      async () => {
        const result = await queryText("Update chain");
        return result?.count > 0;
      },
      "the liftover Update action",
    );

    const partialPath = join(fixtureDir, "GRCh37_to_GRCh38.chain.gz.part");
    if (simulateFailure) await mkdir(partialPath);
    await clickText("Update chain");

    if (simulateFailure) {
      const failed = await waitForPhase("error", "the terminal error update state");
      phases.add(failed.resourceUpdatePhase);
      try {
        await waitFor(
          async () => {
            const retry = await queryText("Retry");
            return retry?.ok === true && retry.count > 0 ? retry : null;
          },
          "the rendered Retry control",
        );
      } catch (error) {
        const snapshot = await request("/ui/snapshot").catch(() => null);
        const updateError = await queryText("Update error").catch(() => ({ count: -1 }));
        const retrying = await queryText("Retrying").catch(() => ({ count: -1 }));
        throw new Error(`${error.message}; phase=${snapshot?.resourceUpdatePhase ?? "unknown"}; resource=${snapshot?.resourceStatus ?? "unknown"}; update_error=${updateError.count}; retrying=${retrying.count}`);
      }
      await rm(partialPath, { recursive: true, force: true });
      await clickText("Retry");
    }

    const ready = await waitFor(
      async () => {
        const snapshot = await request("/ui/snapshot");
        if (snapshot?.resourceUpdatePhase) phases.add(snapshot.resourceUpdatePhase);
        return snapshot?.resourceUpdatePhase === "ready" ? snapshot : null;
      },
      simulateFailure ? "the recovered ready update state" : "the terminal ready update state",
    );
    assert(ready.hasReport === true, "Report was not ready after resource sync");
    assert(ready.resourceStatus !== "error", "Resource status ended in error after successful sync");
    await assertNoUpdateCopy();
    return { initialPhase: available.resourceUpdatePhase, finalPhase: ready.resourceUpdatePhase, phases };
  } finally {
    await stopProcess(child);
    await waitForBridgeStopped();
    await rm(fixtureDir, { recursive: true, force: true });
  }
}

async function main() {
  const success = await runDesktopUpdateFlow(false);
  await sleep(1_000);
  const recovery = await runDesktopUpdateFlow(true);
  assert(recovery.phases.has("error"), "Recovery flow did not observe an error phase");
  console.log("PASS: Tauri desktop update flow audit");
  console.log(`  resource=liftover_chain; success_phases=${[...success.phases].join(",")}; recovery_phases=${[...recovery.phases].join(",")}; update_copy=0; report=ready`);
}

main().catch((error) => {
  console.error(`FAIL: Tauri desktop update flow audit — ${error.message}`);
  process.exitCode = 1;
});
