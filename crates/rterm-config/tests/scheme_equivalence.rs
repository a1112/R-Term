use rterm_config::schemes;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;

const LEGACY_FIXTURE: &str = include_str!("fixtures/legacy-color-schemes.tsv");

fn decode_base64(value: &str) -> Vec<u8> {
    fn sextet(byte: u8) -> u8 {
        match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("invalid base64 byte"),
        }
    }

    let mut decoded = Vec::with_capacity(value.len() / 4 * 3);
    for chunk in value.as_bytes().chunks_exact(4) {
        let values = [
            sextet(chunk[0]),
            sextet(chunk[1]),
            if chunk[2] == b'=' {
                0
            } else {
                sextet(chunk[2])
            },
            if chunk[3] == b'=' {
                0
            } else {
                sextet(chunk[3])
            },
        ];
        decoded.push((values[0] << 2) | (values[1] >> 4));
        if chunk[2] != b'=' {
            decoded.push((values[1] << 4) | (values[2] >> 2));
        }
        if chunk[3] != b'=' {
            decoded.push((values[2] << 6) | values[3]);
        }
    }
    decoded
}

fn sha256(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").unwrap();
            output
        })
}

fn canonical_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(canonical_json).collect())
        }
        toml::Value::Table(table) => serde_json::to_value(
            table
                .iter()
                .map(|(key, value)| (key, canonical_json(value)))
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap(),
        scalar => serde_json::to_value(scalar).unwrap(),
    }
}

#[test]
fn generated_lookup_is_byte_and_semantically_equivalent_to_legacy_table() {
    let fixtures = LEGACY_FIXTURE
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let mut fields = line.split('\t');
            let name = String::from_utf8(decode_base64(fields.next().unwrap())).unwrap();
            let raw_hash = fields.next().unwrap();
            let semantic_hash = fields.next().unwrap();
            assert!(fields.next().is_none());
            (name, raw_hash, semantic_hash)
        })
        .collect::<Vec<_>>();

    assert_eq!(fixtures.len(), 1_113);
    assert_eq!(schemes::MANIFEST.format_version, 1);
    assert_eq!(schemes::MANIFEST.lookup_count, fixtures.len());
    assert_eq!(schemes::MANIFEST.scheme_count, 999);
    assert_eq!(schemes::MANIFEST.active_scheme_count, 966);
    assert_eq!(schemes::MANIFEST.shadowed_scheme_count, 33);
    assert_eq!(schemes::names().len(), fixtures.len());
    assert_eq!(
        schemes::names().collect::<Vec<_>>(),
        fixtures
            .iter()
            .map(|(name, _, _)| name.as_str())
            .collect::<Vec<_>>()
    );

    for (name, expected_raw_hash, expected_semantic_hash) in fixtures {
        let contents = schemes::get(&name).unwrap_or_else(|| panic!("missing scheme {name:?}"));
        assert_eq!(sha256(contents.as_bytes()), expected_raw_hash, "{name}");

        let parsed = toml::from_str::<toml::Value>(contents).unwrap();
        let colors = parsed.get("colors").unwrap();
        let canonical = serde_json::to_vec(&canonical_json(colors)).unwrap();
        assert_eq!(sha256(&canonical), expected_semantic_hash, "{name}");
    }
    assert!(schemes::get("not a built-in color scheme").is_none());
}

#[test]
fn lookup_returns_borrowed_static_data() {
    let first = schemes::get("Builtin Dark").unwrap();
    let second = schemes::get("Builtin Dark").unwrap();
    assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
}
