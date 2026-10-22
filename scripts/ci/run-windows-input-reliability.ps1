[CmdletBinding()]
param(
  [string] $OutputDirectory = "evidence/windows-input-reliability",
  [ValidateRange(1, 20)] [int] $Rounds = 3,
  [switch] $PlanOnly,
  [switch] $SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "process-harness.ps1")

function Assert-SinglePassingTest([string] $Output) {
  $summaries = [regex]::Matches($Output, '(?m)^test result: ([^\r\n]+)')
  if ($summaries.Count -ne 1 -or $summaries[0].Groups[1].Value -notmatch '^ok\. 1 passed; 0 failed; 0 ignored;') {
    throw "Expected exactly one executed, passing test; zero tests or ignored tests cannot certify a case"
  }
}

if ($SelfTest) {
  Assert-SinglePassingTest "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.01s"
  foreach ($invalid in @(
    "",
    "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured",
    "test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured",
    "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured",
    "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured",
    "test result: ok. 1 passed; 0 failed; 0 ignored;`ntest result: ok. 1 passed; 0 failed; 0 ignored;"
  )) {
    $rejected = $false
    try { Assert-SinglePassingTest $invalid } catch { $rejected = $true }
    if (-not $rejected) { throw "Invalid libtest evidence was accepted" }
  }
  Write-Output "Result validation self-test passed"
  return
}

$manifestPath = Join-Path $PSScriptRoot "windows-input-reliability.json"
$matrix = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($matrix.schema -ne 1 -or $matrix.cases.Count -eq 0) { throw "Invalid acceptance matrix" }
$identities = @($matrix.cases | ForEach-Object { "$($_.target)::$($_.name)" })
if (@($identities | Sort-Object -Unique).Count -ne $identities.Count) { throw "Duplicate case identity" }
if ($PlanOnly) {
  [ordered]@{
    schema = 1; rounds = $Rounds; executions = $Rounds * $matrix.cases.Count
    cases = $matrix.cases; manual_pending = $matrix.manual_pending
    known_open_checks = $matrix.known_open_checks
  } | ConvertTo-Json -Depth 6
  return
}

