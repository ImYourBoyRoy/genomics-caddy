# Connected AI, vectors, and evidence

Optional. The desktop report works without Ollama or a vector database.
When you do connect a model, treat the session as DNA-bearing: selected
findings include raw genotype calls, and a remote Ollama URL is disclosed
in the UI.

## Connections

Configure hosts in **Advanced → Connections** (also **Open Connections** in
the sidebar). Values saved in SQLite override `.env`. Copy
`.env.example` to `.env` only as bootstrap; never commit `.env`.

Typical keys:

```env
QDRANT_URL=http://127.0.0.1:6333
QDRANT_API_KEY=
QDRANT_COLLECTION=genomics_evidence
NCBI_API_KEY=
OLLAMA_URL=http://127.0.0.1:11434
OLLAMA_TOKEN=
```

Supported vector providers: Qdrant, Pinecone, Chroma, Weaviate. Dense
research sweeps work on all four; named vectors remain Qdrant-only.
**Reset to localhost** points at `127.0.0.1` and warns if local Ollama or
Qdrant is missing.

Smoke test (optional services):

```bash
pnpm run smoke
```

## Connected Chat

Chat history is isolated per DNA profile. Session context mode (which
findings go to the model) is saved with that session and restored when you
reopen it.

The model is instructed not to invent genes, variants, or conditions that
are not in the loaded genomic JSON. Default generation uses `temperature 0`.
Pack filters and “active findings only” shrink the payload; they do not
strip genotype calls from the findings that are sent.

Specialty modes (General, PGx, nutrients, metabolic, sleep, brain/mood,
connective tissue, thyroid, cardiovascular, hormone/reproductive) change
prompt emphasis. They do not upgrade evidence or diagnose.

Optional secondary-model review can flag overclaiming, dosing, or diagnosis
language. That log can travel with clinician handoffs.

## Evidence Workbench

Under the AI Evidence Library tab:

- Hybrid search: vector discovery plus SQLite `association_facts`
- Quality dashboard and payload backfill (no re-embed when text is unchanged)
- Candidate markers from enrichment; promote only with association evidence
- Evidence packet export (facts, sources, similar hits, prohibited claims)

MCP tools for this surface are listed in [../mcp/README.md](../mcp/README.md).

## Remote Ollama troubleshooting

**Connection refused.** The remote daemon is likely bound to localhost.
Set `OLLAMA_HOST=0.0.0.0:11434` on that machine and restart Ollama.

**HTTP 500 / V cache quantization.** If the remote GPU cannot use Flash
Attention, unset `OLLAMA_KV_CACHE_TYPE` or set it to `f16`, then restart
the Ollama service.
