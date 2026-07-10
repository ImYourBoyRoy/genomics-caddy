# ./scripts/purge_git_secrets.ps1
# Removes sensitive paths from entire git history (local repo rewrite).
# Run from repo root: pwsh -File scripts/purge_git_secrets.ps1
#
# WARNING: Rewrites history. If you have a remote, you must force-push afterward
# and all collaborators must re-clone. Rotate any secrets that were ever committed.

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

$pathsToPurge = @(
    "AncestryDNA.txt"
)

Write-Host "Genomics Caddy — git history purge" -ForegroundColor Cyan
Write-Host "Targets: $($pathsToPurge -join ', ')"
Write-Host ""

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git not found on PATH"
}

$filterRepo = Get-Command git-filter-repo -ErrorAction SilentlyContinue
if (-not $filterRepo) {
    Write-Host "Installing git-filter-repo via pip..." -ForegroundColor Yellow
    python -m pip install --user git-filter-repo
    $env:Path += [System.IO.Path]::PathSeparator + "$env:APPDATA\Python\Python314\Scripts"
    $env:Path += [System.IO.Path]::PathSeparator + "$env:LOCALAPPDATA\Programs\Python\Python314\Scripts"
}

if (-not (Get-Command git-filter-repo -ErrorAction SilentlyContinue)) {
    throw "git-filter-repo not available. Install with: python -m pip install git-filter-repo"
}

$pathArgs = ($pathsToPurge | ForEach-Object { "--path"; $_ }) -join " "
Write-Host "Running: git filter-repo --invert-paths $pathArgs --force" -ForegroundColor Yellow
& git filter-repo --invert-paths @(
    foreach ($p in $pathsToPurge) { "--path"; $p }
) --force

Write-Host ""
Write-Host "Done. Verify with:" -ForegroundColor Green
Write-Host "  git log --all --oneline -- AncestryDNA.txt"
Write-Host "  git rev-list --objects --all | Select-String AncestryDNA"
Write-Host ""
Write-Host "If this repo was pushed to GitHub, run:" -ForegroundColor Yellow
Write-Host "  git push --force --all"
Write-Host "Rotate Qdrant/NCBI keys if they ever appeared in commits."
