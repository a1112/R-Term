use rterm_runtime::{
    MetadataChangeRef, RuntimeBuffers, RuntimeEffectRef, RuntimeProgress, TerminalModeChange,
    TerminalRuntime,
};
use rterm_types::TerminalSize;

fn captured_console(delta: rterm_runtime::RuntimeDelta<'_>) -> Vec<u8> {
    delta
        .console_writes()
        .flat_map(|bytes| bytes.iter().copied())
        .collect()
}

#[test]
fn local_console_passthrough_is_distinct_from_loggable_visible_text() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    runtime.set_capture_host_stream(true);
    let mut buffers = RuntimeBuffers::default();
    let input = b"A\x1b[31mred\x1b[0m\x07\x1b]2;title\x07B";

    let delta = runtime.feed_into(input, &mut buffers);

    assert_eq!(captured_console(delta), input);
    assert_eq!(delta.visible_bytes(), b"AredB");
}

#[test]
fn local_console_suppresses_host_controls_and_keeps_enq() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    runtime.set_capture_host_stream(true);
    let mut buffers = RuntimeBuffers::default();
    let input =
        b"A\x1b[6nB\x1b]8;;https://example.test\x07C\x1b[?1;2cD\x1b]52;c;aGVsbG8=\x07E\x05F";

    let delta = runtime.feed_into(input, &mut buffers);

    assert_eq!(captured_console(delta), b"ABCDE\x05F");
    assert_eq!(delta.responses().count(), 1);
}

#[test]
fn local_console_resynchronization_publishes_escaped_mode_once() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    runtime.set_capture_host_stream(true);
    let mut buffers = RuntimeBuffers::default();

    let delta = runtime.feed_into(b"\x1b]0;hidden\x1b[?2004h\x07", &mut buffers);

    assert_eq!(
        delta.mode_changes().collect::<Vec<_>>(),
        [TerminalModeChange::BracketedPaste(true)]
    );
    assert!(runtime.bracketed_paste());
}

#[test]
fn local_console_effects_preserve_source_order() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    runtime.set_capture_host_stream(true);
    let mut buffers = RuntimeBuffers::default();
    let delta = runtime.feed_into(
        b"A\x1b[6nB\x1b]52;c;aGVsbG8=\x07C\x1b]52;c;?\x07D",
        &mut buffers,
    );

    assert_eq!(
        delta.effects().collect::<Vec<_>>(),
        vec![
            RuntimeEffectRef::ConsoleWrite(b"A"),
            RuntimeEffectRef::TransportWrite(b"\x1b[1;2R"),
            RuntimeEffectRef::ConsoleWrite(b"B"),
            RuntimeEffectRef::ClipboardWrite {
                selection: Some("c"),
                contents: "hello",
            },
            RuntimeEffectRef::ConsoleWrite(b"C"),
            RuntimeEffectRef::ClipboardRead { selection: "c" },
            RuntimeEffectRef::ConsoleWrite(b"D"),
        ]
    );
}

#[test]
fn local_console_sync_and_finish_release_held_output_without_incomplete_controls() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    runtime.set_capture_host_stream(true);
    let mut buffers = RuntimeBuffers::default();

    let held = runtime.feed_into(b"\x1b[?2026habc", &mut buffers);
    assert!(captured_console(held).is_empty());
    assert_eq!(
        held.mode_changes().collect::<Vec<_>>(),
        vec![TerminalModeChange::SynchronizedOutput(true)]
    );

    let released = runtime.feed_into(b"q\x1b[6nr\x1b[?2026l", &mut buffers);
    assert_eq!(
        released.effects().collect::<Vec<_>>(),
        vec![
            RuntimeEffectRef::TransportWrite(b"\x1b[1;5R"),
            RuntimeEffectRef::ModeChange(TerminalModeChange::SynchronizedOutput(false)),
            RuntimeEffectRef::ConsoleWrite(b"abcqr"),
        ]
    );

    let held_again = runtime.feed_into(b"\x1b[?2026hZ\x1b[6", &mut buffers);
    assert!(captured_console(held_again).is_empty());
    let finished = runtime.finish_into(&mut buffers);
    assert_eq!(captured_console(finished), b"Z");
}

