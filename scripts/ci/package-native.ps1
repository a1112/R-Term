[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $Binary,
  [Parameter(Mandatory)] [string] $PackageRoot,
  [Parameter(Mandatory)] [string] $ArtifactName,
  [Parameter(Mandatory)] [string] $RuntimeTarget,
  [Parameter(Mandatory)]
  [ValidateSet("windows-conpty", "unix-pty")]
  [string] $PtyBackend,
  [Parameter(Mandatory)] [string] $Version,
  [switch] $Unsigned
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$runtimeContracts = @{
  "windows-x86_64" = @{ RustTarget = "x86_64-pc-windows-msvc"; Binary = "rterm.exe" }
  "windows-aarch64" = @{ RustTarget = "aarch64-pc-windows-msvc"; Binary = "rterm.exe" }
}
if (-not $runtimeContracts.ContainsKey($RuntimeTarget)) {
  throw "package-native.ps1 only assembles Windows targets, not '$RuntimeTarget'"
}
if ($PtyBackend -ne "windows-conpty") {
  throw "Windows package requires windows-conpty, not '$PtyBackend'"
}
if (-not $ArtifactName.EndsWith(".zip", [StringComparison]::Ordinal)) {
  throw "Windows artifact name must end in .zip"
}
if ($Unsigned -and -not $ArtifactName.EndsWith("-unsigned.zip", [StringComparison]::Ordinal)) {
  throw "unsigned Windows artifact name must end in -unsigned.zip"
}
if (-not $Unsigned -and $ArtifactName.Contains("-unsigned")) {
  throw "release-candidate artifact name must not contain -unsigned"
}

if (Test-Path -LiteralPath $PackageRoot) {
  if (@(Get-ChildItem -LiteralPath $PackageRoot -Force).Count -ne 0) {
    throw "package root must be absent or empty: $PackageRoot"
  }
} else {
  New-Item -ItemType Directory -Path $PackageRoot | Out-Null
}
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

Copy-Item -LiteralPath $binaryPath -Destination (Join-Path $packageRootPath "rterm.exe")
Copy-Item -LiteralPath (Join-Path $repositoryRoot "packaging/rterm-console.cmd") -Destination $packageRootPath
Copy-Item -LiteralPath (Join-Path $repositoryRoot "README.md") -Destination $packageRootPath
Copy-Item -LiteralPath (Join-Path $repositoryRoot "LICENSE") -Destination $packageRootPath
Copy-Item -LiteralPath (Join-Path $repositoryRoot "NOTICE") -Destination $packageRootPath
New-Item -ItemType Directory -Path (Join-Path $packageRootPath "examples") | Out-Null
Copy-Item -LiteralPath (Join-Path $repositoryRoot "examples/rterm-profiles.toml") -Destination (Join-Path $packageRootPath "examples")
New-Item -ItemType Directory -Path (Join-Path $packageRootPath "licenses/fonts/LICENSES") -Force | Out-Null
Copy-Item -Path (Join-Path $repositoryRoot "tests/fixtures/fonts/LICENSES/*") -Destination (Join-Path $packageRootPath "licenses/fonts/LICENSES")
Copy-Item -LiteralPath (Join-Path $repositoryRoot "tests/fixtures/fonts/MANIFEST.tsv") -Destination (Join-Path $packageRootPath "licenses/fonts/MANIFEST.tsv")

$payloadFiles = @(Get-ChildItem -LiteralPath $packageRootPath -File -Recurse | Sort-Object FullName)
$fileEntries = foreach ($file in $payloadFiles) {
  $relative = Get-PackageRelativePath -Root $packageRootPath -Path $file.FullName
  [ordered]@{
    path = $relative
    size = $file.Length
    sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  }
}
$requiredFiles = @(
  "rterm.exe",
  "rterm-console.cmd",
  "README.md",
  "LICENSE",
  "examples/rterm-profiles.toml",
  "licenses/fonts/MANIFEST.tsv"
)
$sourceCommit = if ([string]::IsNullOrWhiteSpace($env:GITHUB_SHA)) { "local" } else { $env:GITHUB_SHA }
$manifest = [ordered]@{
  schema_version = 1
  package = [ordered]@{ name = "R-Term"; version = $Version; source_commit = $sourceCommit }
  artifact = [ordered]@{
    name = $ArtifactName
    format = "zip"
    rust_target = $runtimeContracts[$RuntimeTarget].RustTarget
    runtime_target = $RuntimeTarget
    pty_backend = $PtyBackend
    binary = "rterm.exe"
  }
  signing = [ordered]@{
    status = $(if ($Unsigned) { "unsigned" } else { "pending-protected-signing" })
    unsigned = [bool]$Unsigned
  }
  requirements = [ordered]@{
    external_tools = @()
    optional_external_tools = [ordered]@{ "system-openssh" = @("ssh"); "transfer-tools" = @("sftp", "scp") }
  }
  required_files = $requiredFiles
  files = @($fileEntries)
}
$manifestJson = ($manifest | ConvertTo-Json -Depth 8) + [Environment]::NewLine
[IO.File]::WriteAllText(
  (Join-Path $packageRootPath "manifest.json"),
  $manifestJson,
  [Text.UTF8Encoding]::new($false)
)

$checksummedFiles = @(Get-ChildItem -LiteralPath $packageRootPath -File -Recurse | Sort-Object FullName)
$checksumLines = foreach ($file in $checksummedFiles) {
  $relative = Get-PackageRelativePath -Root $packageRootPath -Path $file.FullName
  $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  "$hash  $relative"
}
[IO.File]::WriteAllLines(
  (Join-Path $packageRootPath "SHA256SUMS"),
  [string[]]$checksumLines,
  [Text.Encoding]::ASCII
)

$artifactPath = Join-Path (Split-Path -Parent $packageRootPath) $ArtifactName
Compress-Archive -Path $packageRootPath -DestinationPath $artifactPath -CompressionLevel Optimal -Force
