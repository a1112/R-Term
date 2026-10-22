[CmdletBinding()]
param(
    [ValidateSet("debug", "release")]
    [string] $Profile = "release",
    [ValidateRange(0, 100)]
    [int] $Warmups = 5,
    [ValidateRange(1, 1000)]
    [int] $Samples = 30,
    [string] $OutputDirectory = "artifacts/gpu-backend-memory-matrix",
    [switch] $SkipBuild,
    [string] $AppPath,
    [string] $LauncherPath
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
$report_only_target_bytes = 47185920 # 45 MiB.
$probes = @("cpu", "dx12", "vulkan", "gl")
$profileDirectory = if ($Profile -eq "release") { "release" } else { "debug" }
$executableSuffix = if ($IsWindows) { ".exe" } else { "" }
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$targetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    Join-Path $repoRoot "target"
} elseif ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
    [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $repoRoot $env:CARGO_TARGET_DIR))
}
$hasAppOverride = -not [string]::IsNullOrWhiteSpace($AppPath)
$hasLauncherOverride = -not [string]::IsNullOrWhiteSpace($LauncherPath)
$binarySource = if ($hasAppOverride) { "override" } else { "cargo-target" }
$certificationEligible = (
    -not $hasAppOverride -and
    $Profile -ceq "release" -and
    $Warmups -ge 5 -and
    $Samples -ge 30
)
if (($hasAppOverride -or $hasLauncherOverride) -and -not $SkipBuild) {
    throw "path overrides require -SkipBuild"
}
if ($hasAppOverride -ne $hasLauncherOverride) {
    throw "both -AppPath and -LauncherPath must be provided together"
}
if ($hasAppOverride) {
    if (-not (Test-Path -LiteralPath $AppPath -PathType Leaf)) {
        throw "-AppPath must identify an existing file"
    }
    if (-not (Test-Path -LiteralPath $LauncherPath -PathType Leaf)) {
        throw "-LauncherPath must identify an existing file"
    }
    $app = (Resolve-Path -LiteralPath $AppPath).Path
    $launcher = (Resolve-Path -LiteralPath $LauncherPath).Path
} else {
    $app = Join-Path (Join-Path $targetRoot $profileDirectory) "rterm$executableSuffix"
    $launcher = Join-Path (Join-Path $targetRoot $profileDirectory) "rterm-bench-launcher$executableSuffix"
}
$rawDirectory = Join-Path $OutputDirectory "raw"
$aggregatePath = Join-Path $OutputDirectory "aggregate.json"

if (Test-Path -LiteralPath $OutputDirectory) {
    if (-not (Test-Path -LiteralPath $OutputDirectory -PathType Container)) {
        throw "OutputDirectory must be a directory"
    }
    if ($null -ne (Get-ChildItem -LiteralPath $OutputDirectory -Force | Select-Object -First 1)) {
        throw "OutputDirectory must be empty before collection"
    }
} else {
    New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
}

if (-not $SkipBuild) {
    $profileArguments = @()
    if ($Profile -eq "release") {
        $profileArguments += "--release"
    }
    Push-Location $repoRoot
    try {
        cargo build --locked -p rterm-app @profileArguments
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for rterm-app with exit code $LASTEXITCODE"
        }
        cargo build --locked -p rterm-diagnostics --bin rterm-bench-launcher @profileArguments
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for rterm-bench-launcher with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}
if (-not (Test-Path -LiteralPath $app -PathType Leaf)) {
    throw "rterm-app executable is missing from the selected Cargo target/profile"
}
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) {
    throw "rterm-bench-launcher executable is missing from the selected Cargo target/profile"
}

New-Item -ItemType Directory -Force -Path $rawDirectory | Out-Null
$previousScale = $env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR
$env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR = "1"

function Get-NearestRankPercentile([UInt64[]] $Values, [double] $Percentile) {
    if ($Values.Count -eq 0) {
        throw "nearest-rank percentile requires at least one value"
    }
    [UInt64[]] $ordered = @($Values | ForEach-Object { [UInt64] $_ } | Sort-Object)
    [int] $rank = [Math]::Ceiling($Percentile * $ordered.Count)
    [int] $index = [Math]::Max(0, $rank - 1)
    return [UInt64] $ordered[$index]
}

