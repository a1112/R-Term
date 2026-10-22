use std::{fs, path::PathBuf};

const PACKAGING_FILES: [&str; 12] = [
    "packaging/Info.plist",
    "packaging/package-manifest.json",
    "packaging/rterm-console.cmd",
    "packaging/rterm-console.sh",
    "scripts/ci/package-native.ps1",
    "scripts/ci/package-native.sh",
    "scripts/ci/package-smoke.ps1",
    "scripts/ci/package-smoke.sh",
    "scripts/ci/run-native-window.ps1",
    "scripts/ci/run-native-window.sh",
    "docs/release-console.md",
    ".github/workflows/windows.yml",
];

#[test]
fn native_packaging_uses_the_rterm_runtime_identity() {
    let info_plist = read_repo_file("packaging/Info.plist");
    let manifest = read_repo_file("packaging/package-manifest.json");
    let unix_package = read_repo_file("scripts/ci/package-native.sh");
    let windows_package = read_repo_file("scripts/ci/package-native.ps1");
    let unix_smoke = read_repo_file("scripts/ci/package-smoke.sh");
    let windows_smoke = read_repo_file("scripts/ci/package-smoke.ps1");
    let unix_runner = read_repo_file("scripts/ci/run-native-window.sh");
    let windows_runner = read_repo_file("scripts/ci/run-native-window.ps1");

    for (path, contents) in [
        ("packaging/Info.plist", info_plist.as_str()),
        ("packaging/package-manifest.json", manifest.as_str()),
        ("scripts/ci/package-native.sh", unix_package.as_str()),
        ("scripts/ci/package-native.ps1", windows_package.as_str()),
        ("scripts/ci/package-smoke.sh", unix_smoke.as_str()),
        ("scripts/ci/package-smoke.ps1", windows_smoke.as_str()),
        ("scripts/ci/run-native-window.sh", unix_runner.as_str()),
        ("scripts/ci/run-native-window.ps1", windows_runner.as_str()),
    ] {
        for legacy in [
            "R-SSH",
            "rssh-app",
            "rssh-console",
            "rssh-profiles",
            "rssh-ssh",
            "RSSH_",
        ] {
            assert!(
                !contents.contains(legacy),
                "{path} still carries legacy packaging identity {legacy}"
            );
        }
    }

    for contract in ["R-Term", "<string>rterm</string>", "io.github.a1112.rterm"] {
        assert!(
            info_plist.contains(contract),
            "Info.plist is missing {contract}"
        );
    }
    assert!(manifest.contains("\"name\": \"R-Term\""));
    assert!(manifest.contains("\"__BINARY_PATH__\""));
}

#[test]
fn native_package_scripts_use_the_rterm_runtime_identity() {
    let unix_package = read_repo_file("scripts/ci/package-native.sh");
    let windows_package = read_repo_file("scripts/ci/package-native.ps1");
    let unix_smoke = read_repo_file("scripts/ci/package-smoke.sh");
    let windows_smoke = read_repo_file("scripts/ci/package-smoke.ps1");
    let unix_runner = read_repo_file("scripts/ci/run-native-window.sh");
    let windows_runner = read_repo_file("scripts/ci/run-native-window.ps1");

    for contract in [
        "binary_relative=rterm",
        "R-Term.app/Contents/MacOS/rterm",
        "rterm-console.sh",
        "examples/rterm-profiles.toml",
        "'name': 'R-Term'",
    ] {
        assert!(
            unix_package.contains(contract),
            "Unix package script is missing {contract}"
        );
    }
    for contract in [
        "Binary = \"rterm.exe\"",
        "rterm-console.cmd",
        "examples/rterm-profiles.toml",
        "name = \"R-Term\"",
        "binary = \"rterm.exe\"",
    ] {
        assert!(
            windows_package.contains(contract),
            "Windows package script is missing {contract}"
        );
    }

    for (name, contents) in [
        ("Unix smoke", unix_smoke.as_str()),
        ("Windows smoke", windows_smoke.as_str()),
    ] {
        for contract in [
            "manifest.json",
            "SHA256SUMS",
            "examples/rterm-profiles.toml",
            "RTERM_TEST_APP_EXECUTABLE",
            "RTERM_REQUIRE_OPENSSH",
            "rterm_app_system_openssh_entrypoint_runs_a_real_loopback_exec",
            "native_window_e2e_presents_ten_frames_from_a_real_pty",
            "rterm-app",
        ] {
            assert!(contents.contains(contract), "{name} is missing {contract}");
        }
    }
    assert!(unix_smoke.contains("-p rterm-app"));
    assert!(
        windows_smoke
            .replace(['"', ','], "")
            .contains("-p rterm-app")
    );

    for (name, contents) in [
        ("Unix native runner", unix_runner.as_str()),
        ("Windows native runner", windows_runner.as_str()),
    ] {
        for contract in [
            "rterm",
            "rterm-app",
            "rterm-ssh",
            "native_window_e2e",
            "RTERM_REQUIRE_OPENSSH",
        ] {
            assert!(contents.contains(contract), "{name} is missing {contract}");
        }
    }
}

