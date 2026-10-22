use std::fmt::Write as _;

use super::*;

#[path = "task10_terminal_trace_codec.rs"]
mod terminal_codec;

#[path = "parser_task10_state.rs"]
mod state_codec;

pub(super) struct ActionStart {
    object: u64,
    state: Vec<u8>,
}

pub(super) fn trace_construct(
    terminal: &Terminal,
    size: TerminalSize,
    default_cursor_style: CursorStyle,
    origin: &'static str,
) -> u64 {
    let state = trace_state(terminal);
    let arguments = format!(
        "origin={origin};columns={};rows={};cursor={}:{}",
        size.columns,
        size.rows,
        cursor_shape_tag(default_cursor_style.shape()),
        u8::from(default_cursor_style.blinking()),
    );
    crate::fixture_trace::new_object(
        "terminal-parser",
        "terminal.construct",
        arguments.as_bytes(),
        b"result=constructed",
        &state,
    )
}

pub(super) fn before_action(terminal: &mut Terminal) -> Option<ActionStart> {
    if terminal.fixture_trace.id == 0 {
        terminal.fixture_trace.id = trace_construct(
            terminal,
            terminal.grid.size(),
            terminal.default_cursor_style,
            "clone",
        );
    }
    (terminal.fixture_trace.id != 0).then(|| ActionStart {
        object: terminal.fixture_trace.id,
        state: trace_state(terminal),
    })
}

pub(super) fn trace_action(
    terminal: &Terminal,
    start: Option<ActionStart>,
    operation: &'static str,
    arguments: &[u8],
    result: &[u8],
) {
    let Some(start) = start else {
        return;
    };
    let state = trace_state(terminal);
    let normalized_arguments;
    let arguments = if matches!(operation, "terminal.feed" | "terminal.feed_all") {
        normalized_arguments = state_codec::normalize_terminal_input(arguments);
        &normalized_arguments
    } else {
        arguments
    };
    crate::fixture_trace::record_action(
        "terminal-parser",
        start.object,
        operation,
        arguments,
        result,
        &start.state,
        &state,
    );
}

pub(super) fn trace_drop(terminal: &Terminal) {
    if terminal.fixture_trace.id == 0 {
        return;
    }
    let state = trace_state(terminal);
    let pending = trace_pending(terminal);
    crate::fixture_trace::finish_object(
        "terminal-parser",
        terminal.fixture_trace.id,
        &pending,
        &state,
        &state,
    );
}

pub(super) fn trace_work_counter_fixture(run: fn()) {
    let earlier = TerminalWorkCounters {
        scrolled_survivor_cell_clones: 100,
        history_row_relocations: 40,
        metadata_rebase_batches: 7,
    };
    let later = TerminalWorkCounters {
        scrolled_survivor_cell_clones: 125,
        history_row_relocations: 35,
        metadata_rebase_batches: 9,
    };
    let result = later.saturating_delta_since(earlier);
    let arguments = b"earlier=100:40:7;later=125:35:9";
    let observables = format!(
        "result={}:{}:{}",
        result.scrolled_survivor_cell_clones,
        result.history_row_relocations,
        result.metadata_rebase_batches,
    );
    let state = b"kind=pure;pending=";
    let object = crate::fixture_trace::new_object(
        "terminal-parser",
        "terminal.work_counter_delta",
        arguments,
        observables.as_bytes(),
        state,
    );
    debug_assert!(object == 0 || crate::fixture_trace::has_object("terminal-parser"));
    run();
    crate::fixture_trace::finish_object("terminal-parser", object, b"", state, state);
}

fn trace_state(terminal: &Terminal) -> Vec<u8> {
    let mut state = terminal_codec::encode_terminal(terminal);
    let mut private = String::new();
    writeln!(
        &mut private,
        "private=scrollback_limit:{};main_offset:{};pending_wrap:{};clear_semantic:{};pending_utf8:{};pending_control:{};last_printable:{};nfc_cell:{};main_screen:{}",
        terminal.scrollback_limit,
        terminal.main_stable_row_offset,
        u8::from(terminal.pending_wrap),
        u8::from(terminal.clear_semantic_type_on_movement),
        crate::fixture_trace::encode_exact_runs(&terminal.pending_utf8),
        encode_chars(&terminal.pending_control),
        terminal.last_printable.as_ref().map_or_else(
            || "none".to_owned(),
            |value| format!("some:{}", crate::fixture_trace::encode_hex(value.as_bytes()))
        ),
        terminal.nfc_last_printable_cell.map_or_else(
            || "none".to_owned(),
            |(row, column)| format!("some:{row}:{column}")
        ),
        u8::from(terminal.main_screen.is_some()),
    )
    .expect("write terminal private state");
    write_modes(&mut private, terminal);
    write_protocol_state(&mut private, terminal);
    state_codec::write_stacks_and_saved_cursor(&mut private, terminal);
    state_codec::write_main_screen(&mut private, terminal.main_screen.as_ref());
    state_codec::write_kitty_state(&mut private, terminal);
    state.extend_from_slice(private.as_bytes());
    state
}

