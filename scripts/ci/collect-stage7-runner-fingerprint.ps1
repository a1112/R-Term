[CmdletBinding()]
param(
    [UInt64] $GpuVendorId,
    [UInt64] $GpuDeviceId,
    [string] $GpuAdapterName,
    [Parameter(Mandatory = $true)]
    [string] $FontInventoryFingerprintSha256,
    [Parameter(Mandatory = $true)]
    [UInt32] $FontIndexPolicyVersion,
    [string] $TestInputPath,
    [string] $OutputPath,
    [ValidateSet(
        "none",
        "collector-failure",
        "collector-timeout",
        "stderr-output",
        "wrong-adapter-name",
        "invalid-wddm",
        "dpi-probe-failure",
        "pagefile-probe-failure",
        "session-probe-failure"
    )]
    [string] $TestFault = "none"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

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
    $bytes = [Text.Encoding]::UTF8.GetBytes((Get-RunnerCanonicalJson $Value))
    return [Convert]::ToHexString(
        [Security.Cryptography.SHA256]::HashData($bytes)
    ).ToLowerInvariant()
}

function Assert-Sha256([string] $Value, [string] $Label) {
    if ($Value -cnotmatch '^[0-9a-f]{64}$') {
        throw "$Label must be a lowercase SHA-256"
    }
}

function Get-ActiveDisplays {
    if (-not ("RTermStage7DisplayProbe" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public sealed class RTermStage7DisplayRecord {
    public uint WidthPx { get; set; }
    public uint HeightPx { get; set; }
    public uint DpiX { get; set; }
    public uint DpiY { get; set; }
    public bool Primary { get; set; }
}

public static class RTermStage7DisplayProbe {
    private const uint MONITORINFOF_PRIMARY = 1;
    private const int MDT_EFFECTIVE_DPI = 0;

    [StructLayout(LayoutKind.Sequential)]
    private struct RECT { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct MONITORINFOEX {
        public int Size;
        public RECT Monitor;
        public RECT Work;
        public uint Flags;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)]
        public string Device;
    }

    private delegate bool MonitorEnumProc(IntPtr monitor, IntPtr dc, ref RECT rect, IntPtr data);

    [DllImport("user32.dll")]
    private static extern bool EnumDisplayMonitors(IntPtr dc, IntPtr clip, MonitorEnumProc callback, IntPtr data);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFOEX info);
    [DllImport("shcore.dll")]
    private static extern int GetDpiForMonitor(IntPtr monitor, int type, out uint dpiX, out uint dpiY);

    public static RTermStage7DisplayRecord[] GetActive() {
        var result = new List<RTermStage7DisplayRecord>();
        MonitorEnumProc callback = delegate(IntPtr monitor, IntPtr dc, ref RECT rect, IntPtr data) {
            var info = new MONITORINFOEX();
            info.Size = Marshal.SizeOf(typeof(MONITORINFOEX));
            if (!GetMonitorInfo(monitor, ref info)) {
                throw new InvalidOperationException("GetMonitorInfo failed");
            }
            uint dpiX, dpiY;
            int dpiResult = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, out dpiX, out dpiY);
            if (dpiResult != 0 || dpiX == 0 || dpiY == 0) {
                throw new InvalidOperationException("GetDpiForMonitor failed or returned a non-positive DPI");
            }
            int width = info.Monitor.Right - info.Monitor.Left;
            int height = info.Monitor.Bottom - info.Monitor.Top;
            if (width <= 0 || height <= 0) {
                throw new InvalidOperationException("active display has non-positive dimensions");
            }
            result.Add(new RTermStage7DisplayRecord {
                WidthPx = checked((uint)width),
                HeightPx = checked((uint)height),
                DpiX = dpiX,
                DpiY = dpiY,
                Primary = (info.Flags & MONITORINFOF_PRIMARY) != 0
            });
            return true;
        };
        if (!EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, callback, IntPtr.Zero)) {
            throw new InvalidOperationException("EnumDisplayMonitors failed");
        }
        return result.ToArray();
    }

    [DllImport("user32.dll")]
    private static extern int GetSystemMetrics(int index);

    public static bool IsRemoteSession() {
        const int SM_REMOTESESSION = 0x1000;
        return GetSystemMetrics(SM_REMOTESESSION) != 0;
    }
}
'@
    }
    $records = @(
        [RTermStage7DisplayProbe]::GetActive() |
            Sort-Object @{ Expression = { -not $_.Primary } }, WidthPx, HeightPx, DpiX, DpiY |
            ForEach-Object {
                [ordered]@{
                    width_px = [UInt64] $_.WidthPx
                    height_px = [UInt64] $_.HeightPx
                    dpi_x = [UInt64] $_.DpiX
                    dpi_y = [UInt64] $_.DpiY
                    primary = [bool] $_.Primary
                }
            }
    )
    if ($records.Count -eq 0 -or @($records | Where-Object primary).Count -ne 1) {
        throw "active display probe did not return exactly one primary display"
    }
    return $records
}

