# Documentation index

Start at the root [README.md](../README.md) for what Genomics Caddy is and how
to launch it. Use this folder for feature and operator detail. These pages are
written for both people and coding agents: prefer the matching file instead of
scanning the whole repository.

| Question | Read |
| --- | --- |
| How do I import DNA and read a report? | [usage/README.md](usage/README.md) |
| How do I connect Claude, Cursor, or another agent? | [mcp/README.md](mcp/README.md) |
| How do I wire Ollama, vectors, or Connected Chat? | [ai-chat/README.md](ai-chat/README.md) |
| How do I build a portable desktop binary or ship a signed GitHub update? | [build/README.md](build/README.md) |
| How do I compile extra packages (deb/rpm) locally? | [../compile_instructions.md](../compile_instructions.md) |
| How do I run tests and avoid committing private data? | [development/README.md](development/README.md) |
| What does each tracked file do? | [../ARCHITECTURE.md](../ARCHITECTURE.md) |
| How do I run the headless sweep worker? | [../docker/README.md](../docker/README.md) |

Repo-local agent rules live in [AGENTS.md](../AGENTS.md). Session notes belong
in local `MEMORY.md` (gitignored) and are not a user manual.

Marker-pack sources are `src/lib/marker-packs/` with a runtime mirror at
`src-tauri/App/Data/marker-packs/`. Do not remove curated packs.
