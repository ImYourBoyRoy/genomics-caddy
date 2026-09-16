# ./scripts/benchmark_build.ps1
# Time Genomics Caddy build-cache purge and/or full production rebuild on Windows.
# Does NOT touch App/Data, genome DBs, downloads, or .env.
#
# Usage (from repo root):
#   pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File .\scripts\benchmark_build.ps1
#   pwsh -File .\scripts\benchmark_build.ps1 -PurgeOnly
#   pwsh -File .\scripts\benchmark_build.ps1 -SkipChecks
#   pwsh -File .\scripts\benchmark_build.ps1 -Json

[CmdletBinding()]
param(
    [switch]$PurgeOnly,
    [switch]$SkipChecks,
    [switch]$Json
)

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $RepoRoot

if (-not (Test-Path (Join-Path $RepoRoot "package.json"))) {
    throw "package.json not found — run from the Genomics Caddy repo root."
}
if (-not (Test-Path (Join-Path $RepoRoot "src-tauri\Cargo.toml"))) {
    throw "src-tauri/Cargo.toml not found — run from the Genomics Caddy repo root."
}

$ReportDir = Join-Path $RepoRoot "App\Data\benchmarks"
New-Item -ItemType Directory -Force -Path $ReportDir | Out-Null

$Stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$HostName = $env:COMPUTERNAME
if ([string]::IsNullOrWhiteSpace($HostName)) { $HostName = "windows" }
$ReportTxt = Join-Path $ReportDir "build_bench_${HostName}_${Stamp}.txt"
$ReportJson = Join-Path $ReportDir "build_bench_${HostName}_${Stamp}.json"

function Get-CpuName {
    try {
        $c = Get-CimInstance Win32_Processor -ErrorAction Stop | Select-Object -First 1
        if ($c -and $c.Name) { return $c.Name.Trim() }
    } catch {}
    return "unknown"
}

function Get-RamGiB {
    try {
        $m = Get-CimInstance Win32_ComputerSystem -ErrorAction Stop
        return ("{0:N1}Gi" -f ($m.TotalPhysicalMemory / 1GB))
    } catch {
        return "?"
    }
}

function Get-ToolVersion([string]$Command, [string[]]$Args) {
    try {
        $out = & $Command @Args 2>$null
        if ($LASTEXITCODE -ne 0 -and $null -eq $out) { return "missing" }
        if ($out -is [array]) { return [string]$out[0] }
        return [string]$out
    } catch {
        return "missing"
    }
}

$Cpu = Get-CpuName
$Cores = [Environment]::ProcessorCount
$Ram = Get-RamGiB
$Os = [System.Environment]::OSVersion.VersionString
$Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$NodeV = Get-ToolVersion "node" @("-v")
$PnpmV = Get-ToolVersion "pnpm" @("-v")
$RustcV = Get-ToolVersion "rustc" @("--version")
$CargoV = Get-ToolVersion "cargo" @("--version")

$Phase = if ($PurgeOnly) { "purge_only" } else { "purge_and_rebuild" }

Write-Host "Genomics Caddy — build benchmark ($Phase)" -ForegroundColor Cyan
Write-Host "Host: $HostName  OS: $Os  Arch: $Arch"
Write-Host "CPU: $Cpu ($Cores cores)  RAM: $Ram"
Write-Host "Node: $NodeV  pnpm: $PnpmV"
Write-Host "Rust: $RustcV / $CargoV"
Write-Host "Report: $ReportTxt"
Write-Host ""

$BuildScript = Join-Path $PSScriptRoot "purge_and_build.ps1"
$BuildArgs = @()
if ($PurgeOnly) { $BuildArgs += "-PurgeOnly" }
if ($SkipChecks) { $BuildArgs += "-SkipChecks" }

$Start = Get-Date
$StartUtc = $Start.ToUniversalTime().ToString("o")

& $BuildScript @BuildArgs
if ($LASTEXITCODE -ne 0) {
    throw "benchmark target failed (exit $LASTEXITCODE)"
}

$End = Get-Date
$EndUtc = $End.ToUniversalTime().ToString("o")
$Elapsed = $End - $Start
$ElapsedSec = [math]::Round($Elapsed.TotalSeconds, 3)
$ElapsedHuman = "{0:d2}h {1:d2}m {2:00.00}s ({3}s)" -f `
    [int]$Elapsed.TotalHours, $Elapsed.Minutes, ($Elapsed.Seconds + $Elapsed.Milliseconds / 1000.0), $ElapsedSec

$Binary = Join-Path $RepoRoot "App\DNA-Tools.exe"
$BinaryBytes = 0
if (Test-Path $Binary) {
    $BinaryBytes = (Get-Item $Binary).Length
} else {
    $Binary = ""
}

$Lines = @(
    "Genomics Caddy build benchmark"
    "phase=$Phase"
    "started_utc=$StartUtc"
    "ended_utc=$EndUtc"
    "elapsed=$ElapsedHuman"
    "elapsed_seconds=$ElapsedSec"
    "host=$HostName"
    "os=$Os"
    "arch=$Arch"
    "cpu=$Cpu"
    "cores=$Cores"
    "ram=$Ram"
    "node=$NodeV"
    "pnpm=$PnpmV"
    "rustc=$RustcV"
    "cargo=$CargoV"
    "skip_checks=$([bool]$SkipChecks)"
    "binary=$Binary"
    "binary_bytes=$BinaryBytes"
    "repo=$RepoRoot"
)

$Lines | Tee-Object -FilePath $ReportTxt | ForEach-Object { Write-Host $_ }

if ($Json) {
    $payload = [ordered]@{
        phase            = $Phase
        started_utc      = $StartUtc
        ended_utc        = $EndUtc
        elapsed_seconds  = $ElapsedSec
        elapsed_human    = $ElapsedHuman
        host             = $HostName
        os               = $Os
        arch             = $Arch
        cpu              = $Cpu
        cores            = $Cores
        ram              = $Ram
        node             = $NodeV
        pnpm             = $PnpmV
        rustc            = $RustcV
        cargo            = $CargoV
        skip_checks      = [bool]$SkipChecks
        binary           = $Binary
        binary_bytes     = $BinaryBytes
        repo             = $RepoRoot
    }
    ($payload | ConvertTo-Json -Depth 4) | Set-Content -Path $ReportJson -Encoding utf8
    Write-Host "JSON: $ReportJson"
}

Write-Host ""
Write-Host "DONE — $Phase in $ElapsedHuman" -ForegroundColor Green
Write-Host "Compare reports under: $ReportDir" -ForegroundColor Green
