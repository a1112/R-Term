[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [ValidateSet("debug", "release")]
    [string] $Profile = "release",
    [ValidateRange(0, 100)]
    [int] $Warmups = 5,
    [ValidateRange(1, 1000)]
    [int] $MeasuredRounds = 30,
    [ValidateRange(1, 60)]
    [int] $ProcessTimeoutSeconds = 60,
    [string] $OutputDirectory = "artifacts/stage7-font-proof",
    [switch] $SkipBuild,
    [string] $AppPath,
    [string] $LauncherPath,
    [string] $TestRunnerFingerprintInputPath,
    [ValidateSet(
        "none",
        "collector-failure",
        "collector-timeout",
        "collector-stderr",
        "final-identity-failure",
        "final-summary-failure",
        "final-output-failure",
        "final-environment-failure"
    )]
    [string] $TestRunnerFingerprintFault = "none"
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

$modes = @("current", "shared", "lazy")
$functionalSpecimens = @("cjk", "emoji")
$samplesPerProcess = 10
$currentSharedMinimumBytes = 64MB
$sharedLazyMinimumBytes = 32MB

function New-InterleavedSchedule([int] $Rounds, [string] $Phase, [string] $Specimen) {
    $schedule = [System.Collections.Generic.List[object]]::new()
    for ($round = 1; $round -le $Rounds; $round++) {
        foreach ($mode in $modes) {
            $schedule.Add([ordered]@{
                phase = $Phase
                round = $round
                mode = $mode
                specimen = $Specimen
            })
        }
    }
    return @($schedule)
}

function New-FunctionalSchedule {
    $schedule = [System.Collections.Generic.List[object]]::new()
    foreach ($mode in $modes) {
        foreach ($specimen in $functionalSpecimens) {
            $schedule.Add([ordered]@{
                phase = "functional"
                round = 1
                mode = $mode
                specimen = $specimen
            })
        }
    }
    return @($schedule)
}

$plan = [ordered]@{
    schema = "rterm.stage7/font-ownership-proof-plan/v1"
    renderer = "auto"
    explicit_backend_override = $false
    warmups_per_mode = $Warmups
    measured_processes_per_mode = $MeasuredRounds
    samples_per_process = $samplesPerProcess
    retained_ascii_raw_samples = $modes.Count * $MeasuredRounds * $samplesPerProcess
    artifact_files = 5
    atomic_raw_record_files = $modes.Count * $MeasuredRounds
    process_timeout_seconds = $ProcessTimeoutSeconds
    aggregation = [ordered]@{
        process_representative = "nearest-rank-p50"
        cross_process = "nearest-rank-p50"
        flattening_for_percentiles = "forbidden"
    }
    thresholds = [ordered]@{
        current_minus_shared_min_bytes = $currentSharedMinimumBytes
        shared_minus_lazy_min_bytes = $sharedLazyMinimumBytes
    }
    schedule = [ordered]@{
        warmups = @(New-InterleavedSchedule -Rounds $Warmups -Phase "warmup" -Specimen "ascii")
        measured = @(New-InterleavedSchedule -Rounds $MeasuredRounds -Phase "measured" -Specimen "ascii")
        functional_specimens = @(New-FunctionalSchedule)
    }
    artifacts = @(
        "font-ownership-raw.json"
        "font-ownership-aggregate.json"
        "runner-fingerprint.json"
        "font-catalog-fingerprint.json"
        "artifact-manifest-fragment.json"
    )
}

if ($WhatIfPreference) {
    $plan | ConvertTo-Json -Depth 20 -Compress
    return
}

if (-not $IsWindows) {
    throw "Stage 7 font ownership proof requires Windows"
}

. (Join-Path $PSScriptRoot "process-harness.ps1")

function Get-FileSha256([string] $Path) {
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function ConvertTo-CanonicalValue([AllowNull()] [object] $Value) {
    if ($null -eq $Value) {
        return $null
    }
    if ($Value -is [System.Collections.IDictionary]) {
        $ordered = [ordered]@{}
        foreach ($key in @($Value.Keys | ForEach-Object { [string] $_ } | Sort-Object)) {
            $ordered[$key] = ConvertTo-CanonicalValue $Value[$key]
        }
        return $ordered
    }
    if ($Value -is [PSCustomObject]) {
        $ordered = [ordered]@{}
        foreach ($property in @($Value.PSObject.Properties.Name | Sort-Object)) {
            $ordered[$property] = ConvertTo-CanonicalValue $Value.$property
        }
        return $ordered
    }
    if ($Value -is [System.Collections.IEnumerable] -and $Value -isnot [string]) {
        return @($Value | ForEach-Object { ConvertTo-CanonicalValue $_ })
    }
    return $Value
}

function Get-CanonicalJson([object] $Value) {
    return (ConvertTo-CanonicalValue $Value) | ConvertTo-Json -Depth 100 -Compress
}

function Get-CanonicalSha256([object] $Value) {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes((Get-CanonicalJson $Value))
    return [Convert]::ToHexString(
        [System.Security.Cryptography.SHA256]::HashData($bytes)
    ).ToLowerInvariant()
}

function Get-RunnerCanonicalJson([AllowNull()] [object] $Value, [string] $Label = "runner fields") {
    if ($null -eq $Value) { throw "$Label contains null outside the runner canonical protocol" }
    if ($Value -is [System.Collections.IDictionary]) {
        [string[]] $keys = @($Value.Keys | ForEach-Object { [string] $_ })
        [Array]::Sort($keys, [StringComparer]::Ordinal)
        $members = foreach ($key in $keys) {
            $encodedKey = ConvertTo-Json -InputObject $key -Compress -EscapeHandling Default
            $encodedValue = Get-RunnerCanonicalJson $Value[$key] "$Label.$key"
            "$encodedKey`:$encodedValue"
        }
        return "{$($members -join ',')}"
    }
    if ($Value -is [PSCustomObject]) {
        [string[]] $properties = @($Value.PSObject.Properties.Name)
        [Array]::Sort($properties, [StringComparer]::Ordinal)
        $members = foreach ($property in $properties) {
            $encodedKey = ConvertTo-Json -InputObject $property -Compress -EscapeHandling Default
            $encodedValue = Get-RunnerCanonicalJson $Value.$property "$Label.$property"
            "$encodedKey`:$encodedValue"
        }
        return "{$($members -join ',')}"
    }
    if ($Value -is [System.Collections.IEnumerable] -and $Value -isnot [string]) {
        $items = [System.Collections.Generic.List[string]]::new()
        $index = 0
        foreach ($item in $Value) {
            $items.Add((Get-RunnerCanonicalJson $item "$Label[$index]"))
            $index++
        }
        return "[$($items -join ',')]"
    }
    $isInteger = $Value.GetType() -in @(
        [byte], [sbyte], [Int16], [UInt16], [Int32], [UInt32], [Int64], [UInt64]
    )
    if ($Value -isnot [string] -and $Value -isnot [bool] -and -not $isInteger) {
        throw "$Label contains a value outside the integer/bool/string runner canonical protocol"
    }
    return ConvertTo-Json -InputObject $Value -Compress -EscapeHandling Default
}

function Get-RunnerCanonicalSha256([object] $Value) {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes((Get-RunnerCanonicalJson $Value))
    return [Convert]::ToHexString(
        [System.Security.Cryptography.SHA256]::HashData($bytes)
    ).ToLowerInvariant()
}

function Write-AtomicJson([string] $Path, [object] $Value) {
    $parent = Split-Path -Parent $Path
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $temporary = Join-Path $parent (".{0}.{1}.{2}.tmp" -f (Split-Path -Leaf $Path), $PID, [Guid]::NewGuid().ToString("N"))
    try {
        $json = $Value | ConvertTo-Json -Depth 100
        [System.IO.File]::WriteAllText(
            $temporary,
            $json + [Environment]::NewLine,
            [System.Text.UTF8Encoding]::new($false)
        )
        Move-Item -LiteralPath $temporary -Destination $Path
    }
    finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Try-GetJsonUInt64Scalar(
    [AllowNull()] [object] $Value,
    [ref] $Result
) {
    if ($null -eq $Value) {
        return $false
    }
    $isIntegerScalar = $Value.GetType() -in @(
        [byte], [sbyte], [Int16], [UInt16], [Int32], [UInt32], [Int64], [UInt64]
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

function Get-RequiredUInt64([AllowNull()] [object] $Value, [string] $Field) {
    [UInt64] $result = 0
    if (-not (Try-GetJsonUInt64Scalar -Value $Value -Result ([ref] $result))) {
        throw "$Field must be an unsigned JSON integer scalar"
    }
    return $result
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

function Assert-ExactPropertySet([object] $Value, [string[]] $Expected, [string] $Label) {
    if ($null -eq $Value) {
        throw "$Label is missing"
    }
    $actual = @($Value.PSObject.Properties.Name | Sort-Object)
    $expectedOrdered = @($Expected | Sort-Object)
    if (($actual -join "`n") -cne ($expectedOrdered -join "`n")) {
        throw "$Label fields differ from the closed schema"
    }
}

function Assert-NoPathKeys([AllowNull()] [object] $Value, [string] $Label) {
    if ($null -eq $Value) {
        return
    }
    if ($Value -is [PSCustomObject] -or $Value -is [System.Collections.IDictionary]) {
        foreach ($property in $Value.PSObject.Properties) {
            if ($property.Name.IndexOf("path", [StringComparison]::OrdinalIgnoreCase) -ge 0) {
                throw "$Label contains forbidden path key '$($property.Name)'"
            }
            Assert-NoPathKeys -Value $property.Value -Label $Label
        }
        return
    }
    if ($Value -is [System.Collections.IEnumerable] -and $Value -isnot [string]) {
        foreach ($item in $Value) {
            Assert-NoPathKeys -Value $item -Label $Label
        }
    }
}

function Get-NearestRankPercentile([UInt64[]] $Values, [double] $Percentile) {
    if ($Values.Count -eq 0) {
        throw "nearest-rank percentile requires at least one value"
    }
    $ordered = @($Values | Sort-Object)
    $rank = [Math]::Ceiling($Percentile * $ordered.Count)
    return [UInt64] $ordered[[Math]::Max(0, $rank - 1)]
}

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$profileDirectory = if ($Profile -eq "release") { "release" } else { "debug" }
if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    throw "CARGO_TARGET_DIR is required for every real font proof run"
}
if (-not [System.IO.Path]::IsPathFullyQualified($env:CARGO_TARGET_DIR)) {
    throw "CARGO_TARGET_DIR must be absolute for every real font proof run"
}
$targetRoot = [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
$repoBoundary = $repoRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
$targetBoundary = $targetRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
$repoPrefix = $repoBoundary + [System.IO.Path]::DirectorySeparatorChar
if (
    $targetBoundary.Equals($repoBoundary, [StringComparison]::OrdinalIgnoreCase) -or
    $targetBoundary.StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase)
) {
    throw "CARGO_TARGET_DIR must be outside the repository"
}
$hasAppOverride = -not [string]::IsNullOrWhiteSpace($AppPath)
$hasLauncherOverride = -not [string]::IsNullOrWhiteSpace($LauncherPath)
if ($hasAppOverride -ne $hasLauncherOverride) {
    throw "both -AppPath and -LauncherPath must be provided together"
}
if (($hasAppOverride -or $hasLauncherOverride) -and -not $SkipBuild) {
    throw "path overrides require -SkipBuild"
}
$hasFingerprintTestSeam =
    -not [string]::IsNullOrWhiteSpace($TestRunnerFingerprintInputPath) -or
    $TestRunnerFingerprintFault -cne "none"
if ($hasFingerprintTestSeam -and (-not $SkipBuild -or -not $hasAppOverride -or -not $hasLauncherOverride)) {
    throw "test-only runner fingerprint input/fault requires -SkipBuild with both binary overrides"
}
$certificationEligible = -not $SkipBuild -and -not $hasAppOverride -and -not $hasLauncherOverride -and $ProcessTimeoutSeconds -eq 60
if ($certificationEligible -and $hasFingerprintTestSeam) {
    throw "certification collection forbids runner fingerprint fixture injection"
}
if ($certificationEligible -and ($Profile -cne "release" -or $Warmups -ne 5 -or $MeasuredRounds -ne 30)) {
    throw "certification collection requires locked release with exactly 5 warmups and 30 measured processes per mode"
}

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
$outputRoot = (Resolve-Path -LiteralPath $OutputDirectory).Path
$temporaryBase = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$stagingRoot = Join-Path $temporaryBase ("rterm-stage7-font-proof-{0}-{1}" -f $PID, [Guid]::NewGuid().ToString("N"))
$outputBoundary = $outputRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
$outputPrefix = $outputBoundary + [System.IO.Path]::DirectorySeparatorChar
if (
    $stagingRoot.Equals($outputBoundary, [StringComparison]::OrdinalIgnoreCase) -or
    $stagingRoot.StartsWith($outputPrefix, [StringComparison]::OrdinalIgnoreCase)
) {
    throw "TEMP atomic-record staging must be outside the evidence root"
}
New-Item -ItemType Directory -Path $stagingRoot | Out-Null

$executableSuffix = ".exe"
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

if (-not $SkipBuild) {
    $profileArguments = @()
    if ($Profile -eq "release") {
        $profileArguments += "--release"
    }
    Push-Location $repoRoot
    try {
        cargo build --locked -p rterm-app --no-default-features --features production-gui,diagnostic-tools @profileArguments
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for diagnostic rterm-app with exit code $LASTEXITCODE"
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
foreach ($binary in @($app, $launcher)) {
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "required diagnostic binary is missing"
    }
}

$sourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceSha -cnotmatch "^[0-9a-f]{40}$") {
    throw "unable to resolve one immutable source commit"
}
if ($certificationEligible) {
    $dirty = @(& git -C $repoRoot status --porcelain)
    if ($LASTEXITCODE -ne 0 -or $dirty.Count -ne 0) {
        throw "certification collection requires a clean source tree including untracked files"
    }
}
$producerScriptSha256 = Get-FileSha256 $PSCommandPath
$initialBinaryHashes = [ordered]@{
    "rterm.exe" = Get-FileSha256 $app
    "rterm-bench-launcher.exe" = Get-FileSha256 $launcher
}
$runId = "stage7-font-proof-{0}-{1}" -f ([DateTimeOffset]::UtcNow.ToUnixTimeSeconds()), $PID
$identity = $null

$fontResourceFields = @(
    "mode",
    "specimen",
    "retained_source_bytes",
    "indexed_source_count",
    "active_source_count",
    "initial_catalog_source_count",
    "catalog_builds",
    "generation",
    "recovery_retained_source_bytes",
    "recovery_generation",
    "activation_latency_micros",
    "tofu_count",
    "frame_catalog_generation",
    "frame_generation_consistent",
    "index_fingerprint_sha256",
    "catalog_fingerprint_sha256",
    "ordered_catalog_fingerprint_sha256"
)
$script:gpuIdentity = $null

function Assert-BinaryIdentityUnchanged {
    $currentAppHash = Get-FileSha256 $app
    $currentLauncherHash = Get-FileSha256 $launcher
    if (
        $currentAppHash -cne $initialBinaryHashes["rterm.exe"] -or
        $currentLauncherHash -cne $initialBinaryHashes["rterm-bench-launcher.exe"]
    ) {
        throw "diagnostic binary identity changed during collection"
    }
}

function Assert-CollectionIdentityUnchanged {
    Assert-BinaryIdentityUnchanged
    $currentSourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $currentSourceSha -cne $sourceSha) {
        throw "source commit changed during collection"
    }
    if ($certificationEligible) {
        $dirty = @(& git -C $repoRoot status --porcelain)
        if ($LASTEXITCODE -ne 0 -or $dirty.Count -ne 0) {
            throw "certification source tree changed during collection including untracked files"
        }
    }
}

function Invoke-FontDiagnostic([string] $Mode, [string] $Specimen) {
    $launcherArguments = @(
        "--app", $app,
        "--scenario", "empty-window",
        "--stabilization-ms", "5000",
        "--sample-interval-ms", "100",
        "--sample-count", "10",
        "--cols", "80",
        "--rows", "24",
        "--renderer", "auto",
        "--font-mode", $Mode,
        "--font-specimen", $Specimen,
        "--json"
    )
    $boundedFile = $launcher
    $boundedArguments = $launcherArguments
    if ([IO.Path]::GetExtension($launcher) -cin @(".cmd", ".bat")) {
        $boundedFile = "cmd.exe"
        $boundedArguments = @("/D", "/S", "/C", $launcher) + $launcherArguments
    }
    $bounded = Invoke-BoundedProcess `
        -Phase "font proof $Mode/$Specimen" `
        -FilePath $boundedFile `
        -ArgumentList $boundedArguments `
        -TimeoutSeconds $ProcessTimeoutSeconds 6>$null
    Assert-BinaryIdentityUnchanged
    $json = $bounded.Stdout
    try {
        $record = $json | ConvertFrom-Json
    }
    catch {
        throw "launcher output was not valid JSON"
    }
    if (
        $record.schema -cne "rterm.diagnostics/v2" -or
        $record.readiness.status -cne "ready" -or
        $record.failures.Count -ne 0
    ) {
        throw "font diagnostic failed for $Mode/$Specimen"
    }
    if ($record.run.id -isnot [string] -or $record.run.id -cnotmatch '^empty-window-[0-9]+-[0-9]+$') {
        throw "font diagnostic did not retain an actual empty-window run ID"
    }
    foreach ($field in @(
        @{ Name = "stabilization_ms"; Value = 5000 },
        @{ Name = "sample_interval_ms"; Value = 100 },
        @{ Name = "sample_count"; Value = 10 },
        @{ Name = "columns"; Value = 80 },
        @{ Name = "rows"; Value = 24 },
        @{ Name = "scale_factor_milli"; Value = 1000 }
    )) {
        $actual = Get-RequiredUInt64 -Value $record.configuration.($field.Name) -Field "configuration.$($field.Name)"
        if ($actual -ne [UInt64] $field.Value) {
            throw "configuration.$($field.Name) mismatch"
        }
    }
    if (
        $record.configuration.requested_renderer -cne "auto" -or
        $record.configuration.requested_font_mode -cne $Mode -or
        $record.configuration.requested_font_specimen -cne $Specimen -or
        $record.configuration.PSObject.Properties.Name -contains "requested_gpu_backend"
    ) {
        throw "diagnostic requested renderer/backend/font identity mismatch"
    }
    if (
        $record.renderer.final -cne "gpu" -or
        $record.renderer.backend -cnotin @("dx12", "vulkan", "gl") -or
        $record.renderer.adapter_name -isnot [string] -or
        [string]::IsNullOrWhiteSpace($record.renderer.adapter_name) -or
        -not (Test-ProductionAdapterType -Value $record.renderer.adapter_type)
    ) {
        throw "diagnostic actual renderer/backend identity or production adapter type is missing or unsupported"
    }
    $gpuIdentity = [ordered]@{
        backend = $record.renderer.backend
        adapter_name = $record.renderer.adapter_name
        adapter_vendor_id = Get-RequiredUInt64 $record.renderer.adapter_vendor_id "renderer.adapter_vendor_id"
        adapter_device_id = Get-RequiredUInt64 $record.renderer.adapter_device_id "renderer.adapter_device_id"
        adapter_type = $record.renderer.adapter_type
    }
    if ($null -eq $script:gpuIdentity) {
        $script:gpuIdentity = $gpuIdentity
    } elseif ((Get-CanonicalJson $script:gpuIdentity) -cne (Get-CanonicalJson $gpuIdentity)) {
        throw "functional and measured runs used mixed actual GPU identity"
    }
    if (
        $record.memory.metric -cne "windows_private_working_set_bytes" -or
        $record.memory.unit -cne "bytes" -or
        $record.memory.samples.Count -ne 10
    ) {
        throw "memory counter or ten-sample residence record is missing"
    }
    [UInt64] $firstSampleElapsed = 0
    for ($index = 0; $index -lt 10; $index++) {
        $sample = $record.memory.samples[$index]
        $sequence = Get-RequiredUInt64 $sample.sequence "memory.samples[$index].sequence"
        $elapsed = Get-RequiredUInt64 $sample.elapsed_ms "memory.samples[$index].elapsed_ms"
        $bytes = Get-RequiredUInt64 $sample.bytes "memory.samples[$index].bytes"
        if ($sequence -ne [UInt64] $index -or $bytes -eq 0) {
            throw "memory sample sequence/value mismatch at index $index"
        }
        if ($index -eq 0) {
            $firstSampleElapsed = $elapsed
        }
    }
    $gpuReady = Get-RequiredUInt64 $record.milestones.gpu_ready_ms "milestones.gpu_ready_ms"
    $fontOwnershipReady = Get-RequiredUInt64 $record.milestones.font_ownership_ready_ms "milestones.font_ownership_ready_ms"
    $scenarioReady = Get-RequiredUInt64 $record.milestones.scenario_ready_ms "milestones.scenario_ready_ms"
    if (
        $gpuReady -gt $fontOwnershipReady -or
        $fontOwnershipReady -gt $scenarioReady -or
        $scenarioReady -gt $firstSampleElapsed
    ) {
        throw "font ownership readiness marker order must be gpu_ready <= font_ownership_ready <= scenario_ready <= first sample"
    }
    $expectedFontResourceFields = @($fontResourceFields)
    if ($Mode -cin @("current", "shared")) {
        $expectedFontResourceFields += @(
            "font_inventory_fingerprint_sha256",
            "font_index_policy_version"
        )
    }
    Assert-ExactPropertySet -Value $record.font_resources -Expected $expectedFontResourceFields -Label "font_resources"
    Assert-NoPathKeys -Value $record.font_resources -Label "font_resources"
    $resources = $record.font_resources
    if ($resources.mode -cne $Mode -or $resources.specimen -cne $Specimen) {
        throw "font mode/specimen fallback detected"
    }
    $retained = Get-RequiredUInt64 $resources.retained_source_bytes "font_resources.retained_source_bytes"
    $recoveryRetained = Get-RequiredUInt64 $resources.recovery_retained_source_bytes "font_resources.recovery_retained_source_bytes"
    $indexed = Get-RequiredUInt64 $resources.indexed_source_count "font_resources.indexed_source_count"
    $active = Get-RequiredUInt64 $resources.active_source_count "font_resources.active_source_count"
    $initial = Get-RequiredUInt64 $resources.initial_catalog_source_count "font_resources.initial_catalog_source_count"
    $builds = Get-RequiredUInt64 $resources.catalog_builds "font_resources.catalog_builds"
    $generation = Get-RequiredUInt64 $resources.generation "font_resources.generation"
    $recoveryGeneration = Get-RequiredUInt64 $resources.recovery_generation "font_resources.recovery_generation"
    $activationLatency = Get-RequiredUInt64 $resources.activation_latency_micros "font_resources.activation_latency_micros"
    $tofu = Get-RequiredUInt64 $resources.tofu_count "font_resources.tofu_count"
    $frameGeneration = Get-RequiredUInt64 $resources.frame_catalog_generation "font_resources.frame_catalog_generation"
    if (
        $indexed -lt $active -or $active -eq 0 -or $initial -eq 0 -or $initial -gt $active -or
        $builds -ne $generation -or $recoveryGeneration -ne $generation -or
        $builds -ne ($active - $initial + 1) -or
        $recoveryRetained -ne $retained -or $tofu -ne 0 -or
        $frameGeneration -ne $generation -or
        $resources.frame_generation_consistent -isnot [bool] -or
        $resources.frame_generation_consistent -ne $true
    ) {
        throw "font resource counters, tofu, frame generation, or recovery retention are inconsistent"
    }
    $modeCounterShapeValid = switch ($Mode) {
        "current" {
            $initial -ge 1
        }
        "shared" { $initial -eq $active -and $builds -eq 1 }
        "lazy" {
            if ($Specimen -eq "ascii") {
                $initial -eq 1 -and $builds -eq 1 -and $active -eq 1
            } else {
                $initial -eq 1 -and $builds -eq 2 -and $active -eq 2
            }
        }
        default { $false }
    }
    if (-not $modeCounterShapeValid) {
        throw "font resource mode counter shape is inconsistent for $Mode/$Specimen"
    }
    foreach ($fingerprint in @(
        "index_fingerprint_sha256",
        "catalog_fingerprint_sha256",
        "ordered_catalog_fingerprint_sha256"
    )) {
        if ($resources.$fingerprint -isnot [string] -or $resources.$fingerprint -cnotmatch "^[0-9a-f]{64}$") {
            throw "font resource $fingerprint is not an irreversible SHA-256"
        }
    }
    if ($Mode -cin @("current", "shared")) {
        if (
            $resources.font_inventory_fingerprint_sha256 -isnot [string] -or
            $resources.font_inventory_fingerprint_sha256 -cnotmatch "^[0-9a-f]{64}$" -or
            (Get-RequiredUInt64 $resources.font_index_policy_version "font_resources.font_index_policy_version") -eq 0
        ) {
            throw "full font inventory fingerprint or policy version is invalid"
        }
    }
    return [ordered]@{
        diagnostics = $record
        retained_source_bytes = $retained
        activation_latency_micros = $activationLatency
        font_resources = $resources
    }
}

function New-RawProcessRecord([object] $Run, [int] $Round) {
    [UInt64[]] $samples = @($Run.diagnostics.memory.samples | ForEach-Object {
        Get-RequiredUInt64 $_.bytes "memory sample bytes"
    })
    return [ordered]@{
        process_id = [string] $Run.diagnostics.run.id
        phase = "measured"
        round_index = $Round
        samples = @($samples)
        representative = Get-NearestRankPercentile -Values $samples -Percentile 0.50
        font_resources = $Run.font_resources
    }
}

function New-GroupStatistics([object[]] $Processes) {
    [UInt64[]] $representatives = @($Processes | ForEach-Object { [UInt64] $_.representative })
    [UInt64[]] $raw = @($Processes | ForEach-Object { $_.samples } | ForEach-Object { [UInt64] $_ })
    return [ordered]@{
        p50 = Get-NearestRankPercentile -Values $representatives -Percentile 0.50
        p95 = Get-NearestRankPercentile -Values $representatives -Percentile 0.95
        max = [UInt64] ($raw | Measure-Object -Maximum).Maximum
    }
}

function Assert-CrossModeResourceEvidence([int] $Round, [object] $Current, [object] $Shared) {
    $currentResources = $Current.font_resources
    $sharedResources = $Shared.font_resources
    foreach ($field in @(
        "indexed_source_count",
        "active_source_count",
        "index_fingerprint_sha256",
        "catalog_fingerprint_sha256",
        "ordered_catalog_fingerprint_sha256",
        "font_inventory_fingerprint_sha256",
        "font_index_policy_version"
    )) {
        if ($currentResources.$field -cne $sharedResources.$field) {
            throw "round $Round CurrentCopied/SharedAll $field differs"
        }
    }
    [UInt64] $currentRetained = Get-RequiredUInt64 $currentResources.retained_source_bytes "current retained_source_bytes"
    [UInt64] $sharedRetained = Get-RequiredUInt64 $sharedResources.retained_source_bytes "shared retained_source_bytes"
    if ($currentRetained -ne (2 * $sharedRetained)) {
        throw "round $Round CurrentCopied retained bytes must equal exactly twice SharedAll"
    }
    return [ordered]@{
        round_index = $Round
        indexed_source_count = Get-RequiredUInt64 $currentResources.indexed_source_count "indexed_source_count"
        active_source_count = Get-RequiredUInt64 $currentResources.active_source_count "active_source_count"
        current_retained_source_bytes = $currentRetained
        shared_retained_source_bytes = $sharedRetained
        index_fingerprint_sha256 = $currentResources.index_fingerprint_sha256
        catalog_fingerprint_sha256 = $currentResources.catalog_fingerprint_sha256
        ordered_catalog_fingerprint_sha256 = $currentResources.ordered_catalog_fingerprint_sha256
        font_inventory_fingerprint_sha256 = $currentResources.font_inventory_fingerprint_sha256
        font_index_policy_version = Get-RequiredUInt64 $currentResources.font_index_policy_version "font_index_policy_version"
    }
}

function Assert-RunnerFingerprintObservation([object] $Observation, [string] $ExpectedSource) {
    Assert-ExactPropertySet -Value $Observation -Expected @(
        "schema", "source", "complete", "fields", "fingerprint_sha256", "collector_script_sha256", "collector_timeout_seconds"
    ) -Label "runner fingerprint observation"
    if (
        $Observation.schema -cne "rterm.stage7/runner-fingerprint/v1" -or
        $Observation.source -cne $ExpectedSource -or
        $Observation.complete -isnot [bool] -or
        $Observation.complete -ne $true
    ) {
        throw "runner fingerprint collector returned incomplete or unexpected evidence"
    }
    Assert-NoPathKeys -Value $Observation.fields -Label "runner fingerprint fields"
    Assert-ExactPropertySet -Value $Observation.fields -Expected @(
        "os", "gpu", "memory", "displays", "power_plan", "session", "locale", "fonts", "cold_cache_policy"
    ) -Label "runner fingerprint fields"
    foreach ($shape in @(
        @{ Value = $Observation.fields.os; Expected = @("version", "build_number", "build_revision", "architecture"); Label = "runner fingerprint fields.os" },
        @{ Value = $Observation.fields.gpu; Expected = @("vendor_id", "device_id", "driver_version", "wddm_version"); Label = "runner fingerprint fields.gpu" },
        @{ Value = $Observation.fields.memory; Expected = @("physical_bytes", "pagefile_mode"); Label = "runner fingerprint fields.memory" },
        @{ Value = $Observation.fields.power_plan; Expected = @("guid"); Label = "runner fingerprint fields.power_plan" },
        @{ Value = $Observation.fields.session; Expected = @("kind"); Label = "runner fingerprint fields.session" },
        @{ Value = $Observation.fields.locale; Expected = @("culture", "ui_culture", "system_locale"); Label = "runner fingerprint fields.locale" },
        @{ Value = $Observation.fields.fonts; Expected = @("inventory_fingerprint_sha256", "index_policy_version"); Label = "runner fingerprint fields.fonts" },
        @{ Value = $Observation.fields.cold_cache_policy; Expected = @("process_cold_start", "os_file_cache"); Label = "runner fingerprint fields.cold_cache_policy" }
    )) {
        Assert-ExactPropertySet -Value $shape.Value -Expected $shape.Expected -Label $shape.Label
    }
    if ($Observation.fields.displays.Count -eq 0) {
        throw "runner display topology is empty"
    }
    foreach ($display in $Observation.fields.displays) {
        Assert-ExactPropertySet -Value $display -Expected @(
            "width_px", "height_px", "dpi_x", "dpi_y", "primary"
        ) -Label "runner display"
    }
    if (
        $Observation.fields.fonts.inventory_fingerprint_sha256 -cnotmatch "^[0-9a-f]{64}$" -or
        (Get-RequiredUInt64 $Observation.fields.fonts.index_policy_version "runner font index policy version") -eq 0 -or
        $Observation.fields.cold_cache_policy.process_cold_start -ne $true -or
        $Observation.fields.cold_cache_policy.os_file_cache -cne "unmodified-no-explicit-flush" -or
        $Observation.fingerprint_sha256 -cne (Get-RunnerCanonicalSha256 $Observation.fields) -or
        $Observation.collector_script_sha256 -cne (Get-FileSha256 (Join-Path $PSScriptRoot "collect-stage7-runner-fingerprint.ps1"))
    ) {
        throw "runner fingerprint observation failed canonical identity validation"
    }
    if ((Get-RequiredUInt64 $Observation.collector_timeout_seconds "collector_timeout_seconds") -ne 60) {
        throw "runner fingerprint collector timeout must be the 60-second bounded process contract"
    }
}

$previousScale = $env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR
$script:benchmarkEnvironmentRestored = $false
function Restore-BenchmarkEnvironment([switch] $AllowTestFault) {
    if ($AllowTestFault -and $TestRunnerFingerprintFault -ceq "final-environment-failure") {
        throw "simulated final environment restoration failpoint"
    }
    if ($null -eq $previousScale) {
        Remove-Item Env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR -ErrorAction Stop
    } else {
        $env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR = $previousScale
    }
    $script:benchmarkEnvironmentRestored = $true
}
$env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR = "1"
try {
    $warmupProcessIds = [System.Collections.Generic.List[string]]::new()
    $warmupProcessIdSet = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $warmupIdsByMode = [ordered]@{
        current = [System.Collections.Generic.List[string]]::new()
        shared = [System.Collections.Generic.List[string]]::new()
        lazy = [System.Collections.Generic.List[string]]::new()
    }
    $sharedWarmupResources = $null
    for ($round = 1; $round -le $Warmups; $round++) {
        foreach ($mode in $modes) {
            $run = Invoke-FontDiagnostic -Mode $mode -Specimen "ascii"
            $childId = [string] $run.diagnostics.run.id
            if (-not $warmupProcessIdSet.Add($childId)) {
                throw "warmup font proof process IDs must be globally unique"
            }
            $warmupIdsByMode[$mode].Add($childId)
            $warmupProcessIds.Add($childId)
            if ($mode -ceq "shared") {
                $sharedWarmupResources = $run.font_resources
            }
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($TestRunnerFingerprintInputPath)) {
        $fixtureFields = Get-Content -LiteralPath $TestRunnerFingerprintInputPath -Raw | ConvertFrom-Json
        $inventoryFingerprint = [string] $fixtureFields.fonts.inventory_fingerprint_sha256
        $fontIndexPolicyVersion = Get-RequiredUInt64 $fixtureFields.fonts.index_policy_version "fixture font index policy version"
        $fingerprintGpuVendorId = Get-RequiredUInt64 $fixtureFields.gpu.vendor_id "fixture GPU vendor ID"
        $fingerprintGpuDeviceId = Get-RequiredUInt64 $fixtureFields.gpu.device_id "fixture GPU device ID"
        $fingerprintGpuAdapterName = "fixture-adapter"
        $expectedFingerprintSource = "fixture"
    } else {
        if ($null -eq $sharedWarmupResources) {
            throw "at least one SharedAll warmup is required before the host runner fingerprint is frozen"
        }
        $inventoryFingerprint = [string] $sharedWarmupResources.font_inventory_fingerprint_sha256
        $fontIndexPolicyVersion = Get-RequiredUInt64 $sharedWarmupResources.font_index_policy_version "shared warmup font index policy version"
        $fingerprintGpuVendorId = [UInt64] $script:gpuIdentity.adapter_vendor_id
        $fingerprintGpuDeviceId = [UInt64] $script:gpuIdentity.adapter_device_id
        $fingerprintGpuAdapterName = [string] $script:gpuIdentity.adapter_name
        $expectedFingerprintSource = "host-probe"
    }
    $collectorPath = Join-Path $PSScriptRoot "collect-stage7-runner-fingerprint.ps1"
    $collectorFault = if ($TestRunnerFingerprintFault -in @("collector-failure", "collector-timeout")) {
        $TestRunnerFingerprintFault
    } elseif ($TestRunnerFingerprintFault -ceq "collector-stderr") {
        "stderr-output"
    } else {
        "none"
    }
    $collectorArguments = @(
        "-NoProfile"
        "-NonInteractive"
        "-File"
        $collectorPath
        "-GpuVendorId"
        ([string] $fingerprintGpuVendorId)
        "-GpuDeviceId"
        ([string] $fingerprintGpuDeviceId)
        "-GpuAdapterName"
        $fingerprintGpuAdapterName
        "-FontInventoryFingerprintSha256"
        $inventoryFingerprint
        "-FontIndexPolicyVersion"
        ([string] $fontIndexPolicyVersion)
        "-TestFault"
        $collectorFault
    )
    if (-not [string]::IsNullOrWhiteSpace($TestRunnerFingerprintInputPath)) {
        $collectorArguments += @("-TestInputPath", $TestRunnerFingerprintInputPath)
    }
    $collectorDeadlineSeconds = if (
        $TestRunnerFingerprintFault -ceq "collector-timeout" -and
        $ProcessTimeoutSeconds -lt 60
    ) {
        $ProcessTimeoutSeconds
    } else {
        60
    }
    try {
        $boundedCollector = Invoke-BoundedProcess `
            -Phase "Stage 7 runner fingerprint collector" `
            -FilePath "pwsh.exe" `
            -ArgumentList $collectorArguments `
            -TimeoutSeconds $collectorDeadlineSeconds 6>$null
        if ($boundedCollector.Stderr.Length -ne 0) {
            throw "runner fingerprint collector stderr must be empty"
        }
        $collectorJson = $boundedCollector.Stdout.Trim()
    }
    catch {
        throw "runner fingerprint collector failed: $($_.Exception.Message)"
    }
    try {
        $runnerObservation = $collectorJson | ConvertFrom-Json
    } catch {
        throw "runner fingerprint collector output was not valid JSON"
    }
    Assert-RunnerFingerprintObservation $runnerObservation $expectedFingerprintSource
    if ($certificationEligible -and $runnerObservation.source -cne "host-probe") {
        throw "certification collection requires host-probe runner fingerprint evidence"
    }
    $runnerFingerprint = [string] $runnerObservation.fingerprint_sha256
    $identity = [ordered]@{
        source_sha = $sourceSha
        binary_hashes = $initialBinaryHashes
        runner_fingerprint_sha256 = $runnerFingerprint
        platform = "windows-x86_64"
        run_id = $runId
    }
    $runnerAnchorIdentity = [ordered]@{
        source_sha = $sourceSha
        platform = "windows-x86_64"
        run_id = $runId
    }

    $processesByMode = [ordered]@{
        current = [System.Collections.Generic.List[object]]::new()
        shared = [System.Collections.Generic.List[object]]::new()
        lazy = [System.Collections.Generic.List[object]]::new()
    }
    $atomicPayloads = [System.Collections.Generic.List[object]]::new()
    $measuredProcessIds = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $crossModeEvidence = [System.Collections.Generic.List[object]]::new()
    for ($round = 1; $round -le $MeasuredRounds; $round++) {
        $runsByMode = [ordered]@{}
        foreach ($mode in $modes) {
            $run = Invoke-FontDiagnostic -Mode $mode -Specimen "ascii"
            $record = New-RawProcessRecord -Run $run -Round $round
            if (
                $warmupProcessIdSet.Contains([string] $record.process_id) -or
                -not $measuredProcessIds.Add([string] $record.process_id)
            ) {
                throw "measured font proof process IDs must be globally unique"
            }
            $atomicRelativePath = "raw/{0}/round-{1:D3}.json" -f $mode, $round
            $atomicPath = Join-Path $stagingRoot $atomicRelativePath
            $atomicPayload = [ordered]@{
                schema = "rterm.stage7/font-ownership-process/v1"
                certification_eligible = $certificationEligible
                identity = $identity
                requested_backend = "auto"
                actual_backend = $script:gpuIdentity.backend
                mode = $mode
                specimen = "ascii"
                round_index = $round
                timeout_seconds = $ProcessTimeoutSeconds
                stabilization_ms = 5000
                sample_interval_ms = 100
                process = $record
            }
            Write-AtomicJson $atomicPath $atomicPayload
            $atomicPayloads.Add([ordered]@{
                relative_path = $atomicRelativePath
                payload = $atomicPayload
            })
            $processesByMode[$mode].Add($record)
            $runsByMode[$mode] = $run
        }
        $crossModeEvidence.Add((Assert-CrossModeResourceEvidence `
            -Round $round `
            -Current $runsByMode.current `
            -Shared $runsByMode.shared))
    }

    $protocol = [ordered]@{
        warmups = $Warmups
        measured_cold_processes = $MeasuredRounds
        timeout_seconds = $ProcessTimeoutSeconds
        cross_process_percentiles = "nearest-rank"
        maximum = "raw-maximum"
        sampling_mode = "residence"
        samples_per_process = 10
        stabilization_ms = 5000
        sample_interval_ms = 100
        process_representative = "nearest-rank-p50"
        flattening_for_percentiles = "forbidden"
        owner_ready_marker = "font_ownership_ready"
    }
    $groupNames = [ordered]@{
        current = "current-copied/ascii"
        shared = "shared-all/ascii"
        lazy = "lazy/ascii"
    }
    $groups = [System.Collections.Generic.List[object]]::new()
    foreach ($mode in $modes) {
        $processes = @($processesByMode[$mode])
        $groups.Add([ordered]@{
            name = $groupNames[$mode]
            metric = "windows_private_working_set_bytes"
            sampling_mode = "residence"
            requested_backend = "auto"
            final_renderer = "gpu"
            actual_backend = $script:gpuIdentity.backend
            adapter_identity = Get-CanonicalSha256 $script:gpuIdentity
            owner_ready_marker = "font_ownership_ready"
            stabilization_ms = 5000
            sample_interval_ms = 100
            warmup_process_ids = @($warmupIdsByMode[$mode])
            processes = $processes
            statistics = New-GroupStatistics $processes
        })
    }
    $rawPayload = [ordered]@{
        schema = "rterm.stage7.metric-raw/v1"
        certification_eligible = $certificationEligible
        identity = $identity
        warmups = $Warmups * $modes.Count
        warmup_process_ids = @($warmupProcessIds)
        measured_cold_processes = $MeasuredRounds
        timeout_seconds = $ProcessTimeoutSeconds
        protocol = $protocol
        groups = @($groups)
    }
    $rawPath = Join-Path $outputRoot "font-ownership-raw.json"
    Write-AtomicJson $rawPath $rawPayload

    $statistics = @($groups | ForEach-Object { $_.statistics })
    [Int64] $currentSharedDelta = [Int64] $statistics[0].p50 - [Int64] $statistics[1].p50
    [Int64] $sharedLazyDelta = [Int64] $statistics[1].p50 - [Int64] $statistics[2].p50
    if ($currentSharedDelta -lt $currentSharedMinimumBytes) {
        throw "current-copied to shared-all p50 reduction is below 67108864 bytes"
    }
    if ($sharedLazyDelta -lt $sharedLazyMinimumBytes) {
        throw "shared-all to lazy p50 reduction is below 33554432 bytes"
    }

    $aggregatePayload = [ordered]@{
        schema = "rterm.stage7.metric-aggregate/v1"
        certification_eligible = $certificationEligible
        identity = $identity
        ok = $true
        raw_children = @("font-ownership-raw")
        group_statistics = $statistics
    }
    $aggregatePath = Join-Path $outputRoot "font-ownership-aggregate.json"
    Write-AtomicJson $aggregatePath $aggregatePayload

    $functionalRecords = [System.Collections.Generic.List[object]]::new()
    foreach ($mode in $modes) {
        foreach ($specimen in $functionalSpecimens) {
            $run = Invoke-FontDiagnostic -Mode $mode -Specimen $specimen
            $resources = $run.diagnostics.font_resources
            $functionalRecord = [ordered]@{
                requested_font_mode = $mode
                actual_font_mode = $resources.mode
                requested_font_specimen = $specimen
                actual_font_specimen = $resources.specimen
                requested_backend = "auto"
                actual_backend = $run.diagnostics.renderer.backend
                activation_latency_ms = [double] $run.activation_latency_micros / 1000.0
                activation_latency_gate = "report-only"
                retained_source_bytes = Get-RequiredUInt64 $resources.retained_source_bytes "retained_source_bytes"
                recovery_retained_source_bytes = Get-RequiredUInt64 $resources.recovery_retained_source_bytes "recovery_retained_source_bytes"
                indexed_source_count = Get-RequiredUInt64 $resources.indexed_source_count "indexed_source_count"
                active_source_count = Get-RequiredUInt64 $resources.active_source_count "active_source_count"
                initial_catalog_source_count = Get-RequiredUInt64 $resources.initial_catalog_source_count "initial_catalog_source_count"
                catalog_builds = Get-RequiredUInt64 $resources.catalog_builds "catalog_builds"
                generation = Get-RequiredUInt64 $resources.generation "generation"
                recovery_generation = Get-RequiredUInt64 $resources.recovery_generation "recovery_generation"
                frame_catalog_generation = Get-RequiredUInt64 $resources.frame_catalog_generation "frame_catalog_generation"
                frame_generation_consistent = $resources.frame_generation_consistent
                tofu_count = Get-RequiredUInt64 $resources.tofu_count "tofu_count"
                index_fingerprint_sha256 = $resources.index_fingerprint_sha256
                catalog_fingerprint_sha256 = $resources.catalog_fingerprint_sha256
                ordered_catalog_fingerprint_sha256 = $resources.ordered_catalog_fingerprint_sha256
            }
            if ($mode -cin @("current", "shared")) {
                $functionalRecord.font_inventory_fingerprint_sha256 = $resources.font_inventory_fingerprint_sha256
                $functionalRecord.font_index_policy_version = Get-RequiredUInt64 $resources.font_index_policy_version "font_index_policy_version"
            }
            $functionalRecords.Add($functionalRecord)
        }
    }

    $runnerPayload = [ordered]@{
        schema = "rterm.stage7.result/v1"
        certification_eligible = $certificationEligible
        identity = $runnerAnchorIdentity
        ok = $true
        proof = "runner-fingerprint"
        claims = [ordered]@{ fingerprint_fields_complete = $true }
        source = $runnerObservation.source
        complete = $runnerObservation.complete
        fields = $runnerObservation.fields
        fingerprint_sha256 = $runnerFingerprint
        producer_script_sha256 = $producerScriptSha256
        collector_script_sha256 = $runnerObservation.collector_script_sha256
        collector_timeout_seconds = Get-RequiredUInt64 $runnerObservation.collector_timeout_seconds "collector_timeout_seconds"
    }
    $runnerPath = Join-Path $outputRoot "runner-fingerprint.json"
    Write-AtomicJson $runnerPath $runnerPayload

    $catalogPayload = [ordered]@{
        schema = "rterm.stage7.result/v1"
        certification_eligible = $certificationEligible
        identity = $identity
        ok = $true
        proof = "font-catalog-fingerprint"
        claims = [ordered]@{
            catalog_policy_version = "stage7-private-v1"
            ordered_sources_hashed = $true
            functional_specimen_count = 6
            zero_tofu = $true
            single_frame_generation = $true
            recovery_retained_bytes_stable = $true
            same_actual_backend = $true
            activation_latency_report_only = $true
        }
        catalog_fingerprint_sha256 = Get-CanonicalSha256 @($functionalRecords)
        functional_specimens = @($functionalRecords)
    }
    $catalogPath = Join-Path $outputRoot "font-catalog-fingerprint.json"
    Write-AtomicJson $catalogPath $catalogPayload

    function New-FragmentEntry(
        [string] $ArtifactType,
        [string] $ArtifactId,
        [string] $Role,
        [string] $PayloadSchema,
        [string] $ArtifactPath,
        [string[]] $Children
    ) {
        $entry = [ordered]@{
            artifact_type = $ArtifactType
            artifact_id = $ArtifactId
            certification_eligible = $certificationEligible
            role = $Role
            scope = "attribution-ready"
            payload_schema = $PayloadSchema
            path = Split-Path -Leaf $ArtifactPath
            sha256 = Get-FileSha256 $ArtifactPath
            size_bytes = [UInt64] (Get-Item -LiteralPath $ArtifactPath).Length
            producing_command = "pwsh -File scripts/ci/run-stage7-font-proof.ps1"
            producing_argv = @("pwsh", "-File", "scripts/ci/run-stage7-font-proof.ps1")
            source_sha = $sourceSha
            subject_refs = [ordered]@{}
            platform = "windows-x86_64"
            run_id = $runId
            cohort_id = ""
            children = @($Children)
        }
        if ($ArtifactType -cne "runner-fingerprint") {
            $entry.binary_hashes = $initialBinaryHashes
            $entry.runner_fingerprint_sha256 = $runnerFingerprint
        }
        $entry.cohort_id = Get-CanonicalSha256 ([ordered]@{
            scope = $entry.scope
            source_sha = $entry.source_sha
            subject_refs = $entry.subject_refs
            platform = $entry.platform
            binary_hashes = if ($entry.Contains("binary_hashes")) { $entry.binary_hashes } else { $null }
            runner_fingerprint_sha256 = if ($entry.Contains("runner_fingerprint_sha256")) { $entry.runner_fingerprint_sha256 } else { $null }
        })
        return $entry
    }

    $entries = @(
        New-FragmentEntry "font-ownership-raw" "font-ownership-raw" "raw" "rterm.stage7.metric-raw/v1" $rawPath @()
        New-FragmentEntry "font-ownership-aggregate" "font-ownership-aggregate" "aggregate" "rterm.stage7.metric-aggregate/v1" $aggregatePath @("font-ownership-raw")
        New-FragmentEntry "runner-fingerprint" "runner-fingerprint" "proof" "rterm.stage7.result/v1" $runnerPath @()
        New-FragmentEntry "font-catalog-fingerprint" "font-catalog-fingerprint" "proof" "rterm.stage7.result/v1" $catalogPath @()
    )
    $fragment = [ordered]@{
        schema = "rterm.stage7-artifact-manifest-fragment/v1"
        requested_state = "attribution-ready"
        certified_commit = $sourceSha
        epoch_id = Get-CanonicalSha256 ([ordered]@{
            state = "attribution-ready"
            certified_commit = $sourceSha
            rterm = $null
            rterm = $null
        })
        rterm = $null
        rterm = $null
        entries = $entries
    }
    $fragmentPath = Join-Path $outputRoot "artifact-manifest-fragment.json"
    Remove-Item -LiteralPath $stagingRoot -Recurse -Force
    if (Test-Path -LiteralPath $stagingRoot) {
        throw "external atomic-record staging still exists after successful collection"
    }
    try {
        if ($TestRunnerFingerprintFault -ceq "final-identity-failure") {
            throw "simulated final collection identity failpoint"
        }
        Assert-CollectionIdentityUnchanged
        if ($TestRunnerFingerprintFault -ceq "final-summary-failure") {
            throw "simulated final summary serialization failpoint"
        }
        $summaryJson = [ordered]@{
            schema = "rterm.stage7/font-ownership-proof-run/v1"
            ok = $true
            certification_eligible = $certificationEligible
            raw_sample_count = $modes.Count * $MeasuredRounds * $samplesPerProcess
            actual_backend = $script:gpuIdentity.backend
            artifact_manifest_fragment = "artifact-manifest-fragment.json"
        } | ConvertTo-Json -Compress
        if ($TestRunnerFingerprintFault -ceq "final-output-failure") {
            throw "simulated final summary output failpoint"
        }
        Write-Output $summaryJson
        Restore-BenchmarkEnvironment -AllowTestFault
    }
    catch {
        foreach ($atomicRecord in $atomicPayloads) {
            Write-AtomicJson `
                -Path (Join-Path $stagingRoot $atomicRecord.relative_path) `
                -Value $atomicRecord.payload
        }
        throw
    }
    # Fragment publication is deliberately the final potentially failing action.
    Write-AtomicJson $fragmentPath $fragment
}
finally {
    if ($script:benchmarkEnvironmentRestored) {
        # The successful publication path is an explicit no-op after the fragment exists.
    } else {
        Restore-BenchmarkEnvironment
    }
}
