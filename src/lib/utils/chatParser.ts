// ./src/lib/utils/chatParser.ts
/**
 * Chat message parsing and markdown formatting utilities for the AI Consultation panel.
 *
 * Responsibilities:
 * - Extract model thinking/reasoning blocks from streamed LLM output.
 * - Convert markdown text to safe, styled HTML for chat message rendering.
 *
 * Supported thinking formats:
 * - Gemma:    <unused94>thought ... <unused95>
 * - DeepSeek: <think> ... </think>
 * - Qwen:     <think> ... </think>  (same tags)
 *
 * Supported markdown features:
 * - Headings (h1–h3), bold, italic
 * - Fenced code blocks with optional language hint
 * - Inline code
 * - Blockquotes
 * - Unordered and ordered lists
 * - Links [text](url)
 * - Horizontal rules (---, ***, ___)
 * - Paragraph breaks
 */

import DOMPurify from "isomorphic-dompurify";

const MARKDOWN_ALLOWED_TAGS = [
  "p", "br", "strong", "em", "h3", "h4", "h5",
  "pre", "code", "blockquote", "hr", "ul", "ol", "li", "a",
];

const MARKDOWN_ALLOWED_ATTR = ["href", "target", "rel", "class"];

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ParsedThinking {
  thought: string;
  response: string;
  thoughtCompleted: boolean;
}

// ---------------------------------------------------------------------------
// Thinking / Reasoning Parser
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Thinking / Reasoning Parser
// ---------------------------------------------------------------------------

const TAG_PAIRS = [
  { open: "<think>", close: "</think>" },
  { open: "<thought>", close: "</thought>" },
  { open: "[think]", close: "[/think]" },
  { open: "[thought]", close: "[/thought]" },
  { open: "<unused94>thought", close: "<unused95>" },
  { open: "<unused94> thought", close: "<unused95>" },
  { open: "<unused94>", close: "<unused95>" }
];

/**
 * Parse thinking / reasoning blocks from streamed model output.
 *
 * Handles partial streams where the closing tag has not yet arrived
 * (returns `thoughtCompleted = false` in that case).
 */
export function parseThinking(content: string): ParsedThinking {
  if (!content) return { thought: "", response: content || "", thoughtCompleted: false };

  // First try exact case matching
  for (const pair of TAG_PAIRS) {
    const openIdx = content.indexOf(pair.open);
    if (openIdx !== -1) {
      const afterOpen = openIdx + pair.open.length;
      const closeIdx  = content.indexOf(pair.close, afterOpen);
      if (closeIdx !== -1) {
        return {
          thought: content.substring(afterOpen, closeIdx).trim(),
          response: (content.substring(0, openIdx) + content.substring(closeIdx + pair.close.length)).trim(),
          thoughtCompleted: true,
        };
      }
      return {
        thought: content.substring(afterOpen).trim(),
        response: content.substring(0, openIdx).trim(),
        thoughtCompleted: false,
      };
    }
  }

  // Fallback to case-insensitive matching
  const lowerContent = content.toLowerCase();
  for (const pair of TAG_PAIRS) {
    const openIdx = lowerContent.indexOf(pair.open.toLowerCase());
    if (openIdx !== -1) {
      const afterOpen = openIdx + pair.open.length;
      const closeIdx  = lowerContent.indexOf(pair.close.toLowerCase(), afterOpen);
      if (closeIdx !== -1) {
        return {
          thought: content.substring(afterOpen, closeIdx).trim(),
          response: (content.substring(0, openIdx) + content.substring(closeIdx + pair.close.length)).trim(),
          thoughtCompleted: true,
        };
      }
      return {
        thought: content.substring(afterOpen).trim(),
        response: content.substring(0, openIdx).trim(),
        thoughtCompleted: false,
      };
    }
  }

  return { thought: "", response: content, thoughtCompleted: false };
}

/**
 * Strip thinking wrapper tokens from raw content to produce clean copy-text.
 */
export function stripThinkingTokens(text: string): string {
  if (!text) return "";
  const parsed = parseThinking(text);
  return parsed.response.trim();
}

// ---------------------------------------------------------------------------
// Markdown → HTML Formatter
// ---------------------------------------------------------------------------

