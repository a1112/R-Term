use std::{fs, path::PathBuf};

const ARTIFACTS: [(&str, &str, &str); 6] = [
    ("R-SSH-windows-x64.zip", "windows-x86_64", "windows-conpty"),
    (
        "R-SSH-windows-arm64.zip",
        "windows-aarch64",
        "windows-conpty",
    ),
    ("R-SSH-linux-x64.tar.gz", "linux-x86_64", "unix-pty"),
    ("R-SSH-linux-arm64.tar.gz", "linux-aarch64", "unix-pty"),
    ("R-SSH-macos-x64.tar.gz", "macos-x86_64", "unix-pty"),
    ("R-SSH-macos-arm64.tar.gz", "macos-aarch64", "unix-pty"),
];

#[test]
fn release_declares_six_stable_native_artifacts_and_runtime_identities() {
    let workflow = read_repo_file("docs/trial/legacy-workflows/release.yml");

    for (artifact, target, pty_backend) in ARTIFACTS {
        assert!(
            workflow.contains(artifact),
            "release workflow is missing stable artifact {artifact}"
        );
        assert!(
            workflow.contains(target),
            "release workflow is missing runtime target {target}"
        );
        assert!(
            workflow.contains(pty_backend),
            "release workflow is missing PTY backend {pty_backend}"
        );
    }
}

#[test]
fn package_smoke_and_machine_readable_manifests_are_mandatory() {
    let workflow = read_repo_file("docs/trial/legacy-workflows/release.yml");
    let info_plist = read_repo_file("packaging/Info.plist");

    for path in [
        "scripts/ci/package-smoke.ps1",
        "scripts/ci/package-smoke.sh",
        "packaging/package-manifest.json",
        "packaging/rssh-console.cmd",
        "packaging/rssh-console.sh",
        "packaging/Info.plist",
    ] {
        assert!(repo_root().join(path).is_file(), "missing {path}");
    }

    for contract in [
        "package-smoke.ps1",
        "package-smoke.sh",
        "manifest.json",
        "SHA256SUMS",
        "--harness-self-test",
        "RSSH_TEST_APP_EXECUTABLE",
        "RSSH_REQUIRE_OPENSSH",
        "native_window_e2e",
    ] {
        assert!(
            workflow.contains(contract),
            "release workflow is missing package contract {contract}"
        );
    }
    assert!(workflow.matches("dist/signed-package-smoke").count() >= 6);
    assert!(workflow.contains(
        "cargo build --locked --release -p rssh-app --no-default-features --features production-gui"
    ));
    assert!(info_plist.contains("LSArchitecturePriority"));
    assert!(info_plist.contains("__ARCHITECTURE__"));
    let unix_smoke = read_repo_file("scripts/ci/package-smoke.sh");
    let unix_package = read_repo_file("scripts/ci/package-native.sh");
    for contract in [
        "packaged macOS CLI launcher",
        "validate packaged macOS bundle identity",
        "CFBundleExecutable",
        "CFBundleShortVersionString",
        "LSArchitecturePriority",
    ] {
        assert!(
            unix_smoke.contains(contract),
            "missing macOS smoke {contract}"
        );
    }
    assert!(unix_package.contains("'rssh-app', 'R-SSH.app/Contents/Info.plist'"));
}

#[test]
fn diagnostic_font_mode_is_excluded_from_the_production_package_feature_graph() {
    let app_manifest = read_repo_file("crates/rssh-app/Cargo.toml");
    let fonts_manifest = read_repo_file("crates/rterm-fonts/Cargo.toml");
    let release = read_repo_file("docs/trial/legacy-workflows/release.yml");
    let production_gui = app_manifest
        .lines()
        .find(|line| line.starts_with("production-gui ="))
        .expect("production GUI feature");
    let production_fonts = app_manifest
        .lines()
        .find(|line| line.starts_with("production-fonts ="))
        .expect("production font feature");

    assert!(production_gui.contains("production-fonts"));
    assert!(!production_gui.contains("diagnostic-tools"));
    assert_eq!(
        production_fonts,
        "production-fonts = [\"rssh-fonts/shared-source-ownership\"]"
    );
    assert!(fonts_manifest.contains("shared-source-ownership = []"));
    assert!(fonts_manifest.contains("diagnostic-tools = []"));
    assert!(release.contains(
        "cargo build --locked --release -p rssh-app --no-default-features --features production-gui"
    ));
    assert!(!release.contains("--features production-gui,diagnostic-tools"));
}