function Try-GetJsonUInt64Scalar(
    [AllowNull()]
    [object] $Value,
    [ref] $Result
) {
    if ($null -eq $Value) {
        return $false
    }
    $isIntegerScalar = $Value.GetType() -in @(
        [byte],
        [sbyte],
        [Int16],
        [UInt16],
        [Int32],
        [UInt32],
        [Int64],
        [UInt64]
    )
    if (-not $isIntegerScalar) {
        return $false
    }
    try {
        $Result.Value = [Convert]::ToUInt64($Value)
        return $true
    }
    catch {
        return $false
    }
}

function Assert-JsonUInt64Equals(
    [AllowNull()]
    [object] $Value,
    [UInt64] $Expected,
    [string] $Field
) {
    [UInt64] $actual = 0
    if (
        -not (Try-GetJsonUInt64Scalar -Value $Value -Result ([ref] $actual)) -or
        $actual -ne $Expected
    ) {
        throw "$Field must be the JSON integer $Expected"
    }
}

function Test-ProductionAdapterType(
    [AllowNull()]
    [object] $Value
) {
    if ($Value -isnot [string]) {
        return $false
    }
    return (
        $Value -ceq "other" -or
        $Value -ceq "integrated-gpu" -or
        $Value -ceq "discrete-gpu" -or
        $Value -ceq "virtual-gpu" -or
        $Value -ceq "cpu"
    )
}

function Get-SafeFailureMessage([System.Management.Automation.ErrorRecord] $Failure) {
    $message = $Failure.Exception.Message
    $pathsToRedact = @(
        $repoRoot,
        $targetRoot,
        $OutputDirectory,
        $AppPath,
        $LauncherPath,
        $app,
        $launcher
    )
    foreach ($path in $pathsToRedact) {
        if (-not [string]::IsNullOrWhiteSpace($path)) {
            $message = $message.Replace($path, "[path]")
        }
    }
    return $message
}

