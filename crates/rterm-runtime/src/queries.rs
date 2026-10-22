use base64::{Engine as _, engine::general_purpose::STANDARD};

const MAX_OSC_COLOR_QUERY_KINDS: usize = 256;
const MAX_OSC_COLOR_QUERY_BYTES: usize = 16 * 1024;
const MAX_CLIPBOARD_BYTES: usize = 1024 * 1024;
const MAX_CLIPBOARD_BASE64_BYTES: usize = MAX_CLIPBOARD_BYTES.div_ceil(3) * 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFamily {
    Csi,
    Osc,
    Dcs,
    Enq,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedQuery {
    CursorPosition,
    PrimaryDeviceAttributes,
    SecondaryDeviceAttributes,
    TertiaryDeviceAttributes,
    TerminalParameters0,
    TerminalParameters1,
    XtVersion,
    OperatingStatus,
    WindowPixelSize,
    CharacterCellSize,
    TextAreaSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowReportRequest {
    WindowPixelSize,
    CharacterCellSize,
    TextAreaSize,
    WindowTitle,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringTerminator {
    Bel,
    St,
    C1St,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscColorKind {
    DefaultForeground,
    DefaultBackground,
    Cursor,
    Palette(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OscColorRequest {
    pub kinds: Vec<OscColorKind>,
    pub terminator: StringTerminator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecrqcraRequest {
    pub request_id: i64,
    pub top: u16,
    pub left: u16,
    pub bottom: u16,
    pub right: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XtSmGraphicsRequest {
    pub item: u64,
    pub action: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardCommand {
    Write {
        selection: Option<String>,
        contents: String,
    },
    Query(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressCommand {
    None,
    Percentage(u8),
    Error(u8),
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationCommand {
    Notify { title: Option<String>, body: String },
    Progress(ProgressCommand),
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateModeSequence {
    pub modes: Vec<u16>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KittyKeyboardOperation {
    Push,
    Pop,
    Apply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KittyKeyboardApplyMode {
    Replace,
    Set,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KittyKeyboardMode {
    pub operation: KittyKeyboardOperation,
    pub value: u16,
    pub apply_mode: KittyKeyboardApplyMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifierOptions {
    pub resource: Option<u16>,
    pub value: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticControl {
    Fixed(FixedQuery),
    WindowReport(WindowReportRequest),
    PrivateModeStatus(u16),
    AnsiModeStatus(u16),
    OscColor(OscColorRequest),
    ItermReportCellSize,
    Decrqcra(DecrqcraRequest),
    Decrqss(crate::query_dcs::DecrqssRequest),
    XtGetTcap(crate::query_dcs::XtGetTcapRequest),
    XtSmGraphics(XtSmGraphicsRequest),
    KittyKeyboardFlags,
    KeyModifierOptionsQuery(u16),
    SynchronizedOutputMode(PrivateModeSequence),
    KittyKeyboardMode(KittyKeyboardMode),
    KeyModifierOptionsSequence(KeyModifierOptions),
    Osc52(ClipboardCommand),
    Osc8Hyperlink,
    Notification(NotificationCommand),
    DeviceAttributesResponse,
    Ignored,
    Enq,
    StandaloneSt,
    Cancelled,
    Unknown,
}

fn csi_body(bytes: &[u8]) -> Option<&[u8]> {
    bytes
        .strip_prefix(b"\x1b[")
        .or_else(|| bytes.strip_prefix(b"\x9b"))
        .or_else(|| bytes.strip_prefix(b"\xc2\x9b"))
}

fn string_body_with_terminator<'a>(
    bytes: &'a [u8],
    esc: &[u8],
    raw: u8,
) -> Option<(&'a [u8], StringTerminator)> {
    let body = bytes
        .strip_prefix(esc)
        .or_else(|| bytes.strip_prefix(&[raw]))
        .or_else(|| bytes.strip_prefix(&[0xc2, raw]))?;
    if let Some(body) = body.strip_suffix(b"\x07") {
        Some((body, StringTerminator::Bel))
    } else if let Some(body) = body.strip_suffix(b"\x1b\\") {
        Some((body, StringTerminator::St))
    } else {
        body.strip_suffix(b"\xc2\x9c")
            .or_else(|| body.strip_suffix(b"\x9c"))
            .map(|body| (body, StringTerminator::C1St))
    }
}

fn parse_u16(bytes: &[u8]) -> Option<u16> {
    if bytes.is_empty() {
        return None;
    }
    bytes.iter().try_fold(0_u16, |value, byte| {
        let digit = byte.checked_sub(b'0')?;
        (digit <= 9)
            .then_some(value)?
            .checked_mul(10)?
            .checked_add(u16::from(digit))
    })
}

fn parse_u16_saturating(bytes: &[u8]) -> Option<u16> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(bytes.iter().fold(0_u16, |value, byte| {
        value
            .saturating_mul(10)
            .saturating_add(u16::from(byte - b'0'))
    }))
}

fn parse_i64(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    bytes.iter().try_fold(0_i64, |value, byte| {
        value.checked_mul(10)?.checked_add(i64::from(byte - b'0'))
    })
}

fn parse_u64(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    bytes.iter().try_fold(0_u64, |value, byte| {
        value.checked_mul(10)?.checked_add(u64::from(byte - b'0'))
    })
}

fn parse_window_report(body: &[u8]) -> WindowReportRequest {
    let Some(params) = body.strip_suffix(b"t") else {
        return WindowReportRequest::Ignored;
    };
    let mut parts = params.split(|byte| *byte == b';');
    let Some(first) = parts.next().and_then(parse_i64) else {
        return WindowReportRequest::Ignored;
    };
    let second = match parts.next() {
        Some([]) | None => None,
        Some(part) => match parse_i64(part) {
            Some(value) => Some(value),
            None => return WindowReportRequest::Ignored,
        },
    };
    if parts.any(|part| !part.is_empty() && parse_i64(part).is_none()) {
        return WindowReportRequest::Ignored;
    }
    match first {
        14 if second.is_none() => WindowReportRequest::WindowPixelSize,
        16 => WindowReportRequest::CharacterCellSize,
        18 => WindowReportRequest::TextAreaSize,
        21 if second.is_none() => WindowReportRequest::WindowTitle,
        _ => WindowReportRequest::Ignored,
    }
}

fn decrqcra_axis(part: Option<&[u8]>, default: i64) -> Option<u16> {
    let value = match part {
        Some([]) | None => default,
        Some(part) => parse_i64(part)?,
    };
    if value <= 0 {
        Some(0)
    } else {
        u16::try_from(value.saturating_sub(1).min(i64::from(u16::MAX))).ok()
    }
}

fn parse_decrqcra(body: &[u8]) -> Option<DecrqcraRequest> {
    let params = body.strip_suffix(b"*y")?;
    let mut parts = params.split(|byte| *byte == b';');
    let request_id = parts.next().and_then(parse_i64)?;
    let _page_number = parts.next().and_then(parse_i64)?;
    let top = decrqcra_axis(parts.next(), 0)?;
    let left = decrqcra_axis(parts.next(), 0)?;
    let bottom = decrqcra_axis(parts.next(), i64::from(u16::MAX))?;
    let right = decrqcra_axis(parts.next(), i64::from(u16::MAX))?;
    (parts.next().is_none()).then_some(DecrqcraRequest {
        request_id,
        top,
        left,
        bottom,
        right,
    })
}

fn parse_xtsmgraphics(body: &[u8]) -> Option<XtSmGraphicsRequest> {
    let content = body.strip_prefix(b"?")?.strip_suffix(b"S")?;
    let mut parameters = content.split(|byte| *byte == b';');
    let item = parse_u64(parameters.next()?)?;
    let action = parse_u64(parameters.next()?)?;
    parameters.try_for_each(|parameter| parse_u64(parameter).map(|_| ()))?;
    Some(XtSmGraphicsRequest { item, action })
}

fn parse_osc_color(body: &[u8], terminator: StringTerminator) -> Option<OscColorRequest> {
    let (selector, content) = split_osc_selector(body)?;
    let kinds = if osc_selector_is(selector, b"10") && content == b"?" {
        vec![OscColorKind::DefaultForeground]
    } else if osc_selector_is(selector, b"11") && content == b"?" {
        vec![OscColorKind::DefaultBackground]
    } else if osc_selector_is(selector, b"12") && content == b"?" {
        vec![OscColorKind::Cursor]
    } else {
        if body.len() > MAX_OSC_COLOR_QUERY_BYTES || !osc_selector_is(selector, b"4") {
            return None;
        }
        let mut parts = content.split(|byte| *byte == b';');
        let mut kinds = Vec::new();
        while let Some(index) = parts.next() {
            if parts.next()? != b"?" {
                return None;
            }
            if kinds.len() == MAX_OSC_COLOR_QUERY_KINDS {
                return None;
            }
            kinds.push(OscColorKind::Palette(u8::try_from(parse_u16(index)?).ok()?));
        }
        if kinds.is_empty() {
            return None;
        }
        kinds
    };
    Some(OscColorRequest { kinds, terminator })
}

fn is_reserved_osc_color_query(body: &[u8]) -> bool {
    split_osc_selector(body).is_some_and(|(selector, content)| {
        ((osc_selector_is(selector, b"10")
            || osc_selector_is(selector, b"11")
            || osc_selector_is(selector, b"12"))
            && content.contains(&b'?'))
            || (osc_selector_is(selector, b"4")
                && content
                    .split(|byte| *byte == b';')
                    .any(|field| field == b"?"))
    })
}

fn split_osc_selector(body: &[u8]) -> Option<(&[u8], &[u8])> {
    let separator = body.iter().position(|byte| *byte == b';')?;
    Some((&body[..separator], &body[separator + 1..]))
}

fn osc_selector_is(selector: &[u8], expected: &[u8]) -> bool {
    if selector.is_empty() || !selector.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let selector = selector
        .iter()
        .position(|byte| *byte != b'0')
        .map_or(b"0".as_slice(), |start| &selector[start..]);
    selector == expected
}

fn is_reserved_clipboard_control(body: &[u8]) -> bool {
    split_osc_selector(body).map_or_else(
        || osc_selector_is(body, b"52"),
        |(selector, content)| {
            osc_selector_is(selector, b"52")
                || (osc_selector_is(selector, b"1337")
                    && (content.starts_with(b"Copy=") || content.starts_with(b"CopyToClipboard=")))
        },
    )
}

fn parse_clipboard(body: &[u8]) -> Option<ClipboardCommand> {
    let (selector, content) = split_osc_selector(body)?;
    if osc_selector_is(selector, b"52") {
        let separator = content.iter().position(|byte| *byte == b';')?;
        let selection = String::from_utf8(content[..separator].to_vec()).ok()?;
        let payload = &content[separator + 1..];
        if payload == b"?" {
            return Some(ClipboardCommand::Query(selection));
        }
        Some(ClipboardCommand::Write {
            selection: Some(selection),
            contents: decode_clipboard_payload(payload)?,
        })
    } else if osc_selector_is(selector, b"1337") {
        let payload = content
            .strip_prefix(b"Copy=;")
            .or_else(|| content.strip_prefix(b"CopyToClipboard=;"))?;
        Some(ClipboardCommand::Write {
            selection: None,
            contents: decode_clipboard_payload(payload)?,
        })
    } else {
        None
    }
}

fn decode_clipboard_payload(payload: &[u8]) -> Option<String> {
    let decoded = decode_clipboard_payload_inner(payload);
    #[cfg(test)]
    task10_trace::trace_clipboard_decode(payload, decoded.as_deref());
    decoded
}

fn decode_clipboard_payload_inner(payload: &[u8]) -> Option<String> {
    if payload.len() > MAX_CLIPBOARD_BASE64_BYTES {
        return None;
    }
    let decoded = STANDARD.decode(payload).ok()?;
    if decoded.len() > MAX_CLIPBOARD_BYTES {
        return None;
    }
    String::from_utf8(decoded).ok()
}

fn parse_private_mode_sequence(body: &[u8]) -> Option<PrivateModeSequence> {
    let (body, enabled) = match body.last()? {
        b'h' => (&body[..body.len() - 1], true),
        b'l' => (&body[..body.len() - 1], false),
        _ => return None,
    };
    let parameters = body.strip_prefix(b"?")?;
    let modes = parameters
        .split(|byte| *byte == b';')
        .map(parse_u16_saturating)
        .collect::<Option<Vec<_>>>()?;
    (!modes.is_empty()).then_some(PrivateModeSequence { modes, enabled })
}

fn parse_kitty_keyboard_mode(body: &[u8]) -> Option<KittyKeyboardMode> {
    let (operation, body) = match body.first()? {
        b'>' => (KittyKeyboardOperation::Push, &body[1..]),
        b'<' => (KittyKeyboardOperation::Pop, &body[1..]),
        b'=' => (KittyKeyboardOperation::Apply, &body[1..]),
        _ => return None,
    };
    let parameters = body.strip_suffix(b"u")?;
    let (value, apply_mode) = if operation == KittyKeyboardOperation::Apply {
        let mut parts = parameters.split(|byte| *byte == b';');
        let value = parse_u16_saturating(parts.next()?)?;
        let apply_mode = match parts.next() {
            None => KittyKeyboardApplyMode::Replace,
            Some(mode) => match parse_u16_saturating(mode)? {
                1 => KittyKeyboardApplyMode::Replace,
                2 => KittyKeyboardApplyMode::Set,
                3 => KittyKeyboardApplyMode::Reset,
                _ => return None,
            },
        };
        (parts.next().is_none()).then_some((value, apply_mode))?
    } else {
        let value = if parameters.is_empty() {
            0
        } else {
            parse_u16_saturating(parameters)?
        };
        (value, KittyKeyboardApplyMode::Replace)
    };
    Some(KittyKeyboardMode {
        operation,
        value,
        apply_mode,
    })
}

fn parse_key_modifier_options(body: &[u8]) -> Option<KeyModifierOptions> {
    let parameters = body.strip_prefix(b">")?.strip_suffix(b"m")?;
    if parameters.is_empty() {
        return Some(KeyModifierOptions {
            resource: None,
            value: None,
        });
    }
    let mut parts = parameters.split(|byte| *byte == b';');
    let resource = parse_u16_saturating(parts.next()?)?;
    let value = match parts.next() {
        Some(value) => Some(parse_u16_saturating(value)?),
        None => None,
    };
    (parts.next().is_none()).then_some(KeyModifierOptions {
        resource: Some(resource),
        value,
    })
}

fn parse_u8_decimal(bytes: &[u8]) -> Option<u8> {
    u8::try_from(parse_u16(bytes)?).ok()
}

fn parse_progress_value(bytes: &[u8]) -> Option<u8> {
    let value = parse_u8_decimal(bytes)?;
    (value <= 100).then_some(value)
}

fn parse_notification(body: &[u8]) -> NotificationCommand {
    let Some((selector, content)) = split_osc_selector(body) else {
        return NotificationCommand::Ignored;
    };
    if osc_selector_is(selector, b"9") {
        if let Some(progress) = content.strip_prefix(b"4;") {
            let mut parts = progress.split(|byte| *byte == b';');
            let command = match parts.next().and_then(parse_u8_decimal) {
                Some(0) => Some(ProgressCommand::None),
                Some(1) => parts
                    .next()
                    .and_then(parse_progress_value)
                    .map(ProgressCommand::Percentage),
                Some(2) => Some(ProgressCommand::Error(
                    parts.next().and_then(parse_progress_value).unwrap_or(0),
                )),
                Some(3) => Some(ProgressCommand::Indeterminate),
                _ => None,
            };
            return command.map_or(NotificationCommand::Ignored, NotificationCommand::Progress);
        }
        return String::from_utf8(content.to_vec()).map_or(NotificationCommand::Ignored, |body| {
            NotificationCommand::Notify { title: None, body }
        });
    }

    if !osc_selector_is(selector, b"777") {
        return NotificationCommand::Ignored;
    }
    let Some(content) = content.strip_prefix(b"notify;") else {
        return NotificationCommand::Ignored;
    };
    let Some(separator) = content.iter().position(|byte| *byte == b';') else {
        return NotificationCommand::Ignored;
    };
    let (title, body) = (&content[..separator], &content[separator + 1..]);
    match (
        String::from_utf8(title.to_vec()),
        String::from_utf8(body.to_vec()),
    ) {
        (Ok(title), Ok(body)) => NotificationCommand::Notify {
            title: Some(title),
            body,
        },
        _ => NotificationCommand::Ignored,
    }
}

#[allow(clippy::too_many_lines)]
fn classify_control(family: ControlFamily, bytes: &[u8]) -> SemanticControl {
    match family {
        ControlFamily::Csi => {
            let Some(body) = csi_body(bytes) else {
                return SemanticControl::Unknown;
            };
            let fixed = match body {
                b"6n" => Some(FixedQuery::CursorPosition),
                b"c" | b"0c" => Some(FixedQuery::PrimaryDeviceAttributes),
                b">c" | b">0c" => Some(FixedQuery::SecondaryDeviceAttributes),
                b"=c" | b"=0c" => Some(FixedQuery::TertiaryDeviceAttributes),
                b"x" | b"0x" => Some(FixedQuery::TerminalParameters0),
                b"1x" => Some(FixedQuery::TerminalParameters1),
                b">q" | b">0q" => Some(FixedQuery::XtVersion),
                b"5n" => Some(FixedQuery::OperatingStatus),
                b"14t" => Some(FixedQuery::WindowPixelSize),
                b"16t" => Some(FixedQuery::CharacterCellSize),
                b"18t" => Some(FixedQuery::TextAreaSize),
                _ => None,
            };
            if let Some(fixed) = fixed {
                return SemanticControl::Fixed(fixed);
            }
            if body.ends_with(b"t") {
                return SemanticControl::WindowReport(parse_window_report(body));
            }
            if let Some(mode) = body
                .strip_prefix(b"?")
                .and_then(|body| body.strip_suffix(b"$p"))
                .and_then(parse_u16)
            {
                return SemanticControl::PrivateModeStatus(mode);
            }
            if let Some(mode) = body.strip_suffix(b"$p").and_then(parse_u16) {
                return SemanticControl::AnsiModeStatus(mode);
            }
            if let Some(request) = parse_decrqcra(body) {
                return SemanticControl::Decrqcra(request);
            }
            if let Some(request) = parse_xtsmgraphics(body) {
                return SemanticControl::XtSmGraphics(request);
            }
            if body == b"?u" {
                return SemanticControl::KittyKeyboardFlags;
            }
            if let Some(resource) = body
                .strip_prefix(b"?")
                .and_then(|body| body.strip_suffix(b"m"))
                .and_then(parse_u16)
            {
                return SemanticControl::KeyModifierOptionsQuery(resource);
            }
            if let Some(sequence) = parse_private_mode_sequence(body)
                && sequence.modes.contains(&2026)
            {
                return SemanticControl::SynchronizedOutputMode(sequence);
            }
            if let Some(sequence) = parse_kitty_keyboard_mode(body) {
                return SemanticControl::KittyKeyboardMode(sequence);
            }
            if let Some(sequence) = parse_key_modifier_options(body) {
                return SemanticControl::KeyModifierOptionsSequence(sequence);
            }
            if body == b"?6n" {
                return SemanticControl::Ignored;
            }
            if matches!(body.first(), Some(b'?' | b'>' | b'='))
                && body.ends_with(b"c")
                && body[1..body.len() - 1]
                    .iter()
                    .all(|byte| byte.is_ascii_digit() || *byte == b';')
            {
                return SemanticControl::DeviceAttributesResponse;
            }
            SemanticControl::Unknown
        }
        ControlFamily::Osc => {
            let Some((body, terminator)) = string_body_with_terminator(bytes, b"\x1b]", 0x9d)
            else {
                return SemanticControl::Unknown;
            };
            let selector_and_content = split_osc_selector(body);
            if let Some(query) = parse_osc_color(body, terminator) {
                SemanticControl::OscColor(query)
            } else if is_reserved_osc_color_query(body) {
                SemanticControl::Ignored
            } else if selector_and_content.is_some_and(|(selector, content)| {
                osc_selector_is(selector, b"1337") && content == b"ReportCellSize"
            }) {
                SemanticControl::ItermReportCellSize
            } else if is_reserved_clipboard_control(body) {
                parse_clipboard(body).map_or(SemanticControl::Ignored, SemanticControl::Osc52)
            } else if selector_and_content
                .is_some_and(|(selector, _)| osc_selector_is(selector, b"8"))
            {
                SemanticControl::Osc8Hyperlink
            } else if selector_and_content.is_some_and(|(selector, _)| {
                osc_selector_is(selector, b"9") || osc_selector_is(selector, b"777")
            }) {
                SemanticControl::Notification(parse_notification(body))
            } else {
                SemanticControl::Unknown
            }
        }
        ControlFamily::Dcs => {
            if let Some(request) = crate::query_dcs::parse_decrqss_request(bytes) {
                SemanticControl::Decrqss(request)
            } else if let Some(request) = crate::query_dcs::parse_xtgettcap_request(bytes) {
                SemanticControl::XtGetTcap(request)
            } else if crate::query_dcs::is_reserved_query(bytes) {
                SemanticControl::Ignored
            } else {
                SemanticControl::Unknown
            }
        }
        ControlFamily::Enq => SemanticControl::Enq,
        ControlFamily::Other if matches!(bytes, b"\x1b\\" | b"\x9c" | b"\xc2\x9c") => {
            SemanticControl::StandaloneSt
        }
        ControlFamily::Other => SemanticControl::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScannedSegment {
    Bytes(Vec<u8>),
    Control {
        family: ControlFamily,
        bytes: Vec<u8>,
        semantic: SemanticControl,
    },
}

impl ScannedSegment {
    #[cfg(test)]
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) | Self::Control { bytes, .. } => bytes,
        }
    }
}

#[derive(Debug)]
pub enum ScannedSegmentRef<'a> {
    Bytes(&'a [u8]),
    Control {
        bytes: &'a [u8],
        semantic: SemanticControl,
    },
}

trait ScanSink {
    fn push_bytes(&mut self, bytes: &[u8]);
    fn push_control(&mut self, family: ControlFamily, bytes: &[u8], semantic: SemanticControl);
}

struct OwnedScanSink<'a> {
    segments: &'a mut Vec<ScannedSegment>,
    payload_copies: u64,
}

impl ScanSink for OwnedScanSink<'_> {
    fn push_bytes(&mut self, bytes: &[u8]) {
        self.segments.push(ScannedSegment::Bytes(bytes.to_vec()));
        self.payload_copies = self.payload_copies.saturating_add(1);
    }

    fn push_control(&mut self, family: ControlFamily, bytes: &[u8], semantic: SemanticControl) {
        self.segments.push(ScannedSegment::Control {
            family,
            bytes: bytes.to_vec(),
            semantic,
        });
        self.payload_copies = self.payload_copies.saturating_add(1);
    }
}

struct CallbackScanSink<'callback, F> {
    callback: &'callback mut F,
}

impl<F> ScanSink for CallbackScanSink<'_, F>
where
    F: for<'segment> FnMut(ScannedSegmentRef<'segment>),
{
    fn push_bytes(&mut self, bytes: &[u8]) {
        (self.callback)(ScannedSegmentRef::Bytes(bytes));
    }

    fn push_control(&mut self, _family: ControlFamily, bytes: &[u8], semantic: SemanticControl) {
        (self.callback)(ScannedSegmentRef::Control { bytes, semantic });
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ScanState {
    #[default]
    Ground,
    Escape,
    Utf8C1,
    Utf8Text {
        remaining: u8,
        next_min: u8,
        next_max: u8,
    },
    Csi,
    CsiUtf8C1,
    Osc,
    OscEscape,
    OscUtf8C1,
    Dcs,
    DcsEscape,
    DcsUtf8C1,
    String,
    StringEscape,
    StringUtf8C1,
}

#[derive(Debug, Clone, Copy)]
struct StringScanStates {
    body: ScanState,
    escape: ScanState,
    utf8_c1: ScanState,
    family: ControlFamily,
    bel_terminates: bool,
}

#[derive(Default)]
pub struct TerminalQueryScanner {
    pending: Vec<u8>,
    head: usize,
    cursor: usize,
    candidate_start: Option<usize>,
    state: ScanState,
    inspected_bytes: u64,
    record_work: bool,
    discarding: bool,
    storage_counters: QueryScanStorageCounters,
    #[cfg(test)]
    fixture_trace_id: u64,
}

/// Storage work performed by the streaming query scanner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueryScanStorageCounters {
    payload_copies: u64,
    growths: u64,
    compactions: u64,
    compacted_bytes: u64,
}

impl QueryScanStorageCounters {
    /// Owned compatibility collectors copy one payload per emitted segment.
    #[must_use]
    pub const fn payload_copies(self) -> u64 {
        self.payload_copies
    }

    /// Number of pending-buffer capacity growth events.
    #[must_use]
    pub const fn growths(self) -> u64 {
        self.growths
    }

    /// Number of live-tail moves used to reclaim a consumed prefix.
    #[must_use]
    pub const fn compactions(self) -> u64 {
        self.compactions
    }

    /// Bytes moved while retaining an incomplete cross-chunk suffix.
    #[must_use]
    pub const fn compacted_bytes(self) -> u64 {
        self.compacted_bytes
    }
}

impl TerminalQueryScanner {
    pub const MAX_PENDING: usize = 1024 * 1024;

    #[must_use]
    pub fn new() -> Self {
        let scanner = Self::with_work_counter(false);
        #[cfg(test)]
        let scanner = {
            let mut scanner = scanner;
            scanner.fixture_trace_id = task10_trace::trace_construct(&scanner);
            scanner
        };
        scanner
    }

    #[must_use]
    pub fn new_with_work_counter() -> Self {
        let scanner = Self::with_work_counter(true);
        #[cfg(test)]
        let scanner = {
            let mut scanner = scanner;
            scanner.fixture_trace_id = task10_trace::trace_construct(&scanner);
            scanner
        };
        scanner
    }

    fn with_work_counter(record_work: bool) -> Self {
        Self {
            pending: Vec::new(),
            head: 0,
            cursor: 0,
            candidate_start: None,
            state: ScanState::default(),
            inspected_bytes: 0,
            record_work,
            discarding: false,
            storage_counters: QueryScanStorageCounters::default(),
            #[cfg(test)]
            fixture_trace_id: 0,
        }
    }

    #[must_use]
    pub const fn inspected_bytes(&self) -> u64 {
        self.inspected_bytes
    }

    #[must_use]
    pub const fn storage_counters(&self) -> QueryScanStorageCounters {
        self.storage_counters
    }

    pub fn reset_storage_counters(&mut self) {
        self.storage_counters = QueryScanStorageCounters::default();
    }

    #[cfg(test)]
    pub(crate) fn task10_trace_state(&self) -> Vec<u8> {
        task10_trace::trace_state(self)
    }

    pub fn discard_incomplete(&mut self) {
        self.pending.clear();
        self.head = 0;
        self.cursor = 0;
        self.candidate_start = None;
        self.state = ScanState::Ground;
        self.discarding = false;
    }

    pub fn process(&mut self, bytes: &[u8]) -> Vec<ScannedSegment> {
        #[cfg(test)]
        let pre_state = (self.fixture_trace_id != 0).then(|| task10_trace::trace_state(self));
        let mut segments = Vec::new();
        let mut sink = OwnedScanSink {
            segments: &mut segments,
            payload_copies: 0,
        };
        self.process_with_sink(bytes, &mut sink);
        self.storage_counters.payload_copies = self
            .storage_counters
            .payload_copies
            .saturating_add(sink.payload_copies);
        #[cfg(test)]
        if let Some(pre_state) = pre_state {
            task10_trace::trace_process(self, bytes, &segments, &pre_state);
        }
        segments
    }

    pub fn for_each_segment<F>(&mut self, bytes: &[u8], mut callback: F)
    where
        F: for<'segment> FnMut(ScannedSegmentRef<'segment>),
    {
        let mut sink = CallbackScanSink {
            callback: &mut callback,
        };
        self.process_with_sink(bytes, &mut sink);
    }

    fn process_with_sink(&mut self, bytes: &[u8], sink: &mut impl ScanSink) {
        let mut offset = 0;
        while offset < bytes.len() {
            let live_len = self.pending.len().saturating_sub(self.head);
            let available = Self::MAX_PENDING.saturating_sub(live_len).max(1);
            let chunk_len = (bytes.len() - offset).min(8 * 1024).min(available);
            let chunk = &bytes[offset..offset + chunk_len];
            if self.record_work {
                self.process_inner::<true>(chunk, sink);
            } else {
                self.process_inner::<false>(chunk, sink);
            }
            offset += chunk_len;
        }
    }

    fn process_inner<const RECORD_WORK: bool>(&mut self, bytes: &[u8], sink: &mut impl ScanSink) {
        self.compact_before_append(bytes.len());
        let capacity = self.pending.capacity();
        self.pending.extend_from_slice(bytes);
        if self.pending.capacity() != capacity {
            self.storage_counters.growths = self.storage_counters.growths.saturating_add(1);
        }
        let mut emitted_end = self.head;

        while self.cursor < self.pending.len() {
            let byte = self.pending[self.cursor];
            if RECORD_WORK {
                self.inspected_bytes = self.inspected_bytes.saturating_add(1);
            }

            if self.discarding {
                self.step_discarding::<RECORD_WORK>(byte, sink, &mut emitted_end);
                continue;
            }

            match self.state {
                ScanState::Ground
                | ScanState::Escape
                | ScanState::Utf8C1
                | ScanState::Utf8Text { .. } => {
                    if self.step_ground_escape_utf8::<RECORD_WORK>(byte, sink, &mut emitted_end) {
                        continue;
                    }
                }
                ScanState::Csi | ScanState::CsiUtf8C1 => {
                    if self.step_csi::<RECORD_WORK>(byte, sink, &mut emitted_end) {
                        continue;
                    }
                }
                ScanState::Osc
                | ScanState::OscEscape
                | ScanState::OscUtf8C1
                | ScanState::Dcs
                | ScanState::DcsEscape
                | ScanState::DcsUtf8C1
                | ScanState::String
                | ScanState::StringEscape
                | ScanState::StringUtf8C1 => {
                    if self.step_control_string::<RECORD_WORK>(byte, sink, &mut emitted_end) {
                        continue;
                    }
                }
            }

            if let Some(start) = self.candidate_start
                && self.cursor.saturating_sub(start) >= Self::MAX_PENDING
            {
                self.head = self.cursor;
                self.compact_live_tail();
                emitted_end = 0;
                self.candidate_start = None;
                self.discarding = true;
            }
        }

        if self.state == ScanState::Ground {
            Self::push_bytes(&self.pending, sink, emitted_end, self.pending.len());
            emitted_end = self.pending.len();
        }

        if emitted_end > self.head {
            self.head = emitted_end;
        }

        if self.discarding && self.cursor > self.head {
            self.head = self.cursor;
        }

        if self.head == self.pending.len() {
            self.pending.clear();
            self.head = 0;
            self.cursor = 0;
            self.candidate_start = None;
        }
    }

    fn step_control_string<const RECORD_WORK: bool>(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
    ) -> bool {
        let states = match self.state {
            ScanState::Osc | ScanState::OscEscape | ScanState::OscUtf8C1 => StringScanStates {
                body: ScanState::Osc,
                escape: ScanState::OscEscape,
                utf8_c1: ScanState::OscUtf8C1,
                family: ControlFamily::Osc,
                bel_terminates: true,
            },
            ScanState::Dcs | ScanState::DcsEscape | ScanState::DcsUtf8C1 => StringScanStates {
                body: ScanState::Dcs,
                escape: ScanState::DcsEscape,
                utf8_c1: ScanState::DcsUtf8C1,
                family: ControlFamily::Dcs,
                bel_terminates: false,
            },
            ScanState::String | ScanState::StringEscape | ScanState::StringUtf8C1 => {
                StringScanStates {
                    body: ScanState::String,
                    escape: ScanState::StringEscape,
                    utf8_c1: ScanState::StringUtf8C1,
                    family: ControlFamily::Other,
                    bel_terminates: false,
                }
            }
            _ => unreachable!("control-string step called for another state"),
        };
        self.step_string_family::<RECORD_WORK>(byte, sink, emitted_end, states)
    }

    fn step_string_family<const RECORD_WORK: bool>(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
        states: StringScanStates,
    ) -> bool {
        if self.state == states.body {
            self.cursor += 1;
            match byte {
                0x18 | 0x1a => *emitted_end = self.finish_cancelled(sink, self.cursor),
                0x07 if states.bel_terminates => {
                    *emitted_end =
                        self.finish_control::<RECORD_WORK>(sink, self.cursor, states.family);
                }
                0x9c => {
                    *emitted_end =
                        self.finish_control::<RECORD_WORK>(sink, self.cursor, states.family);
                }
                0x1b => self.state = states.escape,
                0xc2 => self.state = states.utf8_c1,
                _ => {}
            }
            return false;
        }
        if self.state == states.escape {
            match byte {
                b'\\' => {
                    self.cursor += 1;
                    *emitted_end =
                        self.finish_control::<RECORD_WORK>(sink, self.cursor, states.family);
                }
                0x18 | 0x1a => {
                    self.cursor += 1;
                    *emitted_end = self.finish_cancelled(sink, self.cursor);
                }
                0x1b => {
                    *emitted_end = self.finish_cancelled(sink, self.cursor);
                    self.candidate_start = Some(self.cursor);
                    self.state = ScanState::Escape;
                    self.cursor += 1;
                }
                _ => {
                    let escape = self.cursor - 1;
                    *emitted_end = self.finish_cancelled(sink, escape);
                    self.candidate_start = Some(escape);
                    self.state = ScanState::Escape;
                }
            }
            return false;
        }
        if !(0x80..=0xbf).contains(&byte) {
            self.state = states.body;
            return true;
        }
        self.cursor += 1;
        if byte == 0x9c {
            *emitted_end = self.finish_control::<RECORD_WORK>(sink, self.cursor, states.family);
        } else {
            self.state = states.body;
        }
        false
    }

    fn step_csi<const RECORD_WORK: bool>(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
    ) -> bool {
        match self.state {
            ScanState::Csi => match byte {
                0x18 | 0x1a => {
                    self.cursor += 1;
                    *emitted_end = self.finish_cancelled(sink, self.cursor);
                }
                0x1b => {
                    *emitted_end = self.finish_cancelled(sink, self.cursor);
                    self.candidate_start = Some(self.cursor);
                    self.state = ScanState::Escape;
                    self.cursor += 1;
                }
                0x9b | 0x9d | 0x90 | 0x98 | 0x9e | 0x9f | 0x9c => {
                    *emitted_end = self.finish_cancelled(sink, self.cursor);
                    self.candidate_start = Some(self.cursor);
                    self.state = match byte {
                        0x9b => ScanState::Csi,
                        0x9d => ScanState::Osc,
                        0x90 => ScanState::Dcs,
                        0x98 | 0x9e | 0x9f => ScanState::String,
                        0x9c => ScanState::Ground,
                        _ => unreachable!(),
                    };
                    self.cursor += 1;
                    if byte == 0x9c {
                        *emitted_end = self.finish_control::<RECORD_WORK>(
                            sink,
                            self.cursor,
                            ControlFamily::Other,
                        );
                    }
                }
                0xc2 => {
                    *emitted_end = self.finish_cancelled(sink, self.cursor);
                    self.candidate_start = Some(self.cursor);
                    self.state = ScanState::CsiUtf8C1;
                    self.cursor += 1;
                }
                _ => {
                    self.cursor += 1;
                    if (0x40..=0x7e).contains(&byte) {
                        *emitted_end = self.finish_control::<RECORD_WORK>(
                            sink,
                            self.cursor,
                            ControlFamily::Csi,
                        );
                    }
                }
            },
            ScanState::CsiUtf8C1 => {
                if byte == 0x9b {
                    self.cursor += 1;
                    self.state = ScanState::Csi;
                } else {
                    self.state = ScanState::Utf8C1;
                    return true;
                }
            }
            _ => unreachable!("CSI step called for another state"),
        }
        false
    }

    fn step_ground_escape_utf8<const RECORD_WORK: bool>(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
    ) -> bool {
        match self.state {
            ScanState::Ground => self.step_ground(byte, sink, emitted_end),
            ScanState::Escape => self.step_escape::<RECORD_WORK>(byte, sink, emitted_end),
            ScanState::Utf8C1 => {
                return self.step_utf8_c1::<RECORD_WORK>(byte, sink, emitted_end);
            }
            ScanState::Utf8Text {
                remaining,
                next_min,
                next_max,
            } => {
                return self.step_utf8_text(byte, sink, emitted_end, remaining, next_min, next_max);
            }
            _ => unreachable!("ground/escape/UTF-8 step called for another state"),
        }
        false
    }

    fn step_ground(&mut self, byte: u8, sink: &mut impl ScanSink, emitted_end: &mut usize) {
        match byte {
            0x05 => self.emit_single_control(sink, emitted_end, ControlFamily::Enq),
            0x1b => self.begin_candidate(sink, emitted_end, ScanState::Escape),
            0x9b => self.begin_candidate(sink, emitted_end, ScanState::Csi),
            0x9d => self.begin_candidate(sink, emitted_end, ScanState::Osc),
            0x90 => self.begin_candidate(sink, emitted_end, ScanState::Dcs),
            0x98 | 0x9e | 0x9f => {
                self.begin_candidate(sink, emitted_end, ScanState::String);
            }
            0x9c => self.emit_single_control(sink, emitted_end, ControlFamily::Other),
            0xc2 => self.begin_candidate(sink, emitted_end, ScanState::Utf8C1),
            0xc3..=0xdf => self.begin_utf8_text(sink, emitted_end, 1, 0x80, 0xbf),
            0xe0 => self.begin_utf8_text(sink, emitted_end, 2, 0xa0, 0xbf),
            0xe1..=0xec | 0xee..=0xef => {
                self.begin_utf8_text(sink, emitted_end, 2, 0x80, 0xbf);
            }
            0xed => self.begin_utf8_text(sink, emitted_end, 2, 0x80, 0x9f),
            0xf0 => self.begin_utf8_text(sink, emitted_end, 3, 0x90, 0xbf),
            0xf1..=0xf3 => self.begin_utf8_text(sink, emitted_end, 3, 0x80, 0xbf),
            0xf4 => self.begin_utf8_text(sink, emitted_end, 3, 0x80, 0x8f),
            _ => self.cursor += 1,
        }
    }

    fn begin_utf8_text(
        &mut self,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
        remaining: u8,
        next_min: u8,
        next_max: u8,
    ) {
        self.begin_candidate(
            sink,
            emitted_end,
            ScanState::Utf8Text {
                remaining,
                next_min,
                next_max,
            },
        );
    }

    fn begin_candidate(
        &mut self,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
        state: ScanState,
    ) {
        Self::push_bytes(&self.pending, sink, *emitted_end, self.cursor);
        *emitted_end = self.cursor;
        self.candidate_start = Some(self.cursor);
        self.state = state;
        self.cursor += 1;
    }

    fn emit_single_control(
        &mut self,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
        family: ControlFamily,
    ) {
        Self::push_bytes(&self.pending, sink, *emitted_end, self.cursor);
        let end = self.cursor + 1;
        Self::push_control(&self.pending, sink, self.cursor, end, family);
        *emitted_end = end;
        self.cursor = end;
    }

    fn step_escape<const RECORD_WORK: bool>(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
    ) {
        self.cursor += 1;
        match byte {
            0x18 | 0x1a => *emitted_end = self.finish_cancelled(sink, self.cursor),
            b'[' => self.state = ScanState::Csi,
            b']' => self.state = ScanState::Osc,
            b'P' => self.state = ScanState::Dcs,
            b'X' | b'^' | b'_' => self.state = ScanState::String,
            _ => {
                *emitted_end =
                    self.finish_control::<RECORD_WORK>(sink, self.cursor, ControlFamily::Other);
            }
        }
    }

    fn step_utf8_c1<const RECORD_WORK: bool>(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
    ) -> bool {
        if !(0x80..=0xbf).contains(&byte) {
            self.finish_invalid_utf8(sink, emitted_end);
            return true;
        }
        self.cursor += 1;
        match byte {
            0x9b => self.state = ScanState::Csi,
            0x9d => self.state = ScanState::Osc,
            0x90 => self.state = ScanState::Dcs,
            0x98 | 0x9e | 0x9f => self.state = ScanState::String,
            0x9c => {
                *emitted_end =
                    self.finish_control::<RECORD_WORK>(sink, self.cursor, ControlFamily::Other);
            }
            _ => {
                self.state = ScanState::Ground;
                self.candidate_start = None;
            }
        }
        false
    }

    fn step_utf8_text(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
        remaining: u8,
        next_min: u8,
        next_max: u8,
    ) -> bool {
        if !(next_min..=next_max).contains(&byte) {
            self.finish_invalid_utf8(sink, emitted_end);
            return true;
        }
        self.cursor += 1;
        self.state = if remaining == 1 {
            self.candidate_start = None;
            ScanState::Ground
        } else {
            ScanState::Utf8Text {
                remaining: remaining - 1,
                next_min: 0x80,
                next_max: 0xbf,
            }
        };
        false
    }

    fn finish_invalid_utf8(&mut self, sink: &mut impl ScanSink, emitted_end: &mut usize) {
        let start = self
            .candidate_start
            .take()
            .expect("UTF-8 candidate must have a start");
        Self::push_bytes(&self.pending, sink, start, self.cursor);
        *emitted_end = self.cursor;
        self.state = ScanState::Ground;
    }

    fn step_discarding<const RECORD_WORK: bool>(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
    ) {
        match self.state {
            ScanState::Csi => self.step_discarding_csi::<RECORD_WORK>(byte, sink, emitted_end),
            ScanState::CsiUtf8C1 => {
                self.step_discarding_csi_utf8_c1::<RECORD_WORK>(byte, sink, emitted_end);
            }
            ScanState::Osc | ScanState::Dcs | ScanState::String => {
                self.step_discarding_string_body(byte, emitted_end);
            }
            ScanState::OscEscape | ScanState::DcsEscape | ScanState::StringEscape => {
                self.step_discarding_string_escape(byte, emitted_end);
            }
            ScanState::OscUtf8C1 | ScanState::DcsUtf8C1 | ScanState::StringUtf8C1 => {
                self.step_discarding_string_utf8_c1(byte, emitted_end);
            }
            _ => self.finish_discarding_byte(emitted_end),
        }
    }

    fn step_discarding_csi<const RECORD_WORK: bool>(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
    ) {
        match byte {
            0x18 | 0x1a | 0x40..=0x7e => self.finish_discarding_byte(emitted_end),
            0x1b => {
                *emitted_end = self.cursor;
                self.candidate_start = Some(self.cursor);
                self.state = ScanState::Escape;
                self.discarding = false;
                self.cursor += 1;
            }
            0x9b | 0x9d | 0x90 | 0x98 | 0x9e | 0x9f | 0x9c => {
                *emitted_end = self.cursor;
                self.candidate_start = Some(self.cursor);
                self.state = Self::state_for_c1_control(byte);
                self.discarding = false;
                self.cursor += 1;
                if byte == 0x9c {
                    *emitted_end =
                        self.finish_control::<RECORD_WORK>(sink, self.cursor, ControlFamily::Other);
                }
            }
            0xc2 => {
                self.candidate_start = Some(self.cursor);
                self.state = ScanState::CsiUtf8C1;
                self.cursor += 1;
            }
            _ => self.cursor += 1,
        }
    }

    fn step_discarding_csi_utf8_c1<const RECORD_WORK: bool>(
        &mut self,
        byte: u8,
        sink: &mut impl ScanSink,
        emitted_end: &mut usize,
    ) {
        if matches!(byte, 0x9b | 0x9d | 0x90 | 0x98 | 0x9e | 0x9f | 0x9c) {
            self.state = Self::state_for_c1_control(byte);
            self.discarding = false;
            self.cursor += 1;
            if byte == 0x9c {
                *emitted_end =
                    self.finish_control::<RECORD_WORK>(sink, self.cursor, ControlFamily::Other);
            }
        } else if (0x80..=0xbf).contains(&byte) {
            self.candidate_start = None;
            self.state = ScanState::Csi;
            self.cursor += 1;
        } else {
            self.candidate_start = None;
            self.state = ScanState::Csi;
        }
    }

    fn state_for_c1_control(byte: u8) -> ScanState {
        match byte {
            0x9b => ScanState::Csi,
            0x9d => ScanState::Osc,
            0x90 => ScanState::Dcs,
            0x98 | 0x9e | 0x9f => ScanState::String,
            0x9c => ScanState::Ground,
            _ => unreachable!("caller must pass a C1 control"),
        }
    }

    fn step_discarding_string_body(&mut self, byte: u8, emitted_end: &mut usize) {
        match byte {
            0x18 | 0x1a | 0x9c => self.finish_discarding_byte(emitted_end),
            0x07 if self.state == ScanState::Osc => self.finish_discarding_byte(emitted_end),
            0x1b => {
                self.state = match self.state {
                    ScanState::Osc => ScanState::OscEscape,
                    ScanState::Dcs => ScanState::DcsEscape,
                    ScanState::String => ScanState::StringEscape,
                    _ => unreachable!(),
                };
                self.cursor += 1;
            }
            0xc2 => {
                self.state = match self.state {
                    ScanState::Osc => ScanState::OscUtf8C1,
                    ScanState::Dcs => ScanState::DcsUtf8C1,
                    ScanState::String => ScanState::StringUtf8C1,
                    _ => unreachable!(),
                };
                self.cursor += 1;
            }
            _ => self.cursor += 1,
        }
    }

    fn step_discarding_string_escape(&mut self, byte: u8, emitted_end: &mut usize) {
        if byte == b'\\' {
            self.finish_discarding_byte(emitted_end);
            return;
        }
        let start = if byte == 0x1b {
            self.cursor
        } else {
            self.cursor - 1
        };
        *emitted_end = start;
        self.candidate_start = Some(start);
        self.state = ScanState::Escape;
        self.discarding = false;
        if byte == 0x1b {
            self.cursor += 1;
        }
    }

    fn step_discarding_string_utf8_c1(&mut self, byte: u8, emitted_end: &mut usize) {
        let parent = match self.state {
            ScanState::OscUtf8C1 => ScanState::Osc,
            ScanState::DcsUtf8C1 => ScanState::Dcs,
            ScanState::StringUtf8C1 => ScanState::String,
            _ => unreachable!(),
        };
        if !(0x80..=0xbf).contains(&byte) {
            self.state = parent;
            return;
        }
        self.cursor += 1;
        if byte == 0x9c {
            *emitted_end = self.cursor;
            self.state = ScanState::Ground;
            self.discarding = false;
        } else {
            self.state = parent;
        }
    }

    fn finish_discarding_byte(&mut self, emitted_end: &mut usize) {
        self.cursor += 1;
        *emitted_end = self.cursor;
        self.state = ScanState::Ground;
        self.discarding = false;
    }

    fn compact_before_append(&mut self, additional: usize) {
        if self.head == 0 {
            return;
        }
        let required = self.pending.len().saturating_add(additional);
        if required <= self.pending.capacity() && required <= Self::MAX_PENDING {
            return;
        }
        self.compact_live_tail();
    }

    fn compact_live_tail(&mut self) {
        let consumed = self.head;
        if consumed == 0 {
            return;
        }
        let live_len = self.pending.len().saturating_sub(consumed);
        if live_len > 0 {
            self.pending.copy_within(consumed.., 0);
            self.storage_counters.compactions = self.storage_counters.compactions.saturating_add(1);
            self.storage_counters.compacted_bytes = self
                .storage_counters
                .compacted_bytes
                .saturating_add(u64::try_from(live_len).unwrap_or(u64::MAX));
        }
        self.pending.truncate(live_len);
        self.cursor = self.cursor.saturating_sub(consumed);
        self.candidate_start = self
            .candidate_start
            .map(|start| start.saturating_sub(consumed));
        self.head = 0;
    }

    fn finish_control<const RECORD_WORK: bool>(
        &mut self,
        sink: &mut impl ScanSink,
        end: usize,
        family: ControlFamily,
    ) -> usize {
        let start = self
            .candidate_start
            .take()
            .expect("control sequence must have a start");
        if RECORD_WORK {
            self.inspected_bytes = self
                .inspected_bytes
                .saturating_add(u64::try_from(end.saturating_sub(start)).unwrap_or(u64::MAX));
        }
        Self::push_control(&self.pending, sink, start, end, family);
        self.state = ScanState::Ground;
        end
    }

    fn finish_cancelled(&mut self, sink: &mut impl ScanSink, end: usize) -> usize {
        let start = self
            .candidate_start
            .take()
            .expect("cancelled control sequence must have a start");
        if start < end {
            sink.push_control(
                ControlFamily::Other,
                &self.pending[start..end],
                SemanticControl::Cancelled,
            );
        }
        self.state = ScanState::Ground;
        end
    }

    fn push_bytes(pending: &[u8], sink: &mut impl ScanSink, start: usize, end: usize) {
        if start < end {
            sink.push_bytes(&pending[start..end]);
        }
    }

    fn push_control(
        pending: &[u8],
        sink: &mut impl ScanSink,
        start: usize,
        end: usize,
        family: ControlFamily,
    ) {
        sink.push_control(
            family,
            &pending[start..end],
            classify_control(family, &pending[start..end]),
        );
    }
}

#[cfg(test)]
impl Drop for TerminalQueryScanner {
    fn drop(&mut self) {
        task10_trace::trace_drop(self);
    }
}

#[cfg(test)]
#[path = "queries_task10_trace.rs"]
mod task10_trace;

#[cfg(test)]
pub(crate) fn replay_task10_fixture(test_name: &str) -> bool {
    tests::replay_task10_fixture(test_name)
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    use super::{
        ClipboardCommand, ControlFamily, FixedQuery, KeyModifierOptions, KittyKeyboardApplyMode,
        KittyKeyboardMode, KittyKeyboardOperation, MAX_CLIPBOARD_BYTES, MAX_OSC_COLOR_QUERY_KINDS,
        PrivateModeSequence, ScannedSegment, SemanticControl, TerminalQueryScanner,
        decode_clipboard_payload,
    };

    mod task10_registry {
        include!("queries_task10_registry.rs");
    }

    fn scan_in_chunks(input: &[u8], chunk_size: usize) -> (Vec<ScannedSegment>, u64) {
        let mut scanner = TerminalQueryScanner::new_with_work_counter();
        let mut segments = Vec::new();
        for chunk in input.chunks(chunk_size) {
            segments.extend(scanner.process(chunk));
        }
        (segments, scanner.inspected_bytes())
    }

    fn flattened(segments: &[ScannedSegment]) -> Vec<u8> {
        segments
            .iter()
            .flat_map(|segment| segment.bytes().iter().copied())
            .collect()
    }

    fn controls(segments: &[ScannedSegment]) -> Vec<(ControlFamily, Vec<u8>)> {
        segments
            .iter()
            .filter_map(|segment| match segment {
                ScannedSegment::Control { family, bytes, .. } => Some((*family, bytes.clone())),
                ScannedSegment::Bytes(_) => None,
            })
            .collect()
    }

    fn semantics(segments: &[ScannedSegment]) -> Vec<&SemanticControl> {
        segments
            .iter()
            .filter_map(|segment| match segment {
                ScannedSegment::Control { semantic, .. } => Some(semantic),
                ScannedSegment::Bytes(_) => None,
            })
            .collect()
    }

    #[test]
    fn terminal_queries_frames_every_existing_fixed_and_dynamic_family() {
        let cases: &[(&[u8], ControlFamily)] = &[
            (b"\x1b[6n", ControlFamily::Csi),
            (b"\x1b[c", ControlFamily::Csi),
            (b"\x1b[>c", ControlFamily::Csi),
            (b"\x1b[=c", ControlFamily::Csi),
            (b"\x1b[x", ControlFamily::Csi),
            (b"\x1b[>q", ControlFamily::Csi),
            (b"\x1b[5n", ControlFamily::Csi),
            (b"\x1b[14t", ControlFamily::Csi),
            (b"\x1b[16t", ControlFamily::Csi),
            (b"\x1b[18t", ControlFamily::Csi),
            (b"\x1b[?25$p", ControlFamily::Csi),
            (b"\x1b[4$p", ControlFamily::Csi),
            (b"\x1b[?2026h", ControlFamily::Csi),
            (b"\x1b[?u", ControlFamily::Csi),
            (b"\x1b[>4m", ControlFamily::Csi),
            (b"\x1b[1;1;1;1;1;1*y", ControlFamily::Csi),
            (b"\x1b]4;1;?\x07", ControlFamily::Osc),
            (b"\x1b]1337;ReportCellSize\x1b\\", ControlFamily::Osc),
            (b"\x1b]52;c;?\x07", ControlFamily::Osc),
            (b"\x1b]8;;https://example.test\x1b\\", ControlFamily::Osc),
            (b"\x1bP$qm\x1b\\", ControlFamily::Dcs),
            (b"\x1bP+q544e\x1b\\", ControlFamily::Dcs),
            (b"\x1bP?q\x1b\\", ControlFamily::Dcs),
            (b"\x05", ControlFamily::Enq),
        ];

        for &(query, expected_family) in cases {
            let (segments, _) = scan_in_chunks(query, query.len());
            assert_eq!(flattened(&segments), query);
            assert_eq!(
                controls(&segments),
                vec![(expected_family, query.to_vec())],
                "query {query:?}"
            );
        }
    }

    #[test]
    fn terminal_queries_frames_all_48_fixed_query_forms() {
        let fixed: &[&[u8]] = &[
            b"\x1b[6n",
            b"\x9b6n",
            b"\xc2\x9b6n",
            b"\x1b[c",
            b"\x1b[0c",
            b"\x9bc",
            b"\xc2\x9bc",
            b"\x9b0c",
            b"\xc2\x9b0c",
            b"\x1b[>c",
            b"\x1b[>0c",
            b"\x9b>c",
            b"\x9b>0c",
            b"\xc2\x9b>c",
            b"\xc2\x9b>0c",
            b"\x1b[=c",
            b"\x1b[=0c",
            b"\x9b=c",
            b"\x9b=0c",
            b"\xc2\x9b=c",
            b"\xc2\x9b=0c",
            b"\x1b[x",
            b"\x1b[0x",
            b"\x1b[1x",
            b"\x9bx",
            b"\x9b0x",
            b"\x9b1x",
            b"\xc2\x9bx",
            b"\xc2\x9b0x",
            b"\xc2\x9b1x",
            b"\x1b[>q",
            b"\x1b[>0q",
            b"\x9b>q",
            b"\xc2\x9b>q",
            b"\x9b>0q",
            b"\xc2\x9b>0q",
            b"\x1b[5n",
            b"\x9b5n",
            b"\xc2\x9b5n",
            b"\x1b[14t",
            b"\x9b14t",
            b"\xc2\x9b14t",
            b"\x1b[16t",
            b"\x9b16t",
            b"\xc2\x9b16t",
            b"\x1b[18t",
            b"\x9b18t",
            b"\xc2\x9b18t",
        ];
        assert_eq!(fixed.len(), 48);
        for &query in fixed {
            let (segments, _) = scan_in_chunks(query, 1);
            assert_eq!(
                controls(&segments),
                vec![(ControlFamily::Csi, query.to_vec())],
                "query {query:?}"
            );
        }
    }

    #[test]
    fn terminal_queries_semantically_classifies_all_48_fixed_queries() {
        let fixed: &[(&[u8], FixedQuery)] = &[
            (b"\x1b[6n", FixedQuery::CursorPosition),
            (b"\x9b6n", FixedQuery::CursorPosition),
            (b"\xc2\x9b6n", FixedQuery::CursorPosition),
            (b"\x1b[c", FixedQuery::PrimaryDeviceAttributes),
            (b"\x1b[0c", FixedQuery::PrimaryDeviceAttributes),
            (b"\x9bc", FixedQuery::PrimaryDeviceAttributes),
            (b"\xc2\x9bc", FixedQuery::PrimaryDeviceAttributes),
            (b"\x9b0c", FixedQuery::PrimaryDeviceAttributes),
            (b"\xc2\x9b0c", FixedQuery::PrimaryDeviceAttributes),
            (b"\x1b[>c", FixedQuery::SecondaryDeviceAttributes),
            (b"\x1b[>0c", FixedQuery::SecondaryDeviceAttributes),
            (b"\x9b>c", FixedQuery::SecondaryDeviceAttributes),
            (b"\x9b>0c", FixedQuery::SecondaryDeviceAttributes),
            (b"\xc2\x9b>c", FixedQuery::SecondaryDeviceAttributes),
            (b"\xc2\x9b>0c", FixedQuery::SecondaryDeviceAttributes),
            (b"\x1b[=c", FixedQuery::TertiaryDeviceAttributes),
            (b"\x1b[=0c", FixedQuery::TertiaryDeviceAttributes),
            (b"\x9b=c", FixedQuery::TertiaryDeviceAttributes),
            (b"\x9b=0c", FixedQuery::TertiaryDeviceAttributes),
            (b"\xc2\x9b=c", FixedQuery::TertiaryDeviceAttributes),
            (b"\xc2\x9b=0c", FixedQuery::TertiaryDeviceAttributes),
            (b"\x1b[x", FixedQuery::TerminalParameters0),
            (b"\x1b[0x", FixedQuery::TerminalParameters0),
            (b"\x1b[1x", FixedQuery::TerminalParameters1),
            (b"\x9bx", FixedQuery::TerminalParameters0),
            (b"\x9b0x", FixedQuery::TerminalParameters0),
            (b"\x9b1x", FixedQuery::TerminalParameters1),
            (b"\xc2\x9bx", FixedQuery::TerminalParameters0),
            (b"\xc2\x9b0x", FixedQuery::TerminalParameters0),
            (b"\xc2\x9b1x", FixedQuery::TerminalParameters1),
            (b"\x1b[>q", FixedQuery::XtVersion),
            (b"\x1b[>0q", FixedQuery::XtVersion),
            (b"\x9b>q", FixedQuery::XtVersion),
            (b"\xc2\x9b>q", FixedQuery::XtVersion),
            (b"\x9b>0q", FixedQuery::XtVersion),
            (b"\xc2\x9b>0q", FixedQuery::XtVersion),
            (b"\x1b[5n", FixedQuery::OperatingStatus),
            (b"\x9b5n", FixedQuery::OperatingStatus),
            (b"\xc2\x9b5n", FixedQuery::OperatingStatus),
            (b"\x1b[14t", FixedQuery::WindowPixelSize),
            (b"\x9b14t", FixedQuery::WindowPixelSize),
            (b"\xc2\x9b14t", FixedQuery::WindowPixelSize),
            (b"\x1b[16t", FixedQuery::CharacterCellSize),
            (b"\x9b16t", FixedQuery::CharacterCellSize),
            (b"\xc2\x9b16t", FixedQuery::CharacterCellSize),
            (b"\x1b[18t", FixedQuery::TextAreaSize),
            (b"\x9b18t", FixedQuery::TextAreaSize),
            (b"\xc2\x9b18t", FixedQuery::TextAreaSize),
        ];
        assert_eq!(fixed.len(), 48);
        for &(query, expected) in fixed {
            let (segments, _) = scan_in_chunks(query, 1);
            assert_eq!(
                semantics(&segments),
                vec![&SemanticControl::Fixed(expected)],
                "query {query:?}"
            );
        }
    }

    #[test]
    fn clipboard_payload_decode_enforces_the_one_megabyte_limit() {
        let allowed = vec![b'a'; MAX_CLIPBOARD_BYTES];
        let oversized = vec![b'a'; MAX_CLIPBOARD_BYTES + 1];

        assert_eq!(
            decode_clipboard_payload(STANDARD.encode(&allowed).as_bytes())
                .unwrap()
                .len(),
            MAX_CLIPBOARD_BYTES
        );
        assert!(decode_clipboard_payload(STANDARD.encode(&oversized).as_bytes()).is_none());
    }

    #[test]
    fn terminal_queries_semantically_classifies_every_dynamic_query_family() {
        type SemanticPredicate = fn(&SemanticControl) -> bool;
        let cases: &[(&[u8], SemanticPredicate)] = &[
            (b"\x1b[21t", |value| {
                matches!(value, SemanticControl::WindowReport(_))
            }),
            (b"\x1b[?25$p", |value| {
                matches!(value, SemanticControl::PrivateModeStatus(25))
            }),
            (b"\x1b[4$p", |value| {
                matches!(value, SemanticControl::AnsiModeStatus(4))
            }),
            (b"\x1b]4;1;?\x07", |value| {
                matches!(value, SemanticControl::OscColor(_))
            }),
            (b"\x1b]1337;ReportCellSize\x1b\\", |value| {
                matches!(value, SemanticControl::ItermReportCellSize)
            }),
            (b"\x1b[1;1;1;1;1;1*y", |value| {
                matches!(value, SemanticControl::Decrqcra(_))
            }),
            (b"\x1bP$qm\x1b\\", |value| {
                matches!(value, SemanticControl::Decrqss(_))
            }),
            (b"\x1bP+q544e\x1b\\", |value| {
                matches!(value, SemanticControl::XtGetTcap(_))
            }),
            (b"\x1b[?1;1S", |value| {
                matches!(value, SemanticControl::XtSmGraphics(_))
            }),
            (b"\x1b[?u", |value| {
                matches!(value, SemanticControl::KittyKeyboardFlags)
            }),
            (b"\x1b[?4m", |value| {
                matches!(value, SemanticControl::KeyModifierOptionsQuery(4))
            }),
            (b"\x1b[?2026h", |value| {
                matches!(
                    value,
                    SemanticControl::SynchronizedOutputMode(PrivateModeSequence {
                        modes,
                        enabled: true,
                    }) if modes == &[2026]
                )
            }),
            (b"\x1b[>1u", |value| {
                matches!(
                    value,
                    SemanticControl::KittyKeyboardMode(KittyKeyboardMode {
                        operation: KittyKeyboardOperation::Push,
                        value: 1,
                        apply_mode: KittyKeyboardApplyMode::Replace,
                    })
                )
            }),
            (b"\x1b[>4;2m", |value| {
                matches!(
                    value,
                    SemanticControl::KeyModifierOptionsSequence(KeyModifierOptions {
                        resource: Some(4),
                        value: Some(2),
                    })
                )
            }),
            (b"\x1b]52;c;?\x07", |value| {
                matches!(value, SemanticControl::Osc52(_))
            }),
            (b"\x1b]8;;https://example.test\x1b\\", |value| {
                matches!(value, SemanticControl::Osc8Hyperlink)
            }),
            (b"\x1b]9;hello\x07", |value| {
                matches!(value, SemanticControl::Notification(_))
            }),
        ];
        for &(query, predicate) in cases {
            let (segments, _) = scan_in_chunks(query, 1);
            let semantic = semantics(&segments);
            assert_eq!(semantic.len(), 1, "query {query:?}");
            assert!(predicate(semantic[0]), "query {query:?}: {:?}", semantic[0]);
        }
    }

    #[test]
    fn terminal_queries_rejects_malformed_mode_sequences_and_reserves_clipboard_controls() {
        for malformed in [
            b"\x1b[?2026;badh".as_slice(),
            b"\x1b[?;2026h".as_slice(),
            b"\x1b[?2026;;1h".as_slice(),
            b"\x1b[>badu".as_slice(),
            b"\x1b[=u".as_slice(),
            b"\x1b[=1;4u".as_slice(),
            b"\x1b[>badm".as_slice(),
            b"\x1b[>4;badm".as_slice(),
        ] {
            let (segments, _) = scan_in_chunks(malformed, 1);
            assert!(matches!(
                semantics(&segments).as_slice(),
                [SemanticControl::Unknown]
            ));
            assert_eq!(flattened(&segments), malformed);
        }

        for reserved in [
            b"\x1b]52;c;not-base64!\x07".as_slice(),
            b"\x1b]52;missing-separator\x07".as_slice(),
            b"\x1b]1337;Copy=;not-base64!\x07".as_slice(),
            b"\x1b]1337;CopyToClipboard=clipboard;not-base64!\x07".as_slice(),
            b"\x1b]052;c;not-base64!\x07".as_slice(),
            b"\x1b]00052\x07".as_slice(),
            b"\x9d00052;c;not-base64!\x9c".as_slice(),
            b"\xc2\x9d052;c;not-base64!\xc2\x9c".as_slice(),
            b"\x1b]001337;Copy=;not-base64!\x07".as_slice(),
        ] {
            let (segments, _) = scan_in_chunks(reserved, 1);
            assert!(matches!(
                semantics(&segments).as_slice(),
                [SemanticControl::Ignored]
            ));
        }

        let (clipboard, _) = scan_in_chunks(b"\x1b]00052;c;Y29weQ==\x07", 1);
        assert!(matches!(
            semantics(&clipboard).as_slice(),
            [SemanticControl::Osc52(ClipboardCommand::Write { selection, contents })]
                if selection.as_deref() == Some("c") && contents == "copy"
        ));

        for query in [
            b"\x1b]04;1;?\x07".as_slice(),
            b"\x9d010;?\x9c".as_slice(),
            b"\xc2\x9d0012;?\xc2\x9c".as_slice(),
        ] {
            let (segments, _) = scan_in_chunks(query, 1);
            assert!(matches!(
                semantics(&segments).as_slice(),
                [SemanticControl::OscColor(_)]
            ));
        }
    }

    #[test]
    fn terminal_queries_distinguishes_key_modifier_query_from_reset_sequence() {
        let (query, _) = scan_in_chunks(b"\x1b[?4m", 1);
        assert!(matches!(
            semantics(&query).as_slice(),
            [SemanticControl::KeyModifierOptionsQuery(4)]
        ));

        let (reset, _) = scan_in_chunks(b"\x1b[>4m", 1);
        assert!(matches!(
            semantics(&reset).as_slice(),
            [SemanticControl::KeyModifierOptionsSequence(
                KeyModifierOptions {
                    resource: Some(4),
                    value: None,
                }
            )]
        ));

        let (combined, _) = scan_in_chunks(b"\xc2\x9b?1000;02026h", 1);
        assert!(matches!(
            semantics(&combined).as_slice(),
            [SemanticControl::SynchronizedOutputMode(PrivateModeSequence {
                modes,
                enabled: true,
            })] if modes == &[1000, 2026]
        ));
    }

    #[test]
    fn terminal_queries_fail_closed_on_oversized_reserved_queries_and_recovers() {
        let mut palette = b"\x1b]4;".to_vec();
        for index in 0..=MAX_OSC_COLOR_QUERY_KINDS {
            if index > 0 {
                palette.push(b';');
            }
            palette.extend_from_slice(format!("{};?", index % 256).as_bytes());
        }
        palette.push(b'\x07');
        palette.extend_from_slice(b"\x1b[6n");
        let (segments, _) = scan_in_chunks(&palette, 512);
        assert!(matches!(
            semantics(&segments).as_slice(),
            [
                SemanticControl::Ignored,
                SemanticControl::Fixed(FixedQuery::CursorPosition)
            ]
        ));

        let mut xtgettcap = b"\x1bP+q".to_vec();
        for index in 0..=crate::query_dcs::MAX_XTGETTCAP_NAMES {
            if index > 0 {
                xtgettcap.push(b';');
            }
            xtgettcap.extend_from_slice(b"41");
        }
        xtgettcap.extend_from_slice(b"\x1b\\\x1b[6n");
        let (segments, _) = scan_in_chunks(&xtgettcap, 512);
        assert!(matches!(
            semantics(&segments).as_slice(),
            [
                SemanticControl::Ignored,
                SemanticControl::Fixed(FixedQuery::CursorPosition)
            ]
        ));
    }

    #[test]
    fn terminal_queries_resynchronizes_new_controls_inside_incomplete_csi() {
        for input in [
            b"\x1b[123\x1b[6n".as_slice(),
            b"\x1b[123\x9b6n".as_slice(),
            b"\x1b[123\xc2\x9b6n".as_slice(),
        ] {
            let (segments, _) = scan_in_chunks(input, 1);
            assert!(matches!(
                semantics(&segments).as_slice(),
                [
                    SemanticControl::Cancelled,
                    SemanticControl::Fixed(FixedQuery::CursorPosition)
                ]
            ));
        }
    }

    #[test]
    fn terminal_queries_can_and_sub_cancel_all_control_strings_before_following_queries() {
        let prefixes: &[&[u8]] = &[
            b"\x1b[123",
            b"\x1b]52;c;secret",
            b"\x1bPpayload",
            b"\x1bXpayload",
            b"\x1b^payload",
            b"\x1b_payload",
        ];
        for &prefix in prefixes {
            for cancel in [0x18, 0x1a] {
                let mut input = prefix.to_vec();
                input.push(cancel);
                input.extend_from_slice(b"\x1b]52;c;?\x07\x1b[6n");
                let (segments, _) = scan_in_chunks(&input, 3);
                assert!(matches!(
                    semantics(&segments).as_slice(),
                    [
                        SemanticControl::Cancelled,
                        SemanticControl::Osc52(_),
                        SemanticControl::Fixed(FixedQuery::CursorPosition)
                    ]
                ));
            }
        }
    }

    #[test]
    fn terminal_queries_uses_the_second_escape_for_overlapping_st() {
        let input = b"\x1b]777;payload\x1b\x1b\\after";
        let (segments, _) = scan_in_chunks(input, 1);
        assert_eq!(flattened(&segments), input);
        assert!(matches!(
            semantics(&segments).as_slice(),
            [SemanticControl::Cancelled, SemanticControl::StandaloneSt]
        ));
    }

    #[test]
    fn terminal_queries_reprocesses_invalid_utf8_successors_as_controls() {
        for input in [
            b"\xc2\x1b]52;c;?\x07".as_slice(),
            b"\xe2x\x1b[6n".as_slice(),
            b"\xf0\x9b\x1b[6n".as_slice(),
        ] {
            let (segments, _) = scan_in_chunks(input, 1);
            assert_eq!(flattened(&segments), input);
            assert!(
                semantics(&segments).iter().any(|semantic| matches!(
                    semantic,
                    SemanticControl::Osc52(_) | SemanticControl::Fixed(FixedQuery::CursorPosition)
                )),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn terminal_queries_reprocesses_strict_utf8_range_violations_as_controls() {
        for input in [
            b"\xe0\x80\x9b6n".as_slice(),
            b"\xed\xa0\x9b6n".as_slice(),
            b"\xf0\x80\x80\x9b6n".as_slice(),
            b"\xf4\xbf\x80\x9b6n".as_slice(),
        ] {
            let (segments, _) = scan_in_chunks(input, 1);
            assert!(
                semantics(&segments).iter().any(|semantic| matches!(
                    semantic,
                    SemanticControl::Fixed(FixedQuery::CursorPosition)
                )),
                "input {input:?}, segments {segments:?}"
            );
        }
    }

    #[test]
    fn terminal_queries_holds_only_a_genuine_incomplete_utf8_suffix() {
        let mut scanner = TerminalQueryScanner::new_with_work_counter();
        let first = scanner.process(b"before\xe2\x82");
        assert_eq!(flattened(&first), b"before");
        let second = scanner.process(b"\xacafter");
        assert_eq!(flattened(&second), b"\xe2\x82\xacafter");
    }

    #[test]
    fn terminal_queries_discards_oversized_controls_and_recovers() {
        for (prefix, filler, terminator) in [
            (b"\x1b[".as_slice(), b'1', b"m".as_slice()),
            (b"\x1b]52;c;".as_slice(), b'a', b"\x07".as_slice()),
            (b"\x1bP".as_slice(), b'a', b"\x1b\\".as_slice()),
            (b"\x1bX".as_slice(), b'a', b"\x1b\\".as_slice()),
            (b"\x1b^".as_slice(), b'a', b"\x1b\\".as_slice()),
            (b"\x1b_".as_slice(), b'a', b"\x1b\\".as_slice()),
        ] {
            let mut input = prefix.to_vec();
            input.extend(std::iter::repeat_n(
                filler,
                TerminalQueryScanner::MAX_PENDING + 1,
            ));
            input.extend_from_slice(terminator);
            input.extend_from_slice(b"\x1b[6n");
            let (segments, _) = scan_in_chunks(&input, 8192);
            assert!(matches!(
                semantics(&segments).as_slice(),
                [SemanticControl::Fixed(FixedQuery::CursorPosition)]
            ));
        }
    }

    #[test]
    fn terminal_queries_reprocesses_escape_after_invalid_utf8_c1_in_discard_mode() {
        for prefix in [
            b"\x1b]".as_slice(),
            b"\x1bP".as_slice(),
            b"\x1bX".as_slice(),
            b"\x1b^".as_slice(),
            b"\x1b_".as_slice(),
        ] {
            let mut input = prefix.to_vec();
            input.extend(std::iter::repeat_n(
                b'a',
                TerminalQueryScanner::MAX_PENDING + 1,
            ));
            input.extend_from_slice(b"\xc2\x1b[6n");
            let (segments, _) = scan_in_chunks(&input, 8192);
            assert!(semantics(&segments).iter().any(|semantic| matches!(
                semantic,
                SemanticControl::Fixed(FixedQuery::CursorPosition)
            )));
        }
    }

    #[test]
    fn terminal_queries_keeps_discarding_csi_after_ordinary_utf8_c2_sequence() {
        let mut input = b"\x1b[".to_vec();
        input.extend(std::iter::repeat_n(
            b'1',
            TerminalQueryScanner::MAX_PENDING + 1,
        ));
        input.extend_from_slice(b"\xc2\xa9123;456\x1b[6n");
        let (segments, _) = scan_in_chunks(&input, 8192);
        assert!(matches!(
            semantics(&segments).as_slice(),
            [SemanticControl::Fixed(FixedQuery::CursorPosition)]
        ));
        assert_eq!(flattened(&segments), b"\x1b[6n");

        let mut utf8_c1 = b"\x1b[".to_vec();
        utf8_c1.extend(std::iter::repeat_n(
            b'1',
            TerminalQueryScanner::MAX_PENDING + 1,
        ));
        utf8_c1.extend_from_slice(b"\xc2\x9b6n");
        let (segments, _) = scan_in_chunks(&utf8_c1, 8192);
        assert!(matches!(
            semantics(&segments).as_slice(),
            [SemanticControl::Fixed(FixedQuery::CursorPosition)]
        ));
    }

    #[test]
    fn terminal_queries_preserves_every_byte_boundary_split() {
        let queries: &[&[u8]] = &[
            b"\x1b[?25$p",
            b"\x1b]4;1;?\x1b\\",
            b"\x1bP+q544e\x1b\\",
            b"\xc2\x9b6n",
            b"\xc2\x9d4;1;?\xc2\x9c",
            b"\xc2\x90$qm\xc2\x9c",
        ];

        for &query in queries {
            for split in 1..query.len() {
                let mut scanner = TerminalQueryScanner::new_with_work_counter();
                let mut segments = scanner.process(&query[..split]);
                segments.extend(scanner.process(&query[split..]));
                assert_eq!(
                    flattened(&segments),
                    query,
                    "query {query:?}, split {split}"
                );
                assert_eq!(
                    controls(&segments).len(),
                    1,
                    "query {query:?}, split {split}"
                );
            }
        }
    }

    #[test]
    fn terminal_queries_frames_multiple_queries_in_one_chunk() {
        let input = b"before\x1b[6nbetween\x1b]4;1;?\x07after";
        let (segments, _) = scan_in_chunks(input, input.len());
        assert_eq!(flattened(&segments), input);
        assert_eq!(
            controls(&segments),
            vec![
                (ControlFamily::Csi, b"\x1b[6n".to_vec()),
                (ControlFamily::Osc, b"\x1b]4;1;?\x07".to_vec()),
            ]
        );
    }

    #[test]
    fn terminal_queries_finds_valid_query_after_unknown_same_family_control() {
        let input = b"\x1b[999z\x1b[6n\x1b]777;unknown\x07\x1b]4;1;?\x07";
        let (segments, _) = scan_in_chunks(input, input.len());
        assert_eq!(
            controls(&segments),
            vec![
                (ControlFamily::Csi, b"\x1b[999z".to_vec()),
                (ControlFamily::Csi, b"\x1b[6n".to_vec()),
                (ControlFamily::Osc, b"\x1b]777;unknown\x07".to_vec()),
                (ControlFamily::Osc, b"\x1b]4;1;?\x07".to_vec()),
            ]
        );
    }

    #[test]
    fn terminal_queries_cancels_strings_when_escape_does_not_form_st() {
        let strings: &[&[u8]] = &[
            b"\x1b]777;\x1b[6n\x07",
            b"\x1bPpayload\x1b[6n\x1b\\",
            b"\x1bXpayload\x1b[6n\x1b\\",
            b"\x1b^payload\x1b[6n\x1b\\",
            b"\x1b_payload\x1b[6n\x1b\\",
            b"\x98payload\x1b[6n\x9c",
            b"\x9epayload\x1b[6n\x9c",
            b"\x9fpayload\x1b[6n\x9c",
        ];
        for &control_string in strings {
            let (segments, _) = scan_in_chunks(control_string, 1);
            assert_eq!(flattened(&segments), control_string);
            assert!(
                semantics(&segments)
                    .iter()
                    .any(|semantic| matches!(semantic, SemanticControl::Cancelled))
            );
            assert!(semantics(&segments).iter().any(|semantic| matches!(
                semantic,
                SemanticControl::Fixed(FixedQuery::CursorPosition)
            )));
        }
    }

    #[test]
    fn terminal_queries_does_not_treat_utf8_continuations_as_raw_c1() {
        let input = b"\xc3\x9b6n\xc3\x9d4;1;?\xc3\x90$qm\xc3\x9c";
        let (segments, _) = scan_in_chunks(input, 1);
        assert_eq!(flattened(&segments), input);
        assert!(controls(&segments).is_empty());
    }

    #[test]
    fn terminal_queries_releases_c2_when_the_next_byte_is_not_c1() {
        let input = b"before\xc2\xa9after";
        let (segments, _) = scan_in_chunks(input, 1);
        assert_eq!(flattened(&segments), input);
        assert!(controls(&segments).is_empty());
    }

    #[test]
    fn terminal_queries_frames_standalone_string_terminators() {
        for terminator in [b"\x1b\\".as_slice(), b"\x9c", b"\xc2\x9c"] {
            let (segments, _) = scan_in_chunks(terminator, 1);
            assert_eq!(
                controls(&segments),
                vec![(ControlFamily::Other, terminator.to_vec())]
            );
        }
    }

    #[test]
    fn terminal_queries_passes_unknown_csi_osc_and_dcs_without_loss() {
        let input = b"a\x1b[999zb\x1b]777;unknown\x07c\x1bP+zunknown\x1b\\d";
        let (segments, _) = scan_in_chunks(input, 3);
        assert_eq!(flattened(&segments), input);
        assert_eq!(controls(&segments).len(), 3);
    }

    #[test]
    fn terminal_queries_supports_raw_and_utf8_c1_forms() {
        let input = b"\x9b6n\xc2\x9b5n\x9d4;1;?\x9c\xc2\x90$qm\xc2\x9c";
        let (segments, _) = scan_in_chunks(input, 1);
        assert_eq!(flattened(&segments), input);
        assert_eq!(
            controls(&segments)
                .into_iter()
                .map(|(family, _)| family)
                .collect::<Vec<_>>(),
            vec![
                ControlFamily::Csi,
                ControlFamily::Csi,
                ControlFamily::Osc,
                ControlFamily::Dcs,
            ]
        );
    }

    #[test]
    fn terminal_queries_inspects_no_more_than_four_times_the_input() {
        let input = b"plain output ".repeat(4096);
        let (_, inspected) = scan_in_chunks(&input, 512);
        assert!(
            inspected <= (input.len() as u64).saturating_mul(4),
            "inspected {inspected} bytes for {} input bytes",
            input.len()
        );
    }

    #[test]
    fn terminal_queries_chunk_size_work_ratio_is_bounded() {
        let input = b"plain\x1b[6n\x1b]4;1;?\x07\x1bP$qm\x1b\\".repeat(4096);
        let (_, small_chunk_work) = scan_in_chunks(&input, 512);
        let (_, large_chunk_work) = scan_in_chunks(&input, 16 * 1024);
        let (larger, smaller) = if small_chunk_work >= large_chunk_work {
            (small_chunk_work, large_chunk_work)
        } else {
            (large_chunk_work, small_chunk_work)
        };
        assert!(
            u128::from(larger).saturating_mul(4) <= u128::from(smaller).saturating_mul(5),
            "512-byte work {small_chunk_work}, 16-KiB work {large_chunk_work}"
        );
    }

    #[test]
    fn terminal_queries_work_counter_is_disabled_by_default_and_saturates() {
        let mut normal = TerminalQueryScanner::new();
        let _ = normal.process(b"plain\x1b[6n");
        assert_eq!(normal.inspected_bytes(), 0);

        let mut measured = TerminalQueryScanner::new_with_work_counter();
        measured.inspected_bytes = u64::MAX - 1;
        let _ = measured.process(b"abc");
        assert_eq!(measured.inspected_bytes(), u64::MAX);
    }

    #[test]
    fn task10_query_capture_records_a_replayable_scanner_action() {
        let row_id = "0000000000000000000000000000000000000000000000000000000000000000";
        let (execution, trace) = crate::fixture_trace::capture(row_id, "query", || {
            let mut scanner = TerminalQueryScanner::new_with_work_counter();
            let _ = scanner.process(b"a\x1b[6nb");
        });
        assert!(execution.is_ok());
        let trace = String::from_utf8(trace).expect("query fixture trace UTF-8");
        assert!(trace.contains("action_count=2\n"));
        assert!(trace.contains("api=query.process"));
        assert!(trace.contains("final_object=query:"));
    }

    pub(super) fn replay_task10_fixture(test_name: &str) -> bool {
        task10_registry::replay(test_name)
    }
}
