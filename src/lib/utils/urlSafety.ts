// ./src/lib/utils/urlSafety.ts
/**
 * Safe URL helpers for dynamic href attributes in the Tauri webview.
 */

const SAFE_HREF_RE = /^https?:\/\//i;

export function safeExternalHref(url: string | null | undefined): string | null {
  if (!url) return null;
  // Reject control chars, whitespace, and attribute-breaking characters anywhere in the raw value.
  if (/[\s\x00-\x1f"'<>]/.test(url)) return null;
  const trimmed = url.trim();
  if (!SAFE_HREF_RE.test(trimmed)) return null;
  return trimmed;
}

/** Parse "Title (https://example.com)" citation strings. */
export function parseCitationUrl(citation: string): { title: string; url: string | null } {
  if (!citation.includes("http")) {
    return { title: citation, url: null };
  }
  const parts = citation.split(" (");
  const title = parts[0]?.trim() || citation;
  const rawUrl = parts[1] ? parts[1].replace(/\)$/, "").trim() : "";
  return { title, url: safeExternalHref(rawUrl) };
}
