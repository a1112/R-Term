//! Generated, allocation-free lookup for bundled terminal color schemes.

/// Provenance and cardinality information validated by the build script.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeManifest {
    pub format_version: usize,
    pub lookup_count: usize,
    pub scheme_count: usize,
    pub active_scheme_count: usize,
    pub shadowed_scheme_count: usize,
    pub pack_sha256: &'static str,
}

static CONTENTS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/color_schemes.bin"));

include!(concat!(env!("OUT_DIR"), "/color_scheme_index.rs"));

/// Returns the TOML for a built-in scheme or compatibility alias.
///
/// The sorted static index and borrowed byte bundle keep the lookup free of
/// allocation. Parsing is deliberately left to the caller that needs it.
#[must_use]
pub fn get(name: &str) -> Option<&'static str> {
    let index = INDEX
        .binary_search_by(|(candidate, _, _)| candidate.cmp(&name))
        .ok()?;
    let (_, start, length) = INDEX[index];
    let start = usize::try_from(start).ok()?;
    let length = usize::try_from(length).ok()?;
    let end = start.checked_add(length)?;
    let bytes = CONTENTS.get(start..end)?;
    std::str::from_utf8(bytes).ok()
}

/// Iterates over every canonical name and compatibility alias in sorted order.
#[must_use]
pub fn names() -> impl ExactSizeIterator<Item = &'static str> {
    INDEX.iter().map(|(name, _, _)| *name)
}
