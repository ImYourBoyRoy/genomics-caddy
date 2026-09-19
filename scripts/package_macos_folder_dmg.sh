#!/usr/bin/env bash
# ./scripts/package_macos_folder_dmg.sh
# Purpose: Rebuild the macOS DMG as a folder containing Genomics Caddy.app
#          plus an empty Data/ directory (USB-clean layout).
# How to run: bash ./scripts/package_macos_folder_dmg.sh
# Optional args: APP_PATH [DMG_OUT_PATH]
# Inputs: the signed/bundled .app from cargo tauri build.
# Outputs: overwrites the product DMG (not rw.*.dmg scratch files).
# Notes: macOS-only (hdiutil). Safe to skip on Linux/Windows CI jobs.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_PATH="${1:-}"
DMG_OUT="${2:-}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "package_macos_folder_dmg: skipping on $(uname -s) (needs hdiutil)"
  exit 0
fi

if [[ -z "$APP_PATH" ]]; then
  for candidate in \
    "$ROOT/src-tauri/target/universal-apple-darwin/release/bundle/macos/Genomics Caddy.app" \
    "$ROOT/src-tauri/target/release/bundle/macos/Genomics Caddy.app" \
    "$ROOT/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Genomics Caddy.app"
  do
    if [[ -d "$candidate" ]]; then
      APP_PATH="$candidate"
      break
    fi
  done
fi

if [[ -z "$APP_PATH" || ! -d "$APP_PATH" ]]; then
  echo "package_macos_folder_dmg: Genomics Caddy.app not found" >&2
  exit 1
fi

if [[ -z "$DMG_OUT" ]]; then
  for dir in \
    "$ROOT/src-tauri/target/universal-apple-darwin/release/bundle/dmg" \
    "$ROOT/src-tauri/target/release/bundle/dmg" \
    "$ROOT/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg"
  do
    if [[ -d "$dir" ]]; then
      found="$(find "$dir" -maxdepth 1 -type f -name '*.dmg' ! -name 'rw.*' | head -1 || true)"
      if [[ -n "$found" ]]; then
        DMG_OUT="$found"
        break
      fi
      DMG_OUT="$dir/Genomics Caddy.dmg"
      break
    fi
  done
fi

if [[ -z "$DMG_OUT" ]]; then
  DMG_OUT="$ROOT/builds/macOS/Genomics Caddy.dmg"
fi

mkdir -p "$(dirname "$DMG_OUT")"
STAGING="$(mktemp -d /tmp/genomics-caddy-dmg.XXXXXX)"
cleanup() { rm -rf "$STAGING"; }
trap cleanup EXIT

cp -R "$APP_PATH" "$STAGING/Genomics Caddy.app"
mkdir -p "$STAGING/Data"
touch "$STAGING/Data/.keep"

rm -f "$DMG_OUT"
hdiutil create \
  -volname "Genomics Caddy" \
  -srcfolder "$STAGING" \
  -ov \
  -format UDZO \
  "$DMG_OUT"

echo "Wrote folder DMG $DMG_OUT"
bash "$ROOT/scripts/sign_updater_artifact.sh" "$DMG_OUT"
