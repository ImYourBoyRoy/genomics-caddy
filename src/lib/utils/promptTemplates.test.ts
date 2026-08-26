import { describe, expect, it } from "vitest";
import agentResearchPrompts from "../marker-packs/agent_research_prompts.json";
import aiPromptPolicy from "../marker-packs/ai_prompt_policy.json";
import { renderPromptTemplate } from "./promptTemplates";

describe("resource-authored prompt templates", () => {
  it("renders the consultation safety review without unresolved placeholders", () => {
    const prompt = aiPromptPolicy.safety_review_prompt;
    const rendered = renderPromptTemplate(prompt.template, { draft_text: "A draft response." });

    expect(rendered).toContain("unsupported hormone/cycle causal claims");
    expect(rendered).toContain("A draft response.");
    expect(rendered).not.toMatch(/\{\{[a-z][a-z0-9_]*\}\}/);
  });

  it("keeps every agent template placeholder renderable", () => {
    for (const prompt of Object.values(agentResearchPrompts.templates)) {
      const values = Object.fromEntries(prompt.required_placeholders.map((key) => [key, `[${key}]`]));
      const rendered = renderPromptTemplate(prompt.template, values);

      expect(rendered).not.toMatch(/\{\{[a-z][a-z0-9_]*\}\}/);
    }
  });

  it("fails closed when a caller omits a required value", () => {
    expect(() => renderPromptTemplate("Hello {{name}}", {})).toThrow("unresolved placeholders");
  });

  it("does not treat placeholder-looking user content as a resource placeholder", () => {
    expect(renderPromptTemplate("Draft: {{draft_text}}", { draft_text: "Literal {{not_a_resource_key}}" })).toContain(
      "Literal {{not_a_resource_key}}",
    );
  });
});
