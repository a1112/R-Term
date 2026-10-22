[CmdletBinding()]
param(
  [switch] $ValidationOnly,

  [string] $CandidateRef = $(
    if ([string]::IsNullOrWhiteSpace($env:GITHUB_SHA)) { "HEAD" } else { $env:GITHUB_SHA }
  ),

  [string] $Contract = "scripts/ci/rterm-release-contract.json",

  [string] $OutputDirectory = "artifacts/rterm-release-comparison",

  [ValidateRange(0, 1000)]
  [int] $Warmups = 5,

  [ValidateRange(1, 1000)]
  [int] $Samples = 40,

  [ValidateRange(0.0, 1.0)]
  [double] $RelativeRegressionCeiling = 0.05
)

# Protected fixed-runner comparison for the Stage 6 R-Term release contract.
# Each source is checked out with `git clone --no-local` and receives its own
# Cargo target directory so candidate artifacts cannot contaminate rollback.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

function Test-ValidBaseline([double] $Value) {
  return (
    $Value -gt 0 -and
    -not [double]::IsNaN($Value) -and
    -not [double]::IsInfinity($Value)
  )
}

function Get-RegressionRatio([double] $Candidate, [double] $Baseline) {
  if (-not (Test-ValidBaseline $Baseline)) {
    throw "comparison baseline must be finite and greater than zero"
  }
  if ($Candidate -lt 0 -or [double]::IsNaN($Candidate) -or [double]::IsInfinity($Candidate)) {
    throw "comparison candidate must be finite and non-negative"
  }
  return $Candidate / $Baseline
}

function Test-WithinRelativeCeiling(
  [double] $Candidate,
  [double] $Baseline,
  [double] $Ceiling
) {
  return (Get-RegressionRatio $Candidate $Baseline) -le (1.0 + $Ceiling)
}

if ($ValidationOnly) {
  if (
    (Test-ValidBaseline 0.0) -or
    (Test-ValidBaseline -1.0) -or
    (Test-ValidBaseline ([double]::NaN)) -or
    (Test-ValidBaseline ([double]::PositiveInfinity)) -or
    -not (Test-ValidBaseline 100.0) -or
    -not (Test-WithinRelativeCeiling 105.0 100.0 0.05) -or
    (Test-WithinRelativeCeiling 105.001 100.0 0.05)
  ) {
    throw "R-Term release comparison boundary self-check failed"
  }
  [ordered]@{
    ok = $true
    mode = "validation-only"
    relative_regression_ceiling = $RelativeRegressionCeiling
  } | ConvertTo-Json -Compress
  exit 0
}

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$contractPath = if ([IO.Path]::IsPathRooted($Contract)) {
  $Contract
} else {
  Join-Path $repositoryRoot $Contract
}
$contractDocument = Get-Content -LiteralPath $contractPath -Raw | ConvertFrom-Json
$rollbackRef = [string] $contractDocument.last_known_good_rterm_ref
if ($rollbackRef -notmatch '^[0-9a-f]{40}$') {
  throw "last_known_good_rterm_ref must be an immutable 40-character commit"
}

$outputPath = if ([IO.Path]::IsPathRooted($OutputDirectory)) {
  [IO.Path]::GetFullPath($OutputDirectory)
} else {
  [IO.Path]::GetFullPath((Join-Path $repositoryRoot $OutputDirectory))
}
if (Test-Path -LiteralPath $outputPath) {
  if (@(Get-ChildItem -LiteralPath $outputPath -Force).Count -ne 0) {
    throw "comparison output directory must be absent or empty: $outputPath"
  }
} else {
  New-Item -ItemType Directory -Path $outputPath | Out-Null
}
$workPath = Join-Path $outputPath "work"
New-Item -ItemType Directory -Path $workPath | Out-Null

