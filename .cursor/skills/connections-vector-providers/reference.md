# Vector store adapter reference

## Dispatch entry points (`vector_store/mod.rs`)

- `provider_from_config` / `VectorProvider::parse`
- `test_connection`, `ensure_collection`, `purge_collection`
- `upsert_dense_batch`, `search_dense`
- `classify_points_sweep_state`, `classify_points_index_state`
- `maybe_ensure_payload_indexes` (Qdrant only)
- `capabilities_json` for probes / UI notes
- `flatten_metadata` / `hit_from_metadata` for cloud metadata limits
- `browse_sample_points` — provider-native pagination (Qdrant scroll; Pinecone list+fetch; Chroma get+offset; Weaviate GraphQL offset)
- `sample_vectors_for_atlas` — dense vectors for UMAP (all providers)

## Pack drafts (`pack_draft.rs`)

- Export → `App/Data/exports/pack_drafts/` (paginated browse ≤100k)
- Merge → **only** `App/Data/marker-packs/research_found.json` (+ `.bak`); curated pack IDs refused
- Markers cleaned to curated `MarkerDefinition` shape (`context_dependent`, no `genotype`)
- Commands: `list_runtime_marker_pack_ids`, `merge_pack_draft_into_pack` (hard-locks target)
- Seed pack: `src/lib/marker-packs/research_found.json` (`default_enabled: false`)

## Pinecone

- Headers: `Api-Key`, `X-Pinecone-Api-Version: 2025-10`, `Content-Type: application/json`
- Data plane base = index host (`https://….svc.…pinecone.io`)
- Upsert: `POST /vectors/upsert` (chunk ~64)
- Query: `POST /query` with `includeMetadata`, optional `filter`, `namespace`
- Fetch: `GET /vectors/fetch?ids=&namespace=`
- Purge: `POST /vectors/delete` `{ deleteAll: true, namespace }`
- Empty namespace → `__default__`
- Metadata: flatten nests to strings (40KB limits apply)
- Browse: `POST /vectors/list` + fetch (serverless indexes only)
- Atlas sample: list + fetch embeddings

## Chroma

- Heartbeat: `/api/v2/heartbeat` then `/api/v1/heartbeat`
- Collections list/create/upsert/query/get on v2 then v1 fallbacks
- Auth: `Authorization: Bearer` and/or `X-Chroma-Token`
- Browse / atlas: collection get with `where` sample_id + offset; include embeddings for atlas

## Weaviate

- Meta: `GET /v1/meta`
- Schema class create: `POST /v1/schema` (`vectorizer: none`)
- Upsert: `POST /v1/batch/objects` with deterministic UUID from u64 id
- Search: GraphQL `nearVector`; full payload often in `payload_json` text property
- Purge: `DELETE /v1/schema/{Class}`
- Browse / atlas: GraphQL offset + `_additional { vector }`

## Qdrant

- Keep using `research/qdrant.rs` for named vectors, recommend, payload indexes, scroll
- `map_payload_to_hit` is `pub(crate)` for facade reuse

## Ollama pull progress

`service_ops::pull_ollama_model` streams NDJSON and emits `ollama:pull_progress` with `model` set when missing from the line payload.