#[test]
fn linux_release_jobs_guard_openssh_server_installation() {
    let workflow = read_repo_file("docs/trial/legacy-workflows/release.yml");
    let ci = read_repo_file("docs/trial/legacy-workflows/ci.yml");
    let build = job_section(&workflow, "build-package", "sign-windows");
    let sign_linux = job_section(&workflow, "sign-linux", "sign-macos");
    let native_e2e = ci
        .split("  native-terminal-e2e:\n")
        .nth(1)
        .expect("CI native-terminal-e2e job");

    let build_install = named_step(build, "Install Linux package-smoke dependencies");
    let signed_install = named_step(sign_linux, "signed-package-smoke Linux");
    let ci_install = named_step(native_e2e, "Install Linux native E2E dependencies");
    assert_linux_openssh_install_guard(build_install, "build-package");
    assert_linux_openssh_install_guard(signed_install, "sign-linux");
    assert_eq!(
        openssh_guard_prefix(build_install),
        openssh_guard_prefix(signed_install),
        "release OpenSSH service guards must not drift between jobs"
    );
    assert_eq!(
        openssh_guard_prefix(build_install),
        openssh_guard_prefix(ci_install),
        "release and CI OpenSSH service guards must not drift"
    );
}

#[test]
fn linux_release_smoke_installs_the_xkb_x11_runtime() {
    let workflow = read_repo_file("docs/trial/legacy-workflows/release.yml");
    let ci = read_repo_file("docs/trial/legacy-workflows/ci.yml");
    let build = job_section(&workflow, "build-package", "sign-windows");
    let sign_linux = job_section(&workflow, "sign-linux", "sign-macos");
    let native_e2e = ci
        .split("  native-terminal-e2e:\n")
        .nth(1)
        .expect("CI native-terminal-e2e job");

    for (scope, install) in [
        (
            "build-package",
            named_step(build, "Install Linux package-smoke dependencies"),
        ),
        (
            "sign-linux",
            named_step(sign_linux, "signed-package-smoke Linux"),
        ),
        (
            "native-terminal-e2e",
            named_step(native_e2e, "Install Linux native E2E dependencies"),
        ),
    ] {
        assert!(
            install.contains("libxkbcommon-x11-0"),
            "{scope} must install the XKB X11 runtime used by the packaged GUI smoke"
        );
    }
}

#[test]
fn linux_release_guard_contract_does_not_accept_a_different_job() {
    let workflow = r"
jobs:
  build-package:
    steps:
      - name: Install Linux package-smoke dependencies
        run: apt-get install --yes openssh-server
  decoy:
    steps:
      - name: guarded elsewhere
        run: policy-rc.d backup_path restore_policy_rc exit 101
";
    let build = job_section(workflow, "build-package", "decoy");
    let install = named_step(build, "Install Linux package-smoke dependencies");

    assert!(missing_linux_openssh_guard(install).is_some());
}

#[test]
fn tag_publication_requires_protected_signing_smoke_and_attestation() {
    let workflow = read_repo_file("docs/trial/legacy-workflows/release.yml");

    for contract in [
        "release-windows-signing",
        "release-linux-signing",
        "release-macos-signing",
        "signtool",
        "cosign",
        "notarytool",
        "stapler",
        "sbom",
        "attest-build-provenance",
        "attestations: write",
        "id-token: write",
        "signed-package-smoke",
    ] {
        assert!(
            workflow.contains(contract),
            "release workflow is missing protected release contract {contract}"
        );
    }

    assert!(workflow.contains("needs: attest-release"));
    assert!(workflow.contains("permissions:\n  contents: read"));
    assert_action_refs_are_pinned(&workflow);
}

