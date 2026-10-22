[CmdletBinding(SupportsShouldProcess)]
param(
    [string] $OutputDirectory = "artifacts/stage7-attribution-deterministic-tests",
    [ValidateRange(60, 3600)]
    [int] $ProcessTimeoutSeconds = 1800,
    [string] $RunnerFingerprintPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

$suiteId = "stage7-attribution-deterministic-v1"
[string[]] $cargoCommand = @(
    "cargo",
    "test",
    "--locked",
    "-p",
    "rterm-app",
    "--test",
    "gpu_backend_memory_matrix_behavior",
    "stage7_attribution",
    "-j1"
)
[string[]] $pythonCommand = @(
    "python",
    "-m",
    "unittest",
    "scripts.ci.tests.test_check_stage7_split_gate.Stage7SplitGateTests.test_attribution_matrix",
    "-v"
)
$plannedCommands = [System.Collections.Generic.List[object]]::new()
$plannedCommands.Add($cargoCommand)
$plannedCommands.Add($pythonCommand)
$plan = [ordered]@{
    schema = "rterm.stage7/attribution-deterministic-tests-plan/v1"
    suite_id = $suiteId
    commands = @($plannedCommands)
    process_timeout_seconds = $ProcessTimeoutSeconds
    artifacts = @(
        "attribution-deterministic-tests.json",
        "artifact-manifest-fragment.json"
    )
}

if ($WhatIfPreference) {
    $plan | ConvertTo-Json -Depth 20 -Compress
    return
}

if (-not $IsWindows) {
    throw "Stage 7 attribution deterministic proof requires Windows"
}

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    throw "CARGO_TARGET_DIR is required"
}
if (-not [IO.Path]::IsPathFullyQualified($env:CARGO_TARGET_DIR)) {
    throw "CARGO_TARGET_DIR must be absolute"
}
$targetRoot = [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
$repoBoundary = $repoRoot.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
$targetBoundary = $targetRoot.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
$repoPrefix = $repoBoundary + [IO.Path]::DirectorySeparatorChar
if ($targetBoundary.Equals($repoBoundary, [StringComparison]::OrdinalIgnoreCase) -or $targetBoundary.StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "CARGO_TARGET_DIR must be outside the repository"
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

$sourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceSha -cnotmatch "^[0-9a-f]{40}$") {
    throw "unable to resolve one immutable source commit"
}
$dirty = @(& git -C $repoRoot status --porcelain)
if ($LASTEXITCODE -ne 0 -or $dirty.Count -ne 0) {
    throw "attribution deterministic proof requires a clean source tree including untracked files"
}

$app = Join-Path (Join-Path $targetRoot "release") "rterm.exe"
$launcher = Join-Path (Join-Path $targetRoot "release") "rterm-bench-launcher.exe"
foreach ($binary in @($app, $launcher)) {
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "required release binary is missing: $binary"
    }
}

if ([string]::IsNullOrWhiteSpace($RunnerFingerprintPath)) {
    $RunnerFingerprintPath = Join-Path (Split-Path -Parent $outputRoot) "font/runner-fingerprint.json"
}
if (-not (Test-Path -LiteralPath $RunnerFingerprintPath -PathType Leaf)) {
    throw "runner fingerprint proof is missing"
}
$runnerProof = Get-Content -LiteralPath $RunnerFingerprintPath -Raw | ConvertFrom-Json
if ($runnerProof.schema -cne "rterm.stage7.result/v1" -or $runnerProof.proof -cne "runner-fingerprint" -or $runnerProof.ok -ne $true) {
    throw "runner fingerprint proof is invalid"
}
if ($runnerProof.identity.source_sha -cne $sourceSha) {
    throw "runner fingerprint source differs from the deterministic proof source"
}
$runnerFingerprint = [string] $runnerProof.fingerprint_sha256
if ($runnerFingerprint -cnotmatch "^[0-9a-f]{64}$") {
    throw "runner fingerprint digest is invalid"
}

