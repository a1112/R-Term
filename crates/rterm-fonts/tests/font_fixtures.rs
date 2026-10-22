use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};
use ttf_parser::{Face, Tag, name_id};

const MANIFEST_HEADER: &str = "role\tfile\tlicense\tlicense_file\tlicense_source\tlicense_sha256\tcodepoints\tsequences\tgsub_features\tcolor\tsource\tversion\tsubset_command";

#[derive(Debug)]
struct Fixture<'a> {
    role: &'a str,
    file: &'a str,
    license: &'a str,
    license_file: &'a str,
    license_source: &'a str,
    license_sha256: &'a str,
    codepoints: &'a str,
    sequences: &'a str,
    gsub_features: &'a str,
    color: &'a str,
    source: &'a str,
    version: &'a str,
    subset_command: &'a str,
}

#[test]
fn shaping_font_fixtures_are_licensed_pinned_and_cover_the_manifest() {
    let root = fixture_root();
    assert!(
        root.is_dir(),
        "missing fixture directory: {}",
        root.display()
    );

    let readme = read_utf8(&root.join("README.md"));
    let manifest = read_utf8(&root.join("MANIFEST.tsv"));
    let checksums = parse_checksums(&root.join("SHA256SUMS"));
    let fixtures = parse_manifest(&manifest);
    assert_eq!(
        read_utf8(&root.join("FONTTOOLS_VERSION")).trim(),
        "4.61.1",
        "fixture rebuilds must use the pinned fonttools version"
    );
    let rebuild_script = read_utf8(&root.join("rebuild_check.py"));
    let rebuild_tests = read_utf8(&root.join("test_rebuild_check.py"));
    assert!(
        rebuild_script.contains("FONTTOOLS_VERSION")
            && rebuild_script.contains("MANIFEST.tsv")
            && rebuild_script.contains("SHA256SUMS")
            && rebuild_tests.contains("test_rejects_six_of_seven_outputs")
            && rebuild_tests.contains("test_rejects_traversal_and_absolute_paths")
            && readme.contains("rebuild_check.py")
            && readme.contains("test_rebuild_check.py"),
        "the documented rebuild/check tooling must consume metadata and cover fail-closed paths"
    );

    assert!(
        !fixtures.is_empty(),
        "font fixture manifest must contain at least one fixture"
    );
    let fixture_roles = fixtures
        .iter()
        .map(|fixture| fixture.role)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        fixture_roles.len(),
        fixtures.len(),
        "MANIFEST.tsv contains duplicate fixture roles"
    );
    assert_eq!(
        fixture_roles,
        BTreeSet::from([
            "arabic",
            "cjk",
            "color-emoji",
            "devanagari",
            "hebrew",
            "latin-ligature",
            "symbols-text",
        ]),
        "fixture roles must cover every Task 12 shaping scenario"
    );

    let fixture_files = fixture_font_files(&root);
    let manifest_files = fixtures
        .iter()
        .map(|fixture| fixture.file.to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        manifest_files.len(),
        fixtures.len(),
        "MANIFEST.tsv contains duplicate fixture paths"
    );
    assert_eq!(
        fixture_files, manifest_files,
        "every font fixture must appear exactly once in MANIFEST.tsv"
    );
    assert_eq!(
        checksums.keys().cloned().collect::<BTreeSet<_>>(),
        manifest_files,
        "SHA256SUMS must list every and only fixture font"
    );
    let manifest_licenses = fixtures
        .iter()
        .map(|fixture| fixture.license_file.to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        manifest_licenses,
        fixture_license_files(&root),
        "LICENSES must contain every and only license referenced by MANIFEST.tsv"
    );

    for fixture in fixtures {
        let bytes = validate_fixture_metadata(&root, &readme, &checksums, &fixture);
        validate_font_coverage(&fixture, &bytes);
    }

    let total_size = manifest_files
        .iter()
        .map(|file| {
            fs::metadata(root.join(file))
                .unwrap_or_else(|error| panic!("failed to stat {file}: {error}"))
                .len()
        })
        .sum::<u64>();
    assert!(
        total_size <= 2 * 1024 * 1024,
        "font fixtures exceed the 2 MiB repository budget: {total_size} bytes"
    );
}

#[test]
fn portable_fixture_paths_reject_windows_aliases_and_illegal_components() {
    for invalid in [
        "LICENSES/COM\u{00B9}.txt",
        "LICENSES/LPT\u{00B2}.txt",
        "LICENSES/CONIN$.txt",
        "LICENSES/CONOUT$.txt",
        "LICENSES/trailing.",
        "LICENSES/trailing ",
        "LICENSES/illegal<name>.txt",
    ] {
        assert!(
            std::panic::catch_unwind(|| assert_portable_relative_path(invalid)).is_err(),
            "portable path validator accepted {invalid:?}"
        );
    }
}

