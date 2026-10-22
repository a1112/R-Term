use std::fmt::Write as _;

use crate::{
    Cell, CellAttachment, CellContent, Color, CursorShape, InlineImageFormat, InlineImageFragment,
    ItermInlineImage, SemanticType, Terminal, TerminalScreenDomain, UnderlineStyle, VerticalAlign,
};

pub(crate) fn encode_terminal(terminal: &Terminal) -> Vec<u8> {
    let mut output = String::new();
    write_terminal(&mut output, terminal);
    output.into_bytes()
}

pub(crate) fn encode_grid(grid: &crate::TerminalGrid) -> String {
    let mut output = format!("size={}:{}\n", grid.size().columns, grid.size().rows);
    for row_index in 0..grid.size().rows {
        let row = grid.row(row_index).expect("terminal trace grid row");
        write_row_parts(
            &mut output,
            isize::try_from(row_index).expect("terminal trace grid row index"),
            row.is_wrapped(),
            row.last_change_seqno(),
            row.cells(),
            row.reflow_overflow(),
        );
    }
    output
}

pub(crate) fn encode_cell(cell: &Cell) -> String {
    let mut output = String::new();
    write_cell(&mut output, cell);
    output
}

pub(crate) fn encode_images(images: &[ItermInlineImage], attachments: &[CellAttachment]) -> String {
    let mut output = String::new();
    write_images(&mut output, images);
    write_attachments(&mut output, attachments);
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

fn write_numbers(output: &mut String, values: &[usize]) {
    for (index, value) in values.iter().enumerate() {
        separator(output, index);
        write!(output, "{value}").expect("write number");
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
