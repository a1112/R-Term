use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestIdentity {
    source: String,
    name: String,
    occurrence: usize,
}

#[test]
fn manifest_maps_every_app_test_to_one_unique_behavior_id() {
    let root = repository_root();
    let manifest = fs::read_to_string(
        root.join("crates/rterm-app/tests/fixtures/task23_app_test_manifest.txt"),
    )
    .expect("read Task 23 manifest");
    let mut mapped = HashSet::new();
    let mut behavior_ids = HashSet::new();

    for line in manifest.lines().filter(|line| !line.starts_with('#')) {
        let fields = line.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 7, "malformed manifest row: {line}");
        let identity = TestIdentity {
            source: fields[1].to_owned(),
            name: fields[2].to_owned(),
            occurrence: fields[3].parse().expect("numeric occurrence"),
        };
        assert_eq!(fields[0], behavior_id(&identity, fields[6]));
        assert_eq!(fields[4], "rterm-app");
        assert!(!fields[5].is_empty());
        assert!(mapped.insert(identity), "duplicate test mapping: {line}");
        assert!(
            behavior_ids.insert(fields[0]),
            "duplicate behavior id: {}",
            fields[0]
        );
    }

    let discovered = discover_source_files(&root)
        .iter()
        .flat_map(|source| discover_tests(&root, source))
        .collect::<HashSet<_>>();
    let missing = discovered.difference(&mapped).take(16).collect::<Vec<_>>();
    let stale = mapped.difference(&discovered).take(16).collect::<Vec<_>>();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "regenerate the Task 23 manifest: missing={missing:?}, stale={stale:?}"
    );
}

fn discover_source_files(root: &Path) -> Vec<String> {
    fn visit(root: &Path, directory: &Path, sources: &mut Vec<String>) {
        for entry in fs::read_dir(directory).expect("read app source directory") {
            let path = entry.expect("read source entry").path();
            if path.is_dir() {
                visit(root, &path, sources);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("source under repository root");
                sources.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    let mut sources = Vec::new();
    visit(root, &root.join("crates/rterm-app/src"), &mut sources);
    sources.sort();
    sources
}

fn repository_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn discover_tests(root: &Path, source: &str) -> Vec<TestIdentity> {
    let text = fs::read_to_string(root.join(source)).expect("read test source");
    let mut pending = false;
    let mut occurrences = HashMap::<String, usize>::new();
    let mut discovered = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if is_test_attribute(trimmed) {
            pending = true;
            continue;
        }
        if !pending {
            continue;
        }
        let Some(name) = function_name(trimmed) else {
            continue;
        };
        pending = false;
        let occurrence = occurrences.entry(name.to_owned()).or_default();
        *occurrence += 1;
        discovered.push(TestIdentity {
            source: source.to_owned(),
            name: name.to_owned(),
            occurrence: *occurrence,
        });
    }
    discovered
}

fn is_test_attribute(line: &str) -> bool {
    let compact = line.split_whitespace().collect::<String>();
    compact == "#[test]"
        || compact.starts_with("#[test(")
        || compact.ends_with("::test]")
        || compact.contains("::test(")
}

fn function_name(line: &str) -> Option<&str> {
    let function = line.find("fn ")? + 3;
    let rest = &line[function..];
    let end = rest
        .find(|character: char| !(character == '_' || character.is_ascii_alphanumeric()))
        .unwrap_or(rest.len());
    (end > 0).then_some(&rest[..end])
}

fn behavior_id(identity: &TestIdentity, domain: &str) -> String {
    let digest = Sha256::digest(format!(
        "{}|{}|{}",
        identity.source, identity.name, identity.occurrence
    ));
    let prefix = digest[..6]
        .iter()
        .fold(String::with_capacity(12), |mut output, byte| {
            write!(output, "{byte:02x}").expect("write digest byte");
            output
        });
    format!("T23-{}-{prefix}", domain.to_ascii_uppercase())
}