#[test]
fn license_enumeration_includes_nested_extra_files() {
    let unique = format!(
        "rterm-font-license-enumeration-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time must follow the Unix epoch")
            .as_nanos()
    );
    let root = std::env::temp_dir().join(unique);
    let nested = root.join("LICENSES/nested");
    fs::create_dir_all(&nested).expect("nested test license directory must be created");
    fs::write(nested.join("extra.txt"), b"extra")
        .expect("nested test license file must be created");

    assert_eq!(
        fixture_license_files(&root),
        BTreeSet::from(["LICENSES/nested/extra.txt".to_owned()])
    );

    fs::remove_dir_all(root).expect("test license directory must be removed");
}

fn validate_fixture_metadata(
    root: &Path,
    readme: &str,
    checksums: &BTreeMap<String, String>,
    fixture: &Fixture<'_>,
) -> Vec<u8> {
    let expected_license = expected_license(fixture.role);
    assert_portable_relative_path(fixture.file);
    assert_portable_relative_path(fixture.license_file);
    assert_eq!(
        fixture.license, expected_license.identifier,
        "{} has an unexpected license identifier",
        fixture.file
    );
    assert_eq!(
        fixture.license_file, expected_license.file,
        "{} is mapped to the wrong upstream license file",
        fixture.file
    );
    assert_eq!(
        fixture.license_source, expected_license.source,
        "{} is mapped to the wrong upstream license URL",
        fixture.file
    );
    assert_eq!(
        fixture.license_sha256, expected_license.sha256,
        "{} has an unexpected license SHA-256",
        fixture.file
    );
    assert_eq!(
        fixture.source, expected_license.font_source,
        "{} has an unexpected upstream font URL",
        fixture.file
    );

    let license = fs::read(root.join(fixture.license_file))
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", fixture.license_file));
    assert_eq!(
        sha256_hex(&license),
        expected_license.sha256,
        "{} differs from the exact pinned upstream license",
        fixture.license_file
    );
    assert!(
        official_immutable_source(fixture.source)
            && official_immutable_source(fixture.license_source)
            && github_commit(fixture.source) == github_commit(fixture.license_source),
        "{} font and license sources must be official raw paths at the same immutable commit",
        fixture.file
    );
    assert!(
        !fixture.version.trim().is_empty(),
        "{} must record its upstream font version",
        fixture.file
    );
    assert!(
        fixture.subset_command.starts_with("pyftsubset ")
            && fixture.subset_command.contains("--no-recalc-timestamp")
            && fixture.subset_command.contains("--canonical-order"),
        "{} must record a reproducible pyftsubset command",
        fixture.file
    );
    assert!(
        readme.contains(fixture.file)
            && readme.contains(fixture.source)
            && readme.contains(fixture.version)
            && readme.contains(fixture.subset_command)
            && readme.contains(fixture.license_file)
            && readme.contains(fixture.license_source)
            && readme.contains(fixture.license_sha256),
        "README.md must document source, version, subset, and exact license for {}",
        fixture.file
    );

    let bytes = fs::read(root.join(fixture.file))
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", fixture.file));
    let maximum_size = if fixture.role == "color-emoji" {
        1_500 * 1024
    } else {
        256 * 1024
    };
    assert!(
        bytes.len() <= maximum_size,
        "{} exceeds its minimal fixture size budget of {maximum_size} bytes",
        fixture.file
    );
    assert_eq!(
        checksums[fixture.file],
        sha256_hex(&bytes),
        "SHA-256 mismatch for {}",
        fixture.file
    );
    bytes
}

fn validate_font_coverage(fixture: &Fixture<'_>, bytes: &[u8]) {
    let face = Face::parse(bytes, 0)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error:?}", fixture.file));
    assert!(
        face.names()
            .into_iter()
            .filter(|name| name.name_id == name_id::VERSION)
            .filter_map(|name| name.to_string())
            .any(|version| version.contains(fixture.version)),
        "{} does not embed documented version {}",
        fixture.file,
        fixture.version
    );
    let expected_color = fixture.color.parse::<bool>().unwrap_or_else(|error| {
        panic!(
            "{} has invalid color flag {:?}: {error}",
            fixture.file, fixture.color
        )
    });
    for codepoint in comma_separated(fixture.codepoints) {
        let (value, character) = parse_codepoint(fixture.file, codepoint);
        assert!(
            face.glyph_index(character).is_some(),
            "{} is missing documented glyph U+{value:04X}",
            fixture.file
        );
    }
    validate_sequences(fixture, &face, expected_color);
    validate_gsub_features(fixture, &face);
    validate_reserved_font_names(fixture, &face);

    let has_color = face.tables().colr.is_some()
        || face.tables().cbdt.is_some()
        || face.tables().sbix.is_some()
        || face.tables().svg.is_some();
    assert_eq!(
        has_color, expected_color,
        "{} color-table coverage differs from MANIFEST.tsv",
        fixture.file
    );
}

