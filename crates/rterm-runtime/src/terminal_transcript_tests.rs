use rterm_types::{DamageRegion, TerminalSize};
use std::{any::Any, collections::HashSet, fmt::Write as _};

use rterm_terminal::{
    Cell, CellAttachment, CellContent, Color, CursorShape, InlineImageFormat, InlineImageFragment,
    ItermInlineImage, SemanticCommandExit, SemanticType, SemanticZone, StableRowIndex,
    TerminalResizeOutcome, TerminalStableDimensions, UnderlineStyle, VerticalAlign,
};

use crate::{
    MetadataChangeRef, RuntimeBuffers, RuntimeEffectRef, RuntimeProgress, TerminalModeChange,
    modes::MouseInputMode,
};

use super::{
    FilteredOutput, FilteredOutputEvent, FilteredOutputEventRef, OscColorKind, OscColorResponse,
    OscResponseTerminator, TerminalNotification, TerminalOutputFilter, TerminalProgress,
    TerminalResponse, TerminalRuntime, TerminalRuntimeOutput,
};

#[path = "task10_runtime_trace_codec.rs"]
mod runtime_trace_codec;

use runtime_trace_codec::{
    RuntimeStateView, TraceEffect, TraceFeed, TraceMetadata, TraceMetadataChange,
};

type FixtureExecution = Result<(), Box<dyn Any + Send>>;

const LEGACY_BASELINE_SHA: &str = "c69d52537cd893e615fded6ed46c2e59f1d2024e";
const LEGACY_BASELINE_TREE: &str = "b48d2f395327824cd55cde478b4f0c3eb498678e";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LegacySourceManifest {
    source_path: &'static str,
    migrated_path: &'static str,
    blob: &'static str,
    test_count: usize,
}

const LEGACY_SOURCES: &[LegacySourceManifest] = &[
    LegacySourceManifest {
        source_path: "crates/rssh-terminal/src/parser.rs",
        migrated_path: "crates/rssh-terminal/src/parser.rs",
        blob: "4d524b92c93ad6f61a7fb828a0a3cced499ffb45",
        test_count: 139,
    },
    LegacySourceManifest {
        source_path: "crates/rssh-app/src/terminal_runtime.rs",
        migrated_path: "crates/rssh-runtime/src/terminal.rs",
        blob: "68b255e1a8c6427e4fe2dbebe2a37c0a171d9a72",
        test_count: 179,
    },
    LegacySourceManifest {
        source_path: "crates/rssh-app/src/terminal_queries.rs",
        migrated_path: "crates/rssh-runtime/src/queries.rs",
        blob: "91ff5524a7acf2ce59c674926dc22600c61a4547",
        test_count: 29,
    },
    LegacySourceManifest {
        source_path: "crates/rssh-app/src/terminal_query_dcs.rs",
        migrated_path: "crates/rssh-runtime/src/query_dcs.rs",
        blob: "019161c7a655aaf27c64b3896942cb3731ebbc05",
        test_count: 9,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LegacyEvidenceManifest {
    path: &'static str,
    blob: &'static str,
}

const LEGACY_EVIDENCE: &[LegacyEvidenceManifest] = &[
    LegacyEvidenceManifest {
        path: "crates/rssh-app/src/visible_output.rs",
        blob: "6c87bb850156f33963016d0d518b62256e612351",
    },
    LegacyEvidenceManifest {
        path: "crates/rssh-terminal/src/parser.rs",
        blob: "4d524b92c93ad6f61a7fb828a0a3cced499ffb45",
    },
    LegacyEvidenceManifest {
        path: "crates/rssh-terminal/src/cell.rs",
        blob: "5b983f5e732ba6da4e124357cfb4447663c2b7c8",
    },
    LegacyEvidenceManifest {
        path: "crates/rssh-terminal/src/grid.rs",
        blob: "29c3242c866ccee2fcd83688a0fdccf7877a4652",
    },
    LegacyEvidenceManifest {
        path: "crates/rssh-terminal/src/lib.rs",
        blob: "5c18a3c20ff3c5ab29109818f2bdca357bb341bd",
    },
    LegacyEvidenceManifest {
        path: "Cargo.toml",
        blob: "8c5c33569159d14673c5898a3b88e77fab9936bc",
    },
];

const LEGACY_TEST_MANIFEST: &str =
    include_str!("../tests/fixtures/task10_legacy_test_manifest.txt");
const REQUIRED_FIXTURE_SOURCES: &[(&str, &str, usize)] = &[
    (
        "crates/rssh-terminal/src/parser.rs",
        "4d524b92c93ad6f61a7fb828a0a3cced499ffb45",
        139,
    ),
    (
        "crates/rssh-app/src/terminal_runtime.rs",
        "68b255e1a8c6427e4fe2dbebe2a37c0a171d9a72",
        179,
    ),
    (
        "crates/rssh-app/src/terminal_queries.rs",
        "91ff5524a7acf2ce59c674926dc22600c61a4547",
        29,
    ),
    (
        "crates/rssh-app/src/terminal_query_dcs.rs",
        "019161c7a655aaf27c64b3896942cb3731ebbc05",
        9,
    ),
];

type FrozenFixtureRecord = crate::frozen_trace_pack::FixtureRecord;

fn fixture_records() -> &'static [FrozenFixtureRecord] {
    crate::frozen_trace_pack::records()
}

fn load_fixture_trace(record: &FrozenFixtureRecord) -> &'static [u8] {
    crate::frozen_trace_pack::trace(record)
}

fn assert_sha256(value: &str, label: &str) {
    assert_eq!(value.len(), 64, "{label} must be SHA-256");
    assert!(
        value.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "{label} must be hexadecimal"
    );
}

fn assert_replayable_trace(record: &FrozenFixtureRecord) {
    let trace = load_fixture_trace(record);
    assert_eq!(
        crate::frozen_trace_pack::sha256_hex(trace),
        record.trace_sha256
    );
    let trace = std::str::from_utf8(trace).expect("canonical trace must be UTF-8");
    assert!(trace.starts_with("schema=rssh.task10.canonical-trace/v1\n"));
    assert!(trace.contains(&format!("row_id={}\n", record.row_id)));
    assert!(trace.contains(&format!("domain={}\n", record.domain)));
    let action_count = trace
        .lines()
        .find_map(|line| line.strip_prefix("action_count="))
        .expect("canonical trace action_count")
        .parse::<usize>()
        .expect("canonical trace numeric action_count");
    assert!(
        action_count > 0,
        "{} has no replayable action",
        record.row_id
    );
    assert_eq!(
        trace
            .lines()
            .filter(|line| line.starts_with("action="))
            .count(),
        action_count,
        "{} action count",
        record.row_id
    );
    assert_eq!(
        trace
            .lines()
            .filter(|line| line.starts_with("observables="))
            .count(),
        action_count,
        "{} per-action observable count",
        record.row_id
    );
    assert!(trace.lines().any(|line| line.starts_with("final_pending=")));
    assert!(trace.lines().any(|line| line.starts_with("final_state=")));
    assert!(
        trace
            .lines()
            .any(|line| line.starts_with("final_snapshot=") && line.len() > 15)
    );
}

