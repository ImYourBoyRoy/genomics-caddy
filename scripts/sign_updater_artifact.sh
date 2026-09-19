#!/usr/bin/env bash
# ./scripts/sign_updater_artifact.sh
# Purpose: Sign one updater artifact with the Tauri minisign key.
# How to run: bash ./scripts/sign_updater_artifact.sh /path/to/file
# Inputs: FILE path; TAURI_SIGNING_PRIVATE_KEY or TAURI_SIGNING_PRIVATE_KEY_PATH,
#         plus optional TAURI_SIGNING_PRIVATE_KEY_PASSWORD. Falls back to
#         ~/.config/genomics-caddy/tauri-updater.key without printing it.
# Outputs: writes FILE.sig next to FILE. Never echoes the private key.
# Notes: used after custom folder-layout DMG rebuilds so latest.json matches.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FILE="${1:-}"

if [[ -z "$FILE" || ! -f "$FILE" ]]; then
  echo "sign_updater_artifact: missing file" >&2
  exit 1
fi

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -z "${TAURI_SIGNING_PRIVATE_KEY_PATH:-}" ]]; then
  DEFAULT_KEY="${HOME}/.config/genomics-caddy/tauri-updater.key"
  if [[ -f "$DEFAULT_KEY" ]]; then
    export TAURI_SIGNING_PRIVATE_KEY_PATH="$DEFAULT_KEY"
  fi
fi

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" ]]; then
  DEFAULT_PASSWORD="${HOME}/.config/genomics-caddy/tauri-updater.key.password"
  if [[ -f "$DEFAULT_PASSWORD" ]]; then
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(tr -d '\n\r' < "$DEFAULT_PASSWORD")"
    export TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  fi
fi

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -z "${TAURI_SIGNING_PRIVATE_KEY_PATH:-}" ]]; then
  echo "sign_updater_artifact: no updater private key in env or ~/.config/genomics-caddy/" >&2
  exit 1
fi

CLI="$ROOT/node_modules/@tauri-apps/cli/tauri.js"
if [[ ! -f "$CLI" ]]; then
  echo "sign_updater_artifact: Tauri CLI not found at $CLI" >&2
  exit 1
fi

echo "sign_updater_artifact: signing $(basename "$FILE")"
node "$CLI" signer sign "$FILE"
if [[ ! -f "${FILE}.sig" ]]; then
  echo "sign_updater_artifact: expected ${FILE}.sig after signer" >&2
  exit 1
fi
echo "sign_updater_artifact: wrote $(basename "$FILE").sig"
