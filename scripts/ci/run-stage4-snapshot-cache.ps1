[CmdletBinding()]
param(
    [string] $Profile = 'release',
    [int] $Warmups = 2,
    [int] $Samples = 7,
    [string] $Baseline = 'scripts/perf/baselines/windows-x64-rust-1.89.json',
    [string] $Stage0Aggregate = 'artifacts/stage0-diagnostics/aggregate.json',
    [string] $OutputDirectory = 'artifacts/stage4-snapshot-cache',
    [switch] $SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

if ($Profile -ne 'release') {
    throw 'Stage 4 snapshot/cache evidence requires the locked release profile'
}
if ($Warmups -lt 1 -or $Samples -lt 1) {
    throw 'Stage 4 warmups and samples must be positive'
}

$baselineDocument = Get-Content -Raw -LiteralPath $Baseline | ConvertFrom-Json
$parserBaseline = [double]$baselineDocument.runtime.workloads.'ansi-scroll-query'.baseline.throughput_bytes_per_sec
if ($parserBaseline -le 0) {
    throw 'Stage 4 parser baseline must be positive'
}
$parserMinimum = [long][Math]::Floor($parserBaseline * 0.98)

if (-not $SkipBuild) {
    cargo build --locked --release -p rterm-app
    if ($LASTEXITCODE -ne 0) { throw "release app build failed: $LASTEXITCODE" }
}

$snapshotOutput = @(& cargo bench --locked -p rterm-render-core --bench snapshot_memory)
if ($LASTEXITCODE -ne 0) { throw "snapshot benchmark failed: $LASTEXITCODE" }
$snapshotRecords = @(
    $snapshotOutput |
        Where-Object { $_ -match '^\{"schema_version":1,' } |
        ForEach-Object { $_ | ConvertFrom-Json }
)
if ($snapshotRecords.Count -ne 2) {
    throw "snapshot benchmark must emit exactly two records; observed $($snapshotRecords.Count)"
}
foreach ($shape in @(@(80, 24), @(200, 60))) {
    $record = @($snapshotRecords | Where-Object {
        $_.columns -eq $shape[0] -and $_.rows -eq $shape[1]
    })
    if ($record.Count -ne 1) {
        throw "snapshot benchmark missing $($shape[0])x$($shape[1])"
    }
    if (
        [long]$record[0].full_mean_ns -le 0 -or
        [long]$record[0].damage_mean_ns -le 0 -or
        [long]$record[0].active_snapshot_bytes -le 0 -or
        [long]$record[0].row_reuse_permille -le 0
    ) {
        throw "snapshot benchmark returned invalid metrics for $($shape[0])x$($shape[1])"
    }
}

$targetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    'target'
} else {
    $env:CARGO_TARGET_DIR
}
$binary = Join-Path $targetRoot "$Profile/rterm.exe"
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    throw "release benchmark binary is missing: $binary"
}

function Invoke-ParserBenchmark {
    $json = & $binary bench --json --workload ansi-scroll-query --bytes 1048576 --chunk-size 8192 --render-frames 30 --idle-ms 1000
    if ($LASTEXITCODE -ne 0) { throw "parser benchmark failed: $LASTEXITCODE" }
    $report = $json | ConvertFrom-Json
    if (-not $report.ok -or [long]$report.throughput_bytes_per_sec -le 0) {
        throw 'parser benchmark returned an invalid report'
    }
    return $report
}

for ($index = 0; $index -lt $Warmups; $index++) {
    $null = Invoke-ParserBenchmark
}
$throughput = @()
for ($index = 0; $index -lt $Samples; $index++) {
    $throughput += [long](Invoke-ParserBenchmark).throughput_bytes_per_sec
}
$ordered = @($throughput | Sort-Object)
$parserMedian = [long]$ordered[[int][Math]::Floor($ordered.Count / 2)]
if ($parserMedian -lt $parserMinimum) {
    throw "Stage 4 parser throughput $parserMedian is below 98% baseline $parserMinimum"
}

$memory = $null
if (Test-Path -LiteralPath $Stage0Aggregate -PathType Leaf) {
    $aggregate = Get-Content -Raw -LiteralPath $Stage0Aggregate | ConvertFrom-Json
    $emptyBytes = [long]$aggregate.scenarios.'empty-window'.memory_p95_bytes
    $sshBytes = [long]$aggregate.scenarios.ssh1.memory_p95_bytes
    $emptyBaseline = 312672256L
    $sshBaseline = 57421824L
    $memoryNoiseToleranceRatio = 0.01
    $emptyMaximum = [long][Math]::Ceiling($emptyBaseline * (1.0 + $memoryNoiseToleranceRatio))
    $sshMaximum = [long][Math]::Ceiling($sshBaseline * (1.0 + $memoryNoiseToleranceRatio))
    if ($emptyBytes -gt $emptyMaximum -or $sshBytes -gt $sshMaximum) {
        throw "Stage 4 memory trend is outside the 1% sampling noise band: empty=$emptyBytes/$emptyBaseline/$emptyMaximum ssh1=$sshBytes/$sshBaseline/$sshMaximum"
    }
    $memory = [ordered]@{
        memory_noise_tolerance_ratio = $memoryNoiseToleranceRatio
        empty_window_p95_bytes = $emptyBytes
        empty_window_baseline_bytes = $emptyBaseline
        empty_window_maximum_bytes = $emptyMaximum
        ssh1_p95_bytes = $sshBytes
        ssh1_baseline_bytes = $sshBaseline
        ssh1_maximum_bytes = $sshMaximum
    }
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$report = [ordered]@{
    schema = 'rterm.stage4-snapshot-cache/v1'
    ok = $true
    snapshot_benchmarks = $snapshotRecords
    parser = [ordered]@{
        baseline_bytes_per_sec = [long]$parserBaseline
        minimum_bytes_per_sec = $parserMinimum
        median_bytes_per_sec = $parserMedian
        ratio = $parserMedian / $parserBaseline
    }
    memory = $memory
}
$reportPath = Join-Path $OutputDirectory 'report.json'
$report | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $reportPath -Encoding utf8NoBOM
$report | ConvertTo-Json -Depth 12 -Compress