#[test]
fn build_matrix_maps_each_native_runner_without_secrets_or_write_permissions() {
    let workflow = read_repo_file("docs/trial/legacy-workflows/release.yml");
    let build = job_section(&workflow, "build-package", "sign-windows");
    for mapping in [
        (
            "windows-x64",
            "windows-2025",
            "x86_64-pc-windows-msvc",
            "windows-x86_64",
            "windows-conpty",
            "R-SSH-windows-x64-unsigned",
            "R-SSH-windows-x64.zip",
            "R-SSH-windows-x64-unsigned.zip",
            "target/release/rssh-app.exe",
        ),
        (
            "windows-arm64",
            "windows-11-arm",
            "aarch64-pc-windows-msvc",
            "windows-aarch64",
            "windows-conpty",
            "R-SSH-windows-arm64-unsigned",
            "R-SSH-windows-arm64.zip",
            "R-SSH-windows-arm64-unsigned.zip",
            "target/release/rssh-app.exe",
        ),
        (
            "linux-x64",
            "ubuntu-24.04",
            "x86_64-unknown-linux-gnu",
            "linux-x86_64",
            "unix-pty",
            "R-SSH-linux-x64-unsigned",
            "R-SSH-linux-x64.tar.gz",
            "R-SSH-linux-x64-unsigned.tar.gz",
            "target/release/rssh-app",
        ),
        (
            "linux-arm64",
            "ubuntu-24.04-arm",
            "aarch64-unknown-linux-gnu",
            "linux-aarch64",
            "unix-pty",
            "R-SSH-linux-arm64-unsigned",
            "R-SSH-linux-arm64.tar.gz",
            "R-SSH-linux-arm64-unsigned.tar.gz",
            "target/release/rssh-app",
        ),
        (
            "macos-x64",
            "macos-15-intel",
            "x86_64-apple-darwin",
            "macos-x86_64",
            "unix-pty",
            "R-SSH-macos-x64-unsigned",
            "R-SSH-macos-x64.tar.gz",
            "R-SSH-macos-x64-unsigned.tar.gz",
            "target/release/rssh-app",
        ),
        (
            "macos-arm64",
            "macos-15",
            "aarch64-apple-darwin",
            "macos-aarch64",
            "unix-pty",
            "R-SSH-macos-arm64-unsigned",
            "R-SSH-macos-arm64.tar.gz",
            "R-SSH-macos-arm64-unsigned.tar.gz",
            "target/release/rssh-app",
        ),
    ] {
        let contract = format!(
            "- slug: {}\n            runner: {}\n            rust_target: {}\n            runtime_target: {}\n            pty_backend: {}\n            package_root: {}\n            artifact: {}\n            unsigned_artifact: {}\n            binary: {}",
            mapping.0,
            mapping.1,
            mapping.2,
            mapping.3,
            mapping.4,
            mapping.5,
            mapping.6,
            mapping.7,
            mapping.8,
        );
        assert!(
            build.contains(&contract),
            "missing runner mapping {mapping:?}"
        );
    }
    assert_eq!(build.matches("- slug:").count(), 6);
    assert!(build.contains("permissions:\n      contents: read"));
    assert!(!build.contains("contents: write"));
    assert!(!build.contains("${{ secrets."));
    assert!(build.contains("persist-credentials: false"));
    for command in [
        "cargo test --locked --workspace --all-targets --no-run",
        "cargo clippy --locked --workspace --all-targets -- -D warnings",
        "cargo build --locked --release -p rssh-app --no-default-features --features production-gui",
    ] {
        assert!(build.contains(command), "build job is missing {command}");
    }
}

#[test]
fn release_package_matrix_compiles_tests_before_platform_runtime_smoke() {
    let workflow = read_repo_file("docs/trial/legacy-workflows/release.yml");
    let build = job_section(&workflow, "build-package", "sign-windows");

    assert!(build.contains("name: Compile all workspace test targets"));
    assert!(build.contains("cargo test --locked --workspace --all-targets --no-run"));
    assert!(!build.contains("name: Test all workspace targets"));
    assert!(build.contains("native_window_e2e; --harness-self-test"));
    assert!(build.contains("package-native.ps1"));
    assert!(build.contains("package-native.sh"));
}