fn validate_sequences(fixture: &Fixture<'_>, face: &Face<'_>, expected_color: bool) {
    for sequence in comma_separated(fixture.sequences) {
        let codepoints = sequence
            .split('+')
            .map(|codepoint| parse_codepoint(fixture.file, codepoint))
            .collect::<Vec<_>>();
        assert!(
            codepoints.len() >= 2,
            "{} has a non-sequence coverage entry {sequence:?}",
            fixture.file
        );
        for (index, (value, character)) in codepoints.iter().copied().enumerate() {
            if matches!(value, 0xFE0E | 0xFE0F) {
                validate_variation_selector(
                    fixture,
                    face,
                    expected_color,
                    sequence,
                    &codepoints,
                    index,
                    character,
                );
            } else {
                assert!(
                    face.glyph_index(character).is_some(),
                    "{} is missing U+{value:04X} from sequence {sequence:?}",
                    fixture.file
                );
            }
        }
    }
}

fn validate_variation_selector(
    fixture: &Fixture<'_>,
    face: &Face<'_>,
    expected_color: bool,
    sequence: &str,
    codepoints: &[(u32, char)],
    index: usize,
    selector: char,
) {
    let (value, _) = codepoints[index];
    let (_, base) = codepoints[index.checked_sub(1).unwrap_or_else(|| {
        panic!(
            "{} sequence {sequence:?} starts with a variation selector",
            fixture.file
        )
    })];
    if value == 0xFE0E {
        assert!(
            !expected_color && face.glyph_index(base).is_some(),
            "{} lacks a text-presentation base in {sequence:?}",
            fixture.file
        );
    } else {
        assert!(
            face.glyph_variation_index(base, selector).is_some(),
            "{} lacks documented variation sequence in {sequence:?}",
            fixture.file
        );
    }
}

fn validate_gsub_features(fixture: &Fixture<'_>, face: &Face<'_>) {
    let features = face.tables().gsub.map(|gsub| {
        gsub.features
            .into_iter()
            .map(|feature| feature.tag)
            .collect::<BTreeSet<_>>()
    });
    for feature in comma_separated(fixture.gsub_features) {
        let tag_bytes: [u8; 4] = feature.as_bytes().try_into().unwrap_or_else(|_| {
            panic!(
                "{} has non-four-byte GSUB feature {feature:?}",
                fixture.file
            )
        });
        assert!(
            features
                .as_ref()
                .is_some_and(|features| features.contains(&Tag::from_bytes(&tag_bytes))),
            "{} is missing documented GSUB feature {feature}",
            fixture.file
        );
    }
}

fn validate_reserved_font_names(fixture: &Fixture<'_>, face: &Face<'_>) {
    if fixture.role != "cjk" {
        return;
    }
    for name in face.names().into_iter().filter(|name| {
        matches!(
            name.name_id,
            name_id::FAMILY
                | name_id::FULL_NAME
                | name_id::POST_SCRIPT_NAME
                | name_id::TYPOGRAPHIC_FAMILY
                | name_id::TYPOGRAPHIC_SUBFAMILY
                | name_id::COMPATIBLE_FULL
                | name_id::POST_SCRIPT_CID
                | name_id::WWS_FAMILY
                | name_id::WWS_SUBFAMILY
                | name_id::VARIATIONS_POST_SCRIPT_NAME_PREFIX
        )
    }) {
        let decoded_uses_source = name
            .to_string()
            .is_some_and(|value| value.to_ascii_lowercase().contains("source"));
        let raw_uses_source = name
            .name
            .windows(b"source".len())
            .any(|window| window.eq_ignore_ascii_case(b"source"));
        assert!(
            !decoded_uses_source && !raw_uses_source,
            "{} uses CJK Reserved Font Name 'Source' in name ID {}",
            fixture.file,
            name.name_id
        );
    }
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/fonts")
}