#[test]
fn osc_color_query_is_consumed_once_without_terminal_display_mutation() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();
    runtime.feed_into(b"seed", &mut buffers);
    let sequence_before = runtime.terminal().current_seqno();

    {
        let delta = runtime.feed_into(b"\x1b]10;?\x07", &mut buffers);
        assert_eq!(
            delta.responses().collect::<Vec<_>>(),
            vec![b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07".as_slice()]
        );
        assert_eq!(delta.visible_bytes(), b"");
        assert_eq!(delta.damage(), &[]);
        assert!(!delta.snapshot_changed());
    }

    assert_eq!(buffers.response_commits(), 1);
    assert_eq!(buffers.response_payload_copies(), 0);
    assert_eq!(buffers.owned_response_materializations(), 0);
    assert_eq!(runtime.terminal().current_seqno(), sequence_before);
}

#[test]
fn delta_preserves_response_and_effect_order() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();
    let delta = runtime.feed_into(
        b"a\x1b[6n\x07\x1b]52;c;aGVsbG8=\x07\x1b]52;c;?\x07\x1b[5n",
        &mut buffers,
    );

    assert_eq!(
        delta.effects().collect::<Vec<_>>(),
        vec![
            RuntimeEffectRef::TransportWrite(b"\x1b[1;2R"),
            RuntimeEffectRef::Bell { count: 1 },
            RuntimeEffectRef::ClipboardWrite {
                selection: Some("c"),
                contents: "hello",
            },
            RuntimeEffectRef::ClipboardRead { selection: "c" },
            RuntimeEffectRef::TransportWrite(b"\x1b[0n"),
        ]
    );
}

#[derive(Clone, Copy, Debug)]
enum OscFraming {
    SevenBit,
    RawC1,
    Utf8C1,
}

fn osc52_write(selection: &str, framing: OscFraming) -> Vec<u8> {
    let mut bytes = match framing {
        OscFraming::SevenBit => b"\x1b]".to_vec(),
        OscFraming::RawC1 => vec![0x9d],
        OscFraming::Utf8C1 => vec![0xc2, 0x9d],
    };
    bytes.extend_from_slice(b"52;");
    bytes.extend_from_slice(selection.as_bytes());
    bytes.extend_from_slice(b";aGVsbG8=");
    match framing {
        OscFraming::SevenBit => bytes.push(0x07),
        OscFraming::RawC1 => bytes.push(0x9c),
        OscFraming::Utf8C1 => bytes.extend_from_slice(&[0xc2, 0x9c]),
    }
    bytes
}

#[test]
fn osc52_write_preserves_selection_for_every_framing_and_split() {
    for selection in ["c", "p", "pc", ""] {
        for framing in [OscFraming::SevenBit, OscFraming::RawC1, OscFraming::Utf8C1] {
            let input = osc52_write(selection, framing);
            for split in 0..=input.len() {
                let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
                let mut buffers = RuntimeBuffers::default();
                let mut writes = Vec::new();
                for part in [&input[..split], &input[split..]] {
                    let delta = runtime.feed_into(part, &mut buffers);
                    writes.extend(delta.clipboard_writes().map(|(selection, contents)| {
                        (selection.map(str::to_owned), contents.to_owned())
                    }));
                }
                assert_eq!(
                    writes,
                    vec![(Some(selection.to_owned()), "hello".to_owned())],
                    "selection={selection:?}, framing={framing:?}, split={split}"
                );
            }
        }
    }
}

#[test]
fn iterm_copy_has_no_osc52_selection() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();
    let delta = runtime.feed_into(b"]1337;Copy=;aGVsbG8=", &mut buffers);
    assert_eq!(
        delta.clipboard_writes().collect::<Vec<_>>(),
        vec![(None, "hello")]
    );
}

#[test]
fn delta_exposes_legacy_host_phase_views_without_losing_source_order() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();
    let delta = runtime.feed_into(
        b"\x1b[?9999z\x1b[6n\x1b]52;c;aGVsbG8=\x07\x1b]52;c;?\x07\x1b]9;done\x07\x07",
        &mut buffers,
    );

    let mut phases = Vec::new();
    phases.extend(delta.diagnostics().map(|_| "diagnostic"));
    phases.extend(delta.responses().map(|_| "response"));
    phases.extend(delta.clipboard_writes().map(|_| "clipboard-write"));
    phases.extend(delta.clipboard_reads().map(|_| "clipboard-read"));
    phases.extend(delta.notifications().map(|_| "notification"));
    if delta.bell_count() != 0 {
        phases.push("bell");
    }

    assert_eq!(
        phases,
        [
            "diagnostic",
            "response",
            "clipboard-write",
            "clipboard-read",
            "notification",
            "bell",
        ]
    );
    assert_eq!(
        delta.effects().collect::<Vec<_>>(),
        vec![
            RuntimeEffectRef::Diagnostic {
                message: "CSI ?9999z",
            },
            RuntimeEffectRef::TransportWrite(b"\x1b[1;1R"),
            RuntimeEffectRef::ClipboardWrite {
                selection: Some("c"),
                contents: "hello",
            },
            RuntimeEffectRef::ClipboardRead { selection: "c" },
            RuntimeEffectRef::Notification {
                title: None,
                body: "done",
            },
            RuntimeEffectRef::Bell { count: 1 },
        ]
    );
}

