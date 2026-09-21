# Using Genomics Caddy

This page covers the desktop workflow after [first launch](../../README.md#first-launch).
It is not a medical guide.

## Reference catalogs

Signed GitHub builds do not ship ClinVar, dbSNP, GWAS, PharmGKB, or the
liftover chain. After you install, open **Data & updates** in the left
sidebar and run **Sync All Missing**. Wait until those catalogs are
downloaded **and indexed**. For typical AncestryDNA / 23andMe GRCh37
files, also use **Liftover assembly** → **Download chain**. Marker packs
that ship with the app are not a substitute for these databases.

## Limits

- Consumer-array DNA is probabilistic context, not a diagnosis.
- Missing or uncalled markers stay **unknown**, not negative evidence.
- Chromosome-call sex display (`Male` / `Female` / `Unknown` / `Uncertain`) is
  a coverage hint. It is not gender, anatomy, fertility, pregnancy, or hormone
  status.
- Reproductive, cycle, contraceptive, and pelvic-disease prompts require
  context you enter. DNA does not infer them.
- Raw DNA never by itself changes medication, high-dose supplements, or a
  permanent restrictive diet.

## Import

Supported inputs: AncestryDNA and 23andMe style `.txt`, `.csv`, `.tsv`, and
ZIP. Parsing alone does not write a profile.

1. Choose the file in the sidebar import panel.
2. Read the local review: vendor/format, accepted vs skipped rows, source
   build, orientation evidence, liftover availability, warnings.
3. Choose **Import profile**, **Replace profile**, or **Cancel**.
4. After confirmation, the app reads, normalizes coordinates, ingests
   genotypes, and prepares the report.

Empty files and conflicting duplicate marker rows are rejected. `NN` stays
no-data. Mixed `N` calls are blocked as ambiguous.

**Replace** rebuilds the existing profile ID after confirmation and keeps
entered Context/Diary. A **different name** creates a separate profile.
**Delete** removes that profile’s genotype database and profile-scoped
browser state.

GRCh37 sources map forward when the local UCSC chain is present. Explicit
GRCh38 sources keep source coordinates and use inverse mapping for GRCh37
when available. Unmapped positions stay explicit.

## Reading the report

| Mode | Purpose |
| --- | --- |
| **Simple** (default) | Plain-language findings, next steps, compact guidance |
| **Clinical** | Sources, claim boundaries, technical coverage |
| **Compare** | Simple and Clinical side by side |

Use Simple first. Filters can hide benign/uncalled markers; they do not turn
unknown coverage into a clean bill of health. Cancer and PGx sections that
require clinical confirmation suppress fake-precise risk percentages.

Optional **Context** and **Diary** fields (medications, allergies, labs,
cycle notes, diet) stay per-profile. They inform chat and dashboard prompts.
They never become genotype evidence.

## Exports and sharing

Use **Share this report** under the report header.

| Format | Raw genotype calls |
| --- | --- |
| Personal Simple / PDF | Presentation; not the technical trace |
| AI-ready JSON, AI Review ZIP, Clinician Handoff | Included for the exported findings |

Clinician and AI bundles also carry import provenance, claim boundaries, and
a privacy note. Share them only with the intended clinician, counselor, or
AI endpoint. Connected Chat includes raw calls for whatever finding scope
that session saved; a remote Ollama URL is disclosed in the UI.

Audience ZIP bundles contain `report.md`, `dna_analysis.json`, `context.json`,
`diary.csv`, `manifest.json`, and `PRIVACY.txt`.

## Where data lives

Plaintext SQLite under `App/Data/` (next to a release binary, or the
configured data root):

- `user_genome.db` — profile registry and app settings
- `samples/<id>/genome.db` — that profile’s genotypes, chat, and research

Prefer full-disk encryption for at-rest protection. Deleting a profile in the
UI removes its `samples/<id>/` folder.

## Related

- MCP for agents: [../mcp/README.md](../mcp/README.md)
- Local AI chat: [../ai-chat/README.md](../ai-chat/README.md)