function Assert-ProbeRecord([string] $Probe, [object] $Record, [int] $ExitCode) {
    if ($Record.schema -isnot [string] -or $Record.schema -cne "rterm.diagnostics/v2") {
        throw "unexpected diagnostics schema '$($Record.schema)'"
    }
    if (
        $ExitCode -ne 0 -or
        $Record.failures.Count -ne 0 -or
        $Record.readiness.status -isnot [string] -or
        $Record.readiness.status -cne "ready"
    ) {
        $details = @($Record.failures | ForEach-Object { "$($_.code) [$($_.phase)]: $($_.message)" })
        if ($details.Count -eq 0) {
            $details = @("exit code $ExitCode; readiness=$($Record.readiness.status)")
        }
        throw "probe run failed: $($details -join '; ')"
    }
    Assert-JsonUInt64Equals -Value $Record.configuration.stabilization_ms -Expected 5000 -Field "configuration.stabilization_ms"
    Assert-JsonUInt64Equals -Value $Record.configuration.sample_interval_ms -Expected 100 -Field "configuration.sample_interval_ms"
    Assert-JsonUInt64Equals -Value $Record.configuration.sample_count -Expected 10 -Field "configuration.sample_count"
    Assert-JsonUInt64Equals -Value $Record.configuration.columns -Expected 80 -Field "configuration.columns"
    Assert-JsonUInt64Equals -Value $Record.configuration.rows -Expected 24 -Field "configuration.rows"
    Assert-JsonUInt64Equals -Value $Record.configuration.scale_factor_milli -Expected 1000 -Field "configuration.scale_factor_milli"
    if (
        $Record.memory.metric -isnot [string] -or
        $Record.memory.metric -cne "windows_private_working_set_bytes" -or
        $Record.memory.unit -isnot [string] -or
        $Record.memory.unit -cne "bytes" -or
        $Record.memory.samples.Count -ne 10
    ) {
        throw "memory schema/sample count mismatch"
    }
    for ($index = 0; $index -lt 10; $index++) {
        $sample = $Record.memory.samples[$index]
        if (-not ($sample.PSObject.Properties.Name -contains "sequence")) {
            throw "memory sample $index is missing sequence"
        }
        if (-not ($sample.PSObject.Properties.Name -contains "elapsed_ms")) {
            throw "memory sample $index is missing elapsed_ms"
        }
        if (-not ($sample.PSObject.Properties.Name -contains "bytes")) {
            throw "memory sample $index is missing bytes"
        }
        [UInt64] $sequence = 0
        if (
            -not (Try-GetJsonUInt64Scalar -Value $sample.sequence -Result ([ref] $sequence)) -or
            $sequence -ne [UInt64] $index
        ) {
            throw "memory sample sequence must be an unsigned JSON integer scalar equal to index $index"
        }
        [UInt64] $elapsedMs = 0
        if (-not (Try-GetJsonUInt64Scalar -Value $sample.elapsed_ms -Result ([ref] $elapsedMs))) {
            throw "memory sample elapsed_ms must be an unsigned JSON integer scalar at index $index"
        }
        [UInt64] $bytes = 0
        if (
            -not (Try-GetJsonUInt64Scalar -Value $sample.bytes -Result ([ref] $bytes)) -or
            $bytes -eq 0
        ) {
            throw "memory sample bytes must be a positive unsigned JSON integer scalar at index $index"
        }
    }

    if ($Probe -eq "cpu") {
        if (
            $Record.configuration.requested_renderer -isnot [string] -or
            $Record.configuration.requested_renderer -cne "cpu"
        ) {
            throw "requested renderer mismatch for CPU probe"
        }
        if ($Record.configuration.PSObject.Properties.Name -contains "requested_gpu_backend") {
            throw "CPU probe unexpectedly requested a GPU backend"
        }
        if ($Record.renderer.final -isnot [string] -or $Record.renderer.final -cne "cpu") {
            throw "final renderer mismatch: CPU probe observed '$($Record.renderer.final)'"
        }
        foreach ($identityField in @(
            "backend",
            "adapter_name",
            "adapter_vendor_id",
            "adapter_device_id",
            "adapter_type"
        )) {
            if ($Record.renderer.PSObject.Properties.Name -contains $identityField) {
                throw "CPU probe unexpectedly reported GPU identity field '$identityField'"
            }
        }
        return
    }

    if (
        $Record.configuration.requested_renderer -isnot [string] -or
        $Record.configuration.requested_renderer -cne "auto"
    ) {
        throw "requested renderer mismatch for GPU probe '$Probe'"
    }
    if (
        $Record.configuration.requested_gpu_backend -isnot [string] -or
        $Record.configuration.requested_gpu_backend -cne $Probe
    ) {
        throw "requested GPU backend mismatch: requested '$Probe', recorded '$($Record.configuration.requested_gpu_backend)'"
    }
    if ($Record.renderer.final -isnot [string] -or $Record.renderer.final -cne "gpu") {
        throw "final renderer mismatch: GPU probe '$Probe' observed '$($Record.renderer.final)'"
    }
    if ($Record.renderer.backend -isnot [string] -or $Record.renderer.backend -cne $Probe) {
        throw "actual GPU backend mismatch: requested '$Probe', observed '$($Record.renderer.backend)'"
    }
    if (
        -not ($Record.renderer.PSObject.Properties.Name -contains "adapter_name") -or
        $Record.renderer.adapter_name -isnot [string] -or
        [string]::IsNullOrWhiteSpace($Record.renderer.adapter_name)
    ) {
        throw "GPU probe '$Probe' adapter_name must be an actual non-empty string"
    }
    if (
        -not ($Record.renderer.PSObject.Properties.Name -contains "adapter_type") -or
        -not (Test-ProductionAdapterType -Value $Record.renderer.adapter_type)
    ) {
        throw "GPU probe '$Probe' adapter_type must match a production adapter type"
    }
    foreach ($identityField in @("adapter_vendor_id", "adapter_device_id")) {
        [UInt64] $identityValue = 0
        if (
            -not ($Record.renderer.PSObject.Properties.Name -contains $identityField) -or
            -not (Try-GetJsonUInt64Scalar -Value $Record.renderer.$identityField -Result ([ref] $identityValue)) -or
            $identityValue -gt [UInt32]::MaxValue
        ) {
            throw "GPU probe '$Probe' $identityField must be an unsigned JSON integer scalar within UInt32"
        }
    }
}