#[test]
fn synchronized_output_defers_damage_without_effect_loss() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();

    let held = runtime.feed_into(b"\x1b[?2026habc\x07", &mut buffers);
    assert!(held.damage().is_empty());
    assert_eq!(held.visible_bytes(), b"abc");
    assert_eq!(held.bell_count(), 1);

    let released = runtime.feed_into(b"\x1b[?2026ld", &mut buffers);
    assert!(!released.damage().is_empty());
    assert_eq!(released.visible_bytes(), b"d");
}

#[test]
fn finish_releases_synchronized_damage_and_is_idempotent() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    runtime.set_capture_host_stream(true);
    let mut buffers = RuntimeBuffers::default();

    let held = runtime.feed_into(b"\x1b[?2026habc", &mut buffers);
    assert!(held.damage().is_empty());
    assert!(held.console_writes().next().is_none());

    let finished = runtime.finish_into(&mut buffers);
    assert!(!finished.damage().is_empty());
    assert!(finished.snapshot_changed());
    assert_eq!(captured_console(finished), b"abc");
    assert!(
        finished
            .mode_changes()
            .any(|change| change == TerminalModeChange::SynchronizedOutput(false))
    );

    let repeated = runtime.finish_into(&mut buffers);
    assert!(repeated.damage().is_empty());
    assert!(!repeated.snapshot_changed());
    assert!(repeated.effects().next().is_none());
}

#[test]
fn metadata_delta_is_final_only_and_empty_when_unchanged() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();

    let changed = runtime.feed_into(b"\x1b]0;first\x07\x1b]0;final\x07", &mut buffers);
    assert_eq!(
        changed.metadata().title(),
        Some(MetadataChangeRef::Set("final"))
    );

    let unchanged = runtime.feed_into(b"\x1b]0;final\x07", &mut buffers);
    assert!(unchanged.metadata().is_empty());
}

#[test]
fn metadata_delta_reports_only_changed_final_sources() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();
    let input = b"\x1b]7;file://host/tmp\x07\x1b]1337;SetUserVar=FOO=YmFy\x07\x1b]1337;SetBadgeFormat=YmFkZ2U=\x07\x1b]9;4;1;42\x07\x1b]9;4;2\x07";

    let changed = runtime.feed_into(input, &mut buffers);
    let metadata = changed.metadata();
    assert_eq!(
        metadata.working_directory(),
        Some(MetadataChangeRef::Set("file://host/tmp"))
    );
    assert_eq!(
        metadata.badge_format(),
        Some(MetadataChangeRef::Set("badge"))
    );
    assert_eq!(metadata.progress(), Some(RuntimeProgress::Error(0)));
    assert_eq!(
        metadata.user_vars().collect::<Vec<_>>(),
        vec![("FOO", MetadataChangeRef::Set("bar"))]
    );

    let unchanged = runtime.feed_into(input, &mut buffers);
    assert!(unchanged.metadata().is_empty());
}

#[test]
fn legacy_feed_consumes_metadata_before_the_next_v2_batch() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();
    let metadata = b"\x1b]0;legacy-title\x07\x1b]7;file://host/legacy\x07\x1b]1337;SetUserVar=FOO=bGVnYWN5\x07\x1b]1337;SetBadgeFormat=bGVnYWN5LWJhZGdl\x07\x1b]9;4;1;42\x07";

    runtime.feed_pty_output_with_display(metadata);

    let plain = runtime.feed_into(b"plain", &mut buffers);
    assert!(plain.metadata().is_empty());
    assert!(plain.effects().next().is_none());
    assert_eq!(plain.visible_bytes(), b"plain");
    assert_eq!(runtime.snapshot().terminal().title(), Some("legacy-title"));
    assert_eq!(
        runtime.snapshot().terminal().current_working_dir(),
        Some("file://host/legacy")
    );
    assert_eq!(
        runtime.snapshot().terminal().badge_format(),
        Some("legacy-badge")
    );
    assert_eq!(
        runtime
            .snapshot()
            .terminal()
            .user_vars()
            .get("FOO")
            .map(String::as_str),
        Some("legacy")
    );

    let repeated = runtime.feed_into(metadata, &mut buffers);
    assert!(
        repeated.metadata().is_empty(),
        "unchanged sources must not be republished after a legacy feed"
    );
}