function Resolve-RemoteSessionKind(
    [bool] $RemoteMetric,
    [object] $CurrentSessionId,
    [object] $GlassSessionId,
    [switch] $RegistryProbeFailed
) {
    if ($RemoteMetric) {
        return "remote"
    }
    if ($RegistryProbeFailed) {
        throw "GlassSessionId registry probe failed"
    }
    $normalized = @()
    foreach ($candidate in @(
        @{ Value = $CurrentSessionId; Label = "current process SessionId" },
        @{ Value = $GlassSessionId; Label = "GlassSessionId" }
    )) {
        $isInteger = $null -ne $candidate.Value -and $candidate.Value.GetType() -in @(
            [byte], [UInt16], [Int16], [UInt32], [Int32], [UInt64], [Int64]
        )
        if (-not $isInteger) {
            throw "$($candidate.Label) must be a strict UInt32-compatible integer"
        }
        try {
            $normalized += [Convert]::ToUInt32($candidate.Value)
        }
        catch {
            throw "$($candidate.Label) must be a strict UInt32-compatible integer"
        }
    }
    if ($normalized[0] -eq $normalized[1]) {
        return "local"
    }
    return "remote"
}

function Get-RemoteSessionMetric {
    return [RTermStage7DisplayProbe]::IsRemoteSession()
}

function Get-CurrentProcessSessionId {
    return [Diagnostics.Process]::GetCurrentProcess().SessionId
}

function Get-GlassSessionId {
    return Get-ItemPropertyValue `
        -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\Terminal Server' `
        -Name GlassSessionId `
        -ErrorAction Stop
}

function Get-RemoteSessionKind {
    try {
        $remoteMetric = Get-RemoteSessionMetric
    }
    catch {
        throw "SM_REMOTESESSION API probe failed: $($_.Exception.Message)"
    }
    if ($remoteMetric) {
        return "remote"
    }
    try {
        $currentSessionId = Get-CurrentProcessSessionId
    }
    catch {
        throw "current process SessionId API probe failed: $($_.Exception.Message)"
    }
    try {
        $glassSessionId = Get-GlassSessionId
    }
    catch {
        throw "GlassSessionId registry probe failed: $($_.Exception.Message)"
    }
    return Resolve-RemoteSessionKind `
        -RemoteMetric $false `
        -CurrentSessionId $currentSessionId `
        -GlassSessionId $glassSessionId
}

function Assert-SelectedGpuDriverIdentity(
    [string] $CimDriverVersion,
    [string] $DxdiagDriverVersion
) {
    $normalizedCim = $CimDriverVersion.Trim()
    $normalizedDxdiag = $DxdiagDriverVersion.Trim()
    foreach ($candidate in @(
        @{ Value = $normalizedCim; Label = "CIM" },
        @{ Value = $normalizedDxdiag; Label = "dxdiag" }
    )) {
        if ($candidate.Value -cnotmatch '^[0-9]+(?:\.[0-9]+){1,3}$') {
            throw "$($candidate.Label) selected GPU driver version is invalid"
        }
    }
    if ($normalizedCim -cne $normalizedDxdiag) {
        throw "selected GPU CIM/dxdiag driver version mismatch"
    }
}