function Assert-ConsistentGpuIdentity([string] $Probe, [object[]] $Records) {
    if ($Probe -eq "cpu" -or $Records.Count -lt 2) {
        return
    }
    $expected = $Records[0].renderer
    for ($index = 1; $index -lt $Records.Count; $index++) {
        $actual = $Records[$index].renderer
        if (
            $actual.backend -cne $expected.backend -or
            $actual.adapter_name -cne $expected.adapter_name -or
            $actual.adapter_vendor_id -ne $expected.adapter_vendor_id -or
            $actual.adapter_device_id -ne $expected.adapter_device_id -or
            $actual.adapter_type -cne $expected.adapter_type
        ) {
            throw "GPU identity drift for probe '$Probe' between measured runs 1 and $($index + 1)"
        }
    }
}

function Invoke-GpuBackendMemoryRun(
    [string] $Probe,
    [AllowNull()]
    [string] $RawPath
) {
    $launcherArguments = @(
        "--app", $app,
        "--scenario", "empty-window",
        "--stabilization-ms", "5000",
        "--sample-interval-ms", "100",
        "--sample-count", "10",
        "--cols", "80",
        "--rows", "24",
        "--json"
    )
    if ($Probe -eq "cpu") {
        $launcherArguments += @("--renderer", "cpu")
    } else {
        $launcherArguments += @("--renderer", "auto", "--gpu-backend", $Probe)
    }

    $jsonLines = @(& $launcher @launcherArguments)
    $exitCode = $LASTEXITCODE
    $json = $jsonLines -join [Environment]::NewLine
    try {
        $record = $json | ConvertFrom-Json
    }
    catch {
        throw "launcher output was not valid JSON (exit code $exitCode)"
    }
    if (-not [string]::IsNullOrWhiteSpace($RawPath)) {
        $record | ConvertTo-Json -Depth 20 -Compress | Set-Content -LiteralPath $RawPath -Encoding utf8NoBOM
    }
    Assert-ProbeRecord -Probe $Probe -Record $record -ExitCode $exitCode
    return $record
}

