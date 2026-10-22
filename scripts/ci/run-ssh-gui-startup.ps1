[CmdletBinding()]
param(
  [ValidateSet("release", "debug")]
  [string] $Profile = "release",

  [ValidateRange(0, 1000)]
  [int] $Warmups = 5,

  [ValidateRange(1, 1000)]
  [int] $Samples = 30,

  [ValidateRange(5, 300)]
  [int] $TimeoutSeconds = 60,

  [switch] $SkipBuild,

  [string] $ExecutablePath = "",

  [switch] $CollectOnly
)

# Fixed Windows runner for the SSH GUI first-frame contract. The measured
# process is benchmark-startup: it presents the CPU bootstrap frame and exits
# before configuration, GPU, or SSH transport work begins.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

. (Join-Path $PSScriptRoot "process-harness.ps1")

if ($Warmups -lt 0 -or $Samples -lt 1) {
  throw "Warmups must be >= 0 and Samples must be >= 1"
}

$profileArguments = if ($Profile -eq "release") { @("--release") } else { @() }
$profileDirectory = if ($Profile -eq "release") { "release" } else { "debug" }
$targetDirectory = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  Join-Path $repositoryRoot "target"
} else {
  $env:CARGO_TARGET_DIR
}
$executable = if ([string]::IsNullOrWhiteSpace($ExecutablePath)) {
  Join-Path $targetDirectory "$profileDirectory/rterm.exe"
} else {
  [IO.Path]::GetFullPath($ExecutablePath)
}

Assert-BoundedProcessHarness