fn read_utf8(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn parse_manifest(contents: &str) -> Vec<Fixture<'_>> {
    let mut lines = contents.lines();
    assert_eq!(
        lines.next(),
        Some(MANIFEST_HEADER),
        "MANIFEST.tsv header changed without updating its integrity test"
    );
    lines
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let columns = line.split('\t').collect::<Vec<_>>();
            assert_eq!(
                columns.len(),
                13,
                "manifest row must contain exactly thirteen tab-separated columns: {line:?}"
            );
            Fixture {
                role: columns[0],
                file: columns[1],
                license: columns[2],
                license_file: columns[3],
                license_source: columns[4],
                license_sha256: columns[5],
                codepoints: columns[6],
                sequences: columns[7],
                gsub_features: columns[8],
                color: columns[9],
                source: columns[10],
                version: columns[11],
                subset_command: columns[12],
            }
        })
        .collect()
}

fn parse_checksums(path: &Path) -> BTreeMap<String, String> {
    let mut checksums = BTreeMap::new();
    for line in read_utf8(path)
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let (checksum, file) = line
            .split_once("  ")
            .unwrap_or_else(|| panic!("invalid SHA256SUMS row: {line:?}"));
        assert!(
            checksum.len() == 64
                && checksum
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "invalid lowercase SHA-256 in row: {line:?}"
        );
        assert_portable_relative_path(file);
        assert!(
            checksums
                .insert(file.to_owned(), checksum.to_owned())
                .is_none(),
            "SHA256SUMS contains duplicate fixture path {file:?}"
        );
    }
    checksums
}

fn fixture_font_files(root: &Path) -> BTreeSet<String> {
    fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to list {}: {error}", root.display()))
        .map(|entry| entry.expect("fixture directory entry must be readable"))
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            let path = entry.path();
            let extension = path.extension()?.to_str()?;
            matches!(extension.to_ascii_lowercase().as_str(), "ttf" | "otf").then(|| {
                path.file_name()
                    .expect("font fixture must have a file name")
                    .to_string_lossy()
                    .into_owned()
            })
        })
        .collect()
}

fn fixture_license_files(root: &Path) -> BTreeSet<String> {
    let license_root = root.join("LICENSES");
    let mut files = BTreeSet::new();
    collect_license_files(&license_root, &license_root, &mut files);
    files
}

fn collect_license_files(root: &Path, directory: &Path, files: &mut BTreeSet<String>) {
    for entry in fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to list {}: {error}", directory.display()))
    {
        let entry = entry.expect("license directory entry must be readable");
        let file_type = entry
            .file_type()
            .expect("license directory entry type must be readable");
        assert!(
            !file_type.is_symlink(),
            "license fixture must not be a symlink: {}",
            entry.path().display()
        );
        if file_type.is_dir() {
            collect_license_files(root, &entry.path(), files);
        } else {
            assert!(
                file_type.is_file(),
                "license fixture must be a regular file: {}",
                entry.path().display()
            );
            let relative = entry
                .path()
                .strip_prefix(root)
                .expect("license file must remain under LICENSES")
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            assert!(
                files.insert(format!("LICENSES/{relative}")),
                "duplicate license fixture path: {relative}"
            );
        }
    }
}

fn comma_separated(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').filter(|item| !item.is_empty())
}

fn parse_codepoint(file: &str, codepoint: &str) -> (u32, char) {
    let value = u32::from_str_radix(codepoint, 16)
        .unwrap_or_else(|error| panic!("{file} has invalid codepoint {codepoint}: {error}"));
    let character = char::from_u32(value)
        .unwrap_or_else(|| panic!("{file} has non-scalar codepoint {codepoint}"));
    (value, character)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        })
}

fn assert_portable_relative_path(value: &str) {
    let path = Path::new(value);
    assert!(
        !value.contains('\\')
            && !value.contains(':')
            && !path.is_absolute()
            && value.split('/').all(|component| {
                !component.is_empty()
                    && component != "."
                    && component != ".."
                    && is_portable_windows_component(component)
            })
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "fixture path must be a portable, normalized relative path: {value:?}"
    );
}

fn is_portable_windows_component(value: &str) -> bool {
    !value.ends_with([' ', '.'])
        && !value
            .chars()
            .any(|character| character.is_control() || r#"<>:"/\|?*"#.contains(character))
        && !is_windows_reserved_name(value)
}

fn is_windows_reserved_name(value: &str) -> bool {
    let stem = value
        .split('.')
        .next()
        .unwrap_or(value)
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|suffix| match suffix {
            "\u{00B9}" | "\u{00B2}" | "\u{00B3}" => true,
            _ => suffix
                .parse::<u8>()
                .is_ok_and(|number| (1..=9).contains(&number)),
        })
}