try {
    $probeReports = [System.Collections.Generic.List[object]]::new()
    $rawFiles = [System.Collections.Generic.List[string]]::new()
    foreach ($probe in $probes) {
        $records = [System.Collections.Generic.List[object]]::new()
        $probeFailure = $null
        $completedRuns = 0
        try {
            for ($index = 0; $index -lt $Warmups; $index++) {
                $null = Invoke-GpuBackendMemoryRun -Probe $probe -RawPath $null
            }
            for ($index = 0; $index -lt $Samples; $index++) {
                $rawName = "raw/{0}-{1:D2}.json" -f $probe, ($index + 1)
                $rawPath = Join-Path $OutputDirectory $rawName
                try {
                    $record = Invoke-GpuBackendMemoryRun -Probe $probe -RawPath $rawPath
                }
                finally {
                    if (Test-Path -LiteralPath $rawPath -PathType Leaf) {
                        $rawFiles.Add($rawName)
                    }
                }
                $records.Add($record)
                $completedRuns++
            }
            Assert-ConsistentGpuIdentity -Probe $probe -Records @($records)
        }
        catch {
            $probeFailure = Get-SafeFailureMessage $_
        }

        $requestedRenderer = if ($probe -eq "cpu") { "cpu" } else { "auto" }
        if ($null -ne $probeFailure) {
            $failedReport = [ordered]@{
                name = $probe
                status = "failed"
                requested_renderer = $requestedRenderer
                measured_runs_completed = $completedRuns
                samples_per_run = 10
                probe_failure = [ordered]@{
                    message = $probeFailure
                }
            }
            if ($probe -ne "cpu") {
                $failedReport["requested_gpu_backend"] = $probe
            }
            $probeReports.Add($failedReport)
            Write-Warning "GPU backend memory probe '$probe' failed: $probeFailure"
            continue
        }

        # Residence percentiles are defined over one representative per cold
        # process.  Flattening all 300 samples would let a noisy process
        # contribute thirty times and is explicitly forbidden by the Stage 7
        # evidence contract.  The maximum still comes from every raw sample.
        [UInt64[]] $representatives = @(
            $records |
                ForEach-Object {
                    [UInt64[]] $sampleValues = @($_.memory.samples | ForEach-Object { [UInt64] $_.bytes })
                    Get-NearestRankPercentile -Values $sampleValues -Percentile 0.50
                }
        )
        [UInt64[]] $bytes = @(
            $records |
                ForEach-Object { $_.memory.samples } |
                ForEach-Object { [UInt64] $_.bytes }
        )
        $p50 = Get-NearestRankPercentile -Values $representatives -Percentile 0.50
        $p95 = Get-NearestRankPercentile -Values $representatives -Percentile 0.95
        $maximum = [UInt64] ($bytes | Measure-Object -Maximum).Maximum
        $firstRecord = $records[0]
        $successfulReport = [ordered]@{
            name = $probe
            status = "succeeded"
            requested_renderer = $requestedRenderer
            final_renderer = $firstRecord.renderer.final
            measured_runs = $Samples
            samples_per_run = 10
            memory_metric = $firstRecord.memory.metric
            memory_p50_bytes = $p50
            memory_p95_bytes = $p95
            memory_max_bytes = $maximum
            representatives = @($representatives)
            raw_sample_count = $bytes.Count
            aggregation = [ordered]@{
                process_representative = "nearest-rank-p50"
                cross_process_percentiles = "nearest-rank over per-process representatives"
                maximum = "raw-maximum"
                flattening_for_percentiles = "forbidden"
            }
            report_only_target_bytes = $report_only_target_bytes
            report_only_target_met = ($p95 -le $report_only_target_bytes)
            evidence = [ordered]@{
                raw_pattern = "raw/$probe-NN.json"
            }
        }
        if ($probe -ne "cpu") {
            $successfulReport["requested_gpu_backend"] = $probe
            $successfulReport["actual_gpu_backend"] = $firstRecord.renderer.backend
            $successfulReport["adapter_name"] = $firstRecord.renderer.adapter_name
            $successfulReport["adapter_vendor_id"] = $firstRecord.renderer.adapter_vendor_id
            $successfulReport["adapter_device_id"] = $firstRecord.renderer.adapter_device_id
            $successfulReport["adapter_type"] = $firstRecord.renderer.adapter_type
        }
        $probeReports.Add($successfulReport)
        if ($p95 -gt $report_only_target_bytes) {
            Write-Warning "Report-only memory observation: $probe p95=$p95 target=$report_only_target_bytes"
        }
    }

    $aggregate = [ordered]@{
        schema = "rterm.diagnostics/gpu-backend-memory-matrix-v1"
        profile = $Profile
        binary_source = $binarySource
        certification_eligible = $certificationEligible
        warmups = $Warmups
        measured_runs = $Samples
        geometry = [ordered]@{
            columns = 80
            rows = 24
            benchmark_scale_factor = 1.0
        }
        sampling = [ordered]@{
            stabilization_ms = 5000
            interval_ms = 100
            count_per_run = 10
        }
        memory = [ordered]@{
            unit = "bytes"
            report_only_target_bytes = $report_only_target_bytes
        }
        thresholds = "report-only"
        probes = @($probeReports)
        evidence = [ordered]@{
            raw_directory = "raw"
            raw_files = @($rawFiles)
            aggregate = "aggregate.json"
        }
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
