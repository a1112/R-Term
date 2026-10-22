#![cfg(target_os = "windows")]

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn font_proof_typed_options_are_forwarded_to_sync_and_deferred_gpu_paths() {
    let source = fs::read_to_string(repo_path("crates/rterm-app/src/window_parts/part08.rs"))
        .expect("read GPU initialization owner");
    assert!(source.contains("WindowGpu::new_with_diagnostic_options"));
    assert!(source.contains("WindowGpu::prepare_with_diagnostic_options"));
    assert!(source.matches("diagnostic_font_options()").count() >= 2);
}

#[test]
fn font_proof_what_if_freezes_interleaved_sampling_and_hierarchical_percentiles() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let output_directory = std::env::temp_dir().join(format!(
        "rterm-font-proof-what-if-{}-{unique}",
        std::process::id()
    ));
    let output = Command::new("pwsh.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-File",
            repo_path("scripts/ci/run-stage7-font-proof.ps1")
                .to_str()
                .expect("UTF-8 runner path"),
            "-OutputDirectory",
            output_directory.to_str().expect("UTF-8 output path"),
            "-WhatIf",
        ])
        .current_dir(repo_path("."))
        .output()
        .expect("execute font proof WhatIf contract");
    assert!(
        output.status.success(),
        "font proof WhatIf failed: {}",
        combined_output(&output)
    );
    let plan: Value = serde_json::from_slice(&output.stdout).expect("parse WhatIf plan JSON");
    assert_eq!(plan["schema"], "rterm.stage7/font-ownership-proof-plan/v1");
    assert_eq!(plan["renderer"], "auto");
    assert_eq!(plan["explicit_backend_override"], false);
    assert_eq!(plan["warmups_per_mode"], 5);
    assert_eq!(plan["measured_processes_per_mode"], 30);
    assert_eq!(plan["samples_per_process"], 10);
    assert_eq!(plan["retained_ascii_raw_samples"], 900);
    assert_eq!(plan["artifact_files"], 5);
    assert_eq!(plan["atomic_raw_record_files"], 90);
    assert_eq!(plan["process_timeout_seconds"], 60);
    assert_eq!(
        plan["aggregation"]["process_representative"],
        "nearest-rank-p50"
    );
    assert_eq!(plan["aggregation"]["cross_process"], "nearest-rank-p50");
    assert_eq!(
        plan["aggregation"]["flattening_for_percentiles"],
        "forbidden"
    );
    assert_eq!(
        plan["thresholds"]["current_minus_shared_min_bytes"],
        64 * 1024 * 1024
    );
    assert_eq!(
        plan["thresholds"]["shared_minus_lazy_min_bytes"],
        32 * 1024 * 1024
    );

    let warmups = plan["schedule"]["warmups"]
        .as_array()
        .expect("warmup schedule");
    let measured = plan["schedule"]["measured"]
        .as_array()
        .expect("measured schedule");
    let specimens = plan["schedule"]["functional_specimens"]
        .as_array()
        .expect("functional specimen schedule");
    assert_eq!(warmups.len(), 15);
    assert_eq!(measured.len(), 90);
    assert_eq!(specimens.len(), 6);
    assert_eq!(
        plan["artifacts"],
        serde_json::json!([
            "font-ownership-raw.json",
            "font-ownership-aggregate.json",
            "runner-fingerprint.json",
            "font-catalog-fingerprint.json",
            "artifact-manifest-fragment.json"
        ]),
        "WhatIf must not advertise unreferenced identity or specimen files"
    );
    for round in 0..5 {
        assert_eq!(
            warmups[(round * 3)..(round * 3 + 3)]
                .iter()
                .map(|item| item["mode"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["current", "shared", "lazy"]
        );
    }
    for round in 0..30 {
        assert_eq!(
            measured[(round * 3)..(round * 3 + 3)]
                .iter()
                .map(|item| item["mode"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["current", "shared", "lazy"]
        );
    }
    assert!(!output_directory.exists(), "WhatIf created evidence");
}

#[test]
fn font_proof_retains_process_records_and_functional_specimens_without_flattening() {
    let fixture = FontProofFixture::new("valid", "valid");
    let output = fixture.run(1, 1);
    assert!(
        output.status.success(),
        "font proof failed: {}",
        combined_output(&output)
    );
    let run: Value = serde_json::from_slice(&output.stdout).expect("parse runner summary");
    assert_eq!(
        run["certification_eligible"], false,
        "SkipBuild with binary overrides is never certification evidence"
    );

    let raw = fixture.read_json("font-ownership-raw.json");
    let aggregate = fixture.read_json("font-ownership-aggregate.json");
    let runner = fixture.read_json("runner-fingerprint.json");
    let catalog = fixture.read_json("font-catalog-fingerprint.json");
    for payload in [&raw, &aggregate, &runner, &catalog] {
        assert_eq!(
            payload["certification_eligible"], false,
            "development proof payloads must carry the real non-certifying state"
        );
    }
    assert_eq!(runner["source"], "fixture");
    assert_eq!(runner["complete"], true);
    assert_eq!(runner["collector_timeout_seconds"], 60);
    assert!(runner["identity"].get("binary_hashes").is_none());
    assert!(
        runner["identity"]
            .get("runner_fingerprint_sha256")
            .is_none()
    );
    assert_eq!(runner["claims"]["fingerprint_fields_complete"], true);
    assert!(
        runner["fields"]["displays"]
            .as_array()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(
        runner["fields"]["cold_cache_policy"]["os_file_cache"],
        "unmodified-no-explicit-flush"
    );
    for payload in [&raw, &aggregate, &catalog] {
        assert_eq!(
            payload["identity"]["runner_fingerprint_sha256"],
            runner["fingerprint_sha256"]
        );
    }
    assert_font_proof_raw(&raw);
    assert_font_proof_aggregate(&aggregate);
    assert_font_proof_catalog(&catalog);
    assert_font_proof_fragment(&fixture);
    assert_eq!(
        recursive_relative_files(&fixture.output_directory),
        [
            "artifact-manifest-fragment.json",
            "font-catalog-fingerprint.json",
            "font-ownership-aggregate.json",
            "font-ownership-raw.json",
            "runner-fingerprint.json",
        ],
        "the evidence root must contain exactly the five advertised artifacts"
    );
    assert!(
        recursive_relative_files(&fixture.staging_temp).is_empty(),
        "successful proof must delete its external atomic-record staging tree"
    );
}

fn assert_font_proof_raw(raw: &Value) {
    assert_eq!(raw["warmups"], 3);
    let warmups = raw["warmup_process_ids"].as_array().expect("warmup IDs");
    assert_eq!(warmups.len(), 3);
    assert_eq!(
        warmups
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
    assert!(warmups.iter().all(|id| {
        id.as_str()
            .is_some_and(|value| value.starts_with("empty-window-"))
    }));
    assert_eq!(raw["measured_cold_processes"], 1);
    let groups = raw["groups"].as_array().expect("raw groups");
    assert_eq!(groups.len(), 3);
    for (index, name) in ["current-copied/ascii", "shared-all/ascii", "lazy/ascii"]
        .iter()
        .enumerate()
    {
        assert_eq!(groups[index]["name"], *name);
        assert_eq!(
            groups[index]["warmup_process_ids"]
                .as_array()
                .expect("per-mode warmups")
                .len(),
            1
        );
        assert!(groups[index].get("samples").is_none());
        assert!(groups[index].get("flattened_samples").is_none());
        let processes = groups[index]["processes"]
            .as_array()
            .expect("process records");
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0]["round_index"], 1);
        assert!(processes[0]["font_resources"].is_object());
        assert!(
            processes[0]["font_resources"]["initial_catalog_source_count"]
                .as_u64()
                .is_some_and(|value| value >= 1)
        );
        assert_eq!(processes[0]["samples"].as_array().unwrap().len(), 10);
        assert_eq!(
            processes[0]["representative"], processes[0]["samples"][4],
            "ten-point nearest-rank p50 is the fifth ordered value"
        );
    }
    assert_eq!(
        raw["warmup_process_ids"].as_array().unwrap().len(),
        usize::try_from(raw["warmups"].as_u64().unwrap()).expect("fixture warmup count fits usize")
    );
}

fn assert_font_proof_aggregate(aggregate: &Value) {
    assert_eq!(
        aggregate["raw_children"],
        serde_json::json!(["font-ownership-raw"])
    );
    assert_eq!(aggregate["ok"], true);
    let statistics = aggregate["group_statistics"]
        .as_array()
        .expect("group statistics");
    assert_eq!(statistics.len(), 3);
    assert_eq!(
        statistics[0]["p50"].as_u64().unwrap() - statistics[1]["p50"].as_u64().unwrap(),
        100 * 1024 * 1024
    );
    assert_eq!(
        statistics[1]["p50"].as_u64().unwrap() - statistics[2]["p50"].as_u64().unwrap(),
        50 * 1024 * 1024
    );
}

fn assert_font_proof_catalog(catalog: &Value) {
    let specimens = catalog["functional_specimens"]
        .as_array()
        .expect("functional specimens");
    assert_eq!(specimens.len(), 6);
    let mut specimen_digest = String::with_capacity(64);
    for byte in Sha256::digest(serde_json::to_vec(specimens).unwrap()) {
        write!(&mut specimen_digest, "{byte:02x}").expect("write digest to String");
    }
    assert_eq!(
        catalog["catalog_fingerprint_sha256"], specimen_digest,
        "catalog proof must be the canonical six-record digest"
    );
    for specimen in specimens {
        assert_eq!(specimen["tofu_count"], 0);
        assert_eq!(specimen["frame_generation_consistent"], true);
        assert_eq!(
            specimen["recovery_retained_source_bytes"],
            specimen["retained_source_bytes"]
        );
        assert!(specimen["activation_latency_ms"].is_number());
        assert_eq!(specimen["activation_latency_gate"], "report-only");
    }
}

fn assert_font_proof_fragment(fixture: &FontProofFixture) {
    for path in [
        "font-ownership-raw.json",
        "font-ownership-aggregate.json",
        "runner-fingerprint.json",
        "font-catalog-fingerprint.json",
        "artifact-manifest-fragment.json",
    ] {
        assert!(
            fixture.output_directory.join(path).is_file(),
            "missing {path}"
        );
    }
    let fragment = fixture.read_json("artifact-manifest-fragment.json");
    let entries = fragment["entries"].as_array().expect("fragment entries");
    assert_eq!(entries.len(), 4);
    let mut referenced_paths = entries
        .iter()
        .map(|entry| entry["path"].as_str().expect("entry path"))
        .collect::<Vec<_>>();
    referenced_paths.sort_unstable();
    assert_eq!(
        referenced_paths,
        [
            "font-catalog-fingerprint.json",
            "font-ownership-aggregate.json",
            "font-ownership-raw.json",
            "runner-fingerprint.json",
        ],
        "fragment must reference every non-fragment file and no staging file"
    );
    for entry in entries {
        assert_eq!(entry["certification_eligible"], false);
        assert!(
            entry["source_sha"]
                .as_str()
                .is_some_and(|value| value.len() == 40)
        );
        if entry["artifact_type"] == "runner-fingerprint" {
            assert!(entry.get("binary_hashes").is_none());
            assert!(entry.get("runner_fingerprint_sha256").is_none());
        } else {
            assert!(entry["binary_hashes"].is_object());
            assert!(
                entry["runner_fingerprint_sha256"]
                    .as_str()
                    .is_some_and(|value| value.len() == 64)
            );
        }
    }
    let aggregate_entry = entries
        .iter()
        .find(|entry| entry["artifact_type"] == "font-ownership-aggregate")
        .expect("aggregate fragment entry");
    assert_eq!(
        aggregate_entry["children"],
        serde_json::json!(["font-ownership-raw"])
    );
}

const FONT_PROOF_FAILURE_CASES: &[(&str, &str, &str)] = &[
    ("mode", "mode-fallback", "font mode/specimen fallback"),
    (
        "specimen",
        "specimen-fallback",
        "font mode/specimen fallback",
    ),
    ("backend", "mixed-backend", "mixed actual GPU identity"),
    ("counter", "missing-counter", "memory counter"),
    ("binary", "dirty-binary", "binary identity changed"),
    ("threshold", "threshold", "p50 reduction is below"),
    ("tofu", "tofu", "font resource counters"),
    ("generation", "mixed-generation", "font resource counters"),
    ("recovery", "recovery-duplication", "font resource counters"),
    ("path", "raw-path", "font_resources fields differ"),
    (
        "owner-ready-order",
        "owner-ready-order",
        "font ownership readiness marker order",
    ),
    (
        "current-counter-shape",
        "current-counter-shape",
        "font resource counters",
    ),
    (
        "current-builds-too-large",
        "current-builds-too-large",
        "font resource counters",
    ),
    (
        "current-build-generation-mismatch",
        "current-build-generation-mismatch",
        "font resource counters",
    ),
    (
        "shared-counter-shape",
        "shared-counter-shape",
        "font resource counters",
    ),
    (
        "lazy-ascii-counter-shape",
        "lazy-ascii-counter-shape",
        "font resource mode counter shape",
    ),
    (
        "lazy-activation-counter-shape",
        "lazy-activation-counter-shape",
        "font resource mode counter shape",
    ),
    (
        "uppercase-adapter",
        "uppercase-adapter",
        "production adapter type",
    ),
    (
        "unknown-adapter",
        "unknown-adapter",
        "production adapter type",
    ),
    (
        "missing-initial",
        "missing-initial",
        "font_resources fields differ",
    ),
    (
        "cross-catalog",
        "cross-catalog",
        "catalog_fingerprint_sha256 differs",
    ),
    (
        "cross-retained",
        "cross-retained",
        "retained bytes must equal exactly twice SharedAll",
    ),
];

#[test]
fn font_proof_fails_before_fragment_on_fallback_counter_identity_and_resource_drift() {
    for &(label, mode, expected) in FONT_PROOF_FAILURE_CASES {
        let fixture = FontProofFixture::new(label, mode);
        let output = fixture.run(0, 1);
        assert!(
            !output.status.success(),
            "{label} drift unexpectedly passed: {}",
            combined_output(&output)
        );
        assert!(
            combined_output(&output).contains(expected),
            "{label} drift failure did not mention {expected:?}: {}",
            combined_output(&output)
        );
        assert!(
            !fixture
                .output_directory
                .join("artifact-manifest-fragment.json")
                .exists(),
            "{label} drift emitted a fragment"
        );
        if mode == "threshold" {
            let atomic_records = recursive_relative_files(&fixture.staging_temp);
            assert_eq!(
                atomic_records.len(),
                3,
                "failed collection must retain completed atomic records in external TEMP"
            );
            assert!(
                atomic_records
                    .iter()
                    .all(|path| path.ends_with("round-001.json"))
            );
            assert!(
                !fixture.output_directory.join("raw").exists(),
                "unmanifested atomic records must never enter the evidence root"
            );
        }
    }
}

#[test]
fn font_proof_timeout_kills_the_process_tree_and_never_publishes_a_fragment() {
    let fixture = FontProofFixture::new("timeout", "timeout");
    let output = fixture.run_with_timeout(0, 1, 1);
    assert!(
        !output.status.success(),
        "timeout fixture unexpectedly succeeded"
    );
    let rendered = combined_output(&output);
    assert!(
        rendered.contains("process tree was killed and reaped"),
        "{rendered}"
    );
    assert!(
        !fixture
            .output_directory
            .join("artifact-manifest-fragment.json")
            .exists(),
        "timeout emitted a usable fragment"
    );
}

#[test]
fn font_proof_fixture_fingerprint_seams_fail_closed_and_cannot_certify() {
    let collector_failure = FontProofFixture::new("collector-failure", "valid");
    let output = collector_failure.run_with_fingerprint_fault("collector-failure");
    assert!(!output.status.success());
    assert!(
        combined_output(&output).contains("runner fingerprint collector failed"),
        "{}",
        combined_output(&output)
    );
    assert!(
        !collector_failure
            .output_directory
            .join("artifact-manifest-fragment.json")
            .exists()
    );

    let missing_field = FontProofFixture::new("fingerprint-missing-field", "valid");
    let mut input: Value = serde_json::from_slice(
        &fs::read(&missing_field.fingerprint_input).expect("read fingerprint fixture"),
    )
    .expect("parse fingerprint fixture");
    input["locale"]
        .as_object_mut()
        .unwrap()
        .remove("system_locale");
    fs::write(
        &missing_field.fingerprint_input,
        serde_json::to_vec(&input).unwrap(),
    )
    .expect("write incomplete fingerprint fixture");
    let output = missing_field.run(0, 1);
    assert!(!output.status.success());
    assert!(combined_output(&output).contains("runner fingerprint fields"));

    let certification = FontProofFixture::new("fixture-certification", "valid");
    let output = certification.run_certification_with_fixture_input();
    assert!(!output.status.success());
    assert!(combined_output(&output).contains("test-only runner fingerprint input"));
    assert!(!certification.output_directory.exists());
}

#[test]
fn font_proof_rejects_successful_collector_output_with_any_stderr() {
    let fixture = FontProofFixture::new("collector-stderr", "valid");
    let output = fixture.run_with_fingerprint_fault("collector-stderr");
    assert!(
        !output.status.success(),
        "collector stderr unexpectedly passed"
    );
    assert!(
        combined_output(&output).contains("collector stderr must be empty"),
        "{}",
        combined_output(&output)
    );
    assert!(
        !fixture
            .output_directory
            .join("artifact-manifest-fragment.json")
            .exists()
    );
}

#[test]
fn runner_collector_driver_and_remotefx_helpers_fail_closed() {
    let script = repo_path("scripts/ci/collect-stage7-runner-fingerprint.ps1");
    let command = format!(
        r"$path = '{}'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
foreach ($name in @('Assert-SelectedGpuDriverIdentity', 'Resolve-RemoteSessionKind')) {{
  $node = $ast.Find({{ param($item) $item -is [Management.Automation.Language.FunctionDefinitionAst] -and $item.Name -eq $name }}, $true)
  Invoke-Expression $node.Extent.Text
}}
$driverMismatch = ''
try {{ Assert-SelectedGpuDriverIdentity -CimDriverVersion '32.0.16.2002' -DxdiagDriverVersion '31.0.15.9999' }} catch {{ $driverMismatch = $_.Exception.Message }}
$registryFailure = ''
try {{ Resolve-RemoteSessionKind -RemoteMetric $false -CurrentSessionId 7 -GlassSessionId 7 -RegistryProbeFailed }} catch {{ $registryFailure = $_.Exception.Message }}
[ordered]@{{
  driver_mismatch = $driverMismatch
  remotefx_remote = Resolve-RemoteSessionKind -RemoteMetric $false -CurrentSessionId 7 -GlassSessionId 1
  local = Resolve-RemoteSessionKind -RemoteMetric $false -CurrentSessionId 7 -GlassSessionId 7
  sm_remote = Resolve-RemoteSessionKind -RemoteMetric $true -CurrentSessionId 7 -GlassSessionId 7
  registry_failure = $registryFailure
}} | ConvertTo-Json -Compress",
        script.to_string_lossy().replace('\'', "''")
    );
    let output = Command::new("pwsh.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .current_dir(repo_path("."))
        .output()
        .expect("execute collector helper contract");
    assert!(
        output.status.success(),
        "collector helper probe failed: {}",
        combined_output(&output)
    );
    let result: Value = serde_json::from_slice(&output.stdout).expect("parse helper probe JSON");
    assert!(
        result["driver_mismatch"]
            .as_str()
            .unwrap()
            .contains("CIM/dxdiag driver version mismatch")
    );
    assert_eq!(result["remotefx_remote"], "remote");
    assert_eq!(result["local"], "local");
    assert_eq!(result["sm_remote"], "remote");
    assert!(
        result["registry_failure"]
            .as_str()
            .unwrap()
            .contains("GlassSessionId registry probe failed")
    );
}

macro_rules! collector_host_fields_probe_command {
    ($path:expr) => {
        format!(
        r#"$path = '{}'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
foreach ($name in @('Get-RemoteSessionMetric', 'Get-CurrentProcessSessionId', 'Get-GlassSessionId', 'Resolve-RemoteSessionKind', 'Get-RemoteSessionKind', 'Get-HostFields')) {{
  $node = $ast.Find({{ param($item) $item -is [Management.Automation.Language.FunctionDefinitionAst] -and $item.Name -eq $name }}, $true)
  Invoke-Expression $node.Extent.Text
}}
$driverNode = $ast.Find({{ param($item) $item -is [Management.Automation.Language.FunctionDefinitionAst] -and $item.Name -eq 'Assert-SelectedGpuDriverIdentity' }}, $true)
Invoke-Expression ($driverNode.Extent.Text -replace 'function Assert-SelectedGpuDriverIdentity', 'function Assert-SelectedGpuDriverIdentityImplementation')
$GpuVendorId = [UInt64]4318
$GpuDeviceId = [UInt64]11524
$GpuAdapterName = 'fixture-adapter'
$TestFault = 'none'
$script:automaticManagedPagefile = $true
$script:pagefileSettingCount = 0
$script:currentSessionId = [UInt32]7
$script:glassSessionId = [UInt32]7
$script:currentSessionFailure = $false
$script:glassSessionFailure = $false
$script:currentSessionCalls = 0
$script:glassSessionCalls = 0
$script:driverAssertionCalls = 0
$script:hostSessionBindingCalls = 0
$script:dxdiagDriverVersion = '32.0.16.2002'
function Get-CimInstance {{
  [CmdletBinding()] param([string] $ClassName)
  switch ($ClassName) {{
    'Win32_OperatingSystem' {{ [pscustomobject]@{{ Version = '10.0.26300'; BuildNumber = '26300' }} }}
    'Win32_ComputerSystem' {{ [pscustomobject]@{{ AutomaticManagedPagefile = $script:automaticManagedPagefile; TotalPhysicalMemory = [UInt64]68003237888 }} }}
    'Win32_VideoController' {{ [pscustomobject]@{{ PNPDeviceID = 'PCI\VEN_10DE&DEV_2D04'; Name = 'fixture-adapter'; DriverVersion = '32.0.16.2002' }} }}
    'Win32_PageFileSetting' {{ if ($script:pagefileSettingCount -gt 0) {{ [pscustomobject]@{{ Name = 'pagefile.sys' }} }} }}
    default {{ throw "unexpected CIM class $ClassName" }}
  }}
}}
function Get-ItemPropertyValue {{
  [CmdletBinding()] param([string] $LiteralPath, [string] $Name)
  if ($Name -eq 'UBR') {{ return [UInt32]9032 }}
  if ($Name -eq 'GlassSessionId') {{
    $script:glassSessionCalls++
    if ($script:glassSessionFailure) {{ throw 'simulated GlassSessionId registry failure' }}
    return $script:glassSessionId
  }}
  throw "unexpected registry value $Name"
}}
function Get-ActiveDisplays {{
  return ,([pscustomobject]@{{ width_px = [UInt64]2560; height_px = [UInt64]1440; dpi_x = [UInt64]96; dpi_y = [UInt64]96; primary = $true }})
}}
function powercfg.exe {{ $global:LASTEXITCODE = 0; 'Power Scheme GUID: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c' }}
function Get-RemoteSessionMetric {{ return $false }}
function Get-CurrentProcessSessionId {{
  $script:currentSessionCalls++
  if ($script:currentSessionFailure) {{ throw 'simulated current SessionId failure' }}
  return $script:currentSessionId
}}
function Get-RemoteSessionKind {{ $script:hostSessionBindingCalls++; return 'local' }}
function Get-WddmIdentity {{ return [pscustomobject]@{{ DriverVersion = $script:dxdiagDriverVersion; WddmVersion = 'WDDM 3.2' }} }}
function Assert-SelectedGpuDriverIdentity {{
  param([string] $CimDriverVersion, [string] $DxdiagDriverVersion)
  $script:driverAssertionCalls++
  Assert-SelectedGpuDriverIdentityImplementation -CimDriverVersion $CimDriverVersion -DxdiagDriverVersion $DxdiagDriverVersion
}}
function Get-WinSystemLocale {{ return [pscustomobject]@{{ Name = 'en-US' }} }}
function Invoke-HostFieldsCase([object] $AutomaticManagedPagefile, [int] $PagefileSettings = 0) {{
  $script:automaticManagedPagefile = $AutomaticManagedPagefile
  $script:pagefileSettingCount = $PagefileSettings
  $script:currentSessionCalls = 0
  $script:glassSessionCalls = 0
  $script:driverAssertionCalls = 0
  $script:hostSessionBindingCalls = 0
  $result = Get-HostFields
  $result | Add-Member -NotePropertyName current_session_calls -NotePropertyValue $script:currentSessionCalls
  $result | Add-Member -NotePropertyName glass_session_calls -NotePropertyValue $script:glassSessionCalls
  $result | Add-Member -NotePropertyName driver_assertion_calls -NotePropertyValue $script:driverAssertionCalls
  $result | Add-Member -NotePropertyName host_session_binding_calls -NotePropertyValue $script:hostSessionBindingCalls
  return $result
}}
$automatic = Invoke-HostFieldsCase -AutomaticManagedPagefile $true
$manual = Invoke-HostFieldsCase -AutomaticManagedPagefile $false -PagefileSettings 1
$disabled = Invoke-HostFieldsCase -AutomaticManagedPagefile $false
$nullError = ''
try {{ $null = Invoke-HostFieldsCase -AutomaticManagedPagefile $null }} catch {{ $nullError = $_.Exception.Message }}
$stringError = ''
try {{ $null = Invoke-HostFieldsCase -AutomaticManagedPagefile 'false' }} catch {{ $stringError = $_.Exception.Message }}
$remoteNode = $ast.Find({{ param($item) $item -is [Management.Automation.Language.FunctionDefinitionAst] -and $item.Name -eq 'Get-RemoteSessionKind' }}, $true)
Invoke-Expression $remoteNode.Extent.Text
$script:automaticManagedPagefile = $true
$sessionBinding = $null
$sessionBindingError = ''
try {{ $sessionBinding = Invoke-HostFieldsCase -AutomaticManagedPagefile $true }} catch {{ $sessionBindingError = $_.Exception.Message }}
$boundCurrentSessionCalls = $script:currentSessionCalls
$boundGlassSessionCalls = $script:glassSessionCalls
$script:currentSessionFailure = $true
$currentSessionError = ''
try {{ $null = Get-HostFields }} catch {{ $currentSessionError = $_.Exception.Message }}
$script:currentSessionFailure = $false
$script:glassSessionFailure = $true
$glassSessionError = ''
try {{ $null = Get-HostFields }} catch {{ $glassSessionError = $_.Exception.Message }}
$script:glassSessionFailure = $false
$script:dxdiagDriverVersion = '31.0.15.9999'
$driverError = ''
try {{ $null = Get-HostFields }} catch {{ $driverError = $_.Exception.Message }}
[ordered]@{{
  automatic = $automatic.memory.pagefile_mode
  manual = $manual.memory.pagefile_mode
  disabled = $disabled.memory.pagefile_mode
  session = $sessionBinding.session.kind
  driver = $automatic.gpu.driver_version
  host_session_binding_calls = $automatic.host_session_binding_calls
  current_session_calls = $boundCurrentSessionCalls
  glass_session_calls = $boundGlassSessionCalls
  driver_assertion_calls = $automatic.driver_assertion_calls
  session_binding_error = $sessionBindingError
  null_error = $nullError
  string_error = $stringError
  current_session_error = $currentSessionError
  glass_session_error = $glassSessionError
  driver_error = $driverError
}} | ConvertTo-Json -Compress"#,
            $path
        )
    };
}

#[test]
fn runner_collector_host_fields_bind_strict_pagefile_session_and_driver_probes() {
    let script = repo_path("scripts/ci/collect-stage7-runner-fingerprint.ps1");
    let command =
        collector_host_fields_probe_command!(script.to_string_lossy().replace('\'', "''"));
    let output = Command::new("pwsh.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .current_dir(repo_path("."))
        .output()
        .expect("execute actual collector host-fields contract");
    assert!(
        output.status.success(),
        "actual host-fields probe failed: {}",
        combined_output(&output)
    );
    let result: Value = serde_json::from_slice(&output.stdout).expect("parse host-fields JSON");
    assert_eq!(result["automatic"], "automatic-managed");
    assert_eq!(result["manual"], "manual");
    assert_eq!(result["disabled"], "disabled");
    for field in ["null_error", "string_error"] {
        assert!(
            result[field]
                .as_str()
                .is_some_and(|message| message.contains("AutomaticManagedPagefile")),
            "{field} did not fail closed: {result}"
        );
    }
    assert_eq!(result["session"], "local", "{result}");
    assert_eq!(result["driver"], "32.0.16.2002");
    assert_eq!(result["host_session_binding_calls"], 1);
    assert_eq!(result["current_session_calls"], 1);
    assert_eq!(result["glass_session_calls"], 1);
    assert_eq!(result["driver_assertion_calls"], 1);
    assert_eq!(result["session_binding_error"], "");
    assert!(
        result["current_session_error"]
            .as_str()
            .is_some_and(|message| message.contains("current process SessionId"))
    );
    assert!(
        result["glass_session_error"]
            .as_str()
            .is_some_and(|message| message.contains("GlassSessionId registry probe failed"))
    );
    assert!(
        result["driver_error"]
            .as_str()
            .is_some_and(|message| message.contains("CIM/dxdiag driver version mismatch"))
    );
}

#[test]
fn runner_fingerprint_canonical_protocol_preserves_arrays_and_unicode() {
    let fixture = FontProofFixture::new("runner-canonical-golden", "valid");
    for display_count in [1_usize, 2] {
        for system_locale in ["en-US", "中文-测试"] {
            let mut fields = fixture_runner_fingerprint_fields();
            fields["displays"]
                .as_array_mut()
                .expect("display fixture")
                .truncate(display_count);
            fields["locale"]["system_locale"] = Value::String(system_locale.to_owned());
            fs::write(
                &fixture.fingerprint_input,
                serde_json::to_vec_pretty(&fields).expect("serialize canonical fixture"),
            )
            .expect("write canonical fixture");
            let output = Command::new("pwsh.exe")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-File",
                    repo_path("scripts/ci/collect-stage7-runner-fingerprint.ps1")
                        .to_str()
                        .expect("UTF-8 collector path"),
                    "-GpuVendorId",
                    "4318",
                    "-GpuDeviceId",
                    "11524",
                    "-GpuAdapterName",
                    "fixture-adapter",
                    "-FontInventoryFingerprintSha256",
                    &"8".repeat(64),
                    "-FontIndexPolicyVersion",
                    "1",
                    "-TestInputPath",
                    fixture
                        .fingerprint_input
                        .to_str()
                        .expect("UTF-8 fingerprint fixture path"),
                ])
                .current_dir(repo_path("."))
                .output()
                .expect("execute shared fingerprint collector");
            assert!(
                output.status.success(),
                "collector failed: {}",
                combined_output(&output)
            );
            let observation: Value =
                serde_json::from_slice(&output.stdout).expect("collector emitted one JSON value");
            assert_eq!(
                observation["fields"]["displays"].as_array().unwrap().len(),
                display_count
            );
            assert_eq!(
                observation["fingerprint_sha256"],
                runner_canonical_sha256(&fields),
                "PowerShell/Python runner canonical golden drifted for {display_count} display(s), locale {system_locale}"
            );
        }
    }
}

#[test]
fn runner_fingerprint_host_probe_faults_fail_closed_without_json() {
    for (fault, expected) in [
        ("wrong-adapter-name", "selected GPU vendor/device/name"),
        ("invalid-wddm", "WDDM"),
        ("dpi-probe-failure", "display DPI"),
        ("pagefile-probe-failure", "pagefile"),
        ("session-probe-failure", "remote session"),
    ] {
        let output = Command::new("pwsh.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-File",
                repo_path("scripts/ci/collect-stage7-runner-fingerprint.ps1")
                    .to_str()
                    .expect("UTF-8 collector path"),
                "-GpuVendorId",
                "4318",
                "-GpuDeviceId",
                "11524",
                "-GpuAdapterName",
                "fixture-adapter",
                "-FontInventoryFingerprintSha256",
                &"8".repeat(64),
                "-FontIndexPolicyVersion",
                "1",
                "-TestFault",
                fault,
            ])
            .current_dir(repo_path("."))
            .output()
            .expect("execute failing shared fingerprint collector");
        assert!(!output.status.success(), "{fault} unexpectedly succeeded");
        assert!(
            combined_output(&output).contains(expected),
            "{fault} did not report {expected:?}: {}",
            combined_output(&output)
        );
        assert!(output.stdout.is_empty(), "{fault} emitted usable JSON");
    }
}

#[test]
fn font_proof_bounds_the_whole_collector_process_tree() {
    let fixture = FontProofFixture::new("collector-timeout", "valid");
    let output = fixture.run_with_fingerprint_fault_and_timeout("collector-timeout", 1);
    assert!(
        !output.status.success(),
        "collector timeout unexpectedly succeeded"
    );
    assert!(
        combined_output(&output).contains("tree was killed and reaped"),
        "{}",
        combined_output(&output)
    );
    assert!(
        !fixture
            .output_directory
            .join("artifact-manifest-fragment.json")
            .exists()
    );
}

#[test]
fn font_proof_finalization_failures_rebuild_external_staging_without_a_fragment() {
    for (fault, expected) in [
        (
            "final-identity-failure",
            "final collection identity failpoint",
        ),
        ("final-summary-failure", "summary serialization failpoint"),
        ("final-output-failure", "summary output failpoint"),
        (
            "final-environment-failure",
            "environment restoration failpoint",
        ),
    ] {
        let fixture = FontProofFixture::new(fault, "valid");
        let output = fixture.run_with_fingerprint_fault(fault);
        assert!(!output.status.success(), "{fault} unexpectedly succeeded");
        assert!(
            combined_output(&output).contains(expected),
            "{fault} did not report {expected:?}: {}",
            combined_output(&output)
        );
        assert!(
            !fixture
                .output_directory
                .join("artifact-manifest-fragment.json")
                .exists(),
            "{fault} published a fragment"
        );
        assert_eq!(
            recursive_relative_files(&fixture.staging_temp).len(),
            3,
            "{fault} did not replay all completed atomic records"
        );
        let raw = fixture.read_json("font-ownership-raw.json");
        for group in raw["groups"].as_array().expect("raw font groups") {
            let (mode, expected_name) = match group["name"].as_str().unwrap() {
                "current-copied/ascii" => ("current", "current-copied/ascii"),
                "shared-all/ascii" => ("shared", "shared-all/ascii"),
                "lazy/ascii" => ("lazy", "lazy/ascii"),
                name => panic!("unexpected raw group {name}"),
            };
            assert_eq!(group["name"], expected_name);
            let process = group["processes"]
                .as_array()
                .expect("raw processes")
                .first()
                .expect("one measured process")
                .clone();
            let replay_relative = recursive_relative_files(&fixture.staging_temp)
                .into_iter()
                .find(|path| path.ends_with(&format!("/raw/{mode}/round-001.json")))
                .unwrap_or_else(|| panic!("missing replayed {mode} process payload"));
            let replay_path = fixture.staging_temp.join(replay_relative);
            let replay: Value = serde_json::from_slice(
                &fs::read(&replay_path)
                    .unwrap_or_else(|error| panic!("read {}: {error}", replay_path.display())),
            )
            .expect("parse replayed atomic payload");
            let expected_payload = serde_json::json!({
                "schema": "rterm.stage7/font-ownership-process/v1",
                "certification_eligible": raw["certification_eligible"].clone(),
                "identity": raw["identity"].clone(),
                "requested_backend": group["requested_backend"].clone(),
                "actual_backend": group["actual_backend"].clone(),
                "mode": mode,
                "specimen": "ascii",
                "round_index": 1,
                "timeout_seconds": raw["timeout_seconds"].clone(),
                "stabilization_ms": group["stabilization_ms"].clone(),
                "sample_interval_ms": group["sample_interval_ms"].clone(),
                "process": process,
            });
            assert_eq!(
                replay, expected_payload,
                "{fault} did not replay the complete {mode} process payload"
            );
        }
    }
}

#[test]
fn font_proof_certification_requires_a_build_without_overrides_and_a_fully_clean_tree() {
    let source = fs::read_to_string(repo_path("scripts/ci/run-stage7-font-proof.ps1"))
        .expect("read font proof runner");
    let collector = fs::read_to_string(repo_path(
        "scripts/ci/collect-stage7-runner-fingerprint.ps1",
    ))
    .expect("read shared runner fingerprint collector");
    assert!(source.contains(
        "$certificationEligible = -not $SkipBuild -and -not $hasAppOverride -and -not $hasLauncherOverride"
    ));
    assert!(source.contains("git -C $repoRoot status --porcelain"));
    assert!(
        !source.contains("--untracked-files=no"),
        "untracked source changes must make certification fail closed"
    );
    assert!(source.contains("function Assert-CollectionIdentityUnchanged"));
    assert!(source.contains("process-harness.ps1"));
    assert!(source.contains("Invoke-BoundedProcess"));
    assert!(source.contains("-TimeoutSeconds $ProcessTimeoutSeconds"));
    assert!(source.contains("source commit changed during collection"));
    assert!(collector.contains("-TimeoutSeconds 60"));
    assert!(collector.contains("collector_timeout_seconds = 60"));
    let final_identity_check = source
        .rfind("Assert-CollectionIdentityUnchanged")
        .expect("final source/binary/clean-tree identity check");
    let fragment_publish = source
        .rfind("Write-AtomicJson $fragmentPath $fragment")
        .expect("atomic fragment publication");
    assert!(
        final_identity_check < fragment_publish,
        "final identity check must precede fragment publication"
    );
    let summary_serialization = source
        .rfind("$summaryJson =")
        .expect("pre-serialized stdout summary");
    let summary_output = source
        .rfind("Write-Output $summaryJson")
        .expect("stdout summary emitted before publication");
    let environment_restore = source
        .rfind("Restore-BenchmarkEnvironment -AllowTestFault")
        .expect("explicit pre-publication environment restore");
    assert!(summary_serialization < summary_output);
    assert!(summary_output < environment_restore);
    assert!(environment_restore < fragment_publish);
    let after_publish =
        &source[(fragment_publish + "Write-AtomicJson $fragmentPath $fragment".len())..];
    assert!(!after_publish.contains("ConvertTo-Json"));
    assert!(!after_publish.contains("Write-Output"));
    assert!(!after_publish.contains("Assert-"));
    assert!(
        after_publish.contains("publication path is an explicit no-op"),
        "success finally branch must be visibly no-op after fragment publication"
    );
}

#[test]
fn font_proof_real_run_requires_an_absolute_cargo_target_outside_the_repository() {
    let inside_target = repo_path("target/font-proof-inside");
    for (label, target, expected) in [
        ("missing-target", None, "CARGO_TARGET_DIR is required"),
        (
            "relative-target",
            Some(Path::new("target/font-proof-relative")),
            "CARGO_TARGET_DIR must be absolute",
        ),
        (
            "inside-target",
            Some(inside_target.as_path()),
            "CARGO_TARGET_DIR must be outside the repository",
        ),
    ] {
        let fixture = FontProofFixture::new(label, "valid");
        let output = fixture.run_with_cargo_target(0, 1, target);
        assert!(
            !output.status.success(),
            "{label} unexpectedly passed: {}",
            combined_output(&output)
        );
        assert!(
            combined_output(&output).contains(expected),
            "{label} did not report {expected:?}: {}",
            combined_output(&output)
        );
        assert!(!fixture.output_directory.exists());
    }

    let fixture = FontProofFixture::new("external-target", "valid");
    let external_target = fixture.root.join("external-cargo-target");
    fs::create_dir_all(&external_target).expect("create external cargo target");
    let output = fixture.run_with_cargo_target(0, 1, Some(&external_target));
    assert!(
        output.status.success(),
        "external target failed: {}",
        combined_output(&output)
    );
}

#[test]
fn stage7_gate0_uses_the_font_runners_locked_build_and_measured_round_option() {
    let plan = fs::read_to_string(repo_path("docs/plans/2026-08-23-stage7-split-readiness.md"))
        .expect("read Stage 7 plan");
    let task9 = plan
        .split("### Task 9: Run Gate 0 and publish its decision")
        .nth(1)
        .expect("Task 9 section")
        .split("### Task 10:")
        .next()
        .expect("Task 9 boundary");
    let font_command = task9
        .lines()
        .find(|line| line.contains("run-stage7-font-proof.ps1"))
        .expect("official font proof command");

    assert!(font_command.contains("-MeasuredRounds 30"));
    assert!(!font_command.contains("-Samples"));
    assert!(!font_command.contains("-SkipBuild"));
    assert!(task9.contains("locked release provenance-bound build"));
}

#[test]
fn stage7_font_proof_owns_the_single_reusable_runner_fingerprint_artifact() {
    let plan = fs::read_to_string(repo_path("docs/plans/2026-08-23-stage7-split-readiness.md"))
        .expect("read Stage 7 plan");
    let task4 = plan
        .split("### Task 4:")
        .nth(1)
        .expect("Task 4 section")
        .split("### Task 5:")
        .next()
        .expect("Task 4 boundary");
    let task7 = plan
        .split("### Task 7:")
        .nth(1)
        .expect("Task 7 section")
        .split("### Task 8:")
        .next()
        .expect("Task 7 boundary");

    assert!(task4.contains("Create: `scripts/ci/collect-stage7-runner-fingerprint.ps1`"));
    assert!(task7.contains("Modify: `scripts/ci/collect-stage7-runner-fingerprint.ps1`"));
    assert!(task7.contains("must not emit a second `runner-fingerprint` singleton"));
}

#[test]
fn stage7_attribution_what_if_freezes_interleaved_stage_backend_sampling() {
    let output_directory = std::env::temp_dir().join(format!(
        "rterm-attribution-what-if-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    let output = Command::new("pwsh.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-File",
            repo_path("scripts/ci/run-stage7-attribution-matrix.ps1")
                .to_str()
                .expect("UTF-8 runner path"),
            "-OutputDirectory",
            output_directory.to_str().expect("UTF-8 output path"),
            "-WhatIf",
        ])
        .current_dir(repo_path("."))
        .output()
        .expect("execute attribution matrix WhatIf contract");
    assert!(
        output.status.success(),
        "attribution matrix WhatIf failed: {}",
        combined_output(&output)
    );
    let plan: Value = serde_json::from_slice(&output.stdout).expect("parse WhatIf plan JSON");
    assert_eq!(plan["schema"], "rterm.stage7/attribution-matrix-plan/v1");
    assert_eq!(
        plan["stages"],
        serde_json::json!([
            "cpu-window",
            "instance-surface",
            "adapter-device",
            "configured-surface-clear",
            "layer-pipelines",
            "fixture-font-text",
            "platform-font-index",
            "full-frame"
        ])
    );
    assert_eq!(
        plan["backends"],
        serde_json::json!(["auto", "dx12", "vulkan", "gl"])
    );
    assert_eq!(plan["warmups"], 5);
    assert_eq!(plan["measured_cold_processes"], 30);
    assert_eq!(plan["samples_per_process"], 10);
    assert_eq!(plan["stabilization_ms"], 5_000);
    assert_eq!(plan["sample_interval_ms"], 100);
    assert_eq!(plan["atomic_raw_record_files"], 960);
    assert_eq!(plan["schedule"]["warmups"].as_array().unwrap().len(), 160);
    assert_eq!(plan["schedule"]["measured"].as_array().unwrap().len(), 960);
    assert!(!output_directory.exists(), "WhatIf created evidence");
}

#[test]
fn stage7_attribution_requires_gpu_only_after_the_first_gpu_present() {
    let runner = fs::read_to_string(repo_path("scripts/ci/run-stage7-attribution-matrix.ps1"))
        .expect("read Stage 7 attribution runner");

    assert!(
        runner.contains("$expectedRenderer = if ($stageIndex -ge 3) { \"gpu\" } else { \"cpu\" }"),
        "the runner must derive renderer identity from the actual stage boundary"
    );
    assert!(
        !runner.contains("if ($Record.renderer.final -cne \"gpu\")"),
        "pre-present stages must not be rejected for retaining the CPU renderer"
    );
}

#[test]
fn stage7_attribution_zero_warmups_executes_no_warmup_rounds() {
    let runner = fs::read_to_string(repo_path("scripts/ci/run-stage7-attribution-matrix.ps1"))
        .expect("read Stage 7 attribution runner");

    assert!(
        runner.contains("for ($round = 1; $round -le $Warmups; $round++)"),
        "zero must be represented by an empty warmup loop"
    );
    assert!(
        !runner.contains("foreach ($round in 1..$Warmups)"),
        "PowerShell 1..0 expands to two values and must not drive warmups"
    );
}

#[test]
fn stage7_attribution_deterministic_proof_has_a_real_gate0_runner() {
    let output = Command::new("pwsh.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-File",
            repo_path("scripts/ci/run-stage7-attribution-deterministic-tests.ps1")
                .to_str()
                .expect("UTF-8 deterministic runner path"),
            "-WhatIf",
        ])
        .current_dir(repo_path("."))
        .output()
        .expect("execute attribution deterministic proof WhatIf contract");
    assert!(
        output.status.success(),
        "attribution deterministic proof WhatIf failed: {}",
        combined_output(&output)
    );
    let plan: Value = serde_json::from_slice(&output.stdout).expect("parse WhatIf plan JSON");
    assert_eq!(
        plan["schema"],
        "rterm.stage7/attribution-deterministic-tests-plan/v1"
    );
    assert_eq!(plan["suite_id"], "stage7-attribution-deterministic-v1");
    assert_eq!(
        plan["commands"],
        serde_json::json!([
            [
                "cargo",
                "test",
                "--locked",
                "-p",
                "rterm-app",
                "--test",
                "gpu_backend_memory_matrix_behavior",
                "stage7_attribution",
                "-j1"
            ],
            [
                "python",
                "-m",
                "unittest",
                "scripts.ci.tests.test_check_stage7_split_gate.Stage7SplitGateTests.test_attribution_matrix",
                "-v"
            ]
        ])
    );
}

#[test]
fn stage7_gate0_plan_collects_and_assembles_the_deterministic_proof() {
    let plan = fs::read_to_string(repo_path("docs/plans/2026-08-23-stage7-split-readiness.md"))
        .expect("read Stage 7 plan");
    let gate0 = plan
        .split("### Task 9:")
        .nth(1)
        .expect("Task 9 section")
        .split("### Task 10:")
        .next()
        .expect("Task 9 boundary");

    assert!(gate0.contains("run-stage7-attribution-deterministic-tests.ps1"));
    assert!(gate0.contains("--fragment tests/artifact-manifest-fragment.json"));
}

#[test]
fn matrix_accepts_strict_cpu_and_gpu_records_and_emits_exact_current_evidence() {
    let fixture = MatrixFixture::new("valid", "valid");
    let output = fixture.run();
    assert_success(&output);

    let report = fixture.report();
    assert_eq!(report["binary_source"], "override");
    assert_eq!(report["certification_eligible"], false);
    let probes = report["probes"].as_array().expect("probe reports");
    assert_eq!(probes.len(), 4);
    assert_cpu_identity_omitted(probe(probes, "cpu"));
    for backend in ["dx12", "vulkan", "gl"] {
        let report = probe(probes, backend);
        assert_eq!(report["status"], "succeeded");
        assert_eq!(report["requested_gpu_backend"], backend);
        assert_eq!(report["actual_gpu_backend"], backend);
        assert_eq!(report["adapter_name"], "fixture-adapter");
        assert_eq!(report["adapter_vendor_id"], 4318);
        let expected_device_id = if backend == "gl" { 0 } else { 9860 };
        assert_eq!(report["adapter_device_id"], expected_device_id);
        let expected_adapter_type = if backend == "gl" {
            "other"
        } else {
            "discrete-gpu"
        };
        assert_eq!(report["adapter_type"], expected_adapter_type);
    }
    assert_eq!(
        report["evidence"]["raw_files"],
        serde_json::json!([
            "raw/cpu-01.json",
            "raw/dx12-01.json",
            "raw/vulkan-01.json",
            "raw/gl-01.json"
        ])
    );
}

#[test]
fn matrix_rejects_a_non_empty_output_directory_before_collecting_evidence() {
    let fixture = MatrixFixture::new("non-empty", "valid");
    fs::create_dir_all(&fixture.output_directory).expect("create output directory");
    fs::write(fixture.output_directory.join("stale.json"), b"stale").expect("write stale file");

    let output = fixture.run();
    assert!(
        !output.status.success(),
        "non-empty evidence directory was accepted"
    );
    assert!(
        combined_output(&output).contains("OutputDirectory must be empty"),
        "unexpected output: {}",
        combined_output(&output)
    );
    assert!(!fixture.output_directory.join("aggregate.json").exists());
}

#[test]
fn matrix_rejects_null_memory_bytes_without_coercing_them_to_zero() {
    let fixture = MatrixFixture::new("malformed-bytes", "malformed-dx12-bytes");
    let output = fixture.run();
    assert_success(&output);

    let report = fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    let dx12 = probe(probes, "dx12");
    assert_eq!(dx12["status"], "failed");
    assert!(
        dx12["probe_failure"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("sample bytes"))
    );
    assert_eq!(probe(probes, "vulkan")["status"], "succeeded");
    assert_eq!(probe(probes, "gl")["status"], "succeeded");
}

#[test]
fn matrix_continues_after_failed_cpu_and_gpu_probes_with_safe_identity_fields() {
    let fixture = MatrixFixture::new("failed-probes", "fail-cpu-and-dx12");
    let output = fixture.run();
    assert_success(&output);

    let report = fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    assert_eq!(probes.len(), 4);
    let cpu = probe(probes, "cpu");
    assert_eq!(cpu["status"], "failed");
    assert_cpu_identity_omitted(cpu);
    let dx12 = probe(probes, "dx12");
    assert_eq!(dx12["status"], "failed");
    assert_eq!(dx12["requested_gpu_backend"], "dx12");
    for field in [
        "actual_gpu_backend",
        "adapter_name",
        "adapter_vendor_id",
        "adapter_device_id",
        "adapter_type",
    ] {
        assert!(
            dx12.get(field).is_none(),
            "failed GPU report leaked {field}"
        );
    }
    assert_eq!(probe(probes, "vulkan")["status"], "succeeded");
    assert_eq!(probe(probes, "gl")["status"], "succeeded");
}

#[test]
fn matrix_rejects_numeric_strings_and_object_or_array_adapter_identity() {
    let numeric_fixture = MatrixFixture::new("numeric-strings", "numeric-strings-dx12");
    let output = numeric_fixture.run();
    assert_success(&output);
    let report = numeric_fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    assert_eq!(probe(probes, "dx12")["status"], "failed");
    assert!(
        probe(probes, "dx12")["probe_failure"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("integer scalar"))
    );
    assert_eq!(probe(probes, "vulkan")["status"], "succeeded");

    let adapter_fixture = MatrixFixture::new("adapter-types", "object-array-vulkan-adapter");
    let output = adapter_fixture.run();
    assert_success(&output);
    let report = adapter_fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    assert_eq!(probe(probes, "vulkan")["status"], "failed");
    assert!(
        probe(probes, "vulkan")["probe_failure"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("actual non-empty string"))
    );
    assert_eq!(probe(probes, "gl")["status"], "succeeded");
}

#[test]
fn matrix_rejects_adapter_identity_drift_across_measured_runs() {
    let fixture = MatrixFixture::new("identity-drift", "drift-vulkan-adapter");
    let output = fixture.run_with_samples(2);
    assert_success(&output);

    let report = fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    let vulkan = probe(probes, "vulkan");
    assert_eq!(vulkan["status"], "failed");
    assert!(
        vulkan["probe_failure"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("GPU identity drift"))
    );
    for field in [
        "actual_gpu_backend",
        "adapter_name",
        "adapter_vendor_id",
        "adapter_device_id",
        "adapter_type",
    ] {
        assert!(
            vulkan.get(field).is_none(),
            "drifted GPU report leaked {field}"
        );
    }
    assert_eq!(probe(probes, "gl")["status"], "succeeded");
}

#[test]
fn matrix_redacts_resolved_and_original_override_paths_from_failures() {
    let fixture = MatrixFixture::new("path-redaction", "path-failure");
    let output = fixture.run();
    assert_success(&output);

    let report = fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    let message = probe(probes, "cpu")["probe_failure"]["message"]
        .as_str()
        .expect("CPU failure message");
    for path in fixture.redacted_paths() {
        assert!(
            !message.contains(path.to_str().expect("UTF-8 fixture path")),
            "failure leaked path {}: {message}",
            path.display()
        );
    }
    assert!(message.contains("[path]"));
}

#[test]
fn matrix_rejects_case_changed_wire_identity_values() {
    let first_fixture = MatrixFixture::new("uppercase-wire-a", "uppercase-wire-a");
    let output = first_fixture.run();
    assert_success(&output);
    let report = first_fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    for name in ["cpu", "dx12", "vulkan", "gl"] {
        assert_eq!(
            probe(probes, name)["status"],
            "failed",
            "case-changed wire value was accepted for {name}"
        );
    }

    let second_fixture = MatrixFixture::new("uppercase-wire-b", "uppercase-wire-b");
    let output = second_fixture.run();
    assert_success(&output);
    let report = second_fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    assert_eq!(probe(probes, "dx12")["status"], "failed");
    assert_eq!(probe(probes, "vulkan")["status"], "failed");
    assert_eq!(probe(probes, "gl")["status"], "succeeded");
}

#[test]
fn matrix_rejects_string_encoded_configuration_numbers() {
    let fixture = MatrixFixture::new("string-configuration", "string-config-dx12");
    let output = fixture.run();
    assert_success(&output);

    let report = fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    let dx12 = probe(probes, "dx12");
    assert_eq!(dx12["status"], "failed");
    assert!(
        dx12["probe_failure"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("configuration.stabilization_ms"))
    );
    assert_eq!(probe(probes, "vulkan")["status"], "succeeded");
}

#[test]
fn matrix_rejects_case_changed_and_unknown_production_adapter_types() {
    let fixture = MatrixFixture::new("invalid-adapter-types", "invalid-adapter-types");
    let output = fixture.run();
    assert_success(&output);

    let report = fixture.report();
    let probes = report["probes"].as_array().expect("probe reports");
    for backend in ["dx12", "vulkan"] {
        let failed = probe(probes, backend);
        assert_eq!(failed["status"], "failed");
        assert!(
            failed["probe_failure"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("production adapter type"))
        );
    }
    assert_eq!(probe(probes, "gl")["status"], "succeeded");
    assert_eq!(probe(probes, "gl")["adapter_type"], "other");
}

fn assert_cpu_identity_omitted(report: &Value) {
    for field in [
        "requested_gpu_backend",
        "actual_gpu_backend",
        "adapter_name",
        "adapter_vendor_id",
        "adapter_device_id",
        "adapter_type",
    ] {
        assert!(report.get(field).is_none(), "CPU report leaked {field}");
    }
}

fn probe<'a>(probes: &'a [Value], name: &str) -> &'a Value {
    probes
        .iter()
        .find(|probe| probe["name"] == name)
        .unwrap_or_else(|| panic!("missing probe {name}"))
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "matrix runner failed: {}",
        combined_output(output)
    );
}