if (-not $IsWindows) { throw "This runner requires Windows and PowerShell 7" }
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
if ((Test-Path -LiteralPath $outputRoot) -and @(Get-ChildItem -LiteralPath $outputRoot -Force).Count -gt 0) {
  throw "Evidence directory must be absent or empty: $outputRoot"
}
$null = New-Item -ItemType Directory -Force -Path $outputRoot
$results = [Collections.Generic.List[object]]::new()
$artifacts = @{}
$products = @{}
$previousRequiredSsh = $env:RTERM_REQUIRE_OPENSSH
$report = [ordered]@{
  schema = 1; status = "running"; started_utc = [DateTimeOffset]::UtcNow.ToString("o")
  source_commit = $null; source_diff_sha256 = $null; source_status = $null
  matrix_sha256 = (Get-FileHash $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
  os_version = [Environment]::OSVersion.VersionString; rounds = $Rounds
  expected_executions = $Rounds * $matrix.cases.Count; results = $results
  artifacts = $artifacts; product_artifacts = $products; toolchain = $null
  manual_pending = $matrix.manual_pending
  known_open_checks = $matrix.known_open_checks; error = $null
}
try {
  $identity = Invoke-BoundedProcess -Phase "source commit" -FilePath git -ArgumentList @("rev-parse", "HEAD") -TimeoutSeconds 30
  $report.source_commit = $identity.Stdout.Trim()
  $diff = Invoke-BoundedProcess -Phase "source diff" -FilePath git -ArgumentList @("diff", "--binary", "HEAD") -TimeoutSeconds 30
  [IO.File]::WriteAllText((Join-Path $outputRoot "source.patch"), $diff.Stdout)
  $report.source_diff_sha256 = (Get-FileHash (Join-Path $outputRoot "source.patch") -Algorithm SHA256).Hash.ToLowerInvariant()
  $state = Invoke-BoundedProcess -Phase "source status" -FilePath git -ArgumentList @("status", "--porcelain") -TimeoutSeconds 30
  $report.source_status = $state.Stdout
  Copy-Item -LiteralPath $manifestPath -Destination (Join-Path $outputRoot "matrix.json")
  Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $outputRoot "runner.ps1")
  Assert-BoundedProcessHarness
  $toolchain = Invoke-BoundedProcess -Phase "Rust toolchain" -FilePath rustc -ArgumentList @("-Vv") -TimeoutSeconds 30
  $report.toolchain = $toolchain.Stdout.Trim()
  $env:RTERM_REQUIRE_OPENSSH = "1"
  $null = Invoke-BoundedProcess -Phase "required OpenSSH" -FilePath ssh -ArgumentList @("-V") -TimeoutSeconds 15
  $build = Invoke-BoundedProcess -Phase "acceptance build" -FilePath cargo -ArgumentList @(
    "test", "--locked", "-p", "rterm-app", "--features", "developer-full", "--bin", "rterm", "--test", "local_pty",
    "--test", "openssh_loopback", "--no-run", "--message-format=json"
  ) -TimeoutSeconds 1200
  [IO.File]::WriteAllText((Join-Path $outputRoot "build.stdout.jsonl"), $build.Stdout)
  [IO.File]::WriteAllText((Join-Path $outputRoot "build.stderr.log"), $build.Stderr)
  foreach ($line in ($build.Stdout -split "`n")) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    $record = $line | ConvertFrom-Json
    if ($record.reason -eq "compiler-artifact" -and $record.executable) {
      $artifact = [ordered]@{
        path = $record.executable
        sha256 = (Get-FileHash -LiteralPath $record.executable -Algorithm SHA256).Hash.ToLowerInvariant()
      }
      if ($record.profile.test) { $artifacts[$record.target.name] = $artifact }
      else { $products[$record.target.name] = $artifact }
    }
  }
  foreach ($case in $matrix.cases) {
    if (-not $artifacts.ContainsKey($case.target)) { throw "Missing test artifact: $($case.target)" }
  }
  if (-not $products.ContainsKey("rterm")) { throw "Missing application artifact" }
  for ($round = 1; $round -le $Rounds; $round++) {
    foreach ($case in $matrix.cases) {
      $index = $results.Count + 1
      $entry = [ordered]@{ round = $round; target = $case.target; name = $case.name; group = $case.group; status = "failed" }
      $results.Add($entry)
      $run = Invoke-BoundedProcess -Phase "round $round $($case.name)" -FilePath $artifacts[$case.target].path -ArgumentList @($case.name, "--exact", "--nocapture") -TimeoutSeconds 120
      [IO.File]::WriteAllText((Join-Path $outputRoot "$index.stdout.log"), $run.Stdout)
      [IO.File]::WriteAllText((Join-Path $outputRoot "$index.stderr.log"), $run.Stderr)
      Assert-SinglePassingTest $run.Stdout
      $entry.status = "passed"
    }
  }
  foreach ($artifact in @($artifacts.Values) + @($products.Values)) {
    if ((Get-FileHash -LiteralPath $artifact.path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $artifact.sha256) {
      throw "Artifact changed during acceptance: $($artifact.path)"
    }
  }
  $report.status = "automated_passed_manual_pending"
} catch {
  $report.status = "failed"
  $report.error = $_.ToString()
  throw
} finally {
  $env:RTERM_REQUIRE_OPENSSH = $previousRequiredSsh
  $report["finished_utc"] = [DateTimeOffset]::UtcNow.ToString("o")
  $temporary = Join-Path $outputRoot "report.json.tmp"
  $report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $temporary -Encoding utf8
  Move-Item -LiteralPath $temporary -Destination (Join-Path $outputRoot "report.json") -Force
}
