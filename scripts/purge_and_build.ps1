# ./scripts/purge_and_build.ps1
# Purges frontend/Rust *build caches* only, then compiles a production Tauri release.
# Does NOT touch genome data, offline downloads, chat history, or SQLite progress.
#
# Usage (from repo root):
#   pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File .\scripts\purge_and_build.ps1
#   pwsh -File .\scripts\purge_and_build.ps1 -PurgeOnly
#   pwsh -File .\scripts\purge_and_build.ps1 -SkipChecks
#   pwsh -File .\scripts\purge_and_build.ps1 -DryRun

[CmdletBinding()]
param(
    [switch]$PurgeOnly,
    [switch]$SkipPurge,
    [switch]$SkipChecks,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Write-Step([string]$Message) {
    Write-Host "`n==> $Message" -ForegroundColor Cyan
}

function Write-Ok([string]$Message) {
    Write-Host "    $Message" -ForegroundColor Green
}

function Write-Skip([string]$Message) {
    Write-Host "    (skip) $Message" -ForegroundColor DarkGray
}

function Assert-RepoRoot {
    param([string]$Root)
    if (-not (Test-Path (Join-Path $Root "package.json"))) {
        throw "package.json not found — run from the Genomics Caddy repo root."
    }
    if (-not (Test-Path (Join-Path $Root "src-tauri\Cargo.toml"))) {
        throw "src-tauri/Cargo.toml not found — run from the Genomics Caddy repo root."
    }
}

function Test-ProtectedPath {
    param(
        [string]$Root,
        [string]$RelativePath
    )

    $normalized = ($RelativePath -replace "\\", "/").Trim("/").ToLowerInvariant()
    if ([string]::IsNullOrWhiteSpace($normalized)) {
        return $true
    }

    $protectedPrefixes = @(
        "data",
        "app",
        ".git",
        "src",
        "scripts",
        "static"
    )

    $allowedNodeModuleCaches = @(
        "node_modules/.vite",
        "node_modules/.cache",
        "node_modules/.vitest"
    )

    if ($allowedNodeModuleCaches -contains $normalized) {
        return $false
    }

    if ($normalized -eq "node_modules" -or $normalized.StartsWith("node_modules/")) {
        return $true
    }

    foreach ($prefix in $protectedPrefixes) {
        if ($normalized -eq $prefix -or $normalized.StartsWith("$prefix/")) {
            return $true
        }
    }

    $protectedNames = @(
        ".env",
        "package.json",
        "package-lock.json",
        "readme.md",
        "memory.md"
    )

    $leaf = Split-Path $normalized -Leaf
    if ($protectedNames -contains $leaf) {
        return $true
    }

    # Never delete user databases or sealed copies anywhere under the repo.
    if ($leaf -like "user_genome.db*" -or $leaf -like "*.sqlite*") {
        return $true
    }

    return $false
}

function Remove-BuildArtifact {
    param(
        [string]$Root,
        [string]$RelativePath,
        [ref]$RemovedCount,
        [ref]$FreedBytes
    )

    if (Test-ProtectedPath -Root $Root -RelativePath $RelativePath) {
        Write-Skip "protected: $RelativePath"
        return
    }

    $fullPath = Join-Path $Root $RelativePath
    if (-not (Test-Path $fullPath)) {
        Write-Skip "missing: $RelativePath"
        return
    }

    if ($DryRun) {
        Write-Host "    [dry-run] would remove: $RelativePath" -ForegroundColor Yellow
        return
    }

    $size = 0
    if (Test-Path $fullPath -PathType Leaf) {
        $size = (Get-Item $fullPath).Length
        Remove-Item -LiteralPath $fullPath -Force
    } else {
        $size = (Get-ChildItem -LiteralPath $fullPath -Recurse -Force -ErrorAction SilentlyContinue |
            Measure-Object -Property Length -Sum).Sum
        if ($null -eq $size) { $size = 0 }
        Remove-Item -LiteralPath $fullPath -Recurse -Force
    }

    $RemovedCount.Value++
    $FreedBytes.Value += [int64]$size
    Write-Ok "removed $RelativePath ($([math]::Round($size / 1MB, 2)) MB)"
}

function Invoke-External {
    param(
        [string]$Label,
        [string]$FilePath,
        [string[]]$Arguments
    )

    Write-Step $Label
    Write-Host "    $FilePath $($Arguments -join ' ')" -ForegroundColor DarkGray
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed (exit $LASTEXITCODE)"
    }
}

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $RepoRoot
Assert-RepoRoot -Root $RepoRoot

