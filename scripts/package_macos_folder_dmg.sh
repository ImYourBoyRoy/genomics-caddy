#!/usr/bin/env bash
# ./scripts/package_macos_folder_dmg.sh
# Purpose: Rebuild the macOS DMG as a folder containing Genomics Caddy.app
#          plus an empty Data/ directory (USB-clean layout).
# How to run: bash ./scripts/package_macos_folder_dmg.sh
# Optional args: [--self-test|--print-app-path] [APP_PATH [DMG_OUT_PATH]]
# Inputs: the bundled .app, or Genomics Caddy.app.tar.gz after Tauri cleans
#         the live .app once the DMG exists. MACOS_BUNDLE_ROOT overrides
#         src-tauri/target (used by --self-test).
# Outputs: overwrites the product DMG (not rw.*.dmg scratch files) and signs it.
# Notes: packaging needs hdiutil (Darwin). --self-test/--print-app-path run
#         on any OS so CI can prove the post-clean recovery path.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUNDLE_ROOT="${MACOS_BUNDLE_ROOT:-$ROOT/src-tauri/target}"
PRODUCT_NAME="Genomics Caddy"
APP_NAME="${PRODUCT_NAME}.app"
TARBALL_NAME="${APP_NAME}.tar.gz"

MODE=""
POSITIONAL=()
for arg in "$@"; do
  case "$arg" in
    --self-test) MODE="self-test" ;;
    --print-app-path) MODE="print-app-path" ;;
    *) POSITIONAL+=("$arg") ;;
  esac
done

APP_PATH="${POSITIONAL[0]:-}"
DMG_OUT="${POSITIONAL[1]:-}"
STAGING=""
EXTRACT_DIR=""
ATTACHED_MOUNT=""

cleanup() {
  if [[ -n "${ATTACHED_MOUNT:-}" ]]; then
    hdiutil detach "$ATTACHED_MOUNT" -quiet >/dev/null 2>&1 || true
  fi
  if [[ -n "${STAGING:-}" && -d "$STAGING" ]]; then
    rm -rf "$STAGING"
  fi
  if [[ -n "${EXTRACT_DIR:-}" && -d "$EXTRACT_DIR" ]]; then
    rm -rf "$EXTRACT_DIR"
  fi
}
trap cleanup EXIT

