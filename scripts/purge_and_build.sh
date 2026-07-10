#!/usr/bin/env bash
# ./scripts/purge_and_build.sh
# Purges frontend/Rust *build caches* only, then compiles a production Tauri release.
# Does NOT touch genome data, offline downloads, chat history, or SQLite progress.
#
# Usage (from repo root):
#   bash ./scripts/purge_and_build.sh
#   bash ./scripts/purge_and_build.sh --purge-only
#   bash ./scripts/purge_and_build.sh --skip-purge
#   bash ./scripts/purge_and_build.sh --skip-checks
#   bash ./scripts/purge_and_build.sh --dry-run

set -euo pipefail

PURGE_ONLY=0
SKIP_PURGE=0
SKIP_CHECKS=0
DRY_RUN=0

for arg in "$@"; do
  case "$arg" in
    --purge-only) PURGE_ONLY=1 ;;
    --skip-purge) SKIP_PURGE=1 ;;
    --skip-checks) SKIP_CHECKS=1 ;;
    --dry-run) DRY_RUN=1 ;;
    -h|--help)
      echo "Usage: bash ./scripts/purge_and_build.sh [--purge-only] [--skip-purge] [--skip-checks] [--dry-run]"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 2
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

step() { printf '\n==> %s\n' "$1"; }
ok() { printf '    %s\n' "$1"; }
skip() { printf '    (skip) %s\n' "$1"; }

assert_repo_root() {
  [[ -f "$REPO_ROOT/package.json" ]] || { echo "package.json not found — run from repo root." >&2; exit 1; }
  [[ -f "$REPO_ROOT/src-tauri/Cargo.toml" ]] || { echo "src-tauri/Cargo.toml not found." >&2; exit 1; }
}

is_protected_path() {
  local rel="${1//\\//}"
  rel="${rel#/}"
  rel="$(printf '%s' "$rel" | tr '[:upper:]' '[:lower:]')"

  case "$rel" in
    data|data/*|app/data|app/data/*|.git|.git/*|src|src/*|scripts|scripts/*|static|static/*)
      return 0 ;;
    node_modules/.vite|node_modules/.cache|node_modules/.vitest)
      return 1 ;;
    node_modules|node_modules/*)
      return 0 ;;
    .env|package.json|package-lock.json|readme.md|memory.md)
      return 0 ;;
  esac

  local leaf="${rel##*/}"
  case "$leaf" in
    user_genome.db*|*.sqlite*)
      return 0 ;;
  esac

  return 1
}

remove_build_artifact() {
  local rel="$1"
  if is_protected_path "$rel"; then
    skip "protected: $rel"
    return
  fi

  local full="$REPO_ROOT/$rel"
  if [[ ! -e "$full" ]]; then
    skip "missing: $rel"
    return
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '    [dry-run] would remove: %s\n' "$rel"
    return
  fi

  rm -rf "$full"
  ok "removed $rel"
}

run_cmd() {
  local label="$1"
  shift
  step "$label"
  printf '    %s\n' "$*"
  "$@"
}

assert_repo_root

printf 'Genomics Caddy — purge build caches + production release\n'
printf 'Repo: %s\n\n' "$REPO_ROOT"
printf 'SAFE: App/Data, data/, user_genome.db, raw_downloads/, references/, chat history, .env\n'
printf 'PURGE: build/, .svelte-kit/, Cargo target/, frontend tool caches only\n'

if [[ "$SKIP_PURGE" -eq 0 ]]; then
  step "Purging frontend build artifacts"
  for rel in build .svelte-kit package node_modules/.vite node_modules/.cache node_modules/.vitest src-tauri/gen/schemas; do
    remove_build_artifact "$rel"
  done

  step "Purging Rust/Cargo build cache"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '    [dry-run] would run: cargo clean --manifest-path src-tauri/Cargo.toml\n'
  else
    run_cmd "cargo clean" cargo clean --manifest-path src-tauri/Cargo.toml
  fi
else
  skip "Purge skipped (--skip-purge)"
fi

if [[ "$PURGE_ONLY" -eq 1 ]]; then
  printf '\nPurge complete (--purge-only).\n'
  exit 0
fi

command -v npm >/dev/null 2>&1 || { echo "npm not found on PATH" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "cargo not found on PATH — install Rust: https://rustup.rs" >&2; exit 1; }

if [[ "$(uname -s)" == "Linux" ]]; then
  if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    echo "Tauri Linux dev libraries missing. Run: bash ./scripts/setup_linux_deps.sh" >&2
    exit 1
  fi
fi

if [[ "$SKIP_CHECKS" -eq 0 ]]; then
  run_cmd "Frontend type check" npm run check
  run_cmd "Rust library check" cargo check --manifest-path src-tauri/Cargo.toml --lib
else
  skip "Pre-build checks skipped (--skip-checks)"
fi

# Use every logical CPU for the release compile (Cargo default is already parallel;
# pin explicitly so benchmarks are comparable across machines).
if command -v nproc >/dev/null 2>&1; then
  export CARGO_BUILD_JOBS
  CARGO_BUILD_JOBS="$(nproc)"
elif command -v sysctl >/dev/null 2>&1; then
  export CARGO_BUILD_JOBS
  CARGO_BUILD_JOBS="$(sysctl -n hw.ncpu 2>/dev/null || echo 4)"
fi
ok "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-default}"

run_cmd "Production Tauri build (no installer bundle)" npm run tauri build

step "Stage portable App/ folder"
APP_DIR="$REPO_ROOT/App"
RELEASE_DIR="$REPO_ROOT/src-tauri/target/release"
[[ -d "$RELEASE_DIR" ]] || { echo "Release directory not found: $RELEASE_DIR" >&2; exit 1; }

mkdir -p "$APP_DIR/Data"

stage_file() {
  local src="$1"
  [[ -e "$src" ]] || return 0
  cp -a "$src" "$APP_DIR/"
  ok "staged $(basename "$src")"
}

# Primary binary name from Cargo.toml default-run
for bin in DNA-Tools "Genomics Caddy"; do
  if [[ -f "$RELEASE_DIR/$bin" ]]; then
    stage_file "$RELEASE_DIR/$bin"
  fi
done

# Copy any bundled resources directory
if [[ -d "$RELEASE_DIR/resources" ]]; then
  rm -rf "$APP_DIR/resources"
  cp -a "$RELEASE_DIR/resources" "$APP_DIR/resources"
  ok "staged resources/"
fi

step "Release artifacts"
ok "$APP_DIR/DNA-Tools"
ok "$APP_DIR/Data"
ok "Build cache source (safe to delete): $RELEASE_DIR"

printf '\nBuild complete.\n'
printf 'Run the staged app from: %s/DNA-Tools\n' "$APP_DIR"
printf 'Persistent data belongs in: %s/Data\n' "$APP_DIR"
printf 'Your genome data and offline downloads were not modified.\n'