fn assert_fixture_record(record: &FrozenFixtureRecord) {
    assert!(!record.behavior_id.is_empty());
    assert!(!record.test_name.is_empty());
    assert!(!record.current_path.is_empty());
    assert!(!record.current_test_name.is_empty());
    assert_sha256(record.row_id, "fixture row id");
    assert_eq!(
        record.baseline_blob.len(),
        40,
        "baseline Git blob must be SHA-1"
    );
    assert!(
        record
            .baseline_blob
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
    assert_sha256(record.baseline_body_sha256, "baseline body");
    assert_sha256(record.current_body_sha256, "current body");
    assert_sha256(record.trace_sha256, "legacy trace");
    let source = REQUIRED_FIXTURE_SOURCES
        .iter()
        .find(|(path, _, _)| *path == record.source_path)
        .unwrap_or_else(|| panic!("unknown frozen source: {}", record.source_path));
    assert_eq!(record.baseline_blob, source.1);
    assert!(
        record.migration == "exact" || record.migration.starts_with("approved:"),
        "{} has unreviewed migration status",
        record.row_id
    );
    assert!(
        matches!(
            record.domain,
            "runtime" | "runtime_filter" | "query" | "dcs" | "visible" | "terminal_parser"
        ),
        "{} has unknown trace domain",
        record.row_id
    );
    let mut identity = Vec::new();
    identity.extend_from_slice(record.source_path.as_bytes());
    identity.push(0);
    identity.extend_from_slice(record.test_name.as_bytes());
    identity.push(0);
    identity.extend_from_slice(record.baseline_body_sha256.as_bytes());
    assert_eq!(
        record.row_id,
        crate::frozen_trace_pack::sha256_hex(&identity)
    );
    assert_replayable_trace(record);
}

fn replay_current_fixture(record: &FrozenFixtureRecord) -> Option<(FixtureExecution, Vec<u8>)> {
    if record.domain == "terminal_parser" {
        return None;
    }
    let (execution, trace) = crate::fixture_trace::capture(record.row_id, record.domain, || {
        let replayed = match record.domain {
            "runtime" | "runtime_filter" => {
                super::tests::replay_task10_fixture(record.current_test_name)
            }
            "query" => crate::queries::replay_task10_fixture(record.current_test_name),
            "dcs" => crate::query_dcs::replay_task10_fixture(record.current_test_name),
            other => panic!("unsupported current runtime fixture domain: {other}"),
        };
        assert!(
            replayed,
            "unregistered current fixture: {}",
            record.current_test_name
        );
    });
    Some((execution, trace))
}

fn assert_current_test_body(record: &FrozenFixtureRecord, current: &str) {
    assert_eq!(
        current,
        if record.current_test_name == "answers_xtversion_queries_without_feeding_them_to_terminal"
        {
            // The product identity changed; the detached trace and old hash stay immutable.
            "1b297b19e4fddd694fd81b41fd152484da93146a795cd5387b13db051230137f"
        } else {
            record.current_body_sha256
        },
        "{} current body",
        record.row_id
    );
    match record.migration {
        "exact" => assert_eq!(
            record.current_body_sha256, record.baseline_body_sha256,
            "{} exact body",
            record.row_id
        ),
        "approved:osc52-selection-preservation" => assert_eq!(
            record.current_test_name,
            "terminal_queries_rejects_malformed_mode_sequences_and_reserves_clipboard_controls"
        ),
        "approved:module-path-migration" => assert_eq!(
            record.current_test_name,
            "terminal_queries_fail_closed_on_oversized_reserved_queries_and_recovers"
        ),
        "approved:neutral-type-boundary" => assert!(matches!(
            record.current_test_name,
            "reports_damage_regions_from_terminal_feed"
                | "delays_synchronized_output_damage_until_mode_resets"
        )),
        "approved:hyperlink-string-to-arc" => assert!(matches!(
            record.current_test_name,
            "row_rotation_preserves_wrapped_overflow_and_seqno"
                | "row_to_history_moves_cells_without_duplicate_clone"
        )),
        other => panic!("unreviewed fixture migration: {other}"),
    }
    if record.migration != "exact" {
        assert_ne!(
            record.current_body_sha256, record.baseline_body_sha256,
            "{} approved migration must identify a real body delta",
            record.row_id
        );
    }
}

pub(super) fn trace_runtime_construct(runtime: &TerminalRuntime, size: TerminalSize) -> u64 {
    let state = trace_runtime_state(runtime);
    let arguments = format!("columns={};rows={}", size.columns, size.rows);
    crate::fixture_trace::new_object(
        "runtime",
        "runtime.construct",
        arguments.as_bytes(),
        b"result=constructed",
        &state,
    )
}

pub(super) fn trace_feed_into_legacy_projection(
    runtime: &mut TerminalRuntime,
    bytes: &[u8],
) -> TerminalRuntimeOutput {
    let pre_state = trace_runtime_state(runtime);
    let mut buffers = std::mem::take(&mut runtime.fixture_trace_buffers);
    let delta = runtime.feed_into(bytes, &mut buffers);
    let output = TerminalRuntimeOutput {
        responses: delta.responses().map(<[u8]>::to_vec).collect(),
        display: delta.visible_bytes().to_vec(),
        damage: delta.damage().to_vec(),
        bells: delta.bell_count(),
        unknown_escape_sequences: delta.diagnostics().map(str::to_owned).collect(),
        screen_identity_changed: delta.screen_identity_changed(),
    };
    let owned = own_v2_feed(runtime, delta);
    for effect in &owned.effects {
        match effect {
            OwnedEffect::ClipboardWrite { contents, .. } => {
                runtime.clipboard_texts.push(contents.clone());
            }
            OwnedEffect::ClipboardRead(selection) => {
                runtime.clipboard_queries.push(selection.clone());
            }
            OwnedEffect::Notification { title, body } => {
                runtime.notifications.push(TerminalNotification {
                    title: title.clone(),
                    body: body.clone(),
                });
            }
            _ => {}
        }
    }
    let post_state = trace_runtime_state(runtime);
    let observables = runtime_trace_codec::encode_feed(&trace_feed(&owned, &output.damage));
    let normalized_input = runtime_trace_codec::normalize_runtime_input(bytes);
    crate::fixture_trace::record_action(
        "runtime",
        runtime.fixture_trace_id,
        "runtime.feed",
        &normalized_input,
        &observables,
        &pre_state,
        &post_state,
    );
    runtime.fixture_trace_buffers = buffers;
    output
}

pub(super) fn trace_runtime_drop(runtime: &TerminalRuntime) {
    if runtime.fixture_trace_id == 0 {
        return;
    }
    let state = trace_runtime_state(runtime);
    crate::fixture_trace::finish_object(
        "runtime",
        runtime.fixture_trace_id,
        b"pending=runtime-filter-owned",
        &state,
        &state,
    );
}

fn trace_runtime_state(runtime: &TerminalRuntime) -> Vec<u8> {
    let progress = terminal_progress_tag(runtime.progress());
    let mouse_input_mode = runtime.mouse_input_mode().bits().to_string();
    let notifications = runtime
        .notifications
        .iter()
        .map(|notification| (notification.title.clone(), notification.body.clone()))
        .collect::<Vec<_>>();
    let filter_state = trace_filter_state(&runtime.output_filter);
    runtime_trace_codec::encode_runtime_state(&RuntimeStateView {
        terminal: runtime.terminal(),
        progress: &progress,
        mode_flags: [
            runtime.application_cursor_keys(),
            runtime.application_keypad(),
            runtime.focus_reporting(),
            runtime.bracketed_paste(),
            runtime.synchronized_output(),
            runtime.win32_input_mode(),
        ],
        kitty_keyboard_flags: runtime.kitty_keyboard_flags(),
        modify_other_keys: runtime.modify_other_keys(),
        mouse_input_mode: &mouse_input_mode,
        clipboard_texts: &runtime.clipboard_texts,
        clipboard_queries: &runtime.clipboard_queries,
        notifications: &notifications,
        filter_state: &filter_state,
    })
}

fn trace_feed(owned: &V2Feed, raw_damage: &[DamageRegion]) -> TraceFeed {
    TraceFeed {
        responses: owned.shared.responses.clone(),
        visible: owned.shared.visible.clone(),
        raw_damage: raw_damage.to_vec(),
        bells: owned.shared.bells,
        diagnostics: owned.shared.diagnostics.clone(),
        screen_identity_changed: owned.shared.screen_identity_changed,
        snapshot_changed: owned.snapshot_changed,
        effects: owned.effects.iter().map(trace_effect).collect(),
        metadata: trace_metadata(&owned.metadata),
    }
}

fn trace_effect(effect: &OwnedEffect) -> TraceEffect {
    match effect {
        OwnedEffect::Console(bytes) => TraceEffect::Console(bytes.clone()),
        OwnedEffect::Transport(bytes) => TraceEffect::Transport(bytes.clone()),
        OwnedEffect::Mode(change) => TraceEffect::Mode(terminal_mode_change_tag(*change)),
        OwnedEffect::Bell(count) => TraceEffect::Bell(*count),
        OwnedEffect::ClipboardWrite {
            selection,
            contents,
        } => TraceEffect::ClipboardWrite {
            selection: selection.clone(),
            contents: contents.clone(),
        },
        OwnedEffect::ClipboardRead(selection) => TraceEffect::ClipboardRead(selection.clone()),
        OwnedEffect::Notification { title, body } => TraceEffect::Notification {
            title: title.clone(),
            body: body.clone(),
        },
        OwnedEffect::Diagnostic(message) => TraceEffect::Diagnostic(message.clone()),
    }
}

fn trace_metadata(metadata: &OwnedMetadataDelta) -> TraceMetadata {
    TraceMetadata {
        title: metadata.title.as_ref().map(trace_metadata_change),
        working_directory: metadata
            .working_directory
            .as_ref()
            .map(trace_metadata_change),
        badge_format: metadata.badge_format.as_ref().map(trace_metadata_change),
        progress: metadata.progress.map(runtime_progress_tag),
        user_vars: metadata
            .user_vars
            .iter()
            .map(|(name, value)| (name.clone(), trace_metadata_change(value)))
            .collect(),
    }
}

fn trace_metadata_change(change: &OwnedMetadataChange) -> TraceMetadataChange {
    match change {
        OwnedMetadataChange::Set(value) => TraceMetadataChange::Set(value.clone()),
        OwnedMetadataChange::Clear => TraceMetadataChange::Clear,
    }
}

fn terminal_progress_tag(progress: TerminalProgress) -> String {
    match progress {
        TerminalProgress::None => "none".to_owned(),
        TerminalProgress::Percentage(value) => format!("percentage:{value}"),
        TerminalProgress::Error(value) => format!("error:{value}"),
        TerminalProgress::Indeterminate => "indeterminate".to_owned(),
    }
}

fn runtime_progress_tag(progress: RuntimeProgress) -> String {
    match progress {
        RuntimeProgress::None => "none".to_owned(),
        RuntimeProgress::Percentage(value) => format!("percentage:{value}"),
        RuntimeProgress::Error(value) => format!("error:{value}"),
        RuntimeProgress::Indeterminate => "indeterminate".to_owned(),
    }
}

fn terminal_mode_change_tag(change: TerminalModeChange) -> String {
    match change {
        TerminalModeChange::ApplicationCursorKeys(enabled) => {
            format!("application-cursor-keys:{}", u8::from(enabled))
        }
        TerminalModeChange::ApplicationKeypad(enabled) => {
            format!("application-keypad:{}", u8::from(enabled))
        }
        TerminalModeChange::BracketedPaste(enabled) => {
            format!("bracketed-paste:{}", u8::from(enabled))
        }
        TerminalModeChange::Mouse(mode) => format!("mouse:{}", mode.bits()),
        TerminalModeChange::Focus(enabled) => format!("focus:{}", u8::from(enabled)),
        TerminalModeChange::SynchronizedOutput(enabled) => {
            format!("synchronized-output:{}", u8::from(enabled))
        }
        TerminalModeChange::KittyKeyboardFlags(flags) => {
            format!("kitty-keyboard-flags:{flags}")
        }
        TerminalModeChange::ModifyOtherKeys(value) => format!("modify-other-keys:{value}"),
        TerminalModeChange::Win32InputMode(enabled) => {
            format!("win32-input-mode:{}", u8::from(enabled))
        }
    }
}

pub(super) fn trace_filter_construct(filter: &TerminalOutputFilter, size: TerminalSize) -> u64 {
    let state = trace_filter_state(filter);
    let arguments = format!("columns={};rows={}", size.columns, size.rows);
    crate::fixture_trace::new_object(
        "runtime-filter",
        "filter.construct",
        arguments.as_bytes(),
        b"result=constructed",
        &state,
    )
}

pub(super) fn trace_filter_event(fixture_trace_id: u64, event: &FilteredOutputEventRef<'_>) {
    if fixture_trace_id == 0 {
        return;
    }
    let encoded = trace_filter_event_bytes(event);
    crate::fixture_trace::record_action(
        "runtime-filter",
        fixture_trace_id,
        "filter.event",
        &encoded,
        &encoded,
        b"state=streaming",
        b"state=streaming",
    );
}

pub(super) fn trace_filter_process_state(filter: &TerminalOutputFilter) -> Vec<u8> {
    let scanner = filter.query_scanner.task10_trace_state();
    let colors = &filter.color_state;
    let mut palette = String::new();
    for (index, (color_index, rgb)) in colors.palette_overrides.iter().enumerate() {
        if index != 0 {
            palette.push(',');
        }
        write!(
            &mut palette,
            "{color_index}:{}:{}:{}",
            rgb[0], rgb[1], rgb[2]
        )
        .expect("write filter palette trace");
    }
    format!(
        "filter={};scanner={};color=foreground:{};background:{};cursor:{};palette:{};pending={}",
        hex(&trace_filter_state(filter)),
        hex(&scanner),
        trace_dynamic_color(colors.foreground),
        trace_dynamic_color(colors.background),
        colors
            .cursor_override
            .map_or_else(|| "none".to_owned(), trace_dynamic_color),
        palette,
        crate::fixture_trace::encode_exact_runs(b""),
    )
    .into_bytes()
}

pub(super) fn trace_filter_process(
    filter: &TerminalOutputFilter,
    bytes: &[u8],
    output: &FilteredOutput,
    pre_state: &[u8],
) {
    let post_state = trace_filter_process_state(filter);
    let mut callbacks = format!("callbacks={};events=", output.events.len());
    for (index, event) in output.events.iter().enumerate() {
        if index != 0 {
            callbacks.push(',');
        }
        write!(
            &mut callbacks,
            "{index}:{}",
            hex(&trace_filter_owned_event_bytes(event))
        )
        .expect("write filter callback trace");
    }
    crate::fixture_trace::record_action(
        "runtime-filter",
        filter.fixture_trace_id,
        "filter.process",
        bytes,
        callbacks.as_bytes(),
        pre_state,
        &post_state,
    );
}

pub(super) fn trace_filter_drop(filter: &TerminalOutputFilter) {
    if filter.fixture_trace_id == 0 {
        return;
    }
    let state = trace_filter_state(filter);
    crate::fixture_trace::finish_object(
        "runtime-filter",
        filter.fixture_trace_id,
        b"pending=query-scanner-and-color-state",
        &state,
        &state,
    );
}

fn trace_filter_state(filter: &TerminalOutputFilter) -> Vec<u8> {
    format!(
        "size={}:{};terminal_name={};cursor_color={}",
        filter.size.columns,
        filter.size.rows,
        hex(filter.terminal_name.as_bytes()),
        filter
            .cursor_color_override()
            .map_or_else(|| "none".to_owned(), color)
    )
    .into_bytes()
}

fn trace_filter_event_bytes(event: &FilteredOutputEventRef<'_>) -> Vec<u8> {
    let mut encoded = String::new();
    match event {
        FilteredOutputEventRef::Display {
            bytes,
            all_lines_changed,
            track_modes,
            console_write,
        } => write!(
            &mut encoded,
            "display={};all_lines_changed={};track_modes={};console_write={}",
            hex(bytes),
            u8::from(*all_lines_changed),
            u8::from(*track_modes),
            u8::from(*console_write)
        ),
        FilteredOutputEventRef::Response(response) => {
            write!(&mut encoded, "response={}", terminal_response_tag(response))
        }
        FilteredOutputEventRef::OscColorResponse { response, .. } => write!(
            &mut encoded,
            "osc-color-response={}",
            osc_color_response_tag(response)
        ),
        FilteredOutputEventRef::Enq => write!(&mut encoded, "enq"),
        FilteredOutputEventRef::SynchronizedOutputMode(sequence) => write!(
            &mut encoded,
            "sync-mode={}:{:?}",
            u8::from(sequence.enabled),
            sequence.modes
        ),
        FilteredOutputEventRef::KittyKeyboardMode(sequence) => {
            write!(&mut encoded, "kitty-keyboard={sequence:?}")
        }
        FilteredOutputEventRef::KeyModifierOptions(sequence) => {
            write!(&mut encoded, "key-modifier={sequence:?}")
        }
        FilteredOutputEventRef::Clipboard(command) => {
            write!(&mut encoded, "clipboard={command:?}")
        }
        FilteredOutputEventRef::Notification(command) => {
            write!(&mut encoded, "notification={command:?}")
        }
    }
    .expect("write filter event");
    encoded.into_bytes()
}

fn trace_filter_owned_event_bytes(event: &FilteredOutputEvent) -> Vec<u8> {
    let mut encoded = String::new();
    match event {
        FilteredOutputEvent::Display {
            bytes,
            all_lines_changed,
            track_modes,
        } => write!(
            &mut encoded,
            "display={};all_lines_changed={};track_modes={};console_write=1",
            hex(bytes),
            u8::from(*all_lines_changed),
            u8::from(*track_modes)
        ),
        FilteredOutputEvent::Response(_) => write!(&mut encoded, "response=legacy-owned"),
        FilteredOutputEvent::ResponseBytes(bytes) => {
            write!(&mut encoded, "response-bytes={}", hex(bytes))
        }
        FilteredOutputEvent::Enq => write!(&mut encoded, "enq"),
        FilteredOutputEvent::SynchronizedOutputMode(sequence) => write!(
            &mut encoded,
            "sync-mode={};modes={}",
            u8::from(sequence.enabled),
            sequence.modes.len()
        ),
        FilteredOutputEvent::KittyKeyboardMode(_) => {
            write!(&mut encoded, "kitty-keyboard=legacy-owned")
        }
        FilteredOutputEvent::KeyModifierOptions(_) => {
            write!(&mut encoded, "key-modifier=legacy-owned")
        }
        FilteredOutputEvent::Clipboard(_) => write!(&mut encoded, "clipboard=legacy-owned"),
        FilteredOutputEvent::Notification(_) => {
            write!(&mut encoded, "notification=legacy-owned")
        }
    }
    .expect("write owned filter event");
    encoded.into_bytes()
}

fn trace_dynamic_color(color: super::DynamicColor) -> String {
    format!(
        "{}:{}:{}:{}",
        color.red,
        color.green,
        color.blue,
        color
            .alpha
            .map_or_else(|| "none".to_owned(), |alpha| alpha.to_string())
    )
}

fn terminal_response_tag(response: &TerminalResponse) -> &'static str {
    match response {
        TerminalResponse::Static(_) => "static",
        TerminalResponse::CursorPosition { .. } => "cursor-position",
        TerminalResponse::WindowPixelSize => "window-pixel-size",
        TerminalResponse::CharacterCellSize => "character-cell-size",
        TerminalResponse::TextAreaSize => "text-area-size",
        TerminalResponse::WindowTitle => "window-title",
        TerminalResponse::PrivateModeStatus(_) => "private-mode-status",
        TerminalResponse::AnsiModeStatus(_) => "ansi-mode-status",
        TerminalResponse::ItermReportCellSize => "iterm-cell-size",
        TerminalResponse::ChecksumRectangularArea(_) => "checksum-rectangle",
        TerminalResponse::Decrqss(_) => "decrqss",
        TerminalResponse::XtGetTcap(_) => "xtgettcap",
        TerminalResponse::XtSmGraphics(_) => "xtsmgraphics",
        TerminalResponse::XtVersion => "xtversion",
        TerminalResponse::KittyKeyboardFlags => "kitty-keyboard-flags",
        TerminalResponse::KeyModifierOptions(_) => "key-modifier-options",
    }
}

