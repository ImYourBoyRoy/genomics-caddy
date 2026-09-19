#!/usr/bin/env node
// ./scripts/print_agent_ui_endpoint.mjs
/**
 * Purpose: Print the live Genomics Caddy loopback URL for local agents/tools.
 * How to run: `pnpm run agent-ui:url`
 * Optional: `--print-token` includes the session token (do not log this in shared output).
 * Inputs: runtime agent-ui.json or GENOMICS_AGENT_UI_URL (loopback only).
 * Outputs: http://127.0.0.1:<port> plus token=present|none. Never guesses port 17321.
 */

import { readFile } from "node:fs/promises";
import {
  VITE_DEV_ENDPOINT_NAME,
  VITE_DEV_SERVICE,
  endpointPathFromEnv,
  parseEndpointFile,
  resolveAgentUiEndpoint,
} from "./lib/agentUiEndpoint.mjs";

const printToken = process.argv.includes("--print-token");

try {
  const endpoint = await resolveAgentUiEndpoint();
  console.log(endpoint.url);
  if (printToken && endpoint.token) {
    console.log(endpoint.token);
  } else {
    console.log(endpoint.token ? "token=present" : "token=none");
  }
} catch (error) {
  const detail = error instanceof Error ? error.message : String(error);
  console.error(detail);
  try {
    const vitePath = endpointPathFromEnv(process.env, VITE_DEV_ENDPOINT_NAME);
    const vite = parseEndpointFile(JSON.parse(await readFile(vitePath, "utf8")), VITE_DEV_SERVICE);
    console.error(`Vite preview (not the desktop agent bridge): ${vite.url}`);
  } catch {
    // Vite is also down.
  }
  process.exit(1);
}
