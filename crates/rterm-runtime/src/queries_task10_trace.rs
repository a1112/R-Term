use std::fmt::Write as _;

use super::*;

pub(super) fn trace_construct(scanner: &TerminalQueryScanner) -> u64 {
    let state = trace_state(scanner);
    crate::fixture_trace::new_object(
        "query",
        "query.construct",
        if scanner.record_work {
            b"record_work=1"
        } else {
            b"record_work=0"
        },
        b"result=constructed",
        &state,
    )
}

pub(super) fn trace_process(
    scanner: &TerminalQueryScanner,
    bytes: &[u8],
    segments: &[ScannedSegment],
    pre_state: &[u8],
) {
    let state = trace_state(scanner);
    let result = encode_segments(segments);
    crate::fixture_trace::record_action(
        "query",
        scanner.fixture_trace_id,
        "query.process",
        bytes,
        &result,
        pre_state,
        &state,
    );
}

pub(super) fn trace_drop(scanner: &TerminalQueryScanner) {
    if scanner.fixture_trace_id == 0 {
        return;
    }
    let state = trace_state(scanner);
    let pending = live_pending(scanner);
    crate::fixture_trace::finish_object("query", scanner.fixture_trace_id, pending, &state, &state);
}

pub(super) fn trace_clipboard_decode(payload: &[u8], decoded: Option<&str>) {
    if crate::fixture_trace::has_object("query") {
        return;
    }
    let mut result = String::from("kind=clipboard-decode;decoded=");
    match decoded {
        Some(value) => {
            result.push_str("some:");
            result.push_str(&crate::fixture_trace::encode_hex(value.as_bytes()));
        }
        None => result.push_str("none"),
    }
    let state = b"kind=pure;pending=;inspected=0";
    let object = crate::fixture_trace::new_object(
        "query",
        "query.decode_clipboard",
        payload,
        result.as_bytes(),
        state,
    );
    crate::fixture_trace::finish_object("query", object, b"", state, state);
}

pub(super) fn trace_state(scanner: &TerminalQueryScanner) -> Vec<u8> {
    let head = scanner.head.min(scanner.pending.len());
    let mut state = String::new();
    write!(
        &mut state,
        "pending={};cursor={};candidate={};state={};inspected={};record_work={};discarding={}",
        crate::fixture_trace::encode_exact_runs(live_pending(scanner)),
        scanner.cursor.saturating_sub(head),
        scanner.candidate_start.map_or_else(
            || "none".to_owned(),
            |start| start.saturating_sub(head).to_string()
        ),
        state_tag(scanner.state),
        scanner.inspected_bytes,
        u8::from(scanner.record_work),
        u8::from(scanner.discarding),
    )
    .expect("write query trace state");
    state.into_bytes()
}

fn live_pending(scanner: &TerminalQueryScanner) -> &[u8] {
    &scanner.pending[scanner.head.min(scanner.pending.len())..]
}

fn state_tag(state: ScanState) -> String {
    match state {
        ScanState::Ground => "ground".to_owned(),
        ScanState::Escape => "escape".to_owned(),
        ScanState::Utf8C1 => "utf8-c1".to_owned(),
        ScanState::Utf8Text {
            remaining,
            next_min,
            next_max,
        } => format!("utf8-text:{remaining}:{next_min}:{next_max}"),
        ScanState::Csi => "csi".to_owned(),
        ScanState::CsiUtf8C1 => "csi-utf8-c1".to_owned(),
        ScanState::Osc => "osc".to_owned(),
        ScanState::OscEscape => "osc-escape".to_owned(),
        ScanState::OscUtf8C1 => "osc-utf8-c1".to_owned(),
        ScanState::Dcs => "dcs".to_owned(),
        ScanState::DcsEscape => "dcs-escape".to_owned(),
        ScanState::DcsUtf8C1 => "dcs-utf8-c1".to_owned(),
        ScanState::String => "string".to_owned(),
        ScanState::StringEscape => "string-escape".to_owned(),
        ScanState::StringUtf8C1 => "string-utf8-c1".to_owned(),
    }
}

fn encode_segments(segments: &[ScannedSegment]) -> Vec<u8> {
    let mut encoded = format!("segment_count={}", segments.len());
    for (index, segment) in segments.iter().enumerate() {
        match segment {
            ScannedSegment::Bytes(bytes) => write!(
                &mut encoded,
                ";segment[{index}]=bytes:{}",
                crate::fixture_trace::encode_exact_runs(bytes),
            ),
            ScannedSegment::Control {
                family,
                bytes,
                semantic,
            } => write!(
                &mut encoded,
                ";segment[{index}]=control:{}:{}:{}",
                family_tag(*family),
                crate::fixture_trace::encode_exact_runs(bytes),
                semantic_tag(semantic, bytes),
            ),
        }
        .expect("write query segment");
    }
    encoded.into_bytes()
}