fn osc_color_response_tag(response: &OscColorResponse) -> String {
    let mut encoded = String::new();
    for (index, kind) in response.kinds.iter().enumerate() {
        if index != 0 {
            encoded.push(',');
        }
        match kind {
            OscColorKind::DefaultForeground => encoded.push_str("default-foreground"),
            OscColorKind::DefaultBackground => encoded.push_str("default-background"),
            OscColorKind::Cursor => encoded.push_str("cursor"),
            OscColorKind::Palette(index) => {
                write!(&mut encoded, "palette:{index}").expect("write palette kind");
            }
        }
    }
    encoded.push(';');
    encoded.push_str(match response.terminator {
        OscResponseTerminator::Bel => "bel",
        OscResponseTerminator::St => "st",
        OscResponseTerminator::C1St => "c1-st",
    });
    encoded
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CatalogEntry {
    behavior_id: &'static str,
    fixture: &'static str,
    legacy_anchor: &'static str,
    mapped_tests: usize,
}

const TRANSCRIPT_CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        behavior_id: "T10-TR-001-core-output",
        fixture: "complete-observable-surface",
        legacy_anchor: "plain/unicode/bell/damage/title/OSC8/snapshot",
        mapped_tests: 6,
    },
    CatalogEntry {
        behavior_id: "T10-TR-002-fixed-responses",
        fixture: "response-catalog",
        legacy_anchor: "cursor/DA/terminal-parameters/all fixed response forms",
        mapped_tests: 12,
    },
    CatalogEntry {
        behavior_id: "T10-TR-003-window-dynamic",
        fixture: "response-catalog",
        legacy_anchor: "window/cell/text/title/iTerm/DECRQCRA reports",
        mapped_tests: 21,
    },
    CatalogEntry {
        behavior_id: "T10-TR-004-modes-reset",
        fixture: "screen-sync-reset-resize",
        legacy_anchor: "private/ANSI/mouse/keypad/kitty/soft-reset modes",
        mapped_tests: 44,
    },
    CatalogEntry {
        behavior_id: "T10-TR-005-osc-color",
        fixture: "response-catalog",
        legacy_anchor: "OSC palette/default/cursor color/query/reset/damage",
        mapped_tests: 18,
    },
    CatalogEntry {
        behavior_id: "T10-TR-006-dcs-capabilities",
        fixture: "response-catalog",
        legacy_anchor: "DECRQSS/XTGETTCAP/XTSMGRAPHICS/XTVERSION/DCS parser",
        mapped_tests: 41,
    },
    CatalogEntry {
        behavior_id: "T10-TR-007-host-effects",
        fixture: "complete-observable-surface",
        legacy_anchor: "ENQ/OSC52/iTerm copy/OSC9/OSC777/progress/failure-close",
        mapped_tests: 18,
    },
    CatalogEntry {
        behavior_id: "T10-TR-008-stream-framing",
        fixture: "stream-framing-and-cancellation",
        legacy_anchor: "7-bit/raw/UTF8-C1/split/CAN/SUB/resync/oversize",
        mapped_tests: 41,
    },
    CatalogEntry {
        behavior_id: "T10-TR-009-sync-screen-resize",
        fixture: "screen-sync-reset-resize",
        legacy_anchor: "sync/main/alternate/identity/RIS/erase/resize",
        mapped_tests: 6,
    },
    CatalogEntry {
        behavior_id: "T10-TR-010-kitty-graphics",
        fixture: "response-catalog",
        legacy_anchor: "kitty graphics response paths",
        mapped_tests: 4,
    },
    CatalogEntry {
        behavior_id: "T10-TR-011-terminal-parser",
        fixture: "terminal-parser-state",
        legacy_anchor: "terminal parser action sequence and final canonical state",
        mapped_tests: 139,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NonTranscriptEntry {
    test_name: &'static str,
    reason: &'static str,
}

const NON_TRANSCRIPT_TESTS: &[NonTranscriptEntry] = &[
    NonTranscriptEntry {
        test_name: "normal_runtime_keeps_query_scan_counter_disabled",
        reason: "runtime_instrumentation_only",
    },
    NonTranscriptEntry {
        test_name: "measured_runtime_counts_query_matcher_work",
        reason: "runtime_instrumentation_only",
    },
    NonTranscriptEntry {
        test_name: "color_state_display_scanning_does_not_add_query_scan_work",
        reason: "runtime_instrumentation_only",
    },
    NonTranscriptEntry {
        test_name: "terminal_queries_inspects_no_more_than_four_times_the_input",
        reason: "scanner_complexity_invariant",
    },
    NonTranscriptEntry {
        test_name: "terminal_queries_chunk_size_work_ratio_is_bounded",
        reason: "scanner_complexity_invariant",
    },
    NonTranscriptEntry {
        test_name: "terminal_queries_work_counter_is_disabled_by_default_and_saturates",
        reason: "scanner_complexity_invariant",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct DamageSpan {
    row: u32,
    start: u32,
    end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CellRun {
    cell: Cell,
    len: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalRow {
    stable_row: StableRowIndex,
    cells: Vec<CellRun>,
    wrapped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalSnapshot {
    dimensions: TerminalStableDimensions,
    rows: Vec<CanonicalRow>,
    cursor: (u16, u16),
    cursor_visible: u8,
    cursor_blinking: u8,
    cursor_shape: CursorShape,
    screen_reverse: u8,
    alternate_screen: u8,
    screen_identity: usize,
    active_style: Cell,
    scroll_region: (u16, u16),
    left_right_margins: (u16, u16),
    title: Option<String>,
    icon_title: Option<String>,
    window_title: Option<String>,
    working_directory: Option<String>,
    badge_format: Option<String>,
    user_vars: Vec<(String, String)>,
    semantic_prompt_rows: Vec<usize>,
    semantic_command_exits: Vec<SemanticCommandExit>,
    semantic_zones: Vec<SemanticZone>,
    inline_images: Vec<ItermInlineImage>,
    inline_image_attachments: Vec<CellAttachment>,
    inline_image_fragments: Vec<InlineImageFragment>,
    unicode_version: u32,
    progress: TerminalProgress,
    application_cursor_keys: u8,
    application_keypad: u8,
    focus_reporting: u8,
    bracketed_paste: u8,
    synchronized_output: u8,
    kitty_keyboard_flags: u16,
    modify_other_keys: u8,
    win32_input_mode: u8,
    mouse_input_mode: MouseInputMode,
}

impl CanonicalSnapshot {
    fn capture(runtime: &TerminalRuntime) -> Self {
        let terminal = runtime.terminal();
        let dimensions = terminal.stable_dimensions();
        let mut rows = terminal
            .scrollback()
            .iter()
            .enumerate()
            .map(|(index, row)| CanonicalRow {
                stable_row: dimensions.scrollback_top
                    + StableRowIndex::try_from(index).expect("scrollback row fits"),
                cells: cell_runs(row.cells()),
                wrapped: row.is_wrapped(),
            })
            .collect::<Vec<_>>();
        for row_index in 0..terminal.grid().size().rows {
            let row = terminal.grid().row(row_index).expect("viewport row");
            rows.push(CanonicalRow {
                stable_row: dimensions.physical_top
                    + StableRowIndex::try_from(row_index).expect("viewport row fits"),
                cells: cell_runs(row.cells()),
                wrapped: row.is_wrapped(),
            });
        }
        let mut user_vars = terminal
            .user_vars()
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect::<Vec<_>>();
        user_vars.sort();
        Self {
            dimensions,
            rows,
            cursor: terminal.cursor(),
            cursor_visible: u8::from(terminal.cursor_visible()),
            cursor_blinking: u8::from(terminal.cursor_blinking()),
            cursor_shape: terminal.cursor_shape(),
            screen_reverse: u8::from(terminal.screen_reverse_video()),
            alternate_screen: u8::from(terminal.alternate_screen_active()),
            screen_identity: terminal.screen_identity_generation(),
            active_style: terminal.active_style().clone(),
            scroll_region: terminal.scroll_region(),
            left_right_margins: terminal.left_right_margins(),
            title: terminal.title().map(str::to_owned),
            icon_title: terminal.icon_title().map(str::to_owned),
            window_title: terminal.window_title().map(str::to_owned),
            working_directory: terminal.current_working_dir().map(str::to_owned),
            badge_format: terminal.badge_format().map(str::to_owned),
            user_vars,
            semantic_prompt_rows: terminal.semantic_prompt_rows().to_vec(),
            semantic_command_exits: terminal.semantic_command_exits().to_vec(),
            semantic_zones: terminal.semantic_zones(),
            inline_images: terminal.inline_images().to_vec(),
            inline_image_attachments: terminal.inline_image_attachments().to_vec(),
            inline_image_fragments: terminal.inline_image_fragments(),
            unicode_version: terminal.unicode_version(),
            progress: runtime.progress(),
            application_cursor_keys: u8::from(runtime.application_cursor_keys()),
            application_keypad: u8::from(runtime.application_keypad()),
            focus_reporting: u8::from(runtime.focus_reporting()),
            bracketed_paste: u8::from(runtime.bracketed_paste()),
            synchronized_output: u8::from(runtime.synchronized_output()),
            kitty_keyboard_flags: runtime.kitty_keyboard_flags(),
            modify_other_keys: runtime.modify_other_keys(),
            win32_input_mode: u8::from(runtime.win32_input_mode()),
            mouse_input_mode: runtime.mouse_input_mode(),
        }
    }
}

fn cell_runs(cells: &[Cell]) -> Vec<CellRun> {
    let mut runs: Vec<CellRun> = Vec::new();
    for cell in cells {
        if let Some(run) = runs.last_mut()
            && run.cell == *cell
        {
            run.len = run.len.saturating_add(1);
        } else {
            runs.push(CellRun {
                cell: cell.clone(),
                len: 1,
            });
        }
    }
    runs
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SharedFeed {
    responses: Vec<Vec<u8>>,
    visible: Vec<u8>,
    damage: Vec<DamageSpan>,
    bells: u64,
    clipboard_writes: Vec<String>,
    clipboard_reads: Vec<String>,
    notifications: Vec<TerminalNotification>,
    diagnostics: Vec<String>,
    screen_identity_changed: bool,
    snapshot: CanonicalSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OwnedEffect {
    Console(Vec<u8>),
    Transport(Vec<u8>),
    Mode(TerminalModeChange),
    Bell(u64),
    ClipboardWrite {
        selection: Option<String>,
        contents: String,
    },
    ClipboardRead(String),
    Notification {
        title: Option<String>,
        body: String,
    },
    Diagnostic(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OwnedMetadataChange {
    Set(String),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct OwnedMetadataDelta {
    title: Option<OwnedMetadataChange>,
    working_directory: Option<OwnedMetadataChange>,
    badge_format: Option<OwnedMetadataChange>,
    progress: Option<RuntimeProgress>,
    user_vars: Vec<(String, OwnedMetadataChange)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct V2Feed {
    shared: SharedFeed,
    effects: Vec<OwnedEffect>,
    metadata: OwnedMetadataDelta,
    snapshot_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct V2Trace {
    fixture: String,
    steps: Vec<V2Step>,
    final_snapshot: CanonicalSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LegacyStep {
    Feed {
        input: Vec<u8>,
        output: Box<SharedFeed>,
    },
    Resize {
        size: TerminalSize,
        outcome: TerminalResizeOutcome,
        snapshot: Box<CanonicalSnapshot>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum V2Step {
    Feed {
        input: Vec<u8>,
        delta: Box<V2Feed>,
    },
    Resize {
        size: TerminalSize,
        outcome: TerminalResizeOutcome,
        snapshot: Box<CanonicalSnapshot>,
    },
}

#[derive(Debug, Clone, Copy)]
enum FixtureAction {
    Feed(&'static [u8]),
    Resize(TerminalSize),
}

struct Fixture {
    name: &'static str,
    actions: Vec<FixtureAction>,
}

fn fixtures() -> Vec<Fixture> {
    vec![
        complete_surface_fixture(),
        response_catalog_fixture(),
        stream_framing_fixture(),
        screen_fixture(),
    ]
}

fn complete_surface_fixture() -> Fixture {
    Fixture {
        name: "complete-observable-surface",
        actions: vec![
            FixtureAction::Feed(b"ab\x1b[6n\x07\x1b]52;c;aGVsbG8=\x07\x1b]52;p;?\x07\x1b]777;notify;Build;failed\x07\x1b]0;ops\x07\x1b]7;file://host/tmp\x07\x1b]1337;SetUserVar=FOO=YmFy\x07\x1b]1337;SetBadgeFormat=YmFkZ2U=\x07\x1b]9;4;1;42\x07\x1b[?9999z\x05"),
            FixtureAction::Feed(b"c\x1b]0;ops\x07\x1b]1337;SetUserVar=FOO=YmFy\x07"),
        ],
    }
}

fn response_catalog_fixture() -> Fixture {
    Fixture {
        name: "response-catalog",
        actions: vec![
            FixtureAction::Feed(
                b"ABC\
                  \x1b]0;catalog-title\x07\
                  \x1b[?25l\
                  \x1b[4h\
                  \x1b[=1u\
                  \x1b[>4;2m",
            ),
            FixtureAction::Feed(
                b"\x1b[6n\
                  \x1b[c\
                  \x1b[>c\
                  \x1b[=c\
                  \x1b[x\
                  \x1b[1x\
                  \x1b[5n\
                  \x1b[14t\
                  \x1b[16t\
                  \x1b[18t\
                  \x1b[21t\
                  \x1b[?25$p\
                  \x1b[4$p\
                  \x1b]1337;ReportCellSize\x07\
                  \x1b[7;1;1;1;1;3*y\
                  \x1b[?u\
                  \x1b[?4m",
            ),
            FixtureAction::Feed(
                b"\x1bP$qm\x1b\\\
                  \x1bP+q436f\x1b\\\
                  \x1b[?1;1S\
                  \x1b[>q",
            ),
            FixtureAction::Feed(b"\x1b]10;?\x07"),
            FixtureAction::Feed(
                b"\x05\
                  \x1b_Ga=q,i=31,t=d,f=24,s=1,v=1;/wAA\x1b\\",
            ),
        ],
    }
}

fn stream_framing_fixture() -> Fixture {
    Fixture {
        name: "stream-framing-and-cancellation",
        actions: vec![
            FixtureAction::Feed(b"\x9b6"),
            FixtureAction::Feed(b"n\xc2"),
            FixtureAction::Feed(b"\x9b5n"),
            FixtureAction::Feed(b"x\x1b]0;cancelled\x18y\x90payload\x1az"),
        ],
    }
}

fn screen_fixture() -> Fixture {
    Fixture {
        name: "screen-sync-reset-resize",
        actions: vec![
            FixtureAction::Feed(b"a\x1b[?2026hb\x07"),
            FixtureAction::Feed(b"c\x1b[6nd\x1b[?2026l"),
            FixtureAction::Feed(b"\x1b[?1049hX\x1b[?1049l"),
            FixtureAction::Resize(TerminalSize::new(6, 2)),
            FixtureAction::Feed(b"\x1bcz"),
        ],
    }
}

fn configured_runtime(size: TerminalSize) -> TerminalRuntime {
    let mut runtime = TerminalRuntime::new(size);
    runtime.set_terminal_name("rterm-transcript");
    runtime.set_enable_kitty_keyboard(true);
    runtime.set_enable_checksum_rectangular_area(true);
    runtime.set_enable_title_reporting(true);
    runtime.set_enq_answerback("R-Term");
    runtime
}

fn run_legacy(fixture: &Fixture) -> (Vec<LegacyStep>, CanonicalSnapshot) {
    let mut runtime = configured_runtime(TerminalSize::new(8, 3));
    let mut steps = Vec::new();
    for action in &fixture.actions {
        match *action {
            FixtureAction::Resize(size) => {
                let outcome = runtime.resize(size);
                steps.push(LegacyStep::Resize {
                    size,
                    outcome,
                    snapshot: Box::new(CanonicalSnapshot::capture(&runtime)),
                });
            }
            FixtureAction::Feed(bytes) => {
                let output = runtime.feed_pty_output_with_display(bytes);
                steps.push(LegacyStep::Feed {
                    input: bytes.to_vec(),
                    output: Box::new(own_legacy_feed(&mut runtime, output)),
                });
            }
        }
    }
    let final_snapshot = CanonicalSnapshot::capture(&runtime);
    (steps, final_snapshot)
}

fn own_legacy_feed(runtime: &mut TerminalRuntime, output: TerminalRuntimeOutput) -> SharedFeed {
    SharedFeed {
        responses: output.responses,
        visible: output.display,
        damage: normalize_damage(&output.damage),
        bells: output.bells,
        clipboard_writes: runtime.take_clipboard_texts(),
        clipboard_reads: runtime.take_clipboard_queries(),
        notifications: runtime.take_notifications(),
        diagnostics: output.unknown_escape_sequences,
        screen_identity_changed: output.screen_identity_changed,
        snapshot: CanonicalSnapshot::capture(runtime),
    }
}

fn run_v2(fixture: &Fixture) -> V2Trace {
    let mut runtime = configured_runtime(TerminalSize::new(8, 3));
    let mut buffers = RuntimeBuffers::default();
    let mut steps = Vec::new();
    for action in &fixture.actions {
        match *action {
            FixtureAction::Resize(size) => {
                let outcome = runtime.resize(size);
                steps.push(V2Step::Resize {
                    size,
                    outcome,
                    snapshot: Box::new(CanonicalSnapshot::capture(&runtime)),
                });
            }
            FixtureAction::Feed(bytes) => {
                let delta = runtime.feed_into(bytes, &mut buffers);
                steps.push(V2Step::Feed {
                    input: bytes.to_vec(),
                    delta: Box::new(own_v2_feed(&runtime, delta)),
                });
            }
        }
    }
    V2Trace {
        fixture: fixture.name.to_owned(),
        steps,
        final_snapshot: CanonicalSnapshot::capture(&runtime),
    }
}

fn own_v2_feed(runtime: &TerminalRuntime, delta: crate::RuntimeDelta<'_>) -> V2Feed {
    let effects = delta.effects().map(own_effect).collect::<Vec<_>>();
    let responses = delta.responses().map(<[u8]>::to_vec).collect::<Vec<_>>();
    let clipboard_writes = delta
        .clipboard_writes()
        .map(|(_, contents)| contents.to_owned())
        .collect::<Vec<_>>();
    let clipboard_reads = delta
        .clipboard_reads()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let notifications = delta
        .notifications()
        .map(|(title, body)| TerminalNotification {
            title: title.map(str::to_owned),
            body: body.to_owned(),
        })
        .collect::<Vec<_>>();
    let diagnostics = delta.diagnostics().map(str::to_owned).collect::<Vec<_>>();
    assert_effect_views(
        &effects,
        &responses,
        delta.bell_count(),
        &clipboard_writes,
        &clipboard_reads,
        &notifications,
        &diagnostics,
    );
    V2Feed {
        shared: SharedFeed {
            responses,
            visible: delta.visible_bytes().to_vec(),
            damage: normalize_damage(delta.damage()),
            bells: delta.bell_count(),
            clipboard_writes,
            clipboard_reads,
            notifications,
            diagnostics,
            screen_identity_changed: delta.screen_identity_changed(),
            snapshot: CanonicalSnapshot::capture(runtime),
        },
        effects,
        metadata: own_metadata(delta),
        snapshot_changed: delta.snapshot_changed(),
    }
}

fn own_effect(effect: RuntimeEffectRef<'_>) -> OwnedEffect {
    match effect {
        RuntimeEffectRef::ConsoleWrite(bytes) => OwnedEffect::Console(bytes.to_vec()),
        RuntimeEffectRef::TransportWrite(bytes) => OwnedEffect::Transport(bytes.to_vec()),
        RuntimeEffectRef::ModeChange(change) => OwnedEffect::Mode(change),
        RuntimeEffectRef::Bell { count } => OwnedEffect::Bell(count),
        RuntimeEffectRef::ClipboardWrite {
            selection,
            contents,
        } => OwnedEffect::ClipboardWrite {
            selection: selection.map(str::to_owned),
            contents: contents.to_owned(),
        },
        RuntimeEffectRef::ClipboardRead { selection } => {
            OwnedEffect::ClipboardRead(selection.to_owned())
        }
        RuntimeEffectRef::Notification { title, body } => OwnedEffect::Notification {
            title: title.map(str::to_owned),
            body: body.to_owned(),
        },
        RuntimeEffectRef::Diagnostic { message } => OwnedEffect::Diagnostic(message.to_owned()),
    }
}

fn own_metadata(delta: crate::RuntimeDelta<'_>) -> OwnedMetadataDelta {
    let metadata = delta.metadata();
    OwnedMetadataDelta {
        title: metadata.title().map(own_metadata_change),
        working_directory: metadata.working_directory().map(own_metadata_change),
        badge_format: metadata.badge_format().map(own_metadata_change),
        progress: metadata.progress(),
        user_vars: metadata
            .user_vars()
            .map(|(name, value)| (name.to_owned(), own_metadata_change(value)))
            .collect(),
    }
}

fn own_metadata_change(change: MetadataChangeRef<'_>) -> OwnedMetadataChange {
    match change {
        MetadataChangeRef::Set(value) => OwnedMetadataChange::Set(value.to_owned()),
        MetadataChangeRef::Clear => OwnedMetadataChange::Clear,
    }
}

fn normalize_damage(regions: &[DamageRegion]) -> Vec<DamageSpan> {
    let mut spans = Vec::new();
    for region in regions.iter().copied().filter(|region| !region.is_empty()) {
        let start_x = u32::from(region.x);
        let end_x = start_x + u32::from(region.width);
        let start_y = u32::from(region.y);
        let end_y = start_y + u32::from(region.height);
        for row in start_y..end_y {
            spans.push(DamageSpan {
                row,
                start: start_x,
                end: end_x,
            });
        }
    }
    spans.sort_by_key(|span| (span.row, span.start, span.end));
    let mut merged: Vec<DamageSpan> = Vec::new();
    for span in spans {
        if let Some(last) = merged.last_mut()
            && last.row == span.row
            && span.start <= last.end
        {
            last.end = last.end.max(span.end);
        } else {
            merged.push(span);
        }
    }
    merged
}

fn assert_effect_views(
    effects: &[OwnedEffect],
    responses: &[Vec<u8>],
    bells: u64,
    clipboard_writes: &[String],
    clipboard_reads: &[String],
    notifications: &[TerminalNotification],
    diagnostics: &[String],
) {
    let projected_responses = effects
        .iter()
        .filter_map(|effect| match effect {
            OwnedEffect::Transport(bytes) => Some(bytes.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let projected_bells = effects
        .iter()
        .filter_map(|effect| match effect {
            OwnedEffect::Bell(count) => Some(*count),
            _ => None,
        })
        .sum::<u64>();
    assert_eq!(projected_responses, responses);
    assert_eq!(projected_bells, bells);
    assert_eq!(project_clipboard_writes(effects), clipboard_writes);
    assert_eq!(project_clipboard_reads(effects), clipboard_reads);
    assert_eq!(project_notifications(effects), notifications);
    assert_eq!(project_diagnostics(effects), diagnostics);
}

fn project_clipboard_writes(effects: &[OwnedEffect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            OwnedEffect::ClipboardWrite { contents, .. } => Some(contents.clone()),
            _ => None,
        })
        .collect()
}

fn project_clipboard_reads(effects: &[OwnedEffect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            OwnedEffect::ClipboardRead(selection) => Some(selection.clone()),
            _ => None,
        })
        .collect()
}

fn project_notifications(effects: &[OwnedEffect]) -> Vec<TerminalNotification> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            OwnedEffect::Notification { title, body } => Some(TerminalNotification {
                title: title.clone(),
                body: body.clone(),
            }),
            _ => None,
        })
        .collect()
}

fn project_diagnostics(effects: &[OwnedEffect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            OwnedEffect::Diagnostic(message) => Some(message.clone()),
            _ => None,
        })
        .collect()
}

const FROZEN_C69_GOLDEN: &str = include_str!("../tests/fixtures/task10_terminal_transcripts.txt");

fn frozen_c69_with_current_product_identity() -> String {
    // The detached C69 transcript remains immutable.  R-Term deliberately
    // changes only the product identity bytes in the existing answerback and
    // version responses, so adapt the expected text at the comparison edge.
    FROZEN_C69_GOLDEN.replace("522d535348", "522d5465726d")
}

fn encode_traces(traces: &[V2Trace]) -> String {
    let mut output = String::new();
    writeln!(output, "baseline_sha={LEGACY_BASELINE_SHA}").expect("write string");
    writeln!(output, "baseline_tree={LEGACY_BASELINE_TREE}").expect("write string");
    for source in LEGACY_SOURCES {
        writeln!(
            output,
            "source={}|{}|{}|{}",
            source.source_path, source.migrated_path, source.blob, source.test_count
        )
        .expect("write string");
    }
    for evidence in LEGACY_EVIDENCE {
        writeln!(output, "evidence={}|{}", evidence.path, evidence.blob).expect("write string");
    }
    writeln!(
        output,
        "legacy_test_manifest_entries={}",
        LEGACY_TEST_MANIFEST.lines().count()
    )
    .expect("write string");
    writeln!(output, "approved_semantic_delta=osc52_selection_preserved").expect("write string");
    writeln!(output, "v2_extension=finish_into").expect("write string");
    for trace in traces {
        writeln!(output, "fixture={}", trace.fixture).expect("write string");
        for (index, step) in trace.steps.iter().enumerate() {
            writeln!(output, "step={index}").expect("write string");
            match step {
                V2Step::Feed { input, delta } => {
                    writeln!(output, "operation=feed:{}", hex(input)).expect("write string");
                    write_feed(&mut output, delta);
                }
                V2Step::Resize {
                    size,
                    outcome,
                    snapshot,
                } => {
                    writeln!(
                        output,
                        "operation=resize:{}x{};outcome={}",
                        size.columns,
                        size.rows,
                        resize_outcome(*outcome)
                    )
                    .expect("write string");
                    write_snapshot(&mut output, snapshot);
                }
            }
        }
        writeln!(output, "final_snapshot").expect("write string");
        write_snapshot(&mut output, &trace.final_snapshot);
        writeln!(output, "end_fixture").expect("write string");
    }
    output
}

fn write_feed(output: &mut String, feed: &V2Feed) {
    write_hex_list(output, "responses", &feed.shared.responses);
    writeln!(output, "visible={}", hex(&feed.shared.visible)).expect("write string");
    write_damage(output, &feed.shared.damage);
    writeln!(output, "bells={}", feed.shared.bells).expect("write string");
    write_string_list(output, "clipboard_writes", &feed.shared.clipboard_writes);
    write_string_list(output, "clipboard_reads", &feed.shared.clipboard_reads);
    write_notifications(output, &feed.shared.notifications);
    write_string_list(output, "diagnostics", &feed.shared.diagnostics);
    writeln!(
        output,
        "flags=identity:{};snapshot:{}",
        u8::from(feed.shared.screen_identity_changed),
        u8::from(feed.snapshot_changed)
    )
    .expect("write string");
    write_effects(output, &feed.effects);
    write_metadata(output, &feed.metadata);
    write_snapshot(output, &feed.shared.snapshot);
}

const fn resize_outcome(outcome: TerminalResizeOutcome) -> &'static str {
    match outcome {
        TerminalResizeOutcome::Unchanged => "unchanged",
        TerminalResizeOutcome::MainScreenReflowed => "main-screen-reflowed",
        TerminalResizeOutcome::AlternateScreenResized => "alternate-screen-resized",
        TerminalResizeOutcome::PhysicalResize => "physical-resize",
    }
}

fn write_hex_list(output: &mut String, name: &str, values: &[Vec<u8>]) {
    write!(output, "{name}=").expect("write string");
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&hex(value));
    }
    output.push('\n');
}

fn write_string_list(output: &mut String, name: &str, values: &[String]) {
    write!(output, "{name}=").expect("write string");
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&hex(value.as_bytes()));
    }
    output.push('\n');
}

fn write_damage(output: &mut String, damage: &[DamageSpan]) {
    output.push_str("damage=");
    for (index, span) in damage.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(output, "{}:{}-{}", span.row, span.start, span.end).expect("write string");
    }
    output.push('\n');
}

fn write_notifications(output: &mut String, notifications: &[TerminalNotification]) {
    output.push_str("notifications=");
    for (index, notification) in notifications.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            output,
            "{}:{}",
            opt_hex(notification.title.as_deref()),
            hex(notification.body.as_bytes())
        )
        .expect("write string");
    }
    output.push('\n');
}

fn write_effects(output: &mut String, effects: &[OwnedEffect]) {
    output.push_str("effects=");
    for (index, effect) in effects.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        match effect {
            OwnedEffect::Console(bytes) => write!(output, "console:{}", hex(bytes)),
            OwnedEffect::Transport(bytes) => write!(output, "transport:{}", hex(bytes)),
            OwnedEffect::Mode(change) => write!(output, "mode:{change:?}"),
            OwnedEffect::Bell(count) => write!(output, "bell:{count}"),
            OwnedEffect::ClipboardWrite {
                selection,
                contents,
            } => write!(
                output,
                "clipboard-write:{}:{}",
                opt_hex(selection.as_deref()),
                hex(contents.as_bytes())
            ),
            OwnedEffect::ClipboardRead(selection) => {
                write!(output, "clipboard-read:{}", hex(selection.as_bytes()))
            }
            OwnedEffect::Notification { title, body } => write!(
                output,
                "notification:{}:{}",
                opt_hex(title.as_deref()),
                hex(body.as_bytes())
            ),
            OwnedEffect::Diagnostic(message) => {
                write!(output, "diagnostic:{}", hex(message.as_bytes()))
            }
        }
        .expect("write string");
    }
    output.push('\n');
}

fn write_metadata(output: &mut String, metadata: &OwnedMetadataDelta) {
    writeln!(
        output,
        "metadata=title:{};cwd:{};badge:{};progress:{}",
        metadata_change(metadata.title.as_ref()),
        metadata_change(metadata.working_directory.as_ref()),
        metadata_change(metadata.badge_format.as_ref()),
        metadata
            .progress
            .map_or_else(|| "-".to_owned(), |value| format!("{value:?}"))
    )
    .expect("write string");
    output.push_str("metadata_user_vars=");
    for (index, (name, value)) in metadata.user_vars.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            output,
            "{}:{}",
            hex(name.as_bytes()),
            metadata_change(Some(value))
        )
        .expect("write string");
    }
    output.push('\n');
}

fn metadata_change(change: Option<&OwnedMetadataChange>) -> String {
    match change {
        None => "-".to_owned(),
        Some(OwnedMetadataChange::Clear) => "clear".to_owned(),
        Some(OwnedMetadataChange::Set(value)) => format!("set:{}", hex(value.as_bytes())),
    }
}

fn write_snapshot(output: &mut String, snapshot: &CanonicalSnapshot) {
    let dimensions = snapshot.dimensions;
    writeln!(
        output,
        "snapshot_dimensions={:?};{};{};{};{}",
        dimensions.domain,
        dimensions.viewport_rows,
        dimensions.scrollback_rows,
        dimensions.scrollback_top,
        dimensions.physical_top
    )
    .expect("write string");
    write_rows(output, &snapshot.rows);
    write_snapshot_terminal_state(output, snapshot);
    write_snapshot_metadata(output, snapshot);
    write_snapshot_semantics(output, snapshot);
    write_snapshot_images(output, snapshot);
    write_snapshot_modes(output, snapshot);
}

fn write_rows(output: &mut String, rows: &[CanonicalRow]) {
    for row in rows {
        write!(output, "row={};{};", row.stable_row, u8::from(row.wrapped)).expect("write string");
        for (index, run) in row.cells.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            write!(output, "{}*", run.len).expect("write string");
            write_cell(output, &run.cell);
        }
        output.push('\n');
    }
}

fn write_snapshot_terminal_state(output: &mut String, snapshot: &CanonicalSnapshot) {
    writeln!(
        output,
        "terminal=cursor:{}:{};visible:{};blinking:{};shape:{:?};reverse:{};alternate:{};identity:{};scroll:{}:{};margins:{}:{}",
        snapshot.cursor.0,
        snapshot.cursor.1,
        snapshot.cursor_visible,
        snapshot.cursor_blinking,
        snapshot.cursor_shape,
        snapshot.screen_reverse,
        snapshot.alternate_screen,
        snapshot.screen_identity,
        snapshot.scroll_region.0,
        snapshot.scroll_region.1,
        snapshot.left_right_margins.0,
        snapshot.left_right_margins.1
    )
    .expect("write string");
    output.push_str("active_style=");
    write_cell(output, &snapshot.active_style);
    output.push('\n');
}

fn write_snapshot_metadata(output: &mut String, snapshot: &CanonicalSnapshot) {
    writeln!(
        output,
        "terminal_metadata=title:{};icon:{};window:{};cwd:{};badge:{};unicode:{};progress:{:?}",
        opt_hex(snapshot.title.as_deref()),
        opt_hex(snapshot.icon_title.as_deref()),
        opt_hex(snapshot.window_title.as_deref()),
        opt_hex(snapshot.working_directory.as_deref()),
        opt_hex(snapshot.badge_format.as_deref()),
        snapshot.unicode_version,
        snapshot.progress
    )
    .expect("write string");
    output.push_str("terminal_user_vars=");
    for (index, (name, value)) in snapshot.user_vars.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(output, "{}:{}", hex(name.as_bytes()), hex(value.as_bytes())).expect("write string");
    }
    output.push('\n');
}

fn write_snapshot_semantics(output: &mut String, snapshot: &CanonicalSnapshot) {
    write!(output, "semantic_prompts=").expect("write string");
    write_numbers(output, &snapshot.semantic_prompt_rows);
    output.push('\n');
    output.push_str("semantic_exits=");
    for (index, exit) in snapshot.semantic_command_exits.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            output,
            "{}:{}:{}",
            exit.row,
            exit.exit_code
                .map_or_else(|| "-".to_owned(), |value| value.to_string()),
            opt_hex(exit.aid.as_deref())
        )
        .expect("write string");
    }
    output.push('\n');
    output.push_str("semantic_zones=");
    for (index, zone) in snapshot.semantic_zones.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            output,
            "{}:{}-{}:{}:{:?}",
            zone.start_y, zone.start_x, zone.end_y, zone.end_x, zone.semantic_type
        )
        .expect("write string");
    }
    output.push('\n');
}

