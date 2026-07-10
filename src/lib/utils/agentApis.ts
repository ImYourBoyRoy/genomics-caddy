// ./src/lib/utils/agentApis.ts
import { queryRsids, searchEvidence, getEvidenceForMarker, fetchExternalApi } from "../api/tauri";
import type { DbSnpRecord, GeneratedReport, EvaluatedMarker } from "../types/genomics";
import { asApiJson } from "../types/api";

export interface AgentStep {
  id: string;
  label: string;
  status: "idle" | "running" | "success" | "error";
  message?: string;
}

export interface NcbiData {
  chromosome: string;
  position: string;
  alleles: string;
  clinicalSignificance: string;
  clinvarTraits: string[];
}

export interface PubMedArticle {
  id: string;
  title: string;
  author: string;
  journal: string;
  year: string;
  url: string;
}

export interface ClinicalTrial {
  id: string;
  title: string;
  sponsor: string;
  status: string;
  phase: string;
}

export interface ChemblDrug {
  name: string;
  type: string;
  maxPhase: string;
}

export interface LocalGenotypeResult {
  rsid: string;
  genotype: string;
  gene: string;
  severity: string;
  interpretation: string;
  evidence: string[];
}

/**
 * Fetch genotype and local RAG evidence from the local database.
 */
export async function fetchLocalGenotype(
  sampleId: number,
  rsid: string,
  generatedReport: GeneratedReport | null
): Promise<LocalGenotypeResult> {
  const dbRecords: DbSnpRecord[] = await queryRsids(sampleId, [rsid]).catch(() => []);
  const genotype = dbRecords[0] ? `${dbRecords[0].allele1}/${dbRecords[0].allele2}` : "Not found in raw DNA file";

  let gene = "";
  let severity = "no_data";
  let interpretation = "";

  // Scan generated report for this marker
  if (generatedReport?.sections) {
    for (const section of generatedReport.sections) {
      const marker = section.markers?.find(m => m.rsid.toLowerCase() === rsid.toLowerCase());
      if (marker) {
        gene = marker.gene;
        severity = marker.severity_class;
        interpretation = marker.interpretation;
        break;
      }
    }
  }

  // Load local RAG evidence
  const localEvidence = await getEvidenceForMarker(rsid).catch(() => []);
  const evidenceTexts = localEvidence.map(e => `${e.source_citation}: ${e.evidence_text}`);

  return {
    rsid,
    genotype,
    gene,
    severity,
    interpretation,
    evidence: evidenceTexts
  };
}

/**
 * Resolve a gene symbol to a list of rsIDs.
 */
export async function resolveGeneToRsids(
  gene: string,
  generatedReport: GeneratedReport | null
): Promise<string[]> {
  const rsids = new Set<string>();
  if (generatedReport?.sections) {
    for (const section of generatedReport.sections) {
      for (const m of section.markers || []) {
        if (m.gene.toUpperCase() === gene.toUpperCase()) {
          rsids.add(m.rsid);
        }
      }
    }
  }
  return Array.from(rsids);
}

/**
 * Perform semantic search on the local RAG database to resolve a topic to rsIDs.
 */
export async function resolveTopicToRsids(
  topic: string,
  ollamaUrl: string,
  ollamaToken: string
): Promise<string[]> {
  const rsids = new Set<string>();
  const records = await searchEvidence(topic, ollamaUrl, ollamaToken).catch(() => []);
  for (const r of records) {
    if (r.rsid && r.rsid.startsWith("rs")) {
      rsids.add(r.rsid);
    }
  }
  return Array.from(rsids);
}

/**
 * Query NCBI dbSNP and ClinVar summaries via Tauri Rust reqwest proxy.
 */
