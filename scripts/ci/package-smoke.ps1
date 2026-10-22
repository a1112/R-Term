[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $PackageRoot,
  [Parameter(Mandatory)] [string] $ExpectedTarget,
  [Parameter(Mandatory)]
  [ValidateSet("windows-conpty", "unix-pty")]
  [string] $ExpectedPtyBackend,
  [Parameter(Mandatory)] [string] $ExpectedArtifactName,
  [switch] $ExpectedUnsigned
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$harness = Join-Path $PSScriptRoot "process-harness.ps1"
if (-not (Test-Path -LiteralPath $harness -PathType Leaf)) {
  throw "shared process harness is missing: $harness"
}
. $harness

$packageRootPath = (Resolve-Path -LiteralPath $PackageRoot).Path

function Get-PackageRelativePath([string] $Root, [string] $Path) {
  $normalizedRoot = [IO.Path]::GetFullPath($Root).TrimEnd([char[]]@('\', '/'))
  $normalizedPath = [IO.Path]::GetFullPath($Path)
  $prefix = $normalizedRoot + [IO.Path]::DirectorySeparatorChar
  if (-not $normalizedPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "package path escapes root: $normalizedPath"
  }
  return $normalizedPath.Substring($prefix.Length).Replace('\', '/')
}
$manifestPath = Join-Path $packageRootPath "manifest.json"
$checksumsPath = Join-Path $packageRootPath "SHA256SUMS"
foreach ($required in @("manifest.json", "SHA256SUMS", "README.md", "LICENSE", "examples/rterm-profiles.toml")) {
  if (-not (Test-Path -LiteralPath (Join-Path $packageRootPath $required) -PathType Leaf)) {
    throw "unpacked package is missing required file: $required"
  }
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.schema_version -ne 1) { throw "unsupported package manifest schema" }
if ($manifest.artifact.name -ne $ExpectedArtifactName) {
  throw "artifact name mismatch: '$($manifest.artifact.name)' != '$ExpectedArtifactName'"
}
if ($manifest.artifact.runtime_target -ne $ExpectedTarget) {
  throw "manifest runtime target mismatch: '$($manifest.artifact.runtime_target)' != '$ExpectedTarget'"
}
if ($manifest.artifact.pty_backend -ne $ExpectedPtyBackend) {
  throw "manifest PTY backend mismatch: '$($manifest.artifact.pty_backend)' != '$ExpectedPtyBackend'"
}
if ([bool]$manifest.signing.unsigned -ne [bool]$ExpectedUnsigned) {
  throw "manifest unsigned state mismatch"
}
if ($ExpectedUnsigned -and -not $ExpectedArtifactName.Contains("-unsigned")) {
  throw "expected unsigned artifact name must contain -unsigned"
}
foreach ($required in $manifest.required_files) {
  if (-not (Test-Path -LiteralPath (Join-Path $packageRootPath $required) -PathType Leaf)) {
    throw "manifest required file is missing: $required"
  }
}
$manifestFiles = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($entry in $manifest.files) {
  $relative = ([string]$entry.path).Replace('\', '/')
  if ([IO.Path]::IsPathRooted($relative) -or $relative.Split('/') -contains '..') {
    throw "unsafe manifest file path: $relative"
  }
  if (-not $manifestFiles.Add($relative)) { throw "duplicate manifest file path: $relative" }
  $candidate = Join-Path $packageRootPath $relative
  if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { throw "manifest file is missing: $relative" }
  $file = Get-Item -LiteralPath $candidate
  if ($file.Length -ne [long]$entry.size) { throw "manifest file size mismatch: $relative" }
  $actualHash = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actualHash -ne ([string]$entry.sha256).ToLowerInvariant()) {
    throw "manifest file checksum mismatch: $relative"
  }
}
foreach ($file in Get-ChildItem -LiteralPath $packageRootPath -File -Recurse) {
  $relative = Get-PackageRelativePath -Root $packageRootPath -Path $file.FullName
  if ($relative -notin @("manifest.json", "SHA256SUMS") -and -not $manifestFiles.Contains($relative)) {
    throw "payload file is not covered by manifest.files: $relative"
  }
}

$listed = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($line in Get-Content -LiteralPath $checksumsPath) {
  if ($line -notmatch '^([0-9a-fA-F]{64})  (.+)$') { throw "invalid SHA256SUMS line: $line" }
  $expectedHash = $Matches[1].ToLowerInvariant()
  $relative = $Matches[2].Replace('\', '/')
  if ([IO.Path]::IsPathRooted($relative) -or $relative.Split('/') -contains '..') {
    throw "unsafe SHA256SUMS path: $relative"
  }
  if (-not $listed.Add($relative)) { throw "duplicate SHA256SUMS path: $relative" }
  $candidate = Join-Path $packageRootPath $relative
  if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { throw "checksummed file is missing: $relative" }
  $actualHash = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actualHash -ne $expectedHash) { throw "checksum mismatch: $relative" }
}
foreach ($file in Get-ChildItem -LiteralPath $packageRootPath -File -Recurse) {
  $relative = Get-PackageRelativePath -Root $packageRootPath -Path $file.FullName
  if ($relative -ne "SHA256SUMS" -and -not $listed.Contains($relative)) {
    throw "package file is not covered by SHA256SUMS: $relative"
  }
}

if (@($manifest.requirements.external_tools).Count -ne 0) { throw "R-Term local terminal must not require external transport tools" }

$binary = (Resolve-Path -LiteralPath (Join-Path $packageRootPath $manifest.artifact.binary)).Path
if (-not $binary.StartsWith($packageRootPath + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
  throw "manifest binary escapes package root"
}
$nativeHarness = Join-Path $PSScriptRoot "run-native-window.ps1"
$pwshCommand = Get-Command pwsh -ErrorAction SilentlyContinue
$powerShellHost = if ($null -ne $pwshCommand) {
  $pwshCommand.Source
} else {
  (Get-Process -Id $PID).Path
}

$originalLocation = Get-Location
$oldExecutable = $env:RTERM_TEST_APP_EXECUTABLE
$oldRequireOpenSsh = $env:RTERM_REQUIRE_OPENSSH
try {
  Set-Location $repositoryRoot
  $null = Invoke-BoundedProcess -Phase "shared process harness self-test" -FilePath $powerShellHost -ArgumentList @("-NoProfile", "-File", $nativeHarness, "-HarnessSelfTest") -TimeoutSeconds 90

  $versionResult = Invoke-BoundedProcess -Phase "packaged version" -FilePath $binary -ArgumentList @("version", "--json") -TimeoutSeconds 30
  $version = $versionResult.Stdout | ConvertFrom-Json
  if ($version.target -ne $ExpectedTarget) { throw "version target mismatch: '$($version.target)' != '$ExpectedTarget'" }
  if ($version.pty_backend -ne $ExpectedPtyBackend) { throw "version PTY backend mismatch" }
  if ($version.version -ne $manifest.package.version) { throw "version mismatch with package manifest" }
  if ($version.name -ne "rterm") { throw "packaged command identity is not rterm" }
  if ($version.native_ssh_backend -notin @("disabled", "russh")) { throw "unknown packaged native SSH backend" }

  $launcher = Join-Path $packageRootPath "rterm-console.cmd"
  if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) { throw "Windows console launcher is missing" }
  $null = Invoke-BoundedProcess -Phase "packaged launcher preflight" -FilePath $launcher -ArgumentList @("--preflight", "--", "cmd.exe", "/D", "/C", "echo", "package-launcher-smoke") -TimeoutSeconds 30

  $env:RTERM_TEST_APP_EXECUTABLE = $binary
  if ($version.native_ssh_backend -eq "russh") {
    $env:RTERM_REQUIRE_OPENSSH = "1"
    $null = Invoke-BoundedProcess -Phase "packaged OpenSSH loopback" -FilePath "cargo" -ArgumentList @("test", "--locked", "-p", "rterm-app", "--features", "ssh", "--test", "openssh_loopback", "rterm_app_system_openssh_entrypoint_runs_a_real_loopback_exec", "--", "--exact", "--nocapture") -TimeoutSeconds 300
  } else {
    foreach ($remoteCommand in @("ssh", "scp", "sftp")) {
      $rejected = $false
      try {
        $null = Invoke-BoundedProcess -Phase "packaged disabled $remoteCommand" -FilePath $binary -ArgumentList @($remoteCommand, "user@must-not-resolve.invalid") -TimeoutSeconds 10
      } catch {
        if ($_.Exception.Message -notmatch "SSH support is disabled") { throw }
        $rejected = $true
      }
      if (-not $rejected) { throw "SSH-free package accepted $remoteCommand" }
    }
  }
  $null = Invoke-BoundedProcess -Phase "packaged native ten-frame E2E" -FilePath "cargo" -ArgumentList @("test", "--locked", "-p", "rterm-app", "--test", "native_window_e2e", "native_window_e2e_presents_ten_frames_from_a_real_pty", "--", "--exact", "--nocapture") -TimeoutSeconds 240
} finally {
  $env:RTERM_TEST_APP_EXECUTABLE = $oldExecutable
  $env:RTERM_REQUIRE_OPENSSH = $oldRequireOpenSsh
  Set-Location $originalLocation
}
