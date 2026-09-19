use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn pr_smoke_assembles_unsigned_packages_and_executes_the_unpacked_binary() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    for contract in [
        "package-native.ps1",
        "package-native.sh",
        "-Unsigned",
        "--unsigned",
        "dist/functional-package",
        "package.startup-smoke",
        "check-functional-observer-isolation.py",
        "--capability production_observer_isolation",
    ] {
        assert!(
            workflow.contains(contract),
            "missing package smoke {contract}"
        );
    }
}

#[test]
fn production_web_and_tauri_artifacts_have_black_box_smoke_jobs() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    let tauri_smoke =
        fs::read_to_string(root().join("scripts/functional/smoke-production-tauri.ps1")).unwrap();
    assert!(workflow.contains("production-web-smoke"));
    assert!(workflow.contains("production-tauri-bundle-smoke"));
    assert!(workflow.contains("npm --prefix web run build"));
    assert!(workflow.contains("cargo build --locked --release -p rssh-web"));
    assert!(
        workflow.contains("cd web && npx playwright test --config playwright.production.config.ts")
    );
    assert!(workflow.contains("Black-box production Web PTY interaction and cleanup"));
    assert!(workflow.contains("npm --prefix tauri run build"));
    assert!(workflow.contains("smoke-production-tauri.ps1"));
    assert!(workflow.contains("functional-production-tauri"));
    assert!(tauri_smoke.contains("Get-SessionDescendants"));
    assert!(tauri_smoke.contains("msedgewebview2.exe"));
    assert!(tauri_smoke.contains("owned_process_ids"));
}

#[test]
fn functional_local_socket_dependencies_use_an_explicitly_allowed_license() {
    let deny = fs::read_to_string(root().join("deny.toml")).unwrap();
    let allowed = deny
        .split("[licenses]")
        .nth(1)
        .expect("licenses policy")
        .split("exceptions =")
        .next()
        .expect("license allowlist");
    assert!(allowed.contains("\"0BSD\""));
}

#[test]
fn production_tauri_bundles_run_black_box_input_and_cleanup_on_every_pr_platform() {
    let workflow =
        fs::read_to_string(root().join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    let job = workflow
        .split("  production-tauri-bundle-smoke:")
        .nth(1)
        .expect("production Tauri job")
        .split("  aggregate-evidence:")
        .next()
        .expect("production Tauri job boundary");

    for target in ["windows-x86_64", "linux-x11", "macos-accessibility"] {
        assert!(
            job.contains(target),
            "missing production Tauri target {target}"
        );
    }
    assert!(job.contains("smoke-production-tauri.ps1"));
    assert!(job.contains("smoke-production-tauri.sh"));
    assert!(job.contains("RSSH_FUNCTIONAL_MACOS_CGEVENT_HELPER"));
    assert!(job.contains("scripts/functional/run-x11-seat.sh"));
    assert!(
        fs::read_to_string(root().join("scripts/functional/run-x11-seat.sh"))
            .unwrap()
            .contains("xvfb-run")
    );
    assert!(job.contains("rssh-accessibility"));
}
