use std::{fs, path::PathBuf, process::Command};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("functional test crate is nested under crates")
        .to_owned()
}

#[test]
fn production_app_has_no_observer_dependency_without_the_feature() {
    let output = Command::new(env!("CARGO"))
        .args([
            "tree",
            "--locked",
            "-p",
            "rterm-app",
            "--edges",
            "normal",
            "--prefix",
            "none",
        ])
        .current_dir(workspace_root())
        .output()
        .unwrap();
    assert!(output.status.success(), "cargo tree failed: {output:?}");
    let tree = String::from_utf8(output.stdout).unwrap();
    assert!(!tree.contains("rterm-functional-tests"), "{tree}");
}

#[test]
fn observer_source_is_strictly_feature_gated_in_the_app() {
    let root = workspace_root();
    let manifest = fs::read_to_string(root.join("crates/rterm-app/Cargo.toml")).unwrap();
    let main = fs::read_to_string(root.join("crates/rterm-app/src/main.rs")).unwrap();
    assert!(manifest.contains("functional-test-observer"));
    assert!(manifest.contains("rterm-functional-tests"));
    assert!(main.contains("#[cfg(feature = \"functional-test-observer\")]"));
    assert!(!main.contains("rterm-functional-observer-v1"));
}
