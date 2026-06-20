// ./src/lib/utils/researchConnection.ts
/*
  Purpose: Helpers for Vector Research connection checks (localhost vs remote, invoke timeouts).
  Used by ResearchConnectionCard and related UI to auto-ping remote Qdrant/Ollama without stalling on dead localhost.
*/

const LOCAL_HOST_MARKERS = ["localhost", "127.0.0.1", "[::1]", "0.0.0.0"];

/** True when the URL points at this machine (skip aggressive auto-ping on tab open). */
export function isLocalServiceUrl(url: string): boolean {
  const lower = url.trim().toLowerCase();
  if (!lower) return true;
  return LOCAL_HOST_MARKERS.some((marker) => lower.includes(marker));
}

/** True when the URL looks like a remote HTTP(S) service (Tailscale, LAN, cloud). */
export function isRemoteServiceUrl(url: string): boolean {
  const trimmed = url.trim();
  if (!trimmed) return false;
  const lower = trimmed.toLowerCase();
  if (!lower.startsWith("http://") && !lower.startsWith("https://")) return false;
  return !isLocalServiceUrl(trimmed);
}

/** Whether tab open should run a background connection check without clicking Test Connections. */
export function shouldAutoCheckConnections(qdrantUrl: string, ollamaUrl: string, autoStart: boolean): boolean {
  if (autoStart) return true;
  return isRemoteServiceUrl(qdrantUrl) || isRemoteServiceUrl(ollamaUrl);
}

export async function withInvokeTimeout<T>(
  promise: Promise<T>,
  ms: number,
  label: string
): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error(`${label} timed out after ${Math.round(ms / 1000)}s`)),
          ms
        );
      }),
    ]);
  } finally {
    if (timer !== undefined) clearTimeout(timer);
  }
}

/** Shorter timeout for remote hosts; longer when user explicitly tests localhost. */
export function connectionCheckTimeoutMs(url: string, explicitTest: boolean): number {
  if (explicitTest) return isLocalServiceUrl(url) ? 12_000 : 10_000;
  return isRemoteServiceUrl(url) ? 8_000 : 12_000;
}
