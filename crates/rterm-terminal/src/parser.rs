use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::{Path, PathBuf};

use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::read::ZlibDecoder;
use rterm_types::{DamageRegion, TerminalSize};
use smol_str::SmolStr;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::canonical_combining_class;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    Cell, CellAttachment, Color, CursorShape, CursorStyle, HistoryBuffer, InlineImageFormat,
    InlineImageFragment, ItermInlineImage, ScrollbackLine, SemanticCommandExit, SemanticType,
    SemanticZone, SequenceNo, StableRowIndex, StableSelectionRange, StableSemanticCommandExit,
    StableSemanticZone, TerminalGrid, TerminalResizeOutcome, TerminalScreenDomain,
    TerminalStableDimensions, UnderlineStyle,
};

pub const DEFAULT_SCROLLBACK_LIMIT: usize = 3_500;
const INITIAL_SEQUENCE_NO: SequenceNo = 1;
const DEFAULT_UNICODE_VERSION: u32 = 9;
const UNICODE_PRESENTATION_SELECTOR_VERSION: u32 = 14;
const TEXT_PRESENTATION_SELECTOR: char = '\u{fe0e}';
const EMOJI_PRESENTATION_SELECTOR: char = '\u{fe0f}';
// WezTerm's WidenedIn9 set: width 1 for Unicode <= 8, width 2 for Unicode >= 9.
const WIDENED_IN_UNICODE9: &[(u32, u32)] = &[
    (0x0231A, 0x0231B),
    (0x023E9, 0x023EC),
    (0x023F0, 0x023F0),
    (0x023F3, 0x023F3),
    (0x025FD, 0x025FE),
    (0x02614, 0x02615),
    (0x02648, 0x02653),
    (0x0267F, 0x0267F),
    (0x02693, 0x02693),
    (0x026A1, 0x026A1),
    (0x026AA, 0x026AB),
    (0x026BD, 0x026BE),
    (0x026C4, 0x026C5),
    (0x026CE, 0x026CE),
    (0x026D4, 0x026D4),
    (0x026EA, 0x026EA),
    (0x026F2, 0x026F3),
    (0x026F5, 0x026F5),
    (0x026FA, 0x026FA),
    (0x026FD, 0x026FD),
    (0x02705, 0x02705),
    (0x0270A, 0x0270B),
    (0x02728, 0x02728),
    (0x0274C, 0x0274C),
    (0x0274E, 0x0274E),
    (0x02753, 0x02755),
    (0x02757, 0x02757),
    (0x02795, 0x02797),
    (0x027B0, 0x027B0),
    (0x027BF, 0x027BF),
    (0x02B1B, 0x02B1C),
    (0x02B50, 0x02B50),
    (0x02B55, 0x02B55),
    (0x1F004, 0x1F004),
    (0x1F0CF, 0x1F0CF),
    (0x1F18E, 0x1F18E),
    (0x1F191, 0x1F19A),
    (0x1F201, 0x1F201),
    (0x1F21A, 0x1F21A),
    (0x1F22F, 0x1F22F),
    (0x1F232, 0x1F236),
    (0x1F238, 0x1F23A),
    (0x1F250, 0x1F251),
    (0x1F300, 0x1F320),
    (0x1F32D, 0x1F335),
    (0x1F337, 0x1F37C),
    (0x1F37E, 0x1F393),
    (0x1F3A0, 0x1F3CA),
    (0x1F3CF, 0x1F3D3),
    (0x1F3E0, 0x1F3F0),
    (0x1F3F4, 0x1F3F4),
    (0x1F3F8, 0x1F43E),
    (0x1F440, 0x1F440),
    (0x1F442, 0x1F4FC),
    (0x1F4FF, 0x1F53D),
    (0x1F54B, 0x1F54E),
    (0x1F550, 0x1F567),
    (0x1F595, 0x1F596),
    (0x1F5FB, 0x1F64F),
    (0x1F680, 0x1F6C5),
    (0x1F6CC, 0x1F6CC),
    (0x1F6D0, 0x1F6D0),
    (0x1F6EB, 0x1F6EC),
    (0x1F910, 0x1F918),
    (0x1F980, 0x1F984),
    (0x1F9C0, 0x1F9C0),
];
const ANONYMOUS_KITTY_IMAGE_ID: u32 = 0;
const MAX_KITTY_RELATIVE_CHAIN_DEPTH: usize = 8;
const KITTY_UNICODE_PLACEHOLDER: char = '\u{10eeee}';
const KITTY_PLACEHOLDER_DIACRITICS: [char; 256] = [
    '\u{0305}', '\u{030d}', '\u{030e}', '\u{0310}', '\u{0312}', '\u{033d}', '\u{033e}', '\u{033f}',
    '\u{0346}', '\u{034a}', '\u{034b}', '\u{034c}', '\u{0350}', '\u{0351}', '\u{0352}', '\u{0357}',
    '\u{035b}', '\u{0363}', '\u{0364}', '\u{0365}', '\u{0366}', '\u{0367}', '\u{0368}', '\u{0369}',
    '\u{036a}', '\u{036b}', '\u{036c}', '\u{036d}', '\u{036e}', '\u{036f}', '\u{0483}', '\u{0484}',
    '\u{0485}', '\u{0486}', '\u{0487}', '\u{0592}', '\u{0593}', '\u{0594}', '\u{0595}', '\u{0597}',
    '\u{0598}', '\u{0599}', '\u{059c}', '\u{059d}', '\u{059e}', '\u{059f}', '\u{05a0}', '\u{05a1}',
    '\u{05a8}', '\u{05a9}', '\u{05ab}', '\u{05ac}', '\u{05af}', '\u{05c4}', '\u{0610}', '\u{0611}',
    '\u{0612}', '\u{0613}', '\u{0614}', '\u{0615}', '\u{0616}', '\u{0617}', '\u{0657}', '\u{0658}',
    '\u{0659}', '\u{065a}', '\u{065b}', '\u{065d}', '\u{065e}', '\u{06d6}', '\u{06d7}', '\u{06d8}',
    '\u{06d9}', '\u{06da}', '\u{06db}', '\u{06dc}', '\u{06df}', '\u{06e0}', '\u{06e1}', '\u{06e2}',
    '\u{06e4}', '\u{06e7}', '\u{06e8}', '\u{06eb}', '\u{06ec}', '\u{0730}', '\u{0732}', '\u{0733}',
    '\u{0735}', '\u{0736}', '\u{073a}', '\u{073d}', '\u{073f}', '\u{0740}', '\u{0741}', '\u{0743}',
    '\u{0745}', '\u{0747}', '\u{0749}', '\u{074a}', '\u{07eb}', '\u{07ec}', '\u{07ed}', '\u{07ee}',
    '\u{07ef}', '\u{07f0}', '\u{07f1}', '\u{07f3}', '\u{0816}', '\u{0817}', '\u{0818}', '\u{0819}',
    '\u{081b}', '\u{081c}', '\u{081d}', '\u{081e}', '\u{081f}', '\u{0820}', '\u{0821}', '\u{0822}',
    '\u{0823}', '\u{0825}', '\u{0826}', '\u{0827}', '\u{0829}', '\u{082a}', '\u{082b}', '\u{082c}',
    '\u{082d}', '\u{0951}', '\u{0953}', '\u{0954}', '\u{0f82}', '\u{0f83}', '\u{0f86}', '\u{0f87}',
    '\u{135d}', '\u{135e}', '\u{135f}', '\u{17dd}', '\u{193a}', '\u{1a17}', '\u{1a75}', '\u{1a76}',
    '\u{1a77}', '\u{1a78}', '\u{1a79}', '\u{1a7a}', '\u{1a7b}', '\u{1a7c}', '\u{1b6b}', '\u{1b6d}',
    '\u{1b6e}', '\u{1b6f}', '\u{1b70}', '\u{1b71}', '\u{1b72}', '\u{1b73}', '\u{1cd0}', '\u{1cd1}',
    '\u{1cd2}', '\u{1cda}', '\u{1cdb}', '\u{1ce0}', '\u{1dc0}', '\u{1dc1}', '\u{1dc3}', '\u{1dc4}',
    '\u{1dc5}', '\u{1dc6}', '\u{1dc7}', '\u{1dc8}', '\u{1dc9}', '\u{1dcb}', '\u{1dcc}', '\u{1dd1}',
    '\u{1dd2}', '\u{1dd3}', '\u{1dd4}', '\u{1dd5}', '\u{1dd6}', '\u{1dd7}', '\u{1dd8}', '\u{1dd9}',
    '\u{1dda}', '\u{1ddb}', '\u{1ddc}', '\u{1ddd}', '\u{1dde}', '\u{1ddf}', '\u{1de0}', '\u{1de1}',
    '\u{1de2}', '\u{1de3}', '\u{1de4}', '\u{1de5}', '\u{1de6}', '\u{1dfe}', '\u{20d0}', '\u{20d1}',
    '\u{20d4}', '\u{20d5}', '\u{20d6}', '\u{20d7}', '\u{20db}', '\u{20dc}', '\u{20e1}', '\u{20e7}',
    '\u{20e9}', '\u{20f0}', '\u{2cef}', '\u{2cf0}', '\u{2cf1}', '\u{2de0}', '\u{2de1}', '\u{2de2}',
    '\u{2de3}', '\u{2de4}', '\u{2de5}', '\u{2de6}', '\u{2de7}', '\u{2de8}', '\u{2de9}', '\u{2dea}',
    '\u{2deb}', '\u{2dec}', '\u{2ded}', '\u{2dee}', '\u{2def}', '\u{2df0}', '\u{2df1}', '\u{2df2}',
    '\u{2df3}', '\u{2df4}', '\u{2df5}', '\u{2df6}', '\u{2df7}', '\u{2df8}', '\u{2df9}', '\u{2dfa}',
    '\u{2dfb}', '\u{2dfc}', '\u{2dfd}', '\u{2dfe}', '\u{2dff}', '\u{a66f}', '\u{a67c}', '\u{a67d}',
    '\u{a6f0}', '\u{a6f1}', '\u{a8e0}', '\u{a8e1}', '\u{a8e2}', '\u{a8e3}', '\u{a8e4}', '\u{a8e5}',
];
type KittyPlacementKey = (u32, u32);
type KittyPlaceholderRenderKey = (usize, u16, u32, Option<u32>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalUnknownEscapeSequence {
    pub sequence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharacterSet {
    Ascii,
    DecSpecialGraphics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharacterWriteMode {
    Replace,
    Insert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct TerminalModes {
    cursor_visible: bool,
    cursor_blinking: bool,
    cursor_shape: CursorShape,
    screen_reverse: bool,
    auto_wrap: bool,
    reverse_wrap: bool,
    sixel_display_mode: bool,
    sixel_scrolls_right: bool,
    origin_mode: bool,
    left_right_margin_mode: bool,
    write_mode: CharacterWriteMode,
}

impl Default for TerminalModes {
    fn default() -> Self {
        Self {
            cursor_visible: true,
            cursor_blinking: false,
            cursor_shape: CursorShape::Block,
            screen_reverse: false,
            auto_wrap: true,
            reverse_wrap: false,
            sixel_display_mode: false,
            sixel_scrolls_right: false,
            origin_mode: false,
            left_right_margin_mode: false,
            write_mode: CharacterWriteMode::Replace,
        }
    }
}

impl TerminalModes {
    fn with_cursor_style(cursor_style: CursorStyle) -> Self {
        Self {
            cursor_shape: cursor_style.shape(),
            cursor_blinking: cursor_style.blinking(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
struct PendingKittyGraphics {
    image_format: InlineImageFormat,
    medium: KittyTransmissionMedium,
    compression: Option<char>,
    action: KittyUploadAction,
    image_id: Option<u32>,
    image_number: Option<u32>,
    placement_id: Option<u32>,
    z_index: Option<i32>,
    pixel_width: Option<u32>,
    pixel_height: Option<u32>,
    display_columns: Option<u16>,
    display_rows: Option<u16>,
    no_cursor_movement: bool,
    source_x: Option<u32>,
    source_y: Option<u32>,
    source_width: Option<u32>,
    source_height: Option<u32>,
    target_x: Option<u32>,
    target_y: Option<u32>,
    file_offset: Option<u64>,
    file_size: Option<u64>,
    quiet: Option<u8>,
    virtual_placement: bool,
    encoded_data: String,
}

#[derive(Debug, Clone)]
struct StoredKittyImage {
    image_format: InlineImageFormat,
    pixel_width: Option<u32>,
    pixel_height: Option<u32>,
    display_columns: Option<u16>,
    display_rows: Option<u16>,
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
struct KittyVirtualPlacement {
    image_id: u32,
    placement_id: Option<u32>,
    z_index: Option<i32>,
    display_columns: Option<u16>,
    display_rows: Option<u16>,
    source_rect: KittySourceRect,
    target_x: Option<u32>,
    target_y: Option<u32>,
}

#[derive(Debug, Clone)]
struct PendingKittyPlaceholder {
    row: usize,
    column: u16,
    foreground: Color,
    underline_color: Color,
    image_id: Option<u32>,
    placement_id: Option<u32>,
    diacritics: Vec<u32>,
    rendered_row: Option<usize>,
    rendered_column: Option<u16>,
    rendered_image_id: Option<u32>,
    rendered_placement_id: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
struct LastKittyPlaceholder {
    row: usize,
    column: u16,
    foreground: Color,
    underline_color: Color,
    image_id_high_byte: u32,
    placeholder_row: u32,
    placeholder_column: u32,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedKittyPlaceholder {
    image_id: u32,
    image_id_high_byte: u32,
    placeholder_row: u32,
    placeholder_column: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct KittySourceRect {
    x: Option<u32>,
    y: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default)]
struct KittyPlacementOptions {
    display_columns: Option<u16>,
    display_rows: Option<u16>,
    image_id: Option<u32>,
    placement_id: Option<u32>,
    z_index: Option<i32>,
    parent_placement: Option<KittyPlacementKey>,
    row: Option<usize>,
    column: Option<u16>,
    move_cursor: bool,
    source_rect: KittySourceRect,
    target_x: Option<u32>,
    target_y: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KittyTransmissionMedium {
    Direct,
    File,
    TempFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KittyUploadAction {
    Display,
    Store,
    Query,
}

#[derive(Debug, Clone, Copy, Default)]
struct KittyGraphicsParams {
    action: Option<char>,
    format: Option<u32>,
    medium: Option<char>,
    compression: Option<char>,
    image_id: Option<u32>,
    image_number: Option<u32>,
    placement_id: Option<u32>,
    parent_image_id: Option<u32>,
    parent_placement_id: Option<u32>,
    parent_offset_columns: Option<i32>,
    parent_offset_rows: Option<i32>,
    z_index: Option<i32>,
    delete_target: Option<char>,
    cell_x: Option<u16>,
    cell_y: Option<u16>,
    more_chunks: Option<u8>,
    pixel_width: Option<u32>,
    pixel_height: Option<u32>,
    display_columns: Option<u16>,
    display_rows: Option<u16>,
    no_cursor_movement: bool,
    virtual_placement: bool,
    source_x: Option<u32>,
    source_y: Option<u32>,
    source_width: Option<u32>,
    source_height: Option<u32>,
    target_x: Option<u32>,
    target_y: Option<u32>,
    file_offset: Option<u64>,
    file_size: Option<u64>,
    quiet: Option<u8>,
}

impl PendingKittyGraphics {
    const fn source_rect(&self) -> KittySourceRect {
        KittySourceRect {
            x: self.source_x,
            y: self.source_y,
            width: self.source_width,
            height: self.source_height,
        }
    }
}

impl KittyGraphicsParams {
    const fn source_rect(self) -> KittySourceRect {
        KittySourceRect {
            x: self.source_x,
            y: self.source_y,
            width: self.source_width,
            height: self.source_height,
        }
    }
}

#[derive(Debug, Clone)]
struct TabStops {
    columns: Vec<u16>,
}

impl TabStops {
    fn new(size: TerminalSize) -> Self {
        Self {
            columns: default_tab_stops(size),
        }
    }

    fn resize(&mut self, size: TerminalSize) {
        self.columns.retain(|column| *column < size.columns);
    }

    fn set(&mut self, column: u16, size: TerminalSize) {
        if column >= size.columns || self.columns.binary_search(&column).is_ok() {
            return;
        }

        let index = self.columns.partition_point(|stop| *stop < column);
        self.columns.insert(index, column);
    }

    fn clear(&mut self, column: u16) {
        self.columns.retain(|stop| *stop != column);
    }

    fn clear_all(&mut self) {
        self.columns.clear();
    }

    fn next_after(&self, column: u16, fallback: u16) -> u16 {
        self.columns
            .iter()
            .copied()
            .find(|stop| *stop > column)
            .unwrap_or(fallback)
    }

    fn previous_before(&self, column: u16) -> u16 {
        self.columns
            .iter()
            .rev()
            .copied()
            .find(|stop| *stop < column)
            .unwrap_or(0)
    }
}

/// Cumulative deterministic work performed by the current scrollback implementation.
///
/// Fields saturate at `u64::MAX`, so observing them never changes terminal
/// behavior even during a long-running session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerminalWorkCounters {
    /// Individual surviving grid cells cloned by bounded horizontal-margin scrolls.
    ///
    /// Full-width vertical scrolling rotates row ownership and records zero.
    pub scrolled_survivor_cell_clones: u64,
    /// Surviving history rows physically relocated by prefix pruning.
    ///
    /// Logical deque front eviction records zero because survivors retain their
    /// ring-buffer slots. Removing one front row is O(1), removing `k` rows is
    /// O(k), and the complete terminal prune remains O(k + metadata).
    pub history_row_relocations: u64,
    /// Non-empty history prune operations that run the metadata rebase pass.
    pub metadata_rebase_batches: u64,
}

/// Cumulative growth of reusable terminal feed/decode scratch storage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerminalFeedStorageCounters {
    growths: u64,
    damage_growths: u64,
}

impl TerminalFeedStorageCounters {
    #[must_use]
    pub const fn growths(self) -> u64 {
        self.growths
    }

    #[must_use]
    pub const fn damage_growths(self) -> u64 {
        self.damage_growths
    }
}

#[derive(Debug, Clone, Default)]
struct TerminalFeedScratch {
    input: Vec<u8>,
    chars: Vec<char>,
    counters: TerminalFeedStorageCounters,
}

#[derive(Debug, Clone, Default)]
struct PendingTerminalMetadataChanges {
    title: bool,
    current_working_dir: bool,
    badge_format: bool,
    user_vars: Vec<String>,
}

/// Metadata sources changed since the last runtime observation.
#[derive(Debug, Clone, Copy)]
pub struct TerminalMetadataChanges<'a> {
    title: bool,
    current_working_dir: bool,
    badge_format: bool,
    user_vars: &'a [String],
}

#[cfg(test)]
#[derive(Debug, Default)]
struct FixtureTraceIdentity {
    id: u64,
}

#[cfg(test)]
impl Clone for FixtureTraceIdentity {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<'a> TerminalMetadataChanges<'a> {
    #[must_use]
    pub const fn title(self) -> bool {
        self.title
    }

    #[must_use]
    pub const fn current_working_dir(self) -> bool {
        self.current_working_dir
    }

    #[must_use]
    pub const fn badge_format(self) -> bool {
        self.badge_format
    }

    #[must_use]
    pub const fn user_vars(self) -> &'a [String] {
        self.user_vars
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.title && !self.current_working_dir && !self.badge_format && self.user_vars.is_empty()
    }
}

impl TerminalWorkCounters {
    /// Returns work performed since `earlier`, saturating if snapshots are not
    /// ordered (for example, after replacing an observed terminal).
    #[must_use]
    pub const fn saturating_delta_since(self, earlier: Self) -> Self {
        Self {
            scrolled_survivor_cell_clones: self
                .scrolled_survivor_cell_clones
                .saturating_sub(earlier.scrolled_survivor_cell_clones),
            history_row_relocations: self
                .history_row_relocations
                .saturating_sub(earlier.history_row_relocations),
            metadata_rebase_batches: self
                .metadata_rebase_batches
                .saturating_sub(earlier.metadata_rebase_batches),
        }
    }
}

#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "these flags model independent terminal protocol modes with valid combinations"
)]
pub struct Terminal {
    grid: TerminalGrid,
    scrollback: HistoryBuffer<ScrollbackLine>,
    scrollback_limit: usize,
    seqno: SequenceNo,
    screen_identity_generation: SequenceNo,
    main_stable_row_offset: StableRowIndex,
    title: Option<String>,
    icon_title: Option<String>,
    window_title: Option<String>,
    title_stack: Vec<Option<String>>,
    current_working_dir: Option<String>,
    badge_format: Option<String>,
    user_vars: HashMap<String, String>,
    pending_metadata_changes: PendingTerminalMetadataChanges,
    inline_images: Vec<ItermInlineImage>,
    inline_image_parent_ids: Vec<u64>,
    inline_image_attachments: Vec<CellAttachment>,
    next_inline_image_parent_identity: u64,
    kitty_graphics_responses: Vec<Vec<u8>>,
    pending_kitty_graphics: Option<PendingKittyGraphics>,
    kitty_images: HashMap<u32, StoredKittyImage>,
    kitty_image_numbers: HashMap<u32, u32>,
    kitty_relative_parents: HashMap<KittyPlacementKey, KittyPlacementKey>,
    kitty_virtual_placements: HashMap<KittyPlacementKey, KittyVirtualPlacement>,
    kitty_character_edited_placements: HashSet<KittyPlacementKey>,
    pending_kitty_placeholder: Option<PendingKittyPlaceholder>,
    last_kitty_placeholder: Option<LastKittyPlaceholder>,
    kitty_placeholder_cells: HashMap<(usize, u16), LastKittyPlaceholder>,
    next_kitty_image_id: u32,
    enable_kitty_graphics: bool,
    semantic_prompt_rows: Vec<usize>,
    semantic_command_exits: Vec<SemanticCommandExit>,
    cursor_row: u16,
    cursor_column: u16,
    pending_wrap: bool,
    clear_semantic_type_on_movement: bool,
    pending_utf8: Vec<u8>,
    pending_control: Vec<char>,
    feed_scratch: TerminalFeedScratch,
    last_printable: Option<SmolStr>,
    nfc_last_printable_cell: Option<(u16, u16)>,
    saved_cursor: Option<SavedCursor>,
    main_screen: Option<ScreenState>,
    default_cursor_style: CursorStyle,
    modes: TerminalModes,
    scroll_top: u16,
    scroll_bottom: u16,
    left_margin: u16,
    right_margin: u16,
    character_set: CharacterSet,
    tab_stops: TabStops,
    style: Cell,
    damage: Vec<DamageRegion>,
    bell_count: u64,
    unknown_escape_sequences: Vec<TerminalUnknownEscapeSequence>,
    treat_east_asian_ambiguous_width_as_wide: bool,
    cell_width_overrides: Vec<CellWidthOverride>,
    normalize_output_to_unicode_nfc: bool,
    unicode_version: u32,
    unicode_version_stack: Vec<UnicodeVersionStackEntry>,
    work_counters: TerminalWorkCounters,
    #[cfg(test)]
    fixture_trace: FixtureTraceIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellWidthOverride {
    pub first: u32,
    pub last: u32,
    pub width: u16,
}

impl CellWidthOverride {
    #[must_use]
    pub const fn new(first: u32, last: u32, width: u16) -> Self {
        Self { first, last, width }
    }

    fn contains(self, ch: char) -> bool {
        let codepoint = ch as u32;
        self.first <= codepoint && codepoint <= self.last
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnicodeVersionStackEntry {
    version: u32,
    label: Option<String>,
}

#[derive(Debug, Clone)]
struct ScreenState {
    grid: TerminalGrid,
    inline_images: Vec<ItermInlineImage>,
    inline_image_parent_ids: Vec<u64>,
    inline_image_attachments: Vec<CellAttachment>,
    kitty_character_edited_placements: HashSet<KittyPlacementKey>,
    last_kitty_placeholder: Option<LastKittyPlaceholder>,
    kitty_placeholder_cells: HashMap<(usize, u16), LastKittyPlaceholder>,
    cursor_row: u16,
    cursor_column: u16,
    pending_wrap: bool,
    clear_semantic_type_on_movement: bool,
    last_printable: Option<SmolStr>,
    nfc_last_printable_cell: Option<(u16, u16)>,
    saved_cursor: Option<SavedCursor>,
    modes: TerminalModes,
    scroll_top: u16,
    scroll_bottom: u16,
    left_margin: u16,
    right_margin: u16,
    character_set: CharacterSet,
    style: Cell,
}

/// A terminal-cell copy operation expressed in history coordinates.
///
/// Narrow DECSLRM scrolling mutates only a rectangle of the grid.  Inline
/// image attachments are terminal cells too, so they must use the same source
/// to destination mapping as the grid copy, rather than moving or removing a
/// whole physical placement.
#[derive(Debug, Clone, Copy)]
enum CellTransform {
    ScrollUp {
        top: usize,
        bottom: usize,
        count: usize,
        left: u16,
        right: u16,
    },
    ScrollDown {
        top: usize,
        bottom: usize,
        count: usize,
        left: u16,
        right: u16,
    },
    InsertCharacters {
        row: usize,
        column: u16,
        count: u16,
        right: u16,
    },
    DeleteCharacters {
        row: usize,
        column: u16,
        count: u16,
        right: u16,
    },
}

impl CellTransform {
    const fn is_character_edit(self) -> bool {
        matches!(
            self,
            Self::InsertCharacters { .. } | Self::DeleteCharacters { .. }
        )
    }

    fn apply_coordinate(self, row: usize, column: u16) -> Option<(usize, u16)> {
        let (top, bottom, left, right) = match self {
            Self::ScrollUp {
                top,
                bottom,
                left,
                right,
                ..
            }
            | Self::ScrollDown {
                top,
                bottom,
                left,
                right,
                ..
            } => (top, bottom, left, right),
            Self::InsertCharacters {
                row, column, right, ..
            }
            | Self::DeleteCharacters {
                row, column, right, ..
            } => (row, row, column, right),
        };
        if row < top || row > bottom || column < left || column > right {
            return Some((row, column));
        }

        match self {
            Self::ScrollUp { top, count, .. } => {
                let row = row.checked_sub(count).filter(|row| *row >= top)?;
                Some((row, column))
            }
            Self::ScrollDown { bottom, count, .. } => {
                let blank_start = bottom.checked_add(1)?.checked_sub(count)?;
                if row >= blank_start {
                    return None;
                }
                Some((row.checked_add(count)?, column))
            }
            Self::InsertCharacters { count, right, .. } => {
                let shift_end = right.checked_add(1)?.checked_sub(count)?;
                if column >= shift_end {
                    return None;
                }
                Some((row, column.checked_add(count)?))
            }
            Self::DeleteCharacters {
                column: start_column,
                count,
                ..
            } => {
                let first_source = start_column.checked_add(count)?;
                if column < first_source {
                    return None;
                }
                Some((row, column.checked_sub(count)?))
            }
        }
    }

    fn apply(self, mut attachment: CellAttachment) -> Option<CellAttachment> {
        (attachment.row, attachment.column) =
            self.apply_coordinate(attachment.row, attachment.column)?;
        Some(attachment)
    }
}

#[derive(Debug, Clone)]
struct SavedCursor {
    row: u16,
    column: u16,
    pending_wrap: bool,
    clear_semantic_type_on_movement: bool,
    origin_mode: bool,
    character_set: CharacterSet,
    style: Cell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeedAdvance {
    Next(usize),
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SequenceParse<T> {
    Complete(T),
    Cancelled(usize),
    Pending,
}

impl Terminal {
    #[must_use]
    pub fn new(size: TerminalSize) -> Self {
        Self::new_with_default_cursor_style(size, CursorStyle::default())
    }

    #[must_use]
    pub fn new_with_default_cursor_style(
        size: TerminalSize,
        default_cursor_style: CursorStyle,
    ) -> Self {
        let terminal = Self {
            grid: TerminalGrid::new_with_seqno(size, INITIAL_SEQUENCE_NO),
            scrollback: HistoryBuffer::new(),
            scrollback_limit: DEFAULT_SCROLLBACK_LIMIT,
            seqno: INITIAL_SEQUENCE_NO,
            screen_identity_generation: 0,
            main_stable_row_offset: 0,
            title: None,
            icon_title: None,
            window_title: None,
            title_stack: Vec::new(),
            current_working_dir: None,
            badge_format: None,
            user_vars: HashMap::new(),
            pending_metadata_changes: PendingTerminalMetadataChanges::default(),
            inline_images: Vec::new(),
            inline_image_parent_ids: Vec::new(),
            inline_image_attachments: Vec::new(),
            next_inline_image_parent_identity: 1,
            kitty_graphics_responses: Vec::new(),
            pending_kitty_graphics: None,
            kitty_images: HashMap::new(),
            kitty_image_numbers: HashMap::new(),
            kitty_relative_parents: HashMap::new(),
            kitty_virtual_placements: HashMap::new(),
            kitty_character_edited_placements: HashSet::new(),
            pending_kitty_placeholder: None,
            last_kitty_placeholder: None,
            kitty_placeholder_cells: HashMap::new(),
            next_kitty_image_id: 1,
            enable_kitty_graphics: true,
            semantic_prompt_rows: Vec::new(),
            semantic_command_exits: Vec::new(),
            cursor_row: 0,
            cursor_column: 0,
            pending_wrap: false,
            clear_semantic_type_on_movement: false,
            pending_utf8: Vec::new(),
            pending_control: Vec::new(),
            feed_scratch: TerminalFeedScratch::default(),
            last_printable: None,
            nfc_last_printable_cell: None,
            saved_cursor: None,
            main_screen: None,
            default_cursor_style,
            modes: TerminalModes::with_cursor_style(default_cursor_style),
            scroll_top: 0,
            scroll_bottom: size.rows.saturating_sub(1),
            left_margin: 0,
            right_margin: size.columns.saturating_sub(1),
            character_set: CharacterSet::Ascii,
            tab_stops: TabStops::new(size),
            style: Cell::default(),
            damage: Vec::new(),
            bell_count: 0,
            unknown_escape_sequences: Vec::new(),
            treat_east_asian_ambiguous_width_as_wide: false,
            cell_width_overrides: Vec::new(),
            normalize_output_to_unicode_nfc: false,
            unicode_version: DEFAULT_UNICODE_VERSION,
            unicode_version_stack: Vec::new(),
            work_counters: TerminalWorkCounters::default(),
            #[cfg(test)]
            fixture_trace: FixtureTraceIdentity::default(),
        };
        #[cfg(test)]
        let terminal = {
            let mut terminal = terminal;
            terminal.fixture_trace.id =
                task10_trace::trace_construct(&terminal, size, default_cursor_style, "new");
            terminal
        };
        terminal
    }

    #[must_use]
    pub const fn work_counters(&self) -> TerminalWorkCounters {
        self.work_counters
    }

    #[must_use]
    pub const fn feed_storage_counters(&self) -> TerminalFeedStorageCounters {
        self.feed_scratch.counters
    }

    pub const fn reset_feed_storage_counters(&mut self) {
        self.feed_scratch.counters = TerminalFeedStorageCounters {
            growths: 0,
            damage_growths: 0,
        };
    }

    pub fn set_unicode_version(&mut self, version: u32) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.unicode_version = version;
        self.unicode_version_stack.clear();
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.set_unicode_version",
            version.to_string().as_bytes(),
            b"result=unit",
        );
    }

    pub fn set_normalize_output_to_unicode_nfc(&mut self, enabled: bool) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.normalize_output_to_unicode_nfc = enabled;
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.set_normalize_nfc",
            if enabled { b"1" } else { b"0" },
            b"result=unit",
        );
    }

    pub fn set_treat_east_asian_ambiguous_width_as_wide(&mut self, enabled: bool) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.treat_east_asian_ambiguous_width_as_wide = enabled;
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.set_ambiguous_width",
            if enabled { b"1" } else { b"0" },
            b"result=unit",
        );
    }

    pub fn set_cell_width_overrides(&mut self, overrides: Vec<CellWidthOverride>) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        #[cfg(test)]
        let arguments = task10_trace::encode_width_overrides(&overrides);
        self.cell_width_overrides = overrides;
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.set_width_overrides",
            &arguments,
            b"result=unit",
        );
    }

    #[must_use]
    pub fn char_display_width(&self, ch: char) -> u16 {
        display_width(
            ch,
            self.unicode_version,
            self.treat_east_asian_ambiguous_width_as_wide,
            &self.cell_width_overrides,
        )
    }

    pub fn set_enable_kitty_graphics(&mut self, enabled: bool) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.enable_kitty_graphics = enabled;
        if !enabled {
            self.pending_kitty_graphics = None;
        }
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.set_kitty_graphics",
            if enabled { b"1" } else { b"0" },
            b"result=unit",
        );
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.advance_seqno();
        self.feed_at_current_seqno(bytes);
        #[cfg(test)]
        task10_trace::trace_action(self, trace, "terminal.feed", bytes, b"result=unit");
    }

    pub fn feed_with_all_lines_changed(&mut self, bytes: &[u8]) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.advance_seqno();
        self.feed_at_current_seqno(bytes);
        self.mark_all_lines_changed_at_current_seqno();
        #[cfg(test)]
        task10_trace::trace_action(self, trace, "terminal.feed_all", bytes, b"result=unit");
    }

    fn feed_at_current_seqno(&mut self, bytes: &[u8]) {
        if self.try_feed_direct_ascii_at_current_seqno(bytes) {
            return;
        }
        self.feed_decoded_at_current_seqno(bytes);
    }

    fn try_feed_direct_ascii_at_current_seqno(&mut self, bytes: &[u8]) -> bool {
        if self.normalize_output_to_unicode_nfc
            || !self.pending_utf8.is_empty()
            || !self.pending_control.is_empty()
            || !bytes.is_ascii()
            || bytes.contains(&0x1b)
        {
            return false;
        }

        for byte in bytes {
            if !self.consume_ascii_control(*byte) {
                self.write_char(char::from(*byte));
            }
        }
        true
    }

    fn feed_decoded_at_current_seqno(&mut self, bytes: &[u8]) {
        let mut scratch = std::mem::take(&mut self.feed_scratch);
        let input_capacity = scratch.input.capacity();
        let chars_capacity = scratch.chars.capacity();
        scratch.input.clear();
        scratch.input.extend_from_slice(&self.pending_utf8);
        self.pending_utf8.clear();
        scratch.input.extend_from_slice(bytes);
        let complete_utf8_len = complete_utf8_prefix_len(&scratch.input);
        self.pending_utf8
            .extend_from_slice(&scratch.input[complete_utf8_len..]);

        scratch.chars.clear();
        scratch.chars.append(&mut self.pending_control);
        decode_terminal_chars_into(&scratch.input[..complete_utf8_len], &mut scratch.chars);
        let mut index = 0;

        while index < scratch.chars.len() {
            if let Some(advance) = self.consume_escape_or_c1_sequence(&scratch.chars, index) {
                match advance {
                    FeedAdvance::Next(next_index) => index = next_index,
                    FeedAdvance::Pending => break,
                }
            } else {
                index = self.consume_text_run_or_ascii_control(&scratch.chars, index);
            }
        }
        scratch.input.clear();
        scratch.chars.clear();
        if scratch.input.capacity() != input_capacity || scratch.chars.capacity() != chars_capacity
        {
            scratch.counters.growths = scratch.counters.growths.saturating_add(1);
        }
        scratch.counters.damage_growths = scratch
            .counters
            .damage_growths
            .saturating_add(self.feed_scratch.counters.damage_growths);
        self.feed_scratch = scratch;
    }

    pub fn mark_all_lines_changed(&mut self) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.advance_seqno();
        self.mark_all_lines_changed_at_current_seqno();
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.mark_all_lines_changed",
            b"",
            b"result=unit",
        );
    }

    fn mark_all_lines_changed_at_current_seqno(&mut self) {
        if !self.alternate_screen_active() {
            for line in &mut self.scrollback {
                line.set_last_change_seqno(self.seqno);
            }
        }
        for row in 0..self.grid.size().rows {
            self.grid.set_row_last_change_seqno(row, self.seqno);
        }
    }

    fn consume_escape_or_c1_sequence(
        &mut self,
        chars: &[char],
        index: usize,
    ) -> Option<FeedAdvance> {
        if is_escape_or_c1_sequence_start(chars[index]) {
            self.finish_pending_kitty_placeholder();
            self.nfc_last_printable_cell = None;
        }
        match chars[index] {
            '\u{1b}' => Some(self.consume_escape_sequence(chars, index)),
            '\u{9b}' => Some(next_or_pending(self.apply_csi_sequence(chars, index, 1))),
            '\u{9c}' => Some(FeedAdvance::Next(index + 1)),
            '\u{90}' => Some(next_or_pending(self.apply_dcs_sequence(chars, index, 1))),
            '\u{9d}' => Some(next_or_pending(self.skip_c1_osc(chars, index))),
            ch if is_c1_st_control_string(ch) => Some(next_or_pending(
                self.skip_c1_st_control_string(chars, index),
            )),
            '\u{84}' => {
                self.index_down_control(false);
                Some(FeedAdvance::Next(index + 1))
            }
            '\u{85}' => {
                self.next_line();
                Some(FeedAdvance::Next(index + 1))
            }
            '\u{88}' => {
                self.set_horizontal_tab_stop();
                Some(FeedAdvance::Next(index + 1))
            }
            '\u{8d}' => {
                self.reverse_index();
                Some(FeedAdvance::Next(index + 1))
            }
            _ => None,
        }
    }

    fn consume_escape_sequence(&mut self, chars: &[char], index: usize) -> FeedAdvance {
        match chars.get(index + 1).copied() {
            Some('[') => next_or_pending(self.apply_csi_sequence(chars, index, 2)),
            Some(']') => next_or_pending(self.skip_osc(chars, index)),
            Some('_') => next_or_pending(self.apply_apc_sequence(chars, index, 2)),
            Some('P') => next_or_pending(self.apply_dcs_sequence(chars, index, 2)),
            Some('\\' | '=' | '>') => FeedAdvance::Next(index + 2),
            Some('X' | '^') => next_or_pending(self.skip_st_control_string(chars, index)),
            Some('(') => self.consume_g0_character_set_selection(chars, index),
            Some('#') => self.consume_hash_escape_sequence(chars, index),
            Some('7') => {
                self.save_cursor();
                FeedAdvance::Next(index + 2)
            }
            Some('8') => {
                self.restore_cursor();
                FeedAdvance::Next(index + 2)
            }
            Some('H') => {
                self.set_horizontal_tab_stop();
                FeedAdvance::Next(index + 2)
            }
            Some('c') => {
                self.reset_terminal();
                FeedAdvance::Next(index + 2)
            }
            Some('D') => {
                self.index_down_control(false);
                FeedAdvance::Next(index + 2)
            }
            Some('E') => {
                self.next_line();
                FeedAdvance::Next(index + 2)
            }
            Some('M') => {
                self.reverse_index();
                FeedAdvance::Next(index + 2)
            }
            None => {
                self.pending_control.extend_from_slice(&chars[index..]);
                FeedAdvance::Pending
            }
            Some(command) => {
                self.record_unknown_escape_sequence(format!("ESC {command}"));
                FeedAdvance::Next(index + 2)
            }
        }
    }

    fn consume_hash_escape_sequence(&mut self, chars: &[char], index: usize) -> FeedAdvance {
        match chars.get(index + 2).copied() {
            Some('8') => {
                self.screen_alignment_test();
                FeedAdvance::Next(index + 3)
            }
            Some(_) => FeedAdvance::Next(index + 3),
            None => {
                self.pending_control.extend_from_slice(&chars[index..]);
                FeedAdvance::Pending
            }
        }
    }

    fn consume_g0_character_set_selection(&mut self, chars: &[char], index: usize) -> FeedAdvance {
        if let Some(selector) = chars.get(index + 2).copied() {
            if let Some(character_set) = parse_g0_character_set(selector) {
                self.character_set = character_set;
            }

            FeedAdvance::Next(index + 3)
        } else {
            self.pending_control.extend_from_slice(&chars[index..]);
            FeedAdvance::Pending
        }
    }

    fn consume_text_run_or_ascii_control(&mut self, chars: &[char], index: usize) -> usize {
        let ch = chars[index];
        if ch.is_ascii() && self.consume_ascii_control(ch as u8) {
            return index + 1;
        }
        if !self.normalize_output_to_unicode_nfc {
            self.write_char(ch);
            return index + 1;
        }

        let mut end = index + 1;
        while end < chars.len()
            && !is_escape_or_c1_sequence_start(chars[end])
            && !is_ascii_control(chars[end])
        {
            end += 1;
        }
        let text = chars[index..end].iter().collect::<String>();
        if text.contains(KITTY_UNICODE_PLACEHOLDER) {
            for ch in text.chars() {
                self.write_char(ch);
            }
        } else {
            let remaining_start = self.normalize_leading_combining_marks(&text);
            for normalized in text[remaining_start..].nfc() {
                self.write_char(normalized);
            }
        }
        end
    }

    fn consume_ascii_control(&mut self, byte: u8) -> bool {
        match byte {
            0x00..=0x06 | 0x0e..=0x1a | 0x1c..=0x1f => {
                self.finish_pending_kitty_placeholder();
                self.nfc_last_printable_cell = None;
                true
            }
            0x07 => {
                self.finish_pending_kitty_placeholder();
                self.nfc_last_printable_cell = None;
                self.bell_count = self.bell_count.saturating_add(1);
                true
            }
            0x08 => {
                self.finish_pending_kitty_placeholder();
                self.nfc_last_printable_cell = None;
                self.backspace();
                true
            }
            0x09 => {
                self.finish_pending_kitty_placeholder();
                self.nfc_last_printable_cell = None;
                self.horizontal_tab();
                true
            }
            0x0a..=0x0c => {
                self.finish_pending_kitty_placeholder();
                self.nfc_last_printable_cell = None;
                self.line_feed();
                true
            }
            0x0d => {
                self.finish_pending_kitty_placeholder();
                self.nfc_last_printable_cell = None;
                self.carriage_return();
                true
            }
            _ => false,
        }
    }

    fn normalize_leading_combining_marks(&mut self, text: &str) -> usize {
        let leading_end = leading_combining_marks_end(text);
        if leading_end == 0 {
            return 0;
        }

        let Some((row, column)) = self.nfc_last_printable_cell else {
            return 0;
        };
        let Some(previous_cell) = self.grid.get(row, column).cloned() else {
            return 0;
        };
        if previous_cell.primary_char() == KITTY_UNICODE_PLACEHOLDER {
            return 0;
        }

        let normalized = format!("{}{}", previous_cell.text(), &text[..leading_end])
            .nfc()
            .collect::<String>();
        if normalized == previous_cell.text() {
            return 0;
        }

        let previous_width = grapheme_display_width(
            previous_cell.text(),
            self.unicode_version,
            self.treat_east_asian_ambiguous_width_as_wide,
            &self.cell_width_overrides,
        );
        let normalized_width = grapheme_display_width(
            &normalized,
            self.unicode_version,
            self.treat_east_asian_ambiguous_width_as_wide,
            &self.cell_width_overrides,
        );
        if previous_width == 0 || previous_width != normalized_width {
            return 0;
        }

        let mut normalized_cell = previous_cell;
        normalized_cell.set_text(normalized.clone(), saturating_u8(normalized_width));
        if self.set_grid_cell(row, column, normalized_cell) {
            self.record_damage(DamageRegion::new(column, row, previous_width, 1));
            self.last_printable = Some(normalized.into());
            return leading_end;
        }

        0
    }

    fn apply_csi_sequence(
        &mut self,
        chars: &[char],
        index: usize,
        content_offset: usize,
    ) -> Option<usize> {
        let content_start = index + content_offset;
        match parse_csi(chars, content_start) {
            SequenceParse::Complete((command, sequence_end)) => {
                self.apply_csi(command, &chars[content_start..sequence_end]);
                Some(sequence_end + 1)
            }
            SequenceParse::Cancelled(cancel_index) => Some(cancel_index + 1),
            SequenceParse::Pending => {
                self.pending_control.extend_from_slice(&chars[index..]);
                None
            }
        }
    }

    fn skip_osc(&mut self, chars: &[char], index: usize) -> Option<usize> {
        self.apply_osc_sequence(chars, index, 2)
    }

    fn skip_c1_osc(&mut self, chars: &[char], index: usize) -> Option<usize> {
        self.apply_osc_sequence(chars, index, 1)
    }

    fn skip_st_control_string(&mut self, chars: &[char], index: usize) -> Option<usize> {
        self.skip_control_string(chars, index, 2, parse_st_terminated_control_string)
    }

    fn skip_c1_st_control_string(&mut self, chars: &[char], index: usize) -> Option<usize> {
        self.skip_control_string(chars, index, 1, parse_st_terminated_control_string)
    }

    fn apply_apc_sequence(
        &mut self,
        chars: &[char],
        index: usize,
        content_offset: usize,
    ) -> Option<usize> {
        let content_start = index + content_offset;
        match parse_st_terminated_control_string(chars, content_start) {
            SequenceParse::Complete(sequence_end) => {
                let content_end = st_content_end(chars, content_start, sequence_end);
                self.apply_apc_content(&chars[content_start..content_end]);
                Some(sequence_end + 1)
            }
            SequenceParse::Cancelled(cancel_index) => Some(cancel_index + 1),
            SequenceParse::Pending => {
                self.pending_control.extend_from_slice(&chars[index..]);
                None
            }
        }
    }

    fn apply_dcs_sequence(
        &mut self,
        chars: &[char],
        index: usize,
        content_offset: usize,
    ) -> Option<usize> {
        let content_start = index + content_offset;
        match parse_st_terminated_control_string(chars, content_start) {
            SequenceParse::Complete(sequence_end) => {
                let content_end = st_content_end(chars, content_start, sequence_end);
                self.apply_dcs_content(&chars[content_start..content_end]);
                Some(sequence_end + 1)
            }
            SequenceParse::Cancelled(cancel_index) => Some(cancel_index + 1),
            SequenceParse::Pending => {
                self.pending_control.extend_from_slice(&chars[index..]);
                None
            }
        }
    }

    fn apply_osc_sequence(
        &mut self,
        chars: &[char],
        index: usize,
        content_offset: usize,
    ) -> Option<usize> {
        let content_start = index + content_offset;
        match parse_osc(chars, content_start) {
            SequenceParse::Complete(sequence_end) => {
                let content_end = osc_content_end(chars, content_start, sequence_end);
                self.apply_osc_content(&chars[content_start..content_end]);
                Some(sequence_end + 1)
            }
            SequenceParse::Cancelled(cancel_index) => Some(cancel_index + 1),
            SequenceParse::Pending => {
                self.pending_control.extend_from_slice(&chars[index..]);
                None
            }
        }
    }

    fn apply_osc_content(&mut self, content: &[char]) {
        if let Some((&('L' | 'l'), title)) = content.split_first() {
            let title = title.iter().collect::<String>();
            if content.first() == Some(&'L') {
                self.set_icon_title(title);
            } else {
                self.set_window_title(title);
            }
            return;
        }

        let Some(separator) = content.iter().position(|ch| *ch == ';') else {
            return;
        };

        let command = content[..separator].iter().collect::<String>();
        match command.as_str() {
            "0" => self.set_icon_and_window_title(content[separator + 1..].iter().collect()),
            "1" => self.set_icon_title(content[separator + 1..].iter().collect()),
            "2" => self.set_window_title(content[separator + 1..].iter().collect()),
            "7" => {
                self.set_current_working_dir(content[separator + 1..].iter().collect());
            }
            "8" => self.apply_osc8_hyperlink(&content[separator + 1..]),
            "133" => self.apply_osc133_semantic_prompt(&content[separator + 1..]),
            "1337" => self.apply_osc1337_iterm_metadata(&content[separator + 1..]),
            _ => {}
        }
    }

    fn set_icon_and_window_title(&mut self, title: String) {
        if self.title.as_ref() != Some(&title) {
            self.pending_metadata_changes.title = true;
        }
        self.title = Some(title.clone());
        self.icon_title = Some(title.clone());
        self.window_title = Some(title);
    }

    fn set_icon_title(&mut self, title: String) {
        if self.title.as_ref() != Some(&title) {
            self.pending_metadata_changes.title = true;
        }
        self.title = Some(title.clone());
        self.icon_title = Some(title);
    }

    fn set_window_title(&mut self, title: String) {
        if self.title.as_ref() != Some(&title) {
            self.pending_metadata_changes.title = true;
        }
        self.title = Some(title.clone());
        self.window_title = Some(title);
    }

    fn set_current_working_dir(&mut self, current_working_dir: String) {
        if self.current_working_dir.as_ref() == Some(&current_working_dir) {
            return;
        }
        self.current_working_dir = Some(current_working_dir);
        self.pending_metadata_changes.current_working_dir = true;
    }

    fn apply_apc_content(&mut self, content: &[char]) {
        let content = content.iter().collect::<String>();
        if let Some(graphics) = content.strip_prefix('G') {
            self.apply_kitty_graphics(graphics);
        }
    }

    fn apply_dcs_content(&mut self, content: &[char]) {
        let content = content.iter().collect::<String>();
        if let Some(sixel_start) = sixel_dcs_marker_index(&content) {
            let options = parse_sixel_dcs_options(&content[..sixel_start]);
            self.apply_sixel_content(options, &content[sixel_start + 1..]);
        }
    }

    fn apply_osc1337_iterm_metadata(&mut self, content: &[char]) {
        let content = content.iter().collect::<String>();
        if let Some(current_dir) = content.strip_prefix("CurrentDir=") {
            self.set_current_working_dir(current_dir.to_owned());
            return;
        }

        if let Some(user_var) = content.strip_prefix("SetUserVar=") {
            self.apply_osc1337_set_user_var(user_var);
            return;
        }

        if let Some(encoded_badge_format) = content.strip_prefix("SetBadgeFormat=") {
            self.apply_osc1337_set_badge_format(encoded_badge_format);
            return;
        }

        if let Some(file) = content.strip_prefix("File=") {
            self.apply_osc1337_file(file);
            return;
        }

        if let Some(unicode_version) = content.strip_prefix("UnicodeVersion=") {
            self.apply_osc1337_unicode_version(unicode_version);
        }
    }

    fn apply_osc1337_unicode_version(&mut self, value: &str) {
        let trimmed = value.trim();
        if let Some(label) = trimmed.strip_prefix("push") {
            self.unicode_version_stack.push(UnicodeVersionStackEntry {
                version: self.unicode_version,
                label: non_empty_unicode_version_label(label),
            });
            return;
        }

        if let Some(label) = trimmed.strip_prefix("pop") {
            match non_empty_unicode_version_label(label) {
                Some(label) => self.pop_labeled_unicode_version(&label),
                None => {
                    if let Some(entry) = self.unicode_version_stack.pop() {
                        self.unicode_version = entry.version;
                    }
                }
            }
            return;
        }

        if let Ok(version) = trimmed.parse::<u32>() {
            self.unicode_version = version;
        }
    }

    fn pop_labeled_unicode_version(&mut self, label: &str) {
        let Some(index) = self
            .unicode_version_stack
            .iter()
            .rposition(|entry| entry.label.as_deref() == Some(label))
        else {
            return;
        };
        let entry = self.unicode_version_stack[index].clone();
        self.unicode_version_stack.truncate(index);
        self.unicode_version = entry.version;
    }

    fn apply_osc1337_set_user_var(&mut self, user_var: &str) {
        let Some((name, encoded_value)) = user_var.split_once('=') else {
            return;
        };
        let Ok(decoded_value) = STANDARD.decode(encoded_value) else {
            return;
        };
        let Ok(value) = String::from_utf8(decoded_value) else {
            return;
        };

        if self.user_vars.get(name) == Some(&value) {
            return;
        }
        self.user_vars.insert(name.to_owned(), value);
        if !self
            .pending_metadata_changes
            .user_vars
            .iter()
            .any(|changed| changed == name)
        {
            self.pending_metadata_changes
                .user_vars
                .push(name.to_owned());
        }
    }

    fn apply_osc1337_set_badge_format(&mut self, encoded_badge_format: &str) {
        let Ok(decoded_value) = STANDARD.decode(encoded_badge_format) else {
            return;
        };
        let Ok(value) = String::from_utf8(decoded_value) else {
            return;
        };

        if self.badge_format.as_ref() == Some(&value) {
            return;
        }
        self.badge_format = Some(value);
        self.pending_metadata_changes.badge_format = true;
    }

    fn apply_osc1337_file(&mut self, file: &str) {
        let Some((params, encoded_data)) = file.split_once(':') else {
            return;
        };

        let mut inline = false;
        let mut name = None;
        let mut size = None;
        let mut width = None;
        let mut height = None;
        let mut preserve_aspect_ratio = None;

        for param in params.split(';').filter(|param| !param.is_empty()) {
            let Some((key, value)) = param.split_once('=') else {
                continue;
            };

            match key {
                "inline" => inline = value == "1",
                "name" => name = decode_base64_utf8(value),
                "size" => size = value.parse().ok(),
                "width" => width = Some(value.to_owned()),
                "height" => height = Some(value.to_owned()),
                "preserveAspectRatio" => {
                    preserve_aspect_ratio = match value {
                        "0" => Some(false),
                        "1" => Some(true),
                        _ => None,
                    };
                }
                _ => {}
            }
        }

        if !inline {
            return;
        }

        let Ok(data) = STANDARD.decode(encoded_data) else {
            return;
        };

        let row = self.current_history_row();
        let column = self.cursor_column;
        self.record_inline_image_damage(width.as_deref(), height.as_deref());
        self.push_inline_image(ItermInlineImage {
            row,
            column,
            name,
            kitty_image_id: None,
            kitty_placement_id: None,
            kitty_z_index: None,
            size,
            width,
            height,
            preserve_aspect_ratio,
            image_format: InlineImageFormat::Encoded,
            pixel_width: None,
            pixel_height: None,
            source_x: None,
            source_y: None,
            source_width: None,
            source_height: None,
            target_x: None,
            target_y: None,
            data,
        });
    }

    fn apply_kitty_graphics(&mut self, graphics: &str) {
        if !self.enable_kitty_graphics {
            self.pending_kitty_graphics = None;
            return;
        }

        let (control, encoded_data) = graphics.split_once(';').unwrap_or((graphics, ""));
        let params = parse_kitty_graphics_params(control);
        if params.image_id.is_some() && params.image_number.is_some() {
            self.push_kitty_graphics_error_response(
                params,
                "EINVAL",
                "Image id and image number are mutually exclusive",
            );
            return;
        }

        if params.action == Some('q') && params.more_chunks.is_none() {
            self.apply_kitty_graphics_query(params, encoded_data);
            return;
        }
        if params.action == Some('p') {
            self.apply_kitty_graphics_placement(params);
            return;
        }
        if params.action == Some('d') {
            self.apply_kitty_graphics_delete(params);
            return;
        }

        match (self.pending_kitty_graphics.take(), params.more_chunks) {
            (Some(mut pending), Some(0)) => {
                pending.encoded_data.push_str(encoded_data);
                self.finish_kitty_graphics_upload(&pending);
            }
            (Some(mut pending), Some(1)) => {
                pending.encoded_data.push_str(encoded_data);
                self.pending_kitty_graphics = Some(pending);
            }
            (None, Some(1)) => match start_kitty_graphics_upload(params, encoded_data) {
                Ok(pending) => self.pending_kitty_graphics = Some(pending),
                Err(error) => self.push_kitty_graphics_upload_start_error(params, error),
            },
            (None, Some(0) | None) => match start_kitty_graphics_upload(params, encoded_data) {
                Ok(pending) => self.finish_kitty_graphics_upload(&pending),
                Err(error) => self.push_kitty_graphics_upload_start_error(params, error),
            },
            (Some(_), _) | (None, Some(_)) => {}
        }
    }

    fn apply_kitty_graphics_query(&mut self, params: KittyGraphicsParams, encoded_data: &str) {
        if encoded_data.is_empty() && self.apply_kitty_graphics_stored_image_query(params) {
            return;
        }

        let upload = match start_kitty_graphics_upload(params, encoded_data) {
            Ok(upload) => upload,
            Err(error) => {
                self.push_kitty_graphics_upload_start_error(params, error);
                return;
            }
        };

        let data = match load_kitty_graphics_payload(&upload) {
            Ok(data) => data,
            Err(KittyGraphicsDataError::InvalidBase64) => {
                self.push_kitty_graphics_error_response(params, "EINVAL", "Invalid base64 payload");
                return;
            }
            Err(KittyGraphicsDataError::InvalidFile) => {
                self.push_kitty_graphics_error_response(params, "EINVAL", "Invalid file payload");
                return;
            }
            Err(KittyGraphicsDataError::UnsupportedCompression) => {
                self.push_kitty_graphics_error_response(
                    params,
                    "EINVAL",
                    "Unsupported compression",
                );
                return;
            }
        };
        if !kitty_graphics_payload_is_supported(
            upload.image_format,
            upload.pixel_width,
            upload.pixel_height,
            data.len(),
        ) {
            self.push_kitty_graphics_error_response(params, "EINVAL", "Unsupported image data");
            return;
        }

        self.push_kitty_graphics_ok_response(params);
    }

    fn apply_kitty_graphics_stored_image_query(&mut self, params: KittyGraphicsParams) -> bool {
        if params.image_id.is_none() && params.image_number.is_none() {
            return false;
        }

        let Some(image_id) = self.kitty_image_id_from_params(params) else {
            if let Some(image_number) = params.image_number {
                self.push_kitty_graphics_error_response(
                    params,
                    "ENOENT",
                    &format!("No image with number {image_number}"),
                );
            }
            return true;
        };

        let params = Self::kitty_response_params_with_image_id(params, image_id);
        if self.kitty_images.contains_key(&image_id) {
            self.push_kitty_graphics_ok_response(params);
        } else {
            let subject = params.image_number.map_or_else(
                || format!("id {image_id}"),
                |image_number| format!("number {image_number}"),
            );
            self.push_kitty_graphics_error_response(
                params,
                "ENOENT",
                &format!("No image with {subject}"),
            );
        }
        true
    }

    fn finish_kitty_graphics_upload(&mut self, upload: &PendingKittyGraphics) {
        let data = match load_kitty_graphics_payload(upload) {
            Ok(data) => data,
            Err(error) => {
                if Self::kitty_upload_should_respond(upload) {
                    self.push_kitty_graphics_upload_error_response(upload, error);
                }
                return;
            }
        };

        if !kitty_graphics_payload_is_supported(
            upload.image_format,
            upload.pixel_width,
            upload.pixel_height,
            data.len(),
        ) {
            if Self::kitty_upload_should_respond(upload) {
                self.push_kitty_graphics_error_response(
                    Self::kitty_response_params_from_upload(upload),
                    "EINVAL",
                    "Unsupported image data",
                );
            }
            return;
        }

        if upload.action == KittyUploadAction::Query {
            self.push_kitty_graphics_ok_response(Self::kitty_response_params_from_upload(upload));
            return;
        }

        let image = StoredKittyImage {
            image_format: upload.image_format,
            pixel_width: upload.pixel_width,
            pixel_height: upload.pixel_height,
            display_columns: upload.display_columns,
            display_rows: upload.display_rows,
            data,
        };

        let image_id = upload
            .image_id
            .or_else(|| upload.image_number.map(|_| self.next_kitty_image_id()));
        if let Some(image_id) = upload.image_id {
            self.delete_kitty_placements(false, |image| image.kitty_image_id == Some(image_id));
        }
        if let Some(image_id) = image_id {
            self.kitty_images.insert(image_id, image.clone());
            if let Some(image_number) = upload.image_number {
                self.kitty_image_numbers.insert(image_number, image_id);
            }
            if upload.image_id.is_some() || upload.image_number.is_some() {
                self.push_kitty_graphics_upload_ok_response(upload, image_id);
            }
        }

        if upload.action == KittyUploadAction::Display {
            if upload.virtual_placement {
                if let Some(image_id) = image_id {
                    self.store_kitty_virtual_placement(
                        KittyGraphicsParams {
                            image_id: Some(image_id),
                            image_number: upload.image_number,
                            placement_id: upload.placement_id,
                            z_index: upload.z_index,
                            display_columns: upload.display_columns,
                            display_rows: upload.display_rows,
                            source_x: upload.source_x,
                            source_y: upload.source_y,
                            source_width: upload.source_width,
                            source_height: upload.source_height,
                            target_x: upload.target_x,
                            target_y: upload.target_y,
                            quiet: upload.quiet,
                            virtual_placement: true,
                            ..KittyGraphicsParams::default()
                        },
                        image_id,
                        false,
                    );
                }
                return;
            }
            self.place_kitty_image(
                &image,
                KittyPlacementOptions {
                    display_columns: upload.display_columns,
                    display_rows: upload.display_rows,
                    image_id: Some(image_id.unwrap_or(ANONYMOUS_KITTY_IMAGE_ID)),
                    placement_id: kitty_placement_id(image_id, upload.placement_id),
                    z_index: Some(upload.z_index.unwrap_or(0)),
                    parent_placement: None,
                    row: None,
                    column: None,
                    move_cursor: !upload.no_cursor_movement,
                    source_rect: upload.source_rect(),
                    target_x: upload.target_x,
                    target_y: upload.target_y,
                },
            );
        }
    }

    fn apply_kitty_graphics_placement(&mut self, params: KittyGraphicsParams) {
        let Some(image_id) = self.kitty_image_id_from_params(params) else {
            if let Some(image_number) = params.image_number {
                self.push_kitty_graphics_error_response(
                    params,
                    "ENOENT",
                    &format!("No image with number {image_number}"),
                );
            }
            return;
        };
        if params.parent_image_id.is_some() {
            self.apply_relative_kitty_graphics_placement(params, image_id);
            return;
        }
        if params.virtual_placement {
            self.apply_kitty_virtual_placement(params, image_id);
            return;
        }
        let Some(image) = self.kitty_images.get(&image_id).cloned() else {
            let subject = params.image_number.map_or_else(
                || format!("id {image_id}"),
                |image_number| format!("number {image_number}"),
            );
            self.push_kitty_graphics_error_response(
                Self::kitty_response_params_with_image_id(params, image_id),
                "ENOENT",
                &format!("No image with {subject}"),
            );
            return;
        };

        self.place_kitty_image(
            &image,
            KittyPlacementOptions {
                display_columns: params.display_columns,
                display_rows: params.display_rows,
                image_id: Some(image_id),
                placement_id: params.placement_id,
                z_index: Some(params.z_index.unwrap_or(0)),
                parent_placement: None,
                row: None,
                column: None,
                move_cursor: !params.no_cursor_movement,
                source_rect: params.source_rect(),
                target_x: params.target_x,
                target_y: params.target_y,
            },
        );
        self.push_kitty_graphics_ok_response(Self::kitty_response_params_with_image_id(
            params, image_id,
        ));
    }

    fn apply_kitty_virtual_placement(&mut self, params: KittyGraphicsParams, image_id: u32) {
        if !self.kitty_images.contains_key(&image_id) {
            let subject = params.image_number.map_or_else(
                || format!("id {image_id}"),
                |image_number| format!("number {image_number}"),
            );
            self.push_kitty_graphics_error_response(
                Self::kitty_response_params_with_image_id(params, image_id),
                "ENOENT",
                &format!("No image with {subject}"),
            );
            return;
        }

        self.store_kitty_virtual_placement(params, image_id, true);
    }

    fn store_kitty_virtual_placement(
        &mut self,
        params: KittyGraphicsParams,
        image_id: u32,
        respond: bool,
    ) {
        let placement = KittyVirtualPlacement {
            image_id,
            placement_id: params.placement_id,
            z_index: Some(params.z_index.unwrap_or(0)),
            display_columns: params.display_columns,
            display_rows: params.display_rows,
            source_rect: params.source_rect(),
            target_x: params.target_x,
            target_y: params.target_y,
        };
        self.kitty_virtual_placements.insert(
            kitty_virtual_placement_key(image_id, params.placement_id),
            placement,
        );
        if respond {
            self.push_kitty_graphics_ok_response(Self::kitty_response_params_with_image_id(
                params, image_id,
            ));
        }
    }

    fn apply_relative_kitty_graphics_placement(
        &mut self,
        params: KittyGraphicsParams,
        image_id: u32,
    ) {
        if params.virtual_placement {
            self.push_kitty_graphics_error_response(
                Self::kitty_response_params_with_image_id(params, image_id),
                "EINVAL",
                "Virtual placements cannot be relative",
            );
            return;
        }
        let Some(parent_image_id) = params.parent_image_id else {
            return;
        };
        let Some(parent_placement_id) = params.parent_placement_id else {
            self.push_kitty_graphics_error_response(
                Self::kitty_response_params_with_image_id(params, image_id),
                "ENOPARENT",
                &format!("No parent placement with id {parent_image_id}"),
            );
            return;
        };
        let Some((parent_row, parent_column)) =
            self.kitty_relative_parent_origin(parent_image_id, parent_placement_id)
        else {
            self.push_kitty_graphics_error_response(
                Self::kitty_response_params_with_image_id(params, image_id),
                "ENOPARENT",
                &format!("No parent placement with id {parent_image_id},p={parent_placement_id}"),
            );
            return;
        };
        let row = offset_kitty_history_row(parent_row, params.parent_offset_rows.unwrap_or(0));
        let column = offset_kitty_column(parent_column, params.parent_offset_columns.unwrap_or(0));
        let child_placement = kitty_placement_key(Some(image_id), params.placement_id);
        if child_placement.is_some_and(|child_placement| {
            self.kitty_relative_placement_would_cycle(
                child_placement,
                (parent_image_id, parent_placement_id),
            )
        }) {
            self.push_kitty_graphics_error_response(
                Self::kitty_response_params_with_image_id(params, image_id),
                "ECYCLE",
                "Relative placement cycle",
            );
            return;
        }
        if self.kitty_relative_parent_depth((parent_image_id, parent_placement_id))
            >= MAX_KITTY_RELATIVE_CHAIN_DEPTH
        {
            self.push_kitty_graphics_error_response(
                Self::kitty_response_params_with_image_id(params, image_id),
                "ETOODEEP",
                "Relative placement chain too deep",
            );
            return;
        }
        let Some(image) = self.kitty_images.get(&image_id).cloned() else {
            let subject = params.image_number.map_or_else(
                || format!("id {image_id}"),
                |image_number| format!("number {image_number}"),
            );
            self.push_kitty_graphics_error_response(
                Self::kitty_response_params_with_image_id(params, image_id),
                "ENOENT",
                &format!("No image with {subject}"),
            );
            return;
        };
        self.place_kitty_image(
            &image,
            KittyPlacementOptions {
                display_columns: params.display_columns,
                display_rows: params.display_rows,
                image_id: Some(image_id),
                placement_id: params.placement_id,
                z_index: Some(params.z_index.unwrap_or(0)),
                parent_placement: Some((parent_image_id, parent_placement_id)),
                row: Some(row),
                column: Some(column),
                move_cursor: false,
                source_rect: params.source_rect(),
                target_x: params.target_x,
                target_y: params.target_y,
            },
        );
        self.push_kitty_graphics_ok_response(Self::kitty_response_params_with_image_id(
            params, image_id,
        ));
    }

    fn kitty_relative_parent_origin(
        &self,
        parent_image_id: u32,
        parent_placement_id: u32,
    ) -> Option<(usize, u16)> {
        let placement_key = (parent_image_id, parent_placement_id);
        if self.kitty_virtual_placements.contains_key(&placement_key) {
            let attachment_origin =
                || self.kitty_attachment_parent_origin(parent_image_id, parent_placement_id);
            let cache_origin =
                || self.kitty_virtual_parent_origin(parent_image_id, parent_placement_id);
            return if self
                .kitty_character_edited_placements
                .contains(&placement_key)
            {
                attachment_origin().or_else(cache_origin)
            } else {
                cache_origin().or_else(attachment_origin)
            };
        }

        self.kitty_attachment_parent_origin(parent_image_id, parent_placement_id)
            .or_else(|| {
                self.inline_images
                    .iter()
                    .find(|image| {
                        image.kitty_image_id == Some(parent_image_id)
                            && image.kitty_placement_id == Some(parent_placement_id)
                    })
                    .map(|image| (image.row, image.column))
            })
    }

    fn kitty_attachment_parent_origin(
        &self,
        parent_image_id: u32,
        parent_placement_id: u32,
    ) -> Option<(usize, u16)> {
        let parent_identity = self
            .inline_images
            .iter()
            .zip(self.inline_image_parent_ids.iter())
            .find_map(|(image, parent_identity)| {
                (image.kitty_image_id == Some(parent_image_id)
                    && image.kitty_placement_id == Some(parent_placement_id))
                .then_some(*parent_identity)
            })?;
        self.inline_image_attachments
            .iter()
            .filter(|attachment| attachment.parent_identity == parent_identity)
            .map(|attachment| (attachment.row, attachment.column))
            .min()
    }

    fn kitty_virtual_parent_origin(
        &self,
        parent_image_id: u32,
        parent_placement_id: u32,
    ) -> Option<(usize, u16)> {
        let mut min_row = None;
        let mut min_column = None;
        for (&(row, column), &placeholder) in &self.kitty_placeholder_cells {
            let Some((origin_row, origin_column, image_id, placement_id)) =
                self.kitty_placeholder_render_key(row, column, placeholder)
            else {
                continue;
            };
            if image_id == parent_image_id && placement_id == Some(parent_placement_id) {
                min_row = Some(min_row.map_or(origin_row, |row: usize| row.min(origin_row)));
                min_column =
                    Some(min_column.map_or(origin_column, |column: u16| column.min(origin_column)));
            }
        }

        Some((min_row?, min_column?))
    }

    fn kitty_relative_parent_depth(&self, parent: KittyPlacementKey) -> usize {
        let mut depth = 0;
        let mut seen = HashSet::new();
        let mut current = Some(parent);
        while let Some(placement) = current {
            if !seen.insert(placement) {
                return depth;
            }
            current = self.kitty_relative_parents.get(&placement).copied();
            if current.is_some() {
                depth += 1;
            }
        }
        depth
    }

    fn kitty_relative_placement_would_cycle(
        &self,
        child: KittyPlacementKey,
        parent: KittyPlacementKey,
    ) -> bool {
        let mut seen = HashSet::new();
        let mut current = Some(parent);
        while let Some(placement) = current {
            if placement == child {
                return true;
            }
            if !seen.insert(placement) {
                return false;
            }
            current = self.kitty_relative_parents.get(&placement).copied();
        }
        false
    }

    fn apply_kitty_graphics_delete(&mut self, params: KittyGraphicsParams) {
        self.pending_kitty_graphics = None;

        match params.delete_target.unwrap_or('a') {
            'a' | 'A' => {
                let (first_row, last_row) = self.visible_history_rows();
                let columns = self.grid.size().columns;
                self.delete_kitty_physical_placements(params.delete_target == Some('A'), |image| {
                    image.kitty_image_id.is_some()
                        && inline_image_intersects_region(image, first_row, last_row, 0, columns)
                });
            }
            'i' | 'I' => {
                let Some(image_id) = params.image_id else {
                    return;
                };
                self.delete_kitty_placements_by_image_id(
                    image_id,
                    params.placement_id,
                    params.delete_target == Some('I'),
                );
            }
            'n' | 'N' => {
                let Some(image_id) = params
                    .image_number
                    .and_then(|image_number| self.kitty_image_numbers.get(&image_number).copied())
                else {
                    return;
                };
                self.delete_kitty_placements_by_image_id(
                    image_id,
                    params.placement_id,
                    params.delete_target == Some('N'),
                );
            }
            'r' | 'R' => {
                let (Some(first_image_id), Some(last_image_id)) =
                    (params.cell_x.map(u32::from), params.cell_y.map(u32::from))
                else {
                    return;
                };
                self.delete_kitty_placements_by_image_id_range(
                    first_image_id,
                    last_image_id,
                    params.delete_target == Some('R'),
                );
            }
            'c' | 'C' => {
                let row = self.current_history_row();
                let column = self.cursor_column;
                self.delete_kitty_physical_placements(params.delete_target == Some('C'), |image| {
                    kitty_image_intersects_cell(image, row, column)
                });
            }
            'p' | 'P' => {
                let Some((row, column)) = self.kitty_delete_cell(params) else {
                    return;
                };
                self.delete_kitty_physical_placements(params.delete_target == Some('P'), |image| {
                    kitty_image_intersects_cell(image, row, column)
                });
            }
            'x' | 'X' => {
                let Some(column) = params.cell_x.and_then(zero_based_axis) else {
                    return;
                };
                let (first_row, last_row) = self.visible_history_rows();
                self.delete_kitty_physical_placements(params.delete_target == Some('X'), |image| {
                    kitty_image_intersects_column(image, first_row, last_row, column)
                });
            }
            'y' | 'Y' => {
                let Some(row) = params.cell_y.and_then(|row| self.visible_history_row(row)) else {
                    return;
                };
                self.delete_kitty_physical_placements(params.delete_target == Some('Y'), |image| {
                    kitty_image_intersects_row(image, row)
                });
            }
            'z' | 'Z' => {
                let Some(z_index) = params.z_index else {
                    return;
                };
                self.delete_kitty_physical_placements(params.delete_target == Some('Z'), |image| {
                    image.kitty_image_id.is_some() && image.kitty_z_index == Some(z_index)
                });
            }
            'q' | 'Q' => {
                let Some(z_index) = params.z_index else {
                    return;
                };
                let Some((row, column)) = self.kitty_delete_cell(params) else {
                    return;
                };
                self.delete_kitty_physical_placements(params.delete_target == Some('Q'), |image| {
                    image.kitty_z_index == Some(z_index)
                        && kitty_image_intersects_cell(image, row, column)
                });
            }
            _ => (),
        }
    }

    fn delete_kitty_placements_by_image_id(
        &mut self,
        image_id: u32,
        placement_id: Option<u32>,
        remove_unreferenced_data: bool,
    ) {
        self.delete_kitty_virtual_placements_by_image_id(image_id, placement_id);
        if let Some(placement_id) = placement_id {
            self.delete_kitty_placements(false, |image| {
                image.kitty_image_id == Some(image_id)
                    && image.kitty_placement_id == Some(placement_id)
            });
        } else {
            self.delete_kitty_placements(false, |image| image.kitty_image_id == Some(image_id));
        }
        if remove_unreferenced_data {
            self.remove_unreferenced_kitty_images(vec![image_id]);
        }
    }

    fn delete_kitty_virtual_placements_by_image_id(
        &mut self,
        image_id: u32,
        placement_id: Option<u32>,
    ) {
        let removed_placement_keys = self
            .kitty_virtual_placements
            .iter()
            .filter_map(|(placement_key, placement)| {
                (placement.image_id == image_id
                    && placement_id
                        .is_none_or(|placement_id| placement.placement_id == Some(placement_id)))
                .then_some(*placement_key)
            })
            .collect::<HashSet<_>>();
        self.kitty_virtual_placements.retain(|_, placement| {
            placement.image_id != image_id
                || placement_id
                    .is_some_and(|placement_id| placement.placement_id != Some(placement_id))
        });
        self.discard_kitty_character_edited_placements(&removed_placement_keys);
        self.retain_kitty_character_edited_placements();
    }

    fn delete_kitty_placements_by_image_id_range(
        &mut self,
        first_image_id: u32,
        last_image_id: u32,
        remove_unreferenced_data: bool,
    ) {
        if first_image_id > last_image_id {
            return;
        }
        let removed_image_ids = self
            .kitty_images
            .keys()
            .copied()
            .filter(|image_id| *image_id >= first_image_id && *image_id <= last_image_id)
            .collect::<Vec<_>>();
        let removed_placement_keys = self
            .kitty_virtual_placements
            .iter()
            .filter_map(|(placement_key, placement)| {
                (placement.image_id >= first_image_id && placement.image_id <= last_image_id)
                    .then_some(*placement_key)
            })
            .collect::<HashSet<_>>();
        self.kitty_virtual_placements.retain(|_, placement| {
            placement.image_id < first_image_id || placement.image_id > last_image_id
        });
        self.discard_kitty_character_edited_placements(&removed_placement_keys);
        self.retain_kitty_character_edited_placements();
        self.delete_kitty_placements(false, |image| {
            image
                .kitty_image_id
                .is_some_and(|image_id| image_id >= first_image_id && image_id <= last_image_id)
        });
        if remove_unreferenced_data {
            self.remove_unreferenced_kitty_images(removed_image_ids);
        }
    }

    fn delete_kitty_physical_placements(
        &mut self,
        remove_unreferenced_data: bool,
        mut matches: impl FnMut(&ItermInlineImage) -> bool,
    ) {
        let placeholder_render_keys = self.kitty_placeholder_render_keys();
        self.delete_kitty_placements(remove_unreferenced_data, |image| {
            !kitty_image_matches_placeholder_render(image, &placeholder_render_keys)
                && matches(image)
        });
    }

    fn apply_sixel_content(&mut self, options: SixelOptions, content: &str) {
        let Some(image) = parse_sixel_image(options, content) else {
            return;
        };

        let width = format!("{}px", image.width);
        let height = format!("{}px", image.height);
        let cell_width = inline_image_width_cells(Some(&width));
        let (row, column) = if self.modes.sixel_display_mode {
            (self.visible_history_rows().0, 0)
        } else {
            (self.current_history_row(), self.cursor_column)
        };
        self.record_inline_image_damage_at(row, column, Some(&width), Some(&height));
        self.push_inline_image(ItermInlineImage {
            row,
            column,
            name: None,
            kitty_image_id: None,
            kitty_placement_id: None,
            kitty_z_index: None,
            size: Some(image.data.len()),
            width: Some(width),
            height: Some(height),
            preserve_aspect_ratio: None,
            image_format: InlineImageFormat::Rgba,
            pixel_width: Some(image.width),
            pixel_height: Some(image.height),
            source_x: None,
            source_y: None,
            source_width: None,
            source_height: None,
            target_x: None,
            target_y: None,
            data: image.data,
        });
        if !self.modes.sixel_display_mode {
            self.index_down(false);
            if self.modes.sixel_scrolls_right {
                self.move_cursor_forward(cell_width);
            }
        }
    }

    fn push_kitty_graphics_ok_response(&mut self, params: KittyGraphicsParams) {
        self.push_kitty_graphics_response(params, "OK");
    }

    fn push_kitty_graphics_error_response(
        &mut self,
        params: KittyGraphicsParams,
        code: &str,
        message: &str,
    ) {
        self.push_kitty_graphics_response(params, &format!("{code}:{message}"));
    }

    fn push_kitty_graphics_response(&mut self, params: KittyGraphicsParams, status: &str) {
        if kitty_graphics_response_is_suppressed(params, status) {
            return;
        }

        let mut response = b"\x1b_G".to_vec();
        let mut has_param = false;
        if let Some(image_id) = params.image_id {
            append_kitty_graphics_response_param(&mut response, &mut has_param, b'i', image_id);
        }
        if let Some(image_number) = params.image_number {
            append_kitty_graphics_response_param(&mut response, &mut has_param, b'I', image_number);
        }
        if let Some(placement_id) = params.placement_id {
            append_kitty_graphics_response_param(&mut response, &mut has_param, b'p', placement_id);
        }
        response.push(b';');
        response.extend(
            status
                .bytes()
                .filter(|byte| !matches!(byte, 0x00..=0x1f | 0x7f)),
        );
        response.extend_from_slice(b"\x1b\\");
        self.kitty_graphics_responses.push(response);
    }

    fn delete_kitty_placements(
        &mut self,
        remove_unreferenced_data: bool,
        mut matches: impl FnMut(&ItermInlineImage) -> bool,
    ) {
        let before = self.inline_images.len();
        let mut removed_image_ids = Vec::new();
        let mut relative_removed_image_ids = Vec::new();
        let mut removed_placement_keys = HashSet::new();
        self.retain_inline_images(|image| {
            let remove = matches(image);
            if remove {
                if let Some(image_id) = image.kitty_image_id {
                    removed_image_ids.push(image_id);
                }
                if let Some(placement_key) = kitty_image_placement_key(image) {
                    removed_placement_keys.insert(placement_key);
                }
            }
            !remove
        });
        self.delete_relative_kitty_children(
            &mut removed_image_ids,
            &mut relative_removed_image_ids,
            &mut removed_placement_keys,
        );
        if self.inline_images.len() != before {
            if remove_unreferenced_data {
                self.remove_unreferenced_kitty_images(removed_image_ids);
            }
            self.remove_unreferenced_kitty_images(relative_removed_image_ids);
            let size = self.grid.size();
            self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
        }
        self.delete_orphan_kitty_relative_children();
    }

    fn delete_relative_kitty_children(
        &mut self,
        removed_image_ids: &mut Vec<u32>,
        relative_removed_image_ids: &mut Vec<u32>,
        removed_placement_keys: &mut HashSet<KittyPlacementKey>,
    ) {
        loop {
            let relative_parents = self.kitty_relative_parents.clone();
            let parent_keys = removed_placement_keys.clone();
            let mut removed_this_pass = Vec::new();
            self.retain_inline_images(|image| {
                let Some(placement_key) = kitty_image_placement_key(image) else {
                    return true;
                };
                let remove = relative_parents
                    .get(&placement_key)
                    .is_some_and(|parent_key| parent_keys.contains(parent_key));
                if remove {
                    removed_this_pass.push(placement_key);
                    if let Some(image_id) = image.kitty_image_id {
                        removed_image_ids.push(image_id);
                        relative_removed_image_ids.push(image_id);
                    }
                }
                !remove
            });
            if removed_this_pass.is_empty() {
                return;
            }
            removed_placement_keys.extend(removed_this_pass);
        }
    }

    fn delete_orphan_kitty_relative_children(&mut self) {
        self.remove_orphan_kitty_relative_children(true);
    }

    fn retire_orphan_kitty_relative_children(&mut self) {
        self.remove_orphan_kitty_relative_children(false);
    }

    fn remove_orphan_kitty_relative_children(&mut self, remove_unreferenced_stored_images: bool) {
        loop {
            let live_placements = self
                .inline_images
                .iter()
                .filter_map(kitty_image_placement_key)
                .collect::<HashSet<_>>();
            let orphan_keys = self
                .kitty_relative_parents
                .iter()
                .filter_map(|(child, parent)| {
                    (!live_placements.contains(parent) || !live_placements.contains(child))
                        .then_some(*child)
                })
                .collect::<HashSet<_>>();
            if orphan_keys.is_empty() {
                self.kitty_relative_parents.retain(|child, parent| {
                    live_placements.contains(child) && live_placements.contains(parent)
                });
                return;
            }

            let mut removed_image_ids = Vec::new();
            for orphan_key in &orphan_keys {
                self.kitty_relative_parents.remove(orphan_key);
            }
            self.retain_inline_images(|image| {
                let remove = kitty_image_placement_key(image)
                    .is_some_and(|placement_key| orphan_keys.contains(&placement_key));
                if remove && let Some(image_id) = image.kitty_image_id {
                    removed_image_ids.push(image_id);
                }
                !remove
            });
            if remove_unreferenced_stored_images {
                self.remove_unreferenced_kitty_images(removed_image_ids);
            }
        }
    }

    fn remove_unreferenced_kitty_images(&mut self, image_ids: Vec<u32>) {
        for image_id in image_ids {
            if !self
                .inline_images
                .iter()
                .any(|image| image.kitty_image_id == Some(image_id))
                && !self
                    .kitty_virtual_placements
                    .values()
                    .any(|placement| placement.image_id == image_id)
            {
                self.kitty_images.remove(&image_id);
                self.kitty_image_numbers
                    .retain(|_, mapped_image_id| *mapped_image_id != image_id);
            }
        }
    }

    fn kitty_image_id_from_params(&self, params: KittyGraphicsParams) -> Option<u32> {
        params.image_id.or_else(|| {
            params
                .image_number
                .and_then(|image_number| self.kitty_image_numbers.get(&image_number).copied())
        })
    }

    fn kitty_response_params_with_image_id(
        params: KittyGraphicsParams,
        image_id: u32,
    ) -> KittyGraphicsParams {
        KittyGraphicsParams {
            image_id: Some(image_id),
            ..params
        }
    }

    fn push_kitty_graphics_upload_ok_response(
        &mut self,
        upload: &PendingKittyGraphics,
        image_id: u32,
    ) {
        self.push_kitty_graphics_ok_response(KittyGraphicsParams {
            image_id: Some(image_id),
            image_number: upload.image_number,
            placement_id: upload.placement_id,
            quiet: upload.quiet,
            ..KittyGraphicsParams::default()
        });
    }

    fn push_kitty_graphics_upload_error_response(
        &mut self,
        upload: &PendingKittyGraphics,
        error: KittyGraphicsDataError,
    ) {
        match error {
            KittyGraphicsDataError::InvalidBase64 => self.push_kitty_graphics_error_response(
                Self::kitty_response_params_from_upload(upload),
                "EINVAL",
                "Invalid base64 payload",
            ),
            KittyGraphicsDataError::InvalidFile => self.push_kitty_graphics_error_response(
                Self::kitty_response_params_from_upload(upload),
                "EINVAL",
                "Invalid file payload",
            ),
            KittyGraphicsDataError::UnsupportedCompression => self
                .push_kitty_graphics_error_response(
                    Self::kitty_response_params_from_upload(upload),
                    "EINVAL",
                    "Unsupported compression",
                ),
        }
    }

    fn push_kitty_graphics_upload_start_error(
        &mut self,
        params: KittyGraphicsParams,
        error: KittyGraphicsStartError,
    ) {
        if !Self::kitty_params_should_respond(params) {
            return;
        }
        self.push_kitty_graphics_error_response(params, "EINVAL", error.message());
    }

    fn kitty_upload_should_respond(upload: &PendingKittyGraphics) -> bool {
        upload.action == KittyUploadAction::Query
            || Self::kitty_params_should_respond(Self::kitty_response_params_from_upload(upload))
    }

    fn kitty_params_should_respond(params: KittyGraphicsParams) -> bool {
        params.action == Some('q') || params.image_id.is_some() || params.image_number.is_some()
    }

    fn kitty_response_params_from_upload(upload: &PendingKittyGraphics) -> KittyGraphicsParams {
        KittyGraphicsParams {
            image_id: upload.image_id,
            image_number: upload.image_number,
            placement_id: upload.placement_id,
            quiet: upload.quiet,
            ..KittyGraphicsParams::default()
        }
    }

    fn next_kitty_image_id(&mut self) -> u32 {
        let mut image_id = self.next_kitty_image_id.max(1);
        while image_id == ANONYMOUS_KITTY_IMAGE_ID || self.kitty_images.contains_key(&image_id) {
            image_id = image_id.saturating_add(1).max(1);
        }
        self.next_kitty_image_id = image_id.saturating_add(1).max(1);
        image_id
    }

    fn kitty_delete_cell(&self, params: KittyGraphicsParams) -> Option<(usize, u16)> {
        let row = self.visible_history_row(params.cell_y?)?;
        let column = zero_based_axis(params.cell_x?)?;
        Some((row, column))
    }

    fn visible_history_row(&self, one_based_row: u16) -> Option<usize> {
        let row = zero_based_axis(one_based_row)?;
        (row < self.grid.size().rows)
            .then(|| self.scrollback.len().saturating_add(usize::from(row)))
    }

    fn visible_history_rows(&self) -> (usize, usize) {
        let first = self.scrollback.len();
        let last = first.saturating_add(usize::from(self.grid.size().rows));
        (first, last)
    }

    fn place_kitty_image(&mut self, image: &StoredKittyImage, options: KittyPlacementOptions) {
        let (width, height) = kitty_display_dimensions(image, options);
        let placement_columns = inline_image_width_cells(width.as_deref()).max(1);
        let placement_rows = inline_image_height_cells(height.as_deref()).max(1);
        let row = options.row.unwrap_or_else(|| self.current_history_row());
        let column = options.column.unwrap_or(self.cursor_column);

        let placement_key = kitty_placement_key(options.image_id, options.placement_id);
        let previous_origin = placement_key.and_then(|placement_key| {
            self.inline_images
                .iter()
                .find(|image| kitty_image_placement_key(image) == Some(placement_key))
                .map(|image| (image.row, image.column))
        });
        if let Some((image_id, placement_id)) = placement_key {
            let before = self.inline_images.len();
            self.retain_inline_images(|image| {
                image.kitty_image_id != Some(image_id)
                    || image.kitty_placement_id != Some(placement_id)
            });
            if self.inline_images.len() != before {
                let size = self.grid.size();
                self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
            }
            self.kitty_relative_parents
                .remove(&(image_id, placement_id));
        }

        self.record_inline_image_damage_at(row, column, width.as_deref(), height.as_deref());
        self.push_inline_image(ItermInlineImage {
            row,
            column,
            name: None,
            kitty_image_id: options.image_id,
            kitty_placement_id: options.placement_id,
            kitty_z_index: options.z_index,
            size: Some(image.data.len()),
            width,
            height,
            preserve_aspect_ratio: None,
            image_format: image.image_format,
            pixel_width: image.pixel_width,
            pixel_height: image.pixel_height,
            source_x: options.source_rect.x,
            source_y: options.source_rect.y,
            source_width: options.source_rect.width,
            source_height: options.source_rect.height,
            target_x: options.target_x,
            target_y: options.target_y,
            data: image.data.clone(),
        });
        if let Some(placement_key) = placement_key {
            if let Some(parent_placement) = options.parent_placement {
                self.kitty_relative_parents
                    .insert(placement_key, parent_placement);
            }
            if let Some((old_row, old_column)) = previous_origin {
                self.move_relative_kitty_descendants(
                    placement_key,
                    old_row,
                    old_column,
                    row,
                    column,
                );
            }
        }

        if options.move_cursor {
            self.move_kitty_cursor_after_placement(placement_columns, placement_rows);
        }
    }

    fn move_relative_kitty_descendants(
        &mut self,
        parent_key: KittyPlacementKey,
        old_row: usize,
        old_column: u16,
        new_row: usize,
        new_column: u16,
    ) {
        if old_row == new_row && old_column == new_column {
            return;
        }

        self.ensure_inline_image_parent_ids();
        let mut stack = vec![parent_key];
        let mut seen = HashSet::new();
        while let Some(parent_key) = stack.pop() {
            if !seen.insert(parent_key) {
                continue;
            }
            let child_keys = self
                .kitty_relative_parents
                .iter()
                .filter_map(|(child_key, mapped_parent)| {
                    (*mapped_parent == parent_key).then_some(*child_key)
                })
                .collect::<Vec<_>>();
            for child_key in child_keys {
                if let Some(image_index) = self.inline_images.iter().position(|image| {
                    image.kitty_image_id == Some(child_key.0)
                        && image.kitty_placement_id == Some(child_key.1)
                }) {
                    let parent_identity = self.inline_image_parent_ids[image_index];
                    let image = &mut self.inline_images[image_index];
                    image.row = move_kitty_history_row(image.row, old_row, new_row);
                    image.column = move_kitty_column(image.column, old_column, new_column);
                    for attachment in self
                        .inline_image_attachments
                        .iter_mut()
                        .filter(|attachment| attachment.parent_identity == parent_identity)
                    {
                        attachment.row = move_kitty_history_row(attachment.row, old_row, new_row);
                        attachment.column =
                            move_kitty_column(attachment.column, old_column, new_column);
                    }
                }
                stack.push(child_key);
            }
        }
    }

    fn move_kitty_cursor_after_placement(&mut self, columns: u16, rows: u16) {
        self.move_cursor_forward(columns);
        self.move_cursor_down(rows);
    }

    fn record_inline_image_damage(&mut self, width: Option<&str>, height: Option<&str>) {
        self.record_inline_image_damage_at(
            self.current_history_row(),
            self.cursor_column,
            width,
            height,
        );
    }

    fn record_inline_image_damage_at(
        &mut self,
        row: usize,
        column: u16,
        width: Option<&str>,
        height: Option<&str>,
    ) {
        let size = self.grid.size();
        let Some(visible_row) = row
            .checked_sub(self.scrollback.len())
            .and_then(|row| u16::try_from(row).ok())
        else {
            return;
        };
        if visible_row >= size.rows || column >= size.columns {
            return;
        }

        let width = inline_image_width_cells(width)
            .min(size.columns.saturating_sub(column))
            .max(1);
        let height = inline_image_height_cells(height)
            .min(size.rows.saturating_sub(visible_row))
            .max(1);
        self.record_damage(DamageRegion::new(column, visible_row, width, height));
    }

    fn apply_osc133_semantic_prompt(&mut self, content: &[char]) {
        let content = content.iter().collect::<String>();
        let (marker, params) = content.split_once(';').unwrap_or((&content, ""));

        match marker {
            "A" | "N" => {
                self.fresh_line();
                self.style.semantic_type = SemanticType::Prompt;
                self.clear_semantic_type_on_movement = false;
                self.record_semantic_prompt_row();
            }
            "P" => {
                self.style.semantic_type = SemanticType::Prompt;
                self.clear_semantic_type_on_movement = false;
                self.record_semantic_prompt_row();
            }
            "B" => {
                self.style.semantic_type = SemanticType::Input;
                self.clear_semantic_type_on_movement = false;
            }
            "I" => {
                self.style.semantic_type = SemanticType::Input;
                self.clear_semantic_type_on_movement = true;
            }
            "C" => {
                self.style.semantic_type = SemanticType::Output;
                self.clear_semantic_type_on_movement = false;
            }
            "D" => self.record_semantic_command_exit(params),
            _ => {}
        }
    }

    fn fresh_line(&mut self) {
        if self.cursor_column != 0 {
            self.newline();
        }
    }

    fn clear_semantic_type_due_to_movement(&mut self) {
        if self.clear_semantic_type_on_movement {
            self.clear_semantic_type_on_movement = false;
            self.style.semantic_type = SemanticType::default();
        }
    }

    fn record_semantic_prompt_row(&mut self) {
        let row = self.current_history_row();
        if self.semantic_prompt_rows.last().copied() == Some(row) {
            return;
        }

        match self.semantic_prompt_rows.binary_search(&row) {
            Ok(_) => {}
            Err(index) => self.semantic_prompt_rows.insert(index, row),
        }
    }

    fn record_semantic_command_exit(&mut self, params: &str) {
        let mut exit_code = None;
        let mut aid = None;

        for (index, part) in params
            .split(';')
            .filter(|part| !part.is_empty())
            .enumerate()
        {
            if let Some(value) = part.strip_prefix("aid=") {
                aid = Some(value.to_owned());
            } else if index == 0 {
                exit_code = part.parse::<i32>().ok();
            }
        }

        self.semantic_command_exits.push(SemanticCommandExit {
            row: self.current_history_row(),
            exit_code,
            aid,
        });
    }

    fn current_history_row(&self) -> usize {
        self.scrollback
            .len()
            .saturating_add(usize::from(self.cursor_row))
    }

    fn apply_osc8_hyperlink(&mut self, content: &[char]) {
        let Some(separator) = content.iter().position(|ch| *ch == ';') else {
            return;
        };

        let uri = content[separator + 1..].iter().collect::<String>();
        if uri.is_empty() {
            self.style.hyperlink = None;
        } else {
            self.style.hyperlink = Some(uri.into());
        }
    }

    fn skip_control_string(
        &mut self,
        chars: &[char],
        index: usize,
        content_offset: usize,
        parse: fn(&[char], usize) -> SequenceParse<usize>,
    ) -> Option<usize> {
        match parse(chars, index + content_offset) {
            SequenceParse::Complete(sequence_end) => Some(sequence_end + 1),
            SequenceParse::Cancelled(cancel_index) => Some(cancel_index + 1),
            SequenceParse::Pending => {
                self.pending_control.extend_from_slice(&chars[index..]);
                None
            }
        }
    }

    #[must_use]
    pub const fn grid(&self) -> &TerminalGrid {
        &self.grid
    }

    #[must_use]
    /// Returns retained scrollback in oldest-to-newest logical order.
    ///
    /// The 0.1.0 API intentionally returns [`HistoryBuffer`] instead of a
    /// contiguous slice. See that type's migration guide and complexity
    /// contract.
    pub const fn scrollback(&self) -> &HistoryBuffer<ScrollbackLine> {
        &self.scrollback
    }

    #[must_use]
    pub const fn current_seqno(&self) -> SequenceNo {
        self.seqno
    }

    fn advance_seqno(&mut self) {
        self.seqno = self
            .seqno
            .checked_add(1)
            .expect("terminal sequence number overflow");
    }

    /// Reports the stable-row geometry for the active screen domain.
    ///
    /// # Panics
    ///
    /// Panics if retained row counts or stable-row offsets exceed their
    /// representable ranges.
    #[must_use]
    pub fn stable_dimensions(&self) -> TerminalStableDimensions {
        let viewport_rows = usize::from(self.grid.size().rows);
        if self.alternate_screen_active() {
            return TerminalStableDimensions {
                domain: TerminalScreenDomain::Alternate,
                viewport_rows,
                scrollback_rows: viewport_rows,
                scrollback_top: 0,
                physical_top: 0,
            };
        }

        let physical_top = self
            .main_stable_row_offset
            .checked_add(
                StableRowIndex::try_from(self.scrollback.len())
                    .expect("scrollback length must fit a stable row index"),
            )
            .expect("retained terminal rows must fit the stable row index");
        let scrollback_rows = self
            .scrollback
            .len()
            .checked_add(viewport_rows)
            .expect("retained terminal row count must fit usize");
        TerminalStableDimensions {
            domain: TerminalScreenDomain::Main,
            viewport_rows,
            scrollback_rows,
            scrollback_top: self.main_stable_row_offset,
            physical_top,
        }
    }

    /// Returns the half-open range of stable rows retained by the terminal.
    ///
    /// # Panics
    ///
    /// Panics if retained row counts or stable-row offsets exceed their
    /// representable ranges.
    #[must_use]
    pub fn retained_stable_range(&self) -> Range<StableRowIndex> {
        let dimensions = self.stable_dimensions();
        let retained_rows = StableRowIndex::try_from(dimensions.scrollback_rows)
            .expect("retained terminal row count must fit a stable row index");
        let end = dimensions
            .scrollback_top
            .checked_add(retained_rows)
            .expect("retained terminal rows must fit the stable row index");
        dimensions.scrollback_top..end
    }

    #[must_use]
    pub fn stable_bottom_exclusive(&self) -> Option<StableRowIndex> {
        let dimensions = self.stable_dimensions();
        let viewport_rows = StableRowIndex::try_from(dimensions.viewport_rows).ok()?;
        dimensions.physical_top.checked_add(viewport_rows)
    }

    #[must_use]
    pub fn viewport_stable_range(&self, top: Option<StableRowIndex>) -> Range<StableRowIndex> {
        let dimensions = self.stable_dimensions();
        let start = top.unwrap_or(dimensions.physical_top);
        let Some(viewport_rows) = StableRowIndex::try_from(dimensions.viewport_rows).ok() else {
            return start..start;
        };
        let Some(end) = start.checked_add(viewport_rows) else {
            return start..start;
        };
        start..end
    }

    #[must_use]
    pub fn is_stable_range_fully_retained(&self, rows: Range<StableRowIndex>) -> bool {
        if rows.start > rows.end {
            return false;
        }
        let retained = self.retained_stable_range();
        rows.start >= retained.start && rows.end <= retained.end
    }

    #[must_use]
    pub fn changed_stable_rows_since(
        &self,
        rows: Range<StableRowIndex>,
        seqno: SequenceNo,
    ) -> Vec<StableRowIndex> {
        if rows.start >= rows.end {
            return Vec::new();
        }

        let retained = self.retained_stable_range();
        let start = rows.start.max(retained.start);
        let end = rows.end.min(retained.end);
        if start >= end {
            return Vec::new();
        }

        (start..end)
            .filter(|stable_row| {
                let Some(history_row) = self.stable_row_to_history_index(*stable_row) else {
                    return false;
                };
                let line_seqno = if self.alternate_screen_active() {
                    let Some(grid_row) = u16::try_from(history_row).ok() else {
                        return false;
                    };
                    self.grid.row_last_change_seqno(grid_row).unwrap_or(0)
                } else if let Some(line) = self.scrollback.get(history_row) {
                    line.last_change_seqno()
                } else {
                    let Some(grid_row) = history_row
                        .checked_sub(self.scrollback.len())
                        .and_then(|row| u16::try_from(row).ok())
                    else {
                        return false;
                    };
                    self.grid.row_last_change_seqno(grid_row).unwrap_or(0)
                };
                line_seqno == 0 || line_seqno > seqno
            })
            .collect()
    }

    #[must_use]
    pub fn history_index_to_stable_row(&self, row: usize) -> Option<StableRowIndex> {
        let dimensions = self.stable_dimensions();
        if row >= dimensions.scrollback_rows {
            return None;
        }
        dimensions
            .scrollback_top
            .checked_add(StableRowIndex::try_from(row).ok()?)
    }

    #[must_use]
    pub fn stable_row_to_history_index(&self, row: StableRowIndex) -> Option<usize> {
        let dimensions = self.stable_dimensions();
        let retained_row = row.checked_sub(dimensions.scrollback_top)?;
        let retained_row = usize::try_from(retained_row).ok()?;
        (retained_row < dimensions.scrollback_rows).then_some(retained_row)
    }

    pub fn set_scrollback_limit(&mut self, limit: usize) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.scrollback_limit = limit;
        self.trim_scrollback_to_limit();
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.set_scrollback_limit",
            limit.to_string().as_bytes(),
            b"result=unit",
        );
    }

    #[must_use]
    pub fn semantic_prompt_rows(&self) -> &[usize] {
        &self.semantic_prompt_rows
    }

    #[must_use]
    pub fn semantic_command_exits(&self) -> &[SemanticCommandExit] {
        &self.semantic_command_exits
    }

    #[must_use]
    pub fn semantic_zones(&self) -> Vec<SemanticZone> {
        let mut zones = Vec::new();
        let mut current_zone = None;

        for (row, line) in self.scrollback.iter().enumerate() {
            append_semantic_zones_for_row(row, line.cells(), &mut current_zone, &mut zones);
        }

        let history_len = self.scrollback.len();
        for row in 0..self.grid.size().rows {
            let cells = (0..self.grid.size().columns)
                .map(|column| self.grid.get(row, column).cloned().unwrap_or_default())
                .collect::<Vec<_>>();
            append_semantic_zones_for_row(
                history_len + usize::from(row),
                &cells,
                &mut current_zone,
                &mut zones,
            );
        }

        if let Some(zone) = current_zone {
            zones.push(zone);
        }

        zones
    }

    #[must_use]
    pub fn semantic_zone_at(&self, x: usize, y: usize) -> Option<SemanticZone> {
        self.semantic_zones()
            .into_iter()
            .find(|zone| zone.contains(x, y))
    }

    #[must_use]
    pub fn stable_semantic_prompt_rows(&self) -> Vec<StableRowIndex> {
        if self.alternate_screen_active() {
            return Vec::new();
        }
        self.semantic_prompt_rows
            .iter()
            .filter_map(|row| self.history_index_to_stable_row(*row))
            .collect()
    }

    #[must_use]
    pub fn stable_semantic_zones(&self) -> Vec<StableSemanticZone> {
        if self.alternate_screen_active() {
            return Vec::new();
        }
        self.semantic_zones()
            .into_iter()
            .filter_map(|zone| {
                Some(StableSemanticZone {
                    start_x: zone.start_x,
                    start_y: self.history_index_to_stable_row(zone.start_y)?,
                    end_x: zone.end_x,
                    end_y: self.history_index_to_stable_row(zone.end_y)?,
                    semantic_type: zone.semantic_type,
                })
            })
            .collect()
    }

    #[must_use]
    pub fn stable_semantic_zone_at(
        &self,
        column: usize,
        row: StableRowIndex,
    ) -> Option<StableSemanticZone> {
        if self.alternate_screen_active() {
            return None;
        }
        let history_row = self.stable_row_to_history_index(row)?;
        let zone = self.semantic_zone_at(column, history_row)?;
        Some(StableSemanticZone {
            start_x: zone.start_x,
            start_y: self.history_index_to_stable_row(zone.start_y)?,
            end_x: zone.end_x,
            end_y: self.history_index_to_stable_row(zone.end_y)?,
            semantic_type: zone.semantic_type,
        })
    }

    #[must_use]
    pub fn stable_semantic_command_exits(&self) -> Vec<StableSemanticCommandExit> {
        if self.alternate_screen_active() {
            return Vec::new();
        }
        self.semantic_command_exits
            .iter()
            .filter_map(|command| {
                Some(StableSemanticCommandExit {
                    row: self.history_index_to_stable_row(command.row)?,
                    exit_code: command.exit_code,
                    aid: command.aid.clone(),
                })
            })
            .collect()
    }

    #[must_use]
    pub fn text_from_semantic_zone(&self, zone: SemanticZone) -> Option<String> {
        self.text_from_region(zone.start_x, zone.start_y, zone.end_x, zone.end_y)
    }

    #[must_use]
    pub fn text_from_stable_selection(&self, selection: StableSelectionRange) -> Option<String> {
        let dimensions = self.stable_dimensions();
        if selection.start.domain != selection.end.domain
            || selection.start.domain != dimensions.domain
        {
            return None;
        }

        let (first, last) = if (selection.start.row, selection.start.column)
            <= (selection.end.row, selection.end.column)
        {
            (selection.start, selection.end)
        } else {
            (selection.end, selection.start)
        };
        let retained = self.retained_stable_range();
        let retained_last = retained.end.checked_sub(1)?;
        let first_retained = first.row.max(retained.start);
        let last_retained = last.row.min(retained_last);
        if first_retained > last_retained {
            return None;
        }

        let first_history = self.stable_row_to_text_history_index(first_retained)?;
        let last_history = self.stable_row_to_text_history_index(last_retained)?;
        if selection.rectangular {
            let first_column = selection.start.column.min(selection.end.column);
            let last_column = selection.start.column.max(selection.end.column);
            let mut rows = Vec::new();
            for row in first_history..=last_history {
                rows.push(self.text_from_region(first_column, row, last_column, row)?);
            }
            return Some(rows.join("\n"));
        }

        let first_column = if first_retained == first.row {
            first.column
        } else {
            0
        };
        let last_column = if last_retained == last.row {
            last.column
        } else {
            usize::MAX
        };
        self.text_from_region(first_column, first_history, last_column, last_history)
    }

    fn stable_row_to_text_history_index(&self, row: StableRowIndex) -> Option<usize> {
        let history_row = self.stable_row_to_history_index(row)?;
        if self.alternate_screen_active() {
            self.scrollback.len().checked_add(history_row)
        } else {
            Some(history_row)
        }
    }

    #[must_use]
    pub fn text_from_region(
        &self,
        start_x: usize,
        start_y: usize,
        end_x: usize,
        end_y: usize,
    ) -> Option<String> {
        if start_y > end_y || (start_y == end_y && start_x > end_x) {
            return None;
        }

        let mut lines = Vec::new();
        let mut logical_line = String::new();
        for row in start_y..=end_y {
            let cells = self.cells_for_history_row(row)?;
            if cells.is_empty() {
                if row != start_y && !self.history_row_is_wrapped(row)? {
                    trim_trailing_spaces(&mut logical_line);
                    lines.push(std::mem::take(&mut logical_line));
                }
                continue;
            }

            let mut first_column = if row == start_y { start_x } else { 0 };
            let last_column = if row == end_y {
                end_x.min(cells.len().saturating_sub(1))
            } else {
                cells.len().saturating_sub(1)
            };
            if let Some(crate::CellContent::Continuation { leader_delta }) =
                cells.get(first_column).map(Cell::content)
            {
                first_column = first_column.saturating_sub(usize::from(*leader_delta));
            }

            if first_column > last_column || first_column >= cells.len() {
                if row != start_y && !self.history_row_is_wrapped(row)? {
                    trim_trailing_spaces(&mut logical_line);
                    lines.push(std::mem::take(&mut logical_line));
                }
                continue;
            }

            if row != start_y && !self.history_row_is_wrapped(row)? {
                trim_trailing_spaces(&mut logical_line);
                lines.push(std::mem::take(&mut logical_line));
            }

            for cell in &cells[first_column..=last_column] {
                logical_line.push_str(cell.text());
            }
        }

        trim_trailing_spaces(&mut logical_line);
        lines.push(logical_line);
        Some(lines.join("\n"))
    }

    #[must_use]
    pub fn checksum_rectangle(&self, left: u16, top: u16, right: u16, bottom: u16) -> u16 {
        let size = self.grid.size();
        if size.rows == 0 || size.columns == 0 || top > bottom || left > right {
            return u16::from(b' ');
        }

        let row_origin = if self.modes.origin_mode {
            self.scroll_top
        } else {
            0
        };
        let column_origin = if self.modes.origin_mode {
            self.left_margin
        } else {
            0
        };
        let max_row = size.rows.saturating_sub(1);
        let max_column = size.columns.saturating_sub(1);
        let start_row = row_origin.saturating_add(top).min(max_row);
        let end_row = row_origin.saturating_add(bottom).min(max_row);
        let start_column = column_origin.saturating_add(left).min(max_column);
        let end_column = column_origin.saturating_add(right).min(max_column);

        if start_row > end_row || start_column > end_column {
            return u16::from(b' ');
        }

        let mut checksum = 0_u16;
        for row in start_row..=end_row {
            for column in start_column..=end_column {
                let byte = self
                    .grid
                    .get(row, column)
                    .map_or(b' ', |cell| cell.primary_char() as u8);
                checksum = checksum.wrapping_add(u16::from(byte));
            }
        }

        if checksum == 0 {
            u16::from(b' ')
        } else {
            checksum
        }
    }

    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    #[must_use]
    pub fn icon_title(&self) -> Option<&str> {
        self.icon_title.as_deref()
    }

    #[must_use]
    pub fn window_title(&self) -> Option<&str> {
        self.window_title.as_deref()
    }

    #[must_use]
    pub fn current_working_dir(&self) -> Option<&str> {
        self.current_working_dir.as_deref()
    }

    #[must_use]
    pub fn badge_format(&self) -> Option<&str> {
        self.badge_format.as_deref()
    }

    #[must_use]
    pub fn user_vars(&self) -> &HashMap<String, String> {
        &self.user_vars
    }

    #[must_use]
    pub fn pending_metadata_changes(&self) -> TerminalMetadataChanges<'_> {
        TerminalMetadataChanges {
            title: self.pending_metadata_changes.title,
            current_working_dir: self.pending_metadata_changes.current_working_dir,
            badge_format: self.pending_metadata_changes.badge_format,
            user_vars: &self.pending_metadata_changes.user_vars,
        }
    }

    pub fn clear_pending_metadata_changes(&mut self) {
        self.pending_metadata_changes.title = false;
        self.pending_metadata_changes.current_working_dir = false;
        self.pending_metadata_changes.badge_format = false;
        self.pending_metadata_changes.user_vars.clear();
    }

    #[must_use]
    pub const fn unicode_version(&self) -> u32 {
        self.unicode_version
    }

    #[must_use]
    pub fn inline_images(&self) -> &[ItermInlineImage] {
        &self.inline_images
    }

    /// Returns the persistent logical image-cell mapping for the active
    /// screen. Pixel geometry is intentionally absent from this state.
    #[must_use]
    pub fn inline_image_attachments(&self) -> &[CellAttachment] {
        &self.inline_image_attachments
    }

    /// Returns whether a logical-cell placement has had every attachment
    /// deleted by a cell transform.  Renderers use this explicit state to
    /// distinguish an intentionally empty placement from legacy pixel-only
    /// imagery that has no attachments by design.
    #[must_use]
    pub fn inline_image_attachment_parent_is_empty(&self, image_index: usize) -> bool {
        let Some(image) = self.inline_images.get(image_index) else {
            return false;
        };
        let Some(parent_identity) = self.inline_image_parent_ids.get(image_index) else {
            return false;
        };
        cell_attachment_dimensions(image).is_some()
            && !self
                .inline_image_attachments
                .iter()
                .any(|attachment| attachment.parent_identity == *parent_identity)
    }

    /// Returns the physical inline-image placements split at terminal cell
    /// boundaries. Placements whose source or destination geometry cannot be
    /// represented safely are omitted so renderers can retain their existing
    /// whole-placement path.
    #[must_use]
    pub fn inline_image_fragments(&self) -> Vec<InlineImageFragment> {
        let attachments = self
            .inline_image_attachments
            .iter()
            .filter_map(|attachment| {
                let image_index = self
                    .inline_image_parent_ids
                    .iter()
                    .position(|parent_identity| *parent_identity == attachment.parent_identity)?;
                let image = self.inline_images.get(image_index)?;
                inline_image_attachment_fragment(image_index, image, *attachment)
            })
            .collect::<Vec<_>>();
        let legacy_fragments = self
            .inline_images
            .iter()
            .enumerate()
            .filter(|(_, image)| cell_attachment_dimensions(image).is_none())
            .filter_map(|(image_index, image)| inline_image_fragments(image_index, image))
            .flatten();
        attachments.into_iter().chain(legacy_fragments).collect()
    }

    pub fn take_kitty_graphics_responses(&mut self) -> Vec<Vec<u8>> {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        let responses = std::mem::take(&mut self.kitty_graphics_responses);
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.take_kitty_responses",
            b"",
            task10_trace::encode_byte_vecs(&responses).as_bytes(),
        );
        responses
    }

    #[must_use]
    pub const fn cursor(&self) -> (u16, u16) {
        (self.cursor_row, self.cursor_column)
    }

    fn cells_for_history_row(&self, row: usize) -> Option<Vec<Cell>> {
        if let Some(line) = self.scrollback.get(row) {
            return Some(line.cells().to_vec());
        }

        let grid_row = row.checked_sub(self.scrollback.len())?;
        let grid_row = u16::try_from(grid_row).ok()?;
        if grid_row >= self.grid.size().rows {
            return None;
        }

        Some(
            (0..self.grid.size().columns)
                .map(|column| self.grid.get(grid_row, column).cloned().unwrap_or_default())
                .collect(),
        )
    }

    fn history_row_is_wrapped(&self, row: usize) -> Option<bool> {
        if let Some(line) = self.scrollback.get(row) {
            return Some(line.is_wrapped());
        }

        let grid_row = row.checked_sub(self.scrollback.len())?;
        let grid_row = u16::try_from(grid_row).ok()?;
        if grid_row >= self.grid.size().rows {
            return None;
        }

        Some(self.grid.row_wrapped(grid_row))
    }

    #[must_use]
    pub const fn cursor_visible(&self) -> bool {
        self.modes.cursor_visible
    }

    #[must_use]
    pub const fn cursor_blinking(&self) -> bool {
        self.modes.cursor_blinking
    }

    #[must_use]
    pub const fn cursor_shape(&self) -> CursorShape {
        self.modes.cursor_shape
    }

    pub fn set_default_cursor_style(&mut self, default_cursor_style: CursorStyle) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.default_cursor_style = default_cursor_style;
        self.apply_default_cursor_style();
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.set_default_cursor_style",
            task10_trace::encode_cursor_style(default_cursor_style).as_bytes(),
            b"result=unit",
        );
    }

    #[must_use]
    pub const fn screen_reverse_video(&self) -> bool {
        self.modes.screen_reverse
    }

    #[must_use]
    pub const fn alternate_screen_active(&self) -> bool {
        self.main_screen.is_some()
    }

    #[must_use]
    pub const fn screen_identity_generation(&self) -> SequenceNo {
        self.screen_identity_generation
    }

    #[must_use]
    pub const fn active_style(&self) -> &Cell {
        &self.style
    }

    #[must_use]
    pub const fn scroll_region(&self) -> (u16, u16) {
        (self.scroll_top, self.scroll_bottom)
    }

    #[must_use]
    pub const fn left_right_margins(&self) -> (u16, u16) {
        (self.left_margin, self.right_margin)
    }

    pub fn take_bell_count(&mut self) -> u64 {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        let count = std::mem::take(&mut self.bell_count);
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.take_bells",
            b"",
            count.to_string().as_bytes(),
        );
        count
    }

    pub fn take_unknown_escape_sequences(&mut self) -> Vec<TerminalUnknownEscapeSequence> {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        let sequences = std::mem::take(&mut self.unknown_escape_sequences);
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.take_unknown",
            b"",
            &task10_trace::encode_unknown(&sequences),
        );
        sequences
    }

    fn record_unknown_escape_sequence(&mut self, sequence: String) {
        self.unknown_escape_sequences
            .push(TerminalUnknownEscapeSequence { sequence });
    }

    pub fn erase_scrollback_and_viewport(&mut self) {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        self.erase_scrollback_and_viewport_inner();
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.erase_scrollback_and_viewport",
            b"",
            b"result=unit",
        );
    }

    fn erase_scrollback_and_viewport_inner(&mut self) {
        self.advance_seqno();
        let size = self.grid.size();
        self.prune_scrollback_rows(self.scrollback.len());
        self.title_stack.clear();
        self.clear_inline_images();
        self.pending_kitty_graphics = None;
        self.kitty_images.clear();
        self.kitty_image_numbers.clear();
        self.kitty_relative_parents.clear();
        self.kitty_virtual_placements.clear();
        self.kitty_character_edited_placements.clear();
        self.pending_kitty_placeholder = None;
        self.last_kitty_placeholder = None;
        self.kitty_placeholder_cells.clear();
        self.next_kitty_image_id = 1;
        self.semantic_prompt_rows.clear();
        self.semantic_command_exits.clear();
        if size.rows == 0 || size.columns == 0 {
            self.cursor_row = 0;
            self.cursor_column = 0;
            self.pending_wrap = false;
            self.clear_semantic_type_on_movement = false;
            return;
        }

        let cursor_row = self.cursor_row.min(size.rows.saturating_sub(1));
        let cursor_line = (0..size.columns)
            .map(|column| {
                self.grid
                    .get(cursor_row, column)
                    .cloned()
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        let blank = self.blank_cell();

        for row in 0..size.rows {
            for column in 0..size.columns {
                self.grid.set(row, column, blank.clone());
            }
            self.grid.set_row_wrapped(row, false);
            self.grid.set_row_last_change_seqno(row, self.seqno);
        }

        for (column, cell) in cursor_line.into_iter().enumerate() {
            let column = u16::try_from(column).unwrap_or(u16::MAX);
            self.grid.set(0, column, cell);
        }

        self.cursor_row = 0;
        self.pending_wrap = false;
        self.clear_semantic_type_on_movement = false;
        self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
    }

    pub fn resize(&mut self, size: TerminalSize) -> TerminalResizeOutcome {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        let outcome = self.resize_inner(size);
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.resize",
            &task10_trace::encode_size(size),
            task10_trace::encode_resize_outcome(outcome).as_bytes(),
        );
        outcome
    }

    #[expect(
        clippy::if_not_else,
        reason = "the primary width-changing reflow path remains first ahead of the smaller same-width path"
    )]
    fn resize_inner(&mut self, size: TerminalSize) -> TerminalResizeOutcome {
        let size = TerminalSize::new(size.columns.max(1), size.rows);
        self.advance_seqno();
        let old_size = self.grid.size();
        let active_main_screen = self.main_screen.is_none();
        let old_main_scrollback_rows = self.scrollback.len();
        let cell_width_overrides = self.cell_width_overrides.clone();
        if old_size.columns != size.columns {
            if let Some(mut screen) = self.main_screen.take() {
                // The active alternate screen retains its physical layout.  The
                // dormant main screen and its history are the only buffers that
                // participate in width reflow.
                self.grid.resize_with_seqno(size, self.seqno);
                let mut reflow_cursor_column = screen.cursor_column;
                reflow_main_screen(
                    &mut self.scrollback,
                    &mut screen.grid,
                    size,
                    self.seqno,
                    self.unicode_version,
                    self.treat_east_asian_ambiguous_width_as_wide,
                    &cell_width_overrides,
                    &mut screen.cursor_row,
                    &mut reflow_cursor_column,
                );
                screen.cursor_column = reflow_cursor_column;
                screen.pending_wrap = false;
                self.main_screen = Some(screen);
                self.apply_main_reflow_outcome(MainReflowOutcome::new(
                    old_size,
                    old_main_scrollback_rows,
                ));
            } else {
                let mut reflow_cursor_column = self.cursor_column;
                reflow_main_screen(
                    &mut self.scrollback,
                    &mut self.grid,
                    size,
                    self.seqno,
                    self.unicode_version,
                    self.treat_east_asian_ambiguous_width_as_wide,
                    &cell_width_overrides,
                    &mut self.cursor_row,
                    &mut reflow_cursor_column,
                );
                self.cursor_column = reflow_cursor_column;
                self.pending_wrap = false;
                self.apply_main_reflow_outcome(MainReflowOutcome::new(
                    old_size,
                    old_main_scrollback_rows,
                ));
            }
            self.trim_scrollback_to_limit();
        } else {
            self.grid.resize_with_seqno(size, self.seqno);
            if let Some(screen) = self.main_screen.as_mut() {
                screen.grid.resize_with_seqno(size, self.seqno);
                clamp_screen_state(screen, size);
            }
        }
        let virtual_placements = &self.kitty_virtual_placements;
        if let Some(screen) = self.main_screen.as_mut() {
            retain_kitty_character_edited_placements(
                &screen.inline_images,
                &screen.inline_image_parent_ids,
                &screen.inline_image_attachments,
                virtual_placements,
                &mut screen.kitty_character_edited_placements,
            );
        }
        self.retain_kitty_character_edited_placements();
        self.tab_stops.resize(size);

        self.clamp_to_size();
        self.scroll_top = 0;
        self.scroll_bottom = size.rows.saturating_sub(1);
        self.left_margin = 0;
        self.right_margin = size.columns.saturating_sub(1);
        self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
        if old_size == size {
            TerminalResizeOutcome::Unchanged
        } else if old_size.columns != size.columns {
            if active_main_screen {
                TerminalResizeOutcome::MainScreenReflowed
            } else {
                TerminalResizeOutcome::AlternateScreenResized
            }
        } else {
            TerminalResizeOutcome::PhysicalResize
        }
    }

    fn apply_main_reflow_outcome(&mut self, outcome: MainReflowOutcome) {
        self.main_stable_row_offset = self
            .main_stable_row_offset
            .checked_add(
                StableRowIndex::try_from(outcome.previous_history_rows)
                    .expect("terminal history rows must fit a stable row index"),
            )
            .expect("terminal stable row index overflow");
        self.semantic_prompt_rows.clear();
        self.semantic_command_exits.clear();

        if let Some(screen) = self.main_screen.as_mut() {
            retire_reflow_coordinate_state(screen);
            return;
        }

        self.clear_inline_images();
        self.kitty_placeholder_cells.clear();
        self.last_kitty_placeholder = None;
        self.pending_kitty_placeholder = None;
        self.kitty_relative_parents.clear();
        self.kitty_virtual_placements.clear();
        self.nfc_last_printable_cell = None;
        self.saved_cursor = None;
    }

    pub fn take_damage(&mut self) -> Vec<DamageRegion> {
        #[cfg(test)]
        let trace = task10_trace::before_action(self);
        let damage = std::mem::take(&mut self.damage);
        #[cfg(test)]
        task10_trace::trace_action(
            self,
            trace,
            "terminal.take_damage",
            b"",
            task10_trace::encode_damage(&damage).as_bytes(),
        );
        damage
    }

    pub fn drain_damage_into(&mut self, target: &mut Vec<DamageRegion>) {
        target.append(&mut self.damage);
    }

    fn reset_terminal(&mut self) {
        self.bump_screen_identity_generation();
        let size = self.grid.size();
        self.prune_scrollback_rows(self.scrollback.len());
        self.grid = TerminalGrid::new_with_seqno(size, self.seqno);
        self.clear_inline_images();
        self.pending_kitty_graphics = None;
        self.kitty_images.clear();
        self.kitty_image_numbers.clear();
        self.kitty_relative_parents.clear();
        self.kitty_virtual_placements.clear();
        self.pending_kitty_placeholder = None;
        self.last_kitty_placeholder = None;
        self.kitty_placeholder_cells.clear();
        self.next_kitty_image_id = 1;
        self.semantic_prompt_rows.clear();
        self.semantic_command_exits.clear();
        self.cursor_row = 0;
        self.cursor_column = 0;
        self.pending_wrap = false;
        self.clear_semantic_type_on_movement = false;
        self.last_printable = None;
        self.nfc_last_printable_cell = None;
        self.saved_cursor = None;
        self.main_screen = None;
        self.modes = TerminalModes::with_cursor_style(self.default_cursor_style);
        self.scroll_top = 0;
        self.scroll_bottom = size.rows.saturating_sub(1);
        self.left_margin = 0;
        self.right_margin = size.columns.saturating_sub(1);
        self.character_set = CharacterSet::Ascii;
        self.tab_stops = TabStops::new(size);
        self.style = Cell::default();
        self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
    }

    fn soft_reset_terminal(&mut self) {
        self.set_alternate_screen(false);
        let size = self.grid.size();
        self.modes.write_mode = CharacterWriteMode::Replace;
        self.modes.origin_mode = false;
        self.modes.auto_wrap = true;
        self.modes.left_right_margin_mode = false;
        self.modes.reverse_wrap = false;
        self.modes.screen_reverse = false;
        self.modes.cursor_visible = true;
        self.style = Cell::default();
        self.clear_kitty_graphics();
        self.scroll_top = 0;
        self.scroll_bottom = size.rows.saturating_sub(1);
        self.left_margin = 0;
        self.right_margin = self.grid.size().columns.saturating_sub(1);
        self.character_set = CharacterSet::Ascii;
        self.saved_cursor = None;
        self.pending_wrap = false;
        self.clear_semantic_type_on_movement = false;
        self.last_printable = None;
        self.nfc_last_printable_cell = None;
        self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
    }

    fn clear_kitty_graphics(&mut self) {
        self.retain_inline_images(|image| image.kitty_image_id.is_none());
        self.pending_kitty_graphics = None;
        self.kitty_images.clear();
        self.kitty_image_numbers.clear();
        self.kitty_relative_parents.clear();
        self.kitty_virtual_placements.clear();
        self.kitty_character_edited_placements.clear();
        self.pending_kitty_placeholder = None;
        self.last_kitty_placeholder = None;
        self.kitty_placeholder_cells.clear();
    }

    fn screen_alignment_test(&mut self) {
        let size = self.grid.size();
        self.modes.origin_mode = false;
        self.scroll_top = 0;
        self.scroll_bottom = size.rows.saturating_sub(1);
        self.left_margin = 0;
        self.right_margin = size.columns.saturating_sub(1);
        self.cursor_row = 0;
        self.cursor_column = 0;
        self.pending_wrap = false;
        self.clear_semantic_type_on_movement = false;

        if size.rows == 0 || size.columns == 0 {
            return;
        }

        let cell = Cell::with_char('E');
        for row in 0..size.rows {
            for column in 0..size.columns {
                self.set_grid_cell(row, column, cell.clone());
            }
            self.set_grid_row_wrapped(row, false);
        }

        self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
    }

    fn newline(&mut self) {
        self.carriage_return();
        self.index_down(false);
    }

    fn line_feed(&mut self) {
        if self.cursor_within_horizontal_margins() {
            self.index_down(false);
        } else {
            self.index_down_outside_horizontal_margins(false);
        }
    }

    fn wrapped_newline(&mut self) {
        if self.modes.left_right_margin_mode && !self.cursor_within_horizontal_margins() {
            self.cursor_column = 0;
            self.pending_wrap = false;
            self.index_down_outside_horizontal_margins(true);
        } else {
            self.carriage_return();
            self.index_down(true);
        }
    }

    fn next_line(&mut self) {
        let cursor_within_horizontal_margins = self.cursor_within_horizontal_margins();
        if self.modes.left_right_margin_mode && !cursor_within_horizontal_margins {
            self.cursor_column = if self.cursor_column < self.left_margin {
                self.cursor_column
            } else {
                self.left_margin
            };
            self.pending_wrap = false;
        } else {
            self.carriage_return();
        }
        if cursor_within_horizontal_margins {
            self.index_down(false);
        } else {
            self.index_down_outside_horizontal_margins(false);
        }
    }

    fn index_down(&mut self, wrapped: bool) {
        self.pending_wrap = false;
        let rows = self.grid.size().rows;
        if rows == 0 {
            return;
        }

        let scroll_bottom = self.scroll_bottom.min(rows - 1);
        if self.cursor_row == scroll_bottom {
            self.scroll_up_with_horizontal_margins(self.scroll_top, scroll_bottom, 1);
            self.cursor_row = scroll_bottom;
        } else if self.cursor_row + 1 < rows {
            self.cursor_row += 1;
        }
        self.set_grid_row_wrapped(self.cursor_row, wrapped);
        self.clear_semantic_type_due_to_movement();
    }

    fn index_down_control(&mut self, wrapped: bool) {
        if self.cursor_within_horizontal_margins() {
            self.index_down(wrapped);
        }
    }

    fn index_down_outside_horizontal_margins(&mut self, wrapped: bool) {
        self.pending_wrap = false;
        let rows = self.grid.size().rows;
        if rows == 0 {
            return;
        }

        let scroll_bottom = self.scroll_bottom.min(rows - 1);
        if self.cursor_row != scroll_bottom && self.cursor_row + 1 < rows {
            self.cursor_row += 1;
        }
        self.set_grid_row_wrapped(self.cursor_row, wrapped);
        self.clear_semantic_type_due_to_movement();
    }

    fn reverse_index(&mut self) {
        if !self.cursor_within_horizontal_margins() {
            return;
        }
        self.pending_wrap = false;
        let rows = self.grid.size().rows;
        if rows == 0 {
            return;
        }

        let scroll_top = self.scroll_top.min(rows - 1);
        let scroll_bottom = self.scroll_bottom.min(rows - 1);
        if self.cursor_row == scroll_top {
            self.scroll_down_with_horizontal_margins(scroll_top, scroll_bottom, 1);
            self.cursor_row = scroll_top;
        } else {
            self.cursor_row = self.cursor_row.saturating_sub(1);
        }
        self.clear_semantic_type_due_to_movement();
    }

    fn backspace(&mut self) {
        self.pending_wrap = false;
        let size = self.grid.size();
        if size.columns == 0 || size.rows == 0 {
            return;
        }

        let left_boundary = if self.modes.left_right_margin_mode
            && self.cursor_column >= self.left_margin
            && self.cursor_column <= self.right_margin
        {
            self.left_margin
        } else {
            0
        };
        if self.cursor_column > left_boundary {
            self.cursor_column -= 1;
            return;
        }

        if self.modes.reverse_wrap && self.modes.auto_wrap {
            self.cursor_row = if self.cursor_row == 0 {
                size.rows - 1
            } else {
                self.cursor_row - 1
            };
            self.cursor_column = if self.modes.left_right_margin_mode {
                self.right_margin.min(size.columns - 1)
            } else {
                size.columns - 1
            };
        }
    }

    fn carriage_return(&mut self) {
        self.cursor_column =
            if self.modes.left_right_margin_mode && self.cursor_column >= self.left_margin {
                self.left_margin
            } else {
                0
            };
        self.pending_wrap = false;
    }

    fn horizontal_tab(&mut self) {
        self.move_forward_tabs(1);
    }

    fn set_horizontal_tab_stop(&mut self) {
        self.tab_stops.set(self.cursor_column, self.grid.size());
    }

    fn clear_tab_stop(&mut self, mode: u16) {
        match mode {
            0 => self.tab_stops.clear(self.cursor_column),
            3 => self.tab_stops.clear_all(),
            _ => {}
        }
    }

    fn move_forward_tabs(&mut self, count: u16) {
        self.pending_wrap = false;
        let columns = self.grid.size().columns;
        if columns == 0 {
            return;
        }

        let fallback = columns - 1;
        for _ in 0..count {
            let next = self.tab_stops.next_after(self.cursor_column, fallback);
            if next == self.cursor_column {
                break;
            }
            self.cursor_column = next;
        }
    }

    fn move_backward_tabs(&mut self, count: u16) {
        self.pending_wrap = false;
        if self.grid.size().columns == 0 {
            return;
        }

        for _ in 0..count {
            let previous = self.tab_stops.previous_before(self.cursor_column);
            if previous == self.cursor_column {
                break;
            }
            self.cursor_column = previous;
        }
    }

    fn should_record_scrollback_for_scroll(&self, top: u16, bottom: u16) -> bool {
        let size = self.grid.size();
        self.main_screen.is_none()
            && top == 0
            && bottom < size.rows
            && self.left_margin == 0
            && self.right_margin == size.columns.saturating_sub(1)
            && size.columns > 0
    }

    fn trim_scrollback_to_limit(&mut self) {
        if self.scrollback.len() > self.scrollback_limit {
            let overflow = self.scrollback.len() - self.scrollback_limit;
            self.prune_scrollback_rows(overflow);
        }
    }

    fn prune_scrollback_rows(&mut self, rows: usize) {
        let rows = self.scrollback.evict_front(rows);
        if rows == 0 {
            return;
        }

        let stable_rows =
            StableRowIndex::try_from(rows).expect("pruned row count must fit a stable row index");
        self.main_stable_row_offset = self
            .main_stable_row_offset
            .checked_add(stable_rows)
            .expect("stable row index overflow while pruning scrollback");
        self.semantic_prompt_rows = self
            .semantic_prompt_rows
            .iter()
            .filter_map(|row| row.checked_sub(rows))
            .collect();
        self.semantic_command_exits = self
            .semantic_command_exits
            .iter()
            .filter_map(|command| {
                command
                    .row
                    .checked_sub(rows)
                    .map(|row| SemanticCommandExit {
                        row,
                        exit_code: command.exit_code,
                        aid: command.aid.clone(),
                    })
            })
            .collect();
        self.rebase_inline_images_after_history_prune(rows);
        self.work_counters.metadata_rebase_batches =
            self.work_counters.metadata_rebase_batches.saturating_add(1);
    }

    fn write_char(&mut self, ch: char) {
        let ch = self.map_graphic_character(ch);
        if is_unicode_presentation_selector(ch)
            && self.unicode_version < UNICODE_PRESENTATION_SELECTOR_VERSION
        {
            return;
        }
        if self.apply_kitty_placeholder_diacritic(ch) {
            return;
        }
        if self.extend_previous_grapheme(ch) {
            return;
        }
        self.finish_pending_kitty_placeholder();
        let width = display_width(
            ch,
            self.unicode_version,
            self.treat_east_asian_ambiguous_width_as_wide,
            &self.cell_width_overrides,
        )
        .min(u16::from(u8::MAX));
        if width == 0 {
            return;
        }

        if self.pending_wrap && self.modes.auto_wrap {
            self.wrapped_newline();
        } else if self.pending_wrap {
            self.pending_wrap = false;
        }

        if self.cursor_column.saturating_add(width) > self.grid.size().columns
            && self.modes.auto_wrap
        {
            self.wrapped_newline();
        }

        if self.cursor_row >= self.grid.size().rows
            || self.cursor_column >= self.grid.size().columns
            || self.grid.size().columns == 0
        {
            return;
        }

        let available_width = self.grid.size().columns - self.cursor_column;
        let write_width = width.min(available_width);
        if self.modes.write_mode == CharacterWriteMode::Insert {
            self.insert_blank_characters_for_write(write_width);
        }

        let column = self.cursor_column;
        let row = self.cursor_row;
        for target in column..column.saturating_add(write_width) {
            self.clear_grapheme_span_at(row, target);
        }
        let history_row = self.scrollback.len().saturating_add(usize::from(row));
        let mut cell = self.style.clone();
        cell.set_text(ch.to_string(), saturating_u8(width));

        if self.set_grid_cell(row, column, cell) {
            self.clear_kitty_placeholder_cells(history_row, column, write_width);
            if write_width > 1 {
                let mut continuation = self.style.clone();
                for offset in 1..write_width {
                    continuation.set_continuation(saturating_u8(offset));
                    self.set_grid_cell(row, column + offset, continuation.clone());
                }
            }
            if write_width < width {
                let mut continuation = self.style.clone();
                self.grid.set_reflow_overflow(
                    row,
                    (write_width..width)
                        .map(|offset| {
                            continuation.set_continuation(saturating_u8(offset));
                            continuation.clone()
                        })
                        .collect(),
                );
            }

            self.record_damage(DamageRegion::new(column, row, write_width, 1));
            self.advance_cursor(write_width);
            self.last_printable = Some(SmolStr::new(ch.to_string()));
            self.nfc_last_printable_cell = Some((row, column));
            if ch == KITTY_UNICODE_PLACEHOLDER {
                self.pending_kitty_placeholder = Some(PendingKittyPlaceholder {
                    row: history_row,
                    column,
                    foreground: self.style.foreground,
                    underline_color: self.style.underline_color,
                    image_id: kitty_placeholder_image_id(self.style.foreground),
                    placement_id: kitty_placeholder_placement_id(self.style.underline_color),
                    diacritics: Vec::new(),
                    rendered_row: None,
                    rendered_column: None,
                    rendered_image_id: None,
                    rendered_placement_id: None,
                });
            } else {
                self.last_kitty_placeholder = None;
            }
        }
    }

    fn extend_previous_grapheme(&mut self, ch: char) -> bool {
        let Some((row, column)) = self.nfc_last_printable_cell else {
            return false;
        };
        let Some(previous_cell) = self.grid.get(row, column) else {
            return false;
        };
        if previous_cell.primary_char() == KITTY_UNICODE_PLACEHOLDER
            || previous_cell.is_continuation()
            || previous_cell.is_blank()
            || !may_extend_previous_grapheme(previous_cell.text(), ch)
        {
            return false;
        }
        let previous_cell = previous_cell.clone();

        let mut grapheme = String::with_capacity(previous_cell.text().len() + ch.len_utf8());
        grapheme.push_str(previous_cell.text());
        grapheme.push(ch);
        if grapheme.graphemes(true).count() != 1 {
            return false;
        }

        let previous_width = u16::from(previous_cell.columns());
        let sequence_width = grapheme_display_width(
            &grapheme,
            self.unicode_version,
            self.treat_east_asian_ambiguous_width_as_wide,
            &self.cell_width_overrides,
        );
        if previous_width == 0 || sequence_width == 0 {
            return true;
        }

        let expected_cursor = column
            .saturating_add(previous_width)
            .min(self.character_right_boundary());
        if self.cursor_row != row || self.cursor_column != expected_cursor {
            return true;
        }

        let available_width = self
            .character_right_boundary()
            .saturating_add(1)
            .saturating_sub(column);
        let visible_width = sequence_width.min(available_width).max(1);

        let mut leader = previous_cell.clone();
        leader.set_text(grapheme.clone(), saturating_u8(sequence_width));
        self.set_grid_cell(row, column, leader);

        if visible_width > previous_width {
            let mut continuation = previous_cell.clone();
            for offset in previous_width..visible_width {
                continuation.set_continuation(saturating_u8(offset));
                self.set_grid_cell(row, column + offset, continuation.clone());
            }
        } else {
            for offset in visible_width..previous_width {
                self.set_grid_cell(row, column + offset, self.blank_cell());
            }
        }
        let overflow = if visible_width < sequence_width {
            let mut continuation = previous_cell;
            (visible_width..sequence_width)
                .map(|offset| {
                    continuation.set_continuation(saturating_u8(offset));
                    continuation.clone()
                })
                .collect()
        } else {
            Vec::new()
        };
        self.grid.set_reflow_overflow(row, overflow);

        self.record_damage(DamageRegion::new(
            column,
            row,
            previous_width.min(available_width).max(visible_width),
            1,
        ));
        self.cursor_column = column;
        self.pending_wrap = false;
        self.advance_cursor(visible_width);
        self.last_printable = Some(grapheme.into());
        true
    }

    fn clear_kitty_placeholder_cells(&mut self, row: usize, column: u16, width: u16) {
        for offset in 0..width {
            let column = column.saturating_add(offset);
            if let Some(placeholder) = self.kitty_placeholder_cells.remove(&(row, column))
                && let Some((render_row, render_column, image_id, placement_id)) =
                    self.kitty_placeholder_render_key(row, column, placeholder)
            {
                self.remove_kitty_placeholder_render(
                    render_row,
                    render_column,
                    image_id,
                    placement_id,
                );
            }
        }
        if self.last_kitty_placeholder.is_some_and(|placeholder| {
            placeholder.row == row
                && placeholder.column >= column
                && placeholder.column < column.saturating_add(width)
        }) {
            self.last_kitty_placeholder = None;
        }
    }

    fn apply_kitty_placeholder_diacritic(&mut self, ch: char) -> bool {
        let Some(value) = kitty_placeholder_diacritic_value(ch) else {
            return false;
        };
        let Some(mut pending) = self.pending_kitty_placeholder.take() else {
            return false;
        };

        pending.diacritics.push(value);
        self.place_kitty_virtual_placeholder(&mut pending);
        self.pending_kitty_placeholder = Some(pending);
        true
    }

    fn finish_pending_kitty_placeholder(&mut self) {
        if let Some(mut pending) = self.pending_kitty_placeholder.take() {
            self.place_kitty_virtual_placeholder(&mut pending);
        }
    }

    fn place_kitty_virtual_placeholder(&mut self, pending: &mut PendingKittyPlaceholder) {
        let Some(resolved) = self.resolve_kitty_placeholder(pending) else {
            return;
        };
        let Some((origin_row, origin_column)) = kitty_placeholder_origin(
            pending,
            resolved.placeholder_row,
            resolved.placeholder_column,
        ) else {
            self.last_kitty_placeholder = None;
            return;
        };
        let Some(placement) = self
            .kitty_virtual_placement_for_placeholder(resolved.image_id, pending.placement_id)
            .cloned()
        else {
            self.last_kitty_placeholder = None;
            return;
        };
        let Some(image) = self.kitty_images.get(&placement.image_id).cloned() else {
            self.last_kitty_placeholder = None;
            return;
        };

        if let (Some(rendered_row), Some(rendered_column), Some(rendered_image_id)) = (
            pending.rendered_row,
            pending.rendered_column,
            pending.rendered_image_id,
        ) {
            self.remove_kitty_placeholder_render(
                rendered_row,
                rendered_column,
                rendered_image_id,
                pending.rendered_placement_id,
            );
        }
        self.remove_kitty_placeholder_render(
            origin_row,
            origin_column,
            placement.image_id,
            placement.placement_id,
        );

        self.place_kitty_image(
            &image,
            KittyPlacementOptions {
                display_columns: placement.display_columns,
                display_rows: placement.display_rows,
                image_id: Some(placement.image_id),
                placement_id: placement.placement_id,
                z_index: placement.z_index,
                parent_placement: None,
                row: Some(origin_row),
                column: Some(origin_column),
                move_cursor: false,
                source_rect: placement.source_rect,
                target_x: placement.target_x,
                target_y: placement.target_y,
            },
        );
        pending.rendered_row = Some(origin_row);
        pending.rendered_column = Some(origin_column);
        pending.rendered_image_id = Some(placement.image_id);
        pending.rendered_placement_id = placement.placement_id;
        let placeholder = LastKittyPlaceholder {
            row: pending.row,
            column: pending.column,
            foreground: pending.foreground,
            underline_color: pending.underline_color,
            image_id_high_byte: resolved.image_id_high_byte,
            placeholder_row: resolved.placeholder_row,
            placeholder_column: resolved.placeholder_column,
        };
        self.kitty_placeholder_cells
            .insert((pending.row, pending.column), placeholder);
        self.last_kitty_placeholder = Some(placeholder);
    }

    fn kitty_placeholder_render_keys(&self) -> HashSet<KittyPlaceholderRenderKey> {
        self.kitty_placeholder_cells
            .iter()
            .filter_map(|(&(row, column), &placeholder)| {
                self.kitty_placeholder_render_key(row, column, placeholder)
            })
            .collect()
    }

    fn kitty_placeholder_render_key(
        &self,
        row: usize,
        column: u16,
        placeholder: LastKittyPlaceholder,
    ) -> Option<KittyPlaceholderRenderKey> {
        let low_bytes = kitty_placeholder_image_id(placeholder.foreground)? & 0x00ff_ffff;
        let image_id = low_bytes | (placeholder.image_id_high_byte << 24);
        let placement_id = kitty_placeholder_placement_id(placeholder.underline_color);
        let (origin_row, origin_column) = kitty_placeholder_origin_from_cell(
            row,
            column,
            placeholder.placeholder_row,
            placeholder.placeholder_column,
        )?;
        let placement = self.kitty_virtual_placement_for_placeholder(image_id, placement_id)?;
        Some((
            origin_row,
            origin_column,
            placement.image_id,
            placement.placement_id,
        ))
    }

    fn resolve_kitty_placeholder(
        &self,
        pending: &PendingKittyPlaceholder,
    ) -> Option<ResolvedKittyPlaceholder> {
        let low_bytes = pending.image_id? & 0x00ff_ffff;
        let left = self.left_kitty_placeholder_for(pending);
        let (placeholder_row, placeholder_column, image_id_high_byte) =
            match pending.diacritics.as_slice() {
                [] => {
                    let left = left?;
                    (
                        left.placeholder_row,
                        left.placeholder_column.checked_add(1)?,
                        left.image_id_high_byte,
                    )
                }
                [value] => {
                    if let Some(left) = left {
                        if left
                            .placeholder_column
                            .checked_add(1)
                            .is_some_and(|column| column == *value)
                        {
                            (left.placeholder_row, *value, left.image_id_high_byte)
                        } else {
                            (*value, 0, 0)
                        }
                    } else {
                        (*value, 0, 0)
                    }
                }
                [row, column] => {
                    let image_id_high_byte = left
                        .filter(|left| {
                            left.placeholder_row == *row
                                && left
                                    .placeholder_column
                                    .checked_add(1)
                                    .is_some_and(|left_column| left_column == *column)
                        })
                        .map_or(0, |left| left.image_id_high_byte);
                    (*row, *column, image_id_high_byte)
                }
                [row, column, image_id_high_byte, ..] => (*row, *column, *image_id_high_byte),
            };
        Some(ResolvedKittyPlaceholder {
            image_id: low_bytes | (image_id_high_byte << 24),
            image_id_high_byte,
            placeholder_row,
            placeholder_column,
        })
    }

    fn left_kitty_placeholder_for(
        &self,
        pending: &PendingKittyPlaceholder,
    ) -> Option<LastKittyPlaceholder> {
        let left_column = pending.column.checked_sub(1)?;
        let left = self
            .kitty_placeholder_cells
            .get(&(pending.row, left_column))
            .copied()
            .or(self.last_kitty_placeholder)?;
        if left.foreground == pending.foreground
            && left.underline_color == pending.underline_color
            && left.row == pending.row
            && left
                .column
                .checked_add(1)
                .is_some_and(|column| column == pending.column)
        {
            Some(left)
        } else {
            None
        }
    }

    fn remove_kitty_placeholder_render(
        &mut self,
        row: usize,
        column: u16,
        image_id: u32,
        placement_id: Option<u32>,
    ) {
        self.retain_inline_images(|image| {
            image.row != row
                || image.column != column
                || image.kitty_image_id != Some(image_id)
                || image.kitty_placement_id != placement_id
        });
    }

    fn kitty_virtual_placement_for_placeholder(
        &self,
        image_id: u32,
        placement_id: Option<u32>,
    ) -> Option<&KittyVirtualPlacement> {
        if let Some(placement_id) = placement_id {
            return self.kitty_virtual_placements.get(&(image_id, placement_id));
        }

        self.kitty_virtual_placements
            .get(&kitty_virtual_placement_key(image_id, None))
            .or_else(|| {
                self.kitty_virtual_placements
                    .values()
                    .find(|placement| placement.image_id == image_id)
            })
    }

    fn advance_cursor(&mut self, width: u16) {
        let next_column = self.cursor_column.saturating_add(width);
        let right_boundary = self.character_right_boundary();
        if next_column > right_boundary {
            self.cursor_column = right_boundary;
            self.pending_wrap = self.modes.auto_wrap;
        } else {
            self.cursor_column = next_column;
            self.pending_wrap = false;
        }
    }

    fn apply_csi(&mut self, command: char, params: &[char]) {
        match command {
            '@' => self.insert_blank_characters(csi_count(params)),
            'A' => self.move_cursor_up(csi_count(params)),
            'B' | 'e' => self.move_cursor_down(csi_count(params)),
            'C' | 'a' => self.move_cursor_forward(csi_count(params)),
            'D' => self.move_cursor_back(csi_count(params)),
            'E' => self.move_cursor_next_line(csi_count(params)),
            'F' => self.move_cursor_previous_line(csi_count(params)),
            'G' | '`' => self.position_cursor_column(params),
            'H' | 'f' => self.position_cursor(params),
            'I' => self.move_forward_tabs(csi_count(params)),
            'J' => self.erase_display(csi_or_private_mode(params), csi_is_private(params)),
            'K' => self.erase_line(csi_or_private_mode(params), csi_is_private(params)),
            'L' => self.insert_lines(csi_count(params)),
            'M' => self.delete_lines(csi_count(params)),
            'P' => self.delete_characters(csi_count(params)),
            'S' => self.scroll_up(csi_count(params)),
            'T' => self.scroll_down(csi_count(params)),
            'X' => self.erase_characters(csi_count(params)),
            'Z' => self.move_backward_tabs(csi_count(params)),
            'b' => self.repeat_previous_character(csi_count(params)),
            'd' => self.position_cursor_row(params),
            'g' => self.clear_tab_stop(csi_mode(params)),
            'm' => self.apply_sgr(params),
            'q' if params.ends_with(&['"']) => self.set_character_protection(params),
            'q' => self.set_cursor_shape(params),
            'p' if params == ['!'] => self.soft_reset_terminal(),
            'r' => self.set_scroll_region(params),
            't' => self.apply_window_manipulation(params),
            'h' => self.set_mode(params, true),
            'l' => self.set_mode(params, false),
            's' if self.modes.left_right_margin_mode => self.set_left_right_margins(params),
            's' => self.save_cursor(),
            'u' => self.restore_cursor(),
            _ => self.record_unknown_escape_sequence(format!(
                "CSI {}{command}",
                params.iter().collect::<String>()
            )),
        }
    }

    fn apply_window_manipulation(&mut self, params: &[char]) {
        let values = parse_csi_params(params);
        let Some(action) = values.first().copied() else {
            return;
        };
        let target = values.get(1).copied().unwrap_or(0);
        if target > 2 {
            return;
        }

        match action {
            22 => self.title_stack.push(self.title.clone()),
            23 => {
                if let Some(title) = self.title_stack.pop() {
                    if self.title != title {
                        self.pending_metadata_changes.title = true;
                    }
                    self.title = title;
                }
            }
            _ => {}
        }
    }

    fn set_mode(&mut self, params: &[char], enabled: bool) {
        if params.first() == Some(&'?') {
            self.set_private_mode(params, enabled);
        } else {
            self.set_standard_mode(params, enabled);
        }
    }

    fn set_standard_mode(&mut self, params: &[char], enabled: bool) {
        for value in parse_csi_params(params) {
            if value == 4 {
                self.set_insert_mode(enabled);
            }
        }
    }

    fn set_private_mode(&mut self, params: &[char], enabled: bool) {
        let Some(values) = parse_private_csi_params(params) else {
            return;
        };

        for value in values {
            match value {
                5 => self.set_screen_reverse_video(enabled),
                6 => self.set_origin_mode(enabled),
                7 => self.set_auto_wrap(enabled),
                12 => self.modes.cursor_blinking = enabled,
                25 => self.modes.cursor_visible = enabled,
                45 => self.modes.reverse_wrap = enabled,
                69 => self.set_left_right_margin_mode(enabled),
                80 => self.modes.sixel_display_mode = enabled,
                8452 => self.modes.sixel_scrolls_right = enabled,
                47 | 1047 | 1049 => self.set_alternate_screen(enabled),
                1048 => {
                    if enabled {
                        self.save_cursor();
                    } else {
                        self.restore_cursor();
                    }
                }
                _ => {}
            }
        }
    }

    fn set_insert_mode(&mut self, enabled: bool) {
        self.modes.write_mode = if enabled {
            CharacterWriteMode::Insert
        } else {
            CharacterWriteMode::Replace
        };
    }

    fn set_screen_reverse_video(&mut self, enabled: bool) {
        if self.modes.screen_reverse == enabled {
            return;
        }
        self.modes.screen_reverse = enabled;
        let size = self.grid.size();
        self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
    }

    fn set_cursor_shape(&mut self, params: &[char]) {
        let Some(params) = params.strip_suffix(&[' ']) else {
            return;
        };
        let value = parse_csi_params(params).first().copied().unwrap_or(0);
        if value == 0 {
            self.apply_default_cursor_style();
            return;
        }
        self.modes.cursor_shape = match value {
            1 | 2 => CursorShape::Block,
            3 | 4 => CursorShape::Underline,
            5 | 6 => CursorShape::Bar,
            _ => self.modes.cursor_shape,
        };
        self.modes.cursor_blinking = match value {
            1 | 3 | 5 => true,
            0 | 2 | 4 | 6 => false,
            _ => self.modes.cursor_blinking,
        };
    }

    fn apply_default_cursor_style(&mut self) {
        self.modes.cursor_shape = self.default_cursor_style.shape();
        self.modes.cursor_blinking = self.default_cursor_style.blinking();
    }

    fn set_character_protection(&mut self, params: &[char]) {
        let Some(params) = params.strip_suffix(&['"']) else {
            return;
        };
        let value = parse_csi_params(params).first().copied().unwrap_or(0);
        self.style.protected = value == 1;
    }

    fn set_origin_mode(&mut self, enabled: bool) {
        self.modes.origin_mode = enabled;
        self.cursor_home();
    }

    fn set_auto_wrap(&mut self, enabled: bool) {
        self.modes.auto_wrap = enabled;
        if !enabled {
            self.pending_wrap = false;
        }
    }

    fn set_alternate_screen(&mut self, enabled: bool) {
        if enabled {
            if self.main_screen.is_some() {
                return;
            }

            self.bump_screen_identity_generation();
            let size = self.grid.size();
            self.main_screen = Some(self.screen_state());
            self.grid = TerminalGrid::new_with_seqno(size, self.seqno);
            self.clear_inline_images();
            self.last_kitty_placeholder = None;
            self.kitty_placeholder_cells.clear();
            self.cursor_row = 0;
            self.cursor_column = 0;
            self.pending_wrap = false;
            self.last_printable = None;
            self.nfc_last_printable_cell = None;
            self.saved_cursor = None;
            self.modes.cursor_visible = true;
            self.modes.origin_mode = false;
            self.modes.left_right_margin_mode = false;
            self.scroll_top = 0;
            self.scroll_bottom = size.rows.saturating_sub(1);
            self.left_margin = 0;
            self.right_margin = size.columns.saturating_sub(1);
            self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
        } else if let Some(screen) = self.main_screen.take() {
            self.bump_screen_identity_generation();
            self.restore_screen_state(screen);
            self.delete_orphan_kitty_relative_children();
            let size = self.grid.size();
            self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
        }
    }

    fn bump_screen_identity_generation(&mut self) {
        self.screen_identity_generation = self
            .screen_identity_generation
            .checked_add(1)
            .expect("terminal screen identity generation overflow");
    }

    fn push_inline_image(&mut self, image: ItermInlineImage) {
        let parent_identity = self.next_inline_image_parent_identity;
        self.next_inline_image_parent_identity = self
            .next_inline_image_parent_identity
            .checked_add(1)
            .expect("inline image parent identity overflow");
        self.inline_image_attachments
            .extend(cell_attachments_for_image(parent_identity, &image));
        self.inline_images.push(image);
        self.inline_image_parent_ids.push(parent_identity);
    }

    fn clear_inline_images(&mut self) {
        self.inline_images.clear();
        self.inline_image_parent_ids.clear();
        self.inline_image_attachments.clear();
        self.kitty_character_edited_placements.clear();
    }

    fn ensure_inline_image_parent_ids(&mut self) {
        while self.inline_image_parent_ids.len() < self.inline_images.len() {
            let parent_identity = self.next_inline_image_parent_identity;
            self.next_inline_image_parent_identity = self
                .next_inline_image_parent_identity
                .checked_add(1)
                .expect("inline image parent identity overflow");
            self.inline_image_parent_ids.push(parent_identity);
        }
        self.inline_image_parent_ids
            .truncate(self.inline_images.len());
    }

    fn retain_inline_images(&mut self, mut retain: impl FnMut(&ItermInlineImage) -> bool) {
        self.ensure_inline_image_parent_ids();
        let images = std::mem::take(&mut self.inline_images);
        let parent_ids = std::mem::take(&mut self.inline_image_parent_ids);
        let mut retained_images = Vec::with_capacity(images.len());
        let mut retained_parent_ids = Vec::with_capacity(parent_ids.len());
        for (image, parent_identity) in images.into_iter().zip(parent_ids) {
            if retain(&image) {
                retained_images.push(image);
                retained_parent_ids.push(parent_identity);
            }
        }
        let retained_parent_ids_set = retained_parent_ids.iter().copied().collect::<HashSet<_>>();
        self.inline_image_attachments
            .retain(|attachment| retained_parent_ids_set.contains(&attachment.parent_identity));
        self.inline_images = retained_images;
        self.inline_image_parent_ids = retained_parent_ids;
        self.retain_kitty_character_edited_placements();
    }

    fn retain_kitty_character_edited_placements(&mut self) {
        retain_kitty_character_edited_placements(
            &self.inline_images,
            &self.inline_image_parent_ids,
            &self.inline_image_attachments,
            &self.kitty_virtual_placements,
            &mut self.kitty_character_edited_placements,
        );
    }

    fn discard_kitty_character_edited_placements(
        &mut self,
        removed_placement_keys: &HashSet<KittyPlacementKey>,
    ) {
        self.kitty_character_edited_placements
            .retain(|key| !removed_placement_keys.contains(key));
        if let Some(screen) = self.main_screen.as_mut() {
            screen
                .kitty_character_edited_placements
                .retain(|key| !removed_placement_keys.contains(key));
        }
    }

    fn screen_state(&self) -> ScreenState {
        ScreenState {
            grid: self.grid.clone(),
            inline_images: self.inline_images.clone(),
            inline_image_parent_ids: self.inline_image_parent_ids.clone(),
            inline_image_attachments: self.inline_image_attachments.clone(),
            kitty_character_edited_placements: self.kitty_character_edited_placements.clone(),
            last_kitty_placeholder: self.last_kitty_placeholder,
            kitty_placeholder_cells: self.kitty_placeholder_cells.clone(),
            cursor_row: self.cursor_row,
            cursor_column: self.cursor_column,
            pending_wrap: self.pending_wrap,
            clear_semantic_type_on_movement: self.clear_semantic_type_on_movement,
            last_printable: self.last_printable.clone(),
            nfc_last_printable_cell: self.nfc_last_printable_cell,
            saved_cursor: self.saved_cursor.clone(),
            modes: self.modes,
            scroll_top: self.scroll_top,
            scroll_bottom: self.scroll_bottom,
            left_margin: self.left_margin,
            right_margin: self.right_margin,
            character_set: self.character_set,
            style: self.style.clone(),
        }
    }

    fn restore_screen_state(&mut self, screen: ScreenState) {
        self.grid = screen.grid;
        self.inline_images = screen.inline_images;
        self.inline_image_parent_ids = screen.inline_image_parent_ids;
        self.inline_image_attachments = screen.inline_image_attachments;
        self.kitty_character_edited_placements = screen.kitty_character_edited_placements;
        self.last_kitty_placeholder = screen.last_kitty_placeholder;
        self.kitty_placeholder_cells = screen.kitty_placeholder_cells;
        self.cursor_row = screen.cursor_row;
        self.cursor_column = screen.cursor_column;
        self.pending_wrap = screen.pending_wrap;
        self.clear_semantic_type_on_movement = screen.clear_semantic_type_on_movement;
        self.last_printable = screen.last_printable;
        self.nfc_last_printable_cell = screen.nfc_last_printable_cell;
        self.saved_cursor = screen.saved_cursor;
        self.modes = screen.modes;
        self.scroll_top = screen.scroll_top;
        self.scroll_bottom = screen.scroll_bottom;
        self.left_margin = screen.left_margin;
        self.right_margin = screen.right_margin;
        self.character_set = screen.character_set;
        self.style = screen.style;
        self.clamp_to_size();
    }

    fn clamp_to_size(&mut self) {
        let size = self.grid.size();
        self.cursor_row = clamp_axis(self.cursor_row, size.rows);
        self.cursor_column = clamp_axis(self.cursor_column, size.columns);
        self.scroll_top = clamp_axis(self.scroll_top, size.rows);
        self.scroll_bottom = clamp_axis(self.scroll_bottom, size.rows);
        self.left_margin = clamp_axis(self.left_margin, size.columns);
        self.right_margin = clamp_axis(self.right_margin, size.columns);
        if self.scroll_top >= self.scroll_bottom {
            self.scroll_top = 0;
            self.scroll_bottom = size.rows.saturating_sub(1);
        }
        if self.left_margin >= self.right_margin {
            self.left_margin = 0;
            self.right_margin = size.columns.saturating_sub(1);
        }
        if size.columns == 0 || size.rows == 0 {
            self.pending_wrap = false;
        }
    }

    fn cursor_home(&mut self) {
        self.pending_wrap = false;
        self.cursor_column = if self.modes.origin_mode && self.modes.left_right_margin_mode {
            self.left_margin
        } else {
            0
        };
        self.cursor_row = if self.modes.origin_mode {
            self.scroll_top.min(self.grid.size().rows.saturating_sub(1))
        } else {
            0
        };
        self.clear_semantic_type_due_to_movement();
    }

    fn set_left_right_margin_mode(&mut self, enabled: bool) {
        self.modes.left_right_margin_mode = enabled;
        if !enabled {
            self.left_margin = 0;
            self.right_margin = self.grid.size().columns.saturating_sub(1);
        }
    }

    fn set_scroll_region(&mut self, params: &[char]) {
        let rows = self.grid.size().rows;
        if rows == 0 {
            return;
        }

        let values = parse_csi_params(params);
        let top = param_or_one(values.first().copied()).saturating_sub(1);
        let bottom = values
            .get(1)
            .copied()
            .filter(|value| *value != 0)
            .unwrap_or(rows)
            .saturating_sub(1)
            .min(rows - 1);

        if top >= bottom {
            return;
        }

        self.scroll_top = top;
        self.scroll_bottom = bottom;
        self.cursor_home();
    }

    fn set_left_right_margins(&mut self, params: &[char]) {
        let columns = self.grid.size().columns;
        if columns == 0 {
            return;
        }

        let values = parse_csi_params(params);
        let left = param_or_one(values.first().copied()).saturating_sub(1);
        let right = values
            .get(1)
            .copied()
            .filter(|value| *value != 0)
            .unwrap_or(columns)
            .saturating_sub(1)
            .min(columns - 1);

        if left >= right {
            return;
        }

        self.left_margin = left;
        self.right_margin = right;
        self.cursor_home();
    }

    fn save_cursor(&mut self) {
        self.saved_cursor = Some(SavedCursor {
            row: self.cursor_row,
            column: self.cursor_column,
            pending_wrap: self.pending_wrap,
            clear_semantic_type_on_movement: self.clear_semantic_type_on_movement,
            origin_mode: self.modes.origin_mode,
            character_set: self.character_set,
            style: self.style.clone(),
        });
    }

    fn restore_cursor(&mut self) {
        let Some(saved) = self.saved_cursor.clone() else {
            return;
        };

        self.modes.origin_mode = saved.origin_mode;
        self.character_set = saved.character_set;
        self.style = saved.style;

        let size = self.grid.size();
        if size.rows == 0 || size.columns == 0 {
            self.cursor_row = 0;
            self.cursor_column = 0;
            self.pending_wrap = false;
            return;
        }

        self.cursor_row = saved.row.min(size.rows - 1);
        self.cursor_column = saved.column.min(size.columns - 1);
        self.pending_wrap = saved.pending_wrap;
        self.clear_semantic_type_on_movement = saved.clear_semantic_type_on_movement;
    }

    fn move_cursor_up(&mut self, count: u16) {
        self.pending_wrap = false;
        let previous_row = self.cursor_row;
        self.cursor_row = self.cursor_row.saturating_sub(count);
        if self.cursor_row != previous_row {
            self.clear_semantic_type_due_to_movement();
        }
    }

    fn move_cursor_down(&mut self, count: u16) {
        self.pending_wrap = false;
        let rows = self.grid.size().rows;
        if rows == 0 {
            return;
        }

        let previous_row = self.cursor_row;
        self.cursor_row = self.cursor_row.saturating_add(count).min(rows - 1);
        if self.cursor_row != previous_row {
            self.clear_semantic_type_due_to_movement();
        }
    }

    fn move_cursor_forward(&mut self, count: u16) {
        self.pending_wrap = false;
        let columns = self.grid.size().columns;
        if columns == 0 {
            return;
        }

        self.cursor_column = self.cursor_column.saturating_add(count).min(columns - 1);
    }

    fn move_cursor_back(&mut self, count: u16) {
        self.pending_wrap = false;
        self.cursor_column = self.cursor_column.saturating_sub(count);
    }

    fn move_cursor_next_line(&mut self, count: u16) {
        self.move_cursor_down(count);
        self.cursor_column = 0;
    }

    fn move_cursor_previous_line(&mut self, count: u16) {
        self.move_cursor_up(count);
        self.cursor_column = 0;
    }

    fn position_cursor(&mut self, params: &[char]) {
        self.pending_wrap = false;
        let values = parse_csi_params(params);
        let rows = self.grid.size().rows;
        let columns = self.grid.size().columns;

        if rows == 0 || columns == 0 {
            return;
        }

        let row = self.cursor_row_from_position_param(param_or_one(values.first().copied()));
        let column = self.cursor_column_from_position_param(param_or_one(values.get(1).copied()));

        let previous_row = self.cursor_row;
        self.cursor_row = row;
        self.cursor_column = column;
        if self.cursor_row != previous_row {
            self.clear_semantic_type_due_to_movement();
        }
    }

    fn cursor_row_from_position_param(&self, param: u16) -> u16 {
        let row = param.saturating_sub(1);
        let rows = self.grid.size().rows;
        if rows == 0 {
            return 0;
        }

        if self.modes.origin_mode {
            let top = self.scroll_top.min(rows - 1);
            let bottom = self.scroll_bottom.min(rows - 1);
            top.saturating_add(row).min(bottom)
        } else {
            row.min(rows - 1)
        }
    }

    fn cursor_column_from_position_param(&self, param: u16) -> u16 {
        let column = param.saturating_sub(1);
        let columns = self.grid.size().columns;
        if columns == 0 {
            return 0;
        }

        if self.modes.origin_mode && self.modes.left_right_margin_mode {
            self.left_margin
                .saturating_add(column)
                .min(self.right_margin)
        } else {
            column.min(columns - 1)
        }
    }

    fn position_cursor_column(&mut self, params: &[char]) {
        self.pending_wrap = false;
        let columns = self.grid.size().columns;
        if columns == 0 {
            return;
        }

        let column = csi_count(params).saturating_sub(1);
        self.cursor_column = column.min(columns - 1);
    }

    fn position_cursor_row(&mut self, params: &[char]) {
        self.pending_wrap = false;
        let rows = self.grid.size().rows;
        if rows == 0 {
            return;
        }

        let row = csi_count(params).saturating_sub(1);
        let previous_row = self.cursor_row;
        self.cursor_row = row.min(rows - 1);
        if self.cursor_row != previous_row {
            self.clear_semantic_type_due_to_movement();
        }
    }

    fn erase_display(&mut self, mode: u16, selective: bool) {
        if mode == 3 {
            self.prune_scrollback_rows(self.scrollback.len());
            return;
        }

        let size = self.grid.size();
        if size.rows == 0 || size.columns == 0 {
            return;
        }

        match mode {
            0 => {
                self.clear_cells(self.cursor_row, self.cursor_column, size.columns, selective);
                for row in self.cursor_row.saturating_add(1)..size.rows {
                    self.clear_cells(row, 0, size.columns, selective);
                }
            }
            1 => {
                for row in 0..self.cursor_row {
                    self.clear_cells(row, 0, size.columns, selective);
                }
                self.clear_cells(
                    self.cursor_row,
                    0,
                    self.cursor_column.saturating_add(1).min(size.columns),
                    selective,
                );
            }
            2 => {
                if !selective {
                    self.bump_screen_identity_generation();
                }
                for row in 0..size.rows {
                    self.clear_cells(row, 0, size.columns, selective);
                }
                if !selective {
                    self.delete_visible_inline_images();
                }
            }
            _ => {}
        }
    }

    fn rebase_inline_images_after_history_prune(&mut self, removed_rows: usize) {
        if removed_rows == 0 {
            return;
        }
        let has_dormant_main = self.main_screen.is_some();
        if let Some(main_screen) = self.main_screen.as_mut() {
            rebase_image_and_placeholder_metadata(
                &mut main_screen.inline_images,
                &mut main_screen.inline_image_parent_ids,
                &mut main_screen.inline_image_attachments,
                &mut main_screen.kitty_placeholder_cells,
                &mut main_screen.last_kitty_placeholder,
                removed_rows,
            );
            retain_kitty_character_edited_placements(
                &main_screen.inline_images,
                &main_screen.inline_image_parent_ids,
                &main_screen.inline_image_attachments,
                &self.kitty_virtual_placements,
                &mut main_screen.kitty_character_edited_placements,
            );
        }
        rebase_image_and_placeholder_metadata(
            &mut self.inline_images,
            &mut self.inline_image_parent_ids,
            &mut self.inline_image_attachments,
            &mut self.kitty_placeholder_cells,
            &mut self.last_kitty_placeholder,
            removed_rows,
        );
        self.retain_kitty_character_edited_placements();
        if !has_dormant_main {
            self.delete_orphan_kitty_relative_children();
        }
    }

    fn delete_visible_inline_images(&mut self) {
        let first_row = self.scrollback.len();
        let last_row = first_row.saturating_add(usize::from(self.grid.size().rows));
        let columns = self.grid.size().columns;
        self.retain_inline_images(|image| {
            !inline_image_intersects_region(image, first_row, last_row, 0, columns)
        });
        self.delete_orphan_kitty_relative_children();
    }

    fn erase_line(&mut self, mode: u16, selective: bool) {
        let columns = self.grid.size().columns;
        if self.cursor_row >= self.grid.size().rows || columns == 0 {
            return;
        }

        match mode {
            0 => self.clear_cells(self.cursor_row, self.cursor_column, columns, selective),
            1 => self.clear_cells(
                self.cursor_row,
                0,
                self.cursor_column.saturating_add(1).min(columns),
                selective,
            ),
            2 => self.clear_cells(self.cursor_row, 0, columns, selective),
            _ => {}
        }
    }

    fn insert_lines(&mut self, count: u16) {
        self.pending_wrap = false;
        if !self.cursor_within_horizontal_margins() {
            return;
        }
        let Some((top, bottom)) = self.active_scroll_range_from_cursor() else {
            return;
        };

        self.scroll_down_with_horizontal_margins(top, bottom, count);
    }

    fn delete_lines(&mut self, count: u16) {
        self.pending_wrap = false;
        if !self.cursor_within_horizontal_margins() {
            return;
        }
        let Some((top, bottom)) = self.active_scroll_range_from_cursor() else {
            return;
        };

        self.scroll_up_with_horizontal_margins(top, bottom, count);
    }

    fn scroll_up(&mut self, count: u16) {
        self.pending_wrap = false;
        let Some((top, bottom)) = self.active_scroll_range() else {
            return;
        };

        self.scroll_up_with_horizontal_margins(top, bottom, count);
    }

    fn scroll_down(&mut self, count: u16) {
        self.pending_wrap = false;
        let Some((top, bottom)) = self.active_scroll_range() else {
            return;
        };

        self.scroll_down_with_horizontal_margins(top, bottom, count);
    }

    fn active_scroll_range(&self) -> Option<(u16, u16)> {
        let size = self.grid.size();
        if size.rows == 0 || size.columns == 0 {
            return None;
        }

        let scroll_top = self.scroll_top.min(size.rows - 1);
        let scroll_bottom = self.scroll_bottom.min(size.rows - 1);
        if scroll_top > scroll_bottom {
            return None;
        }

        Some((scroll_top, scroll_bottom))
    }

    fn cursor_within_horizontal_margins(&self) -> bool {
        !self.modes.left_right_margin_mode
            || (self.cursor_column >= self.left_margin && self.cursor_column <= self.right_margin)
    }

    fn cursor_within_vertical_margins(&self) -> bool {
        self.cursor_row >= self.scroll_top && self.cursor_row <= self.scroll_bottom
    }

    fn character_right_boundary(&self) -> u16 {
        let physical_right = self.grid.size().columns.saturating_sub(1);
        if self.modes.left_right_margin_mode && self.cursor_within_horizontal_margins() {
            self.right_margin.min(physical_right)
        } else {
            physical_right
        }
    }

    fn active_scroll_range_from_cursor(&self) -> Option<(u16, u16)> {
        let size = self.grid.size();
        if size.rows == 0 || size.columns == 0 {
            return None;
        }

        let scroll_top = self.scroll_top.min(size.rows - 1);
        let scroll_bottom = self.scroll_bottom.min(size.rows - 1);
        if self.cursor_row < scroll_top || self.cursor_row > scroll_bottom {
            return None;
        }

        Some((self.cursor_row, scroll_bottom))
    }

    fn scroll_down_region(&mut self, top: u16, bottom: u16, count: u16) {
        let size = self.grid.size();
        if size.rows == 0 || size.columns == 0 || top > bottom || count == 0 {
            return;
        }

        let height = bottom - top + 1;
        let count = count.min(height);
        self.scroll_inline_images_down_region(top, bottom, count);
        self.scroll_kitty_placeholder_cells_down_region(top, bottom, count);
        let blank = self.blank_cell();
        self.grid
            .scroll_down_rows(top, bottom, count, &blank, self.seqno);
        for row in top..=bottom {
            self.grid.set_row_last_change_seqno(row, self.seqno);
        }

        self.record_damage(DamageRegion::new(0, top, size.columns, height));
    }

    fn bounded_horizontal_scroll_columns(&self) -> Option<(u16, u16)> {
        let columns = self.grid.size().columns;
        if !self.modes.left_right_margin_mode || columns == 0 {
            return None;
        }

        let left = self.left_margin.min(columns - 1);
        let right = self.right_margin.min(columns - 1);
        (left < right && (left != 0 || right != columns - 1)).then_some((left, right))
    }

    fn scroll_down_with_horizontal_margins(&mut self, top: u16, bottom: u16, count: u16) {
        let size = self.grid.size();
        if size.rows == 0 || size.columns == 0 || top > bottom || count == 0 {
            return;
        }

        let height = bottom - top + 1;
        let count = count.min(height);
        let Some((left, right)) = self.bounded_horizontal_scroll_columns() else {
            self.scroll_down_region(top, bottom, count);
            return;
        };

        let graphics_retired =
            self.retire_malformed_graphics_in_bounded_scroll_region(top, bottom, left, right);
        let attachment_overflow_changed = self.apply_cell_transform(CellTransform::ScrollDown {
            top: self.scrollback.len().saturating_add(usize::from(top)),
            bottom: self.scrollback.len().saturating_add(usize::from(bottom)),
            count: usize::from(count),
            left,
            right,
        });
        self.scroll_down_bounded_cells(top, bottom, count, left, right);
        for row in top..=bottom {
            self.grid.set_row_last_change_seqno(row, self.seqno);
        }
        self.record_damage(DamageRegion::new(left, top, right - left + 1, height));
        if graphics_retired || attachment_overflow_changed {
            self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
        }
    }

    fn scroll_up_with_horizontal_margins(&mut self, top: u16, bottom: u16, count: u16) {
        let size = self.grid.size();
        if size.rows == 0 || size.columns == 0 || top > bottom || count == 0 {
            return;
        }

        let height = bottom - top + 1;
        let count = count.min(height);
        let Some((left, right)) = self.bounded_horizontal_scroll_columns() else {
            self.scroll_up_region_by(top, bottom, count);
            return;
        };

        let graphics_retired =
            self.retire_malformed_graphics_in_bounded_scroll_region(top, bottom, left, right);
        let attachment_overflow_changed = self.apply_cell_transform(CellTransform::ScrollUp {
            top: self.scrollback.len().saturating_add(usize::from(top)),
            bottom: self.scrollback.len().saturating_add(usize::from(bottom)),
            count: usize::from(count),
            left,
            right,
        });
        self.scroll_up_bounded_cells(top, bottom, count, left, right);
        for row in top..=bottom {
            self.grid.set_row_last_change_seqno(row, self.seqno);
        }
        self.record_damage(DamageRegion::new(left, top, right - left + 1, height));
        if graphics_retired || attachment_overflow_changed {
            self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
        }
    }

    fn scroll_down_bounded_cells(
        &mut self,
        top: u16,
        bottom: u16,
        count: u16,
        left: u16,
        right: u16,
    ) {
        let height = bottom - top + 1;
        if count < height {
            let shift_bottom = bottom - count;
            self.record_scrolled_survivor_cell_clones(height - count, right - left + 1);
            for row in (top..=shift_bottom).rev() {
                for column in left..=right {
                    let cell = self.grid.get(row, column).cloned().unwrap_or_default();
                    self.grid.set(row + count, column, cell);
                }
            }
        }

        for row in top..top + count {
            for column in left..=right {
                self.grid.set(row, column, self.blank_cell());
            }
        }
    }

    fn retire_graphics_in_bounded_scroll_region(
        &mut self,
        top: u16,
        bottom: u16,
        left: u16,
        right: u16,
    ) -> bool {
        let first_row = self.scrollback.len().saturating_add(usize::from(top));
        let last_row = self
            .scrollback
            .len()
            .saturating_add(usize::from(bottom))
            .saturating_add(1);
        let last_column = right.saturating_add(1);
        let inline_image_count = self.inline_images.len();
        let placeholder_count = self.kitty_placeholder_cells.len();
        self.retain_inline_images(|image| {
            !inline_image_intersects_region(image, first_row, last_row, left, last_column)
        });
        self.kitty_placeholder_cells.retain(|(row, column), _| {
            !(*row >= first_row && *row < last_row && *column >= left && *column <= right)
        });
        let last_placeholder_retired = self.last_kitty_placeholder.is_some_and(|placeholder| {
            placeholder.row >= first_row
                && placeholder.row < last_row
                && placeholder.column >= left
                && placeholder.column <= right
        });
        if last_placeholder_retired {
            self.last_kitty_placeholder = None;
        }
        self.retire_orphan_kitty_relative_children();
        let stale_placeholder_caches_retired = self.retire_stale_kitty_placeholder_caches();
        self.inline_images.len() != inline_image_count
            || self.kitty_placeholder_cells.len() != placeholder_count
            || last_placeholder_retired
            || stale_placeholder_caches_retired
    }

    /// Retire only placements that cannot be represented by persistent logical
    /// attachments.  Valid attachment-backed placements remain in storage even
    /// when a transform blanks their final cell; attachment absence means an
    /// empty render, not a legacy whole-placement fallback.
    fn retire_malformed_graphics_in_bounded_scroll_region(
        &mut self,
        top: u16,
        bottom: u16,
        left: u16,
        right: u16,
    ) -> bool {
        let first_row = self.scrollback.len().saturating_add(usize::from(top));
        let last_row = self
            .scrollback
            .len()
            .saturating_add(usize::from(bottom))
            .saturating_add(1);
        let last_column = right.saturating_add(1);
        let inline_image_count = self.inline_images.len();
        self.retain_inline_images(|image| {
            cell_attachment_dimensions(image).is_some()
                || !inline_image_intersects_region(image, first_row, last_row, left, last_column)
        });
        if self.inline_images.len() == inline_image_count {
            return false;
        }

        self.kitty_placeholder_cells.retain(|(row, column), _| {
            !(*row >= first_row && *row < last_row && *column >= left && *column <= right)
        });
        if self.last_kitty_placeholder.is_some_and(|placeholder| {
            placeholder.row >= first_row
                && placeholder.row < last_row
                && placeholder.column >= left
                && placeholder.column <= right
        }) {
            self.last_kitty_placeholder = None;
        }
        self.retire_orphan_kitty_relative_children();
        self.retire_stale_kitty_placeholder_caches();
        true
    }

    /// Returns whether a moved or deleted attachment has a nonzero target
    /// offset. Such an attachment can paint outside its source LR cell, so a
    /// rectangular cell damage region cannot erase every old pixel safely.
    fn apply_cell_transform(&mut self, transform: CellTransform) -> bool {
        self.ensure_inline_image_parent_ids();
        let offset_parent_ids = self
            .inline_images
            .iter()
            .zip(self.inline_image_parent_ids.iter())
            .filter_map(|(image, parent_identity)| {
                (image.target_x.unwrap_or(0) != 0 || image.target_y.unwrap_or(0) != 0)
                    .then_some(*parent_identity)
            })
            .collect::<HashSet<_>>();
        let virtual_placement_keys_by_parent_id = if transform.is_character_edit() {
            self.inline_images
                .iter()
                .zip(self.inline_image_parent_ids.iter())
                .filter_map(|(image, parent_identity)| {
                    kitty_image_placement_key(image)
                        .filter(|key| self.kitty_virtual_placements.contains_key(key))
                        .map(|key| (*parent_identity, key))
                })
                .collect::<HashMap<_, _>>()
        } else {
            HashMap::new()
        };
        let mut attachment_overflow_changed = false;
        let mut character_edited_placements = HashSet::new();
        self.inline_image_attachments = self
            .inline_image_attachments
            .drain(..)
            .filter_map(|attachment| {
                let transformed = transform.apply(attachment);
                if transformed != Some(attachment) {
                    if offset_parent_ids.contains(&attachment.parent_identity) {
                        attachment_overflow_changed = true;
                    }
                    if let Some(placement_key) =
                        virtual_placement_keys_by_parent_id.get(&attachment.parent_identity)
                    {
                        character_edited_placements.insert(*placement_key);
                    }
                }
                transformed
            })
            .collect();
        if transform.is_character_edit() {
            self.synchronize_kitty_image_origins_with_attachments();
        }
        self.kitty_character_edited_placements
            .extend(character_edited_placements);
        self.retain_kitty_character_edited_placements();
        attachment_overflow_changed
    }

    /// `ItermInlineImage` stores a single placement origin, while a bounded
    /// character edit can split its cell footprint. For Kitty placements with
    /// surviving attachments, the top-left live attachment is the only exact
    /// screen-coordinate origin representable by that legacy metadata.
    fn synchronize_kitty_image_origins_with_attachments(&mut self) {
        let mut origins = HashMap::<u64, (usize, u16)>::new();
        for attachment in &self.inline_image_attachments {
            origins
                .entry(attachment.parent_identity)
                .and_modify(|origin| *origin = (*origin).min((attachment.row, attachment.column)))
                .or_insert((attachment.row, attachment.column));
        }
        for (image, parent_identity) in self
            .inline_images
            .iter_mut()
            .zip(self.inline_image_parent_ids.iter())
        {
            if image.kitty_image_id.is_some()
                && let Some((row, column)) = origins.get(parent_identity).copied()
            {
                image.row = row;
                image.column = column;
            }
        }
    }

    fn retire_stale_kitty_placeholder_caches(&mut self) -> bool {
        let live_placements = self
            .inline_images
            .iter()
            .filter_map(|image| {
                image
                    .kitty_image_id
                    .map(|image_id| (image_id, image.kitty_placement_id))
            })
            .collect::<HashSet<_>>();
        let placeholder_count = self.kitty_placeholder_cells.len();
        self.kitty_placeholder_cells.retain(|_, placeholder| {
            kitty_placeholder_references_live_placement(*placeholder, &live_placements)
        });
        let last_placeholder_retired = self.last_kitty_placeholder.is_some_and(|placeholder| {
            !kitty_placeholder_references_live_placement(placeholder, &live_placements)
        });
        if last_placeholder_retired {
            self.last_kitty_placeholder = None;
        }

        self.kitty_placeholder_cells.len() != placeholder_count || last_placeholder_retired
    }

    fn scroll_inline_images_down_region(&mut self, top: u16, bottom: u16, count: u16) {
        if count == 0 || top > bottom {
            return;
        }

        let first_row = self.scrollback.len().saturating_add(usize::from(top));
        let last_row = self
            .scrollback
            .len()
            .saturating_add(usize::from(bottom))
            .saturating_add(1);
        let count = usize::from(count);
        self.ensure_inline_image_parent_ids();
        let images = std::mem::take(&mut self.inline_images);
        let parent_ids = std::mem::take(&mut self.inline_image_parent_ids);
        let mut shifted_images = Vec::with_capacity(self.inline_images.len());
        let mut shifted_parent_ids = Vec::with_capacity(parent_ids.len());
        let mut moved_parent_ids = HashSet::new();

        for (mut image, parent_identity) in images.into_iter().zip(parent_ids) {
            let (image_top, image_bottom) = kitty_image_row_range(&image);
            if image_bottom <= first_row || image_top >= last_row {
                shifted_images.push(image);
                shifted_parent_ids.push(parent_identity);
                continue;
            }

            if image_top < first_row || image_bottom > last_row {
                continue;
            }

            let Some(new_row) = image.row.checked_add(count) else {
                continue;
            };
            let image_height = image_bottom.saturating_sub(image_top);
            if new_row < first_row || new_row.saturating_add(image_height) > last_row {
                continue;
            }

            image.row = new_row;
            shifted_images.push(image);
            shifted_parent_ids.push(parent_identity);
            moved_parent_ids.insert(parent_identity);
        }

        self.inline_images = shifted_images;
        self.inline_image_parent_ids = shifted_parent_ids;
        let live_parent_ids = self
            .inline_image_parent_ids
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        self.inline_image_attachments.retain_mut(|attachment| {
            if !live_parent_ids.contains(&attachment.parent_identity) {
                return false;
            }
            if moved_parent_ids.contains(&attachment.parent_identity) {
                let Some(row) = attachment.row.checked_add(count) else {
                    return false;
                };
                attachment.row = row;
            }
            true
        });
        self.delete_orphan_kitty_relative_children();
    }

    fn scroll_kitty_placeholder_cells_down_region(&mut self, top: u16, bottom: u16, count: u16) {
        if count == 0 || top > bottom {
            return;
        }

        let first_row = self.scrollback.len().saturating_add(usize::from(top));
        let last_row = self
            .scrollback
            .len()
            .saturating_add(usize::from(bottom))
            .saturating_add(1);
        let count = usize::from(count);
        let mut shifted = HashMap::new();

        for ((row, column), mut placeholder) in self.kitty_placeholder_cells.drain() {
            if row < first_row || row >= last_row {
                shifted.insert((row, column), placeholder);
                continue;
            }
            let Some(new_row) = row.checked_add(count) else {
                continue;
            };
            if new_row >= last_row {
                continue;
            }
            placeholder.row = new_row;
            shifted.insert((new_row, column), placeholder);
        }

        self.kitty_placeholder_cells = shifted;
        self.last_kitty_placeholder = None;
    }

    fn scroll_inline_images_up_region(&mut self, top: u16, bottom: u16, count: u16) {
        if count == 0 || top > bottom {
            return;
        }

        let first_row = self.scrollback.len().saturating_add(usize::from(top));
        let last_row = self
            .scrollback
            .len()
            .saturating_add(usize::from(bottom))
            .saturating_add(1);
        let count = usize::from(count);
        self.ensure_inline_image_parent_ids();
        let images = std::mem::take(&mut self.inline_images);
        let parent_ids = std::mem::take(&mut self.inline_image_parent_ids);
        let mut shifted_images = Vec::with_capacity(self.inline_images.len());
        let mut shifted_parent_ids = Vec::with_capacity(parent_ids.len());
        let mut moved_parent_ids = HashSet::new();

        for (mut image, parent_identity) in images.into_iter().zip(parent_ids) {
            let (image_top, image_bottom) = kitty_image_row_range(&image);
            if image_bottom <= first_row || image_top >= last_row {
                shifted_images.push(image);
                shifted_parent_ids.push(parent_identity);
                continue;
            }

            if image_top < first_row || image_bottom > last_row {
                continue;
            }

            let Some(new_row) = image.row.checked_sub(count) else {
                continue;
            };
            let image_height = image_bottom.saturating_sub(image_top);
            if new_row < first_row || new_row.saturating_add(image_height) > last_row {
                continue;
            }

            image.row = new_row;
            shifted_images.push(image);
            shifted_parent_ids.push(parent_identity);
            moved_parent_ids.insert(parent_identity);
        }

        self.inline_images = shifted_images;
        self.inline_image_parent_ids = shifted_parent_ids;
        let live_parent_ids = self
            .inline_image_parent_ids
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        self.inline_image_attachments.retain_mut(|attachment| {
            if !live_parent_ids.contains(&attachment.parent_identity) {
                return false;
            }
            if moved_parent_ids.contains(&attachment.parent_identity) {
                let Some(row) = attachment.row.checked_sub(count) else {
                    return false;
                };
                attachment.row = row;
            }
            true
        });
        self.delete_orphan_kitty_relative_children();
    }

    fn scroll_kitty_placeholder_cells_up_region(&mut self, top: u16, bottom: u16, count: u16) {
        if count == 0 || top > bottom {
            return;
        }

        let first_row = self.scrollback.len().saturating_add(usize::from(top));
        let last_row = self
            .scrollback
            .len()
            .saturating_add(usize::from(bottom))
            .saturating_add(1);
        let count = usize::from(count);
        let mut shifted = HashMap::new();

        for ((row, column), mut placeholder) in self.kitty_placeholder_cells.drain() {
            if row < first_row || row >= last_row {
                shifted.insert((row, column), placeholder);
                continue;
            }
            let Some(new_row) = row.checked_sub(count) else {
                continue;
            };
            if new_row < first_row {
                continue;
            }
            placeholder.row = new_row;
            shifted.insert((new_row, column), placeholder);
        }

        self.kitty_placeholder_cells = shifted;
        self.last_kitty_placeholder = None;
    }

    fn scroll_up_region_by(&mut self, top: u16, bottom: u16, count: u16) {
        let size = self.grid.size();
        if size.rows == 0 || size.columns == 0 || top > bottom || count == 0 {
            return;
        }

        let height = bottom - top + 1;
        let count = count.min(height);
        let records_scrollback = self.should_record_scrollback_for_scroll(top, bottom);
        if records_scrollback {
            let suffix_start = self
                .scrollback
                .len()
                .checked_add(usize::from(bottom))
                .and_then(|row| row.checked_add(1))
                .expect("terminal metadata row overflow");
            if bottom.saturating_add(1) < size.rows {
                self.drop_inline_images_crossing_history_boundary(suffix_start);
            }
            self.shift_suffix_metadata_for_recorded_rows(suffix_start, usize::from(count));
        } else {
            self.scroll_inline_images_up_region(top, bottom, count);
            self.scroll_kitty_placeholder_cells_up_region(top, bottom, count);
        }

        let blank = self.blank_cell();
        if records_scrollback {
            let exiting_rows = self
                .grid
                .scroll_up_rows(top, bottom, count, &blank, self.seqno);
            for row in exiting_rows {
                self.scrollback.push(ScrollbackLine::from_grid_row(row));
            }
            self.trim_scrollback_to_limit();
        } else {
            self.grid
                .scroll_up_rows_discarding(top, bottom, count, &blank, self.seqno);
        }

        if records_scrollback {
            for row in bottom.saturating_add(1)..size.rows {
                self.grid.set_row_last_change_seqno(row, self.seqno);
            }
        } else {
            for row in top..=bottom {
                self.grid.set_row_last_change_seqno(row, self.seqno);
            }
        }

        self.record_damage(DamageRegion::new(0, top, size.columns, height));
    }

    fn scroll_up_bounded_cells(
        &mut self,
        top: u16,
        bottom: u16,
        count: u16,
        left: u16,
        right: u16,
    ) {
        let height = bottom - top + 1;
        if count < height {
            let shift_bottom = bottom - count;
            self.record_scrolled_survivor_cell_clones(height - count, right - left + 1);
            for row in top..=shift_bottom {
                for column in left..=right {
                    let cell = self
                        .grid
                        .get(row + count, column)
                        .cloned()
                        .unwrap_or_default();
                    self.grid.set(row, column, cell);
                }
            }
        }

        let blank_start = if count == height {
            top
        } else {
            bottom - count + 1
        };
        for row in blank_start..=bottom {
            for column in left..=right {
                self.grid.set(row, column, self.blank_cell());
            }
        }
    }

    fn record_scrolled_survivor_cell_clones(&mut self, rows: u16, columns: u16) {
        let clones = u64::from(rows).saturating_mul(u64::from(columns));
        self.work_counters.scrolled_survivor_cell_clones = self
            .work_counters
            .scrolled_survivor_cell_clones
            .saturating_add(clones);
    }

    fn drop_inline_images_crossing_history_boundary(&mut self, boundary: usize) {
        self.retain_inline_images(|image| {
            let (top, bottom) = kitty_image_row_range(image);
            !(top < boundary && bottom > boundary)
        });
        self.delete_orphan_kitty_relative_children();
    }

    fn shift_suffix_metadata_for_recorded_rows(&mut self, suffix_start: usize, rows: usize) {
        if rows == 0 {
            return;
        }
        debug_assert!(!self.alternate_screen_active());

        for row in &mut self.semantic_prompt_rows {
            if *row >= suffix_start {
                *row = row.checked_add(rows).expect("semantic prompt row overflow");
            }
        }
        for command in &mut self.semantic_command_exits {
            if command.row >= suffix_start {
                command.row = command
                    .row
                    .checked_add(rows)
                    .expect("semantic command row overflow");
            }
        }
        shift_image_and_placeholder_suffix_metadata(
            &mut self.inline_images,
            &mut self.inline_image_attachments,
            &mut self.kitty_placeholder_cells,
            &mut self.last_kitty_placeholder,
            suffix_start,
            rows,
        );
    }

    fn insert_blank_characters(&mut self, count: u16) {
        self.pending_wrap = false;
        if !self.cursor_within_horizontal_margins() || !self.cursor_within_vertical_margins() {
            return;
        }

        let right = self.character_right_boundary();
        self.cursor_column = self.grapheme_leader_column(self.cursor_column);
        self.insert_blank_characters_with_right_boundary(count, right, true);
    }

    fn insert_blank_characters_for_write(&mut self, count: u16) {
        let right = self.character_right_boundary();
        self.cursor_column = self.grapheme_leader_column(self.cursor_column);
        self.insert_blank_characters_with_right_boundary(count, right, false);
    }

    fn grapheme_leader_column(&self, column: u16) -> u16 {
        match self.grid.get(self.cursor_row, column).map(Cell::content) {
            Some(crate::CellContent::Continuation { leader_delta }) => {
                column.saturating_sub(u16::from(*leader_delta))
            }
            _ => column,
        }
    }

    fn insert_blank_characters_with_right_boundary(
        &mut self,
        count: u16,
        right: u16,
        transform_attachments: bool,
    ) {
        let size = self.grid.size();
        if self.cursor_row >= size.rows
            || self.cursor_column >= size.columns
            || self.cursor_column > right
            || count == 0
        {
            return;
        }

        let count = count.min(right - self.cursor_column + 1);
        let (graphics_retired, attachment_overflow_changed) = self
            .prepare_bounded_character_edit_attachments(
                right,
                transform_attachments.then_some(CellTransform::InsertCharacters {
                    row: self
                        .scrollback
                        .len()
                        .saturating_add(usize::from(self.cursor_row)),
                    column: self.cursor_column,
                    count,
                    right,
                }),
            );
        let shift_end = right + 1 - count;
        for column in (self.cursor_column..shift_end).rev() {
            let cell = self
                .grid
                .get(self.cursor_row, column)
                .cloned()
                .unwrap_or_default();
            self.set_grid_cell(self.cursor_row, column + count, cell);
        }

        for column in self.cursor_column..self.cursor_column + count {
            self.set_grid_cell(self.cursor_row, column, self.blank_cell());
        }
        self.sanitize_grapheme_spans(self.cursor_row);

        self.record_damage(DamageRegion::new(
            self.cursor_column,
            self.cursor_row,
            right - self.cursor_column + 1,
            1,
        ));
        if graphics_retired || attachment_overflow_changed {
            self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
        }
    }

    fn delete_characters(&mut self, count: u16) {
        self.pending_wrap = false;
        if !self.cursor_within_horizontal_margins() {
            return;
        }

        let right = self.character_right_boundary();
        let (start, count) = self.grapheme_aligned_delete_range(count, right);
        self.cursor_column = start;
        self.delete_characters_with_right_boundary(count, right);
    }

    fn grapheme_aligned_delete_range(&self, count: u16, right: u16) -> (u16, u16) {
        let requested_start = self.cursor_column;
        let requested_end = requested_start
            .saturating_add(count)
            .min(right.saturating_add(1));
        let start = match self
            .grid
            .get(self.cursor_row, requested_start)
            .map(Cell::content)
        {
            Some(crate::CellContent::Continuation { leader_delta }) => {
                requested_start.saturating_sub(u16::from(*leader_delta))
            }
            _ => requested_start,
        };
        let mut end = requested_end.max(start);
        let mut column = start;
        while column < end && column <= right {
            let Some(cell) = self.grid.get(self.cursor_row, column) else {
                break;
            };
            if let crate::CellContent::Continuation { leader_delta } = cell.content() {
                column = column.saturating_sub(u16::from(*leader_delta));
                continue;
            }
            let width = u16::from(cell.columns()).max(1);
            end = end.max(column.saturating_add(width).min(right.saturating_add(1)));
            column = column.saturating_add(width);
        }
        (start, end.saturating_sub(start))
    }

    fn delete_characters_with_right_boundary(&mut self, count: u16, right: u16) {
        let size = self.grid.size();
        if self.cursor_row >= size.rows
            || self.cursor_column >= size.columns
            || self.cursor_column > right
            || count == 0
        {
            return;
        }

        let count = count.min(right - self.cursor_column + 1);
        let (graphics_retired, attachment_overflow_changed) = self
            .prepare_bounded_character_edit_attachments(
                right,
                Some(CellTransform::DeleteCharacters {
                    row: self
                        .scrollback
                        .len()
                        .saturating_add(usize::from(self.cursor_row)),
                    column: self.cursor_column,
                    count,
                    right,
                }),
            );
        let shift_end = right + 1 - count;
        for column in self.cursor_column..shift_end {
            let cell = self
                .grid
                .get(self.cursor_row, column + count)
                .cloned()
                .unwrap_or_default();
            self.set_grid_cell(self.cursor_row, column, cell);
        }

        for column in shift_end..=right {
            self.set_grid_cell(self.cursor_row, column, self.blank_cell());
        }
        self.sanitize_grapheme_spans(self.cursor_row);

        self.record_damage(DamageRegion::new(
            self.cursor_column,
            self.cursor_row,
            right - self.cursor_column + 1,
            1,
        ));
        if graphics_retired || attachment_overflow_changed {
            self.record_damage(DamageRegion::new(0, 0, size.columns, size.rows));
        }
    }

    /// Applies the persistent-cell mapping only to CSI ICH/DCH under a narrow
    /// DECSLRM region.  Other character paths retain their established
    /// conservative graphics policy until they receive their own transform.
    fn prepare_bounded_character_edit_attachments(
        &mut self,
        right: u16,
        transform: Option<CellTransform>,
    ) -> (bool, bool) {
        if let Some(transform) =
            transform.filter(|_| self.bounded_horizontal_scroll_columns().is_some())
        {
            self.transform_kitty_placeholder_state_for_character_edit(transform);
            let graphics_retired = self.retire_malformed_graphics_in_bounded_scroll_region(
                self.cursor_row,
                self.cursor_row,
                self.cursor_column,
                right,
            );
            let attachment_overflow_changed = self.apply_cell_transform(transform);
            (graphics_retired, attachment_overflow_changed)
        } else {
            (self.bounded_character_edit_retires_graphics(right), false)
        }
    }

    /// Applies an ICH/DCH cell mapping to every Kitty placeholder coordinate.
    ///
    /// The map key and the embedded `row`/`column` are both terminal-cell
    /// coordinates. `rendered_*` names the placement origin produced for a
    /// pending diacritic sequence, so it must follow the same mapping too.
    /// Coordinates whose source cell is blanked are retired, just like their
    /// grid cells and persistent image attachments.
    fn transform_kitty_placeholder_state_for_character_edit(&mut self, transform: CellTransform) {
        debug_assert!(transform.is_character_edit());
        self.kitty_placeholder_cells = self
            .kitty_placeholder_cells
            .drain()
            .filter_map(|((row, column), mut placeholder)| {
                let (row, column) = transform.apply_coordinate(row, column)?;
                placeholder.row = row;
                placeholder.column = column;
                Some(((row, column), placeholder))
            })
            .collect();

        self.last_kitty_placeholder = self.last_kitty_placeholder.and_then(|mut placeholder| {
            let (row, column) = transform.apply_coordinate(placeholder.row, placeholder.column)?;
            placeholder.row = row;
            placeholder.column = column;
            Some(placeholder)
        });

        self.pending_kitty_placeholder =
            self.pending_kitty_placeholder
                .take()
                .and_then(|mut placeholder| {
                    let (row, column) =
                        transform.apply_coordinate(placeholder.row, placeholder.column)?;
                    placeholder.row = row;
                    placeholder.column = column;
                    if let Some((row, column)) =
                        placeholder.rendered_row.zip(placeholder.rendered_column)
                    {
                        if let Some((row, column)) = transform.apply_coordinate(row, column) {
                            placeholder.rendered_row = Some(row);
                            placeholder.rendered_column = Some(column);
                        } else {
                            placeholder.rendered_row = None;
                            placeholder.rendered_column = None;
                            placeholder.rendered_image_id = None;
                            placeholder.rendered_placement_id = None;
                        }
                    } else {
                        placeholder.rendered_row = None;
                        placeholder.rendered_column = None;
                        placeholder.rendered_image_id = None;
                        placeholder.rendered_placement_id = None;
                    }
                    Some(placeholder)
                });
    }

    fn bounded_character_edit_retires_graphics(&mut self, right: u16) -> bool {
        self.bounded_horizontal_scroll_columns().is_some_and(|_| {
            self.retire_graphics_in_bounded_scroll_region(
                self.cursor_row,
                self.cursor_row,
                self.cursor_column,
                right,
            )
        })
    }

    fn erase_characters(&mut self, count: u16) {
        self.pending_wrap = false;
        let size = self.grid.size();
        if self.cursor_row >= size.rows || self.cursor_column >= size.columns || count == 0 {
            return;
        }

        let count = count.min(size.columns - self.cursor_column);
        for column in self.cursor_column..self.cursor_column + count {
            self.set_grid_cell(self.cursor_row, column, self.blank_cell());
        }
        self.sanitize_grapheme_spans(self.cursor_row);

        self.record_damage(DamageRegion::new(
            self.cursor_column,
            self.cursor_row,
            count,
            1,
        ));
    }

    fn repeat_previous_character(&mut self, count: u16) {
        let Some(grapheme) = self.last_printable.clone() else {
            return;
        };

        for _ in 0..count {
            self.nfc_last_printable_cell = None;
            for ch in grapheme.chars() {
                self.write_char(ch);
            }
        }
    }

    fn clear_cells(&mut self, row: u16, start_column: u16, end_column: u16, selective: bool) {
        let columns = self.grid.size().columns;
        if row >= self.grid.size().rows || start_column >= columns {
            return;
        }

        let end_column = end_column.min(columns);
        if start_column >= end_column {
            return;
        }

        let source_row = self.scrollback.len().saturating_add(usize::from(row));
        if !selective {
            self.clear_kitty_placeholder_cells(source_row, start_column, end_column - start_column);
        }
        for column in start_column..end_column {
            if selective
                && self
                    .grid
                    .get(row, column)
                    .is_some_and(|cell| cell.protected)
            {
                continue;
            }
            if selective {
                self.clear_kitty_placeholder_cells(source_row, column, 1);
            }
            self.set_grid_cell(row, column, self.blank_cell());
        }
        self.sanitize_grapheme_spans(row);
        if start_column == 0 && end_column >= columns {
            self.set_grid_row_wrapped(row, false);
        }

        self.record_damage(DamageRegion::new(
            start_column,
            row,
            end_column - start_column,
            1,
        ));
    }

    fn blank_cell(&self) -> Cell {
        let mut cell = self.style.clone();
        cell.set_blank();
        cell
    }

    fn clear_grapheme_span_at(&mut self, row: u16, column: u16) {
        let Some(cell) = self.grid.get(row, column).cloned() else {
            return;
        };
        let leader = match cell.content() {
            crate::CellContent::Continuation { leader_delta } => {
                column.saturating_sub(u16::from(*leader_delta))
            }
            _ => column,
        };
        let width = self
            .grid
            .get(row, leader)
            .map_or(1, |cell| u16::from(cell.columns()).max(1));
        for offset in 0..width {
            let target = leader.saturating_add(offset);
            if target < self.grid.size().columns {
                self.set_grid_cell(row, target, self.blank_cell());
            }
        }
    }

    fn sanitize_grapheme_spans(&mut self, row: u16) {
        let columns = self.grid.size().columns;
        let mut column = 0;
        while column < columns {
            let Some(cell) = self.grid.get(row, column).cloned() else {
                break;
            };
            if cell.is_continuation() {
                self.set_grid_cell(row, column, self.blank_cell());
                column += 1;
                continue;
            }
            let width = u16::from(cell.columns()).max(1);
            let valid = width == 1
                || column.saturating_add(width) <= columns
                    && (1..width).all(|offset| {
                        matches!(
                            self.grid.get(row, column + offset).map(Cell::content),
                            Some(crate::CellContent::Continuation { leader_delta })
                                if u16::from(*leader_delta) == offset
                        )
                    });
            if valid {
                column += width;
            } else {
                self.clear_grapheme_span_at(row, column);
                column += 1;
            }
        }
    }

    fn set_grid_cell(&mut self, row: u16, column: u16, cell: Cell) -> bool {
        if !self.grid.set(row, column, cell) {
            return false;
        }
        self.grid.set_row_last_change_seqno(row, self.seqno);
        true
    }

    fn set_grid_row_wrapped(&mut self, row: u16, wrapped: bool) {
        if self.grid.row_wrapped(row) == wrapped {
            return;
        }
        self.grid.set_row_wrapped(row, wrapped);
        self.grid.set_row_last_change_seqno(row, self.seqno);
    }

    fn apply_sgr(&mut self, params: &[char]) {
        let values = parse_sgr_params(params);
        let mut index = 0;

        while index < values.len() {
            match values[index] {
                SgrParameter::UnderlineStyle(style) => self.apply_underline_style(style),
                SgrParameter::Code(code) => match code {
                    0 => self.reset_style(),
                    1 => self.style.bold = true,
                    2 => self.style.faint = true,
                    3 => self.style.italic = true,
                    4 => self.apply_underline_style(UnderlineStyle::Single),
                    5 => {
                        self.style.blink = true;
                        self.style.rapid_blink = false;
                    }
                    6 => {
                        self.style.blink = true;
                        self.style.rapid_blink = true;
                    }
                    7 => self.style.inverse = true,
                    8 => self.style.conceal = true,
                    9 => self.style.strikethrough = true,
                    22 => {
                        self.style.bold = false;
                        self.style.faint = false;
                    }
                    21 => self.apply_underline_style(UnderlineStyle::Double),
                    23 => self.style.italic = false,
                    24 => self.apply_underline_style(UnderlineStyle::None),
                    25 => {
                        self.style.blink = false;
                        self.style.rapid_blink = false;
                    }
                    27 => self.style.inverse = false,
                    28 => self.style.conceal = false,
                    29 => self.style.strikethrough = false,
                    30..=37 => {
                        self.style.foreground = Color::Indexed(saturating_u8(code - 30));
                    }
                    39 => self.style.foreground = Color::Default,
                    40..=47 => {
                        self.style.background = Color::Indexed(saturating_u8(code - 40));
                    }
                    49 => self.style.background = Color::Default,
                    53 => self.style.overline = true,
                    55 => self.style.overline = false,
                    73 => self.style.vertical_align = crate::VerticalAlign::Superscript,
                    74 => self.style.vertical_align = crate::VerticalAlign::Subscript,
                    75 => self.style.vertical_align = crate::VerticalAlign::Baseline,
                    59 => self.style.underline_color = Color::Default,
                    90..=97 => {
                        self.style.foreground = Color::Indexed(saturating_u8(code - 90 + 8));
                    }
                    100..=107 => {
                        self.style.background = Color::Indexed(saturating_u8(code - 100 + 8));
                    }
                    38 | 48 | 58 => {
                        if let Some((color, consumed)) = parse_extended_color(&values[index + 1..])
                        {
                            match code {
                                38 => self.style.foreground = color,
                                48 => self.style.background = color,
                                58 => self.style.underline_color = color,
                                _ => {}
                            }
                            index += consumed;
                        }
                    }
                    _ => {}
                },
            }

            index += 1;
        }
    }

    fn apply_underline_style(&mut self, style: UnderlineStyle) {
        self.style.underline_style = style;
        self.style.underline = matches!(
            style,
            UnderlineStyle::Single
                | UnderlineStyle::Curly
                | UnderlineStyle::Dotted
                | UnderlineStyle::Dashed
        );
        self.style.double_underline = style == UnderlineStyle::Double;
    }

    fn reset_style(&mut self) {
        let hyperlink = self.style.hyperlink.take();
        let semantic_type = self.style.semantic_type;
        self.style = Cell::default();
        self.style.hyperlink = hyperlink;
        self.style.semantic_type = semantic_type;
    }

    fn record_damage(&mut self, region: DamageRegion) {
        if region.is_empty() {
            return;
        }

        if let Some(last) = self.damage.last_mut() {
            let adjacent =
                last.y == region.y && last.height == region.height && last.right() == region.x;

            if adjacent {
                last.width = last.width.saturating_add(region.width);
                return;
            }
        }

        let capacity = self.damage.capacity();
        self.damage.push(region);
        if self.damage.capacity() != capacity {
            self.feed_scratch.counters.damage_growths =
                self.feed_scratch.counters.damage_growths.saturating_add(1);
        }
    }

    fn map_graphic_character(&self, ch: char) -> char {
        match self.character_set {
            CharacterSet::Ascii => ch,
            CharacterSet::DecSpecialGraphics => map_dec_special_graphics(ch),
        }
    }
}

fn parse_g0_character_set(selector: char) -> Option<CharacterSet> {
    match selector {
        'B' => Some(CharacterSet::Ascii),
        '0' => Some(CharacterSet::DecSpecialGraphics),
        _ => None,
    }
}

fn map_dec_special_graphics(ch: char) -> char {
    match ch {
        '`' => '◆',
        'a' => '▒',
        'j' => '┘',
        'k' => '┐',
        'l' => '┌',
        'm' => '└',
        'n' => '┼',
        'o' => '⎺',
        'p' => '⎻',
        'q' => '─',
        'r' => '⎼',
        's' => '⎽',
        't' => '├',
        'u' => '┤',
        'v' => '┴',
        'w' => '┬',
        'x' => '│',
        '~' => '·',
        _ => ch,
    }
}

fn default_tab_stops(size: TerminalSize) -> Vec<u16> {
    let mut stops = Vec::new();
    let mut column = 8;
    while column < size.columns {
        stops.push(column);
        column = column.saturating_add(8);
    }
    stops
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "reflow atomically coordinates buffers, cursor state, and configuration inputs"
)]
fn reflow_main_screen(
    scrollback: &mut HistoryBuffer<ScrollbackLine>,
    grid: &mut TerminalGrid,
    size: TerminalSize,
    seqno: SequenceNo,
    unicode_version: u32,
    treat_east_asian_ambiguous_width_as_wide: bool,
    cell_width_overrides: &[CellWidthOverride],
    cursor_row: &mut u16,
    cursor_column: &mut u16,
) {
    let old_size = grid.size();
    let mut logical_lines = Vec::new();
    let mut logical_line = Vec::new();
    let mut logical_cursor_offset = None;
    let mut has_logical_line = false;

    for line in scrollback.iter() {
        if !line.is_wrapped() || !has_logical_line {
            if has_logical_line {
                logical_lines.push(ReflowLogicalLine {
                    cells: std::mem::take(&mut logical_line),
                    cursor_offset: logical_cursor_offset.take(),
                });
            }
            has_logical_line = true;
        }
        let mut cells = line.cells_with_reflow_overflow();
        trim_reflow_padding(
            &mut cells,
            unicode_version,
            treat_east_asian_ambiguous_width_as_wide,
            cell_width_overrides,
        );
        logical_line.extend(cells);
    }
    let mut grid_rows = (0..old_size.rows)
        .map(|row| {
            let physical_cells = grid.cells_with_reflow_overflow(row);
            let cursor_cells = (row == *cursor_row).then(|| {
                physical_cells
                    .iter()
                    .take(usize::from(*cursor_column).saturating_add(1))
                    .cloned()
                    .collect::<Vec<_>>()
            });
            let mut cells = physical_cells;
            trim_reflow_padding(
                &mut cells,
                unicode_version,
                treat_east_asian_ambiguous_width_as_wide,
                cell_width_overrides,
            );
            (row, cells, cursor_cells)
        })
        .collect::<Vec<_>>();
    let last_content_row = grid_rows
        .iter()
        .filter_map(|(row, cells, _)| (!cells.is_empty()).then_some(*row))
        .max();
    let relevant_grid_rows = last_content_row
        .map_or(0, |row| usize::from(row).saturating_add(1))
        .max(usize::from(*cursor_row).saturating_add(1))
        .min(usize::from(old_size.rows));
    grid_rows.truncate(relevant_grid_rows);
    for (row, mut cells, cursor_cells) in grid_rows {
        if !grid.row_wrapped(row) || !has_logical_line {
            if has_logical_line {
                logical_lines.push(ReflowLogicalLine {
                    cells: std::mem::take(&mut logical_line),
                    cursor_offset: logical_cursor_offset.take(),
                });
            }
            has_logical_line = true;
        }
        if row == *cursor_row {
            // Rewrap needs the original physical cursor offset, even when
            // the cells between content and the cursor are default padding.
            // Preserve that padding through the cursor before recording the
            // logical offset, just as the upstream rewrap uses the untrimmed
            // line length for cursor mapping.
            if let Some(cursor_cells) = cursor_cells
                && cells.len() < cursor_cells.len()
            {
                cells.extend(cursor_cells.into_iter().skip(cells.len()));
            }
            logical_cursor_offset = Some(
                logical_line
                    .len()
                    .saturating_add(usize::from(*cursor_column)),
            );
        }
        logical_line.extend(cells);
    }
    if has_logical_line {
        logical_lines.push(ReflowLogicalLine {
            cells: logical_line,
            cursor_offset: logical_cursor_offset,
        });
    }

    let mut rows = Vec::new();
    let mut reflowed_cursor = None;
    for logical_line in logical_lines {
        let row_offset = rows.len();
        let (reflowed_rows, cursor) = reflow_logical_line(
            &logical_line.cells,
            size.columns,
            unicode_version,
            treat_east_asian_ambiguous_width_as_wide,
            cell_width_overrides,
            logical_line.cursor_offset,
        );
        if let Some((row, column)) = cursor {
            reflowed_cursor = Some((row_offset.saturating_add(row), column));
        }
        rows.extend(
            reflowed_rows
                .into_iter()
                .enumerate()
                .map(|(index, row)| (row, index != 0)),
        );
    }

    let grid_rows = usize::from(size.rows);
    let scrollback_rows = rows.len().saturating_sub(grid_rows);
    let reflowed_scrollback = rows
        .drain(..scrollback_rows)
        .map(|(mut row, wrapped)| {
            row.cells.resize(usize::from(size.columns), Cell::default());
            ScrollbackLine::from_reflow_cells_wrapped(
                row.cells,
                row.reflow_overflow,
                wrapped,
                seqno,
            )
        })
        .collect::<Vec<_>>();

    let mut reflowed_grid = TerminalGrid::new_with_seqno(size, seqno);
    for (row, (mut reflowed_row, wrapped)) in rows.into_iter().enumerate() {
        let Ok(row) = u16::try_from(row) else {
            break;
        };
        reflowed_row
            .cells
            .resize(usize::from(size.columns), Cell::default());
        for (column, cell) in reflowed_row.cells.into_iter().enumerate() {
            let Ok(column) = u16::try_from(column) else {
                break;
            };
            reflowed_grid.set(row, column, cell);
        }
        reflowed_grid.set_reflow_overflow(row, reflowed_row.reflow_overflow);
        reflowed_grid.set_row_wrapped(row, wrapped);
        reflowed_grid.set_row_last_change_seqno(row, seqno);
    }

    scrollback.rebuild(reflowed_scrollback);
    *grid = reflowed_grid;
    if let Some((row, column)) = reflowed_cursor {
        let grid_row = row.saturating_sub(scrollback_rows);
        *cursor_row = u16::try_from(grid_row)
            .unwrap_or(u16::MAX)
            .min(size.rows.saturating_sub(1));
        *cursor_column = u16::try_from(column)
            .unwrap_or(u16::MAX)
            .min(size.columns.saturating_sub(1));
    }
}

#[derive(Debug)]
struct ReflowRow {
    cells: Vec<Cell>,
    reflow_overflow: Vec<Cell>,
}

#[derive(Debug)]
struct ReflowLogicalLine {
    cells: Vec<Cell>,
    cursor_offset: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct MainReflowOutcome {
    previous_history_rows: usize,
}

impl MainReflowOutcome {
    fn new(size: TerminalSize, scrollback_rows: usize) -> Self {
        Self {
            previous_history_rows: scrollback_rows
                .checked_add(usize::from(size.rows))
                .expect("terminal history row count overflow"),
        }
    }
}

fn retire_reflow_coordinate_state(screen: &mut ScreenState) {
    screen.inline_images.clear();
    screen.inline_image_parent_ids.clear();
    screen.inline_image_attachments.clear();
    screen.kitty_character_edited_placements.clear();
    screen.kitty_placeholder_cells.clear();
    screen.last_kitty_placeholder = None;
    screen.nfc_last_printable_cell = None;
    screen.saved_cursor = None;
}

fn retain_kitty_character_edited_placements(
    inline_images: &[ItermInlineImage],
    inline_image_parent_ids: &[u64],
    inline_image_attachments: &[CellAttachment],
    kitty_virtual_placements: &HashMap<KittyPlacementKey, KittyVirtualPlacement>,
    character_edited_placements: &mut HashSet<KittyPlacementKey>,
) {
    let virtual_parent_ids = inline_images
        .iter()
        .zip(inline_image_parent_ids.iter())
        .filter_map(|(image, parent_identity)| {
            kitty_image_placement_key(image)
                .filter(|key| kitty_virtual_placements.contains_key(key))
                .map(|key| (key, *parent_identity))
        })
        .collect::<HashMap<_, _>>();
    let live_attachment_parent_ids = inline_image_attachments
        .iter()
        .map(|attachment| attachment.parent_identity)
        .collect::<HashSet<_>>();
    character_edited_placements.retain(|key| {
        virtual_parent_ids
            .get(key)
            .is_some_and(|parent_identity| live_attachment_parent_ids.contains(parent_identity))
    });
}

fn trim_reflow_padding(
    cells: &mut Vec<Cell>,
    _unicode_version: u32,
    _treat_east_asian_ambiguous_width_as_wide: bool,
    _cell_width_overrides: &[CellWidthOverride],
) {
    let padding = cells
        .iter()
        .rev()
        .take_while(|cell| **cell == Cell::default())
        .count();
    if padding == 0 {
        return;
    }

    let content_end = cells.len().saturating_sub(padding);
    let continuation_cells = cells
        .get(content_end.saturating_sub(1))
        .map(|cell| u16::from(cell.columns()).saturating_sub(1))
        .map_or(0, usize::from)
        .min(padding);
    cells.truncate(content_end.saturating_add(continuation_cells));
}

fn reflow_logical_line(
    cells: &[Cell],
    columns: u16,
    _unicode_version: u32,
    _treat_east_asian_ambiguous_width_as_wide: bool,
    _cell_width_overrides: &[CellWidthOverride],
    cursor_offset: Option<usize>,
) -> (Vec<ReflowRow>, Option<(usize, usize)>) {
    if columns == 0 {
        return (
            vec![ReflowRow {
                cells: Vec::new(),
                reflow_overflow: Vec::new(),
            }],
            cursor_offset.map(|_| (0, 0)),
        );
    }

    if cells.is_empty() {
        return (
            vec![ReflowRow {
                cells: Vec::new(),
                reflow_overflow: Vec::new(),
            }],
            cursor_offset.map(|_| (0, 0)),
        );
    }

    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut row_overflow = Vec::new();
    let mut reflowed_cursor = None;
    let mut index = 0;
    while index < cells.len() {
        if cursor_offset == Some(index) {
            reflowed_cursor = Some((rows.len(), row.len()));
        }
        let cell = &cells[index];
        let cell_width = u16::from(cell.columns()).max(1);
        let cell_width = usize::from(cell_width);
        let source_width = cell_width.min(cells.len() - index);
        let output_width = cell_width.min(usize::from(columns));

        if !row.is_empty() && row.len().saturating_add(output_width) > usize::from(columns) {
            rows.push(ReflowRow {
                cells: std::mem::take(&mut row),
                reflow_overflow: std::mem::take(&mut row_overflow),
            });
        }

        if let Some(cursor_offset) = cursor_offset
            .filter(|cursor_offset| index < *cursor_offset && *cursor_offset < index + source_width)
        {
            reflowed_cursor = Some((
                rows.len(),
                row.len()
                    .saturating_add(cursor_offset.saturating_sub(index).min(output_width)),
            ));
        }

        let mut glyph_cells = cells[index..index + source_width].to_vec();
        while glyph_cells.len() < cell_width {
            let mut continuation = cell.clone();
            continuation.set_continuation(saturating_u8(
                u16::try_from(glyph_cells.len()).unwrap_or(u16::MAX),
            ));
            glyph_cells.push(continuation);
        }
        row.extend(glyph_cells[..output_width].iter().cloned());
        if cell_width > output_width {
            row_overflow.extend(glyph_cells[output_width..].iter().cloned());
        }
        index = index.saturating_add(source_width);
        if cursor_offset == Some(index) {
            reflowed_cursor = Some((rows.len(), row.len()));
        }
        if row.len() == usize::from(columns) && index < cells.len() {
            rows.push(ReflowRow {
                cells: std::mem::take(&mut row),
                reflow_overflow: std::mem::take(&mut row_overflow),
            });
        }
    }
    rows.push(ReflowRow {
        cells: row,
        reflow_overflow: row_overflow,
    });
    (
        rows,
        reflowed_cursor.or_else(|| cursor_offset.map(|_| (0, 0))),
    )
}

fn parse_csi(chars: &[char], mut index: usize) -> SequenceParse<(char, usize)> {
    while index < chars.len() {
        let ch = chars[index];
        if is_cancel_control(ch) {
            return SequenceParse::Cancelled(index);
        }
        if ('@'..='~').contains(&ch) {
            return SequenceParse::Complete((ch, index));
        }
        index += 1;
    }

    SequenceParse::Pending
}

fn clamp_screen_state(screen: &mut ScreenState, size: TerminalSize) {
    screen.cursor_row = clamp_axis(screen.cursor_row, size.rows);
    screen.cursor_column = clamp_axis(screen.cursor_column, size.columns);
    screen.scroll_top = clamp_axis(screen.scroll_top, size.rows);
    screen.scroll_bottom = clamp_axis(screen.scroll_bottom, size.rows);
    if screen.scroll_top >= screen.scroll_bottom {
        screen.scroll_top = 0;
        screen.scroll_bottom = size.rows.saturating_sub(1);
    }
    if size.columns == 0 || size.rows == 0 {
        screen.pending_wrap = false;
    }
}

fn clamp_axis(value: u16, limit: u16) -> u16 {
    value.min(limit.saturating_sub(1))
}

fn parse_osc(chars: &[char], mut index: usize) -> SequenceParse<usize> {
    while index < chars.len() {
        match chars[index] {
            '\u{7}' | '\u{9c}' => return SequenceParse::Complete(index),
            '\u{1b}' if chars.get(index + 1) == Some(&'\\') => {
                return SequenceParse::Complete(index + 1);
            }
            ch if is_cancel_control(ch) => return SequenceParse::Cancelled(index),
            _ => index += 1,
        }
    }

    SequenceParse::Pending
}

fn osc_content_end(chars: &[char], content_start: usize, sequence_end: usize) -> usize {
    st_content_end(chars, content_start, sequence_end)
}

fn st_content_end(chars: &[char], content_start: usize, sequence_end: usize) -> usize {
    if sequence_end > content_start
        && chars.get(sequence_end) == Some(&'\\')
        && chars.get(sequence_end - 1) == Some(&'\u{1b}')
    {
        sequence_end - 1
    } else {
        sequence_end
    }
}

fn parse_st_terminated_control_string(chars: &[char], mut index: usize) -> SequenceParse<usize> {
    while index < chars.len() {
        match chars[index] {
            '\u{9c}' => return SequenceParse::Complete(index),
            '\u{1b}' if chars.get(index + 1) == Some(&'\\') => {
                return SequenceParse::Complete(index + 1);
            }
            ch if is_cancel_control(ch) => return SequenceParse::Cancelled(index),
            _ => index += 1,
        }
    }

    SequenceParse::Pending
}

fn is_c1_st_control_string(ch: char) -> bool {
    matches!(ch, '\u{90}' | '\u{98}' | '\u{9e}' | '\u{9f}')
}

fn is_escape_or_c1_sequence_start(ch: char) -> bool {
    matches!(
        ch,
        '\u{1b}'
            | '\u{84}'
            | '\u{85}'
            | '\u{88}'
            | '\u{8d}'
            | '\u{90}'
            | '\u{9b}'
            | '\u{9c}'
            | '\u{9d}'
    ) || is_c1_st_control_string(ch)
}

fn is_cancel_control(ch: char) -> bool {
    matches!(ch, '\u{18}' | '\u{1a}')
}

fn is_ascii_control(ch: char) -> bool {
    matches!(ch, '\0'..='\u{1f}' | '\u{7f}')
}

fn may_extend_previous_grapheme(previous_text: &str, ch: char) -> bool {
    !(previous_text.is_ascii() && ch.is_ascii())
}

fn leading_combining_marks_end(text: &str) -> usize {
    let mut end = 0;
    for (index, ch) in text.char_indices() {
        if canonical_combining_class(ch) == 0 {
            break;
        }
        end = index + ch.len_utf8();
    }
    end
}

fn decode_base64_utf8(value: &str) -> Option<String> {
    let decoded = STANDARD.decode(value).ok()?;
    String::from_utf8(decoded).ok()
}

#[derive(Debug, Clone)]
struct SixelImage {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SixelBackground {
    Opaque,
    Transparent,
}

#[derive(Debug, Clone, Copy)]
struct SixelOptions {
    background: SixelBackground,
    pixel_height_scale: u32,
}

impl Default for SixelOptions {
    fn default() -> Self {
        Self {
            background: SixelBackground::Opaque,
            pixel_height_scale: 2,
        }
    }
}

struct SixelCanvas {
    pixels: HashMap<(u32, u32), [u8; 4]>,
    palette: HashMap<u16, [u8; 4]>,
    current_color: [u8; 4],
    background: SixelBackground,
    x: u32,
    y: u32,
    max_x: u32,
    max_y: u32,
    declared_width: Option<u32>,
    declared_height: Option<u32>,
    pixel_height_scale: u32,
}

const VT340_DEFAULT_SIXEL_PALETTE: [[u8; 4]; 16] = [
    [0, 0, 0, 255],
    [51, 51, 204, 255],
    [204, 33, 33, 255],
    [51, 204, 51, 255],
    [204, 51, 204, 255],
    [51, 204, 204, 255],
    [204, 204, 51, 255],
    [135, 135, 135, 255],
    [66, 66, 66, 255],
    [84, 84, 153, 255],
    [153, 66, 66, 255],
    [84, 153, 84, 255],
    [153, 84, 153, 255],
    [84, 153, 153, 255],
    [153, 153, 84, 255],
    [204, 204, 204, 255],
];

impl SixelCanvas {
    fn new(options: SixelOptions) -> Self {
        let palette = VT340_DEFAULT_SIXEL_PALETTE
            .iter()
            .copied()
            .enumerate()
            .map(|(index, color)| {
                (
                    u16::try_from(index).expect("default palette index fits u16"),
                    color,
                )
            })
            .collect();

        Self {
            pixels: HashMap::new(),
            palette,
            current_color: VT340_DEFAULT_SIXEL_PALETTE[0],
            background: options.background,
            x: 0,
            y: 0,
            max_x: 0,
            max_y: 0,
            declared_width: None,
            declared_height: None,
            pixel_height_scale: options.pixel_height_scale,
        }
    }

    fn write_bits(&mut self, bits: u8, repeat: u32) {
        for _ in 0..repeat.max(1) {
            for bit in 0..6 {
                if bits & (1 << bit) != 0 {
                    let y = self.y.saturating_add(bit);
                    self.pixels.insert((self.x, y), self.current_color);
                    self.max_x = self.max_x.max(self.x);
                    self.max_y = self.max_y.max(y);
                }
            }
            self.x = self.x.saturating_add(1);
        }
    }

    fn carriage_return(&mut self) {
        self.x = 0;
    }

    fn newline(&mut self) {
        self.x = 0;
        self.y = self.y.saturating_add(6);
    }

    fn select_color(&mut self, index: u16) {
        if let Some(color) = self.palette.get(&index).copied() {
            self.current_color = color;
        }
    }

    fn define_rgb_color(&mut self, index: u16, red: u16, green: u16, blue: u16) {
        let color = [
            sixel_percent_to_u8(red),
            sixel_percent_to_u8(green),
            sixel_percent_to_u8(blue),
            255,
        ];
        self.palette.insert(index, color);
        self.current_color = color;
    }

    fn define_hls_color(&mut self, index: u16, hue: u16, lightness: u16, saturation: u16) {
        let color = sixel_hls_to_rgba(hue, lightness, saturation);
        self.palette.insert(index, color);
        self.current_color = color;
    }

    fn set_raster_attributes(
        &mut self,
        aspect_numerator: u32,
        aspect_denominator: u32,
        width: u32,
        height: u32,
    ) {
        self.pixel_height_scale = sixel_pixel_height_scale(aspect_numerator, aspect_denominator);
        if width > 0 {
            self.declared_width = Some(width);
        }
        if height > 0 {
            self.declared_height = Some(height);
        }
    }

    fn into_image(self) -> Option<SixelImage> {
        if self.pixels.is_empty() && self.background == SixelBackground::Transparent {
            return None;
        }

        let (drawn_width, drawn_height) = if self.pixels.is_empty() {
            (0, 0)
        } else {
            (self.max_x.checked_add(1)?, self.max_y.checked_add(1)?)
        };
        let width = self
            .declared_width
            .map_or(drawn_width, |declared| declared.max(drawn_width));
        let logical_height = self
            .declared_height
            .map_or(drawn_height, |declared| declared.max(drawn_height));
        let height = logical_height.checked_mul(self.pixel_height_scale)?;
        if width == 0 || logical_height == 0 || height == 0 {
            return None;
        }
        let len = usize::try_from(width)
            .ok()?
            .checked_mul(usize::try_from(height).ok()?)?
            .checked_mul(4)?;
        let background_color = match self.background {
            SixelBackground::Opaque => self.palette.get(&0).copied().unwrap_or([0, 0, 0, 255]),
            SixelBackground::Transparent => [0, 0, 0, 0],
        };
        let mut data = vec![0; len];
        for pixel in data.chunks_exact_mut(4) {
            pixel.copy_from_slice(&background_color);
        }

        for ((x, y), color) in self.pixels {
            if x >= width || y >= logical_height {
                continue;
            }
            let physical_y = y.checked_mul(self.pixel_height_scale)?;
            for row_offset in 0..self.pixel_height_scale {
                let row = physical_y.checked_add(row_offset)?;
                if row >= height {
                    continue;
                }
                let index = usize::try_from((row * width + x) * 4).ok()?;
                data.get_mut(index..index + 4)?.copy_from_slice(&color);
            }
        }

        Some(SixelImage {
            width,
            height,
            data,
        })
    }
}

fn parse_sixel_dcs_options(params: &str) -> SixelOptions {
    let params = parse_sixel_numeric_params(params);
    let pixel_height_scale = sixel_dcs_macro_pixel_height_scale(params.first().copied());
    let background = match params.get(1).copied() {
        Some(1) => SixelBackground::Transparent,
        _ => SixelBackground::Opaque,
    };

    SixelOptions {
        background,
        pixel_height_scale,
    }
}

fn sixel_dcs_marker_index(content: &str) -> Option<usize> {
    let marker = content.find('q')?;
    let params = &content[..marker];
    if !params.chars().all(|ch| ch.is_ascii_digit() || ch == ';') {
        return None;
    }

    (parse_sixel_numeric_params(params).first().copied() != Some(1000)).then_some(marker)
}

fn parse_sixel_image(options: SixelOptions, content: &str) -> Option<SixelImage> {
    let mut canvas = SixelCanvas::new(options);
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '$' => canvas.carriage_return(),
            '-' => canvas.newline(),
            '!' => {
                let repeat = parse_sixel_number(&mut chars).unwrap_or(1);
                if let Some(bits) = chars.next().and_then(sixel_bits) {
                    canvas.write_bits(bits, repeat);
                }
            }
            '#' => parse_sixel_color_introducer(&mut chars, &mut canvas),
            '"' => parse_sixel_raster_attributes(&mut chars, &mut canvas),
            ch => {
                if let Some(bits) = sixel_bits(ch) {
                    canvas.write_bits(bits, 1);
                }
            }
        }
    }

    canvas.into_image()
}

fn parse_sixel_color_introducer(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    canvas: &mut SixelCanvas,
) {
    let params = parse_sixel_parameter_list(chars);
    let Some(index) = params.first().copied() else {
        return;
    };

    match params.get(1).copied() {
        Some(1) if params.len() >= 5 => {
            canvas.define_hls_color(index, params[2], params[3], params[4]);
        }
        Some(2) if params.len() >= 5 => {
            canvas.define_rgb_color(index, params[2], params[3], params[4]);
        }
        _ => canvas.select_color(index),
    }
}

fn parse_sixel_raster_attributes(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    canvas: &mut SixelCanvas,
) {
    let params = parse_sixel_parameter_list(chars);
    if params.len() >= 4 {
        canvas.set_raster_attributes(
            u32::from(params[0]),
            u32::from(params[1]),
            u32::from(params[2]),
            u32::from(params[3]),
        );
    }
}

fn parse_sixel_parameter_list(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Vec<u16> {
    let mut raw = String::new();
    while let Some(ch) = chars.peek().copied() {
        if ch.is_ascii_digit() || ch == ';' {
            raw.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    parse_sixel_numeric_params(&raw)
}

fn parse_sixel_numeric_params(raw: &str) -> Vec<u16> {
    raw.split(';')
        .map(|value| {
            if value.is_empty() {
                0
            } else {
                value.parse::<u16>().unwrap_or(0)
            }
        })
        .collect()
}

fn parse_sixel_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<u32> {
    let mut raw = String::new();
    while let Some(ch) = chars.peek().copied() {
        if ch.is_ascii_digit() {
            raw.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    raw.parse::<u32>().ok()
}

fn sixel_bits(ch: char) -> Option<u8> {
    let code = u32::from(ch);
    (0x3f..=0x7e)
        .contains(&code)
        .then(|| u8::try_from(code - 0x3f).ok())
        .flatten()
}

fn sixel_percent_to_u8(value: u16) -> u8 {
    let value = u32::from(value.min(100));
    u8::try_from((value * 255 + 50) / 100).unwrap_or(255)
}

fn sixel_pixel_height_scale(numerator: u32, denominator: u32) -> u32 {
    if numerator == 0 || denominator == 0 {
        return 1;
    }

    ((numerator + denominator / 2) / denominator).max(1)
}

fn sixel_dcs_macro_pixel_height_scale(value: Option<u16>) -> u32 {
    match value.unwrap_or(0) {
        2 => 5,
        3 | 4 => 3,
        7..=9 => 1,
        _ => 2,
    }
}

fn sixel_hls_to_rgba(hue: u16, lightness: u16, saturation: u16) -> [u8; 4] {
    let hue = u16::try_from((u32::from(hue) + 240) % 360)
        .map(f32::from)
        .expect("normalized sixel HLS hue fits u16")
        / 360.0;
    let lightness = f32::from(lightness.min(100)) / 100.0;
    let saturation = f32::from(saturation.min(100)) / 100.0;

    let (red, green, blue) = if saturation == 0.0 {
        (lightness, lightness, lightness)
    } else {
        let q = if lightness < 0.5 {
            lightness * (1.0 + saturation)
        } else {
            lightness + saturation - lightness * saturation
        };
        let p = 2.0 * lightness - q;
        (
            hue_to_rgb(p, q, hue + 1.0 / 3.0),
            hue_to_rgb(p, q, hue),
            hue_to_rgb(p, q, hue - 1.0 / 3.0),
        )
    };

    [
        unit_float_to_u8(red),
        unit_float_to_u8(green),
        unit_float_to_u8(blue),
        255,
    ]
}

fn hue_to_rgb(p: f32, q: f32, mut hue: f32) -> f32 {
    if hue < 0.0 {
        hue += 1.0;
    }
    if hue > 1.0 {
        hue -= 1.0;
    }

    if hue < 1.0 / 6.0 {
        p + (q - p) * 6.0 * hue
    } else if hue < 1.0 / 2.0 {
        q
    } else if hue < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - hue) * 6.0
    } else {
        p
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn unit_float_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn start_kitty_graphics_upload(
    params: KittyGraphicsParams,
    encoded_data: &str,
) -> Result<PendingKittyGraphics, KittyGraphicsStartError> {
    let action = match params.action {
        Some('T') => KittyUploadAction::Display,
        Some('t') | None => KittyUploadAction::Store,
        Some('q') => KittyUploadAction::Query,
        _ => return Err(KittyGraphicsStartError::Action),
    };

    let medium = match params.medium.unwrap_or('d') {
        'd' => KittyTransmissionMedium::Direct,
        'f' => KittyTransmissionMedium::File,
        't' => KittyTransmissionMedium::TempFile,
        _ => return Err(KittyGraphicsStartError::TransmissionMedium),
    };
    if params
        .compression
        .is_some_and(|compression| compression != 'z')
    {
        return Err(KittyGraphicsStartError::Compression);
    }

    let image_format = match params.format.unwrap_or(32) {
        24 => InlineImageFormat::Rgb,
        32 => InlineImageFormat::Rgba,
        100 => InlineImageFormat::Encoded,
        _ => return Err(KittyGraphicsStartError::ImageFormat),
    };

    Ok(PendingKittyGraphics {
        image_format,
        medium,
        compression: params.compression,
        action,
        image_id: params.image_id,
        image_number: params.image_number,
        placement_id: params.placement_id,
        z_index: params.z_index,
        pixel_width: params.pixel_width,
        pixel_height: params.pixel_height,
        display_columns: params.display_columns,
        display_rows: params.display_rows,
        no_cursor_movement: params.no_cursor_movement,
        source_x: params.source_x,
        source_y: params.source_y,
        source_width: params.source_width,
        source_height: params.source_height,
        target_x: params.target_x,
        target_y: params.target_y,
        file_offset: params.file_offset,
        file_size: params.file_size,
        quiet: params.quiet,
        virtual_placement: params.virtual_placement,
        encoded_data: encoded_data.to_owned(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KittyGraphicsStartError {
    Action,
    TransmissionMedium,
    Compression,
    ImageFormat,
}

impl KittyGraphicsStartError {
    const fn message(self) -> &'static str {
        match self {
            Self::Action => "Unsupported action",
            Self::TransmissionMedium => "Unsupported transmission medium",
            Self::Compression => "Unsupported compression",
            Self::ImageFormat => "Unsupported image format",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KittyGraphicsDataError {
    InvalidBase64,
    InvalidFile,
    UnsupportedCompression,
}

fn load_kitty_graphics_payload(
    upload: &PendingKittyGraphics,
) -> Result<Vec<u8>, KittyGraphicsDataError> {
    let data = match upload.medium {
        KittyTransmissionMedium::Direct => STANDARD
            .decode(&upload.encoded_data)
            .map_err(|_| KittyGraphicsDataError::InvalidBase64)?,
        KittyTransmissionMedium::File => read_kitty_graphics_file_payload(
            &upload.encoded_data,
            upload.file_offset,
            upload.file_size,
            false,
        )?,
        KittyTransmissionMedium::TempFile => read_kitty_graphics_file_payload(
            &upload.encoded_data,
            upload.file_offset,
            upload.file_size,
            true,
        )?,
    };

    decode_kitty_graphics_payload(upload.compression, data)
        .ok_or(KittyGraphicsDataError::UnsupportedCompression)
}

fn read_kitty_graphics_file_payload(
    encoded_path: &str,
    offset: Option<u64>,
    size: Option<u64>,
    delete_after_read: bool,
) -> Result<Vec<u8>, KittyGraphicsDataError> {
    let path = STANDARD
        .decode(encoded_path)
        .map_err(|_| KittyGraphicsDataError::InvalidBase64)?;
    let path = String::from_utf8(path).map_err(|_| KittyGraphicsDataError::InvalidFile)?;
    let path = PathBuf::from(path);

    let metadata = fs::metadata(&path).map_err(|_| KittyGraphicsDataError::InvalidFile)?;
    if !metadata.is_file() {
        return Err(KittyGraphicsDataError::InvalidFile);
    }

    let data = {
        let mut file = fs::File::open(&path).map_err(|_| KittyGraphicsDataError::InvalidFile)?;
        if let Some(offset) = offset {
            file.seek(SeekFrom::Start(offset))
                .map_err(|_| KittyGraphicsDataError::InvalidFile)?;
        }

        let mut data = Vec::new();
        if let Some(size) = size {
            let mut limited = file.take(size);
            limited
                .read_to_end(&mut data)
                .map_err(|_| KittyGraphicsDataError::InvalidFile)?;
        } else {
            file.read_to_end(&mut data)
                .map_err(|_| KittyGraphicsDataError::InvalidFile)?;
        }
        data
    };

    if delete_after_read && kitty_temp_file_can_be_deleted(&path) {
        let _ = fs::remove_file(&path);
    }

    Ok(data)
}

fn kitty_temp_file_can_be_deleted(path: &Path) -> bool {
    let Ok(path) = fs::canonicalize(path) else {
        return false;
    };
    if !path
        .as_os_str()
        .to_string_lossy()
        .contains("tty-graphics-protocol")
    {
        return false;
    }

    kitty_temp_directories()
        .into_iter()
        .any(|temp_dir| fs::canonicalize(temp_dir).is_ok_and(|temp_dir| path.starts_with(temp_dir)))
}

fn kitty_temp_directories() -> Vec<PathBuf> {
    #[cfg(unix)]
    {
        let mut dirs = vec![std::env::temp_dir()];
        dirs.push(PathBuf::from("/tmp"));
        dirs.push(PathBuf::from("/var/tmp"));
        dirs.push(PathBuf::from("/dev/shm"));
        dirs
    }

    #[cfg(not(unix))]
    {
        vec![std::env::temp_dir()]
    }
}

fn append_kitty_graphics_response_param(
    response: &mut Vec<u8>,
    has_param: &mut bool,
    name: u8,
    value: u32,
) {
    if *has_param {
        response.push(b',');
    }
    response.push(name);
    response.push(b'=');
    response.extend_from_slice(value.to_string().as_bytes());
    *has_param = true;
}

fn kitty_graphics_response_is_suppressed(params: KittyGraphicsParams, status: &str) -> bool {
    match params.quiet {
        Some(1) => status == "OK",
        Some(2) => status != "OK",
        _ => false,
    }
}

fn kitty_placement_id(image_id: Option<u32>, placement_id: Option<u32>) -> Option<u32> {
    image_id.and(placement_id)
}

fn kitty_placement_key(
    image_id: Option<u32>,
    placement_id: Option<u32>,
) -> Option<KittyPlacementKey> {
    Some((image_id?, placement_id?))
}

fn kitty_virtual_placement_key(image_id: u32, placement_id: Option<u32>) -> KittyPlacementKey {
    (image_id, placement_id.unwrap_or(0))
}

fn kitty_image_placement_key(image: &ItermInlineImage) -> Option<KittyPlacementKey> {
    kitty_placement_key(image.kitty_image_id, image.kitty_placement_id)
}

fn kitty_display_dimensions(
    image: &StoredKittyImage,
    options: KittyPlacementOptions,
) -> (Option<String>, Option<String>) {
    let columns = options.display_columns.or(image.display_columns);
    let rows = options.display_rows.or(image.display_rows);
    let aspect = kitty_source_aspect_pixels(image, options.source_rect);

    let derived_rows = match (columns, rows, aspect) {
        (Some(columns), None, Some((source_width, source_height))) => {
            derive_kitty_display_axis(columns, source_height, source_width)
        }
        _ => None,
    };
    let derived_columns = match (columns, rows, aspect) {
        (None, Some(rows), Some((source_width, source_height))) => {
            derive_kitty_display_axis(rows, source_width, source_height)
        }
        _ => None,
    };

    let width = columns
        .or(derived_columns)
        .map(|columns| columns.to_string())
        .or_else(|| image.pixel_width.map(|width| format!("{width}px")));
    let height = rows
        .or(derived_rows)
        .map(|rows| rows.to_string())
        .or_else(|| image.pixel_height.map(|height| format!("{height}px")));

    (width, height)
}

fn kitty_source_aspect_pixels(
    image: &StoredKittyImage,
    source_rect: KittySourceRect,
) -> Option<(u32, u32)> {
    let pixel_width = image.pixel_width?;
    let pixel_height = image.pixel_height?;
    let source_x = source_rect.x.unwrap_or(0);
    let source_y = source_rect.y.unwrap_or(0);
    if source_x >= pixel_width || source_y >= pixel_height {
        return None;
    }

    let width = source_rect.width.unwrap_or(pixel_width - source_x);
    let height = source_rect.height.unwrap_or(pixel_height - source_y);
    Some((
        width.min(pixel_width - source_x),
        height.min(pixel_height - source_y),
    ))
    .filter(|(width, height)| *width > 0 && *height > 0)
}

fn derive_kitty_display_axis(
    known_cells: u16,
    numerator_pixels: u32,
    denominator_pixels: u32,
) -> Option<u16> {
    if denominator_pixels == 0 {
        return None;
    }
    let derived = u64::from(known_cells)
        .saturating_mul(u64::from(numerator_pixels))
        .saturating_add(u64::from(denominator_pixels) - 1)
        / u64::from(denominator_pixels);
    Some(u16::try_from(derived.max(1)).unwrap_or(u16::MAX))
}

fn kitty_image_matches_placeholder_render(
    image: &ItermInlineImage,
    placeholder_render_keys: &HashSet<KittyPlaceholderRenderKey>,
) -> bool {
    image.kitty_image_id.is_some_and(|image_id| {
        placeholder_render_keys.contains(&(
            image.row,
            image.column,
            image_id,
            image.kitty_placement_id,
        ))
    })
}

fn kitty_placeholder_references_live_placement(
    placeholder: LastKittyPlaceholder,
    live_placements: &HashSet<(u32, Option<u32>)>,
) -> bool {
    let Some(low_bytes) = kitty_placeholder_image_id(placeholder.foreground) else {
        return true;
    };
    let image_id = (low_bytes & 0x00ff_ffff) | ((placeholder.image_id_high_byte & 0xff) << 24);
    let placement_id = kitty_placeholder_placement_id(placeholder.underline_color);
    live_placements
        .iter()
        .any(|(live_image_id, live_placement_id)| {
            *live_image_id == image_id
                && (placement_id.is_none() || *live_placement_id == placement_id)
        })
}

fn kitty_placeholder_image_id(color: Color) -> Option<u32> {
    kitty_placeholder_color_value(color).filter(|image_id| *image_id != ANONYMOUS_KITTY_IMAGE_ID)
}

fn kitty_placeholder_placement_id(color: Color) -> Option<u32> {
    kitty_placeholder_color_value(color).filter(|placement_id| *placement_id != 0)
}

fn kitty_placeholder_origin(
    pending: &PendingKittyPlaceholder,
    placeholder_row: u32,
    placeholder_column: u32,
) -> Option<(usize, u16)> {
    kitty_placeholder_origin_from_cell(
        pending.row,
        pending.column,
        placeholder_row,
        placeholder_column,
    )
}

fn kitty_placeholder_origin_from_cell(
    row: usize,
    column: u16,
    placeholder_row: u32,
    placeholder_column: u32,
) -> Option<(usize, u16)> {
    let row_offset = usize::try_from(placeholder_row).ok()?;
    let column_offset = u16::try_from(placeholder_column).ok()?;
    Some((
        row.checked_sub(row_offset)?,
        column.checked_sub(column_offset)?,
    ))
}

fn kitty_placeholder_color_value(color: Color) -> Option<u32> {
    match color {
        Color::Default => None,
        Color::Indexed(value) => Some(u32::from(value)),
        Color::Rgb(red, green, blue) | Color::Rgba(red, green, blue, _) => {
            Some((u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue))
        }
    }
}

fn kitty_placeholder_diacritic_value(ch: char) -> Option<u32> {
    KITTY_PLACEHOLDER_DIACRITICS
        .iter()
        .position(|diacritic| *diacritic == ch)
        .and_then(|index| u32::try_from(index).ok())
}

fn offset_kitty_history_row(row: usize, offset: i32) -> usize {
    let magnitude = usize::try_from(offset.unsigned_abs()).unwrap_or(usize::MAX);
    if offset >= 0 {
        row.saturating_add(magnitude)
    } else {
        row.saturating_sub(magnitude)
    }
}

fn offset_kitty_column(column: u16, offset: i32) -> u16 {
    let column = offset_kitty_history_row(usize::from(column), offset);
    u16::try_from(column).unwrap_or(u16::MAX)
}

fn move_kitty_history_row(row: usize, old_origin: usize, new_origin: usize) -> usize {
    if new_origin >= old_origin {
        row.saturating_add(new_origin - old_origin)
    } else {
        row.saturating_sub(old_origin - new_origin)
    }
}

fn move_kitty_column(column: u16, old_origin: u16, new_origin: u16) -> u16 {
    if new_origin >= old_origin {
        column.saturating_add(new_origin - old_origin)
    } else {
        column.saturating_sub(old_origin - new_origin)
    }
}

fn kitty_image_intersects_cell(image: &ItermInlineImage, row: usize, column: u16) -> bool {
    if image.kitty_image_id.is_none() {
        return false;
    }

    let (left, right) = kitty_image_column_range(image);
    let (top, bottom) = kitty_image_row_range(image);

    row >= top && row < bottom && column >= left && column < right
}

fn kitty_image_intersects_column(
    image: &ItermInlineImage,
    first_row: usize,
    last_row: usize,
    column: u16,
) -> bool {
    if image.kitty_image_id.is_none() {
        return false;
    }

    let (left, right) = kitty_image_column_range(image);
    let (top, bottom) = kitty_image_row_range(image);

    column >= left && column < right && top < last_row && bottom > first_row
}

fn kitty_image_intersects_row(image: &ItermInlineImage, row: usize) -> bool {
    if image.kitty_image_id.is_none() {
        return false;
    }

    let (top, bottom) = kitty_image_row_range(image);
    row >= top && row < bottom
}

fn inline_image_intersects_region(
    image: &ItermInlineImage,
    first_row: usize,
    last_row: usize,
    first_column: u16,
    last_column: u16,
) -> bool {
    let (left, right) = kitty_image_column_range(image);
    let (top, bottom) = kitty_image_row_range(image);

    top < last_row && bottom > first_row && left < last_column && right > first_column
}

fn kitty_image_column_range(image: &ItermInlineImage) -> (u16, u16) {
    let width = inline_image_width_cells(image.width.as_deref()).max(1);
    (
        image.column,
        image
            .column
            .saturating_add(width)
            .max(image.column.saturating_add(1)),
    )
}

fn kitty_image_row_range(image: &ItermInlineImage) -> (usize, usize) {
    let height = usize::from(inline_image_height_cells(image.height.as_deref()).max(1));
    (
        image.row,
        image
            .row
            .saturating_add(height)
            .max(image.row.saturating_add(1)),
    )
}

fn zero_based_axis(value: u16) -> Option<u16> {
    value.checked_sub(1)
}

fn parse_kitty_graphics_params(control: &str) -> KittyGraphicsParams {
    let mut params = KittyGraphicsParams::default();

    for param in control.split(',').filter(|param| !param.is_empty()) {
        let Some((key, value)) = param.split_once('=') else {
            continue;
        };

        match key {
            "a" => params.action = parse_single_char(value),
            "d" => params.delete_target = parse_single_char(value),
            "f" => params.format = value.parse::<u32>().ok(),
            "i" => params.image_id = parse_positive_u32(value),
            "I" => params.image_number = parse_positive_u32(value),
            "p" => params.placement_id = parse_positive_u32(value),
            "P" => params.parent_image_id = parse_positive_u32(value),
            "Q" => params.parent_placement_id = parse_positive_u32(value),
            "t" => params.medium = parse_single_char(value),
            "o" => params.compression = parse_single_char(value),
            "m" => params.more_chunks = value.parse::<u8>().ok(),
            "s" => params.pixel_width = parse_positive_u32(value),
            "v" => params.pixel_height = parse_positive_u32(value),
            "c" => params.display_columns = parse_positive_u16(value),
            "r" => params.display_rows = parse_positive_u16(value),
            "C" => params.no_cursor_movement = value == "1",
            "U" => params.virtual_placement = value == "1",
            "w" => params.source_width = parse_positive_u32(value),
            "h" => params.source_height = parse_positive_u32(value),
            "X" => params.target_x = value.parse::<u32>().ok(),
            "Y" => params.target_y = value.parse::<u32>().ok(),
            "H" => params.parent_offset_columns = value.parse::<i32>().ok(),
            "V" => params.parent_offset_rows = value.parse::<i32>().ok(),
            "x" => {
                params.cell_x = parse_positive_u16(value);
                params.source_x = value.parse::<u32>().ok();
            }
            "y" => {
                params.cell_y = parse_positive_u16(value);
                params.source_y = value.parse::<u32>().ok();
            }
            "z" => params.z_index = value.parse::<i32>().ok(),
            "O" => params.file_offset = value.parse::<u64>().ok(),
            "S" => params.file_size = value.parse::<u64>().ok(),
            "q" => params.quiet = value.parse::<u8>().ok(),
            _ => {}
        }
    }

    params
}

fn parse_single_char(value: &str) -> Option<char> {
    let mut chars = value.chars();
    let ch = chars.next()?;
    chars.next().is_none().then_some(ch)
}

fn decode_kitty_graphics_payload(compression: Option<char>, data: Vec<u8>) -> Option<Vec<u8>> {
    match compression {
        None => Some(data),
        Some('z') => {
            let mut decoder = ZlibDecoder::new(data.as_slice());
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).ok()?;
            Some(decompressed)
        }
        Some(_) => None,
    }
}

fn kitty_graphics_payload_is_supported(
    image_format: InlineImageFormat,
    pixel_width: Option<u32>,
    pixel_height: Option<u32>,
    payload_len: usize,
) -> bool {
    let bytes_per_pixel = match image_format {
        InlineImageFormat::Encoded => return true,
        InlineImageFormat::Rgb => 3,
        InlineImageFormat::Rgba => 4,
    };
    let Some(pixel_width) = pixel_width else {
        return false;
    };
    let Some(pixel_height) = pixel_height else {
        return false;
    };
    let Some(expected_len) = usize::try_from(pixel_width)
        .ok()
        .and_then(|width| {
            usize::try_from(pixel_height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(bytes_per_pixel))
    else {
        return false;
    };

    payload_len == expected_len
}

fn parse_positive_u32(value: &str) -> Option<u32> {
    value.parse::<u32>().ok().filter(|value| *value > 0)
}

fn parse_positive_u16(value: &str) -> Option<u16> {
    value.parse::<u16>().ok().filter(|value| *value > 0)
}

const DEFAULT_INLINE_IMAGE_CELL_WIDTH_PIXELS: u16 = 8;
const DEFAULT_INLINE_IMAGE_CELL_HEIGHT_PIXELS: u16 = 16;

fn inline_image_width_cells(value: Option<&str>) -> u16 {
    inline_image_axis_cells(value, DEFAULT_INLINE_IMAGE_CELL_WIDTH_PIXELS)
}

fn inline_image_height_cells(value: Option<&str>) -> u16 {
    inline_image_axis_cells(value, DEFAULT_INLINE_IMAGE_CELL_HEIGHT_PIXELS)
}

fn inline_image_axis_cells(value: Option<&str>, cell_pixels: u16) -> u16 {
    let Some(value) = value else {
        return 1;
    };
    if let Some(pixels) = value.strip_suffix("px").and_then(parse_positive_u32) {
        let cell_pixels = u32::from(cell_pixels.max(1));
        let cells = pixels.saturating_add(cell_pixels - 1) / cell_pixels;
        return u16::try_from(cells).unwrap_or(u16::MAX).max(1);
    }
    value
        .parse::<u16>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn inline_image_axis_pixels(value: Option<&str>, cell_pixels: u16) -> u32 {
    let cell_pixels = u32::from(cell_pixels.max(1));
    let Some(value) = value else {
        return cell_pixels;
    };
    if let Some(pixels) = value.strip_suffix("px").and_then(parse_positive_u32) {
        return pixels;
    }
    value
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .map_or(cell_pixels, |cells| cells.saturating_mul(cell_pixels))
}

#[expect(
    clippy::too_many_lines,
    reason = "fragment generation keeps checked geometry and every overflow exit together"
)]
fn inline_image_fragments(
    image_index: usize,
    image: &ItermInlineImage,
) -> Option<Vec<InlineImageFragment>> {
    let pixel_width = image.pixel_width?;
    let pixel_height = image.pixel_height?;
    let source_x = image.source_x.unwrap_or(0);
    let source_y = image.source_y.unwrap_or(0);
    let available_width = pixel_width.checked_sub(source_x)?;
    let available_height = pixel_height.checked_sub(source_y)?;
    let source_width = image
        .source_width
        .unwrap_or(available_width)
        .min(available_width);
    let source_height = image
        .source_height
        .unwrap_or(available_height)
        .min(available_height);
    if source_width == 0 || source_height == 0 {
        return None;
    }

    let cell_width = u32::from(DEFAULT_INLINE_IMAGE_CELL_WIDTH_PIXELS);
    let cell_height = u32::from(DEFAULT_INLINE_IMAGE_CELL_HEIGHT_PIXELS);
    let destination_width = inline_image_axis_pixels(
        image.width.as_deref(),
        DEFAULT_INLINE_IMAGE_CELL_WIDTH_PIXELS,
    );
    let destination_height = inline_image_axis_pixels(
        image.height.as_deref(),
        DEFAULT_INLINE_IMAGE_CELL_HEIGHT_PIXELS,
    );
    if destination_width == 0 || destination_height == 0 {
        return None;
    }

    let destination_left = u32::from(image.column)
        .checked_mul(cell_width)?
        .checked_add(image.target_x.unwrap_or(0))?;
    let destination_top = u32::try_from(image.row)
        .ok()?
        .checked_mul(cell_height)?
        .checked_add(image.target_y.unwrap_or(0))?;
    let destination_right = destination_left.checked_add(destination_width)?;
    let destination_bottom = destination_top.checked_add(destination_height)?;
    let first_column = destination_left / cell_width;
    let first_row = destination_top / cell_height;
    let last_column = destination_right.checked_sub(1)? / cell_width;
    let last_row = destination_bottom.checked_sub(1)? / cell_height;
    let fragment_count = u64::from(last_column.checked_sub(first_column)?.saturating_add(1))
        .saturating_mul(u64::from(
            last_row.checked_sub(first_row)?.saturating_add(1),
        ));
    // A pathological protocol payload must not turn a renderer snapshot into
    // an unbounded allocation. The original placement remains renderable.
    if fragment_count == 0 || fragment_count > 1_000_000 {
        return None;
    }

    let mut fragments = Vec::with_capacity(usize::try_from(fragment_count).ok()?);
    for row in first_row..=last_row {
        let cell_top = row.checked_mul(cell_height)?;
        let fragment_top = destination_top.max(cell_top);
        let fragment_bottom = destination_bottom.min(cell_top.checked_add(cell_height)?);
        for column in first_column..=last_column {
            let cell_left = column.checked_mul(cell_width)?;
            let fragment_left = destination_left.max(cell_left);
            let fragment_right = destination_right.min(cell_left.checked_add(cell_width)?);
            let column = u16::try_from(column).ok()?;
            let row = usize::try_from(row).ok()?;
            let source_destination_x = fragment_left.checked_sub(destination_left)?;
            let source_destination_y = fragment_top.checked_sub(destination_top)?;
            let source_destination_right =
                source_destination_x.checked_add(fragment_right.checked_sub(fragment_left)?)?;
            let source_destination_bottom =
                source_destination_y.checked_add(fragment_bottom.checked_sub(fragment_top)?)?;
            let fragment_source_x = source_x.checked_add(
                source_destination_x.saturating_mul(source_width) / destination_width,
            )?;
            let fragment_source_y = source_y.checked_add(
                source_destination_y.saturating_mul(source_height) / destination_height,
            )?;
            let fragment_source_right = source_x.checked_add(
                source_destination_right
                    .saturating_mul(source_width)
                    .saturating_add(destination_width - 1)
                    / destination_width,
            )?;
            let fragment_source_bottom = source_y.checked_add(
                source_destination_bottom
                    .saturating_mul(source_height)
                    .saturating_add(destination_height - 1)
                    / destination_height,
            )?;
            fragments.push(InlineImageFragment {
                image_index,
                cell_attachment: false,
                row,
                column,
                source_row: row,
                source_column: column,
                destination_x: fragment_left.checked_sub(cell_left)?,
                destination_y: fragment_top.checked_sub(cell_top)?,
                destination_width: fragment_right.checked_sub(fragment_left)?,
                destination_height: fragment_bottom.checked_sub(fragment_top)?,
                source_x: fragment_source_x,
                source_y: fragment_source_y,
                source_width: fragment_source_right.checked_sub(fragment_source_x)?,
                source_height: fragment_source_bottom.checked_sub(fragment_source_y)?,
                sampling_source_x: source_x,
                sampling_source_y: source_y,
                sampling_source_width: source_width,
                sampling_source_height: source_height,
                source_destination_x,
                source_destination_y,
                source_destination_width: destination_width,
                source_destination_height: destination_height,
                kitty_image_id: image.kitty_image_id,
                kitty_placement_id: image.kitty_placement_id,
                kitty_z_index: image.kitty_z_index,
                image_format: image.image_format,
            });
        }
    }
    Some(fragments)
}

fn cell_attachments_for_image(
    parent_identity: u64,
    image: &ItermInlineImage,
) -> Vec<CellAttachment> {
    // A placement expressed only in pixels has no declared terminal-cell
    // footprint. Keep that legacy case on the whole-image path; explicit cell
    // dimensions are geometry-independent even when a target pixel offset is
    // also present.
    let Some((columns, rows)) = cell_attachment_dimensions(image) else {
        return Vec::new();
    };
    let attachment_count = u64::from(columns).saturating_mul(u64::from(rows));

    let mut attachments = Vec::with_capacity(usize::try_from(attachment_count).unwrap_or(0));
    for source_row in 0..rows {
        let Some(row) = image.row.checked_add(usize::from(source_row)) else {
            return Vec::new();
        };
        for source_column in 0..columns {
            let Some(column) = image.column.checked_add(source_column) else {
                return Vec::new();
            };
            attachments.push(CellAttachment {
                parent_identity,
                source_row,
                source_column,
                row,
                column,
            });
        }
    }
    attachments
}

fn cell_attachment_dimensions(image: &ItermInlineImage) -> Option<(u16, u16)> {
    if image
        .width
        .as_deref()
        .is_some_and(|value| value.ends_with("px"))
        || image
            .height
            .as_deref()
            .is_some_and(|value| value.ends_with("px"))
    {
        return None;
    }
    let columns = image
        .width
        .as_deref()
        .and_then(parse_positive_u16)
        .unwrap_or(1);
    let rows = image
        .height
        .as_deref()
        .and_then(parse_positive_u16)
        .unwrap_or(1);
    let attachment_count = u64::from(columns).saturating_mul(u64::from(rows));
    (attachment_count > 0 && attachment_count <= 1_000_000).then_some((columns, rows))
}

#[expect(
    clippy::too_many_lines,
    reason = "attachment sampling keeps the checked whole-image mapping invariants together"
)]
fn inline_image_attachment_fragment(
    image_index: usize,
    image: &ItermInlineImage,
    attachment: CellAttachment,
) -> Option<InlineImageFragment> {
    let (columns, rows) = cell_attachment_dimensions(image)?;
    if attachment.source_column >= columns || attachment.source_row >= rows {
        return None;
    }
    let cell_width = u32::from(DEFAULT_INLINE_IMAGE_CELL_WIDTH_PIXELS);
    let cell_height = u32::from(DEFAULT_INLINE_IMAGE_CELL_HEIGHT_PIXELS);
    let destination_width = u32::from(columns).checked_mul(cell_width)?;
    let destination_height = u32::from(rows).checked_mul(cell_height)?;
    let source_destination_x = u32::from(attachment.source_column).checked_mul(cell_width)?;
    let source_destination_y = u32::from(attachment.source_row).checked_mul(cell_height)?;

    // iTerm payload metadata often omits pixel dimensions.  Preserve the
    // logical attachment so the renderer can resolve its source slab from the
    // decoded PNG/JPEG/GIF dimensions at draw time.
    if image.pixel_width.is_none() || image.pixel_height.is_none() {
        return Some(InlineImageFragment {
            image_index,
            cell_attachment: true,
            row: attachment.row,
            column: attachment.column,
            source_row: usize::from(attachment.source_row),
            source_column: attachment.source_column,
            destination_x: image.target_x.unwrap_or(0),
            destination_y: image.target_y.unwrap_or(0),
            destination_width: cell_width,
            destination_height: cell_height,
            source_x: 0,
            source_y: 0,
            source_width: 0,
            source_height: 0,
            sampling_source_x: 0,
            sampling_source_y: 0,
            sampling_source_width: 0,
            sampling_source_height: 0,
            source_destination_x,
            source_destination_y,
            source_destination_width: destination_width,
            source_destination_height: destination_height,
            kitty_image_id: image.kitty_image_id,
            kitty_placement_id: image.kitty_placement_id,
            kitty_z_index: image.kitty_z_index,
            image_format: image.image_format,
        });
    }
    let pixel_width = image.pixel_width?;
    let pixel_height = image.pixel_height?;
    let source_x = image.source_x.unwrap_or(0);
    let source_y = image.source_y.unwrap_or(0);
    let source_width = image
        .source_width
        .unwrap_or(pixel_width.checked_sub(source_x)?)
        .min(pixel_width.checked_sub(source_x)?);
    let source_height = image
        .source_height
        .unwrap_or(pixel_height.checked_sub(source_y)?)
        .min(pixel_height.checked_sub(source_y)?);
    if source_width == 0 || source_height == 0 {
        return None;
    }
    let source_destination_right = source_destination_x.checked_add(cell_width)?;
    let source_destination_bottom = source_destination_y.checked_add(cell_height)?;
    let fragment_source_x = source_x
        .checked_add(source_destination_x.saturating_mul(source_width) / destination_width)?;
    let fragment_source_y = source_y
        .checked_add(source_destination_y.saturating_mul(source_height) / destination_height)?;
    let fragment_source_right = source_x.checked_add(
        source_destination_right
            .saturating_mul(source_width)
            .saturating_add(destination_width - 1)
            / destination_width,
    )?;
    let fragment_source_bottom = source_y.checked_add(
        source_destination_bottom
            .saturating_mul(source_height)
            .saturating_add(destination_height - 1)
            / destination_height,
    )?;

    Some(InlineImageFragment {
        image_index,
        cell_attachment: true,
        row: attachment.row,
        column: attachment.column,
        source_row: usize::from(attachment.source_row),
        source_column: attachment.source_column,
        destination_x: image.target_x.unwrap_or(0),
        destination_y: image.target_y.unwrap_or(0),
        destination_width: cell_width,
        destination_height: cell_height,
        source_x: fragment_source_x,
        source_y: fragment_source_y,
        source_width: fragment_source_right.checked_sub(fragment_source_x)?,
        source_height: fragment_source_bottom.checked_sub(fragment_source_y)?,
        sampling_source_x: source_x,
        sampling_source_y: source_y,
        sampling_source_width: source_width,
        sampling_source_height: source_height,
        source_destination_x,
        source_destination_y,
        source_destination_width: destination_width,
        source_destination_height: destination_height,
        kitty_image_id: image.kitty_image_id,
        kitty_placement_id: image.kitty_placement_id,
        kitty_z_index: image.kitty_z_index,
        image_format: image.image_format,
    })
}

fn next_or_pending(next_index: Option<usize>) -> FeedAdvance {
    match next_index {
        Some(next_index) => FeedAdvance::Next(next_index),
        None => FeedAdvance::Pending,
    }
}

fn csi_count(params: &[char]) -> u16 {
    param_or_one(parse_csi_params(params).first().copied())
}

fn csi_mode(params: &[char]) -> u16 {
    parse_csi_params(params).first().copied().unwrap_or(0)
}

fn csi_or_private_mode(params: &[char]) -> u16 {
    parse_private_csi_params(params)
        .unwrap_or_else(|| parse_csi_params(params))
        .first()
        .copied()
        .unwrap_or(0)
}

fn csi_is_private(params: &[char]) -> bool {
    matches!(params, ['?', ..])
}

fn param_or_one(value: Option<u16>) -> u16 {
    match value {
        Some(0) | None => 1,
        Some(value) => value,
    }
}

fn append_semantic_zones_for_row(
    row: usize,
    cells: &[Cell],
    current_zone: &mut Option<SemanticZone>,
    zones: &mut Vec<SemanticZone>,
) {
    let Some(first_non_blank) = cells.iter().position(|cell| !cell.is_blank()) else {
        return;
    };
    let Some(last_non_blank) = cells.iter().rposition(|cell| !cell.is_blank()) else {
        return;
    };

    let mut start = first_non_blank;
    while start <= last_non_blank {
        let semantic_type = cells[start].semantic_type;
        let mut end = start;
        while end < last_non_blank && cells[end + 1].semantic_type == semantic_type {
            end += 1;
        }

        if cells[start..=end].iter().any(|cell| !cell.is_blank()) {
            append_semantic_zone(
                SemanticZone::new(row, start, row, end, semantic_type),
                current_zone,
                zones,
            );
        }

        start = end + 1;
    }
}

fn append_semantic_zone(
    zone: SemanticZone,
    current_zone: &mut Option<SemanticZone>,
    zones: &mut Vec<SemanticZone>,
) {
    if let Some(current) = current_zone.as_mut()
        && current.semantic_type == zone.semantic_type
    {
        current.end_y = zone.end_y;
        current.end_x = zone.end_x;
        return;
    }

    if let Some(current) = current_zone.replace(zone) {
        zones.push(current);
    }
}

fn trim_trailing_spaces(text: &mut String) {
    while text.ends_with(' ') {
        text.pop();
    }
}

fn parse_csi_params(params: &[char]) -> Vec<u16> {
    let raw = params.iter().collect::<String>();
    raw.split(';')
        .map(|part| {
            if part.is_empty() {
                0
            } else {
                part.parse::<u16>().unwrap_or(0)
            }
        })
        .collect()
}

fn parse_private_csi_params(params: &[char]) -> Option<Vec<u16>> {
    let ['?', rest @ ..] = params else {
        return None;
    };

    Some(parse_csi_params(rest))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SgrParameter {
    Code(u16),
    UnderlineStyle(UnderlineStyle),
}

fn parse_sgr_params(params: &[char]) -> Vec<SgrParameter> {
    if params.is_empty() {
        return vec![SgrParameter::Code(0)];
    }

    let raw = params.iter().collect::<String>();
    let mut parsed = Vec::new();

    for group in raw.split(';') {
        if group.is_empty() {
            parsed.push(SgrParameter::Code(0));
            continue;
        }

        let values = group
            .split(':')
            .map(parse_sgr_numeric_part)
            .collect::<Vec<_>>();

        if group.contains(':') && values.first() == Some(&4) {
            let style = values.get(1).map_or(UnderlineStyle::None, |value| {
                underline_style_from_sgr(*value)
            });
            parsed.push(SgrParameter::UnderlineStyle(style));
        } else {
            parsed.extend(values.into_iter().map(SgrParameter::Code));
        }
    }

    parsed
}

fn parse_sgr_numeric_part(part: &str) -> u16 {
    if part.is_empty() {
        0
    } else {
        part.parse::<u16>().unwrap_or(0)
    }
}

const fn underline_style_from_sgr(style: u16) -> UnderlineStyle {
    match style {
        0 => UnderlineStyle::None,
        2 => UnderlineStyle::Double,
        3 => UnderlineStyle::Curly,
        4 => UnderlineStyle::Dotted,
        5 => UnderlineStyle::Dashed,
        _ => UnderlineStyle::Single,
    }
}

fn parse_extended_color(values: &[SgrParameter]) -> Option<(Color, usize)> {
    use SgrParameter::Code;

    match values {
        [Code(5), Code(index), ..] => Some((Color::Indexed(saturating_u8(*index)), 2)),
        [Code(2), Code(0), Code(red), Code(green), Code(blue), ..] => Some((
            Color::Rgb(
                saturating_u8(*red),
                saturating_u8(*green),
                saturating_u8(*blue),
            ),
            5,
        )),
        [Code(2), Code(red), Code(green), Code(blue), ..] => Some((
            Color::Rgb(
                saturating_u8(*red),
                saturating_u8(*green),
                saturating_u8(*blue),
            ),
            4,
        )),
        [
            Code(6),
            Code(0),
            Code(red),
            Code(green),
            Code(blue),
            Code(alpha),
            ..,
        ] => Some((
            Color::Rgba(
                saturating_u8(*red),
                saturating_u8(*green),
                saturating_u8(*blue),
                saturating_u8(*alpha),
            ),
            6,
        )),
        [Code(6), Code(red), Code(green), Code(blue), Code(alpha), ..] => Some((
            Color::Rgba(
                saturating_u8(*red),
                saturating_u8(*green),
                saturating_u8(*blue),
                saturating_u8(*alpha),
            ),
            5,
        )),
        _ => None,
    }
}

fn complete_utf8_prefix_len(bytes: &[u8]) -> usize {
    let Some(mut start) = bytes.len().checked_sub(1) else {
        return 0;
    };

    while start > 0 && is_utf8_continuation(bytes[start]) {
        start -= 1;
    }

    let first = bytes[start];
    let Some(expected_len) = utf8_sequence_len(first) else {
        return bytes.len();
    };

    let available_len = bytes.len() - start;
    if available_len >= expected_len {
        return bytes.len();
    }

    if bytes[start + 1..]
        .iter()
        .all(|byte| is_utf8_continuation(*byte))
    {
        start
    } else {
        bytes.len()
    }
}

fn decode_terminal_chars_into(bytes: &[u8], chars: &mut Vec<char>) {
    let mut index = 0;

    while index < bytes.len() {
        if let Some(ch) = raw_c1_control(bytes[index]) {
            chars.push(ch);
            index += 1;
            continue;
        }

        match std::str::from_utf8(&bytes[index..]) {
            Ok(text) => {
                chars.extend(text.chars());
                break;
            }
            Err(error) if error.valid_up_to() > 0 => {
                let valid_end = index + error.valid_up_to();
                let text = std::str::from_utf8(&bytes[index..valid_end]).unwrap_or("");
                chars.extend(text.chars());
                index = valid_end;
            }
            Err(error) => {
                chars.push('\u{fffd}');
                index += error.error_len().unwrap_or(1);
            }
        }
    }
}

fn raw_c1_control(byte: u8) -> Option<char> {
    if (0x80..=0x9f).contains(&byte) {
        char::from_u32(u32::from(byte))
    } else {
        None
    }
}

fn utf8_sequence_len(byte: u8) -> Option<usize> {
    match byte {
        0x00..=0x7f => Some(1),
        0xc2..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf4 => Some(4),
        _ => None,
    }
}

fn is_utf8_continuation(byte: u8) -> bool {
    byte & 0b1100_0000 == 0b1000_0000
}

fn saturating_u8(value: u16) -> u8 {
    u8::try_from(value).unwrap_or(u8::MAX)
}

fn display_width(
    ch: char,
    unicode_version: u32,
    treat_east_asian_ambiguous_width_as_wide: bool,
    cell_width_overrides: &[CellWidthOverride],
) -> u16 {
    if let Some(override_width) = cell_width_overrides
        .iter()
        .find(|override_width| override_width.contains(ch))
        .map(|override_width| override_width.width)
    {
        return override_width;
    }

    if unicode_version <= 8 && is_widened_in_unicode9(ch) {
        return 1;
    }

    let width = if treat_east_asian_ambiguous_width_as_wide {
        UnicodeWidthChar::width_cjk(ch)
    } else {
        UnicodeWidthChar::width(ch)
    };
    match width {
        Some(0) => 0,
        Some(width) => u16_display_width(width),
        None => 1,
    }
}

fn grapheme_display_width(
    grapheme: &str,
    unicode_version: u32,
    treat_east_asian_ambiguous_width_as_wide: bool,
    cell_width_overrides: &[CellWidthOverride],
) -> u16 {
    let Some(ch) = grapheme.chars().next() else {
        return 0;
    };
    if let Some(override_width) = cell_width_overrides
        .iter()
        .find(|override_width| override_width.contains(ch))
        .map(|override_width| override_width.width)
    {
        return override_width;
    }

    if unicode_version <= 8 && is_widened_in_unicode9(ch) {
        return 1;
    }

    let width = if treat_east_asian_ambiguous_width_as_wide {
        grapheme.width_cjk()
    } else {
        grapheme.width()
    };
    u16_display_width(width)
}

fn u16_display_width(width: usize) -> u16 {
    if width > usize::from(u16::MAX) {
        u16::MAX
    } else {
        u16::try_from(width).unwrap_or(1)
    }
}

fn is_unicode_presentation_selector(ch: char) -> bool {
    matches!(ch, TEXT_PRESENTATION_SELECTOR | EMOJI_PRESENTATION_SELECTOR)
}

fn is_widened_in_unicode9(ch: char) -> bool {
    let codepoint = u32::from(ch);
    WIDENED_IN_UNICODE9
        .binary_search_by(|(start, end)| {
            if codepoint >= *start && codepoint <= *end {
                core::cmp::Ordering::Equal
            } else {
                start.cmp(&codepoint)
            }
        })
        .is_ok()
}

fn non_empty_unicode_version_label(label: &str) -> Option<String> {
    let label = label.trim();
    (!label.is_empty()).then(|| label.to_owned())
}

fn rebase_image_and_placeholder_metadata(
    inline_images: &mut Vec<ItermInlineImage>,
    inline_image_parent_ids: &mut Vec<u64>,
    inline_image_attachments: &mut Vec<CellAttachment>,
    kitty_placeholder_cells: &mut HashMap<(usize, u16), LastKittyPlaceholder>,
    last_kitty_placeholder: &mut Option<LastKittyPlaceholder>,
    removed_rows: usize,
) {
    let images = std::mem::take(inline_images);
    let mut parent_ids = std::mem::take(inline_image_parent_ids);
    let missing_parent_id_start = parent_ids.len();
    parent_ids.extend(
        (missing_parent_id_start..images.len())
            .map(|index| u64::MAX.saturating_sub(u64::try_from(index).unwrap_or(u64::MAX))),
    );
    let mut rebased_images = Vec::with_capacity(images.len());
    let mut rebased_parent_ids = Vec::with_capacity(parent_ids.len());
    for (mut image, parent_identity) in images.into_iter().zip(parent_ids) {
        let Some(row) = image.row.checked_sub(removed_rows) else {
            continue;
        };
        image.row = row;
        rebased_images.push(image);
        rebased_parent_ids.push(parent_identity);
    }
    let live_parent_ids = rebased_parent_ids.iter().copied().collect::<HashSet<_>>();
    *inline_images = rebased_images;
    *inline_image_parent_ids = rebased_parent_ids;
    *inline_image_attachments = inline_image_attachments
        .drain(..)
        .filter_map(|mut attachment| {
            attachment.row = attachment.row.checked_sub(removed_rows)?;
            live_parent_ids
                .contains(&attachment.parent_identity)
                .then_some(attachment)
        })
        .collect();
    *kitty_placeholder_cells = kitty_placeholder_cells
        .drain()
        .filter_map(|((row, column), mut placeholder)| {
            let row = row.checked_sub(removed_rows)?;
            placeholder.row = row;
            Some(((row, column), placeholder))
        })
        .collect();
    *last_kitty_placeholder = last_kitty_placeholder.and_then(|mut placeholder| {
        placeholder.row = placeholder.row.checked_sub(removed_rows)?;
        Some(placeholder)
    });
}

fn shift_image_and_placeholder_suffix_metadata(
    inline_images: &mut [ItermInlineImage],
    inline_image_attachments: &mut [CellAttachment],
    kitty_placeholder_cells: &mut HashMap<(usize, u16), LastKittyPlaceholder>,
    last_kitty_placeholder: &mut Option<LastKittyPlaceholder>,
    suffix_start: usize,
    rows: usize,
) {
    for image in inline_images {
        if image.row >= suffix_start {
            image.row = image
                .row
                .checked_add(rows)
                .expect("inline image row overflow");
        }
    }
    for attachment in inline_image_attachments {
        if attachment.row >= suffix_start {
            attachment.row = attachment
                .row
                .checked_add(rows)
                .expect("inline image attachment row overflow");
        }
    }
    *kitty_placeholder_cells = kitty_placeholder_cells
        .drain()
        .map(|((row, column), mut placeholder)| {
            let row = if row >= suffix_start {
                row.checked_add(rows)
                    .expect("kitty placeholder row overflow")
            } else {
                row
            };
            placeholder.row = row;
            ((row, column), placeholder)
        })
        .collect();
    if let Some(placeholder) = last_kitty_placeholder
        && placeholder.row >= suffix_start
    {
        placeholder.row = placeholder
            .row
            .checked_add(rows)
            .expect("last kitty placeholder row overflow");
    }
}

#[cfg(test)]
impl Drop for Terminal {
    fn drop(&mut self) {
        task10_trace::trace_drop(self);
    }
}

#[cfg(test)]
#[path = "parser_task10_trace.rs"]
mod task10_trace;

#[cfg(test)]
#[path = "parser_ascii_fast_path_tests.rs"]
mod ascii_fast_path_tests;

#[cfg(test)]
mod stable_row_tests {
    use super::*;

    mod task10_registry {
        include!("parser_task10_registry.rs");
    }

    const HIGH_BYTE_KITTY_IMAGE_ID: u32 = 0x0200_001e;

    #[test]
    fn terminal_preserves_decomposed_grapheme_across_feed_calls() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 1));

        terminal.feed(b"e");
        terminal.feed("\u{301}".as_bytes());

        assert_eq!(
            terminal.text_from_region(0, 0, 7, 0).as_deref(),
            Some("e\u{301}")
        );
        assert_eq!(terminal.cursor(), (0, 1));
    }

    #[test]
    fn terminal_treats_family_emoji_as_one_two_column_grapheme() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 1));

        for scalar in "👨‍👩‍👧‍👦".chars() {
            let mut encoded = [0; 4];
            terminal.feed(scalar.encode_utf8(&mut encoded).as_bytes());
        }

        assert_eq!(terminal.text_from_region(0, 0, 7, 0).as_deref(), Some("👨‍👩‍👧‍👦"));
        assert_eq!(terminal.cursor(), (0, 2));
    }

    #[test]
    fn terminal_preserves_extended_grapheme_matrix_and_continuations() {
        let cases = [
            ("e\u{301}", 1),
            ("ن\u{64e}", 1),
            ("क\u{94d}ष", 2),
            ("☁\u{fe0e}", 1),
            ("☁\u{fe0f}", 2),
            ("👍🏽", 2),
            ("🇺🇸", 2),
            ("👨‍👩‍👧‍👦", 2),
            ("1\u{fe0f}\u{20e3}", 2),
        ];

        for (grapheme, columns) in cases {
            let mut terminal = Terminal::new(TerminalSize::new(8, 1));
            terminal.set_unicode_version(14);
            for ch in grapheme.chars() {
                let mut encoded = [0; 4];
                terminal.feed(ch.encode_utf8(&mut encoded).as_bytes());
            }

            let leader = terminal.grid.get(0, 0).unwrap();
            assert_eq!(leader.text(), grapheme, "{grapheme:?}");
            assert_eq!(leader.columns(), columns, "{grapheme:?}");
            assert_eq!(terminal.cursor(), (0, u16::from(columns)), "{grapheme:?}");
            if columns == 2 {
                assert!(terminal.grid.get(0, 1).unwrap().is_continuation());
                assert_eq!(terminal.grid.get(0, 1).unwrap().text(), "");
            }
        }
    }

    #[test]
    fn terminal_rep_repeats_complete_grapheme_without_duplicate_continuation_text() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 1));
        terminal.feed("👍🏽".as_bytes());
        terminal.feed(b"\x1b[2b");

        assert_eq!(
            terminal.text_from_region(0, 0, 7, 0).as_deref(),
            Some("👍🏽👍🏽👍🏽")
        );
        assert_eq!(terminal.cursor(), (0, 6));
        assert!(terminal.grid.get(0, 1).unwrap().is_continuation());
        assert!(terminal.grid.get(0, 3).unwrap().is_continuation());
        assert!(terminal.grid.get(0, 5).unwrap().is_continuation());
    }

    #[test]
    fn terminal_preserves_grapheme_extension_at_right_boundary() {
        let mut terminal = Terminal::new(TerminalSize::new(1, 1));
        terminal.set_unicode_version(14);
        terminal.feed("☁".as_bytes());
        terminal.feed("\u{fe0f}".as_bytes());

        assert_eq!(terminal.grid.get(0, 0).unwrap().text(), "☁\u{fe0f}");
        assert_eq!(terminal.grid.get(0, 0).unwrap().columns(), 2);
        let narrow_cells = terminal.grid.cells_with_reflow_overflow(0);
        assert_eq!(narrow_cells.len(), 2);
        assert!(matches!(
            narrow_cells[1].content(),
            crate::CellContent::Continuation { leader_delta: 1 }
        ));
        assert_eq!(terminal.cursor(), (0, 0));

        terminal.resize(TerminalSize::new(2, 1));

        assert_eq!(terminal.grid.get(0, 0).unwrap().text(), "☁\u{fe0f}");
        assert!(terminal.grid.get(0, 1).unwrap().is_continuation());
        assert_eq!(
            terminal.text_from_region(0, 0, 1, 0).as_deref(),
            Some("☁\u{fe0f}")
        );
        assert_eq!(terminal.cursor(), (0, 0));
    }

    #[test]
    fn terminal_shrinking_grapheme_clears_narrow_reflow_overflow() {
        let mut terminal = Terminal::new(TerminalSize::new(1, 1));
        terminal.set_unicode_version(14);
        terminal.feed("⌚".as_bytes());
        assert_eq!(terminal.grid.get(0, 0).unwrap().columns(), 2);
        assert!(terminal.grid.cells_with_reflow_overflow(0)[1].is_continuation());

        terminal.feed("\u{fe0e}".as_bytes());

        assert_eq!(terminal.grid.get(0, 0).unwrap().text(), "⌚\u{fe0e}");
        assert_eq!(terminal.grid.get(0, 0).unwrap().columns(), 1);
        assert_eq!(terminal.grid.cells_with_reflow_overflow(0).len(), 1);

        terminal.resize(TerminalSize::new(2, 1));

        assert_eq!(terminal.grid.get(0, 0).unwrap().columns(), 1);
        assert!(terminal.grid.get(0, 1).unwrap().is_blank());
        assert!(!terminal.grid.get(0, 1).unwrap().is_continuation());
        assert_eq!(
            terminal.text_from_region(0, 0, 1, 0).as_deref(),
            Some("⌚\u{fe0e}")
        );
    }

    #[test]
    fn terminal_clamps_large_width_override_to_cell_column_capacity() {
        let mut terminal = Terminal::new(TerminalSize::new(300, 1));
        terminal.set_cell_width_overrides(vec![CellWidthOverride::new(
            u32::from('x'),
            u32::from('x'),
            300,
        )]);
        terminal.feed(b"x");

        assert_eq!(terminal.grid.get(0, 0).unwrap().columns(), u8::MAX);
        assert_eq!(terminal.cursor(), (0, 255));
        assert!(terminal.grid.get(0, 254).unwrap().is_continuation());
        assert!(terminal.grid.get(0, 255).unwrap().is_blank());
    }

    #[test]
    fn terminal_character_edits_do_not_leave_orphan_grapheme_cells() {
        for (cursor, count, expected) in [(2, 1, "AB"), (3, 1, "AB"), (1, 2, "B")] {
            let mut terminal = Terminal::new(TerminalSize::new(6, 1));
            terminal.feed("A👍🏽B".as_bytes());
            terminal.feed(format!("\x1b[1;{cursor}H\x1b[{count}P").as_bytes());

            assert_eq!(
                terminal.text_from_region(0, 0, 5, 0).as_deref(),
                Some(expected),
                "cursor={cursor} count={count}"
            );
            assert!(
                (0..6).all(|column| !terminal.grid.get(0, column).unwrap().is_continuation()),
                "cursor={cursor} count={count}"
            );
        }
    }

    #[test]
    fn terminal_insert_characters_moves_complete_grapheme_spans() {
        for cursor in [2, 3] {
            let mut terminal = Terminal::new(TerminalSize::new(7, 1));
            terminal.feed("A👍🏽B".as_bytes());
            terminal.feed(format!("\x1b[1;{cursor}H\x1b[@").as_bytes());

            assert_eq!(
                terminal.text_from_region(0, 0, 6, 0).as_deref(),
                Some("A 👍🏽B"),
                "cursor={cursor}"
            );
            assert_eq!(terminal.grid.get(0, 2).unwrap().text(), "👍🏽");
            assert!(terminal.grid.get(0, 3).unwrap().is_continuation());
        }
    }

    #[test]
    fn terminal_margin_insert_at_continuation_keeps_exterior_cells() {
        for insertion in ["\x1b[@", "\x1b[4hX\x1b[4l"] {
            let mut terminal = Terminal::new(TerminalSize::new(6, 1));
            terminal.feed("A界BCD".as_bytes());
            terminal.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;3H");
            terminal.feed(insertion.as_bytes());

            assert_eq!(terminal.grid.get(0, 0).unwrap().text(), "A");
            assert_eq!(terminal.grid.get(0, 5).unwrap().text(), "D");
        }
    }

    #[test]
    fn terminal_narrow_grapheme_extension_damage_stays_inside_viewport() {
        let mut terminal = Terminal::new(TerminalSize::new(1, 1));
        terminal.set_unicode_version(14);
        terminal.feed("☁".as_bytes());
        terminal.take_damage();

        terminal.feed("\u{fe0f}".as_bytes());

        assert_eq!(terminal.take_damage(), vec![DamageRegion::new(0, 0, 1, 1)]);
    }

    #[test]
    fn terminal_selection_normalizes_continuation_boundaries_to_the_leader() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.feed("👍🏽".as_bytes());

        assert_eq!(terminal.text_from_region(1, 0, 1, 0).as_deref(), Some("👍🏽"));
        assert_eq!(terminal.text_from_region(0, 0, 1, 0).as_deref(), Some("👍🏽"));

        let continuation = stable_coordinate(&terminal, 0, 1);
        let stable = stable_selection(continuation, continuation, false);
        let rectangular = stable_selection(continuation, continuation, true);
        assert_eq!(
            terminal.text_from_stable_selection(stable).as_deref(),
            Some("👍🏽")
        );
        assert_eq!(
            terminal.text_from_stable_selection(rectangular).as_deref(),
            Some("👍🏽")
        );
    }

    #[test]
    fn terminal_grapheme_survives_copy_resize_reflow_and_scrollback() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed("A👨‍👩‍👧‍👦B\r\n1234\r\n".as_bytes());
        let family_line = terminal
            .scrollback
            .iter()
            .find(|line| line.cells().iter().any(|cell| cell.text() == "👨‍👩‍👧‍👦"))
            .expect("family grapheme reached scrollback");
        assert_eq!(family_line.cells()[1].text(), "👨‍👩‍👧‍👦");
        assert!(family_line.cells()[2].is_continuation());

        terminal.resize(TerminalSize::new(3, 2));

        assert_eq!(
            terminal.text_from_region(0, 0, 2, 0).as_deref(),
            Some("A👨‍👩‍👧‍👦")
        );
    }

    #[test]
    fn terminal_work_counters_track_executed_scroll_and_prune_operations() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.set_scrollback_limit(1);

        assert_eq!(terminal.work_counters(), TerminalWorkCounters::default());

        terminal.feed(b"a\r\nb\r\nc\r\n");

        assert_eq!(
            terminal.work_counters(),
            TerminalWorkCounters {
                scrolled_survivor_cell_clones: 0,
                history_row_relocations: 0,
                metadata_rebase_batches: 1,
            }
        );
    }

    #[test]
    fn bounded_history_eviction_does_not_relocate_survivors() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"line-00\r\nline-01\r\nline-02\r\nline-03\r\nline-04\r\nline-05");
        let before = terminal.work_counters();

        terminal.set_scrollback_limit(2);

        let work = terminal.work_counters().saturating_delta_since(before);
        assert_eq!(terminal.scrollback.len(), 2);
        assert_eq!(
            terminal
                .scrollback
                .iter()
                .map(|line| line
                    .cells()
                    .iter()
                    .map(Cell::primary_char)
                    .collect::<String>())
                .collect::<Vec<_>>(),
            ["line-02 ", "line-03 "]
        );
        assert_eq!(work.history_row_relocations, 0);
        assert_eq!(work.metadata_rebase_batches, 1);
    }

    #[test]
    fn history_prune_preserves_cell_attachment_source_and_stable_selection() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"old-line\r\nkeep-one\r\nkeep-two\r\nvisible!");
        assert_eq!(terminal.scrollback.len(), 2);
        terminal.push_inline_image(attachment_test_image(1, 2, 2, 1, "survivor"));
        let selection = stable_selection(
            stable_coordinate(&terminal, 1, 0),
            stable_coordinate(&terminal, 1, 7),
            false,
        );
        let sources = terminal
            .inline_image_attachments
            .iter()
            .map(|attachment| (attachment.source_row, attachment.source_column))
            .collect::<Vec<_>>();

        terminal.set_scrollback_limit(1);

        assert_eq!(
            terminal.text_from_stable_selection(selection),
            Some("keep-one".to_owned())
        );
        assert_eq!(
            terminal
                .inline_image_attachments
                .iter()
                .map(|attachment| (attachment.source_row, attachment.source_column))
                .collect::<Vec<_>>(),
            sources
        );
        assert_eq!(
            attachment_locations(&terminal),
            vec![(1, 0, 0, 0, 2), (1, 0, 1, 0, 3)]
        );
    }

    #[test]
    fn batched_scroll_prune_matches_incremental_prune() {
        let mut seed = Terminal::new(TerminalSize::new(8, 4));
        seed.set_scrollback_limit(2);
        seed.feed(b"line-00\r\nline-01\r\nline-02\r\nline-03\r\nline-04\r\nline-05");
        assert_eq!(seed.scrollback.len(), 2);
        seed.push_inline_image(attachment_test_image(4, 2, 2, 1, "survivor"));

        let mut batched = seed.clone();
        let mut incremental = seed;
        let batched_before = batched.work_counters();
        let incremental_before = incremental.work_counters();

        batched.scroll_up_region_by(0, 3, 2);
        incremental.scroll_up_region_by(0, 3, 1);
        incremental.scroll_up_region_by(0, 3, 1);

        assert_eq!(batched.scrollback, incremental.scrollback);
        assert_eq!(batched.stable_dimensions(), incremental.stable_dimensions());
        assert_eq!(main_screen_rows(&batched), main_screen_rows(&incremental));
        assert_eq!(batched.inline_images, incremental.inline_images);
        assert_eq!(
            batched.inline_image_attachments,
            incremental.inline_image_attachments
        );
        assert_eq!(
            batched
                .work_counters()
                .saturating_delta_since(batched_before)
                .metadata_rebase_batches,
            1
        );
        assert_eq!(
            incremental
                .work_counters()
                .saturating_delta_since(incremental_before)
                .metadata_rebase_batches,
            2
        );
    }

    #[test]
    fn alternate_prune_rebases_active_and_dormant_attachment_destinations() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"line-00\r\nline-01\r\nline-02\r\nline-03");
        let history_rows = terminal.scrollback.len();
        assert_eq!(history_rows, 2);
        terminal.push_inline_image(attachment_test_image(history_rows - 1, 1, 2, 1, "main"));
        terminal.feed(b"\x1b[?1049h");
        terminal.push_inline_image(attachment_test_image(history_rows, 3, 2, 1, "alternate"));

        terminal.prune_scrollback_rows(1);

        assert_eq!(
            attachment_locations(&terminal),
            vec![
                (2, 0, 0, history_rows - 1, 3),
                (2, 0, 1, history_rows - 1, 4)
            ]
        );
        let dormant_main = terminal.main_screen.as_ref().expect("dormant main screen");
        assert_eq!(
            dormant_main
                .inline_image_attachments
                .iter()
                .map(|attachment| {
                    (
                        attachment.parent_identity,
                        attachment.source_row,
                        attachment.source_column,
                        attachment.row,
                        attachment.column,
                    )
                })
                .collect::<Vec<_>>(),
            vec![(1, 0, 0, 0, 1), (1, 0, 1, 0, 2)]
        );
    }

    #[test]
    fn history_container_reflow_preserves_wrapped_and_overflow_cells() {
        let mut terminal = Terminal::new(TerminalSize::new(1, 1));
        terminal.feed(b"\x1b[31m");
        terminal.feed("界".as_bytes());
        terminal.feed(b"x");
        assert_eq!(terminal.scrollback.len(), 2);
        let before = terminal
            .scrollback
            .iter()
            .map(|line| (line.cells_with_reflow_overflow(), line.is_wrapped()))
            .collect::<Vec<_>>();
        assert_eq!(before[1].0.len(), 2);
        assert_eq!(before[1].0[0].primary_char(), '界');
        assert_eq!(before[1].0[1].foreground, Color::Indexed(1));

        terminal.resize(TerminalSize::new(2, 1));

        let rows = main_screen_rows(&terminal);
        assert_eq!(rows, [("界 ".to_owned(), false), ("x ".to_owned(), true)]);
        let wide = terminal
            .scrollback
            .get(terminal.scrollback.len().saturating_sub(1))
            .expect("wide row remains in history");
        assert_eq!(wide.cells()[0].primary_char(), '界');
        assert_eq!(wide.cells()[1].primary_char(), ' ');
        assert_eq!(wide.cells()[1].foreground, Color::Indexed(1));
    }

    #[test]
    fn terminal_work_counter_delta_excludes_prior_work_and_saturates() {
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

        assert_eq!(
            later.saturating_delta_since(earlier),
            TerminalWorkCounters {
                scrolled_survivor_cell_clones: 25,
                history_row_relocations: 0,
                metadata_rebase_batches: 2,
            }
        );
    }

    #[test]
    fn terminal_scroll_clone_counter_matches_full_and_bounded_copy_regions() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));

        terminal.scroll_down_region(0, 3, 1);
        terminal.scroll_up_region_by(0, 3, 2);
        terminal.scroll_down_bounded_cells(0, 3, 1, 2, 5);
        terminal.scroll_up_bounded_cells(0, 3, 3, 2, 5);

        // Full-width row rotations clone no surviving cells. Bounded down:
        // 3*4, bounded up: 1*4. These are exactly the remaining cell-copy
        // loop iteration counts.
        assert_eq!(
            terminal.work_counters().scrolled_survivor_cell_clones,
            12 + 4
        );
    }

    #[test]
    fn full_screen_scroll_does_not_clone_surviving_cells() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.feed(b"row-000\r\nrow-111\r\nrow-222\r\nrow-333");
        let before = terminal.work_counters();

        terminal.scroll_up_region_by(0, 3, 1);

        assert_eq!(
            terminal
                .work_counters()
                .saturating_delta_since(before)
                .scrolled_survivor_cell_clones,
            0
        );
    }

    #[test]
    fn partial_region_scroll_preserves_exterior_rows() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 5));
        for row in 0..5 {
            for column in 0..4 {
                let mut cell = Cell::with_char(char::from(b'A' + u8::try_from(row).unwrap()));
                cell.foreground = Color::Indexed(u8::try_from(row).unwrap());
                assert!(terminal.grid.set(row, column, cell));
            }
            terminal.grid.set_row_wrapped(row, row % 2 == 0);
            assert!(
                terminal
                    .grid
                    .set_row_last_change_seqno(row, 100 + usize::from(row))
            );
        }
        terminal
            .grid
            .set_reflow_overflow(0, vec![Cell::with_char('↑')]);
        terminal
            .grid
            .set_reflow_overflow(4, vec![Cell::with_char('↓')]);
        let top = terminal.grid.cells_with_reflow_overflow(0);
        let bottom = terminal.grid.cells_with_reflow_overflow(4);
        let top_wrapped = terminal.grid.row_wrapped(0);
        let bottom_wrapped = terminal.grid.row_wrapped(4);
        let top_seqno = terminal.grid.row_last_change_seqno(0);
        let bottom_seqno = terminal.grid.row_last_change_seqno(4);

        terminal.scroll_up_region_by(1, 3, 1);

        assert_eq!(terminal.grid.cells_with_reflow_overflow(0), top);
        assert_eq!(terminal.grid.cells_with_reflow_overflow(4), bottom);
        assert_eq!(terminal.grid.row_wrapped(0), top_wrapped);
        assert_eq!(terminal.grid.row_wrapped(4), bottom_wrapped);
        assert_eq!(terminal.grid.row_last_change_seqno(0), top_seqno);
        assert_eq!(terminal.grid.row_last_change_seqno(4), bottom_seqno);
    }

    #[test]
    fn row_rotation_preserves_wrapped_overflow_and_seqno() {
        let mut terminal = Terminal::new(TerminalSize::new(2, 3));
        let source_seqno = 4242;
        let mut first = Cell::with_char('S');
        first.foreground = Color::Indexed(5);
        assert!(terminal.grid.set(1, 0, first));
        let mut second = Cell::with_char('T');
        second.background = Color::Indexed(6);
        assert!(terminal.grid.set(1, 1, second));
        let mut overflow = Cell::with_char('界');
        overflow.hyperlink = Some("overflow".into());
        terminal.grid.set_reflow_overflow(1, vec![overflow]);
        terminal.grid.set_row_wrapped(1, true);
        assert!(terminal.grid.set_row_last_change_seqno(1, source_seqno));
        let expected = terminal.grid.cells_with_reflow_overflow(1);

        terminal.scroll_up_region_by(0, 2, 1);

        assert_eq!(terminal.grid.cells_with_reflow_overflow(0), expected);
        assert!(terminal.grid.row_wrapped(0));
        assert_eq!(terminal.grid.row_last_change_seqno(0), Some(source_seqno));
    }

    #[test]
    fn row_to_history_moves_cells_without_duplicate_clone() {
        let mut terminal = Terminal::new(TerminalSize::new(2, 2));
        let hyperlink = "https://example.test/owned-row-allocation".repeat(4);
        let mut cell = Cell::with_char('x');
        cell.hyperlink = Some(hyperlink.into());
        assert!(terminal.grid.set(0, 0, cell));
        let allocation = terminal
            .grid
            .get(0, 0)
            .and_then(|cell| cell.hyperlink.as_ref())
            .map(|hyperlink| hyperlink.as_ptr())
            .expect("test row has an owned allocation");

        terminal.scroll_up_region_by(0, 1, 1);

        let recorded_allocation = terminal
            .scrollback
            .last()
            .and_then(|line| line.cells().first())
            .and_then(|cell| cell.hyperlink.as_ref())
            .map(|hyperlink| hyperlink.as_ptr())
            .expect("scrolled row was recorded in history");
        assert_eq!(recorded_allocation, allocation);
    }

    fn stable_coordinate(
        terminal: &Terminal,
        history_row: usize,
        column: usize,
    ) -> crate::StableSelectionCoordinate {
        crate::StableSelectionCoordinate {
            domain: terminal.stable_dimensions().domain,
            row: terminal
                .history_index_to_stable_row(history_row)
                .expect("history row has a stable identity"),
            column,
        }
    }

    fn stable_selection(
        start: crate::StableSelectionCoordinate,
        end: crate::StableSelectionCoordinate,
        rectangular: bool,
    ) -> crate::StableSelectionRange {
        crate::StableSelectionRange {
            start,
            end,
            rectangular,
        }
    }

    fn stable_row_text(terminal: &Terminal, row: StableRowIndex) -> Option<String> {
        let history_row = terminal.stable_row_to_history_index(row)?;
        Some(
            terminal
                .cells_for_history_row(history_row)?
                .iter()
                .map(Cell::primary_char)
                .collect(),
        )
    }

    fn stable_row_seqno(terminal: &Terminal, row: StableRowIndex) -> Option<SequenceNo> {
        let history_row = terminal.stable_row_to_history_index(row)?;
        if let Some(line) = terminal.scrollback.get(history_row) {
            return Some(line.last_change_seqno());
        }
        let grid_row = u16::try_from(history_row.checked_sub(terminal.scrollback.len())?).ok()?;
        terminal.grid.row_last_change_seqno(grid_row)
    }

    fn metadata_test_image(row: usize, name: &str) -> ItermInlineImage {
        ItermInlineImage {
            row,
            column: 0,
            name: Some(name.to_owned()),
            kitty_image_id: None,
            kitty_placement_id: None,
            kitty_z_index: None,
            size: None,
            width: None,
            height: None,
            preserve_aspect_ratio: None,
            image_format: InlineImageFormat::Encoded,
            pixel_width: None,
            pixel_height: None,
            source_x: None,
            source_y: None,
            source_width: None,
            source_height: None,
            target_x: None,
            target_y: None,
            data: Vec::new(),
        }
    }

    fn metadata_test_placeholder(row: usize) -> LastKittyPlaceholder {
        LastKittyPlaceholder {
            row,
            column: 0,
            foreground: Color::Default,
            underline_color: Color::Default,
            image_id_high_byte: 0,
            placeholder_row: 0,
            placeholder_column: 0,
        }
    }

    fn attachment_test_image(
        row: usize,
        column: u16,
        width: u16,
        height: u16,
        name: &str,
    ) -> ItermInlineImage {
        let mut image = metadata_test_image(row, name);
        image.column = column;
        image.width = Some(width.to_string());
        image.height = Some(height.to_string());
        image
    }

    fn attachment_locations(terminal: &Terminal) -> Vec<(u64, u16, u16, usize, u16)> {
        let mut locations = terminal
            .inline_image_attachments
            .iter()
            .map(|attachment| {
                (
                    attachment.parent_identity,
                    attachment.source_row,
                    attachment.source_column,
                    attachment.row,
                    attachment.column,
                )
            })
            .collect::<Vec<_>>();
        locations.sort_unstable();
        locations
    }

    fn install_suffix_metadata(terminal: &mut Terminal, row: usize) {
        terminal.semantic_prompt_rows.push(row);
        terminal.semantic_command_exits.push(SemanticCommandExit {
            row,
            exit_code: Some(0),
            aid: Some("suffix".to_owned()),
        });
        terminal
            .inline_images
            .push(metadata_test_image(row, "suffix"));
        let placeholder = metadata_test_placeholder(row);
        terminal
            .kitty_placeholder_cells
            .insert((row, 0), placeholder);
        terminal.last_kitty_placeholder = Some(placeholder);
    }

    fn assert_suffix_metadata_row(terminal: &Terminal, row: usize) {
        assert_eq!(terminal.semantic_prompt_rows, vec![row]);
        assert_eq!(terminal.semantic_command_exits[0].row, row);
        assert_eq!(terminal.inline_images[0].row, row);
        assert!(terminal.kitty_placeholder_cells.contains_key(&(row, 0)));
        assert_eq!(
            terminal
                .last_kitty_placeholder
                .expect("last placeholder")
                .row,
            row
        );
    }

    fn install_cross_boundary_real_images(terminal: &mut Terminal) {
        terminal.feed(b"\x1b[2;1H");
        terminal.feed(b"\x1b]1337;File=inline=1;name=Y3Jvc3M=;width=1;height=2:QUJDRA==\x07");
        terminal.feed(b"\x1b_Ga=t,i=30,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
        terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;AP8A\x1b\\");
        terminal.take_kitty_graphics_responses();
        terminal.feed(b"\x1b_Ga=p,i=30,p=4,c=2,r=2,C=1\x1b\\");
        terminal.feed(b"\x1b_Ga=p,i=7,p=2,P=30,Q=4,H=0,V=2,c=1,r=1,C=1\x1b\\");
        terminal.take_kitty_graphics_responses();

        assert_eq!(terminal.inline_images.len(), 3);
        assert!(
            terminal
                .inline_images
                .iter()
                .any(|image| image.name.as_deref() == Some("cross"))
        );
        assert!(
            terminal
                .inline_images
                .iter()
                .any(|image| image.kitty_image_id == Some(30))
        );
        assert!(
            terminal
                .inline_images
                .iter()
                .any(|image| image.kitty_image_id == Some(7))
        );
        assert_eq!(terminal.kitty_relative_parents.get(&(7, 2)), Some(&(30, 4)));
    }

    #[test]
    fn terminal_stable_dimensions_start_on_main_screen() {
        let terminal = Terminal::new(TerminalSize::new(4, 3));

        assert_eq!(
            terminal.stable_dimensions(),
            TerminalStableDimensions {
                domain: TerminalScreenDomain::Main,
                viewport_rows: 3,
                scrollback_rows: 3,
                scrollback_top: 0,
                physical_top: 0,
            }
        );
        assert_eq!(terminal.retained_stable_range(), 0..3);
    }

    #[test]
    fn terminal_stable_text_reads_offscreen_rows() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        terminal.feed(b"first\r\nsecond\r\nthird");
        let start = stable_coordinate(&terminal, 0, 1);
        let end = stable_coordinate(&terminal, 0, 3);

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(start, end, false)),
            Some("irs".to_owned())
        );
    }

    #[test]
    fn terminal_stable_text_returns_surviving_partial_prefix_prune() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        terminal.feed(b"first\r\nsecond\r\nthird");
        let start = stable_coordinate(&terminal, 0, 2);
        let end = stable_coordinate(&terminal, 2, 2);

        terminal.set_scrollback_limit(0);

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(start, end, false)),
            Some("second\nthi".to_owned())
        );
    }

    #[test]
    fn terminal_stable_text_returns_surviving_partial_suffix_prune() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 3));
        terminal.feed(b"first\r\nsecond\r\nthird");
        let start = stable_coordinate(&terminal, 0, 2);
        let end = stable_coordinate(&terminal, 2, 2);

        terminal.resize(TerminalSize::new(6, 2));

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(start, end, false)),
            Some("rst\nsecond".to_owned())
        );
    }

    #[test]
    fn terminal_stable_text_reverse_anchor_focus_survives_partial_prune() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        terminal.feed(b"first\r\nsecond\r\nthird");
        let anchor = stable_coordinate(&terminal, 2, 2);
        let focus = stable_coordinate(&terminal, 0, 2);

        terminal.set_scrollback_limit(0);

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(anchor, focus, false)),
            Some("second\nthi".to_owned())
        );
    }

    #[test]
    fn terminal_stable_text_rejects_mixed_or_inactive_domains() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        terminal.feed(b"main");
        let main = stable_coordinate(&terminal, 0, 0);
        let alternate = crate::StableSelectionCoordinate {
            domain: TerminalScreenDomain::Alternate,
            row: 0,
            column: 1,
        };

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(main, alternate, false)),
            None
        );

        terminal.feed(b"\x1b[?1049h");

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(main, main, false)),
            None
        );
    }

    #[test]
    fn terminal_stable_rectangular_text_keeps_original_columns_after_prune() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        terminal.feed(b"abcdef\r\nghijkl\r\nmnopqr");
        let start = stable_coordinate(&terminal, 0, 1);
        let end = stable_coordinate(&terminal, 2, 3);

        terminal.set_scrollback_limit(0);

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(start, end, true)),
            Some("hij\nnop".to_owned())
        );
    }

    #[test]
    fn terminal_stable_soft_wrapped_text_joins_surviving_spans() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"abcdefgh");
        let start = stable_coordinate(&terminal, 0, 1);
        let end = stable_coordinate(&terminal, 1, 2);

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(start, end, false)),
            Some("bcdefg".to_owned())
        );
    }

    #[test]
    fn terminal_stable_text_fully_pruned_returns_none() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        terminal.feed(b"first\r\nsecond\r\nthird");
        let start = stable_coordinate(&terminal, 0, 1);
        let end = stable_coordinate(&terminal, 0, 3);

        terminal.set_scrollback_limit(0);

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(start, end, false)),
            None
        );
    }

    #[test]
    fn terminal_stable_text_same_row_reverse_selection() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 1));
        terminal.feed(b"abcdef");
        let anchor = stable_coordinate(&terminal, 0, 4);
        let focus = stable_coordinate(&terminal, 0, 1);

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(anchor, focus, false)),
            Some("bcde".to_owned())
        );
    }

    #[test]
    fn terminal_stable_rectangular_text_supports_reverse_anchor_focus() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 3));
        terminal.feed(b"abcdef\r\nghijkl\r\nmnopqr");
        let anchor = stable_coordinate(&terminal, 2, 4);
        let focus = stable_coordinate(&terminal, 0, 1);

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(anchor, focus, true)),
            Some("bcde\nhijk\nnopq".to_owned())
        );
    }

    #[test]
    fn terminal_alternate_legacy_text_from_region_preserves_history_grid_ordinals() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        terminal.feed(b"main1\r\nmain2\r\nmain3");
        let grid_top = terminal.scrollback.len();

        terminal.feed(b"\x1b[?1049h");
        terminal.feed(b"alt");

        assert_eq!(
            terminal.text_from_region(0, 0, 4, 0),
            Some("main1".to_owned())
        );
        assert_eq!(
            terminal.text_from_region(0, grid_top, 2, grid_top),
            Some("alt".to_owned())
        );
    }

    #[test]
    fn terminal_alternate_legacy_text_from_semantic_zone_preserves_ordinals() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"main\r\nsecond\r\nthird");
        let grid_top = terminal.scrollback.len();

        terminal.feed(b"\x1b[?1049h");
        terminal.feed(b"\x1b]133;A\x07> \x1b]133;B\x07run");
        let main_zone = terminal.semantic_zone_at(1, 0).expect("main zone");
        let input_zone = terminal
            .semantic_zone_at(3, grid_top)
            .expect("alternate input zone");

        assert_eq!(
            terminal.text_from_semantic_zone(main_zone),
            Some("main".to_owned())
        );
        assert_eq!(
            terminal.text_from_semantic_zone(input_zone),
            Some("run".to_owned())
        );
    }

    #[test]
    fn terminal_screen_domain_changes_on_alternate_switch() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        let main = terminal.stable_dimensions();

        terminal.feed(b"\x1b[?1049h");
        let alternate = terminal.stable_dimensions();

        assert_eq!(main.domain, TerminalScreenDomain::Main);
        assert_eq!(alternate.domain, TerminalScreenDomain::Alternate);
        assert_eq!(alternate.scrollback_rows, alternate.viewport_rows);
        assert_eq!(alternate.scrollback_top, 0);
        assert_eq!(alternate.physical_top, 0);

        terminal.feed(b"\x1b[?1049l");

        assert_eq!(terminal.stable_dimensions(), main);
    }

    #[test]
    fn terminal_screen_identity_generation_tracks_same_feed_roundtrip() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        let before = terminal.screen_identity_generation();

        terminal.feed(b"\x1b[?1049hinside\x1b[?1049l");

        assert_eq!(
            terminal.screen_identity_generation(),
            before.checked_add(2).unwrap()
        );
        assert_eq!(
            terminal.stable_dimensions().domain,
            TerminalScreenDomain::Main
        );
    }

    #[test]
    fn terminal_screen_identity_generation_ignores_noop_screen_sets() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        let before = terminal.screen_identity_generation();

        terminal.feed(b"\x1b[?1049l\x1b[?1049l");
        assert_eq!(terminal.screen_identity_generation(), before);

        terminal.feed(b"\x1b[?1049h\x1b[?1049h");
        assert_eq!(
            terminal.screen_identity_generation(),
            before.checked_add(1).unwrap()
        );

        terminal.feed(b"\x1b[?1049l\x1b[?1049l");
        assert_eq!(
            terminal.screen_identity_generation(),
            before.checked_add(2).unwrap()
        );
    }

    #[test]
    fn terminal_screen_identity_generation_tracks_hard_reset_and_full_erase() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"data");
        let before = terminal.screen_identity_generation();

        terminal.feed(b"\x1bc");
        assert_eq!(
            terminal.screen_identity_generation(),
            before.checked_add(1).unwrap()
        );

        terminal.feed(b"\x1b[2J");
        assert_eq!(
            terminal.screen_identity_generation(),
            before.checked_add(2).unwrap()
        );
    }

    #[test]
    fn terminal_screen_identity_generation_ignores_partial_selective_and_scrollback_erases() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"data");
        let before = terminal.screen_identity_generation();

        terminal.feed(b"\x1b[J\x1b[1J\x1b[?2J\x1b[3J");

        assert_eq!(terminal.screen_identity_generation(), before);
    }

    #[test]
    fn terminal_alternate_stable_text_never_reads_main_history() {
        let mut terminal = Terminal::new(TerminalSize::new(5, 2));
        terminal.feed(b"main1\r\nmain2\r\nmain3");
        let main_row = stable_coordinate(&terminal, 0, 0);

        terminal.feed(b"\x1b[?1049h");
        terminal.feed(b"alt");
        let alternate_row = stable_coordinate(&terminal, 0, 0);

        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(
                alternate_row,
                crate::StableSelectionCoordinate {
                    column: 2,
                    ..alternate_row
                },
                false,
            )),
            Some("alt".to_owned())
        );
        assert_eq!(
            terminal.text_from_stable_selection(stable_selection(main_row, main_row, false)),
            None
        );
    }

    #[test]
    fn terminal_height_resize_reports_identity_boundary() {
        let mut terminal = Terminal::new(TerminalSize::new(5, 3));
        terminal.feed(b"one\r\ntwo\r\nthree");
        let before = terminal.stable_dimensions();

        terminal.resize(TerminalSize::new(5, 2));
        let after = terminal.stable_dimensions();

        assert_eq!(before.domain, after.domain);
        assert_eq!(before.viewport_rows, 3);
        assert_eq!(after.viewport_rows, 2);
        assert_ne!(before, after);
        assert_eq!(after.scrollback_top, before.scrollback_top);
        assert_eq!(after.physical_top, before.physical_top);
    }

    #[test]
    fn terminal_width_resize_marks_replaced_rows_changed() {
        let mut terminal = Terminal::new(TerminalSize::new(5, 3));
        let before = terminal.current_seqno();

        terminal.resize(TerminalSize::new(7, 3));

        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            terminal.retained_stable_range().collect::<Vec<_>>()
        );
    }

    #[test]
    fn terminal_stable_semantic_prompt_rows_survive_history_growth() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"\x1b]133;A\x07> one");
        let prompt = terminal.stable_semantic_prompt_rows();

        terminal.feed(b"\r\noutput\r\ntail");

        assert_eq!(prompt, vec![0]);
        assert_eq!(terminal.stable_semantic_prompt_rows(), prompt);
    }

    #[test]
    fn terminal_stable_semantic_zones_survive_prune_without_retargeting() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"\x1b]133;A\x07> \x1b]133;B\x07run\r\n\x1b]133;C\x07out\r\ntail");
        let removed = terminal
            .stable_semantic_zones()
            .into_iter()
            .find(|zone| zone.semantic_type == SemanticType::Prompt)
            .expect("prompt zone before prune");
        let surviving = terminal
            .stable_semantic_zones()
            .into_iter()
            .find(|zone| zone.start_y == 1 && zone.semantic_type == SemanticType::Output)
            .expect("output zone before prune");

        terminal.set_scrollback_limit(0);
        let zones = terminal.stable_semantic_zones();

        assert!(
            zones
                .iter()
                .all(|zone| zone.start_y != removed.start_y && zone.end_y != removed.end_y)
        );
        assert!(zones.iter().any(|zone| {
            zone.start_y == surviving.start_y
                && zone.end_y == surviving.end_y
                && zone.semantic_type == surviving.semantic_type
        }));
    }

    #[test]
    fn terminal_stable_semantic_zone_at_uses_stable_row() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"old\r\n\x1b]133;A\x07> \x1b]133;B\x07run\r\ntail");
        let input_row = terminal
            .stable_semantic_zones()
            .into_iter()
            .find(|zone| zone.semantic_type == SemanticType::Input)
            .expect("input zone")
            .start_y;

        terminal.set_scrollback_limit(1);

        assert_eq!(
            terminal
                .stable_semantic_zone_at(3, input_row)
                .map(|zone| zone.semantic_type),
            Some(SemanticType::Input)
        );
        assert_eq!(terminal.stable_semantic_zone_at(3, input_row - 1), None);
    }

    #[test]
    fn terminal_stable_semantic_command_exits_survive_prune() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"old\r\n\x1b]133;C\x07build\x1b]133;D;7;aid=42\x07\r\ntail");
        let before = terminal.stable_semantic_command_exits();

        terminal.set_scrollback_limit(0);

        assert_eq!(
            terminal.stable_semantic_command_exits(),
            vec![crate::StableSemanticCommandExit {
                row: before[0].row,
                exit_code: Some(7),
                aid: Some("42".to_owned()),
            }]
        );
    }

    #[test]
    fn terminal_stable_row_conversion_is_strict() {
        let terminal = Terminal::new(TerminalSize::new(4, 3));

        assert_eq!(terminal.history_index_to_stable_row(0), Some(0));
        assert_eq!(terminal.stable_row_to_history_index(0), Some(0));
        assert_eq!(terminal.history_index_to_stable_row(3), None);
        assert_eq!(terminal.stable_row_to_history_index(-1), None);
        assert_eq!(terminal.stable_row_to_history_index(3), None);
    }

    #[test]
    fn terminal_alternate_dimensions_have_no_main_scrollback() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"aa\r\nbb\r\ncc");
        assert!(!terminal.scrollback().is_empty());

        terminal.feed(b"\x1b[?1049h");

        assert_eq!(
            terminal.stable_dimensions(),
            TerminalStableDimensions {
                domain: TerminalScreenDomain::Alternate,
                viewport_rows: 2,
                scrollback_rows: 2,
                scrollback_top: 0,
                physical_top: 0,
            }
        );
        assert_eq!(terminal.retained_stable_range(), 0..2);
        assert_eq!(terminal.history_index_to_stable_row(0), Some(0));
        assert_eq!(terminal.history_index_to_stable_row(2), None);
        assert_eq!(terminal.stable_row_to_history_index(0), Some(0));
        assert_eq!(terminal.stable_row_to_history_index(2), None);
    }

    #[test]
    fn terminal_stable_bottom_and_viewport_range_are_checked() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");

        assert_eq!(terminal.stable_bottom_exclusive(), Some(4));
        assert_eq!(terminal.viewport_stable_range(None), 1..4);
        assert_eq!(terminal.viewport_stable_range(Some(0)), 0..3);

        let mut overflow = Terminal::new(TerminalSize::new(1, 1));
        overflow.main_stable_row_offset = StableRowIndex::MAX;
        assert_eq!(overflow.stable_bottom_exclusive(), None);
    }

    #[test]
    #[expect(
        clippy::reversed_empty_ranges,
        reason = "the test intentionally verifies rejection of a reversed range"
    )]
    fn terminal_stable_range_retention_is_strict() {
        let terminal = Terminal::new(TerminalSize::new(4, 3));

        assert!(terminal.is_stable_range_fully_retained(0..3));
        assert!(terminal.is_stable_range_fully_retained(1..2));
        assert!(!terminal.is_stable_range_fully_retained(-1..2));
        assert!(!terminal.is_stable_range_fully_retained(1..4));
        assert!(!terminal.is_stable_range_fully_retained(2..1));
        assert!(!terminal.is_stable_range_fully_retained(StableRowIndex::MIN..StableRowIndex::MAX));
    }

    #[test]
    fn terminal_sequence_starts_non_zero() {
        let terminal = Terminal::new(TerminalSize::new(4, 3));

        assert_ne!(terminal.current_seqno(), 0);
    }

    #[test]
    fn terminal_feed_advances_sequence_once_per_batch() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        let before = terminal.current_seqno();

        terminal.feed(b"");

        assert_eq!(terminal.current_seqno(), before.checked_add(1).unwrap());
        let before = terminal.current_seqno();

        terminal.feed(b"abc");

        assert_eq!(terminal.current_seqno(), before.checked_add(1).unwrap());
    }

    #[test]
    fn terminal_cursor_only_feed_advances_sequence_once() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        let before = terminal.current_seqno();

        terminal.feed(b"\x1b[2;3H");

        assert_eq!(terminal.current_seqno(), before.checked_add(1).unwrap());
    }

    #[test]
    fn terminal_same_size_resize_advances_sequence_once() {
        let size = TerminalSize::new(4, 3);
        let mut terminal = Terminal::new(size);
        let before = terminal.current_seqno();

        terminal.resize(size);

        assert_eq!(terminal.current_seqno(), before.checked_add(1).unwrap());
    }

    #[test]
    fn terminal_public_erase_advances_sequence_once() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        let before = terminal.current_seqno();

        terminal.erase_scrollback_and_viewport();

        assert_eq!(terminal.current_seqno(), before.checked_add(1).unwrap());
    }

    #[test]
    fn terminal_grid_row_sequences_initialize_and_resize() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        let initial = terminal.current_seqno();

        assert_eq!(terminal.grid.row_last_change_seqno(0), Some(initial));
        assert_eq!(terminal.grid.row_last_change_seqno(1), Some(initial));

        terminal.resize(TerminalSize::new(4, 3));

        assert_eq!(terminal.grid.row_last_change_seqno(0), Some(initial));
        assert_eq!(terminal.grid.row_last_change_seqno(1), Some(initial));
        assert_eq!(
            terminal.grid.row_last_change_seqno(2),
            Some(terminal.current_seqno())
        );
        assert_eq!(terminal.grid.row_last_change_seqno(3), None);
    }

    #[test]
    fn terminal_scrollback_preserves_captured_row_sequence() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"aa\r\nbb");
        let captured = terminal.current_seqno().checked_add(10).unwrap();
        assert!(terminal.grid.set_row_last_change_seqno(0, captured));

        terminal.feed(b"\r\ncc");

        assert!(!terminal.scrollback.is_empty());
        assert_eq!(
            terminal.scrollback.get(0).unwrap().last_change_seqno(),
            captured
        );

        let updated = captured.checked_add(1).unwrap();
        terminal
            .scrollback
            .iter_mut()
            .next()
            .unwrap()
            .set_last_change_seqno(updated);
        assert_eq!(
            terminal.scrollback.get(0).unwrap().last_change_seqno(),
            updated
        );
    }

    #[test]
    fn terminal_full_screen_scroll_preserves_stable_row_identity() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        terminal.feed(b"aa");
        terminal.feed(b"\r\nbb");
        terminal.feed(b"\r\ncc");
        let rows = (0..3)
            .map(|row| {
                let stable = terminal
                    .history_index_to_stable_row(row)
                    .expect("visible stable row");
                (
                    stable,
                    stable_row_text(&terminal, stable).unwrap(),
                    stable_row_seqno(&terminal, stable).unwrap(),
                )
            })
            .collect::<Vec<_>>();

        terminal.feed(b"\r\ndd");

        assert_eq!(terminal.stable_dimensions().physical_top, 1);
        for (stable, text, seqno) in rows {
            assert_eq!(
                stable_row_text(&terminal, stable).as_deref(),
                Some(text.as_str())
            );
            assert_eq!(stable_row_seqno(&terminal, stable), Some(seqno));
        }
    }

    #[test]
    fn terminal_full_screen_scroll_marks_only_new_bottom_row_changed() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        terminal.feed(b"aa\r\nbb\r\ncc");
        let before = terminal.current_seqno();

        terminal.feed(b"\r\ndd");

        let retained = terminal.retained_stable_range();
        assert_eq!(
            terminal.changed_stable_rows_since(retained, before),
            vec![3]
        );
    }

    #[test]
    fn terminal_top_anchored_short_su_records_history_and_dirties_suffix() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 4));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        let before = terminal.current_seqno();
        let stable_rows = (0..4)
            .map(|row| terminal.history_index_to_stable_row(row).unwrap())
            .collect::<Vec<_>>();
        let old_seqnos = stable_rows
            .iter()
            .map(|row| stable_row_seqno(&terminal, *row).unwrap())
            .collect::<Vec<_>>();

        terminal.feed(b"\x1b[1;3r\x1b[S");

        assert_eq!(terminal.scrollback.len(), 1);
        assert_eq!(
            stable_row_text(&terminal, stable_rows[0]).as_deref(),
            Some("aa  ")
        );
        assert_eq!(
            stable_row_text(&terminal, stable_rows[1]).as_deref(),
            Some("bb  ")
        );
        assert_eq!(
            stable_row_text(&terminal, stable_rows[2]).as_deref(),
            Some("cc  ")
        );
        assert_eq!(
            stable_row_seqno(&terminal, stable_rows[0]),
            Some(old_seqnos[0])
        );
        assert_eq!(
            stable_row_seqno(&terminal, stable_rows[1]),
            Some(old_seqnos[1])
        );
        assert_eq!(
            stable_row_seqno(&terminal, stable_rows[2]),
            Some(old_seqnos[2])
        );
        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            vec![3, 4]
        );
        assert_eq!(stable_row_text(&terminal, 4).as_deref(), Some("dd  "));
    }

    #[test]
    fn terminal_top_anchored_short_scroll_rebases_suffix_metadata_with_spare_capacity() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        install_suffix_metadata(&mut terminal, 2);

        terminal.feed(b"\x1b[1;2r\x1b[S");

        assert_eq!(terminal.scrollback.len(), 1);
        assert_eq!(stable_row_text(&terminal, 2).as_deref(), Some("        "));
        assert_eq!(stable_row_text(&terminal, 3).as_deref(), Some("cc      "));
        assert_suffix_metadata_row(&terminal, 3);
    }

    #[test]
    fn terminal_top_anchored_short_scroll_rebases_suffix_metadata_at_capacity() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.set_scrollback_limit(1);
        terminal.feed(b"old\r\naa\r\nbb\r\ncc\r\ndd");
        assert_eq!(terminal.scrollback.len(), 1);
        let suffix_row = terminal.scrollback.len() + 2;
        install_suffix_metadata(&mut terminal, suffix_row);
        let suffix_stable = terminal.history_index_to_stable_row(suffix_row).unwrap();
        let suffix_text = stable_row_text(&terminal, suffix_stable).unwrap();

        terminal.feed(b"\x1b[1;2r\x1b[S");

        assert_eq!(terminal.scrollback.len(), 1);
        assert_suffix_metadata_row(&terminal, suffix_row);
        let rebased_suffix_stable = terminal.history_index_to_stable_row(suffix_row).unwrap();
        assert_eq!(
            stable_row_text(&terminal, rebased_suffix_stable).as_deref(),
            Some(suffix_text.as_str())
        );
        assert_eq!(
            terminal.history_index_to_stable_row(terminal.inline_images[0].row),
            Some(rebased_suffix_stable)
        );
        assert!(rebased_suffix_stable > suffix_stable);
    }

    #[test]
    fn terminal_top_anchored_short_scroll_drops_cross_boundary_images_with_spare_capacity() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        install_cross_boundary_real_images(&mut terminal);

        terminal.feed(b"\x1b[1;2r\x1b[S");

        assert_eq!(terminal.scrollback.len(), 1);
        assert_eq!(stable_row_text(&terminal, 2).as_deref(), Some("        "));
        assert!(terminal.inline_images.is_empty());
        assert!(terminal.kitty_relative_parents.is_empty());
    }

    #[test]
    fn terminal_top_anchored_short_scroll_drops_cross_boundary_images_at_capacity() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.set_scrollback_limit(1);
        terminal.feed(b"old\r\naa\r\nbb\r\ncc\r\ndd");
        assert_eq!(terminal.scrollback.len(), 1);
        install_cross_boundary_real_images(&mut terminal);

        terminal.feed(b"\x1b[1;2r\x1b[S");

        assert_eq!(terminal.scrollback.len(), 1);
        let blank_row = terminal.history_index_to_stable_row(2).unwrap();
        assert_eq!(
            stable_row_text(&terminal, blank_row).as_deref(),
            Some("        ")
        );
        assert!(terminal.inline_images.is_empty());
        assert!(terminal.kitty_relative_parents.is_empty());
    }

    #[test]
    fn terminal_row_zero_delete_line_records_history() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 4));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        let before = terminal.current_seqno();
        let old_seqnos = (0..3)
            .map(|row| stable_row_seqno(&terminal, row).unwrap())
            .collect::<Vec<_>>();

        terminal.feed(b"\x1b[1;1H\x1b[1;3r\x1b[M");

        assert_eq!(terminal.scrollback.len(), 1);
        assert_eq!(stable_row_text(&terminal, 0).as_deref(), Some("aa  "));
        assert_eq!(stable_row_text(&terminal, 1).as_deref(), Some("bb  "));
        assert_eq!(stable_row_text(&terminal, 2).as_deref(), Some("cc  "));
        assert_eq!(stable_row_seqno(&terminal, 0), Some(old_seqnos[0]));
        assert_eq!(stable_row_seqno(&terminal, 1), Some(old_seqnos[1]));
        assert_eq!(stable_row_seqno(&terminal, 2), Some(old_seqnos[2]));
        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            vec![3, 4]
        );
    }

    #[test]
    fn terminal_top_zero_narrow_margin_scroll_does_not_record_history() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        terminal.feed(b"aa\r\nbb\r\ncc");
        let physical_top = terminal.stable_dimensions().physical_top;

        terminal.feed(b"\x1b[?69h\x1b[2;3s\x1b[1;3r\x1b[S");

        assert!(terminal.scrollback.is_empty());
        assert_eq!(terminal.stable_dimensions().physical_top, physical_top);
    }

    #[test]
    fn terminal_horizontal_margin_su_default_count_moves_only_bounded_cells() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.feed(b"abcdefgh\r\nijklmnop\r\nqrstuvwx\r\nyz012345");
        let physical_top = terminal.stable_dimensions().physical_top;

        terminal.feed(b"\x1b[?69h\x1b[3;6s\x1b[2;4r\x1b[S");

        let rows = (0..4)
            .map(|row| {
                terminal
                    .cells_for_history_row(row)
                    .unwrap()
                    .iter()
                    .map(Cell::primary_char)
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        assert_eq!(rows, ["abcdefgh", "ijstuvop", "qr0123wx", "yz    45"]);
        assert!(terminal.scrollback.is_empty());
        assert_eq!(terminal.stable_dimensions().physical_top, physical_top);
    }

    #[test]
    fn terminal_horizontal_margin_cell_attachment_vertical_su_moves_only_bounded_cells() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 4));
        terminal.push_inline_image(attachment_test_image(2, 2, 2, 2, "inside"));
        terminal.push_inline_image(attachment_test_image(2, 1, 2, 2, "crossing"));
        terminal.push_inline_image(attachment_test_image(2, 0, 1, 2, "outside"));
        terminal.push_inline_image(attachment_test_image(1, 2, 1, 1, "blanked"));

        terminal.feed(b"\x1b[?69h\x1b[3;4s\x1b[2;4r\x1b[S");

        assert_eq!(
            attachment_locations(&terminal),
            vec![
                (1, 0, 0, 1, 2),
                (1, 0, 1, 1, 3),
                (1, 1, 0, 2, 2),
                (1, 1, 1, 2, 3),
                (2, 0, 0, 2, 1),
                (2, 0, 1, 1, 2),
                (2, 1, 0, 3, 1),
                (2, 1, 1, 2, 2),
                (3, 0, 0, 2, 0),
                (3, 1, 0, 3, 0),
            ]
        );
        assert_eq!(terminal.inline_images().len(), 4);
    }

    #[test]
    fn terminal_horizontal_margin_cell_attachment_vertical_sd_deletes_blanked_cells() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 4));
        terminal.push_inline_image(attachment_test_image(1, 2, 2, 2, "inside"));
        terminal.push_inline_image(attachment_test_image(1, 1, 2, 2, "crossing"));
        terminal.push_inline_image(attachment_test_image(1, 0, 1, 2, "outside"));
        terminal.push_inline_image(attachment_test_image(3, 2, 1, 1, "blanked"));

        terminal.feed(b"\x1b[?69h\x1b[3;4s\x1b[2;4r\x1b[T");

        assert_eq!(
            attachment_locations(&terminal),
            vec![
                (1, 0, 0, 2, 2),
                (1, 0, 1, 2, 3),
                (1, 1, 0, 3, 2),
                (1, 1, 1, 3, 3),
                (2, 0, 0, 1, 1),
                (2, 0, 1, 2, 2),
                (2, 1, 0, 2, 1),
                (2, 1, 1, 3, 2),
                (3, 0, 0, 1, 0),
                (3, 1, 0, 2, 0),
            ]
        );
        assert_eq!(terminal.inline_images().len(), 4);
    }

    #[test]
    fn terminal_horizontal_margin_cell_attachment_vertical_line_editing_uses_row_transform() {
        let mut insert = Terminal::new(TerminalSize::new(6, 4));
        insert.push_inline_image(attachment_test_image(1, 2, 1, 2, "insert"));
        insert.push_inline_image(attachment_test_image(3, 2, 1, 1, "blanked"));
        insert.feed(b"\x1b[?69h\x1b[3;4s\x1b[2;4r\x1b[2;3H\x1b[L");
        assert_eq!(
            attachment_locations(&insert),
            vec![(1, 0, 0, 2, 2), (1, 1, 0, 3, 2)]
        );

        let mut delete = Terminal::new(TerminalSize::new(6, 4));
        delete.push_inline_image(attachment_test_image(2, 2, 1, 2, "delete"));
        delete.push_inline_image(attachment_test_image(1, 2, 1, 1, "blanked"));
        delete.feed(b"\x1b[?69h\x1b[3;4s\x1b[2;4r\x1b[2;3H\x1b[M");
        assert_eq!(
            attachment_locations(&delete),
            vec![(1, 0, 0, 1, 2), (1, 1, 0, 2, 2)]
        );
    }

    #[test]
    fn terminal_horizontal_margin_ich_moves_attachment_cells_and_blanks_the_clipped_suffix() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        // The first image crosses the left LR edge. ICH must leave that
        // exterior cell in place while moving its first in-margin source cell
        // and discarding the source cell that falls into the clipped suffix.
        terminal.push_inline_image(attachment_test_image(0, 1, 3, 1, "crossing"));
        terminal.push_inline_image(attachment_test_image(0, 4, 1, 1, "blanked"));

        terminal.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;2r\x1b[1;3H\x1b[2@");

        assert_eq!(
            attachment_locations(&terminal),
            vec![(1, 0, 0, 0, 1), (1, 0, 1, 0, 4)]
        );
        assert_eq!(terminal.inline_images().len(), 2);
    }

    #[test]
    fn terminal_horizontal_margin_dch_moves_attachment_cells_outside_tb_and_blanks_sources() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        // DCH is LR-gated, not TB-gated. Its source cells at and after the
        // cursor map left, while the LR-exterior cell remains unchanged.
        terminal.push_inline_image(attachment_test_image(0, 1, 3, 1, "crossing"));
        terminal.push_inline_image(attachment_test_image(0, 4, 1, 1, "moved"));

        terminal.feed(b"\x1b[?69h\x1b[3;5s\x1b[2;2r\x1b[1;3H\x1b[2P");

        assert_eq!(
            attachment_locations(&terminal),
            vec![(1, 0, 0, 0, 1), (2, 0, 0, 0, 2)]
        );
        assert_eq!(terminal.inline_images().len(), 2);
    }

    #[test]
    fn terminal_horizontal_margin_character_attachment_edits_clip_counts_and_obey_their_gates() {
        let mut ich_clipped = Terminal::new(TerminalSize::new(6, 3));
        ich_clipped.push_inline_image(attachment_test_image(0, 1, 4, 1, "crossing"));
        ich_clipped.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;3r\x1b[1;3H\x1b[99@");
        assert_eq!(attachment_locations(&ich_clipped), vec![(1, 0, 0, 0, 1)]);

        let mut dch_clipped = Terminal::new(TerminalSize::new(6, 3));
        dch_clipped.push_inline_image(attachment_test_image(0, 1, 4, 1, "crossing"));
        dch_clipped.feed(b"\x1b[?69h\x1b[3;5s\x1b[2;3r\x1b[1;3H\x1b[99P");
        assert_eq!(attachment_locations(&dch_clipped), vec![(1, 0, 0, 0, 1)]);

        let mut ich_outside_tb = Terminal::new(TerminalSize::new(6, 3));
        ich_outside_tb.push_inline_image(attachment_test_image(0, 2, 1, 1, "inside"));
        ich_outside_tb.feed(b"\x1b[?69h\x1b[3;5s\x1b[2;3r\x1b[1;3H\x1b[@");
        assert_eq!(attachment_locations(&ich_outside_tb), vec![(1, 0, 0, 0, 2)]);

        let mut dch_outside_lr = Terminal::new(TerminalSize::new(6, 3));
        dch_outside_lr.push_inline_image(attachment_test_image(0, 2, 1, 1, "inside"));
        dch_outside_lr.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;3r\x1b[1;1H\x1b[P");
        assert_eq!(attachment_locations(&dch_outside_lr), vec![(1, 0, 0, 0, 2)]);
    }

    #[test]
    fn terminal_horizontal_margin_dch_moves_high_byte_kitty_placeholder_state_with_cells() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 3));
        terminal.feed(b"\x1b_Ga=t,i=33554462,f=24,s=1,v=1;/wAA\x1b\\");
        terminal.take_kitty_graphics_responses();

        let mut image = attachment_test_image(0, 2, 2, 1, "kitty");
        image.kitty_image_id = Some(HIGH_BYTE_KITTY_IMAGE_ID);
        image.kitty_placement_id = Some(4);
        terminal.push_inline_image(image);
        terminal.kitty_virtual_placements.insert(
            (HIGH_BYTE_KITTY_IMAGE_ID, 4),
            KittyVirtualPlacement {
                image_id: HIGH_BYTE_KITTY_IMAGE_ID,
                placement_id: Some(4),
                z_index: None,
                display_columns: Some(2),
                display_rows: Some(1),
                source_rect: KittySourceRect::default(),
                target_x: None,
                target_y: None,
            },
        );
        let placeholder = LastKittyPlaceholder {
            row: 0,
            column: 3,
            foreground: Color::Rgb(0, 0, 30),
            underline_color: Color::Rgb(0, 0, 4),
            image_id_high_byte: 2,
            placeholder_row: 0,
            placeholder_column: 0,
        };
        terminal.kitty_placeholder_cells.insert((0, 3), placeholder);
        terminal.last_kitty_placeholder = Some(placeholder);
        terminal.pending_kitty_placeholder = Some(PendingKittyPlaceholder {
            row: 0,
            column: 3,
            foreground: placeholder.foreground,
            underline_color: placeholder.underline_color,
            image_id: Some(HIGH_BYTE_KITTY_IMAGE_ID),
            placement_id: Some(4),
            diacritics: Vec::new(),
            rendered_row: Some(0),
            rendered_column: Some(3),
            rendered_image_id: Some(HIGH_BYTE_KITTY_IMAGE_ID),
            rendered_placement_id: Some(4),
        });

        terminal.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;3r\x1b[1;3H\x1b[P");

        assert_eq!(terminal.inline_images.len(), 1);
        assert_eq!(attachment_locations(&terminal), vec![(1, 0, 1, 0, 2)]);
        assert_eq!(
            terminal
                .kitty_placeholder_cells
                .get(&(0, 2))
                .map(|placeholder| (
                    placeholder.row,
                    placeholder.column,
                    placeholder.image_id_high_byte,
                )),
            Some((0, 2, 2))
        );
        assert_eq!(
            terminal
                .last_kitty_placeholder
                .map(|placeholder| (placeholder.row, placeholder.column)),
            Some((0, 2))
        );
        // CSI starts a new escape sequence, so the parser commits a pending
        // placeholder before applying DCH. Its committed cache above is what
        // must move with the text cells.
        assert!(terminal.pending_kitty_placeholder.is_none());
        assert!(
            terminal
                .kitty_images
                .contains_key(&HIGH_BYTE_KITTY_IMAGE_ID)
        );
    }

    #[test]
    fn terminal_character_edit_transform_moves_pending_kitty_placeholder_and_render_origin() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 3));
        terminal.pending_kitty_placeholder = Some(PendingKittyPlaceholder {
            row: 1,
            column: 3,
            foreground: Color::Rgb(0, 0, 30),
            underline_color: Color::Rgb(0, 0, 4),
            image_id: Some(HIGH_BYTE_KITTY_IMAGE_ID),
            placement_id: Some(4),
            diacritics: Vec::new(),
            rendered_row: Some(1),
            rendered_column: Some(3),
            rendered_image_id: Some(HIGH_BYTE_KITTY_IMAGE_ID),
            rendered_placement_id: Some(4),
        });

        terminal.transform_kitty_placeholder_state_for_character_edit(
            CellTransform::DeleteCharacters {
                row: 1,
                column: 2,
                count: 1,
                right: 4,
            },
        );

        assert_eq!(
            terminal
                .pending_kitty_placeholder
                .as_ref()
                .map(|placeholder| (
                    placeholder.row,
                    placeholder.column,
                    placeholder.rendered_row,
                    placeholder.rendered_column,
                    placeholder.rendered_image_id,
                    placeholder.rendered_placement_id,
                )),
            Some((
                1,
                2,
                Some(1),
                Some(2),
                Some(HIGH_BYTE_KITTY_IMAGE_ID),
                Some(4),
            ))
        );
    }

    #[test]
    fn terminal_character_edit_transform_drops_blanked_high_byte_kitty_placeholder_state() {
        for (transform, source_column, exterior_column) in [
            (
                CellTransform::DeleteCharacters {
                    row: 1,
                    column: 2,
                    count: 1,
                    right: 4,
                },
                2,
                5,
            ),
            (
                CellTransform::InsertCharacters {
                    row: 1,
                    column: 2,
                    count: 1,
                    right: 4,
                },
                4,
                5,
            ),
        ] {
            let mut terminal = Terminal::new(TerminalSize::new(6, 3));
            let source = LastKittyPlaceholder {
                row: 1,
                column: source_column,
                foreground: Color::Rgb(0, 0, 30),
                underline_color: Color::Rgb(0, 0, 4),
                image_id_high_byte: 2,
                placeholder_row: 0,
                placeholder_column: 0,
            };
            let exterior = LastKittyPlaceholder {
                column: exterior_column,
                ..source
            };
            terminal
                .kitty_placeholder_cells
                .insert((1, source_column), source);
            terminal
                .kitty_placeholder_cells
                .insert((1, exterior_column), exterior);
            terminal.last_kitty_placeholder = Some(source);
            terminal.pending_kitty_placeholder = Some(PendingKittyPlaceholder {
                row: 1,
                column: source_column,
                foreground: source.foreground,
                underline_color: source.underline_color,
                image_id: Some(HIGH_BYTE_KITTY_IMAGE_ID),
                placement_id: Some(4),
                diacritics: Vec::new(),
                rendered_row: Some(1),
                rendered_column: Some(source_column),
                rendered_image_id: Some(HIGH_BYTE_KITTY_IMAGE_ID),
                rendered_placement_id: Some(4),
            });

            terminal.transform_kitty_placeholder_state_for_character_edit(transform);

            assert!(
                !terminal
                    .kitty_placeholder_cells
                    .contains_key(&(1, source_column)),
                "transform={transform:?}"
            );
            assert_eq!(
                terminal
                    .kitty_placeholder_cells
                    .get(&(1, exterior_column))
                    .map(|placeholder| (
                        placeholder.row,
                        placeholder.column,
                        placeholder.image_id_high_byte,
                    )),
                Some((1, exterior_column, 2)),
                "transform={transform:?}"
            );
            assert!(
                terminal.last_kitty_placeholder.is_none(),
                "transform={transform:?}"
            );
            assert!(
                terminal.pending_kitty_placeholder.is_none(),
                "transform={transform:?}"
            );
        }
    }

    #[test]
    fn terminal_dch_commits_then_drops_pending_high_byte_placeholder_at_cursor() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 3));
        terminal.kitty_images.insert(
            HIGH_BYTE_KITTY_IMAGE_ID,
            StoredKittyImage {
                image_format: InlineImageFormat::Rgb,
                pixel_width: Some(1),
                pixel_height: Some(1),
                display_columns: Some(2),
                display_rows: Some(1),
                data: vec![0xff, 0, 0],
            },
        );
        terminal.kitty_virtual_placements.insert(
            (HIGH_BYTE_KITTY_IMAGE_ID, 4),
            KittyVirtualPlacement {
                image_id: HIGH_BYTE_KITTY_IMAGE_ID,
                placement_id: Some(4),
                z_index: None,
                display_columns: Some(2),
                display_rows: Some(1),
                source_rect: KittySourceRect::default(),
                target_x: None,
                target_y: None,
            },
        );
        let left = LastKittyPlaceholder {
            row: 0,
            column: 1,
            foreground: Color::Rgb(0, 0, 30),
            underline_color: Color::Rgb(0, 0, 4),
            image_id_high_byte: 2,
            placeholder_row: 0,
            placeholder_column: 0,
        };
        terminal.kitty_placeholder_cells.insert((0, 1), left);
        terminal.last_kitty_placeholder = Some(left);
        terminal.pending_kitty_placeholder = Some(PendingKittyPlaceholder {
            row: 0,
            column: 2,
            foreground: left.foreground,
            underline_color: left.underline_color,
            image_id: Some(HIGH_BYTE_KITTY_IMAGE_ID),
            placement_id: Some(4),
            diacritics: Vec::new(),
            rendered_row: None,
            rendered_column: None,
            rendered_image_id: None,
            rendered_placement_id: None,
        });
        terminal.modes.left_right_margin_mode = true;
        terminal.left_margin = 2;
        terminal.right_margin = 4;
        terminal.cursor_row = 0;
        terminal.cursor_column = 2;

        terminal.feed(b"\x1b[P");

        assert!(terminal.pending_kitty_placeholder.is_none());
        assert!(terminal.last_kitty_placeholder.is_none());
        assert!(
            !terminal.kitty_placeholder_cells.contains_key(&(0, 2)),
            "the placeholder committed at the DCH cursor is blanked"
        );
        assert_eq!(
            terminal
                .kitty_placeholder_cells
                .get(&(0, 1))
                .map(|placeholder| (
                    placeholder.row,
                    placeholder.column,
                    placeholder.image_id_high_byte,
                )),
            Some((0, 1, 2)),
            "the exterior left cell is outside the LR edit"
        );
        assert!(
            terminal
                .kitty_images
                .contains_key(&HIGH_BYTE_KITTY_IMAGE_ID)
        );
    }

    #[test]
    fn terminal_horizontal_margin_ich_moves_kitty_cache_without_retiring_relative_attachments() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 3));
        let mut parent = attachment_test_image(0, 2, 2, 1, "parent");
        parent.kitty_image_id = Some(30);
        parent.kitty_placement_id = Some(4);
        terminal.push_inline_image(parent);
        let mut child = attachment_test_image(0, 2, 1, 1, "child");
        child.kitty_image_id = Some(7);
        child.kitty_placement_id = Some(2);
        terminal.push_inline_image(child);
        terminal.kitty_relative_parents.insert((7, 2), (30, 4));
        terminal.kitty_virtual_placements.insert(
            (30, 4),
            KittyVirtualPlacement {
                image_id: 30,
                placement_id: Some(4),
                z_index: None,
                display_columns: Some(2),
                display_rows: Some(1),
                source_rect: KittySourceRect::default(),
                target_x: None,
                target_y: None,
            },
        );
        let placeholder = LastKittyPlaceholder {
            row: 0,
            column: 2,
            foreground: Color::Rgb(0, 0, 30),
            underline_color: Color::Rgb(0, 0, 4),
            image_id_high_byte: 0,
            placeholder_row: 0,
            placeholder_column: 0,
        };
        terminal.kitty_placeholder_cells.insert((0, 2), placeholder);
        terminal.last_kitty_placeholder = Some(placeholder);

        terminal.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;3r\x1b[1;3H\x1b[@");

        assert_eq!(terminal.inline_images.len(), 2);
        assert_eq!(
            attachment_locations(&terminal),
            vec![(1, 0, 0, 0, 3), (1, 0, 1, 0, 4), (2, 0, 0, 0, 3)]
        );
        assert_eq!(terminal.kitty_relative_parents.get(&(7, 2)), Some(&(30, 4)));
        assert_eq!(
            terminal
                .kitty_placeholder_cells
                .get(&(0, 3))
                .map(|placeholder| (placeholder.row, placeholder.column)),
            Some((0, 3))
        );
        assert_eq!(
            terminal
                .last_kitty_placeholder
                .map(|placeholder| (placeholder.row, placeholder.column)),
            Some((0, 3))
        );
    }

    #[test]
    fn terminal_horizontal_margin_irm_keeps_legacy_graphics_retirement_path() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 3));
        terminal.push_inline_image(attachment_test_image(0, 2, 1, 1, "insert-mode"));

        terminal.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;3r\x1b[1;3H\x1b[4hX\x1b[4l");

        assert!(terminal.inline_images.is_empty());
        assert!(terminal.inline_image_attachments.is_empty());
    }

    #[test]
    fn terminal_horizontal_margin_character_edit_cache_invalidation_without_offset_keeps_damage_local()
     {
        for command in [b"\x1b[@".as_slice(), b"\x1b[P"] {
            let mut terminal = Terminal::new(TerminalSize::new(6, 3));
            let placeholder = LastKittyPlaceholder {
                row: 0,
                column: 2,
                foreground: Color::Default,
                underline_color: Color::Default,
                image_id_high_byte: 0,
                placeholder_row: 0,
                placeholder_column: 0,
            };
            terminal.kitty_placeholder_cells.insert((0, 2), placeholder);
            terminal.last_kitty_placeholder = Some(placeholder);
            terminal.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;3r\x1b[1;3H");
            terminal.take_damage();

            terminal.feed(command);

            assert_eq!(
                terminal.take_damage(),
                vec![DamageRegion::new(2, 0, 3, 1)],
                "command={command:?}"
            );
        }
    }

    #[test]
    fn terminal_horizontal_margin_character_edit_with_offset_attachment_invalidates_viewport() {
        for command in [b"\x1b[@".as_slice(), b"\x1b[P"] {
            let mut terminal = Terminal::new(TerminalSize::new(6, 3));
            let mut image = attachment_test_image(0, 2, 1, 1, "offset");
            image.target_x = Some(1);
            terminal.push_inline_image(image);
            terminal.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;3r\x1b[1;3H");
            terminal.take_damage();

            terminal.feed(command);

            assert_eq!(
                terminal.take_damage(),
                vec![DamageRegion::new(2, 0, 3, 1), DamageRegion::new(0, 0, 6, 3),],
                "command={command:?}"
            );
        }
    }

    #[test]
    fn terminal_horizontal_margin_ich_uses_attachment_origin_for_virtual_parent_with_offset_and_history()
     {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        terminal.feed(b"one\r\ntwo\r\nthree");
        let row = terminal.scrollback.len();
        let mut parent = attachment_test_image(row, 2, 2, 1, "parent");
        parent.kitty_image_id = Some(30);
        parent.kitty_placement_id = Some(4);
        parent.target_x = Some(7);
        terminal.push_inline_image(parent);
        terminal.kitty_virtual_placements.insert(
            (30, 4),
            KittyVirtualPlacement {
                image_id: 30,
                placement_id: Some(4),
                z_index: None,
                display_columns: Some(2),
                display_rows: Some(1),
                source_rect: KittySourceRect::default(),
                target_x: Some(7),
                target_y: None,
            },
        );
        let placeholder = LastKittyPlaceholder {
            row,
            column: 2,
            foreground: Color::Rgb(0, 0, 30),
            underline_color: Color::Rgb(0, 0, 4),
            image_id_high_byte: 0,
            placeholder_row: 0,
            placeholder_column: 0,
        };
        terminal
            .kitty_placeholder_cells
            .insert((row, 2), placeholder);

        terminal.feed(b"\x1b[?69h\x1b[3;5s\x1b[1;2r\x1b[1;3H\x1b[@");

        assert_eq!(terminal.kitty_relative_parent_origin(30, 4), Some((row, 3)));
    }

    #[test]
    fn terminal_horizontal_margin_ich_prefers_live_virtual_parent_attachments_over_right_cache() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"\x1b_Ga=T,U=1,q=1,i=30,p=4,f=24,s=1,v=1,c=2,r=1;/wAA\x1b\\");
        terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;AP8A\x1b\\");
        terminal.take_kitty_graphics_responses();
        terminal.feed(b"\x1b[1;3H\x1b[38;5;30m\x1b[58;5;4m");
        terminal.feed("\u{10eeee}\u{0305}\u{0305}".as_bytes());
        let residual_right_cache = *terminal
            .kitty_placeholder_cells
            .get(&(0, 2))
            .expect("virtual parent cache");
        // This models a source-cell cache that lies outside the LR edit but
        // still names the same virtual placement. The physical parent spans
        // cells 2..=3 and is transformed by ICH.
        terminal
            .kitty_placeholder_cells
            .insert((0, 4), residual_right_cache);

        terminal.feed(b"\x1b[?69h\x1b[3;4s\x1b[1;2r\x1b[1;3H\x1b[@");
        assert!(terminal.kitty_placeholder_cells.contains_key(&(0, 3)));
        assert!(
            terminal.kitty_placeholder_cells.contains_key(&(0, 4)),
            "the cache exterior to the LR edit remains in its terminal cell"
        );
        terminal.feed(b"\x1b_Ga=p,i=7,p=2,P=30,Q=4,H=0,V=0,c=1,r=1\x1b\\");

        assert_eq!(
            terminal.take_kitty_graphics_responses(),
            vec![b"\x1b_Gi=7,p=2;OK\x1b\\".to_vec()]
        );
        assert!(terminal.inline_images.iter().any(|image| {
            image.kitty_image_id == Some(7)
                && image.kitty_placement_id == Some(2)
                && image.row == 0
                && image.column == 3
        }));
    }

    #[test]
    fn terminal_same_size_resize_retains_character_edit_origin_for_virtual_parent() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"\x1b_Ga=T,U=1,q=1,i=30,p=4,f=24,s=1,v=1,c=2,r=1;/wAA\x1b\\");
        terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;AP8A\x1b\\");
        terminal.take_kitty_graphics_responses();
        terminal.feed(b"\x1b[1;3H\x1b[38;5;30m\x1b[58;5;4m");
        terminal.feed("\u{10eeee}\u{0305}\u{0305}".as_bytes());
        let residual_right_cache = *terminal
            .kitty_placeholder_cells
            .get(&(0, 2))
            .expect("virtual parent cache");
        terminal
            .kitty_placeholder_cells
            .insert((0, 4), residual_right_cache);

        terminal.feed(b"\x1b[?69h\x1b[3;4s\x1b[1;2r\x1b[1;3H\x1b[@");
        terminal.resize(TerminalSize::new(8, 2));
        terminal.feed(b"\x1b_Ga=p,i=7,p=2,P=30,Q=4,H=0,V=0,c=1,r=1\x1b\\");

        assert_eq!(
            terminal.take_kitty_graphics_responses(),
            vec![b"\x1b_Gi=7,p=2;OK\x1b\\".to_vec()]
        );
        assert!(terminal.inline_images.iter().any(|image| {
            image.kitty_image_id == Some(7)
                && image.kitty_placement_id == Some(2)
                && image.row == 0
                && image.column == 3
        }));
    }

    #[test]
    fn terminal_alt_virtual_delete_discards_dormant_character_edit_marker() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"\x1b_Ga=T,U=1,q=1,i=30,p=4,f=24,s=1,v=1,c=2,r=1;/wAA\x1b\\");
        terminal.feed(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;AP8A\x1b\\");
        terminal.take_kitty_graphics_responses();
        terminal.feed(b"\x1b[1;3H\x1b[38;5;30m\x1b[58;5;4m");
        terminal.feed("\u{10eeee}\u{0305}\u{0305}".as_bytes());
        let residual_right_cache = *terminal
            .kitty_placeholder_cells
            .get(&(0, 2))
            .expect("virtual parent cache");
        terminal
            .kitty_placeholder_cells
            .insert((0, 4), residual_right_cache);
        terminal.feed(b"\x1b[?69h\x1b[3;4s\x1b[1;2r\x1b[1;3H\x1b[@");

        terminal.feed(b"\x1b[?1049h");
        terminal.feed(b"\x1b_Ga=d,d=i,i=30,p=4\x1b\\");
        terminal.feed(b"\x1b_Ga=p,U=1,i=30,p=4,c=2,r=1\x1b\\");
        terminal.feed(b"\x1b[?1049l");
        terminal.feed(b"\x1b_Ga=p,i=7,p=2,P=30,Q=4,H=0,V=0,c=1,r=1\x1b\\");

        assert_eq!(
            terminal.take_kitty_graphics_responses(),
            vec![
                b"\x1b_Gi=30,p=4;OK\x1b\\".to_vec(),
                b"\x1b_Gi=7,p=2;OK\x1b\\".to_vec(),
            ]
        );
        assert!(terminal.inline_images.iter().any(|image| {
            image.kitty_image_id == Some(7)
                && image.kitty_placement_id == Some(2)
                && image.row == 0
                && image.column == 3
        }));
    }

    #[test]
    fn terminal_discards_character_edit_marker_when_virtual_parent_attachments_are_invalidated() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        let mut parent = attachment_test_image(0, 2, 1, 1, "parent");
        parent.kitty_image_id = Some(30);
        parent.kitty_placement_id = Some(4);
        terminal.push_inline_image(parent);
        terminal.kitty_virtual_placements.insert(
            (30, 4),
            KittyVirtualPlacement {
                image_id: 30,
                placement_id: Some(4),
                z_index: None,
                display_columns: Some(1),
                display_rows: Some(1),
                source_rect: KittySourceRect::default(),
                target_x: None,
                target_y: None,
            },
        );
        terminal.kitty_character_edited_placements.insert((30, 4));

        terminal.inline_image_attachments.clear();
        terminal.retain_kitty_character_edited_placements();

        assert!(terminal.kitty_character_edited_placements.is_empty());
    }

    #[test]
    fn terminal_discards_character_edit_marker_when_virtual_parent_is_deleted() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 2));
        let mut parent = attachment_test_image(0, 2, 1, 1, "parent");
        parent.kitty_image_id = Some(30);
        parent.kitty_placement_id = Some(4);
        terminal.push_inline_image(parent);
        terminal.kitty_virtual_placements.insert(
            (30, 4),
            KittyVirtualPlacement {
                image_id: 30,
                placement_id: Some(4),
                z_index: None,
                display_columns: Some(1),
                display_rows: Some(1),
                source_rect: KittySourceRect::default(),
                target_x: None,
                target_y: None,
            },
        );
        terminal.kitty_character_edited_placements.insert((30, 4));

        terminal.delete_kitty_placements_by_image_id(30, Some(4), false);

        assert!(terminal.kitty_character_edited_placements.is_empty());
    }

    #[test]
    fn terminal_discards_character_edit_markers_when_resized() {
        for size in [TerminalSize::new(8, 2), TerminalSize::new(6, 3)] {
            let mut terminal = Terminal::new(TerminalSize::new(6, 2));
            terminal.kitty_character_edited_placements.insert((30, 4));

            terminal.resize(size);

            assert!(
                terminal.kitty_character_edited_placements.is_empty(),
                "size={size:?}"
            );
        }
    }

    #[test]
    fn terminal_horizontal_margin_cell_attachment_vertical_line_controls_follow_bounded_scrolls() {
        for control in [b"\n".as_slice(), b"\x1bD", b"\x1bE"] {
            let mut terminal = Terminal::new(TerminalSize::new(6, 4));
            terminal.push_inline_image(attachment_test_image(2, 2, 1, 2, "up"));
            terminal.push_inline_image(attachment_test_image(1, 2, 1, 1, "blanked"));
            terminal.feed(b"\x1b[?69h\x1b[3;4s\x1b[2;4r\x1b[4;3H");
            terminal.feed(control);
            assert_eq!(
                attachment_locations(&terminal),
                vec![(1, 0, 0, 1, 2), (1, 1, 0, 2, 2)],
                "control={control:?}"
            );
        }

        let mut terminal = Terminal::new(TerminalSize::new(6, 4));
        terminal.push_inline_image(attachment_test_image(1, 2, 1, 2, "down"));
        terminal.push_inline_image(attachment_test_image(3, 2, 1, 1, "blanked"));
        terminal.feed(b"\x1b[?69h\x1b[3;4s\x1b[2;4r\x1b[2;3H\x1bM");
        assert_eq!(
            attachment_locations(&terminal),
            vec![(1, 0, 0, 2, 2), (1, 1, 0, 3, 2)]
        );
    }

    #[test]
    fn terminal_horizontal_margin_cell_attachment_vertical_keeps_kitty_storage() {
        let mut terminal = Terminal::new(TerminalSize::new(6, 4));
        terminal.feed(b"\x1b_Ga=t,i=30,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
        terminal.take_kitty_graphics_responses();
        terminal.feed(b"\x1b[3;3H\x1b_Ga=p,i=30,p=4,c=2,r=2\x1b\\");
        terminal.take_kitty_graphics_responses();
        assert_eq!(terminal.inline_image_attachments().len(), 4);

        terminal.feed(b"\x1b[?69h\x1b[3;4s\x1b[2;4r\x1b[2S");

        assert_eq!(terminal.inline_image_attachments().len(), 2);
        terminal.feed(b"\x1b_Ga=p,i=30,p=5,c=1,r=1\x1b\\");
        assert_eq!(
            terminal.take_kitty_graphics_responses(),
            vec![b"\x1b_Gi=30,p=5;OK\x1b\\".to_vec()]
        );
    }

    #[test]
    fn terminal_horizontal_margin_su_retires_only_intersecting_graphics_metadata() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.feed(b"abcdefgh\r\nijklmnop\r\nqrstuvwx\r\nyz012345");
        let mut outside = metadata_test_image(1, "outside");
        outside.column = 0;
        outside.width = Some("1px".to_owned());
        let mut intersecting = metadata_test_image(1, "intersecting");
        intersecting.column = 3;
        intersecting.width = Some("1px".to_owned());
        terminal.inline_images.extend([outside, intersecting]);
        let outside_placeholder = metadata_test_placeholder(1);
        let mut intersecting_placeholder = metadata_test_placeholder(1);
        intersecting_placeholder.column = 3;
        terminal
            .kitty_placeholder_cells
            .insert((1, 0), outside_placeholder);
        terminal
            .kitty_placeholder_cells
            .insert((1, 3), intersecting_placeholder);
        terminal.last_kitty_placeholder = Some(intersecting_placeholder);

        terminal.feed(b"\x1b[?69h\x1b[3;6s\x1b[2;4r\x1b[S");

        assert_eq!(terminal.inline_images.len(), 1);
        assert_eq!(terminal.inline_images[0].name.as_deref(), Some("outside"));
        assert_eq!(terminal.inline_images[0].row, 1);
        assert_eq!(terminal.inline_images[0].column, 0);
        assert!(terminal.kitty_placeholder_cells.contains_key(&(1, 0)));
        assert!(!terminal.kitty_placeholder_cells.contains_key(&(1, 3)));
        assert!(terminal.last_kitty_placeholder.is_none());
    }

    #[test]
    fn terminal_horizontal_margin_su_invalidates_viewport_after_retiring_graphics() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.feed(b"abcdefgh\r\nijklmnop\r\nqrstuvwx\r\nyz012345");
        let mut parent = metadata_test_image(1, "parent");
        parent.column = 3;
        parent.width = Some("1px".to_owned());
        parent.kitty_image_id = Some(30);
        parent.kitty_placement_id = Some(4);
        let mut child = metadata_test_image(2, "child");
        child.column = 0;
        child.width = Some("1px".to_owned());
        child.kitty_image_id = Some(7);
        child.kitty_placement_id = Some(2);
        terminal.inline_images.extend([parent, child]);
        terminal.kitty_relative_parents.insert((7, 2), (30, 4));
        terminal.take_damage();
        terminal.feed(b"\x1b[?69h\x1b[3;6s\x1b[2;4r");
        terminal.take_damage();

        terminal.feed(b"\x1b[S");

        assert!(terminal.inline_images.is_empty());
        assert!(terminal.kitty_relative_parents.is_empty());
        assert_eq!(
            terminal.take_damage(),
            vec![DamageRegion::new(2, 1, 4, 3), DamageRegion::new(0, 0, 8, 4),]
        );
    }

    #[test]
    fn terminal_horizontal_margin_su_retires_stale_external_kitty_placeholder_caches() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.feed(b"abcdefgh\r\nijklmnop\r\nqrstuvwx\r\nyz012345");
        let mut parent = metadata_test_image(1, "parent");
        parent.column = 3;
        parent.width = Some("1px".to_owned());
        parent.kitty_image_id = Some(30);
        parent.kitty_placement_id = Some(4);
        let mut orphaned_child = metadata_test_image(2, "orphaned-child");
        orphaned_child.column = 0;
        orphaned_child.width = Some("1px".to_owned());
        orphaned_child.kitty_image_id = Some(7);
        orphaned_child.kitty_placement_id = Some(2);
        let mut unrelated = metadata_test_image(0, "unrelated");
        unrelated.column = 0;
        unrelated.width = Some("1px".to_owned());
        unrelated.kitty_image_id = Some(99);
        unrelated.kitty_placement_id = Some(1);
        terminal
            .inline_images
            .extend([parent, orphaned_child, unrelated]);
        terminal.kitty_relative_parents.insert((7, 2), (30, 4));
        let stale_placeholder = LastKittyPlaceholder {
            row: 2,
            column: 0,
            foreground: Color::Rgb(0, 0, 7),
            underline_color: Color::Rgb(0, 0, 2),
            image_id_high_byte: 0,
            placeholder_row: 0,
            placeholder_column: 0,
        };
        let unrelated_placeholder = LastKittyPlaceholder {
            row: 0,
            column: 0,
            foreground: Color::Rgb(0, 0, 99),
            underline_color: Color::Rgb(0, 0, 1),
            image_id_high_byte: 0,
            placeholder_row: 0,
            placeholder_column: 0,
        };
        terminal
            .kitty_placeholder_cells
            .insert((2, 0), stale_placeholder);
        terminal
            .kitty_placeholder_cells
            .insert((0, 0), unrelated_placeholder);
        terminal.last_kitty_placeholder = Some(stale_placeholder);
        terminal.take_damage();
        terminal.feed(b"\x1b[?69h\x1b[3;6s\x1b[2;4r");
        terminal.take_damage();

        terminal.feed(b"\x1b[S");

        assert!(!terminal.kitty_placeholder_cells.contains_key(&(2, 0)));
        assert_eq!(
            terminal
                .kitty_placeholder_cells
                .get(&(0, 0))
                .copied()
                .map(|placeholder| (
                    kitty_placeholder_image_id(placeholder.foreground),
                    kitty_placeholder_placement_id(placeholder.underline_color),
                )),
            Some((Some(99), Some(1)))
        );
        assert!(terminal.last_kitty_placeholder.is_none());
        let pending = PendingKittyPlaceholder {
            row: 2,
            column: 1,
            foreground: stale_placeholder.foreground,
            underline_color: stale_placeholder.underline_color,
            image_id: Some(7),
            placement_id: Some(2),
            diacritics: Vec::new(),
            rendered_row: None,
            rendered_column: None,
            rendered_image_id: None,
            rendered_placement_id: None,
        };
        assert!(terminal.left_kitty_placeholder_for(&pending).is_none());
        assert_eq!(
            terminal.take_damage(),
            vec![DamageRegion::new(2, 1, 4, 3), DamageRegion::new(0, 0, 8, 4)]
        );
    }

    #[test]
    fn terminal_horizontal_margin_su_keeps_live_msb_kitty_placeholder_cache() {
        let mut terminal = Terminal::new(TerminalSize::new(24, 4));
        let image_id = (2_u32 << 24) | 0x2c;
        let mut image = metadata_test_image(0, "msb-image");
        image.kitty_image_id = Some(image_id);
        terminal.inline_images.push(image);
        let placeholder = LastKittyPlaceholder {
            row: 0,
            column: 0,
            foreground: Color::Indexed(0x2c),
            underline_color: Color::Default,
            image_id_high_byte: 2,
            placeholder_row: 0,
            placeholder_column: 0,
        };
        terminal.kitty_placeholder_cells.insert((0, 0), placeholder);
        terminal.last_kitty_placeholder = Some(placeholder);
        terminal.take_damage();
        terminal.feed(b"\x1b[?69h\x1b[10;20s");
        terminal.take_damage();

        terminal.feed(b"\x1b[S");

        assert_eq!(
            terminal
                .kitty_placeholder_cells
                .get(&(0, 0))
                .copied()
                .map(|cached| cached.image_id_high_byte),
            Some(2)
        );
        assert_eq!(
            terminal
                .last_kitty_placeholder
                .map(|cached| cached.image_id_high_byte),
            Some(2)
        );
        assert_eq!(terminal.take_damage(), vec![DamageRegion::new(9, 0, 11, 4)]);
    }

    #[test]
    fn terminal_horizontal_margin_sd_zero_count_moves_only_bounded_cells() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.feed(b"abcdefgh\r\nijklmnop\r\nqrstuvwx\r\nyz012345");
        let physical_top = terminal.stable_dimensions().physical_top;

        terminal.feed(b"\x1b[?69h\x1b[3;6s\x1b[2;4r\x1b[0T");

        let rows = (0..4)
            .map(|row| {
                terminal
                    .cells_for_history_row(row)
                    .unwrap()
                    .iter()
                    .map(Cell::primary_char)
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        assert_eq!(rows, ["abcdefgh", "ij    op", "qrklmnwx", "yzstuv45"]);
        assert!(terminal.scrollback.is_empty());
        assert_eq!(terminal.stable_dimensions().physical_top, physical_top);
    }

    #[test]
    fn terminal_horizontal_margin_scroll_uses_rectangular_damage_without_stable_row_churn() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 4));
        terminal.feed(b"abcdefgh\r\nijklmnop\r\nqrstuvwx\r\nyz012345");
        let stable_rows = (0..4)
            .map(|row| terminal.history_index_to_stable_row(row).unwrap())
            .collect::<Vec<_>>();
        terminal.take_damage();
        terminal.feed(b"\x1b[?69h\x1b[3;6s\x1b[2;4r");
        terminal.take_damage();
        let before = terminal.current_seqno();

        terminal.feed(b"\x1b[S");

        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            stable_rows[1..]
        );
        assert_eq!(
            (0..4)
                .map(|row| terminal.history_index_to_stable_row(row).unwrap())
                .collect::<Vec<_>>(),
            stable_rows
        );
        assert_eq!(terminal.take_damage(), vec![DamageRegion::new(2, 1, 4, 3)]);
    }

    #[test]
    fn terminal_top_horizontal_margin_su_never_records_scrollback() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 3));
        terminal.feed(b"abcdefgh\r\nijklmnop\r\nqrstuvwx");
        let physical_top = terminal.stable_dimensions().physical_top;

        terminal.feed(b"\x1b[?69h\x1b[3;6s\x1b[1;3r\x1b[0S");

        let rows = (0..3)
            .map(|row| {
                terminal
                    .cells_for_history_row(row)
                    .unwrap()
                    .iter()
                    .map(Cell::primary_char)
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        assert_eq!(rows, ["abklmngh", "ijstuvop", "qr    wx"]);
        assert!(terminal.scrollback.is_empty());
        assert_eq!(terminal.stable_dimensions().physical_top, physical_top);
    }

    #[test]
    fn terminal_scrollback_prune_advances_stable_top_without_retargeting() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        let removed = terminal.history_index_to_stable_row(0).unwrap();
        let survivor = terminal.history_index_to_stable_row(1).unwrap();
        let survivor_text = stable_row_text(&terminal, survivor).unwrap();
        let survivor_seqno = stable_row_seqno(&terminal, survivor).unwrap();

        terminal.set_scrollback_limit(1);

        assert_eq!(terminal.stable_dimensions().scrollback_top, survivor);
        assert_eq!(terminal.stable_row_to_history_index(removed), None);
        assert_eq!(
            stable_row_text(&terminal, survivor).as_deref(),
            Some(survivor_text.as_str())
        );
        assert_eq!(stable_row_seqno(&terminal, survivor), Some(survivor_seqno));
    }

    #[test]
    fn terminal_zero_scrollback_limit_keeps_ids_monotonic() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.set_scrollback_limit(0);
        let initial_top = terminal.stable_dimensions().physical_top;

        terminal.feed(b"aa\r\nbb\r\ncc");
        let first_top = terminal.stable_dimensions().physical_top;
        terminal.feed(b"\r\ndd");
        let second_top = terminal.stable_dimensions().physical_top;

        assert!(first_top > initial_top);
        assert!(second_top > first_top);
        assert!(terminal.scrollback.is_empty());
    }

    #[test]
    fn terminal_runtime_limit_reduction_preserves_survivor_ids() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd\r\nee");
        let survivor = terminal.history_index_to_stable_row(2).unwrap();
        let text = stable_row_text(&terminal, survivor).unwrap();

        terminal.set_scrollback_limit(1);

        assert_eq!(terminal.stable_dimensions().scrollback_top, survivor);
        assert_eq!(
            stable_row_text(&terminal, survivor).as_deref(),
            Some(text.as_str())
        );
    }

    #[test]
    fn terminal_ed3_removes_history_without_dirtying_visible_rows() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        let removed = terminal.history_index_to_stable_row(0).unwrap();
        let visible_top = terminal.stable_dimensions().physical_top;
        let before = terminal.current_seqno();

        terminal.feed(b"\x1b[3J");

        assert!(terminal.scrollback.is_empty());
        assert_eq!(terminal.stable_dimensions().physical_top, visible_top);
        assert_eq!(terminal.stable_row_to_history_index(removed), None);
        assert!(
            terminal
                .changed_stable_rows_since(terminal.retained_stable_range(), before)
                .is_empty()
        );
    }

    #[test]
    fn terminal_erase_scrollback_and_viewport_prunes_then_dirties_replaced_rows() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        let removed = terminal.history_index_to_stable_row(0).unwrap();
        let visible_top = terminal.stable_dimensions().physical_top;
        let before = terminal.current_seqno();

        terminal.erase_scrollback_and_viewport();

        assert!(terminal.scrollback.is_empty());
        assert_eq!(terminal.stable_dimensions().physical_top, visible_top);
        assert_eq!(terminal.stable_row_to_history_index(removed), None);
        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            vec![visible_top, visible_top + 1]
        );
    }

    #[test]
    fn terminal_ris_prunes_history_without_stable_retargeting_and_dirties_new_grid() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        let removed = terminal.history_index_to_stable_row(0).unwrap();
        let visible_top = terminal.stable_dimensions().physical_top;
        let before = terminal.current_seqno();

        terminal.feed(b"\x1bc");

        assert!(terminal.scrollback.is_empty());
        assert_eq!(terminal.stable_dimensions().physical_top, visible_top);
        assert_eq!(terminal.stable_row_to_history_index(removed), None);
        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            vec![visible_top, visible_top + 1]
        );
    }

    #[test]
    fn terminal_prune_rebases_semantic_metadata_without_retargeting() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"\x1b]133;A\x07> one\r\n");
        terminal.feed(b"\x1b]133;C\x07run\x1b]133;D;0\x07\r\n");
        terminal.feed(b"tail\r\nmore");
        assert_eq!(terminal.semantic_prompt_rows, vec![0]);
        assert_eq!(terminal.semantic_command_exits[0].row, 1);
        let command_stable = terminal.history_index_to_stable_row(1).unwrap();

        terminal.set_scrollback_limit(1);

        assert!(terminal.semantic_prompt_rows.is_empty());
        assert_eq!(terminal.semantic_command_exits[0].row, 0);
        assert_eq!(
            terminal.history_index_to_stable_row(terminal.semantic_command_exits[0].row),
            Some(command_stable)
        );
    }

    #[test]
    fn terminal_prune_rebases_inline_image_metadata() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"history\r\n");
        terminal.feed(b"\x1b]1337;File=inline=1;width=1;height=1:QUJDRA==\x07\r\n");
        terminal.feed(b"tail\r\nmore");
        assert_eq!(terminal.inline_images.len(), 1);
        let image_stable = terminal
            .history_index_to_stable_row(terminal.inline_images[0].row)
            .unwrap();

        terminal.set_scrollback_limit(1);

        assert_eq!(terminal.inline_images.len(), 1);
        assert_eq!(
            terminal.history_index_to_stable_row(terminal.inline_images[0].row),
            Some(image_stable)
        );
    }

    #[test]
    fn terminal_prune_rebases_kitty_placeholder_metadata() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"history\r\n");
        terminal.feed(b"\x1b_Ga=T,U=1,q=1,i=57,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
        terminal.feed(b"\x1b[2;1H\x1b[38;5;57m");
        terminal.feed("\u{10eeee}\u{0305}\u{030d}".as_bytes());
        terminal.feed(b"\x1b[2;1H\r\n");
        terminal.feed(b"tail\r\nmore");
        assert_eq!(terminal.kitty_placeholder_cells.len(), 1);
        let ((placeholder_row, _), _) = terminal.kitty_placeholder_cells.iter().next().unwrap();
        let placeholder_stable = terminal
            .history_index_to_stable_row(*placeholder_row)
            .unwrap();

        terminal.set_scrollback_limit(1);

        let ((placeholder_row, _), _) = terminal.kitty_placeholder_cells.iter().next().unwrap();
        assert_eq!(
            terminal.history_index_to_stable_row(*placeholder_row),
            Some(placeholder_stable)
        );
    }

    #[test]
    fn terminal_runtime_prune_rebases_dormant_main_metadata() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        assert_eq!(terminal.scrollback.len(), 2);
        let survivor_stable = terminal.history_index_to_stable_row(1).unwrap();
        terminal
            .inline_images
            .push(metadata_test_image(0, "pruned"));
        terminal
            .inline_images
            .push(metadata_test_image(1, "survivor"));
        let pruned_placeholder = metadata_test_placeholder(0);
        let survivor_placeholder = metadata_test_placeholder(1);
        terminal
            .kitty_placeholder_cells
            .insert((0, 0), pruned_placeholder);
        terminal
            .kitty_placeholder_cells
            .insert((1, 0), survivor_placeholder);
        terminal.last_kitty_placeholder = Some(survivor_placeholder);

        terminal.feed(b"\x1b[?1049h");
        terminal.set_scrollback_limit(1);
        terminal.feed(b"\x1b[?1049l");

        assert_eq!(terminal.scrollback.len(), 1);
        assert_eq!(terminal.inline_images.len(), 1);
        assert_eq!(terminal.inline_images[0].name.as_deref(), Some("survivor"));
        assert_eq!(terminal.inline_images[0].row, 0);
        assert_eq!(
            terminal.history_index_to_stable_row(terminal.inline_images[0].row),
            Some(survivor_stable)
        );
        assert_eq!(terminal.kitty_placeholder_cells.len(), 1);
        assert!(terminal.kitty_placeholder_cells.contains_key(&(0, 0)));
        assert_eq!(
            terminal
                .last_kitty_placeholder
                .expect("surviving placeholder")
                .row,
            0
        );
    }

    #[test]
    fn terminal_runtime_prune_rebases_active_alt_and_dormant_main_metadata() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
        assert_eq!(terminal.scrollback.len(), 2);
        let main_survivor_stable = terminal.history_index_to_stable_row(1).unwrap();
        terminal
            .inline_images
            .push(metadata_test_image(1, "main-survivor"));
        let main_placeholder = metadata_test_placeholder(1);
        terminal
            .kitty_placeholder_cells
            .insert((1, 0), main_placeholder);
        terminal.last_kitty_placeholder = Some(main_placeholder);

        terminal.feed(b"\x1b[?1049h");
        let alt_visible_row = 1;
        let alt_metadata_row = terminal.scrollback.len() + alt_visible_row;
        terminal
            .inline_images
            .push(metadata_test_image(alt_metadata_row, "alt"));
        let alt_placeholder = metadata_test_placeholder(alt_metadata_row);
        terminal
            .kitty_placeholder_cells
            .insert((alt_metadata_row, 0), alt_placeholder);
        terminal.last_kitty_placeholder = Some(alt_placeholder);

        terminal.set_scrollback_limit(1);

        assert!(terminal.alternate_screen_active());
        assert_eq!(terminal.scrollback.len(), 1);
        assert_eq!(terminal.inline_images.len(), 1);
        assert_eq!(terminal.inline_images[0].name.as_deref(), Some("alt"));
        assert_eq!(
            terminal.inline_images[0].row - terminal.scrollback.len(),
            alt_visible_row
        );
        let rebased_alt_row = terminal.scrollback.len() + alt_visible_row;
        assert!(
            terminal
                .kitty_placeholder_cells
                .contains_key(&(rebased_alt_row, 0))
        );
        assert_eq!(
            terminal
                .last_kitty_placeholder
                .expect("active alternate placeholder")
                .row,
            rebased_alt_row
        );
        let dormant_main = terminal.main_screen.as_ref().expect("dormant main screen");
        assert_eq!(dormant_main.inline_images.len(), 1);
        assert_eq!(dormant_main.inline_images[0].row, 0);
        assert!(dormant_main.kitty_placeholder_cells.contains_key(&(0, 0)));

        terminal.feed(b"\x1b[?1049l");

        assert_eq!(terminal.inline_images.len(), 1);
        assert_eq!(
            terminal.inline_images[0].name.as_deref(),
            Some("main-survivor")
        );
        assert_eq!(
            terminal.history_index_to_stable_row(terminal.inline_images[0].row),
            Some(main_survivor_stable)
        );
        assert!(terminal.kitty_placeholder_cells.contains_key(&(0, 0)));
    }

    #[test]
    fn terminal_cursor_only_batch_does_not_mark_lines_changed() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        let before = terminal.current_seqno();

        terminal.feed(b"\x1b[2;3H");

        assert!(
            terminal
                .changed_stable_rows_since(terminal.retained_stable_range(), before)
                .is_empty()
        );
    }

    #[test]
    fn terminal_line_feed_without_scroll_does_not_mark_lines_changed() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        let before = terminal.current_seqno();

        terminal.feed(b"\n");

        assert!(
            terminal
                .changed_stable_rows_since(terminal.retained_stable_range(), before)
                .is_empty()
        );
    }

    #[test]
    fn terminal_same_size_resize_does_not_mark_lines_changed() {
        let size = TerminalSize::new(4, 3);
        let mut terminal = Terminal::new(size);
        let before = terminal.current_seqno();

        terminal.resize(size);

        assert!(
            terminal
                .changed_stable_rows_since(terminal.retained_stable_range(), before)
                .is_empty()
        );
    }

    #[test]
    fn terminal_width_resize_marks_retained_rows_changed() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        terminal.feed(b"aa\r\nbb");
        let before = terminal.current_seqno();

        terminal.resize(TerminalSize::new(6, 3));

        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            terminal.retained_stable_range().collect::<Vec<_>>()
        );
    }

    fn main_screen_rows(terminal: &Terminal) -> Vec<(String, bool)> {
        let mut rows = terminal
            .scrollback
            .iter()
            .map(|line| {
                (
                    line.cells().iter().map(Cell::primary_char).collect(),
                    line.is_wrapped(),
                )
            })
            .collect::<Vec<_>>();
        rows.extend((0..terminal.grid.size().rows).map(|row| {
            (
                (0..terminal.grid.size().columns)
                    .map(|column| terminal.grid.get(row, column).unwrap().primary_char())
                    .collect(),
                terminal.grid.row_wrapped(row),
            )
        }));
        rows
    }

    #[test]
    fn terminal_width_resize_reflows_soft_wraps_without_joining_hard_lines() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        terminal.feed(b"abcdef\r\nxyz");

        terminal.resize(TerminalSize::new(5, 3));
        assert_eq!(
            main_screen_rows(&terminal),
            vec![
                ("abcde".to_owned(), false),
                ("f    ".to_owned(), true),
                ("xyz  ".to_owned(), false),
            ]
        );

        terminal.resize(TerminalSize::new(6, 3));
        assert_eq!(
            main_screen_rows(&terminal),
            vec![
                ("abcdef".to_owned(), false),
                ("xyz   ".to_owned(), false),
                ("      ".to_owned(), false),
            ]
        );

        terminal.resize(TerminalSize::new(4, 3));
        assert_eq!(
            main_screen_rows(&terminal),
            vec![
                ("abcd".to_owned(), false),
                ("ef  ".to_owned(), true),
                ("xyz ".to_owned(), false),
            ]
        );
    }

    #[test]
    fn terminal_width_resize_reflows_scrollback_and_refills_exact_grid_height() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"abcdefghijkl");

        terminal.resize(TerminalSize::new(3, 2));

        assert_eq!(terminal.scrollback.len(), 2);
        assert_eq!(
            main_screen_rows(&terminal),
            vec![
                ("abc".to_owned(), false),
                ("def".to_owned(), true),
                ("ghi".to_owned(), true),
                ("jkl".to_owned(), true),
            ]
        );
        assert_eq!(terminal.grid.size(), TerminalSize::new(3, 2));
    }

    #[test]
    fn terminal_width_resize_keeps_wide_and_custom_width_cells_together() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        terminal.set_cell_width_overrides(vec![
            CellWidthOverride::new(u32::from('x'), u32::from('x'), 2),
            CellWidthOverride::new(u32::from('z'), u32::from('z'), 0),
        ]);
        terminal.feed("a界xzbc".as_bytes());

        terminal.resize(TerminalSize::new(3, 2));

        assert_eq!(
            main_screen_rows(&terminal),
            vec![
                ("a界 ".to_owned(), false),
                ("x b".to_owned(), true),
                ("c  ".to_owned(), true),
            ]
        );
        assert_eq!(terminal.scrollback.len(), 1);
        assert_eq!(
            terminal.scrollback.get(0).unwrap().cells()[1].primary_char(),
            '界'
        );
        assert_eq!(
            terminal.scrollback.get(0).unwrap().cells()[2].primary_char(),
            ' '
        );
        assert_eq!(terminal.grid.get(0, 0).unwrap().primary_char(), 'x');
        assert_eq!(terminal.grid.get(0, 1).unwrap().primary_char(), ' ');
    }

    #[test]
    fn terminal_width_resize_leaves_active_alternate_physical_and_reflows_dormant_main() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"abcdefgh");
        terminal.feed(b"\x1b[?1049h");
        terminal.feed(b"abcdef");

        terminal.resize(TerminalSize::new(5, 2));

        assert!(terminal.alternate_screen_active());
        assert_eq!(
            main_screen_rows(&terminal),
            vec![("abcd ".to_owned(), false), ("ef   ".to_owned(), true),]
        );

        terminal.feed(b"\x1b[?1049l");

        assert!(!terminal.alternate_screen_active());
        assert_eq!(
            main_screen_rows(&terminal),
            vec![("abcde".to_owned(), false), ("fgh  ".to_owned(), true),]
        );
    }

    #[test]
    fn terminal_width_resize_retires_unmapped_main_metadata_and_stable_rows() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"abcdefgh");
        let old_selection = stable_selection(
            stable_coordinate(&terminal, 0, 0),
            stable_coordinate(&terminal, 0, 1),
            false,
        );
        install_suffix_metadata(&mut terminal, 0);

        terminal.resize(TerminalSize::new(3, 2));

        assert!(terminal.semantic_prompt_rows.is_empty());
        assert!(terminal.semantic_command_exits.is_empty());
        assert!(terminal.inline_images.is_empty());
        assert!(terminal.kitty_placeholder_cells.is_empty());
        assert!(terminal.last_kitty_placeholder.is_none());
        assert_eq!(terminal.text_from_stable_selection(old_selection), None);
    }

    #[test]
    fn terminal_width_resize_retires_active_main_coordinate_state() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.set_unicode_version(14);
        terminal.feed("ab☁".as_bytes());
        terminal.feed(b"\x1b[s");

        terminal.resize(TerminalSize::new(3, 1));

        assert!(terminal.nfc_last_printable_cell.is_none());
        assert!(terminal.saved_cursor.is_none());
        terminal.feed(b"\x1b[u");
        assert_eq!(terminal.cursor(), (0, 0));
        terminal.feed("\u{fe0f}".as_bytes());
        assert_eq!(terminal.cursor(), (0, 0));
    }

    #[test]
    fn terminal_width_resize_retires_dormant_main_coordinate_state() {
        let mut terminal = Terminal::new(TerminalSize::new(2, 2));
        terminal.set_unicode_version(14);
        terminal.feed("ab☁".as_bytes());
        terminal.feed(b"\x1b[1;4H\x1b[s\x1b[?1049h");

        terminal.resize(TerminalSize::new(4, 2));

        let main = terminal.main_screen.as_ref().expect("dormant main screen");
        assert!(main.nfc_last_printable_cell.is_none());
        assert!(main.saved_cursor.is_none());

        terminal.feed(b"\x1b[?1049l\x1b[u");
        assert_eq!(terminal.cursor(), (0, 1));
        terminal.feed("\u{fe0f}".as_bytes());
        assert_eq!(terminal.cursor(), (0, 1));
    }

    #[test]
    fn terminal_width_resize_clears_dormant_main_pending_wrap() {
        let mut terminal = Terminal::new(TerminalSize::new(2, 1));
        terminal.feed("界".as_bytes());
        terminal.feed(b"\x1b[?1049h");

        terminal.resize(TerminalSize::new(3, 1));
        terminal.feed(b"\x1b[?1049lX");

        assert_eq!(terminal.grid.get(0, 0).unwrap().primary_char(), ' ');
        assert_eq!(terminal.grid.get(0, 1).unwrap().primary_char(), 'X');
        assert_eq!(terminal.grid.get(0, 2).unwrap().primary_char(), ' ');
    }

    #[test]
    fn terminal_width_resize_clears_active_main_pending_wrap() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.feed(b"abcd");

        terminal.resize(TerminalSize::new(5, 1));
        terminal.feed(b"X");

        assert_eq!(
            (0..5)
                .map(|column| terminal.grid.get(0, column).unwrap().primary_char())
                .collect::<String>(),
            "abcX "
        );
    }

    #[test]
    fn terminal_width_resize_keeps_active_alternate_pending_wrap_physical() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"\x1b[?1049habcd");

        terminal.resize(TerminalSize::new(5, 2));
        terminal.feed(b"X");

        assert_eq!(
            (0..5)
                .map(|column| terminal.grid.get(0, column).unwrap().primary_char())
                .collect::<String>(),
            "abcd "
        );
        assert_eq!(terminal.grid.get(1, 0).unwrap().primary_char(), 'X');
    }

    #[test]
    fn terminal_width_resize_through_zero_preserves_main_screen_content() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"abcd\r\nef");

        terminal.resize(TerminalSize::new(0, 2));
        terminal.resize(TerminalSize::new(4, 2));
        terminal.resize(TerminalSize::new(0, 2));
        terminal.resize(TerminalSize::new(4, 2));

        assert_eq!(
            main_screen_rows(&terminal),
            vec![("abcd".to_owned(), false), ("ef  ".to_owned(), false),]
        );
    }

    #[test]
    fn terminal_width_resize_preserves_empty_hard_line_between_text_lines() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 4));
        terminal.feed(b"A\r\n\r\nB");

        terminal.resize(TerminalSize::new(5, 2));

        assert_eq!(
            main_screen_rows(&terminal),
            vec![
                ("A    ".to_owned(), false),
                ("     ".to_owned(), false),
                ("B    ".to_owned(), false),
            ]
        );
    }

    #[test]
    fn terminal_width_resize_preserves_trailing_empty_hard_lines_without_viewport_padding() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 4));
        terminal.feed(b"A\r\n\r\n");

        terminal.resize(TerminalSize::new(5, 2));

        assert_eq!(
            main_screen_rows(&terminal),
            vec![
                ("A    ".to_owned(), false),
                ("     ".to_owned(), false),
                ("     ".to_owned(), false),
            ]
        );
    }

    #[test]
    fn terminal_width_resize_through_one_column_restores_wide_continuation_style() {
        let mut terminal = Terminal::new(TerminalSize::new(2, 1));
        terminal.feed(b"\x1b[31m\xE7\x95\x8C");

        terminal.resize(TerminalSize::new(1, 1));
        terminal.resize(TerminalSize::new(2, 1));
        terminal.resize(TerminalSize::new(1, 1));
        terminal.resize(TerminalSize::new(2, 1));

        assert_eq!(terminal.grid.get(0, 0).unwrap().primary_char(), '界');
        assert_eq!(
            terminal.grid.get(0, 0).unwrap().foreground,
            Color::Indexed(1)
        );
        assert_eq!(terminal.grid.get(0, 1).unwrap().primary_char(), ' ');
        assert_eq!(
            terminal.grid.get(0, 1).unwrap().foreground,
            Color::Indexed(1)
        );
    }

    #[test]
    fn terminal_width_resize_through_one_column_restores_custom_wide_continuation_style() {
        let mut terminal = Terminal::new(TerminalSize::new(2, 1));
        terminal.set_cell_width_overrides(vec![CellWidthOverride::new(
            u32::from('x'),
            u32::from('x'),
            2,
        )]);
        terminal.feed(b"\x1b[32mx");

        terminal.resize(TerminalSize::new(1, 1));
        terminal.resize(TerminalSize::new(2, 1));
        terminal.resize(TerminalSize::new(1, 1));
        terminal.resize(TerminalSize::new(2, 1));

        assert_eq!(terminal.grid.get(0, 0).unwrap().primary_char(), 'x');
        assert_eq!(
            terminal.grid.get(0, 0).unwrap().foreground,
            Color::Indexed(2)
        );
        assert_eq!(terminal.grid.get(0, 1).unwrap().primary_char(), ' ');
        assert_eq!(
            terminal.grid.get(0, 1).unwrap().foreground,
            Color::Indexed(2)
        );
    }

    #[test]
    fn terminal_width_one_column_keeps_reflow_continuation_out_of_public_scrollback_cells() {
        let mut terminal = Terminal::new(TerminalSize::new(1, 1));
        terminal.feed(b"\x1b[31m\xE7\x95\x8C");
        terminal.feed(b"x");

        assert_eq!(terminal.scrollback.len(), 2);
        let narrow_wide = terminal
            .scrollback
            .get(terminal.scrollback.len().saturating_sub(1))
            .unwrap();
        assert_eq!(narrow_wide.cells().len(), 1);
        assert_eq!(narrow_wide.cells()[0].primary_char(), '界');

        terminal.resize(TerminalSize::new(2, 1));

        let wide = terminal
            .scrollback
            .get(terminal.scrollback.len().saturating_sub(1))
            .unwrap();
        assert_eq!(wide.cells().len(), 2);
        assert_eq!(wide.cells()[1].primary_char(), ' ');
        assert_eq!(wide.cells()[1].foreground, Color::Indexed(1));
    }

    #[test]
    fn terminal_placeholder_cell_assignment_marks_row_changed() {
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"\x1b_Ga=T,U=1,q=1,i=57,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
        terminal.feed(b"\x1b[2;1H\x1b[38;5;57m");
        let before = terminal.current_seqno();

        terminal.feed("\u{10eeee}\u{0305}\u{030d}".as_bytes());

        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            vec![1]
        );
    }

    #[test]
    fn terminal_mark_all_lines_changed_marks_active_domain_rows_changed() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"aa\r\nbb\r\ncc");
        let before = terminal.current_seqno();

        terminal.mark_all_lines_changed();

        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            terminal.retained_stable_range().collect::<Vec<_>>()
        );

        terminal.feed(b"\x1b[?1049h");
        let before = terminal.current_seqno();
        terminal.mark_all_lines_changed();
        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            vec![0, 1]
        );
    }

    #[test]
    fn terminal_public_mark_all_lines_changed_advances_sequence_once() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        let before = terminal.current_seqno();

        terminal.mark_all_lines_changed();

        assert_eq!(terminal.current_seqno(), before.checked_add(1).unwrap());
    }

    #[test]
    fn terminal_feed_with_all_lines_changed_advances_sequence_once() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        let before = terminal.current_seqno();

        terminal.feed_with_all_lines_changed(b"\x1b[2;2H");

        assert_eq!(terminal.current_seqno(), before.checked_add(1).unwrap());
        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            vec![0, 1]
        );
    }

    #[test]
    fn terminal_ris_with_whole_line_dirty_advances_sequence_once() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 2));
        terminal.feed(b"aa\r\nbb\r\ncc");
        let before = terminal.current_seqno();

        terminal.feed_with_all_lines_changed(b"\x1bc");

        assert_eq!(terminal.current_seqno(), before.checked_add(1).unwrap());
        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            terminal.retained_stable_range().collect::<Vec<_>>()
        );
    }

    #[test]
    fn terminal_partial_region_scroll_marks_only_affected_slots_changed() {
        type ScrollCase = (
            &'static str,
            &'static [u8],
            &'static [u8],
            &'static [StableRowIndex],
        );
        let cases: &[ScrollCase] = &[
            ("non-top SU", b"\x1b[2;4r", b"\x1b[S", &[1, 2, 3]),
            ("non-top SD", b"\x1b[2;4r", b"\x1b[T", &[1, 2, 3]),
            ("insert line", b"\x1b[2;4r\x1b[2;1H", b"\x1b[L", &[1, 2, 3]),
            ("delete line", b"\x1b[2;4r\x1b[2;1H", b"\x1b[M", &[1, 2, 3]),
            (
                // This case locks stable row identity/sequence metadata only.
                // Cell-level horizontal-margin scrolling is a separate parity slice.
                "top narrow margins",
                b"\x1b[?69h\x1b[2;3s\x1b[1;3r",
                b"\x1b[S",
                &[0, 1, 2],
            ),
        ];

        for (name, setup, operation, expected) in cases {
            let mut terminal = Terminal::new(TerminalSize::new(4, 4));
            terminal.feed(b"aa\r\nbb\r\ncc\r\ndd");
            terminal.feed(setup);
            let before = terminal.current_seqno();
            let unaffected = terminal
                .retained_stable_range()
                .filter(|row| !expected.contains(row))
                .map(|row| {
                    (
                        row,
                        terminal.stable_row_to_history_index(row).unwrap(),
                        stable_row_text(&terminal, row).unwrap(),
                        stable_row_seqno(&terminal, row).unwrap(),
                    )
                })
                .collect::<Vec<_>>();

            terminal.feed(operation);

            assert_eq!(
                terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
                *expected,
                "{name}"
            );
            for (row, history_index, text, seqno) in &unaffected {
                assert_eq!(
                    terminal.stable_row_to_history_index(*row),
                    Some(*history_index),
                    "{name}: unaffected stable identity {row}"
                );
                assert_eq!(
                    stable_row_text(&terminal, *row).as_deref(),
                    Some(text.as_str()),
                    "{name}: unaffected text at {row}"
                );
                assert_eq!(
                    stable_row_seqno(&terminal, *row),
                    Some(*seqno),
                    "{name}: unaffected sequence at {row}"
                );
            }
        }
    }

    #[test]
    fn terminal_alternate_scroll_marks_alt_slots_changed_without_main_history() {
        let mut terminal = Terminal::new(TerminalSize::new(4, 3));
        terminal.feed(b"main\r\nhistory\r\nline");
        let main_history = terminal.scrollback.len();
        terminal.feed(b"\x1b[?1049h");
        terminal.feed(b"aa\r\nbb\r\ncc");
        let before = terminal.current_seqno();

        terminal.feed(b"\r\ndd");

        assert_eq!(terminal.scrollback.len(), main_history);
        assert_eq!(
            terminal.changed_stable_rows_since(terminal.retained_stable_range(), before),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn task10_terminal_capture_records_construct_feed_and_final_snapshot() {
        let row_id = "0000000000000000000000000000000000000000000000000000000000000000";
        let (execution, trace) = crate::fixture_trace::capture(row_id, "terminal_parser", || {
            let mut terminal = Terminal::new(TerminalSize::new(4, 2));
            terminal.feed(b"abc");
        });
        assert!(execution.is_ok());
        let trace = String::from_utf8(trace).expect("terminal fixture trace UTF-8");
        assert!(trace.contains("action_count=2\n"));
        assert!(trace.contains("api=terminal.feed"));
        assert!(trace.contains("final_object=terminal-parser:"));
    }

    #[test]
    fn current_terminal_parser_fixtures_match_detached_c69_traces() {
        let records = crate::frozen_trace_pack::records()
            .iter()
            .filter(|record| record.domain == "terminal_parser")
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 139, "terminal parser fixture rows");
        let source = include_str!("parser.rs");
        let bodies = crate::test_body_digest::test_body_sha256s(source);
        for record in records {
            let current_body = bodies.get(record.current_test_name).unwrap_or_else(|| {
                panic!("missing terminal parser body: {}", record.current_test_name)
            });
            assert_eq!(
                current_body, record.current_body_sha256,
                "{} current terminal parser body",
                record.row_id
            );
            let (execution, trace) =
                crate::fixture_trace::capture(record.row_id, "terminal_parser", || {
                    assert!(task10_registry::replay(record.current_test_name));
                });
            crate::frozen_trace_pack::assert_current_trace(record, &trace);
            if let Err(payload) = execution {
                std::panic::resume_unwind(payload);
            }
        }
    }
}
