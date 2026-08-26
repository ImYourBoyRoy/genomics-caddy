# Genomics Caddy

A privacy-first, high-performance local personal genomic exploration application. Parse raw DNA genotype exports (AncestryDNA and 23andMe), standardize coordinates to the modern GRCh38 assembly, run local clinical and wellness trait reports, and connect securely to AI assistants using the built-in Model Context Protocol (MCP) server.

Created by **Roy Dawson IV**.

---

## 🚀 Use Case Synopsis

Many public genomic analysis platforms sell user data or require uploading sensitive files to cloud servers. **Genomics Caddy** runs completely local on your machine. It standardizes raw genotype files, maps positions GRCh37 → GRCh38 using local UCSC chain alignments, evaluates markers for mood (PMDD), neurodiversity (ADHD/dopamine), and inflammatory profiles, and acts as an MCP server so you can query your genome using local AI agents (like Claude Desktop, Cursor, Cline, or Claude Code).

---

## 💎 Core Features & Elements

### 1. Genomic Position Liftover (GRCh37 ➔ GRCh38)
* Standardizes coordinates of raw uploads from AncestryDNA or 23andMe.
* Alignments are executed locally using UCSC chain mapping files (`GRCh37_to_GRCh38.chain.gz`) to guarantee coordinate accuracy.

### 2. Direction-Aware Scoring & Severity Engine
* Matches user genotypes against candidate alleles to separate findings into **Risk, Protective, Trait, and Context-Dependent** categories.
* **Risk-Only Scoring:** Overall and section signal scores only reflect risk-direction alleles. Protective or neutral alleles do not inflate scores.
* **Clinical Suppression:** High-stakes sections where all markers require clinical confirmation (e.g., Cancer predisposition or Pharmacogenomics) suppress numerical percentages, showing descriptive safety summaries instead.
* **Color Legend:** Standardized card styling maps results to 8 severity classes (`no_data`, `benign`, `confirmation_required`, `protective`, `trait`, `context_dependent`, `moderate_risk`, `high_risk`).
* **Probabilistic evidence bands:** Marker tiers A–E are interpreted by evidence prefix, including custom subtiers. Tier E is an evidence-gap/guardrail, not a positive finding. Common SNP coverage expands the questions the tool can ask; it does not establish a diagnosis, current hormone level, medication response, or personal disease probability by itself.
* **Claim framing:** Each active marker displays a separate claim-status frame for clinical confirmation, replicated context, preliminary research, research-only, or evidence-gap status. AI payloads carry the marker's claim boundaries, callability state, and source names beside the interpretation.
* **Menstrual and reproductive context:** The hormone pack includes explicit cycle-phase physiology, PMDD symptom-timing and steroid-sensitivity guardrails, exact-contraceptive-ingredient prompts, adenomyosis workup limits, androgen/prostate/testicular boundaries, and clearly labeled research-only loci. An optional per-profile context selector controls whether menstrual, uterine/pelvic, ovarian, androgen, menopause, or pregnancy/postpartum guidance is shown; it is never inferred from genotype, chromosome calls, gender, anatomy, fertility, pregnancy, or hormone status. When a context is selected, the report can prioritize resource-mapped findings for that context while keeping every reproductive marker visible and exportable. The report must not infer current estrogen/progesterone levels, contraceptive composition, PMDD, or adenomyosis from raw DNA. Confirmed Factor V Leiden/prothrombin context can add a clinical contraception/VTE review prompt; it never recommends a medication change.
* **Conditional actionability:** Food, supplement, activity, and medication context is qualified by symptoms, labs, allergies, pregnancy/lactation status, kidney/liver conditions, and clinician/pharmacist review. Raw DNA alone never triggers medication changes, high-dose supplements, or permanent restrictive diets.
* **Supplement safety layer:** Selenium, magnesium, omega-3, iron, vitamin D, folate, and creatine prompts are food-first and medication-aware. They ask for labs or clinical context where appropriate and do not infer supplement need, dose, safety, or efficacy from a common SNP.
* **Cycle-support layer:** When the person selects a relevant context, reports expose structured timing logs, PMDD-like symptom tracking, exact contraceptive ingredient prompts, symptom-day support, heavy-bleeding/pelvic-pain escalation, adenomyosis workup boundaries, or androgen/prostate/testicular follow-up. With no selection, context-specific guidance and contraceptive composition warnings remain hidden. The layer does not infer current estrogen/progesterone, PMDD, adenomyosis, or medication response from DNA.
* **Runtime support-resource layer:** AI consultations receive pack-aware evidence policy, phenotype questions, lab overlays, callability rules, food-decision constraints, medication safety notes, PRS limits, activity stop-sign guardrails, and cycle-support guidance. Self-reported body-system and reproductive/hormone context is kept separate from conservative chromosome-call context.
* **Actionability dashboard:** Report summaries now expose relevant activity/recovery and cycle/reproductive guardrails plus medication-context questions alongside conditional food, supplement, and lab prompts. PGx explanations are framed as allele components requiring a complete clinical interpretation.
* **Metabolic and lipid follow-up:** Existing FTO, PPARG, TCF7L2, SLC30A8, KCNJ11, GCKR, APOA5, PNPLA3, TM6SF2, LPA, LDLR, APOB, and PCSK9 coverage can now surface conditional glucose, triglyceride/liver, and atherogenic-lipid follow-up. The resource layer points users toward measured HbA1c, fasting glucose, lipid panel, ApoB, Lp(a), blood-pressure, and liver evaluation when clinically appropriate; it never diagnoses diabetes/fatty liver or prescribes therapy from DNA.
* **Broader conditional follow-up:** Thyroid-pathway, circadian/sleep, and allergy/food-reaction markers can now surface symptom logs and appropriate discussion tests such as TSH/free T4, sleep studies when indicated, or allergist-directed testing. These rules remain phenotype- and lab-conditioned; common SNPs do not diagnose thyroid disease, sleep apnea, food allergy, or medication response.
* **Medication safety follow-up:** PGx findings can now surface drug-specific clinical review prompts for CYP2C19/clopidogrel, CYP2C9/VKORC1/CYP4F2/warfarin, SLCO1B1/ABCG2/CYP2C9/statins, CYP2D6/codeine or tramadol, CYP2D6/tamoxifen, and CYP2D6/CYP2C19/CYP2B6 antidepressant questions, alongside DPYD/fluoropyrimidines, TPMT/NUDT15/thiopurines, HLA drug hypersensitivity, G6PD/oxidative medications, and RYR1/CACNA1S anesthesia risk. Composite gene labels are matched by their component symbols; raw consumer-array calls still cannot select, clear, or dose medication.
* **Cross-domain activity safety:** Activity guidance is authored by related pack signals, so bone findings can surface connective-tissue guardrails, airway/allergy findings can surface sleep-recovery context, and kidney, pain/migraine, and sleep packs can expose individualized activity, hydration, or recovery follow-up. Symptoms, history, measured labs, and clinical clearance remain higher priority than genotype.
* **DNA callability visibility:** Each report section shows how many curated markers have usable DNA calls and explicitly labels missing/uncalled markers as unknown rather than negative evidence. AI context uses `chromosome_call_context` for the conservative Y-call hint.
* **Dynamic discovery catalog:** The Curated Catalog scan derives its filters from every category present in `discovery_catalog.json`, displays per-category entry counts, and provides Select all/Clear all controls. The default selection stays bounded to the established high-value domains; newly added research categories remain reachable without being silently scanned.

