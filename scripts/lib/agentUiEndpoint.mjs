// ./scripts/lib/agentUiEndpoint.mjs
/**
 * Purpose: Discover the live Genomics Caddy loopback endpoints without guessing
 *          localhost or port 17321.
 * How to run: imported by audit scripts and `pnpm run agent-ui:url`.
 * Inputs: env, optional 0600 runtime JSON under XDG_RUNTIME_DIR / LOCALAPPDATA.
 * Outputs: canonical http://127.0.0.1:<port> plus token when present.
 * Notes: `localhost` is rewritten to 127.0.0.1. Non-loopback URLs are refused.
 */

import { readFile } from "node:fs/promises";
import { tmpdir, userInfo } from "node:os";
import { join } from "node:path";

export const APP_RUNTIME_DIRNAME = "com.dna.explorer";
export const AGENT_UI_ENDPOINT_NAME = "agent-ui.json";
export const VITE_DEV_ENDPOINT_NAME = "vite-dev.json";
export const AGENT_UI_SERVICE = "genomics-caddy-agent-ui";
export const VITE_DEV_SERVICE = "genomics-caddy-vite-dev";
export const LOOPBACK_HOST = "127.0.0.1";

/** @param {unknown} host */
export function isLoopbackHostname(host) {
  const normalized = String(host || "")
    .trim()
    .replace(/^\[|\]$/g, "");
  return (
    normalized.toLowerCase() === "localhost" ||
    normalized === LOOPBACK_HOST ||
    normalized === "::1"
  );
}

/** @param {number} port */
export function loopbackUrl(port) {
  return `http://${LOOPBACK_HOST}:${port}`;
}

/** @param {unknown} raw */
export function canonicalizeLoopbackUrl(raw) {
  let parsed;
  try {
    parsed = new URL(String(raw || "").trim());
  } catch {
    throw new Error("invalid URL");
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error("agent UI URL must be http(s) on loopback");
  }
  const host = parsed.hostname;
  if (host === "0.0.0.0" || host === "::" || !isLoopbackHostname(host)) {
    throw new Error(`refusing non-loopback URL host ${host}; use 127.0.0.1`);
  }
  const port = parsed.port || (parsed.protocol === "https:" ? "443" : "80");
  return loopbackUrl(Number.parseInt(port, 10));
}

/** @param {NodeJS.ProcessEnv} [env] */
export function runtimeDirFromEnv(env = process.env) {
  const xdg = env.XDG_RUNTIME_DIR?.trim();
  if (xdg) return join(xdg, APP_RUNTIME_DIRNAME);
  const local = env.LOCALAPPDATA?.trim();
  if (local) return join(local, APP_RUNTIME_DIRNAME, "run");
  let uid = "user";
  try {
    uid = String(userInfo().uid ?? env.USERNAME ?? env.USER ?? "user");
  } catch {
    uid = env.USERNAME || env.USER || "user";
  }
  return join(tmpdir(), `${APP_RUNTIME_DIRNAME}-${uid}`);
}

/**
 * @param {NodeJS.ProcessEnv} [env]
 * @param {string} [fileName]
 */
export function endpointPathFromEnv(env = process.env, fileName = AGENT_UI_ENDPOINT_NAME) {
  return join(runtimeDirFromEnv(env), fileName);
}

/**
 * @param {unknown} value
 * @param {string} [expectedService]
 */
export function parseEndpointFile(value, expectedService = AGENT_UI_SERVICE) {
  if (!value || typeof value !== "object") {
    throw new Error("endpoint file is not an object");
  }
  const record = /** @type {Record<string, unknown>} */ (value);
  if (record.service !== expectedService) {
    throw new Error("endpoint file has the wrong service name");
  }
  const url = canonicalizeLoopbackUrl(record.url);
  const port = Number.parseInt(String(record.port ?? ""), 10);
  if (!Number.isInteger(port) || port <= 0 || loopbackUrl(port) !== url) {
    throw new Error("endpoint file URL does not match its port");
  }
  return {
    service: record.service,
    bind: LOOPBACK_HOST,
    port,
    url,
    pid: record.pid,
    auth_required: Boolean(record.auth_required),
    token: typeof record.token === "string" && record.token.trim() ? record.token.trim() : undefined,
    token_header:
      typeof record.token_header === "string" && record.token_header.trim()
        ? record.token_header.trim()
        : "X-Genomics-Agent-Ui-Token",
  };
}

