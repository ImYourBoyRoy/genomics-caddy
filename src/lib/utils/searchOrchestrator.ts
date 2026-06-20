// ./src/lib/utils/searchOrchestrator.ts
/*
Module Docstring:
Purpose: Search Orchestration Engine for the AI Assistant.
Responsibilities:
- Parse user prompts for RSIDs and Genes.
- Query approved clinical databases (PubMed, ClinVar, dbSNP) using public APIs.
- Perform secure web searches via DuckDuckGo HTML-lite scraping.
- Apply approved domain filters (e.g. site:wikipedia.org) to search queries.
Key Inputs: User query, search toggles, and approved domains list.
Key Outputs: Array of SearchSourceResult objects containing snippets and links.
Operational Notes: Completely private, runs 100% locally/proxied on client.
*/

import { fetchNcbiDbsnpAndClinvar, searchPubMedQuery } from "./agentApis";
import { fetchExternalApi } from "../api/tauri";
import type { SearchSourceResult } from "../types/agent";

const EXCLUDED_GENES = new Set([
  // Common English words / sentence structure
  "AND", "THE", "FOR", "NOT", "BUT", "ARE", "WAS", "HAS", "HAD", "CAN", "MAY", "THAT", "THIS",
  "WITH", "FROM", "INTO", "HAVE", "WILL", "YOUR", "WHAT", "WHEN", "THEN", "THEY", "THAN",
  "YOU", "HIM", "HER", "SHE", "HIS", "ITS", "ALL", "ANY", "HOW", "WHY", "WHO",
  // Tech acronyms
  "DNA", "SNP", "VRAM", "LLM", "API", "TTL", "RAG", "SSE", "URL", "CSV", "TSV",
  "JSON", "HTML", "HTTP", "UUID", "SQL", "REST", "GPU", "CPU", "RAM", "SSD",
  // Medical non-gene terms
  "RNA", "PCR", "MRI", "ICU", "IUD", "BMI", "IBS", "IBD", "ADHD", "PTSD",
  // Common abbreviations
  "MSG", "MGS", "NOTE", "INFO", "HELP", "USER", "DATA", "CASE", "LIST", "TEST",
  // Roman numerals and single tokens
  "II", "III", "IV", "VI", "VII", "VIII",
]);

/**
 * Extracts candidate genes and rsIDs from a query string.
 */
export function extractEntities(query: string): { rsids: string[]; genes: string[] } {
  const rsids: string[] = [];
  const genes: string[] = [];

  // Extract rsIDs (e.g. rs1801133)
  const rsidRegex = /rs\d+/gi;
  let match;
  while ((match = rsidRegex.exec(query)) !== null) {
    if (!rsids.includes(match[0].toLowerCase())) {
      rsids.push(match[0].toLowerCase());
    }
  }

  // Extract candidate genes: uppercase, 3–10 chars, not purely numeric
  // Minimum 3 chars aligns with HGNC gene symbol standards (e.g. APC, TP53)
  const geneRegex = /\b[A-Z][A-Z0-9]{2,9}\b/g;
  const cleanQuery = query.replace(/rs\d+/gi, "").replace(/[^a-zA-Z0-9 ]/g, " ");
  while ((match = geneRegex.exec(cleanQuery)) !== null) {
    const candidate = match[0].toUpperCase();
    if (!EXCLUDED_GENES.has(candidate) && !genes.includes(candidate) && isNaN(Number(candidate))) {
      genes.push(candidate);
    }
  }

  return { rsids, genes };
}

/**
 * Scrapes DuckDuckGo HTML-lite for a query, filtering by approved domains if provided.
 */
async function searchWebDuckDuckGo(query: string, approvedDomains: string[]): Promise<SearchSourceResult[]> {
  try {
    let finalQuery = query;
    if (approvedDomains.length > 0) {
      const siteFilters = approvedDomains.map(domain => `site:${domain}`).join(" OR ");
      finalQuery = `(${siteFilters}) ${query}`;
    }

    const searchUrl = `https://html.duckduckgo.com/html/?q=${encodeURIComponent(finalQuery)}`;
    const raw = await fetchExternalApi(searchUrl).catch(() => null);
    const htmlText = typeof raw === "string" ? raw : null;
    if (!htmlText) return [];

    const parser = new DOMParser();
    const doc = parser.parseFromString(htmlText, "text/html");
    const results: SearchSourceResult[] = [];
    const elements = doc.querySelectorAll(".result__body");

    elements.forEach((el, index) => {
      if (index >= 3) return; // Limit to top 3 web results
      const titleEl = el.querySelector(".result__title .result__a");
      const snippetEl = el.querySelector(".result__snippet");
      if (titleEl) {
        const title = titleEl.textContent?.trim() || "Web Search Result";
        const rawUrl = titleEl.getAttribute("href") || "";
        
        // Decode DDG redirect url
        let url = rawUrl;
        if (rawUrl.includes("uddg=")) {
          const parts = rawUrl.split("uddg=");
          if (parts[1]) {
            url = decodeURIComponent(parts[1].split("&")[0]);
          }
        }

        const snippet = snippetEl?.textContent?.trim() || "No excerpt available.";
        results.push({
          sourceType: "web",
          title,
          url,
          snippet
        });
      }
    });

    return results;
  } catch (e) {
    console.error("DuckDuckGo HTML-lite scraping failed:", e);
    return [];
  }
}

