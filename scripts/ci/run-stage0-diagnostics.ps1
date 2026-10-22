[CmdletBinding()]
param(
    [ValidateSet("debug", "release")]
    [string] $Profile = "release",
    [ValidateRange(0, 100)]
    [int] $Warmups = 5,
    [ValidateRange(1, 1000)]
    [int] $Samples = 30,
    [string] $OutputDirectory = "artifacts/stage0-diagnostics",
    [switch] $SkipBuild
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$stage0_empty_window_target_bytes = 47185920 # 45 MiB, report-only.
$stage0_ssh1_target_bytes = 62914560 # 60 MiB, report-only.
$scenarios = @("empty-window", "ssh1")
$profileDirectory = if ($Profile -eq "release") { "release" } else { "debug" }
$executableSuffix = if ($IsWindows) { ".exe" } else { "" }
$app = Join-Path "target/$profileDirectory" "rterm$executableSuffix"
$launcher = Join-Path "target/$profileDirectory" "rterm-bench-launcher$executableSuffix"
$rawDirectory = Join-Path $OutputDirectory "raw"
$aggregatePath = Join-Path $OutputDirectory "aggregate.json"

if (-not $SkipBuild) {
    $profileArguments = @()
    if ($Profile -eq "release") {
        $profileArguments += "--release"
    }
    cargo build --locked -p rterm-app --features developer-full @profileArguments
    cargo build --locked -p rterm-diagnostics --bin rterm-bench-launcher @profileArguments
}
if (-not (Test-Path -LiteralPath $app -PathType Leaf)) {
    throw "rterm-app executable is missing: $app"
}
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) {
    throw "rterm-bench-launcher executable is missing: $launcher"
}

New-Item -ItemType Directory -Force -Path $rawDirectory | Out-Null
$previousScale = $env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR
$env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR = "1"

function Invoke-Stage0Run([string] $Scenario) {
    $launcherArguments = @(
        "--app", (Resolve-Path -LiteralPath $app).Path,
        "--scenario", $Scenario,
        "--stabilization-ms", "5000",
        "--sample-interval-ms", "100",
        "--sample-count", "10",
        "--cols", "80",
        "--rows", "24",
        "--json"
    )
    $json = & $launcher @launcherArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Stage 0 launcher failed for $Scenario with exit code $LASTEXITCODE; result: $json"
    }
    $record = $json | ConvertFrom-Json
    if ($record.schema -ne "rterm.diagnostics/v2") {
        throw "Stage 0 launcher returned an unexpected schema for ${Scenario}: $($record.schema)"
    }
    if ($record.memory.samples.Count -ne 10) {
        throw "Stage 0 launcher returned $($record.memory.samples.Count) samples for $Scenario"
    }
    return $record
}

try {
    foreach ($scenario in $scenarios) {
        for ($index = 0; $index -lt $Warmups; $index++) {
            $null = Invoke-Stage0Run $scenario
        }
    }

    $recordsByScenario = @{}
    foreach ($scenario in $scenarios) {
        $recordsByScenario[$scenario] = [System.Collections.Generic.List[object]]::new()
        for ($index = 0; $index -lt $Samples; $index++) {
            $record = Invoke-Stage0Run $scenario
            $recordsByScenario[$scenario].Add($record)
            $path = Join-Path $rawDirectory ("{0}-{1:D2}.json" -f $scenario, ($index + 1))
            $record | ConvertTo-Json -Depth 20 -Compress | Set-Content -LiteralPath $path -Encoding utf8NoBOM
        }
    }

    function Get-NearestRankP95([UInt64[]] $Values) {
        $ordered = @($Values | Sort-Object)
        $rank = [Math]::Ceiling(0.95 * $ordered.Count)
        return [UInt64] $ordered[[Math]::Max(0, $rank - 1)]
    }

    $scenarioReports = @{}
    foreach ($scenario in $scenarios) {
        [UInt64[]] $bytes = @(
            $recordsByScenario[$scenario] |
                ForEach-Object { $_.memory.samples } |
                ForEach-Object { [UInt64] $_.bytes }
        )
        $p95 = Get-NearestRankP95 $bytes
        $target = if ($scenario -eq "empty-window") {
            $stage0_empty_window_target_bytes
        } else {
            $stage0_ssh1_target_bytes
        }
        $scenarioReports[$scenario] = [ordered]@{
            measured_runs = $Samples
            samples_per_run = 10
            memory_metric = $recordsByScenario[$scenario][0].memory.metric
            memory_p95_bytes = $p95
            report_only_target_bytes = $target
            report_only_target_met = ($p95 -le $target)
        }
        if ($p95 -gt $target) {
            Write-Warning "Stage 0 report-only memory observation: $scenario p95=$p95 target=$target"
        }
    }

    $aggregate = [ordered]@{
        schema = "rterm.diagnostics/aggregate-v1"
        profile = $Profile
        warmups = $Warmups
        measured_runs = $Samples
        columns = 80
        rows = 24
        scale_factor = 1.0
        thresholds = "report-only"
        scenarios = $scenarioReports
    }
    $aggregate | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $aggregatePath -Encoding utf8NoBOM
    $aggregate | ConvertTo-Json -Depth 20 -Compress
}
finally {
    if ($null -eq $previousScale) {
        Remove-Item Env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR -ErrorAction SilentlyContinue
    } else {
        $env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR = $previousScale
    }
}