fn combined_output(output: &Output) -> String {
    format!(
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn recursive_relative_files(root: &Path) -> Vec<String> {
    if !root.is_dir() {
        return Vec::new();
    }
    let mut pending = vec![root.to_owned()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("read fixture directory") {
            let entry = entry.expect("read fixture entry");
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(
                    path.strip_prefix(root)
                        .expect("file under fixture root")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    files.sort_unstable();
    files
}

struct MatrixFixture {
    root: PathBuf,
    app: PathBuf,
    launcher: PathBuf,
    app_argument: PathBuf,
    launcher_argument: PathBuf,
    output_directory: PathBuf,
}

struct FontProofFixture {
    root: PathBuf,
    app_argument: PathBuf,
    launcher_argument: PathBuf,
    output_directory: PathBuf,
    staging_temp: PathBuf,
    cargo_target: PathBuf,
    fingerprint_input: PathBuf,
}

impl FontProofFixture {
    fn new(label: &str, mode: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rterm-font-proof-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("alias")).expect("create fixture root");
        let app = root.join("fake-rterm-app.exe");
        fs::write(&app, b"font-proof-fixture").expect("write fake app");
        let launcher_script = root.join("fake-font-launcher.ps1");
        let launcher = root.join("fake-font-launcher.cmd");
        let configurations = serialized_font_proof_results();
        fs::write(
            &launcher_script,
            FAKE_FONT_LAUNCHER
                .replace("__FIXTURE_MODE__", mode)
                .replace("__CONFIGURATIONS_JSON__", &configurations),
        )
        .expect("write fake font launcher");
        fs::write(
            &launcher,
            b"@echo off\r\npwsh.exe -NoProfile -NonInteractive -File \"%~dp0fake-font-launcher.ps1\" %*\r\nexit /b %errorlevel%\r\n",
        )
        .expect("write fake font launcher command");
        let staging_temp = root.join("temp");
        let cargo_target = root.join("cargo-target");
        let fingerprint_input = root.join("runner-fingerprint-input.json");
        fs::create_dir_all(&staging_temp).expect("create external staging temp");
        fs::create_dir_all(&cargo_target).expect("create external cargo target");
        fs::write(
            &fingerprint_input,
            serde_json::to_vec_pretty(&fixture_runner_fingerprint_fields()).unwrap(),
        )
        .expect("write runner fingerprint fixture");
        Self {
            app_argument: root.join("alias/../fake-rterm-app.exe"),
            launcher_argument: root.join("alias/../fake-font-launcher.cmd"),
            output_directory: root.join("evidence"),
            staging_temp,
            cargo_target,
            fingerprint_input,
            root,
        }
    }

    fn run(&self, warmups: u32, measured_rounds: u32) -> Output {
        self.run_with_timeout(warmups, measured_rounds, 60)
    }

    fn run_with_timeout(&self, warmups: u32, measured_rounds: u32, timeout_seconds: u32) -> Output {
        self.command(warmups, measured_rounds, timeout_seconds)
            .env("CARGO_TARGET_DIR", &self.cargo_target)
            .output()
            .expect("execute font proof runner")
    }

    fn run_with_cargo_target(
        &self,
        warmups: u32,
        measured_rounds: u32,
        target: Option<&Path>,
    ) -> Output {
        let mut command = self.command(warmups, measured_rounds, 60);
        if let Some(target) = target {
            command.env("CARGO_TARGET_DIR", target);
        } else {
            command.env_remove("CARGO_TARGET_DIR");
        }
        command
            .output()
            .expect("execute font proof target contract")
    }

    fn run_with_fingerprint_fault(&self, fault: &str) -> Output {
        self.command(0, 1, 60)
            .args(["-TestRunnerFingerprintFault", fault])
            .env("CARGO_TARGET_DIR", &self.cargo_target)
            .output()
            .expect("execute font proof fingerprint fault")
    }

    fn run_with_fingerprint_fault_and_timeout(&self, fault: &str, timeout_seconds: u32) -> Output {
        self.command(0, 1, timeout_seconds)
            .args(["-TestRunnerFingerprintFault", fault])
            .env("CARGO_TARGET_DIR", &self.cargo_target)
            .output()
            .expect("execute bounded font proof fingerprint fault")
    }

    fn run_certification_with_fixture_input(&self) -> Output {
        Command::new("pwsh.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-File",
                repo_path("scripts/ci/run-stage7-font-proof.ps1")
                    .to_str()
                    .expect("UTF-8 runner path"),
                "-Profile",
                "release",
                "-Warmups",
                "5",
                "-MeasuredRounds",
                "30",
                "-OutputDirectory",
                self.output_directory.to_str().expect("UTF-8 output path"),
                "-TestRunnerFingerprintInputPath",
                self.fingerprint_input
                    .to_str()
                    .expect("UTF-8 fingerprint fixture path"),
            ])
            .current_dir(repo_path("."))
            .env("TEMP", &self.staging_temp)
            .env("TMP", &self.staging_temp)
            .env("TMPDIR", &self.staging_temp)
            .env("CARGO_TARGET_DIR", &self.cargo_target)
            .output()
            .expect("execute certification fixture rejection")
    }

    fn command(&self, warmups: u32, measured_rounds: u32, timeout_seconds: u32) -> Command {
        let mut command = Command::new("pwsh.exe");
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-File",
                repo_path("scripts/ci/run-stage7-font-proof.ps1")
                    .to_str()
                    .expect("UTF-8 runner path"),
                "-Profile",
                "release",
                "-Warmups",
                &warmups.to_string(),
                "-MeasuredRounds",
                &measured_rounds.to_string(),
                "-ProcessTimeoutSeconds",
                &timeout_seconds.to_string(),
                "-OutputDirectory",
                self.output_directory.to_str().expect("UTF-8 output path"),
                "-SkipBuild",
                "-AppPath",
                self.app_argument.to_str().expect("UTF-8 app path"),
                "-LauncherPath",
                self.launcher_argument
                    .to_str()
                    .expect("UTF-8 launcher path"),
                "-TestRunnerFingerprintInputPath",
                self.fingerprint_input
                    .to_str()
                    .expect("UTF-8 fingerprint fixture path"),
            ])
            .current_dir(repo_path("."))
            .env("TEMP", &self.staging_temp)
            .env("TMP", &self.staging_temp)
            .env("TMPDIR", &self.staging_temp);
        command
    }

    fn read_json(&self, relative: &str) -> Value {
        let path = self.output_directory.join(relative);
        serde_json::from_slice(
            &fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
        )
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
    }
}

fn fixture_runner_fingerprint_fields() -> Value {
    serde_json::json!({
        "os": {
            "version": "10.0.26100",
            "build_number": "26100",
            "build_revision": 4946,
            "architecture": "x86_64"
        },
        "gpu": {
            "vendor_id": 4318,
            "device_id": 11524,
            "driver_version": "32.0.16.2002",
            "wddm_version": "WDDM 3.2"
        },
        "memory": {"physical_bytes": 68_719_476_736_u64, "pagefile_mode": "automatic-managed"},
        "displays": [
            {"width_px": 2560, "height_px": 1440, "dpi_x": 120, "dpi_y": 120, "primary": true},
            {"width_px": 1920, "height_px": 1080, "dpi_x": 96, "dpi_y": 96, "primary": false}
        ],
        "power_plan": {"guid": "381b4222-f694-41f0-9685-ff5bb260df2e"},
        "session": {"kind": "local"},
        "locale": {"culture": "en-US", "ui_culture": "en-US", "system_locale": "en-US"},
        "fonts": {"inventory_fingerprint_sha256": "8".repeat(64), "index_policy_version": 1},
        "cold_cache_policy": {
            "process_cold_start": true,
            "os_file_cache": "unmodified-no-explicit-flush"
        }
    })
}

fn runner_canonical_sha256(value: &Value) -> String {
    fn write(value: &Value, output: &mut String) {
        match value {
            Value::Null => panic!("runner canonical protocol forbids null"),
            Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Value::Number(value) if value.is_i64() || value.is_u64() => {
                output.push_str(&value.to_string());
            }
            Value::Number(_) => panic!("runner canonical protocol forbids non-integers"),
            Value::String(value) => {
                output.push_str(&serde_json::to_string(value).expect("serialize string"));
            }
            Value::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    write(value, output);
                }
                output.push(']');
            }
            Value::Object(values) => {
                output.push('{');
                let mut keys = values.keys().collect::<Vec<_>>();
                keys.sort_unstable();
                for (index, key) in keys.into_iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    output.push_str(&serde_json::to_string(key).expect("serialize key"));
                    output.push(':');
                    write(&values[key], output);
                }
                output.push('}');
            }
        }
    }

    let mut canonical = String::new();
    write(value, &mut canonical);
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(canonical.as_bytes()) {
        write!(&mut encoded, "{byte:02x}").expect("write digest hex");
    }
    encoded
}

