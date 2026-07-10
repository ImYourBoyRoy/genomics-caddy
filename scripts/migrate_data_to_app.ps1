# ./scripts/migrate_data_to_app.ps1
# One-time copy from legacy ./data to ./App/Data (does not delete source unless -Move).
#
# Usage:
#   pwsh -File .\scripts\migrate_data_to_app.ps1
#   pwsh -File .\scripts\migrate_data_to_app.ps1 -Move

[CmdletBinding()]
param(
    [switch]$Move
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Legacy = Join-Path $RepoRoot "data"
$Target = Join-Path $RepoRoot "App\Data"

if (-not (Test-Path $Legacy)) {
    Write-Host "No legacy data/ folder found — nothing to migrate." -ForegroundColor Yellow
    exit 0
}

New-Item -ItemType Directory -Force -Path $Target | Out-Null

Write-Host "Migrating genome data:" -ForegroundColor Cyan
Write-Host "  From: $Legacy"
Write-Host "  To:   $Target"

if ($Move) {
    robocopy $Legacy $Target /E /MOVE /NFL /NDL /NJH /NJS /NC /NS | Out-Null
    if ($LASTEXITCODE -ge 8) {
        throw "robocopy move failed (exit $LASTEXITCODE)"
    }
    Write-Host "Move complete." -ForegroundColor Green
} else {
    robocopy $Legacy $Target /E /NFL /NDL /NJH /NJS /NC /NS | Out-Null
    if ($LASTEXITCODE -ge 8) {
        throw "robocopy copy failed (exit $LASTEXITCODE)"
    }
    Write-Host "Copy complete. Legacy data/ left in place." -ForegroundColor Green
    Write-Host "Re-run with -Move after verifying App/Data looks correct." -ForegroundColor Yellow
}
