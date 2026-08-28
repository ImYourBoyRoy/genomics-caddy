#!/usr/bin/env node
/*
Purpose: Exercise the compiled Genomics Caddy MCP process without exposing
private report or genotype content.
Usage: `npm run audit:mcp` after a local Tauri build, or set
GENOMICS_MCP_EXECUTABLE to an alternate compiled binary.
Privacy: Only protocol/tool counts and aggregate status fields are printed.
*/

import { access } from "node:fs/promises";
import { resolve } from "node:path";
import { spawn } from "node:child_process";

const projectRoot = resolve(new URL("..", import.meta.url).pathname);
const executable = process.env.GENOMICS_MCP_EXECUTABLE
  ? resolve(projectRoot, process.env.GENOMICS_MCP_EXECUTABLE)
  : resolve(projectRoot, "src-tauri/target/debug/DNA-Tools");
const timeoutMs = parsePositiveInteger(process.env.GENOMICS_MCP_AUDIT_TIMEOUT_MS, 120_000);
const configuredSampleId = parseOptionalInteger(process.env.GENOMICS_MCP_SAMPLE_ID);
const authToken = process.env.GENOMICS_MCP_TOKEN?.trim() || "";

function parsePositiveInteger(value, fallback) {
  const parsed = Number.parseInt(value || "", 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function parseOptionalInteger(value) {
  if (!value) return null;
  const parsed = Number.parseInt(value, 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : null;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function withAuth(params = {}) {
  return authToken ? { ...params, _meta: { authToken } } : params;
}

function summarizeStatus(status) {
  const tiers = Array.isArray(status?.tiers) ? status.tiers : [];
  const assets = tiers.flatMap((tier) => Array.isArray(tier?.assets) ? tier.assets : []);
  return {
    tiers: tiers.length,
    assets: assets.length,
    localAssets: assets.filter((asset) => asset?.local_present === true).length,
    updates: Number.isInteger(status?.total_updates_available) ? status.total_updates_available : -1,
    indexedRows: assets.reduce((total, asset) => {
      const rows = Number.isInteger(asset?.row_count) && asset.row_count > 0 ? asset.row_count : 0;
      return total + rows;
    }, 0),
  };
}

function toolPayload(result, method) {
  const text = result?.content?.find((item) => item?.type === "text")?.text;
  assert(typeof text === "string" && text.length > 0, `${method} returned no text payload`);
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`${method} returned an invalid JSON tool payload`);
  }
}

async function run() {
  await access(executable);
  const child = spawn(executable, ["--mcp"], {
    cwd: projectRoot,
    env: process.env,
    stdio: ["pipe", "pipe", "pipe"],
  });

  let nextId = 1;
  let buffered = "";
  let settled = false;
  const pending = new Map();

  const rejectPending = (error) => {
    for (const { reject } of pending.values()) reject(error);
    pending.clear();
  };

  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    buffered += chunk;
    let newline;
    while ((newline = buffered.indexOf("\n")) >= 0) {
      const line = buffered.slice(0, newline).trim();
      buffered = buffered.slice(newline + 1);
      if (!line) continue;
      let response;
      try {
        response = JSON.parse(line);
      } catch {
        rejectPending(new Error("MCP emitted a non-JSON response"));
        return;
      }
      const request = pending.get(response.id);
      if (!request) continue;
      pending.delete(response.id);
      if (response.error) {
        request.reject(new Error(`MCP ${request.method} returned an error`));
      } else {
        request.resolve(response.result);
      }
    }
  });
  // Drain stderr for a clean child-process lifecycle, but never print it:
  // diagnostics can contain local paths or other user-environment details.
  child.stderr.resume();
  child.on("error", (error) => rejectPending(new Error(`MCP process could not start: ${error.message}`)));
  child.on("close", () => {
    if (!settled) rejectPending(new Error("MCP process exited before the audit completed"));
  });

  const request = (method, params = {}) => {
    const id = nextId++;
    return new Promise((resolveResponse, rejectResponse) => {
      pending.set(id, { method, resolve: resolveResponse, reject: rejectResponse });
      child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
    });
  };

  const requestWithTimeout = async (method, params = {}) => {
    let timer;
    try {
      return await Promise.race([
        request(method, params),
        new Promise((_, reject) => {
          timer = setTimeout(() => reject(new Error(`${method} timed out`)), timeoutMs);
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
  };

  try {
    await requestWithTimeout("initialize", withAuth());
    const toolCatalog = await requestWithTimeout("tools/list", withAuth());
    const toolNames = new Set(
      Array.isArray(toolCatalog?.tools)
        ? toolCatalog.tools.map((tool) => tool?.name).filter((name) => typeof name === "string")
        : [],
    );
    for (const required of ["get_offline_update_status", "reload_report"]) {
      assert(toolNames.has(required), `MCP tool catalog is missing ${required}`);
    }

    const status = toolPayload(
      await requestWithTimeout("tools/call", withAuth({
        name: "get_offline_update_status",
        arguments: {},
      })),
      "get_offline_update_status",
    );
    const statusSummary = summarizeStatus(status);
    assert(statusSummary.tiers > 0, "MCP status returned no resource tiers");
    assert(statusSummary.updates >= 0, "MCP status omitted its update count");

    const samples = toolPayload(
      await requestWithTimeout("tools/call", withAuth({
        name: "list_samples",
        arguments: {},
      })),
      "list_samples",
    );
    assert(Array.isArray(samples), "MCP list_samples returned an invalid result");
    const sampleId = configuredSampleId ?? samples.find((sample) => Number.isInteger(sample?.id))?.id;
    assert(Number.isInteger(sampleId) && sampleId > 0, "MCP audit requires an imported sample");

    const reload = toolPayload(
      await requestWithTimeout("tools/call", withAuth({
        name: "reload_report",
        arguments: { sample_id: sampleId },
      })),
      "reload_report",
    );
    assert(reload?.status === "ready", "MCP report reload did not reach status=ready");
    assert(Number.isInteger(reload?.sample_id), "MCP report reload omitted sample identity");
    assert(Array.isArray(reload?.report?.sections), "MCP report reload omitted report sections");

    console.log("PASS: MCP runtime audit");
    console.log(
      `  tools=${toolNames.size}; tiers=${statusSummary.tiers}; assets=${statusSummary.assets}; ` +
      `local_assets=${statusSummary.localAssets}; updates=${statusSummary.updates}; ` +
      `indexed_rows=${statusSummary.indexedRows}; reload=ready; sections=${reload.report.sections.length}`,
    );
  } finally {
    settled = true;
    child.stdin.end();
    if (!child.killed) child.kill();
  }
}

run().catch((error) => {
  console.error(`FAIL: MCP runtime audit — ${error.message}`);
  process.exitCode = 1;
});
