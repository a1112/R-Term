//! Renderer-neutral terminal snapshots, geometry, damage, and stable digests.

use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    sync::{Arc, OnceLock},
};

use font8x8::{BASIC_FONTS, UnicodeFonts as _};
use rterm_terminal::{
    Cell, Color, CursorShape, InlineImageFormat, InlineImageFragment, ItermInlineImage, Terminal,
    TerminalGrid, UnderlineStyle, VerticalAlign,
};
pub use rterm_types::DamageRegion;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(clippy::struct_excessive_bools)]
pub struct RenderStyle {
    pub foreground: Color,
    pub background: Color,
    pub underline_color: Color,
    pub underline_style: UnderlineStyle,
    pub bold: bool,
    pub faint: bool,
    pub italic: bool,
    pub blink: bool,
    pub rapid_blink: bool,
    pub underline: bool,
    pub double_underline: bool,
    pub conceal: bool,
    pub strikethrough: bool,
    pub overline: bool,
    pub vertical_align: VerticalAlign,
    pub inverse: bool,
}

impl Default for RenderStyle {
    fn default() -> Self {
        Self {
            foreground: Color::Default,
            background: Color::Default,
            underline_color: Color::Default,
            underline_style: UnderlineStyle::None,
            bold: false,
            faint: false,
            italic: false,
            blink: false,
            rapid_blink: false,
            underline: false,
            double_underline: false,
            conceal: false,
            strikethrough: false,
            overline: false,
            vertical_align: VerticalAlign::Baseline,
            inverse: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderCell {
    pub row: u16,
    pub column: u16,
    /// Complete terminal grapheme. Empty for continuation cells.
    pub text: Arc<str>,
    /// Logical width stored on a grapheme leader.
    pub columns: u8,
    pub continuation: bool,
    /// Temporary first-scalar compatibility field for the bitmap renderer.
    pub ch: char,
    style: Arc<RenderStyle>,
    pub hyperlink: Option<Arc<str>>,
}

impl RenderCell {
    #[must_use]
    pub fn new(row: u16, column: u16, text: impl Into<Arc<str>>) -> Self {
        let text = text.into();
        Self {
            row,
            column,
            columns: u8::from(!text.is_empty()),
            continuation: false,
            ch: text.chars().next().unwrap_or(' '),
            text,
            style: Arc::new(RenderStyle::default()),
            hyperlink: None,
        }
    }

    #[must_use]
    pub fn with_style(mut self, style: RenderStyle) -> Self {
        self.style = Arc::new(style);
        self
    }

    #[must_use]
    pub const fn grapheme(&self) -> &Arc<str> {
        &self.text
    }

    #[must_use]
    pub const fn style(&self) -> &Arc<RenderStyle> {
        &self.style
    }

    #[must_use]
    pub const fn hyperlink(&self) -> Option<&Arc<str>> {
        self.hyperlink.as_ref()
    }
}

impl Deref for RenderCell {
    type Target = RenderStyle;

    fn deref(&self) -> &Self::Target {
        &self.style
    }
}

impl DerefMut for RenderCell {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.style)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderCursor {
    pub row: u16,
    pub column: u16,
    pub shape: CursorShape,
    pub blinking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderCellColorRole {
    Foreground,
    Background,
    Underline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderInlineImage {
    pub row: u16,
    pub column: u16,
    pub name: Option<String>,
    pub kitty_image_id: Option<u32>,
    pub kitty_placement_id: Option<u32>,
    pub kitty_z_index: Option<i32>,
    pub size: Option<usize>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub preserve_aspect_ratio: Option<bool>,
    pub image_format: InlineImageFormat,
    pub pixel_width: Option<u32>,
    pub pixel_height: Option<u32>,
    pub source_x: Option<u32>,
    pub source_y: Option<u32>,
    pub source_width: Option<u32>,
    pub source_height: Option<u32>,
    pub target_x: Option<u32>,
    pub target_y: Option<u32>,
    pub data: Arc<[u8]>,
}

impl RenderInlineImage {
    #[must_use]
    pub const fn payload(&self) -> &Arc<[u8]> {
        &self.data
    }
}

/// Pixel rectangle for one cell-granular inline-image fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderInlineImageFragment {
    pub parent_image_index: usize,
    pub cell_attachment: bool,
    pub row: u16,
    pub column: u16,
    pub source_row: i64,
    pub source_column: i64,
    pub destination_x: u32,
    pub destination_y: u32,
    pub destination_width: u32,
    pub destination_height: u32,
    pub source_x: u32,
    pub source_y: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub sampling_source_x: u32,
    pub sampling_source_y: u32,
    pub sampling_source_width: u32,
    pub sampling_source_height: u32,
    pub source_destination_x: u32,
    pub source_destination_y: u32,
    pub source_destination_width: u32,
    pub source_destination_height: u32,
}

#[doc(hidden)]
pub const KITTY_NON_DEFAULT_BACKGROUND_Z_CUTOFF: i32 = i32::MIN / 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderRowSnapshot {
    row: u16,
    cells: Arc<[RenderCell]>,
}

impl RenderRowSnapshot {
    #[must_use]
    pub const fn row(&self) -> u16 {
        self.row
    }

    #[must_use]
    pub fn cells(&self) -> &[RenderCell] {
        &self.cells
    }
}

#[derive(Debug, Clone)]
pub struct TerminalRenderSnapshot {
    rows: Vec<Arc<RenderRowSnapshot>>,
    compatibility_cells: OnceLock<Arc<[RenderCell]>>,
    #[doc(hidden)]
    pub cursor: Option<RenderCursor>,
    #[doc(hidden)]
    pub cursor_color: Option<Color>,
    #[doc(hidden)]
    pub inline_images: Vec<RenderInlineImage>,
    #[doc(hidden)]
    pub inline_image_fragments: Vec<RenderInlineImageFragment>,
    #[doc(hidden)]
    pub inline_image_parent_origins: Vec<(i64, i64)>,
    /// Render-parent indices for logical placements that have no surviving
    /// cells. Kept private so protocol/image metadata remains stable.
    #[doc(hidden)]
    pub empty_inline_image_attachment_parents: HashSet<usize>,
    #[doc(hidden)]
    pub inline_image_attachment_viewport_offsets: HashMap<(usize, i64, i64), (i64, i64)>,
    #[doc(hidden)]
    pub inline_image_attachment_viewport_clips: HashMap<(usize, i64, i64), AttachmentViewportClip>,
    #[doc(hidden)]
    pub scrollback_offset: usize,
}

impl PartialEq for TerminalRenderSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.rows == other.rows
            && self.cursor == other.cursor
            && self.cursor_color == other.cursor_color
            && self.inline_images == other.inline_images
            && self.inline_image_fragments == other.inline_image_fragments
            && self.inline_image_parent_origins == other.inline_image_parent_origins
            && self.empty_inline_image_attachment_parents
                == other.empty_inline_image_attachment_parents
            && self.inline_image_attachment_viewport_offsets
                == other.inline_image_attachment_viewport_offsets
            && self.inline_image_attachment_viewport_clips
                == other.inline_image_attachment_viewport_clips
            && self.scrollback_offset == other.scrollback_offset
    }
}

impl Eq for TerminalRenderSnapshot {}

pub const DEFAULT_SNAPSHOT_CACHE_BYTE_BUDGET: usize = 8 * 1024 * 1024;
pub const DEFAULT_SNAPSHOT_IMAGE_CACHE_BYTE_BUDGET: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotCacheConfig {
    pub snapshot_budget_bytes: usize,
    pub image_budget_bytes: usize,
}

impl SnapshotCacheConfig {
    #[must_use]
    pub const fn new(snapshot_budget_bytes: usize, image_budget_bytes: usize) -> Self {
        Self {
            snapshot_budget_bytes,
            image_budget_bytes,
        }
    }
}

impl Default for SnapshotCacheConfig {
    fn default() -> Self {
        Self::new(
            DEFAULT_SNAPSHOT_CACHE_BYTE_BUDGET,
            DEFAULT_SNAPSHOT_IMAGE_CACHE_BYTE_BUDGET,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SnapshotCacheMetrics {
    pub snapshot_budget_bytes: usize,
    pub image_budget_bytes: usize,
    pub active_snapshot_bytes: usize,
    pub retained_snapshot_bytes: usize,
    pub retained_image_bytes: usize,
    pub retained_rows: usize,
    pub retained_images: usize,
    pub row_hits: u64,
    pub row_misses: u64,
    pub image_hits: u64,
    pub image_misses: u64,
    pub evictions: u64,
    pub oversize_bypasses: u64,
}

#[derive(Debug)]
pub struct TerminalSnapshotCache {
    config: SnapshotCacheConfig,
    rows: Vec<Arc<RenderRowSnapshot>>,
    image_payloads: Vec<Arc<[u8]>>,
    metrics: SnapshotCacheMetrics,
}

impl TerminalSnapshotCache {
    #[must_use]
    pub fn new(config: SnapshotCacheConfig) -> Self {
        Self {
            config,
            rows: Vec::new(),
            image_payloads: Vec::new(),
            metrics: SnapshotCacheMetrics {
                snapshot_budget_bytes: config.snapshot_budget_bytes,
                image_budget_bytes: config.image_budget_bytes,
                ..SnapshotCacheMetrics::default()
            },
        }
    }

    #[must_use]
    pub fn build(&mut self, terminal: &Terminal) -> TerminalRenderSnapshot {
        let mut snapshot = TerminalRenderSnapshot::from_terminal(terminal);
        self.canonicalize(&mut snapshot);
        self.retain_from(&snapshot);
        snapshot
    }

    #[must_use]
    pub fn update(
        &mut self,
        previous: &TerminalRenderSnapshot,
        terminal: &Terminal,
        damage: &[DamageRegion],
    ) -> TerminalRenderSnapshot {
        let mut snapshot = previous.clone();
        snapshot.update_from_terminal_damage(terminal, damage);
        self.canonicalize(&mut snapshot);
        self.retain_from(&snapshot);
        snapshot
    }

    #[must_use]
    pub const fn metrics(&self) -> SnapshotCacheMetrics {
        self.metrics
    }

    pub fn set_config(&mut self, config: SnapshotCacheConfig) {
        self.config = config;
        self.metrics.snapshot_budget_bytes = config.snapshot_budget_bytes;
        self.metrics.image_budget_bytes = config.image_budget_bytes;
        self.trim_to_budget();
    }

    fn canonicalize(&mut self, snapshot: &mut TerminalRenderSnapshot) {
        for (index, row) in snapshot.rows.iter_mut().enumerate() {
            // Damage updates clone the prior snapshot, so unchanged rows keep
            // their Arc identity. Avoid re-comparing their cells and styles.
            if self
                .rows
                .get(index)
                .is_some_and(|cached| Arc::ptr_eq(cached, row))
            {
                self.metrics.row_hits = self.metrics.row_hits.saturating_add(1);
                continue;
            }
            if let Some(cached) = self
                .rows
                .iter()
                .find(|cached| cached.as_ref() == row.as_ref())
            {
                *row = Arc::clone(cached);
                self.metrics.row_hits = self.metrics.row_hits.saturating_add(1);
            } else {
                self.metrics.row_misses = self.metrics.row_misses.saturating_add(1);
            }
        }
        for image in &mut snapshot.inline_images {
            if let Some(cached) = self
                .image_payloads
                .iter()
                .find(|cached| cached.as_ref() == image.data.as_ref())
            {
                image.data = Arc::clone(cached);
                self.metrics.image_hits = self.metrics.image_hits.saturating_add(1);
            } else {
                self.metrics.image_misses = self.metrics.image_misses.saturating_add(1);
            }
        }
        snapshot.compatibility_cells = OnceLock::new();
    }

    fn retain_from(&mut self, snapshot: &TerminalRenderSnapshot) {
        let old_rows = std::mem::take(&mut self.rows);
        let old_images = std::mem::take(&mut self.image_payloads);
        self.metrics.retained_snapshot_bytes = 0;
        self.metrics.retained_image_bytes = 0;

        for row in &snapshot.rows {
            let bytes = render_row_retained_bytes(row);
            if self
                .metrics
                .retained_snapshot_bytes
                .checked_add(bytes)
                .is_some_and(|total| total <= self.config.snapshot_budget_bytes)
            {
                self.rows.push(Arc::clone(row));
                self.metrics.retained_snapshot_bytes =
                    self.metrics.retained_snapshot_bytes.saturating_add(bytes);
            } else {
                self.metrics.oversize_bypasses = self.metrics.oversize_bypasses.saturating_add(1);
            }
        }

        let mut seen_payloads = HashSet::<*const [u8]>::new();
        for image in &snapshot.inline_images {
            let identity = Arc::as_ptr(&image.data);
            if !seen_payloads.insert(identity) {
                continue;
            }
            let bytes = image.data.len();
            if self
                .metrics
                .retained_image_bytes
                .checked_add(bytes)
                .is_some_and(|total| total <= self.config.image_budget_bytes)
            {
                self.image_payloads.push(Arc::clone(&image.data));
                self.metrics.retained_image_bytes =
                    self.metrics.retained_image_bytes.saturating_add(bytes);
            } else {
                self.metrics.oversize_bypasses = self.metrics.oversize_bypasses.saturating_add(1);
            }
        }

        self.metrics.evictions = self.metrics.evictions.saturating_add(
            old_rows
                .iter()
                .filter(|old| !self.rows.iter().any(|new| Arc::ptr_eq(old, new)))
                .count()
                .saturating_add(
                    old_images
                        .iter()
                        .filter(|old| !self.image_payloads.iter().any(|new| Arc::ptr_eq(old, new)))
                        .count(),
                ) as u64,
        );
        self.metrics.retained_rows = self.rows.len();
        self.metrics.retained_images = self.image_payloads.len();
        self.metrics.active_snapshot_bytes = snapshot_retained_bytes(snapshot);
    }

    fn trim_to_budget(&mut self) {
        while self.metrics.retained_snapshot_bytes > self.config.snapshot_budget_bytes {
            let Some(row) = self.rows.pop() else {
                break;
            };
            self.metrics.retained_snapshot_bytes = self
                .metrics
                .retained_snapshot_bytes
                .saturating_sub(render_row_retained_bytes(&row));
            self.metrics.evictions = self.metrics.evictions.saturating_add(1);
        }
        while self.metrics.retained_image_bytes > self.config.image_budget_bytes {
            let Some(image) = self.image_payloads.pop() else {
                break;
            };
            self.metrics.retained_image_bytes = self
                .metrics
                .retained_image_bytes
                .saturating_sub(image.len());
            self.metrics.evictions = self.metrics.evictions.saturating_add(1);
        }
        self.metrics.retained_rows = self.rows.len();
        self.metrics.retained_images = self.image_payloads.len();
    }
}

impl Default for TerminalSnapshotCache {
    fn default() -> Self {
        Self::new(SnapshotCacheConfig::default())
    }
}

#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct RuntimeInlineImageFragment {
    pub fragment: RenderInlineImageFragment,
    pub row_offset: i64,
    pub column_offset: i64,
    pub clip: Option<AttachmentViewportClip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub struct AttachmentViewportClip {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

impl AttachmentViewportClip {
    #[doc(hidden)]
    #[must_use]
    pub fn translated(self, row: u16, column: u16) -> Self {
        Self {
            left: self.left.saturating_add(i64::from(column)),
            top: self.top.saturating_add(i64::from(row)),
            right: self.right.saturating_add(i64::from(column)),
            bottom: self.bottom.saturating_add(i64::from(row)),
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn intersection(self, other: Self) -> Self {
        Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderGeometry {
    pub target_width: u32,
    pub target_height: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    pub content_x: u32,
    pub content_y: u32,
    pub content_width: u32,
    pub content_height: u32,
    /// Optional one-pixel frame chrome painted outside the content viewport.
    ///
    /// This is deliberately part of the surface geometry rather than the
    /// terminal snapshot so the physical frame never changes PTY dimensions,
    /// cell placement, or hit testing.
    pub frame_border_color: Option<[u8; 4]>,
    /// Optional horizontal chrome separator in target-surface pixel space.
    pub frame_separator: Option<(u32, [u8; 4])>,
}

impl RenderGeometry {
    #[must_use]
    pub const fn new(
        target_width: u32,
        target_height: u32,
        cell_width: u32,
        cell_height: u32,
    ) -> Self {
        Self {
            target_width,
            target_height,
            cell_width,
            cell_height,
            content_x: 0,
            content_y: 0,
            content_width: target_width,
            content_height: target_height,
            frame_border_color: None,
            frame_separator: None,
        }
    }

    #[must_use]
    pub fn with_content_rect(mut self, x: u32, y: u32, width: u32, height: u32) -> Self {
        self.content_x = x.min(self.target_width);
        self.content_y = y.min(self.target_height);
        self.content_width = width.min(self.target_width.saturating_sub(self.content_x));
        self.content_height = height.min(self.target_height.saturating_sub(self.content_y));
        self
    }

    #[must_use]
    pub fn with_frame_border(mut self, color: [u8; 4]) -> Self {
        self.frame_border_color = Some(color);
        self
    }

    #[must_use]
    pub fn with_frame_separator(mut self, y: u32, color: [u8; 4]) -> Self {
        self.frame_separator = Some((y, color));
        self
    }
}

pub const SCROLLBAR_TRACK_COLOR: [u8; 4] = [0x10, 0x18, 0x27, 0xff];
pub const SCROLLBAR_THUMB_COLOR: [u8; 4] = [0x47, 0x55, 0x69, 0xff];
pub const SCROLLBAR_WIDTH: u32 = 4;
#[doc(hidden)]
pub const DEFAULT_DPI: u32 = 96;
pub type RenderIndexedPalette = [Option<[u8; 4]>; 256];

pub type TerminalContentDigest = [u8; 32];

#[must_use]
pub fn terminal_bytes_content_digest(bytes: &[u8]) -> TerminalContentDigest {
    Sha256::digest(bytes).into()
}

/// Hashes the exact ordered terminal render plan, including cell placement and
/// grapheme span, with SHA-256.
#[must_use]
pub fn terminal_snapshot_content_digest(
    snapshot: &TerminalRenderSnapshot,
) -> TerminalContentDigest {
    let mut digest = Sha256::new();
    digest.update(b"rterm-terminal-render-plan-v1\0");
    digest.update(
        u64::try_from(snapshot.iter_cells().count())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for cell in snapshot.iter_cells() {
        digest.update(cell.row.to_le_bytes());
        digest.update(cell.column.to_le_bytes());
        digest.update([cell.columns, u8::from(cell.continuation)]);
        digest.update(
            u64::try_from(cell.text.len())
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
        digest.update(cell.text.as_bytes());
    }
    digest.finalize().into()
}

impl TerminalRenderSnapshot {
    #[doc(hidden)]
    #[must_use]
    pub fn from_inline_image_projection(projection: TerminalInlineImageProjection) -> Self {
        let (
            mut inline_images,
            inline_image_fragments,
            inline_image_parent_origins,
            empty_inline_image_attachment_parents,
            inline_image_attachment_viewport_offsets,
        ) = projection;
        intern_render_image_payloads(&mut inline_images);
        Self {
            rows: Vec::new(),
            compatibility_cells: OnceLock::new(),
            cursor: None,
            cursor_color: None,
            inline_images,
            inline_image_fragments,
            inline_image_parent_origins,
            empty_inline_image_attachment_parents,
            inline_image_attachment_viewport_offsets,
            inline_image_attachment_viewport_clips: HashMap::new(),
            scrollback_offset: 0,
        }
    }

    #[must_use]
    pub fn from_grid(grid: &TerminalGrid) -> Self {
        Self::from_grid_with_cursor(grid, None)
    }

    #[must_use]
    pub fn from_terminal(terminal: &Terminal) -> Self {
        Self::from_terminal_viewport(terminal, 0)
    }

    #[must_use]
    pub fn from_terminal_viewport(terminal: &Terminal, scrollback_offset: usize) -> Self {
        let grid = terminal.grid();
        let size = grid.size();
        let scrollback = terminal.scrollback();
        let offset = scrollback_offset.min(scrollback.len());
        let first_source_row = scrollback.len().saturating_sub(offset);
        let cursor = render_cursor_from_terminal(terminal, offset);

        let mut cells = Vec::new();
        for viewport_row in 0..size.rows {
            let source_row = first_source_row + usize::from(viewport_row);
            if let Some(line) = scrollback.get(source_row) {
                append_render_cells(
                    &mut cells,
                    viewport_row,
                    line.cells(),
                    size.columns,
                    terminal.screen_reverse_video(),
                );
            } else {
                let grid_row = source_row - scrollback.len();
                append_grid_row(
                    &mut cells,
                    grid,
                    viewport_row,
                    grid_row,
                    size.columns,
                    terminal.screen_reverse_video(),
                );
            }
        }

        intern_render_cells(&mut cells);

        let (
            mut inline_images,
            inline_image_fragments,
            inline_image_parent_origins,
            empty_inline_image_attachment_parents,
            inline_image_attachment_viewport_offsets,
        ) = render_inline_images_from_terminal(terminal, first_source_row, size.rows, size.columns);
        intern_render_image_payloads(&mut inline_images);

        Self {
            rows: rows_from_cells(cells, size.rows),
            compatibility_cells: OnceLock::new(),
            cursor,
            cursor_color: None,
            inline_images,
            inline_image_fragments,
            inline_image_parent_origins,
            empty_inline_image_attachment_parents,
            inline_image_attachment_viewport_offsets,
            inline_image_attachment_viewport_clips: std::collections::HashMap::new(),
            scrollback_offset: offset,
        }
    }

    fn from_grid_with_cursor(grid: &TerminalGrid, cursor: Option<RenderCursor>) -> Self {
        let size = grid.size();
        let mut cells = Vec::new();

        for row in 0..size.rows {
            for column in 0..size.columns {
                let Some(cell) = grid.get(row, column) else {
                    continue;
                };

                if !cell_has_renderable_content(cell) {
                    continue;
                }

                cells.push(render_cell_from_terminal(row, column, cell, false));
            }
        }

        intern_render_cells(&mut cells);

        Self {
            rows: rows_from_cells(cells, size.rows),
            compatibility_cells: OnceLock::new(),
            cursor,
            cursor_color: None,
            inline_images: Vec::new(),
            inline_image_fragments: Vec::new(),
            inline_image_parent_origins: Vec::new(),
            empty_inline_image_attachment_parents: HashSet::new(),
            inline_image_attachment_viewport_offsets: HashMap::new(),
            inline_image_attachment_viewport_clips: std::collections::HashMap::new(),
            scrollback_offset: 0,
        }
    }

    #[must_use]
    pub fn cells(&self) -> &[RenderCell] {
        self.compatibility_cells
            .get_or_init(|| self.iter_cells().cloned().collect::<Vec<_>>().into())
            .as_ref()
    }

    /// Returns the snapshot's v1 logical explicit bytes.
    ///
    /// The formula counts the snapshot value, explicit `Vec` capacities,
    /// logical hash entries, compatibility cells, image descriptors, and each
    /// unique owned payload. It deliberately excludes allocator metadata,
    /// `Arc` control blocks, and hash-table control/bucket overhead. This is an
    /// attribution formula, not a heap allocator measurement.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectOwnedLogicalBytesV1Error`] if any size calculation
    /// overflows instead of silently clamping the attributed byte count.
    pub fn project_owned_logical_bytes_v1(&self) -> Result<usize, ProjectOwnedLogicalBytesV1Error> {
        let mut retained = std::mem::size_of::<Self>();
        checked_add_capacity::<Arc<RenderRowSnapshot>>(&mut retained, self.rows.capacity())?;
        checked_add_capacity::<RenderInlineImage>(&mut retained, self.inline_images.capacity())?;
        checked_add_capacity::<RenderInlineImageFragment>(
            &mut retained,
            self.inline_image_fragments.capacity(),
        )?;
        checked_add_capacity::<(i64, i64)>(
            &mut retained,
            self.inline_image_parent_origins.capacity(),
        )?;
        checked_add_len::<usize>(
            &mut retained,
            self.empty_inline_image_attachment_parents.len(),
        )?;
        checked_add_map_len::<(usize, i64, i64), (i64, i64)>(
            &mut retained,
            self.inline_image_attachment_viewport_offsets.len(),
        )?;
        checked_add_map_len::<(usize, i64, i64), AttachmentViewportClip>(
            &mut retained,
            self.inline_image_attachment_viewport_clips.len(),
        )?;

        let mut rows = HashSet::<usize>::new();
        let mut text_payloads = HashSet::<(usize, usize)>::new();
        let mut styles = HashSet::<usize>::new();
        for row in &self.rows {
            if rows.insert(Arc::as_ptr(row) as usize) {
                checked_add(&mut retained, std::mem::size_of::<RenderRowSnapshot>())?;
                checked_add_len::<RenderCell>(&mut retained, row.cells.len())?;
            }
            checked_add(
                &mut retained,
                render_cells_owned_payload_bytes_checked(
                    &row.cells,
                    &mut text_payloads,
                    &mut styles,
                )?,
            )?;
        }
        if let Some(cells) = self.compatibility_cells.get() {
            checked_add_len::<RenderCell>(&mut retained, cells.len())?;
            checked_add(
                &mut retained,
                render_cells_owned_payload_bytes_checked(cells, &mut text_payloads, &mut styles)?,
            )?;
        }

        let mut image_payloads = HashSet::<(usize, usize)>::new();
        for image in &self.inline_images {
            checked_add(
                &mut retained,
                image.name.as_ref().map_or(0, String::capacity),
            )?;
            checked_add(
                &mut retained,
                image.width.as_ref().map_or(0, String::capacity),
            )?;
            checked_add(
                &mut retained,
                image.height.as_ref().map_or(0, String::capacity),
            )?;
            let payload = (image.data.as_ptr() as usize, image.data.len());
            if image_payloads.insert(payload) {
                checked_add(&mut retained, image.data.len())?;
            }
        }
        Ok(retained)
    }

    #[must_use]
    pub fn rows(&self) -> &[Arc<RenderRowSnapshot>] {
        &self.rows
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = &RenderCell> {
        self.rows.iter().flat_map(|row| row.cells.iter())
    }

    fn flattened_cells(&self) -> Vec<RenderCell> {
        self.iter_cells().cloned().collect()
    }

    fn replace_cells(&mut self, mut cells: Vec<RenderCell>, row_count: u16) {
        intern_render_cells(&mut cells);
        self.rows = rows_from_cells(cells, row_count);
        self.compatibility_cells = OnceLock::new();
    }

    /// Reconstructs one immutable row as complete graphemes with terminal-owned spans.
    ///
    /// Default blank cells are represented explicitly so the result is contiguous
    /// and can be passed directly to the shaping engine.
    #[must_use]
    pub fn terminal_clusters_for_row(
        &self,
        row: u16,
        columns: u16,
    ) -> Vec<rterm_fonts::TerminalCluster> {
        let mut by_column = self
            .iter_cells()
            .filter(|cell| cell.row == row)
            .map(|cell| (cell.column, cell))
            .collect::<HashMap<_, _>>();
        let mut clusters = Vec::new();
        let mut column = 0_u16;
        while column < columns {
            match by_column.remove(&column) {
                Some(cell) if cell.continuation => {
                    clusters.push(rterm_fonts::TerminalCluster::new(
                        " ",
                        usize::from(column)..usize::from(column.saturating_add(1)),
                    ));
                    column = column.saturating_add(1);
                }
                Some(cell) => {
                    let width = u16::from(cell.columns.max(1))
                        .min(columns.saturating_sub(column))
                        .max(1);
                    clusters.push(rterm_fonts::TerminalCluster::new(
                        cell.text.as_ref().to_owned(),
                        usize::from(column)..usize::from(column.saturating_add(width)),
                    ));
                    column = column.saturating_add(width);
                }
                None => {
                    clusters.push(rterm_fonts::TerminalCluster::new(
                        " ",
                        usize::from(column)..usize::from(column.saturating_add(1)),
                    ));
                    column = column.saturating_add(1);
                }
            }
        }
        clusters
    }

    #[must_use]
    pub fn missing_glyphs(&self) -> Vec<char> {
        let mut missing = Vec::new();
        for cell in self.iter_cells() {
            // The native GPU path supplies the modern terminal UI symbols
            // from its configured fallback catalog.  Keep the compatibility
            // warning focused on characters that neither the legacy 8x8
            // renderer nor the guaranteed UI fallback set can represent.
            if BASIC_FONTS.get(cell.ch).is_none()
                && !modern_ui_fallback_glyph(cell.ch)
                && !missing.contains(&cell.ch)
            {
                missing.push(cell.ch);
            }
        }
        missing
    }

    #[must_use]
    pub fn inline_images(&self) -> &[RenderInlineImage] {
        &self.inline_images
    }

    #[must_use]
    pub fn inline_image_fragments(&self) -> &[RenderInlineImageFragment] {
        &self.inline_image_fragments
    }

    #[must_use]
    pub const fn scrollback_offset(&self) -> usize {
        self.scrollback_offset
    }

    #[must_use]
    pub const fn cursor(&self) -> Option<RenderCursor> {
        self.cursor
    }

    #[must_use]
    pub const fn cursor_color(&self) -> Option<Color> {
        self.cursor_color
    }

    #[must_use]
    pub const fn with_cursor_color(mut self, cursor_color: Option<Color>) -> Self {
        self.cursor_color = cursor_color;
        self
    }

    pub fn set_cursor_color(&mut self, cursor_color: Option<Color>) {
        self.cursor_color = cursor_color;
    }

    #[must_use]
    pub fn with_row_offset(mut self, offset: u16) -> Self {
        if offset == 0 {
            return self;
        }

        let mut cells = self.flattened_cells();
        for cell in &mut cells {
            cell.row = cell.row.saturating_add(offset);
        }
        let row_count = u16::try_from(self.rows.len())
            .unwrap_or(u16::MAX)
            .saturating_add(offset);
        self.replace_cells(cells, row_count);
        for image in &mut self.inline_images {
            image.row = image.row.saturating_add(offset);
        }
        for fragment in &mut self.inline_image_fragments {
            fragment.row = fragment.row.saturating_add(offset);
            if !fragment.cell_attachment {
                fragment.source_row = fragment.source_row.saturating_add(i64::from(offset));
            }
        }
        for (_, row) in &mut self.inline_image_parent_origins {
            *row = row.saturating_add(i64::from(offset));
        }
        for clip in self.inline_image_attachment_viewport_clips.values_mut() {
            clip.top = clip.top.saturating_add(i64::from(offset));
            clip.bottom = clip.bottom.saturating_add(i64::from(offset));
        }
        if let Some(cursor) = self.cursor.as_mut() {
            cursor.row = cursor.row.saturating_add(offset);
        }

        self
    }

    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "viewport projection atomically remaps cells, images, fragments, and attachment metadata"
    )]
    pub fn with_viewport(
        mut self,
        origin_row: u16,
        origin_column: u16,
        rows: u16,
        columns: u16,
    ) -> Self {
        let mut cells = self.flattened_cells();
        cells.retain(|cell| cell.row < rows && cell.column < columns);
        let old_inline_images = std::mem::take(&mut self.inline_images);
        let old_inline_image_parent_origins = std::mem::take(&mut self.inline_image_parent_origins);
        let old_inline_image_fragments = std::mem::take(&mut self.inline_image_fragments);
        let old_empty_attachment_parents =
            std::mem::take(&mut self.empty_inline_image_attachment_parents);
        let old_attachment_offsets =
            std::mem::take(&mut self.inline_image_attachment_viewport_offsets);
        let old_attachment_clips = std::mem::take(&mut self.inline_image_attachment_viewport_clips);
        let mut attachment_parents = HashSet::new();
        let mut visible_attachment_parents = HashSet::new();
        let mut visible_attachment_keys = HashSet::new();
        let mut visible_fragments = Vec::new();
        for fragment in old_inline_image_fragments {
            let visible = if fragment.cell_attachment {
                attachment_parents.insert(fragment.parent_image_index);
                let key = (
                    fragment.parent_image_index,
                    fragment.source_row,
                    fragment.source_column,
                );
                let (row_offset, column_offset) =
                    old_attachment_offsets.get(&key).copied().unwrap_or((0, 0));
                let row = i64::from(fragment.row).saturating_add(row_offset);
                let column = i64::from(fragment.column).saturating_add(column_offset);
                let visible =
                    row >= 0 && row < i64::from(rows) && column >= 0 && column < i64::from(columns);
                if visible {
                    visible_attachment_parents.insert(fragment.parent_image_index);
                    visible_attachment_keys.insert(key);
                }
                visible
            } else {
                fragment.row < rows && fragment.column < columns
            };
            if visible {
                visible_fragments.push(fragment);
            }
        }
        let mut parent_indices = vec![None; old_inline_images.len()];
        for (old_index, image) in old_inline_images.into_iter().enumerate() {
            let keep = if attachment_parents.contains(&old_index) {
                visible_attachment_parents.contains(&old_index)
            } else {
                image.row < rows && image.column < columns
            };
            if keep {
                let new_index = self.inline_images.len();
                parent_indices[old_index] = Some(new_index);
                if old_empty_attachment_parents.contains(&old_index) {
                    self.empty_inline_image_attachment_parents.insert(new_index);
                }
                self.inline_images.push(image);
                self.inline_image_parent_origins.push(
                    old_inline_image_parent_origins
                        .get(old_index)
                        .copied()
                        .unwrap_or((0, 0)),
                );
            }
        }
        self.inline_image_fragments = visible_fragments
            .into_iter()
            .filter_map(|mut fragment| {
                let Some(Some(parent_image_index)) =
                    parent_indices.get(fragment.parent_image_index)
                else {
                    return None;
                };
                fragment.parent_image_index = *parent_image_index;
                Some(fragment)
            })
            .collect();
        for cell in &mut cells {
            cell.row = cell.row.saturating_add(origin_row);
            cell.column = cell.column.saturating_add(origin_column);
        }
        self.replace_cells(cells, origin_row.saturating_add(rows));
        for image in &mut self.inline_images {
            image.row = image.row.saturating_add(origin_row);
            image.column = image.column.saturating_add(origin_column);
        }
        for fragment in &mut self.inline_image_fragments {
            fragment.row = fragment.row.saturating_add(origin_row);
            fragment.column = fragment.column.saturating_add(origin_column);
            if !fragment.cell_attachment {
                fragment.source_row = fragment.source_row.saturating_add(i64::from(origin_row));
                fragment.source_column = fragment
                    .source_column
                    .saturating_add(i64::from(origin_column));
            }
        }
        self.inline_image_attachment_viewport_offsets = old_attachment_offsets
            .into_iter()
            .filter_map(|((old_parent, source_row, source_column), offset)| {
                visible_attachment_keys
                    .contains(&(old_parent, source_row, source_column))
                    .then_some(())?;
                let new_parent = *parent_indices.get(old_parent)?.as_ref()?;
                Some(((new_parent, source_row, source_column), offset))
            })
            .collect();
        let viewport_clip = AttachmentViewportClip {
            left: i64::from(origin_column),
            top: i64::from(origin_row),
            right: i64::from(origin_column).saturating_add(i64::from(columns)),
            bottom: i64::from(origin_row).saturating_add(i64::from(rows)),
        };
        self.inline_image_attachment_viewport_clips = visible_attachment_keys
            .into_iter()
            .filter_map(|(old_parent, source_row, source_column)| {
                let new_parent = *parent_indices.get(old_parent)?.as_ref()?;
                let clip = old_attachment_clips
                    .get(&(old_parent, source_row, source_column))
                    .copied()
                    .map_or(viewport_clip, |clip| {
                        clip.translated(origin_row, origin_column)
                    })
                    .intersection(viewport_clip);
                Some(((new_parent, source_row, source_column), clip))
            })
            .collect();
        for (column, row) in &mut self.inline_image_parent_origins {
            *column = column.saturating_add(i64::from(origin_column));
            *row = row.saturating_add(i64::from(origin_row));
        }

        self.cursor = self.cursor.and_then(|mut cursor| {
            if cursor.row >= rows || cursor.column >= columns {
                return None;
            }
            cursor.row = cursor.row.saturating_add(origin_row);
            cursor.column = cursor.column.saturating_add(origin_column);
            Some(cursor)
        });

        self
    }

    #[must_use]
    pub fn with_overlay_cells(mut self, cells: impl IntoIterator<Item = RenderCell>) -> Self {
        let cells = cells.into_iter().collect::<Vec<_>>();
        if cells.is_empty() {
            return self;
        }
        let mut by_row = HashMap::<u16, Vec<RenderCell>>::new();
        for cell in cells {
            by_row.entry(cell.row).or_default().push(cell);
        }
        let required_rows = by_row
            .keys()
            .copied()
            .max()
            .map_or(0, |row| row.saturating_add(1));
        while self.rows.len() < usize::from(required_rows) {
            let row = u16::try_from(self.rows.len()).unwrap_or(u16::MAX);
            self.rows.push(empty_render_row(row));
        }
        for (row, overlay) in by_row {
            let positions = overlay
                .iter()
                .map(|cell| cell.column)
                .collect::<HashSet<_>>();
            let mut row_cells = self.rows[usize::from(row)].cells.to_vec();
            row_cells.retain(|cell| !positions.contains(&cell.column));
            row_cells.extend(overlay);
            row_cells.sort_by_key(|cell| cell.column);
            intern_render_cells(&mut row_cells);
            self.rows[usize::from(row)] = Arc::new(RenderRowSnapshot {
                row,
                cells: row_cells.into(),
            });
        }
        self.compatibility_cells = OnceLock::new();
        self
    }

    #[must_use]
    pub fn with_overlay_snapshot(mut self, snapshot: Self) -> Self {
        let overlay_cells = snapshot.iter_cells().cloned().collect::<Vec<_>>();
        let row_count = self
            .rows
            .len()
            .max(snapshot.rows.len())
            .try_into()
            .unwrap_or(u16::MAX);
        let mut cells = self.flattened_cells();
        cells.extend(overlay_cells);
        cells.sort_by_key(|cell| (cell.row, cell.column));
        self.replace_cells(cells, row_count);
        let parent_index_offset = self.inline_images.len();
        self.inline_images.extend(snapshot.inline_images);
        self.empty_inline_image_attachment_parents.extend(
            snapshot
                .empty_inline_image_attachment_parents
                .into_iter()
                .map(|parent| parent.saturating_add(parent_index_offset)),
        );
        self.inline_image_parent_origins
            .extend(snapshot.inline_image_parent_origins);
        self.inline_image_fragments
            .extend(
                snapshot
                    .inline_image_fragments
                    .into_iter()
                    .map(|mut fragment| {
                        fragment.parent_image_index = fragment
                            .parent_image_index
                            .saturating_add(parent_index_offset);
                        fragment
                    }),
            );
        self.inline_image_attachment_viewport_offsets.extend(
            snapshot
                .inline_image_attachment_viewport_offsets
                .into_iter()
                .map(|((parent, source_row, source_column), offset)| {
                    (
                        (
                            parent.saturating_add(parent_index_offset),
                            source_row,
                            source_column,
                        ),
                        offset,
                    )
                }),
        );
        self.inline_image_attachment_viewport_clips.extend(
            snapshot
                .inline_image_attachment_viewport_clips
                .into_iter()
                .map(|((parent, source_row, source_column), clip)| {
                    (
                        (
                            parent.saturating_add(parent_index_offset),
                            source_row,
                            source_column,
                        ),
                        clip,
                    )
                }),
        );
        self.sort_inline_images_and_remap_fragments();
        self
    }

    #[must_use]
    pub fn with_cells_mapped(mut self, mut map_cell: impl FnMut(RenderCell) -> RenderCell) -> Self {
        let row_count = self.rows.len().try_into().unwrap_or(u16::MAX);
        let cells = self
            .flattened_cells()
            .into_iter()
            .map(&mut map_cell)
            .collect();
        self.replace_cells(cells, row_count);
        self
    }

    #[must_use]
    pub fn with_cell_colors_mapped(
        mut self,
        mut map_color: impl FnMut(RenderCellColorRole, Color) -> Color,
    ) -> Self {
        let row_count = self.rows.len().try_into().unwrap_or(u16::MAX);
        let mut cells = self.flattened_cells();
        for cell in &mut cells {
            cell.foreground = map_color(RenderCellColorRole::Foreground, cell.foreground);
            cell.background = map_color(RenderCellColorRole::Background, cell.background);
            cell.underline_color = map_color(RenderCellColorRole::Underline, cell.underline_color);
        }
        self.replace_cells(cells, row_count);
        self
    }

    pub fn update_from_terminal_damage(&mut self, terminal: &Terminal, damage: &[DamageRegion]) {
        let grid = terminal.grid();
        let size = grid.size();
        while self.rows.len() < usize::from(size.rows) {
            let row = u16::try_from(self.rows.len()).unwrap_or(u16::MAX);
            self.rows.push(empty_render_row(row));
        }
        for region in damage.iter().copied().filter(|region| !region.is_empty()) {
            let start_row = region.y.min(size.rows);
            let end_row = region.y.saturating_add(region.height).min(size.rows);
            let start_column = region.x.min(size.columns);
            let end_column = region.x.saturating_add(region.width).min(size.columns);
            if start_row >= end_row || start_column >= end_column {
                continue;
            }

            for row in start_row..end_row {
                let mut row_cells = Vec::new();
                for column in 0..size.columns {
                    let Some(cell) = grid.get(row, column) else {
                        continue;
                    };
                    append_render_cell(
                        &mut row_cells,
                        row,
                        column,
                        cell,
                        terminal.screen_reverse_video(),
                    );
                }
                intern_render_cells(&mut row_cells);
                self.rows[usize::from(row)] = Arc::new(RenderRowSnapshot {
                    row,
                    cells: row_cells.into(),
                });
            }
        }

        self.rows.truncate(usize::from(size.rows));
        self.compatibility_cells = OnceLock::new();
        let previous_payloads = self
            .inline_images
            .iter()
            .map(|image| Arc::clone(&image.data))
            .collect::<Vec<_>>();
        (
            self.inline_images,
            self.inline_image_fragments,
            self.inline_image_parent_origins,
            self.empty_inline_image_attachment_parents,
            self.inline_image_attachment_viewport_offsets,
        ) = render_inline_images_from_terminal(terminal, 0, size.rows, size.columns);
        for image in &mut self.inline_images {
            if let Some(payload) = previous_payloads
                .iter()
                .find(|payload| payload.as_ref() == image.data.as_ref())
            {
                image.data = Arc::clone(payload);
            }
        }
        intern_render_image_payloads(&mut self.inline_images);
        self.inline_image_attachment_viewport_clips.clear();
        self.cursor = render_cursor_from_terminal(terminal, 0);
    }

    pub fn update_cursor_from_terminal(&mut self, terminal: &Terminal, scrollback_offset: usize) {
        self.cursor = render_cursor_from_terminal(terminal, scrollback_offset);
    }

    #[must_use]
    pub fn with_inverse_overlay(mut self, mut selected: impl FnMut(u16, u16) -> bool) -> Self {
        let row_count = self.rows.len().try_into().unwrap_or(u16::MAX);
        let mut cells = self.flattened_cells();
        for cell in &mut cells {
            if selected(cell.row, cell.column) {
                cell.inverse = !cell.inverse;
            }
        }
        self.replace_cells(cells, row_count);

        self
    }

    #[must_use]
    pub fn with_selection_colors_overlay(
        mut self,
        mut selected: impl FnMut(u16, u16) -> bool,
        selection_foreground: Option<Option<Color>>,
        selection_background: Option<Color>,
    ) -> Self {
        if selection_foreground.is_none() && selection_background.is_none() {
            return self.with_inverse_overlay(selected);
        }

        let row_count = self.rows.len().try_into().unwrap_or(u16::MAX);
        let mut cells = self.flattened_cells();
        for cell in &mut cells {
            if selected(cell.row, cell.column) {
                let inverse_foreground = cell.background;
                let inverse_background = cell.foreground;
                cell.foreground = match selection_foreground {
                    Some(Some(color)) => color,
                    Some(None) => cell.foreground,
                    None => inverse_foreground,
                };
                cell.background = selection_background.map_or(inverse_background, |background| {
                    blend_selection_background(background, cell.background)
                });
                cell.inverse = false;
            }
        }
        self.replace_cells(cells, row_count);

        self
    }

    fn sort_inline_images_and_remap_fragments(&mut self) {
        let old_inline_image_parent_origins = std::mem::take(&mut self.inline_image_parent_origins);
        let old_empty_attachment_parents =
            std::mem::take(&mut self.empty_inline_image_attachment_parents);
        let mut indexed_images = std::mem::take(&mut self.inline_images)
            .into_iter()
            .enumerate()
            .map(|(index, image)| {
                (
                    index,
                    image,
                    old_inline_image_parent_origins
                        .get(index)
                        .copied()
                        .unwrap_or((0, 0)),
                )
            })
            .collect::<Vec<_>>();
        indexed_images.sort_by_key(|(_, image, _)| (image.row, image.column));
        let mut parent_indices = vec![0; indexed_images.len()];
        let mut inline_images = Vec::with_capacity(indexed_images.len());
        let mut inline_image_parent_origins = Vec::with_capacity(indexed_images.len());
        for (new_index, (old_index, image, origin)) in indexed_images.into_iter().enumerate() {
            parent_indices[old_index] = new_index;
            inline_images.push(image);
            inline_image_parent_origins.push(origin);
        }
        self.inline_images = inline_images;
        self.inline_image_parent_origins = inline_image_parent_origins;
        self.empty_inline_image_attachment_parents = old_empty_attachment_parents
            .into_iter()
            .filter_map(|old_parent| parent_indices.get(old_parent).copied())
            .collect();
        self.inline_image_fragments.retain_mut(|fragment| {
            let Some(parent_image_index) = parent_indices.get(fragment.parent_image_index) else {
                return false;
            };
            fragment.parent_image_index = *parent_image_index;
            true
        });
        let old_attachment_offsets =
            std::mem::take(&mut self.inline_image_attachment_viewport_offsets);
        self.inline_image_attachment_viewport_offsets = old_attachment_offsets
            .into_iter()
            .filter_map(|((old_parent, source_row, source_column), offset)| {
                let new_parent = *parent_indices.get(old_parent)?;
                Some(((new_parent, source_row, source_column), offset))
            })
            .collect();
        let old_attachment_clips = std::mem::take(&mut self.inline_image_attachment_viewport_clips);
        self.inline_image_attachment_viewport_clips = old_attachment_clips
            .into_iter()
            .filter_map(|((old_parent, source_row, source_column), clip)| {
                let new_parent = *parent_indices.get(old_parent)?;
                Some(((new_parent, source_row, source_column), clip))
            })
            .collect();
    }
}

/// Glyphs used by the modern tab bar and native window chrome.
///
/// These are intentionally kept small and explicit: the GPU font catalog
/// carries the actual outlines, while this predicate prevents the legacy
/// compatibility diagnostic from reporting known fallback-backed UI glyphs
/// as missing before the GPU pass has shaped them.
fn modern_ui_fallback_glyph(character: char) -> bool {
    matches!(character, '×' | '▾' | '—' | '□' | '●')
}

fn blend_selection_background(selection_background: Color, cell_background: Color) -> Color {
    let Color::Rgba(red, green, blue, alpha) = selection_background else {
        return selection_background;
    };
    match cell_background {
        Color::Rgb(base_red, base_green, base_blue)
        | Color::Rgba(base_red, base_green, base_blue, _) => {
            let alpha = u16::from(alpha);
            let inverse_alpha = u16::from(u8::MAX).saturating_sub(alpha);
            Color::Rgb(
                blend_channel(red, base_red, alpha, inverse_alpha),
                blend_channel(green, base_green, alpha, inverse_alpha),
                blend_channel(blue, base_blue, alpha, inverse_alpha),
            )
        }
        Color::Default | Color::Indexed(_) => selection_background,
    }
}

#[doc(hidden)]
pub type AttachmentViewportOffset = ((usize, i64, i64), (i64, i64));
#[doc(hidden)]
pub type TerminalInlineImageProjection = (
    Vec<RenderInlineImage>,
    Vec<RenderInlineImageFragment>,
    Vec<(i64, i64)>,
    HashSet<usize>,
    HashMap<(usize, i64, i64), (i64, i64)>,
);

#[doc(hidden)]
#[must_use]
pub fn render_inline_images_from_terminal(
    terminal: &Terminal,
    first_source_row: usize,
    rows: u16,
    columns: u16,
) -> TerminalInlineImageProjection {
    let last_source_row = first_source_row.saturating_add(usize::from(rows));
    let terminal_fragments = terminal.inline_image_fragments();
    let fragment_parent_indices = terminal_fragments
        .iter()
        .filter(|fragment| {
            (fragment.cell_attachment
                && ((fragment.row >= first_source_row && fragment.row < last_source_row)
                    || terminal
                        .inline_images()
                        .get(fragment.image_index)
                        .is_some_and(|image| image.target_y.is_some())))
                || (fragment.row >= first_source_row
                    && fragment.row < last_source_row
                    && fragment.column < columns)
        })
        .map(|fragment| fragment.image_index)
        .collect::<HashSet<_>>();
    let mut images = terminal
        .inline_images()
        .iter()
        .enumerate()
        .filter_map(|(image_index, image)| {
            render_inline_image_item(image, first_source_row, last_source_row, columns)
                .or_else(|| {
                    fragment_parent_indices
                        .contains(&image_index)
                        .then(|| {
                            render_inline_image_item(image, image.row, image.row + 1, u16::MAX)
                        })
                        .flatten()
                })
                .and_then(|render_image| {
                    Some((
                        image_index,
                        render_image,
                        (
                            i64::from(image.column),
                            i64::try_from(image.row).ok()?
                                - i64::try_from(first_source_row).ok()?,
                        ),
                    ))
                })
        })
        .collect::<Vec<_>>();
    images.sort_by_key(|(_, image, _)| (image.row, image.column));
    let parent_indices = images
        .iter()
        .enumerate()
        .map(|(render_index, (terminal_index, _, _))| (*terminal_index, render_index))
        .collect::<HashMap<_, _>>();
    let empty_inline_image_attachment_parents = images
        .iter()
        .enumerate()
        .filter_map(|(render_index, (terminal_index, _, _))| {
            terminal
                .inline_image_attachment_parent_is_empty(*terminal_index)
                .then_some(render_index)
        })
        .collect::<HashSet<_>>();
    let (fragments, attachment_viewport_offsets) = terminal_fragments
        .into_iter()
        .filter_map(|fragment| {
            render_inline_image_fragment_item(
                &fragment,
                &parent_indices,
                first_source_row,
                last_source_row,
                columns,
            )
        })
        .unzip();
    (
        images.iter().map(|(_, image, _)| image.clone()).collect(),
        fragments,
        images.into_iter().map(|(_, _, origin)| origin).collect(),
        empty_inline_image_attachment_parents,
        attachment_viewport_offsets,
    )
}

fn render_inline_image_fragment_item(
    fragment: &InlineImageFragment,
    parent_indices: &HashMap<usize, usize>,
    first_source_row: usize,
    last_source_row: usize,
    columns: u16,
) -> Option<(RenderInlineImageFragment, AttachmentViewportOffset)> {
    if fragment.column >= columns
        || (!fragment.cell_attachment
            && (fragment.row < first_source_row || fragment.row >= last_source_row))
    {
        return None;
    }
    let parent_image_index = *parent_indices.get(&fragment.image_index)?;
    let relative_row = i64::try_from(fragment.row).ok()? - i64::try_from(first_source_row).ok()?;
    let row = u16::try_from(relative_row.max(0)).ok()?;
    let render_fragment = RenderInlineImageFragment {
        parent_image_index,
        cell_attachment: fragment.cell_attachment,
        row,
        column: fragment.column,
        source_row: if fragment.cell_attachment {
            i64::try_from(fragment.source_row).ok()?
        } else {
            i64::try_from(fragment.source_row).ok()? - i64::try_from(first_source_row).ok()?
        },
        source_column: i64::from(fragment.source_column),
        destination_x: fragment.destination_x,
        destination_y: fragment.destination_y,
        destination_width: fragment.destination_width,
        destination_height: fragment.destination_height,
        source_x: fragment.source_x,
        source_y: fragment.source_y,
        source_width: fragment.source_width,
        source_height: fragment.source_height,
        sampling_source_x: fragment.sampling_source_x,
        sampling_source_y: fragment.sampling_source_y,
        sampling_source_width: fragment.sampling_source_width,
        sampling_source_height: fragment.sampling_source_height,
        source_destination_x: fragment.source_destination_x,
        source_destination_y: fragment.source_destination_y,
        source_destination_width: fragment.source_destination_width,
        source_destination_height: fragment.source_destination_height,
    };
    let attachment_key = (
        parent_image_index,
        render_fragment.source_row,
        render_fragment.source_column,
    );
    Some((
        render_fragment,
        (attachment_key, (relative_row - i64::from(row), 0)),
    ))
}

fn render_inline_image_item(
    image: &ItermInlineImage,
    first_source_row: usize,
    last_source_row: usize,
    columns: u16,
) -> Option<RenderInlineImage> {
    if image.row < first_source_row || image.row >= last_source_row || image.column >= columns {
        return None;
    }

    let row = u16::try_from(image.row - first_source_row).ok()?;
    Some(RenderInlineImage {
        row,
        column: image.column,
        name: image.name.clone(),
        kitty_image_id: image.kitty_image_id,
        kitty_placement_id: image.kitty_placement_id,
        kitty_z_index: image.kitty_z_index,
        size: image.size,
        width: image.width.clone(),
        height: image.height.clone(),
        preserve_aspect_ratio: image.preserve_aspect_ratio,
        image_format: image.image_format,
        pixel_width: image.pixel_width,
        pixel_height: image.pixel_height,
        source_x: image.source_x,
        source_y: image.source_y,
        source_width: image.source_width,
        source_height: image.source_height,
        target_x: image.target_x,
        target_y: image.target_y,
        data: Arc::from(image.data.as_slice()),
    })
}

fn render_cursor_from_terminal(
    terminal: &Terminal,
    scrollback_offset: usize,
) -> Option<RenderCursor> {
    if !terminal.cursor_visible() || scrollback_offset != 0 {
        return None;
    }

    let (row, column) = terminal.cursor();
    Some(RenderCursor {
        row,
        column,
        shape: terminal.cursor_shape(),
        blinking: terminal.cursor_blinking(),
    })
}

fn append_grid_row(
    cells: &mut Vec<RenderCell>,
    grid: &TerminalGrid,
    viewport_row: u16,
    grid_row: usize,
    columns: u16,
    screen_reverse: bool,
) {
    let Ok(grid_row) = u16::try_from(grid_row) else {
        return;
    };

    if grid_row >= grid.size().rows {
        return;
    }

    for column in 0..columns {
        let Some(cell) = grid.get(grid_row, column) else {
            continue;
        };
        append_render_cell(cells, viewport_row, column, cell, screen_reverse);
    }
}

fn append_render_cells(
    cells: &mut Vec<RenderCell>,
    viewport_row: u16,
    source_cells: &[Cell],
    columns: u16,
    screen_reverse: bool,
) {
    for (column, cell) in source_cells.iter().take(usize::from(columns)).enumerate() {
        let Ok(column) = u16::try_from(column) else {
            continue;
        };
        append_render_cell(cells, viewport_row, column, cell, screen_reverse);
    }
}

fn append_render_cell(
    cells: &mut Vec<RenderCell>,
    row: u16,
    column: u16,
    cell: &Cell,
    screen_reverse: bool,
) {
    if !screen_reverse && !cell_has_renderable_content(cell) {
        return;
    }

    cells.push(render_cell_from_terminal(row, column, cell, screen_reverse));
}

fn render_cell_from_terminal(
    row: u16,
    column: u16,
    cell: &Cell,
    screen_reverse: bool,
) -> RenderCell {
    RenderCell {
        row,
        column,
        text: Arc::from(cell.text()),
        columns: cell.columns(),
        continuation: cell.is_continuation(),
        ch: cell.primary_char(),
        style: Arc::new(RenderStyle {
            foreground: cell.foreground,
            background: cell.background,
            underline_color: cell.underline_color,
            underline_style: cell.underline_style,
            bold: cell.bold,
            faint: cell.faint,
            italic: cell.italic,
            blink: cell.blink,
            rapid_blink: cell.rapid_blink,
            underline: cell.underline,
            double_underline: cell.double_underline,
            conceal: cell.conceal,
            strikethrough: cell.strikethrough,
            overline: cell.overline,
            vertical_align: cell.vertical_align,
            inverse: cell.inverse ^ screen_reverse,
        }),
        hyperlink: cell.hyperlink.clone(),
    }
}

fn intern_render_cells(cells: &mut [RenderCell]) {
    let mut graphemes = HashMap::<Arc<str>, Arc<str>>::new();
    let mut styles = HashMap::<RenderStyle, Arc<RenderStyle>>::new();

    for cell in cells {
        if let Some(grapheme) = graphemes.get(cell.text.as_ref()) {
            cell.text = Arc::clone(grapheme);
        } else {
            graphemes.insert(Arc::clone(&cell.text), Arc::clone(&cell.text));
        }

        if let Some(style) = styles.get(cell.style.as_ref()) {
            cell.style = Arc::clone(style);
        } else {
            styles.insert(cell.style.as_ref().clone(), Arc::clone(&cell.style));
        }
    }
}

fn intern_render_image_payloads(images: &mut [RenderInlineImage]) {
    let mut payloads = HashMap::<Arc<[u8]>, Arc<[u8]>>::new();
    for image in images {
        if let Some(payload) = payloads.get(image.data.as_ref()) {
            image.data = Arc::clone(payload);
        } else {
            payloads.insert(Arc::clone(&image.data), Arc::clone(&image.data));
        }
    }
}

fn rows_from_cells(cells: Vec<RenderCell>, minimum_row_count: u16) -> Vec<Arc<RenderRowSnapshot>> {
    let row_count = cells
        .iter()
        .map(|cell| cell.row.saturating_add(1))
        .max()
        .unwrap_or(0)
        .max(minimum_row_count);
    let mut rows = (0..row_count).map(|_| Vec::new()).collect::<Vec<_>>();
    for cell in cells {
        rows[usize::from(cell.row)].push(cell);
    }
    rows.into_iter()
        .enumerate()
        .map(|(row, mut cells)| {
            cells.sort_by_key(|cell| cell.column);
            Arc::new(RenderRowSnapshot {
                row: u16::try_from(row).unwrap_or(u16::MAX),
                cells: cells.into(),
            })
        })
        .collect()
}

fn empty_render_row(row: u16) -> Arc<RenderRowSnapshot> {
    Arc::new(RenderRowSnapshot {
        row,
        cells: Arc::from([]),
    })
}

fn render_row_retained_bytes(row: &RenderRowSnapshot) -> usize {
    row.cells.iter().fold(
        std::mem::size_of::<RenderRowSnapshot>()
            .saturating_add(std::mem::size_of_val(row.cells.as_ref())),
        |total, cell| {
            total
                .saturating_add(cell.text.len())
                .saturating_add(std::mem::size_of::<RenderStyle>())
                .saturating_add(cell.hyperlink.as_ref().map_or(0, |link| link.len()))
        },
    )
}

fn snapshot_retained_bytes(snapshot: &TerminalRenderSnapshot) -> usize {
    let row_bytes = snapshot.rows.iter().fold(0usize, |total, row| {
        total.saturating_add(render_row_retained_bytes(row))
    });
    let mut payloads = HashSet::<*const [u8]>::new();
    let image_bytes = snapshot.inline_images.iter().fold(0usize, |total, image| {
        if payloads.insert(Arc::as_ptr(&image.data)) {
            total.saturating_add(image.data.len())
        } else {
            total
        }
    });
    row_bytes.saturating_add(image_bytes)
}

fn render_cells_owned_payload_bytes_checked(
    cells: &[RenderCell],
    text_payloads: &mut HashSet<(usize, usize)>,
    styles: &mut HashSet<usize>,
) -> Result<usize, ProjectOwnedLogicalBytesV1Error> {
    let mut retained = 0usize;
    for cell in cells {
        let text = (cell.text.as_ptr() as usize, cell.text.len());
        let hyperlink = cell
            .hyperlink
            .as_ref()
            .map(|link| (link.as_ptr() as usize, link.len()));
        let style = Arc::as_ptr(&cell.style) as usize;
        if text_payloads.insert(text) {
            checked_add(&mut retained, cell.text.len())?;
        }
        if styles.insert(style) {
            checked_add(&mut retained, std::mem::size_of::<RenderStyle>())?;
        }
        if let Some(identity) = hyperlink
            && text_payloads.insert(identity)
        {
            checked_add(&mut retained, identity.1)?;
        }
    }
    Ok(retained)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectOwnedLogicalBytesV1Error;

impl std::fmt::Display for ProjectOwnedLogicalBytesV1Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("project-owned logical byte calculation overflowed")
    }
}

impl std::error::Error for ProjectOwnedLogicalBytesV1Error {}

fn checked_add(retained: &mut usize, bytes: usize) -> Result<(), ProjectOwnedLogicalBytesV1Error> {
    *retained = retained
        .checked_add(bytes)
        .ok_or(ProjectOwnedLogicalBytesV1Error)?;
    Ok(())
}

fn checked_add_len<T>(
    retained: &mut usize,
    len: usize,
) -> Result<(), ProjectOwnedLogicalBytesV1Error> {
    let bytes = len
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(ProjectOwnedLogicalBytesV1Error)?;
    checked_add(retained, bytes)
}

fn checked_add_capacity<T>(
    retained: &mut usize,
    capacity: usize,
) -> Result<(), ProjectOwnedLogicalBytesV1Error> {
    checked_add_len::<T>(retained, capacity)
}

fn checked_add_map_len<K, V>(
    retained: &mut usize,
    len: usize,
) -> Result<(), ProjectOwnedLogicalBytesV1Error> {
    let entry_bytes = std::mem::size_of::<K>()
        .checked_add(std::mem::size_of::<V>())
        .ok_or(ProjectOwnedLogicalBytesV1Error)?;
    let bytes = len
        .checked_mul(entry_bytes)
        .ok_or(ProjectOwnedLogicalBytesV1Error)?;
    checked_add(retained, bytes)
}

fn cell_has_renderable_content(cell: &Cell) -> bool {
    cell.is_continuation()
        || (!cell.is_blank() && cell.text() != " ")
        || cell.background != Color::Default
        || cell.inverse
        || cell.underline
        || cell.double_underline
        || cell.strikethrough
        || cell.overline
}

fn blend_channel(foreground: u8, background: u8, alpha: u16, inverse_alpha: u16) -> u8 {
    let blended = u16::from(foreground)
        .saturating_mul(alpha)
        .saturating_add(u16::from(background).saturating_mul(inverse_alpha))
        / u16::from(u8::MAX);
    u8::try_from(blended).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod project_owned_retained_bytes_tests {
    use super::*;

    fn fixture_snapshot() -> TerminalRenderSnapshot {
        let mut cell = RenderCell::new(0, 0, "wide-owned-grapheme");
        cell.hyperlink = Some(Arc::from("https://stage7.invalid/owned"));
        let mut row_cells = Vec::with_capacity(3);
        row_cells.push(cell);
        let mut rows = Vec::with_capacity(4);
        rows.push(Arc::new(RenderRowSnapshot {
            row: 0,
            cells: row_cells.into(),
        }));

        let payload: Arc<[u8]> = Arc::from(vec![7_u8; 257]);
        let image = |name: &str| RenderInlineImage {
            row: 0,
            column: 0,
            name: Some(name.to_owned()),
            kitty_image_id: Some(7),
            kitty_placement_id: Some(8),
            kitty_z_index: Some(9),
            size: Some(payload.len()),
            width: Some("17cells".to_owned()),
            height: Some("9cells".to_owned()),
            preserve_aspect_ratio: Some(true),
            image_format: InlineImageFormat::Rgba,
            pixel_width: Some(17),
            pixel_height: Some(9),
            source_x: Some(1),
            source_y: Some(2),
            source_width: Some(3),
            source_height: Some(4),
            target_x: Some(5),
            target_y: Some(6),
            data: Arc::clone(&payload),
        };
        let mut inline_images = Vec::with_capacity(4);
        inline_images.push(image("owned-image-one"));
        inline_images.push(image("owned-image-two"));

        let mut inline_image_fragments = Vec::with_capacity(3);
        inline_image_fragments.push(RenderInlineImageFragment {
            parent_image_index: 0,
            cell_attachment: true,
            row: 0,
            column: 0,
            source_row: 0,
            source_column: 0,
            destination_x: 0,
            destination_y: 0,
            destination_width: 1,
            destination_height: 1,
            source_x: 0,
            source_y: 0,
            source_width: 1,
            source_height: 1,
            sampling_source_x: 0,
            sampling_source_y: 0,
            sampling_source_width: 1,
            sampling_source_height: 1,
            source_destination_x: 0,
            source_destination_y: 0,
            source_destination_width: 1,
            source_destination_height: 1,
        });
        let mut inline_image_parent_origins = Vec::with_capacity(3);
        inline_image_parent_origins.push((1, 2));
        let mut empty_inline_image_attachment_parents = HashSet::with_capacity(4);
        empty_inline_image_attachment_parents.insert(1);
        let mut inline_image_attachment_viewport_offsets = HashMap::with_capacity(4);
        inline_image_attachment_viewport_offsets.insert((0, 1, 2), (3, 4));
        let mut inline_image_attachment_viewport_clips = HashMap::with_capacity(4);
        inline_image_attachment_viewport_clips.insert(
            (0, 1, 2),
            AttachmentViewportClip {
                left: 1,
                top: 2,
                right: 3,
                bottom: 4,
            },
        );

        let snapshot = TerminalRenderSnapshot {
            rows,
            compatibility_cells: OnceLock::new(),
            cursor: Some(RenderCursor {
                row: 0,
                column: 0,
                shape: CursorShape::Block,
                blinking: true,
            }),
            cursor_color: Some(Color::Indexed(7)),
            inline_images,
            inline_image_fragments,
            inline_image_parent_origins,
            empty_inline_image_attachment_parents,
            inline_image_attachment_viewport_offsets,
            inline_image_attachment_viewport_clips,
            scrollback_offset: 3,
        };
        let _ = snapshot.cells();
        snapshot
    }

    #[test]
    fn project_owned_snapshot_bytes_cover_all_containers_and_unique_payloads() {
        let snapshot = fixture_snapshot();
        let bytes = snapshot
            .project_owned_logical_bytes_v1()
            .expect("fixture snapshot size");
        assert!(
            bytes > snapshot_retained_bytes(&snapshot),
            "the attribution metric must include metadata omitted by the cache budget metric"
        );
        let structural_floor = std::mem::size_of::<TerminalRenderSnapshot>()
            .saturating_add(
                snapshot
                    .rows
                    .capacity()
                    .saturating_mul(std::mem::size_of::<Arc<RenderRowSnapshot>>()),
            )
            .saturating_add(
                snapshot
                    .inline_images
                    .capacity()
                    .saturating_mul(std::mem::size_of::<RenderInlineImage>()),
            )
            .saturating_add(
                snapshot
                    .inline_image_fragments
                    .capacity()
                    .saturating_mul(std::mem::size_of::<RenderInlineImageFragment>()),
            )
            .saturating_add(
                snapshot
                    .inline_image_parent_origins
                    .capacity()
                    .saturating_mul(std::mem::size_of::<(i64, i64)>()),
            )
            .saturating_add(
                snapshot
                    .cells()
                    .len()
                    .saturating_mul(std::mem::size_of::<RenderCell>()),
            );
        assert!(bytes >= structural_floor);

        let shared = snapshot.clone();
        let mut distinct = snapshot.clone();
        distinct.inline_images[1].data = Arc::from(vec![9_u8; 257]);
        assert_eq!(
            distinct
                .project_owned_logical_bytes_v1()
                .expect("distinct snapshot bytes")
                .checked_sub(
                    shared
                        .project_owned_logical_bytes_v1()
                        .expect("shared snapshot bytes"),
                )
                .expect("distinct payload is larger"),
            257,
            "shared inline-image payloads count once while distinct payloads count independently"
        );
    }

    #[test]
    fn project_owned_snapshot_v1_is_checked_and_does_not_guess_allocator_overhead() {
        let source = include_str!("lib.rs");
        let metric = source
            .split("pub fn project_owned_logical_bytes_v1")
            .nth(1)
            .expect("project-owned snapshot metric")
            .split("pub fn rows")
            .next()
            .expect("bounded project-owned snapshot metric");

        assert!(
            metric.contains("Result<usize, ProjectOwnedLogicalBytesV1Error>"),
            "the v1 logical byte formula must fail closed on arithmetic overflow"
        );
        assert!(
            !metric.contains("saturating_") && !metric.contains("unwrap_or"),
            "overflow must not silently clamp the attributed owner size"
        );
        assert!(
            source.contains("v1 logical explicit bytes")
                && source.contains("excludes allocator")
                && source.contains("hash-table control"),
            "the stable v1 formula and its excluded overhead must be documented"
        );
        let map = source
            .split("fn hash_map_retained_bytes")
            .nth(1)
            .expect("map logical byte helper")
            .split("fn cell_has_renderable_content")
            .next()
            .expect("bounded map logical byte helper");
        assert!(
            !map.contains(".saturating_add(1)"),
            "the logical metric must not guess one allocator byte per hash bucket"
        );
    }

    #[test]
    fn project_owned_snapshot_v1_overflow_fails_closed() {
        let mut retained = usize::MAX;
        assert_eq!(
            checked_add(&mut retained, 1),
            Err(ProjectOwnedLogicalBytesV1Error)
        );
        assert_eq!(retained, usize::MAX, "failure must not wrap the total");

        let mut retained = 0;
        assert_eq!(
            checked_add_len::<u64>(&mut retained, usize::MAX),
            Err(ProjectOwnedLogicalBytesV1Error)
        );
        assert_eq!(retained, 0, "failure must not partially update the total");
    }
}
