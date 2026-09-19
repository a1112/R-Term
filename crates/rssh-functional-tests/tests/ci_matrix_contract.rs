use std::{collections::BTreeMap, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn pr_functional_workflow_has_the_fixed_required_matrix_and_hard_deadlines() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    for contract in [
        "contract-catalog",
        "cli-transport",
        "native-windows",
        "native-x11",
        "native-wayland",
        "native-macos-hosted",
        "native-macos-accessibility",
        "web-browser",
        "tauri-platform",
        "production-package-smoke",
        "aggregate-evidence",
        "chromium",
        "firefox",
        "webkit",
        "timeout-minutes: 18",
    ] {
        assert!(
            workflow.contains(contract),
            "missing CI contract {contract}"
        );
    }
    assert!(!workflow.contains("continue-on-error: true"));
    assert!(!workflow.contains("retry"));
    assert!(workflow.contains("if-no-files-found: error"));
    assert!(workflow.contains("$PSNativeCommandUseErrorActionPreference = $true"));
    let upload_sha = "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02";
    assert_eq!(
        workflow.matches("actions/upload-artifact@").count(),
        workflow.matches(upload_sha).count(),
        "every artifact upload must use the reviewed pinned action SHA"
    );
    assert_eq!(
        workflow.matches("actions/upload-artifact@").count(),
        workflow.matches("if: ${{ always() }}").count(),
        "every evidence upload must run after both success and failure"
    );
}

#[test]
fn cli_transport_matrix_executes_runner_owned_lpt_shards() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    assert!(
        workflow.contains("run-shard --count 2 --index"),
        "the PR CLI/transport matrix must execute stable runner-owned LPT shards"
    );
    assert!(workflow.contains("shard_index:"));
    assert!(workflow.contains("--surface console"));
    assert!(
        !workflow.contains("--scenario startup.version"),
        "hand-written scenario commands bypass LPT scheduling"
    );
}

#[test]
fn wayland_job_routes_xtest_through_a_nested_weston_x11_seat() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    assert!(workflow.contains("scripts/functional/run-wayland-seat.sh"));
    assert!(workflow.contains("RSSH_FUNCTIONAL_WESTON_BACKEND"));
    assert!(workflow.contains("weston"));
    assert!(workflow.contains("wl-clipboard"));
    assert!(workflow.contains("xvfb"));
}

#[test]
fn native_x11_job_runs_with_an_ewmh_window_manager() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    let job = workflow
        .split("  native-x11:")
        .nth(1)
        .expect("native X11 job")
        .split("  native-wayland:")
        .next()
        .expect("following native Wayland job");
    assert!(job.contains("scripts/functional/run-x11-seat.sh"));
    assert!(job.contains("libxkbcommon-x11-0"));
    assert!(
        fs::read_to_string(root().join("scripts/functional/run-x11-seat.sh"))
            .unwrap()
            .contains("openbox")
    );
}

#[test]
fn windows_input_jobs_use_powershell_core_for_bounded_helper_startup() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    for (job, next_job) in [
        ("  native-windows:", "  host-terminal-windows:"),
        ("  tauri-platform:", "  tauri-platform-macos:"),
    ] {
        let definition = workflow
            .split(job)
            .nth(1)
            .expect("Windows input job")
            .split(next_job)
            .next()
            .expect("following job");
        assert!(definition.contains("RSSH_FUNCTIONAL_POWERSHELL: pwsh.exe"));
        assert!(definition.contains("RSSH_FUNCTIONAL_WINDOWS_SENDINPUT_ASSEMBLY"));
        assert!(definition.contains("-CompileOnly"));
    }
}