fn write_modes(output: &mut String, terminal: &Terminal) {
    writeln!(
        output,
        "modes=cursor_visible:{};cursor_blinking:{};cursor_shape:{};reverse:{};auto_wrap:{};reverse_wrap:{};sixel_display:{};sixel_scroll_right:{};origin:{};lr_margin:{};write:{};charset:{};tabs:{}",
        u8::from(terminal.modes.cursor_visible),
        u8::from(terminal.modes.cursor_blinking),
        cursor_shape_tag(terminal.modes.cursor_shape),
        u8::from(terminal.modes.screen_reverse),
        u8::from(terminal.modes.auto_wrap),
        u8::from(terminal.modes.reverse_wrap),
        u8::from(terminal.modes.sixel_display_mode),
        u8::from(terminal.modes.sixel_scrolls_right),
        u8::from(terminal.modes.origin_mode),
        u8::from(terminal.modes.left_right_margin_mode),
        match terminal.modes.write_mode {
            CharacterWriteMode::Replace => "replace",
            CharacterWriteMode::Insert => "insert",
        },
        match terminal.character_set {
            CharacterSet::Ascii => "ascii",
            CharacterSet::DecSpecialGraphics => "dec-special",
        },
        join_numbers(&terminal.tab_stops.columns),
    )
    .expect("write terminal modes");
}

fn write_protocol_state(output: &mut String, terminal: &Terminal) {
    writeln!(
        output,
        "protocol=damage:{};bells:{};unknown:{};kitty_responses:{};pending_kitty:{};kitty_images:{};kitty_numbers:{};relative:{};virtual:{};edited:{};placeholder:{};last_placeholder:{};placeholder_cells:{};next_image:{};kitty_enabled:{};next_inline_parent:{};ambiguous_wide:{};normalize_nfc:{};unicode_stack:{};width_overrides:{};work:{}:{}:{}",
        encode_damage(&terminal.damage),
        terminal.bell_count,
        terminal
            .unknown_escape_sequences
            .iter()
            .map(|value| crate::fixture_trace::encode_hex(value.sequence.as_bytes()))
            .collect::<Vec<_>>()
            .join(","),
        encode_byte_vecs(&terminal.kitty_graphics_responses),
        u8::from(terminal.pending_kitty_graphics.is_some()),
        terminal.kitty_images.len(),
        terminal.kitty_image_numbers.len(),
        terminal.kitty_relative_parents.len(),
        terminal.kitty_virtual_placements.len(),
        terminal.kitty_character_edited_placements.len(),
        u8::from(terminal.pending_kitty_placeholder.is_some()),
        u8::from(terminal.last_kitty_placeholder.is_some()),
        terminal.kitty_placeholder_cells.len(),
        terminal.next_kitty_image_id,
        u8::from(terminal.enable_kitty_graphics),
        terminal.next_inline_image_parent_identity,
        u8::from(terminal.treat_east_asian_ambiguous_width_as_wide),
        u8::from(terminal.normalize_output_to_unicode_nfc),
        terminal.unicode_version_stack.len(),
        terminal.cell_width_overrides.len(),
        terminal.work_counters.scrolled_survivor_cell_clones,
        terminal.work_counters.history_row_relocations,
        terminal.work_counters.metadata_rebase_batches,
    )
    .expect("write terminal protocol state");
    writeln!(
        output,
        "inline_parent_ids={};cell_width_overrides={}",
        state_codec::join_u64(&terminal.inline_image_parent_ids),
        String::from_utf8_lossy(&encode_width_overrides(&terminal.cell_width_overrides)),
    )
    .expect("write terminal inline parent state");
}

fn trace_pending(terminal: &Terminal) -> Vec<u8> {
    format!(
        "utf8={};control={};kitty={};placeholder={}",
        crate::fixture_trace::encode_exact_runs(&terminal.pending_utf8),
        encode_chars(&terminal.pending_control),
        u8::from(terminal.pending_kitty_graphics.is_some()),
        u8::from(terminal.pending_kitty_placeholder.is_some()),
    )
    .into_bytes()
}

fn encode_chars(chars: &[char]) -> String {
    chars
        .iter()
        .map(|value| u32::from(*value).to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn encode_damage(damage: &[DamageRegion]) -> String {
    damage
        .iter()
        .map(|value| format!("{}:{}:{}:{}", value.x, value.y, value.width, value.height))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn encode_byte_vecs(values: &[Vec<u8>]) -> String {
    values
        .iter()
        .map(|value| crate::fixture_trace::encode_hex(value))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn encode_unknown(values: &[TerminalUnknownEscapeSequence]) -> Vec<u8> {
    values
        .iter()
        .map(|value| crate::fixture_trace::encode_hex(value.sequence.as_bytes()))
        .collect::<Vec<_>>()
        .join(",")
        .into_bytes()
}

pub(super) fn encode_width_overrides(values: &[CellWidthOverride]) -> Vec<u8> {
    values
        .iter()
        .map(|value| format!("{}:{}:{}", value.first, value.last, value.width))
        .collect::<Vec<_>>()
        .join(",")
        .into_bytes()
}

pub(super) fn encode_cursor_style(value: CursorStyle) -> String {
    format!(
        "{}:{}",
        cursor_shape_tag(value.shape()),
        u8::from(value.blinking())
    )
}

pub(super) fn encode_size(value: TerminalSize) -> Vec<u8> {
    format!("columns={};rows={}", value.columns, value.rows).into_bytes()
}

pub(super) fn encode_resize_outcome(value: TerminalResizeOutcome) -> &'static str {
    match value {
        TerminalResizeOutcome::Unchanged => "unchanged",
        TerminalResizeOutcome::PhysicalResize => "physical",
        TerminalResizeOutcome::MainScreenReflowed => "main-reflow",
        TerminalResizeOutcome::AlternateScreenResized => "alternate-resize",
    }
}

fn join_numbers(values: &[u16]) -> String {
    values
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn cursor_shape_tag(shape: CursorShape) -> &'static str {
    match shape {
        CursorShape::Block => "block",
        CursorShape::Underline => "underline",
        CursorShape::Bar => "bar",
    }
}