fn serialized_font_proof_results() -> String {
    use rterm_diagnostics::{
        DiagnosticFontMode, DiagnosticFontSpecimen, DiagnosticGpuBackend, DiagnosticRendererMode,
        DiagnosticsResult, MemoryMetric, MemorySample, Platform, ProcessExitKind, RendererKind,
        RunConfiguration, RunIdentity, Scenario,
    };

    let mut results = std::collections::BTreeMap::new();
    for (mode_name, mode) in [
        ("current", DiagnosticFontMode::CurrentCopied),
        ("shared", DiagnosticFontMode::SharedAll),
        ("lazy", DiagnosticFontMode::Lazy),
    ] {
        for (specimen_name, specimen) in [
            ("ascii", DiagnosticFontSpecimen::Ascii),
            ("cjk", DiagnosticFontSpecimen::Cjk),
            ("emoji", DiagnosticFontSpecimen::Emoji),
        ] {
            let configuration = RunConfiguration {
                stabilization_ms: 5_000,
                sample_interval_ms: 100,
                sample_count: 10,
                columns: 80,
                rows: 24,
                scale_factor_milli: 1_000,
                requested_renderer: DiagnosticRendererMode::Auto,
                requested_gpu_backend: None,
                requested_font_mode: Some(mode),
                requested_font_specimen: Some(specimen),
                requested_attribution_stage: None,
            };
            let base: u64 = match mode {
                DiagnosticFontMode::CurrentCopied => 300 * 1024 * 1024,
                DiagnosticFontMode::SharedAll => 200 * 1024 * 1024,
                DiagnosticFontMode::Lazy => 150 * 1024 * 1024,
            };
            let mut result = DiagnosticsResult::successful_fixture(
                RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
                MemoryMetric::WindowsPrivateWorkingSetBytes,
                configuration,
            );
            result.run.id = format!("empty-window-1-1{mode_name}{specimen_name}");
            result.milestones.first_present_ms = Some(10);
            result.milestones.gpu_ready_ms = Some(100);
            result.milestones.font_ownership_ready_ms = Some(110);
            result.milestones.scenario_ready_ms = Some(120);
            result.renderer.first = Some(RendererKind::Cpu);
            result.renderer.final_renderer = Some(RendererKind::Gpu);
            result.renderer.backend = Some(DiagnosticGpuBackend::Dx12);
            result.renderer.adapter_name = Some("fixture-adapter".to_owned());
            result.renderer.adapter_vendor_id = Some(4318);
            result.renderer.adapter_device_id = Some(9860);
            result.renderer.adapter_type = Some("discrete-gpu".to_owned());
            result.memory.samples = (0..10)
                .map(|sequence| MemorySample {
                    sequence,
                    elapsed_ms: 5_000 + u64::from(sequence) * 100,
                    bytes: base + u64::from(sequence),
                })
                .collect();
            result.process.pid = 4242;
            result.process.exit_kind = ProcessExitKind::Requested;
            result.process.exit_code = Some(0);
            result.font_resources = Some(serialized_font_resource_summary(mode, specimen));
            results.insert(format!("{mode_name}/{specimen_name}"), result);
        }
    }
    serde_json::to_string(&results).expect("serialize real font proof diagnostics results")
}

