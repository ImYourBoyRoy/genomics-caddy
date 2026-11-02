# Connected AI, vectors, and evidence

Optional. The desktop report works without Ollama, ChatGPT, or a vector
database. When you do connect a model, treat the session as DNA-bearing:
selected findings include raw genotype calls. The UI discloses the
destination for a remote Ollama URL and for the ChatGPT plan provider
(`api.openai.com`).

## Connections

Configure hosts in **Advanced → Connections** (also **Open Connections** in
the sidebar). Values saved in SQLite override `.env`. Copy
`.env.example` to `.env` only as bootstrap; never commit `.env`.

Typical keys (Ollama / vectors only — ChatGPT needs none of these):

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

## Ask AI providers

In **Ask AI → Connection & Model**, choose:

| Provider | What it uses | Secrets |
| --- | --- | --- |
| Local / Ollama | Your Ollama URL (local or remote) | Optional bearer token in Settings / `.env` |
| ChatGPT plan | Your ChatGPT account via browser sign-in | **No API key, no client secret, no env var** |

Embeddings and vector research still use Ollama even when ChatGPT answers
the consultation chat.

### Continue with ChatGPT

1. Select **ChatGPT plan**.
2. Click **Continue with ChatGPT**. Your system browser opens OpenAI’s
   authorize page (OAuth 2.0 authorization code + PKCE + OIDC).
3. Approve plan usage. The app listens on a `127.0.0.1` loopback callback
   (preferred port `1455`, or an ephemeral free port).
4. Access / refresh / ID tokens are stored only in the OS credential
   manager (`GenomicsCaddy` / `chatgpt_credentials`). The webview never
   receives a token — only a connection summary (email/name, sharing
   flag, expiry).
5. Pick a listed model and chat. Requests use OpenAI’s Responses API with
   `store=false` and `stream=true`. Raw genotype calls in the selected
   context leave this device for that request; OpenAI’s own terms still
   apply. ChatGPT conversation history is not accessed.

**Manage usage** opens ChatGPT Settings → Usage. Disconnect revokes the
refresh token when possible and clears the local credential.

There is nothing to put in GitHub Actions secrets for ChatGPT. Release
signing still uses only `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## Connected Chat

Chat history is isolated per DNA profile. Session context mode (which
findings go to the model) is saved with that session and restored when you
reopen it. The selected provider (Ollama vs ChatGPT) is remembered in
local storage for the next Ask AI session.

The model is instructed not to invent genes, variants, or conditions that
are not in the loaded genomic JSON. Default generation uses `temperature 0`
on Ollama; the ChatGPT plan route rejects temperature / max-output knobs
and uses the hosted model defaults.

Pack filters and “active findings only” shrink the payload; they do not
strip genotype calls from the findings that are sent.

Specialty modes (General, PGx, nutrients, metabolic, sleep, brain/mood,
connective tissue, thyroid, cardiovascular, hormone/reproductive) change
prompt emphasis. They do not upgrade evidence or diagnose.

Optional secondary-model review can flag overclaiming, dosing, or diagnosis
language. That log can travel with clinician handoffs. When ChatGPT is
selected, the review model also uses the ChatGPT plan path.

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

## ChatGPT troubleshooting

**Sign-in window waits, then cancels.** Finish in the browser within five
minutes, or click **Cancel sign-in** and try again. Another app may already
be bound to port `1455`; Genomics Caddy falls back to a free port when that
happens.

**“Plan usage is not enabled.”** Re-run Continue with ChatGPT and approve
the `chatgpt.tokens.use.direct` scope, or open **Manage usage**.

**Usage limit.** Open ChatGPT Settings → Usage (or the in-app Manage usage
control) and retry after the plan window resets.

**Switching accounts.** Disconnect first, then sign in again. The app
rejects a different ChatGPT subject than the saved connection.

**`string_above_max_length` / instructions too long.** OpenAI caps the
Responses API `instructions` field at 1,048,576 characters. Genomics Caddy
keeps the short rule text in `instructions` and moves `[JSON CONTEXT]` into
the chat `input`. It also emits compact JSON for ChatGPT, refuses to send
Developer raw JSON’s full `raw_report`, and trims heavy support/catalog
blobs or finding counts until the context fits. Prefer **Active findings**
over **Developer raw JSON** / **Full selected packs** for ChatGPT.
