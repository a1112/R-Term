param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineExecutable,
    [Parameter(Mandatory = $true)]
    [string]$CandidateExecutable,
    [int]$Warmups = 2,
    [int]$Samples = 7,
    [string]$Output = "artifacts/perf/task19-native-window.json"
)

$ErrorActionPreference = "Stop"

function Invoke-NativeWindowProbe([string]$Executable) {
    $previous = $env:RTERM_TEST_APP_EXECUTABLE
    $env:RTERM_TEST_APP_EXECUTABLE = $Executable
    try {
        $previousErrorPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $lines = & cargo test --locked -p rterm-app --test native_window_e2e `
            native_window_release_performance_probe -- --ignored --exact --nocapture 2>&1
        $ErrorActionPreference = $previousErrorPreference
        if ($LASTEXITCODE -ne 0) {
            throw "native-window probe failed for ${Executable}:`n$($lines -join [Environment]::NewLine)"
        }
        $prefix = "RTERM_NATIVE_RELEASE_PROBE="
        $record = $lines | Where-Object { $_.ToString().StartsWith($prefix) } | Select-Object -Last 1
        if ($null -eq $record) {
            throw "native-window probe emitted no metrics for $Executable"
        }
        return $record.ToString().Substring($prefix.Length) | ConvertFrom-Json
    }
    finally {
        $ErrorActionPreference = "Stop"
        $env:RTERM_TEST_APP_EXECUTABLE = $previous
    }
}

function Get-Median([object[]]$Values) {
    $ordered = @($Values | Sort-Object)
    return $ordered[[int][Math]::Floor($ordered.Count / 2)]
}

function Get-Medians([object[]]$Records) {
    $metrics = @(
        "elapsed_us",
        "first_pty_byte_ms",
        "first_rendered_cell_ms",
        "pty_chunk_process_p95_us",
        "render_frame_p95_us",
        "input_write_p95_us"
    )
    $result = [ordered]@{}
    foreach ($name in $metrics) {
        if ($name -eq "elapsed_us") {
            $values = @($Records | ForEach-Object { [uint64]$_.elapsed_us })
        }
        else {
            $values = @($Records | ForEach-Object { [uint64]$_.metrics.$name })
        }
        $result[$name] = Get-Median $values
    }
    return $result
}

foreach ($executable in @($BaselineExecutable, $CandidateExecutable)) {
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "release executable does not exist: $executable"
    }
}

for ($index = 0; $index -lt $Warmups; $index++) {
    $null = Invoke-NativeWindowProbe $BaselineExecutable
    $null = Invoke-NativeWindowProbe $CandidateExecutable
}

$baseline = [Collections.Generic.List[object]]::new()
$candidate = [Collections.Generic.List[object]]::new()
for ($index = 0; $index -lt $Samples; $index++) {
    if (($index % 2) -eq 0) {
        $baseline.Add((Invoke-NativeWindowProbe $BaselineExecutable))
        $candidate.Add((Invoke-NativeWindowProbe $CandidateExecutable))
    }
    else {
        $candidate.Add((Invoke-NativeWindowProbe $CandidateExecutable))
        $baseline.Add((Invoke-NativeWindowProbe $BaselineExecutable))
    }
}

$baselineMedians = Get-Medians $baseline.ToArray()
$candidateMedians = Get-Medians $candidate.ToArray()
$violations = [Collections.Generic.List[string]]::new()
foreach ($name in $baselineMedians.Keys) {
    if ([uint64]$candidateMedians[$name] -gt [uint64]$baselineMedians[$name]) {
        $violations.Add("$name candidate=$($candidateMedians[$name]) baseline=$($baselineMedians[$name])")
    }
}

$fingerprintFields = @(
    "gpu_adapter_vendor_id",
    "gpu_adapter_device_id",
    "gpu_backend",
    "gpu_surface_format",
    "gpu_surface_width",
    "gpu_surface_height",
    "pty_linkage_digest",
    "gpu_text_content_digest",
    "gpu_rendered_frames",
    "gpu_presented_frames",
    "gpu_text_rendered_frames",
    "gpu_compatibility_frame_uploads",
    "gpu_device_losses",
    "gpu_uncaptured_errors"
)
$reference = $baseline[0].metrics
foreach ($record in @($baseline.ToArray()) + @($candidate.ToArray())) {
    foreach ($field in $fingerprintFields) {
        if ($record.metrics.$field -ne $reference.$field) {
            $violations.Add("fingerprint mismatch for $field")
        }
    }
}
foreach ($record in $baseline) {
    $reportedRuntime = $record.metrics.runtime_api
    if ($null -ne $reportedRuntime -and $reportedRuntime -ne "legacy-window-feed") {
        $violations.Add("baseline did not execute the legacy runtime")
    }
}
foreach ($record in $candidate) {
    if ($record.metrics.runtime_api -ne "v2-runtime-hub") {
        $violations.Add("candidate did not execute runtime V2")
    }
}

$report = [ordered]@{
    ok = $violations.Count -eq 0
    workload = "release-native-window-ten-frame-pty"
    warmups = $Warmups
    samples = $Samples
    baseline_executable = (Resolve-Path -LiteralPath $BaselineExecutable).Path
    candidate_executable = (Resolve-Path -LiteralPath $CandidateExecutable).Path
    baseline_runtime = "legacy"
    candidate_runtime = "v2"
    baseline_median = $baselineMedians
    candidate_median = $candidateMedians
    violations = $violations.ToArray()
    baseline_samples = $baseline.ToArray()
    candidate_samples = $candidate.ToArray()
}
$json = $report | ConvertTo-Json -Depth 12
$outputPath = [IO.Path]::GetFullPath((Join-Path (Get-Location) $Output))
$outputDirectory = [IO.Path]::GetDirectoryName($outputPath)
[IO.Directory]::CreateDirectory($outputDirectory) | Out-Null
[IO.File]::WriteAllText($outputPath, $json, [Text.UTF8Encoding]::new($false))
$report | ConvertTo-Json -Depth 5
if (-not $report.ok) {
    exit 1
}
