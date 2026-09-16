# ./scripts/remote_build/toolchain.py
"""
Build environment upgrade & auto-healing runner — runs before every compile pass.

Ensures each remote target auto-heals any missing requirements:
  - Rust & targets (rustup update stable, rustup target add …)
  - Tauri CLI (cargo install tauri-cli)
  - Node.js and pnpm (auto-installed via Homebrew on macOS or fnm/pkg-manager on Linux)
  - Project frontend dependencies (temporary-lock pnpm install + update, no retained lockfile)

Platform-specific upgrade scripts are delivered via stdin (bash -s) so no
secrets or env values appear in process arguments or SSH command strings.

Inputs:
  platform:  "macos" | "linux" | "windows"
  settings:  Remote_Build Settings (used for host_bash / vm_ssh routing)
  guest_dir: Path on the target where the project was synced

Outputs / side effects:
  Auto-healed and fully upgraded toolchain on the target machine.
  Prints upgrade progress lines.
"""
from __future__ import annotations

from remote_build.config import Settings, get_settings
from remote_build.upgrade.host import host_bash
from remote_build.ssh import vm_ssh

_MACOS_UPGRADE_SCRIPT = """
set -euo pipefail
source ~/.cargo/env 2>/dev/null || true
export PATH="$HOME/.cargo/bin:/usr/local/bin:/opt/homebrew/bin:$PATH"

echo "[TOOLCHAIN] Auto-healing and checking prerequisites on macOS guest…"

# 1. Auto-heal Rust / rustup
if ! command -v rustup >/dev/null 2>&1; then
    echo "[AUTO-HEAL] rustup missing — installing Rust toolchain…"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

echo "[TOOLCHAIN] rustup update stable…"
rustup update stable

echo "[TOOLCHAIN] Registering Apple targets…"
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# 2. Auto-heal cargo-tauri / tauri-cli
if ! command -v cargo-tauri >/dev/null 2>&1 && ! command -v tauri >/dev/null 2>&1; then
    echo "[AUTO-HEAL] tauri-cli missing — installing via cargo install…"
    cargo install tauri-cli
else
    echo "[TOOLCHAIN] tauri-cli (latest check)…"
    cargo install tauri-cli 2>&1 | tail -5
fi

# 3. Auto-heal Node.js / npm / pnpm
if ! command -v node >/dev/null 2>&1 || ! command -v npm >/dev/null 2>&1; then
    echo "[AUTO-HEAL] Node.js/npm missing on macOS guest — auto-installing via Homebrew…"
    if command -v brew >/dev/null 2>&1; then
        brew install node
    elif [ -x /usr/local/bin/brew ]; then
        /usr/local/bin/brew install node
    elif [ -x /opt/homebrew/bin/brew ]; then
        /opt/homebrew/bin/brew install node
    else
        echo "[AUTO-HEAL] Homebrew missing — installing Homebrew…"
        NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        /usr/local/bin/brew install node || /opt/homebrew/bin/brew install node
    fi
fi

# Verify active versions
echo "[TOOLCHAIN] Verified tools:"
echo "  rustc: $(rustc --version 2>/dev/null || echo missing)"
echo "  tauri: $(tauri --version 2>/dev/null || cargo tauri --version 2>/dev/null || echo missing)"
echo "  node:  $(node -v 2>/dev/null || echo missing)"
echo "  npm:   $(npm -v 2>/dev/null || echo missing)"
if command -v npm >/dev/null 2>&1; then npm install --global pnpm@latest >/dev/null 2>&1 || true; fi
echo "  pnpm:  $(pnpm -v 2>/dev/null || echo missing)"

if [ -d '{guest_dir}' ]; then
    echo "[TOOLCHAIN] pnpm update (temporary lock, not retained)…"
    cd '{guest_dir}'
    node ./scripts/pnpm_unlocked.mjs install 2>&1 | tail -10
    node ./scripts/pnpm_unlocked.mjs update --latest 2>&1 | tail -10
fi

echo "[TOOLCHAIN] UPGRADE_DONE"
"""