fn serialized_font_resource_summary(
    mode: rterm_diagnostics::DiagnosticFontMode,
    specimen: rterm_diagnostics::DiagnosticFontSpecimen,
) -> rterm_diagnostics::DiagnosticFontResourceSummary {
    use rterm_diagnostics::{
        DiagnosticFontMode, DiagnosticFontResourceSummary, DiagnosticFontSpecimen,
    };

    let (active, initial, retained): (usize, usize, usize) = match mode {
        DiagnosticFontMode::CurrentCopied => (3, 2, 200 * 1024 * 1024),
        DiagnosticFontMode::SharedAll => (3, 3, 100 * 1024 * 1024),
        DiagnosticFontMode::Lazy if specimen == DiagnosticFontSpecimen::Ascii => {
            (1, 1, 50 * 1024 * 1024)
        }
        DiagnosticFontMode::Lazy => (2, 1, 50 * 1024 * 1024),
    };
    let generation = active - initial + 1;
    DiagnosticFontResourceSummary {
        mode,
        specimen,
        retained_source_bytes: retained,
        indexed_source_count: 3,
        active_source_count: active,
        initial_catalog_source_count: initial,
        catalog_builds: generation as u64,
        generation: generation as u64,
        recovery_retained_source_bytes: retained,
        recovery_generation: generation as u64,
        activation_latency_micros: 9,
        tofu_count: 0,
        frame_catalog_generation: Some(generation as u64),
        frame_generation_consistent: Some(true),
        index_fingerprint_sha256: "1".repeat(64),
        catalog_fingerprint_sha256: if mode == DiagnosticFontMode::Lazy {
            "4".repeat(64)
        } else {
            "2".repeat(64)
        },
        ordered_catalog_fingerprint_sha256: if mode == DiagnosticFontMode::Lazy {
            "5".repeat(64)
        } else {
            "3".repeat(64)
        },
        font_inventory_fingerprint_sha256: (mode != DiagnosticFontMode::Lazy)
            .then(|| "8".repeat(64)),
        font_index_policy_version: (mode != DiagnosticFontMode::Lazy).then_some(1),
    }
}

