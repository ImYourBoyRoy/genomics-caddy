// ./src/lib/utils/chatGptPrompt.ts
/*
Purpose: Fit Genomics Caddy consultation prompts into ChatGPT's Responses API
limits. OpenAI rejects `instructions` above 1_048_576 characters; a full local
prompt (especially developer raw JSON) can be several MB.
Strategy:
- Prefer compact JSON.
- Never send developer `raw_report` to ChatGPT; fall back to active findings.
- Progressively omit heavy support/catalog blobs and cap findings until the
  `[JSON CONTEXT]` block fits a safe budget. Rust then moves that block out of
  `instructions` into the Responses `input` array.
Usage: `buildChatGptSystemPrompt(params)` from consultationChat when provider
is chatgpt.
*/

import { buildSystemPrompt, type PromptBuildParams } from "./aiPrompt";

/** OpenAI Responses API hard cap on the `instructions` string. */
export const CHATGPT_INSTRUCTIONS_MAX_CHARS = 1_048_576;

/**
 * Soft budget for the genomic JSON block after it is moved into `input`.
 * Leaves headroom under the instructions cap for rules text and wrappers.
 */
export const CHATGPT_CONTEXT_BUDGET_CHARS = 900_000;

const JSON_CONTEXT_MARKER = "[JSON CONTEXT]";

export interface ChatGptPromptBuildResult {
  prompt: string;
  warnings: string[];
  /** Characters in the JSON CONTEXT section (0 if marker missing). */
  contextChars: number;
  /** Characters in the rules/instructions section. */
  instructionsChars: number;
}

function splitPrompt(prompt: string): { instructions: string; context: string } {
  const idx = prompt.indexOf(JSON_CONTEXT_MARKER);
  if (idx < 0) return { instructions: prompt, context: "" };
  return {
    instructions: prompt.slice(0, idx).trimEnd(),
    context: prompt.slice(idx),
  };
}

/**
 * Build a ChatGPT-safe system prompt: compact JSON, no multi-MB raw report,
 * progressively reduced until the context block fits the budget.
 */
export function buildChatGptSystemPrompt(params: PromptBuildParams): ChatGptPromptBuildResult {
  const warnings: string[] = [];
  let contextMode = params.contextMode;
  if (contextMode === "developer_raw_json") {
    contextMode = "active_findings";
    warnings.push(
      "Developer raw JSON exceeds ChatGPT size limits; using active findings without the full raw report.",
    );
  }

  const attempts: Array<{
    omitHeavyContext: boolean;
    maxFindings: number | null;
    label: string;
  }> = [
    { omitHeavyContext: false, maxFindings: null, label: "compact JSON" },
    { omitHeavyContext: true, maxFindings: null, label: "without heavy support/catalog blobs" },
    { omitHeavyContext: true, maxFindings: 250, label: "top 250 findings" },
    { omitHeavyContext: true, maxFindings: 100, label: "top 100 findings" },
    { omitHeavyContext: true, maxFindings: 40, label: "top 40 findings" },
  ];

  let prompt = "";
  let contextChars = 0;
  let instructionsChars = 0;

  for (const attempt of attempts) {
    prompt = buildSystemPrompt({
      ...params,
      contextMode,
      jsonIndent: null,
      omitHeavyContext: attempt.omitHeavyContext,
      maxFindings: attempt.maxFindings,
    });
    const parts = splitPrompt(prompt);
    instructionsChars = parts.instructions.length;
    contextChars = parts.context.length;
    if (
      instructionsChars < 100_000 &&
      contextChars <= CHATGPT_CONTEXT_BUDGET_CHARS &&
      // Belt-and-suspenders: if Rust split ever fails, instructions alone must fit.
      (parts.context ? instructionsChars : prompt.length) <= CHATGPT_INSTRUCTIONS_MAX_CHARS
    ) {
      if (attempt.label !== "compact JSON") {
        warnings.push(`ChatGPT context reduced (${attempt.label}) to fit OpenAI size limits.`);
      }
      return { prompt, warnings, contextChars, instructionsChars };
    }
  }

  warnings.push(
    "ChatGPT context is still large after reduction; the request may fail if OpenAI rejects the input payload.",
  );
  return { prompt, warnings, contextChars, instructionsChars };
}