### 3. Plain-English Layperson Translation System
* Includes a layperson dictionary for curated candidate rsIDs, translating terms such as "homozygous" to "two copies of the variant" and "metabolizes" to "breaks down/clears".
* **View Modes:** Toggle dynamically between:
  * **Simple Mode 🌱:** Hides all complex terminology and evidence lists for average users.
  * **Clinical Mode 🏥:** Exposes CPIC guidelines, PubMed citations, and exact biological mechanisms.
  * **Dual Mode 👥 (Default):** Stacks both views, placing the plain-English translation in a highlighted summary box above the clinical card.

### 4. Local RAG Evidence Library & Search Engine
* **Local SQLite Vector Store:** Stores guideline citations and references mapped directly from trait JSONs.
* **On-the-fly Cosine Similarity:** Computes vector cosine similarity locally in memory (Rust) to keep the app portable.
* **Dynamic Embedding Fetching:** Fetches embeddings dynamically from the configured remote Ollama instance using the user-provided `ollamaUrl` and `ollamaToken` (no hardcoded IP addresses).
* **Evidence Library Panel:** Provides keyword and semantic searching directly within Svelte to reference CPIC and PubMed guidelines.

### 5. 10 Specialty Consultation Modes
* **Specialized System Prompts:** Tailors AI behavior to 10 specific health contexts:
  * **🧬 General:** Broad genomic overview and prioritization guide.
  * **💊 Pharmacogenomics (PGx):** Strict focus on drug metabolism (CYP450, DPYD) and safety warnings.
  * **🍎 Nutrients & Methylation:** One-carbon cycle dynamics (MTHFR, COMT, PEMT) and dietary recommendations.
  * **🏃 Metabolic Health & T2D:** Blood sugar, insulin sensitivity (APOE, FTO), and lifestyle variables.
  * **🌙 Sleep & Circadian:** Sleep duration and timing optimization based on CLOCK/PER2 predispositions.
  * **🧠 Brain & Mood:** Dopamine/serotonin synthesis and stress responses (COMT, DRD2).
  * **🦴 Joints & Connective Tissue:** Collagen structure and recovery protocols (COL1A1, COL5A1).
  * **🛡️ Thyroid & Autoimmune:** Thyroid hormone conversion (DIO1, DIO2) and immune cofactor links.
  * **❤️ Cardiovascular Health:** Vascular integrity, blood pressure regulation, and cardiovascular habit guides.
  * **🌸 Menstrual Cycle & Hormone Context:** Cycle physiology, PMDD/PMS timing, progestin-versus-combined contraception context, and adenomyosis/endometriosis workup boundaries.