export async function fetchNcbiDbsnpAndClinvar(rsid: string, apiKey?: string): Promise<NcbiData> {
  const snpId = rsid.replace(/^rs/i, "");
  const defaultData: NcbiData = {
    chromosome: "Unknown",
    position: "Unknown",
    alleles: "Unknown",
    clinicalSignificance: "No ClinVar annotation",
    clinvarTraits: []
  };

  try {
    // 1. Fetch dbSNP summary info
    const dbSnpUrl = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=snp&id=${snpId}&retmode=json`;
    const snpRes = asApiJson(await fetchExternalApi(dbSnpUrl, apiKey).catch(() => null));
    if (snpRes?.result && typeof snpRes.result === "object" && snpRes.result !== null) {
      const result = snpRes.result as Record<string, Record<string, unknown>>;
      const data = result[snpId];
      if (data) {
      defaultData.chromosome = String(data.chr ?? "Unknown");
      defaultData.position = String(data.chrpos ?? "Unknown");
      const doclist = data.doclist as { flat_alleles?: string } | undefined;
      defaultData.alleles = String(doclist?.flat_alleles ?? data.alleles ?? "Unknown");
      }
    }

    // 2. Fetch ClinVar search IDs
    const clinVarSearch = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=clinvar&term=${rsid}&retmode=json`;
    const searchRes = asApiJson(await fetchExternalApi(clinVarSearch, apiKey).catch(() => null));
    const esearch = searchRes?.esearchresult as { idlist?: string[] } | undefined;
    const ids = esearch?.idlist;
    
    if (ids && ids.length > 0) {
      const clinVarSummaryUrl = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=clinvar&id=${ids.slice(0, 3).join(",")}&retmode=json`;
      const summaryRes = asApiJson(await fetchExternalApi(clinVarSummaryUrl, apiKey).catch(() => null));
      const summaryResult = summaryRes?.result as Record<string, Record<string, unknown>> | undefined;
      if (summaryResult) {
        const sigs = new Set<string>();
        const traits = new Set<string>();
        for (const id of ids) {
          const summary = summaryResult[id];
          if (summary) {
            const sigObj = summary.clinical_significance as { description?: string } | undefined;
            const sig = sigObj?.description;
            if (sig) sigs.add(sig);
            const traitList = (summary.trait_set as Array<{ trait_name?: string }> | undefined)?.map((t) => t.trait_name) || [];
            for (const t of traitList) if (t) traits.add(t);
          }
        }
        if (sigs.size > 0) {
          defaultData.clinicalSignificance = Array.from(sigs).join(", ");
        }
        defaultData.clinvarTraits = Array.from(traits);
      }
    }
  } catch (e) {
    console.error("NCBI fetch failed", e);
  }

  return defaultData;
}

/**
 * Search PubMed articles by custom query term via Tauri Rust reqwest proxy.
 */
export async function searchPubMedQuery(term: string, apiKey?: string, limit: number = 3): Promise<PubMedArticle[]> {
  try {
    const searchUrl = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term=${encodeURIComponent(term)}&retmode=json&retmax=${limit}`;
    const searchRes = asApiJson(await fetchExternalApi(searchUrl, apiKey).catch(() => null));
    const pubmedSearch = searchRes?.esearchresult as { idlist?: string[] } | undefined;
    const ids = pubmedSearch?.idlist;

    if (ids && ids.length > 0) {
      const summaryUrl = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id=${ids.join(",")}&retmode=json`;
      const summaryRes = asApiJson(await fetchExternalApi(summaryUrl, apiKey).catch(() => null));
      const summaryResult = summaryRes?.result as Record<string, Record<string, unknown>> | undefined;
      if (summaryResult) {
        return ids.map((id: string) => {
          const item = summaryResult[id];
          const authors = item?.authors as Array<{ name?: string }> | undefined;
          const author = String(item?.sortauthors ?? authors?.[0]?.name ?? "Unknown Author");
          const pubdate = item?.pubdate;
          const year = typeof pubdate === "string" ? pubdate.substring(0, 4) : "Unknown Year";
          return {
            id,
            title: String(item?.title ?? "Untitled Article"),
            author,
            journal: String(item?.source ?? "PubMed Journal"),
            year,
            url: `https://pubmed.ncbi.nlm.nih.gov/${id}/`
          };
        });
      }
    }
  } catch (e) {
    console.error("PubMed search failed", e);
  }
  return [];
}

/**
 * Fetch relevant PubMed articles via Tauri Rust reqwest proxy.
 */
export async function fetchPubMedArticles(rsid: string, apiKey?: string): Promise<PubMedArticle[]> {
  return searchPubMedQuery(`${rsid} AND human`, apiKey, 3);
}

/**
 * Fetch clinical trials from ClinicalTrials.gov via Tauri Rust reqwest proxy.
 */