fn immutable_github_source(source: &str) -> bool {
    github_commit(source).is_some()
}

fn github_commit(source: &str) -> Option<&str> {
    source.split('/').find(|segment| {
        segment.len() == 40
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn official_immutable_source(source: &str) -> bool {
    (source.starts_with("https://raw.githubusercontent.com/google/fonts/")
        || source.starts_with("https://raw.githubusercontent.com/googlefonts/noto-emoji/"))
        && immutable_github_source(source)
}

struct ExpectedLicense {
    identifier: &'static str,
    file: &'static str,
    font_source: &'static str,
    source: &'static str,
    sha256: &'static str,
}

fn expected_license(role: &str) -> ExpectedLicense {
    match role {
        "latin-ligature" => ExpectedLicense {
            identifier: "OFL-1.1",
            file: "LICENSES/NotoSans-OFL-1.1.txt",
            font_source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/",
                "ofl/notosans/NotoSans%5Bwdth,wght%5D.ttf"
            ),
            source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/ofl/notosans/OFL.txt"
            ),
            sha256: "cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a",
        },
        "cjk" => ExpectedLicense {
            identifier: "OFL-1.1",
            file: "LICENSES/NotoSansSC-OFL-1.1.txt",
            font_source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/",
                "ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf"
            ),
            source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/ofl/notosanssc/OFL.txt"
            ),
            sha256: "1c05c68c34f9708415aada51f17e1b0092d2cea709bf4a94cd38114f9e73d7d9",
        },
        "arabic" => ExpectedLicense {
            identifier: "OFL-1.1",
            file: "LICENSES/NotoSansArabic-OFL-1.1.txt",
            font_source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/",
                "ofl/notosansarabic/NotoSansArabic%5Bwdth,wght%5D.ttf"
            ),
            source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/ofl/notosansarabic/OFL.txt"
            ),
            sha256: "07fc70bfeb985cc1a87a8587d0a0c80bab11c86c9dc3fd95b6f0cb332f983e96",
        },
        "devanagari" => ExpectedLicense {
            identifier: "OFL-1.1",
            file: "LICENSES/NotoSansDevanagari-OFL-1.1.txt",
            font_source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/",
                "ofl/notosansdevanagari/NotoSansDevanagari%5Bwdth,wght%5D.ttf"
            ),
            source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/ofl/notosansdevanagari/OFL.txt"
            ),
            sha256: "a216f6f8d85c7228093e0ee5e258d9d377e6671f68acb4db1930b29583d0f331",
        },
        "hebrew" => ExpectedLicense {
            identifier: "OFL-1.1",
            file: "LICENSES/NotoSansHebrew-OFL-1.1.txt",
            font_source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/",
                "ofl/notosanshebrew/NotoSansHebrew%5Bwdth,wght%5D.ttf"
            ),
            source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/ofl/notosanshebrew/OFL.txt"
            ),
            sha256: "9b9fe028b5ba74d231659a1bbaf0ed09b11e759d1ca6a070999e16d151616b47",
        },
        "symbols-text" => ExpectedLicense {
            identifier: "OFL-1.1",
            file: "LICENSES/NotoSansSymbols2-OFL-1.1.txt",
            font_source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/",
                "ofl/notosanssymbols2/NotoSansSymbols2-Regular.ttf"
            ),
            source: concat!(
                "https://raw.githubusercontent.com/google/fonts/",
                "7ff85c87f93ea6cca5f41c69f2e4edcb90240f26/ofl/notosanssymbols2/OFL.txt"
            ),
            sha256: "b118dd41337806a5d4797052c77caf3bd096aed783e5eb21b4d11154351e1ac0",
        },
        "color-emoji" => ExpectedLicense {
            identifier: "OFL-1.1",
            file: "LICENSES/NotoColorEmoji-OFL-1.1.txt",
            font_source: concat!(
                "https://raw.githubusercontent.com/googlefonts/noto-emoji/",
                "8998f5dd683424a73e2314a8c1f1e359c19e8742/fonts/NotoColorEmoji.ttf"
            ),
            source: concat!(
                "https://raw.githubusercontent.com/googlefonts/noto-emoji/",
                "8998f5dd683424a73e2314a8c1f1e359c19e8742/fonts/LICENSE"
            ),
            sha256: "6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2",
        },
        _ => panic!("fixture role has no allow-listed license mapping: {role}"),
    }
}
