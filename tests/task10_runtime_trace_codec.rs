use std::fmt::Write as _;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use rterm_core::DamageRegion;
use rterm_terminal::{
    Cell, CellAttachment, CellContent, Color, CursorShape, InlineImageFormat, InlineImageFragment,
    ItermInlineImage, SemanticType, Terminal, TerminalScreenDomain, UnderlineStyle, VerticalAlign,
};
use sha2::{Digest, Sha256};

pub(super) struct RuntimeStateView<'a> {
    pub terminal: &'a Terminal,
    pub progress: &'a str,
    pub mode_flags: [bool; 6],
    pub kitty_keyboard_flags: u16,
    pub modify_other_keys: u8,
    pub mouse_input_mode: &'a str,
    pub clipboard_texts: &'a [String],
    pub clipboard_queries: &'a [String],
    pub notifications: &'a [(Option<String>, String)],
    pub filter_state: &'a [u8],
}

#[derive(Clone, PartialEq, Eq)]
pub(super) enum TraceEffect {
    Console(Vec<u8>),
    Transport(Vec<u8>),
    Mode(String),
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

#[derive(Clone, PartialEq, Eq)]
pub(super) enum TraceMetadataChange {
    Set(String),
    Clear,
}

#[derive(Default)]
pub(super) struct TraceMetadata {
    pub title: Option<TraceMetadataChange>,
    pub working_directory: Option<TraceMetadataChange>,
    pub badge_format: Option<TraceMetadataChange>,
    pub progress: Option<String>,
    pub user_vars: Vec<(String, TraceMetadataChange)>,
}

pub(super) struct TraceFeed {
    pub responses: Vec<Vec<u8>>,
    pub visible: Vec<u8>,
    pub raw_damage: Vec<DamageRegion>,
    pub bells: u64,
    pub diagnostics: Vec<String>,
    pub screen_identity_changed: bool,
    pub snapshot_changed: bool,
    pub effects: Vec<TraceEffect>,
    pub metadata: TraceMetadata,
}

pub(super) fn encode_runtime_state(view: &RuntimeStateView<'_>) -> Vec<u8> {
    let mut output = String::new();
    write_terminal(&mut output, view.terminal);
    let [
        application_cursor_keys,
        application_keypad,
        focus_reporting,
        bracketed_paste,
        synchronized_output,
        win32_input_mode,
    ] = view.mode_flags;
    writeln!(
        &mut output,
        "runtime=progress:{};application_cursor:{};application_keypad:{};focus:{};bracketed:{};sync:{};kitty:{};modify_other:{};win32:{};mouse:{}",
        view.progress,
        bit(application_cursor_keys),
        bit(application_keypad),
        bit(focus_reporting),
        bit(bracketed_paste),
        bit(synchronized_output),
        view.kitty_keyboard_flags,
        view.modify_other_keys,
        bit(win32_input_mode),
        view.mouse_input_mode,
    )
    .expect("write runtime state");
    output.push_str("clipboard_texts=");
    write_string_list(&mut output, view.clipboard_texts);
    output.push('\n');
    output.push_str("clipboard_queries=");
    write_string_list(&mut output, view.clipboard_queries);
    output.push('\n');
    output.push_str("notifications=");
    for (index, (title, body)) in view.notifications.iter().enumerate() {
        separator(&mut output, index);
        write!(
            &mut output,
            "{}:{}",
            optional_bytes(title.as_deref()),
            hex(body.as_bytes())
        )
        .expect("write notifications");
    }
    output.push('\n');
    writeln!(&mut output, "filter_state={}", hex(view.filter_state)).expect("write filter state");
    output.into_bytes()
}

pub(super) fn encode_feed(feed: &TraceFeed) -> Vec<u8> {
    let mut output = String::new();
    output.push_str("responses=");
    write_byte_list(&mut output, &feed.responses);
    output.push('\n');
    writeln!(&mut output, "visible={}", hex(&feed.visible)).expect("write visible bytes");
    output.push_str("raw_damage=");
    write_damage(&mut output, &feed.raw_damage);
    output.push('\n');
    output.push_str("normalized_damage=");
    write_normalized_damage(&mut output, &feed.raw_damage);
    output.push('\n');
    writeln!(
        &mut output,
        "bells={};identity={};snapshot={}",
        feed.bells,
        bit(feed.screen_identity_changed),
        bit(feed.snapshot_changed)
    )
    .expect("write feed flags");
    output.push_str("diagnostics=");
    write_string_list(&mut output, &feed.diagnostics);
    output.push('\n');
    write_effects(&mut output, &feed.effects);
    write_metadata(&mut output, &feed.metadata);
    output.into_bytes()
}

pub(super) fn normalize_runtime_input(bytes: &[u8]) -> Vec<u8> {
    let Some(introducer) = bytes.windows(3).position(|window| window == b"\x1b_G") else {
        return bytes.to_vec();
    };
    let control = &bytes[introducer + 3..];
    let Some(payload_start) = control.iter().position(|byte| *byte == b';') else {
        return bytes.to_vec();
    };
    let parameters = &control[..payload_start];
    if !parameters
        .split(|byte| *byte == b',')
        .any(|parameter| parameter == b"t=f")
    {
        return bytes.to_vec();
    }
    let payload_start = introducer + 3 + payload_start + 1;
    let Some(payload_end) = bytes[payload_start..]
        .windows(2)
        .position(|window| window == b"\x1b\\")
        .map(|offset| payload_start + offset)
    else {
        return bytes.to_vec();
    };
    let content_digest = STANDARD
        .decode(&bytes[payload_start..payload_end])
        .ok()
        .and_then(|path| String::from_utf8(path).ok())
        .and_then(|path| std::fs::read(path).ok())
        .map_or_else(|| "unavailable".to_owned(), |content| sha256_hex(&content));
    let token = format!("<TEMP-KITTY-FILE:sha256={content_digest}>");
    let mut normalized = Vec::with_capacity(bytes.len());
    normalized.extend_from_slice(&bytes[..payload_start]);
    normalized.extend_from_slice(token.as_bytes());
    normalized.extend_from_slice(&bytes[payload_end..]);
    normalized
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("write SHA-256");
    }
    output
}

