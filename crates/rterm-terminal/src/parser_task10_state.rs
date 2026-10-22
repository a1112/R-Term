use std::fmt::Write as _;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};

use super::super::*;
use super::terminal_codec;

pub(super) fn write_stacks_and_saved_cursor(output: &mut String, terminal: &Terminal) {
    let titles = terminal
        .title_stack
        .iter()
        .map(|value| optional_string(value.as_deref()))
        .collect::<Vec<_>>()
        .join(",");
    let unicode = terminal
        .unicode_version_stack
        .iter()
        .map(|entry| {
            format!(
                "{}:{}",
                entry.version,
                optional_string(entry.label.as_deref())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    writeln!(
        output,
        "stacks=title:{titles};unicode:{unicode};default_cursor:{};saved:{}",
        cursor_style(terminal.default_cursor_style),
        saved_cursor(terminal.saved_cursor.as_ref()),
    )
    .expect("write terminal stacks");
}

pub(super) fn write_main_screen(output: &mut String, screen: Option<&ScreenState>) {
    let Some(screen) = screen else {
        output.push_str("main_screen=none\n");
        return;
    };
    let grid = terminal_codec::encode_grid(&screen.grid);
    let images =
        terminal_codec::encode_images(&screen.inline_images, &screen.inline_image_attachments);
    writeln!(
        output,
        "main_screen=some;cursor={}:{};pending_wrap={};clear_semantic={};last_printable={};nfc_cell={};saved={};modes={};scroll={}:{};margins={}:{};charset={};style={};parent_ids={};edited={};last_placeholder={};placeholder_cells={};grid={};images={}",
        screen.cursor_row,
        screen.cursor_column,
        u8::from(screen.pending_wrap),
        u8::from(screen.clear_semantic_type_on_movement),
        screen.last_printable.as_ref().map_or_else(
            || "none".to_owned(),
            |value| format!("some:{}", crate::fixture_trace::encode_hex(value.as_bytes()))
        ),
        optional_coordinate(screen.nfc_last_printable_cell),
        saved_cursor(screen.saved_cursor.as_ref()),
        modes(&screen.modes),
        screen.scroll_top,
        screen.scroll_bottom,
        screen.left_margin,
        screen.right_margin,
        character_set(screen.character_set),
        terminal_codec::encode_cell(&screen.style),
        join_u64(&screen.inline_image_parent_ids),
        placement_set(&screen.kitty_character_edited_placements),
        last_placeholder(screen.last_kitty_placeholder.as_ref()),
        placeholder_cells(&screen.kitty_placeholder_cells),
        crate::fixture_trace::encode_hex(grid.as_bytes()),
        crate::fixture_trace::encode_hex(images.as_bytes()),
    )
    .expect("write dormant main screen");
}

pub(super) fn write_kitty_state(output: &mut String, terminal: &Terminal) {
    writeln!(
        output,
        "kitty=pending:{};images:{};numbers:{};relative:{};virtual:{};edited:{};pending_placeholder:{};last_placeholder:{};placeholder_cells:{}",
        pending_kitty(terminal.pending_kitty_graphics.as_ref()),
        kitty_images(&terminal.kitty_images),
        kitty_numbers(&terminal.kitty_image_numbers),
        relative_parents(&terminal.kitty_relative_parents),
        virtual_placements(&terminal.kitty_virtual_placements),
        placement_set(&terminal.kitty_character_edited_placements),
        pending_placeholder(terminal.pending_kitty_placeholder.as_ref()),
        last_placeholder(terminal.last_kitty_placeholder.as_ref()),
        placeholder_cells(&terminal.kitty_placeholder_cells),
    )
    .expect("write kitty state");
}

pub(super) fn normalize_terminal_input(bytes: &[u8]) -> Vec<u8> {
    let Some(introducer) = bytes.windows(3).position(|window| window == b"\x1b_G") else {
        return bytes.to_vec();
    };
    let control = &bytes[introducer + 3..];
    let Some(payload_start) = control.iter().position(|byte| *byte == b';') else {
        return bytes.to_vec();
    };
    if !control[..payload_start]
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
    let digest = decoded_file_digest(&bytes[payload_start..payload_end]);
    let mut normalized = Vec::with_capacity(bytes.len());
    normalized.extend_from_slice(&bytes[..payload_start]);
    normalized.extend_from_slice(format!("<TEMP-KITTY-FILE:sha256={digest}>").as_bytes());
    normalized.extend_from_slice(&bytes[payload_end..]);
    normalized
}

fn pending_kitty(value: Option<&PendingKittyGraphics>) -> String {
    let Some(value) = value else {
        return "none".to_owned();
    };
    let data = if value.medium == KittyTransmissionMedium::TempFile {
        format!(
            "file:{}",
            decoded_file_digest(value.encoded_data.as_bytes())
        )
    } else {
        format!(
            "data:{}",
            crate::fixture_trace::encode_hex(value.encoded_data.as_bytes())
        )
    };
    format!(
        "some:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{data}",
        image_format(value.image_format),
        transmission_medium(value.medium),
        option_char(value.compression),
        upload_action(value.action),
        option_number(value.image_id),
        option_number(value.image_number),
        option_number(value.placement_id),
        option_number(value.z_index),
        option_number(value.pixel_width),
        option_number(value.pixel_height),
        option_number(value.display_columns),
        option_number(value.display_rows),
        u8::from(value.no_cursor_movement),
        option_number(value.source_x),
        option_number(value.source_y),
        option_number(value.source_width),
        option_number(value.source_height),
        option_number(value.target_x),
        option_number(value.target_y),
        option_number(value.file_offset),
        option_number(value.file_size),
        option_number(value.quiet),
        u8::from(value.virtual_placement),
        option_number(value.image_id),
        option_number(value.placement_id),
    )
}

fn kitty_images(values: &HashMap<u32, StoredKittyImage>) -> String {
    let mut values = values.iter().collect::<Vec<_>>();
    values.sort_by_key(|(key, _)| **key);
    values
        .into_iter()
        .map(|(key, value)| {
            format!(
                "{}:{}:{}:{}:{}:{}:{}:{}",
                key,
                image_format(value.image_format),
                option_number(value.pixel_width),
                option_number(value.pixel_height),
                option_number(value.display_columns),
                option_number(value.display_rows),
                value.data.len(),
                crate::fixture_trace::encode_hex(&value.data),
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn kitty_numbers(values: &HashMap<u32, u32>) -> String {
    let mut values = values
        .iter()
        .map(|(key, value)| (*key, *value))
        .collect::<Vec<_>>();
    values.sort_unstable();
    values
        .into_iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn relative_parents(values: &HashMap<KittyPlacementKey, KittyPlacementKey>) -> String {
    let mut values = values
        .iter()
        .map(|(key, value)| (*key, *value))
        .collect::<Vec<_>>();
    values.sort_unstable();
    values
        .into_iter()
        .map(|(key, value)| format!("{}:{}>{}:{}", key.0, key.1, value.0, value.1))
        .collect::<Vec<_>>()
        .join(",")
}

fn virtual_placements(values: &HashMap<KittyPlacementKey, KittyVirtualPlacement>) -> String {
    let mut values = values.iter().collect::<Vec<_>>();
    values.sort_by_key(|(key, _)| **key);
    values
        .into_iter()
        .map(|(key, value)| {
            format!(
                "{}:{}={}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                key.0,
                key.1,
                value.image_id,
                option_number(value.placement_id),
                option_number(value.z_index),
                option_number(value.display_columns),
                option_number(value.display_rows),
                option_number(value.source_rect.x),
                option_number(value.source_rect.y),
                option_number(value.source_rect.width),
                option_number(value.source_rect.height),
                option_number(value.target_x),
                option_number(value.target_y),
                value.image_id,
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn placement_set(values: &HashSet<KittyPlacementKey>) -> String {
    let mut values = values.iter().copied().collect::<Vec<_>>();
    values.sort_unstable();
    values
        .into_iter()
        .map(|value| format!("{}:{}", value.0, value.1))
        .collect::<Vec<_>>()
        .join(",")
}

fn pending_placeholder(value: Option<&PendingKittyPlaceholder>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |value| {
            format!(
                "some:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                value.row,
                value.column,
                color(value.foreground),
                color(value.underline_color),
                option_number(value.image_id),
                option_number(value.placement_id),
                join_u32(&value.diacritics),
                option_number(value.rendered_row),
                option_number(value.rendered_column),
                option_number(value.rendered_image_id),
            )
        },
    )
}

fn last_placeholder(value: Option<&LastKittyPlaceholder>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |value| {
            format!(
                "some:{}:{}:{}:{}:{}:{}:{}",
                value.row,
                value.column,
                color(value.foreground),
                color(value.underline_color),
                value.image_id_high_byte,
                value.placeholder_row,
                value.placeholder_column,
            )
        },
    )
}

fn placeholder_cells(values: &HashMap<(usize, u16), LastKittyPlaceholder>) -> String {
    let mut values = values.iter().collect::<Vec<_>>();
    values.sort_by_key(|(key, _)| **key);
    values
        .into_iter()
        .map(|(key, value)| format!("{}:{}={}", key.0, key.1, last_placeholder(Some(value))))
        .collect::<Vec<_>>()
        .join(",")
}

fn saved_cursor(value: Option<&SavedCursor>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |value| {
            format!(
                "some:{}:{}:{}:{}:{}:{}:{}",
                value.row,
                value.column,
                u8::from(value.pending_wrap),
                u8::from(value.clear_semantic_type_on_movement),
                u8::from(value.origin_mode),
                character_set(value.character_set),
                terminal_codec::encode_cell(&value.style),
            )
        },
    )
}

fn modes(value: &TerminalModes) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        u8::from(value.cursor_visible),
        u8::from(value.cursor_blinking),
        cursor_shape(value.cursor_shape),
        u8::from(value.screen_reverse),
        u8::from(value.auto_wrap),
        u8::from(value.reverse_wrap),
        u8::from(value.sixel_display_mode),
        u8::from(value.sixel_scrolls_right),
        u8::from(value.origin_mode),
        u8::from(value.left_right_margin_mode),
        match value.write_mode {
            CharacterWriteMode::Replace => "replace",
            CharacterWriteMode::Insert => "insert",
        },
    )
}

pub(super) fn join_u64(values: &[u64]) -> String {
    values
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn join_u32(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn decoded_file_digest(encoded_path: &[u8]) -> String {
    STANDARD
        .decode(encoded_path)
        .ok()
        .and_then(|path| String::from_utf8(path).ok())
        .and_then(|path| std::fs::read(path).ok())
        .map_or_else(|| "unavailable".to_owned(), |content| sha256_hex(&content))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("write SHA-256");
    }
    output
}

fn optional_string(value: Option<&str>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |value| {
            format!(
                "some:{}",
                crate::fixture_trace::encode_hex(value.as_bytes())
            )
        },
    )
}

fn optional_coordinate(value: Option<(u16, u16)>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |(row, column)| format!("some:{row}:{column}"),
    )
}

fn option_number(value: Option<impl ToString>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |value| format!("some:{}", value.to_string()),
    )
}

fn option_char(value: Option<char>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |value| format!("some:{}", u32::from(value)),
    )
}

fn cursor_style(value: CursorStyle) -> String {
    format!(
        "{}:{}",
        cursor_shape(value.shape()),
        u8::from(value.blinking())
    )
}

fn cursor_shape(value: CursorShape) -> &'static str {
    match value {
        CursorShape::Block => "block",
        CursorShape::Underline => "underline",
        CursorShape::Bar => "bar",
    }
}

fn character_set(value: CharacterSet) -> &'static str {
    match value {
        CharacterSet::Ascii => "ascii",
        CharacterSet::DecSpecialGraphics => "dec-special",
    }
}

fn image_format(value: InlineImageFormat) -> &'static str {
    match value {
        InlineImageFormat::Encoded => "encoded",
        InlineImageFormat::Rgb => "rgb",
        InlineImageFormat::Rgba => "rgba",
    }
}

fn transmission_medium(value: KittyTransmissionMedium) -> &'static str {
    match value {
        KittyTransmissionMedium::Direct => "direct",
        KittyTransmissionMedium::File => "file",
        KittyTransmissionMedium::TempFile => "temp-file",
    }
}

fn upload_action(value: KittyUploadAction) -> &'static str {
    match value {
        KittyUploadAction::Display => "display",
        KittyUploadAction::Store => "store",
        KittyUploadAction::Query => "query",
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
