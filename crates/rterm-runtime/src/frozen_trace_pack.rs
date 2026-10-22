use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    io::Read as _,
    ops::Range,
    sync::OnceLock,
};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

const TRACE_PACK_GZIP: &[u8] = include_bytes!("../tests/fixtures/task10_legacy_trace_pack.gz");
const FIXTURE_RECORDS: &str = include_str!("../tests/fixtures/task10_legacy_fixture_records.txt");
const PACK_MAGIC: &[u8] = b"rssh.task10.trace-pack/v1\n";
const TRACE_COUNT: usize = 356;
const PACK_GZIP_LEN: usize = 949_468;
const PACK_GZIP_SHA256: &str = "d717ebfe675c181e967edcf00b53790bf771e310be0d29eed0cd1216a05f23b2";
const PACK_LEN: usize = 26_053_343;
const PACK_SHA256: &str = "0a171d6524e6fda679fa3a57693449a542c698fae1a268a87ee85668cf9007f2";

#[derive(Clone, Copy)]
struct SourcePolicy {
    source_path: &'static str,
    baseline_blob: &'static str,
    current_path: &'static str,
    domain: &'static str,
}

const SOURCE_POLICIES: &[SourcePolicy] = &[
    SourcePolicy {
        source_path: "crates/rssh-terminal/src/parser.rs",
        baseline_blob: "4d524b92c93ad6f61a7fb828a0a3cced499ffb45",
        current_path: "crates/rssh-terminal/src/parser.rs",
        domain: "terminal_parser",
    },
    SourcePolicy {
        source_path: "crates/rssh-app/src/terminal_runtime.rs",
        baseline_blob: "68b255e1a8c6427e4fe2dbebe2a37c0a171d9a72",
        current_path: "crates/rssh-runtime/src/terminal.rs",
        domain: "runtime",
    },
    SourcePolicy {
        source_path: "crates/rssh-app/src/terminal_queries.rs",
        baseline_blob: "91ff5524a7acf2ce59c674926dc22600c61a4547",
        current_path: "crates/rssh-runtime/src/queries.rs",
        domain: "query",
    },
    SourcePolicy {
        source_path: "crates/rssh-app/src/terminal_query_dcs.rs",
        baseline_blob: "019161c7a655aaf27c64b3896942cb3731ebbc05",
        current_path: "crates/rssh-runtime/src/query_dcs.rs",
        domain: "dcs",
    },
];

#[derive(Clone, Copy)]
struct ApprovedMigration {
    row_id: &'static str,
    source_path: &'static str,
    test_name: &'static str,
    baseline_body: &'static str,
    current_path: &'static str,
    current_test_name: &'static str,
    current_body: &'static str,
    trace_digest: &'static str,
    reason: &'static str,
}

