// ./src/lib/utils/qdrantRag.ts
/*
Purpose: Format Qdrant vector hits for AI Consultation system prompts and UI citations.
Inputs: QdrantHit[] from semantic search, query metadata.
Outputs: Structured vector_research block embedded in JSON CONTEXT.
*/

import type { QdrantHit } from "../types/research";

export interface VectorSearchMeta {
  query: string;
  enabled: boolean;
  error?: string;
}

export function formatHitForContext(hit: QdrantHit) {
  return {
    rsid: hit.rsid,
    genotype: hit.genotype,
    gene: hit.gene,
    gene_confidence: hit.gene_confidence,
    semantic_score: Number(hit.score.toFixed(4)),
    significance_score: hit.significance_score,
    evidence_tier: hit.category,
    enrichment_version: hit.enrichment_version,
    clinvar_significance: hit.clinvar_significance,
    gnomad_af: hit.gnomad_af,
    gwas_traits: hit.gwas_trait,
    gwas_associations: hit.gwas_associations?.slice(0, 6),
    gene_candidates: hit.gene_candidates?.slice(0, 6),
    sources_provenance: hit.sources_provenance,
    cross_refs: hit.cross_refs,
    trait_categories: hit.trait_categories,
    consultation_modes: hit.consultation_modes,
    pack_refs: hit.pack_refs?.slice(0, 4),
    association_summary: hit.association_summary,
    synthesis: hit.text,
  };
}

export function buildVectorResearchBlock(hits: QdrantHit[], meta: VectorSearchMeta) {
  if (!meta.enabled) {
    return {
      status: "disabled",
      purpose:
        "Vector research retrieval is turned off for this consultation. Answer using sample_context only.",
    };
  }

  if (meta.error) {
    return {
      status: "error",
      query: meta.query,
      error: meta.error,
      purpose:
        "Semantic vector search failed. Answer from sample_context and note that vector research was unavailable.",
    };
  }

  if (hits.length === 0) {
    return {
      status: "no_matches",
      query: meta.query,
      hit_count: 0,
      purpose:
        "No semantically similar enriched variants matched this query in the Qdrant index for this sample.",
      usage_rules: [
        "Answer from sample_context.sections only.",
        "Tell the user no indexed vector research matched their question — suggest Vector Research tab to enrich more variants or rephrase.",
      ],
    };
  }

  return {
    status: "ok",
    query: meta.query,
    hit_count: hits.length,
    purpose:
      "Semantically retrieved enriched variants from the local Qdrant vector index (GWAS associations, ClinVar, gnomAD, gene mapping). This data IS available to you inside this JSON block.",
    usage_rules: [
      "You HAVE access to vector_research.hits — do NOT ask the user for database credentials or claim you cannot parse vector data.",
      "Cite rsIDs from vector_research.hits when discussing traits, genes, or population frequency.",
      "When the same rsID appears in sample_context.sections, merge both: trait report for user genotype/effect direction; vector_research for literature traits, gene provenance, and gnomAD frequency.",
      "Prefer gwas_associations and gene_candidates over synthesis text alone when explaining gene-trait links.",
      "If vector_research contradicts the trait report on gene symbol, mention both and note enrichment provenance.",
    ],
    hits: hits.map(formatHitForContext),
  };
}

export function summarizeHitForUi(hit: QdrantHit): string {
  const parts = [hit.rsid];
  if (hit.gene) parts.push(hit.gene);
  if (hit.gwas_trait) parts.push(hit.gwas_trait.slice(0, 60));
  return parts.join(" · ");
}
