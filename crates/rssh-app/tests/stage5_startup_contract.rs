use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use rssh_fonts::{FontCatalog, FontSource};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn read_repository_file(path: &str) -> String {
    fs::read_to_string(repository_root().join(path)).expect("read repository contract file")
}

#[test]
fn cpu_renderer_separates_basic_gif_and_legacy_image_decoders() {
    let manifest = read_repository_file("crates/rterm-render-cpu/Cargo.toml");

    assert!(manifest.contains("default = [\"image-basic\", \"image-gif\", \"image-legacy\"]"));
    assert!(manifest.contains("image-basic = [\"image/png\", \"image/jpeg\"]"));
    assert!(manifest.contains("image-gif = [\"image/gif\"]"));
    assert!(manifest.contains(
        "image-legacy = [\"image/dds\", \"image/ff\", \"image/ico\", \"image/pnm\", \"image/tga\", \"image/tiff\"]"
    ));
    assert!(manifest.contains("default-features = false"));
}

#[test]
fn packaged_gui_feature_excludes_diagnostics_transfers_and_optional_images() {
    let manifest = read_repository_file("crates/rssh-app/Cargo.toml");

    #[cfg(not(feature = "rterm-legacy-0-1"))]
    assert!(
        manifest.contains(
            "production-gui = [\"native-gui\", \"ssh\", \"local-pty\", \"image-basic\", \"production-fonts\"]"
        )
    );
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    assert!(manifest.contains("diagnostic-tools = [\"rssh-fonts/diagnostic-tools\"]"));
    #[cfg(feature = "rterm-legacy-0-1")]
    {
        assert!(manifest.contains(
            "production-gui = [\"native-gui\", \"ssh\", \"local-pty\", \"image-basic\", \"production-fonts\", \"rterm-legacy-0-1\"]"
        ));
        assert!(manifest.contains("diagnostic-tools = []"));
    }
    assert!(manifest.contains("transfer-tools = []"));
    assert!(
        !manifest
            .lines()
            .find(|line| line.starts_with("production-gui ="))
            .expect("production GUI feature")
            .contains("diagnostic-tools")
    );
    assert!(
        !manifest
            .lines()
            .find(|line| line.starts_with("production-gui ="))
            .expect("production GUI feature")
            .contains("transfer-tools")
    );
}

#[test]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn production_fonts_select_the_shared_lazy_feature_without_diagnostics() {
    let app_manifest = read_repository_file("crates/rssh-app/Cargo.toml");
    let fonts_manifest = read_repository_file("crates/rterm-fonts/Cargo.toml");
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
}

#[test]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn production_fonts_share_the_normal_catalog_source_allocation() {
    const LATIN: &[u8] = include_bytes!("../../../tests/fixtures/fonts/NotoSans-Latin.fixture.ttf");
    let source = FontSource::new("production-latin", LATIN.to_vec());
    let catalog = FontCatalog::from_sources("en-US", [source]).expect("production catalog");

    assert_eq!(catalog.memory_metrics().retained_source_bytes, LATIN.len());
}

// Frozen font catalogs support real loading, but cannot certify shared allocation.
#[test]
#[cfg(feature = "rterm-legacy-0-1")]
fn legacy_production_fonts_load_without_claiming_shared_allocation() {
    const LATIN: &[u8] = include_bytes!("../../../tests/fixtures/fonts/NotoSans-Latin.fixture.ttf");
    let app_manifest = read_repository_file("crates/rssh-app/Cargo.toml");
    let fonts_manifest = read_repository_file("crates/rterm-fonts/Cargo.toml");
    assert_eq!(
        app_manifest
            .lines()
            .find(|line| line.starts_with("production-fonts =")),
        Some("production-fonts = []")
    );
    assert!(!fonts_manifest.contains("shared-source-ownership ="));
    assert!(!fonts_manifest.contains("diagnostic-tools ="));

    let source = FontSource::new("production-latin", LATIN.to_vec());
    let catalog = FontCatalog::from_sources("en-US", [source]).expect("legacy production catalog");
    assert_eq!(catalog.face_count(), 1);
}

#[test]
fn production_fonts_remain_deferred_until_after_the_cpu_first_present() {
    let platform_fonts = read_repository_file("crates/rssh-app/src/platform_fonts.rs");
    let window_gpu = read_repository_file("crates/rssh-app/src/window_gpu.rs");
    let window_runtime = read_repository_file("crates/rssh-app/src/window_parts/part08.rs");
    let deferred_gpu = window_runtime
        .split_once("fn initialize_deferred_gpu")
        .expect("deferred GPU entry")
        .1
        .split_once("fn handle_deferred_gpu_initialized")
        .expect("bounded deferred GPU entry")
        .0;
    let finish_prepared = window_gpu
        .split_once("pub(crate) async fn finish_prepared")
        .expect("GPU worker finish entry")
        .1;

    assert!(platform_fonts.contains("FontCatalogMode::Lazy"));
    assert!(deferred_gpu.contains("self.rendered_frames == 0"));
    assert!(deferred_gpu.contains("spawn_deferred_gpu_task"));
    assert!(
        finish_prepared
            .find("GpuContext::finish_windowed")
            .expect("GPU context finish")
            < finish_prepared
                .find("PlatformFontRepository::production_index")
                .expect("deferred platform font index")
    );
}