impl Drop for FontProofFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl MatrixFixture {
    fn new(label: &str, mode: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rterm-gpu-matrix-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create fixture root");
        fs::create_dir_all(root.join("alias")).expect("create alias path component");
        let app = root.join("fake-rterm-app.exe");
        fs::write(&app, b"fixture").expect("write fake app");
        let launcher_script = root.join("fake-launcher.ps1");
        let launcher = root.join("fake-launcher.cmd");
        let app_argument = root.join("alias/../fake-rterm-app.exe");
        let launcher_argument = root.join("alias/../fake-launcher.cmd");
        let fake_launcher = FAKE_LAUNCHER
            .replace("__FIXTURE_MODE__", mode)
            .replace(
                "__RESOLVED_LAUNCHER_PATH__",
                launcher.to_str().expect("UTF-8 launcher path"),
            )
            .replace(
                "__ORIGINAL_APP_PATH__",
                app_argument.to_str().expect("UTF-8 app argument"),
            )
            .replace(
                "__ORIGINAL_LAUNCHER_PATH__",
                launcher_argument.to_str().expect("UTF-8 launcher argument"),
            );
        fs::write(&launcher_script, fake_launcher).expect("write fake launcher script");
        fs::write(
            &launcher,
            b"@echo off\r\npwsh.exe -NoProfile -NonInteractive -File \"%~dp0fake-launcher.ps1\" %*\r\nexit /b %errorlevel%\r\n",
        )
        .expect("write fake launcher command");
        let output_directory = root.join("evidence");
        Self {
            root,
            app,
            launcher,
            app_argument,
            launcher_argument,
            output_directory,
        }
    }

