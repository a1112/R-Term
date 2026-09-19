use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn production_isolation_gate_checks_tree_markers_and_startup_probe() {
    let source =
        fs::read_to_string(root().join("scripts/ci/check-functional-observer-isolation.py"))
            .unwrap();
    for contract in [
        "cargo tree",
        "rssh-functional-tests",
        "rssh-functional-observer-v1",
        "__RSSH_FUNCTIONAL_SNAPSHOT__",
        "RSSH_FUNCTIONAL_OBSERVER_ENDPOINT",
        "version",
        "--json",
        "web-server",
        "gui",
        "process.terminate",
        "CreateFileW",
        "endpoint_path_hash",
        "\\\\.\\pipe\\rssh-functional-",
    ] {
        assert!(
            source.contains(contract),
            "missing isolation check {contract}"
        );
    }
    assert!(!source.contains("shell=True"));
}

#[test]
fn web_and_tauri_production_jobs_use_real_startup_probes_for_their_binary_shape() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    assert!(workflow.contains("--package rssh-web --startup-probe web-server"));
    assert!(workflow.contains("--package rssh-tauri --startup-probe gui"));
    assert!(!workflow.contains("npm --prefix tauri run build -- --no-bundle"));
    assert!(workflow.contains("target/release/bundle"));
}

#[test]
fn linux_tauri_isolation_probe_runs_inside_the_private_x11_seat() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    let job = workflow
        .split("  production-tauri-bundle-smoke:")
        .nth(1)
        .unwrap()
        .split("  production-tauri-bundle-smoke-macos:")
        .next()
        .unwrap();
    let linux_probe = job
        .split("      - name: Probe production Tauri executable on X11")
        .nth(1)
        .unwrap()
        .split("      - if: ${{ always() }}")
        .next()
        .unwrap();

    assert!(linux_probe.contains("check-functional-observer-isolation.py"));
    assert!(linux_probe.contains("--startup-probe gui"));
    assert_eq!(
        linux_probe
            .matches("bash scripts/functional/run-x11-seat.sh")
            .count(),
        2,
        "the smoke and isolation probe must each run in a private X11 seat"
    );
}

#[test]
fn ci_runs_production_isolation_for_prs_and_release_packages() {
    let ci = fs::read_to_string(root().join("docs/trial/legacy-workflows/ci.yml")).unwrap();
    let release =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/release.yml")).unwrap();
    assert!(ci.contains("check-functional-observer-isolation.py"));
    assert!(release.contains("check-functional-observer-isolation.py"));
}

#[test]
fn production_artifact_has_no_functional_observer() {
    let gate = fs::read_to_string(root().join("scripts/ci/check-functional-observer-isolation.py"))
        .unwrap();
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    for contract in [
        "cargo tree",
        "PROTOCOL_MARKERS",
        "observer_endpoint_exists",
        "production startup created a functional observer endpoint",
    ] {
        assert!(
            gate.contains(contract),
            "missing production probe {contract}"
        );
    }
    assert!(workflow.contains("check-functional-observer-isolation.py"));
}