### 6. Dual-Model Safety Review Pipeline
* **Secondary Model cross-checking:** Primary model drafts can be automatically cross-checked by a secondary safety review model (e.g., MedGemma).
* **Clinical Correction warnings:** Highlight clinical overclaiming, dosing advice, or diagnosing assertions in an amber safety warning card.
* **Verification transparency:** Keeps the safety review log fully transparent and embeds review logs directly into clinical handoff exports.

### 7. Allele-Orientation Validation & DPYD Gate
* **DPYD rs55886062 Correction:** Corrected the `effect_allele` for `rs55886062` to genomic plus-strand `C`.
* **Orientation Validation Gate:** Implemented Rust backend verification to check strand orientation. If orientation is unverified or mismatch, it replaces the interpretation with a block warning and restricts severity, completely preventing false clinical claims.

### 8. Private AI Consultation & Multimodal Assistance
* Connects directly to local/remote Ollama servers with optional token/reverse-proxy authentication.
* **Three-Panel Layout Overhaul:** Splits the interface into a left Chat History Sidebar (backed by local storage, supporting rename/delete), a center Chat Area, and a right sliding Settings Drawer. Toggling the settings drawer closed maximizes the chat viewport width.
* **Sliding Sidebar Transition:** Integrated reactive state binding and CSS transitions (`width: 0`, `flex: 0 0 0px`, `overflow: hidden`) on the history sidebar, allowing it to slide open and closed smoothly.
* **Strict Profile Isolation (Round 2):** Chat history is strictly isolated per DNA profile (`sampleId`). Switching dashboard profiles automatically saves/restores the active session for the chosen profile and filters the sidebar to display only that profile's session history.
* **Interactive Thinking Controls & Details Collapse Fix (Round 2):** Adds toggles to hide/show thoughts or auto-collapse completed thoughts in the settings drawer. Resolved details tag force-open issues during streaming by replacing index-based logic with unique message-key mapping.
* **Pruning & Branching (Round 2):** Allows individual message deletion (`🗑`) and prompt editing (`✏️`). Editing a previous prompt copies it to the input field and truncates the conversation history from that point forward, allowing you to branch chats.
* **Dynamic Curated Prompt Filtering (Round 2):** Renders curated helpers bar dynamically, displaying shortcut questions only when their respective genomic categories contain active findings (`effect > 0`).
* **AI Scope Pushback Rules (Round 2):** Injects strict scope pushback commands into the system prompt configuration. The model is trained to decline to speculate or answer questions from general knowledge about genes, variants, or conditions that are not explicitly present in the loaded JSON genomic context.
* **Extended Thinking Mode:** Slider allows adjusting output tokens (`num_predict` from `512` to `8,192` tokens). A checkbox **"Extended Thinking"** forces the predicted limit to `8,192` tokens, preventing deep reasoning models (like DeepSeek R1) from truncating mid-thought. Reasoning models are automatically detected by name to pre-enable this mode.
* **Precise Model Capabilities Detection:** Automatically queries `/api/show` to extract model families (`clip`/`mllama`/`vision`/`vl`) and layers (`model_info`). This guarantees that only models with active vision projector layers enable image attachments, avoiding false positives on text-only quantizations (like a stripped `medgemma:latest` GGUF).
* **Multimodal Vision Integration:** Allows dragging, pasting, or uploading images directly into the chat. Images are normalized using HTML5 Canvas to a max size of 1024px in a standard JPEG format to fit comfortably in local vision models.
* **Session Token Tracker:** Live statistics bar in the sidebar tracking accumulated prompt, response, and total tokens. Includes a visual context gauge reflecting how much of the context window has been consumed.
* **Prompt Context Inspector:** Collapse/expand modal that lets you audit the exact system prompt and JSON context payload currently being sent to the AI.
* **Copy & Export Options:** Instantly copy assistant responses to the clipboard (stripping raw thinking tags) or export entire conversations as Markdown files saved locally.
* **Global Dialog State Store:** Extracted custom alert and confirmation dialog states into a global, reactive Svelte 5 store ([dialogState.svelte.ts](src/lib/utils/dialogState.svelte.ts)), replacing browser popups and component prop boilerplate.
* **Session Management Serialization:** Offloaded session loading, saving, and template factory creation to [chatSession.ts](src/lib/utils/chatSession.ts) to keep components modular and under 500 lines.
* **Privacy Controls:** Choose which trait packs to feed into the model, and toggle the `Only active findings (effect > 0)` option to restrict context strictly to positive variants, saving tokens and preserving privacy.
* **Anti-Hallucination Formatting:** Uses bullet-point structures and clear bracketed markers rather than numbered lists to prevent low-parameter models (like `gemma4:e4b` or `tinyllama`) from mimicking the prompt rules.
* **Strict Parameters:** Enforces `temperature: 0.0` in Ollama options by default to completely eliminate genomic hallucinations.