#[test]
fn protected_jobs_are_scoped_and_publication_has_a_complete_dag() {
    let workflow = read_repo_file("docs/trial/legacy-workflows/release.yml");
    let windows = job_section(&workflow, "sign-windows", "sign-linux");
    let linux = job_section(&workflow, "sign-linux", "sign-macos");
    let macos = job_section(&workflow, "sign-macos", "attest-release");
    let attest = job_section(&workflow, "attest-release", "publish-release");
    let publish = workflow
        .split("  publish-release:\n")
        .nth(1)
        .expect("publish-release job");

    for (section, environment, signed_smoke) in [
        (
            windows,
            "release-windows-signing",
            "signed-package-smoke Windows",
        ),
        (linux, "release-linux-signing", "signed-package-smoke Linux"),
        (macos, "release-macos-signing", "signed-package-smoke macOS"),
    ] {
        assert!(section.contains("if: startsWith(github.ref, 'refs/tags/v')"));
        assert!(section.contains(&format!("environment: {environment}")));
        assert!(section.contains("needs: build-package"));
        assert!(section.contains(signed_smoke));
        assert!(section.contains("dist/signed-package-smoke"));
    }
    assert!(windows.contains("${{ secrets.WINDOWS_SIGNING_CERTIFICATE_BASE64 }}"));
    assert!(windows.contains("$archivedBinary = (Resolve-Path"));
    assert!(windows.contains("signtool verify /pa /all /v $archivedBinary"));
    assert!(windows.contains("$archivedSignature = Get-AuthenticodeSignature"));
    assert!(linux.contains("id-token: write"));
    assert!(macos.contains("${{ secrets.MACOS_NOTARY_PRIVATE_KEY_BASE64 }}"));
    assert!(macos.contains("archived_app=\"dist/signed-package-smoke/"));
    assert!(macos.contains("codesign --verify --deep --strict --verbose=2 \"$archived_app\""));
    assert!(macos.contains("xcrun stapler validate \"$archived_app\""));
    assert!(attest.contains("needs: [sign-windows, sign-linux, sign-macos]"));
    assert!(attest.contains("environment: release-provenance"));
    assert!(attest.contains("attestations: write"));
    assert!(attest.contains("id-token: write"));
    assert!(publish.contains("needs: attest-release"));
    assert!(attest.contains("if: startsWith(github.ref, 'refs/tags/v')"));
    assert!(publish.contains("if: startsWith(github.ref, 'refs/tags/v')"));
    assert!(publish.contains("environment: release-publish"));
    assert!(publish.contains("contents: write"));
    assert!(!publish.contains("${{ secrets."));
    for artifact in ARTIFACTS.map(|entry| entry.0) {
        assert!(publish.contains(artifact));
    }
    assert_all_sign_matrices(windows, linux, macos);
}

fn assert_all_sign_matrices(windows: &str, linux: &str, macos: &str) {
    assert_sign_matrix(
        windows,
        [
            (
                "windows-x64",
                "windows-2025",
                "windows-x86_64",
                "R-SSH-windows-x64",
                "R-SSH-windows-x64.zip",
            ),
            (
                "windows-arm64",
                "windows-11-arm",
                "windows-aarch64",
                "R-SSH-windows-arm64",
                "R-SSH-windows-arm64.zip",
            ),
        ],
    );
    assert_sign_matrix(
        linux,
        [
            (
                "linux-x64",
                "ubuntu-24.04",
                "linux-x86_64",
                "R-SSH-linux-x64",
                "R-SSH-linux-x64.tar.gz",
            ),
            (
                "linux-arm64",
                "ubuntu-24.04-arm",
                "linux-aarch64",
                "R-SSH-linux-arm64",
                "R-SSH-linux-arm64.tar.gz",
            ),
        ],
    );
    assert_sign_matrix(
        macos,
        [
            (
                "macos-x64",
                "macos-15-intel",
                "macos-x86_64",
                "R-SSH-macos-x64",
                "R-SSH-macos-x64.tar.gz",
            ),
            (
                "macos-arm64",
                "macos-15",
                "macos-aarch64",
                "R-SSH-macos-arm64",
                "R-SSH-macos-arm64.tar.gz",
            ),
        ],
    );
}