Write-Host "Genomics Caddy — purge build caches + production release" -ForegroundColor Cyan
Write-Host "Repo: $RepoRoot"
Write-Host ""
Write-Host "SAFE: App/Data, data/, user_genome.db, raw_downloads/, references/, chat history, .env" -ForegroundColor Green
Write-Host "PURGE: build/, .svelte-kit/, Cargo target/, frontend tool caches only" -ForegroundColor Yellow

$removed = 0
$freed = [int64]0

if (-not $SkipPurge) {
    Write-Step "Purging frontend build artifacts"

    $frontendArtifacts = @(
        "build",
        ".svelte-kit",
        "package",
        "node_modules\.vite",
        "node_modules\.cache",
        "node_modules\.vitest",
        "src-tauri\gen\schemas"
    )

    foreach ($relative in $frontendArtifacts) {
        Remove-BuildArtifact -Root $RepoRoot -RelativePath $relative -RemovedCount ([ref]$removed) -FreedBytes ([ref]$freed)
    }

    Write-Step "Purging Rust/Cargo build cache"
    if ($DryRun) {
        Write-Host "    [dry-run] would run: cargo clean --manifest-path src-tauri/Cargo.toml" -ForegroundColor Yellow
    } else {
        Invoke-External -Label "cargo clean" -FilePath "cargo" -Arguments @(
            "clean",
            "--manifest-path", "src-tauri/Cargo.toml"
        )
        $removed++
    }
} else {
    Write-Skip "Purge skipped (-SkipPurge)"
}

if ($PurgeOnly) {
    Write-Host "`nPurge complete (-PurgeOnly). Removed $removed target(s); freed ~$([math]::Round($freed / 1MB, 2)) MB." -ForegroundColor Green
    exit 0
}

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "npm not found on PATH"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo not found on PATH"
}

if (-not $SkipChecks) {
    Invoke-External -Label "Frontend type check" -FilePath "npm" -Arguments @("run", "check")
    Invoke-External -Label "Rust library check" -FilePath "cargo" -Arguments @(
        "check",
        "--manifest-path", "src-tauri/Cargo.toml",
        "--lib"
    )
} else {
    Write-Skip "Pre-build checks skipped (-SkipChecks)"
}

Invoke-External -Label "Production Tauri build (no installer bundle)" -FilePath "npm" -Arguments @(
    "run", "tauri", "build"
)

Write-Step "Stage portable App/ folder"
$AppDir = Join-Path $RepoRoot "App"
$ReleaseDir = Join-Path $RepoRoot "src-tauri\target\release"
if (-not (Test-Path $ReleaseDir)) {
    throw "Release directory not found: $ReleaseDir"
}
New-Item -ItemType Directory -Force -Path $AppDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $AppDir "Data") | Out-Null

$runtimePatterns = @("*.exe", "*.dll")
foreach ($pattern in $runtimePatterns) {
    Get-ChildItem -Path $ReleaseDir -Filter $pattern -File -ErrorAction SilentlyContinue |
        ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination $AppDir -Force
            Write-Ok "staged $($_.Name)"
        }
}

foreach ($extra in @("resources", "WebView2Loader.dll")) {
    $source = Join-Path $ReleaseDir $extra
    if (Test-Path $source) {
        if ((Get-Item $source).PSIsContainer) {
            Copy-Item -LiteralPath $source -Destination (Join-Path $AppDir (Split-Path $extra -Leaf)) -Recurse -Force
        } else {
            Copy-Item -LiteralPath $source -Destination $AppDir -Force
        }
        Write-Ok "staged $extra"
    }
}

Write-Step "Release artifacts"
Write-Ok (Join-Path $AppDir "Genomics Caddy.exe")
Write-Ok (Join-Path $AppDir "DNA-Tools.exe")
Write-Ok (Join-Path $AppDir "Data")
Write-Ok "Build cache source (safe to delete): $ReleaseDir"

Write-Host "`nBuild complete." -ForegroundColor Green
if (-not $SkipPurge) {
    Write-Host "Purged $removed build target(s); freed ~$([math]::Round($freed / 1MB, 2)) MB of cache." -ForegroundColor Green
}
Write-Host "Run the staged app from: $(Join-Path $AppDir 'DNA-Tools.exe')" -ForegroundColor Green
Write-Host "Persistent data belongs in: $(Join-Path $AppDir 'Data')" -ForegroundColor Green
Write-Host "Your genome data and offline downloads were not modified." -ForegroundColor Green
