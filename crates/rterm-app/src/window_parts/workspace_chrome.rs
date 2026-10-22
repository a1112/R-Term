use rterm_render_core::{RenderCell, RenderStyle};
use rterm_terminal::Color;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// The three controls that belong to the native workspace strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceChromeAction {
    Search,
    Split,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkspaceChromeActionTarget {
    pub(crate) action: WorkspaceChromeAction,
    pub(crate) start_column: u16,
    pub(crate) end_column: u16,
}

pub(crate) struct PaneHeaderSpec<'a> {
    pub(crate) row: u16,
    pub(crate) column: u16,
    pub(crate) columns: u16,
    pub(crate) title: &'a str,
    pub(crate) shell: &'a str,
    pub(crate) cwd: &'a str,
    pub(crate) progress: &'a str,
    pub(crate) active: bool,
    pub(crate) attention: bool,
}

pub(crate) const ACTION_RESERVE_COLUMNS: u16 = 9;
pub(crate) const WORKSPACE_CHROME_ROWS: u16 = 2;

pub(crate) const GRAPHITE_TITLEBAR_COLOR: Color = Color::Rgb(0x0c, 0x12, 0x19);
pub(crate) const GRAPHITE_TERMINAL_BACKGROUND: Color = Color::Rgb(0x14, 0x1b, 0x22);
pub(crate) const GRAPHITE_SEPARATOR: Color = Color::Rgb(0x5d, 0x70, 0x7c);
const GRAPHITE_ACTION_HOVER: Color = Color::Rgb(0x1e, 0x2a, 0x35);
const GRAPHITE_PANE_ACTIVE: Color = Color::Rgb(0x16, 0x22, 0x2d);
const GRAPHITE_PANE_INACTIVE: Color = Color::Rgb(0x10, 0x17, 0x1e);
const GRAPHITE_STATUS: Color = Color::Rgb(0x0a, 0x10, 0x16);
const GRAPHITE_FOREGROUND: Color = Color::Rgb(0xd7, 0xe1, 0xe7);
const GRAPHITE_MUTED: Color = Color::Rgb(0x8c, 0xa0, 0xad);
const GRAPHITE_DIM: Color = Color::Rgb(0x5d, 0x70, 0x7c);
const GRAPHITE_ACCENT: Color = Color::Rgb(0x4f, 0xd1, 0xc5);
const GRAPHITE_WARNING: Color = Color::Rgb(0xf0, 0xb8, 0x5a);

const ACTIONS: [(WorkspaceChromeAction, char); 3] = [
    (WorkspaceChromeAction::Search, '⌕'),
    (WorkspaceChromeAction::Split, '◫'),
    (WorkspaceChromeAction::Settings, '⚙'),
];

pub(crate) fn frame_rows_for_layout(modern_tab_bar: bool, top_tab_bar_visible: bool) -> u16 {
    if modern_tab_bar && top_tab_bar_visible {
        WORKSPACE_CHROME_ROWS
    } else {
        0
    }
}

pub(crate) fn viewport_rows_for_chrome(terminal_rows: u16, chrome_enabled: bool) -> u16 {
    terminal_rows.saturating_add(u16::from(chrome_enabled))
}

pub(crate) fn cursor_opacity_for_workspace_overlay(overlay_active: bool, blink_alpha: u8) -> f32 {
    if overlay_active {
        0.0
    } else {
        f32::from(blink_alpha) / f32::from(u8::MAX)
    }
}

pub(crate) fn action_targets(
    columns: u16,
    right_reserved_columns: u16,
) -> [WorkspaceChromeActionTarget; 3] {
    let end = columns.saturating_sub(right_reserved_columns);
    let start = end.saturating_sub(ACTION_RESERVE_COLUMNS);
    [
        WorkspaceChromeActionTarget {
            action: WorkspaceChromeAction::Search,
            start_column: start,
            end_column: start.saturating_add(3).min(end),
        },
        WorkspaceChromeActionTarget {
            action: WorkspaceChromeAction::Split,
            start_column: start.saturating_add(3).min(end),
            end_column: start.saturating_add(6).min(end),
        },
        WorkspaceChromeActionTarget {
            action: WorkspaceChromeAction::Settings,
            start_column: start.saturating_add(6).min(end),
            end_column: end,
        },
    ]
}

pub(crate) fn action_at_column(
    column: u16,
    columns: u16,
    right_reserved_columns: u16,
) -> Option<WorkspaceChromeAction> {
    action_targets(columns, right_reserved_columns)
        .into_iter()
        .find(|target| column >= target.start_column && column < target.end_column)
        .map(|target| target.action)
}