function Invoke-GitCapture([string[]] $Arguments, [string] $WorkingDirectory) {
  Push-Location $WorkingDirectory
  try {
    $output = & git @Arguments
    if ($LASTEXITCODE -ne 0) {
      throw "git $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
    return ($output -join "`n").Trim()
  } finally {
    Pop-Location
  }
}

function Resolve-Commit([string] $Reference) {
  $resolved = Invoke-GitCapture @("rev-parse", "$Reference^{commit}") $repositoryRoot
  if ($resolved -notmatch '^[0-9a-f]{40}$') {
    throw "reference '$Reference' did not resolve to one commit"
  }
  return $resolved
}

function New-DetachedCheckout([string] $Name, [string] $Commit) {
  $destination = Join-Path $workPath "$Name-source"
  & git clone --no-local --no-checkout $repositoryRoot $destination
  if ($LASTEXITCODE -ne 0) {
    throw "git clone failed for $Name with exit code $LASTEXITCODE"
  }
  $null = Invoke-GitCapture @("checkout", "--detach", $Commit) $destination
  return $destination
}

function Get-MachineFingerprint {
  $processors = @(
    Get-CimInstance Win32_Processor |
      Sort-Object DeviceID |
      ForEach-Object {
        "$($_.Name.Trim())|cores=$($_.NumberOfCores)|logical=$($_.NumberOfLogicalProcessors)"
      }
  ) -join ";"
  $rustc = (& rustc --version).Trim()
  if ($LASTEXITCODE -ne 0) {
    throw "rustc fingerprint failed with exit code $LASTEXITCODE"
  }
  return [ordered]@{
    runner_name = [string] $env:RUNNER_NAME
    os = [Environment]::OSVersion.VersionString
    architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    cpu = $processors
    rustc = $rustc
  }
}

function Convert-FingerprintToKey($Fingerprint) {
  return @(
    $Fingerprint.runner_name,
    $Fingerprint.os,
    $Fingerprint.architecture,
    $Fingerprint.cpu,
    $Fingerprint.rustc
  ) -join "|"
}

function Assert-StartupSummary($Summary, [string] $Name) {
  if (
    $null -eq $Summary -or
    [int] $Summary.warmups -ne $Warmups -or
    [int] $Summary.samples -ne $Samples -or
    @($Summary.samples_detail).Count -ne $Samples
  ) {
    throw "$Name startup samples mismatch"
  }
  if (
    [string] $Summary.profile -ne "release" -or
    [double] $Summary.first_present_ms_p50 -le 0 -or
    [double] $Summary.first_present_ms_p95 -le 0 -or
    [double] $Summary.private_bytes_p95 -le 0 -or
    [double] $Summary.private_bytes_max -le 0 -or
    @($Summary.renderer_values).Count -ne 1 -or
    [string] @($Summary.renderer_values)[0] -ne "cpu"
  ) {
    throw "$Name startup summary is incomplete or incompatible"
  }
}

function New-StartupSummary(
  [System.Collections.Generic.List[object]] $Results,
  [string] $Name
) {
  if ($Results.Count -ne $Samples) {
    throw "$Name startup samples mismatch"
  }
  $orderedTimes = @($Results | ForEach-Object process_to_first_present_ms | Sort-Object)
  $orderedMemory = @($Results | ForEach-Object first_frame_private_bytes | Sort-Object)
  $p50Index = [Math]::Max(0, [Math]::Ceiling($Samples * 0.50) - 1)
  $p95Index = [Math]::Max(0, [Math]::Ceiling($Samples * 0.95) - 1)
  $summary = [ordered]@{
    profile = "release"
    columns = 80
    rows = 24
    dpi_scale = 1.0
    warmups = $Warmups
    samples = $Samples
    first_present_ms_p50 = $orderedTimes[$p50Index]
    first_present_ms_p95 = $orderedTimes[$p95Index]
    private_bytes_p95 = $orderedMemory[$p95Index]
    private_bytes_max = $orderedMemory[-1]
    renderer_values = @($Results | Select-Object -ExpandProperty final_renderer -Unique)
    samples_detail = $Results
  }
  Assert-StartupSummary $summary $Name
  if (
    [double] $summary.first_present_ms_p50 -gt 400 -or
    [double] $summary.first_present_ms_p95 -gt 500
  ) {
    throw "$Name startup first-present contract failed"
  }
  if (
    [uint64] $summary.private_bytes_p95 -gt (55 * 1MB) -or
    [uint64] $summary.private_bytes_max -ge (60 * 1MB)
  ) {
    throw "$Name startup Private Bytes contract failed"
  }
  return $summary
}

function Invoke-SingleStartupSample($Mode) {
  $startupJson = & (Join-Path $repositoryRoot "scripts/ci/run-ssh-gui-startup.ps1") `
    -Profile release -Warmups 0 -Samples 1 -SkipBuild `
    -ExecutablePath ([string] $Mode.executable_path) -CollectOnly
  $startup = ($startupJson -join "`n") | ConvertFrom-Json
  if (@($startup.samples_detail).Count -ne 1) {
    throw "$($Mode.name) single startup sample is incomplete"
  }
  return @($startup.samples_detail)[0]
}

function Invoke-InterleavedStartupComparison($candidate, $rollback) {
  for ($index = 1; $index -le $Warmups; $index++) {
    $candidateFirst = ($index % 2 -eq 1)
    $warmupOrder = if ($candidateFirst) {
      @($candidate, $rollback)
    } else {
      @($rollback, $candidate)
    }
    foreach ($mode in $warmupOrder) {
      $null = Invoke-SingleStartupSample $mode
    }
  }

  $candidateResults = [System.Collections.Generic.List[object]]::new()
  $rollbackResults = [System.Collections.Generic.List[object]]::new()
  for ($index = 1; $index -le $Samples; $index++) {
    $candidateFirst = ($index % 2 -eq 1)
    $measurementOrder = if ($candidateFirst) {
      @($candidate, $rollback)
    } else {
      @($rollback, $candidate)
    }
    foreach ($mode in $measurementOrder) {
      $sample = Invoke-SingleStartupSample $mode
      $detail = [pscustomobject]@{
        sample = $index
        process_to_first_present_ms = [double] $sample.process_to_first_present_ms
        reported_process_to_first_present_ms = [double] $sample.reported_process_to_first_present_ms
        first_frame_private_bytes = [uint64] $sample.first_frame_private_bytes
        final_renderer = [string] $sample.final_renderer
      }
      if ([string] $mode.name -eq "candidate") {
        $candidateResults.Add($detail)
      } else {
        $rollbackResults.Add($detail)
      }
    }
  }
  return [ordered]@{
    measurement_order = "alternating-ab-ba"
    candidate = New-StartupSummary $candidateResults "candidate"
    rollback = New-StartupSummary $rollbackResults "rollback"
  }
}

function Invoke-RtermReleaseMode(
  [string] $Name,
  [string] $Commit,
  [string] $TargetDirectoryName
) {
  $sourcePath = New-DetachedCheckout $Name $Commit
  $targetPath = Join-Path $workPath $TargetDirectoryName
  $packageParent = Join-Path $workPath "$Name-package"
  $packageRoot = Join-Path $packageParent "R-Term-$Name-windows-x64-unsigned"
  $artifactName = "R-Term-$Name-windows-x64-unsigned.zip"
  New-Item -ItemType Directory -Path $packageParent | Out-Null

  $oldTarget = $env:CARGO_TARGET_DIR
  $oldSourceCommit = $env:GITHUB_SHA
  $env:CARGO_TARGET_DIR = $targetPath
  $env:GITHUB_SHA = $Commit
  try {
    Push-Location $sourcePath
    try {
      & cargo build --locked --release -p rterm-app --no-default-features --features production-gui
      if ($LASTEXITCODE -ne 0) {
        throw "$Name production-gui build failed with exit code $LASTEXITCODE"
      }

      $binary = Join-Path $targetPath "release/rterm.exe"
      $versionJson = & $binary version --json
      if ($LASTEXITCODE -ne 0) {
        throw "$Name version query failed with exit code $LASTEXITCODE"
      }
      $version = ($versionJson -join "`n") | ConvertFrom-Json

      & (Join-Path $sourcePath "scripts/ci/package-native.ps1") `
        -Binary $binary `
        -PackageRoot $packageRoot `
        -ArtifactName $artifactName `
        -RuntimeTarget "windows-x86_64" `
        -PtyBackend "windows-conpty" `
        -Version ([string] $version.version) `
        -Unsigned

      & (Join-Path $sourcePath "scripts/ci/package-smoke.ps1") `
        -PackageRoot $packageRoot `
        -ExpectedTarget "windows-x86_64" `
        -ExpectedPtyBackend "windows-conpty" `
        -ExpectedArtifactName $artifactName `
        -ExpectedUnsigned
    } finally {
      Pop-Location
    }
    return [ordered]@{
      name = $Name
      commit = $Commit
      source_path = $sourcePath
      cargo_target = $targetPath
      executable_path = $binary
      machine = Get-MachineFingerprint
      startup = $null
      package = [ordered]@{
        root = $packageRoot
        artifact = Join-Path $packageParent $artifactName
        smoke = "passed"
      }
    }
  } finally {
    $env:CARGO_TARGET_DIR = $oldTarget
    $env:GITHUB_SHA = $oldSourceCommit
  }
}

$candidateCommit = Resolve-Commit $CandidateRef
$rollbackCommit = Resolve-Commit $rollbackRef
$candidate = Invoke-RtermReleaseMode "candidate" $candidateCommit "candidate-target"
$rollback = Invoke-RtermReleaseMode "rollback" $rollbackCommit "rollback-target"
$startupComparison = Invoke-InterleavedStartupComparison $candidate $rollback
$candidate["startup"] = $startupComparison.candidate
$rollback["startup"] = $startupComparison.rollback

$thresholdViolations = [System.Collections.Generic.List[object]]::new()
$candidateFingerprint = Convert-FingerprintToKey $candidate.machine
$rollbackFingerprint = Convert-FingerprintToKey $rollback.machine
if ($candidateFingerprint -ne $rollbackFingerprint) {
  $thresholdViolations.Add([ordered]@{
    metric = "machine_fingerprint"
    observed = $candidateFingerprint
    expected = $rollbackFingerprint
    reason = "machine fingerprint mismatch"
  })
}

$firstPresentRatio = Get-RegressionRatio `
  ([double] $candidate.startup.first_present_ms_p95) `
  ([double] $rollback.startup.first_present_ms_p95)
$privateBytesRatio = Get-RegressionRatio `
  ([double] $candidate.startup.private_bytes_p95) `
  ([double] $rollback.startup.private_bytes_p95)
$ratioLimit = 1.0 + $RelativeRegressionCeiling
foreach ($comparison in @(
  @("first_present_p95_ratio", $firstPresentRatio),
  @("private_bytes_p95_ratio", $privateBytesRatio)
)) {
  if ([double] $comparison[1] -gt $ratioLimit) {
    $thresholdViolations.Add([ordered]@{
      metric = [string] $comparison[0]
      observed = [double] $comparison[1]
      expected = "<= $ratioLimit"
    })
  }
}

$report = [ordered]@{
  schema_version = 1
  ok = ($thresholdViolations.Count -eq 0)
  mode = "fixed-windows-release-comparison"
  warmups = $Warmups
  samples = $Samples
  relative_regression_ceiling = $RelativeRegressionCeiling
  measurement_order = $startupComparison.measurement_order
  candidate = $candidate
  rollback = $rollback
  comparison = [ordered]@{
    first_present_p95_ratio = $firstPresentRatio
    private_bytes_p95_ratio = $privateBytesRatio
    ratio_limit = $ratioLimit
  }
  threshold_violations = @($thresholdViolations)
}
$reportPath = Join-Path $outputPath "report.json"
$temporaryReport = Join-Path $outputPath ("report-{0}.tmp" -f [Guid]::NewGuid().ToString("N"))
[IO.File]::WriteAllText(
  $temporaryReport,
  (($report | ConvertTo-Json -Depth 12) + [Environment]::NewLine),
  [Text.UTF8Encoding]::new($false)
)
Move-Item -LiteralPath $temporaryReport -Destination $reportPath

$report | ConvertTo-Json -Depth 12 -Compress
if (-not $report.ok) {
  throw "R-Term release comparison failed; see $reportPath"
}