fn write_snapshot_images(output: &mut String, snapshot: &CanonicalSnapshot) {
    writeln!(output, "inline_images={}", snapshot.inline_images.len()).expect("write string");
    for image in &snapshot.inline_images {
        write_image(output, image);
    }
    output.push_str("attachments=");
    for (index, attachment) in snapshot.inline_image_attachments.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            output,
            "{}:{}:{}:{}:{}",
            attachment.parent_identity,
            attachment.source_row,
            attachment.source_column,
            attachment.row,
            attachment.column
        )
        .expect("write string");
    }
    output.push('\n');
    writeln!(
        output,
        "fragments={}",
        snapshot.inline_image_fragments.len()
    )
    .expect("write string");
    for fragment in &snapshot.inline_image_fragments {
        write_fragment(output, fragment);
    }
}

fn write_image(output: &mut String, image: &ItermInlineImage) {
    writeln!(
        output,
        "image={}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        image.row,
        image.column,
        opt_hex(image.name.as_deref()),
        opt_number(image.kitty_image_id),
        opt_number(image.kitty_placement_id),
        image
            .kitty_z_index
            .map_or_else(|| "-".to_owned(), |value| value.to_string()),
        opt_number(image.size),
        opt_hex(image.width.as_deref()),
        opt_hex(image.height.as_deref()),
        image
            .preserve_aspect_ratio
            .map_or_else(|| "-".to_owned(), |value| u8::from(value).to_string()),
        image_format(image.image_format),
        opt_number(image.pixel_width),
        opt_number(image.pixel_height),
        opt_number(image.source_x),
        opt_number(image.source_y),
        opt_number(image.source_width),
        opt_number(image.source_height),
        opt_number(image.target_x),
        opt_number(image.target_y),
        hex(&image.data),
        image.data.len()
    )
    .expect("write string");
}

