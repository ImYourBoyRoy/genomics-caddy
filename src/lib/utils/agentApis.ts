// ./src/lib/utils/agentApis.ts
import { queryRsids, searchEvidence, getEvidenceForMarker, fetchExternalApi } from "../api/tauri";
import type { DbSnpRecord, GeneratedReport, EvaluatedMarker } from "../types/genomics";

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
    const snpRes = await fetchExternalApi(dbSnpUrl, apiKey).catch(() => null);
    if (snpRes?.result?.[snpId]) {
      const data = snpRes.result[snpId];
      defaultData.chromosome = data.chr || "Unknown";
      defaultData.position = data.chrpos || "Unknown";
      defaultData.alleles = data.doclist?.flat_alleles || data.alleles || "Unknown";
    }

    // 2. Fetch ClinVar search IDs
    const clinVarSearch = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=clinvar&term=${rsid}&retmode=json`;
    const searchRes = await fetchExternalApi(clinVarSearch, apiKey).catch(() => null);
    const ids = searchRes?.esearchresult?.idlist;
    
    if (ids && ids.length > 0) {
      const clinVarSummaryUrl = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=clinvar&id=${ids.slice(0, 3).join(",")}&retmode=json`;
      const summaryRes = await fetchExternalApi(clinVarSummaryUrl, apiKey).catch(() => null);
      if (summaryRes?.result) {
        const sigs = new Set<string>();
        const traits = new Set<string>();
        for (const id of ids) {
          const summary = summaryRes.result[id];
          if (summary) {
            const sig = summary.clinical_significance?.description;
            if (sig) sigs.add(sig);
            const traitList = summary.trait_set?.map((t: any) => t.trait_name) || [];
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
 * Fetch relevant PubMed articles via Tauri Rust reqwest proxy.
 */
export async function fetchPubMedArticles(rsid: string, apiKey?: string): Promise<PubMedArticle[]> {
  try {
    const searchUrl = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term=${rsid}+AND+human&retmode=json&retmax=3`;
    const searchRes = await fetchExternalApi(searchUrl, apiKey).catch(() => null);
    const ids = searchRes?.esearchresult?.idlist;

    if (ids && ids.length > 0) {
      const summaryUrl = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id=${ids.join(",")}&retmode=json`;
      const summaryRes = await fetchExternalApi(summaryUrl, apiKey).catch(() => null);
      if (summaryRes?.result) {
        return ids.map((id: string) => {
          const item = summaryRes.result[id];
          const author = item?.sortauthors || item?.authors?.[0]?.name || "Unknown Author";
          const year = item?.pubdate ? item.pubdate.substring(0, 4) : "Unknown Year";
          return {
            id,
            title: item?.title || "Untitled Article",
            author,
            journal: item?.source || "PubMed Journal",
            year,
            url: `https://pubmed.ncbi.nlm.nih.gov/${id}/`
          };
        });
      }
    }
  } catch (e) {
    console.error("PubMed fetch failed", e);
  }
  return [];
}

/**
 * Fetch clinical trials from ClinicalTrials.gov via Tauri Rust reqwest proxy.
 */
export async function fetchClinicalTrials(rsid: string, geneSymbol?: string, apiKey?: string): Promise<ClinicalTrial[]> {
  try {
    const term = rsid + (geneSymbol ? ` OR ${geneSymbol}` : "");
    const url = `https://clinicaltrials.gov/api/v2/studies?query.term=${encodeURIComponent(term)}&pageSize=3`;
    const res = await fetchExternalApi(url, apiKey).catch(() => null);
    if (res?.studies) {
      return res.studies.map((study: any) => {
        const NCTId = study.protocolSection?.identificationModule?.nctId || "Unknown NCT";
        const title = study.protocolSection?.identificationModule?.officialTitle || study.protocolSection?.identificationModule?.briefTitle || "No Title";
        const sponsor = study.protocolSection?.sponsorCollaboratorsModule?.leadSponsor?.name || "Unknown Sponsor";
        const status = study.protocolSection?.statusModule?.overallStatus || "Unknown Status";
        const phases = study.protocolSection?.designModule?.phases || [];
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
    const res = await fetchExternalApi(url).catch(() => null);
    if (res?.molecules) {
      return res.molecules.map((m: any) => ({
        name: m.pref_name || m.molecule_synonyms?.[0]?.synonym || "Compound ID: " + m.molecule_chembl_id,
        type: m.molecule_type || "Unknown Type",
        maxPhase: m.max_phase ? `Phase ${m.max_phase}` : "N/A"
      }));
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
