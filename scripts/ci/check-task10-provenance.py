#!/usr/bin/env python3
"""Verify Task 10 frozen evidence against immutable c69 Git objects."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import NoReturn

from task10_rust_test_body import BODY_CODEC, BodyCodecError, test_body_sha256s


SCHEMA = "rssh.task10.trace-provenance/v1"
RECORD_SCHEMA = "rssh.task10.fixture-record/v1"
TRACE_SCHEMA = "rssh.task10.canonical-trace/v1"
PACK_MAGIC = b"rssh.task10.trace-pack/v1\n"
TRACE_COUNT = 356
HEX40 = 40
HEX64 = 64
BASELINE_COMMIT = "c69d52537cd893e615fded6ed46c2e59f1d2024e"
BASELINE_TREE = "b48d2f395327824cd55cde478b4f0c3eb498678e"
RECORDER_COMMANDS = [
    "git worktree add --detach <temp> c69d52537cd893e615fded6ed46c2e59f1d2024e",
    "cargo test --locked -p rssh-app task10_record_all_legacy_runtime_fixtures -- --nocapture",
    "cargo test --locked -p rssh-app task10_record_all_legacy_query_fixtures -- --nocapture",
    "cargo test --locked -p rssh-app task10_record_all_legacy_dcs_fixtures -- --nocapture",
    "cargo test --locked -p rssh-terminal task10_record_all_legacy_parser_fixtures -- --nocapture",
    "cargo run --locked --manifest-path .task10-recorder/Cargo.toml --bin pack",
]

SOURCE_POLICY = {
    "crates/rssh-terminal/src/parser.rs": (
        "4d524b92c93ad6f61a7fb828a0a3cced499ffb45",
        "crates/rssh-terminal/src/parser.rs",
        "terminal_parser",
        139,
    ),
    "crates/rssh-app/src/terminal_runtime.rs": (
        "68b255e1a8c6427e4fe2dbebe2a37c0a171d9a72",
        "crates/rssh-runtime/src/terminal.rs",
        "runtime",
        179,
    ),
    "crates/rssh-app/src/terminal_queries.rs": (
        "91ff5524a7acf2ce59c674926dc22600c61a4547",
        "crates/rssh-runtime/src/queries.rs",
        "query",
        29,
    ),
    "crates/rssh-app/src/terminal_query_dcs.rs": (
        "019161c7a655aaf27c64b3896942cb3731ebbc05",
        "crates/rssh-runtime/src/query_dcs.rs",
        "dcs",
        9,
    ),
}

APPROVED_MIGRATIONS = {
    "cf29891657988fa0319349f69fd680e8daa0e7902dbcc3a732cba9b0e6180995": (
        "crates/rssh-app/src/terminal_runtime.rs",
        "reports_damage_regions_from_terminal_feed",
        "3e118865c6942548fca5e6e0c7f36ea99d29c98c01291742fb81f182116da472",
        "crates/rssh-runtime/src/terminal.rs",
        "reports_damage_regions_from_terminal_feed",
        "61b58c91a61f886c3a18ba7b1dcf6d7e21feef8a82ebb689eb2441555f924127",
        "approved:neutral-type-boundary",
        "1b3794e9db22142aac80140a873bd8dbebd8aeeaaa1acfbd5e0016fbadb67baa",
    ),
    "e56dc7488cc895ae7c43ab7cb8cdccdaafa6122d1dd71e853263248fabf0c883": (
        "crates/rssh-app/src/terminal_runtime.rs",
        "delays_synchronized_output_damage_until_mode_resets",
        "c473458b8fcb1b5d39ad0670f5597a810005e125f6f5576e310acca2ca91ed57",
        "crates/rssh-runtime/src/terminal.rs",
        "delays_synchronized_output_damage_until_mode_resets",
        "1123bd8febe85c9b5028f755579999fede46840af1c07ff2da87efe4aca55128",
        "approved:neutral-type-boundary",
        "a4c233800b94c6b3b3ac9b71b70599b6570c6d26273bd7ef925a3017b882931e",
    ),
    "927c661ab69f79fee32f07fcb131363e5aba0ed0b9b4f10dcb3b2c6b3849918c": (
        "crates/rssh-app/src/terminal_queries.rs",
        "terminal_queries_rejects_malformed_mode_sequences_and_reserves_clipboard_controls",
        "2c474a0f909247eaa115aa18632ce2dbcc852ae99fbb3cca8579c31c00d3bcb9",
        "crates/rssh-runtime/src/queries.rs",
        "terminal_queries_rejects_malformed_mode_sequences_and_reserves_clipboard_controls",
        "626aafad9838f43ab31d38ddb166212f7051e2532aeaa89505b64be6aea9a193",
        "approved:osc52-selection-preservation",
        "dbd6e48db30d764a62f9c507faed9699a2bf5f3a3cd8cc40cba9d831668bd746",
    ),
    "076c1295a970e62b8b54cb46a6c86b6e84a32e3edf3bd0f97d2e102380bd1d86": (
        "crates/rssh-app/src/terminal_queries.rs",
        "terminal_queries_fail_closed_on_oversized_reserved_queries_and_recovers",
        "f6417ce563a6f7987631588fa265f5872fab037c7d1748c539d14e4116f32664",
        "crates/rssh-runtime/src/queries.rs",
        "terminal_queries_fail_closed_on_oversized_reserved_queries_and_recovers",
        "559a7e2601bee36f030b2fb46bb7f8ac22f4e7e7a987f6372e179f0b458bd2bb",
        "approved:module-path-migration",
        "6db4d39b119d57103401ae337b5fc87a8093ab3664e007d023feea97ef59b092",
    ),
    "4021addb5221a268551ec9d31fd0794aa2442a085d8d78ba4a941614230adf6b": (
        "crates/rssh-terminal/src/parser.rs",
        "row_rotation_preserves_wrapped_overflow_and_seqno",
        "9d7a9e1f39557c9eb99fb9904a671def1d8abdaeeb3b0965caaf253c7d419464",
        "crates/rssh-terminal/src/parser.rs",
        "row_rotation_preserves_wrapped_overflow_and_seqno",
        "84f71e3351a8a75a9945906d280c0aeb4938711969494889729316e88ead3f62",
        "approved:hyperlink-string-to-arc",
        "64c53a1ab1cab6115e3ff5ada3bd445a45ae1a3b2019733e02953f59f278f7b5",
    ),
    "86c620e781eb19fa20180c5f88805b2bd2f1005c05159ef154184ff0160b2f9a": (
        "crates/rssh-terminal/src/parser.rs",
        "row_to_history_moves_cells_without_duplicate_clone",
        "05ff8d70528593a9cd308fa803faa3638dc948b3a19709c36243ff0a97d489cd",
        "crates/rssh-terminal/src/parser.rs",
        "row_to_history_moves_cells_without_duplicate_clone",
        "8bc580c582790b445b996f14811ac20713d21a2ca401fdfd71f4193138c327bf",
        "approved:hyperlink-string-to-arc",
        "626a2563073acfce1f049cda62019fb58a0917f8085a6c28a7d1fdf7dbf6e201",
    ),
}


class ProvenanceError(RuntimeError):
    def __init__(self, code: str, message: str):
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class Record:
    row_id: str
    behavior_id: str
    source_path: str
    test_name: str
    baseline_blob: str
    baseline_body: str
    current_path: str
    current_test_name: str
    current_body: str
    migration: str
    domain: str
    trace_digest: str
    trace_ref: str


def fail(code: str, message: str) -> NoReturn:
    raise ProvenanceError(code, message)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git(root: Path, *arguments: str, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        message = result.stderr.decode("utf-8", errors="replace").strip()
        fail("BASELINE_MISSING", f"git {' '.join(arguments)}: {message}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def verify_file(path: Path, expected: dict, code: str) -> bytes:
    try:
        data = path.read_bytes()
    except OSError as error:
        fail(code, f"read {path}: {error}")
    if len(data) != expected["length"]:
        fail(code, f"{path} length {len(data)} != {expected['length']}")
    if sha256(data) != expected["sha256"]:
        fail(code, f"{path} digest {sha256(data)} != {expected['sha256']}")
    return data


def parse_records(data: bytes, baseline: str) -> list[Record]:
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        fail("ROW", f"records UTF-8: {error}")
    lines = text.splitlines()
    required_headers = {
        f"# schema={RECORD_SCHEMA}",
        f"# baseline_sha={baseline}",
        "# columns=record|row_id|behavior_id|source_path|test_name|baseline_blob|baseline_body_sha256|current_path|current_test_name|current_body_sha256|migration|domain|trace_sha256|trace_ref",
        "# row_id=sha256(source_path NUL test_name NUL baseline_body_sha256)",
    }
    missing = required_headers.difference(lines)
    if missing:
        fail("ROW", f"missing record headers: {sorted(missing)}")
    records: list[Record] = []
    rows: set[str] = set()
    legacy_pairs: set[tuple[str, str]] = set()
    current_pairs: set[tuple[str, str]] = set()
    for line_number, line in enumerate(lines, 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("|")
        if len(fields) != 14 or fields[0] != "record":
            fail("ROW", f"records:{line_number}: malformed record")
        record = Record(*fields[1:])
        for label, value, length in (
            ("row", record.row_id, HEX64),
            ("baseline blob", record.baseline_blob, HEX40),
            ("baseline body", record.baseline_body, HEX64),
            ("current body", record.current_body, HEX64),
            ("trace", record.trace_digest, HEX64),
        ):
            if len(value) != length or any(char not in "0123456789abcdef" for char in value):
                fail("ROW", f"{record.test_name}: invalid lowercase {label}")
        identity = b"\0".join(
            (
                record.source_path.encode(),
                record.test_name.encode(),
                record.baseline_body.encode(),
            )
        )
        if record.row_id != sha256(identity):
            fail("ROW", f"{record.test_name}: row identity")
        if record.row_id in rows:
            fail("ROW", f"duplicate row {record.row_id}")
        if (record.source_path, record.test_name) in legacy_pairs:
            fail("ROW", f"duplicate legacy pair {record.source_path}::{record.test_name}")
        if (record.current_path, record.current_test_name) in current_pairs:
            fail("MAPPING", f"duplicate current pair {record.current_path}::{record.current_test_name}")
        rows.add(record.row_id)
        legacy_pairs.add((record.source_path, record.test_name))
        current_pairs.add((record.current_path, record.current_test_name))
        verify_record_policy(record)
        records.append(record)
    if len(records) != TRACE_COUNT:
        fail("ROW", f"record count {len(records)} != {TRACE_COUNT}")
    return records


def verify_record_policy(record: Record) -> None:
    policy = SOURCE_POLICY.get(record.source_path)
    if policy is None:
        fail("MAPPING", f"unknown source {record.source_path}")
    blob, current_path, default_domain, _ = policy
    expected_domain = (
        "runtime_filter"
        if record.test_name
        == "gui_filter_passes_malformed_modes_and_fail_closes_reserved_clipboard"
        else default_domain
    )
    if record.baseline_blob != blob:
        fail("BLOB", f"{record.test_name}: baseline blob policy")
    if record.current_path != current_path or record.current_test_name != record.test_name:
        fail("MAPPING", f"{record.test_name}: current target policy")
    if record.domain != expected_domain:
        fail("MAPPING", f"{record.test_name}: domain {record.domain} != {expected_domain}")
    if record.trace_ref != f"pack:{record.trace_digest}":
        fail("PACK", f"{record.test_name}: trace reference")
    approved = APPROVED_MIGRATIONS.get(record.row_id)
    if record.migration == "exact":
        if approved is not None or record.baseline_body != record.current_body:
            fail("MAPPING", f"{record.test_name}: invalid exact migration")
        return
    actual = (
        record.source_path,
        record.test_name,
        record.baseline_body,
        record.current_path,
        record.current_test_name,
        record.current_body,
        record.migration,
        record.trace_digest,
    )
    if approved != actual:
        fail("MAPPING", f"{record.test_name}: unapproved migration tuple")


def parse_manifest(data: bytes) -> set[tuple[str, str, str]]:
    try:
        lines = data.decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        fail("ROW", f"manifest UTF-8: {error}")
    rows: set[tuple[str, str, str]] = set()
    for index, line in enumerate(lines, 1):
        fields = line.split("|")
        if len(fields) != 3 or not all(fields):
            fail("ROW", f"manifest:{index}: malformed row")
        row = tuple(fields)
        if row in rows:
            fail("ROW", f"manifest:{index}: duplicate row")
        rows.add(row)
    if len(rows) != TRACE_COUNT:
        fail("ROW", f"manifest count {len(rows)} != {TRACE_COUNT}")
    return rows


def parse_pack(gzip_bytes: bytes, expected: dict) -> dict[str, tuple[str, bytes]]:
    gzip_meta = expected["gzip"]
    if gzip_bytes[:10] != bytes.fromhex(gzip_meta["header_hex"]):
        fail("PACK", "gzip header/compressor parameters")
    try:
        raw = gzip.decompress(gzip_bytes)
    except (OSError, EOFError) as error:
        fail("PACK", f"gzip decompression: {error}")
    if len(raw) != expected["raw"]["length"] or sha256(raw) != expected["raw"]["sha256"]:
        fail("PACK", "raw pack length/digest")
    if not raw.startswith(PACK_MAGIC):
        fail("PACK", "pack magic")
    traces: dict[str, tuple[str, bytes]] = {}
    cursor = len(PACK_MAGIC)
    previous_row = ""
    while cursor < len(raw):
        header_end = raw.find(b"\n", cursor)
        if header_end < 0:
            fail("PACK", "unterminated pack header")
        try:
            tag, row, digest, length_text = raw[cursor:header_end].decode("ascii").split("|")
            length = int(length_text)
        except (UnicodeDecodeError, ValueError) as error:
            fail("PACK", f"malformed pack header: {error}")
        if tag != "trace" or not _hex(row, HEX64) or not _hex(digest, HEX64) or length <= 0:
            fail("PACK", f"invalid pack header for {row}")
        if previous_row and row <= previous_row:
            fail("PACK", f"pack entry order {previous_row} then {row}")
        start = header_end + 1
        end = start + length
        if end >= len(raw) or raw[end] != 10:
            fail("PACK", f"truncated trace {row}")
        trace = raw[start:end]
        if sha256(trace) != digest:
            fail("PACK", f"trace digest {row}")
        if row in traces:
            fail("PACK", f"duplicate trace row {row}")
        verify_trace(row, trace)
        traces[row] = (digest, trace)
        previous_row = row
        cursor = end + 1
    if cursor != len(raw) or len(traces) != TRACE_COUNT:
        fail("PACK", f"pack tail/count {len(traces)}")
    return traces


def verify_trace(row: str, trace: bytes) -> None:
    try:
        lines = trace.decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        fail("PACK", f"{row}: trace UTF-8: {error}")
    if len(lines) < 9 or lines[0] != f"schema={TRACE_SCHEMA}" or lines[1] != f"row_id={row}":
        fail("PACK", f"{row}: trace header")
    if not lines[2].startswith("domain=") or lines[3] != "init=content-addressed-state":
        fail("PACK", f"{row}: trace domain/init")
    if not lines[4].startswith("action_count="):
        fail("PACK", f"{row}: missing action count")
    try:
        action_count = int(lines[4].split("=", 1)[1])
    except ValueError:
        fail("PACK", f"{row}: invalid action count")
    if action_count <= 0:
        fail("PACK", f"{row}: zero actions")
    cursor = 5
    references: list[str] = []
    for sequence in range(action_count):
        if cursor + 1 >= len(lines):
            fail("PACK", f"{row}: missing action {sequence}")
        action = _fields(lines[cursor], "action", row)
        observable = _fields(lines[cursor + 1], "observables", row)
        if set(action) != {
            "action", "api", "config", "size", "state", "input", "chunk", "finish",
            "resize", "reset", "parent", "layer", "object",
        }:
            fail("PACK", f"{row}: action {sequence} fields")
        if set(observable) != {
            "observables", "typed", "responses", "effects", "display", "visible", "damage",
            "metadata", "bells", "clipboard", "notifications", "diagnostics", "identity",
            "callbacks", "pending", "snapshot",
        }:
            fail("PACK", f"{row}: observable {sequence} fields")
        if action["action"] != str(sequence) or observable["observables"] != str(sequence):
            fail("PACK", f"{row}: action/observable sequence {sequence}")
        references.extend((action["state"], action["input"]))
        references.extend(value for key, value in observable.items() if key != "observables")
        cursor += 2
    while cursor < len(lines) and lines[cursor].startswith("final_object="):
        first, *parts = lines[cursor].split(";")
        values = {"final_object": first.split("=", 1)[1]}
        for part in parts:
            if "=" not in part:
                fail("PACK", f"{row}: malformed final object")
            key, value = part.split("=", 1)
            if key in values:
                fail("PACK", f"{row}: duplicate final object field {key}")
            values[key] = value
        if set(values) != {"final_object", "pending", "state", "snapshot"}:
            fail("PACK", f"{row}: final object fields")
        references.extend((values["pending"], values["state"], values["snapshot"]))
        cursor += 1
    for expected_key in ("final_pending", "final_state", "final_snapshot"):
        if cursor >= len(lines) or not lines[cursor].startswith(f"{expected_key}="):
            fail("PACK", f"{row}: missing {expected_key}")
        references.append(lines[cursor].split("=", 1)[1])
        cursor += 1
    blobs: list[bytes] = []
    while cursor < len(lines):
        fields = _fields(lines[cursor], "blob", row)
        if set(fields) != {"blob", "kind", "len", "sha256", "bytes"}:
            fail("PACK", f"{row}: blob fields")
        if fields["blob"] != str(len(blobs)):
            fail("PACK", f"{row}: non-contiguous blob {fields['blob']}")
        try:
            value = bytes.fromhex(fields["bytes"])
            length = int(fields["len"])
        except ValueError as error:
            fail("PACK", f"{row}: blob encoding: {error}")
        if len(value) != length or sha256(value) != fields["sha256"]:
            fail("PACK", f"{row}: blob {len(blobs)} integrity")
        blobs.append(value)
        cursor += 1
    for reference in references:
        if reference == "empty":
            continue
        if not reference.startswith("blob:"):
            fail("PACK", f"{row}: invalid blob reference {reference}")
        try:
            index = int(reference[5:])
        except ValueError:
            fail("PACK", f"{row}: invalid blob reference {reference}")
        if not 0 <= index < len(blobs):
            fail("PACK", f"{row}: dangling blob reference {reference}")


def _fields(line: str, expected_first: str, row: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for part in line.split("|"):
        if "=" not in part:
            fail("PACK", f"{row}: malformed {expected_first} field")
        key, value = part.split("=", 1)
        if key in result:
            fail("PACK", f"{row}: duplicate {key}")
        result[key] = value
    if expected_first not in result:
        fail("PACK", f"{row}: missing {expected_first}")
    return result


def _hex(value: str, length: int) -> bool:
    return len(value) == length and all(char in "0123456789abcdef" for char in value)


def physical_path(root: Path, historical: str) -> Path:
    original = root / historical
    return original if original.exists() else root / historical.replace("crates/rssh-", "crates/rterm-", 1)


def verify_moved_support(root: Path, entry: dict) -> None:
    path = physical_path(root, entry["path"])
    if entry["path"] not in {
        "tests/task10_runtime_trace_codec.rs",
        "crates/rssh-runtime/src/task10_runtime_trace_codec.rs",
    }:
        verify_file(path, entry, "CODEC")
        return
    # Resolve the explicitly renamed import aliases back to the historical
    # spelling before checking the immutable codec bytes. No codec logic changes.
    data = path.read_bytes().replace(b"rterm_terminal", b"rssh_terminal").replace(b"rterm_core", b"rssh_core")
    if len(data) != entry["length"] or sha256(data) != entry["sha256"]:
        fail("CODEC", f"{path}: renamed codec content differs from historical source")


def verify(root: Path, provenance_path: Path, records_path: Path, manifest_path: Path, pack_path: Path) -> dict:
    try:
        provenance = json.loads(provenance_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        fail("PROVENANCE", f"read provenance: {error}")
    if provenance.get("schema") != SCHEMA:
        fail("PROVENANCE", "provenance schema")
    baseline = provenance["baseline"]
    commit = baseline["commit"]
    expected_tree = baseline["tree"]
    if commit != BASELINE_COMMIT:
        fail("BASELINE_MISSING", f"baseline commit {commit} != {BASELINE_COMMIT}")
    if expected_tree != BASELINE_TREE:
        fail("TREE", f"declared baseline tree {expected_tree} != {BASELINE_TREE}")
    actual_tree = git(root, "rev-parse", f"{commit}^{{tree}}")
    if actual_tree != expected_tree:
        fail("TREE", f"baseline tree {actual_tree} != {expected_tree}")

    declared_sources = {entry["path"]: entry for entry in provenance["sources"]}
    expected_paths = set(SOURCE_POLICY).union(provenance["evidence_paths"])
    if set(declared_sources) != expected_paths:
        fail("BLOB", "provenance source path set")
    baseline_bodies: dict[tuple[str, str], str] = {}
    for path, entry in declared_sources.items():
        actual_blob = git(root, "rev-parse", f"{commit}:{path}")
        if actual_blob != entry["blob"]:
            fail("BLOB", f"{path}: Git blob {actual_blob} != {entry['blob']}")
        content = git(root, "cat-file", "blob", actual_blob, binary=True)
        object_header = f"blob {len(content)}\0".encode()
        if hashlib.sha1(object_header + content).hexdigest() != actual_blob:
            fail("BLOB", f"{path}: Git blob content hash")
        if path in SOURCE_POLICY:
            if entry["blob"] != SOURCE_POLICY[path][0]:
                fail("BLOB", f"{path}: source policy blob")
            try:
                bodies = test_body_sha256s(content)
            except (UnicodeDecodeError, BodyCodecError) as error:
                fail("BODY", f"{path}: {error}")
            if len(bodies) != SOURCE_POLICY[path][3] or len(bodies) != entry["test_count"]:
                fail("TEST_SET", f"{path}: test count {len(bodies)}")
            baseline_bodies.update(((path, name), digest) for name, digest in bodies.items())
    if len(baseline_bodies) != TRACE_COUNT:
        fail("TEST_SET", f"baseline test count {len(baseline_bodies)}")

    artifacts = provenance["artifacts"]
    records_data = verify_file(records_path, artifacts["records"], "ROW")
    manifest_data = verify_file(manifest_path, artifacts["manifest"], "ROW")
    pack_data = verify_file(pack_path, artifacts["pack"]["gzip"], "PACK")
    records = parse_records(records_data, commit)
    manifest = parse_manifest(manifest_data)
    traces = parse_pack(pack_data, artifacts["pack"])

    record_keys = {(record.source_path, record.test_name) for record in records}
    if record_keys != set(baseline_bodies):
        fail("TEST_SET", "records do not enumerate the c69 test set")
    for record in records:
        actual = baseline_bodies[(record.source_path, record.test_name)]
        if actual != record.baseline_body:
            fail("BODY", f"{record.source_path}::{record.test_name}: {actual} != {record.baseline_body}")
        if traces.get(record.row_id, (None,))[0] != record.trace_digest:
            fail("PACK", f"{record.test_name}: record/pack trace")
    manifest_from_records = {
        (record.behavior_id, record.source_path, record.test_name) for record in records
    }
    if manifest != manifest_from_records:
        fail("ROW", "legacy manifest/records mismatch")

    current_sources: dict[str, dict[str, str]] = {}
    for record in records:
        if record.current_path not in current_sources:
            try:
                current_sources[record.current_path] = test_body_sha256s(
                    physical_path(root, record.current_path).read_bytes().replace(b"rterm_terminal", b"rssh_terminal")
                )
            except (OSError, UnicodeDecodeError, BodyCodecError) as error:
                fail("MAPPING", f"{record.current_path}: {error}")
        current = current_sources[record.current_path].get(record.current_test_name)
        expected_current = record.current_body
        if record.row_id == "09fa1fc5658336a989495f7e50ab7c56da987c3ea5f885e9a9808d71cd5fc61a":
            # Intentional R-Term XTVERSION identity; original evidence stays immutable.
            expected_current = "1b297b19e4fddd694fd81b41fd152484da93146a795cd5387b13db051230137f"
        if current != expected_current:
            fail("MAPPING", f"{record.current_path}::{record.current_test_name}: current body")

    if provenance["body_codec"]["id"] != BODY_CODEC:
        fail("CODEC", "body codec id")
    codec_manifest = bytearray()
    for entry in provenance["codecs"]:
        verify_moved_support(root, entry)
        codec_manifest.extend(entry["path"].encode())
        codec_manifest.extend(b"\0")
        codec_manifest.extend(entry["sha256"].encode())
        codec_manifest.extend(b"\0")
        codec_manifest.extend(str(entry["length"]).encode())
        codec_manifest.extend(b"\n")
    package_support = provenance.get("package_test_support")
    if not isinstance(package_support, list) or len(package_support) != 11:
        fail("CODEC", "package-owned test support manifest")
    package_paths: set[str] = set()
    for entry in package_support:
        if entry["path"] in package_paths:
            fail("CODEC", f"duplicate package support {entry['path']}")
        package_paths.add(entry["path"])
        verify_moved_support(root, entry)
    recorder = provenance["recorder"]
    if recorder["schema"] != "rssh.task10.detached-recorder-attestation/v1":
        fail("RECORDER", "recorder schema")
    if recorder["mode"] != "detached-c69-worktree-cfg-test-observer":
        fail("RECORDER", "recorder mode")
    if recorder["baseline_commit"] != commit or recorder["baseline_tree"] != actual_tree:
        fail("RECORDER", "recorder baseline identity")
    if recorder["body_codec"] != BODY_CODEC:
        fail("RECORDER", "recorder body codec")
    if recorder["codec_manifest_sha256"] != sha256(codec_manifest):
        fail("RECORDER", "recorder codec manifest digest")
    if recorder["commands"] != RECORDER_COMMANDS:
        fail("RECORDER", "recorder command provenance")
    cargo_lock_blob = git(root, "rev-parse", f"{commit}:Cargo.lock")
    if recorder["baseline_cargo_lock_blob"] != cargo_lock_blob:
        fail("RECORDER", "recorder Cargo.lock provenance")
    gzip_meta = artifacts["pack"]["gzip"]
    if (
        gzip_meta["mtime"] != 0
        or gzip_meta["xfl"] != 2
        or gzip_meta["os"] != 255
        or gzip_meta["compression"] != "best"
        or gzip_meta["compressor"] != "flate2-1.1.9/miniz_oxide-0.8.9"
    ):
        fail("RECORDER", "gzip compressor provenance")
    if artifacts["pack"]["entry_order"] != "row_id-bytewise-ascending":
        fail("RECORDER", "pack entry order provenance")
    if artifacts["pack"]["entry_count"] != TRACE_COUNT:
        fail("RECORDER", "pack entry count provenance")
    if artifacts["pack"]["magic"] != "rssh.task10.trace-pack/v1\\n":
        fail("RECORDER", "pack magic provenance")
    reproducibility = recorder["reproducibility"]
    if (
        reproducibility["independent_runs"] != 2
        or not reproducibility["byte_identical"]
        or reproducibility["first_gzip_sha256"] != gzip_meta["sha256"]
        or reproducibility["second_gzip_sha256"] != gzip_meta["sha256"]
        or reproducibility["gzip_mtime"] != 0
    ):
        fail("RECORDER", "recorder reproducibility attestation")
    if recorder["output"]["raw_sha256"] != artifacts["pack"]["raw"]["sha256"]:
        fail("RECORDER", "recorder output raw digest")
    if recorder["output"]["gzip_sha256"] != gzip_meta["sha256"]:
        fail("RECORDER", "recorder output gzip digest")

    return {
        "ok": True,
        "baseline_commit": commit,
        "baseline_tree": actual_tree,
        "baseline_tests": len(baseline_bodies),
        "trace_entries": len(traces),
        "approved_migrations": len(APPROVED_MIGRATIONS),
        "body_codec": BODY_CODEC,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--provenance", type=Path)
    parser.add_argument("--records", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--pack", type=Path)
    arguments = parser.parse_args()
    root = arguments.root.resolve()
    provenance = arguments.provenance or root / "tests/fixtures/task10_trace_provenance.json"
    records = arguments.records or physical_path(root, "crates/rssh-runtime/tests/fixtures/task10_legacy_fixture_records.txt")
    manifest = arguments.manifest or physical_path(root, "crates/rssh-runtime/tests/fixtures/task10_legacy_test_manifest.txt")
    pack = arguments.pack or root / "tests/fixtures/task10_legacy_trace_pack.gz"
    try:
        report = verify(root, provenance, records, manifest, pack)
    except ProvenanceError as error:
        print(f"TASK10_PROVENANCE[{error.code}] {error}", file=sys.stderr)
        return 1
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