Push-Location $repositoryRoot
$previousBenchmarkScale = $env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR
$env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR = "1"
try {
  if (-not $SkipBuild) {
    $buildArguments = @("build", "--locked", "-p", "rterm-app", "--features", "ssh") + $profileArguments
    $null = Invoke-BoundedProcess `
      -Phase "SSH GUI startup build ($Profile)" `
      -FilePath "cargo" `
      -ArgumentList $buildArguments `
      -TimeoutSeconds 1200
  }
  if (-not (Test-Path -LiteralPath $executable)) {
    throw "rterm-app executable was not found at '$executable'"
  }

  $arguments = @(
    "ssh",
    "--gui",
    "--renderer", "auto",
    "--benchmark-startup",
    "--metrics-json",
    "--cols", "80",
    "--rows", "24",
    "--host", "127.0.0.1",
    "--user", "startup-benchmark"
  )

  function Invoke-FirstPresentSample {
    param(
      [Parameter(Mandatory = $true)] [string] $ExecutablePath,
      [Parameter(Mandatory = $true)] [string[]] $ArgumentList,
      [Parameter(Mandatory = $true)] [int] $Timeout
    )

    $resolvedPath = Resolve-NativeExecutable -FilePath $ExecutablePath
    $commandLine = @(
      ConvertTo-WindowsCommandLineArgument -Argument $resolvedPath
      $ArgumentList | ForEach-Object {
        ConvertTo-WindowsCommandLineArgument -Argument $_
      }
    ) -join " "
    $owned = $null
    $closedJob = $false
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($Timeout)
    try {
      $owned = [RTermCiOwnedProcess]::Start(
        $resolvedPath,
        $commandLine,
        $repositoryRoot,
        $false
      )

      $marker = $null
      $markerTimestamp = 0L
      while ($null -eq $marker) {
        $remaining = Get-RemainingMilliseconds -Deadline $deadline
        if ($remaining -le 0) {
          throw "first_present marker was not observed before the ${Timeout}s deadline"
        }
        $lineTask = $owned.StandardError.ReadLineAsync()
        if (-not $lineTask.Wait($remaining)) {
          throw "first_present marker was not observed before the ${Timeout}s deadline"
        }
        $line = $lineTask.Result
        if ($null -eq $line) {
          throw "rterm-app exited before emitting first_present"
        }
        if ($line.StartsWith("first_present ", [StringComparison]::Ordinal)) {
          $marker = $line
          $markerTimestamp = [Diagnostics.Stopwatch]::GetTimestamp()
        }
      }

      $stdoutTask = $owned.StandardOutput.ReadToEndAsync()
      $stderrTask = $owned.StandardError.ReadToEndAsync()
      $remaining = Get-RemainingMilliseconds -Deadline $deadline
      if ($remaining -le 0 -or -not $owned.Process.WaitForExit($remaining)) {
        throw "rterm-app did not exit after first_present"
      }
      $remainingStdout = Complete-StreamBeforeDeadline `
        -Task $stdoutTask -Deadline $deadline -StreamName "startup stdout"
      $remainingStderr = Complete-StreamBeforeDeadline `
        -Task $stderrTask -Deadline $deadline -StreamName "startup stderr"
      if ($owned.ExitCode -ne 0) {
        throw "rterm-app exited with code $($owned.ExitCode)`nstdout:`n$remainingStdout`nstderr:`n$remainingStderr"
      }

      $markerFields = @{}
      foreach ($field in ($marker -split " ")) {
        $pair = $field -split "=", 2
        if ($pair.Count -eq 2) {
          $markerFields[$pair[0]] = $pair[1]
        }
      }
      $requiredMarkerFields = @(
        "process_to_first_present_ms",
        "final_renderer"
      )
      foreach ($requiredField in $requiredMarkerFields) {
        if (
          -not $markerFields.ContainsKey($requiredField) -or
          [string]::IsNullOrWhiteSpace($markerFields[$requiredField])
        ) {
          throw "first_present marker is missing required field '$requiredField': $marker"
        }
      }

      $reportedProcessToFirstPresentMs = 0.0
      if (
        -not [double]::TryParse(
          $markerFields["process_to_first_present_ms"],
          [Globalization.NumberStyles]::Float,
          [Globalization.CultureInfo]::InvariantCulture,
          [ref] $reportedProcessToFirstPresentMs
        ) -or
        $reportedProcessToFirstPresentMs -le 0
      ) {
        throw "first_present process_to_first_present_ms must be a positive number"
      }

      $memoryMarker = $null
      foreach ($line in ($remainingStderr -split "`r?`n")) {
        if ($line.StartsWith("first_frame_memory ", [StringComparison]::Ordinal)) {
          if ($null -ne $memoryMarker) {
            throw "rterm-app emitted more than one first_frame_memory marker"
          }
          $memoryMarker = $line
        }
      }
      if ($null -eq $memoryMarker) {
        throw "rterm-app exited before emitting first_frame_memory"
      }
      $memoryMarkerFields = @{}
      foreach ($field in ($memoryMarker -split " ")) {
        $pair = $field -split "=", 2
        if ($pair.Count -eq 2) {
          $memoryMarkerFields[$pair[0]] = $pair[1]
        }
      }
      $requiredMemoryMarkerFields = @("first_frame_private_bytes")
      foreach ($requiredField in $requiredMemoryMarkerFields) {
        if (
          -not $memoryMarkerFields.ContainsKey($requiredField) -or
          [string]::IsNullOrWhiteSpace($memoryMarkerFields[$requiredField])
        ) {
          throw "first_frame_memory marker is missing required field '$requiredField': $memoryMarker"
        }
      }

      $privateBytes = [UInt64] 0
      if (
        -not [UInt64]::TryParse(
          $memoryMarkerFields["first_frame_private_bytes"],
          [Globalization.NumberStyles]::Integer,
          [Globalization.CultureInfo]::InvariantCulture,
          [ref] $privateBytes
        ) -or
        $privateBytes -eq 0
      ) {
        throw "first_frame_memory first_frame_private_bytes must be a positive integer"
      }

      $renderer = [string] $markerFields["final_renderer"]
      if ($renderer -ne "cpu") {
        throw "benchmark-startup must report the CPU renderer, observed '$renderer'"
      }

      $resumeToMarkerMs = (($markerTimestamp - $owned.ResumeTimestamp) * 1000.0) /
        [Diagnostics.Stopwatch]::Frequency
      $metrics = $remainingStdout.Trim() | ConvertFrom-Json
      if ($metrics.process_to_first_present_ms -ne $reportedProcessToFirstPresentMs) {
        throw "stdout metrics process_to_first_present_ms does not match first_present marker"
      }
      if ($metrics.first_frame_private_bytes -ne $privateBytes) {
        throw "stdout metrics first_frame_private_bytes does not match first_present marker"
      }
      if ($metrics.final_renderer -ne $renderer) {
        throw "stdout metrics final_renderer does not match first_present marker"
      }
      return [pscustomobject]@{
        ExternalProcessToFirstPresentMs = [double] $resumeToMarkerMs
        ReportedProcessToFirstPresentMs = $reportedProcessToFirstPresentMs
        PrivateBytes = $privateBytes
        Renderer = $renderer
        Metrics = $metrics
      }
    } finally {
      if ($null -ne $owned) {
        if (-not $owned.Process.HasExited) {
          $owned.CloseJob()
          $closedJob = $true
          $null = $owned.Process.WaitForExit(10000)
        } elseif (-not $closedJob) {
          $owned.CloseJob()
          $closedJob = $true
        }
        $owned.Dispose()
      }
    }
  }

  for ($index = 1; $index -le $Warmups; $index++) {
    Write-Host "warmup $index/$Warmups"
    $null = Invoke-FirstPresentSample -ExecutablePath $executable -ArgumentList $arguments -Timeout $TimeoutSeconds
  }

  $results = [System.Collections.Generic.List[object]]::new()
  for ($index = 1; $index -le $Samples; $index++) {
    $sample = Invoke-FirstPresentSample `
      -ExecutablePath $executable -ArgumentList $arguments -Timeout $TimeoutSeconds
    $results.Add([pscustomobject]@{
      sample = $index
      process_to_first_present_ms = $sample.ExternalProcessToFirstPresentMs
      reported_process_to_first_present_ms = $sample.ReportedProcessToFirstPresentMs
      first_frame_private_bytes = $sample.PrivateBytes
      final_renderer = $sample.Renderer
    })
    Write-Host ("sample {0}/{1}: {2:N2} ms, {3:N1} MiB, renderer={4}" -f `
      $index, $Samples, $sample.ExternalProcessToFirstPresentMs,
      ($sample.PrivateBytes / 1MB), $sample.Renderer)
  }

  $orderedTimes = @($results | ForEach-Object process_to_first_present_ms | Sort-Object)
  $orderedMemory = @($results | ForEach-Object first_frame_private_bytes | Sort-Object)
  $p50 = $orderedTimes[[Math]::Max(0, [Math]::Ceiling($Samples * 0.50) - 1)]
  $p95Index = [Math]::Max(0, [Math]::Ceiling($Samples * 0.95) - 1)
  $p95 = $orderedTimes[$p95Index]
  $memoryP95 = $orderedMemory[$p95Index]
  $memoryMax = $orderedMemory[-1]

  # Absolute startup gates are intentionally limited to the protected
  # Windows release runner. Shared/debug runners still produce measurements
  # without turning host scheduling noise into a hard failure.
  if ($Profile -eq "release" -and -not $CollectOnly) {
    if ($p50 -gt 400 -or $p95 -gt 500) {
      throw ("SSH GUI first-present contract failed: p50={0:N2} ms (limit 400), " +
        "p95={1:N2} ms (limit 500)" -f $p50, $p95)
    }
    if ($memoryP95 -gt (55 * 1MB) -or $memoryMax -ge (60 * 1MB)) {
      throw ("SSH GUI Private Bytes contract failed: p95={0:N1} MiB (limit 55), " +
        "max={1:N1} MiB (must be <60)" -f ($memoryP95 / 1MB), ($memoryMax / 1MB))
    }
  }

  $summary = [ordered]@{
    profile = $Profile
    columns = 80
    rows = 24
    dpi_scale = 1.0
    warmups = $Warmups
    samples = $Samples
    first_present_ms_p50 = $p50
    first_present_ms_p95 = $p95
    private_bytes_p95 = $memoryP95
    private_bytes_max = $memoryMax
    renderer_values = @($results | Select-Object -ExpandProperty final_renderer -Unique)
    samples_detail = $results
  }
  $summary | ConvertTo-Json -Depth 8
} finally {
  $env:RTERM_BENCHMARK_WINDOW_SCALE_FACTOR = $previousBenchmarkScale
  Pop-Location
}
