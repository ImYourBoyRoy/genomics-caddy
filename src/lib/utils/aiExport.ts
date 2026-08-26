// ./src/lib/utils/aiExport.ts
import type { GenomeSample } from "../types/genomics";
import { parseThinking } from "./chatParser";

/**
 * Builds a standard markdown transcript of the chat conversation.
 */
export function buildStandardMarkdown(
  messages: any[],
  selectedSample: GenomeSample | null,
  selectedModel: string,
  includeTrace: boolean = false
): string {
  let md = `# Genomics Caddy AI - Consultation Export\n`;
  md += `*Sample Name:* ${selectedSample ? selectedSample.name : "N/A"}\n`;
  md += `*Genetic Sex:* ${selectedSample ? selectedSample.genetic_sex : "N/A"}\n`;
  md += `*Date:* ${new Date().toLocaleString()}\n`;
  md += `*Model:* ${selectedModel}\n\n`;
  md += `---\n\n`;
  
  for (const msg of messages) {
    if (msg.role === "system") {
      md += `> **System Settings Notification:** ${msg.content}\n\n`;
    } else if (msg.role === "user") {
      md += `### You:\n${msg.content}\n\n`;
    } else if (msg.role === "assistant") {
      const parsed = parseThinking(msg.content);
      if (parsed.thought && includeTrace) {
        md += `> **[Reasoning Stream]**\n> ${parsed.thought.replace(/\n/g, "\n> ")}\n\n`;
      }
      md += `### Genomics Caddy AI:\n${parsed.response.trim()}\n\n`;
      if (msg.safetyReview) {
        md += `> **[Safety Review Notes]**\n> ${msg.safetyReview.trim().replace(/\n/g, "\n> ")}\n\n`;
      }
    }
    md += `---\n\n`;
  }
  return md;
}

/**
 * Builds a comprehensive clinical handoff summary containing the user health profile,
 * active variant summaries, frontier model RAG prompt payload, and conversation history.
 */
export function buildClinicalHandoffMarkdown(
  messages: any[],
  selectedSample: GenomeSample | null,
  selectedModel: string,
  userProfile: any,
  currentSystemPrompt: string,
  generatedReport: any,
  includeTrace: boolean = false
): string {
  let md = `# Genomics Caddy AI - Clinical Handoff & Biohacking Summary\n`;
  md += `## Patient & Metadata\n`;
  md += `* **Patient Name:** ${selectedSample ? selectedSample.name : "N/A"}\n`;
  md += `* **Genetic Sex:** ${selectedSample ? selectedSample.genetic_sex : "N/A"}\n`;
  md += `* **Date of Report:** ${new Date().toLocaleString()}\n`;
  md += `* **Inference Model:** ${selectedModel}\n\n`;
  md += `---\n\n`;

  md += `## 👤 Biohacking & Health Profile (Self-Reported)\n`;
  md += `* **Goals:** ${userProfile.goals || "None declared"}\n`;
  md += `* **Challenges & Symptoms:** ${userProfile.challenges || "None declared"}\n`;
  md += `* **Relevant Body Systems & Life Context:** ${userProfile.relevantBodySystems || "Not specified"}\n`;
  md += `* **Reproductive & Hormone Context:** ${userProfile.reproductiveHormoneContext || "Not specified"}\n`;
  md += `* **Average Diet:** ${userProfile.diet || "Not specified"}\n`;
  md += `* **Supplements List:** ${userProfile.supplements || "None"}\n`;
  md += `* **Medications List:** ${userProfile.medications || "None"}\n`;
  md += `* **Blood Work History:** ${userProfile.bloodwork || "None provided"}\n`;
  md += `* **Diagnosis List:** ${userProfile.diagnoses || "None declared"}\n`;
  md += `* **Supportive Tests:** ${userProfile.supportiveTests || "None"}\n\n`;
  md += `---\n\n`;

  md += `## 🧬 Clinical Findings Summary (Active Variants of Interest)\n`;
  
  let clinicalCount = 0;
  if (generatedReport) {
    for (const sec of generatedReport.sections) {
      for (const m of sec.markers) {
        if (m.effect_count > 0 && (m.clinical_confirmation_required || m.severity_class === "high_risk" || m.severity_class === "moderate_risk")) {
          clinicalCount++;
          md += `### ⚠️ ${m.gene} (${m.rsid})\n`;
          md += `* **Genotype:** ${m.user_genotype} (Effect allele: ${m.effect_allele}, count: ${m.effect_count})\n`;
          md += `* **Severity Rating:** ${m.severity_class.toUpperCase().replace("_", " ")}\n`;
          md += `* **Clinical Confirmation Required:** ${m.clinical_confirmation_required ? "Yes" : "No"}\n`;
          md += `* **Layperson Summary:** ${m.impact || "N/A"}\n`;
          md += `* **Medical Context:** ${m.interpretation || "N/A"}\n\n`;
        }
      }
    }
  }
  if (clinicalCount === 0) {
    md += `*No high-stakes or risk-associated active variants were detected in the evaluated packs.*\n\n`;
  }
  md += `---\n\n`;

  md += `## 🤖 Frontier LLM Copy-Paste Context Bundle\n`;
  md += `> [!TIP]\n`;
  md += `> Copy the block below to paste directly into frontier models (Claude 3.5 Sonnet, GPT-4o, Gemini 1.5 Pro) to continue biohacking consultations outside of MCP.\n\n`;
  md += `\`\`\`markdown\n`;
  md += `${currentSystemPrompt}\n`;
  md += `\`\`\`\n\n`;
  md += `---\n\n`;

  md += `## 💬 Consultation Conversation History\n`;
  for (const msg of messages) {
    if (msg.role === "system") {
      md += `> **System Event:** ${msg.content}\n\n`;
    } else if (msg.role === "user") {
      md += `### User:\n${msg.content}\n\n`;
    } else if (msg.role === "assistant") {
      const parsed = parseThinking(msg.content);
      if (parsed.thought && includeTrace) {
        md += `> **[Reasoning Stream]**\n> ${parsed.thought.replace(/\n/g, "\n> ")}\n\n`;
      }
      md += `### AI Assistant:\n${parsed.response.trim()}\n\n`;
      if (msg.safetyReview) {
        md += `> **[Safety Review Notes]**\n> ${msg.safetyReview.trim().replace(/\n/g, "\n> ")}\n\n`;
      }
    }
  }
  return md;
}
