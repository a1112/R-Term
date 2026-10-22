use std::{collections::HashMap, num::NonZeroU64, sync::Arc};

use rterm_domain::PaneId;
use rterm_runtime::{PaneToken, RuntimeRevision};
use rterm_types::DamageRegion;

use crate::{OverlayPresentation, PaneLayout, PaneRenderRect, PaneSeparator, TabPresentation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameRevision(NonZeroU64);

impl FrameRevision {
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleFactor(u32);

impl ScaleFactor {
    pub const ONE: Self = Self(1_000);

    #[must_use]
    pub const fn from_milli(value: u32) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    #[must_use]
    pub const fn milli(self) -> u32 {
        self.0
    }

    const fn scale(self, logical: u32) -> u32 {
        logical.saturating_mul(self.0).saturating_add(500) / 1_000
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellMetrics {
    pub width: u32,
    pub height: u32,
}

impl CellMetrics {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfacePresentation {
    pub physical_width: u32,
    pub physical_height: u32,
    pub scale: ScaleFactor,
    pub logical_cell: CellMetrics,
    pub titlebar_rows: u16,
    pub tab_bar_rows: u16,
}

impl SurfacePresentation {
    #[must_use]
    pub const fn new(
        physical_width: u32,
        physical_height: u32,
        scale: ScaleFactor,
        logical_cell: CellMetrics,
        titlebar_rows: u16,
        tab_bar_rows: u16,
    ) -> Self {
        Self {
            physical_width,
            physical_height,
            scale,
            logical_cell,
            titlebar_rows,
            tab_bar_rows,
        }
    }

    #[must_use]
    pub const fn physical_cell_width(self) -> u32 {
        self.scale.scale(self.logical_cell.width)
    }

    #[must_use]
    pub const fn physical_cell_height(self) -> u32 {
        self.scale.scale(self.logical_cell.height)
    }

    #[must_use]
    pub const fn reserved_top_rows(self) -> u16 {
        self.titlebar_rows.saturating_add(self.tab_bar_rows)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPresentation {
    pub row: u16,
    pub column: u16,
    pub visible: bool,
}

impl CursorPresentation {
    #[must_use]
    pub const fn new(row: u16, column: u16, visible: bool) -> Self {
        Self {
            row,
            column,
            visible,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionPresentation {
    pub start_row: u16,
    pub start_column: u16,
    pub end_row: u16,
    pub end_column: u16,
}

impl SelectionPresentation {
    #[must_use]
    pub const fn new(start_row: u16, start_column: u16, end_row: u16, end_column: u16) -> Self {
        Self {
            start_row,
            start_column,
            end_row,
            end_column,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarPresentation {
    pub first_visible_row: usize,
    pub total_rows: usize,
    pub viewport_rows: u16,
}

impl ScrollbarPresentation {
    #[must_use]
    pub const fn new(first_visible_row: usize, total_rows: usize, viewport_rows: u16) -> Self {
        Self {
            first_visible_row,
            total_rows,
            viewport_rows,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneFrameCandidate<S> {
    pub pane: PaneToken,
    pub revision: RuntimeRevision,
    pub snapshot: Arc<S>,
    pub damage: Vec<DamageRegion>,
    pub cursor: Option<CursorPresentation>,
    pub selection: Option<SelectionPresentation>,
    pub scrollbar: Option<ScrollbarPresentation>,
}

impl<S> PaneFrameCandidate<S> {
    #[must_use]
    pub fn new(
        pane: PaneToken,
        revision: RuntimeRevision,
        snapshot: Arc<S>,
        damage: Vec<DamageRegion>,
    ) -> Self {
        Self {
            pane,
            revision,
            snapshot,
            damage,
            cursor: None,
            selection: None,
            scrollbar: None,
        }
    }

    #[must_use]
    pub const fn with_cursor(mut self, cursor: CursorPresentation) -> Self {
        self.cursor = Some(cursor);
        self
    }

    #[must_use]
    pub const fn with_selection(mut self, selection: SelectionPresentation) -> Self {
        self.selection = Some(selection);
        self
    }

    #[must_use]
    pub const fn with_scrollbar(mut self, scrollbar: ScrollbarPresentation) -> Self {
        self.scrollbar = Some(scrollbar);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationInput<S> {
    pub revision: FrameRevision,
    pub last_presented_revision: Option<FrameRevision>,
    pub surface: SurfacePresentation,
    pub layout: PaneLayout,
    pub owners: Vec<PaneToken>,
    pub candidates: Vec<PaneFrameCandidate<S>>,
    pub tabs: Vec<TabPresentation>,
    pub overlays: Vec<OverlayPresentation>,
    pub force_full_repaint: bool,
}

impl<S> PresentationInput<S> {
    #[must_use]
    pub fn new(
        revision: FrameRevision,
        last_presented_revision: Option<FrameRevision>,
        surface: SurfacePresentation,
        layout: PaneLayout,
        owners: Vec<PaneToken>,
        candidates: Vec<PaneFrameCandidate<S>>,
    ) -> Self {
        Self {
            revision,
            last_presented_revision,
            surface,
            layout,
            owners,
            candidates,
            tabs: Vec::new(),
            overlays: Vec::new(),
            force_full_repaint: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Full,
    Damage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentedPane<S> {
    pub pane: PaneToken,
    pub revision: RuntimeRevision,
    pub rect: PaneRenderRect,
    pub snapshot: Arc<S>,
    pub cursor: Option<CursorPresentation>,
    pub selection: Option<SelectionPresentation>,
    pub scrollbar: Option<ScrollbarPresentation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationFrame<S> {
    pub revision: FrameRevision,
    pub surface: SurfacePresentation,
    pub tabs: Vec<TabPresentation>,
    pub panes: Vec<PresentedPane<S>>,
    pub separators: Vec<PaneSeparator>,
    pub overlays: Vec<OverlayPresentation>,
    pub damage: Vec<DamageRegion>,
    pub render_mode: RenderMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationError {
    StaleFrame(FrameRevision),
    MissingOwner(PaneId),
    MissingSnapshot(PaneId),
}

impl PresentationError {
    #[must_use]
    pub const fn frame_revision(self) -> Option<FrameRevision> {
        match self {
            Self::StaleFrame(revision) => Some(revision),
            Self::MissingOwner(_) | Self::MissingSnapshot(_) => None,
        }
    }
}

/// Selects one immutable snapshot per current pane owner and builds a frame.
///
/// # Errors
///
/// Returns [`PresentationError`] when the frame revision is stale, a visible
/// pane has no current owner, or its current generation has no snapshot.
pub fn build_presentation<S>(
    input: PresentationInput<S>,
) -> Result<PresentationFrame<S>, PresentationError> {
    if input
        .last_presented_revision
        .is_some_and(|last| input.revision <= last)
    {
        return Err(PresentationError::StaleFrame(input.revision));
    }
    let owners: HashMap<_, _> = input
        .owners
        .iter()
        .copied()
        .map(|token| (token.pane(), token))
        .collect();
    let mut panes = Vec::with_capacity(input.layout.panes.len());
    let mut damage = Vec::new();
    for placement in &input.layout.panes {
        let owner = owners
            .get(&placement.pane)
            .copied()
            .ok_or(PresentationError::MissingOwner(placement.pane))?;
        let candidate = input
            .candidates
            .iter()
            .filter(|candidate| candidate.pane == owner)
            .max_by_key(|candidate| candidate.revision)
            .ok_or(PresentationError::MissingSnapshot(placement.pane))?;
        damage.extend(candidate.damage.iter().filter_map(|region| {
            let width = region
                .width
                .min(placement.rect.columns.saturating_sub(region.x));
            let height = region
                .height
                .min(placement.rect.rows.saturating_sub(region.y));
            (width != 0 && height != 0).then(|| {
                DamageRegion::new(
                    region.x.saturating_add(placement.rect.column),
                    region.y.saturating_add(placement.rect.row),
                    width,
                    height,
                )
            })
        }));
        panes.push(PresentedPane {
            pane: owner,
            revision: candidate.revision,
            rect: placement.rect,
            snapshot: Arc::clone(&candidate.snapshot),
            cursor: candidate.cursor,
            selection: candidate.selection,
            scrollbar: candidate.scrollbar,
        });
    }
    let render_mode = if input.force_full_repaint || damage.is_empty() || !input.overlays.is_empty()
    {
        RenderMode::Full
    } else {
        RenderMode::Damage
    };
    Ok(PresentationFrame {
        revision: input.revision,
        surface: input.surface,
        tabs: input.tabs,
        panes,
        separators: input.layout.separators,
        overlays: input.overlays,
        damage,
        render_mode,
    })
}
