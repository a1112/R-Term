Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

function Get-RTermRepositoryRoot {
    [CmdletBinding()]
    param()

    return [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
}

function Read-RTermPerformanceBaseline {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)][string]$Path)

    $resolved = [IO.Path]::GetFullPath($Path)
    if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
        throw "performance baseline does not exist: $resolved"
    }
    $baseline = Get-Content -LiteralPath $resolved -Raw | ConvertFrom-Json
    if ($baseline.schema_version -ne 1) {
        throw "unsupported performance baseline schema version: $($baseline.schema_version)"
    }
    if ($baseline.baseline_commit -notmatch '^[0-9a-f]{40}$') {
        throw "performance baseline commit is not a full lowercase Git hash"
    }
    if ($baseline.protocol.warmups -ne 2 -or $baseline.protocol.samples -ne 7) {
        throw "performance baseline must use two warmups and seven samples"
    }
    foreach ($name in @('ansi-scroll-query', 'plain-scroll', 'ansi-scroll')) {
        $entry = $baseline.runtime.workloads.PSObject.Properties[$name]
        if ($null -eq $entry) {
            throw "performance baseline is missing workload: $name"
        }
        $workload = $entry.Value
        foreach ($metric in @(
            'throughput_bytes_per_sec',
            'chunk_p95_us',
            'render_frame_p95_us',
            'process_memory_bytes'
        )) {
            if ([long]$workload.baseline.$metric -le 0) {
                throw "performance baseline workload $name has invalid metric: $metric"
            }
        }
    }
    return $baseline
}

function Test-RTermPerformanceSchema {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)][string]$Path)

    $resolved = [IO.Path]::GetFullPath($Path)
    if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
        throw "performance scorecard schema does not exist: $resolved"
    }
    $schema = Get-Content -LiteralPath $resolved -Raw | ConvertFrom-Json
    if ($schema.'$id' -ne 'https://github.com/a1112/R-Term/blob/main/scripts/perf/scorecard.schema.json') {
        throw "unexpected performance scorecard schema ID: $($schema.'$id')"
    }
}

function Get-RTermMachineFingerprint {
    [CmdletBinding()]
    param()

    $processor = Get-CimInstance Win32_Processor | Select-Object -First 1
    $operatingSystem = Get-CimInstance Win32_OperatingSystem
    $rustVersion = (& rustc -Vv | Select-String '^release:').Line.Split(':', 2)[1].Trim()
    $rustHost = (& rustc -Vv | Select-String '^host:').Line.Split(':', 2)[1].Trim()
    $cargoVersion = (& cargo -V) -replace '^cargo\s+([^\s]+).+$', '$1'
    $powerProfile = ((& powercfg /getactivescheme) -join ' ').Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "powercfg /getactivescheme failed with exit code $LASTEXITCODE"
    }

    return [ordered]@{
        os = 'windows'
        os_caption = [string]$operatingSystem.Caption
        os_version = [string]$operatingSystem.Version
        os_build = [string]$operatingSystem.BuildNumber
        arch = 'x86_64'
        cpu = ([string]$processor.Name).Trim()
        power_profile = $powerProfile
        rustc = $rustVersion
        cargo = $cargoVersion
        toolchain_host = $rustHost
    }
}

function Assert-RTermComparableMachine {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]$Baseline,
        [Parameter(Mandatory = $true)]$Actual,
        [switch]$AllowDifferentMachine
    )

    $mismatches = [System.Collections.Generic.List[string]]::new()
    foreach ($property in @('os', 'os_build', 'arch', 'cpu', 'rustc', 'cargo', 'toolchain_host')) {
        if ([string]$Baseline.machine.$property -ne [string]$Actual.$property) {
            $mismatches.Add(
                "$property expected=$($Baseline.machine.$property) actual=$($Actual.$property)"
            )
        }
    }
    $expectedPowerGuid = [regex]::Match(
        [string]$Baseline.machine.power_profile,
        '[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}'
    ).Value
    $actualPowerGuid = [regex]::Match(
        [string]$Actual.power_profile,
        '[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}'
    ).Value
    if ($expectedPowerGuid -ne $actualPowerGuid) {
        $mismatches.Add("power_profile expected=$expectedPowerGuid actual=$actualPowerGuid")
    }

    if ($mismatches.Count -gt 0 -and -not $AllowDifferentMachine) {
        throw "machine fingerprint differs from the authoritative baseline: $($mismatches -join '; ')"
    }
    return @($mismatches)
}

function Write-RTermScorecardResult {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]$Result,
        [string]$Output
    )

    $json = $Result | ConvertTo-Json -Depth 12 -Compress
    if (-not [string]::IsNullOrWhiteSpace($Output)) {
        $resolved = [IO.Path]::GetFullPath($Output)
        $parent = Split-Path -Parent $resolved
        if (-not [string]::IsNullOrWhiteSpace($parent)) {
            $null = New-Item -ItemType Directory -Force -Path $parent
        }
        Set-Content -LiteralPath $resolved -Value $json -Encoding UTF8
    }
    [Console]::Out.WriteLine($json)
}
