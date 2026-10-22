[CmdletBinding()]
param(
    [string]$Baseline,
    [string]$Candidate = 'current',
    [ValidateSet('ansi-scroll-query', 'plain-scroll', 'ansi-scroll')]
    [string[]]$Workload = @('ansi-scroll-query', 'plain-scroll', 'ansi-scroll'),
    [string]$Output,
    [switch]$ValidationOnly,
    [switch]$AllowDifferentMachine
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
. (Join-Path $PSScriptRoot 'scorecard-common.ps1')

$repoRoot = Get-RTermRepositoryRoot
$schemaPath = Join-Path $PSScriptRoot 'scorecard.schema.json'
if ([string]::IsNullOrWhiteSpace($Baseline)) {
    $Baseline = Join-Path $PSScriptRoot 'baselines\windows-x64-rust-1.89.json'
}
Test-RTermPerformanceSchema -Path $schemaPath
$baselineDocument = Read-RTermPerformanceBaseline -Path $Baseline

if ($ValidationOnly) {
    Write-RTermScorecardResult -Result ([ordered]@{
        ok = $true
        mode = 'validation-only'
        schema_version = $baselineDocument.schema_version
        baseline_commit = $baselineDocument.baseline_commit
    }) -Output $Output
    exit 0
}

if ($Candidate -ne 'current') {
    throw "unsupported candidate '$Candidate'; this runner currently measures the checked-out source as 'current'"
}

$fingerprint = Get-RTermMachineFingerprint
$fingerprintMismatches = @(
    Assert-RTermComparableMachine `
        -Baseline $baselineDocument `
        -Actual $fingerprint `
        -AllowDifferentMachine:$AllowDifferentMachine
)

function Invoke-RTermRuntimeSample {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)][string]$Name)

    $protocol = $baselineDocument.protocol
    $lines = @(
        & cargo run --locked --release -p rterm-app -- bench --json `
            --workload $Name `
            --bytes $protocol.bytes `
            --chunk-size $protocol.chunk_size `
            --render-frames $protocol.render_frames `
            --idle-ms $protocol.idle_ms
    )
    if ($LASTEXITCODE -ne 0) {
        throw "runtime benchmark '$Name' failed with exit code $LASTEXITCODE"
    }
    $jsonLine = $lines |
        Where-Object { $_.TrimStart().StartsWith('{') } |
        Select-Object -Last 1
    if ([string]::IsNullOrWhiteSpace($jsonLine)) {
        throw "runtime benchmark '$Name' did not emit a JSON report"
    }
    return $jsonLine | ConvertFrom-Json
}

function Get-RTermMedian {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)][object[]]$Values)

    if ($Values.Count -eq 0 -or ($Values.Count % 2) -eq 0) {
        throw "scorecard median requires a non-empty odd sample count"
    }
    $sorted = @($Values | Sort-Object)
    return $sorted[[math]::Floor($sorted.Count / 2)]
}

Push-Location $repoRoot
try {
    foreach ($name in $Workload) {
        for ($index = 0; $index -lt [int]$baselineDocument.protocol.warmups; $index++) {
            $null = Invoke-RTermRuntimeSample -Name $name
        }
    }

    $samples = [ordered]@{}
    foreach ($name in $Workload) {
        $samples[$name] = [System.Collections.Generic.List[object]]::new()
    }
    for ($sampleIndex = 0; $sampleIndex -lt [int]$baselineDocument.protocol.samples; $sampleIndex++) {
        $order = @($Workload)
        if (($sampleIndex % 2) -eq 1) {
            [array]::Reverse($order)
        }
        foreach ($name in $order) {
            $samples[$name].Add((Invoke-RTermRuntimeSample -Name $name))
        }
    }
} finally {
    Pop-Location
}

$violations = [System.Collections.Generic.List[object]]::new()
$workloadResults = [ordered]@{}
foreach ($name in $Workload) {
    $sampleSet = @($samples[$name])
    $median = [ordered]@{
        throughput_bytes_per_sec = [long](Get-RTermMedian @($sampleSet.throughput_bytes_per_sec))
        chunk_p95_us = [long](Get-RTermMedian @($sampleSet.chunk_p95_us))
        render_frame_p95_us = [long](Get-RTermMedian @($sampleSet.render_frame_p95_us))
        process_memory_bytes = [long](Get-RTermMedian @($sampleSet.process_memory_bytes))
        process_virtual_memory_bytes = [long](Get-RTermMedian @($sampleSet.process_virtual_memory_bytes))
        elapsed_ms = [long](Get-RTermMedian @($sampleSet.elapsed_ms))
        inspected_query_bytes = [long](Get-RTermMedian @($sampleSet.inspected_query_bytes))
        scrolled_survivor_cell_clones = [long](Get-RTermMedian @($sampleSet.scrolled_survivor_cell_clones))
        history_row_relocations = [long](Get-RTermMedian @($sampleSet.history_row_relocations))
        metadata_rebase_batches = [long](Get-RTermMedian @($sampleSet.metadata_rebase_batches))
    }
    $expected = $baselineDocument.runtime.workloads.PSObject.Properties[$name].Value
    foreach ($gate in @(
        @('throughput_bytes_per_sec', 'throughput_bytes_per_sec_min', 'min'),
        @('chunk_p95_us', 'chunk_p95_us_max', 'max'),
        @('render_frame_p95_us', 'render_frame_p95_us_max', 'max'),
        @('process_memory_bytes', 'process_memory_bytes_max', 'max'),
        @('scrolled_survivor_cell_clones', 'scrolled_survivor_cell_clones_max', 'max'),
        @('history_row_relocations', 'history_row_relocations_max', 'max'),
        @('inspected_query_bytes', 'inspected_query_bytes_max', 'max')
    )) {
        $observed = [long]$median[$gate[0]]
        $limit = [long]$expected.gates.($gate[1])
        $failed = if ($gate[2] -eq 'min') { $observed -lt $limit } else { $observed -gt $limit }
        if ($failed) {
            $violations.Add([ordered]@{
                workload = $name
                metric = $gate[0]
                observed = $observed
                expected = if ($gate[2] -eq 'min') { ">=$limit" } else { "<=$limit" }
            })
        }
    }
    $workloadResults[$name] = [ordered]@{
        baseline = $expected.baseline
        gates = $expected.gates
        samples = $sampleSet
        median = $median
    }
}

$result = [ordered]@{
    ok = $violations.Count -eq 0
    mode = 'runtime'
    schema_version = $baselineDocument.schema_version
    baseline_commit = $baselineDocument.baseline_commit
    candidate = $Candidate
    machine = $fingerprint
    machine_mismatches = $fingerprintMismatches
    protocol = $baselineDocument.protocol
    workloads = $workloadResults
    threshold_violations = @($violations)
}
Write-RTermScorecardResult -Result $result -Output $Output
if (-not $result.ok) {
    exit 1
}
