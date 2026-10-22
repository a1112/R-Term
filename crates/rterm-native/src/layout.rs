use std::collections::HashMap;

use rterm_domain::PaneId;
use rterm_types::TerminalSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSplitDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSplitSpec {
    pub source_pane: PaneId,
    pub direction: PaneSplitDirection,
    pub source_size_delta: i16,
}

impl PaneSplitSpec {
    #[must_use]
    pub const fn new(
        source_pane: PaneId,
        direction: PaneSplitDirection,
        source_size_delta: i16,
    ) -> Self {
        Self {
            source_pane,
            direction,
            source_size_delta,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneLayoutPane {
    pub pane: PaneId,
    pub split: Option<PaneSplitSpec>,
}

impl PaneLayoutPane {
    #[must_use]
    pub const fn root(pane: PaneId) -> Self {
        Self { pane, split: None }
    }

    #[must_use]
    pub const fn split(pane: PaneId, split: PaneSplitSpec) -> Self {
        Self {
            pane,
            split: Some(split),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneLayoutSpec {
    pub size: TerminalSize,
    pub row_offset: u16,
    pub panes: Vec<PaneLayoutPane>,
    pub zoomed_pane: Option<PaneId>,
}

impl PaneLayoutSpec {
    #[must_use]
    pub const fn new(
        size: TerminalSize,
        row_offset: u16,
        panes: Vec<PaneLayoutPane>,
        zoomed_pane: Option<PaneId>,
    ) -> Self {
        Self {
            size,
            row_offset,
            panes,
            zoomed_pane,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneRenderRect {
    pub row: u16,
    pub column: u16,
    pub rows: u16,
    pub columns: u16,
}

impl PaneRenderRect {
    #[must_use]
    pub const fn new(row: u16, column: u16, rows: u16, columns: u16) -> Self {
        Self {
            row,
            column,
            rows,
            columns,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanePlacement {
    pub pane: PaneId,
    pub rect: PaneRenderRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSeparator {
    pub rect: PaneRenderRect,
    pub direction: PaneSplitDirection,
    pub source_pane: PaneId,
    pub new_pane: PaneId,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneLayout {
    pub panes: Vec<PanePlacement>,
    pub separators: Vec<PaneSeparator>,
}

#[must_use]
pub fn build_pane_layout(spec: &PaneLayoutSpec) -> PaneLayout {
    let Some(first) = spec.panes.first() else {
        return PaneLayout::default();
    };
    let full = PaneRenderRect::new(spec.row_offset, 0, spec.size.rows, spec.size.columns);
    if let Some(zoomed) = spec.zoomed_pane {
        return PaneLayout {
            panes: vec![PanePlacement {
                pane: zoomed,
                rect: full,
            }],
            separators: Vec::new(),
        };
    }

    let mut rects = HashMap::from([(first.pane, full)]);
    let mut separators = Vec::new();
    for pane in spec.panes.iter().skip(1) {
        let Some(split) = pane.split else {
            continue;
        };
        let Some(source) = rects.get(&split.source_pane).copied() else {
            continue;
        };
        let Some((next_source, new_rect, separator)) =
            split_pane_rect(source, split.source_pane, pane.pane, split)
        else {
            continue;
        };
        rects.insert(split.source_pane, next_source);
        rects.insert(pane.pane, new_rect);
        separators.push(separator);
    }

    PaneLayout {
        panes: spec
            .panes
            .iter()
            .filter_map(|pane| {
                rects.get(&pane.pane).copied().map(|rect| PanePlacement {
                    pane: pane.pane,
                    rect,
                })
            })
            .collect(),
        separators,
    }
}

fn split_pane_rect(
    source: PaneRenderRect,
    source_pane: PaneId,
    new_pane: PaneId,
    split: PaneSplitSpec,
) -> Option<(PaneRenderRect, PaneRenderRect, PaneSeparator)> {
    match split.direction {
        PaneSplitDirection::Left | PaneSplitDirection::Right => {
            split_columns(source, source_pane, new_pane, split)
        }
        PaneSplitDirection::Up | PaneSplitDirection::Down => {
            split_rows(source, source_pane, new_pane, split)
        }
    }
}

fn split_columns(
    source: PaneRenderRect,
    source_pane: PaneId,
    new_pane: PaneId,
    split: PaneSplitSpec,
) -> Option<(PaneRenderRect, PaneRenderRect, PaneSeparator)> {
    if source.columns < 3 || source.rows == 0 {
        return None;
    }
    let source_columns = adjusted_source_size(
        source.columns,
        source.columns.saturating_sub(1) / 2,
        split.source_size_delta,
    );
    let new_columns = source
        .columns
        .saturating_sub(source_columns)
        .saturating_sub(1);
    if source_columns == 0 || new_columns == 0 {
        return None;
    }
    let new_on_left = split.direction == PaneSplitDirection::Left;
    let separator_column = if new_on_left {
        source.column.saturating_add(new_columns)
    } else {
        source.column.saturating_add(source_columns)
    };
    let next_source = PaneRenderRect {
        column: if new_on_left {
            separator_column.saturating_add(1)
        } else {
            source.column
        },
        columns: source_columns,
        ..source
    };
    let new_rect = PaneRenderRect {
        row: source.row,
        column: if new_on_left {
            source.column
        } else {
            separator_column.saturating_add(1)
        },
        rows: source.rows,
        columns: new_columns,
    };
    let separator = PaneSeparator {
        rect: PaneRenderRect::new(source.row, separator_column, source.rows, 1),
        direction: split.direction,
        source_pane,
        new_pane,
    };
    Some((next_source, new_rect, separator))
}

fn split_rows(
    source: PaneRenderRect,
    source_pane: PaneId,
    new_pane: PaneId,
    split: PaneSplitSpec,
) -> Option<(PaneRenderRect, PaneRenderRect, PaneSeparator)> {
    if source.rows < 3 || source.columns == 0 {
        return None;
    }
    let source_rows = adjusted_source_size(
        source.rows,
        source.rows.saturating_sub(1) / 2,
        split.source_size_delta,
    );
    let new_rows = source.rows.saturating_sub(source_rows).saturating_sub(1);
    if source_rows == 0 || new_rows == 0 {
        return None;
    }
    let new_on_top = split.direction == PaneSplitDirection::Up;
    let separator_row = if new_on_top {
        source.row.saturating_add(new_rows)
    } else {
        source.row.saturating_add(source_rows)
    };
    let next_source = PaneRenderRect {
        row: if new_on_top {
            separator_row.saturating_add(1)
        } else {
            source.row
        },
        rows: source_rows,
        ..source
    };
    let new_rect = PaneRenderRect {
        row: if new_on_top {
            source.row
        } else {
            separator_row.saturating_add(1)
        },
        column: source.column,
        rows: new_rows,
        columns: source.columns,
    };
    let separator = PaneSeparator {
        rect: PaneRenderRect::new(separator_row, source.column, 1, source.columns),
        direction: split.direction,
        source_pane,
        new_pane,
    };
    Some((next_source, new_rect, separator))
}

fn adjusted_source_size(total: u16, default_source: u16, delta: i16) -> u16 {
    let max_source = total.saturating_sub(2).max(1);
    let adjusted = i32::from(default_source) + i32::from(delta);
    u16::try_from(adjusted.clamp(1, i32::from(max_source))).unwrap_or(max_source)
}
