#!/usr/bin/env bash
# ./scripts/benchmark_build.sh
# Time Genomics Caddy build-cache purge and/or full production rebuild on Linux/macOS.
# Does NOT touch App/Data, genome DBs, downloads, or .env.
#
# Usage (from repo root):
#   bash ./scripts/benchmark_build.sh                 # purge + full rebuild (default)
#   bash ./scripts/benchmark_build.sh --purge-only     # time purge only
#   bash ./scripts/benchmark_build.sh --rebuild        # same as default
#   bash ./scripts/benchmark_build.sh --skip-checks    # faster rebuild (skip npm/cargo check)
#   bash ./scripts/benchmark_build.sh --json           # also write JSON beside the text report

set -euo pipefail

MODE="rebuild"   # purge | rebuild
SKIP_CHECKS=0
WRITE_JSON=0

for arg in "$@"; do
  case "$arg" in
    --purge-only) MODE="purge" ;;
    --rebuild|--full) MODE="rebuild" ;;
    --skip-checks) SKIP_CHECKS=1 ;;
    --json) WRITE_JSON=1 ;;
    -h|--help)
      sed -n '2,14p' "$0"
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

[[ -f package.json && -f src-tauri/Cargo.toml ]] || {
  echo "Run from Genomics Caddy repo root." >&2
  exit 1
}

REPORT_DIR="$REPO_ROOT/App/Data/benchmarks"
mkdir -p "$REPORT_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
HOST="$(hostname -s 2>/dev/null || hostname)"
OS_NAME="$(uname -s)"
ARCH="$(uname -m)"
KERNEL="$(uname -r)"
REPORT_TXT="$REPORT_DIR/build_bench_${HOST}_${STAMP}.txt"
REPORT_JSON="$REPORT_DIR/build_bench_${HOST}_${STAMP}.json"

cpu_model() {
  if [[ -r /proc/cpuinfo ]]; then
    awk -F': ' '/model name/ {print $2; exit}' /proc/cpuinfo
  elif command -v sysctl >/dev/null 2>&1; then
    sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "unknown"
  else
    echo "unknown"
  fi
}

cpu_cores() {
  if command -v nproc >/dev/null 2>&1; then
    nproc
  elif command -v sysctl >/dev/null 2>&1; then
    sysctl -n hw.ncpu 2>/dev/null || echo "?"
  else
    echo "?"
  fi
}

mem_total() {
  if command -v free >/dev/null 2>&1; then
    free -h | awk '/^Mem:/ {print $2}'
  elif command -v sysctl >/dev/null 2>&1; then
    # bytes → GiB-ish
    local b
    b="$(sysctl -n hw.memsize 2>/dev/null || echo 0)"
    awk -v b="$b" 'BEGIN { printf "%.1fGi\n", b/1024/1024/1024 }'
  else
    echo "?"
  fi
}

now_ns() {
  date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))'
}

fmt_secs() {
  local ns="$1"
  python3 - "$ns" <<'PY'
import sys
ns = int(sys.argv[1])
s = ns / 1e9
m, rem = divmod(s, 60)
h, m = divmod(m, 60)
if h >= 1:
    print(f"{int(h)}h {int(m)}m {rem:05.2f}s ({s:.3f}s)")
elif m >= 1:
    print(f"{int(m)}m {rem:05.2f}s ({s:.3f}s)")
else:
    print(f"{s:.3f}s")
PY
}

CPU="$(cpu_model)"
CORES="$(cpu_cores)"
MEM="$(mem_total)"
NODE_V="$(node -v 2>/dev/null || echo missing)"
NPM_V="$(npm -v 2>/dev/null || echo missing)"
RUSTC_V="$(rustc --version 2>/dev/null || echo missing)"
CARGO_V="$(cargo --version 2>/dev/null || echo missing)"

echo "Genomics Caddy — build benchmark ($MODE)"
echo "Host: $HOST  OS: $OS_NAME/$ARCH  Kernel: $KERNEL"
echo "CPU: $CPU ($CORES cores)  RAM: $MEM"
echo "Node: $NODE_V  npm: $NPM_V"
echo "Rust: $RUSTC_V / $CARGO_V"
echo "Report: $REPORT_TXT"
echo

EXTRA_ARGS=()
if [[ "$SKIP_CHECKS" -eq 1 ]]; then
  EXTRA_ARGS+=(--skip-checks)
fi

START_NS="$(now_ns)"
START_ISO="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

if [[ "$MODE" == "purge" ]]; then
  bash "$SCRIPT_DIR/purge_and_build.sh" --purge-only
  PHASE="purge_only"
else
  bash "$SCRIPT_DIR/purge_and_build.sh" "${EXTRA_ARGS[@]}"
  PHASE="purge_and_rebuild"
fi

END_NS="$(now_ns)"
END_ISO="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ELAPSED_NS=$((END_NS - START_NS))
ELAPSED_HUMAN="$(fmt_secs "$ELAPSED_NS")"
ELAPSED_SEC="$(python3 -c "print(f'{int('$ELAPSED_NS')/1e9:.3f}')")"

BINARY=""
BINARY_BYTES=0
if [[ -f "$REPO_ROOT/App/DNA-Tools" ]]; then
  BINARY="$REPO_ROOT/App/DNA-Tools"
  BINARY_BYTES="$(stat -c%s "$BINARY" 2>/dev/null || stat -f%z "$BINARY")"
fi

{
  echo "Genomics Caddy build benchmark"
  echo "phase=$PHASE"
  echo "started_utc=$START_ISO"
  echo "ended_utc=$END_ISO"
  echo "elapsed=$ELAPSED_HUMAN"
  echo "elapsed_seconds=$ELAPSED_SEC"
  echo "host=$HOST"
  echo "os=$OS_NAME"
  echo "arch=$ARCH"
  echo "kernel=$KERNEL"
  echo "cpu=$CPU"
  echo "cores=$CORES"
  echo "ram=$MEM"
  echo "node=$NODE_V"
  echo "npm=$NPM_V"
  echo "rustc=$RUSTC_V"
  echo "cargo=$CARGO_V"
  echo "skip_checks=$SKIP_CHECKS"
  echo "binary=$BINARY"
  echo "binary_bytes=$BINARY_BYTES"
  echo "repo=$REPO_ROOT"
} | tee "$REPORT_TXT"

if [[ "$WRITE_JSON" -eq 1 ]]; then
  python3 - "$REPORT_JSON" <<PY
import json, pathlib
path = pathlib.Path("$REPORT_JSON")
path.write_text(json.dumps({
  "phase": "$PHASE",
  "started_utc": "$START_ISO",
  "ended_utc": "$END_ISO",
  "elapsed_seconds": float("$ELAPSED_SEC"),
  "elapsed_human": "$ELAPSED_HUMAN",
  "host": "$HOST",
  "os": "$OS_NAME",
  "arch": "$ARCH",
  "kernel": "$KERNEL",
  "cpu": "$CPU",
  "cores": "$CORES",
  "ram": "$MEM",
  "node": "$NODE_V",
  "npm": "$NPM_V",
  "rustc": "$RUSTC_V",
  "cargo": "$CARGO_V",
  "skip_checks": bool($SKIP_CHECKS),
  "binary": "$BINARY",
  "binary_bytes": int("$BINARY_BYTES"),
  "repo": "$REPO_ROOT",
}, indent=2) + "\n")
print(f"JSON: {path}")
PY
fi

echo
echo "DONE — $PHASE in $ELAPSED_HUMAN"
echo "Compare reports under: $REPORT_DIR"