#[test]
fn native_package_scripts_declare_the_supported_archive_contracts() {
    let unix_package = read_repo_file("scripts/ci/package-native.sh");
    let windows_package = read_repo_file("scripts/ci/package-native.ps1");

    for contract in [
        "\"$artifact_name\" != *.tar.gz",
        "*-unsigned.tar.gz",
        "package root must be absent or empty",
        "format': 'tar.gz'",
        "schema_version': 1",
    ] {
        assert!(
            unix_package.contains(contract),
            "Unix package script is missing {contract}"
        );
    }
    for contract in [
        "ArtifactName.EndsWith(\".zip\"",
        "EndsWith(\"-unsigned.zip\"",
        "package root must be absent or empty",
        "format = \"zip\"",
        "schema_version = 1",
    ] {
        assert!(
            windows_package.contains(contract),
            "Windows package script is missing {contract}"
        );
    }
}

#[test]
fn production_package_feature_graph_excludes_diagnostic_fonts() {
    let app_manifest = read_repo_file("crates/rterm-app/Cargo.toml");
    let fonts_manifest = read_repo_file("crates/rterm-fonts/Cargo.toml");
    let active_ci = read_repo_file(".github/workflows/windows.yml");

    assert!(app_manifest.contains("name = \"rterm-app\""));
    assert!(app_manifest.contains(
        "production-gui = [\"native-gui\", \"local-pty\", \"image-basic\", \"production-fonts\"]"
    ));
    assert!(app_manifest.contains("production-fonts = [\"rterm-fonts/shared-source-ownership\"]"));
    assert!(fonts_manifest.contains("shared-source-ownership = []"));
    assert!(fonts_manifest.contains("diagnostic-tools = []"));
    assert!(!active_ci.contains("--features production-gui,diagnostic-tools"));
    assert!(active_ci.contains(
        "cargo test --locked -p rterm-app --no-default-features --test feature_boundary"
    ));
}

#[test]
fn active_ci_validates_the_rterm_binary_and_owned_ssh_adapter() {
    let workflow = read_repo_file(".github/workflows/windows.yml");

    assert!(workflow.contains("name: R-Term independence"));
    assert!(workflow.contains(
        "cargo test --locked -p rterm-app --no-default-features --test feature_boundary"
    ));
    assert!(workflow.contains("cargo test --locked -p rterm-ssh --all-targets"));
    assert!(workflow.contains("cargo test --locked -p rterm-app --test native_window_e2e"));
    assert!(workflow.contains("cargo build --locked -j 2"));
    assert!(!workflow.contains("docs/trial/legacy-workflows"));
    assert!(!workflow.contains("rssh-ssh"));
}

#[test]
fn packaging_assets_exist_and_contain_no_legacy_product_identity() {
    for path in PACKAGING_FILES {
        let contents = read_repo_file(path);
        for legacy in [
            "R-SSH",
            "rssh-app",
            "rssh-console",
            "rssh-profiles",
            "RSSH_",
        ] {
            assert!(
                !contents.contains(legacy),
                "{path} still carries legacy product identity {legacy}"
            );
        }
    }
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
        .expect("workspace root above rterm-app")
        .to_owned()
}