/**
 * Client-side markdown-to-HTML converter optimised for chat message rendering.
 *
 * Handles: headings (h1–h3), bold, italic, fenced code blocks, inline code,
 * blockquotes, unordered lists, ordered lists, horizontal rules, links, and
 * paragraph breaks.
 *
 * Design notes:
 * - All user-provided text is HTML-escaped first to prevent XSS.
 * - Link `href` attributes are emitted with `target="_blank" rel="noopener"`.
 * - Tables are not yet supported (complex streaming partial renders).
 */
export function formatMarkdown(md: string): string {
  if (!md) return "";

  // 1. Escape HTML entities
  let html = md
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  // 2. Fenced code blocks  ```lang\ncode\n```
  html = html.replace(/```(\w*)\n?([\s\S]*?)```/g, (_match, _lang, code) => {
    return `<pre class="code-block"><code>${code.trimEnd()}</code></pre>`;
  });

  // 3. Inline code  `code`
  html = html.replace(/`([^`\n]+)`/g, '<code class="code-inline">$1</code>');

  // 4. Bold  **text**
  html = html.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");

  // 5. Italic  *text*  (negative lookbehind to avoid matching **)
  html = html.replace(/(?<!\*)\*([^*\n]+)\*(?!\*)/g, "<em>$1</em>");

  // 6. Headings (must come after bold to avoid conflicts)
  html = html.replace(/^### (.*$)/gim, '<h5 class="md-h3">$1</h5>');
  html = html.replace(/^## (.*$)/gim, '<h4 class="md-h2">$1</h4>');
  html = html.replace(/^# (.*$)/gim, '<h3 class="md-h1">$1</h3>');

  // 7. Blockquotes
  html = html.replace(/^&gt;\s?(.*$)/gim, '<blockquote class="md-quote">$1</blockquote>');

  // 8. Horizontal rules (must be a line by itself)
  html = html.replace(/^(---+|\*\*\*+|___+)\s*$/gim, '<hr class="md-hr" />');

  // 9. Links [text](url) — allow http(s) only; escape attribute values
  html = html.replace(
    /\[([^\]]+)\]\(([^)]+)\)/g,
    (_match, label: string, rawUrl: string) => {
      const url = rawUrl.trim();
      if (!/^https?:\/\//i.test(url) || /[\s"'<>]/.test(url)) {
        return `${label} (${url})`;
      }
      const safeUrl = url.replace(/"/g, "&quot;");
      const safeLabel = String(label);
      return `<a href="${safeUrl}" target="_blank" rel="noopener noreferrer" class="md-link">${safeLabel}</a>`;
    }
  );

  // 10. Process lists line-by-line
  const lines = html.split("\n");
  const result: string[] = [];
  let inUL = false;
  let inOL = false;

  for (const rawLine of lines) {
    const line = rawLine.trim();

    // Unordered list item: starts with - or *
    const ulMatch = line.match(/^[-*]\s+(.*)/);
    // Ordered list item: starts with digit(s).
    const olMatch = line.match(/^\d+\.\s+(.*)/);

    if (ulMatch) {
      if (inOL) { result.push("</ol>"); inOL = false; }
      if (!inUL) { result.push('<ul class="md-list">'); inUL = true; }
      result.push(`<li>${ulMatch[1]}</li>`);
    } else if (olMatch) {
      if (inUL) { result.push("</ul>"); inUL = false; }
      if (!inOL) { result.push('<ol class="md-list md-ol">'); inOL = true; }
      result.push(`<li>${olMatch[1]}</li>`);
    } else {
      if (inUL) { result.push("</ul>"); inUL = false; }
      if (inOL) { result.push("</ol>"); inOL = false; }
      result.push(rawLine);
    }
  }
  if (inUL) result.push("</ul>");
  if (inOL) result.push("</ol>");

  html = result.join("\n");

  // 11. Paragraph breaks
  html = html.replace(/\n\n/g, "</p><p>");
  html = html.replace(/\n/g, "<br/>");

  return "<p>" + html + "</p>";
}

/** Format markdown then sanitize for safe `{@html}` rendering. */
export function formatMarkdownSafe(md: string): string {
  const textOnly = DOMPurify.sanitize(md, { ALLOWED_TAGS: [], ALLOWED_ATTR: [] });
  const raw = formatMarkdown(textOnly);
  return DOMPurify.sanitize(raw, {
    ALLOWED_TAGS: MARKDOWN_ALLOWED_TAGS,
    ALLOWED_ATTR: MARKDOWN_ALLOWED_ATTR,
  });
}