pub(crate) fn action_cells(
    row: u16,
    columns: u16,
    right_reserved_columns: u16,
    hovered: Option<WorkspaceChromeAction>,
) -> Vec<RenderCell> {
    let targets = action_targets(columns, right_reserved_columns);
    let mut cells = Vec::with_capacity(usize::from(ACTION_RESERVE_COLUMNS));
    for (index, target) in targets.into_iter().enumerate() {
        let hovered = hovered == Some(target.action);
        let background = if hovered {
            GRAPHITE_ACTION_HOVER
        } else {
            GRAPHITE_TITLEBAR_COLOR
        };
        for column in target.start_column..target.end_column {
            cells.push(styled_cell(
                row,
                column,
                ' ',
                GRAPHITE_FOREGROUND,
                background,
                false,
            ));
        }
        if let Some(column) = target
            .start_column
            .checked_add(1)
            .filter(|column| *column < target.end_column)
        {
            cells.push(styled_cell(
                row,
                column,
                ACTIONS[index].1,
                if hovered {
                    GRAPHITE_ACCENT
                } else {
                    GRAPHITE_MUTED
                },
                background,
                false,
            ));
        }
    }
    cells
}

pub(crate) fn pane_header_cells(spec: &PaneHeaderSpec<'_>) -> Vec<RenderCell> {
    if spec.columns == 0 {
        return Vec::new();
    }
    let background = if spec.active {
        GRAPHITE_PANE_ACTIVE
    } else {
        GRAPHITE_PANE_INACTIVE
    };
    let foreground = if spec.active {
        GRAPHITE_FOREGROUND
    } else {
        GRAPHITE_MUTED
    };
    let mut cells = pane_header_background(spec, foreground, background);
    let right = spec.column.saturating_add(spec.columns.saturating_sub(1));
    paint_pane_header_controls(&mut cells, spec, right, background);
    paint_pane_header_label(&mut cells, spec, right, foreground, background);
    cells
}

fn pane_header_background(
    spec: &PaneHeaderSpec<'_>,
    foreground: Color,
    background: Color,
) -> Vec<RenderCell> {
    (0..spec.columns)
        .map(|offset| {
            styled_cell(
                spec.row,
                spec.column.saturating_add(offset),
                ' ',
                foreground,
                background,
                false,
            )
        })
        .collect()
}

fn paint_pane_header_controls(
    cells: &mut [RenderCell],
    spec: &PaneHeaderSpec<'_>,
    right: u16,
    background: Color,
) {
    if spec.columns >= 2 {
        set_cell(
            cells,
            spec.column,
            '▎',
            if spec.attention {
                GRAPHITE_WARNING
            } else {
                GRAPHITE_ACCENT
            },
            background,
            true,
        );
        set_cell(
            cells,
            spec.column.saturating_add(1),
            '●',
            if spec.active {
                GRAPHITE_ACCENT
            } else {
                GRAPHITE_DIM
            },
            background,
            false,
        );
    }
    if spec.columns >= 5 {
        set_cell(
            cells,
            spec.column.saturating_add(3),
            '▣',
            GRAPHITE_MUTED,
            background,
            false,
        );
    }
    if spec.columns >= 4 {
        set_cell(
            cells,
            right.saturating_sub(1),
            '⋯',
            GRAPHITE_MUTED,
            background,
            false,
        );
        set_cell(cells, right, '×', GRAPHITE_DIM, background, false);
    }
}

fn paint_pane_header_label(
    cells: &mut [RenderCell],
    spec: &PaneHeaderSpec<'_>,
    right: u16,
    foreground: Color,
    background: Color,
) {
    let label_end = right.saturating_sub(3);
    if spec.column.saturating_add(5) >= label_end {
        return;
    }
    let label = pane_header_label(spec);
    let available = usize::from(label_end.saturating_sub(spec.column.saturating_add(5)));
    let mut offset = 0usize;
    for grapheme in label.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme).max(1);
        if offset.saturating_add(width) > available {
            break;
        }
        let cell_column = spec
            .column
            .saturating_add(5)
            .saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
        set_grapheme(
            cells,
            cell_column,
            grapheme,
            width,
            foreground,
            background,
            spec.active,
        );
        offset = offset.saturating_add(width);
    }
}