const APPROVED_MIGRATIONS: &[ApprovedMigration] = &[
    ApprovedMigration {
        row_id: "e56dc7488cc895ae7c43ab7cb8cdccdaafa6122d1dd71e853263248fabf0c883",
        source_path: "crates/rssh-app/src/terminal_runtime.rs",
        test_name: "delays_synchronized_output_damage_until_mode_resets",
        baseline_body: "c473458b8fcb1b5d39ad0670f5597a810005e125f6f5576e310acca2ca91ed57",
        current_path: "crates/rssh-runtime/src/terminal.rs",
        current_test_name: "delays_synchronized_output_damage_until_mode_resets",
        current_body: "1123bd8febe85c9b5028f755579999fede46840af1c07ff2da87efe4aca55128",
        trace_digest: "a4c233800b94c6b3b3ac9b71b70599b6570c6d26273bd7ef925a3017b882931e",
        reason: "approved:neutral-type-boundary",
    },
    ApprovedMigration {
        row_id: "cf29891657988fa0319349f69fd680e8daa0e7902dbcc3a732cba9b0e6180995",
        source_path: "crates/rssh-app/src/terminal_runtime.rs",
        test_name: "reports_damage_regions_from_terminal_feed",
        baseline_body: "3e118865c6942548fca5e6e0c7f36ea99d29c98c01291742fb81f182116da472",
        current_path: "crates/rssh-runtime/src/terminal.rs",
        current_test_name: "reports_damage_regions_from_terminal_feed",
        current_body: "61b58c91a61f886c3a18ba7b1dcf6d7e21feef8a82ebb689eb2441555f924127",
        trace_digest: "1b3794e9db22142aac80140a873bd8dbebd8aeeaaa1acfbd5e0016fbadb67baa",
        reason: "approved:neutral-type-boundary",
    },
    ApprovedMigration {
        row_id: "927c661ab69f79fee32f07fcb131363e5aba0ed0b9b4f10dcb3b2c6b3849918c",
        source_path: "crates/rssh-app/src/terminal_queries.rs",
        test_name: "terminal_queries_rejects_malformed_mode_sequences_and_reserves_clipboard_controls",
        baseline_body: "2c474a0f909247eaa115aa18632ce2dbcc852ae99fbb3cca8579c31c00d3bcb9",
        current_path: "crates/rssh-runtime/src/queries.rs",
        current_test_name: "terminal_queries_rejects_malformed_mode_sequences_and_reserves_clipboard_controls",
        current_body: "626aafad9838f43ab31d38ddb166212f7051e2532aeaa89505b64be6aea9a193",
        trace_digest: "dbd6e48db30d764a62f9c507faed9699a2bf5f3a3cd8cc40cba9d831668bd746",
        reason: "approved:osc52-selection-preservation",
    },
    ApprovedMigration {
        row_id: "076c1295a970e62b8b54cb46a6c86b6e84a32e3edf3bd0f97d2e102380bd1d86",
        source_path: "crates/rssh-app/src/terminal_queries.rs",
        test_name: "terminal_queries_fail_closed_on_oversized_reserved_queries_and_recovers",
        baseline_body: "f6417ce563a6f7987631588fa265f5872fab037c7d1748c539d14e4116f32664",
        current_path: "crates/rssh-runtime/src/queries.rs",
        current_test_name: "terminal_queries_fail_closed_on_oversized_reserved_queries_and_recovers",
        current_body: "559a7e2601bee36f030b2fb46bb7f8ac22f4e7e7a987f6372e179f0b458bd2bb",
        trace_digest: "6db4d39b119d57103401ae337b5fc87a8093ab3664e007d023feea97ef59b092",
        reason: "approved:module-path-migration",
    },
    ApprovedMigration {
        row_id: "4021addb5221a268551ec9d31fd0794aa2442a085d8d78ba4a941614230adf6b",
        source_path: "crates/rssh-terminal/src/parser.rs",
        test_name: "row_rotation_preserves_wrapped_overflow_and_seqno",
        baseline_body: "9d7a9e1f39557c9eb99fb9904a671def1d8abdaeeb3b0965caaf253c7d419464",
        current_path: "crates/rssh-terminal/src/parser.rs",
        current_test_name: "row_rotation_preserves_wrapped_overflow_and_seqno",
        current_body: "84f71e3351a8a75a9945906d280c0aeb4938711969494889729316e88ead3f62",
        trace_digest: "64c53a1ab1cab6115e3ff5ada3bd445a45ae1a3b2019733e02953f59f278f7b5",
        reason: "approved:hyperlink-string-to-arc",
    },
    ApprovedMigration {
        row_id: "86c620e781eb19fa20180c5f88805b2bd2f1005c05159ef154184ff0160b2f9a",
        source_path: "crates/rssh-terminal/src/parser.rs",
        test_name: "row_to_history_moves_cells_without_duplicate_clone",
        baseline_body: "05ff8d70528593a9cd308fa803faa3638dc948b3a19709c36243ff0a97d489cd",
        current_path: "crates/rssh-terminal/src/parser.rs",
        current_test_name: "row_to_history_moves_cells_without_duplicate_clone",
        current_body: "8bc580c582790b445b996f14811ac20713d21a2ca401fdfd71f4193138c327bf",
        trace_digest: "626a2563073acfce1f049cda62019fb58a0917f8085a6c28a7d1fdf7dbf6e201",
        reason: "approved:hyperlink-string-to-arc",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FixtureRecord {
    pub(crate) row_id: &'static str,
    pub(crate) behavior_id: &'static str,
    pub(crate) source_path: &'static str,
    pub(crate) test_name: &'static str,
    pub(crate) baseline_blob: &'static str,
    pub(crate) baseline_body_sha256: &'static str,
    pub(crate) current_path: &'static str,
    pub(crate) current_test_name: &'static str,
    pub(crate) current_body_sha256: &'static str,
    pub(crate) migration: &'static str,
    pub(crate) domain: &'static str,
    pub(crate) trace_sha256: &'static str,
    pub(crate) trace_ref: &'static str,
}

#[derive(Debug)]
struct TraceEntry {
    digest: String,
    bytes: Range<usize>,
}

#[derive(Debug)]
struct FrozenEvidence {
    records: Vec<FixtureRecord>,
    pack: Vec<u8>,
    traces: HashMap<String, TraceEntry>,
}

static EVIDENCE: OnceLock<FrozenEvidence> = OnceLock::new();

pub(crate) fn records() -> &'static [FixtureRecord] {
    &evidence().records
}

pub(crate) fn trace(record: &FixtureRecord) -> &'static [u8] {
    let evidence = evidence();
    let entry = evidence
        .traces
        .get(record.row_id)
        .unwrap_or_else(|| panic!("missing frozen trace for {}", record.row_id));
    assert_eq!(
        entry.digest, record.trace_sha256,
        "{} trace digest",
        record.row_id
    );
    &evidence.pack[entry.bytes.clone()]
}

