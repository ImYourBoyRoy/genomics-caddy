#requires -Version 7.0
<#
Inspect-GenomicsRefs.ps1

Purpose:
  Safely inspect huge ClinVar/dbSNP reference files without loading them into memory.

Outputs:
  - genomics_reference_insight.md
  - genomics_reference_insight.json

Examples:
  pwsh .\Inspect-GenomicsRefs.ps1 `
    -ClinVarTxt "D:\genomics\clinvar\variant_summary.txt" `
    -DbSnpJson "D:\genomics\dbsnp\refsnp-merged.json" `
    -OutDir ".\genomics_ref_inspection"

  pwsh .\Inspect-GenomicsRefs.ps1 `
    -ClinVarTxt "D:\genomics\clinvar\variant_summary.txt" `
    -DbSnpJson "D:\genomics\dbsnp\refsnp-merged.json" `
    -OutDir ".\genomics_ref_inspection" `
    -CountLines
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $false)]
    [string]$ClinVarTxt,

    [Parameter(Mandatory = $false)]
    [string]$DbSnpJson,

    [Parameter(Mandatory = $false)]
    [string]$OutDir = ".\genomics_ref_inspection",

    [Parameter(Mandatory = $false)]
    [int]$SampleLines = 25,

    [Parameter(Mandatory = $false)]
    [int]$SampleBytes = 1048576,

    [Parameter(Mandatory = $false)]
    [int]$TailBytes = 65536,

    [Parameter(Mandatory = $false)]
    [switch]$CountLines
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function New-DirectoryIfMissing {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        New-Item -ItemType Directory -Path $Path | Out-Null
    }
}

function Format-ByteSize {
    param([long]$Bytes)

    if ($Bytes -ge 1TB) { return "{0:N2} TB" -f ($Bytes / 1TB) }
    if ($Bytes -ge 1GB) { return "{0:N2} GB" -f ($Bytes / 1GB) }
    if ($Bytes -ge 1MB) { return "{0:N2} MB" -f ($Bytes / 1MB) }
    if ($Bytes -ge 1KB) { return "{0:N2} KB" -f ($Bytes / 1KB) }
    return "$Bytes B"
}

function Get-FileBasics {
    param([string]$Path)

    $item = Get-Item -LiteralPath $Path
    [ordered]@{
        path               = $item.FullName
        exists             = $true
        length_bytes       = $item.Length
        length_human       = Format-ByteSize $item.Length
        created_utc        = $item.CreationTimeUtc.ToString("o")
        modified_utc       = $item.LastWriteTimeUtc.ToString("o")
        extension          = $item.Extension
        name               = $item.Name
    }
}

function Read-FirstTextBytes {
    param(
        [string]$Path,
        [int]$Bytes = 1048576
    )

    $fs = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
    try {
        $fileLength = [int64]$fs.Length
        $requestedBytes = [int64]$Bytes
        $bufferSize64 = [Math]::Min($requestedBytes, $fileLength)
        $bufferSize = [int]$bufferSize64

        $buffer = New-Object byte[] $bufferSize
        [void]$fs.Read($buffer, 0, $bufferSize)

        return [System.Text.Encoding]::UTF8.GetString($buffer)
    }
    finally {
        $fs.Dispose()
    }
}

function Read-TailTextBytes {
    param(
        [string]$Path,
        [int]$Bytes = 65536
    )

    $fs = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
    try {
        $fileLength = [int64]$fs.Length
        $requestedBytes = [int64]$Bytes
        $readBytes64 = [Math]::Min($requestedBytes, $fileLength)
        $readBytes = [int]$readBytes64

        if ($readBytes -le 0) {
            return ""
        }

        [void]$fs.Seek(-1L * [int64]$readBytes, [System.IO.SeekOrigin]::End)

        $buffer = New-Object byte[] $readBytes
        [void]$fs.Read($buffer, 0, $readBytes)

        return [System.Text.Encoding]::UTF8.GetString($buffer)
    }
    finally {
        $fs.Dispose()
    }
}

function Get-FirstLinesSafe {
    param(
        [string]$Path,
        [int]$LineCount = 25
    )

    $lines = New-Object System.Collections.Generic.List[string]
    $sr = [System.IO.StreamReader]::new($Path, [System.Text.Encoding]::UTF8, $true, 65536)
    try {
        while (-not $sr.EndOfStream -and $lines.Count -lt $LineCount) {
            $lines.Add($sr.ReadLine())
        }
    }
    finally {
        $sr.Dispose()
    }

    return $lines.ToArray()
}

function Count-LinesStreaming {
    param([string]$Path)

    $count = 0L
    $sr = [System.IO.StreamReader]::new($Path, [System.Text.Encoding]::UTF8, $true, 1048576)
    try {
        while (-not $sr.EndOfStream) {
            [void]$sr.ReadLine()
            $count++
        }
    }
    finally {
        $sr.Dispose()
    }

    return $count
}

function Count-CharInString {
    param(
        [AllowNull()]
        [string]$Text,

        [Parameter(Mandatory = $true)]
        [char]$Char
    )

    if ($null -eq $Text) { return 0 }

    $count = 0
    foreach ($c in $Text.ToCharArray()) {
        if ($c -eq $Char) { $count++ }
    }

    return $count
}

function Get-SafeCount {
    param($Value)

    if ($null -eq $Value) { return 0 }
    return @($Value).Count
}

function Get-DelimitedTextInsight {
    param(
        [string]$Path,
        [int]$SampleLines = 25
    )

    $lines = @(Get-FirstLinesSafe -Path $Path -LineCount $SampleLines)
    $nonEmpty = @($lines | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })

    if ((Get-SafeCount $nonEmpty) -eq 0) {
        return [ordered]@{
            type  = "delimited_text"
            error = "No non-empty lines found."
        }
    }

    $header = [string]$nonEmpty[0]

    $delimiterCandidates = [ordered]@{
        tab       = [char]9
        comma     = [char]44
        pipe      = [char]124
        semicolon = [char]59
    }

    $delimiterStats = [ordered]@{}

    foreach ($name in $delimiterCandidates.Keys) {
        $delimChar = $delimiterCandidates[$name]

        $counts = @()
        foreach ($line in $nonEmpty) {
            $counts += (Count-CharInString -Text ([string]$line) -Char $delimChar)
        }

        $min = ($counts | Measure-Object -Minimum).Minimum
        $max = ($counts | Measure-Object -Maximum).Maximum
        $avg = ($counts | Measure-Object -Average).Average

        $delimiterStats[$name] = [ordered]@{
            delimiter_codepoint = [int]$delimChar
            header_count        = Count-CharInString -Text $header -Char $delimChar
            sample_min          = $min
            sample_max          = $max
            sample_average      = [Math]::Round($avg, 2)
        }
    }

    $bestDelimiterName = (
        $delimiterStats.GetEnumerator() |
        Sort-Object { $_.Value.header_count } -Descending |
        Select-Object -First 1
    ).Key

    $bestDelimiterChar = $delimiterCandidates[$bestDelimiterName]
    $escapedDelimiter = [regex]::Escape([string]$bestDelimiterChar)

    $columns = @($header -split $escapedDelimiter)
    $sampleRows = @()

    $rowSampleCount = [Math]::Min(5, [Math]::Max(0, (Get-SafeCount $nonEmpty) - 1))
    $sampleDataLines = @($nonEmpty | Select-Object -Skip 1 -First $rowSampleCount)

    foreach ($line in $sampleDataLines) {
        $parts = @(([string]$line) -split $escapedDelimiter)
        $row = [ordered]@{}

        $limit = [Math]::Min((Get-SafeCount $columns), (Get-SafeCount $parts))
        for ($i = 0; $i -lt $limit; $i++) {
            $row[[string]$columns[$i]] = [string]$parts[$i]
        }

        $sampleRows += $row
    }

    $interestingColumns = @($columns | Where-Object {
        $_ -match 'rs|RS|dbSNP|variation|Variation|Allele|Gene|Clinical|Review|Condition|Assembly|Chromosome|Start|Stop|Reference|Alternate|Phenotype|Submitter|Last'
    })

    return [ordered]@{
        type                   = "delimited_text"
        likely_delimiter_name  = $bestDelimiterName
        column_count           = Get-SafeCount $columns
        columns                = $columns
        interesting_columns    = $interestingColumns
        delimiter_stats        = $delimiterStats
        first_lines            = $lines
        sample_rows_as_objects = $sampleRows
    }
}

function Get-JsonKeyFrequencyFromSample {
    param([string]$Text)

    $matches = [regex]::Matches($Text, '"(?<key>[A-Za-z0-9_\.\-\:\*\+\/]+)"\s*:')
    $counts = @{}

    foreach ($m in $matches) {
        $key = $m.Groups["key"].Value
        if (-not $counts.ContainsKey($key)) {
            $counts[$key] = 0
        }
        $counts[$key]++
    }

    return $counts.GetEnumerator() |
        Sort-Object Value -Descending |
        Select-Object -First 100 |
        ForEach-Object {
            [ordered]@{
                key   = $_.Key
                count = $_.Value
            }
        }
}

function Try-ParseJsonLine {
    param([string]$Line)

    try {
        $obj = $Line | ConvertFrom-Json -Depth 64
        $props = @()
        if ($null -ne $obj.PSObject.Properties) {
            $props = $obj.PSObject.Properties.Name
        }

        return [ordered]@{
            success = $true
            keys    = $props
            object  = $obj
            error   = $null
        }
    }
    catch {
        return [ordered]@{
            success = $false
            keys    = @()
            object  = $null
            error   = $_.Exception.Message
        }
    }
}

function Extract-FirstBalancedJsonObjectFromText {
    param(
        [string]$Text,
        [int]$MaxChars = 2000000
    )

    $start = $Text.IndexOf("{")
    if ($start -lt 0) {
        return $null
    }

    $depth = 0
    $inString = $false
    $escape = $false
    $endLimit = [Math]::Min($Text.Length, $start + $MaxChars)

    for ($i = $start; $i -lt $endLimit; $i++) {
        $ch = $Text[$i]

        if ($escape) {
            $escape = $false
            continue
        }

        if ($ch -eq [char]92) {
            if ($inString) { $escape = $true }
            continue
        }

        if ($ch -eq '"') {
            $inString = -not $inString
            continue
        }

        if (-not $inString) {
            if ($ch -eq "{") { $depth++ }
            elseif ($ch -eq "}") {
                $depth--
                if ($depth -eq 0) {
                    return $Text.Substring($start, $i - $start + 1)
                }
            }
        }
    }

    return $null
}

function Get-JsonInsight {
    param(
        [string]$Path,
        [int]$SampleBytes = 1048576,
        [int]$SampleLines = 25,
        [int]$TailBytes = 65536
    )

    $firstText = Read-FirstTextBytes -Path $Path -Bytes $SampleBytes
    $tailText = Read-TailTextBytes -Path $Path -Bytes $TailBytes
    $firstLines = Get-FirstLinesSafe -Path $Path -LineCount $SampleLines
    $firstNonEmptyLine = ($firstLines | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1)

    $trimStart = $firstText.TrimStart()
    $firstChar = if ($trimStart.Length -gt 0) { [string]$trimStart[0] } else { $null }

    $looksLikeNdjson = $false
    $firstLineParse = $null

    if ($firstNonEmptyLine -and $firstNonEmptyLine.TrimStart().StartsWith("{") -and $firstNonEmptyLine.TrimEnd().EndsWith("}")) {
        $firstLineParse = Try-ParseJsonLine -Line $firstNonEmptyLine
        $looksLikeNdjson = [bool]$firstLineParse.success
    }

    $firstBalancedObjectText = Extract-FirstBalancedJsonObjectFromText -Text $firstText
    $firstObjectParse = $null
    if ($firstBalancedObjectText) {
        $firstObjectParse = Try-ParseJsonLine -Line $firstBalancedObjectText
    }

    $keyFreq = Get-JsonKeyFrequencyFromSample -Text $firstText

    $tailTrim = $tailText.Trim()
    $tailEndsWithArray = $tailTrim.EndsWith("]")
    $tailEndsWithObject = $tailTrim.EndsWith("}")

    return [ordered]@{
        type                         = "json_or_json_like"
        first_non_whitespace_char    = $firstChar
        likely_ndjson                = $looksLikeNdjson
        first_line_parse_success     = if ($firstLineParse) { $firstLineParse.success } else { $false }
        first_line_parse_error       = if ($firstLineParse) { $firstLineParse.error } else { $null }
        first_line_keys              = if ($firstLineParse) { $firstLineParse.keys } else { @() }
        first_object_parse_success   = if ($firstObjectParse) { $firstObjectParse.success } else { $false }
        first_object_parse_error     = if ($firstObjectParse) { $firstObjectParse.error } else { $null }
        first_object_keys            = if ($firstObjectParse) { $firstObjectParse.keys } else { @() }
        common_keys_in_first_mb      = $keyFreq
        first_lines                  = $firstLines
        tail_ends_with_json_array    = $tailEndsWithArray
        tail_ends_with_json_object   = $tailEndsWithObject
        tail_preview_last_2000_chars = if ($tailText.Length -gt 2000) { $tailText.Substring($tailText.Length - 2000) } else { $tailText }
    }
}

function Get-ReferenceFileInsight {
    param(
        [string]$Path,
        [string]$Kind
    )

    if (-not $Path) {
        return [ordered]@{
            kind   = $Kind
            status = "not_provided"
        }
    }

    if (-not (Test-Path -LiteralPath $Path)) {
        return [ordered]@{
            kind   = $Kind
            status = "missing"
            path   = $Path
        }
    }

    Write-Host "Inspecting $Kind file: $Path" -ForegroundColor Cyan

    $basics = Get-FileBasics -Path $Path
    $extension = [System.IO.Path]::GetExtension($Path).ToLowerInvariant()

    $insight = if ($extension -in @(".txt", ".tsv", ".csv")) {
        Get-DelimitedTextInsight -Path $Path -SampleLines $SampleLines
    }
    elseif ($extension -in @(".json", ".jsonl", ".ndjson")) {
        Get-JsonInsight -Path $Path -SampleBytes $SampleBytes -SampleLines $SampleLines -TailBytes $TailBytes
    }
    else {
        $firstText = Read-FirstTextBytes -Path $Path -Bytes $SampleBytes
        if ($firstText.TrimStart().StartsWith("{") -or $firstText.TrimStart().StartsWith("[")) {
            Get-JsonInsight -Path $Path -SampleBytes $SampleBytes -SampleLines $SampleLines -TailBytes $TailBytes
        }
        else {
            Get-DelimitedTextInsight -Path $Path -SampleLines $SampleLines
        }
    }

    $lineCount = $null
    if ($CountLines) {
        Write-Host "Counting lines for $Kind. This may take a while on huge files..." -ForegroundColor Yellow
        $lineCount = Count-LinesStreaming -Path $Path
    }

    return [ordered]@{
        kind       = $Kind
        status     = "inspected"
        basics     = $basics
        line_count = $lineCount
        insight    = $insight
    }
}

function ConvertTo-MarkdownReport {
    param([hashtable]$Report)

    function MdCode {
        param($Value)
        $bt = [char]96
        return "$bt$Value$bt"
    }

    $lines = New-Object System.Collections.Generic.List[string]

    $lines.Add("# Genomics Reference File Insight")
    $lines.Add("")
    $lines.Add(("Generated UTC: {0}" -f $Report.generated_utc))
    $lines.Add(("PowerShell: {0}" -f $Report.powershell_version))
    $lines.Add(("OS: {0}" -f $Report.os))
    $lines.Add("")

    foreach ($fileKey in @("clinvar", "dbsnp")) {
        $section = $Report[$fileKey]

        $lines.Add(("## {0}" -f $section.kind))
        $lines.Add("")
        $lines.Add(("Status: {0}" -f (MdCode $section.status)))
        $lines.Add("")

        if ($section.status -ne "inspected") {
            if ($section.path) {
                $lines.Add(("Path: {0}" -f (MdCode $section.path)))
            }
            $lines.Add("")
            continue
        }

        $b = $section.basics
        $lines.Add("### File")
        $lines.Add("")
        $lines.Add(("- Path: {0}" -f (MdCode $b.path)))
        $lines.Add(("- Size: {0} / {1} bytes" -f $b.length_human, $b.length_bytes))
        $lines.Add(("- Modified UTC: {0}" -f $b.modified_utc))

        if ($null -ne $section.line_count) {
            $lines.Add(("- Line count: {0}" -f $section.line_count))
        }

        $lines.Add("")

        $ins = $section.insight
        $lines.Add("### Detected structure")
        $lines.Add("")
        $lines.Add(("- Type: {0}" -f (MdCode $ins.type)))

        if ($ins.type -eq "delimited_text") {
            $lines.Add(("- Likely delimiter: {0}" -f (MdCode $ins.likely_delimiter_name)))
            $lines.Add(("- Column count: {0}" -f $ins.column_count))
            $lines.Add("")

            $lines.Add("#### Columns")
            $lines.Add("")
            foreach ($col in $ins.columns) {
                $lines.Add(("- {0}" -f (MdCode $col)))
            }

            $lines.Add("")
            $lines.Add("#### Interesting columns")
            $lines.Add("")
            foreach ($col in $ins.interesting_columns) {
                $lines.Add(("- {0}" -f (MdCode $col)))
            }

            $lines.Add("")
            $lines.Add("#### First lines")
            $lines.Add("")
            $lines.Add("~~~text")
            foreach ($line in $ins.first_lines) {
                $lines.Add($line)
            }
            $lines.Add("~~~")
            $lines.Add("")
        }
        elseif ($ins.type -eq "json_or_json_like") {
            $lines.Add(("- First non-whitespace char: {0}" -f (MdCode $ins.first_non_whitespace_char)))
            $lines.Add(("- Likely NDJSON: {0}" -f (MdCode $ins.likely_ndjson)))
            $lines.Add(("- First line parse success: {0}" -f (MdCode $ins.first_line_parse_success)))
            $lines.Add(("- First object parse success: {0}" -f (MdCode $ins.first_object_parse_success)))
            $lines.Add(("- Tail ends with JSON array: {0}" -f (MdCode $ins.tail_ends_with_json_array)))
            $lines.Add(("- Tail ends with JSON object: {0}" -f (MdCode $ins.tail_ends_with_json_object)))
            $lines.Add("")

            $lines.Add("#### First line keys")
            $lines.Add("")
            foreach ($keyName in $ins.first_line_keys) {
                $lines.Add(("- {0}" -f (MdCode $keyName)))
            }

            $lines.Add("")
            $lines.Add("#### First object keys")
            $lines.Add("")
            foreach ($keyName in $ins.first_object_keys) {
                $lines.Add(("- {0}" -f (MdCode $keyName)))
            }

            $lines.Add("")
            $lines.Add("#### Common keys in first sample")
            $lines.Add("")
            foreach ($entry in $ins.common_keys_in_first_mb) {
                $lines.Add(("- {0}: {1}" -f (MdCode $entry.key), $entry.count))
            }

            $lines.Add("")
            $lines.Add("#### First lines")
            $lines.Add("")
            $lines.Add("~~~text")
            foreach ($line in $ins.first_lines) {
                $lines.Add($line)
            }
            $lines.Add("~~~")
            $lines.Add("")

            $lines.Add("#### Tail preview")
            $lines.Add("")
            $lines.Add("~~~text")
            $lines.Add($ins.tail_preview_last_2000_chars)
            $lines.Add("~~~")
            $lines.Add("")
        }
    }

    return ($lines -join [Environment]::NewLine)
}

New-DirectoryIfMissing -Path $OutDir

$report = [ordered]@{
    generated_utc      = (Get-Date).ToUniversalTime().ToString("o")
    powershell_version = $PSVersionTable.PSVersion.ToString()
    os                 = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
    parameters         = [ordered]@{
        clinvar_txt  = $ClinVarTxt
        dbsnp_json   = $DbSnpJson
        sample_lines = $SampleLines
        sample_bytes = $SampleBytes
        tail_bytes   = $TailBytes
        count_lines  = [bool]$CountLines
    }
    clinvar            = Get-ReferenceFileInsight -Path $ClinVarTxt -Kind "ClinVar"
    dbsnp              = Get-ReferenceFileInsight -Path $DbSnpJson -Kind "dbSNP"
}

$jsonOut = Join-Path $OutDir "genomics_reference_insight.json"
$mdOut = Join-Path $OutDir "genomics_reference_insight.md"

$report | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $jsonOut -Encoding UTF8
$md = ConvertTo-MarkdownReport -Report $report
$md | Set-Content -LiteralPath $mdOut -Encoding UTF8

Write-Host ""
Write-Host "Done." -ForegroundColor Green
Write-Host "JSON: $jsonOut"
Write-Host "Markdown: $mdOut"
Write-Host ""
Write-Host "Send me both files, or paste the markdown if it is not too large."