### 9. Dynamic MCP Integration Hub
* Implements a stdin/stdout Model Context Protocol (MCP) server directly.
* **Vite Environment Sync:** Automatically toggles commands between Development mode (using `npm run mcp`) and Production mode (using the compiled app binary).
* **Tauri Executable Auto-Detection:** Dynamically queries the exact path of the running executable on the user's filesystem.
* **Active Tool Catalog:** Exposes a list of all active MCP tools and parameters in Svelte using live schemas queried from the Rust backend.

### 10. Live Chromosome Density & Variant Map
* **SQLite Live Density Query:** Computes and renders SNP density across 24 chromosomes (1-22, X, Y) based on actual records stored in the local SQLite genotypes database.
* **Risk Coordinate Mapping:** Maps all evaluated risk/moderate/needs-confirmation variants at their exact physical base-pair positions on the chromosome capsules.
* **Interactive Tooltips:** Hovering over mapped variant indicators displays full gene details, rsID, and severity class.

### 11. Advanced Report Filters & Severity Sorting
* **Multiple Filtering Criteria:** Toggle between showing undetected benign markers, filtering strictly to active risk findings, and selecting specific evidence tiers (Tier A/B only).
* **Severity Ranking Sort:** Reorders markers dynamically within each section to bubble up High Risk and Needs Confirmation markers to the top.
* **Wrapped Exports:** JSON exports wrap raw reports in metadata envelopes containing version numbers, timestamps, and sample chromosome-call context.
* **Conservative chromosome context:** Missing Y calls remain unknown rather than being treated as proof of XX. Chromosome-call context is a biological hint only; it is not gender identity, anatomy, fertility, pregnancy status, or hormone status. Marker packs can declare biological applicability such as `xx_reproductive`, `xy_reproductive`, `x_linked`, or `y_linked`.

---

## 🛠️ Integration Guide

### 1. Standalone Desktop GUI
To run the Svelte dev server and the Tauri desktop window:
```bash
npm install
npm run tauri dev
```

To build the static production bundle (frontend only):
```bash
npm run build
```

To **purge build caches** (npm + Cargo only — never touches `data/`, downloads, or SQLite) and compile a **production desktop release** (Windows, macOS, or Linux):
```bash
npm run build:release
```

Or platform-native scripts:
```powershell
# Windows
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File .\scripts\purge_and_build.ps1
```
```bash
# Linux / macOS
bash ./scripts/purge_and_build.sh
```

**Linux one-time prerequisites** (Tauri WebKitGTK 4.1 + GTK headers):
```bash
npm run setup:linux
# or: bash ./scripts/setup_linux_deps.sh && npm run system:check
```

**Linux desktop logo / launcher** (DNA icon instead of a generic gear in the dock):
```bash
npm run desktop:linux
# or regenerate icons from static/logo.png then reinstall:
npm run icons:regen && npm run desktop:linux
```

Options (via `npm run build:release -- …` or the shell/PowerShell scripts):
- `--purge-only` / `-PurgeOnly` — clear caches without building (`npm run purge:build`)
- `--skip-purge` / `-SkipPurge` — build without clearing caches first
- `--skip-checks` / `-SkipChecks` — skip `npm run check` and `cargo check` before the release build
- `--dry-run` / `-DryRun` — show what would be removed

Release output lands in **`App/`** (portable binary + sidecars) with persistence in **`App/Data/`**. Build caches stay in `src-tauri/target/` and are safe to wipe. On Linux the staged binary is `App/DNA-Tools`; on Windows it is `App/DNA-Tools.exe`.

### Remote Cross-Compilation (Ubuntu Builder)

If you have configured the `Remote_Build` infrastructure, you can orchestrate builds across macOS, Linux, and Windows from a single machine without needing native toolchains locally:

```bash
# Build for all platforms (produces raw executables by default for fast testing)
npm run build:remote:all

# Or explicitly create the full packaged installers (.dmg, .msi, .nsis, .deb)
python scripts/build.py --target all --create-bundle

# Or target a specific OS
npm run build:remote:windows
```