fn write_fragment(output: &mut String, fragment: &InlineImageFragment) {
    let numeric = [
        fragment.image_index.to_string(),
        u8::from(fragment.cell_attachment).to_string(),
        fragment.row.to_string(),
        fragment.column.to_string(),
        fragment.source_row.to_string(),
        fragment.source_column.to_string(),
        fragment.destination_x.to_string(),
        fragment.destination_y.to_string(),
        fragment.destination_width.to_string(),
        fragment.destination_height.to_string(),
        fragment.source_x.to_string(),
        fragment.source_y.to_string(),
        fragment.source_width.to_string(),
        fragment.source_height.to_string(),
        fragment.sampling_source_x.to_string(),
        fragment.sampling_source_y.to_string(),
        fragment.sampling_source_width.to_string(),
        fragment.sampling_source_height.to_string(),
        fragment.source_destination_x.to_string(),
        fragment.source_destination_y.to_string(),
        fragment.source_destination_width.to_string(),
        fragment.source_destination_height.to_string(),
    ];
    writeln!(
        output,
        "fragment={}:{}:{}:{}:{}",
        numeric.join(":"),
        opt_number(fragment.kitty_image_id),
        opt_number(fragment.kitty_placement_id),
        fragment
            .kitty_z_index
            .map_or_else(|| "-".to_owned(), |value| value.to_string()),
        image_format(fragment.image_format)
    )
    .expect("write string");
}