/** @param {string} path */
async function defaultReadFile(path) {
  return readFile(path, "utf8");
}

/**
 * @param {string} url
 * @param {string} [token]
 */
async function defaultProbe(url, token) {
  const response = await fetch(`${url}/health`, {
    headers: {
      Accept: "application/json",
      ...(token ? { "X-Genomics-Agent-Ui-Token": token } : {}),
    },
    signal: AbortSignal.timeout(2000),
  });
  if (!response.ok) {
    throw new Error(`health HTTP ${response.status}`);
  }
  const body = await response.json();
  if (body?.service !== AGENT_UI_SERVICE) {
    throw new Error("health response is not the Genomics Caddy agent UI");
  }
  return body;
}

/**
 * @typedef {object} ResolveAgentUiOptions
 * @property {NodeJS.ProcessEnv} [env]
 * @property {(path: string) => Promise<string>} [readFile]
 * @property {(url: string, token?: string) => Promise<unknown>} [probe]
 * @property {boolean} [skipProbe]
 * @param {ResolveAgentUiOptions} [options]
 */
export async function resolveAgentUiEndpoint(options = {}) {
  const env = options.env ?? process.env;
  const read = options.readFile ?? defaultReadFile;
  const probe = options.probe;
  const tokenFromEnv = env.GENOMICS_AGENT_UI_TOKEN?.trim();

  if (env.GENOMICS_AGENT_UI_URL?.trim()) {
    const url = canonicalizeLoopbackUrl(env.GENOMICS_AGENT_UI_URL);
    if (probe) {
      await probe(url, tokenFromEnv);
    }
    return {
      url,
      token: tokenFromEnv,
      source: "env",
    };
  }

  const path = endpointPathFromEnv(env);
  try {
    const parsed = parseEndpointFile(JSON.parse(await read(path)));
    const token = tokenFromEnv || parsed.token;
    if (probe) {
      await probe(parsed.url, token);
    } else if (!options.skipProbe) {
      await defaultProbe(parsed.url, token);
    }
    return {
      ...parsed,
      token,
      source: "file",
      path,
    };
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(
      `No live Genomics Caddy agent UI endpoint. Start the desktop app (debug, or GENOMICS_AGENT_UI=1) and run pnpm run agent-ui:url. Do not guess localhost:17321. (${detail})`,
    );
  }
}

/**
 * @param {string} fileName
 * @param {Record<string, unknown>} payload
 * @param {NodeJS.ProcessEnv} [env]
 */
export async function writeLoopbackEndpoint(fileName, payload, env = process.env) {
  const { mkdir, writeFile, rename, chmod, unlink } = await import("node:fs/promises");
  const { randomBytes } = await import("node:crypto");
  const dir = runtimeDirFromEnv(env);
  await mkdir(dir, { recursive: true, mode: 0o700 });
  const path = join(dir, fileName);
  // Unique tmp so overlapping Vite configureServer hooks cannot rename the same file twice.
  const tmp = `${path}.${process.pid}.${randomBytes(6).toString("hex")}.tmp`;
  await writeFile(tmp, `${JSON.stringify(payload, null, 2)}\n`, { mode: 0o600 });
  try {
    await chmod(dir, 0o700);
    await chmod(tmp, 0o600);
  } catch {
    // Windows has no POSIX modes.
  }
  try {
    await rename(tmp, path);
  } catch (error) {
    await unlink(tmp).catch(() => {});
    throw error;
  }
  return path;
}