fn assert_sign_matrix(section: &str, entries: [(&str, &str, &str, &str, &str); 2]) {
    for (slug, runner, runtime, root, artifact) in entries {
        let pty = if runtime.starts_with("windows-") {
            "windows-conpty"
        } else {
            "unix-pty"
        };
        let contract = format!(
            "- slug: {slug}\n            runner: {runner}\n            runtime_target: {runtime}\n            pty_backend: {pty}\n            package_root: {root}\n            artifact: {artifact}"
        );
        assert!(
            section.contains(&contract),
            "miswired signed artifact {slug}"
        );
    }
}

fn assert_linux_openssh_install_guard(step: &str, job: &str) {
    if let Some(missing) = missing_linux_openssh_guard(step) {
        panic!("{job} Linux OpenSSH install is missing {missing}");
    }

    let mut previous = None;
    for marker in [
        "sudo cp -a \"$policy_path\" \"$backup_path\"",
        "trap restore_policy_rc EXIT",
        "sudo rm -f -- \"$policy_path\"",
        "sudo tee \"$policy_path\"",
        "apt-get install --yes",
    ] {
        let position = step
            .find(marker)
            .unwrap_or_else(|| panic!("{job} Linux OpenSSH install is missing {marker}"));
        if let Some(previous) = previous {
            assert!(
                previous < position,
                "{job} Linux OpenSSH install orders {marker} before its prerequisite"
            );
        }
        previous = Some(position);
    }
}

fn missing_linux_openssh_guard(step: &str) -> Option<&'static str> {
    [
        "set -euo pipefail",
        "policy-rc.d",
        "backup_path",
        "restore_policy_rc",
        "exit 101",
        "sudo cp -a \"$backup_path\" \"$policy_path\"",
        "sudo chmod 0755 \"$policy_path\"",
        "openssh-client",
        "openssh-server",
    ]
    .into_iter()
    .find(|contract| !step.contains(contract))
}

fn openssh_guard_prefix(step: &str) -> &str {
    let start = step
        .find("set -euo pipefail")
        .expect("Linux OpenSSH guard start");
    let update = "sudo apt-get update";
    let end = step
        .find(update)
        .map(|offset| offset + update.len())
        .expect("Linux OpenSSH guard apt update");
    &step[start..end]
}

fn named_step<'a>(job: &'a str, name: &str) -> &'a str {
    let marker = format!("      - name: {name}\n");
    let start = job
        .find(&marker)
        .unwrap_or_else(|| panic!("missing step {name}"));
    let after_header = start + marker.len();
    let end = job[after_header..]
        .find("\n      - ")
        .map_or(job.len(), |offset| after_header + offset);
    &job[start..end]
}

fn read_repo_file(path: &str) -> String {
    let path = repo_root().join(path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

fn assert_action_refs_are_pinned(workflow: &str) {
    for action in workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- uses: "))
    {
        let revision = action
            .split_once('@')
            .unwrap_or_else(|| panic!("action has no revision: {action}"))
            .1
            .split_whitespace()
            .next()
            .expect("action revision after @");
        assert_eq!(
            revision.len(),
            40,
            "action is not pinned to a full SHA: {action}"
        );
        assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
}

fn job_section<'a>(workflow: &'a str, job: &str, next_job: &str) -> &'a str {
    workflow
        .split(&format!("  {job}:\n"))
        .nth(1)
        .unwrap_or_else(|| panic!("missing job {job}"))
        .split(&format!("\n  {next_job}:\n"))
        .next()
        .expect("job section before next job")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root above rssh-app")
        .to_owned()
}
