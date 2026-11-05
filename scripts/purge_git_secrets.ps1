# Safely preview (or explicitly rewrite) local Git history to remove sensitive paths.
# Default is read-only. No package installation, push, or remote mutation is performed.
#
# Preview: pwsh -File scripts/purge_git_secrets.ps1
# Execute: pwsh -File scripts/purge_git_secrets.ps1 -Execute

param(
    [switch]$Execute
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location $repoRoot

$pathsToPurge = @(
    "AncestryDNA.txt"
)

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git not found on PATH"
}

$actualRoot = (& git rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $actualRoot) {
    throw "Run this script from inside the target Git repository."
}
$actualRoot = [System.IO.Path]::GetFullPath($actualRoot.Trim())
if ($actualRoot -ne [System.IO.Path]::GetFullPath($repoRoot)) {
    throw "Repository root mismatch; refusing to inspect or rewrite another repository."
}

Write-Host "Genomics Caddy — Git history purge" -ForegroundColor Cyan
Write-Host "Target paths: $($pathsToPurge -join ', ')"
Write-Host ""

$objects = @(& git rev-list --objects --all)
if ($LASTEXITCODE -ne 0) {
    throw "Could not inspect reachable Git objects."
}
$pathMatches = @(
    foreach ($line in $objects) {
        foreach ($target in $pathsToPurge) {
            if ($line -match ("\s" + [regex]::Escape($target) + "$")) { $line }
        }
    }
)

if ($pathMatches.Count -gt 0) {
    Write-Host "Reachable history contains matching path entries:" -ForegroundColor Yellow
    $pathMatches | ForEach-Object { Write-Host "  $_" }
} else {
    Write-Host "No matching paths were found in reachable history." -ForegroundColor Green
}

if (-not $Execute) {
    Write-Host ""
    Write-Host "Dry run only. No files, refs, or Git history were changed."
    Write-Host "If removal is needed, review the matches, back up the repository, then run with -Execute."
    exit 0
}

if ($pathMatches.Count -eq 0) {
    Write-Host "Nothing to rewrite."
    exit 0
}

$status = @(& git status --porcelain)
if ($LASTEXITCODE -ne 0) { throw "Could not inspect the worktree state." }
if ($status.Count -gt 0) {
    throw "The worktree must be completely clean before a history rewrite. Commit or back up unrelated changes, then retry."
}

if (-not (Get-Command git-filter-repo -ErrorAction SilentlyContinue)) {
    throw "git-filter-repo is required but was not found. Install it using its official instructions, then retry; this script will not install software automatically."
}

Write-Host ""
Write-Host "This rewrites local Git history and can affect collaborators and remote refs." -ForegroundColor Red
Write-Host "Back up the repository first. No push will be performed by this script."
$confirmation = Read-Host "Type REWRITE HISTORY to continue"
if ($confirmation -cne "REWRITE HISTORY") {
    throw "Confirmation did not match; no history was changed."
}

$filterArgs = @("filter-repo", "--invert-paths")
foreach ($target in $pathsToPurge) {
    $filterArgs += @("--path", $target)
}
$filterArgs += "--force"
& git @filterArgs
if ($LASTEXITCODE -ne 0) {
    throw "git-filter-repo failed. Inspect the repository and its backup before retrying."
}

$remainingObjects = @(& git rev-list --objects --all)
if ($LASTEXITCODE -ne 0) { throw "History rewrite ran, but post-rewrite verification failed." }
$remainingPathMatches = @(
    foreach ($line in $remainingObjects) {
        foreach ($target in $pathsToPurge) {
            if ($line -match ("\s" + [regex]::Escape($target) + "$")) { $line }
        }
    }
)
if ($remainingPathMatches.Count -gt 0) {
    throw "The rewrite completed but matching paths remain reachable. Inspect the repository and backup before publishing."
}

Write-Host ""
Write-Host "Local history rewrite verified; target paths are no longer reachable." -ForegroundColor Green
Write-Host "No remote push was performed. Review remotes, refs, and collaborators before any publication."