pub(crate) fn assert_current_trace(record: &FixtureRecord, current: &[u8]) {
    let frozen = trace(record);
    if frozen == current {
        return;
    }
    let frozen_digest = sha256_hex(frozen);
    let current_digest = sha256_hex(current);
    let frozen_text = String::from_utf8_lossy(frozen);
    let current_text = String::from_utf8_lossy(current);
    let mut difference = None;
    for (index, (left, right)) in frozen_text.lines().zip(current_text.lines()).enumerate() {
        if left != right {
            difference = Some((index + 1, left.to_owned(), right.to_owned()));
            break;
        }
    }
    if difference.is_none() {
        let shared = frozen_text
            .lines()
            .count()
            .min(current_text.lines().count());
        difference = Some((
            shared + 1,
            frozen_text
                .lines()
                .nth(shared)
                .unwrap_or("<end>")
                .to_owned(),
            current_text
                .lines()
                .nth(shared)
                .unwrap_or("<end>")
                .to_owned(),
        ));
    }
    let (line, frozen_line, current_line) = difference.expect("trace difference");
    panic!(
        "{}::{} ({}) diverged from detached c69 trace at canonical line {line}\n\
         frozen sha256={frozen_digest}\ncurrent sha256={current_digest}\n\
         frozen: {frozen_line}\ncurrent: {current_line}",
        record.current_path, record.current_test_name, record.row_id
    );
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("write SHA-256 to String");
    }
    encoded
}

fn evidence() -> &'static FrozenEvidence {
    EVIDENCE.get_or_init(load_evidence)
}