_LINUX_UPGRADE_SCRIPT = """
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

echo "[TOOLCHAIN] Auto-healing and checking prerequisites on Linux builder…"

# 1. Auto-heal Rust / rustup
if ! command -v rustup >/dev/null 2>&1; then
    echo "[AUTO-HEAL] rustup missing — installing Rust toolchain…"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "[TOOLCHAIN] rustup update stable…"
rustup update stable

echo "[TOOLCHAIN] Registering Linux target…"
rustup target add x86_64-unknown-linux-gnu

# 2. Auto-heal cargo-tauri / tauri-cli
if ! command -v cargo-tauri >/dev/null 2>&1 && ! command -v tauri >/dev/null 2>&1; then
    echo "[AUTO-HEAL] tauri-cli missing — installing via cargo install…"
    cargo install tauri-cli
else
    echo "[TOOLCHAIN] tauri-cli (latest check)…"
    cargo install tauri-cli 2>&1 | tail -5
fi

# 3. Auto-heal Node.js / npm / pnpm to latest Node 26+
if [ -d "$HOME/.local/share/fnm" ] || [ -f "$HOME/.local/bin/fnm" ]; then
    export PATH="$HOME/.local/bin:$HOME/.local/share/fnm:$PATH"
    eval "$(fnm env 2>/dev/null || true)"
fi

if ! command -v fnm >/dev/null 2>&1; then
    echo "[AUTO-HEAL] fnm missing on Linux builder — installing fnm…"
    curl -fsSL https://fnm.vercel.app/install | bash
    export PATH="$HOME/.local/bin:$HOME/.local/share/fnm:$PATH"
    eval "$(fnm env 2>/dev/null || true)"
fi

echo "[TOOLCHAIN] Upgrading Node to latest 26+ via fnm…"
fnm install 26 2>/dev/null || fnm install --lts || true
fnm use 26 2>/dev/null || fnm use lts-latest || true
fnm default 26 2>/dev/null || true

# Verify active versions
echo "[TOOLCHAIN] Verified tools:"
echo "  rustc: $(rustc --version 2>/dev/null || echo missing)"
echo "  tauri: $(tauri --version 2>/dev/null || cargo tauri --version 2>/dev/null || echo missing)"
echo "  node:  $(node -v 2>/dev/null || echo missing)"
echo "  npm:   $(npm -v 2>/dev/null || echo missing)"
if command -v npm >/dev/null 2>&1; then npm install --global pnpm@latest >/dev/null 2>&1 || true; fi
echo "  pnpm:  $(pnpm -v 2>/dev/null || echo missing)"

# 4. Auto-heal system dev packages (pkg-config, libdbus-1-dev, GTK, WebKit, SSL)
echo "[TOOLCHAIN] Checking Linux system dev packages…"
MISSING_PKGS=0
for pkg in pkg-config dbus-1 gtk+-3.0 webkit2gtk-4.1; do
    if ! pkg-config --exists "$pkg" 2>/dev/null; then
        echo "  MISSING system package: $pkg"
        MISSING_PKGS=1
    fi
done

if [ "$MISSING_PKGS" -eq 1 ]; then
    echo "[AUTO-HEAL] Installing missing Linux dev packages via apt-get…"
    if command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
        sudo apt-get update -qq
        sudo DEBIAN_FRONTEND=noninteractive apt-get install -y \
            build-essential curl wget file \
            pkg-config libdbus-1-dev libssl-dev libxdo-dev \
            libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
    else
        echo "[AUTO-HEAL] sudo requires elevation — trying non-interactive apt-get if available"
    fi
fi

if [ -d '{guest_dir}' ]; then
    echo "[TOOLCHAIN] pnpm update (temporary lock, not retained)…"
    cd '{guest_dir}'
    node ./scripts/pnpm_unlocked.mjs install 2>&1 | tail -10
    node ./scripts/pnpm_unlocked.mjs update --latest 2>&1 | tail -10
fi

echo "[TOOLCHAIN] UPGRADE_DONE"
"""