function Get-WddmIdentity(
    [string] $AdapterName,
    [UInt64] $VendorId,
    [UInt64] $DeviceId
) {
    $repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
    . (Join-Path $PSScriptRoot "process-harness.ps1")
    $temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $xmlPath = Join-Path $temporaryRoot ("rterm-stage7-dxdiag-{0}-{1}.xml" -f $PID, [Guid]::NewGuid().ToString("N"))
    try {
        try {
            $null = Invoke-BoundedProcess `
                -Phase "Stage 7 dxdiag runner fingerprint probe" `
                -FilePath "dxdiag.exe" `
                -ArgumentList @("/whql:off", "/x", $xmlPath) `
                -TimeoutSeconds 60
        }
        catch {
            # Some display drivers leave dxdiag alive after its complete XML has
            # been atomically written. The bounded job still kills/reaps that
            # process tree; only a parseable, closed document may be consumed.
            if (-not (Test-Path -LiteralPath $xmlPath -PathType Leaf)) {
                throw
            }
            try {
                [xml] $completedDocument = Get-Content -LiteralPath $xmlPath -Raw
            }
            catch {
                throw "dxdiag exceeded its bounded deadline without a complete XML observation"
            }
        }
        if (-not (Test-Path -LiteralPath $xmlPath -PathType Leaf)) {
            throw "dxdiag did not produce its bounded XML observation"
        }
        [xml] $document = Get-Content -LiteralPath $xmlPath -Raw
        $devices = @($document.DxDiag.DisplayDevices.DisplayDevice)
        $expectedVendor = "0x{0:X4}" -f $VendorId
        $expectedDevice = "0x{0:X4}" -f $DeviceId
        $normalizedAdapterName = $AdapterName.Trim()
        $matchedDevices = @($devices | Where-Object {
            [StringComparer]::OrdinalIgnoreCase.Equals(([string] $_.CardName).Trim(), $normalizedAdapterName) -and
            [StringComparer]::OrdinalIgnoreCase.Equals(([string] $_.VendorID).Trim(), $expectedVendor) -and
            [StringComparer]::OrdinalIgnoreCase.Equals(([string] $_.DeviceID).Trim(), $expectedDevice)
        })
        if ($matchedDevices.Count -eq 0) {
            throw "dxdiag did not identify the selected GPU adapter"
        }
        $driverIdentities = @(
            $matchedDevices |
                ForEach-Object { "{0}`n{1}" -f ([string] $_.DriverVersion), ([string] $_.DriverModel) } |
                Sort-Object -Unique
        )
        if ($driverIdentities.Count -ne 1) {
            throw "duplicate selected GPU adapters disagree on driver/WDDM identity"
        }
        $driverVersion = ([string] $matchedDevices[0].DriverVersion).Trim()
        if ($driverVersion -cnotmatch '^[0-9]+(?:\.[0-9]+){1,3}$') {
            throw "dxdiag selected adapter has an invalid driver version"
        }
        $driverModel = ([string] $matchedDevices[0].DriverModel).Trim()
        if ($driverModel -cnotmatch '^WDDM [0-9]+\.[0-9]+$') {
            throw "dxdiag selected adapter has an invalid WDDM driver model"
        }
        return [pscustomobject]@{
            DriverVersion = $driverVersion
            WddmVersion = $driverModel
        }
    }
    finally {
        if (Test-Path -LiteralPath $xmlPath) {
            Remove-Item -LiteralPath $xmlPath -Force
        }
    }
}