find_live_app() {
  local candidate found
  for candidate in \
    "$BUNDLE_ROOT/universal-apple-darwin/release/bundle/macos/$APP_NAME" \
    "$BUNDLE_ROOT/release/bundle/macos/$APP_NAME" \
    "$BUNDLE_ROOT/aarch64-apple-darwin/release/bundle/macos/$APP_NAME" \
    "$BUNDLE_ROOT/x86_64-apple-darwin/release/bundle/macos/$APP_NAME"
  do
    if [[ -d "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  if [[ -d "$BUNDLE_ROOT" ]]; then
    found="$(find "$BUNDLE_ROOT" -type d -name "$APP_NAME" -print -quit 2>/dev/null || true)"
    if [[ -n "$found" && -d "$found" ]]; then
      printf '%s\n' "$found"
      return 0
    fi
  fi
  return 1
}

find_app_tarball() {
  local candidate found
  for candidate in \
    "$BUNDLE_ROOT/universal-apple-darwin/release/bundle/macos/$TARBALL_NAME" \
    "$BUNDLE_ROOT/release/bundle/macos/$TARBALL_NAME" \
    "$BUNDLE_ROOT/aarch64-apple-darwin/release/bundle/macos/$TARBALL_NAME" \
    "$BUNDLE_ROOT/x86_64-apple-darwin/release/bundle/macos/$TARBALL_NAME"
  do
    if [[ -f "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  if [[ -d "$BUNDLE_ROOT" ]]; then
    found="$(find "$BUNDLE_ROOT" -type f -name "*.app.tar.gz" -print -quit 2>/dev/null || true)"
    if [[ -n "$found" && -f "$found" ]]; then
      printf '%s\n' "$found"
      return 0
    fi
  fi
  return 1
}

extract_app_from_tarball() {
  local tarball="$1"
  EXTRACT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/genomics-caddy-app.XXXXXX")"
  tar -xzf "$tarball" -C "$EXTRACT_DIR"
  local found
  found="$(find "$EXTRACT_DIR" -type d -name "*.app" -print -quit 2>/dev/null || true)"
  if [[ -z "$found" || ! -d "$found" ]]; then
    echo "package_macos_folder_dmg: $tarball did not contain a .app" >&2
    return 1
  fi
  printf '%s\n' "$found"
}

find_product_dmg() {
  local dir found
  for dir in \
    "$BUNDLE_ROOT/universal-apple-darwin/release/bundle/dmg" \
    "$BUNDLE_ROOT/release/bundle/dmg" \
    "$BUNDLE_ROOT/aarch64-apple-darwin/release/bundle/dmg" \
    "$BUNDLE_ROOT/x86_64-apple-darwin/release/bundle/dmg"
  do
    if [[ -d "$dir" ]]; then
      found="$(find "$dir" -maxdepth 1 -type f -name "*.dmg" ! -name "rw.*" -print -quit 2>/dev/null || true)"
      if [[ -n "$found" ]]; then
        printf '%s\n' "$found"
        return 0
      fi
    fi
  done
  if [[ -d "$BUNDLE_ROOT" ]]; then
    found="$(find "$BUNDLE_ROOT" -type f -name "*.dmg" ! -name "rw.*" -print -quit 2>/dev/null || true)"
    if [[ -n "$found" ]]; then
      printf '%s\n' "$found"
      return 0
    fi
  fi
  return 1
}

extract_app_from_dmg() {
  local dmg="$1"
  if [[ "$(uname -s)" != "Darwin" ]]; then
    return 1
  fi
  EXTRACT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/genomics-caddy-dmg-src.XXXXXX")"
  local mount_point="$EXTRACT_DIR/mnt"
  mkdir -p "$mount_point"
  if ! hdiutil attach -nobrowse -readonly -mountpoint "$mount_point" "$dmg" >/dev/null; then
    echo "package_macos_folder_dmg: failed to attach $dmg" >&2
    return 1
  fi
  ATTACHED_MOUNT="$mount_point"
  local found
  found="$(find "$mount_point" -type d -name "*.app" -print -quit 2>/dev/null || true)"
  if [[ -z "$found" || ! -d "$found" ]]; then
    echo "package_macos_folder_dmg: $dmg did not contain a .app" >&2
    return 1
  fi
  local copied="$EXTRACT_DIR/$APP_NAME"
  cp -R "$found" "$copied"
  hdiutil detach "$ATTACHED_MOUNT" -quiet >/dev/null 2>&1 || true
  ATTACHED_MOUNT=""
  printf '%s\n' "$copied"
}

resolve_app_path() {
  local source_kind="arg"
  if [[ -n "$APP_PATH" ]]; then
    if [[ ! -d "$APP_PATH" ]]; then
      echo "package_macos_folder_dmg: $APP_PATH is not a directory" >&2
      return 1
    fi
    printf '%s\t%s\n' "$APP_PATH" "$source_kind"
    return 0
  fi

  local resolved
  if resolved="$(find_live_app)"; then
    printf '%s\t%s\n' "$resolved" "live"
    return 0
  fi

  local tarball
  if tarball="$(find_app_tarball)"; then
    echo "package_macos_folder_dmg: live $APP_NAME missing; unpacking $tarball" >&2
    resolved="$(extract_app_from_tarball "$tarball")"
    printf '%s\t%s\n' "$resolved" "tarball"
    return 0
  fi

  local dmg
  if dmg="$(find_product_dmg)"; then
    echo "package_macos_folder_dmg: live $APP_NAME missing; copying from $dmg" >&2
    if resolved="$(extract_app_from_dmg "$dmg")"; then
      printf '%s\t%s\n' "$resolved" "dmg"
      return 0
    fi
  fi

  echo "package_macos_folder_dmg: $APP_NAME not found under $BUNDLE_ROOT (looked for the live .app, $TARBALL_NAME, and a product .dmg)" >&2
  return 1
}

run_self_test() {
  local tmp macos_dir payload
  tmp="$(mktemp -d "${TMPDIR:-/tmp}/genomics-caddy-macos-self-test.XXXXXX")"
  macos_dir="$tmp/universal-apple-darwin/release/bundle/macos"
  payload="$macos_dir/payload"
  mkdir -p "$payload/$APP_NAME/Contents"
  printf 'stub\n' > "$payload/$APP_NAME/Contents/Info.plist"
  tar -C "$payload" -czf "$macos_dir/$TARBALL_NAME" "$APP_NAME"
  rm -rf "$payload"

  local printed
  printed="$(MACOS_BUNDLE_ROOT="$tmp" bash "$ROOT/scripts/package_macos_folder_dmg.sh" --print-app-path)"
  if [[ "$printed" != *.app ]]; then
    echo "package_macos_folder_dmg self-test: expected a .app path, got: $printed" >&2
    rm -rf "$tmp"
    return 1
  fi
  rm -rf "$tmp"
  echo "package_macos_folder_dmg self-test: recovered .app from tar.gz"
}

if [[ "$MODE" == "self-test" ]]; then
  run_self_test
  exit 0
fi

if [[ "$MODE" != "print-app-path" && "$(uname -s)" != "Darwin" ]]; then
  echo "package_macos_folder_dmg: skipping on $(uname -s) (needs hdiutil)"
  exit 0
fi

RESOLVE_LINE="$(resolve_app_path)"
APP_PATH="${RESOLVE_LINE%%$'\t'*}"
SOURCE_KIND="${RESOLVE_LINE#*$'\t'}"

if [[ "$MODE" == "print-app-path" ]]; then
  printf '%s\n' "$APP_PATH"
  exit 0
fi

if [[ -z "$APP_PATH" || ! -d "$APP_PATH" ]]; then
  echo "package_macos_folder_dmg: $APP_NAME not found" >&2
  exit 1
fi

echo "package_macos_folder_dmg: using $APP_PATH (source=$SOURCE_KIND)"

if [[ -z "$DMG_OUT" ]]; then
  DMG_OUT="$(find_product_dmg || true)"
fi
if [[ -z "$DMG_OUT" ]]; then
  DMG_OUT="$ROOT/builds/macOS/${PRODUCT_NAME}.dmg"
fi

mkdir -p "$(dirname "$DMG_OUT")"
STAGING="$(mktemp -d "${TMPDIR:-/tmp}/genomics-caddy-dmg.XXXXXX")"

cp -R "$APP_PATH" "$STAGING/$APP_NAME"
mkdir -p "$STAGING/Data"
touch "$STAGING/Data/.keep"

rm -f "$DMG_OUT"
hdiutil create \
  -volname "$PRODUCT_NAME" \
  -srcfolder "$STAGING" \
  -ov \
  -format UDZO \
  "$DMG_OUT"

echo "Wrote folder DMG $DMG_OUT"
bash "$ROOT/scripts/sign_updater_artifact.sh" "$DMG_OUT"
