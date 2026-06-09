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

### 3. Plain-English Layperson Translation System
* Includes a complete dictionary mapping of all **99 candidate rsIDs** to simplified language (e.g., translating "homozygous" to "copies of variant gene" and "metabolizes" to "breaks down/clears").
* **View Modes:** Toggle dynamically between:
  * **Simple Mode 🌱:** Hides all complex terminology and evidence lists for average users.
  * **Clinical Mode 🏥:** Exposes CPIC guidelines, PubMed citations, and exact biological mechanisms.
  * **Dual Mode 👥 (Default):** Stacks both views, placing the plain-English translation in a highlighted summary box above the clinical card.

### 4. Local RAG Evidence Library & Search Engine
* **Local SQLite Vector Store:** Stores guideline citations and references mapped directly from trait JSONs.
* **On-the-fly Cosine Similarity:** Computes vector cosine similarity locally in memory (Rust) to keep the app portable.
* **Dynamic Embedding Fetching:** Fetches embeddings dynamically from the configured remote Ollama instance using the user-provided `ollamaUrl` and `ollamaToken` (no hardcoded IP addresses).
* **Evidence Library Panel:** Provides keyword and semantic searching directly within Svelte to reference CPIC and PubMed guidelines.

### 5. 9 Specialty Consultation Modes
* **Specialized System Prompts:** Tailors AI behavior to 9 specific health contexts:
  * **🧬 General:** Broad genomic overview and prioritization guide.
  * **💊 Pharmacogenomics (PGx):** Strict focus on drug metabolism (CYP450, DPYD) and safety warnings.
  * **🍎 Nutrients & Methylation:** One-carbon cycle dynamics (MTHFR, COMT, PEMT) and dietary recommendations.
  * **🏃 Metabolic Health & T2D:** Blood sugar, insulin sensitivity (APOE, FTO), and lifestyle variables.
  * **🌙 Sleep & Circadian:** Sleep duration and timing optimization based on CLOCK/PER2 predispositions.
  * **🧠 Brain & Mood:** Dopamine/serotonin synthesis and stress responses (COMT, DRD2).
  * **🦴 Joints & Connective Tissue:** Collagen structure and recovery protocols (COL1A1, COL5A1).
  * **🛡️ Thyroid & Autoimmune:** Thyroid hormone conversion (DIO1, DIO2) and immune cofactor links.
  * **❤️ Cardiovascular Health:** Vascular integrity, blood pressure regulation, and cardiovascular habit guides.

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

### 5. Dynamic MCP Integration Hub
* Implements a stdin/stdout Model Context Protocol (MCP) server directly.
* **Vite Environment Sync:** Automatically toggles commands between Development mode (using `npm run mcp`) and Production mode (using the compiled app binary).
* **Tauri Executable Auto-Detection:** Dynamically queries the exact path of the running executable on the user's filesystem.
* **Active Tool Catalog:** Exposes a list of all active MCP tools and parameters in Svelte using live schemas queried from the Rust backend.

---

## 🛠️ Integration Guide

### 1. Standalone Desktop GUI
To run the Svelte dev server and the Tauri desktop window:
```bash
npm install
npm run tauri dev
```

To build the static static production bundle:
```bash
npm run build
```

---

### 2. Headless MCP Server Integration
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

## 🐚 Commands & Arguments

### NPM Build and Execution Scripts
* **`npm run dev`**: Spawns Vite development web server.
* **`npm run tauri dev`**: Starts Vite server and mounts the Tauri desktop window.
* **`npm run build`**: Compiles static production web assets into `/build`.
* **`npm run check`**: Runs Svelte compiler and TypeScript diagnostics.
* **`npm run mcp`**: Spawns the Tauri dev process in headless MCP server mode.

### Application Executable Flags
* **`--mcp`**: Launches the stdin/stdout JSON-RPC 2.0 Model Context Protocol loop.

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
│    │    ├── marker-packs/  # Modular category JSON databases and manifest.json
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