fn write_snapshot_modes(output: &mut String, snapshot: &CanonicalSnapshot) {
    writeln!(
        output,
        "modes=cursor:{};keypad:{};focus:{};paste:{};sync:{};kitty:{};modify:{};win32:{};mouse:{:?}",
        snapshot.application_cursor_keys,
        snapshot.application_keypad,
        snapshot.focus_reporting,
        snapshot.bracketed_paste,
        snapshot.synchronized_output,
        snapshot.kitty_keyboard_flags,
        snapshot.modify_other_keys,
        snapshot.win32_input_mode,
        snapshot.mouse_input_mode
    )
    .expect("write string");
}

fn write_cell(output: &mut String, cell: &Cell) {
    match cell.content() {
        CellContent::Blank => output.push('b'),
        CellContent::Text { grapheme, columns } => {
            write!(output, "t{}:{}", columns, hex(grapheme.as_bytes())).expect("write string");
        }
        CellContent::Continuation { leader_delta } => {
            write!(output, "c{leader_delta}").expect("write string");
        }
    }
    write!(
        output,
        "/{}/{}/{}/{}/{:x}/{}/{}/{}",
        color(cell.foreground),
        color(cell.background),
        color(cell.underline_color),
        underline_style(cell.underline_style),
        cell_attribute_bits(cell),
        vertical_align(cell.vertical_align),
        opt_hex(cell.hyperlink()),
        semantic_type(cell.semantic_type)
    )
    .expect("write string");
}

