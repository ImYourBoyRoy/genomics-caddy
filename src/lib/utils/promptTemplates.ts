/** Render a resource-authored prompt template without silently dropping data. */
export function renderPromptTemplate(template: string, values: Record<string, string>): string {
  const placeholders = Array.from(template.matchAll(/\{\{([a-z][a-z0-9_]*)\}\}/g), (match) => match[1]);
  const missing = Array.from(new Set(placeholders.filter((key) => !Object.prototype.hasOwnProperty.call(values, key))));
  if (missing.length > 0) {
    throw new Error(`Prompt template has unresolved placeholders: ${missing.map((key) => `{{${key}}}`).join(", ")}`);
  }
  return template.replace(/\{\{([a-z][a-z0-9_]*)\}\}/g, (_placeholder, key: string) => values[key]);
}