function Get-FileSha256([string] $Path) {
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function ConvertTo-CanonicalValue([AllowNull()] [object] $Value) {
    if ($null -eq $Value) { return $null }
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

function Get-CanonicalSha256([object] $Value) {
    $json = ConvertTo-CanonicalValue $Value | ConvertTo-Json -Depth 100 -Compress
    $bytes = [Text.Encoding]::UTF8.GetBytes($json)
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
}

function Write-AtomicJson([string] $Path, [object] $Value) {
    $directory = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
    $temporary = Join-Path ([IO.Path]::GetTempPath()) ("rterm-stage7-attribution-tests-{0}-{1}.tmp" -f $PID, [Guid]::NewGuid().ToString("N"))
    try {
        $json = $Value | ConvertTo-Json -Depth 100
        [IO.File]::WriteAllText($temporary, $json, [Text.UTF8Encoding]::new($false))
        Move-Item -LiteralPath $temporary -Destination $Path -Force
    } finally {
        if (Test-Path -LiteralPath $temporary) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

$binaryHashes = [ordered]@{
    "rterm.exe" = Get-FileSha256 $app
    "rterm-bench-launcher.exe" = Get-FileSha256 $launcher
}
$runnerProofHash = Get-FileSha256 $RunnerFingerprintPath

function Assert-IdentityUnchanged {
    $currentSource = (& git -C $repoRoot rev-parse HEAD).Trim()
    $currentDirty = @(& git -C $repoRoot status --porcelain)
    if ($LASTEXITCODE -ne 0 -or $currentSource -cne $sourceSha -or $currentDirty.Count -ne 0) {
        throw "source identity changed during deterministic proof"
    }
    if ((Get-FileSha256 $app) -cne $binaryHashes["rterm.exe"] -or (Get-FileSha256 $launcher) -cne $binaryHashes["rterm-bench-launcher.exe"]) {
        throw "release binary identity changed during deterministic proof"
    }
    if ((Get-FileSha256 $RunnerFingerprintPath) -cne $runnerProofHash) {
        throw "runner fingerprint identity changed during deterministic proof"
    }
}

. (Join-Path $PSScriptRoot "process-harness.ps1")
$null = Invoke-BoundedProcess `
    -Phase "Stage 7 attribution Rust deterministic tests" `
    -FilePath "cargo.exe" `
    -ArgumentList @($cargoCommand | Select-Object -Skip 1) `
    -TimeoutSeconds $ProcessTimeoutSeconds
Assert-IdentityUnchanged
$null = Invoke-BoundedProcess `
    -Phase "Stage 7 attribution Python deterministic tests" `
    -FilePath "python.exe" `
    -ArgumentList @($pythonCommand | Select-Object -Skip 1) `
    -TimeoutSeconds $ProcessTimeoutSeconds
Assert-IdentityUnchanged

$runId = "stage7-attribution-tests-{0}-{1}" -f ([DateTimeOffset]::UtcNow.ToUnixTimeSeconds()), $PID
$identity = [ordered]@{
    source_sha = $sourceSha
    platform = "windows-x86_64"
    run_id = $runId
    binary_hashes = $binaryHashes
    runner_fingerprint_sha256 = $runnerFingerprint
}
$proofPath = Join-Path $outputRoot "attribution-deterministic-tests.json"
$proof = [ordered]@{
    schema = "rterm.stage7.result/v1"
    identity = $identity
    ok = $true
    proof = "attribution-deterministic-tests"
    claims = [ordered]@{
        suite_id = $suiteId
        passed = $true
    }
}
Write-AtomicJson -Path $proofPath -Value $proof

$subjectRefs = [ordered]@{}
$scope = "attribution-ready"
$cohortId = Get-CanonicalSha256 ([ordered]@{
    scope = $scope
    source_sha = $sourceSha
    subject_refs = $subjectRefs
    platform = "windows-x86_64"
    binary_hashes = $binaryHashes
    runner_fingerprint_sha256 = $runnerFingerprint
})
$entry = [ordered]@{
    artifact_type = "attribution-deterministic-tests"
    artifact_id = "attribution-deterministic-tests"
    role = "proof"
    scope = $scope
    payload_schema = "rterm.stage7.result/v1"
    path = "attribution-deterministic-tests.json"
    sha256 = Get-FileSha256 $proofPath
    size_bytes = [UInt64] (Get-Item -LiteralPath $proofPath).Length
    producing_command = "pwsh -File scripts/ci/run-stage7-attribution-deterministic-tests.ps1"
    producing_argv = @("pwsh", "-File", "scripts/ci/run-stage7-attribution-deterministic-tests.ps1")
    source_sha = $sourceSha
    subject_refs = $subjectRefs
    binary_hashes = $binaryHashes
    runner_fingerprint_sha256 = $runnerFingerprint
    platform = "windows-x86_64"
    run_id = $runId
    cohort_id = $cohortId
    children = @()
}
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
    entries = @($entry)
}
Write-AtomicJson -Path (Join-Path $outputRoot "artifact-manifest-fragment.json") -Value $fragment

[ordered]@{
    schema = "rterm.stage7/attribution-deterministic-tests-run/v1"
    ok = $true
    suite_id = $suiteId
    artifact_manifest_fragment = "artifact-manifest-fragment.json"
} | ConvertTo-Json -Compress