fn load_evidence() -> FrozenEvidence {
    assert_eq!(
        TRACE_PACK_GZIP.len(),
        PACK_GZIP_LEN,
        "compressed pack length"
    );
    assert_eq!(
        sha256_hex(TRACE_PACK_GZIP),
        PACK_GZIP_SHA256,
        "compressed pack digest"
    );
    assert_eq!(&TRACE_PACK_GZIP[..3], b"\x1f\x8b\x08", "gzip header");
    assert_eq!(
        &TRACE_PACK_GZIP[4..8],
        &[0, 0, 0, 0],
        "gzip mtime must be zero"
    );

    let mut pack = Vec::with_capacity(PACK_LEN);
    GzDecoder::new(TRACE_PACK_GZIP)
        .read_to_end(&mut pack)
        .expect("decompress frozen Task 10 trace pack");
    assert_eq!(pack.len(), PACK_LEN, "uncompressed pack length");
    assert_eq!(sha256_hex(&pack), PACK_SHA256, "uncompressed pack digest");

    let traces = parse_pack(&pack);
    let records = parse_records();
    assert_eq!(records.len(), TRACE_COUNT, "frozen fixture record count");
    assert_eq!(traces.len(), TRACE_COUNT, "frozen trace count");

    let mut referenced = HashSet::with_capacity(TRACE_COUNT);
    for record in &records {
        let expected_ref = format!("pack:{}", record.trace_sha256);
        assert_eq!(
            record.trace_ref, expected_ref,
            "{} trace reference",
            record.row_id
        );
        let entry = traces
            .get(record.row_id)
            .unwrap_or_else(|| panic!("record has no trace: {}", record.row_id));
        assert_eq!(
            entry.digest, record.trace_sha256,
            "{} record digest",
            record.row_id
        );
        let trace_text =
            std::str::from_utf8(&pack[entry.bytes.clone()]).expect("canonical trace UTF-8");
        assert_eq!(
            trace_text.lines().nth(2),
            Some(format!("domain={}", record.domain).as_str()),
            "{} trace domain",
            record.row_id
        );
        assert!(
            referenced.insert(record.row_id),
            "duplicate record row {}",
            record.row_id
        );
    }
    assert_eq!(referenced.len(), traces.len(), "unreferenced trace in pack");

    FrozenEvidence {
        records,
        pack,
        traces,
    }
}

fn parse_pack(pack: &[u8]) -> HashMap<String, TraceEntry> {
    assert!(pack.starts_with(PACK_MAGIC), "trace pack magic");
    let mut traces = HashMap::with_capacity(TRACE_COUNT);
    let mut digests = HashSet::with_capacity(TRACE_COUNT);
    let mut cursor = PACK_MAGIC.len();
    let mut previous_end = cursor;
    let mut previous_row = None::<String>;
    while cursor < pack.len() {
        let header_end = pack[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| cursor + offset)
            .expect("trace pack header terminator");
        assert!(header_end >= previous_end, "overlapping trace header");
        let header =
            std::str::from_utf8(&pack[cursor..header_end]).expect("trace pack header UTF-8");
        let fields = header.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 4, "trace pack header fields");
        assert_eq!(fields[0], "trace", "trace pack entry tag");
        assert_sha256(fields[1], "trace row id");
        assert_sha256(fields[2], "trace digest");
        if let Some(previous) = previous_row.as_deref() {
            assert!(previous < fields[1], "trace pack row order");
        }
        previous_row = Some(fields[1].to_owned());
        let length = fields[3].parse::<usize>().expect("trace pack entry length");
        assert!(length > 0, "zero-length frozen trace");
        let start = header_end + 1;
        let end = start
            .checked_add(length)
            .expect("trace pack entry overflow");
        assert!(end < pack.len(), "truncated frozen trace");
        assert_eq!(pack[end], b'\n', "trace pack entry terminator");
        assert!(start >= previous_end, "overlapping trace payload");
        let trace = &pack[start..end];
        assert_eq!(
            sha256_hex(trace),
            fields[2],
            "{} trace payload digest",
            fields[1]
        );
        assert_trace_shape(fields[1], trace);
        assert!(
            digests.insert(fields[2].to_owned()),
            "duplicate trace digest {}",
            fields[2]
        );
        assert!(
            traces
                .insert(
                    fields[1].to_owned(),
                    TraceEntry {
                        digest: fields[2].to_owned(),
                        bytes: start..end,
                    },
                )
                .is_none(),
            "duplicate trace row {}",
            fields[1]
        );
        cursor = end + 1;
        previous_end = cursor;
    }
    assert_eq!(cursor, pack.len(), "trailing bytes in trace pack");
    assert_eq!(traces.len(), TRACE_COUNT, "trace pack entry count");
    traces
}

