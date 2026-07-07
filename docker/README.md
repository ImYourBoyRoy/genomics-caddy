# ./docker/README.md
# Docker deployment

Genomics Caddy runs as a **portable Tauri desktop app** locally and as a **headless sweep worker** in Docker on high-core servers.

## Worker (recommended for 80-core servers)

The worker runs `genomics-caddy --headless-sweep` with **dynamic pipeline tuning** from `GENOMICS_CPU_LIMIT` and measured Ollama/Qdrant latency.

```bash
docker compose -f docker/docker-compose.yml build genomics-worker
docker compose -f docker/docker-compose.yml up genomics-worker
```

Mount persistent data at `/data` (SQLite, references, offline tiers). Copy your existing `App/Data` tree into the volume or set `GENOMICS_DATA_DIR` to a bind mount.

### Environment

| Variable | Purpose |
|----------|---------|
| `GENOMICS_DATA_DIR` | Data root (default `/data` in container) |
| `GENOMICS_SAMPLE_ID` | Sample id for sweep (default `1`) |
| `GENOMICS_CPU_LIMIT` | Host logical core count (optional). Tuning uses **one fewer** (e.g. 80 → 79, 4 → 3). |
| `GENOMICS_OLLAMA_URL` / `OLLAMA_URL` | Embedding server |
| `QDRANT_URL` | Used when saving config; primary Qdrant settings live in SQLite |
| `GENOMICS_TUNING` | `dynamic` (default) or `fixed` for manual env overrides |

### Progress output

Stdout lines:

- `GENOMICS_PROGRESS {json}` — sweep progress events
- `GENOMICS_FINDING {json}` — live finding previews

```bash
docker compose -f docker/docker-compose.yml logs -f genomics-worker | grep GENOMICS_PROGRESS
```

### Resume / force re-enrich

```bash
docker compose -f docker/docker-compose.yml run --rm genomics-worker \
  --headless-sweep --sample-id=1 --resume
```

## Static UI (optional)

The Svelte UI is a **static bundle** in Docker. Tauri `invoke` APIs are not available without the desktop binary — use the UI container for reference or pair it with the desktop app against the same data volume.

```bash
docker compose -f docker/docker-compose.yml --profile ui up -d genomics-ui
# http://localhost:8080
```

## Updating

```bash
docker compose -f docker/docker-compose.yml build --pull
docker compose -f docker/docker-compose.yml up -d --force-recreate genomics-worker
```

Data persists in the `genomics-data` volume across image updates.