#[test]
fn linux_graphics_jobs_install_a_software_adapter_and_use_private_x11_sessions() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    let x11 = fs::read_to_string(root().join("scripts/functional/run-x11-seat.sh")).unwrap();

    for package in ["mesa-vulkan-drivers", "libvulkan1", "libgl1-mesa-dri"] {
        assert!(
            workflow.contains(package),
            "missing software GPU package {package}"
        );
    }
    for contract in [
        "XDG_RUNTIME_DIR",
        "dbus-run-session",
        "xvfb-run",
        "openbox",
        "WEBKIT_DISABLE_DMABUF_RENDERER=1",
        "WEBKIT_DISABLE_COMPOSITING_MODE=1",
        "GDK_BACKEND=x11",
    ] {
        assert!(
            x11.contains(contract),
            "missing X11 session contract {contract}"
        );
    }
}

#[test]
fn contract_catalog_builds_runtime_entries_before_functional_contracts() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    let job = workflow
        .split("  contract-catalog:")
        .nth(1)
        .unwrap()
        .split("  cli-transport:")
        .next()
        .unwrap();
    let build = job
        .find("cargo build --locked -p rssh-app")
        .expect("contract runtime build");
    let contracts = job
        .find("cargo test --locked -p rssh-functional-tests")
        .expect("functional contracts");
    assert!(build < contracts);
}

#[test]
fn linux_tauri_x11_probes_disable_the_webkit_dmabuf_path() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    let x11 = fs::read_to_string(root().join("scripts/functional/run-x11-seat.sh")).unwrap();
    for (job, next_job) in [
        ("  tauri-platform:", "  tauri-platform-macos:"),
        (
            "  production-tauri-bundle-smoke:",
            "  production-tauri-bundle-smoke-macos:",
        ),
    ] {
        let definition = workflow
            .split(job)
            .nth(1)
            .expect("Tauri job")
            .split(next_job)
            .next()
            .expect("following Tauri job");
        assert!(definition.contains("scripts/functional/run-x11-seat.sh"));
    }
    assert!(x11.contains("WEBKIT_DISABLE_DMABUF_RENDERER=1"));
    assert!(x11.contains("WEBKIT_DISABLE_COMPOSITING_MODE=1"));
}

#[test]
fn xterm_host_smoke_installs_its_bitmap_font_dependency() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    let job = workflow
        .split("  host-terminal-linux:")
        .nth(1)
        .expect("Linux host terminal job")
        .split("  native-x11:")
        .next()
        .expect("following native X11 job");
    assert!(job.contains("xfonts-base"));
}

#[test]
fn coverage_builds_transport_runtime_outside_llvm_covs_isolated_target() {
    let workflow = fs::read_to_string(root().join("docs/trial/legacy-workflows/ci.yml")).unwrap();
    let job = workflow
        .split("  coverage:")
        .nth(1)
        .unwrap()
        .split("  web-quality:")
        .next()
        .unwrap();
    let build = job
        .find("cargo build --locked -p rssh-app")
        .expect("coverage transport runtime build");
    let coverage = job.find("cargo llvm-cov").expect("coverage command");
    assert!(build < coverage);
}

#[test]
fn functional_workflow_has_no_expression_inside_an_inline_yaml_map() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    assert!(
        !workflow
            .lines()
            .any(|line| line.contains("with: {") && line.contains("${{")),
        "GitHub expressions must use block mappings so the workflow parses as YAML"
    );
}

#[test]
fn privileged_self_hosted_jobs_run_only_on_manual_dispatch() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    for (job, next_job) in [
        ("  native-macos-accessibility:", "  web-browser:"),
        ("  tauri-platform-macos:", "  production-package-smoke:"),
        (
            "  production-tauri-bundle-smoke-macos:",
            "  aggregate-evidence:",
        ),
    ] {
        let start = workflow.find(job).expect("privileged job is present");
        let end = workflow[start..]
            .find(next_job)
            .map_or(workflow.len(), |offset| start + offset);
        let definition = &workflow[start..end];
        assert!(
            definition.contains("self-hosted")
                && definition.contains("if: github.event_name == 'workflow_dispatch'")
                && !definition.contains("github.event.pull_request"),
            "privileged job {job} must run only on explicit manual dispatch"
        );
    }
}

