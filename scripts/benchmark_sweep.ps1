# ./scripts/benchmark_sweep.ps1
<#
.SYNOPSIS
  Smoke-benchmark dynamic pipeline tuning and service latency probes.

.DESCRIPTION
  Prints resolved tuning knobs for the current machine (or GENOMICS_CPU_LIMIT)
  and optionally probes Qdrant/Ollama from .env. Does not start a sweep.

.EXAMPLE
  pwsh -NoLogo -NoProfile -File ./scripts/benchmark_sweep.ps1
  GENOMICS_CPU_LIMIT=80 pwsh -NoLogo -NoProfile -File ./scripts/benchmark_sweep.ps1
#>
param(
  [string]$OllamaUrl = $env:OLLAMA_URL,
  [string]$QdrantUrl = $env:QDRANT_URL
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $OllamaUrl) { $OllamaUrl = "http://localhost:11434" }
if (-not $QdrantUrl) { $QdrantUrl = "http://localhost:6333" }

Write-Host "=== Genomics Caddy — pipeline tuning benchmark ===" -ForegroundColor Cyan
Write-Host "CPUs (logical): $([Environment]::ProcessorCount)"
if ($env:GENOMICS_CPU_LIMIT) {
  Write-Host "GENOMICS_CPU_LIMIT: $($env:GENOMICS_CPU_LIMIT)"
}

Push-Location src-tauri
try {
  $env:GENOMICS_TUNING = if ($env:GENOMICS_TUNING) { $env:GENOMICS_TUNING } else { "dynamic" }
  cargo test -q tuning::tests -- --nocapture 2>&1 | ForEach-Object { Write-Host $_ }
} finally {
  Pop-Location
}

function Measure-HttpMs($url) {
  try {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $null = Invoke-WebRequest -Uri $url -TimeoutSec 8 -UseBasicParsing
    $sw.Stop()
    return [int]$sw.ElapsedMilliseconds
  } catch {
    return $null
  }
}

$ollamaMs = Measure-HttpMs "$($OllamaUrl.TrimEnd('/'))/api/tags"
$qdrantMs = Measure-HttpMs "$($QdrantUrl.TrimEnd('/'))/collections"
Write-Host ""
Write-Host "Latency probes:" -ForegroundColor Cyan
Write-Host "  Ollama ($OllamaUrl): $(if ($null -ne $ollamaMs) { "${ollamaMs}ms" } else { 'unreachable' })"
Write-Host "  Qdrant ($QdrantUrl): $(if ($null -ne $qdrantMs) { "${qdrantMs}ms" } else { 'unreachable' })"
Write-Host ""
Write-Host "Headless Docker worker:" -ForegroundColor Cyan
Write-Host "  docker compose -f docker/docker-compose.yml up genomics-worker"
Write-Host "  logs: docker compose -f docker/docker-compose.yml logs -f genomics-worker | Select-String GENOMICS_PROGRESS"
