# Using Genomics Caddy

This page covers the desktop workflow after [first launch](../../README.md#first-launch).
It is not a medical guide.

## Reference catalogs

Signed GitHub builds do not ship ClinVar, dbSNP, GWAS, PharmGKB, or the
liftover chain. After you install, open **Data & updates** in the left
sidebar and run **Sync All Missing**. Wait until those catalogs are
downloaded **and indexed**. For typical AncestryDNA / 23andMe GRCh37
files, also use **Liftover assembly** → **Download chain**. Marker packs
that ship with the app are not a substitute for these databases. The current
ClinVar variant and submission-summary downloads together are about 846 MB
compressed; their local indexes require extra disk space. Transfer estimates
use reported file size and observed speed; indexing reports processed rows and
does not invent a byte-based ETA for decompressed data.

## Limits

- Consumer-array DNA is probabilistic context, not a diagnosis.
- Missing or uncalled markers stay **unknown**, not negative evidence.
- Chromosome-pattern output (`Female-like`, `Male-like`, `Unknown`, `Uncertain`,
  or `Inconclusive`) is a conservative consumer-array routing hint, not a
  clinical karyotype. Mixed X/Y calls do not diagnose mosaicism or chimerism;
  the display does not determine gender identity, anatomy, fertility,
  pregnancy, or hormone status.
- When the pattern is **Inconclusive**, the app may offer a one-time prompt to
  open the optional reproductive/hormone Context settings. The chromosome
  indicator in the profile header or Active profiles list can reopen that
  explanation at any time.
  Context is user-provided routing information; it never changes the DNA
  result. No biological-sex answer is required.
- Reproductive, cycle, contraceptive, menopause, postpartum, and pelvic-disease
  prompts require context you enter. DNA does not infer them.
- Fibroid prompts are clinician-workup context, not a DNA risk score, diagnosis,
  recurrence/growth prediction, or treatment selector. The app can organize
  user-entered imaging, symptoms, prior treatment/pathology, and goals; common
  inherited associations cannot establish whether a fibroid is present or
  explain a particular person's recurrence. Five selected GWAS loci may be
  reported separately as population research associations; they are not summed
  into a personal score and do not predict IUD suitability or response. A small
  number of specific hereditary syndromes require separate clinical genetics
  evaluation.
- Raw DNA never by itself changes medication, high-dose supplements, botanical
  products, or a permanent restrictive diet.

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

Findings with an authored biological or life-stage applicability show a compact
context badge, such as **Pregnancy**, **Postpartum**, **Menopause**, or
**Hormone therapy**. Cards show the primary badge; additional context stays
behind a `+N more` disclosure that expands in the page flow. These badges
describe why a finding may be relevant; they do not infer pregnancy, current
hormone levels, anatomy, or reproductive status from DNA.

The report separates **catalog-linked associations** on curated findings from
**potential disease associations** discovered by scanning the full imported
genome against the local ClinVar variant and condition-specific submission
indexes. The latter requires both ClinVar indexes, a known source build, and
verified allele orientation; only exact single-base matches are included.
Individual ClinVar submission classifications and review status remain visible,
and records with explicit somatic/oncogenic labels are excluded. Some records
may not specify origin. This is not a diagnosis, a personal-risk estimate, or a
complete disease screen. GWAS rows remain locus–trait statistics, ClinGen
entries gene-level disease-validity context, and ClinPGx entries medication
response annotations.

POTS and dysautonomia appear as clinical context, not a DNA score. Standing-
related symptoms, positional heart rate/blood pressure, medication context, and
other possible causes are matters for clinical evaluation; the app does not
recommend salt loading, supplements, or medication changes from DNA.

For fibroids, the clinician discussion prompts distinguish symptom control
from treating the fibroid itself and include fibroid location and uterine-cavity
distortion as details to review with the treating clinician. For example,
ACOG describes a progestin-releasing IUD as an option for some people whose
fibroids do not distort the uterine cavity; it may reduce bleeding but does not
treat the fibroids themselves ([ACOG](https://www.acog.org/womens-health/faqs/uterine-fibroids),
[NICE NG88](https://www.nice.org.uk/guidance/ng88/chapter/Recommendations)).
This is general background, not an assessment of an individual's suitability.

For a bounded local report, the GWAS Catalog index retains up to the 12
strongest study records per rsID (ordered by reported p-value); the linked
study records remain available for source review. This cap does not imply that
the catalog contains only 12 associations for that marker.

Long lab follow-up lists open with the highest-priority group; lower-priority
groups and the deep-dive health-area directory expand in place. Counts keep the
full scope visible without crowding the first report view.

Optional **Context** and **Diary** fields (medications, allergies, labs,
cycle/menopause/postpartum notes, diet, and product labels) stay per-profile.
They inform chat and dashboard prompts.
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