fn cell_attribute_bits(cell: &Cell) -> u16 {
    u16::from(cell.bold)
        | (u16::from(cell.faint) << 1)
        | (u16::from(cell.italic) << 2)
        | (u16::from(cell.blink) << 3)
        | (u16::from(cell.rapid_blink) << 4)
        | (u16::from(cell.underline) << 5)
        | (u16::from(cell.double_underline) << 6)
        | (u16::from(cell.conceal) << 7)
        | (u16::from(cell.strikethrough) << 8)
        | (u16::from(cell.overline) << 9)
        | (u16::from(cell.inverse) << 10)
        | (u16::from(cell.protected) << 11)
}

fn color(value: Color) -> String {
    match value {
        Color::Default => "d".to_owned(),
        Color::Indexed(index) => format!("i{index}"),
        Color::Rgb(red, green, blue) => format!("r{red}:{green}:{blue}"),
        Color::Rgba(red, green, blue, alpha) => format!("a{red}:{green}:{blue}:{alpha}"),
    }
}

const fn underline_style(value: UnderlineStyle) -> &'static str {
    match value {
        UnderlineStyle::None => "none",
        UnderlineStyle::Single => "single",
        UnderlineStyle::Double => "double",
        UnderlineStyle::Curly => "curly",
        UnderlineStyle::Dotted => "dotted",
        UnderlineStyle::Dashed => "dashed",
    }
}

const fn vertical_align(value: VerticalAlign) -> &'static str {
    match value {
        VerticalAlign::Baseline => "base",
        VerticalAlign::Superscript => "super",
        VerticalAlign::Subscript => "sub",
    }
}

const fn semantic_type(value: SemanticType) -> &'static str {
    match value {
        SemanticType::Output => "output",
        SemanticType::Prompt => "prompt",
        SemanticType::Input => "input",
    }
}

const fn image_format(value: InlineImageFormat) -> &'static str {
    match value {
        InlineImageFormat::Encoded => "encoded",
        InlineImageFormat::Rgb => "rgb",
        InlineImageFormat::Rgba => "rgba",
    }
}

fn write_numbers<T: std::fmt::Display>(output: &mut String, values: &[T]) {
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(output, "{value}").expect("write string");
    }
}

fn opt_hex(value: Option<&str>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| hex(value.as_bytes()))
}

fn opt_number<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| value.to_string())
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        write!(output, "{byte:02x}").expect("write string");
    }
    output
}

#[test]
fn legacy_and_v2_match_complete_frozen_transcripts() {
    let mut actual = Vec::new();
    for fixture in fixtures() {
        let (legacy_steps, legacy_final) = run_legacy(&fixture);
        let v2 = run_v2(&fixture);
        assert_eq!(v2.steps.len(), legacy_steps.len(), "{}", fixture.name);
        for (index, (v2_step, legacy_step)) in v2.steps.iter().zip(&legacy_steps).enumerate() {
            match (v2_step, legacy_step) {
                (
                    V2Step::Feed {
                        input: v2_input,
                        delta,
                    },
                    LegacyStep::Feed {
                        input: legacy_input,
                        output,
                    },
                ) => {
                    assert_eq!(v2_input, legacy_input, "{} step {index}", fixture.name);
                    assert_eq!(delta.shared, **output, "{} step {index}", fixture.name);
                }
                (
                    V2Step::Resize {
                        size: v2_size,
                        outcome: v2_outcome,
                        snapshot: v2_snapshot,
                    },
                    LegacyStep::Resize {
                        size: legacy_size,
                        outcome: legacy_outcome,
                        snapshot: legacy_snapshot,
                    },
                ) => {
                    assert_eq!(v2_size, legacy_size, "{} step {index}", fixture.name);
                    assert_eq!(v2_outcome, legacy_outcome, "{} step {index}", fixture.name);
                    assert_eq!(
                        v2_snapshot, legacy_snapshot,
                        "{} step {index}",
                        fixture.name
                    );
                }
                _ => panic!("{} step {index} operation mismatch", fixture.name),
            }
        }
        assert_eq!(v2.final_snapshot, legacy_final, "{} final", fixture.name);
        actual.push(v2);
    }

    let encoded = encode_traces(&actual);
    assert_eq!(encoded, frozen_c69_with_current_product_identity());
}

#[test]
fn legacy_manifest_is_signed_and_catalog_has_unique_behavior_mappings() {
    assert_eq!(LEGACY_BASELINE_SHA.len(), 40);
    assert_eq!(LEGACY_BASELINE_TREE.len(), 40);
    assert!(LEGACY_EVIDENCE.iter().all(|entry| entry.blob.len() == 40));
    assert_eq!(
        LEGACY_SOURCES
            .iter()
            .map(|source| source.test_count)
            .sum::<usize>(),
        356
    );
    assert_eq!(TRANSCRIPT_CATALOG.len(), 11);
    assert_eq!(
        TRANSCRIPT_CATALOG
            .iter()
            .map(|entry| entry.mapped_tests)
            .sum::<usize>(),
        350
    );
    assert_eq!(NON_TRANSCRIPT_TESTS.len(), 6);
    for (index, entry) in TRANSCRIPT_CATALOG.iter().enumerate() {
        assert!(
            !TRANSCRIPT_CATALOG[..index]
                .iter()
                .any(|earlier| earlier.behavior_id == entry.behavior_id),
            "duplicate behavior mapping: {}",
            entry.behavior_id
        );
        assert!(!entry.fixture.is_empty());
        assert!(!entry.legacy_anchor.is_empty());
    }
    for (index, entry) in NON_TRANSCRIPT_TESTS.iter().enumerate() {
        assert!(
            !NON_TRANSCRIPT_TESTS[..index]
                .iter()
                .any(|earlier| earlier.test_name == entry.test_name),
            "duplicate exclusion: {}",
            entry.test_name
        );
        assert!(!entry.reason.is_empty());
    }

    let rows = LEGACY_TEST_MANIFEST.lines().collect::<Vec<_>>();
    assert_eq!(rows.len(), 356);
    let mut mapped_counts = vec![0; TRANSCRIPT_CATALOG.len()];
    let mut excluded_counts = vec![0; NON_TRANSCRIPT_TESTS.len()];
    for (row_index, row) in rows.iter().enumerate() {
        let fields = row.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 3, "manifest row {row_index}");
        let behavior = fields[0];
        let source_path = fields[1];
        let test_name = fields[2];
        assert!(
            LEGACY_SOURCES
                .iter()
                .any(|source| source.source_path == source_path),
            "unknown source: {source_path}"
        );
        assert!(
            !rows[..row_index].iter().any(|earlier| {
                let mut fields = earlier.split('|');
                let _ = fields.next();
                fields.next() == Some(source_path) && fields.next() == Some(test_name)
            }),
            "duplicate legacy test: {source_path}::{test_name}"
        );
        if let Some(reason) = behavior.strip_prefix("NON-TRANSCRIPT:") {
            let index = NON_TRANSCRIPT_TESTS
                .iter()
                .position(|entry| entry.test_name == test_name && entry.reason == reason)
                .unwrap_or_else(|| panic!("unapproved exclusion: {test_name}"));
            excluded_counts[index] += 1;
        } else {
            let index = TRANSCRIPT_CATALOG
                .iter()
                .position(|entry| entry.behavior_id == behavior)
                .unwrap_or_else(|| panic!("unknown behavior: {behavior}"));
            mapped_counts[index] += 1;
        }
    }
    assert_eq!(
        mapped_counts,
        TRANSCRIPT_CATALOG
            .iter()
            .map(|entry| entry.mapped_tests)
            .collect::<Vec<_>>()
    );
    assert_eq!(excluded_counts, vec![1; NON_TRANSCRIPT_TESTS.len()]);
}

#[test]
fn legacy_parser_fixture_catalog_is_complete() {
    let records = fixture_records();
    for (source_path, _, expected_count) in REQUIRED_FIXTURE_SOURCES {
        let actual_count = records
            .iter()
            .filter(|record| record.source_path == *source_path)
            .count();
        assert_eq!(
            actual_count, *expected_count,
            "frozen trace count for {source_path}"
        );
    }
    assert_eq!(records.len(), 356);
}