_WINDOWS_UPGRADE_SCRIPT = """
# Windows: run via PowerShell over SSH
$ErrorActionPreference = 'Stop'
$env:PATH += ";$env:USERPROFILE\\.cargo\\bin"

Write-Host '[TOOLCHAIN] Checking prerequisites on WinServer…'

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Host '[AUTO-HEAL] Installing Rustup…'
    Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
    .\rustup-init.exe -y
    Remove-Item rustup-init.exe
}

Write-Host '[TOOLCHAIN] rustup update stable…'
rustup update stable

Write-Host '[TOOLCHAIN] Registering Windows target…'
rustup target add x86_64-pc-windows-msvc

Write-Host '[TOOLCHAIN] tauri-cli (latest)…'
cargo install tauri-cli

if ([System.IO.Directory]::Exists('{guest_dir}')) {
    Write-Host '[TOOLCHAIN] pnpm update (temporary lock, not retained)…'
    Set-Location '{guest_dir}'
    npm install --global pnpm@latest
    node ./scripts/pnpm_unlocked.mjs install
    node ./scripts/pnpm_unlocked.mjs update --latest
}

Write-Host '[TOOLCHAIN] UPGRADE_DONE'
"""


def upgrade_macos(
    guest_dir: str,
    *,
    settings: Settings | None = None,
) -> None:
    """Upgrade and auto-heal toolchain on the macOS VM guest via jump-host."""
    s = settings or get_settings()
    print(f"[TOOLCHAIN] Auto-healing & upgrading macOS guest ({s.vm_host}) toolchain…")
    script = _MACOS_UPGRADE_SCRIPT.format(guest_dir=guest_dir)
    vm_ssh(
        host=s.remote_host,
        user=s.remote_user,
        key=s.ssh_key_expanded,
        vm_host=s.vm_host,
        vm_user=s.vm_user,
        guest_script=script,
        check=True,
        timeout=1200,  # 20 min (allows brew install if needed)
    )
    print("[TOOLCHAIN] macOS auto-heal & upgrade complete.")


def upgrade_linux(
    guest_dir: str,
    *,
    settings: Settings | None = None,
) -> None:
    """Upgrade and auto-heal toolchain on the Ubuntu builder host."""
    s = settings or get_settings()
    print(f"[TOOLCHAIN] Auto-healing & upgrading Linux builder ({s.remote_host}) toolchain…")
    script = _LINUX_UPGRADE_SCRIPT.format(guest_dir=guest_dir)
    host_bash(script, settings=s, check=True, timeout=1200)
    print("[TOOLCHAIN] Linux auto-heal & upgrade complete.")


def upgrade_windows(
    guest_dir: str,
    win_host: str,
    win_user: str,
    *,
    settings: Settings | None = None,
) -> None:
    """Upgrade and auto-heal toolchain on the WinServer VM guest via jump-host."""
    s = settings or get_settings()
    print(f"[TOOLCHAIN] Auto-healing & upgrading WinServer guest ({win_host}) toolchain…")
    script = _WINDOWS_UPGRADE_SCRIPT.format(guest_dir=guest_dir)
    vm_ssh(
        host=s.remote_host,
        user=s.remote_user,
        key=s.ssh_key_expanded,
        vm_host=win_host,
        vm_user=win_user,
        guest_script=script,
        check=True,
        timeout=1200,
    )
    print("[TOOLCHAIN] Windows auto-heal & upgrade complete.")


def upgrade(
    platform: str,
    guest_dir: str,
    *,
    settings: Settings | None = None,
    win_host: str = "",
    win_user: str = "",
) -> None:
    """Dispatch toolchain upgrade & auto-healing to the correct platform handler."""
    if platform == "macos":
        upgrade_macos(guest_dir, settings=settings)
    elif platform == "linux":
        upgrade_linux(guest_dir, settings=settings)
    elif platform == "windows":
        if not win_host or not win_user:
            raise SystemExit(
                "[FAIL] Windows upgrade requires WIN_VM_HOST and WIN_VM_USER in .env"
            )
        upgrade_windows(guest_dir, win_host, win_user, settings=settings)
    else:
        raise ValueError(f"Unknown platform: {platform!r}")