fn write_terminal(output: &mut String, terminal: &Terminal) {
    let dimensions = terminal.stable_dimensions();
    writeln!(
        output,
        "terminal=domain:{};viewport:{};scrollback:{};scrollback_top:{};physical_top:{};seqno:{};identity:{};size:{}:{}",
        screen_domain(dimensions.domain),
        dimensions.viewport_rows,
        dimensions.scrollback_rows,
        dimensions.scrollback_top,
        dimensions.physical_top,
        terminal.current_seqno(),
        terminal.screen_identity_generation(),
        terminal.grid().size().columns,
        terminal.grid().size().rows,
    )
    .expect("write terminal header");
    write_terminal_rows(output, terminal);
    let cursor = terminal.cursor();
    let scroll = terminal.scroll_region();
    let margins = terminal.left_right_margins();
    writeln!(
        output,
        "cursor={}:{};visible:{};blinking:{};shape:{};reverse:{};alternate:{};scroll:{}:{};margins:{}:{}",
        cursor.0,
        cursor.1,
        bit(terminal.cursor_visible()),
        bit(terminal.cursor_blinking()),
        cursor_shape(terminal.cursor_shape()),
        bit(terminal.screen_reverse_video()),
        bit(terminal.alternate_screen_active()),
        scroll.0,
        scroll.1,
        margins.0,
        margins.1,
    )
    .expect("write terminal cursor");
    output.push_str("active_style=");
    write_cell(output, terminal.active_style());
    output.push('\n');
    write_terminal_metadata(output, terminal);
    write_terminal_semantics(output, terminal);
    write_images(output, terminal.inline_images());
    write_attachments(output, terminal.inline_image_attachments());
    let fragments = terminal.inline_image_fragments();
    write_fragments(output, &fragments);
}

fn write_terminal_rows(output: &mut String, terminal: &Terminal) {
    let dimensions = terminal.stable_dimensions();
    for (index, row) in terminal.scrollback().iter().enumerate() {
        write_row_parts(
            output,
            dimensions.scrollback_top + isize::try_from(index).expect("stable row"),
            row.is_wrapped(),
            row.last_change_seqno(),
            row.cells(),
            row.reflow_overflow(),
        );
    }
    for index in 0..terminal.grid().size().rows {
        let row = terminal.grid().row(index).expect("terminal viewport row");
        write_row_parts(
            output,
            dimensions.physical_top + isize::try_from(index).expect("viewport row"),
            row.is_wrapped(),
            row.last_change_seqno(),
            row.cells(),
            row.reflow_overflow(),
        );
    }
}