#[test]
fn v2_and_legacy_effect_queues_remain_isolated_across_mixed_feeds() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();
    let v2 = runtime.feed_into(
        b"\x1b]0;v2\x07\x1b]52;c;djI=\x07\x1b]9;v2-notify\x07\x1b]9;4;3\x07",
        &mut buffers,
    );
    assert_eq!(v2.metadata().title(), Some(MetadataChangeRef::Set("v2")));
    assert_eq!(
        v2.metadata().progress(),
        Some(RuntimeProgress::Indeterminate)
    );
    assert_eq!(
        v2.clipboard_writes().collect::<Vec<_>>(),
        vec![(Some("c"), "v2")]
    );
    assert_eq!(
        v2.notifications().collect::<Vec<_>>(),
        vec![(None, "v2-notify")]
    );
    assert!(runtime.take_clipboard_texts().is_empty());
    assert!(runtime.take_notifications().is_empty());

    runtime.feed_pty_output(
        b"\x1b]0;legacy\x07\x1b]52;p;bGVnYWN5\x07\x1b]52;c;?\x07\x1b]777;notify;Legacy;done\x07\x1b]9;4;2\x07",
    );
    let same_sources = runtime.feed_into(b"\x1b]0;legacy\x07\x1b]9;4;2\x07", &mut buffers);
    assert!(same_sources.metadata().is_empty());
    assert!(same_sources.effects().next().is_none());
    assert_eq!(runtime.take_clipboard_texts(), vec!["legacy".to_owned()]);
    assert_eq!(runtime.take_clipboard_queries(), vec!["c".to_owned()]);
    assert_eq!(runtime.take_notifications().len(), 1);
}

#[test]
fn plain_chunks_do_not_scan_or_clone_large_user_var_sets() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
    let mut buffers = RuntimeBuffers::with_capacity(8192);
    let mut metadata = Vec::new();
    for index in 0..512 {
        metadata.extend_from_slice(
            format!("\x1b]1337;SetUserVar=VAR{index:04}=dmFsdWU=\x07").as_bytes(),
        );
    }
    let delta = runtime.feed_into(&metadata, &mut buffers);
    assert_eq!(delta.metadata().user_vars().count(), 512);
    let inspected = runtime.metadata_source_entries_inspected();

    let plain = runtime.feed_into(b"plain output without metadata", &mut buffers);
    assert!(plain.metadata().is_empty());
    assert_eq!(runtime.metadata_source_entries_inspected(), inspected);
}

#[test]
fn repeated_user_var_updates_emit_one_final_value_per_key() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();

    let changed = runtime.feed_into(
        b"\x1b]1337;SetUserVar=FOO=YQ==\x07\x1b]1337;SetUserVar=FOO=Yg==\x07",
        &mut buffers,
    );
    assert_eq!(
        changed.metadata().user_vars().collect::<Vec<_>>(),
        vec![("FOO", MetadataChangeRef::Set("b"))]
    );

    let unchanged = runtime.feed_into(b"\x1b]1337;SetUserVar=FOO=Yg==\x07", &mut buffers);
    assert!(unchanged.metadata().is_empty());
}

#[test]
fn title_stack_controls_preserve_legacy_noop_and_hard_reset_keeps_metadata_quiet() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
    let mut buffers = RuntimeBuffers::default();
    let initial = runtime.feed_into(b"\x1b]0;main\x07", &mut buffers);
    assert_eq!(
        initial.metadata().title(),
        Some(MetadataChangeRef::Set("main"))
    );

    let roundtrip = runtime.feed_into(b"\x1b[22;0t\x1b]0;temporary\x07\x1b[23;0t", &mut buffers);
    assert_eq!(
        roundtrip.metadata().title(),
        Some(MetadataChangeRef::Set("temporary"))
    );
    assert_eq!(runtime.snapshot().terminal().title(), Some("temporary"));

    let reset = runtime.feed_into(b"\x1bc", &mut buffers);
    assert!(reset.metadata().is_empty());
    assert_eq!(runtime.snapshot().terminal().title(), Some("temporary"));
}