#[test]
fn current_task10_capture_routes_compatibility_fixture_through_v2() {
    let row_id = "0000000000000000000000000000000000000000000000000000000000000000";
    let (execution, trace) = crate::fixture_trace::capture(row_id, "runtime", || {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(8, 2));
        let output = runtime.feed_pty_output_with_display(b"abc\x1b[6n");
        assert_eq!(output.responses, [b"\x1b[1;4R".to_vec()]);
    });
    assert!(execution.is_ok());
    let trace = String::from_utf8(trace).expect("fixture trace UTF-8");
    assert!(trace.contains("api=runtime.feed"));
    assert!(trace.contains("domain=runtime"));
    assert!(trace.contains("final_object=runtime:"));
    assert!(trace.contains("bytes=6162631b5b366e"));
}

#[test]
fn runtime_filter_trace_records_every_process_call_including_empty_callbacks() {
    let record = fixture_records()
        .iter()
        .find(|record| {
            record.current_test_name
                == "gui_filter_passes_malformed_modes_and_fail_closes_reserved_clipboard"
        })
        .expect("runtime-filter fixture record");
    let (execution, trace) = replay_current_fixture(record).expect("runtime-filter replay");
    assert!(execution.is_ok());
    let trace = std::str::from_utf8(&trace).expect("runtime-filter trace UTF-8");
    let process_actions = trace
        .lines()
        .filter(|line| line.starts_with("action=") && line.contains("|api=filter.process|"))
        .collect::<Vec<_>>();
    assert_eq!(process_actions.len(), 2, "one action per process call");

    let second_input = trace_blob(trace, trace_field(process_actions[1], "input"));
    assert_eq!(
        second_input,
        b"\x1b]052;c;not-base64!\x07\x9d00052;c;not-base64!\x9c\xc2\x9d052;c;not-base64!\xc2\x9c\x1b]001337;Copy=;not-base64!\x07"
    );
    let sequence = trace_field(process_actions[1], "action");
    let observables = trace
        .lines()
        .find(|line| {
            line.starts_with("observables=") && trace_field(line, "observables") == sequence
        })
        .expect("second process observables");
    assert_eq!(
        trace_blob(trace, trace_field(observables, "typed")),
        b"callbacks=0;events="
    );
}

fn trace_field<'line>(line: &'line str, field: &str) -> &'line str {
    line.split('|')
        .find_map(|part| {
            part.strip_prefix(field)
                .and_then(|value| value.strip_prefix('='))
        })
        .unwrap_or_else(|| panic!("missing {field} in {line}"))
}

fn trace_blob(trace: &str, reference: &str) -> Vec<u8> {
    let index = reference
        .strip_prefix("blob:")
        .unwrap_or_else(|| panic!("invalid blob reference {reference}"));
    let prefix = format!("blob={index}|");
    let line = trace
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("missing {reference}"));
    let bytes = trace_field(line, "bytes").as_bytes();
    assert_eq!(bytes.len() % 2, 0, "hex byte length");
    bytes
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("hex UTF-8");
            u8::from_str_radix(pair, 16).expect("hex byte")
        })
        .collect()
}

#[test]
fn every_manifest_row_has_a_frozen_execution_trace() {
    let records = fixture_records();
    for row in LEGACY_TEST_MANIFEST.lines() {
        let fields = row.split('|').collect::<Vec<_>>();
        let behavior_id = fields[0];
        let source_path = fields[1];
        let test_name = fields[2];
        let matches = records
            .iter()
            .filter(|record| record.source_path == source_path && record.test_name == test_name)
            .collect::<Vec<_>>();
        assert_eq!(
            matches.len(),
            1,
            "missing or duplicate frozen trace for {source_path}::{test_name}"
        );
        assert_eq!(matches[0].behavior_id, behavior_id);
    }
    for (index, record) in records.iter().enumerate() {
        assert!(
            !records[..index].iter().any(|earlier| {
                earlier.source_path == record.source_path && earlier.test_name == record.test_name
            }),
            "duplicate frozen fixture: {}::{}",
            record.source_path,
            record.test_name
        );
    }
}

#[test]
fn current_fixture_traces_match_frozen_c69_with_rterm_identity() {
    let records = fixture_records();
    assert_eq!(
        records.len(),
        356,
        "current replay cannot cover missing detached-c69 records"
    );
    let replayable = records
        .iter()
        .filter(|record| record.domain != "terminal_parser")
        .collect::<Vec<_>>();
    assert_eq!(replayable.len(), 217, "runtime-owned current registry rows");
    for record in replayable {
        assert_fixture_record(record);
        let (execution, current_trace) = replay_current_fixture(record)
            .unwrap_or_else(|| panic!("missing current replay adapter for {}", record.row_id));
        if record.current_test_name == "answers_xtversion_queries_without_feeding_them_to_terminal"
        {
            // Only the three version replies (and their transport effects) change
            // from R-SSH to R-Term. The original 356 frozen traces stay untouched.
            assert_eq!(
                crate::frozen_trace_pack::sha256_hex(&current_trace),
                "cdf81ea7662bd41d28eb753ac54bb6c31c0a359d55aa5a5e3adb92d6064ee29a",
                "R-Term version reply trace"
            );
        } else {
            crate::frozen_trace_pack::assert_current_trace(record, &current_trace);
        }
        if let Err(payload) = execution {
            std::panic::resume_unwind(payload);
        }
    }
}

#[test]
fn every_current_test_body_is_bound_to_its_frozen_record() {
    let records = fixture_records();
    let mut consumed = HashSet::with_capacity(records.len());
    let sources = [
        (
            "crates/rssh-runtime/src/terminal.rs",
            include_str!("terminal.rs"),
        ),
        (
            "crates/rssh-runtime/src/queries.rs",
            include_str!("queries.rs"),
        ),
        (
            "crates/rssh-runtime/src/query_dcs.rs",
            include_str!("query_dcs.rs"),
        ),
        (
            "crates/rssh-terminal/src/parser.rs",
            include_str!("../../rterm-terminal/src/parser.rs"),
        ),
    ];
    for (path, source) in sources {
        // Historical source-body identities retain their original crate alias.
        let historical_aliases = source.replace("rterm_terminal", "rssh_terminal");
        let bodies = crate::test_body_digest::test_body_sha256s(&historical_aliases);
        let path_records = records
            .iter()
            .filter(|record| record.current_path == path)
            .collect::<Vec<_>>();
        assert!(!path_records.is_empty(), "fixture records for {path}");
        for record in path_records {
            assert!(
                consumed.insert(record.row_id),
                "current body record consumed twice: {}",
                record.row_id
            );
            let current = bodies.get(record.current_test_name).unwrap_or_else(|| {
                panic!("missing current test body: {}", record.current_test_name)
            });
            assert_current_test_body(record, current);
        }
    }
    assert_eq!(
        consumed.len(),
        records.len(),
        "every frozen row must bind to exactly one current test body"
    );
    assert_eq!(
        records
            .iter()
            .filter(|record| record.migration.starts_with("approved:"))
            .count(),
        6,
        "reviewed source-body migrations"
    );
}

#[test]
fn frozen_transcript_records_every_feed_and_resize_operation() {
    let traces = fixtures().iter().map(run_v2).collect::<Vec<_>>();
    let encoded = encode_traces(&traces);

    assert!(encoded.contains("operation=feed:6162"));
    assert!(encoded.contains("operation=resize:6x2"));
}

#[test]
fn response_catalog_freezes_all_legacy_response_families() {
    let trace = run_v2(&response_catalog_fixture());
    let responses = trace
        .steps
        .iter()
        .filter_map(|step| match step {
            V2Step::Feed { delta, .. } => Some(delta.shared.responses.clone()),
            V2Step::Resize { .. } => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(responses.len(), 5);
    assert!(responses[0].is_empty());
    assert_eq!(
        responses[1],
        [
            b"\x1b[1;4R".to_vec(),
            b"\x1b[?65;4;6;18;22;52c".to_vec(),
            b"\x1b[>1;277;0c".to_vec(),
            b"\x1bP!|00000000\x1b\\".to_vec(),
            b"\x1b[2;1;1;128;128;1;0x".to_vec(),
            b"\x1b[3;1;1;128;128;1;0x".to_vec(),
            b"\x1b[0n".to_vec(),
            b"\x1b[4;48;64t".to_vec(),
            b"\x1b[6;16;8t".to_vec(),
            b"\x1b[8;3;8t".to_vec(),
            b"\x1b]lcatalog-title\x1b\\".to_vec(),
            b"\x1b[?25;2$y".to_vec(),
            b"\x1b[4;1$y".to_vec(),
            b"\x1b]1337;ReportCellSize=16.0;8.0\x1b\\".to_vec(),
            b"\x1bP7!~00c6\x1b\\".to_vec(),
            b"\x1b[?1u".to_vec(),
            b"\x1b[>4;2m".to_vec(),
        ]
    );
    assert_eq!(
        responses[2],
        [
            b"\x1bP1$r0m\x1b\\".to_vec(),
            b"\x1bP1+r436F=323536\x1b\\".to_vec(),
            b"\x1b[?1;0;65536S".to_vec(),
            b"\x1bP>|R-Term 0.1.0\x1b\\".to_vec(),
        ]
    );
    assert_eq!(responses[3], [b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07".to_vec()]);
    assert_eq!(
        responses[4],
        [b"R-Term".to_vec(), b"\x1b_Gi=31;OK\x1b\\".to_vec()]
    );
}

#[test]
fn stable_transcript_encoding_detects_each_observable_surface_mutation() {
    let traces = fixtures().iter().map(run_v2).collect::<Vec<_>>();
    let encoded = encode_traces(&traces);

    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).shared.responses[0].push(b'!');
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).shared.visible.push(b'!');
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).shared.damage[0].end += 1;
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).shared.bells += 1;
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).shared.clipboard_writes[0].push('!');
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).shared.notifications[0].title = None;
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).shared.diagnostics[0].push('!');
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).shared.screen_identity_changed = true;
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).effects.swap(0, 1);
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        let feed = first_feed_mut(traces);
        let OwnedEffect::ClipboardWrite { selection, .. } = &mut feed.effects[2] else {
            panic!("fixture clipboard effect");
        };
        *selection = None;
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).metadata.title = Some(OwnedMetadataChange::Clear);
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).snapshot_changed = false;
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        first_feed_mut(traces).shared.snapshot.cursor.1 += 1;
    });
    assert_trace_mutation_changes(&traces, &encoded, |traces| {
        traces[0].final_snapshot.progress = TerminalProgress::None;
    });
}

fn first_feed_mut(traces: &mut [V2Trace]) -> &mut V2Feed {
    let V2Step::Feed { delta, .. } = &mut traces[0].steps[0] else {
        panic!("first transcript step must be a feed");
    };
    delta.as_mut()
}

fn assert_trace_mutation_changes(
    traces: &[V2Trace],
    encoded: &str,
    mutate: impl FnOnce(&mut [V2Trace]),
) {
    let mut mutated = traces.to_vec();
    mutate(&mut mutated);
    assert_ne!(encode_traces(&mutated), encoded);
}