*Note: The Windows build is natively cross-compiled on the Ubuntu host using `mingw-w64`.*

### Build timing benchmarks (laptop A vs B)

Timed purge / full purge+rebuild (writes reports under `App/Data/benchmarks/`):

```bash
# Purge caches only (timed)
npm run bench:purge

# Full purge + production rebuild (timed) — best apples-to-apples comparison
npm run bench:rebuild

# Same rebuild, skip pre-checks (faster iteration)
npm run bench:rebuild:fast
```

Windows-native:

```powershell
pwsh -File .\scripts\benchmark_build.ps1 -Json
pwsh -File .\scripts\benchmark_build.ps1 -PurgeOnly -Json
```

Linux/macOS-native:

```bash
bash ./scripts/benchmark_build.sh --json
bash ./scripts/benchmark_build.sh --purge-only --json
```

One-time migration from legacy `data/`:
```powershell
pwsh -File .\scripts\migrate_data_to_app.ps1
```

---

### 2. Headless sweep worker (Docker / server)

For high-core servers (e.g. 80 CPUs), run the enrichment sweep without the desktop UI. Pipeline tuning is **dynamic** — it auto-detects logical cores, uses **all but one** for sweep work (4-core laptop → 3 workers, 80-core server → 79), and scales batch/concurrency from measured Ollama/Qdrant latency.

```bash
docker compose -f docker/docker-compose.yml build genomics-worker
GENOMICS_CPU_LIMIT=80 GENOMICS_SAMPLE_ID=1 docker compose -f docker/docker-compose.yml up genomics-worker
```

Mount your data at `/data` (same layout as `App/Data/`). Progress streams as NDJSON on stdout (`GENOMICS_PROGRESS`, `GENOMICS_FINDING`). See **`docker/README.md`** for resume, updates, and optional static UI.

Local CLI (same binary as the desktop app):

```powershell
$env:GENOMICS_DATA_DIR = "C:\path\to\App\Data"
$env:GENOMICS_CPU_LIMIT = "80"
.\App\DNA-Tools.exe --headless-sweep --sample-id=1
```

---

### 3. Headless MCP Server Integration
Genomics Caddy embeds a Model Context Protocol (MCP) server. Start the server in headless MCP mode:

* **In Development:**
  ```bash
  npm run mcp
  ```
* **In Production:**
  Launch the compiled executable with the `--mcp` flag:
  ```bash
  "C:/path/to/installed/tauri-app.exe" --mcp
  ```

#### Claude Desktop Configuration
Add the server configuration to your global `claude_desktop_config.json` (`%APPDATA%\Claude\claude_desktop_config.json`):

* **For Dev Mode (npm):**
  ```json
  {
    "mcpServers": {
      "genomics-caddy-dev": {
        "command": "npm",
        "args": ["run", "mcp"],
        "options": {
          "cwd": "/path/to/AI/DNA_Tools"
        }
      }
    }
  }
  ```
* **For Production Mode (Executable):**
  ```json
  {
    "mcpServers": {
      "genomics-caddy": {
        "command": "C:/path/to/installed/tauri-app.exe",
        "args": ["--mcp"]
      }
    }
  }
  ```

#### Cursor Configuration
1. Open Cursor and navigate to **Settings > Features > MCP**.
2. Click **+ Add New MCP Server**.
3. Add a **Command** type server:
   * **Dev Command:** `npm run mcp`
   * **Prod Command:** `"C:/path/to/installed/tauri-app.exe" --mcp`

#### Cline / Roo Code Configuration
Add the configuration to `cline_mcp_settings.json` (`%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json`):
```json
{
  "mcpServers": {
    "genomics-caddy": {
      "command": "C:/path/to/installed/tauri-app.exe",
      "args": ["--mcp"],
      "disabled": false
    }
  }
}
```

#### Claude Code (CLI) Configuration
Add the server globally to Claude Code by executing:
* **Dev Mode:** `claude mcp add genomics-caddy-dev npm -- run mcp`
* **Prod Mode:** `claude mcp add genomics-caddy "C:/path/to/installed/tauri-app.exe" -- --mcp`

---

## Evidence Workbench (vector + SQLite associations)

The **Evidence Workbench** tab in the AI Evidence Library provides inspectable, source-grounded association cards:

- **Hybrid search** — Qdrant semantic discovery + SQLite `association_facts` with filters (data quality, direction, evidence tier).
- **Quality dashboard** — stale vector counts, schema mismatch, source coverage bars, cache health; **Backfill payloads** normalizes existing Qdrant points without re-embedding when text is unchanged.
- **Dynamic candidates** — surfaced during enrichment; promote to `curated_lite` only when GWAS/association evidence exists; rejected rsIDs are suppressed from search.
- **Evidence packets** — export JSON (facts + source records + similar hits + prohibited claims) via save dialog.

