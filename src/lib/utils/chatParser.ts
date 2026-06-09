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

const GEMMA_OPEN  = "\x3Cunused94>thought";   // "<unused94>thought"
const GEMMA_CLOSE = "\x3Cunused95>";           // "<unused95>"
const THINK_OPEN  = "\x3Cthink>";              // "<think>"
const THINK_CLOSE = "\x3C/think>";             // "</think>"

/**
 * Parse thinking / reasoning blocks from streamed model output.
 *
 * Handles partial streams where the closing tag has not yet arrived
 * (returns `thoughtCompleted = false` in that case).
 */
export function parseThinking(content: string): ParsedThinking {
  if (!content) return { thought: "", response: content || "", thoughtCompleted: false };

  // --- Gemma format ---
  const gemmaIdx = content.indexOf(GEMMA_OPEN);
  if (gemmaIdx !== -1) {
    const afterOpen = gemmaIdx + GEMMA_OPEN.length;
    const closeIdx  = content.indexOf(GEMMA_CLOSE, afterOpen);
    if (closeIdx !== -1) {
      return {
        thought: content.substring(afterOpen, closeIdx).trim(),
        response: content.substring(closeIdx + GEMMA_CLOSE.length),
        thoughtCompleted: true,
      };
    }
    return {
      thought: content.substring(afterOpen).trim(),
      response: "",
      thoughtCompleted: false,
    };
  }

  // --- DeepSeek / Qwen <think> format ---
  const thinkIdx = content.indexOf(THINK_OPEN);
  if (thinkIdx !== -1) {
    const afterOpen = thinkIdx + THINK_OPEN.length;
    const closeIdx  = content.indexOf(THINK_CLOSE, afterOpen);
    if (closeIdx !== -1) {
      return {
        thought: content.substring(afterOpen, closeIdx).trim(),
        response: content.substring(closeIdx + THINK_CLOSE.length),
        thoughtCompleted: true,
      };
    }
    return {
      thought: content.substring(afterOpen).trim(),
      response: "",
      thoughtCompleted: false,
    };
  }

  return { thought: "", response: content, thoughtCompleted: false };
}

/**
 * Strip thinking wrapper tokens from raw content to produce clean copy-text.
 */
export function stripThinkingTokens(text: string): string {
  let clean = text;
  // Gemma format
  if (clean.includes(GEMMA_CLOSE)) {
    clean = clean.substring(clean.indexOf(GEMMA_CLOSE) + GEMMA_CLOSE.length);
  }
  // DeepSeek / Qwen format
  if (clean.includes(THINK_CLOSE)) {
    clean = clean.substring(clean.indexOf(THINK_CLOSE) + THINK_CLOSE.length);
  }
  return clean.trim();
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

  // 9. Links [text](url) — after HTML escaping so parens are intact
  html = html.replace(
    /\[([^\]]+)\]\(([^)]+)\)/g,
    '<a href="$2" target="_blank" rel="noopener noreferrer" class="md-link">$1</a>'
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
