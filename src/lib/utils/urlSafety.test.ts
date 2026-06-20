// ./src/lib/utils/urlSafety.test.ts
import { describe, expect, it } from "vitest";
import { parseCitationUrl, safeExternalHref } from "./urlSafety";

describe("safeExternalHref", () => {
  it("allows http and https URLs", () => {
    expect(safeExternalHref("https://example.com/path")).toBe("https://example.com/path");
    expect(safeExternalHref("http://localhost:8080")).toBe("http://localhost:8080");
  });

  it("rejects javascript and other schemes", () => {
    expect(safeExternalHref("javascript:alert(1)")).toBeNull();
    expect(safeExternalHref("data:text/html,hi")).toBeNull();
    expect(safeExternalHref("file:///etc/passwd")).toBeNull();
  });

  it("rejects URLs with unsafe characters", () => {
    expect(safeExternalHref('https://evil.com"><script')).toBeNull();
    expect(safeExternalHref("https://evil.com\n")).toBeNull();
  });
});

describe("parseCitationUrl", () => {
  it("extracts title and safe URL from citation strings", () => {
    const parsed = parseCitationUrl("GWAS Catalog (https://www.ebi.ac.uk/gwas/studies/GCST123)");
    expect(parsed.title).toBe("GWAS Catalog");
    expect(parsed.url).toBe("https://www.ebi.ac.uk/gwas/studies/GCST123");
  });

  it("returns null URL when citation has no link", () => {
    expect(parseCitationUrl("Local reference only").url).toBeNull();
  });
});