fn pane_header_label(spec: &PaneHeaderSpec<'_>) -> String {
    let title = compact_program_name(spec.title);
    let shell = compact_program_name(spec.shell);
    let mut label = String::new();
    push_label(&mut label, title.as_str());
    if !shell.is_empty() && shell != title {
        label.push_str(" · ");
        label.push_str(&shell);
    }
    if !spec.cwd.is_empty() {
        label.push_str("  ");
        label.push_str(&compact_path(spec.cwd));
    }
    if !spec.progress.is_empty() {
        label.push_str("  ");
        label.push_str(spec.progress);
    }
    label
}

pub(crate) fn status_cells(
    row: u16,
    columns: u16,
    domain: &str,
    shell: &str,
    terminal_columns: u16,
    terminal_rows: u16,
    renderer: &str,
) -> Vec<RenderCell> {
    if columns == 0 {
        return Vec::new();
    }
    let mut cells = (0..columns)
        .map(|column| styled_cell(row, column, ' ', GRAPHITE_MUTED, GRAPHITE_STATUS, false))
        .collect::<Vec<_>>();
    let left = format!("●  {domain}  {shell}");
    for (offset, ch) in left.chars().take(usize::from(columns)).enumerate() {
        set_cell(
            &mut cells,
            u16::try_from(offset).unwrap_or(u16::MAX),
            ch,
            if offset == 0 {
                GRAPHITE_ACCENT
            } else {
                GRAPHITE_MUTED
            },
            GRAPHITE_STATUS,
            offset == 0,
        );
    }
    let right = format!("{terminal_columns} × {terminal_rows}   UTF-8   {renderer}");
    let right_width = u16::try_from(right.chars().count()).unwrap_or(u16::MAX);
    let start = columns.saturating_sub(right_width.saturating_add(1));
    for (offset, ch) in right
        .chars()
        .take(usize::from(columns.saturating_sub(start)))
        .enumerate()
    {
        set_cell(
            &mut cells,
            start.saturating_add(u16::try_from(offset).unwrap_or(u16::MAX)),
            ch,
            if right.ends_with(renderer) {
                GRAPHITE_FOREGROUND
            } else {
                GRAPHITE_MUTED
            },
            GRAPHITE_STATUS,
            false,
        );
    }
    cells
}

fn compact_path(path: &str) -> String {
    let path = path.trim();
    if path.chars().count() <= 28 {
        return path.to_owned();
    }
    let tail = path.rsplit_once('/').map_or(path, |(_, tail)| tail);
    format!("…/{tail}")
}

fn compact_program_name(program: &str) -> String {
    let program = program.trim();
    program
        .rsplit_once('/')
        .map_or(program, |(_, name)| name)
        .to_owned()
}

fn push_label(label: &mut String, value: &str) {
    if value.is_empty() {
        label.push_str("shell");
    } else {
        label.push_str(value);
    }
}

fn set_grapheme(
    cells: &mut [RenderCell],
    column: u16,
    grapheme: &str,
    width: usize,
    fg: Color,
    bg: Color,
    bold: bool,
) {
    let first_scalar = grapheme.chars().next().unwrap_or(' ');
    if let Some(cell) = cells.iter_mut().find(|cell| cell.column == column) {
        let mut leader = styled_cell(cell.row, column, first_scalar, fg, bg, bold);
        leader.text = grapheme.to_owned().into();
        leader.columns = u8::try_from(width.min(usize::from(u8::MAX))).unwrap_or(u8::MAX);
        *cell = leader;
    }
    for continuation_offset in 1..width {
        let continuation_column =
            column.saturating_add(u16::try_from(continuation_offset).unwrap_or(u16::MAX));
        if let Some(cell) = cells
            .iter_mut()
            .find(|cell| cell.column == continuation_column)
        {
            let mut continuation = styled_cell(cell.row, continuation_column, ' ', fg, bg, bold);
            continuation.text = "".into();
            continuation.columns = 0;
            continuation.continuation = true;
            *cell = continuation;
        }
    }
}

fn set_cell(cells: &mut [RenderCell], column: u16, ch: char, fg: Color, bg: Color, bold: bool) {
    if let Some(cell) = cells.iter_mut().find(|cell| cell.column == column) {
        *cell = styled_cell(cell.row, column, ch, fg, bg, bold);
    }
}

fn styled_cell(row: u16, column: u16, ch: char, fg: Color, bg: Color, bold: bool) -> RenderCell {
    RenderCell::new(row, column, ch.to_string()).with_style(RenderStyle {
        foreground: fg,
        background: bg,
        bold,
        ..RenderStyle::default()
    })
}