#[test]
fn snapshot_ref_is_renderer_neutral_and_tracks_final_state() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(8, 2));
    let mut buffers = RuntimeBuffers::default();
    let delta = runtime.feed_into(b"abc\x1b]0;ops\x07", &mut buffers);
    assert!(delta.snapshot_changed());
    let _ = delta;

    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.terminal().title(), Some("ops"));
    assert_eq!(
        snapshot.terminal().grid().get(0, 0).unwrap().primary_char(),
        'a'
    );
    assert_eq!(
        snapshot.terminal().grid().get(0, 2).unwrap().primary_char(),
        'c'
    );
}

#[test]
fn feed_into_reuses_reserved_buffers_without_growth() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(120, 30));
    let mut buffers = RuntimeBuffers::with_capacity(8192);
    let input = b"line\x1b[6n\x1b]0;stable\x07\r\n".repeat(48 * 1024);

    for chunk in input.chunks(8192) {
        let _ = runtime.feed_into(chunk, &mut buffers);
    }
    let warm_capacity = buffers.capacities();
    let warm_relocations = buffers.relocations();
    runtime.reset_query_scan_storage_counters();

    for chunk in input.chunks(8192) {
        let _ = runtime.feed_into(chunk, &mut buffers);
    }

    assert_eq!(buffers.capacities(), warm_capacity);
    assert_eq!(buffers.relocations(), warm_relocations);
    let scanner = runtime.query_scan_storage_counters();
    assert_eq!(scanner.payload_copies(), 0);
    assert_eq!(scanner.growths(), 0);
    assert!(scanner.compactions() <= 128);
    assert!(scanner.compacted_bytes() <= 4096);
}

#[test]
fn query_responses_are_committed_directly_into_the_caller_arena() {
    let mut runtime = TerminalRuntime::new(TerminalSize::new(120, 30));
    let mut buffers = RuntimeBuffers::with_capacity(8192);
    let queries = b"\x1b[6n".repeat(1024);

    let delta = runtime.feed_into(&queries, &mut buffers);

    assert_eq!(delta.responses().count(), 1024);
    assert_eq!(buffers.response_payload_copies(), 0);
    assert_eq!(buffers.owned_response_materializations(), 0);
    assert_eq!(buffers.response_commits(), 1024);
}

#[test]
fn feed_into_does_not_build_a_legacy_owned_batch() {
    let terminal = include_str!("../src/terminal.rs");
    let queries = include_str!("../src/queries.rs");
    let feed_into = terminal
        .split("pub fn feed_into")
        .nth(1)
        .and_then(|tail| {
            tail.split("/// Returns the current renderer-independent")
                .next()
        })
        .expect("feed_into source");
    let event_helper = terminal
        .split("fn feed_event_into")
        .nth(1)
        .and_then(|tail| tail.split("fn feed_enq").next())
        .expect("borrowed event helper source");
    let response_writers = terminal
        .split("fn emit_response_into")
        .nth(1)
        .and_then(|tail| tail.split("fn apply_notification_into").next())
        .expect("direct response writer source");

    for source in [feed_into, event_helper, response_writers] {
        assert!(!source.contains("feed_pty_output_with_display"));
        assert!(!source.contains("TerminalRuntimeOutput"));
        assert!(!source.contains("output_filter.process("));
        assert!(!source.contains("response.response_bytes"));
        assert!(!source.contains("into_owned()"));
    }
    assert!(feed_into.contains("buffers.begin_feed()"));
    assert!(feed_into.contains("for_each_event"));
    assert!(feed_into.contains("feed_event_into"));
    assert!(feed_into.contains("publish_pending_metadata"));
    assert!(event_helper.contains("self.feed_display_into"));
    assert!(event_helper.contains("emit_response_into"));
    assert!(event_helper.contains("emit_osc_color_response_into"));
    assert!(event_helper.contains("buffers.visible_mut()"));
    assert!(event_helper.contains("terminal.drain_damage_into"));
    assert!(response_writers.contains("try_push_transport_write_with"));
    assert!(queries.contains("for_each_segment"));
    assert!(!queries.contains("QueryScanBuffers"));
}
