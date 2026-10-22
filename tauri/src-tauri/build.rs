fn main() {
    let functional = std::env::var_os("CARGO_FEATURE_FUNCTIONAL_TEST_OBSERVER").is_some();
    let (capabilities, permissions) = if functional {
        (
            "./capabilities/functional/*.json",
            "./permissions/functional/*.toml",
        )
    } else {
        (
            "./capabilities/production/*.json",
            "./permissions/production/*.toml",
        )
    };
    println!("cargo:rerun-if-changed={capabilities}");
    println!("cargo:rerun-if-changed={permissions}");

    let attributes = tauri_build::Attributes::new()
        .capabilities_path_pattern(capabilities)
        .app_manifest(tauri_build::AppManifest::new().permissions_path_pattern(permissions));
    tauri_build::try_build(attributes).expect("failed to build the R-Term Tauri application");
}