    fn run(&self) -> Output {
        self.run_with_samples(1)
    }

    fn run_with_samples(&self, samples: u32) -> Output {
        Command::new("pwsh.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-File",
                repo_path("scripts/ci/run-gpu-backend-memory-matrix.ps1")
                    .to_str()
                    .expect("UTF-8 runner path"),
                "-Profile",
                "release",
                "-Warmups",
                "0",
                "-Samples",
                &samples.to_string(),
                "-OutputDirectory",
                self.output_directory.to_str().expect("UTF-8 output path"),
                "-SkipBuild",
                "-AppPath",
                self.app_argument.to_str().expect("UTF-8 app path"),
                "-LauncherPath",
                self.launcher_argument
                    .to_str()
                    .expect("UTF-8 launcher path"),
            ])
            .current_dir(repo_path("."))
            .output()
            .expect("execute matrix runner")
    }

    fn report(&self) -> Value {
        let path = self.output_directory.join("aggregate.json");
        let bytes =
            fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        serde_json::from_slice(&bytes).expect("parse aggregate JSON")
    }

    fn redacted_paths(&self) -> [&Path; 4] {
        [
            &self.app,
            &self.launcher,
            &self.app_argument,
            &self.launcher_argument,
        ]
    }
}

impl Drop for MatrixFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn repo_path(relative: impl AsRef<Path>) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