MCP tools: `search_vector_associations`, `get_variant_evidence_card`, `explain_vector_match`, `export_evidence_packet`, `backfill_evidence_payloads`, `get_quality_dashboard`, `list_candidate_markers`.

Data layers: `association_facts` (per-sample truth) → `api_cache_entries` (cross-sample HTTP cache) → live APIs; raw responses stored in `source_records`.

---

## 🐚 Commands & Arguments

### NPM Build and Execution Scripts
* **`npm run dev`**: Spawns Vite development web server.
* **`npm run tauri:dev`**: Starts Vite server and mounts the Tauri desktop window.
* **`npm run build`**: Compiles static production web assets into `/build`.
* **`npm run check`**: Runs Svelte compiler and TypeScript diagnostics.
* **`npm run validate:packs`**: Validates curated marker-pack schemas, evidence tiers, actionability policy, source-only support-resource contracts, and runtime mirror manifests without removing any pack.
* **`npm run audit:resources`**: Audits every curated pack, discovery catalog, and support resource for probability/callability/actionability gates, claim-boundary fields, source-registry coverage, source/runtime parity, actionability-rule source coverage, actionability coverage, and deterministic wording that needs human review.
* **`npm run audit:dna-fixtures`**: Read-only coverage audit for root DNA `.txt`/`.zip` fixtures. Reports row counts, curated rsID coverage, chromosome-call counts, and Y-call counts; never prints or imports genotype values.
* **`npm run smoke`**: Runs local integration smoke tests (Qdrant/NCBI/Ollama) using `.env` beside the project root.
* **`npm run mcp`**: Spawns the Tauri dev process in headless MCP server mode.
* **`npm run update:all`**: Full refresh — npm itself, rustup stable (+ sync `rust-version`), bump **all** npm deps to latest (including TypeScript majors), Cargo upgrade/update, then verify. TypeScript **7** is primary (`tsc` via `scripts/run_tsc.mjs`). `npm run check` shims the TS6 API for Svelte tooling (`@typescript/typescript6` + preload). `.npmrc` sets `legacy-peer-deps=true` so Kit’s outdated TS peerOptional range does not block installs. Flags: `--dry-run`, `--skip-toolchains`, `--skip-npm`, `--skip-cargo`, `--skip-verify`, `--update-node`.
* **`npm run update:all:dry`**: Same plan as Update All without writing anything.
* **`npm run update:deps`**: Packages only (npm + Cargo); skip toolchain self-updates.
* **`npm run update:toolchains`**: Toolchains only (npm global + rustup); skip project deps and verify.

### Local `.env` and smoke tests

Connection hosts are configured in-app under **Advanced → Connections** (also **Open Connections** in the left sidebar). **SQLite (UI-saved) overrides `.env`**; `.env` only fills empty fields as bootstrap.

1. Copy the template: `copy .env.example .env` (Windows) or `cp .env.example .env`.
2. Optionally set secrets / bootstrap URLs in `.env` (never commit this file):

```env
QDRANT_URL=http://your-host:6333
QDRANT_API_KEY=your-key
QDRANT_COLLECTION=your_collection
NCBI_API_KEY=
OLLAMA_URL=http://your-ollama-host:11434
OLLAMA_TOKEN=
```

3. Or leave `.env` blank and enter URLs in **Advanced → Connections**, then **Save & Verify**. Choose a vector provider (**Qdrant**, **Pinecone**, **Chroma**, or **Weaviate**) — dense research sweeps work on all four; named vectors remain Qdrant-only. Use **Reset to localhost** for `127.0.0.1` (warns if Ollama/Qdrant are missing locally). Same tab: **Install / Update / Update all / Remove** Ollama models, **Browse library** / **Search models** links to ollama.com, and **Check for updates** (Ollama + Qdrant when applicable).

4. Run smoke tests:

```bash
npm run smoke
```

Checks performed:
- Vector store reachability for the configured provider (Qdrant collection / Pinecone index / Chroma collection / Weaviate class)
- NCBI esearch (when `NCBI_API_KEY` is set)
- Ollama model list (when `OLLAMA_URL` is set)

### Purging sensitive files from git history

If personal genotype files were ever committed, run (rewrites local history):

```powershell
pwsh -File scripts/purge_git_secrets.ps1
```

If the repo was pushed to a remote, follow with `git push --force --all` and rotate any exposed API keys.