fn write_terminal_metadata(output: &mut String, terminal: &Terminal) {
    writeln!(
        output,
        "metadata=title:{};icon:{};window:{};cwd:{};badge:{};unicode:{}",
        optional_bytes(terminal.title()),
        optional_bytes(terminal.icon_title()),
        optional_bytes(terminal.window_title()),
        optional_bytes(terminal.current_working_dir()),
        optional_bytes(terminal.badge_format()),
        terminal.unicode_version(),
    )
    .expect("write terminal metadata");
    let mut user_vars = terminal.user_vars().iter().collect::<Vec<_>>();
    user_vars.sort_by(|left, right| left.0.cmp(right.0));
    output.push_str("user_vars=");
    for (index, (name, value)) in user_vars.into_iter().enumerate() {
        separator(output, index);
        write!(output, "{}:{}", hex(name.as_bytes()), hex(value.as_bytes()))
            .expect("write user vars");
    }
    output.push('\n');
}

fn write_terminal_semantics(output: &mut String, terminal: &Terminal) {
    output.push_str("semantic_prompts=");
    write_numbers(output, terminal.semantic_prompt_rows());
    output.push('\n');
    output.push_str("semantic_exits=");
    for (index, exit) in terminal.semantic_command_exits().iter().enumerate() {
        separator(output, index);
        write!(
            output,
            "{}:{}:{}",
            exit.row,
            exit.exit_code
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            optional_bytes(exit.aid.as_deref())
        )
        .expect("write semantic exits");
    }
    output.push('\n');
    output.push_str("semantic_zones=");
    for (index, zone) in terminal.semantic_zones().iter().enumerate() {
        separator(output, index);
        write!(
            output,
            "{}:{}:{}:{}:{}",
            zone.start_y,
            zone.start_x,
            zone.end_y,
            zone.end_x,
            semantic_type(zone.semantic_type)
        )
        .expect("write semantic zones");
    }
    output.push('\n');
}

fn write_row_parts(
    output: &mut String,
    stable_row: isize,
    wrapped: bool,
    seqno: usize,
    cells: &[Cell],
    reflow_overflow: &[Cell],
) {
    write!(
        output,
        "row={stable_row};wrapped={};seqno={};cells=",
        bit(wrapped),
        seqno
    )
    .expect("write row");
    write_cell_runs(output, cells);
    output.push_str(";reflow_overflow=");
    write_cell_runs(output, reflow_overflow);
    output.push('\n');
}

fn write_cell_runs(output: &mut String, cells: &[Cell]) {
    let mut start = 0;
    while start < cells.len() {
        let mut end = start + 1;
        while end < cells.len() && cells[end] == cells[start] {
            end += 1;
        }
        if start != 0 {
            output.push(',');
        }
        write!(output, "{}*", end - start).expect("write cell run length");
        write_cell(output, &cells[start]);
        start = end;
    }
}

fn write_cell(output: &mut String, cell: &Cell) {
    match cell.content() {
        CellContent::Blank => output.push_str("blank"),
        CellContent::Text { grapheme, columns } => {
            write!(output, "text:{}:{columns}", hex(grapheme.as_bytes())).expect("write cell text");
        }
        CellContent::Continuation { leader_delta } => {
            write!(output, "continuation:{leader_delta}").expect("write cell continuation");
        }
    }
    write!(
        output,
        "/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}",
        color(cell.foreground),
        color(cell.background),
        color(cell.underline_color),
        underline_style(cell.underline_style),
        bit(cell.bold),
        bit(cell.faint),
        bit(cell.italic),
        bit(cell.blink),
        bit(cell.rapid_blink),
        bit(cell.underline),
        bit(cell.double_underline),
        bit(cell.conceal),
        bit(cell.strikethrough),
        bit(cell.overline),
        vertical_align(cell.vertical_align),
        bit(cell.inverse),
        bit(cell.protected),
        optional_bytes(cell.hyperlink.as_deref()),
        semantic_type(cell.semantic_type),
        cell.columns(),
        bit(cell.is_continuation()),
    )
    .expect("write cell style");
}