const FAKE_LAUNCHER: &str = r#"
$mode = "__FIXTURE_MODE__"
$options = @{}
for ($index = 0; $index -lt $args.Count; $index++) {
    if ($args[$index].StartsWith("--") -and $index + 1 -lt $args.Count -and -not $args[$index + 1].StartsWith("--")) {
        $options[$args[$index]] = $args[$index + 1]
        $index++
    }
}
$renderer = $options["--renderer"]
$backend = $options["--gpu-backend"]
$probe = if ($renderer -eq "cpu") { "cpu" } else { $backend }
$counterPath = Join-Path $PSScriptRoot "invocations-$probe.txt"
$invocation = if (Test-Path -LiteralPath $counterPath) { [int] (Get-Content -Raw $counterPath) + 1 } else { 1 }
Set-Content -LiteralPath $counterPath -Value $invocation
$configuration = [ordered]@{
    stabilization_ms = if ($mode -eq "string-config-dx12" -and $probe -eq "dx12") { "5000" } else { 5000 }
    sample_interval_ms = 100
    sample_count = 10
    columns = 80
    rows = 24
    scale_factor_milli = 1000
    requested_renderer = $renderer
}
if ($null -ne $backend) {
    $configuration["requested_gpu_backend"] = $backend
}
$schema = if ($mode -eq "uppercase-wire-a" -and $probe -eq "cpu") { "RTERM.DIAGNOSTICS/V2" } else { "rterm.diagnostics/v2" }
if ($mode -eq "uppercase-wire-a" -and $probe -eq "dx12") {
    $configuration["requested_renderer"] = "AUTO"
}
if ($mode -eq "uppercase-wire-a" -and $probe -eq "vulkan") {
    $configuration["requested_gpu_backend"] = "VULKAN"
}
$rendererSummary = if ($probe -eq "cpu") {
    [ordered]@{ first = "cpu"; final = "cpu" }
} else {
    $adapterName = if ($mode -eq "drift-vulkan-adapter" -and $probe -eq "vulkan" -and $invocation -gt 1) { "fixture-adapter-drifted" } else { "fixture-adapter" }
    if ($mode -eq "object-array-vulkan-adapter" -and $probe -eq "vulkan") {
        $adapterName = [ordered]@{ unexpected = "object" }
    }
    $adapterType = if ($mode -eq "object-array-vulkan-adapter" -and $probe -eq "vulkan") {
        @("discrete-gpu")
    } elseif ($mode -eq "invalid-adapter-types" -and $probe -eq "dx12") {
        "DISCRETE-GPU"
    } elseif ($mode -eq "invalid-adapter-types" -and $probe -eq "vulkan") {
        "mystery-gpu"
    } elseif ($probe -eq "gl") {
        "other"
    } else {
        "discrete-gpu"
    }
    [ordered]@{
        first = "cpu"
        final = if ($mode -eq "uppercase-wire-b" -and $probe -eq "dx12") { "GPU" } else { "gpu" }
        backend = if ($mode -eq "uppercase-wire-b" -and $probe -eq "vulkan") { "VULKAN" } else { $backend }
        adapter_name = $adapterName
        adapter_vendor_id = if ($mode -eq "numeric-strings-dx12" -and $probe -eq "dx12") { "4318" } else { 4318 }
        adapter_device_id = if ($backend -eq "gl") { 0 } else { 9860 }
        adapter_type = $adapterType
    }
}
$samples = @(
    for ($sequence = 0; $sequence -lt 10; $sequence++) {
        $bytes = if ($mode -eq "malformed-dx12-bytes" -and $probe -eq "dx12" -and $sequence -eq 4) { $null } elseif ($mode -eq "numeric-strings-dx12" -and $probe -eq "dx12") { [string] (1048576 + $sequence) } else { 1048576 + $sequence }
        $sampleSequence = if ($mode -eq "numeric-strings-dx12" -and $probe -eq "dx12") { [string] $sequence } else { $sequence }
        [ordered]@{ sequence = $sampleSequence; elapsed_ms = 5000 + (100 * $sequence); bytes = $bytes }
    }
)
$failed = ($mode -eq "fail-cpu-and-dx12" -and ($probe -eq "cpu" -or $probe -eq "dx12")) -or $mode -eq "path-failure"
$failureMessage = if ($mode -eq "path-failure") {
    "$($options['--app'])|__RESOLVED_LAUNCHER_PATH__|__ORIGINAL_APP_PATH__|__ORIGINAL_LAUNCHER_PATH__"
} else {
    "requested fixture failure"
}
$record = [ordered]@{
    schema = $schema
    configuration = $configuration
    readiness = [ordered]@{ status = if ($failed) { "failed" } else { "ready" } }
    renderer = $rendererSummary
    memory = [ordered]@{
        metric = if ($mode -eq "uppercase-wire-a" -and $probe -eq "gl") { "WINDOWS_PRIVATE_WORKING_SET_BYTES" } else { "windows_private_working_set_bytes" }
        unit = "bytes"
        samples = $samples
    }
    failures = if ($failed) { @([ordered]@{ code = "fixture_failure"; phase = "fixture"; message = $failureMessage }) } else { @() }
}
$record | ConvertTo-Json -Depth 20 -Compress
"#;

