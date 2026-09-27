# Deferred research and infrastructure

## Report overhaul (user feedback 2026-09-26)

Disease links shipped first (strand verification, ClinVar consensus gating,
may-be-relevant / carrier / disputed grouping). Remaining, in order:

- Copy cleanup: one report-level scope statement; remove repeated "not a
  diagnosis / not a personal risk estimate" prose from report sections and
  AI/clinician exports (keep structured `do_not_claim`/`confirm_with` fields);
  hide sections that render only caveats with no findings.
- UI polish: theme the Focus chips, Clear filters, Filters & ordering, and
  reading-mode buttons; replace the orange coverage banner with a small note;
  relabel "Context-dependent" as a targeted "Add context to refine" prompt.
- Overview drill-down: make Review first / Higher-priority / Context signals /
  Potentially favorable tiles open a list filtered to that group.
- Sex and anatomy: show Male / Female / Unknown; for Unknown, prompt for
  user-supplied reproductive anatomy; a clearly labelled Change button for all
  profiles. Anatomy is never inferred from DNA.
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

## Full-genome discovery beyond ClinVar

- Evaluate a separate, clearly research-labeled scan of local GWAS records across
  all imported calls. Match the reported/effect allele before showing any
  direction; keep trait association, study ancestry, p-value, and source record
  visible. Do not convert loci into a user-specific risk score.
- Do not treat a variant anywhere in a ClinGen-valid gene as a disease finding;
  require the curated variant/mechanism and inheritance evidence needed for that
  relationship. Keep PGx calls limited to interpretable gene/allele models.
- Benchmark output volume, query latency, report payload size, and usefulness
  before broadening beyond the current ClinVar SNV scan.

## Autonomic/POTS evidence

- Reassess only when replicated, clinically curated evidence supports a useful
  consumer-array model. Until then, keep POTS/dysautonomia in explicit clinical
  symptom-context routing; do not add candidate-gene scores or genotype-based
  diet, supplement, or medication advice.