/**
 * Orchestrates queries across enabled databases and web search.
 */
export async function orchestrateSearch(
  query: string,
  sources: { pubmed: boolean; clinvar: boolean; dbsnp: boolean; web: boolean },
  approvedDomains: string[],
  ncbiApiKey?: string
): Promise<SearchSourceResult[]> {
  const { rsids, genes } = extractEntities(query);
  const results: SearchSourceResult[] = [];
  const promises: Promise<void>[] = [];

  // 1. dbSNP / ClinVar lookups (if rsIDs are found and toggles are on)
  if (rsids.length > 0 && (sources.dbsnp || sources.clinvar)) {
    for (const rsid of rsids) {
      promises.push((async () => {
        try {
          const data = await fetchNcbiDbsnpAndClinvar(rsid, ncbiApiKey);
          
          if (sources.dbsnp && data.chromosome !== "Unknown") {
            results.push({
              sourceType: "dbsnp",
              title: `NCBI dbSNP ${rsid.toUpperCase()}`,
              url: `https://www.ncbi.nlm.nih.gov/snp/${rsid.replace(/^rs/i, "")}`,
              snippet: `Chromosome: ${data.chromosome}, Position: ${data.position}, Alleles: ${data.alleles}`
            });
          }

          if (sources.clinvar && data.clinicalSignificance !== "No ClinVar annotation") {
            const traitsStr = data.clinvarTraits.length > 0 ? ` (Associated Traits: ${data.clinvarTraits.join(", ")})` : "";
            results.push({
              sourceType: "clinvar",
              title: `NCBI ClinVar ${rsid.toUpperCase()}`,
              url: `https://www.ncbi.nlm.nih.gov/clinvar/?term=${rsid}`,
              snippet: `Clinical Significance: ${data.clinicalSignificance}${traitsStr}`
            });
          }
        } catch (e) {
          console.error(`dbSNP/ClinVar search failed for ${rsid}:`, e);
        }
      })());
    }
  }

  // 2. PubMed search
  if (sources.pubmed) {
    // Determine the query to send to PubMed
    let pubMedTerm = query;
    if (rsids.length > 0) {
      pubMedTerm = `${rsids[0]} AND human`;
    } else if (genes.length > 0) {
      pubMedTerm = `${genes.join(" AND ")} mutation human`;
    }

    promises.push((async () => {
      try {
        const articles = await searchPubMedQuery(pubMedTerm, ncbiApiKey, 3);
        for (const art of articles) {
          results.push({
            sourceType: "pubmed",
            title: art.title,
            url: art.url,
            snippet: `${art.author} (${art.year}) - Published in ${art.journal}`,
            author: art.author,
            year: art.year
          });
        }
      } catch (e) {
        console.error("PubMed search failed:", e);
      }
    })());
  }

  // 3. Web Search
  if (sources.web) {
    promises.push((async () => {
      const webHits = await searchWebDuckDuckGo(query, approvedDomains);
      results.push(...webHits);
    })());
  }

  // Wait for all searches to finish
  await Promise.all(promises);
  return results;
}

/**
 * Formats a list of SearchSourceResults into an injectible system prompt markdown segment.
 */
export function formatSearchPromptContext(results: SearchSourceResult[]): string {
  if (results.length === 0) return "";

  const lines: string[] = [
    "",
    "<search_evidence source=\"approved_databases\">",
    `Retrieved ${results.length} source(s) from approved scientific/medical databases:`,
    "",
  ];

  results.forEach((res, idx) => {
    const id = idx + 1;
    lines.push(`[${id}] Type: ${res.sourceType.toUpperCase()}`);
    lines.push(`    Title: ${res.title}`);
    if (res.snippet) lines.push(`    Excerpt: ${res.snippet}`);
    if (res.author && res.year) lines.push(`    Author/Year: ${res.author} (${res.year})`);
    lines.push(`    URL: ${res.url}`);
    lines.push("");
  });

  lines.push("INSTRUCTIONS: Ground clinical assertions using these sources. Cite PMIDs or URLs where relevant. Maintain all safety disclaimers.");
  lines.push("</search_evidence>");
  return "\n" + lines.join("\n");
}