#[test]
fn packaged_font_dependency_cannot_name_private_proof_constructors() {
    let packaged = compile_font_proof_consumer("packaged", false);
    assert!(
        !packaged.status.success(),
        "packaged dependency unexpectedly named proof constructors"
    );
    let packaged_stderr = String::from_utf8_lossy(&packaged.stderr);
    for constructor in [
        "from_sources_shared_for_diagnostics",
        "from_sources_copied_for_diagnostics",
    ] {
        assert!(
            packaged_stderr.contains(constructor),
            "packaged compile failure omitted {constructor}: {packaged_stderr}"
        );
    }

    let diagnostic = compile_font_proof_consumer("diagnostic", true);
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    assert!(
        diagnostic.status.success(),
        "diagnostic feature did not expose proof constructors: stdout={} stderr={}",
        String::from_utf8_lossy(&diagnostic.stdout),
        String::from_utf8_lossy(&diagnostic.stderr)
    );
    #[cfg(feature = "rterm-legacy-0-1")]
    {
        assert!(
            !diagnostic.status.success(),
            "frozen fonts exposed diagnostic proof features"
        );
        let stderr = String::from_utf8_lossy(&diagnostic.stderr);
        assert!(stderr.contains("diagnostic-tools"), "{stderr}");
        assert!(stderr.contains("does not have that feature"), "{stderr}");
    }
}

fn compile_font_proof_consumer(label: &str, diagnostic_tools: bool) -> Output {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "rssh-font-feature-contract-{label}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("src")).expect("create compile contract fixture");
    let font_path = repository_root()
        .join("crates/rterm-fonts")
        .to_string_lossy()
        .replace('\\', "/");
    let features = if diagnostic_tools {
        ", features = [\"diagnostic-tools\"]"
    } else {
        ""
    };
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"font-feature-contract-{label}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nrterm-fonts = {{ path = \"{font_path}\", default-features = false{features} }}\n"
        ),
    )
    .expect("write compile contract manifest");
    fs::write(
        root.join("src/main.rs"),
        r#"use rterm_fonts::{FontCatalog, FontSource};

fn main() {
    let _ = FontCatalog::from_sources_shared_for_diagnostics(
        "en-US",
        std::iter::empty::<FontSource>(),
    );
    let _ = FontCatalog::from_sources_copied_for_diagnostics(
        "en-US",
        std::iter::empty::<FontSource>(),
    );
}
"#,
    )
    .expect("write compile contract source");
    let target = root.join("target");
    let temporary = root.join("tmp");
    fs::create_dir_all(&temporary).expect("create compile contract temp");
    let output = Command::new("cargo")
        .args(["check", "--offline", "--quiet", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .env("TEMP", &temporary)
        .env("TMP", &temporary)
        .env("TMPDIR", &temporary)
        .output()
        .expect("execute compile feature contract");
    remove_fixture(&root);
    output
}

fn remove_fixture(root: &Path) {
    fs::remove_dir_all(root).unwrap_or_else(|error| {
        panic!(
            "remove compile contract fixture {}: {error}",
            root.display()
        )
    });
}

#[test]
fn release_packaging_builds_the_explicit_minimal_gui_feature_set() {
    let release = read_repository_file("docs/trial/legacy-workflows/release.yml");
    let windows_smoke = read_repository_file("scripts/ci/package-smoke.ps1");
    let unix_smoke = read_repository_file("scripts/ci/package-smoke.sh");
    let package_job = release
        .split_once("  build-package:")
        .expect("package job")
        .1
        .split_once("\n  publish-release:")
        .map_or_else(|| release.as_str(), |(job, _)| job);

    assert!(package_job.contains(
        "cargo build --locked --release -p rssh-app --no-default-features --features production-gui"
    ));
    for smoke in [&windows_smoke, &unix_smoke] {
        assert!(!smoke.contains("packaged doctor"));
        assert!(!smoke.contains("packaged self-test"));
        assert!(!smoke.contains("packaged benchmark gate"));
        assert!(smoke.contains("packaged OpenSSH loopback"));
        assert!(smoke.contains("packaged native ten-frame E2E"));
    }
}