### Application Executable Flags
* **`--mcp`**: Launches the stdin/stdout JSON-RPC 2.0 Model Context Protocol loop (read-only tools by default).
* **`--mcp-write`**: Enables mutating MCP tools (`delete_chat_session`, `backfill_evidence_payloads`, `update_candidate_marker_status`, `enable_named_vectors_collection`). Combine with `--mcp` when an agent must perform writes.
* **`--mcp-auth-token=<secret>`** (optional): When set (or when `GENOMICS_MCP_TOKEN` is in the environment), every MCP `tools/call` must include `params._meta.authToken` matching that value. `ping` is exempt. Omit the flag for local-only, unauthenticated MCP (default).

### Database storage (plaintext, per-sample)
User genomes are stored as **plaintext SQLite** under `App/Data/`:
- `user_genome.db` — sample registry + app settings only
- `samples/<id>/genome.db` — that profile’s genotypes, chat, research jobs, and findings

Deleting a sample in the UI removes the registry row and the entire `samples/<id>/` folder. Shared public databases (`clinvar.db`, `dbsnp.db`, `api_cache.db`, `genomics_reference.db`) are unchanged.

App-level AES sealing of `user_genome.db` was removed: genotype source files are already plaintext, and OS keyring sealing broke cross-OS copies. Prefer full-disk encryption (BitLocker / FileVault / LUKS) for at-rest protection.

Large offline raw downloads (uncompressed text/JSON over ~64 MiB) are gzip-compacted after a successful import; readers open `.gz` / `.bz2` transparently.

---

## 🤖 Available MCP Tools

1. **`list_samples`**
   * **Description:** Lists all imported DNA samples in the local SQLite database.
   * **Arguments:** None.

2. **`get_variants_by_rsid`**
   * **Description:** Queries specific genotypes for a list of rsIDs in a given sample.
   * **Arguments:**
     * `sample_id` (integer, required): The target sample ID.
     * `rsids` (array of strings, required): List of rsIDs to query, e.g., `["rs4680", "rs1801132"]`.

3. **`get_variants_in_region`**
   * **Description:** Queries all standard variants in a given chromosome region (GRCh38 coordinates).
   * **Arguments:**
     * `sample_id` (integer, required): The target sample ID.
     * `chromosome` (string, required): Chromosome number or label, e.g., `"1"`, `"X"`.
     * `start` (integer, required): Start base-pair position.
     * `end` (integer, required): End base-pair position.

4. **`generate_report`**
   * **Description:** Generates a full direction-aware trait report for a sample using a provided template. Returns evaluated markers with severity classes, section summaries, and risk-direction-only signal scores.
   * **Arguments:**
     * `sample_id` (integer, required): The target sample ID.
     * `template_json` (string, required): JSON string of the report template containing sections and markers.

5. **`list_packs`**
   * **Description:** Lists all available predefined genomic marker packs/bundles (like Core, PGx, Metabolic, etc.) and their descriptions.
   * **Arguments:** None.

6. **`get_report_for_packs`**
   * **Description:** Generates an evaluated genomic report for specific pack IDs (or all if omitted). Can optionally filter to only return active variant findings (effect alleles detected).
   * **Arguments:**
     * `sample_id` (integer, required): The target sample ID.
     * `pack_ids` (array of strings, optional): Predefined pack IDs to evaluate, e.g., `["core", "pgx"]`. If omitted/empty, evaluates all packs.
     * `only_active_findings` (boolean, optional): If true, filters out any evaluated markers where `effect_count == 0` or data is missing, returning only positive variant findings. Defaults to false.

7. **`list_evidence_sources`**
   * **Description:** Lists all unique source citations in the local RAG evidence library.
   * **Arguments:** None.

8. **`get_evidence_for_marker`**
   * **Description:** Queries the local evidence library for references and interpretation notes associated with a specific rsID.
   * **Arguments:**
     * `rsid` (string, required): The target rsID, e.g., `"rs4680"`.

9. **`search_evidence`**
   * **Description:** Performs keyword and semantic vector search in the local evidence library.
   * **Arguments:**
     * `query` (string, required): The search term or query.
     * `ollama_url` (string, optional): Ollama server URL for semantic search embeddings.
     * `ollama_token` (string, optional): Authentication token for remote Ollama server.

10. **`get_chat_sessions`**
    * **Description:** Lists saved consultation chat sessions from the local SQLite database.
    * **Arguments:**
      * `sample_id` (integer, optional): Filter sessions by sample ID.

11. **`delete_chat_session`**
    * **Description:** Deletes a specific consultation chat session.
    * **Arguments:**
      * `session_id` (string, required): The session ID to delete.

12. **`export_chat_history`**
    * **Description:** Exports a saved chat session history in a clean, human-readable Markdown format.
    * **Arguments:**
      * `session_id` (string, required): The session ID to export.

13. **`get_app_paths`**
    * **Description:** Retrieves the local application directory paths (database and marker packs folders).
    * **Arguments:** None.

14. **`check_chain_status`**
    * **Description:** Checks if the GRCh37-to-GRCh38 liftover chain alignment file is locally present.
    * **Arguments:** None.

