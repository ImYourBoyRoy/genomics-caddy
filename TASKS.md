# Deferred research and infrastructure

## Report overhaul (user feedback 2026-09-26)

Shipped: disease links (strand verification, ClinVar consensus gating,
grouped results); copy cleanup in report and AI/clinician exports; themed
report controls (root cause: Tauri CSP nonce blocked Svelte runtime style
injection, now emitted as CSS files in production builds); small coverage
note; "Lifestyle-linked" plus add-context prompts; clickable overview tiles;
Male / Female / Unknown sex with a user-supplied anatomy prompt. Remaining:

- Dependencies: once 24 h have passed (after 2026-09-27T23:00Z), bump Tauri
  plugins (opener 2.6.0, process 2.4.0, updater 2.13.0) together on npm and
  Cargo, and drop the `@tauri-apps/*@2.12.0` age exclusions.

## gnomAD local-versus-remote source

- Keep the current remote indexed-VCF query path as the default until a local
  service is sized and tested.
- Prototype an explicit source mode: remote gnomAD versus a user-configured,
  authenticated local/server endpoint. Keep release/build selection and source
  provenance visible in results.
- Before adopting local hosting, benchmark import/query latency, concurrent
  access, storage, indexing, backup/upgrade, and operational/security costs.
  gnomAD reports roughly 1.4 TB for v4 exomes plus genomes short-variant VCFs;
  confirm current release artifacts and requirements before sizing. See the
  [gnomAD toolbox note](https://gnomad.broadinstitute.org/news/2025-01-gnomad-toolbox/).
- Compare privacy properties: remote range queries disclose requested genomic
  regions to the remote host; a private local endpoint avoids that lookup but
  introduces server exposure, access control, and maintenance responsibilities.

## Full ClinVar RCV XML evaluation

- The condition-specific `submission_summary.txt.gz` is the first local source
  for condition-level discovery. Keep full RCV XML optional until a benchmark
  shows material additional value beyond per-submission classifications.
- Benchmark full RCV import time, expanded storage, query latency, and useful
  added fields (including phenotype/observation detail) against the submission
  summary. Use only synthetic fixtures for routine CI; any private-genome
  comparison stays on-device and reports aggregate counts only.
- Revisit the decision if users/professionals need per-condition RCV assertions
  or richer observation provenance not retained by the tab-delimited source.

## GWAS detail in the existing all-call Discovery view

- The in-app Discovery view already joins all imported sample rsIDs against the
  local GWAS, ClinVar, and PharmGKB/ClinPGx reference indexes. GWAS results are
  research context and are separate from the curated clinical report.
- The installed GWAS Catalog TSV importer aggregates by rsID and retains trait,
  p-value, genes, and study accession. It counts all imported associations but
  retains at most 12 study records per rsID, prioritizing smaller p-values. It
  does not currently retain the TSV's effect-allele, ancestry, or sample-size
  fields, and the Discovery view displays an rsID summary rather than per-study
  records. Extend the import and UI to expose per-study evidence only after
  retaining those source fields; show direction only when allele orientation and
  genome build are verified.
- Do not turn association counts into a personal risk score. Preserve the
  study/source link and distinguish Catalog inclusion from genome-wide
  significance. Benchmark extra storage, query time, and report usefulness
  before importing full summary-statistics datasets.
- Do not treat any variant in a ClinGen-valid gene as a disease finding;
  require the variant mechanism and inheritance evidence for that relationship.
  Keep PGx calls limited to interpretable gene/allele models.

## Autonomic/POTS evidence

- Reassess only when replicated, clinically curated evidence supports a useful
  consumer-array model. Until then, keep POTS/dysautonomia in explicit clinical
  symptom-context routing; do not add candidate-gene scores or genotype-based
  diet, supplement, or medication advice.

## Two-profile comparison (couple / family) — proposed

- Goal: let a user open two local profiles side by side and see (a) shared
  carrier-status overlap for recessive conditions where both profiles carry
  one copy of a Pathogenic/Likely pathogenic variant in the same ClinGen
  Definitive/Strong/Moderate gene, (b) relatedness/shared-segment context
  (genome-wide identity-by-state summary, not a paternity or kinship verdict),
  and (c) pharmacogenomic and trait differences worth discussing separately.
- Reuse the Discovery rules: known source build and allele orientation,
  non-conflicting variant-wide ClinVar summary, benign-consensus variants
  dropped, somatic/oncogenic records excluded. Serialize copy counts only,
  never bases, in any export, MCP result, or AI handoff.
- Required limits, shown in the UI and exports: a consumer array covers a
  small fraction of known carrier variants, so "no overlap found" never means
  "not at risk" and never means "compatible". Do not output offspring risk
  percentages, do not infer phase, penetrance, anatomy, fertility,
  pregnancy, or hormone status, and route reproductive planning to clinical
  carrier screening and genetic counseling.
- Consent: both profiles are local, but treat the comparison as sensitive.
  Require an explicit acknowledgement that both people agreed, label the
  report with both profile names, and keep it out of AI chat context unless
  the user selects it.
- Prototype with synthetic profile pairs only; any private-genome check
  reports aggregate counts and stays on-device.

## Upgrade backlog from the 2026-11-07 review (items marked "inferred" were
not verified in the running UI)

AI chat
- "Bring your own AI": a `custom` provider for any OpenAI-compatible
  `/v1/chat/completions` SSE endpoint (OpenRouter, LM Studio, vLLM, llama.cpp,
  Together, Groq, OpenAI API keys). Add a Rust streaming command beside
  `stream_ollama_chat`, a settings card beside `ChatGptConnectionCard.svelte`
  (base URL, model, key), keep the key in the OS credential manager only, reuse
  `validate_service_url`, and disclose the exact origin unless the host is
  loopback. Anthropic Messages as a second adapter later. Wire the secondary
  model review path to the provider.
- Persist the provider per chat session, as context mode already is, so a
  reopened session cannot silently route DNA to a different host.
- Per-provider context budgets for `consultationContextPack` (user-supplied
  context-window size for `custom`). Document that embeddings stay on Ollama.

MCP server
- Constant-time token compare; prefer an env var over `--mcp-auth-token` (it
  shows in process listings); document that most clients do not send
  `params._meta`.
- A no-genotype mode (counts and metadata only), result caps, and a tool-name
  audit log that never records values.
- Add `resources/list` (evidence policy, prohibited claims) and `prompts/list`
  (consult-with-evidence-tiers), and `readOnlyHint`/`destructiveHint`
  annotations. Verify `notifications/initialized` gets no reply.
- Stop accepting `ollama_token` as a call argument (use the keyring) and limit
  `scan_ollama_models` to the configured endpoint.
- Generate the README tool table from `tools/list` in `audit:mcp`; state that
  "read-only" tools still return genotype values.

UX and research
- First-run checklist card (liftover chain, catalogs, import, optional AI) and
  a "Connected services" status strip (Ollama, ChatGPT, Qdrant) — inferred.
- Command palette / keyboard tab switching — inferred; check responsive
  overflow of research and agent panels with `audit:tauri-ui` at 640px.
- Per-job research report export with provenance and source build; "what
  changed since last sync" in the evidence library; inline `explain_vector_match`
  in the vector atlas; saved Discovery queries and watchlists (counts only, no
  GWAS-count risk scores); show the gnomAD remote-query disclosure in setup.