fn assert_trace_shape(row_id: &str, trace: &[u8]) {
    let text = std::str::from_utf8(trace).expect("canonical trace UTF-8");
    assert!(text.ends_with('\n'), "{row_id} canonical trace terminator");
    let lines = text.lines().collect::<Vec<_>>();
    assert!(lines.len() >= 9, "{row_id} canonical trace length");
    assert_eq!(lines[0], "schema=rssh.task10.canonical-trace/v1");
    assert_eq!(lines[1], format!("row_id={row_id}"));
    let domain = lines[2]
        .strip_prefix("domain=")
        .expect("canonical trace domain");
    assert!(
        matches!(
            domain,
            "runtime" | "runtime_filter" | "query" | "dcs" | "terminal_parser"
        ),
        "{row_id} unknown trace domain {domain}"
    );
    assert_eq!(lines[3], "init=content-addressed-state");
    let action_count = parse_canonical_usize(
        lines[4]
            .strip_prefix("action_count=")
            .expect("canonical trace action count"),
        "canonical trace action count",
    );
    assert!(action_count > 0, "{row_id} has no replayable action");
    let mut cursor = 5;
    let mut references = Vec::new();
    assert_trace_actions(row_id, &lines, &mut cursor, action_count, &mut references);
    assert_trace_finals(row_id, &lines, &mut cursor, &mut references);
    let blob_count = assert_trace_blobs(row_id, &lines[cursor..]);
    for reference in references {
        assert_trace_blob_reference(row_id, reference, blob_count);
    }
}

fn assert_trace_actions<'a>(
    row_id: &str,
    lines: &[&'a str],
    cursor: &mut usize,
    action_count: usize,
    references: &mut Vec<&'a str>,
) {
    const ACTION_KEYS: &[&str] = &[
        "action", "api", "config", "size", "state", "input", "chunk", "finish", "resize", "reset",
        "parent", "layer", "object",
    ];
    const OBSERVABLE_KEYS: &[&str] = &[
        "observables",
        "typed",
        "responses",
        "effects",
        "display",
        "visible",
        "damage",
        "metadata",
        "bells",
        "clipboard",
        "notifications",
        "diagnostics",
        "identity",
        "callbacks",
        "pending",
        "snapshot",
    ];
    for sequence in 0..action_count {
        let action_line = *lines
            .get(*cursor)
            .unwrap_or_else(|| panic!("{row_id} missing action {sequence}"));
        let observable_line = *lines
            .get(*cursor + 1)
            .unwrap_or_else(|| panic!("{row_id} missing observable {sequence}"));
        let action = parse_trace_fields(row_id, action_line, ACTION_KEYS);
        let observable = parse_trace_fields(row_id, observable_line, OBSERVABLE_KEYS);
        assert_eq!(action["action"], sequence.to_string());
        assert_eq!(observable["observables"], sequence.to_string());
        assert_eq!(action["config"], "explicit");
        assert_eq!(action["size"], "state");
        assert_eq!(action["chunk"], sequence.to_string());
        assert_eq!(action["parent"], "none");
        assert!(!action["api"].is_empty(), "{row_id} empty action API");
        assert_trace_layer(row_id, action["layer"]);
        assert!(
            parse_canonical_usize(action["object"], "trace object") > 0,
            "{row_id} zero trace object"
        );
        for key in ["finish", "resize", "reset"] {
            assert!(matches!(action[key], "0" | "1"), "{row_id} {key} flag");
        }
        references.extend([action["state"], action["input"]]);
        references.extend(OBSERVABLE_KEYS[1..].iter().map(|key| observable[*key]));
        *cursor += 2;
    }
}

