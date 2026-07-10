// ./src/lib/utils/chatParser.test.ts
import { describe, expect, it } from "vitest";
import { formatMarkdown, formatMarkdownSafe, parseThinking, stripThinkingTokens } from "./chatParser";

describe("formatMarkdown", () => {
  it("escapes HTML in plain text", () => {
    const html = formatMarkdown('<script>alert("x")</script>');
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;");
  });

  it("allows only http(s) links", () => {
    const safe = formatMarkdown("[ok](https://example.com)");
    expect(safe).toContain('href="https://example.com"');

    const blocked = formatMarkdown("[bad](javascript:alert(1))");
    expect(blocked).not.toContain('href="javascript');
    expect(blocked).toContain("bad (javascript:alert(1))");
  });
});

describe("formatMarkdownSafe", () => {
  it("neutralizes raw HTML before markdown rendering", () => {
    const html = formatMarkdownSafe('<img src=x onerror=alert(1)>');
    expect(html.toLowerCase()).not.toMatch(/<img[\s>]/);
    expect(html.toLowerCase()).not.toContain("<script");
    expect(html.toLowerCase()).not.toContain("onerror=");
  });
});

describe("parseThinking", () => {
  it("splits completed thinking blocks from response text", () => {
    const parsed = parseThinking(
      "Answer prefix<think>internal reasoning</think>final answer",
    );
    expect(parsed.thoughtCompleted).toBe(true);
    expect(parsed.thought).toBe("internal reasoning");
    expect(parsed.response).toContain("Answer prefix");
    expect(parsed.response).toContain("final answer");
  });
});

describe("stripThinkingTokens", () => {
  it("removes thinking tags from streamed content", () => {
    const cleaned = stripThinkingTokens(
      "visible<think>hidden</think>tail",
    );
    expect(cleaned).not.toContain("redacted_thinking");
    expect(cleaned).toContain("visible");
    expect(cleaned).toContain("tail");
  });
});