export async function fetchClinicalTrials(rsid: string, geneSymbol?: string, apiKey?: string): Promise<ClinicalTrial[]> {
  try {
    const term = rsid + (geneSymbol ? ` OR ${geneSymbol}` : "");
    const url = `https://clinicaltrials.gov/api/v2/studies?query.term=${encodeURIComponent(term)}&pageSize=3`;
    const res = asApiJson(await fetchExternalApi(url, apiKey).catch(() => null));
    const studies = res?.studies as Array<Record<string, unknown>> | undefined;
    if (studies) {
      return studies.map((study) => {
        const protocol = study.protocolSection as Record<string, Record<string, unknown>> | undefined;
        const ident = protocol?.identificationModule;
        const NCTId = String(ident?.nctId ?? "Unknown NCT");
        const title = String(ident?.officialTitle ?? ident?.briefTitle ?? "No Title");
        const sponsorMod = protocol?.sponsorCollaboratorsModule as { leadSponsor?: { name?: string } } | undefined;
        const sponsor = String(sponsorMod?.leadSponsor?.name ?? "Unknown Sponsor");
        const statusMod = protocol?.statusModule as { overallStatus?: string } | undefined;
        const status = String(statusMod?.overallStatus ?? "Unknown Status");
        const designMod = protocol?.designModule as { phases?: string[] } | undefined;
        const phases = designMod?.phases ?? [];
        const phase = phases.join(", ") || "N/A";
        return { id: NCTId, title, sponsor, status, phase };
      });
    }
  } catch (e) {
    console.error("ClinicalTrials fetch failed", e);
  }
  return [];
}

/**
 * Fetch molecules from ChEMBL associated with a gene target via Tauri Rust reqwest proxy.
 */
export async function fetchChemblDrugs(geneSymbol: string): Promise<ChemblDrug[]> {
  if (!geneSymbol) return [];
  try {
    // ChEMBL supports wildcard search on compounds
    const url = `https://www.ebi.ac.uk/chembl/api/data/molecule.json?q=${encodeURIComponent(geneSymbol)}&limit=3&format=json`;
    const res = asApiJson(await fetchExternalApi(url).catch(() => null));
    const molecules = res?.molecules as Array<Record<string, unknown>> | undefined;
    if (molecules) {
      return molecules.map((m) => {
        const synonyms = m.molecule_synonyms as Array<{ synonym?: string }> | undefined;
        return {
          name: String(m.pref_name ?? synonyms?.[0]?.synonym ?? `Compound ID: ${m.molecule_chembl_id}`),
          type: String(m.molecule_type ?? "Unknown Type"),
          maxPhase: m.max_phase ? `Phase ${m.max_phase}` : "N/A",
        };
      });
    }
  } catch (e) {
    console.error("ChEMBL fetch failed", e);
  }
  return [];
}

// ---------------------------------------------------------------------------
// Prompt Builders for Agent Loop
// ---------------------------------------------------------------------------

export function buildCritiquePrompt(
  rsid: string,
  localGenotype: LocalGenotypeResult,
  ncbi: NcbiData,
  pubmed: PubMedArticle[],
  trials: ClinicalTrial[],
  drugs: ChemblDrug[]
): string {
  return `You are a genomic database auditor. Analyze the gathered raw data for variant ${rsid} and identify any gaps, conflicts, or mismatches.

VARIANT INFORMATION:
- RSID: ${rsid}
- User Genotype: ${localGenotype.genotype} (Gene: ${localGenotype.gene || "Unknown"})
- dbSNP Location: Chr ${ncbi.chromosome}, Position ${ncbi.position}
- dbSNP Alleles: ${ncbi.alleles}
- ClinVar Clinical Significance: ${ncbi.clinicalSignificance}
- ClinVar Associated Traits: ${ncbi.clinvarTraits.join(", ") || "None"}
- PubMed Papers: ${pubmed.map(p => p.title).join("; ")}
- Clinical Trials: ${trials.map(t => t.id).join("; ")}
- Related Compounds: ${drugs.map(d => d.name).join("; ")}

INSTRUCTIONS:
1. Auditing Strand Mismatch: Compare dbSNP alleles (${ncbi.alleles}) with the user's genotype (${localGenotype.genotype}). Note if there is a risk of a forward/reverse strand orientation mismatch.
2. Gap Identification: Is there missing data? (e.g. no associated drugs in ChEMBL, or no active clinical trials listed?). Recommend if we should perform a refined search (e.g. searching by Gene symbol instead).
3. Conflict Resolution: Note if ClinVar lists conflicting reports.

Respond ONLY with a JSON block in this exact format:
{
  "strandWarning": "yes/no explanation",
  "missingGaps": ["gap1", "gap2"],
  "recommendedGeneHealing": "${localGenotype.gene || "none"}",
  "conflicts": "explanation of any conflicts or none"
}`;
}