15. **`get_current_exe`**
    * **Description:** Returns the absolute path of the running Genomics Caddy executable.
    * **Arguments:** None.

16. **`scan_ollama_models`**
    * **Description:** Queries a local or remote Ollama server to list all available LLM models.
    * **Arguments:**
      * `url` (string, required): Ollama server URL.
      * `token` (string, optional): Authentication token.

17. **`show_ollama_model`**
    * **Description:** Retrieves detailed configuration and parameters for a specific Ollama model.
    * **Arguments:**
      * `url` (string, required): Ollama server URL.
      * `token` (string, optional): Authentication token.
      * `name` (string, required): The model tag name.

18. **`get_active_ollama_models`**
    * **Description:** Queries a local or remote Ollama server to list currently loaded models and VRAM usage.
    * **Arguments:**
      * `url` (string, required): Ollama server URL.
      * `token` (string, optional): Authentication token.

---

## 🔧 Troubleshooting Remote Ollama Servers

If you are connecting to a remote Ollama server (e.g., `http://192.168.1.21:11434`) and your chat consultation responses are failing, review these issues:

### 1. Connection Refused
* **Problem:** Ollama is only listening on `localhost:11434` on the remote machine.
* **Fix:** On the remote machine, set `OLLAMA_HOST=0.0.0.0:11434` as an environment variable before launching the Ollama daemon.
  * *On Ubuntu (Systemd):*
    Add `Environment="OLLAMA_HOST=0.0.0.0"` or `EnvironmentFile=/etc/ollama/ollama.env` to `/etc/systemd/system/ollama.service.d/override.conf` and run `sudo systemctl restart ollama`.

### 2. HTTP 500 / Server Crash during Inference
* **Problem:** llama-server panics with `"llama_init_from_model: V cache quantization requires flash_attn"`.
* **Why:** The remote Ollama has KV cache quantization enabled (`OLLAMA_KV_CACHE_TYPE` is set to `q8_0` or `q4_0`), which requires **Flash Attention**. However, your remote GPU (such as a **Tesla P40** / Pascal architecture) does not support Flash Attention.
* **Fix:** Disable KV cache quantization by unsetting or removing `OLLAMA_KV_CACHE_TYPE` (or explicitly setting it to `f16`) on the remote server:
  * *On Ubuntu (Systemd):*
    Open your override configuration with `sudo systemctl edit ollama` and add/edit:
    ```ini
    [Service]
    Environment="OLLAMA_KV_CACHE_TYPE=f16"
    ```
    Reload and restart the service:
    ```bash
    sudo systemctl daemon-reload
    sudo systemctl restart ollama
    ```

---

## 📂 Project Layout

```
/DNA_Tools
│── README.md                # Public-facing usage and integration guide (ROY-STANDARD)
│── MEMORY.md                # Living session memory for AI developers
│── package.json             # NPM frontend scripts, shortcuts, and configuration
│── /src-tauri/testdata/     # Synthetic import fixtures only (no user DNA)
│── /src                     # Svelte 5 frontend app
│    ├── routes/
│    │    ├── +layout.ts     # Configures SPA routing
│    │    ├── +page.svelte   # Glassmorphic page orchestrator (under 500 lines)
│    ├── lib/
│    │    ├── api/           # Tauri RPC async wrapper layer
│    │    ├── components/    # Modular Svelte 5 views (each under 500 lines)
│    │    ├── marker-packs/  # Curated packs plus evidence, lab, food, activity, and safety resources
│    │    ├── styles/        # theme.css and print.css external stylesheets
│    │    ├── types/         # TypeScript interface definitions
│    │    ├── utils/         # Evidence tier and genotype calculation logic
│── /src-tauri               # Rust backend modules
│    ├── Cargo.toml          # Rust dependencies (rusqlite, flate2, rayon, reqwest)
│    ├── src/
│    │    ├── main.rs        # CLI handler (runs MCP server or GUI)
│    │    ├── lib.rs         # Tauri command handlers
│    │    ├── parser.rs      # Raw DNA file parser
│    │    ├── liftover.rs    # UCSC chain coordinate mapping (GRCh37 -> GRCh38)
│    │    ├── db.rs          # SQLite transactions and query logic
│    │    ├── mcp.rs         # Stdin/stdout MCP server loop
│    │    ├── report.rs      # Marker evaluation and markdown renderer
```

---

## 👤 Author & Links

* **Created by**: Roy Dawson IV
* **Email**: <Roy.Dawson.IV@gmail.com>
* **GitHub**: [https://github.com/imyourboyroy](https://github.com/imyourboyroy)
* **PyPi**: [https://pypi.org/user/ImYourBoyRoy/](https://pypi.org/user/ImYourBoyRoy/)