fn assert_trace_finals<'a>(
    row_id: &str,
    lines: &[&'a str],
    cursor: &mut usize,
    references: &mut Vec<&'a str>,
) {
    while lines
        .get(*cursor)
        .is_some_and(|line| line.starts_with("final_object="))
    {
        let fields = parse_trace_delimited_fields(
            row_id,
            lines[*cursor],
            ';',
            &["final_object", "pending", "state", "snapshot"],
        );
        let (layer, object) = fields["final_object"]
            .split_once(':')
            .expect("canonical final object identity");
        assert_trace_layer(row_id, layer);
        assert!(
            parse_canonical_usize(object, "final object") > 0,
            "{row_id} zero final object"
        );
        references.extend([fields["pending"], fields["state"], fields["snapshot"]]);
        *cursor += 1;
    }
    for key in ["final_pending", "final_state", "final_snapshot"] {
        let line = lines
            .get(*cursor)
            .unwrap_or_else(|| panic!("{row_id} missing {key}"));
        let value = line
            .strip_prefix(&format!("{key}="))
            .unwrap_or_else(|| panic!("{row_id} expected {key}"));
        assert!(!value.contains('='), "{row_id} malformed {key}");
        references.push(value);
        *cursor += 1;
    }
}

fn assert_trace_blobs(row_id: &str, lines: &[&str]) -> usize {
    const BLOB_KEYS: &[&str] = &["blob", "kind", "len", "sha256", "bytes"];
    for (index, line) in lines.iter().enumerate() {
        let fields = parse_trace_fields(row_id, line, BLOB_KEYS);
        assert_eq!(fields["blob"], index.to_string(), "{row_id} blob index");
        assert!(
            matches!(
                fields["kind"],
                "arguments"
                    | "ordered-observables"
                    | "pre-state"
                    | "post-state"
                    | "final-pending"
                    | "final-state"
                    | "final-snapshot"
            ),
            "{row_id} unknown blob kind {}",
            fields["kind"]
        );
        assert_sha256(fields["sha256"], "trace blob digest");
        let bytes = decode_canonical_hex(fields["bytes"], "trace blob bytes");
        assert_eq!(
            bytes.len(),
            parse_canonical_usize(fields["len"], "trace blob length"),
            "{row_id} blob length"
        );
        assert_eq!(sha256_hex(&bytes), fields["sha256"], "{row_id} blob digest");
    }
    lines.len()
}

fn parse_trace_fields<'a>(
    row_id: &str,
    line: &'a str,
    expected_keys: &[&str],
) -> HashMap<&'a str, &'a str> {
    parse_trace_delimited_fields(row_id, line, '|', expected_keys)
}

fn parse_trace_delimited_fields<'a>(
    row_id: &str,
    line: &'a str,
    delimiter: char,
    expected_keys: &[&str],
) -> HashMap<&'a str, &'a str> {
    let mut fields = HashMap::with_capacity(expected_keys.len());
    for part in line.split(delimiter) {
        let (key, value) = part
            .split_once('=')
            .unwrap_or_else(|| panic!("{row_id} malformed trace field"));
        assert!(!key.is_empty(), "{row_id} empty trace field name");
        assert!(
            fields.insert(key, value).is_none(),
            "{row_id} duplicate trace field {key}"
        );
    }
    assert_eq!(fields.len(), expected_keys.len(), "{row_id} trace fields");
    for key in expected_keys {
        assert!(
            fields.contains_key(key),
            "{row_id} missing trace field {key}"
        );
    }
    fields
}

fn assert_trace_blob_reference(row_id: &str, reference: &str, blob_count: usize) {
    if reference == "empty" {
        return;
    }
    let index = reference.strip_prefix("blob:").map_or_else(
        || panic!("{row_id} invalid blob reference {reference}"),
        |value| parse_canonical_usize(value, "trace blob reference"),
    );
    assert!(
        index < blob_count,
        "{row_id} dangling blob reference {reference}"
    );
}

fn assert_trace_layer(row_id: &str, layer: &str) {
    assert!(
        matches!(
            layer,
            "runtime" | "runtime-filter" | "query" | "dcs" | "terminal-parser"
        ),
        "{row_id} unknown trace layer {layer}"
    );
}

fn parse_canonical_usize(value: &str, label: &str) -> usize {
    let parsed = value
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("invalid {label}"));
    assert_eq!(value, parsed.to_string(), "non-canonical {label}");
    parsed
}