fn family_tag(family: ControlFamily) -> &'static str {
    match family {
        ControlFamily::Csi => "csi",
        ControlFamily::Osc => "osc",
        ControlFamily::Dcs => "dcs",
        ControlFamily::Enq => "enq",
        ControlFamily::Other => "other",
    }
}

fn semantic_tag(semantic: &SemanticControl, bytes: &[u8]) -> String {
    match semantic {
        SemanticControl::Fixed(value) => format!("fixed:{}", fixed_tag(*value)),
        SemanticControl::WindowReport(value) => format!("window:{}", window_tag(*value)),
        SemanticControl::PrivateModeStatus(mode) => format!("private-mode-status:{mode}"),
        SemanticControl::AnsiModeStatus(mode) => format!("ansi-mode-status:{mode}"),
        SemanticControl::OscColor(request) => osc_color_tag(request),
        SemanticControl::ItermReportCellSize => "iterm-report-cell-size".to_owned(),
        SemanticControl::Decrqcra(request) => format!(
            "decrqcra:{}:{}:{}:{}:{}",
            request.request_id, request.top, request.left, request.bottom, request.right
        ),
        SemanticControl::Decrqss(request) => format!(
            "decrqss:{}:{}",
            decrqss_kind_tag(request.kind),
            dcs_terminator_tag(request.terminator)
        ),
        SemanticControl::XtGetTcap(request) => xtgettcap_tag(request),
        SemanticControl::XtSmGraphics(request) => {
            format!("xtsmgraphics:{}:{}", request.item, request.action)
        }
        SemanticControl::KittyKeyboardFlags => "kitty-keyboard-flags".to_owned(),
        SemanticControl::KeyModifierOptionsQuery(resource) => {
            format!("key-modifier-options-query:{resource}")
        }
        SemanticControl::SynchronizedOutputMode(sequence) => {
            format!(
                "sync-output:{}:{}",
                u8::from(sequence.enabled),
                join_u16(&sequence.modes)
            )
        }
        SemanticControl::KittyKeyboardMode(mode) => kitty_mode_tag(*mode),
        SemanticControl::KeyModifierOptionsSequence(options) => format!(
            "key-modifier-options-sequence:{}:{}",
            option_u16(options.resource),
            option_u16(options.value)
        ),
        SemanticControl::Osc52(command) => clipboard_tag(command, bytes),
        SemanticControl::Osc8Hyperlink => "osc8-hyperlink".to_owned(),
        SemanticControl::Notification(command) => notification_tag(command),
        SemanticControl::DeviceAttributesResponse => "device-attributes-response".to_owned(),
        SemanticControl::Ignored => "ignored".to_owned(),
        SemanticControl::Enq => "enq".to_owned(),
        SemanticControl::StandaloneSt => "standalone-st".to_owned(),
        SemanticControl::Cancelled => "cancelled".to_owned(),
        SemanticControl::Unknown => "unknown".to_owned(),
    }
}

fn fixed_tag(value: FixedQuery) -> &'static str {
    match value {
        FixedQuery::CursorPosition => "cursor-position",
        FixedQuery::PrimaryDeviceAttributes => "primary-device-attributes",
        FixedQuery::SecondaryDeviceAttributes => "secondary-device-attributes",
        FixedQuery::TertiaryDeviceAttributes => "tertiary-device-attributes",
        FixedQuery::TerminalParameters0 => "terminal-parameters-0",
        FixedQuery::TerminalParameters1 => "terminal-parameters-1",
        FixedQuery::XtVersion => "xt-version",
        FixedQuery::OperatingStatus => "operating-status",
        FixedQuery::WindowPixelSize => "window-pixel-size",
        FixedQuery::CharacterCellSize => "character-cell-size",
        FixedQuery::TextAreaSize => "text-area-size",
    }
}

fn window_tag(value: WindowReportRequest) -> &'static str {
    match value {
        WindowReportRequest::WindowPixelSize => "window-pixel-size",
        WindowReportRequest::CharacterCellSize => "character-cell-size",
        WindowReportRequest::TextAreaSize => "text-area-size",
        WindowReportRequest::WindowTitle => "window-title",
        WindowReportRequest::Ignored => "ignored",
    }
}