const FAKE_FONT_LAUNCHER: &str = r#"
$fixtureMode = "__FIXTURE_MODE__"
$templates = '__CONFIGURATIONS_JSON__' | ConvertFrom-Json
$options = @{}
for ($index = 0; $index -lt $args.Count; $index++) {
    if ($args[$index].StartsWith("--") -and $index + 1 -lt $args.Count -and -not $args[$index + 1].StartsWith("--")) {
        $options[$args[$index]] = $args[$index + 1]
        $index++
    }
}
$mode = $options["--font-mode"]
$specimen = $options["--font-specimen"]
$record = $templates."$mode/$specimen" | ConvertTo-Json -Depth 20 | ConvertFrom-Json
$record.run.id = "empty-window-$PID-$([DateTime]::UtcNow.Ticks)"
if ($fixtureMode -eq "timeout") {
    Start-Sleep -Seconds 5
}
if ($fixtureMode -eq "dirty-binary") {
    Add-Content -LiteralPath $options["--app"] -Value "dirty" -NoNewline
}
if ($fixtureMode -eq "mode-fallback" -and $mode -eq "current") {
    $record.font_resources.mode = "shared"
}
if ($fixtureMode -eq "specimen-fallback" -and $specimen -eq "cjk") {
    $record.font_resources.specimen = "emoji"
}
if ($fixtureMode -eq "mixed-backend" -and $mode -eq "shared") {
    $record.renderer.backend = "vulkan"
}
if ($fixtureMode -eq "threshold" -and $mode -eq "shared") {
    for ($sequence = 0; $sequence -lt 10; $sequence++) {
        $record.memory.samples[$sequence].bytes = [UInt64] (300MB - 64MB + 1 + $sequence)
    }
}
if ($fixtureMode -eq "current-counter-shape" -and $mode -eq "current") {
    $record.font_resources.initial_catalog_source_count = 0
}
if ($fixtureMode -eq "current-builds-too-large" -and $mode -eq "current") {
    $record.font_resources.catalog_builds = $record.font_resources.active_source_count + 1
}
if ($fixtureMode -eq "shared-counter-shape" -and $mode -eq "shared") {
    $record.font_resources.initial_catalog_source_count = 1
}
if ($fixtureMode -eq "lazy-ascii-counter-shape" -and $mode -eq "lazy" -and $specimen -eq "ascii") {
    $record.font_resources.active_source_count = 2
    $record.font_resources.catalog_builds = 2
    $record.font_resources.generation = 2
    $record.font_resources.recovery_generation = 2
    $record.font_resources.frame_catalog_generation = 2
}
if ($fixtureMode -eq "lazy-activation-counter-shape" -and $mode -eq "lazy" -and $specimen -eq "cjk") {
    $record.font_resources.active_source_count = 1
    $record.font_resources.catalog_builds = 1
    $record.font_resources.generation = 1
    $record.font_resources.recovery_generation = 1
    $record.font_resources.frame_catalog_generation = 1
}
if ($fixtureMode -eq "current-build-generation-mismatch" -and $mode -eq "current") {
    $record.font_resources.generation = $record.font_resources.catalog_builds + 1
}
if ($fixtureMode -eq "missing-counter") {
    $record.memory.samples = @()
}
if ($fixtureMode -eq "recovery-duplication" -and $specimen -eq "cjk") {
    $record.font_resources.recovery_retained_source_bytes++
}
if ($fixtureMode -eq "tofu" -and $specimen -eq "cjk") {
    $record.font_resources.tofu_count = 1
}
if ($fixtureMode -eq "mixed-generation" -and $specimen -eq "cjk") {
    $record.font_resources.frame_catalog_generation++
}
if ($fixtureMode -eq "owner-ready-order") {
    $record.milestones.font_ownership_ready_ms = 99
}
if ($fixtureMode -eq "uppercase-adapter") {
    $record.renderer.adapter_type = "Discrete-GPU"
}
if ($fixtureMode -eq "unknown-adapter") {
    $record.renderer.adapter_type = "hardware"
}
if ($fixtureMode -eq "missing-initial") {
    $record.font_resources.PSObject.Properties.Remove("initial_catalog_source_count")
}
if ($fixtureMode -eq "cross-catalog" -and $mode -eq "shared" -and $specimen -eq "ascii") {
    $record.font_resources.catalog_fingerprint_sha256 = "9999999999999999999999999999999999999999999999999999999999999999"
}
if ($fixtureMode -eq "cross-retained" -and $mode -eq "current" -and $specimen -eq "ascii") {
    $record.font_resources.retained_source_bytes++
    $record.font_resources.recovery_retained_source_bytes++
}
if ($fixtureMode -eq "raw-path") {
    $record.font_resources | Add-Member -NotePropertyName "font_path" -NotePropertyValue "C:/Windows/Fonts/fixture.ttf"
}
$record | ConvertTo-Json -Depth 20 -Compress
"#;