export function buildSynthesisPrompt(
  rsid: string,
  localGenotype: LocalGenotypeResult,
  ncbi: NcbiData,
  pubmed: PubMedArticle[],
  trials: ClinicalTrial[],
  drugs: ChemblDrug[],
  critique: string,
  healedTrials: ClinicalTrial[],
  healedDrugs: ChemblDrug[]
): string {
  const finalTrials = trials.length > 0 ? trials : healedTrials;
  const finalDrugs = drugs.length > 0 ? drugs : healedDrugs;

  return `You are an expert clinical genomic researcher. Synthesize a comprehensive personal research report for variant ${rsid} based on the gathered data and self-critique audits.

RESEARCH PROFILE DATA:
- rsID: ${rsid}
- User Genotype: ${localGenotype.genotype}
- Gene Symbol: ${localGenotype.gene || "N/A"}
- Local DB Interpretation: ${localGenotype.interpretation || "No local interpretation."}
- dbSNP Location: Chr ${ncbi.chromosome}, Position ${ncbi.position}
- ClinVar Significance: ${ncbi.clinicalSignificance}
- ClinVar Associated Traits: ${ncbi.clinvarTraits.join(", ") || "None"}

CRITIQUE & STRAND AUDIT:
${critique}

PubMed ARTICLES FETCHED:
${pubmed.map((p, i) => `${i+1}. [${p.year}] "${p.title}" by ${p.author} (${p.journal}) - Link: ${p.url}`).join("\n")}

CLINICAL TRIALS:
${finalTrials.map((t, i) => `${i+1}. Trial ${t.id}: "${t.title}" (Status: ${t.status}, Sponsor: ${t.sponsor}, Phase: ${t.phase})`).join("\n")}

THERAPEUTIC DRUGS & COMPOUNDS (ChEMBL):
${finalDrugs.map((d, i) => `${i+1}. ${d.name} (${d.type}, Max Phase: ${d.maxPhase})`).join("\n")}

REPORT FORMATTING REQUIREMENT:
Provide a highly polished markdown report containing:
1. **Overview & Local Significance**: Chromosome position, ClinVar classification, and what this marker means.
2. **Personalized Genotype Analysis**: Compare the user's genotype (${localGenotype.genotype}) with ClinVar's risk alleles. Explicitly note the findings from the Critique & Strand audit.
3. **Recent Scientific Literature Summary**: Summarize the PubMed articles, explaining their relevance to the user's health.
4. **Clinical Trials & Drugs**: Detail active clinical trials and small molecules associated with the variant/gene.
5. **Supportive Lifestyle & Biohacking Actions**: Nutritional, dietary, or lifestyle options that support the molecular pathway (e.g. one-carbon cycle for MTHFR). Keep it safe, supportive, and guardrailed.
6. **Required Disclaimer**: Place a standard disclaimer at the bottom: "This report is for educational purposes only and does not constitute medical advice."`;
}

export function buildValidationPrompt(draftText: string): string {
  return `You are a clinical quality assurance auditor. Verify the draft report for medical safety, structure, and disclaimers.

DRAFT REPORT TO AUDIT:
"""
${draftText}
"""

VERIFICATION CRITERIA:
1. Medical Overclaiming: Does the text diagnose the patient (e.g. saying "you have disease X") or recommend specific drug dosages? (This must be flagged as false).
2. Proper Disclaimer: Is there a medical disclaimer at the bottom?
3. Formatting: Are there clear headings and lists?

Respond ONLY with a JSON block in this exact format:
{
  "medicalClaimingFree": true/false,
  "disclaimerPresent": true/false,
  "structureOk": true/false,
  "auditComments": "Brief summary of quality findings",
  "approved": true/false
}`;
}