fn osc_color_tag(request: &OscColorRequest) -> String {
    let kinds = request
        .kinds
        .iter()
        .map(|kind| match kind {
            OscColorKind::DefaultForeground => "default-foreground".to_owned(),
            OscColorKind::DefaultBackground => "default-background".to_owned(),
            OscColorKind::Cursor => "cursor".to_owned(),
            OscColorKind::Palette(index) => format!("palette:{index}"),
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "osc-color:{}:{kinds}",
        string_terminator_tag(request.terminator)
    )
}

fn clipboard_tag(command: &ClipboardCommand, _bytes: &[u8]) -> String {
    match command {
        ClipboardCommand::Write {
            selection,
            contents,
        } => format!(
            "osc52-write:{}:{}",
            selection.as_ref().map_or_else(
                || "none".to_owned(),
                |value| format!(
                    "some:{}",
                    crate::fixture_trace::encode_hex(value.as_bytes())
                )
            ),
            crate::fixture_trace::encode_hex(contents.as_bytes())
        ),
        ClipboardCommand::Query(selection) => format!(
            "osc52-query:{}",
            crate::fixture_trace::encode_hex(selection.as_bytes())
        ),
    }
}

fn notification_tag(command: &NotificationCommand) -> String {
    match command {
        NotificationCommand::Notify { title, body } => format!(
            "notification:{}:{}",
            title.as_ref().map_or_else(
                || "none".to_owned(),
                |value| format!(
                    "some:{}",
                    crate::fixture_trace::encode_hex(value.as_bytes())
                )
            ),
            crate::fixture_trace::encode_hex(body.as_bytes())
        ),
        NotificationCommand::Progress(progress) => format!("progress:{}", progress_tag(*progress)),
        NotificationCommand::Ignored => "notification-ignored".to_owned(),
    }
}

fn progress_tag(progress: ProgressCommand) -> String {
    match progress {
        ProgressCommand::None => "none".to_owned(),
        ProgressCommand::Percentage(value) => format!("percentage:{value}"),
        ProgressCommand::Error(value) => format!("error:{value}"),
        ProgressCommand::Indeterminate => "indeterminate".to_owned(),
    }
}

fn kitty_mode_tag(mode: KittyKeyboardMode) -> String {
    format!(
        "kitty-mode:{}:{}:{}",
        match mode.operation {
            KittyKeyboardOperation::Push => "push",
            KittyKeyboardOperation::Pop => "pop",
            KittyKeyboardOperation::Apply => "apply",
        },
        mode.value,
        match mode.apply_mode {
            KittyKeyboardApplyMode::Replace => "replace",
            KittyKeyboardApplyMode::Set => "set",
            KittyKeyboardApplyMode::Reset => "reset",
        }
    )
}

fn xtgettcap_tag(request: &crate::query_dcs::XtGetTcapRequest) -> String {
    let names = request
        .names
        .iter()
        .map(|name| {
            format!(
                "{}:{}",
                crate::fixture_trace::encode_hex(&name.encoded),
                name.decoded.as_ref().map_or_else(
                    || "none".to_owned(),
                    |value| format!("some:{}", crate::fixture_trace::encode_hex(value))
                )
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "xtgettcap:{}:{names}",
        dcs_terminator_tag(request.terminator)
    )
}

fn decrqss_kind_tag(kind: crate::query_dcs::DecrqssKind) -> &'static str {
    match kind {
        crate::query_dcs::DecrqssKind::Sgr => "sgr",
        crate::query_dcs::DecrqssKind::CursorShape => "cursor-shape",
        crate::query_dcs::DecrqssKind::ScrollRegion => "scroll-region",
        crate::query_dcs::DecrqssKind::ConformanceLevel => "conformance-level",
        crate::query_dcs::DecrqssKind::LeftRightMargins => "left-right-margins",
        crate::query_dcs::DecrqssKind::Unknown => "unknown",
    }
}

fn dcs_terminator_tag(terminator: crate::query_dcs::DcsTerminator) -> &'static str {
    match terminator {
        crate::query_dcs::DcsTerminator::SevenBit => "7-bit",
        crate::query_dcs::DcsTerminator::EightBit => "8-bit",
    }
}

fn string_terminator_tag(terminator: StringTerminator) -> &'static str {
    match terminator {
        StringTerminator::Bel => "bel",
        StringTerminator::St => "st",
        StringTerminator::C1St => "c1-st",
    }
}

fn option_u16(value: Option<u16>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| format!("some:{value}"))
}

fn join_u16(values: &[u16]) -> String {
    values
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