#[test]
fn pull_requests_never_wait_for_privileged_self_hosted_macos_jobs() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    for (job, next_job) in [
        ("  native-macos-accessibility:", "  web-browser:"),
        ("  tauri-platform-macos:", "  production-package-smoke:"),
        (
            "  production-tauri-bundle-smoke-macos:",
            "  aggregate-evidence:",
        ),
    ] {
        let start = workflow.find(job).expect("privileged job is present");
        let end = start
            + workflow[start..]
                .find(next_job)
                .expect("following job is present");
        let definition = &workflow[start..end];
        assert!(definition.contains("if: github.event_name == 'workflow_dispatch'"));
        assert!(!definition.contains("github.event.pull_request"));
    }

    let pr_aggregate = workflow
        .split("  aggregate-evidence-pr:")
        .nth(1)
        .expect("hosted PR aggregate must be present");
    assert!(pr_aggregate.contains("if: github.event_name == 'pull_request'"));
    assert!(pr_aggregate.contains("functional-tests/hosted-matrix.toml"));
    let definition = pr_aggregate.split("  aggregate-evidence:").next().unwrap();
    assert!(!definition.contains("native-macos-accessibility"));
    assert!(!definition.contains("tauri-platform-macos"));
    assert!(!definition.contains("production-tauri-bundle-smoke-macos"));

    let full_aggregate = workflow
        .split("  aggregate-evidence:")
        .nth(1)
        .expect("manual full aggregate must be present");
    assert!(full_aggregate.contains("if: github.event_name == 'workflow_dispatch'"));
    assert!(full_aggregate.contains("functional-tests/matrix.toml"));
}

#[test]
fn pull_requests_keep_hosted_tauri_rows_and_an_exact_hosted_aggregate() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    for (job, next_job) in [
        ("  tauri-platform:", "  tauri-platform-macos:"),
        (
            "  production-tauri-bundle-smoke:",
            "  production-tauri-bundle-smoke-macos:",
        ),
    ] {
        let start = workflow.find(job).expect("hosted Tauri job is present");
        let end = start
            + workflow[start..]
                .find(next_job)
                .expect("next job is present");
        let definition = &workflow[start..end];
        assert!(!definition.contains("self-hosted"));
        assert!(!definition.contains("github.event.pull_request.head.repo.full_name"));
        assert!(definition.contains("windows-2025"));
        assert!(definition.contains("ubuntu-24.04"));
    }
    assert!(workflow.contains("  aggregate-evidence-pr:"));
    assert!(workflow.contains("functional-tests/hosted-matrix.toml"));
    assert!(workflow.contains("if: github.event_name == 'pull_request'"));
}

#[test]
fn hosted_matrix_is_the_full_matrix_without_privileged_macos_targets() {
    fn scenario_targets(document: &toml::Value) -> BTreeMap<String, Vec<String>> {
        document["scenario_runs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|run| {
                let id = run["scenario_id"].as_str().unwrap().to_owned();
                let targets = run["targets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|target| target.as_str().unwrap().to_owned())
                    .collect();
                (id, targets)
            })
            .collect()
    }

    let full: toml::Value =
        toml::from_str(&fs::read_to_string(root().join("functional-tests/matrix.toml")).unwrap())
            .unwrap();
    let hosted: toml::Value = toml::from_str(
        &fs::read_to_string(root().join("functional-tests/hosted-matrix.toml")).unwrap(),
    )
    .unwrap();
    let full_targets = scenario_targets(&full);
    let hosted_targets = scenario_targets(&hosted);
    assert_eq!(
        full_targets.keys().collect::<Vec<_>>(),
        hosted_targets.keys().collect::<Vec<_>>()
    );
    for (scenario, targets) in &hosted_targets {
        assert!(
            targets
                .iter()
                .all(|target| { target != "macos-accessibility" && target != "macos-terminalapp" })
        );
        assert!(
            targets
                .iter()
                .all(|target| full_targets[scenario].contains(target))
        );
    }
    assert_eq!(full["playwright_runs"], hosted["playwright_runs"]);
}