function Get-HostFields {
    if (-not $IsWindows) { throw "runner fingerprint host probe requires Windows" }
    if ($GpuVendorId -eq 0 -or $GpuDeviceId -eq 0 -or [string]::IsNullOrWhiteSpace($GpuAdapterName)) {
        throw "selected GPU vendor/device/name are required for the host probe"
    }
    if ($TestFault -ceq "invalid-wddm") { throw "simulated invalid WDDM probe result" }
    if ($TestFault -ceq "dpi-probe-failure") { throw "simulated display DPI probe failure" }
    if ($TestFault -ceq "pagefile-probe-failure") { throw "simulated pagefile probe failure" }
    if ($TestFault -ceq "session-probe-failure") { throw "simulated authoritative remote session probe failure" }
    $os = Get-CimInstance Win32_OperatingSystem -ErrorAction Stop
    $computer = Get-CimInstance Win32_ComputerSystem -ErrorAction Stop
    $normalizedRequestedName = $GpuAdapterName.Trim()
    if ($TestFault -ceq "wrong-adapter-name") {
        $normalizedRequestedName = "$normalizedRequestedName-test-mismatch"
    }
    $videoMatches = @(Get-CimInstance Win32_VideoController -ErrorAction Stop | Where-Object {
        $pnp = [string] $_.PNPDeviceID
        $pnp -match ("VEN_{0:X4}" -f $GpuVendorId) -and
        $pnp -match ("DEV_{0:X4}" -f $GpuDeviceId) -and
        [StringComparer]::OrdinalIgnoreCase.Equals(([string] $_.Name).Trim(), $normalizedRequestedName)
    })
    if ($videoMatches.Count -eq 0) {
        throw "selected GPU vendor/device/name did not match a Windows video controller"
    }
    $videoIdentities = @(
        $videoMatches |
            ForEach-Object { "{0}`n{1}" -f ([string] $_.Name), ([string] $_.DriverVersion) } |
            Sort-Object -Unique
    )
    if ($videoIdentities.Count -ne 1) {
        throw "duplicate selected Windows video controllers disagree on adapter/driver identity"
    }
    $video = $videoMatches[0]
    $driverVersion = ([string] $video.DriverVersion).Trim()
    if ($driverVersion -cnotmatch '^[0-9]+(?:\.[0-9]+){1,3}$') {
        throw "selected GPU driver version is invalid"
    }
    $ubr = Get-ItemPropertyValue `
        -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion' `
        -Name UBR
    if ($computer.AutomaticManagedPagefile -isnot [bool]) {
        throw "AutomaticManagedPagefile must be a strict Boolean value"
    }
    $automaticPagefile = $computer.AutomaticManagedPagefile
    $pagefileMode = if ($automaticPagefile) {
        "automatic-managed"
    } elseif (@(Get-CimInstance Win32_PageFileSetting -ErrorAction Stop).Count -gt 0) {
        "manual"
    } else {
        "disabled"
    }
    $displays = @(Get-ActiveDisplays)
    $powerText = (& powercfg.exe /getactivescheme 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $powerText -notmatch '(?i)([0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12})') {
        throw "unable to identify the active Windows power plan"
    }
    $powerGuid = $Matches[1].ToLowerInvariant()
    $sessionKind = Get-RemoteSessionKind
    $dxdiagIdentity = Get-WddmIdentity `
        -AdapterName ([string] $video.Name) `
        -VendorId $GpuVendorId `
        -DeviceId $GpuDeviceId
    Assert-SelectedGpuDriverIdentity `
        -CimDriverVersion $driverVersion `
        -DxdiagDriverVersion $dxdiagIdentity.DriverVersion
    return [ordered]@{
        os = [ordered]@{
            version = [string] $os.Version
            build_number = [string] $os.BuildNumber
            build_revision = [UInt64] $ubr
            architecture = switch ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
                ([Runtime.InteropServices.Architecture]::X64) { "x86_64" }
                ([Runtime.InteropServices.Architecture]::X86) { "x86" }
                ([Runtime.InteropServices.Architecture]::Arm64) { "arm64" }
                ([Runtime.InteropServices.Architecture]::Arm) { "arm" }
                default { throw "unsupported Windows architecture enum" }
            }
        }
        gpu = [ordered]@{
            vendor_id = $GpuVendorId
            device_id = $GpuDeviceId
            driver_version = $driverVersion
            wddm_version = $dxdiagIdentity.WddmVersion
        }
        memory = [ordered]@{
            physical_bytes = [UInt64] $computer.TotalPhysicalMemory
            pagefile_mode = $pagefileMode
        }
        displays = $displays
        power_plan = [ordered]@{ guid = $powerGuid }
        session = [ordered]@{ kind = $sessionKind }
        locale = [ordered]@{
            culture = [Globalization.CultureInfo]::CurrentCulture.Name
            ui_culture = [Globalization.CultureInfo]::CurrentUICulture.Name
            system_locale = (Get-WinSystemLocale).Name
        }
        fonts = [ordered]@{
            inventory_fingerprint_sha256 = $FontInventoryFingerprintSha256
            index_policy_version = [UInt64] $FontIndexPolicyVersion
        }
        cold_cache_policy = [ordered]@{
            process_cold_start = $true
            os_file_cache = "unmodified-no-explicit-flush"
        }
    }
}

if ($TestFault -ceq "collector-failure") {
    throw "simulated Stage 7 runner fingerprint collector failure"
}
if ($TestFault -ceq "collector-timeout") {
    Start-Sleep -Seconds 120
    throw "collector timeout seam unexpectedly survived"
}
Assert-Sha256 $FontInventoryFingerprintSha256 "FontInventoryFingerprintSha256"
if ($FontIndexPolicyVersion -eq 0) {
    throw "FontIndexPolicyVersion must be positive"
}

$source = "host-probe"
if (-not [string]::IsNullOrWhiteSpace($TestInputPath)) {
    if (-not (Test-Path -LiteralPath $TestInputPath -PathType Leaf)) {
        throw "TestInputPath must identify a fixture JSON file"
    }
    $fields = Get-Content -LiteralPath $TestInputPath -Raw | ConvertFrom-Json
    $source = "fixture"
} else {
    $fields = Get-HostFields
}
if ($TestFault -ceq "stderr-output") {
    [Console]::Error.WriteLine("simulated successful collector stderr")
}

$result = [ordered]@{
    schema = "rterm.stage7/runner-fingerprint/v1"
    source = $source
    complete = $true
    fields = $fields
    fingerprint_sha256 = Get-RunnerCanonicalSha256 $fields
    collector_script_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $PSCommandPath).Hash.ToLowerInvariant()
    collector_timeout_seconds = 60
}
$resultJson = $result | ConvertTo-Json -Depth 100 -Compress
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $outputDirectory = Split-Path -Parent $OutputPath
    if (-not [string]::IsNullOrWhiteSpace($outputDirectory)) {
        New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
    }
    $temporaryPath = Join-Path ([IO.Path]::GetTempPath()) ("rterm-stage7-runner-fingerprint-{0}-{1}.tmp" -f $PID, [Guid]::NewGuid().ToString("N"))
    try {
        [IO.File]::WriteAllText($temporaryPath, $resultJson, [Text.UTF8Encoding]::new($false))
        Move-Item -LiteralPath $temporaryPath -Destination $OutputPath -Force
    }
    finally {
        if (Test-Path -LiteralPath $temporaryPath) {
            Remove-Item -LiteralPath $temporaryPath -Force
        }
    }
}
Write-Output $resultJson