fn write_images(output: &mut String, images: &[ItermInlineImage]) {
    writeln!(output, "images={}", images.len()).expect("write image count");
    for image in images {
        writeln!(
            output,
            "image={}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            image.row,
            image.column,
            optional_bytes(image.name.as_deref()),
            optional_number(image.kitty_image_id),
            optional_number(image.kitty_placement_id),
            optional_number(image.kitty_z_index),
            optional_number(image.size),
            optional_bytes(image.width.as_deref()),
            optional_bytes(image.height.as_deref()),
            optional_bool(image.preserve_aspect_ratio),
            image_format(image.image_format),
            optional_number(image.pixel_width),
            optional_number(image.pixel_height),
            optional_number(image.source_x),
            optional_number(image.source_y),
            optional_number(image.source_width),
            optional_number(image.source_height),
            optional_number(image.target_x),
            optional_number(image.target_y),
            image.data.len(),
            hex(&image.data),
        )
        .expect("write image");
    }
}

fn write_attachments(output: &mut String, attachments: &[CellAttachment]) {
    output.push_str("attachments=");
    for (index, attachment) in attachments.iter().enumerate() {
        separator(output, index);
        write!(
            output,
            "{}:{}:{}:{}:{}",
            attachment.parent_identity,
            attachment.source_row,
            attachment.source_column,
            attachment.row,
            attachment.column
        )
        .expect("write attachment");
    }
    output.push('\n');
}

fn write_fragments(output: &mut String, fragments: &[InlineImageFragment]) {
    writeln!(output, "fragments={}", fragments.len()).expect("write fragment count");
    for fragment in fragments {
        writeln!(
            output,
            "fragment={}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            fragment.image_index,
            bit(fragment.cell_attachment),
            fragment.row,
            fragment.column,
            fragment.source_row,
            fragment.source_column,
            fragment.destination_x,
            fragment.destination_y,
            fragment.destination_width,
            fragment.destination_height,
            fragment.source_x,
            fragment.source_y,
            fragment.source_width,
            fragment.source_height,
            fragment.sampling_source_x,
            fragment.sampling_source_y,
            fragment.sampling_source_width,
            fragment.sampling_source_height,
            fragment.source_destination_x,
            fragment.source_destination_y,
            fragment.source_destination_width,
            fragment.source_destination_height,
            optional_number(fragment.kitty_image_id),
            optional_number(fragment.kitty_placement_id),
            optional_number(fragment.kitty_z_index),
            image_format(fragment.image_format),
            fragment.image_index,
        )
        .expect("write fragment");
    }
}

fn write_effects(output: &mut String, effects: &[TraceEffect]) {
    output.push_str("effects=");
    for (index, effect) in effects.iter().enumerate() {
        separator(output, index);
        match effect {
            TraceEffect::Console(bytes) => write!(output, "console:{}", hex(bytes)),
            TraceEffect::Transport(bytes) => write!(output, "transport:{}", hex(bytes)),
            TraceEffect::Mode(mode) => write!(output, "mode:{}", hex(mode.as_bytes())),
            TraceEffect::Bell(count) => write!(output, "bell:{count}"),
            TraceEffect::ClipboardWrite {
                selection,
                contents,
            } => write!(
                output,
                "clipboard-write:{}:{}",
                optional_bytes(selection.as_deref()),
                hex(contents.as_bytes())
            ),
            TraceEffect::ClipboardRead(selection) => {
                write!(output, "clipboard-read:{}", hex(selection.as_bytes()))
            }
            TraceEffect::Notification { title, body } => write!(
                output,
                "notification:{}:{}",
                optional_bytes(title.as_deref()),
                hex(body.as_bytes())
            ),
            TraceEffect::Diagnostic(message) => {
                write!(output, "diagnostic:{}", hex(message.as_bytes()))
            }
        }
        .expect("write effect");
    }
    output.push('\n');
}

