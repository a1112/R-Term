use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const ASSET_DIRECTORY: &str = "assets/color-schemes";
// Bundled scheme archives retain their original versioned wire format.
const PACK_MAGIC: &[u8; 8] = b"RSSHCS1\0";

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn sha256(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        })
}

fn rust_integer(value: u32) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.char_indices() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push('_');
        }
        formatted.push(digit);
    }
    formatted
}

fn take_u16(input: &[u8], cursor: &mut usize) -> Result<u16, io::Error> {
    let end = cursor
        .checked_add(2)
        .ok_or_else(|| invalid_data("pack cursor overflow"))?;
    let bytes = input
        .get(*cursor..end)
        .ok_or_else(|| invalid_data("truncated u16 in scheme pack"))?;
    *cursor = end;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn take_u32(input: &[u8], cursor: &mut usize) -> Result<u32, io::Error> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| invalid_data("pack cursor overflow"))?;
    let bytes = input
        .get(*cursor..end)
        .ok_or_else(|| invalid_data("truncated u32 in scheme pack"))?;
    *cursor = end;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn take<'a>(input: &'a [u8], cursor: &mut usize, length: usize) -> Result<&'a [u8], io::Error> {
    let end = cursor
        .checked_add(length)
        .ok_or_else(|| invalid_data("pack cursor overflow"))?;
    let value = input
        .get(*cursor..end)
        .ok_or_else(|| invalid_data("truncated scheme pack record"))?;
    *cursor = end;
    Ok(value)
}

fn parse_pack(input: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    if input.get(..PACK_MAGIC.len()) != Some(PACK_MAGIC) {
        return Err(invalid_data("invalid color scheme pack magic").into());
    }
    let mut cursor = PACK_MAGIC.len();
    let count = usize::try_from(take_u32(input, &mut cursor)?)?;
    let mut schemes = BTreeMap::new();
    for _ in 0..count {
        let name_length = usize::from(take_u16(input, &mut cursor)?);
        let data_length = usize::try_from(take_u32(input, &mut cursor)?)?;
        let name = std::str::from_utf8(take(input, &mut cursor, name_length)?)?.to_owned();
        let contents = take(input, &mut cursor, data_length)?.to_vec();
        let toml = std::str::from_utf8(&contents)?;
        toml::from_str::<toml::Value>(toml)
            .map_err(|error| invalid_data(format!("invalid TOML in {name}: {error}")))?;
        if schemes.insert(name.clone(), contents).is_some() {
            return Err(invalid_data(format!("duplicate scheme record: {name}")).into());
        }
    }
    if cursor != input.len() {
        return Err(invalid_data("trailing bytes in color scheme pack").into());
    }
    Ok(schemes)
}

