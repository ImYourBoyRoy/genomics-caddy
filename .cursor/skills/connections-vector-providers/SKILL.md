---
name: connections-vector-providers
description: >-
  Genomics Caddy Advanced → Connections and multi-provider vector DB conventions
  (Qdrant, Pinecone, Chroma, Weaviate), Ollama model install/update-all, and
  research dense-path wiring. Use when editing ConnectionsPanel, vector_store,
  qdrant_config save/load, Ollama pull progress, or research sweep/search
  provider dispatch.
---

# Connections & vector providers

Project skill for Genomics Caddy (`DNA_Tools`). Read this before changing Connections UX or vector backends.

## File map

| Area | Path |
|------|------|
| Connections UI | `src/lib/components/settings/ConnectionsPanel.svelte` |
| Connections CSS | `src/lib/styles/components/connections-panel.css` |
| TS config types | `src/lib/types/research.ts` (`vector_provider`, `namespace`) |
| API wrappers | `src/lib/api/tauri.ts` (`probeVectorProvider`, pull/delete) |
| Persist / load | `src-tauri/src/config.rs` (`load_qdrant_config*`, `save_qdrant_config`) |
| SQLite migrate | `src-tauri/src/db.rs` (`vector_provider`, `namespace` ALTERs) |
| Facade | `src-tauri/src/research/vector_store/` |
| Qdrant REST | `src-tauri/src/research/qdrant.rs` |
| Sweep / search / browse / packs | `research/sweep.rs`, `commands.rs`, `pack_draft.rs`, `evidence/atlas.rs` |
| Ollama ops | `src-tauri/src/service_ops.rs` (`pull_ollama_model`, `probe_vector_provider`) |

## Capability matrix (honest UI)

| Provider | Dense research | Browse / atlas | Named vectors | Payload indexes | Notes |
|----------|----------------|----------------|---------------|-----------------|-------|
| `qdrant` | yes | yes | yes | yes | Default. LAN/self-host/cloud OK |
| `pinecone` | yes | yes* | no | no | URL = **index host**; API key required; `namespace`; *list API = serverless |
| `chroma` | yes | yes | no | no | Collection name required |
| `weaviate` | yes | yes | no | no | Class name ≈ collection |

Do **not** claim full Qdrant feature parity for other providers. Recommend / category-only scroll without a query / named vectors remain Qdrant-only.

## Save / config rules

1. **SQLite wins over `.env`** for URLs — `.env` only fills blanks.
2. On `save_qdrant_config`: if `vector_provider` / `namespace` are `None`, **preserve** existing DB values (do not reset to `qdrant` / `""`).
3. Allowed providers: `qdrant`, `pinecone`, `chroma`, `weaviate` (no Milvus stub in UI).
4. Secrets stay in keyring; never plaintext in SQLite.
5. Embeddings always via **Ollama** (`embedding_model`) — independent of vector provider.
6. Named vectors: only when provider is Qdrant **and** `named_vectors_enabled`.

## Research wiring

- All dense upsert / search / classify / ensure / purge / browse / atlas sample → `vector_store::*`.
- Sweep preflight uses `vector_store::test_connection` (not Qdrant-only messaging).
- Named upsert path only when `provider.supports_named_vectors()`.
- Pack drafts: export under `App/Data/exports/pack_drafts/`; merge into runtime `App/Data/marker-packs/` via `merge_pack_draft_into_pack` (not repo `src/lib/marker-packs`).
- Extending a provider: add adapter under `vector_store/`, dispatch in `mod.rs`, update Connections `PROVIDER_META` + capability chips.

## Connections UX conventions

- Sections: **1 · Inference (Ollama)** then **2 · Vector store**.
- Show capability chips (dense / named / payload indexes) from provider meta.
- Hosting copy must mention LAN/home-server hosts (not only localhost/cloud).
- Ollama model source: models live on the **Ollama host**, not the app install.
- Links: `https://ollama.com/library`, `https://ollama.com/search` via `@tauri-apps/plugin-opener`.
- **Update all**: sequential `pullOllamaModel`; progress bar + listen `ollama:pull_progress` (payload includes `model`).
- Suggested tags are soft hints only — install by exact registry tag.
- Styles: external `connections-panel.css` (tokens/classes), not large scoped `<style>` blocks.
- After Svelte edits: `svelte-autofixer` + `npm run check`.

## Verify

Always run clippy (project treats warnings as errors) before claiming Rust work done:

```bash
npm run cargo:clippy
cargo check --manifest-path src-tauri/Cargo.toml
npm run check
```

Optional: `npm run verify:css` if Vite CSS load is in play.

## Anti-patterns

- Hardcoding a single Qdrant URL or requiring Qdrant for dense sweeps.
- Saving Connections without `vector_provider` / wiping it from other settings forms.
- Probe-only stubs labeled as “full support”.
- Hiding pull progress or omitting model name on Update all.
- Committing `.env` / API keys.

## More detail

- Adapter notes and Pinecone headers: [reference.md](reference.md)
