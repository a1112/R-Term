[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$sources = @(
    Get-ChildItem -LiteralPath (Join-Path $root 'crates/rterm-app/src') -Recurse -Filter '*.rs' -File |
        ForEach-Object {
            $_.FullName.Substring($root.Length + 1).Replace('\', '/')
        } |
        Sort-Object
)

function Get-Domain([string]$Name) {
    switch -Regex ($Name) {
        'config|scheme|font|color|dpi' { return 'config' }
        'restart|pane|split|inspect' { return 'panes' }
        'tab|workspace' { return 'tabs' }
        'key|input|paste|mouse|ime|leader' { return 'input' }
        'overlay|palette|search|select|launcher|confirm|copy_mode|char_select' { return 'overlays' }
        'persist|frecency|recent' { return 'persistence' }
        'accessib|screen_reader' { return 'accessibility' }
        'runtime|pty|ssh|generation|exit' { return 'runtime' }
        'render|frame|snapshot|damage|surface|scrollbar' { return 'presentation' }
        'window|focus|resize|fullscreen|titlebar' { return 'platform' }
        'command|action|uri|clipboard|notification' { return 'commands' }
        default { return 'compatibility' }
    }
}

function Get-TargetModule([string]$Source) {
    $stem = [IO.Path]::GetFileNameWithoutExtension($Source)
    if ($Source -like 'crates/rterm-app/src/window_compat_tests/*') {
        return 'window::tests'
    }
    switch ($Source) {
        'crates/rterm-app/src/window_inspect_pane_tests.rs' { return 'window::window_inspect_pane_tests' }
        'crates/rterm-app/src/window_restart_pane_tests.rs' { return 'window::window_restart_pane_tests' }
        'crates/rterm-app/src/window.rs' { return 'window::tests' }
        'crates/rterm-app/src/main.rs' { return 'crate' }
        default { return "$stem::tests" }
    }
}

function Get-Sha256Prefix([byte[]]$Bytes) {
    $sha256 = [Security.Cryptography.SHA256]::Create()
    try {
        $hashBytes = $sha256.ComputeHash($Bytes)
    } finally {
        $sha256.Dispose()
    }
    return ([BitConverter]::ToString($hashBytes) -replace '-', '').ToLowerInvariant().Substring(0, 12)
}

$rows = [System.Collections.Generic.List[string]]::new()
foreach ($source in $sources) {
    $lines = Get-Content -LiteralPath (Join-Path $root $source)
    $pending = $false
    $occurrences = @{}
    foreach ($line in $lines) {
        if ($line -match '^\s*#\s*\[\s*(?:[A-Za-z0-9_]+::)?test(?:\s*\([^\]]*\))?\s*\]') {
            $pending = $true
            continue
        }
        if (-not $pending) {
            continue
        }
        if ($line -notmatch '^\s*(?:(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?)fn\s+([A-Za-z_][A-Za-z0-9_]*)') {
            continue
        }
        $name = $Matches[1]
        $pending = $false
        $currentOccurrence = if ($occurrences.ContainsKey($name)) {
            [int]$occurrences[$name]
        } else {
            0
        }
        $occurrence = 1 + $currentOccurrence
        $occurrences[$name] = $occurrence
        $domain = Get-Domain $name
        $identity = "$source|$name|$occurrence"
        $bytes = [Text.Encoding]::UTF8.GetBytes($identity)
        $hash = Get-Sha256Prefix $bytes
        $behavior = "T23-$($domain.ToUpperInvariant())-$hash"
        $targetModule = Get-TargetModule $source
        $rows.Add("$behavior|$source|$name|$occurrence|rterm-app|$targetModule|$domain")
    }
}

$output = Join-Path $root 'crates/rterm-app/tests/fixtures/task23_app_test_manifest.txt'
$null = New-Item -ItemType Directory -Path (Split-Path $output) -Force
$header = @(
    '# task23_app_test_manifest_v1',
    '# columns=behavior_id|source_path|test_name|occurrence|target_crate|target_module|domain'
)
[IO.File]::WriteAllLines($output, @($header + $rows), [Text.UTF8Encoding]::new($false))
Write-Output "wrote $($rows.Count) test mappings to $output"