fn decode_base64(input: &str) -> Result<Vec<u8>, io::Error> {
    fn sextet(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    if input.len() % 4 != 0 {
        return Err(invalid_data("invalid base64 length"));
    }
    let mut decoded = Vec::with_capacity(input.len() / 4 * 3);
    for chunk in input.as_bytes().chunks_exact(4) {
        let values = [
            sextet(chunk[0]).ok_or_else(|| invalid_data("invalid base64 byte"))?,
            sextet(chunk[1]).ok_or_else(|| invalid_data("invalid base64 byte"))?,
            if chunk[2] == b'=' {
                0
            } else {
                sextet(chunk[2]).ok_or_else(|| invalid_data("invalid base64 byte"))?
            },
            if chunk[3] == b'=' {
                0
            } else {
                sextet(chunk[3]).ok_or_else(|| invalid_data("invalid base64 byte"))?
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
    Ok(decoded)
}

fn parse_aliases(input: &str) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut aliases = BTreeMap::new();
    for line in input.lines().filter(|line| !line.starts_with('#')) {
        let (encoded_name, scheme) = line
            .split_once('\t')
            .ok_or_else(|| invalid_data("invalid alias record"))?;
        let name = String::from_utf8(decode_base64(encoded_name)?)?;
        if aliases.insert(name.clone(), scheme.to_owned()).is_some() {
            return Err(invalid_data(format!("duplicate scheme alias: {name:?}")).into());
        }
    }
    Ok(aliases)
}

fn manifest_usize(manifest: &serde_json::Value, field: &str) -> Result<usize, Box<dyn Error>> {
    let value = manifest
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| invalid_data(format!("manifest field {field:?} is missing")))?;
    Ok(usize::try_from(value)?)
}

fn run() -> Result<(), Box<dyn Error>> {
    let asset_directory = Path::new(ASSET_DIRECTORY);
    for name in [
        "schemes.pack",
        "aliases.tsv",
        "shadowed.txt",
        "manifest.json",
    ] {
        println!(
            "cargo:rerun-if-changed={}",
            asset_directory.join(name).display()
        );
    }

    let pack = fs::read(asset_directory.join("schemes.pack"))?;
    let schemes = parse_pack(&pack)?;
    let aliases = parse_aliases(&fs::read_to_string(asset_directory.join("aliases.tsv"))?)?;
    let shadowed = fs::read_to_string(asset_directory.join("shadowed.txt"))?
        .lines()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let active = aliases.values().cloned().collect::<BTreeSet<_>>();
    let all = schemes.keys().cloned().collect::<BTreeSet<_>>();

    for scheme in &active {
        if !schemes.contains_key(scheme) {
            return Err(invalid_data(format!("alias references missing scheme: {scheme}")).into());
        }
    }
    if active.intersection(&shadowed).next().is_some()
        || active.union(&shadowed).cloned().collect::<BTreeSet<_>>() != all
    {
        return Err(invalid_data("active and shadowed schemes do not partition the pack").into());
    }

    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(asset_directory.join("manifest.json"))?)?;
    let format_version = manifest_usize(&manifest, "format_version")?;
    let expected_pack_hash = manifest
        .get("pack_sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_data("manifest pack_sha256 is missing"))?;
    let actual_pack_hash = sha256(&pack);
    let counts = [
        ("lookup_count", aliases.len()),
        ("scheme_count", schemes.len()),
        ("active_scheme_count", active.len()),
        ("shadowed_scheme_count", shadowed.len()),
    ];
    for (field, actual) in counts {
        let expected = manifest_usize(&manifest, field)?;
        if actual != expected {
            return Err(invalid_data(format!(
                "manifest {field} mismatch: got {actual}, expected {expected}"
            ))
            .into());
        }
    }
    if actual_pack_hash != expected_pack_hash {
        return Err(invalid_data(format!(
            "scheme pack checksum mismatch: got {actual_pack_hash}, expected {expected_pack_hash}"
        ))
        .into());
    }

    let mut compiled = Vec::new();
    let mut ranges = BTreeMap::new();
    for scheme in &active {
        let contents = &schemes[scheme];
        let start = u32::try_from(compiled.len())?;
        let length = u32::try_from(contents.len())?;
        compiled.extend_from_slice(contents);
        ranges.insert(scheme.clone(), (start, length));
    }

    let mut generated = String::new();
    writeln!(
        generated,
        "pub(crate) static INDEX: &[(&str, u32, u32)] = &["
    )?;
    for (name, scheme) in &aliases {
        let (start, length) = ranges[scheme];
        let start = rust_integer(start);
        let length = rust_integer(length);
        writeln!(generated, "    ({name:?}, {start}, {length}),")?;
    }
    writeln!(generated, "];")?;
    writeln!(
        generated,
        "pub const MANIFEST: SchemeManifest = SchemeManifest {{ format_version: {format_version}, lookup_count: {}, scheme_count: {}, active_scheme_count: {}, shadowed_scheme_count: {}, pack_sha256: {actual_pack_hash:?} }};",
        aliases.len(),
        schemes.len(),
        active.len(),
        shadowed.len()
    )?;

    let output_directory =
        PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| invalid_data("OUT_DIR is missing"))?);
    fs::write(output_directory.join("color_schemes.bin"), compiled)?;
    fs::write(output_directory.join("color_scheme_index.rs"), generated)?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        panic!("failed to compile built-in color scheme assets: {error}");
    }
}