fn write_metadata(output: &mut String, metadata: &TraceMetadata) {
    writeln!(
        output,
        "metadata=title:{};cwd:{};badge:{};progress:{}",
        metadata_change(metadata.title.as_ref()),
        metadata_change(metadata.working_directory.as_ref()),
        metadata_change(metadata.badge_format.as_ref()),
        metadata
            .progress
            .as_deref()
            .map_or_else(|| "no-change".to_owned(), |value| format!("set:{value}")),
    )
    .expect("write metadata");
    let mut vars = metadata.user_vars.iter().collect::<Vec<_>>();
    vars.sort_by(|left, right| left.0.cmp(&right.0));
    output.push_str("metadata_user_vars=");
    for (index, (name, value)) in vars.into_iter().enumerate() {
        separator(output, index);
        write!(
            output,
            "{}:{}",
            hex(name.as_bytes()),
            metadata_change(Some(value))
        )
        .expect("write metadata user var");
    }
    output.push('\n');
}

fn write_damage(output: &mut String, damage: &[DamageRegion]) {
    for (index, region) in damage.iter().enumerate() {
        separator(output, index);
        write!(
            output,
            "{}:{}:{}:{}",
            region.x, region.y, region.width, region.height
        )
        .expect("write damage");
    }
}

fn write_normalized_damage(output: &mut String, damage: &[DamageRegion]) {
    let mut spans = Vec::new();
    for region in damage.iter().copied().filter(|region| !region.is_empty()) {
        let end_x = u32::from(region.x) + u32::from(region.width);
        let end_y = u32::from(region.y) + u32::from(region.height);
        for row in u32::from(region.y)..end_y {
            spans.push((row, u32::from(region.x), end_x));
        }
    }
    spans.sort_unstable();
    let mut merged: Vec<(u32, u32, u32)> = Vec::new();
    for span in spans {
        if let Some(last) = merged.last_mut()
            && last.0 == span.0
            && span.1 <= last.2
        {
            last.2 = last.2.max(span.2);
        } else {
            merged.push(span);
        }
    }
    for (index, (row, start, end)) in merged.into_iter().enumerate() {
        separator(output, index);
        write!(output, "{row}:{start}-{end}").expect("write normalized damage");
    }
}

fn write_byte_list(output: &mut String, values: &[Vec<u8>]) {
    for (index, value) in values.iter().enumerate() {
        separator(output, index);
        output.push_str(&hex(value));
    }
}

fn write_string_list(output: &mut String, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        separator(output, index);
        output.push_str(&hex(value.as_bytes()));
    }
}

fn write_numbers(output: &mut String, values: &[usize]) {
    for (index, value) in values.iter().enumerate() {
        separator(output, index);
        write!(output, "{value}").expect("write number");
    }
}

fn metadata_change(change: Option<&TraceMetadataChange>) -> String {
    match change {
        None => "no-change".to_owned(),
        Some(TraceMetadataChange::Clear) => "clear".to_owned(),
        Some(TraceMetadataChange::Set(value)) => format!("set:{}", hex(value.as_bytes())),
    }
}

fn color(value: Color) -> String {
    match value {
        Color::Default => "default".to_owned(),
        Color::Indexed(index) => format!("indexed:{index}"),
        Color::Rgb(red, green, blue) => format!("rgb:{red}:{green}:{blue}"),
        Color::Rgba(red, green, blue, alpha) => format!("rgba:{red}:{green}:{blue}:{alpha}"),
    }
}

const fn screen_domain(value: TerminalScreenDomain) -> &'static str {
    match value {
        TerminalScreenDomain::Main => "main",
        TerminalScreenDomain::Alternate => "alternate",
    }
}

const fn cursor_shape(value: CursorShape) -> &'static str {
    match value {
        CursorShape::Block => "block",
        CursorShape::Underline => "underline",
        CursorShape::Bar => "bar",
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
        VerticalAlign::Baseline => "baseline",
        VerticalAlign::Superscript => "superscript",
        VerticalAlign::Subscript => "subscript",
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

fn optional_bytes(value: Option<&str>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |value| format!("some:{}", hex(value.as_bytes())),
    )
}

fn optional_number(value: Option<impl ToString>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |value| format!("some:{}", value.to_string()),
    )
}

fn optional_bool(value: Option<bool>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| format!("some:{}", bit(value)))
}

const fn bit(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

fn separator(output: &mut String, index: usize) {
    if index != 0 {
        output.push(',');
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
