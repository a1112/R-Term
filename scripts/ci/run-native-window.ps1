[CmdletBinding()]
param(
  [switch] $HarnessSelfTest,

  [ValidateSet("debug", "release")]
  [string] $Profile = "debug",

  [string] $ExpectedTarget,

  [ValidateSet("windows-conpty", "unix-pty")]
  [string] $ExpectedPtyBackend
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

. (Join-Path $PSScriptRoot "process-harness.ps1")

$profileArguments = if ($Profile -eq "release") { @("--release") } else { @() }
$profileDirectory = if ($Profile -eq "release") { "release" } else { "debug" }
$targetDirectory = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  Join-Path $repositoryRoot "target"
} else {
  $env:CARGO_TARGET_DIR
}
$executable = Join-Path $targetDirectory "$profileDirectory/rterm.exe"

Assert-BoundedProcessHarness

if ($HarnessSelfTest) {
  return
}
if ([string]::IsNullOrWhiteSpace($ExpectedTarget) -or [string]::IsNullOrWhiteSpace($ExpectedPtyBackend)) {
  throw "-ExpectedTarget and -ExpectedPtyBackend are required outside -HarnessSelfTest"
}

Push-Location $repositoryRoot
try {
  $buildArguments = @("build", "--locked", "-p", "rterm-app", "--bin", "rterm", "--test", "openssh_loopback", "--test", "native_window_e2e") + $profileArguments
  $null = Invoke-BoundedProcess -Phase "native E2E build ($Profile)" -FilePath "cargo" -ArgumentList $buildArguments -TimeoutSeconds 1200

  $versionResult = Invoke-BoundedProcess -Phase "version identity" -FilePath $executable -ArgumentList @("version", "--json") -TimeoutSeconds 30
  $version = $versionResult.Stdout | ConvertFrom-Json
  if ($version.target -ne $ExpectedTarget) {
    throw "version target mismatch: observed '$($version.target)', expected '$ExpectedTarget'"
  }
  if ($version.pty_backend -ne $ExpectedPtyBackend) {
    throw "PTY backend mismatch: observed '$($version.pty_backend)', expected '$ExpectedPtyBackend'"
  }

  $null = Invoke-BoundedProcess -Phase "OpenSSH client probe" -FilePath "ssh" -ArgumentList @("-V") -TimeoutSeconds 15

  $nativeSshArguments = @(
    "test", "--locked", "-p", "rterm-ssh", "--all-targets"
  ) + $profileArguments + @("--", "--nocapture")
  $null = Invoke-BoundedProcess -Phase "hermetic native SSH tests ($Profile)" -FilePath "cargo" -ArgumentList $nativeSshArguments -TimeoutSeconds 300

  $env:RTERM_REQUIRE_OPENSSH = "1"
  $openSshArguments = @(
    "test", "--locked", "-p", "rterm-app", "--test", "openssh_loopback"
  ) + $profileArguments + @("--", "--nocapture")
  $null = Invoke-BoundedProcess -Phase "system OpenSSH interoperability ($Profile)" -FilePath "cargo" -ArgumentList $openSshArguments -TimeoutSeconds 300

  $testArguments = @(
    "test", "--locked", "-p", "rterm-app", "--test", "native_window_e2e"
  ) + $profileArguments + @(
    "native_window_e2e_presents_ten_frames_from_a_real_pty",
    "--", "--exact", "--nocapture"
  )
  $null = Invoke-BoundedProcess -Phase "native ten-frame E2E ($Profile)" -FilePath "cargo" -ArgumentList $testArguments -TimeoutSeconds 180

  $nativeScenarioAttempts = 2
  foreach ($scenario in @(
    "native_window_e2e_preserves_gpu_text_at_scale_100",
    "native_window_e2e_preserves_gpu_text_at_scale_125",
    "native_window_e2e_preserves_gpu_text_at_scale_150",
    "native_window_e2e_preserves_gpu_text_at_scale_200",
    "native_window_local_pane_v2_writes_visible_session_log"
  )) {
    $isScaledScenario = $scenario.StartsWith(
      "native_window_e2e_preserves_gpu_text_at_scale_",
      [StringComparison]::Ordinal
    )
    $scenarioArguments = @(
      "test", "--locked", "-p", "rterm-app", "--test", "native_window_e2e"
    ) + $profileArguments + @(
      $scenario,
      "--", "--exact", "--ignored", "--nocapture"
    )
    $previousScaleAdapterOverride = [Environment]::GetEnvironmentVariable(
      "RTERM_TEST_NATIVE_SCALE_PRIMARY",
      "Process"
    )
    try {
      for ($attempt = 1; $attempt -le $nativeScenarioAttempts; $attempt++) {
        if ($isScaledScenario -and $attempt -eq 1) {
          # Fresh hosted Windows runners are more reliable when the scaled
          # probe starts on the primary adapter. Keep the software fallback
          # as a bounded alternate path for runners whose GPU stack wedges.
          $env:RTERM_TEST_NATIVE_SCALE_PRIMARY = "1"
        } elseif ($isScaledScenario -and $attempt -eq 2) {
          if ($null -eq $previousScaleAdapterOverride) {
            Remove-Item Env:RTERM_TEST_NATIVE_SCALE_PRIMARY -ErrorAction SilentlyContinue
          } else {
            $env:RTERM_TEST_NATIVE_SCALE_PRIMARY = $previousScaleAdapterOverride
          }
        } elseif ($attempt -eq 1) {
          if ($null -eq $previousScaleAdapterOverride) {
            Remove-Item Env:RTERM_TEST_NATIVE_SCALE_PRIMARY -ErrorAction SilentlyContinue
          } else {
            $env:RTERM_TEST_NATIVE_SCALE_PRIMARY = $previousScaleAdapterOverride
          }
        } else {
          # A non-scaled scenario still retries once through the alternate
          # adapter path instead of repeating the same bounded timeout.
          $env:RTERM_TEST_NATIVE_SCALE_PRIMARY = "1"
        }
        try {
          $null = Invoke-BoundedProcess -Phase "native E2E scenario $scenario ($Profile), attempt $attempt" -FilePath "cargo" -ArgumentList $scenarioArguments -TimeoutSeconds 300
          break
        } catch {
          if ($attempt -ge $nativeScenarioAttempts) {
            throw
          }
          Write-Warning "native E2E scenario $scenario attempt $attempt failed; retrying after bounded cleanup with the alternate adapter: $_"
        }
      }
    } finally {
      if ($null -eq $previousScaleAdapterOverride) {
        Remove-Item Env:RTERM_TEST_NATIVE_SCALE_PRIMARY -ErrorAction SilentlyContinue
      } else {
        $env:RTERM_TEST_NATIVE_SCALE_PRIMARY = $previousScaleAdapterOverride
      }
    }
  }
} finally {
  Pop-Location
}
