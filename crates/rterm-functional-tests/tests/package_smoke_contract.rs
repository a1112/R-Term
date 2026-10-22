use std::{fs, path::PathBuf};

#[test]
fn native_package_smoke_uses_the_current_rterm_scripts() {
    let unix_package = read_repo_file("scripts/ci/package-native.sh");
    let windows_package = read_repo_file("scripts/ci/package-native.ps1");
    let unix_smoke = read_repo_file("scripts/ci/package-smoke.sh");
    let windows_smoke = read_repo_file("scripts/ci/package-smoke.ps1");

    for (name, contents) in [
        ("Unix package", unix_package.as_str()),
        ("Windows package", windows_package.as_str()),
        ("Unix smoke", unix_smoke.as_str()),
        ("Windows smoke", windows_smoke.as_str()),
    ] {
        for legacy in [
            "R-SSH",
            "rssh-app",
            "rssh-console",
            "rssh-profiles",
            "RSSH_",
        ] {
            assert!(
                !contents.contains(legacy),
                "{name} still contains legacy product identity {legacy}"
            );
        }
    }

    for contract in [
        "rterm",
        "R-Term",
        "manifest.json",
        "SHA256SUMS",
        "examples/rterm-profiles.toml",
        "rterm-console",
        "RTERM_TEST_APP_EXECUTABLE",
        "RTERM_REQUIRE_OPENSSH",
        "rterm_app_system_openssh_entrypoint_runs_a_real_loopback_exec",
        "native_window_e2e_presents_ten_frames_from_a_real_pty",
    ] {
        let found = [
            unix_package.as_str(),
            windows_package.as_str(),
            unix_smoke.as_str(),
            windows_smoke.as_str(),
        ]
        .iter()
        .any(|contents| contents.contains(contract));
        assert!(found, "native package smoke scripts omit {contract}");
    }
}

#[test]
fn active_ci_runs_the_rterm_binary_and_its_owned_ssh_adapter() {
    let workflow = read_repo_file(".github/workflows/windows.yml");

    for contract in [
        "name: R-Term independence",
        "cargo test --locked -p rterm-app --features developer-full --bin rterm",
        "cargo test --locked -p rterm-ssh --all-targets",
        "cargo test --locked -p rterm-app --test native_window_e2e",
        "cargo build --locked -j 2",
    ] {
        assert!(
            workflow.contains(contract),
            "active CI is missing {contract}"
        );
    }
    assert!(!workflow.contains("docs/trial/legacy-workflows"));
    assert!(!workflow.contains("rssh-ssh"));
}

#[test]
fn windows_input_reliability_tracks_the_rterm_bin_artifact() {
    let app_manifest = read_repo_file("crates/rterm-app/Cargo.toml");
    assert!(app_manifest.contains("[[bin]]\nname = \"rterm\""));

    let matrix: serde_json::Value =
        serde_json::from_str(&read_repo_file("scripts/ci/windows-input-reliability.json"))
            .expect("Windows reliability matrix JSON");
    let cases = matrix["cases"].as_array().expect("reliability cases");
    let targets = cases
        .iter()
        .map(|case| case["target"].as_str().expect("case target"))
        .collect::<Vec<_>>();
    assert!(targets.contains(&"rterm"));
    assert!(
        targets
            .iter()
            .all(|target| matches!(*target, "rterm" | "local_pty" | "openssh_loopback"))
    );

    let runner = read_repo_file("scripts/ci/run-windows-input-reliability.ps1");
    assert!(runner.contains("\"--features\", \"developer-full\", \"--bin\", \"rterm\""));
    assert!(runner.contains("$products.ContainsKey(\"rterm\")"));
    assert!(!runner.contains("$products.ContainsKey(\"rterm-app\")"));
}

#[test]
fn active_ci_installs_the_runtime_dependencies_for_native_smoke() {
    let workflow = read_repo_file(".github/workflows/windows.yml");

    for dependency in [
        "libxkbcommon-x11-0",
        "libvulkan1",
        "mesa-vulkan-drivers",
        "xvfb",
        "xauth",
        "openssh-client",
    ] {
        assert!(
            workflow.contains(dependency),
            "active CI is missing {dependency}"
        );
    }
    assert!(workflow.contains("xvfb-run -a env -u WAYLAND_DISPLAY"));
}

#[test]
fn production_web_ci_build_remains_part_of_the_active_boundary() {
    let workflow = read_repo_file(".github/workflows/windows.yml");

    for contract in [
        "  web:",
        "npm --prefix web ci",
        "npm --prefix web run lint",
        "npm --prefix web test",
        "npm --prefix web run build",
    ] {
        assert!(
            workflow.contains(contract),
            "active CI is missing {contract}"
        );
    }
    assert!(!workflow.contains("docs/trial/legacy-workflows/functional.yml"));
}

#[test]
fn functional_local_socket_dependencies_use_an_explicitly_allowed_license() {
    let deny = read_repo_file("deny.toml");
    let allowed = deny
        .split("[licenses]")
        .nth(1)
        .expect("licenses policy")
        .split("exceptions =")
        .next()
        .expect("license allowlist");
    assert!(allowed.contains("\"0BSD\""));
}

fn read_repo_file(path: &str) -> String {
    let path = repo_root().join(path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root above rterm-functional-tests")
        .to_owned()
}
