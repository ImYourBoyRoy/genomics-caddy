# Genomics Caddy

Local-first desktop app for exploring your own consumer DNA export. It runs on
your machine: parse AncestryDNA or 23andMe files, normalize GRCh37/GRCh38
coordinates, score curated marker packs, and optionally talk to a local AI
through chat or MCP.

Created by **Roy Dawson IV**.

## Warning

This is a research and education tool, not a diagnostic or treatment product.
A consumer array call is an observation. It does not establish a disease,
current hormone level, medication response, or care plan. High-stakes findings
need validated clinical testing. AI chat and clinician/AI exports include the
raw genotype calls behind the selected findings; share those files only with
the person or system you intend.

Do not commit raw DNA, reports, `.env`, or `App/` data. The repo gitignores
them. The only committed genotype-shaped files are synthetic fixtures in
`src-tauri/testdata/`.

## What it does

- Import `.txt`, `.csv`, `.tsv`, or ZIP exports and review them before any
  profile is written
- Build a Simple-first report with Clinical and Compare views
- Keep food, supplement, and medication prompts conditional on context you
  enter, not DNA alone
- Optional local Ollama chat and a stdin/stdout MCP server for agents

## Quick start

Needs **Node 26+**, a current **Rust** toolchain (see `src-tauri/Cargo.toml`),
and **pnpm**.

Linux one-time WebKit/GTK headers:

```bash
pnpm run setup:linux
```

Install and open the desktop window:

```bash
node ./scripts/pnpm_unlocked.mjs install
pnpm run tauri:dev
```

Then in the app:

1. Import your AncestryDNA or 23andMe export.
2. Confirm the pre-write review (vendor, row counts, build, warnings).
3. Read **Simple** mode first. Use **Clinical** or **Compare** when you want
   sources and technical detail.
4. Add Context/Diary yourself if you want food, cycle, or medication prompts
   tailored to real life. The app does not infer those from DNA.

GitHub installers and the Windows portable zip notify you when a newer
signed release is published. Confirm in the app: installer copies run the
signed setup; a portable zip copy replaces its own files and leaves `Data/`
in place. Copies from before v0.2.3 still offer the Windows installer once
— download the new zip if you want to stay portable. `tauri:dev` does not
apply GitHub artifacts.

People may use and share this for noncommercial purposes. Companies may not
use it commercially. See [LICENSE](LICENSE)
([PolyForm Noncommercial 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0)).

## Downloads

GitHub [Releases](https://github.com/ImYourBoyRoy/genomics-caddy/releases) ship:

| Platform | Artifact |
| --- | --- |
| Windows | NSIS setup (`Install for me only` or `Install for everyone`; the installer asks for administrator rights) and a portable zip |
| Linux | AppImage |
| macOS | Universal DMG |

Portable copies keep genomes in a `Data/` folder next to the launched file.
`Install for everyone` uses a per-account folder. Change it from the left
**Library** row. To remove personal files from an AppImage, DMG, or zip copy,
use **Erase** on that Library row, or delete the whole folder that contains
the app and its `Data/` directory. Other package formats:
[compile_instructions.md](compile_instructions.md).

Optional production build (portable binary lands in `App/`):

```bash
pnpm run build:release
```

Frontend-only web preview (`pnpm run dev`) cannot import DNA. Use the Tauri
window for a real workflow.

## Docs

| Doc | Use it when |
| --- | --- |
| [docs/README.md](docs/README.md) | You need the full map (humans and agents) |
| [ARCHITECTURE.md](ARCHITECTURE.md) | File-by-file map: size, lines, and role |
| [docs/usage/README.md](docs/usage/README.md) | Import, reports, exports, privacy |
| [docs/mcp/README.md](docs/mcp/README.md) | MCP server, client configs, tools |
| [docs/ai-chat/README.md](docs/ai-chat/README.md) | Ollama, connections, evidence workbench |
| [docs/build/README.md](docs/build/README.md) | Release builds, Linux launcher, Docker |
| [compile_instructions.md](compile_instructions.md) | Extra OS packages and GitHub-shaped local bundles |
| [docs/development/README.md](docs/development/README.md) | Tests, audits, repo layout, `.env` |

## Author

- Roy Dawson IV
- Email: <Roy.Dawson.IV@gmail.com>
- GitHub: [https://github.com/imyourboyroy](https://github.com/imyourboyroy)
- PyPI: [https://pypi.org/user/ImYourBoyRoy/](https://pypi.org/user/ImYourBoyRoy/)