fn decode_canonical_hex(value: &str, label: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "odd-length {label}");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|digits| {
            let high = canonical_hex_nibble(digits[0], label);
            let low = canonical_hex_nibble(digits[1], label);
            (high << 4) | low
        })
        .collect()
}

fn canonical_hex_nibble(value: u8, label: &str) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        _ => panic!("non-canonical {label}"),
    }
}

fn parse_records() -> Vec<FixtureRecord> {
    for required in [
        "# schema=rssh.task10.fixture-record/v1",
        "# baseline_sha=c69d52537cd893e615fded6ed46c2e59f1d2024e",
        "# columns=record|row_id|behavior_id|source_path|test_name|baseline_blob|baseline_body_sha256|current_path|current_test_name|current_body_sha256|migration|domain|trace_sha256|trace_ref",
        "# row_id=sha256(source_path NUL test_name NUL baseline_body_sha256)",
    ] {
        assert!(FIXTURE_RECORDS.lines().any(|line| line == required));
    }
    let mut rows = HashSet::with_capacity(TRACE_COUNT);
    let mut tests = HashSet::with_capacity(TRACE_COUNT);
    let mut records = Vec::with_capacity(TRACE_COUNT);
    for (index, line) in FIXTURE_RECORDS.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 14, "fixture record line {}", index + 1);
        assert_eq!(fields[0], "record", "fixture record line {}", index + 1);
        let record = FixtureRecord {
            row_id: fields[1],
            behavior_id: fields[2],
            source_path: fields[3],
            test_name: fields[4],
            baseline_blob: fields[5],
            baseline_body_sha256: fields[6],
            current_path: fields[7],
            current_test_name: fields[8],
            current_body_sha256: fields[9],
            migration: fields[10],
            domain: fields[11],
            trace_sha256: fields[12],
            trace_ref: fields[13],
        };
        assert_sha256(record.row_id, "fixture row id");
        assert_sha256(record.baseline_body_sha256, "baseline body");
        assert_sha256(record.current_body_sha256, "current body");
        assert_sha256(record.trace_sha256, "trace digest");
        assert_eq!(record.baseline_blob.len(), 40, "baseline Git blob");
        assert!(record.baseline_blob.bytes().all(is_lower_hex));
        let mut identity = Vec::new();
        identity.extend_from_slice(record.source_path.as_bytes());
        identity.push(0);
        identity.extend_from_slice(record.test_name.as_bytes());
        identity.push(0);
        identity.extend_from_slice(record.baseline_body_sha256.as_bytes());
        assert_eq!(record.row_id, sha256_hex(&identity), "fixture row identity");
        assert!(
            rows.insert(record.row_id),
            "duplicate fixture row {}",
            record.row_id
        );
        assert!(
            tests.insert((record.source_path, record.test_name)),
            "duplicate fixture test {}::{}",
            record.source_path,
            record.test_name
        );
        assert_record_policy(&record);
        records.push(record);
    }
    assert_record_set_policy(&records);
    records
}

