# MCP server

Genomics Caddy embeds a stdin/stdout [Model Context Protocol](https://modelcontextprotocol.io/)
server so local agents can query imported profiles without uploading DNA to a
third-party host.

- MCP Registry name: `mcp-name: io.github.ImYourBoyRoy/genomics-caddy`

The desktop binary is the MCP server (`DNA-Tools --mcp`). There is no separate
npm/crates MCP package yet; wire clients to the installed or portable binary.

Read-only tools are the default. Mutating tools require `--mcp-write`.
Connected Chat and report exports that include findings still carry raw
genotype calls for those findings; treat MCP query results as sensitive.

## Start the server

Development (from the repo, after `node ./scripts/pnpm_unlocked.mjs install`):

```bash
pnpm run mcp
```

Production (compiled desktop binary):

```bash
/path/to/DNA-Tools --mcp
```

On Windows the binary is `DNA-Tools.exe`. Use the real install or `App/`
path from [../build/README.md](../build/README.md).

### Flags

| Flag | Meaning |
| --- | --- |
| `--mcp` | JSON-RPC 2.0 MCP loop (read-only tools unless write is enabled) |
| `--mcp-write` | Enable mutating tools listed below |
| `--mcp-auth-token=<secret>` | Require `params._meta.authToken` on `tools/call` (`ping` exempt). Also accepted as `GENOMICS_MCP_TOKEN`. |

Omit the auth flag for local unauthenticated MCP.

## Client configs

Replace paths with your clone or installed binary.

### Claude Desktop

`claude_desktop_config.json` (`%APPDATA%\Claude\` on Windows):

```json
{
  "mcpServers": {
    "genomics-caddy-dev": {
      "command": "pnpm",
      "args": ["run", "mcp"],
      "options": {
        "cwd": "/path/to/DNA_Tools"
      }
    }
  }
}
```

Production:

```json
{
  "mcpServers": {
    "genomics-caddy": {
      "command": "/path/to/DNA-Tools",
      "args": ["--mcp"]
    }
  }
}
```

### Cursor

Settings → Features → MCP → add a command server:

- Dev: `pnpm run mcp` with working directory set to this repo
- Prod: `/path/to/DNA-Tools --mcp`

### Claude Code

```bash
claude mcp add genomics-caddy-dev pnpm -- run mcp
claude mcp add genomics-caddy /path/to/DNA-Tools -- --mcp
```

### Cline / Roo

`cline_mcp_settings.json`:

```json
{
  "mcpServers": {
    "genomics-caddy": {
      "command": "/path/to/DNA-Tools",
      "args": ["--mcp"],
      "disabled": false
    }
  }
}
```

## Tools (read-only by default)

| Tool | Arguments | Returns |
| --- | --- | --- |
| `list_samples` | none | Imported profile ids/names |
| `get_variants_by_rsid` | `sample_id`, `rsids[]` | Genotypes for those rsIDs |
| `get_variants_in_region` | `sample_id`, `chromosome`, `start`, `end` | Variants in a GRCh38 region |
| `generate_report` | `sample_id`, `template_json` | Evaluated report for a template |
| `list_packs` | none | Curated pack ids and descriptions |
| `get_report_for_packs` | `sample_id`, optional `pack_ids[]`, optional `only_active_findings` | Evaluated pack report |
| `list_evidence_sources` | none | Citation list from the local evidence library |
| `get_evidence_for_marker` | `rsid` | Notes/references for that marker |
| `search_evidence` | `query`, optional `ollama_url`, `ollama_token` | Keyword/semantic search |
| `get_chat_sessions` | optional `sample_id` | Saved consultation sessions |
| `export_chat_history` | `session_id` | Markdown export |
| `get_app_paths` | none | Data and pack directories |
| `check_chain_status` | none | Whether the GRCh37→GRCh38 chain is present |
| `get_current_exe` | none | Absolute path of this binary |
| `scan_ollama_models` | `url`, optional `token` | Installed Ollama tags |
| `show_ollama_model` | `url`, `name`, optional `token` | Model details |
| `get_active_ollama_models` | `url`, optional `token` | Loaded models / VRAM |
| `reload_report` | `sample_id` | Reload report; `status: ready` when settled |
| `get_offline_update_status` | see live schema | Catalog inventory; no genotypes |
| `get_app_bootstrap` | none | Startup paths/status |
| `get_research_job_status` | see live schema | Research job progress |
| `get_discovered_findings_summary` | see live schema | Aggregate discovery counts |
| `search_vector_associations` | see live schema | Vector + SQLite association search |
| `get_variant_evidence_card` | see live schema | Source-grounded evidence card |
| `explain_vector_match` | see live schema | Why a vector hit was returned |
| `get_similar_associations` | see live schema | Nearby association hits |
| `get_quality_dashboard` | see live schema | Vector/SQLite quality snapshot |
| `get_trait_clusters` | see live schema | Trait cluster view |
| `export_evidence_packet` | see live schema | Facts + sources + prohibited claims |
| `list_candidate_markers` | see live schema | Enrichment candidates |

The in-app MCP panel lists live schemas from the running backend. Prefer that
list if this table and the binary disagree.

`get_variants_by_rsid` and report tools return genotype values. Do not log
them. Offline status/sync metadata payloads are resource counts, not calls.

## Write-gated tools

Require `--mcp-write` (hidden from `tools/list` otherwise):

- `delete_chat_session`
- `update_candidate_marker_status`
- `sync_offline_asset`
- `sync_offline_data`
- `backfill_evidence_payloads`
- `build_vector_atlas`
- `enable_named_vectors_collection`

## Checks

```bash
pnpm run audit:mcp
```

Write-path fixture (temporary synthetic sample only):

```bash
pnpm run audit:mcp:write
```

These audits do not print genotype values.
