// ./scripts/lib/agentUiEndpoint.test.ts
import { describe, expect, it } from "vitest";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  AGENT_UI_ENDPOINT_NAME,
  VITE_DEV_ENDPOINT_NAME,
  VITE_DEV_SERVICE,
  canonicalizeLoopbackUrl,
  endpointPathFromEnv,
  isLoopbackHostname,
  parseEndpointFile,
  resolveAgentUiEndpoint,
  writeLoopbackEndpoint,
} from "./agentUiEndpoint.mjs";

describe("canonicalizeLoopbackUrl", () => {
  it("rewrites localhost and ::1 to 127.0.0.1", () => {
    expect(canonicalizeLoopbackUrl("http://localhost:1420")).toBe("http://127.0.0.1:1420");
    expect(canonicalizeLoopbackUrl("http://[::1]:48123/")).toBe("http://127.0.0.1:48123");
  });

  it("rejects non-loopback and wildcard binds", () => {
    expect(() => canonicalizeLoopbackUrl("http://0.0.0.0:1420")).toThrow(/loopback/i);
    expect(() => canonicalizeLoopbackUrl("http://192.168.1.21:17321")).toThrow(/loopback/i);
    expect(() => canonicalizeLoopbackUrl("https://example.com")).toThrow(/loopback/i);
  });
});

describe("isLoopbackHostname", () => {
  it("accepts only loopback names", () => {
    expect(isLoopbackHostname("127.0.0.1")).toBe(true);
    expect(isLoopbackHostname("localhost")).toBe(true);
    expect(isLoopbackHostname("::1")).toBe(true);
    expect(isLoopbackHostname("0.0.0.0")).toBe(false);
    expect(isLoopbackHostname("example.local")).toBe(false);
  });
});

describe("endpointPathFromEnv", () => {
  it("prefers XDG_RUNTIME_DIR over temp fallbacks", () => {
    const path = endpointPathFromEnv({
      XDG_RUNTIME_DIR: "/run/user/1000",
      LOCALAPPDATA: "C:\\Users\\x\\AppData\\Local",
    });
    expect(path.replaceAll("\\", "/")).toBe(`/run/user/1000/com.dna.explorer/${AGENT_UI_ENDPOINT_NAME}`);
  });
});

describe("parseEndpointFile", () => {
  it("requires the genomics agent service and a loopback URL", () => {
    const parsed = parseEndpointFile({
      service: "genomics-caddy-agent-ui",
      bind: "127.0.0.1",
      port: 48123,
      url: "http://localhost:48123",
      pid: 42,
      token: "secret-token",
    });
    expect(parsed.url).toBe("http://127.0.0.1:48123");
    expect(parsed.port).toBe(48123);
    expect(parsed.token).toBe("secret-token");
  });

  it("rejects the old hardcoded 17321 fallback when the file is not live", () => {
    expect(() =>
      parseEndpointFile({
        service: "something-else",
        url: "http://127.0.0.1:17321",
        port: 17321,
      }),
    ).toThrow(/service/i);
  });
});

describe("resolveAgentUiEndpoint", () => {
  it("uses GENOMICS_AGENT_UI_URL when it is loopback and does not default to 17321", async () => {
    const resolved = await resolveAgentUiEndpoint({
      env: {
        GENOMICS_AGENT_UI_URL: "http://localhost:48200",
        GENOMICS_AGENT_UI_TOKEN: "from-env",
      },
      readFile: async () => {
        throw new Error("should not read a fallback file");
      },
      probe: async () => ({ ok: true, service: "genomics-caddy-agent-ui", port: 48200 }),
    });
    expect(resolved.url).toBe("http://127.0.0.1:48200");
    expect(resolved.token).toBe("from-env");
    expect(resolved.url).not.toContain("17321");
    expect(resolved.url).not.toContain("localhost");
  });

  it("fails closed when no live endpoint exists instead of guessing 17321", async () => {
    await expect(
      resolveAgentUiEndpoint({
        env: {},
        readFile: async () => {
          throw Object.assign(new Error("ENOENT"), { code: "ENOENT" });
        },
        probe: async () => {
          throw new Error("connection refused");
        },
      }),
    ).rejects.toThrow(/live Genomics Caddy/i);
  });
});

describe("writeLoopbackEndpoint", () => {
  it("keeps a readable endpoint when overlapping writers share a runtime dir", async () => {
    const root = await mkdtemp(join(tmpdir(), "gc-endpoint-"));
    const env = { XDG_RUNTIME_DIR: root };
    await Promise.all(
      Array.from({ length: 8 }, (_, index) =>
        writeLoopbackEndpoint(
          VITE_DEV_ENDPOINT_NAME,
          {
            service: VITE_DEV_SERVICE,
            bind: "127.0.0.1",
            port: 1420 + index,
            url: `http://127.0.0.1:${1420 + index}`,
            pid: process.pid,
          },
          env,
        ),
      ),
    );
    const text = await readFile(join(root, "com.dna.explorer", VITE_DEV_ENDPOINT_NAME), "utf8");
    const parsed = JSON.parse(text);
    expect(parsed.service).toBe(VITE_DEV_SERVICE);
    expect(String(parsed.url)).toMatch(/^http:\/\/127\.0\.0\.1:\d+$/);
  });
});