fn assert_record_policy(record: &FixtureRecord) {
    let source = SOURCE_POLICIES
        .iter()
        .find(|source| source.source_path == record.source_path)
        .unwrap_or_else(|| panic!("unknown fixture source {}", record.source_path));
    assert_eq!(
        record.baseline_blob, source.baseline_blob,
        "{} baseline source blob",
        record.test_name
    );
    assert_eq!(
        record.current_path, source.current_path,
        "{} current source path",
        record.test_name
    );
    assert_eq!(
        record.current_test_name, record.test_name,
        "{} current test mapping",
        record.test_name
    );
    let expected_domain = if record.test_name
        == "gui_filter_passes_malformed_modes_and_fail_closes_reserved_clipboard"
    {
        "runtime_filter"
    } else {
        source.domain
    };
    assert_eq!(
        record.domain, expected_domain,
        "{} domain",
        record.test_name
    );
    assert_eq!(
        record.trace_ref,
        format!("pack:{}", record.trace_sha256),
        "{} trace reference",
        record.test_name
    );

    let approved = APPROVED_MIGRATIONS
        .iter()
        .find(|approved| approved.row_id == record.row_id);
    if record.migration == "exact" {
        assert!(
            approved.is_none(),
            "{} lost approved migration",
            record.test_name
        );
        assert_eq!(
            record.current_body_sha256, record.baseline_body_sha256,
            "{} exact body",
            record.test_name
        );
        return;
    }
    let approved = approved.unwrap_or_else(|| {
        panic!(
            "{} uses unapproved migration {}",
            record.test_name, record.migration
        )
    });
    assert_eq!(record.source_path, approved.source_path);
    assert_eq!(record.test_name, approved.test_name);
    assert_eq!(record.baseline_body_sha256, approved.baseline_body);
    assert_eq!(record.current_path, approved.current_path);
    assert_eq!(record.current_test_name, approved.current_test_name);
    assert_eq!(record.current_body_sha256, approved.current_body);
    assert_eq!(record.trace_sha256, approved.trace_digest);
    assert_eq!(record.migration, approved.reason);
}

fn assert_record_set_policy(records: &[FixtureRecord]) {
    let mut current_pairs = HashSet::with_capacity(records.len());
    for record in records {
        assert!(
            current_pairs.insert((record.current_path, record.current_test_name)),
            "duplicate current fixture {}::{}",
            record.current_path,
            record.current_test_name
        );
    }
    if records.len() == TRACE_COUNT {
        assert_eq!(
            records
                .iter()
                .filter(|record| record.migration.starts_with("approved:"))
                .count(),
            APPROVED_MIGRATIONS.len(),
            "approved migration count"
        );
    }
}

fn assert_sha256(value: &str, label: &str) {
    assert_eq!(value.len(), 64, "{label} must be SHA-256");
    assert!(
        value.bytes().all(is_lower_hex),
        "{label} must be lowercase hexadecimal"
    );
}

fn is_lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_panics(run: impl FnOnce() + std::panic::UnwindSafe) {
        assert!(std::panic::catch_unwind(run).is_err());
    }

    #[test]
    fn record_policy_rejects_unknown_current_path() {
        let mut record = records()[0];
        record.current_path = "unknown/current.rs";

        assert_panics(|| assert_record_policy(&record));
    }

    #[test]
    fn record_set_policy_rejects_duplicate_current_pair() {
        let first = records()[0];
        let mut second = records()[1];
        second.current_path = first.current_path;
        second.current_test_name = first.current_test_name;

        assert_panics(|| assert_record_set_policy(&[first, second]));
    }

    #[test]
    fn approved_policy_rejects_body_change_with_the_same_label() {
        let mut record = records()
            .iter()
            .copied()
            .find(|record| record.migration.starts_with("approved:"))
            .expect("approved fixture record");
        record.current_body_sha256 =
            "0000000000000000000000000000000000000000000000000000000000000000";

        assert_panics(|| assert_record_policy(&record));
    }

    #[test]
    fn trace_schema_rejects_mismatched_observable_sequence() {
        let record = records()[0];
        let frozen = std::str::from_utf8(trace(&record)).expect("frozen trace UTF-8");
        let mutated = frozen.replacen("observables=0|", "observables=9|", 1);
        assert_ne!(mutated, frozen);

        assert_panics(|| assert_trace_shape(record.row_id, mutated.as_bytes()));
    }

    #[test]
    fn trace_schema_rejects_dangling_blob_reference() {
        let record = records()[0];
        let frozen = std::str::from_utf8(trace(&record)).expect("frozen trace UTF-8");
        let input = frozen
            .lines()
            .find_map(|line| {
                line.starts_with("action=")
                    .then(|| line.split('|').find(|field| field.starts_with("input=")))
                    .flatten()
            })
            .expect("action input field");
        let mutated = frozen.replacen(input, "input=blob:999999", 1);

        assert_panics(|| assert_trace_shape(record.row_id, mutated.as_bytes()));
    }
}
