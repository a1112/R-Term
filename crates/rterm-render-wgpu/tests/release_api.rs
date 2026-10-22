use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn release_renderer_excludes_identifier_limit_test_api_and_artifacts() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("renderer crate belongs to workspace");
    let probe = cargo_target_root(workspace)
        .join("tmp")
        .join("renderer-release-api");
    let source = probe.join("src");
    fs::create_dir_all(&source).expect("create release API probe");
    let renderer = toml_path(&workspace.join("crates/rterm-renderer"));
    let fonts = toml_path(&workspace.join("crates/rterm-fonts"));
    let glyphon = toml_path(&workspace.join("vendor/glyphon-0.12.0"));
    let gpu_allocator = toml_path(&workspace.join("vendor/gpu-allocator-0.28.0"));
    fs::write(
        probe.join("Cargo.toml"),
        format!(
            "[package]\nname = \"renderer-release-api-probe\"\nversion = \"0.0.0\"\n\
             edition = \"2024\"\n\n[workspace]\n\n[dependencies]\nrterm-renderer = {{ path = \"{renderer}\" }}\n\
             rterm-fonts = {{ package = \"rterm-fonts\", path = \"{fonts}\" }}\n\n[patch.crates-io]\n\
             glyphon = {{ path = \"{glyphon}\" }}\n\
             gpu-allocator = {{ path = \"{gpu_allocator}\" }}\n"
        ),
    )
    .expect("write release API probe manifest");
    fs::write(
        source.join("main.rs"),
        "use rterm_fonts::RasterCacheConfig;\n\
         use rterm_renderer::gpu::GpuTextConfig;\n\
         fn main() {\n\
             let _ = GpuTextConfig::new(1024, RasterCacheConfig::new(1024))\n\
                 .with_identifier_limit_for_tests(1);\n\
         }\n",
    )
    .expect("write release API probe source");
    for (mode, debug_assertions, rustflags) in [
        ("default", false, ""),
        ("debug-assertions", true, "-C debug-assertions=yes"),
    ] {
        verify_release_mode(&probe, mode, debug_assertions, rustflags);
    }
}

fn cargo_target_root(workspace: &Path) -> PathBuf {
    cargo_target_root_from(workspace, std::env::var_os("CARGO_TARGET_DIR"))
}

fn cargo_target_root_from(
    workspace: &Path,
    configured_target: Option<std::ffi::OsString>,
) -> PathBuf {
    configured_target.map_or_else(|| workspace.join("target"), PathBuf::from)
}

#[test]
fn release_api_probe_respects_the_configured_cargo_target_directory() {
    let workspace = Path::new("workspace");
    let external = PathBuf::from("external-target");
    assert_eq!(
        cargo_target_root_from(workspace, Some(external.clone().into_os_string())),
        external
    );
    assert_eq!(
        cargo_target_root_from(workspace, None),
        workspace.join("target")
    );
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn verify_release_mode(probe: &Path, mode: &str, debug_assertions: bool, rustflags: &str) {
    let target = probe.join("target");
    let mut command = Command::new(env!("CARGO"));
    command
        .args([
            "build",
            "--release",
            "--offline",
            "--message-format=json",
            "--manifest-path",
            probe.join("Cargo.toml").to_str().expect("UTF-8 probe path"),
            "--target-dir",
            target.to_str().expect("UTF-8 target path"),
            "--jobs",
            "2",
        ])
        .env("CARGO_INCREMENTAL", "0")
        .env(
            "CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS",
            debug_assertions.to_string(),
        )
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env("RUSTFLAGS", rustflags);
    let output = command.output().expect("run release API probe");
    assert!(
        !output.status.success(),
        "{mode} release renderer unexpectedly exposed with_identifier_limit_for_tests"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let messages = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .collect::<Vec<_>>();
    let diagnostics = format!("{stdout}\n{}", String::from_utf8_lossy(&output.stderr));
    assert!(
        messages.iter().any(|message| {
            message["reason"] == "compiler-message"
                && message["message"]["code"]["code"] == "E0599"
                && message["message"]["message"]
                    .as_str()
                    .is_some_and(|text| text.contains("with_identifier_limit_for_tests"))
        }),
        "{mode} release API probe failed for an unrelated reason:\n{diagnostics}"
    );

    let artifacts = messages
        .into_iter()
        .filter(|message| message["reason"] == "compiler-artifact")
        .filter(|message| message["target"]["name"] == "rterm_renderer")
        .flat_map(|message| message["filenames"].as_array().cloned().unwrap_or_default())
        .filter_map(|filename| filename.as_str().map(ToOwned::to_owned))
        .filter(|filename| {
            Path::new(filename)
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("rlib")
                        || extension.eq_ignore_ascii_case("rmeta")
                })
        })
        .collect::<Vec<_>>();
    assert!(
        !artifacts.is_empty(),
        "{mode} release probe emitted no current rterm-renderer rlib/rmeta artifacts"
    );
    for artifact in artifacts {
        let bytes = fs::read(&artifact).unwrap_or_else(|error| {
            panic!("{mode} release artifact {artifact} cannot be read: {error}")
        });
        for forbidden in [
            b"with_identifier_limit_for_tests".as_slice(),
            b"identifier_ceiling_for_unit_tests".as_slice(),
            b"configured identifier limit".as_slice(),
        ] {
            assert!(
                !bytes
                    .windows(forbidden.len())
                    .any(|window| window == forbidden),
                "{mode} release artifact {artifact} contains forbidden test seam {:?}",
                String::from_utf8_lossy(forbidden)
            );
        }
    }
}
