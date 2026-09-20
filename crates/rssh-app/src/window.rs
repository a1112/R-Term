#[cfg(feature = "ssh")]
use crate::cli::{NativeHostKeyPolicy, SshTarget};
#[cfg(feature = "ssh")]
use rssh_core::app_shell::SshTargetKind;
use std::{
    any::Any,
    borrow::Cow,
    cell::{Cell, Ref, RefCell},
    cmp::Reverse,
    collections::{BTreeMap, HashMap, HashSet},
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, Write},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex, OnceLock, mpsc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
#[cfg(test)]
use std::io::Read;

use base64::{Engine, engine::general_purpose::STANDARD};
use rssh_core::{
    DamageRegion, TerminalSize,
    app_shell::{
        AppAction, AppShell, AppShellError, ClosedTabEntry, ClosedTabHistory,
        CloseTabSelection, PaneDirection, PaneLaunch, PaneLaunchDomain, PaneProgress,
        PaneRotationDirection, ResizeDirection, SplitDirection, SshAuthDescription,
        SshKnownHostsPolicy, SshPaneLaunch,
    },
};
use rssh_native::input::{
    CanonicalizePastedNewlines as NativeCanonicalizePastedNewlines,
    encode_paste as encode_window_paste,
};
use rssh_pty::{
    LocalPtyTransport, PtyCommand, PtyExitStatus, PtyMasterClose, PtyMasterCloseStatus, PtySession,
    PtySize,
};
#[cfg(feature = "ssh")]
use rssh_ssh::SshAuthMethod;
use rterm_runtime::{PaneWorkerConfig, SessionTransport};
#[cfg(feature = "ssh")]
use rssh_ssh::{
    HostKeyChallenge, HostKeyDecision, HostKeyVerifier, RusshChannelOpener, RusshHostKeyPolicy,
    SecretPrompt, SecretPromptKind, SecretProvider, SshChannelConnector, SshConnectionPhase,
    SshConnectRequest, SshSessionConfig,
    SshSessionStartup, SshShellConnector, SshShellWriter,
};
use rterm_render_wgpu::{GpuFramePlanner, gpu::{GpuFrameStatus, GpuPresentationMetrics}};
use rterm_render_core::{
    RenderCell, RenderCellColorRole, RenderGeometry, RenderStyle, SCROLLBAR_WIDTH,
    TerminalRenderSnapshot,
};
use rterm_render_cpu::{
    PixelRenderer, RenderBackgroundGradient, RenderBackgroundGradientBlend,
    RenderBackgroundGradientHsb, RenderBackgroundGradientInterpolation,
    RenderBackgroundGradientOrientation, RenderBackgroundGradientPreset,
    RenderBackgroundGradientSegment, RenderBackgroundImage, RenderBackgroundImageAttachment,
    RenderBackgroundImageDimension, RenderBackgroundImageHorizontalAlign,
    RenderBackgroundImageLength, RenderBackgroundImageRepeat, RenderBackgroundImageVerticalAlign,
    RenderBackgroundLayer, RenderBoldBrightensAnsiColors, RenderCursorThickness,
    RenderInlineImage, RenderScrollbarThumbSize,
    RenderStrikethroughPosition, RenderUnderlinePosition, RenderUnderlineThickness,
    ScrollbackScrollbar, background_gradient_color_strings, color_to_rgba,
};
use rssh_terminal::{
    CellWidthOverride, Color, CursorStyle, DEFAULT_SCROLLBACK_LIMIT, InlineImageFormat,
    SemanticType, SequenceNo, StableRowIndex, StableSelectionCoordinate, StableSelectionRange,
    Terminal, TerminalResizeOutcome, TerminalScreenDomain, UnderlineStyle, VerticalAlign,
};
use serde::{Deserialize, Serialize};
use sysinfo::{ProcessesToUpdate, System, get_current_pid};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
#[cfg(target_os = "macos")]
use winit::platform::macos::{WindowAttributesExtMacOS, WindowExtMacOS};
#[cfg(target_os = "windows")]
use winit::platform::windows::{
    CornerPreference, WindowAttributesExtWindows, WindowExtWindows,
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, KeyCode as WinitKeyCode, ModifiersState, NamedKey, PhysicalKey},
    window::{CursorIcon, Fullscreen, Window, WindowAttributes, WindowLevel as WinitWindowLevel},
};

use crate::{
    cli::{
        DiagnosticGuiOptions, Osc52Policy, RendererMode, SshOptions,
        WindowOptions, WindowPosition, WindowPositionOrigin,
    },
    config_lifecycle::{
        ConfigDiscoveryInputs, NativeConfigLoadError, bind_native_config_projection,
        validate_cli_config_overrides,
    },
    terminal_input::{TerminalKey, encode_terminal_key},
    terminal_modes::{
        KITTY_KEYBOARD_ALTERNATE_KEYS, KITTY_KEYBOARD_ASSOCIATED_TEXT, KITTY_KEYBOARD_DISAMBIGUATE,
        KITTY_KEYBOARD_REPORT_ALL, KITTY_KEYBOARD_REPORT_EVENTS, MouseInputMode, MouseProtocolMode,
        MouseReportingMode,
    },
    runtime_composition::{
        ActiveWindowRuntime, PaneCapturePolicy, PaneRuntimeRoute, RuntimeComposition,
        RuntimeHostEvent, WindowPaneRuntime,
    },
    startup_metrics::{ConnectionState, RendererKind, StartupTrace},
    terminal_runtime::{TerminalNotification, TerminalProgress, TerminalRuntime},
    window_bootstrap::WindowBootstrapSurface,
    window_gpu::WindowGpu,
};
use crate::diagnostic_markers::DiagnosticMarkerHandle;
use rssh_diagnostics::{
    ConnectionState as DiagnosticConnectionState, MarkerKind as DiagnosticMarkerKind,
    RendererKind as DiagnosticRendererKind, Scenario as DiagnosticScenario,
};
const PRODUCT_GUI_PROBE_ENV: &str = "RSSH_STAGE7_PRODUCT_GUI_PROBE";
const PRODUCT_GUI_PROBE_SCHEMA: &str = "rssh.stage7/product-gui-probe/v1";
#[path = "window_config.rs"]
mod window_config;
use window_config::{
    native_resolved_palette_from_overrides, native_resolved_palette_with_overrides,
};
#[cfg(test)]
fn configure_native_config_snapshot(
    mut snapshot: NativeConfigSnapshot,
    apply: impl FnOnce(&mut NativeConfigSnapshot),
) -> NativeConfigSnapshot {
    apply(&mut snapshot);
    snapshot
}

#[cfg(test)]
macro_rules! native_config_snapshot {
    ($($field:ident: $value:expr,)* ..$base:expr $(,)?) => {
        $crate::window::configure_native_config_snapshot($base, |snapshot| {
            $(snapshot.$field = $value;)*
        })
    };
}

#[cfg(test)]
macro_rules! native_config_view {
    ($($field:ident: $value:expr,)*) => {{
        let mut view = NativeWindowApp::new(None).native_effective_config();
        $(view.$field = $value;)*
        view
    }};
}

#[cfg(test)]
fn configure_native_window_config_patch_values(
    mut values: NativeWindowConfigPatchValues,
    apply: impl FnOnce(&mut NativeWindowConfigPatchValues),
) -> NativeWindowConfigPatchValues {
    apply(&mut values);
    values
}

#[cfg(test)]
macro_rules! native_window_config_patch_values {
    ($($field:ident: $value:expr,)* ..$base:expr $(,)?) => {
        $crate::window::configure_native_window_config_patch_values($base, |values| {
            $(values.$field = $value;)*
        })
    };
}

#[cfg(test)]
#[path = "window_config_tests.rs"]
mod window_config_tests;
bind_native_config_projection!(NativeConfigSnapshot, native_config_overrides_from_wezterm_lua_config, automatically_reload_config);
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;
const DEFAULT_INITIAL_COLS: u16 = TERMINAL_COLUMNS;
const DEFAULT_INITIAL_ROWS: u16 = TERMINAL_ROWS;
const TAB_BAR_ROWS: u16 = 1;
// Keep the default grid geometry in physical pixels.  The native renderer and
// the initial frame size use these values directly, so changing them here
// updates the default 80x24 viewport without changing explicit font/line
// height overrides or compatibility fixtures that provide their own sizes.
const CELL_WIDTH: u32 = 9;
const CELL_HEIGHT: u32 = 18;
// Modern native windows use a more relaxed visual grid than legacy/test
// fixtures.  Keeping the compatibility constants above unchanged means
// explicit WezTerm geometry and the legacy renderer remain byte-for-byte
// stable while production defaults can follow the concept target.
const MODERN_CELL_WIDTH: u32 = 10;
const MODERN_CELL_HEIGHT: u32 = 21;
const DEFAULT_WINDOW_TITLE: &str = "R-SSH";
static NEXT_PANE_RUNTIME_TOKEN: AtomicU64 = AtomicU64::new(1);
#[cfg(test)]
thread_local! {
    static PROCESS_CWD_PROBE_COUNT: Cell<u64> = const { Cell::new(0) };
}
static PANE_PTY_REAPER_PENDING: AtomicUsize = AtomicUsize::new(0);
static PANE_PTY_REAPER_THREADS: OnceLock<Mutex<Vec<thread::JoinHandle<()>>>> = OnceLock::new();
type RetainedPanePtyOwnership = Box<dyn Any + Send>;
static PANE_PTY_REAPER_RETAINED: OnceLock<Mutex<Vec<RetainedPanePtyOwnership>>> = OnceLock::new();
const DEFAULT_DOMAIN_NAME: &str = "local";
const DEFAULT_GUI_STARTUP_ARGS: &[&str] = &["start"];
const DEFAULT_MUX_ENABLE_SSH_AGENT: bool = true;
const DEFAULT_MUX_ENV_REMOVE: &[&str] = &["SSH_AUTH_SOCK", "SSH_CLIENT", "SSH_CONNECTION"];
const DEFAULT_RATELIMIT_MUX_LINE_PREFETCHES_PER_SECOND: u32 = 50;
const DEFAULT_MUX_OUTPUT_PARSER_BUFFER_SIZE: usize = 128 * 1024;
const DEFAULT_MUX_OUTPUT_PARSER_COALESCE_DELAY_MS: u64 = 3;
const DEFAULT_UNIX_DOMAIN_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_SSH_DOMAIN_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_SSH_DOMAIN_LOCAL_ECHO_THRESHOLD_MS: u64 = 100;
const DEFAULT_TLS_DOMAIN_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_TLS_DOMAIN_LOCAL_ECHO_THRESHOLD_MS: u64 = 100;
const DEFAULT_PERIODIC_STAT_LOGGING: u64 = 0;
const DEFAULT_ULIMIT_NOFILE: u64 = 2048;
const DEFAULT_ULIMIT_NPROC: u64 = 2048;
const DEFAULT_TILING_DESKTOP_ENVIRONMENTS: &[&str] = &[
    "X11 LG3D",
    "X11 Qtile",
    "X11 awesome",
    "X11 bspwm",
    "X11 dwm",
    "X11 i3",
    "X11 xmonad",
];
const DEFAULT_WORKSPACE_NAME: &str = "default";
const DEFAULT_AUTOMATICALLY_RELOAD_CONFIG: bool = true;
const DEFAULT_CHECK_FOR_UPDATES: bool = true;
const DEFAULT_CHECK_FOR_UPDATES_INTERVAL_SECONDS: u64 = 86_400;
const DEFAULT_SHOW_UPDATE_WINDOW: bool = false;
const DEFAULT_NATIVE_MACOS_FULLSCREEN_MODE: bool = false;
const DEFAULT_MACOS_FULLSCREEN_EXTEND_BEHIND_NOTCH: bool = false;
const DEFAULT_USE_RESIZE_INCREMENTS: bool = false;
const DEFAULT_PREFER_TO_SPAWN_TABS: bool = false;
const DEFAULT_DEBUG_KEY_EVENTS: bool = false;
const DEFAULT_LOG_UNKNOWN_ESCAPE_SEQUENCES: bool = false;
const DEFAULT_WARN_ABOUT_MISSING_GLYPHS: bool = true;
const DEBUG_OVERLAY_MAX_LOG_LINES: usize = 16;
const DEFAULT_FONT_SIZE_SCALE: f64 = 1.0;
const FONT_SIZE_STEP: f64 = 1.1;
#[cfg(test)]
const FRAME_WIDTH: u32 = TERMINAL_COLUMNS as u32 * CELL_WIDTH;
#[cfg(test)]
const FRAME_HEIGHT: u32 = (TERMINAL_ROWS as u32 + TAB_BAR_ROWS as u32) * CELL_HEIGHT;
const MODERN_WINDOW_PADDING_HORIZONTAL_PIXELS: u32 = 28;
const MODERN_WINDOW_PADDING_VERTICAL_PIXELS: u32 = 20;
const MODERN_TAB_BAR_BRAND_INSET_COLUMNS: u16 = 2;
const MODERN_TAB_BAR_BRAND_GAP_COLUMNS: u16 = 3;
const DOUBLE_CLICK_MAX_INTERVAL: Duration = Duration::from_millis(500);
const DEFAULT_LEADER_TIMEOUT: Duration = Duration::from_millis(1_000);
const DEFAULT_STATUS_UPDATE_INTERVAL: Duration = Duration::from_millis(1_000);
const DEFAULT_MAX_FPS: usize = 60;
const DEFAULT_ANIMATION_FPS: usize = 10;
const DEFAULT_RENDER_FRONT_END: NativeRenderFrontEnd = NativeRenderFrontEnd::OpenGl;
const DEFAULT_WEBGPU_POWER_PREFERENCE: NativeWebGpuPowerPreference =
    NativeWebGpuPowerPreference::LowPower;
const DEFAULT_WEBGPU_FORCE_FALLBACK_ADAPTER: bool = false;
const DEFAULT_PREFER_EGL: bool = true;
const DEFAULT_ENABLE_WAYLAND: bool = true;
const DEFAULT_ENABLE_ZWLR_OUTPUT_MANAGER: bool = false;
const DEFAULT_USE_BOX_MODEL_RENDER: bool = false;
const DEFAULT_EXPERIMENTAL_PIXEL_POSITIONING: bool = false;
const DEFAULT_SHAPE_CACHE_SIZE: usize = 1_024;
const DEFAULT_LINE_STATE_CACHE_SIZE: usize = 1_024;
const DEFAULT_LINE_QUAD_CACHE_SIZE: usize = 1_024;
const DEFAULT_LINE_TO_ELE_SHAPE_CACHE_SIZE: usize = 1_024;
const DEFAULT_GLYPH_CACHE_IMAGE_CACHE_SIZE: usize = 256;
// Modern native windows and the GPU emergency shaper share a 17px visual
// baseline. Legacy/test fixtures retain the 15px compatibility default; each
// path scales configured font sizes from its own baseline.
const DEFAULT_FONT_SIZE: NativeFontSize = NativeFontSize::from_millipoints(15_000);
const MODERN_DEFAULT_FONT_SIZE: NativeFontSize = NativeFontSize::from_millipoints(17_000);
const DEFAULT_COMMAND_PALETTE_FONT_SIZE: NativeFontSize = NativeFontSize::from_millipoints(14_000);
const DEFAULT_CHAR_SELECT_FONT_SIZE: NativeFontSize = NativeFontSize::from_millipoints(18_000);
const DEFAULT_PANE_SELECT_FONT_SIZE: NativeFontSize = NativeFontSize::from_millipoints(36_000);
const DEFAULT_COMMAND_PALETTE_FG_COLOR: Color = Color::Rgb(0xd8, 0xe2, 0xf0);
const DEFAULT_COMMAND_PALETTE_BG_COLOR: Color = Color::Rgb(0x10, 0x18, 0x27);
const DEFAULT_CHAR_SELECT_FG_COLOR: Color = Color::Rgb(0xd8, 0xe2, 0xf0);
const DEFAULT_CHAR_SELECT_BG_COLOR: Color = Color::Rgb(0x10, 0x18, 0x27);
const DEFAULT_PANE_SELECT_FG_COLOR: Color = Color::Rgb(0xd8, 0xe2, 0xf0);
const DEFAULT_PANE_SELECT_BG_COLOR: Color = Color::Rgba(0x0b, 0x12, 0x20, 0xe6);
// Keep transient action surfaces on the same deep-blue/cyan system as the
// terminal and tab chrome.  These defaults are intentionally separate from
// command/character selector body colors so explicit WezTerm overrides keep
// their existing precedence while selected rows remain visually consistent.
const DEFAULT_UI_ACCENT_FOREGROUND: Color = Color::Rgb(0x0b, 0x12, 0x20);
const DEFAULT_UI_ACCENT_BACKGROUND: Color = Color::Rgb(0x38, 0xbd, 0xf8);
const DEFAULT_UI_SURFACE_FOREGROUND: Color = Color::Rgb(0xd8, 0xe2, 0xf0);
const DEFAULT_UI_SURFACE_BACKGROUND: Color = Color::Rgb(0x1b, 0x2b, 0x44);
const DEFAULT_UI_SUBDUED_FOREGROUND: Color = Color::Rgb(0x84, 0x92, 0xa6);
const DEFAULT_SPLIT_ACTIVE_COLOR: Color = Color::Rgb(0x38, 0xbd, 0xf8);
const DEFAULT_SPLIT_INACTIVE_COLOR: Color = Color::Rgb(0x47, 0x55, 0x69);
const DEFAULT_SPLIT_BACKGROUND_COLOR: Color = Color::Rgb(0x10, 0x18, 0x27);
const PANE_CLOSE_BUTTON_GLYPH: char = '×';
const PANE_CLOSE_BUTTON_FOREGROUND: Color = Color::Rgb(0x0b, 0x12, 0x20);
const PANE_CLOSE_BUTTON_BACKGROUND: Color = Color::Rgb(0xf8, 0x71, 0x71);
const PANE_INSPECTION_FOREGROUND: Color = Color::Rgb(0xd8, 0xe2, 0xf0);
const PANE_INSPECTION_BACKGROUND: Color = Color::Rgb(0x1b, 0x2b, 0x44);
const DEFAULT_CELL_WIDTH: NativeCellWidth = NativeCellWidth::from_per_mille(1_000);
const DEFAULT_LINE_HEIGHT: NativeLineHeight = NativeLineHeight::from_per_mille(1_000);
const DEFAULT_FONT_ANTIALIAS: NativeFontAntialias = NativeFontAntialias::Greyscale;
const DEFAULT_FONT_HINTING: NativeFontHinting = NativeFontHinting::Full;
const DEFAULT_FONT_RASTERIZER: NativeFontRasterizer = NativeFontRasterizer::FreeType;
const DEFAULT_FONT_COLR_RASTERIZER: NativeFontRasterizer = NativeFontRasterizer::Harfbuzz;
const DEFAULT_FONT_SHAPER: NativeFontShaper = NativeFontShaper::Harfbuzz;
const DEFAULT_FONT_LOCATOR: Option<NativeFontLocator> = None;
const DEFAULT_USE_CAP_HEIGHT_TO_SCALE_FALLBACK_FONTS: bool = false;
const DEFAULT_IGNORE_SVG_FONTS: bool = false;
const DEFAULT_SORT_FALLBACK_FONTS_BY_COVERAGE: bool = false;
const DEFAULT_SEARCH_FONT_DIRS_FOR_FALLBACK: bool = false;
const DEFAULT_CUSTOM_BLOCK_GLYPHS: bool = true;
const DEFAULT_ANTI_ALIAS_CUSTOM_BLOCK_GLYPHS: bool = true;
const DEFAULT_ALLOW_SQUARE_GLYPHS_TO_OVERFLOW_WIDTH: NativeSquareGlyphOverflow =
    NativeSquareGlyphOverflow::WhenFollowedBySpace;
const DEFAULT_FREETYPE_LOAD_TARGET: NativeFreetypeTarget = NativeFreetypeTarget::Normal;
const FREETYPE_LOAD_FLAGS_NO_HINTING_DPI_THRESHOLD: u32 = 100;
const DEFAULT_FREETYPE_PCF_LONG_FAMILY_NAMES: bool = false;
const DEFAULT_DISPLAY_PIXEL_GEOMETRY: NativeDisplayPixelGeometry = NativeDisplayPixelGeometry::Rgb;
const DEFAULT_CURSOR_BLINK_RATE: Duration = Duration::from_millis(800);
const DEFAULT_CURSOR_BLINK_EASE_IN: NativeEasingFunction = NativeEasingFunction::Linear;
const DEFAULT_CURSOR_BLINK_EASE_OUT: NativeEasingFunction = NativeEasingFunction::Linear;
const DEFAULT_TEXT_BLINK_RATE: Duration = Duration::from_millis(500);
const DEFAULT_TEXT_BLINK_RATE_RAPID: Duration = Duration::from_millis(250);
const DEFAULT_TEXT_BLINK_EASE_IN: NativeEasingFunction = NativeEasingFunction::Linear;
const DEFAULT_TEXT_BLINK_EASE_OUT: NativeEasingFunction = NativeEasingFunction::Linear;
const DEFAULT_TEXT_BLINK_RAPID_EASE_IN: NativeEasingFunction = NativeEasingFunction::Linear;
const DEFAULT_TEXT_BLINK_RAPID_EASE_OUT: NativeEasingFunction = NativeEasingFunction::Linear;
const DEFAULT_RENDER_FOREGROUND_RGBA: [u8; 4] = [0xd8, 0xe2, 0xf0, 0xff];
const DEFAULT_RENDER_BACKGROUND_RGBA: [u8; 4] = [0x0b, 0x12, 0x20, 0xff];
const DEFAULT_WINDOW_CHROME_BORDER_RGBA: [u8; 4] = [0x47, 0x55, 0x69, 0xff];
const DEFAULT_TAB_BAR_SEPARATOR_RGBA: [u8; 4] = [0x2b, 0x3b, 0x53, 0xff];
const DEFAULT_FOREGROUND_COLOR: Color = Color::Rgb(0xd8, 0xe2, 0xf0);
const DEFAULT_MODERN_WINDOW_BUTTON_FOREGROUND_COLOR: Color = Color::Rgb(0xf8, 0xfa, 0xfc);
const DEFAULT_BACKGROUND_COLOR: Color = Color::Rgb(0x0b, 0x12, 0x20);
const DEFAULT_CURSOR_FG_COLOR: Color = Color::Rgb(0x0b, 0x12, 0x20);
const DEFAULT_CURSOR_BG_COLOR: Color = Color::Rgb(0x67, 0xe8, 0xf9);
const DEFAULT_SELECTION_BG_COLOR: Color = Color::Rgba(0x33, 0x41, 0x55, 0xb3);
const DEFAULT_QUICK_SELECT_LABEL_FG_COLOR: Color = Color::Rgb(0x0b, 0x12, 0x20);
const DEFAULT_QUICK_SELECT_LABEL_BG_COLOR: Color = Color::Rgb(0x38, 0xbd, 0xf8);
const LEGACY_COLOR_SCHEME_CURSOR_BG_COLOR: Color = Color::Rgb(229, 229, 229);
#[cfg(test)]
const LEGACY_TEST_FOREGROUND_COLOR: Color = Color::Rgb(229, 229, 229);
#[cfg(test)]
const LEGACY_TEST_BACKGROUND_COLOR: Color = Color::Rgb(12, 12, 12);
#[cfg(test)]
const LEGACY_TEST_CURSOR_FG_COLOR: Option<Color> = None;
#[cfg(test)]
const LEGACY_TEST_CURSOR_BG_COLOR: Color = LEGACY_TEST_FOREGROUND_COLOR;
const DEFAULT_TAB_BAR_BACKGROUND_COLOR: Color = Color::Rgb(0x08, 0x0d, 0x18);
const DEFAULT_MODERN_BRAND_BADGE_BACKGROUND: Color = Color::Rgb(0x1b, 0x2b, 0x44);
// Keep the active tab on the concept's visible blue surface.  The brand badge
// and active tab intentionally share this family while the tab-bar background
// remains substantially darker, preserving a clear focus layer at native
// terminal-cell scale.
const DEFAULT_MODERN_ACTIVE_TAB_BACKGROUND: Color = Color::Rgb(0x1b, 0x2b, 0x44);
const DEFAULT_TAB_BAR_ACTIVE_TAB_COLORS: NativeTabBarItemColors = NativeTabBarItemColors {
    fg_color: Some(Color::Rgb(0xf8, 0xfa, 0xfc)),
    // Give the active surface a restrained blue lift so it separates from
    // the tab strip without competing with the cyan brand accent.
    bg_color: Some(DEFAULT_MODERN_ACTIVE_TAB_BACKGROUND),
    intensity: Some(NativeFormatIntensity::Normal),
    underline: None,
    italic: None,
    strikethrough: None,
};
const DEFAULT_TAB_BAR_INACTIVE_TAB_COLORS: NativeTabBarItemColors = NativeTabBarItemColors {
    fg_color: Some(Color::Rgb(0x84, 0x92, 0xa6)),
    bg_color: Some(Color::Rgb(0x10, 0x18, 0x27)),
    intensity: None,
    underline: None,
    italic: None,
    strikethrough: None,
};
const DEFAULT_TAB_BAR_INACTIVE_TAB_HOVER_COLORS: NativeTabBarItemColors =
    NativeTabBarItemColors {
        fg_color: Some(Color::Rgb(0xd8, 0xe2, 0xf0)),
        bg_color: Some(Color::Rgb(0x1e, 0x29, 0x3b)),
        intensity: None,
        underline: None,
        italic: None,
        strikethrough: None,
    };
const DEFAULT_TAB_BAR_NEW_TAB_COLORS: NativeTabBarItemColors = NativeTabBarItemColors {
    // Keep the action legible without competing with the cyan brand accent.
    fg_color: Some(Color::Rgb(0xd8, 0xe2, 0xf0)),
    bg_color: Some(Color::Rgb(0x08, 0x0d, 0x18)),
    intensity: None,
    underline: None,
    italic: None,
    strikethrough: None,
};
const DEFAULT_TAB_BAR_NEW_TAB_HOVER_COLORS: NativeTabBarItemColors =
    DEFAULT_TAB_BAR_INACTIVE_TAB_HOVER_COLORS;
// Keep the menu affordance visibly readable beside the high-emphasis '+',
// while remaining quieter than the title controls and tab labels.
const DEFAULT_MODERN_NEW_TAB_CHEVRON_FOREGROUND: Color = Color::Rgb(0xa5, 0xb4, 0xc7);
const DEFAULT_MODERN_TAB_CLOSE_HOVER_FOREGROUND: Color = Color::Rgb(0x0b, 0x12, 0x20);
const DEFAULT_MODERN_TAB_CLOSE_HOVER_BACKGROUND: Color = Color::Rgb(0xf8, 0x71, 0x71);
const DEFAULT_MODERN_WINDOW_BUTTON_HOVER_BACKGROUND: Color = Color::Rgb(0x1e, 0x29, 0x3b);
const DEFAULT_ANSI_PALETTE_COLORS: [Color; 16] = [
    Color::Rgb(0x11, 0x18, 0x27),
    Color::Rgb(0xf8, 0x71, 0x71),
    Color::Rgb(0x4a, 0xde, 0x80),
    Color::Rgb(0xfb, 0xbf, 0x24),
    Color::Rgb(0x60, 0xa5, 0xfa),
    Color::Rgb(0xc0, 0x84, 0xfc),
    Color::Rgb(0x22, 0xd3, 0xee),
    Color::Rgb(0xcb, 0xd5, 0xe1),
    Color::Rgb(0x64, 0x74, 0x8b),
    Color::Rgb(0xfb, 0x71, 0x85),
    Color::Rgb(0x86, 0xef, 0xac),
    Color::Rgb(0xfd, 0xe0, 0x47),
    Color::Rgb(0x93, 0xc5, 0xfd),
    Color::Rgb(0xd8, 0xb4, 0xfe),
    Color::Rgb(0x67, 0xe8, 0xf9),
    Color::Rgb(0xf8, 0xfa, 0xfc),
];
const DEFAULT_CURSOR_STYLE: NativeCursorStyle = NativeCursorStyle::SteadyBlock;
const DEFAULT_CURSOR_THICKNESS: Option<NativeCursorThickness> = None;
const DEFAULT_UNDERLINE_THICKNESS: Option<NativeUnderlineThickness> = None;
const DEFAULT_UNDERLINE_POSITION: Option<NativeUnderlinePosition> = None;
const DEFAULT_STRIKETHROUGH_POSITION: Option<NativeStrikethroughPosition> = None;
const DEFAULT_FORCE_REVERSE_VIDEO_CURSOR: bool = false;
const DEFAULT_WINDOW_PADDING: NativeWindowPadding = NativeWindowPadding {
    left: NativeWindowPaddingDimension::Pixels(8),
    right: NativeWindowPaddingDimension::Pixels(8),
    top: NativeWindowPaddingDimension::Pixels(6),
    bottom: NativeWindowPaddingDimension::Pixels(6),
};
const MODERN_DEFAULT_WINDOW_PADDING: NativeWindowPadding = NativeWindowPadding {
    left: NativeWindowPaddingDimension::Pixels(14),
    right: NativeWindowPaddingDimension::Pixels(14),
    top: NativeWindowPaddingDimension::Pixels(10),
    bottom: NativeWindowPaddingDimension::Pixels(10),
};
const DEFAULT_BOLD_BRIGHTENS_ANSI_COLORS: NativeBoldBrightensAnsiColors =
    NativeBoldBrightensAnsiColors::BrightAndBold;
const DEFAULT_WINDOW_DPI: u32 = 96;
const DEFAULT_TAB_MAX_WIDTH: usize = 16;
const MODERN_DEFAULT_TAB_MAX_WIDTH: usize = 20;
const DEFAULT_TAB_MIN_WIDTH: usize = 8;
const DEFAULT_CLOSED_TAB_HISTORY_SIZE: usize = 25;
const MODERN_FRAME_WIDTH: u32 = TERMINAL_COLUMNS as u32 * MODERN_CELL_WIDTH;
const MODERN_FRAME_HEIGHT: u32 =
    (TERMINAL_ROWS as u32 + TAB_BAR_ROWS as u32) * MODERN_CELL_HEIGHT;
const DEFAULT_TERM: &str = "xterm-256color";
const DEFAULT_ENQ_ANSWERBACK: &str = "";
const DEFAULT_SHOW_CLOSE_TAB_BUTTON_IN_TABS: bool = true;
const DEFAULT_SHOW_NEW_TAB_BUTTON_IN_TAB_BAR: bool = true;
const DEFAULT_SHOW_TAB_INDEX_IN_TAB_BAR: bool = true;
const DEFAULT_SHOW_TABS_IN_TAB_BAR: bool = true;
const DEFAULT_MOUSE_WHEEL_SCROLLS_TABS: bool = true;
const DEFAULT_SWITCH_TO_LAST_ACTIVE_TAB_WHEN_CLOSING_TAB: bool = false;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum NativeTabShortcutStyle {
    #[default]
    Terminal,
    Browser,
}

impl NativeTabShortcutStyle {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "terminal" => Some(Self::Terminal),
            "browser" => Some(Self::Browser),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum NativeTabBarWheelBehavior {
    #[default]
    Scroll,
    Switch,
    Disabled,
}

impl NativeTabBarWheelBehavior {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "scroll" => Some(Self::Scroll),
            "switch" => Some(Self::Switch),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}
const DEFAULT_QUIT_WHEN_ALL_WINDOWS_ARE_CLOSED: bool = true;
const DEFAULT_WINDOW_CLOSE_CONFIRMATION: NativeWindowCloseConfirmation =
    NativeWindowCloseConfirmation::AlwaysPrompt;
const DEFAULT_CONFIRMATION_MESSAGE: &str = " Really continue?";
const DEFAULT_EXIT_BEHAVIOR: NativeExitBehavior = NativeExitBehavior::Close;
const DEFAULT_CLEAN_EXIT_CODES: &[u32] = &[];
const DEFAULT_EXIT_BEHAVIOR_MESSAGING: NativeExitBehaviorMessaging =
    NativeExitBehaviorMessaging::Verbose;
const DEFAULT_SKIP_CLOSE_CONFIRMATION_FOR_PROCESSES_NAMED: &[&str] = &[
    "bash",
    "sh",
    "zsh",
    "fish",
    "tmux",
    "nu",
    "cmd.exe",
    "pwsh.exe",
    "powershell.exe",
];
const DEFAULT_ENABLE_TAB_BAR: bool = true;
const DEFAULT_ENABLE_SCROLL_BAR: bool = false;
const DEFAULT_MIN_SCROLL_BAR_HEIGHT: Option<NativeScrollBarHeight> =
    Some(NativeScrollBarHeight::CellFractionPerMille(500));
const DEFAULT_HIDE_TAB_BAR_IF_ONLY_ONE_TAB: bool = false;
const DEFAULT_USE_FANCY_TAB_BAR: bool = true;
const DEFAULT_UNZOOM_ON_SWITCH_PANE: bool = true;
const DEFAULT_TAB_BAR_AT_BOTTOM: bool = false;
const DEFAULT_TAB_AND_SPLIT_INDICES_ARE_ZERO_BASED: bool = false;
const DEFAULT_SCROLL_TO_BOTTOM_ON_INPUT: bool = true;
const DEFAULT_ENABLE_KITTY_GRAPHICS: bool = true;
const DEFAULT_ENABLE_CHECKSUM_RECTANGULAR_AREA: bool = false;
const DEFAULT_ENABLE_TITLE_REPORTING: bool = false;
const DEFAULT_ENABLE_CSI_U_KEY_ENCODING: bool = false;
const DEFAULT_ENABLE_KITTY_KEYBOARD: bool = false;
const DEFAULT_ALLOW_DOWNLOAD_PROTOCOLS: bool = true;
const DEFAULT_ALLOW_WIN32_INPUT_MODE: bool = true;
const DEFAULT_PALETTE_MAX_KEY_ASSIGMENTS_FOR_ACTION: usize = 1;
const DEFAULT_TREAT_LEFT_CTRLALT_AS_ALTGR: bool = false;
const DEFAULT_SEND_COMPOSED_KEY_WHEN_LEFT_ALT_IS_PRESSED: bool = false;
const DEFAULT_SEND_COMPOSED_KEY_WHEN_RIGHT_ALT_IS_PRESSED: bool = true;
const DEFAULT_TREAT_EAST_ASIAN_AMBIGUOUS_WIDTH_AS_WIDE: bool = false;
const DEFAULT_NORMALIZE_OUTPUT_TO_UNICODE_NFC: bool = false;
const DEFAULT_UNICODE_VERSION: u32 = 9;
const DEFAULT_BIDI_ENABLED: bool = false;
const DEFAULT_BIDI_DIRECTION: NativeBidiDirection = NativeBidiDirection::LeftToRight;
const DEFAULT_USE_IME: bool = true;
const DEFAULT_USE_DEAD_KEYS: bool = true;
const DEFAULT_IME_PREEDIT_RENDERING: NativeImePreeditRendering = NativeImePreeditRendering::Builtin;
const DEFAULT_MACOS_FORWARD_TO_IME_MODIFIER_MASK: ModifiersState = ModifiersState::SHIFT;
const DEFAULT_UI_KEY_CAP_RENDERING: NativeUiKeyCapRendering = if cfg!(target_os = "macos") {
    NativeUiKeyCapRendering::AppleSymbols
} else if cfg!(target_os = "windows") {
    NativeUiKeyCapRendering::WindowsLong
} else {
    NativeUiKeyCapRendering::UnixLong
};
const DEFAULT_DETECT_PASSWORD_INPUT: bool = true;
const DEFAULT_CANONICALIZE_PASTED_NEWLINES: NativeCanonicalizePastedNewlines = if cfg!(windows) {
    NativeCanonicalizePastedNewlines::CarriageReturnAndLineFeed
} else {
    NativeCanonicalizePastedNewlines::CarriageReturn
};
const DEFAULT_QUOTE_DROPPED_FILES: NativeQuoteDroppedFiles = if cfg!(windows) {
    NativeQuoteDroppedFiles::Windows
} else {
    NativeQuoteDroppedFiles::SpacesOnly
};
const DEFAULT_DISABLE_DEFAULT_KEY_BINDINGS: bool = false;
const DEFAULT_DISABLE_DEFAULT_MOUSE_BINDINGS: bool = false;
const DEFAULT_HIDE_MOUSE_CURSOR_WHEN_TYPING: bool = true;
const DEFAULT_ALTERNATE_BUFFER_WHEEL_SCROLL_SPEED: usize = 3;
const DEFAULT_ADJUST_WINDOW_SIZE_WHEN_CHANGING_FONT_SIZE: bool = true;
const DEFAULT_PANE_FOCUS_FOLLOWS_MOUSE: bool = false;
const DEFAULT_SWALLOW_MOUSE_CLICK_ON_PANE_FOCUS: bool = false;
const DEFAULT_SWALLOW_MOUSE_CLICK_ON_WINDOW_FOCUS: bool = cfg!(target_os = "macos");
const DEFAULT_BYPASS_MOUSE_REPORTING_MODIFIERS: ModifiersState = ModifiersState::SHIFT;
const DEFAULT_INACTIVE_PANE_HSB: NativeInactivePaneHsb = NativeInactivePaneHsb {
    hue: NativeHsbMultiplier::ONE,
    saturation: NativeHsbMultiplier::from_per_mille(900),
    brightness: NativeHsbMultiplier::from_per_mille(800),
};
const DEFAULT_FOREGROUND_TEXT_HSB: NativeInactivePaneHsb = NativeInactivePaneHsb {
    hue: NativeHsbMultiplier::ONE,
    saturation: NativeHsbMultiplier::ONE,
    brightness: NativeHsbMultiplier::ONE,
};
const DEFAULT_TEXT_BACKGROUND_OPACITY: NativeTextBackgroundOpacity =
    NativeTextBackgroundOpacity::ONE;
const DEFAULT_WINDOW_BACKGROUND_OPACITY: NativeTextBackgroundOpacity =
    NativeTextBackgroundOpacity::ONE;
const DEFAULT_KDE_WINDOW_BACKGROUND_BLUR: bool = false;
const DEFAULT_MACOS_WINDOW_BACKGROUND_BLUR: u32 = 0;
const DEFAULT_WIN32_SYSTEM_BACKDROP: NativeWin32SystemBackdrop = NativeWin32SystemBackdrop::Auto;
const DEFAULT_REVERSE_VIDEO_CURSOR_MIN_CONTRAST: NativeContrastRatio =
    NativeContrastRatio::from_centi(250);
#[cfg(target_os = "windows")]
const DEFAULT_WINDOW_DECORATIONS: NativeWindowDecorations = NativeWindowDecorations {
    // Keep the title bar inside the terminal surface on Windows so the tab
    // row, workspace label, and window controls share one visual hierarchy.
    // The existing `StartWindowDrag` action remains available for custom
    // bindings and the integrated tab bar handles its own hit testing.
    title: false,
    resize: true,
    integrated_buttons: true,
    macos_force_disable_shadow: false,
    macos_force_enable_shadow: false,
    macos_force_square_corners: false,
    macos_use_background_color_as_titlebar_color: false,
};
#[cfg(target_os = "macos")]
const DEFAULT_WINDOW_DECORATIONS: NativeWindowDecorations = NativeWindowDecorations {
    // Keep AppKit's traffic-light controls and resize/shadow behavior, but
    // extend the renderer into a transparent title bar so the tab strip is
    // the only visible top-level chrome.
    title: true,
    resize: true,
    integrated_buttons: true,
    macos_force_disable_shadow: false,
    macos_force_enable_shadow: false,
    macos_force_square_corners: false,
    macos_use_background_color_as_titlebar_color: true,
};
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const DEFAULT_WINDOW_DECORATIONS: NativeWindowDecorations = NativeWindowDecorations {
    title: true,
    resize: true,
    integrated_buttons: false,
    macos_force_disable_shadow: false,
    macos_force_enable_shadow: false,
    macos_force_square_corners: false,
    macos_use_background_color_as_titlebar_color: false,
};
const DEFAULT_INTEGRATED_TITLE_BUTTON_ALIGNMENT: NativeIntegratedTitleButtonAlignment =
    NativeIntegratedTitleButtonAlignment::Right;
const DEFAULT_INTEGRATED_TITLE_BUTTON_COLOR: NativeIntegratedTitleButtonColor =
    NativeIntegratedTitleButtonColor::Auto;
#[cfg(target_os = "macos")]
const DEFAULT_INTEGRATED_TITLE_BUTTON_STYLE: NativeIntegratedTitleButtonStyle =
    NativeIntegratedTitleButtonStyle::MacOsNative;
#[cfg(not(target_os = "macos"))]
const DEFAULT_INTEGRATED_TITLE_BUTTON_STYLE: NativeIntegratedTitleButtonStyle =
    NativeIntegratedTitleButtonStyle::Windows;
const DEFAULT_QUICK_SELECT_ALPHABET: &str = "asdfqwerzxcvjklmiuopghtybn";
const DEFAULT_LAUNCHER_ALPHABET: &str = "1234567890abcdefghilmnopqrstuvwxyz";
const DEFAULT_LAUNCHER_HELP_TEXT: &str =
    "Select an item and press Enter=launch Esc=cancel /=filter";
const DEFAULT_LAUNCHER_FUZZY_HELP_TEXT: &str = "Fuzzy matching: ";
const DEFAULT_QUICK_SELECT_SCOPE_LINES: usize = 1_000;
const DEFAULT_SELECTION_WORD_BOUNDARY: &str = " \t\n{}[]()\"'`";
const DEFAULT_AUDIBLE_BELL: NativeAudibleBell = NativeAudibleBell::SystemBeep;
const DEFAULT_NOTIFICATION_HANDLING: NativeNotificationHandling =
    NativeNotificationHandling::AlwaysShow;
const RECENTLY_USED_CHAR_SELECT_GROUP: &str = "RecentlyUsed";
const CHAR_SELECT_GROUPS: &[&str] = &[
    RECENTLY_USED_CHAR_SELECT_GROUP,
    "SmileysAndEmotion",
    "PeopleAndBody",
    "AnimalsAndNature",
    "FoodAndDrink",
    "TravelAndPlaces",
    "Activities",
    "Objects",
    "Symbols",
    "Flags",
    "NerdFonts",
    "UnicodeNames",
];
const DEFAULT_CHAR_SELECT_GROUP: &str = "SmileysAndEmotion";
const CHAR_SELECT_VISIBLE_ROWS: usize = 5;
const CHAR_SELECT_MATCH_LIMIT: usize = 64;
const SMILEYS_AND_EMOTION_CHAR_SELECT_CANDIDATES: &[char] =
    &['😀', '😃', '😄', '😁', '😆', '😅', '😂', '🙂', '😉', '😍'];
const PEOPLE_AND_BODY_CHAR_SELECT_CANDIDATES: &[char] =
    &['👋', '🤚', '🖐', '✋', '🖖', '👍', '👎', '👏', '🙌', '🧑'];
const ANIMALS_AND_NATURE_CHAR_SELECT_CANDIDATES: &[char] =
    &['🐶', '🐱', '🐭', '🐹', '🐰', '🦊', '🐻', '🐼', '🐨', '🐯'];
const FOOD_AND_DRINK_CHAR_SELECT_CANDIDATES: &[char] =
    &['🍎', '🍐', '🍊', '🍋', '🍌', '🍉', '🍇', '🍓', '🥐', '🍞'];
const TRAVEL_AND_PLACES_CHAR_SELECT_CANDIDATES: &[char] =
    &['🚗', '🚕', '🚙', '🚌', '🚎', '🏎', '🚓', '🚑', '🚒', '✈'];
const ACTIVITIES_CHAR_SELECT_CANDIDATES: &[char] =
    &['⚽', '🏀', '🏈', '⚾', '🎾', '🏐', '🏉', '🎱', '🏓', '🏸'];
const OBJECTS_CHAR_SELECT_CANDIDATES: &[char] =
    &['⌚', '📱', '💻', '⌨', '🖥', '🖨', '🖱', '💡', '🔦', '📚'];
const SYMBOLS_CHAR_SELECT_CANDIDATES: &[char] =
    &['❤', '⭐', '✅', '☑', '☢', '☮', '☯', '♈', '♉', '♊'];
const FLAGS_CHAR_SELECT_CANDIDATES: &[char] = &['⚐', '⚑', '🏁', '🚩', '🏳', '🏴'];
const NERD_FONTS_CHAR_SELECT_CANDIDATES: &[(char, &str)] = &[
    ('\u{f09b}', "NF-FA-GITHUB"),
    ('\u{f017}', "NF-FA-CLOCK_O"),
    ('\u{f120}', "NF-FA-TERMINAL"),
    ('\u{f121}', "NF-FA-CODE"),
    ('\u{f126}', "NF-FA-CODE_FORK"),
    ('\u{f1c9}', "NF-FA-FILE_CODE_O"),
    ('\u{ea84}', "NF-COD-GITHUB"),
    ('\u{f034b}', "NF-MD-MAGNIFY_PLUS"),
    ('\u{f0455}', "NF-MD-RENAME_BOX"),
    ('\u{e795}', "NF-DEV-RUST"),
    ('\u{e718}', "NF-DEV-JAVASCRIPT"),
    ('\u{e73c}', "NF-DEV-PYTHON"),
];
const UNICODE_NAMES_CHAR_SELECT_CANDIDATES: &[char] =
    &['A', 'B', 'C', '0', '1', 'α', 'β', 'Ω', '中', '字'];

pub fn run(
    options: &WindowOptions,
    composition: RuntimeComposition,
) -> Result<(), Box<dyn Error>> {
    if options.state || options.state_json {
        return run_configured_window_state_report(options);
    }

    let ConfiguredStartupApp {
        mut app,
        mut lifecycle,
    } = configured_startup_app(options, ConfigDiscoveryInputs::capture_current_process())?;
    app.runtime.set_composition(composition);
    if let Some(diagnostic) = lifecycle.latest_diagnostic() {
        eprintln!("failed to load WezTerm config: {diagnostic}");
    }

    let event_loop = EventLoop::<WindowUserEvent>::with_user_event().build()?;
    let event_proxy = event_loop.create_proxy();
    let config_event_proxy = event_proxy.clone();
    crate::stage7_attribution::audit_product_service_start(
        crate::stage7_attribution::ProductServiceEntry::ConfigWatcher,
    )?;
    if let Err(diagnostic) = lifecycle.install_watcher_sink(Arc::new(move || {
        config_event_proxy
            .send_event(WindowUserEvent::ConfigFileChanged)
            .is_ok()
    })) {
        eprintln!(
            "failed to initialize WezTerm config watcher: {}",
            diagnostic.detail
        );
    }
    let reload_event_proxy = event_proxy.clone();
    app.session_log = match &options.log {
        Some(path) => Some(Box::new(File::create(path)?) as Box<dyn Write + Send>),
        None => None,
    };
    app.reload_request_sender = Some(Arc::new(move |event| {
        reload_event_proxy.send_event(event).is_ok()
    }));
    app.event_proxy = Some(event_proxy);
    app.set_command_palette_frecency_path(default_command_palette_frecency_path());
    app.set_char_select_recently_used_path(default_char_select_recently_used_path());
    let mut app = NativeWindowManager::new(app).with_config_lifecycle(lifecycle);

    event_loop.run_app(&mut app)?;
    app.shutdown_runtime_owners();
    app.reap_retired_apps();
    if options.metrics_json {
        println!("{}", app.metrics_json_report()?);
    } else if options.metrics {
        print!("{}", app.metrics_report());
    }

    Ok(())
}

/// Starts an SSH target in the native GUI without doing configuration or GPU
/// work on the CLI thread.  The app is intentionally created from the small
/// default projection first; its CPU bootstrap frame is presented before the
/// SSH transport (and any future deferred configuration work) is started.
#[cfg(feature = "ssh")]
pub fn run_ssh_gui(
    options: &SshOptions,
    process_started_at: Instant,
) -> Result<(), Box<dyn Error>> {
    let launch = pane_launch_from_ssh_options(options);
    let mut app = NativeWindowApp::new_with_workspace_class_position_and_osc52_policy(
        None,
        options.osc52_policy,
        PtyCommand::default_shell(),
        None,
        None,
        None,
    );
    configure_ssh_gui_initial_size(&mut app, options);
    app.metrics.startup_trace = StartupTrace::from_process_started_at(process_started_at);
    app.set_initial_pane_launch(launch);
    app.set_renderer_mode(if options.benchmark_startup {
        RendererMode::Cpu
    } else {
        options.renderer
    });
    app.set_benchmark_startup(options.benchmark_startup);
    let product_markers = install_product_gui_probe(&mut app, options, process_started_at)?;
    if let Some(path) = &options.log {
        app.session_log = Some(Box::new(File::create(path)?) as Box<dyn Write + Send>);
    }

    let event_loop = EventLoop::<WindowUserEvent>::with_user_event().build()?;
    let event_proxy = event_loop.create_proxy();
    app.event_proxy = Some(event_proxy.clone());
    if product_markers.is_some() {
        spawn_diagnostic_stdin_shutdown_listener(event_proxy)?;
    }
    app.set_command_palette_frecency_path(default_command_palette_frecency_path());
    app.set_char_select_recently_used_path(default_char_select_recently_used_path());
    let cli = validate_cli_config_overrides(&[])?;
    let lifecycle = Box::new(NativeConfigLifecycle::new(
        ConfigDiscoveryInputs::capture_current_process(),
        false,
        None,
        cli,
    ));
    let mut manager = NativeWindowManager::new(app)
        .with_config_lifecycle(lifecycle)
        .with_deferred_config();
    event_loop.run_app(&mut manager)?;
    if let Some(markers) = product_markers {
        manager.shutdown_runtime_owners();
        manager.reap_retired_apps();
        if !markers.emit(DiagnosticMarkerKind::ProcessExited, None, None)? {
            return Err(io::Error::other("product probe process_exited marker was not unique").into());
        }
    }
    if options.console.metrics_json {
        println!("{}", manager.metrics_json_report()?);
    } else if options.console.metrics {
        print!("{}", manager.metrics_report());
    }
    Ok(())
}


#[cfg(feature = "ssh")]
fn configure_ssh_gui_initial_size(app: &mut NativeWindowApp, options: &SshOptions) {
    let size = match &options.target {
        SshTarget::Direct(request) => request.config.initial_size.into(),
        SshTarget::OpenSsh(target) => target.initial_size,
    };
    app.initial_cols = size.columns;
    app.initial_rows = size.rows;
    *app.runtime = TerminalRuntime::new(size);
    app.snapshot = terminal_runtime_snapshot(&app.runtime, PaneStableViewport::default());
    let frame_size = app.initial_frame_size();
    app.frame_width = frame_size.width;
    app.frame_height = frame_size.height;
    app.window_frame.set_size(frame_size);
}

#[cfg(feature = "ssh")]
fn pane_launch_from_ssh_options(options: &SshOptions) -> PaneLaunch {
    let (target, auth, kind) = match &options.target {
        SshTarget::Direct(request) => (
            format_ssh_gui_target(
                &request.config.username,
                &request.config.host,
                request.config.port,
            ),
            ssh_auth_description(&request.auth),
            SshTargetKind::Direct,
        ),
        SshTarget::OpenSsh(target) => (
            target.target.clone(),
            ssh_auth_description(&target.auth),
            SshTargetKind::OpenSsh,
        ),
    };
    let policy = match options.native_host_key_policy {
        NativeHostKeyPolicy::RejectUnknown => SshKnownHostsPolicy::Prompt,
        NativeHostKeyPolicy::TrustOnFirstUse => SshKnownHostsPolicy::TrustOnFirstUse,
        NativeHostKeyPolicy::AcceptUnknown => SshKnownHostsPolicy::AcceptUnknown,
    };
    let launch = match (&options.target, kind) {
        (SshTarget::Direct(_), SshTargetKind::Direct) => SshPaneLaunch::new(target, auth, policy),
        (SshTarget::OpenSsh(target), SshTargetKind::OpenSsh) => {
            SshPaneLaunch::openssh(target.target.clone(), auth, policy)
                .with_target_overrides(target.username.clone(), target.port)
        }
        _ => unreachable!("SSH target kind must match the parsed target"),
    };
    PaneLaunch::ssh(
        launch.with_remote_command(options.remote_command.clone()),
    )
}

#[cfg(feature = "ssh")]
fn format_ssh_gui_target(user: &str, host: &str, port: u16) -> String {
    let host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    if user.is_empty() {
        format!("{host}:{port}")
    } else {
        format!("{user}@{host}:{port}")
    }
}

#[cfg(feature = "ssh")]
fn ssh_auth_description(auth: &SshAuthMethod) -> SshAuthDescription {
    match auth {
        SshAuthMethod::Agent => SshAuthDescription::Agent,
        SshAuthMethod::PasswordPrompt | SshAuthMethod::Password { .. } => {
            SshAuthDescription::PasswordPrompt
        }
        SshAuthMethod::PrivateKey { path, .. } => SshAuthDescription::PrivateKey {
            path: path.to_string_lossy().into_owned(),
        },
    }
}

#[derive(Debug)]
struct ConfiguredWindowStateReport {
    diagnostic: Option<String>,
    format: window_state_report::WindowStateFormat,
    report: String,
}

const WINDOW_STATE_REPORT_STACK_SIZE: usize = 8 * 1024 * 1024;

fn run_configured_window_state_report(options: &WindowOptions) -> Result<(), Box<dyn Error>> {
    let options = options.clone();
    // Keep the report-only path reliable on the smaller Windows main-thread
    // stack while configuration and the native app projection are materialized.
    let spawned = thread::Builder::new()
        .name("rssh-window-state-report".to_owned())
        .stack_size(WINDOW_STATE_REPORT_STACK_SIZE)
        .spawn(move || configured_window_state_report(&options));
    let output =
        resolve_configured_window_state_report_thread(spawned.map(std::thread::JoinHandle::join))?;

    if let Some(diagnostic) = &output.diagnostic {
        eprintln!("failed to load WezTerm config: {diagnostic}");
    }
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_configured_window_state_report(&output, &mut stdout)?;
    Ok(())
}

fn resolve_configured_window_state_report_thread(
    result: io::Result<thread::Result<Result<ConfiguredWindowStateReport, String>>>,
) -> io::Result<ConfiguredWindowStateReport> {
    let joined = result.map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to start state reporter: {error}"),
        )
    })?;
    let generated = joined.map_err(|_| io::Error::other("window state reporter panicked"))?;
    generated.map_err(io::Error::other)
}

fn write_configured_window_state_report(
    output: &ConfiguredWindowStateReport,
    writer: &mut impl Write,
) -> io::Result<()> {
    let mut bytes = output.report.as_bytes().to_vec();
    if output.format == window_state_report::WindowStateFormat::Json {
        bytes.push(b'\n');
    }
    writer.write_all(&bytes)?;
    writer.flush()
}

fn configured_window_state_report(
    options: &WindowOptions,
) -> Result<ConfiguredWindowStateReport, String> {
    let ConfiguredStartupApp { app, lifecycle } =
        configured_startup_app(options, ConfigDiscoveryInputs::capture_current_process())
            .map_err(|error| error.to_string())?;
    let diagnostic = lifecycle.latest_diagnostic().map(ToString::to_string);
    let Some((format, report)) = window_state_report::render_requested_window_state(options, &app)
        .map_err(|error| error.to_string())?
    else {
        return Err("window state report was not requested".to_owned());
    };
    Ok(ConfiguredWindowStateReport {
        diagnostic,
        format,
        report,
    })
}

#[cfg(test)]
pub fn demo_snapshot() -> TerminalRenderSnapshot {
    let mut terminal = Terminal::new(TerminalSize::new(TERMINAL_COLUMNS, TERMINAL_ROWS));
    terminal.feed(b"\x1b[1;32mR-SSH\x1b[0m native renderer\r\n");
    terminal.feed(b"winit window + renderer terminal grid");

    TerminalRenderSnapshot::from_terminal(&terminal)
}

#[cfg(target_os = "windows")]
fn window_attributes_with_class(
    attributes: WindowAttributes,
    window_class: Option<&str>,
) -> WindowAttributes {
    match window_class {
        Some(window_class) => attributes.with_class_name(window_class.to_owned()),
        None => attributes,
    }
}

#[cfg(not(target_os = "windows"))]
fn window_attributes_with_class(
    attributes: WindowAttributes,
    window_class: Option<&str>,
) -> WindowAttributes {
    let _ = window_class;
    attributes
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeMonitorPosition {
    name: Option<String>,
    position: PhysicalPosition<i32>,
}

fn resolve_initial_window_position(
    position: &WindowPosition,
    primary_monitor_position: Option<PhysicalPosition<i32>>,
    active_monitor_position: Option<PhysicalPosition<i32>>,
    monitor_positions: &[NativeMonitorPosition],
) -> Option<PhysicalPosition<i32>> {
    match &position.origin {
        WindowPositionOrigin::Screen => Some(PhysicalPosition::new(position.x, position.y)),
        WindowPositionOrigin::Main => match primary_monitor_position {
            Some(origin) => Some(PhysicalPosition::new(
                origin.x + position.x,
                origin.y + position.y,
            )),
            None => Some(PhysicalPosition::new(position.x, position.y)),
        },
        WindowPositionOrigin::Active => {
            match active_monitor_position.or(primary_monitor_position) {
                Some(origin) => Some(PhysicalPosition::new(
                    origin.x + position.x,
                    origin.y + position.y,
                )),
                None => Some(PhysicalPosition::new(position.x, position.y)),
            }
        }
        WindowPositionOrigin::Monitor(name) => monitor_positions
            .iter()
            .find(|monitor| monitor.name.as_deref() == Some(name.as_str()))
            .map(|monitor| {
                PhysicalPosition::new(
                    monitor.position.x + position.x,
                    monitor.position.y + position.y,
                )
            }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowUserVarChange {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
    name: String,
    value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeWindowBell {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeAudibleBell {
    SystemBeep,
    Disabled,
}

impl NativeAudibleBell {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "SystemBeep" => Some(Self::SystemBeep),
            "Disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::SystemBeep => "SystemBeep",
            Self::Disabled => "Disabled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeRenderFrontEnd {
    OpenGl,
    Software,
    WebGpu,
}

impl NativeRenderFrontEnd {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "OpenGL" => Some(Self::OpenGl),
            "Software" => Some(Self::Software),
            "WebGpu" => Some(Self::WebGpu),
            _ => None,
        }
    }

    fn as_wezterm_config_str(self) -> &'static str {
        match self {
            Self::OpenGl => "OpenGL",
            Self::Software => "Software",
            Self::WebGpu => "WebGpu",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeSshBackend {
    Ssh2,
    LibSsh,
}

impl NativeSshBackend {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Ssh2" => Some(Self::Ssh2),
            "LibSsh" => Some(Self::LibSsh),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeWebGpuPowerPreference {
    LowPower,
    HighPerformance,
}

impl NativeWebGpuPowerPreference {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "LowPower" => Some(Self::LowPower),
            "HighPerformance" => Some(Self::HighPerformance),
            _ => None,
        }
    }

    fn as_wezterm_config_str(self) -> &'static str {
        match self {
            Self::LowPower => "LowPower",
            Self::HighPerformance => "HighPerformance",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeWebGpuPreferredAdapter {
    backend: Option<String>,
    device: Option<u32>,
    device_type: Option<String>,
    driver: Option<String>,
    driver_info: Option<String>,
    name: Option<String>,
    vendor: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeCellWidth(u16);

impl NativeCellWidth {
    const fn from_per_mille(per_mille: u16) -> Self {
        Self(per_mille)
    }

    fn as_f64(self) -> f64 {
        f64::from(self.0) / 1_000.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeCellWidthOverride {
    first: u32,
    last: u32,
    width: u16,
}

impl NativeCellWidthOverride {
    const fn new(first: u32, last: u32, width: u16) -> Self {
        Self { first, last, width }
    }

    const fn to_terminal(self) -> CellWidthOverride {
        CellWidthOverride::new(self.first, self.last, self.width)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeLineHeight(u16);

impl NativeLineHeight {
    const fn from_per_mille(per_mille: u16) -> Self {
        Self(per_mille)
    }

    fn as_f64(self) -> f64 {
        f64::from(self.0) / 1_000.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFontAntialias {
    None,
    Greyscale,
    Subpixel,
}

impl NativeFontAntialias {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "None" => Some(Self::None),
            "Greyscale" => Some(Self::Greyscale),
            "Subpixel" => Some(Self::Subpixel),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Greyscale => "Greyscale",
            Self::Subpixel => "Subpixel",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFontHinting {
    None,
    Vertical,
    VerticalSubpixel,
    Full,
}

impl NativeFontHinting {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "None" => Some(Self::None),
            "Vertical" => Some(Self::Vertical),
            "VerticalSubpixel" => Some(Self::VerticalSubpixel),
            "Full" => Some(Self::Full),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Vertical => "Vertical",
            Self::VerticalSubpixel => "VerticalSubpixel",
            Self::Full => "Full",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFontRasterizer {
    FreeType,
    Harfbuzz,
}

impl NativeFontRasterizer {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "FreeType" => Some(Self::FreeType),
            "Harfbuzz" => Some(Self::Harfbuzz),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::FreeType => "FreeType",
            Self::Harfbuzz => "Harfbuzz",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFontShaper {
    Harfbuzz,
}

impl NativeFontShaper {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Harfbuzz" => Some(Self::Harfbuzz),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::Harfbuzz => "Harfbuzz",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFontLocator {
    ConfigDirsOnly,
}

impl NativeFontLocator {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "ConfigDirsOnly" => Some(Self::ConfigDirsOnly),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::ConfigDirsOnly => "ConfigDirsOnly",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeImePreeditRendering {
    Builtin,
    System,
}

impl NativeImePreeditRendering {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Builtin" => Some(Self::Builtin),
            "System" => Some(Self::System),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeBidiDirection {
    LeftToRight,
    RightToLeft,
    AutoLeftToRight,
    AutoRightToLeft,
}

impl NativeBidiDirection {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "LeftToRight" => Some(Self::LeftToRight),
            "RightToLeft" => Some(Self::RightToLeft),
            "AutoLeftToRight" => Some(Self::AutoLeftToRight),
            "AutoRightToLeft" => Some(Self::AutoRightToLeft),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::LeftToRight => "LeftToRight",
            Self::RightToLeft => "RightToLeft",
            Self::AutoLeftToRight => "AutoLeftToRight",
            Self::AutoRightToLeft => "AutoRightToLeft",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeSquareGlyphOverflow {
    WhenFollowedBySpace,
    Always,
    Never,
}

impl NativeSquareGlyphOverflow {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "WhenFollowedBySpace" => Some(Self::WhenFollowedBySpace),
            "Always" => Some(Self::Always),
            "Never" => Some(Self::Never),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::WhenFollowedBySpace => "WhenFollowedBySpace",
            Self::Always => "Always",
            Self::Never => "Never",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFreetypeTarget {
    Normal,
    Light,
    Mono,
    HorizontalLcd,
    VerticalLcd,
}

impl NativeFreetypeTarget {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Normal" => Some(Self::Normal),
            "Light" => Some(Self::Light),
            "Mono" => Some(Self::Mono),
            "HorizontalLcd" => Some(Self::HorizontalLcd),
            "VerticalLcd" => Some(Self::VerticalLcd),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Light => "Light",
            Self::Mono => "Mono",
            Self::HorizontalLcd => "HorizontalLcd",
            Self::VerticalLcd => "VerticalLcd",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeDisplayPixelGeometry {
    Rgb,
    Bgr,
}

impl NativeDisplayPixelGeometry {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "RGB" => Some(Self::Rgb),
            "BGR" => Some(Self::Bgr),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::Rgb => "RGB",
            Self::Bgr => "BGR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeFreetypeLoadFlags(u8);

impl NativeFreetypeLoadFlags {
    const DEFAULT: Self = Self(0);
    const NO_HINTING: Self = Self(1 << 0);
    const NO_BITMAP: Self = Self(1 << 1);
    const FORCE_AUTOHINT: Self = Self(1 << 2);
    const MONOCHROME: Self = Self(1 << 3);
    const NO_AUTOHINT: Self = Self(1 << 4);

    const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn parse(value: &str) -> Option<Self> {
        let mut flags = Self::DEFAULT;
        let mut saw_flag = false;

        for flag in value.split('|').map(str::trim) {
            if flag.is_empty() {
                return None;
            }
            flags = flags.union(match flag {
                "DEFAULT" => Self::DEFAULT,
                "NO_HINTING" => Self::NO_HINTING,
                "NO_BITMAP" => Self::NO_BITMAP,
                "FORCE_AUTOHINT" => Self::FORCE_AUTOHINT,
                "MONOCHROME" => Self::MONOCHROME,
                "NO_AUTOHINT" => Self::NO_AUTOHINT,
                _ => return None,
            });
            saw_flag = true;
        }

        saw_flag.then_some(flags)
    }

    fn config_text(self) -> String {
        if self == Self::DEFAULT {
            return "DEFAULT".to_owned();
        }

        [
            (Self::NO_HINTING, "NO_HINTING"),
            (Self::NO_BITMAP, "NO_BITMAP"),
            (Self::FORCE_AUTOHINT, "FORCE_AUTOHINT"),
            (Self::MONOCHROME, "MONOCHROME"),
            (Self::NO_AUTOHINT, "NO_AUTOHINT"),
        ]
        .into_iter()
        .filter_map(|(flag, name)| ((self.0 & flag.0) != 0).then_some(name))
        .collect::<Vec<_>>()
        .join("|")
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeWindowPadding {
    left: NativeWindowPaddingDimension,
    right: NativeWindowPaddingDimension,
    top: NativeWindowPaddingDimension,
    bottom: NativeWindowPaddingDimension,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct NativeWindowPaddingPixels {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

impl NativeWindowPaddingPixels {
    const fn horizontal(self) -> u32 {
        self.left.saturating_add(self.right)
    }

    const fn vertical(self) -> u32 {
        self.top.saturating_add(self.bottom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeWindowPaddingDimension {
    Pixels(u32),
    Points(u32),
    Percent(u32),
    CellFractionPerMille(u32),
}

impl Default for NativeWindowPaddingDimension {
    fn default() -> Self {
        Self::Pixels(0)
    }
}

impl NativeWindowPaddingDimension {
    fn parse(value: &str) -> Option<Self> {
        parse_native_unsigned_dimension(
            value,
            Self::Pixels,
            Self::Points,
            Self::Percent,
            Self::CellFractionPerMille,
        )
    }

    fn config_text(self) -> String {
        match self {
            Self::Pixels(pixels) => format!("{pixels}px"),
            Self::Points(points) => format!("{points}pt"),
            Self::Percent(percent) => format!("{percent}%"),
            Self::CellFractionPerMille(per_mille) => {
                format!("{}cell", native_per_mille_decimal_text(per_mille))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeTextBackgroundOpacity(u16);

impl NativeTextBackgroundOpacity {
    const ONE: Self = Self::from_per_mille(1_000);

    const fn from_per_mille(per_mille: u16) -> Self {
        Self(per_mille)
    }

    #[cfg(test)]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn from_f32(value: f32) -> Self {
        if !value.is_finite() {
            return Self::ONE;
        }
        let per_mille = (value.clamp(0.0, 1.0) * 1_000.0).round();
        Self(per_mille as u16)
    }

    fn as_alpha(self) -> u8 {
        opacity_alpha(f64::from(self.0.min(1_000)) / 1_000.0)
    }

    fn config_text(self) -> String {
        native_per_mille_decimal_text(u32::from(self.0.min(1_000)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeWindowBackgroundGradientOrientation {
    Horizontal,
    Vertical,
    Linear {
        angle_millidegrees: i32,
    },
    Radial {
        cx_millis: u32,
        cy_millis: u32,
        radius_millis: u32,
    },
}

impl NativeWindowBackgroundGradientOrientation {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Horizontal" => Some(Self::Horizontal),
            "Vertical" => Some(Self::Vertical),
            _ => None,
        }
    }

    fn parse_lua_value(source: &str, value: &str, max_start: usize) -> Option<Self> {
        let value = value.trim();
        if value.starts_with('{') {
            return Self::parse_lua_table(source, value, max_start);
        }

        let value = lua_static_string_assignment_value_from_query(source, value)
            .and_then(parse_maybe_quoted_query_text)?;
        Self::parse(&value)
    }

    fn parse_lua_table(source: &str, value: &str, max_start: usize) -> Option<Self> {
        let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
        for field in split_lua_table_top_level_fields(table)? {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
                continue;
            };
            let key = split_lua_table_key_from_query(key.trim())?;
            if key == "Linear" {
                return Self::parse_linear_lua_table(source, value.trim(), max_start);
            }
            if key == "Radial" {
                return Self::parse_radial_lua_table(source, value.trim(), max_start);
            }
        }
        None
    }

    fn parse_linear_lua_table(source: &str, value: &str, max_start: usize) -> Option<Self> {
        let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
        let static_source = Some(LuaStaticSource { source, max_start });
        let mut angle_millidegrees = 0;

        for field in split_lua_table_top_level_fields(table)? {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
                continue;
            };
            let key = split_lua_table_key_from_query(key.trim())?;
            if key == "angle" {
                let angle = parse_maybe_static_query_f64(static_source, value.trim())?;
                angle_millidegrees = native_gradient_angle_millidegrees_from_f64(angle)?;
            }
        }

        Some(Self::Linear { angle_millidegrees })
    }

    #[expect(
        clippy::similar_names,
        reason = "singular and plural names mirror distinct compatibility API parameters"
    )]
    fn parse_radial_lua_table(source: &str, value: &str, max_start: usize) -> Option<Self> {
        let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
        let static_source = Some(LuaStaticSource { source, max_start });
        let mut cx_millis = 500;
        let mut cy_millis = 500;
        let mut radius_millis = 500;

        for field in split_lua_table_top_level_fields(table)? {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
                continue;
            };
            let key = split_lua_table_key_from_query(key.trim())?;
            let value = value.trim();
            match key.as_str() {
                "cx" => {
                    let cx = parse_maybe_static_query_f64(static_source, value)?;
                    cx_millis = native_gradient_unit_interval_millis_from_f64(cx)?;
                }
                "cy" => {
                    let cy = parse_maybe_static_query_f64(static_source, value)?;
                    cy_millis = native_gradient_unit_interval_millis_from_f64(cy)?;
                }
                "radius" => {
                    let radius = parse_maybe_static_query_f64(static_source, value)?;
                    radius_millis = native_gradient_positive_millis_from_f64(radius)?;
                }
                _ => {}
            }
        }

        Some(Self::Radial {
            cx_millis,
            cy_millis,
            radius_millis,
        })
    }

    const fn to_render(self) -> RenderBackgroundGradientOrientation {
        match self {
            Self::Horizontal => RenderBackgroundGradientOrientation::Horizontal,
            Self::Vertical => RenderBackgroundGradientOrientation::Vertical,
            Self::Linear { angle_millidegrees } => {
                RenderBackgroundGradientOrientation::Linear { angle_millidegrees }
            }
            Self::Radial {
                cx_millis,
                cy_millis,
                radius_millis,
            } => RenderBackgroundGradientOrientation::Radial {
                cx_millis,
                cy_millis,
                radius_millis,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeWindowBackgroundGradientInterpolation {
    Linear,
    Basis,
    CatmullRom,
}

impl NativeWindowBackgroundGradientInterpolation {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Linear" => Some(Self::Linear),
            "Basis" => Some(Self::Basis),
            "CatmullRom" => Some(Self::CatmullRom),
            _ => None,
        }
    }

    const fn to_render(self) -> RenderBackgroundGradientInterpolation {
        match self {
            Self::Linear => RenderBackgroundGradientInterpolation::Linear,
            Self::Basis => RenderBackgroundGradientInterpolation::Basis,
            Self::CatmullRom => RenderBackgroundGradientInterpolation::CatmullRom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeWindowBackgroundGradientBlend {
    Rgb,
    LinearRgb,
    Hsv,
    Oklab,
}

impl NativeWindowBackgroundGradientBlend {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Rgb" => Some(Self::Rgb),
            "LinearRgb" => Some(Self::LinearRgb),
            "Hsv" => Some(Self::Hsv),
            "Oklab" => Some(Self::Oklab),
            _ => None,
        }
    }

    const fn to_render(self) -> RenderBackgroundGradientBlend {
        match self {
            Self::Rgb => RenderBackgroundGradientBlend::Rgb,
            Self::LinearRgb => RenderBackgroundGradientBlend::LinearRgb,
            Self::Hsv => RenderBackgroundGradientBlend::Hsv,
            Self::Oklab => RenderBackgroundGradientBlend::Oklab,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeWindowBackgroundGradientSegment {
    size: usize,
    smoothness_millis: u32,
}

impl NativeWindowBackgroundGradientSegment {
    const fn to_render(self) -> RenderBackgroundGradientSegment {
        RenderBackgroundGradientSegment {
            size: self.size,
            smoothness_millis: self.smoothness_millis,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeWindowBackgroundGradientPreset {
    Blues,
    BrBg,
    BuGn,
    BuPu,
    Cividis,
    Cool,
    CubeHelixDefault,
    GnBu,
    Greens,
    Greys,
    Inferno,
    Magma,
    OrRd,
    Oranges,
    PiYg,
    Plasma,
    PrGn,
    PuBu,
    PuBuGn,
    PuOr,
    PuRd,
    Purples,
    Rainbow,
    RdBu,
    RdGy,
    RdPu,
    RdYlBu,
    RdYlGn,
    Reds,
    Sinebow,
    Spectral,
    Turbo,
    Viridis,
    Warm,
    YlGn,
    YlGnBu,
    YlOrBr,
    YlOrRd,
}

impl NativeWindowBackgroundGradientPreset {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Blues" => Some(Self::Blues),
            "BrBg" => Some(Self::BrBg),
            "BuGn" => Some(Self::BuGn),
            "BuPu" => Some(Self::BuPu),
            "Cividis" => Some(Self::Cividis),
            "Cool" => Some(Self::Cool),
            "CubeHelixDefault" => Some(Self::CubeHelixDefault),
            "GnBu" => Some(Self::GnBu),
            "Greens" => Some(Self::Greens),
            "Greys" => Some(Self::Greys),
            "Inferno" => Some(Self::Inferno),
            "Magma" => Some(Self::Magma),
            "OrRd" => Some(Self::OrRd),
            "Oranges" => Some(Self::Oranges),
            "PiYg" => Some(Self::PiYg),
            "Plasma" => Some(Self::Plasma),
            "PrGn" => Some(Self::PrGn),
            "PuBu" => Some(Self::PuBu),
            "PuBuGn" => Some(Self::PuBuGn),
            "PuOr" => Some(Self::PuOr),
            "PuRd" => Some(Self::PuRd),
            "Purples" => Some(Self::Purples),
            "Rainbow" => Some(Self::Rainbow),
            "RdBu" => Some(Self::RdBu),
            "RdGy" => Some(Self::RdGy),
            "RdPu" => Some(Self::RdPu),
            "RdYlBu" => Some(Self::RdYlBu),
            "RdYlGn" => Some(Self::RdYlGn),
            "Reds" => Some(Self::Reds),
            "Sinebow" => Some(Self::Sinebow),
            "Spectral" => Some(Self::Spectral),
            "Turbo" => Some(Self::Turbo),
            "Viridis" => Some(Self::Viridis),
            "Warm" => Some(Self::Warm),
            "YlGn" => Some(Self::YlGn),
            "YlGnBu" => Some(Self::YlGnBu),
            "YlOrBr" => Some(Self::YlOrBr),
            "YlOrRd" => Some(Self::YlOrRd),
            _ => None,
        }
    }

    const fn to_render(self) -> RenderBackgroundGradientPreset {
        match self {
            Self::Blues => RenderBackgroundGradientPreset::Blues,
            Self::BrBg => RenderBackgroundGradientPreset::BrBg,
            Self::BuGn => RenderBackgroundGradientPreset::BuGn,
            Self::BuPu => RenderBackgroundGradientPreset::BuPu,
            Self::Cividis => RenderBackgroundGradientPreset::Cividis,
            Self::Cool => RenderBackgroundGradientPreset::Cool,
            Self::CubeHelixDefault => RenderBackgroundGradientPreset::CubeHelixDefault,
            Self::GnBu => RenderBackgroundGradientPreset::GnBu,
            Self::Greens => RenderBackgroundGradientPreset::Greens,
            Self::Greys => RenderBackgroundGradientPreset::Greys,
            Self::Inferno => RenderBackgroundGradientPreset::Inferno,
            Self::Magma => RenderBackgroundGradientPreset::Magma,
            Self::OrRd => RenderBackgroundGradientPreset::OrRd,
            Self::Oranges => RenderBackgroundGradientPreset::Oranges,
            Self::PiYg => RenderBackgroundGradientPreset::PiYg,
            Self::Plasma => RenderBackgroundGradientPreset::Plasma,
            Self::PrGn => RenderBackgroundGradientPreset::PrGn,
            Self::PuBu => RenderBackgroundGradientPreset::PuBu,
            Self::PuBuGn => RenderBackgroundGradientPreset::PuBuGn,
            Self::PuOr => RenderBackgroundGradientPreset::PuOr,
            Self::PuRd => RenderBackgroundGradientPreset::PuRd,
            Self::Purples => RenderBackgroundGradientPreset::Purples,
            Self::Rainbow => RenderBackgroundGradientPreset::Rainbow,
            Self::RdBu => RenderBackgroundGradientPreset::RdBu,
            Self::RdGy => RenderBackgroundGradientPreset::RdGy,
            Self::RdPu => RenderBackgroundGradientPreset::RdPu,
            Self::RdYlBu => RenderBackgroundGradientPreset::RdYlBu,
            Self::RdYlGn => RenderBackgroundGradientPreset::RdYlGn,
            Self::Reds => RenderBackgroundGradientPreset::Reds,
            Self::Sinebow => RenderBackgroundGradientPreset::Sinebow,
            Self::Spectral => RenderBackgroundGradientPreset::Spectral,
            Self::Turbo => RenderBackgroundGradientPreset::Turbo,
            Self::Viridis => RenderBackgroundGradientPreset::Viridis,
            Self::Warm => RenderBackgroundGradientPreset::Warm,
            Self::YlGn => RenderBackgroundGradientPreset::YlGn,
            Self::YlGnBu => RenderBackgroundGradientPreset::YlGnBu,
            Self::YlOrBr => RenderBackgroundGradientPreset::YlOrBr,
            Self::YlOrRd => RenderBackgroundGradientPreset::YlOrRd,
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn native_gradient_angle_millidegrees_from_f64(angle_degrees: f64) -> Option<i32> {
    if !angle_degrees.is_finite() {
        return None;
    }

    let angle_millidegrees = (angle_degrees * 1_000.0).round();
    if angle_millidegrees < f64::from(i32::MIN) || angle_millidegrees > f64::from(i32::MAX) {
        return None;
    }

    Some(angle_millidegrees as i32)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn native_gradient_unit_interval_millis_from_f64(value: f64) -> Option<u32> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return None;
    }

    Some((value * 1_000.0).round() as u32)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn native_gradient_positive_millis_from_f64(value: f64) -> Option<u32> {
    if !value.is_finite() || value <= 0.0 || value > f64::from(u32::MAX) / 1_000.0 {
        return None;
    }

    Some((value * 1_000.0).round() as u32)
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeWindowBackgroundGradient {
    orientation: NativeWindowBackgroundGradientOrientation,
    interpolation: NativeWindowBackgroundGradientInterpolation,
    blend: NativeWindowBackgroundGradientBlend,
    noise: Option<usize>,
    segment: Option<NativeWindowBackgroundGradientSegment>,
    preset: Option<NativeWindowBackgroundGradientPreset>,
    opacity_alpha: u8,
    blend_with_background_color: bool,
    hsb: NativeInactivePaneHsb,
    colors: Vec<Color>,
}

impl NativeWindowBackgroundGradient {
    fn to_render(&self) -> RenderBackgroundGradient {
        RenderBackgroundGradient {
            orientation: self.orientation.to_render(),
            interpolation: self.interpolation.to_render(),
            blend: self.blend.to_render(),
            noise: self.noise,
            segment: self
                .segment
                .map(NativeWindowBackgroundGradientSegment::to_render),
            preset: self
                .preset
                .map(NativeWindowBackgroundGradientPreset::to_render),
            opacity_alpha: self.opacity_alpha,
            blend_with_default_background: self.blend_with_background_color,
            hsb: RenderBackgroundGradientHsb {
                hue: self.hsb.hue.0,
                saturation: self.hsb.saturation.0,
                brightness: self.hsb.brightness.0,
            },
            colors: self
                .colors
                .iter()
                .copied()
                .map(|color| color_to_rgba(color, DEFAULT_RENDER_BACKGROUND_RGBA))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeWindowBackgroundImage {
    data: Vec<u8>,
    opacity_alpha: u8,
    hsb: NativeInactivePaneHsb,
    animation_speed_millis: u32,
    attachment: RenderBackgroundImageAttachment,
    layout: NativeWindowBackgroundImageLayout,
}

impl NativeWindowBackgroundImage {
    fn to_render(&self) -> RenderBackgroundImage {
        RenderBackgroundImage {
            data: self.data.clone(),
            opacity_alpha: self.opacity_alpha,
            hsb: RenderBackgroundGradientHsb {
                hue: self.hsb.hue.0,
                saturation: self.hsb.saturation.0,
                brightness: self.hsb.brightness.0,
            },
            animation_speed_millis: self.animation_speed_millis,
            attachment: self.attachment,
            width: self.layout.width,
            height: self.layout.height,
            repeat_x: self.layout.repeat_x,
            repeat_y: self.layout.repeat_y,
            horizontal_align: self.layout.horizontal_align,
            vertical_align: self.layout.vertical_align,
            horizontal_offset: self.layout.horizontal_offset,
            vertical_offset: self.layout.vertical_offset,
            repeat_x_size: self.layout.repeat_x_size,
            repeat_y_size: self.layout.repeat_y_size,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeWindowBackgroundImageLayout {
    width: RenderBackgroundImageDimension,
    height: RenderBackgroundImageDimension,
    repeat_x: RenderBackgroundImageRepeat,
    repeat_y: RenderBackgroundImageRepeat,
    horizontal_align: RenderBackgroundImageHorizontalAlign,
    vertical_align: RenderBackgroundImageVerticalAlign,
    horizontal_offset: RenderBackgroundImageLength,
    vertical_offset: RenderBackgroundImageLength,
    repeat_x_size: Option<RenderBackgroundImageLength>,
    repeat_y_size: Option<RenderBackgroundImageLength>,
}

impl Default for NativeWindowBackgroundImageLayout {
    fn default() -> Self {
        Self {
            width: RenderBackgroundImageDimension::Cover,
            height: RenderBackgroundImageDimension::Cover,
            repeat_x: RenderBackgroundImageRepeat::Repeat,
            repeat_y: RenderBackgroundImageRepeat::Repeat,
            horizontal_align: RenderBackgroundImageHorizontalAlign::Left,
            vertical_align: RenderBackgroundImageVerticalAlign::Top,
            horizontal_offset: RenderBackgroundImageLength::Pixels(0),
            vertical_offset: RenderBackgroundImageLength::Pixels(0),
            repeat_x_size: None,
            repeat_y_size: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeWindowBackgroundVisualLayer {
    Color(Color),
    Gradient(NativeWindowBackgroundGradient),
    Image(NativeWindowBackgroundImage),
}

impl NativeWindowBackgroundVisualLayer {
    fn to_render(&self) -> RenderBackgroundLayer {
        match self {
            Self::Color(color) => {
                RenderBackgroundLayer::Color(color_to_rgba(*color, DEFAULT_RENDER_BACKGROUND_RGBA))
            }
            Self::Gradient(gradient) => RenderBackgroundLayer::Gradient(gradient.to_render()),
            Self::Image(image) => RenderBackgroundLayer::Image(image.to_render()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeTextMinContrastRatio(u16);

impl NativeTextMinContrastRatio {
    #[cfg(test)]
    const fn from_centi(value: u16) -> Self {
        Self(value)
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn from_f32(value: f32) -> Option<Self> {
        if !value.is_finite() || value < 0.0 {
            return None;
        }
        let centi = (value * 100.0).round().min(f32::from(u16::MAX));
        Some(Self(centi as u16))
    }

    fn as_f64(self) -> f64 {
        f64::from(self.0) / 100.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeContrastRatio(u16);

impl NativeContrastRatio {
    const fn from_centi(value: u16) -> Self {
        Self(value)
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn from_f32(value: f32) -> Option<Self> {
        if !value.is_finite() || value < 0.0 {
            return None;
        }
        let centi = (value * 100.0).round().min(f32::from(u16::MAX));
        Some(Self(centi as u16))
    }

    fn as_f64(self) -> f64 {
        f64::from(self.0) / 100.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeFontSize {
    millipoints: u32,
}

impl NativeFontSize {
    const fn from_millipoints(millipoints: u32) -> Self {
        Self { millipoints }
    }

    fn scale_against_default(self) -> f64 {
        f64::from(self.millipoints) / f64::from(DEFAULT_FONT_SIZE.millipoints)
    }

    fn config_text(self) -> String {
        let whole_points = self.millipoints / 1000;
        let fractional_points = self.millipoints % 1000;
        if fractional_points == 0 {
            return whole_points.to_string();
        }
        let mut fraction = format!("{fractional_points:03}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        format!("{whole_points}.{fraction}")
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeNotificationHandling {
    #[default]
    AlwaysShow,
    NeverShow,
    SuppressFromFocusedPane,
    SuppressFromFocusedTab,
    SuppressFromFocusedWindow,
}

impl NativeNotificationHandling {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "AlwaysShow" => Some(Self::AlwaysShow),
            "NeverShow" => Some(Self::NeverShow),
            "SuppressFromFocusedPane" => Some(Self::SuppressFromFocusedPane),
            "SuppressFromFocusedTab" => Some(Self::SuppressFromFocusedTab),
            "SuppressFromFocusedWindow" => Some(Self::SuppressFromFocusedWindow),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeVisualBellTarget {
    #[default]
    BackgroundColor,
    CursorColor,
}

impl NativeVisualBellTarget {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "BackgroundColor" => Some(Self::BackgroundColor),
            "CursorColor" => Some(Self::CursorColor),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::BackgroundColor => "BackgroundColor",
            Self::CursorColor => "CursorColor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeVisualBell {
    fade_in_duration_ms: u64,
    fade_out_duration_ms: u64,
    fade_in_function: NativeEasingFunction,
    fade_out_function: NativeEasingFunction,
    target: NativeVisualBellTarget,
}

impl NativeVisualBell {
    fn total_duration(self) -> Duration {
        Duration::from_millis(
            self.fade_in_duration_ms
                .saturating_add(self.fade_out_duration_ms),
        )
    }

    fn is_enabled(self) -> bool {
        self.fade_in_duration_ms
            .saturating_add(self.fade_out_duration_ms)
            > 0
    }
}

impl Default for NativeVisualBell {
    fn default() -> Self {
        Self {
            fade_in_duration_ms: 0,
            fade_out_duration_ms: 0,
            fade_in_function: NativeEasingFunction::Ease,
            fade_out_function: NativeEasingFunction::Ease,
            target: NativeVisualBellTarget::BackgroundColor,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeCursorStyle {
    #[default]
    SteadyBlock,
    BlinkingBlock,
    SteadyUnderline,
    BlinkingUnderline,
    SteadyBar,
    BlinkingBar,
}

impl NativeCursorStyle {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "SteadyBlock" => Some(Self::SteadyBlock),
            "BlinkingBlock" => Some(Self::BlinkingBlock),
            "SteadyUnderline" => Some(Self::SteadyUnderline),
            "BlinkingUnderline" => Some(Self::BlinkingUnderline),
            "SteadyBar" => Some(Self::SteadyBar),
            "BlinkingBar" => Some(Self::BlinkingBar),
            _ => None,
        }
    }

    fn config_text(self) -> &'static str {
        match self {
            Self::SteadyBlock => "SteadyBlock",
            Self::BlinkingBlock => "BlinkingBlock",
            Self::SteadyUnderline => "SteadyUnderline",
            Self::BlinkingUnderline => "BlinkingUnderline",
            Self::SteadyBar => "SteadyBar",
            Self::BlinkingBar => "BlinkingBar",
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
struct NativeAppliedBackendConfig {
    status_update_interval: Duration,
    max_fps: usize,
    animation_fps: usize,
    front_end: NativeRenderFrontEnd,
    webgpu_power_preference: NativeWebGpuPowerPreference,
    webgpu_force_fallback_adapter: bool,
    webgpu_preferred_adapter: Option<NativeWebGpuPreferredAdapter>,
    prefer_egl: bool,
    enable_wayland: bool,
    enable_zwlr_output_manager: bool,
    use_box_model_render: bool,
    experimental_pixel_positioning: bool,
    shape_cache_size: usize,
    line_state_cache_size: usize,
    line_quad_cache_size: usize,
    line_to_ele_shape_cache_size: usize,
    glyph_cache_image_cache_size: usize,
    cursor_blink_rate: Duration,
    cursor_blink_ease_in: NativeEasingFunction,
    cursor_blink_ease_out: NativeEasingFunction,
    text_blink_rate: Duration,
    text_blink_rate_rapid: Duration,
    text_blink_ease_in: NativeEasingFunction,
    text_blink_ease_out: NativeEasingFunction,
    text_blink_rapid_ease_in: NativeEasingFunction,
    text_blink_rapid_ease_out: NativeEasingFunction,
    default_cursor_style: NativeCursorStyle,
    cursor_thickness: Option<NativeCursorThickness>,
    underline_thickness: Option<NativeUnderlineThickness>,
    underline_position: Option<NativeUnderlinePosition>,
    strikethrough_position: Option<NativeStrikethroughPosition>,
    force_reverse_video_cursor: bool,
    reverse_video_cursor_min_contrast: NativeContrastRatio,
    text_min_contrast_ratio: Option<NativeTextMinContrastRatio>,
    window_padding: NativeWindowPadding,
    window_content_alignment: NativeWindowContentAlignment,
}

impl Default for NativeAppliedBackendConfig {
    fn default() -> Self {
        Self {
            status_update_interval: DEFAULT_STATUS_UPDATE_INTERVAL,
            max_fps: DEFAULT_MAX_FPS,
            animation_fps: DEFAULT_ANIMATION_FPS,
            front_end: DEFAULT_RENDER_FRONT_END,
            webgpu_power_preference: DEFAULT_WEBGPU_POWER_PREFERENCE,
            webgpu_force_fallback_adapter: DEFAULT_WEBGPU_FORCE_FALLBACK_ADAPTER,
            webgpu_preferred_adapter: None,
            prefer_egl: DEFAULT_PREFER_EGL,
            enable_wayland: DEFAULT_ENABLE_WAYLAND,
            enable_zwlr_output_manager: DEFAULT_ENABLE_ZWLR_OUTPUT_MANAGER,
            use_box_model_render: DEFAULT_USE_BOX_MODEL_RENDER,
            experimental_pixel_positioning: DEFAULT_EXPERIMENTAL_PIXEL_POSITIONING,
            shape_cache_size: DEFAULT_SHAPE_CACHE_SIZE,
            line_state_cache_size: DEFAULT_LINE_STATE_CACHE_SIZE,
            line_quad_cache_size: DEFAULT_LINE_QUAD_CACHE_SIZE,
            line_to_ele_shape_cache_size: DEFAULT_LINE_TO_ELE_SHAPE_CACHE_SIZE,
            glyph_cache_image_cache_size: DEFAULT_GLYPH_CACHE_IMAGE_CACHE_SIZE,
            cursor_blink_rate: DEFAULT_CURSOR_BLINK_RATE,
            cursor_blink_ease_in: DEFAULT_CURSOR_BLINK_EASE_IN,
            cursor_blink_ease_out: DEFAULT_CURSOR_BLINK_EASE_OUT,
            text_blink_rate: DEFAULT_TEXT_BLINK_RATE,
            text_blink_rate_rapid: DEFAULT_TEXT_BLINK_RATE_RAPID,
            text_blink_ease_in: DEFAULT_TEXT_BLINK_EASE_IN,
            text_blink_ease_out: DEFAULT_TEXT_BLINK_EASE_OUT,
            text_blink_rapid_ease_in: DEFAULT_TEXT_BLINK_RAPID_EASE_IN,
            text_blink_rapid_ease_out: DEFAULT_TEXT_BLINK_RAPID_EASE_OUT,
            default_cursor_style: DEFAULT_CURSOR_STYLE,
            cursor_thickness: DEFAULT_CURSOR_THICKNESS,
            underline_thickness: DEFAULT_UNDERLINE_THICKNESS,
            underline_position: DEFAULT_UNDERLINE_POSITION,
            strikethrough_position: DEFAULT_STRIKETHROUGH_POSITION,
            force_reverse_video_cursor: DEFAULT_FORCE_REVERSE_VIDEO_CURSOR,
            reverse_video_cursor_min_contrast: DEFAULT_REVERSE_VIDEO_CURSOR_MIN_CONTRAST,
            text_min_contrast_ratio: None,
            window_padding: MODERN_DEFAULT_WINDOW_PADDING,
            window_content_alignment: DEFAULT_WINDOW_CONTENT_ALIGNMENT,
        }
    }
}

#[expect(
    clippy::struct_field_names,
    reason = "field names mirror the upstream configuration schema"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeCubicBezier {
    x1_per_mille: i32,
    y1_per_mille: i32,
    x2_per_mille: i32,
    y2_per_mille: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeEasingFunction {
    Constant,
    #[default]
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(NativeCubicBezier),
}

impl NativeEasingFunction {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Constant" => Some(Self::Constant),
            "Linear" => Some(Self::Linear),
            "Ease" => Some(Self::Ease),
            "EaseIn" => Some(Self::EaseIn),
            "EaseOut" => Some(Self::EaseOut),
            "EaseInOut" => Some(Self::EaseInOut),
            _ => None,
        }
    }

    fn config_text(&self) -> &'static str {
        match self {
            Self::Constant => "Constant",
            Self::Linear => "Linear",
            Self::Ease => "Ease",
            Self::EaseIn => "EaseIn",
            Self::EaseOut => "EaseOut",
            Self::EaseInOut => "EaseInOut",
            Self::CubicBezier(_) => "CubicBezier",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeBoldBrightensAnsiColors {
    No,
    #[default]
    BrightAndBold,
    BrightOnly,
}

impl NativeBoldBrightensAnsiColors {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "No" => Some(Self::No),
            "BrightAndBold" => Some(Self::BrightAndBold),
            "BrightOnly" => Some(Self::BrightOnly),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::No => "No",
            Self::BrightAndBold => "BrightAndBold",
            Self::BrightOnly => "BrightOnly",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeAnsiColor {
    Black,
    Maroon,
    Green,
    Olive,
    Navy,
    Purple,
    Teal,
    Silver,
    Grey,
    Red,
    Lime,
    Yellow,
    Blue,
    Fuchsia,
    Aqua,
    White,
}

impl NativeAnsiColor {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "black" => Some(Self::Black),
            "maroon" => Some(Self::Maroon),
            "green" => Some(Self::Green),
            "olive" => Some(Self::Olive),
            "navy" => Some(Self::Navy),
            "purple" => Some(Self::Purple),
            "teal" => Some(Self::Teal),
            "silver" => Some(Self::Silver),
            "grey" | "gray" => Some(Self::Grey),
            "red" => Some(Self::Red),
            "lime" => Some(Self::Lime),
            "yellow" => Some(Self::Yellow),
            "blue" => Some(Self::Blue),
            "fuchsia" => Some(Self::Fuchsia),
            "aqua" => Some(Self::Aqua),
            "white" => Some(Self::White),
            _ => None,
        }
    }

    const fn palette_index(self) -> u8 {
        match self {
            Self::Black => 0,
            Self::Maroon => 1,
            Self::Green => 2,
            Self::Olive => 3,
            Self::Navy => 4,
            Self::Purple => 5,
            Self::Teal => 6,
            Self::Silver => 7,
            Self::Grey => 8,
            Self::Red => 9,
            Self::Lime => 10,
            Self::Yellow => 11,
            Self::Blue => 12,
            Self::Fuchsia => 13,
            Self::Aqua => 14,
            Self::White => 15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeColorSpec {
    Color(Color),
    AnsiColor(NativeAnsiColor),
}

fn native_color_spec_to_render_color(color: NativeColorSpec) -> Color {
    match color {
        NativeColorSpec::Color(color) => color,
        NativeColorSpec::AnsiColor(color) => Color::Indexed(color.palette_index()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeCursorThickness {
    Pixels(u32),
    Points(u32),
    Percent(u32),
    CellFractionPerMille(u32),
}

impl NativeCursorThickness {
    fn parse(value: &str) -> Option<Self> {
        parse_native_unsigned_dimension(
            value,
            Self::Pixels,
            Self::Points,
            Self::Percent,
            Self::CellFractionPerMille,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeUnderlineThickness {
    Pixels(u32),
    Points(u32),
    Percent(u32),
    CellFractionPerMille(u32),
}

impl NativeUnderlineThickness {
    fn parse(value: &str) -> Option<Self> {
        parse_native_unsigned_dimension(
            value,
            Self::Pixels,
            Self::Points,
            Self::Percent,
            Self::CellFractionPerMille,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeUnderlinePosition {
    Pixels(i32),
    Points(i32),
    Percent(i32),
    CellFractionPerMille(i32),
}

impl NativeUnderlinePosition {
    fn parse(value: &str) -> Option<Self> {
        parse_native_signed_dimension(
            value,
            Self::Pixels,
            Self::Points,
            Self::Percent,
            Self::CellFractionPerMille,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeStrikethroughPosition {
    Pixels(u32),
    Points(u32),
    Percent(u32),
    CellFractionPerMille(u32),
}

impl NativeStrikethroughPosition {
    fn parse(value: &str) -> Option<Self> {
        parse_native_unsigned_dimension(
            value,
            Self::Pixels,
            Self::Points,
            Self::Percent,
            Self::CellFractionPerMille,
        )
    }
}

fn native_ratio_config_text(value: f64) -> String {
    let mut text = format!("{value:.3}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn native_cell_fraction_config_text(per_mille: u32) -> String {
    format!(
        "{}cell",
        native_ratio_config_text(f64::from(per_mille) / 1_000.0)
    )
}

fn native_signed_cell_fraction_config_text(per_mille: i32) -> String {
    let text = native_cell_fraction_config_text(per_mille.unsigned_abs());
    if per_mille < 0 {
        format!("-{text}")
    } else {
        text
    }
}

fn native_cursor_thickness_config_text(value: NativeCursorThickness) -> String {
    match value {
        NativeCursorThickness::Pixels(pixels) => format!("{pixels}px"),
        NativeCursorThickness::Points(points) => format!("{points}pt"),
        NativeCursorThickness::Percent(percent) => format!("{percent}%"),
        NativeCursorThickness::CellFractionPerMille(per_mille) => {
            native_cell_fraction_config_text(per_mille)
        }
    }
}

fn native_underline_thickness_config_text(value: NativeUnderlineThickness) -> String {
    match value {
        NativeUnderlineThickness::Pixels(pixels) => format!("{pixels}px"),
        NativeUnderlineThickness::Points(points) => format!("{points}pt"),
        NativeUnderlineThickness::Percent(percent) => format!("{percent}%"),
        NativeUnderlineThickness::CellFractionPerMille(per_mille) => {
            native_cell_fraction_config_text(per_mille)
        }
    }
}

fn native_underline_position_config_text(value: NativeUnderlinePosition) -> String {
    match value {
        NativeUnderlinePosition::Pixels(pixels) => format!("{pixels}px"),
        NativeUnderlinePosition::Points(points) => format!("{points}pt"),
        NativeUnderlinePosition::Percent(percent) => format!("{percent}%"),
        NativeUnderlinePosition::CellFractionPerMille(per_mille) => {
            native_signed_cell_fraction_config_text(per_mille)
        }
    }
}

fn native_strikethrough_position_config_text(value: NativeStrikethroughPosition) -> String {
    match value {
        NativeStrikethroughPosition::Pixels(pixels) => format!("{pixels}px"),
        NativeStrikethroughPosition::Points(points) => format!("{points}pt"),
        NativeStrikethroughPosition::Percent(percent) => format!("{percent}%"),
        NativeStrikethroughPosition::CellFractionPerMille(per_mille) => {
            native_cell_fraction_config_text(per_mille)
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeWindowCloseConfirmation {
    #[default]
    AlwaysPrompt,
    NeverPrompt,
}

impl NativeWindowCloseConfirmation {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "AlwaysPrompt" => Some(Self::AlwaysPrompt),
            "NeverPrompt" => Some(Self::NeverPrompt),
            _ => None,
        }
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent compatibility flags represent valid combinations"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct NativeWindowDecorations {
    title: bool,
    resize: bool,
    integrated_buttons: bool,
    macos_force_disable_shadow: bool,
    macos_force_enable_shadow: bool,
    macos_force_square_corners: bool,
    macos_use_background_color_as_titlebar_color: bool,
}

impl NativeWindowDecorations {
    fn parse(value: &str) -> Option<Self> {
        let mut decorations = Self {
            title: false,
            resize: false,
            integrated_buttons: false,
            macos_force_disable_shadow: false,
            macos_force_enable_shadow: false,
            macos_force_square_corners: false,
            macos_use_background_color_as_titlebar_color: false,
        };
        let mut saw_flag = false;

        for flag in value.split('|') {
            let flag = flag.trim();
            if flag.is_empty() {
                continue;
            }
            saw_flag = true;
            match flag {
                "NONE" => {}
                "TITLE" => decorations.title = true,
                "RESIZE" => decorations.resize = true,
                "INTEGRATED_BUTTONS" => decorations.integrated_buttons = true,
                "MACOS_FORCE_DISABLE_SHADOW" => decorations.macos_force_disable_shadow = true,
                "MACOS_FORCE_ENABLE_SHADOW" => decorations.macos_force_enable_shadow = true,
                "MACOS_FORCE_SQUARE_CORNERS" => decorations.macos_force_square_corners = true,
                "MACOS_USE_BACKGROUND_COLOR_AS_TITLEBAR_COLOR" => {
                    decorations.macos_use_background_color_as_titlebar_color = true;
                }
                _ => return None,
            }
        }

        saw_flag.then_some(decorations)
    }

    const fn winit_decorations_enabled(self) -> bool {
        // Integrated title buttons are rendered by the terminal tab row; ask
        // winit for a borderless surface even when the configuration also
        // requests resize affordances.  Resizability is applied separately
        // on the window builder below.
        if cfg!(target_os = "windows") && self.integrated_buttons {
            false
        } else {
            self.title || self.resize
        }
    }

    fn as_wezterm_config_value(self) -> String {
        let mut flags = Vec::new();

        if self.title {
            flags.push("TITLE");
        }
        if self.resize {
            flags.push("RESIZE");
        }
        if self.integrated_buttons {
            flags.push("INTEGRATED_BUTTONS");
        }
        if self.macos_force_disable_shadow {
            flags.push("MACOS_FORCE_DISABLE_SHADOW");
        }
        if self.macos_force_enable_shadow {
            flags.push("MACOS_FORCE_ENABLE_SHADOW");
        }
        if self.macos_force_square_corners {
            flags.push("MACOS_FORCE_SQUARE_CORNERS");
        }
        if self.macos_use_background_color_as_titlebar_color {
            flags.push("MACOS_USE_BACKGROUND_COLOR_AS_TITLEBAR_COLOR");
        }

        if flags.is_empty() {
            "NONE".to_owned()
        } else {
            flags.join("|")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeIntegratedTitleButton {
    Hide,
    Maximize,
    Close,
}

impl NativeIntegratedTitleButton {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "Hide" => Some(Self::Hide),
            "Maximize" => Some(Self::Maximize),
            "Close" => Some(Self::Close),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::Hide => "Hide",
            Self::Maximize => "Maximize",
            Self::Close => "Close",
        }
    }
}

fn default_integrated_title_buttons() -> Vec<NativeIntegratedTitleButton> {
    vec![
        NativeIntegratedTitleButton::Hide,
        NativeIntegratedTitleButton::Maximize,
        NativeIntegratedTitleButton::Close,
    ]
}

fn native_integrated_title_buttons_from_strings(
    buttons: Vec<String>,
) -> Option<Vec<NativeIntegratedTitleButton>> {
    buttons
        .into_iter()
        .map(|button| NativeIntegratedTitleButton::parse(&button))
        .collect()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeIntegratedTitleButtonAlignment {
    Left,
    #[default]
    Right,
}

impl NativeIntegratedTitleButtonAlignment {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "Left" => Some(Self::Left),
            "Right" => Some(Self::Right),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeIntegratedTitleButtonColor {
    #[default]
    Auto,
    Color(Color),
}

impl NativeIntegratedTitleButtonColor {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "Auto" => Some(Self::Auto),
            color => Some(Self::Color(lua_opaque_color_from_query(color)?)),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeIntegratedTitleButtonStyle {
    #[default]
    Windows,
    Gnome,
    MacOsNative,
}

impl NativeIntegratedTitleButtonStyle {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "Windows" => Some(Self::Windows),
            "Gnome" => Some(Self::Gnome),
            "MacOsNative" => Some(Self::MacOsNative),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::Gnome => "Gnome",
            Self::MacOsNative => "MacOsNative",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeWin32SystemBackdrop {
    #[default]
    Auto,
    Disable,
    Acrylic,
    Mica,
    Tabbed,
}

impl NativeWin32SystemBackdrop {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "Auto" => Some(Self::Auto),
            "Disable" => Some(Self::Disable),
            "Acrylic" => Some(Self::Acrylic),
            "Mica" => Some(Self::Mica),
            "Tabbed" => Some(Self::Tabbed),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Disable => "Disable",
            Self::Acrylic => "Acrylic",
            Self::Mica => "Mica",
            Self::Tabbed => "Tabbed",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeExitBehavior {
    #[default]
    Close,
    Hold,
    CloseOnCleanExit,
}

impl NativeExitBehavior {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "Close" => Some(Self::Close),
            "Hold" => Some(Self::Hold),
            "CloseOnCleanExit" => Some(Self::CloseOnCleanExit),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::Hold => "Hold",
            Self::CloseOnCleanExit => "CloseOnCleanExit",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeExitBehaviorMessaging {
    #[default]
    Verbose,
    Brief,
    Terse,
    None,
}

impl NativeExitBehaviorMessaging {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "Verbose" => Some(Self::Verbose),
            "Brief" => Some(Self::Brief),
            "Terse" => Some(Self::Terse),
            "None" => Some(Self::None),
            _ => None,
        }
    }

    fn as_wezterm_config_value(self) -> &'static str {
        match self {
            Self::Verbose => "Verbose",
            Self::Brief => "Brief",
            Self::Terse => "Terse",
            Self::None => "None",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeQuoteDroppedFiles {
    None,
    #[default]
    SpacesOnly,
    Posix,
    Windows,
    WindowsAlwaysQuoted,
}

impl NativeQuoteDroppedFiles {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "None" => Some(Self::None),
            "SpacesOnly" => Some(Self::SpacesOnly),
            "Posix" => Some(Self::Posix),
            "Windows" => Some(Self::Windows),
            "WindowsAlwaysQuoted" => Some(Self::WindowsAlwaysQuoted),
            _ => None,
        }
    }

    fn config_text(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::SpacesOnly => "SpacesOnly",
            Self::Posix => "Posix",
            Self::Windows => "Windows",
            Self::WindowsAlwaysQuoted => "WindowsAlwaysQuoted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeScrollBarHeight {
    Pixels(u32),
    Points(u32),
    CellFractionPerMille(u32),
    Percent(u32),
}

impl NativeScrollBarHeight {
    #[allow(dead_code)]
    fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.is_empty() {
            return None;
        }

        if let Some(number) = value.strip_suffix("cell") {
            return parse_non_negative_f64(number)
                .map(|value| Self::CellFractionPerMille(rounded_u32(value * 1_000.0)));
        }

        if let Some(number) = value.strip_suffix("px") {
            return parse_non_negative_f64(number).map(|value| Self::Pixels(rounded_u32(value)));
        }

        if let Some(number) = value.strip_suffix("pt") {
            return parse_non_negative_f64(number).map(|value| Self::Points(rounded_u32(value)));
        }

        if let Some(number) = value.strip_suffix('%') {
            return parse_non_negative_f64(number).map(|value| Self::Percent(rounded_u32(value)));
        }

        parse_non_negative_f64(value).map(|value| Self::Pixels(rounded_u32(value)))
    }

    fn config_text(self) -> String {
        match self {
            Self::Pixels(pixels) => format!("{pixels}px"),
            Self::Points(points) => format!("{points}pt"),
            Self::CellFractionPerMille(per_mille) => {
                format!("{}cell", native_per_mille_decimal_text(per_mille))
            }
            Self::Percent(percent) => format!("{percent}%"),
        }
    }
}

fn native_per_mille_decimal_text(per_mille: u32) -> String {
    let whole = per_mille / 1_000;
    let fraction = per_mille % 1_000;
    if fraction == 0 {
        return whole.to_string();
    }

    let mut fraction_text = format!("{fraction:03}");
    while fraction_text.ends_with('0') {
        fraction_text.pop();
    }
    format!("{whole}.{fraction_text}")
}

#[allow(dead_code)]
fn parse_non_negative_f64(value: &str) -> Option<f64> {
    let value = value.trim().parse::<f64>().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value)
}

#[allow(dead_code)]
fn parse_finite_f64(value: &str) -> Option<f64> {
    let value = value.trim().parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[allow(dead_code)]
fn rounded_u32(value: f64) -> u32 {
    value.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

#[allow(clippy::cast_possible_truncation)]
#[allow(dead_code)]
fn rounded_i32(value: f64) -> i32 {
    value
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

#[allow(dead_code)]
fn parse_native_unsigned_dimension<T>(
    value: &str,
    pixels: impl Fn(u32) -> T,
    points: impl Fn(u32) -> T,
    percent: impl Fn(u32) -> T,
    cell_fraction_per_mille: impl Fn(u32) -> T,
) -> Option<T> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(number) = value.strip_suffix("cell") {
        return parse_non_negative_f64(number)
            .map(|value| cell_fraction_per_mille(rounded_u32(value * 1_000.0)));
    }

    if let Some(number) = value.strip_suffix("px") {
        return parse_non_negative_f64(number).map(|value| pixels(rounded_u32(value)));
    }

    if let Some(number) = value.strip_suffix("pt") {
        return parse_non_negative_f64(number).map(|value| points(rounded_u32(value)));
    }

    if let Some(number) = value.strip_suffix('%') {
        return parse_non_negative_f64(number).map(|value| percent(rounded_u32(value)));
    }

    parse_non_negative_f64(value).map(|value| pixels(rounded_u32(value)))
}

#[allow(dead_code)]
fn parse_native_signed_dimension<T>(
    value: &str,
    pixels: impl Fn(i32) -> T,
    points: impl Fn(i32) -> T,
    percent: impl Fn(i32) -> T,
    cell_fraction_per_mille: impl Fn(i32) -> T,
) -> Option<T> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(number) = value.strip_suffix("cell") {
        return parse_finite_f64(number)
            .map(|value| cell_fraction_per_mille(rounded_i32(value * 1_000.0)));
    }

    if let Some(number) = value.strip_suffix("px") {
        return parse_finite_f64(number).map(|value| pixels(rounded_i32(value)));
    }

    if let Some(number) = value.strip_suffix("pt") {
        return parse_finite_f64(number).map(|value| points(rounded_i32(value)));
    }

    if let Some(number) = value.strip_suffix('%') {
        return parse_finite_f64(number).map(|value| percent(rounded_i32(value)));
    }

    parse_finite_f64(value).map(|value| pixels(rounded_i32(value)))
}

impl From<NativeCursorStyle> for CursorStyle {
    fn from(value: NativeCursorStyle) -> Self {
        match value {
            NativeCursorStyle::SteadyBlock => Self::SteadyBlock,
            NativeCursorStyle::BlinkingBlock => Self::BlinkingBlock,
            NativeCursorStyle::SteadyUnderline => Self::SteadyUnderline,
            NativeCursorStyle::BlinkingUnderline => Self::BlinkingUnderline,
            NativeCursorStyle::SteadyBar => Self::SteadyBar,
            NativeCursorStyle::BlinkingBar => Self::BlinkingBar,
        }
    }
}

impl From<NativeBoldBrightensAnsiColors> for RenderBoldBrightensAnsiColors {
    fn from(value: NativeBoldBrightensAnsiColors) -> Self {
        match value {
            NativeBoldBrightensAnsiColors::No => Self::No,
            NativeBoldBrightensAnsiColors::BrightAndBold => Self::BrightAndBold,
            NativeBoldBrightensAnsiColors::BrightOnly => Self::BrightOnly,
        }
    }
}

impl From<NativeCursorThickness> for RenderCursorThickness {
    fn from(value: NativeCursorThickness) -> Self {
        match value {
            NativeCursorThickness::Pixels(pixels) => Self::Pixels(pixels),
            NativeCursorThickness::Points(points) => Self::Points(points),
            NativeCursorThickness::Percent(percent) => Self::Percent(percent),
            NativeCursorThickness::CellFractionPerMille(per_mille) => {
                Self::CellFractionPerMille(per_mille)
            }
        }
    }
}

impl From<NativeUnderlineThickness> for RenderUnderlineThickness {
    fn from(value: NativeUnderlineThickness) -> Self {
        match value {
            NativeUnderlineThickness::Pixels(pixels) => Self::Pixels(pixels),
            NativeUnderlineThickness::Points(points) => Self::Points(points),
            NativeUnderlineThickness::Percent(percent) => Self::Percent(percent),
            NativeUnderlineThickness::CellFractionPerMille(per_mille) => {
                Self::CellFractionPerMille(per_mille)
            }
        }
    }
}

impl From<NativeUnderlinePosition> for RenderUnderlinePosition {
    fn from(value: NativeUnderlinePosition) -> Self {
        match value {
            NativeUnderlinePosition::Pixels(pixels) => Self::Pixels(pixels),
            NativeUnderlinePosition::Points(points) => Self::Points(points),
            NativeUnderlinePosition::Percent(percent) => Self::Percent(percent),
            NativeUnderlinePosition::CellFractionPerMille(per_mille) => {
                Self::CellFractionPerMille(per_mille)
            }
        }
    }
}

impl From<NativeStrikethroughPosition> for RenderStrikethroughPosition {
    fn from(value: NativeStrikethroughPosition) -> Self {
        match value {
            NativeStrikethroughPosition::Pixels(pixels) => Self::Pixels(pixels),
            NativeStrikethroughPosition::Points(points) => Self::Points(points),
            NativeStrikethroughPosition::Percent(percent) => Self::Percent(percent),
            NativeStrikethroughPosition::CellFractionPerMille(per_mille) => {
                Self::CellFractionPerMille(per_mille)
            }
        }
    }
}

impl From<NativeScrollBarHeight> for RenderScrollbarThumbSize {
    fn from(value: NativeScrollBarHeight) -> Self {
        match value {
            NativeScrollBarHeight::Pixels(pixels) => Self::Pixels(pixels),
            NativeScrollBarHeight::Points(points) => Self::Points(points),
            NativeScrollBarHeight::CellFractionPerMille(per_mille) => {
                Self::CellFractionPerMille(per_mille)
            }
            NativeScrollBarHeight::Percent(percent) => Self::Percent(percent),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeHsbMultiplier(u16);

impl NativeHsbMultiplier {
    const ONE: Self = Self::from_per_mille(1_000);

    const fn from_per_mille(value: u16) -> Self {
        Self(value)
    }

    #[cfg(test)]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn from_f32(value: f32) -> Self {
        if !value.is_finite() || value < 0.0 {
            return Self::ONE;
        }
        let per_mille = (value * 1_000.0).round().min(f32::from(u16::MAX));
        Self(per_mille as u16)
    }

    fn as_f64(self) -> f64 {
        f64::from(self.0) / 1_000.0
    }

    fn config_text(self) -> String {
        native_per_mille_decimal_text(u32::from(self.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeInactivePaneHsb {
    hue: NativeHsbMultiplier,
    saturation: NativeHsbMultiplier,
    brightness: NativeHsbMultiplier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeWindowFocusChange {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
    focused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeWindowResize {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
    pixel_width: u32,
    pixel_height: u32,
    terminal_size: TerminalSize,
    is_full_screen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeWindowConfigReloaded {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeCommandPaletteAugment {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeCommandPaletteEntry {
    brief: String,
    doc: Option<String>,
    icon: Option<String>,
    key_assignment: Option<String>,
    action: WindowCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativePromptInputLine {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
    line: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeInputSelector {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
    id: Option<String>,
    label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeConfirmation {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
    accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowEmitEvent {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLaunchMenuItem {
    label: Option<String>,
    command: NativeLaunchMenuCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLaunchMenuCommand {
    Command(WindowSpawnCommandQuery),
    Options(WindowSpawnCommandQueryOptions),
}

impl NativeLaunchMenuCommand {
    fn launch_menu_label(&self) -> String {
        match self {
            Self::Command(command) => command.launch_menu_label(),
            Self::Options(options) => options.launch_menu_label(),
        }
    }

    fn window_command(&self) -> WindowCommand {
        match self {
            Self::Command(command) => WindowCommand::SpawnCommandInNewTab(command.clone()),
            Self::Options(options) => WindowCommand::SpawnCommandOptionsInNewTab(options.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeUserKeyAssignment {
    keys: String,
    command: WindowCommand,
}

struct ConfiguredStartupApp {
    app: Box<NativeWindowApp>,
    lifecycle: Box<NativeConfigLifecycle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReloadDisposition {
    SilentStartup,
    ReloadAttempt,
}

fn configured_startup_app(
    options: &WindowOptions,
    discovery: ConfigDiscoveryInputs,
) -> Result<ConfiguredStartupApp, NativeConfigLoadError> {
    configured_startup_app_with_constructor(options, discovery, |startup| {
        NativeWindowApp::new_with_workspace_class_position_and_osc52_policy(
            options.frame_limit,
            options.osc52_policy,
            startup.command,
            startup.workspace.as_deref(),
            startup.window_class,
            startup.position,
        )
    })
}

#[cfg(test)]
fn configured_startup_app_for_test(
    options: &WindowOptions,
    discovery: ConfigDiscoveryInputs,
) -> Result<ConfiguredStartupApp, NativeConfigLoadError> {
    configured_startup_app(options, discovery)
}

fn configured_startup_app_with_constructor(
    options: &WindowOptions,
    discovery: ConfigDiscoveryInputs,
    constructor: impl FnOnce(NativeWindowStartup) -> Box<NativeWindowApp>,
) -> Result<ConfiguredStartupApp, NativeConfigLoadError> {
    let cli = validate_cli_config_overrides(&options.config.config_overrides)?;
    let mut lifecycle = NativeConfigLifecycle::new(
        discovery,
        options.config.skip_config,
        options.config.config_file.clone(),
        cli,
    );
    let attempt = lifecycle.attempt_reload();
    lifecycle.install_initial_attempt(attempt);

    let startup = NativeWindowStartup::from_options(options);
    let mut app = constructor(startup);
    app.metrics.mark_config_started();
    app.set_base_config(lifecycle.effective(), ReloadDisposition::SilentStartup);
    app.metrics.mark_config_finished();
    Ok(ConfiguredStartupApp {
        app,
        lifecycle: Box::new(lifecycle),
    })
}

#[cfg(test)]
impl NativeUserKeyAssignment {
    pub(crate) fn test_projection(&self) -> (&str, Option<&str>) {
        let send_string = match &self.command {
            WindowCommand::SendString(payload) => Some(payload.as_str()),
            _ => None,
        };
        (&self.keys, send_string)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeUserMouseAssignment {
    event: NativeMouseAssignmentEvent,
    modifiers: ModifiersState,
    mouse_reporting: bool,
    alt_screen: NativeMouseAssignmentAltScreen,
    command: WindowCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeMouseAssignmentButton {
    Mouse(MouseButton),
    WheelUp,
    WheelDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeMouseAssignmentAltScreen {
    Any,
    Active(bool),
}

impl NativeMouseAssignmentAltScreen {
    fn matches(self, alternate_screen_active: bool) -> bool {
        match self {
            Self::Any => true,
            Self::Active(expected) => expected == alternate_screen_active,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeMouseAssignmentEvent {
    kind: NativeMouseAssignmentEventKind,
    button: NativeMouseAssignmentButton,
    streak: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeMouseAssignmentEventKind {
    Down,
    Up,
    Drag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLeaderKey {
    keys: String,
    timeout_milliseconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowOpenUri {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
    uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowNewTabButtonClick {
    window_id: rssh_core::WindowId,
    pane: rssh_core::PaneId,
    button: MouseButton,
    default_action: Option<WindowCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowTitleFormat {
    default_title: String,
    active_tab: rssh_core::TabId,
    active_pane: rssh_core::PaneId,
    active_key_table: Option<String>,
    tab_count: usize,
    pane_count: usize,
    config: NativeConfigView,
    active_tab_info: NativeTabInformation,
    active_pane_info: NativePaneInformation,
    tabs: Vec<NativeTabInformation>,
    panes: Vec<NativePaneInformation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeTabTitleFormat {
    default_title: Option<String>,
    tab: rssh_core::TabId,
    active_pane: rssh_core::PaneId,
    tab_index: usize,
    tab_count: usize,
    pane_count: usize,
    is_active: bool,
    is_last_active: bool,
    hover: bool,
    max_width: usize,
    config: NativeConfigView,
    window_id: rssh_core::WindowId,
    window_title: String,
    tab_title: Option<String>,
    tab_info: NativeTabInformation,
    active_pane_info: NativePaneInformation,
    tabs: Vec<NativeTabInformation>,
    panes: Vec<NativePaneInformation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeTabInformation {
    tab_id: rssh_core::TabId,
    tab_index: usize,
    is_active: bool,
    is_last_active: bool,
    active_pane: NativePaneInformation,
    panes: Vec<NativePaneInformation>,
    window_id: rssh_core::WindowId,
    window_title: String,
    tab_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativePaneInformation {
    pane_id: rssh_core::PaneId,
    pane_index: usize,
    is_active: bool,
    is_zoomed: bool,
    left: u16,
    top: u16,
    width: u16,
    height: u16,
    pixel_width: u32,
    pixel_height: u32,
    title: Option<String>,
    foreground_process_name: String,
    current_working_dir: Option<String>,
    has_unseen_output: bool,
    domain_name: String,
    tty_name: Option<String>,
    user_vars: HashMap<String, String>,
    progress: PaneProgress,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NativeTabBarItemColors {
    fg_color: Option<Color>,
    bg_color: Option<Color>,
    intensity: Option<NativeFormatIntensity>,
    underline: Option<NativeFormatUnderline>,
    italic: Option<bool>,
    strikethrough: Option<bool>,
}

#[cfg(test)]
impl NativeTabBarItemColors {
    #[expect(
        clippy::type_complexity,
        reason = "tuple shape mirrors the compatibility data contract"
    )]
    pub(crate) fn test_projection(
        &self,
    ) -> (
        Option<Color>,
        Option<Color>,
        Option<&'static str>,
        Option<&'static str>,
        Option<bool>,
        Option<bool>,
    ) {
        let intensity = self.intensity.map(|intensity| match intensity {
            NativeFormatIntensity::Normal => "Normal",
            NativeFormatIntensity::Bold => "Bold",
            NativeFormatIntensity::Half => "Half",
        });
        let underline = self.underline.map(|underline| match underline {
            NativeFormatUnderline::None => "None",
            NativeFormatUnderline::Single => "Single",
            NativeFormatUnderline::Double => "Double",
            NativeFormatUnderline::Curly => "Curly",
            NativeFormatUnderline::Dotted => "Dotted",
            NativeFormatUnderline::Dashed => "Dashed",
        });
        (
            self.fg_color,
            self.bg_color,
            intensity,
            underline,
            self.italic,
            self.strikethrough,
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeTabBarStyle {
    active_tab_left: Option<Vec<NativeFormatItem>>,
    active_tab_right: Option<Vec<NativeFormatItem>>,
    inactive_tab_left: Option<Vec<NativeFormatItem>>,
    inactive_tab_right: Option<Vec<NativeFormatItem>>,
    inactive_tab_hover_left: Option<Vec<NativeFormatItem>>,
    inactive_tab_hover_right: Option<Vec<NativeFormatItem>>,
    new_tab: Option<Vec<NativeFormatItem>>,
    new_tab_hover: Option<Vec<NativeFormatItem>>,
    new_tab_left: Option<Vec<NativeFormatItem>>,
    new_tab_right: Option<Vec<NativeFormatItem>>,
    new_tab_hover_left: Option<Vec<NativeFormatItem>>,
    new_tab_hover_right: Option<Vec<NativeFormatItem>>,
    window_hide: Option<Vec<NativeFormatItem>>,
    window_hide_hover: Option<Vec<NativeFormatItem>>,
    window_maximize: Option<Vec<NativeFormatItem>>,
    window_maximize_hover: Option<Vec<NativeFormatItem>>,
    window_close: Option<Vec<NativeFormatItem>>,
    window_close_hover: Option<Vec<NativeFormatItem>>,
}

impl NativeTabBarStyle {
    fn is_empty(&self) -> bool {
        self.active_tab_left.is_none()
            && self.active_tab_right.is_none()
            && self.inactive_tab_left.is_none()
            && self.inactive_tab_right.is_none()
            && self.inactive_tab_hover_left.is_none()
            && self.inactive_tab_hover_right.is_none()
            && self.new_tab.is_none()
            && self.new_tab_hover.is_none()
            && self.new_tab_left.is_none()
            && self.new_tab_right.is_none()
            && self.new_tab_hover_left.is_none()
            && self.new_tab_hover_right.is_none()
            && self.window_hide.is_none()
            && self.window_hide_hover.is_none()
            && self.window_maximize.is_none()
            && self.window_maximize_hover.is_none()
            && self.window_close.is_none()
            && self.window_close_hover.is_none()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeFontRule {
    italic: Option<bool>,
    intensity: Option<NativeFormatIntensity>,
    underline: Option<NativeFormatUnderline>,
    blink: Option<NativeFontRuleBlink>,
    reverse: Option<bool>,
    strikethrough: Option<bool>,
    invisible: Option<bool>,
    font: Option<String>,
    font_fallbacks: Vec<String>,
    font_attributes: NativeFontAttributes,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeFontAttributes {
    weight: Option<String>,
    stretch: Option<String>,
    style: Option<String>,
    harfbuzz_features: Vec<String>,
    assume_emoji_presentation: Option<bool>,
    freetype_load_target: Option<NativeFreetypeTarget>,
    freetype_render_target: Option<NativeFreetypeTarget>,
    freetype_load_flags: Option<NativeFreetypeLoadFlags>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeFontConfig {
    families: Vec<String>,
    attributes: NativeFontAttributes,
}

fn native_font_config(family: &str) -> NativeFontConfig {
    NativeFontConfig {
        families: vec![family.to_owned()],
        attributes: NativeFontAttributes::default(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeFontRuleBlink {
    None,
    Slow,
    Rapid,
}

impl NativeFontRuleBlink {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "None" => Some(Self::None),
            "Slow" => Some(Self::Slow),
            "Rapid" => Some(Self::Rapid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeHorizontalContentAlignment {
    Left,
    Center,
    Right,
}

impl NativeHorizontalContentAlignment {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Left" => Some(Self::Left),
            "Center" => Some(Self::Center),
            "Right" => Some(Self::Right),
            _ => None,
        }
    }

    fn offset(self, gap: u32) -> u32 {
        match self {
            Self::Left => 0,
            Self::Center => gap / 2,
            Self::Right => gap,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeVerticalContentAlignment {
    Top,
    Center,
    Bottom,
}

impl NativeVerticalContentAlignment {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Top" => Some(Self::Top),
            "Center" => Some(Self::Center),
            "Bottom" => Some(Self::Bottom),
            _ => None,
        }
    }

    fn offset(self, gap: u32) -> u32 {
        match self {
            Self::Top => 0,
            Self::Center => gap / 2,
            Self::Bottom => gap,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeWindowContentAlignment {
    horizontal: NativeHorizontalContentAlignment,
    vertical: NativeVerticalContentAlignment,
}

const DEFAULT_WINDOW_CONTENT_ALIGNMENT: NativeWindowContentAlignment =
    NativeWindowContentAlignment {
        horizontal: NativeHorizontalContentAlignment::Left,
        vertical: NativeVerticalContentAlignment::Top,
    };

#[cfg(target_os = "windows")]
const DEFAULT_WINDOW_FRAME_FONT: &str = "Cascadia Mono";
#[cfg(target_os = "macos")]
const DEFAULT_WINDOW_FRAME_FONT: &str = "Menlo";
#[cfg(target_os = "linux")]
const DEFAULT_WINDOW_FRAME_FONT: &str = "Noto Sans Mono";
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
const DEFAULT_WINDOW_FRAME_FONT: &str = "Cascadia Mono";

#[cfg(target_os = "windows")]
const DEFAULT_WINDOW_FRAME_FONT_SIZE: NativeFontSize = NativeFontSize::from_millipoints(10_000);
#[cfg(not(target_os = "windows"))]
const DEFAULT_WINDOW_FRAME_FONT_SIZE: NativeFontSize = NativeFontSize::from_millipoints(12_000);

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowFrameAppearance {
    inactive_titlebar_bg: Option<Color>,
    active_titlebar_bg: Option<Color>,
    inactive_titlebar_fg: Option<Color>,
    active_titlebar_fg: Option<Color>,
    inactive_titlebar_border_bottom: Option<Color>,
    active_titlebar_border_bottom: Option<Color>,
    button_fg: Option<Color>,
    button_bg: Option<Color>,
    button_hover_fg: Option<Color>,
    button_hover_bg: Option<Color>,
    border_left_width: Option<NativeWindowPaddingDimension>,
    border_right_width: Option<NativeWindowPaddingDimension>,
    border_top_height: Option<NativeWindowPaddingDimension>,
    border_bottom_height: Option<NativeWindowPaddingDimension>,
    border_left_color: Option<Color>,
    border_right_color: Option<Color>,
    border_top_color: Option<Color>,
    border_bottom_color: Option<Color>,
    font: Option<String>,
    font_size: Option<NativeFontSize>,
}

impl Default for NativeWindowFrameAppearance {
    fn default() -> Self {
        Self {
            inactive_titlebar_bg: None,
            active_titlebar_bg: None,
            inactive_titlebar_fg: None,
            active_titlebar_fg: None,
            inactive_titlebar_border_bottom: None,
            active_titlebar_border_bottom: None,
            button_fg: None,
            button_bg: None,
            button_hover_fg: None,
            button_hover_bg: None,
            border_left_width: None,
            border_right_width: None,
            border_top_height: None,
            border_bottom_height: None,
            border_left_color: None,
            border_right_color: None,
            border_top_color: None,
            border_bottom_color: None,
            font: Some(DEFAULT_WINDOW_FRAME_FONT.to_owned()),
            font_size: Some(DEFAULT_WINDOW_FRAME_FONT_SIZE),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeHyperlinkRule {
    regex: String,
    format: String,
    highlight: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeDaemonOptions {
    pid_file: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeSerialDomain {
    name: String,
    port: Option<String>,
    baud: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeExecDomainLabel {
    Value(String),
    Function(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeExecDomain {
    name: String,
    fixup_command: String,
    label: Option<NativeExecDomainLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWslDomain {
    name: String,
    distribution: Option<String>,
    username: Option<String>,
    default_cwd: Option<String>,
    default_prog: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct NativeUnixDomain {
    name: String,
    socket_path: Option<String>,
    connect_automatically: bool,
    no_serve_automatically: bool,
    serve_command: Option<Vec<String>>,
    proxy_command: Option<Vec<String>>,
    skip_permissions_check: bool,
    read_timeout_ms: u64,
    write_timeout_ms: u64,
    local_echo_threshold_ms: Option<u64>,
    overlay_lag_indicator: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum NativeSshMultiplexing {
    #[default]
    WezTerm,
    None,
}

impl NativeSshMultiplexing {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "WezTerm" => Some(Self::WezTerm),
            "None" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum NativeShellAssumption {
    #[default]
    Unknown,
    Posix,
}

impl NativeShellAssumption {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Unknown" => Some(Self::Unknown),
            "Posix" => Some(Self::Posix),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct NativeSshDomain {
    name: String,
    remote_address: String,
    no_agent_auth: bool,
    username: Option<String>,
    connect_automatically: bool,
    timeout_ms: u64,
    local_echo_threshold_ms: Option<u64>,
    overlay_lag_indicator: bool,
    remote_wezterm_path: Option<String>,
    override_proxy_command: Option<String>,
    ssh_backend: Option<NativeSshBackend>,
    multiplexing: NativeSshMultiplexing,
    ssh_option: BTreeMap<String, String>,
    default_prog: Option<Vec<String>>,
    assume_shell: NativeShellAssumption,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeTlsServerDomain {
    bind_address: String,
    pem_private_key: Option<String>,
    pem_cert: Option<String>,
    pem_ca: Option<String>,
    pem_root_certs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct NativeTlsClientDomain {
    name: String,
    bootstrap_via_ssh: Option<String>,
    remote_address: String,
    pem_private_key: Option<String>,
    pem_cert: Option<String>,
    pem_ca: Option<String>,
    pem_root_certs: Vec<String>,
    accept_invalid_hostnames: bool,
    expected_cn: Option<String>,
    connect_automatically: bool,
    read_timeout_ms: u64,
    write_timeout_ms: u64,
    local_echo_threshold_ms: Option<u64>,
    remote_wezterm_path: Option<String>,
    overlay_lag_indicator: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct NativePalette {
    pub(crate) foreground: Option<Color>,
    pub(crate) background: Option<Color>,
    pub(crate) cursor_fg: Option<Color>,
    pub(crate) cursor_bg: Option<Color>,
    pub(crate) cursor_border: Option<Color>,
    #[expect(
        clippy::option_option,
        reason = "palette overrides distinguish absent, explicit nil, and concrete values"
    )]
    pub(crate) selection_fg: Option<Option<Color>>,
    pub(crate) selection_bg: Option<Color>,
    pub(crate) ansi: Option<[Color; 8]>,
    pub(crate) brights: Option<[Color; 8]>,
    indexed: [Option<Color>; 256],
    pub(crate) tab_bar_background: Option<Color>,
    pub(crate) tab_bar_inactive_tab_edge: Option<Color>,
    pub(crate) tab_bar_active_tab: NativeTabBarItemColors,
    pub(crate) tab_bar_inactive_tab: NativeTabBarItemColors,
    pub(crate) tab_bar_inactive_tab_hover: NativeTabBarItemColors,
    pub(crate) tab_bar_new_tab: NativeTabBarItemColors,
    pub(crate) tab_bar_new_tab_hover: NativeTabBarItemColors,
    scrollbar_thumb: Option<Color>,
    split: Option<Color>,
    visual_bell: Option<Color>,
    pub(crate) compose_cursor: Option<Color>,
    copy_mode_active_highlight_fg: Option<NativeColorSpec>,
    copy_mode_active_highlight_bg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_fg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_bg: Option<NativeColorSpec>,
    quick_select_label_fg: Option<NativeColorSpec>,
    quick_select_label_bg: Option<NativeColorSpec>,
    quick_select_match_fg: Option<NativeColorSpec>,
    quick_select_match_bg: Option<NativeColorSpec>,
    input_selector_label_fg: Option<NativeColorSpec>,
    input_selector_label_bg: Option<NativeColorSpec>,
    launcher_label_fg: Option<NativeColorSpec>,
    launcher_label_bg: Option<NativeColorSpec>,
}

impl Default for NativePalette {
    fn default() -> Self {
        Self {
            foreground: None,
            background: None,
            cursor_fg: None,
            cursor_bg: None,
            cursor_border: None,
            selection_fg: None,
            selection_bg: None,
            ansi: None,
            brights: None,
            indexed: [None; 256],
            tab_bar_background: None,
            tab_bar_inactive_tab_edge: None,
            tab_bar_active_tab: NativeTabBarItemColors::default(),
            tab_bar_inactive_tab: NativeTabBarItemColors::default(),
            tab_bar_inactive_tab_hover: NativeTabBarItemColors::default(),
            tab_bar_new_tab: NativeTabBarItemColors::default(),
            tab_bar_new_tab_hover: NativeTabBarItemColors::default(),
            scrollbar_thumb: None,
            split: None,
            visual_bell: None,
            compose_cursor: None,
            copy_mode_active_highlight_fg: None,
            copy_mode_active_highlight_bg: None,
            copy_mode_inactive_highlight_fg: None,
            copy_mode_inactive_highlight_bg: None,
            quick_select_label_fg: None,
            quick_select_label_bg: None,
            quick_select_match_fg: None,
            quick_select_match_bg: None,
            input_selector_label_fg: None,
            input_selector_label_bg: None,
            launcher_label_fg: None,
            launcher_label_bg: None,
        }
    }
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct NativeResolvedPalette {
    foreground: Color,
    background: Color,
    cursor_fg: Option<Color>,
    cursor_bg: Color,
    cursor_border: Option<Color>,
    selection_fg: Option<Option<Color>>,
    selection_bg: Option<Color>,
    ansi: [Color; 8],
    brights: [Color; 8],
    indexed: [Option<Color>; 256],
    tab_bar_background: Option<Color>,
    tab_bar_inactive_tab_edge: Option<Color>,
    tab_bar_active_tab: NativeTabBarItemColors,
    tab_bar_inactive_tab: NativeTabBarItemColors,
    tab_bar_inactive_tab_hover: NativeTabBarItemColors,
    tab_bar_new_tab: NativeTabBarItemColors,
    tab_bar_new_tab_hover: NativeTabBarItemColors,
    scrollbar_thumb: Option<Color>,
    split: Option<Color>,
    visual_bell: Option<Color>,
    compose_cursor: Option<Color>,
    copy_mode_active_highlight_fg: Option<NativeColorSpec>,
    copy_mode_active_highlight_bg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_fg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_bg: Option<NativeColorSpec>,
    quick_select_label_fg: Option<NativeColorSpec>,
    quick_select_label_bg: Option<NativeColorSpec>,
    quick_select_match_fg: Option<NativeColorSpec>,
    quick_select_match_bg: Option<NativeColorSpec>,
    input_selector_label_fg: Option<NativeColorSpec>,
    input_selector_label_bg: Option<NativeColorSpec>,
    launcher_label_fg: Option<NativeColorSpec>,
    launcher_label_bg: Option<NativeColorSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeTerminalLinePalette {
    foreground: Color,
    background: Color,
    ansi: [Color; 8],
    brights: [Color; 8],
    indexed: [Option<Color>; 256],
}

impl NativeResolvedPalette {
    fn terminal_line_palette(&self) -> NativeTerminalLinePalette {
        NativeTerminalLinePalette {
            foreground: self.foreground,
            background: self.background,
            ansi: self.ansi,
            brights: self.brights,
            indexed: self.indexed,
        }
    }
}

impl Default for NativeResolvedPalette {
    fn default() -> Self {
        let (ansi, brights) = native_split_ansi_palette(DEFAULT_ANSI_PALETTE_COLORS);
        Self {
            foreground: DEFAULT_FOREGROUND_COLOR,
            background: DEFAULT_BACKGROUND_COLOR,
            cursor_fg: None,
            cursor_bg: DEFAULT_CURSOR_BG_COLOR,
            cursor_border: None,
            selection_fg: None,
            selection_bg: None,
            ansi,
            brights,
            indexed: [None; 256],
            tab_bar_background: None,
            tab_bar_inactive_tab_edge: None,
            tab_bar_active_tab: NativeTabBarItemColors::default(),
            tab_bar_inactive_tab: NativeTabBarItemColors::default(),
            tab_bar_inactive_tab_hover: NativeTabBarItemColors::default(),
            tab_bar_new_tab: NativeTabBarItemColors::default(),
            tab_bar_new_tab_hover: NativeTabBarItemColors::default(),
            scrollbar_thumb: None,
            split: None,
            visual_bell: None,
            compose_cursor: None,
            copy_mode_active_highlight_fg: None,
            copy_mode_active_highlight_bg: None,
            copy_mode_inactive_highlight_fg: None,
            copy_mode_inactive_highlight_bg: None,
            quick_select_label_fg: None,
            quick_select_label_bg: None,
            quick_select_match_fg: None,
            quick_select_match_bg: None,
            input_selector_label_fg: None,
            input_selector_label_bg: None,
            launcher_label_fg: None,
            launcher_label_bg: None,
        }
    }
}

/// Returns the default palette exposed by `wezterm.color.get_default_colors()`.
///
/// This is pinned to `WezTerm` commit `093bf6b`, primarily
/// `term/src/color.rs::ColorPalette::compute_default` and the conversion in
/// `config/src/color.rs`. It intentionally does not use R-SSH's own palette
/// defaults.
#[cfg_attr(not(test), allow(dead_code))]
fn native_wezterm_default_colors_palette() -> NativeResolvedPalette {
    const ANSI_COLORS: [Color; 16] = [
        Color::Rgb(0x00, 0x00, 0x00),
        Color::Rgb(0xcc, 0x55, 0x55),
        Color::Rgb(0x55, 0xcc, 0x55),
        Color::Rgb(0xcd, 0xcd, 0x55),
        Color::Rgb(0x54, 0x55, 0xcb),
        Color::Rgb(0xcc, 0x55, 0xcc),
        Color::Rgb(0x7a, 0xca, 0xca),
        Color::Rgb(0xcc, 0xcc, 0xcc),
        Color::Rgb(0x55, 0x55, 0x55),
        Color::Rgb(0xff, 0x55, 0x55),
        Color::Rgb(0x55, 0xff, 0x55),
        Color::Rgb(0xff, 0xff, 0x55),
        Color::Rgb(0x55, 0x55, 0xff),
        Color::Rgb(0xff, 0x55, 0xff),
        Color::Rgb(0x55, 0xff, 0xff),
        Color::Rgb(0xff, 0xff, 0xff),
    ];
    const CUBE_RAMP: [u8; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

    let (ansi, brights) = native_split_ansi_palette(ANSI_COLORS);
    let mut indexed = [None; 256];
    for cube_index in 0..216 {
        let blue = CUBE_RAMP[cube_index % 6];
        let green = CUBE_RAMP[cube_index / 6 % 6];
        let red = CUBE_RAMP[cube_index / 36];
        indexed[16 + cube_index] = Some(Color::Rgb(red, green, blue));
    }
    for grey_index in 0_u8..24 {
        let grey = 8 + 10 * grey_index;
        indexed[232 + usize::from(grey_index)] = Some(Color::Rgb(grey, grey, grey));
    }

    NativeResolvedPalette {
        foreground: Color::Rgb(0xb2, 0xb2, 0xb2),
        background: Color::Rgb(0x00, 0x00, 0x00),
        cursor_fg: Some(Color::Rgb(0x00, 0x00, 0x00)),
        cursor_bg: Color::Rgb(0x52, 0xad, 0x70),
        cursor_border: Some(Color::Rgb(0x52, 0xad, 0x70)),
        selection_fg: Some(None),
        selection_bg: Some(Color::Rgba(127, 102, 153, 127)),
        ansi,
        brights,
        indexed,
        tab_bar_background: None,
        tab_bar_inactive_tab_edge: None,
        tab_bar_active_tab: NativeTabBarItemColors::default(),
        tab_bar_inactive_tab: NativeTabBarItemColors::default(),
        tab_bar_inactive_tab_hover: NativeTabBarItemColors::default(),
        tab_bar_new_tab: NativeTabBarItemColors::default(),
        tab_bar_new_tab_hover: NativeTabBarItemColors::default(),
        scrollbar_thumb: Some(Color::Rgb(0x22, 0x22, 0x22)),
        split: Some(Color::Rgb(0x44, 0x44, 0x44)),
        visual_bell: None,
        compose_cursor: None,
        copy_mode_active_highlight_fg: None,
        copy_mode_active_highlight_bg: None,
        copy_mode_inactive_highlight_fg: None,
        copy_mode_inactive_highlight_bg: None,
        quick_select_label_fg: None,
        quick_select_label_bg: None,
        quick_select_match_fg: None,
        quick_select_match_bg: None,
        input_selector_label_fg: None,
        input_selector_label_bg: None,
        launcher_label_fg: None,
        launcher_label_bg: None,
    }
}

fn apply_wezterm_default_colors_overrides(overrides: &mut NativeConfigSnapshot) -> bool {
    let palette = native_wezterm_default_colors_palette();
    overrides.foreground_color = Some(palette.foreground);
    overrides.background_color = Some(palette.background);
    overrides.cursor_fg_color = palette.cursor_fg;
    overrides.cursor_bg_color = Some(palette.cursor_bg);
    overrides.cursor_border_color = palette.cursor_border;
    overrides.selection_fg_color = palette.selection_fg;
    overrides.selection_bg_color = palette.selection_bg;
    overrides.ansi_palette = Some(std::array::from_fn(|index| {
        if index < 8 {
            palette.ansi[index]
        } else {
            palette.brights[index - 8]
        }
    }));
    overrides.indexed_palette = Some(palette.indexed);
    overrides.scrollbar_thumb_color = palette.scrollbar_thumb;
    overrides.split_color = palette.split;
    true
}

fn native_split_ansi_palette(palette: [Color; 16]) -> ([Color; 8], [Color; 8]) {
    (
        std::array::from_fn(|index| palette[index]),
        std::array::from_fn(|index| palette[index + 8]),
    )
}

fn native_tab_bar_item_colors_with_overrides(
    base: NativeTabBarItemColors,
    overrides: NativeTabBarItemColors,
) -> NativeTabBarItemColors {
    NativeTabBarItemColors {
        fg_color: overrides.fg_color.or(base.fg_color),
        bg_color: overrides.bg_color.or(base.bg_color),
        intensity: overrides.intensity.or(base.intensity),
        underline: overrides.underline.or(base.underline),
        italic: overrides.italic.or(base.italic),
        strikethrough: overrides.strikethrough.or(base.strikethrough),
    }
}

fn native_palette_from_overrides(overrides: &NativeConfigSnapshot) -> NativePalette {
    let (ansi, brights) = overrides
        .ansi_palette
        .map(native_split_ansi_palette)
        .map_or((None, None), |(ansi, brights)| (Some(ansi), Some(brights)));

    NativePalette {
        foreground: overrides.foreground_color,
        background: overrides.background_color,
        cursor_fg: overrides.cursor_fg_color,
        cursor_bg: overrides.cursor_bg_color,
        cursor_border: overrides.cursor_border_color,
        selection_fg: overrides.selection_fg_color,
        selection_bg: overrides.selection_bg_color,
        ansi,
        brights,
        indexed: overrides.indexed_palette.unwrap_or([None; 256]),
        tab_bar_background: overrides.tab_bar_background_color,
        tab_bar_inactive_tab_edge: overrides.tab_bar_inactive_tab_edge_color,
        tab_bar_active_tab: overrides.tab_bar_active_tab_colors,
        tab_bar_inactive_tab: overrides.tab_bar_inactive_tab_colors,
        tab_bar_inactive_tab_hover: overrides.tab_bar_inactive_tab_hover_colors,
        tab_bar_new_tab: overrides.tab_bar_new_tab_colors,
        tab_bar_new_tab_hover: overrides.tab_bar_new_tab_hover_colors,
        scrollbar_thumb: overrides.scrollbar_thumb_color,
        split: overrides.split_color,
        visual_bell: overrides.visual_bell_color,
        compose_cursor: overrides.compose_cursor_color,
        copy_mode_active_highlight_fg: overrides.copy_mode_active_highlight_fg,
        copy_mode_active_highlight_bg: overrides.copy_mode_active_highlight_bg,
        copy_mode_inactive_highlight_fg: overrides.copy_mode_inactive_highlight_fg,
        copy_mode_inactive_highlight_bg: overrides.copy_mode_inactive_highlight_bg,
        quick_select_label_fg: overrides.quick_select_label_fg,
        quick_select_label_bg: overrides.quick_select_label_bg,
        quick_select_match_fg: overrides.quick_select_match_fg,
        quick_select_match_bg: overrides.quick_select_match_bg,
        input_selector_label_fg: overrides.input_selector_label_fg,
        input_selector_label_bg: overrides.input_selector_label_bg,
        launcher_label_fg: overrides.launcher_label_fg,
        launcher_label_bg: overrides.launcher_label_bg,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct NativeConfigView  {

    dpi: u32,
    dpi_by_screen: BTreeMap<String, u32>,
    tab_max_width: usize,
    tab_min_width: usize,
    status_update_interval: u64,
    status_update_interval_ms: u64,
    max_fps: usize,
    animation_fps: usize,
    front_end: NativeRenderFrontEnd,
    webgpu_power_preference: NativeWebGpuPowerPreference,
    webgpu_force_fallback_adapter: bool,
    webgpu_preferred_adapter: Option<NativeWebGpuPreferredAdapter>,
    prefer_egl: bool,
    enable_wayland: bool,
    enable_zwlr_output_manager: bool,
    use_box_model_render: bool,
    experimental_pixel_positioning: bool,
    shape_cache_size: usize,
    line_state_cache_size: usize,
    line_quad_cache_size: usize,
    line_to_ele_shape_cache_size: usize,
    glyph_cache_image_cache_size: usize,
    cursor_blink_rate: u64,
    cursor_blink_rate_ms: u64,
    cursor_blink_ease_in: NativeEasingFunction,
    cursor_blink_ease_out: NativeEasingFunction,
    text_blink_rate: u64,
    text_blink_rate_ms: u64,
    text_blink_rate_rapid: u64,
    text_blink_rate_rapid_ms: u64,
    text_blink_ease_in: NativeEasingFunction,
    text_blink_ease_out: NativeEasingFunction,
    text_blink_rapid_ease_in: NativeEasingFunction,
    text_blink_rapid_ease_out: NativeEasingFunction,
    font: Option<String>,
    font_fallbacks: Vec<String>,
    font_attributes: NativeFontAttributes,
    font_rules: Vec<NativeFontRule>,
    font_size: NativeFontSize,
    cell_width: NativeCellWidth,
    cell_widths: Vec<NativeCellWidthOverride>,
    line_height: NativeLineHeight,
    font_antialias: NativeFontAntialias,
    font_hinting: NativeFontHinting,
    font_rasterizer: NativeFontRasterizer,
    font_colr_rasterizer: NativeFontRasterizer,
    font_shaper: NativeFontShaper,
    harfbuzz_features: Vec<String>,
    font_dirs: Vec<String>,
    font_locator: Option<NativeFontLocator>,
    use_cap_height_to_scale_fallback_fonts: bool,
    ignore_svg_fonts: bool,
    sort_fallback_fonts_by_coverage: bool,
    search_font_dirs_for_fallback: bool,
    next: NativeConfigView1,
}

impl Deref for NativeConfigView {
    type Target = NativeConfigView1;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeConfigView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct NativeConfigView1 {
    custom_block_glyphs: bool,
    anti_alias_custom_block_glyphs: bool,
    allow_square_glyphs_to_overflow_width: NativeSquareGlyphOverflow,
    freetype_load_target: NativeFreetypeTarget,
    freetype_render_target: NativeFreetypeTarget,
    freetype_load_flags: NativeFreetypeLoadFlags,
    freetype_interpreter_version: Option<u32>,
    freetype_pcf_long_family_names: bool,
    display_pixel_geometry: NativeDisplayPixelGeometry,
    foreground_text_hsb: NativeInactivePaneHsb,
    bold_brightens_ansi_colors: NativeBoldBrightensAnsiColors,
    text_min_contrast_ratio: Option<NativeTextMinContrastRatio>,
    text_background_opacity: NativeTextBackgroundOpacity,
    window_background_opacity: NativeTextBackgroundOpacity,
    background: Vec<NativeWindowBackgroundVisualLayer>,
    window_background_image: Option<String>,
    window_background_image_hsb: Option<NativeInactivePaneHsb>,
    window_background_gradient: Option<NativeWindowBackgroundGradient>,
    window_background_images: Vec<NativeWindowBackgroundImage>,
    window_background_layers: Vec<NativeWindowBackgroundVisualLayer>,
    kde_window_background_blur: bool,
    macos_window_background_blur: u32,
    win32_system_backdrop: NativeWin32SystemBackdrop,
    win32_acrylic_accent_color: Option<Color>,
    window_decorations: NativeWindowDecorations,
    window_frame: NativeWindowFrameAppearance,
    window_frame_appearance: NativeWindowFrameAppearance,
    integrated_title_buttons: Vec<NativeIntegratedTitleButton>,
    integrated_title_button_alignment: NativeIntegratedTitleButtonAlignment,
    integrated_title_button_color: NativeIntegratedTitleButtonColor,
    integrated_title_button_style: NativeIntegratedTitleButtonStyle,
    default_cursor_style: NativeCursorStyle,
    cursor_thickness: Option<NativeCursorThickness>,
    underline_thickness: Option<NativeUnderlineThickness>,
    underline_position: Option<NativeUnderlinePosition>,
    strikethrough_position: Option<NativeStrikethroughPosition>,
    force_reverse_video_cursor: bool,
    reverse_video_cursor_min_contrast: NativeContrastRatio,
    window_padding: NativeWindowPadding,
    window_content_alignment: NativeWindowContentAlignment,
    initial_cols: u16,
    initial_rows: u16,
    inactive_pane_hsb: NativeInactivePaneHsb,
    command_palette_rows: Option<usize>,
    command_palette_font: Option<NativeFontConfig>,
    command_palette_font_size: NativeFontSize,
    command_palette_bg_color: Option<Color>,
    command_palette_fg_color: Option<Color>,
    char_select_font: Option<NativeFontConfig>,
    char_select_font_size: NativeFontSize,
    char_select_bg_color: Option<Color>,
    char_select_fg_color: Option<Color>,
    pane_select_font: Option<NativeFontConfig>,
    next: NativeConfigView2,
}

impl Deref for NativeConfigView1 {
    type Target = NativeConfigView2;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeConfigView1 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct NativeConfigView2 {
    pane_select_font_size: NativeFontSize,
    pane_select_bg_color: Option<Color>,
    pane_select_fg_color: Option<Color>,
    launcher_alphabet: String,
    quick_select_alphabet: String,
    quick_select_patterns: Vec<String>,
    disable_default_quick_select_patterns: bool,
    quick_select_remove_styling: bool,
    hyperlink_rules: Vec<NativeHyperlinkRule>,
    copy_mode_active_highlight_bg: Option<NativeColorSpec>,
    copy_mode_active_highlight_fg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_bg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_fg: Option<NativeColorSpec>,
    quick_select_label_bg: Option<NativeColorSpec>,
    quick_select_label_fg: Option<NativeColorSpec>,
    quick_select_match_bg: Option<NativeColorSpec>,
    quick_select_match_fg: Option<NativeColorSpec>,
    input_selector_label_bg: Option<NativeColorSpec>,
    input_selector_label_fg: Option<NativeColorSpec>,
    launcher_label_bg: Option<NativeColorSpec>,
    launcher_label_fg: Option<NativeColorSpec>,
    selection_word_boundary: String,
    term: String,
    enq_answerback: String,
    audible_bell: NativeAudibleBell,
    visual_bell: NativeVisualBell,
    colors: Option<Box<NativePalette>>,
    color_scheme: Option<String>,
    color_scheme_dirs: Vec<String>,
    color_schemes: HashMap<String, NativeResolvedPalette>,
    resolved_palette: NativeResolvedPalette,
    foreground_color: Color,
    background_color: Color,
    ansi_palette: Option<[Color; 16]>,
    indexed_palette: Option<[Option<Color>; 256]>,
    #[expect(
        clippy::option_option,
        reason = "effective configuration preserves both inherited and explicit selection clearing"
    )]
    selection_fg_color: Option<Option<Color>>,
    selection_bg_color: Option<Color>,
    cursor_bg_color: Color,
    cursor_border_color: Option<Color>,
    cursor_fg_color: Option<Color>,
    compose_cursor_color: Option<Color>,
    split_color: Option<Color>,
    scrollbar_thumb_color: Option<Color>,
    tab_bar_background_color: Option<Color>,
    tab_bar_inactive_tab_edge_color: Option<Color>,
    tab_bar_active_tab_colors: NativeTabBarItemColors,
    tab_bar_inactive_tab_colors: NativeTabBarItemColors,
    tab_bar_inactive_tab_hover_colors: NativeTabBarItemColors,
    tab_bar_new_tab_colors: NativeTabBarItemColors,
    tab_bar_new_tab_hover_colors: NativeTabBarItemColors,
    tab_bar_style: NativeTabBarStyle,
    visual_bell_color: Option<Color>,
    notification_handling: NativeNotificationHandling,
    next: NativeConfigView3,
}

impl Deref for NativeConfigView2 {
    type Target = NativeConfigView3;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeConfigView2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct NativeConfigView3 {
    default_prog: Option<Vec<String>>,
    default_gui_startup_args: Vec<String>,
    default_domain: String,
    default_workspace: String,
    prefer_to_spawn_tabs: bool,
    automatically_reload_config: bool,
    check_for_updates: bool,
    check_for_updates_interval_seconds: u64,
    show_update_window: bool,
    native_macos_fullscreen_mode: bool,
    macos_fullscreen_extend_behind_notch: bool,
    use_resize_increments: bool,
    debug_key_events: bool,
    log_unknown_escape_sequences: bool,
    warn_about_missing_glyphs: bool,
    default_cwd: Option<String>,
    default_ssh_auth_sock: Option<String>,
    default_mux_server_domain: Option<String>,
    daemon_options: NativeDaemonOptions,
    exec_domains: Vec<NativeExecDomain>,
    wsl_domains: Vec<NativeWslDomain>,
    unix_domains: Vec<NativeUnixDomain>,
    ssh_domains: Vec<NativeSshDomain>,
    tls_servers: Vec<NativeTlsServerDomain>,
    tls_clients: Vec<NativeTlsClientDomain>,
    serial_ports: Vec<NativeSerialDomain>,
    mux_enable_ssh_agent: bool,
    ssh_backend: NativeSshBackend,
    ratelimit_mux_line_prefetches_per_second: u32,
    mux_output_parser_buffer_size: usize,
    mux_output_parser_coalesce_delay_ms: u64,
    periodic_stat_logging: u64,
    ulimit_nofile: u64,
    ulimit_nproc: u64,
    mux_env_remove: Vec<String>,
    tiling_desktop_environments: Vec<String>,
    set_environment_variables: BTreeMap<String, String>,
    launch_menu: Vec<NativeLaunchMenuItem>,
    leader: Option<NativeLeaderKey>,
    keys: Vec<NativeUserKeyAssignment>,
    key_tables: BTreeMap<String, Vec<NativeUserKeyAssignment>>,
    mouse_bindings: Vec<NativeUserMouseAssignment>,
    key_map_preference: NativeKeyMapPreference,
    ui_key_cap_rendering: NativeUiKeyCapRendering,
    swap_backspace_and_delete: bool,
    enable_kitty_graphics: bool,
    enable_checksum_rectangular_area: bool,
    enable_title_reporting: bool,
    enable_csi_u_key_encoding: bool,
    enable_kitty_keyboard: bool,
    allow_download_protocols: bool,
    xcursor_theme: Option<String>,
    xcursor_size: Option<u32>,
    next: NativeConfigView4,
}

impl Deref for NativeConfigView3 {
    type Target = NativeConfigView4;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeConfigView3 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct NativeConfigView4 {
    palette_max_key_assigments_for_action: usize,
    allow_win32_input_mode: bool,
    treat_left_ctrlalt_as_altgr: bool,
    send_composed_key_when_left_alt_is_pressed: bool,
    send_composed_key_when_right_alt_is_pressed: bool,
    treat_east_asian_ambiguous_width_as_wide: bool,
    normalize_output_to_unicode_nfc: bool,
    unicode_version: u32,
    bidi_enabled: bool,
    bidi_direction: NativeBidiDirection,
    use_ime: bool,
    use_dead_keys: bool,
    ime_preedit_rendering: NativeImePreeditRendering,
    macos_forward_to_ime_modifier_mask: ModifiersState,
    xim_im_name: Option<String>,
    detect_password_input: bool,
    scroll_to_bottom_on_input: bool,
    adjust_window_size_when_changing_font_size: bool,
    canonicalize_pasted_newlines: NativeCanonicalizePastedNewlines,
    quote_dropped_files: NativeQuoteDroppedFiles,
    disable_default_key_bindings: bool,
    disable_default_mouse_bindings: bool,
    hide_mouse_cursor_when_typing: bool,
    alternate_buffer_wheel_scroll_speed: usize,
    pane_focus_follows_mouse: bool,
    swallow_mouse_click_on_pane_focus: bool,
    swallow_mouse_click_on_window_focus: bool,
    bypass_mouse_reporting_modifiers: ModifiersState,
    enable_scroll_bar: bool,
    scrollback_lines: usize,
    min_scroll_bar_height: Option<NativeScrollBarHeight>,
    enable_tab_bar: bool,
    hide_tab_bar_if_only_one_tab: bool,
    use_fancy_tab_bar: bool,
    unzoom_on_switch_pane: bool,
    tab_bar_at_bottom: bool,
    tab_and_split_indices_are_zero_based: bool,
    mouse_wheel_scrolls_tabs: bool,
    switch_to_last_active_tab_when_closing_tab: bool,
    tab_shortcut_style: NativeTabShortcutStyle,
    closed_tab_history_size: usize,
    close_tab_selection: CloseTabSelection,
    tab_bar_wheel_behavior: NativeTabBarWheelBehavior,
    quit_when_all_windows_are_closed: bool,
    window_close_confirmation: NativeWindowCloseConfirmation,
    exit_behavior: NativeExitBehavior,
    clean_exit_codes: Vec<u32>,
    exit_behavior_messaging: NativeExitBehaviorMessaging,
    skip_close_confirmation_for_processes_named: Vec<String>,
    show_close_tab_button_in_tabs: bool,
    show_new_tab_button_in_tab_bar: bool,
    show_tab_index_in_tab_bar: bool,
    show_tabs_in_tab_bar: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NativeConfigSnapshot {
    effective: Arc<rssh_config::EffectiveConfig>,
    dpi: Option<u32>,
    dpi_by_screen: Option<BTreeMap<String, u32>>,
    tab_max_width: Option<usize>,
    tab_min_width: Option<usize>,
    status_update_interval_ms: Option<u64>,
    pub(crate) max_fps: Option<usize>,
    animation_fps: Option<usize>,
    front_end: Option<NativeRenderFrontEnd>,
    webgpu_power_preference: Option<NativeWebGpuPowerPreference>,
    webgpu_force_fallback_adapter: Option<bool>,
    webgpu_preferred_adapter: Option<NativeWebGpuPreferredAdapter>,
    prefer_egl: Option<bool>,
    enable_wayland: Option<bool>,
    enable_zwlr_output_manager: Option<bool>,
    use_box_model_render: Option<bool>,
    experimental_pixel_positioning: Option<bool>,
    shape_cache_size: Option<usize>,
    line_state_cache_size: Option<usize>,
    line_quad_cache_size: Option<usize>,
    line_to_ele_shape_cache_size: Option<usize>,
    glyph_cache_image_cache_size: Option<usize>,
    cursor_blink_rate_ms: Option<u64>,
    cursor_blink_ease_in: Option<NativeEasingFunction>,
    cursor_blink_ease_out: Option<NativeEasingFunction>,
    text_blink_rate_ms: Option<u64>,
    text_blink_rate_rapid_ms: Option<u64>,
    text_blink_ease_in: Option<NativeEasingFunction>,
    text_blink_ease_out: Option<NativeEasingFunction>,
    text_blink_rapid_ease_in: Option<NativeEasingFunction>,
    text_blink_rapid_ease_out: Option<NativeEasingFunction>,
    font: Option<String>,
    font_fallbacks: Option<Vec<String>>,
    font_attributes: Option<NativeFontAttributes>,
    font_rules: Option<Vec<NativeFontRule>>,
    font_size: Option<NativeFontSize>,
    cell_width: Option<NativeCellWidth>,
    cell_widths: Option<Vec<NativeCellWidthOverride>>,
    line_height: Option<NativeLineHeight>,
    font_antialias: Option<NativeFontAntialias>,
    font_hinting: Option<NativeFontHinting>,
    font_rasterizer: Option<NativeFontRasterizer>,
    font_colr_rasterizer: Option<NativeFontRasterizer>,
    font_shaper: Option<NativeFontShaper>,
    harfbuzz_features: Option<Vec<String>>,
    font_dirs: Option<Vec<String>>,
    font_locator: Option<NativeFontLocator>,
    use_cap_height_to_scale_fallback_fonts: Option<bool>,
    ignore_svg_fonts: Option<bool>,
    sort_fallback_fonts_by_coverage: Option<bool>,
    search_font_dirs_for_fallback: Option<bool>,
    custom_block_glyphs: Option<bool>,
    anti_alias_custom_block_glyphs: Option<bool>,
    allow_square_glyphs_to_overflow_width: Option<NativeSquareGlyphOverflow>,
    next: NativeConfigSnapshot1,
}

impl Deref for NativeConfigSnapshot {
    type Target = NativeConfigSnapshot1;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeConfigSnapshot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NativeConfigSnapshot1 {
    freetype_load_target: Option<NativeFreetypeTarget>,
    freetype_render_target: Option<NativeFreetypeTarget>,
    freetype_load_flags: Option<NativeFreetypeLoadFlags>,
    freetype_interpreter_version: Option<u32>,
    freetype_pcf_long_family_names: Option<bool>,
    display_pixel_geometry: Option<NativeDisplayPixelGeometry>,
    foreground_text_hsb: Option<NativeInactivePaneHsb>,
    bold_brightens_ansi_colors: Option<NativeBoldBrightensAnsiColors>,
    text_min_contrast_ratio: Option<NativeTextMinContrastRatio>,
    text_background_opacity: Option<NativeTextBackgroundOpacity>,
    window_background_opacity: Option<NativeTextBackgroundOpacity>,
    background: Option<Vec<NativeWindowBackgroundVisualLayer>>,
    window_background_image: Option<String>,
    window_background_image_hsb: Option<NativeInactivePaneHsb>,
    window_background_gradient: Option<NativeWindowBackgroundGradient>,
    window_background_images: Option<Vec<NativeWindowBackgroundImage>>,
    window_background_layers: Option<Vec<NativeWindowBackgroundVisualLayer>>,
    kde_window_background_blur: Option<bool>,
    macos_window_background_blur: Option<u32>,
    win32_system_backdrop: Option<NativeWin32SystemBackdrop>,
    win32_acrylic_accent_color: Option<Color>,
    window_decorations: Option<NativeWindowDecorations>,
    window_frame_appearance: Option<NativeWindowFrameAppearance>,
    integrated_title_buttons: Option<Vec<NativeIntegratedTitleButton>>,
    integrated_title_button_alignment: Option<NativeIntegratedTitleButtonAlignment>,
    integrated_title_button_color: Option<NativeIntegratedTitleButtonColor>,
    integrated_title_button_style: Option<NativeIntegratedTitleButtonStyle>,
    default_cursor_style: Option<NativeCursorStyle>,
    cursor_thickness: Option<NativeCursorThickness>,
    underline_thickness: Option<NativeUnderlineThickness>,
    underline_position: Option<NativeUnderlinePosition>,
    strikethrough_position: Option<NativeStrikethroughPosition>,
    force_reverse_video_cursor: Option<bool>,
    reverse_video_cursor_min_contrast: Option<NativeContrastRatio>,
    window_padding: Option<NativeWindowPadding>,
    window_content_alignment: Option<NativeWindowContentAlignment>,
    pub(crate) initial_cols: Option<u16>,
    pub(crate) initial_rows: Option<u16>,
    inactive_pane_hsb: Option<NativeInactivePaneHsb>,
    command_palette_rows: Option<usize>,
    command_palette_font: Option<NativeFontConfig>,
    command_palette_font_size: Option<NativeFontSize>,
    command_palette_bg_color: Option<Color>,
    command_palette_fg_color: Option<Color>,
    char_select_font: Option<NativeFontConfig>,
    char_select_font_size: Option<NativeFontSize>,
    char_select_bg_color: Option<Color>,
    char_select_fg_color: Option<Color>,
    pane_select_font: Option<NativeFontConfig>,
    pane_select_font_size: Option<NativeFontSize>,
    pane_select_bg_color: Option<Color>,
    pane_select_fg_color: Option<Color>,
    launcher_alphabet: Option<String>,
    quick_select_alphabet: Option<String>,
    next: NativeConfigSnapshot2,
}

impl Deref for NativeConfigSnapshot1 {
    type Target = NativeConfigSnapshot2;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeConfigSnapshot1 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NativeConfigSnapshot2 {
    quick_select_patterns: Option<Vec<String>>,
    disable_default_quick_select_patterns: Option<bool>,
    quick_select_remove_styling: Option<bool>,
    hyperlink_rules: Option<Vec<NativeHyperlinkRule>>,
    copy_mode_active_highlight_bg: Option<NativeColorSpec>,
    copy_mode_active_highlight_fg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_bg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_fg: Option<NativeColorSpec>,
    quick_select_label_bg: Option<NativeColorSpec>,
    quick_select_label_fg: Option<NativeColorSpec>,
    quick_select_match_bg: Option<NativeColorSpec>,
    quick_select_match_fg: Option<NativeColorSpec>,
    input_selector_label_bg: Option<NativeColorSpec>,
    input_selector_label_fg: Option<NativeColorSpec>,
    launcher_label_bg: Option<NativeColorSpec>,
    launcher_label_fg: Option<NativeColorSpec>,
    selection_word_boundary: Option<String>,
    pub(crate) term: Option<String>,
    enq_answerback: Option<String>,
    audible_bell: Option<NativeAudibleBell>,
    visual_bell: Option<NativeVisualBell>,
    pub(crate) colors: Option<Box<NativePalette>>,
    pub(crate) color_scheme: Option<String>,
    color_scheme_dirs: Option<Vec<String>>,
    color_schemes: Option<HashMap<String, NativeResolvedPalette>>,
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    ansi_palette: Option<[Color; 16]>,
    indexed_palette: Option<[Option<Color>; 256]>,
    selection_fg_color: Option<Option<Color>>,
    selection_bg_color: Option<Color>,
    cursor_bg_color: Option<Color>,
    cursor_border_color: Option<Color>,
    cursor_fg_color: Option<Color>,
    compose_cursor_color: Option<Color>,
    split_color: Option<Color>,
    scrollbar_thumb_color: Option<Color>,
    tab_bar_background_color: Option<Color>,
    tab_bar_inactive_tab_edge_color: Option<Color>,
    tab_bar_active_tab_colors: NativeTabBarItemColors,
    tab_bar_inactive_tab_colors: NativeTabBarItemColors,
    tab_bar_inactive_tab_hover_colors: NativeTabBarItemColors,
    tab_bar_new_tab_colors: NativeTabBarItemColors,
    tab_bar_new_tab_hover_colors: NativeTabBarItemColors,
    tab_bar_style: NativeTabBarStyle,
    visual_bell_color: Option<Color>,
    notification_handling: Option<NativeNotificationHandling>,
    pub(crate) default_prog: Option<Vec<String>>,
    pub(crate) default_gui_startup_args: Option<Vec<String>>,
    default_domain: Option<String>,
    default_workspace: Option<String>,
    prefer_to_spawn_tabs: Option<bool>,
    pub(crate) automatically_reload_config: Option<bool>,
    check_for_updates: Option<bool>,
    next: NativeConfigSnapshot3,
}

impl Deref for NativeConfigSnapshot2 {
    type Target = NativeConfigSnapshot3;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeConfigSnapshot2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NativeConfigSnapshot3 {
    check_for_updates_interval_seconds: Option<u64>,
    show_update_window: Option<bool>,
    native_macos_fullscreen_mode: Option<bool>,
    macos_fullscreen_extend_behind_notch: Option<bool>,
    use_resize_increments: Option<bool>,
    debug_key_events: Option<bool>,
    log_unknown_escape_sequences: Option<bool>,
    warn_about_missing_glyphs: Option<bool>,
    pub(crate) default_cwd: Option<String>,
    default_ssh_auth_sock: Option<String>,
    default_mux_server_domain: Option<String>,
    daemon_options: Option<NativeDaemonOptions>,
    exec_domains: Option<Vec<NativeExecDomain>>,
    wsl_domains: Option<Vec<NativeWslDomain>>,
    unix_domains: Option<Vec<NativeUnixDomain>>,
    ssh_domains: Option<Vec<NativeSshDomain>>,
    tls_servers: Option<Vec<NativeTlsServerDomain>>,
    tls_clients: Option<Vec<NativeTlsClientDomain>>,
    serial_ports: Option<Vec<NativeSerialDomain>>,
    mux_enable_ssh_agent: Option<bool>,
    ssh_backend: Option<NativeSshBackend>,
    ratelimit_mux_line_prefetches_per_second: Option<u32>,
    mux_output_parser_buffer_size: Option<usize>,
    mux_output_parser_coalesce_delay_ms: Option<u64>,
    periodic_stat_logging: Option<u64>,
    ulimit_nofile: Option<u64>,
    ulimit_nproc: Option<u64>,
    mux_env_remove: Option<Vec<String>>,
    tiling_desktop_environments: Option<Vec<String>>,
    pub(crate) set_environment_variables: Option<BTreeMap<String, String>>,
    launch_menu: Option<Vec<NativeLaunchMenuItem>>,
    key_map_preference: Option<NativeKeyMapPreference>,
    ui_key_cap_rendering: Option<NativeUiKeyCapRendering>,
    swap_backspace_and_delete: Option<bool>,
    enable_kitty_graphics: Option<bool>,
    enable_checksum_rectangular_area: Option<bool>,
    enable_title_reporting: Option<bool>,
    enable_csi_u_key_encoding: Option<bool>,
    enable_kitty_keyboard: Option<bool>,
    allow_download_protocols: Option<bool>,
    xcursor_theme: Option<String>,
    xcursor_size: Option<u32>,
    palette_max_key_assigments_for_action: Option<usize>,
    allow_win32_input_mode: Option<bool>,
    treat_left_ctrlalt_as_altgr: Option<bool>,
    send_composed_key_when_left_alt_is_pressed: Option<bool>,
    send_composed_key_when_right_alt_is_pressed: Option<bool>,
    treat_east_asian_ambiguous_width_as_wide: Option<bool>,
    normalize_output_to_unicode_nfc: Option<bool>,
    unicode_version: Option<u32>,
    bidi_enabled: Option<bool>,
    bidi_direction: Option<NativeBidiDirection>,
    use_ime: Option<bool>,
    use_dead_keys: Option<bool>,
    next: NativeConfigSnapshot4,
}

impl Deref for NativeConfigSnapshot3 {
    type Target = NativeConfigSnapshot4;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeConfigSnapshot3 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NativeConfigSnapshot4 {
    ime_preedit_rendering: Option<NativeImePreeditRendering>,
    macos_forward_to_ime_modifier_mask: Option<ModifiersState>,
    xim_im_name: Option<String>,
    detect_password_input: Option<bool>,
    leader: Option<NativeLeaderKey>,
    pub(crate) key_assignments: Option<Vec<NativeUserKeyAssignment>>,
    key_tables: Option<BTreeMap<String, Vec<NativeUserKeyAssignment>>>,
    mouse_assignments: Option<Vec<NativeUserMouseAssignment>>,
    lua_tab_title: Option<NativeLuaTabTitle>,
    lua_window_title: Option<NativeLuaWindowTitle>,
    lua_update_status: Option<NativeLuaWindowStatusUpdate>,
    lua_update_status_config_overrides: Option<NativeWindowConfigPatch>,
    lua_bell: Option<NativeLuaWindowStatusUpdate>,
    lua_focus_changed: Option<NativeLuaWindowStatusUpdate>,
    lua_resized: Option<NativeLuaWindowStatusUpdate>,
    lua_config_reloaded: Option<NativeLuaWindowStatusUpdate>,
    lua_user_var_changed: Option<NativeLuaUserVarChanged>,
    lua_open_uri: Option<NativeLuaOpenUri>,
    lua_new_tab_button_click: Option<NativeLuaNewTabButtonClick>,
    lua_command_palette_entries: Option<Vec<NativeCommandPaletteEntry>>,
    lua_emit_event_handlers: Option<BTreeMap<String, Vec<NativeLuaEmitEventHandler>>>,
    scroll_to_bottom_on_input: Option<bool>,
    adjust_window_size_when_changing_font_size: Option<bool>,
    canonicalize_pasted_newlines: Option<NativeCanonicalizePastedNewlines>,
    quote_dropped_files: Option<NativeQuoteDroppedFiles>,
    disable_default_key_bindings: Option<bool>,
    disable_default_mouse_bindings: Option<bool>,
    hide_mouse_cursor_when_typing: Option<bool>,
    alternate_buffer_wheel_scroll_speed: Option<usize>,
    pane_focus_follows_mouse: Option<bool>,
    swallow_mouse_click_on_pane_focus: Option<bool>,
    swallow_mouse_click_on_window_focus: Option<bool>,
    bypass_mouse_reporting_modifiers: Option<ModifiersState>,
    enable_scroll_bar: Option<bool>,
    pub(crate) scrollback_lines: Option<usize>,
    min_scroll_bar_height: Option<NativeScrollBarHeight>,
    pub(crate) enable_tab_bar: Option<bool>,
    hide_tab_bar_if_only_one_tab: Option<bool>,
    use_fancy_tab_bar: Option<bool>,
    unzoom_on_switch_pane: Option<bool>,
    tab_bar_at_bottom: Option<bool>,
    tab_and_split_indices_are_zero_based: Option<bool>,
    mouse_wheel_scrolls_tabs: Option<bool>,
    switch_to_last_active_tab_when_closing_tab: Option<bool>,
    tab_shortcut_style: Option<NativeTabShortcutStyle>,
    closed_tab_history_size: Option<usize>,
    close_tab_selection: Option<CloseTabSelection>,
    tab_bar_wheel_behavior: Option<NativeTabBarWheelBehavior>,
    quit_when_all_windows_are_closed: Option<bool>,
    window_close_confirmation: Option<NativeWindowCloseConfirmation>,
    exit_behavior: Option<NativeExitBehavior>,
    clean_exit_codes: Option<Vec<u32>>,
    exit_behavior_messaging: Option<NativeExitBehaviorMessaging>,
    skip_close_confirmation_for_processes_named: Option<Vec<String>>,
    show_close_tab_button_in_tabs: Option<bool>,
    show_new_tab_button_in_tab_bar: Option<bool>,
    show_tab_index_in_tab_bar: Option<bool>,
    show_tabs_in_tab_bar: Option<bool>,
}

impl NativeConfigSnapshot {
    fn write_patch_values(self, values: &mut NativeWindowConfigPatchValues) {
        values.dpi = self.dpi;
        values.dpi_by_screen = self.dpi_by_screen;
        values.tab_max_width = self.tab_max_width;
        values.tab_min_width = self.tab_min_width;
        values.status_update_interval_ms = self.status_update_interval_ms;
        values.max_fps = self.max_fps;
        values.animation_fps = self.animation_fps;
        values.front_end = self.front_end;
        values.webgpu_power_preference = self.webgpu_power_preference;
        values.webgpu_force_fallback_adapter = self.webgpu_force_fallback_adapter;
        values.webgpu_preferred_adapter = self.webgpu_preferred_adapter;
        values.prefer_egl = self.prefer_egl;
        values.enable_wayland = self.enable_wayland;
        values.enable_zwlr_output_manager = self.enable_zwlr_output_manager;
        values.use_box_model_render = self.use_box_model_render;
        values.experimental_pixel_positioning = self.experimental_pixel_positioning;
        values.shape_cache_size = self.shape_cache_size;
        values.line_state_cache_size = self.line_state_cache_size;
        values.line_quad_cache_size = self.line_quad_cache_size;
        values.line_to_ele_shape_cache_size = self.line_to_ele_shape_cache_size;
        values.glyph_cache_image_cache_size = self.glyph_cache_image_cache_size;
        values.cursor_blink_rate_ms = self.cursor_blink_rate_ms;
        values.cursor_blink_ease_in = self.cursor_blink_ease_in;
        values.cursor_blink_ease_out = self.cursor_blink_ease_out;
        values.text_blink_rate_ms = self.text_blink_rate_ms;
        values.text_blink_rate_rapid_ms = self.text_blink_rate_rapid_ms;
        values.text_blink_ease_in = self.text_blink_ease_in;
        values.text_blink_ease_out = self.text_blink_ease_out;
        values.text_blink_rapid_ease_in = self.text_blink_rapid_ease_in;
        values.text_blink_rapid_ease_out = self.text_blink_rapid_ease_out;
        values.font = self.font;
        values.font_fallbacks = self.font_fallbacks;
        values.font_attributes = self.font_attributes;
        values.font_rules = self.font_rules;
        values.font_size = self.font_size;
        values.cell_width = self.cell_width;
        values.cell_widths = self.cell_widths;
        values.line_height = self.line_height;
        values.font_antialias = self.font_antialias;
        values.font_hinting = self.font_hinting;
        values.font_rasterizer = self.font_rasterizer;
        values.font_colr_rasterizer = self.font_colr_rasterizer;
        values.font_shaper = self.font_shaper;
        values.harfbuzz_features = self.harfbuzz_features;
        values.font_dirs = self.font_dirs;
        values.font_locator = self.font_locator;
        values.use_cap_height_to_scale_fallback_fonts = self.use_cap_height_to_scale_fallback_fonts;
        values.ignore_svg_fonts = self.ignore_svg_fonts;
        values.sort_fallback_fonts_by_coverage = self.sort_fallback_fonts_by_coverage;
        values.search_font_dirs_for_fallback = self.search_font_dirs_for_fallback;
        values.custom_block_glyphs = self.custom_block_glyphs;
        values.anti_alias_custom_block_glyphs = self.anti_alias_custom_block_glyphs;
        values.allow_square_glyphs_to_overflow_width = self.allow_square_glyphs_to_overflow_width;
        self.next.write_patch_values(values);
    }
}

impl NativeConfigSnapshot1 {
    fn write_patch_values(self, values: &mut NativeWindowConfigPatchValues) {
        values.freetype_load_target = self.freetype_load_target;
        values.freetype_render_target = self.freetype_render_target;
        values.freetype_load_flags = self.freetype_load_flags;
        values.freetype_interpreter_version = self.freetype_interpreter_version;
        values.freetype_pcf_long_family_names = self.freetype_pcf_long_family_names;
        values.display_pixel_geometry = self.display_pixel_geometry;
        values.foreground_text_hsb = self.foreground_text_hsb;
        values.bold_brightens_ansi_colors = self.bold_brightens_ansi_colors;
        values.text_min_contrast_ratio = self.text_min_contrast_ratio;
        values.text_background_opacity = self.text_background_opacity;
        values.window_background_opacity = self.window_background_opacity;
        values.background = self.background;
        values.window_background_image = self.window_background_image;
        values.window_background_image_hsb = self.window_background_image_hsb;
        values.window_background_gradient = self.window_background_gradient;
        values.window_background_images = self.window_background_images;
        values.window_background_layers = self.window_background_layers;
        values.kde_window_background_blur = self.kde_window_background_blur;
        values.macos_window_background_blur = self.macos_window_background_blur;
        values.win32_system_backdrop = self.win32_system_backdrop;
        values.win32_acrylic_accent_color = self.win32_acrylic_accent_color;
        values.window_decorations = self.window_decorations;
        values.window_frame_appearance = self.window_frame_appearance;
        values.integrated_title_buttons = self.integrated_title_buttons;
        values.integrated_title_button_alignment = self.integrated_title_button_alignment;
        values.integrated_title_button_color = self.integrated_title_button_color;
        values.integrated_title_button_style = self.integrated_title_button_style;
        values.default_cursor_style = self.default_cursor_style;
        values.cursor_thickness = self.cursor_thickness;
        values.underline_thickness = self.underline_thickness;
        values.underline_position = self.underline_position;
        values.strikethrough_position = self.strikethrough_position;
        values.force_reverse_video_cursor = self.force_reverse_video_cursor;
        values.reverse_video_cursor_min_contrast = self.reverse_video_cursor_min_contrast;
        values.window_padding = self.window_padding;
        values.window_content_alignment = self.window_content_alignment;
        values.initial_cols = self.initial_cols;
        values.initial_rows = self.initial_rows;
        values.inactive_pane_hsb = self.inactive_pane_hsb;
        values.command_palette_rows = self.command_palette_rows;
        values.command_palette_font = self.command_palette_font;
        values.command_palette_font_size = self.command_palette_font_size;
        values.command_palette_bg_color = self.command_palette_bg_color;
        values.command_palette_fg_color = self.command_palette_fg_color;
        values.char_select_font = self.char_select_font;
        values.char_select_font_size = self.char_select_font_size;
        values.char_select_bg_color = self.char_select_bg_color;
        values.char_select_fg_color = self.char_select_fg_color;
        values.pane_select_font = self.pane_select_font;
        values.pane_select_font_size = self.pane_select_font_size;
        values.pane_select_bg_color = self.pane_select_bg_color;
        values.pane_select_fg_color = self.pane_select_fg_color;
        values.launcher_alphabet = self.launcher_alphabet;
        values.quick_select_alphabet = self.quick_select_alphabet;
        self.next.write_patch_values(values);
    }
}

impl NativeConfigSnapshot2 {
    fn write_patch_values(self, values: &mut NativeWindowConfigPatchValues) {
        values.quick_select_patterns = self.quick_select_patterns;
        values.disable_default_quick_select_patterns = self.disable_default_quick_select_patterns;
        values.quick_select_remove_styling = self.quick_select_remove_styling;
        values.hyperlink_rules = self.hyperlink_rules;
        values.copy_mode_active_highlight_bg = self.copy_mode_active_highlight_bg;
        values.copy_mode_active_highlight_fg = self.copy_mode_active_highlight_fg;
        values.copy_mode_inactive_highlight_bg = self.copy_mode_inactive_highlight_bg;
        values.copy_mode_inactive_highlight_fg = self.copy_mode_inactive_highlight_fg;
        values.quick_select_label_bg = self.quick_select_label_bg;
        values.quick_select_label_fg = self.quick_select_label_fg;
        values.quick_select_match_bg = self.quick_select_match_bg;
        values.quick_select_match_fg = self.quick_select_match_fg;
        values.input_selector_label_bg = self.input_selector_label_bg;
        values.input_selector_label_fg = self.input_selector_label_fg;
        values.launcher_label_bg = self.launcher_label_bg;
        values.launcher_label_fg = self.launcher_label_fg;
        values.selection_word_boundary = self.selection_word_boundary;
        values.term = self.term;
        values.enq_answerback = self.enq_answerback;
        values.audible_bell = self.audible_bell;
        values.visual_bell = self.visual_bell;
        values.colors = self.colors;
        values.color_scheme = self.color_scheme;
        values.color_scheme_dirs = self.color_scheme_dirs;
        values.color_schemes = self.color_schemes;
        values.foreground_color = self.foreground_color;
        values.background_color = self.background_color;
        values.ansi_palette = self.ansi_palette;
        values.indexed_palette = self.indexed_palette;
        values.selection_fg_color = self.selection_fg_color;
        values.selection_bg_color = self.selection_bg_color;
        values.cursor_bg_color = self.cursor_bg_color;
        values.cursor_border_color = self.cursor_border_color;
        values.cursor_fg_color = self.cursor_fg_color;
        values.compose_cursor_color = self.compose_cursor_color;
        values.split_color = self.split_color;
        values.scrollbar_thumb_color = self.scrollbar_thumb_color;
        values.tab_bar_background_color = self.tab_bar_background_color;
        values.tab_bar_inactive_tab_edge_color = self.tab_bar_inactive_tab_edge_color;
        let tab_bar_active_tab_colors = self.tab_bar_active_tab_colors;
        values.tab_bar_active_tab_colors = (tab_bar_active_tab_colors != NativeTabBarItemColors::default()).then_some(tab_bar_active_tab_colors);
        let tab_bar_inactive_tab_colors = self.tab_bar_inactive_tab_colors;
        values.tab_bar_inactive_tab_colors = (tab_bar_inactive_tab_colors != NativeTabBarItemColors::default()).then_some(tab_bar_inactive_tab_colors);
        let tab_bar_inactive_tab_hover_colors = self.tab_bar_inactive_tab_hover_colors;
        values.tab_bar_inactive_tab_hover_colors = (tab_bar_inactive_tab_hover_colors != NativeTabBarItemColors::default()).then_some(tab_bar_inactive_tab_hover_colors);
        let tab_bar_new_tab_colors = self.tab_bar_new_tab_colors;
        values.tab_bar_new_tab_colors = (tab_bar_new_tab_colors != NativeTabBarItemColors::default()).then_some(tab_bar_new_tab_colors);
        let tab_bar_new_tab_hover_colors = self.tab_bar_new_tab_hover_colors;
        values.tab_bar_new_tab_hover_colors = (tab_bar_new_tab_hover_colors != NativeTabBarItemColors::default()).then_some(tab_bar_new_tab_hover_colors);
        let tab_bar_style = self.tab_bar_style;
        values.tab_bar_style = (!tab_bar_style.is_empty()).then_some(tab_bar_style);
        values.visual_bell_color = self.visual_bell_color;
        values.notification_handling = self.notification_handling;
        values.default_prog = self.default_prog;
        values.default_gui_startup_args = self.default_gui_startup_args;
        values.default_domain = self.default_domain;
        values.default_workspace = self.default_workspace;
        values.prefer_to_spawn_tabs = self.prefer_to_spawn_tabs;
        values.automatically_reload_config = self.automatically_reload_config;
        values.check_for_updates = self.check_for_updates;
        self.next.write_patch_values(values);
    }
}

impl NativeConfigSnapshot3 {
    fn write_patch_values(self, values: &mut NativeWindowConfigPatchValues) {
        values.check_for_updates_interval_seconds = self.check_for_updates_interval_seconds;
        values.show_update_window = self.show_update_window;
        values.native_macos_fullscreen_mode = self.native_macos_fullscreen_mode;
        values.macos_fullscreen_extend_behind_notch = self.macos_fullscreen_extend_behind_notch;
        values.use_resize_increments = self.use_resize_increments;
        values.debug_key_events = self.debug_key_events;
        values.log_unknown_escape_sequences = self.log_unknown_escape_sequences;
        values.warn_about_missing_glyphs = self.warn_about_missing_glyphs;
        values.default_cwd = self.default_cwd;
        values.default_ssh_auth_sock = self.default_ssh_auth_sock;
        values.default_mux_server_domain = self.default_mux_server_domain;
        values.daemon_options = self.daemon_options;
        values.exec_domains = self.exec_domains;
        values.wsl_domains = self.wsl_domains;
        values.unix_domains = self.unix_domains;
        values.ssh_domains = self.ssh_domains;
        values.tls_servers = self.tls_servers;
        values.tls_clients = self.tls_clients;
        values.serial_ports = self.serial_ports;
        values.mux_enable_ssh_agent = self.mux_enable_ssh_agent;
        values.ssh_backend = self.ssh_backend;
        values.ratelimit_mux_line_prefetches_per_second = self.ratelimit_mux_line_prefetches_per_second;
        values.mux_output_parser_buffer_size = self.mux_output_parser_buffer_size;
        values.mux_output_parser_coalesce_delay_ms = self.mux_output_parser_coalesce_delay_ms;
        values.periodic_stat_logging = self.periodic_stat_logging;
        values.ulimit_nofile = self.ulimit_nofile;
        values.ulimit_nproc = self.ulimit_nproc;
        values.mux_env_remove = self.mux_env_remove;
        values.tiling_desktop_environments = self.tiling_desktop_environments;
        values.set_environment_variables = self.set_environment_variables;
        values.launch_menu = self.launch_menu;
        values.key_map_preference = self.key_map_preference;
        values.ui_key_cap_rendering = self.ui_key_cap_rendering;
        values.swap_backspace_and_delete = self.swap_backspace_and_delete;
        values.enable_kitty_graphics = self.enable_kitty_graphics;
        values.enable_checksum_rectangular_area = self.enable_checksum_rectangular_area;
        values.enable_title_reporting = self.enable_title_reporting;
        values.enable_csi_u_key_encoding = self.enable_csi_u_key_encoding;
        values.enable_kitty_keyboard = self.enable_kitty_keyboard;
        values.allow_download_protocols = self.allow_download_protocols;
        values.xcursor_theme = self.xcursor_theme;
        values.xcursor_size = self.xcursor_size;
        values.palette_max_key_assigments_for_action = self.palette_max_key_assigments_for_action;
        values.allow_win32_input_mode = self.allow_win32_input_mode;
        values.treat_left_ctrlalt_as_altgr = self.treat_left_ctrlalt_as_altgr;
        values.send_composed_key_when_left_alt_is_pressed = self.send_composed_key_when_left_alt_is_pressed;
        values.send_composed_key_when_right_alt_is_pressed = self.send_composed_key_when_right_alt_is_pressed;
        values.treat_east_asian_ambiguous_width_as_wide = self.treat_east_asian_ambiguous_width_as_wide;
        values.normalize_output_to_unicode_nfc = self.normalize_output_to_unicode_nfc;
        values.unicode_version = self.unicode_version;
        values.bidi_enabled = self.bidi_enabled;
        values.bidi_direction = self.bidi_direction;
        values.use_ime = self.use_ime;
        values.use_dead_keys = self.use_dead_keys;
        self.next.write_patch_values(values);
    }
}

impl NativeConfigSnapshot4 {
    fn write_patch_values(self, values: &mut NativeWindowConfigPatchValues) {
        values.ime_preedit_rendering = self.ime_preedit_rendering;
        values.macos_forward_to_ime_modifier_mask = self.macos_forward_to_ime_modifier_mask;
        values.xim_im_name = self.xim_im_name;
        values.detect_password_input = self.detect_password_input;
        values.leader = self.leader;
        values.key_assignments = self.key_assignments;
        values.key_tables = self.key_tables;
        values.mouse_assignments = self.mouse_assignments;
        values.scroll_to_bottom_on_input = self.scroll_to_bottom_on_input;
        values.adjust_window_size_when_changing_font_size = self.adjust_window_size_when_changing_font_size;
        values.canonicalize_pasted_newlines = self.canonicalize_pasted_newlines;
        values.quote_dropped_files = self.quote_dropped_files;
        values.disable_default_key_bindings = self.disable_default_key_bindings;
        values.disable_default_mouse_bindings = self.disable_default_mouse_bindings;
        values.hide_mouse_cursor_when_typing = self.hide_mouse_cursor_when_typing;
        values.alternate_buffer_wheel_scroll_speed = self.alternate_buffer_wheel_scroll_speed;
        values.pane_focus_follows_mouse = self.pane_focus_follows_mouse;
        values.swallow_mouse_click_on_pane_focus = self.swallow_mouse_click_on_pane_focus;
        values.swallow_mouse_click_on_window_focus = self.swallow_mouse_click_on_window_focus;
        values.bypass_mouse_reporting_modifiers = self.bypass_mouse_reporting_modifiers;
        values.enable_scroll_bar = self.enable_scroll_bar;
        values.scrollback_lines = self.scrollback_lines;
        values.min_scroll_bar_height = self.min_scroll_bar_height;
        values.enable_tab_bar = self.enable_tab_bar;
        values.hide_tab_bar_if_only_one_tab = self.hide_tab_bar_if_only_one_tab;
        values.use_fancy_tab_bar = self.use_fancy_tab_bar;
        values.unzoom_on_switch_pane = self.unzoom_on_switch_pane;
        values.tab_bar_at_bottom = self.tab_bar_at_bottom;
        values.tab_and_split_indices_are_zero_based = self.tab_and_split_indices_are_zero_based;
        values.mouse_wheel_scrolls_tabs = self.mouse_wheel_scrolls_tabs;
        values.switch_to_last_active_tab_when_closing_tab = self.switch_to_last_active_tab_when_closing_tab;
        values.tab_shortcut_style = self.tab_shortcut_style;
        values.closed_tab_history_size = self.closed_tab_history_size;
        values.close_tab_selection = self.close_tab_selection;
        values.tab_bar_wheel_behavior = self.tab_bar_wheel_behavior;
        values.quit_when_all_windows_are_closed = self.quit_when_all_windows_are_closed;
        values.window_close_confirmation = self.window_close_confirmation;
        values.exit_behavior = self.exit_behavior;
        values.clean_exit_codes = self.clean_exit_codes;
        values.exit_behavior_messaging = self.exit_behavior_messaging;
        values.skip_close_confirmation_for_processes_named = self.skip_close_confirmation_for_processes_named;
        values.show_close_tab_button_in_tabs = self.show_close_tab_button_in_tabs;
        values.show_new_tab_button_in_tab_bar = self.show_new_tab_button_in_tab_bar;
        values.show_tab_index_in_tab_bar = self.show_tab_index_in_tab_bar;
        values.show_tabs_in_tab_bar = self.show_tabs_in_tab_bar;
    }
}

#[allow(dead_code)]
pub(crate) fn native_config_overrides_from_wezterm_lua_config(

    config: &str,
) -> Option<NativeConfigSnapshot> {
    let mut overrides = NativeConfigSnapshot::default();
    let mut parsed = false;
    let config_receiver =
        lua_config_static_return_identifier_from_query(config).unwrap_or("config");

    parsed |= parse_native_config_group_1(config, config_receiver, &mut overrides)?;
    parsed |= parse_native_config_group_2(config, config_receiver, &mut overrides)?;
    parsed |= parse_native_config_group_3(config, config_receiver, &mut overrides)?;
    parsed |= parse_native_config_group_4(config, config_receiver, &mut overrides)?;
    parsed |= parse_native_config_group_5(config, config_receiver, &mut overrides)?;
    parsed |= parse_native_config_group_6(config, config_receiver, &mut overrides)?;
    parsed |= parse_native_config_group_7(config, config_receiver, &mut overrides)?;
    parsed |= parse_native_config_group_8(config, config_receiver, &mut overrides)?;
    parsed |= parse_native_config_group_9(config, config_receiver, &mut overrides)?;
    parsed |= parse_native_config_group_10(config, config_receiver, &mut overrides)?;

    overrides.finish_parsing(parsed)
}

#[expect(
    clippy::too_many_lines,
    reason = "each ordered compatibility parser group keeps a coherent Lua configuration domain"
)]
fn parse_native_config_group_1(
    config: &str,
    _config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(update_status) = lua_static_wezterm_status_update_event_from_query(config) {
        overrides.lua_update_status = Some(update_status);
        parsed = true;
    }
    if let Some(config_overrides) =
        lua_static_wezterm_status_update_config_overrides_from_query(config)
    {
        overrides.lua_update_status_config_overrides = Some(config_overrides);
        parsed = true;
    }
    if let Some(bell) = lua_static_wezterm_bell_event_from_query(config) {
        overrides.lua_bell = Some(bell);
        parsed = true;
    }
    if let Some(focus_changed) = lua_static_wezterm_focus_changed_event_from_query(config) {
        overrides.lua_focus_changed = Some(focus_changed);
        parsed = true;
    }
    if let Some(resized) = lua_static_wezterm_resized_event_from_query(config) {
        overrides.lua_resized = Some(resized);
        parsed = true;
    }
    if let Some(config_reloaded) = lua_static_wezterm_config_reloaded_event_from_query(config) {
        overrides.lua_config_reloaded = Some(config_reloaded);
        parsed = true;
    }
    if let Some(user_var_changed) = lua_static_wezterm_user_var_changed_event_from_query(config) {
        overrides.lua_user_var_changed = Some(user_var_changed);
        parsed = true;
    }
    if let Some(window_title) = lua_static_wezterm_window_title_return_event_from_query(config) {
        overrides.lua_window_title = Some(window_title);
        parsed = true;
    }
    if let Some(tab_title) = lua_static_wezterm_tab_title_return_event_from_query(config) {
        overrides.lua_tab_title = Some(tab_title);
        parsed = true;
    }
    if let Some(open_uri) = lua_static_wezterm_open_uri_event_from_query(config) {
        overrides.lua_open_uri = Some(open_uri);
        parsed = true;
    }
    if let Some(new_tab_button_click) =
        lua_static_wezterm_new_tab_button_click_event_from_query(config)
    {
        overrides.lua_new_tab_button_click = Some(new_tab_button_click);
        parsed = true;
    }
    if let Some(command_palette_entries) =
        lua_static_wezterm_augment_command_palette_event_from_query(config)
    {
        overrides.lua_command_palette_entries = Some(command_palette_entries);
        parsed = true;
    }
    if let Some(emit_event_handlers) = lua_static_wezterm_emit_event_handlers_from_query(config) {
        overrides.lua_emit_event_handlers = Some(emit_event_handlers);
        parsed = true;
    }

    if let Some(dpi) = lua_config_f32_assignment_from_query(config, "dpi") {
        overrides.dpi = Some(native_dpi_from_f32(dpi)?);
        parsed = true;
    }
    if let Some(dpi_by_screen) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "dpi_by_screen",
        )
        .and_then(|dpi_by_screen| {
            native_dpi_by_screen_lua_table_from_query(
                config,
                &dpi_by_screen.value,
                dpi_by_screen.max_start,
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "dpi_by_screen")
                .and_then(|dpi_by_screen| {
                    let max_start = lua_source_slice_start_offset(config, dpi_by_screen)?;
                    native_dpi_by_screen_lua_table_from_query(config, dpi_by_screen, max_start)
                })
        })
    {
        overrides.dpi_by_screen = Some(dpi_by_screen);
        parsed = true;
    }
    if let Some(tab_max_width) = lua_config_usize_assignment_from_query(config, "tab_max_width") {
        overrides.tab_max_width = Some(tab_max_width);
        parsed = true;
    }
    if let Some(tab_min_width) = lua_config_usize_assignment_from_query(config, "tab_min_width") {
        overrides.tab_min_width = Some(tab_min_width);
        parsed = true;
    }
    if let Some(default_prog) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "default_prog",
        )
    {
        overrides.default_prog = Some(split_lua_table_string_array_with_static_source(
            Some(LuaStaticSource {
                source: config,
                max_start: default_prog.max_start,
            }),
            &default_prog.value,
        )?);
        parsed = true;
    }
    if let Some(default_gui_startup_args) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "default_gui_startup_args",
        )
    {
        overrides.default_gui_startup_args = Some(split_lua_table_string_array_with_static_source(
            Some(LuaStaticSource {
                source: config,
                max_start: default_gui_startup_args.max_start,
            }),
            &default_gui_startup_args.value,
        )?);
        parsed = true;
    }
    if let Some(default_cwd) = lua_config_string_assignment_from_query(config, "default_cwd") {
        overrides.default_cwd = Some(non_empty_spawn_command_option_value(&default_cwd).ok()?);
        parsed = true;
    }
    if let Some(default_ssh_auth_sock) =
        lua_config_string_assignment_from_query(config, "default_ssh_auth_sock")
    {
        overrides.default_ssh_auth_sock =
            Some(non_empty_spawn_command_option_value(&default_ssh_auth_sock).ok()?);
        parsed = true;
    }
    if let Some(default_mux_server_domain) =
        lua_config_string_assignment_from_query(config, "default_mux_server_domain")
    {
        overrides.default_mux_server_domain =
            Some(non_empty_spawn_command_option_value(&default_mux_server_domain).ok()?);
        parsed = true;
    }
    if let Some(daemon_options) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "daemon_options",
        )
        .and_then(|daemon_options| {
            native_daemon_options_lua_table_from_query(config, &daemon_options.value)
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "daemon_options")
                .and_then(|daemon_options| {
                    native_daemon_options_lua_table_from_query(config, daemon_options)
                })
        })
    {
        overrides.daemon_options = Some(daemon_options);
        parsed = true;
    }
    if let Some(exec_domains) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "exec_domains",
        )
        .and_then(|exec_domains| {
            native_exec_domains_lua_table_from_query(
                config,
                &exec_domains.value,
                exec_domains.max_start,
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "exec_domains")
                .and_then(|exec_domains| {
                    let max_start = lua_source_slice_start_offset(config, exec_domains)?;
                    native_exec_domains_lua_table_from_query(config, exec_domains, max_start)
                })
        })
    {
        overrides.exec_domains = Some(exec_domains);
        parsed = true;
    }
    if let Some(wsl_domains) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "wsl_domains",
        )
        .and_then(|wsl_domains| {
            native_wsl_domains_lua_table_from_query(
                config,
                &wsl_domains.value,
                wsl_domains.max_start,
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "wsl_domains")
                .and_then(|wsl_domains| {
                    let max_start = lua_source_slice_start_offset(config, wsl_domains)?;
                    native_wsl_domains_lua_table_from_query(config, wsl_domains, max_start)
                })
        })
    {
        overrides.wsl_domains = Some(wsl_domains);
        parsed = true;
    }
    if let Some(unix_domains) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "unix_domains",
        )
        .and_then(|unix_domains| {
            native_unix_domains_lua_table_from_query(
                config,
                &unix_domains.value,
                unix_domains.max_start,
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "unix_domains")
                .and_then(|unix_domains| {
                    let max_start = lua_source_slice_start_offset(config, unix_domains)?;
                    native_unix_domains_lua_table_from_query(config, unix_domains, max_start)
                })
        })
    {
        overrides.unix_domains = Some(unix_domains);
        parsed = true;
    }
    Some(parsed)
}

#[expect(
    clippy::too_many_lines,
    reason = "each ordered compatibility parser group keeps a coherent Lua configuration domain"
)]
fn parse_native_config_group_2(
    config: &str,
    _config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(ssh_domains) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "ssh_domains",
        )
        .and_then(|ssh_domains| {
            native_ssh_domains_lua_table_from_query(
                config,
                &ssh_domains.value,
                ssh_domains.max_start,
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "ssh_domains")
                .and_then(|ssh_domains| {
                    let max_start = lua_source_slice_start_offset(config, ssh_domains)?;
                    native_ssh_domains_lua_table_from_query(config, ssh_domains, max_start)
                })
        })
    {
        overrides.ssh_domains = Some(ssh_domains);
        parsed = true;
    }
    if let Some(tls_servers) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "tls_servers",
        )
        .and_then(|tls_servers| {
            native_tls_server_domains_lua_table_from_query(
                config,
                &tls_servers.value,
                tls_servers.max_start,
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "tls_servers")
                .and_then(|tls_servers| {
                    let max_start = lua_source_slice_start_offset(config, tls_servers)?;
                    native_tls_server_domains_lua_table_from_query(config, tls_servers, max_start)
                })
        })
    {
        overrides.tls_servers = Some(tls_servers);
        parsed = true;
    }
    if let Some(tls_clients) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "tls_clients",
        )
        .and_then(|tls_clients| {
            native_tls_client_domains_lua_table_from_query(
                config,
                &tls_clients.value,
                tls_clients.max_start,
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "tls_clients")
                .and_then(|tls_clients| {
                    let max_start = lua_source_slice_start_offset(config, tls_clients)?;
                    native_tls_client_domains_lua_table_from_query(config, tls_clients, max_start)
                })
        })
    {
        overrides.tls_clients = Some(tls_clients);
        parsed = true;
    }
    if let Some(serial_ports) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "serial_ports",
        )
        .and_then(|serial_ports| {
            native_serial_ports_lua_table_from_query(
                config,
                &serial_ports.value,
                serial_ports.max_start,
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "serial_ports")
                .and_then(|serial_ports| {
                    let max_start = lua_source_slice_start_offset(config, serial_ports)?;
                    native_serial_ports_lua_table_from_query(config, serial_ports, max_start)
                })
        })
    {
        overrides.serial_ports = Some(serial_ports);
        parsed = true;
    }
    if let Some(mux_enable_ssh_agent) =
        lua_config_bool_assignment_from_query(config, "mux_enable_ssh_agent")
    {
        overrides.mux_enable_ssh_agent = Some(mux_enable_ssh_agent);
        parsed = true;
    }
    if let Some(ssh_backend) = lua_config_string_assignment_from_query(config, "ssh_backend") {
        overrides.ssh_backend = Some(NativeSshBackend::parse(&ssh_backend)?);
        parsed = true;
    }
    if let Some(ratelimit_mux_line_prefetches_per_second) =
        lua_config_usize_assignment_from_query(config, "ratelimit_mux_line_prefetches_per_second")
    {
        let Ok(ratelimit_mux_line_prefetches_per_second) =
            u32::try_from(ratelimit_mux_line_prefetches_per_second)
        else {
            return None;
        };
        overrides.ratelimit_mux_line_prefetches_per_second =
            Some(ratelimit_mux_line_prefetches_per_second);
        parsed = true;
    }
    if let Some(mux_output_parser_buffer_size) =
        lua_config_usize_assignment_from_query(config, "mux_output_parser_buffer_size")
    {
        overrides.mux_output_parser_buffer_size = Some(mux_output_parser_buffer_size);
        parsed = true;
    }
    if let Some(mux_output_parser_coalesce_delay_ms) =
        lua_config_usize_assignment_from_query(config, "mux_output_parser_coalesce_delay_ms")
    {
        overrides.mux_output_parser_coalesce_delay_ms =
            Some(u64::try_from(mux_output_parser_coalesce_delay_ms).ok()?);
        parsed = true;
    }
    if let Some(periodic_stat_logging) =
        lua_config_usize_assignment_from_query(config, "periodic_stat_logging")
    {
        overrides.periodic_stat_logging = Some(u64::try_from(periodic_stat_logging).ok()?);
        parsed = true;
    }
    if let Some(ulimit_nofile) = lua_config_usize_assignment_from_query(config, "ulimit_nofile") {
        overrides.ulimit_nofile = Some(u64::try_from(ulimit_nofile).ok()?);
        parsed = true;
    }
    if let Some(ulimit_nproc) = lua_config_usize_assignment_from_query(config, "ulimit_nproc") {
        overrides.ulimit_nproc = Some(u64::try_from(ulimit_nproc).ok()?);
        parsed = true;
    }
    if let Some(mux_env_remove) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "mux_env_remove",
        )
    {
        overrides.mux_env_remove = Some(split_lua_table_string_array_with_static_source(
            Some(LuaStaticSource {
                source: config,
                max_start: mux_env_remove.max_start,
            }),
            &mux_env_remove.value,
        )?);
        parsed = true;
    }
    if let Some(tiling_desktop_environments) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "tiling_desktop_environments",
        )
    {
        overrides.tiling_desktop_environments =
            Some(split_lua_table_string_array_with_static_source(
                Some(LuaStaticSource {
                    source: config,
                    max_start: tiling_desktop_environments.max_start,
                }),
                &tiling_desktop_environments.value,
            )?);
        parsed = true;
    }
    if let Some(default_workspace) =
        lua_config_string_assignment_from_query(config, "default_workspace")
    {
        overrides.default_workspace =
            Some(non_empty_spawn_command_option_value(&default_workspace).ok()?);
        parsed = true;
    }
    if let Some(prefer_to_spawn_tabs) =
        lua_config_bool_assignment_from_query(config, "prefer_to_spawn_tabs")
    {
        overrides.prefer_to_spawn_tabs = Some(prefer_to_spawn_tabs);
        parsed = true;
    }
    if let Some(default_domain) = lua_config_string_assignment_from_query(config, "default_domain")
    {
        overrides.default_domain =
            Some(non_empty_spawn_command_option_value(&default_domain).ok()?);
        parsed = true;
    }
    if let Some(term) = lua_config_string_assignment_from_query(config, "term") {
        overrides.term = Some(non_empty_spawn_command_option_value(&term).ok()?);
        parsed = true;
    }
    if let Some(audible_bell) = lua_config_string_assignment_from_query(config, "audible_bell") {
        overrides.audible_bell = Some(NativeAudibleBell::parse(&audible_bell)?);
        parsed = true;
    }
    if let Some(visual_bell) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "visual_bell",
        )
        .and_then(|visual_bell| {
            native_visual_bell_lua_table_from_query(
                config,
                &visual_bell.value,
                Some(visual_bell.max_start),
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "visual_bell")
                .and_then(|visual_bell| {
                    native_visual_bell_lua_table_from_query(
                        config,
                        visual_bell,
                        lua_source_slice_start_offset(config, visual_bell),
                    )
                })
        })
    {
        overrides.visual_bell = Some(visual_bell);
        parsed = true;
    }
    Some(parsed)
}

#[expect(
    clippy::too_many_lines,
    reason = "each ordered compatibility parser group keeps a coherent Lua configuration domain"
)]
fn parse_native_config_group_3(
    config: &str,
    config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(tab_bar_style) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "tab_bar_style",
        )
        .and_then(|tab_bar_style| {
            native_tab_bar_style_lua_table_from_query(
                Some(LuaStaticSource {
                    source: config,
                    max_start: tab_bar_style.max_start,
                }),
                &tab_bar_style.value,
            )
            .flatten()
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "tab_bar_style")
                .and_then(|tab_bar_style| {
                    native_tab_bar_style_lua_table_from_query(
                        lua_source_slice_start_offset(config, tab_bar_style).map(|max_start| {
                            LuaStaticSource {
                                source: config,
                                max_start,
                            }
                        }),
                        tab_bar_style,
                    )
                    .flatten()
                })
        })
    {
        overrides.tab_bar_style = tab_bar_style;
        parsed = true;
    }
    let color_scheme = lua_config_string_assignment_from_query(config, "color_scheme");
    if let Some(color_scheme) = color_scheme.clone() {
        overrides.color_scheme = Some(color_scheme);
        parsed = true;
    }
    if let Some(color_schemes) =
        native_color_schemes_from_wezterm_lua_config(config, config_receiver)?
    {
        overrides.color_schemes = Some(color_schemes);
        parsed = true;
    }
    let mut in_file_color_scheme_found = false;
    let mut external_color_scheme_found = false;
    if let Some(color_scheme) = color_scheme.as_deref()
        && let Some(source) =
            color_scheme_lua_source_from_config_query(config, color_scheme, config_receiver)?
    {
        in_file_color_scheme_found = true;
        parsed |=
            apply_lua_color_scheme_source_overrides(config, color_scheme, source, overrides)?;
        let (mutation_start, mutation_max_start) =
            color_scheme_lua_mutation_range_from_config_query(
                config,
                color_scheme,
                config_receiver,
            )?;
        parsed |= apply_lua_selected_color_scheme_mutation_overrides(
            config,
            color_scheme,
            config_receiver,
            mutation_start,
            mutation_max_start,
            overrides,
        )?;
    }
    if let Some(color_scheme_dirs) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "color_scheme_dirs",
        )
    {
        let color_scheme_dirs = split_lua_table_string_array_with_static_source(
            Some(LuaStaticSource {
                source: config,
                max_start: color_scheme_dirs.max_start,
            }),
            &color_scheme_dirs.value,
        )?;
        if !in_file_color_scheme_found && let Some(color_scheme) = color_scheme.as_deref() {
            external_color_scheme_found = apply_toml_color_scheme_dirs_overrides(
                &color_scheme_dirs,
                color_scheme,
                overrides,
            )?;
        }
        overrides.color_scheme_dirs = Some(color_scheme_dirs);
        parsed = true;
    }
    if !in_file_color_scheme_found
        && !external_color_scheme_found
        && let Some(color_scheme) = color_scheme.as_deref()
    {
        let default_color_scheme_found =
            apply_default_toml_color_scheme_dirs_overrides(color_scheme, overrides)?;
        parsed |= default_color_scheme_found;
        if !default_color_scheme_found {
            parsed |= apply_builtin_color_scheme_overrides(color_scheme, overrides)?;
        }
    }
    if let Some(colors_source) = lua_config_colors_source_from_query(config)? {
        let mut colors_overrides = NativeConfigSnapshot::default();
        match colors_source {
            NativeConfigColorsLuaSource::Table { colors, variable } => {
                let static_source = Some(LuaStaticSource {
                    source: config,
                    max_start: lua_source_slice_start_offset(config, colors)?,
                });
                parsed |= apply_lua_colors_table_overrides(static_source, colors, overrides)?;
                apply_lua_colors_table_overrides(static_source, colors, &mut colors_overrides)?;
                if let Some(variable) = variable.as_ref() {
                    parsed |= apply_lua_color_variable_mutation_overrides(
                        config,
                        variable,
                        overrides,
                    )?;
                    apply_lua_color_variable_mutation_overrides(
                        config,
                        variable,
                        &mut colors_overrides,
                    )?;
                }
            }
            NativeConfigColorsLuaSource::LoadScheme(load_scheme) => {
                parsed |= apply_toml_color_scheme_file_overrides(
                    Path::new(&load_scheme.path),
                    overrides,
                )?;
                apply_toml_color_scheme_file_overrides(
                    Path::new(&load_scheme.path),
                    &mut colors_overrides,
                )?;
                if let Some(variable) = load_scheme.variable.as_ref() {
                    parsed |= apply_lua_color_variable_mutation_overrides(
                        config,
                        variable,
                        overrides,
                    )?;
                    apply_lua_color_variable_mutation_overrides(
                        config,
                        variable,
                        &mut colors_overrides,
                    )?;
                }
            }
            NativeConfigColorsLuaSource::Builtin(builtin) => {
                parsed |= apply_builtin_color_scheme_overrides(&builtin.name, overrides)?;
                apply_builtin_color_scheme_overrides(&builtin.name, &mut colors_overrides)?;
                if let Some(variable) = builtin.variable.as_ref() {
                    parsed |= apply_lua_color_variable_mutation_overrides(
                        config,
                        variable,
                        overrides,
                    )?;
                    apply_lua_color_variable_mutation_overrides(
                        config,
                        variable,
                        &mut colors_overrides,
                    )?;
                }
            }
            NativeConfigColorsLuaSource::DefaultColors { variable } => {
                parsed |= apply_wezterm_default_colors_overrides(overrides);
                apply_wezterm_default_colors_overrides(&mut colors_overrides);
                if let Some(variable) = variable.as_ref() {
                    parsed |= apply_lua_color_variable_mutation_overrides(
                        config,
                        variable,
                        overrides,
                    )?;
                    apply_lua_color_variable_mutation_overrides(
                        config,
                        variable,
                        &mut colors_overrides,
                    )?;
                }
            }
        }
        parsed |= apply_lua_config_colors_tab_bar_mutation_overrides(
            config,
            config_receiver,
            config.len(),
            overrides,
        )?;
        apply_lua_config_colors_tab_bar_mutation_overrides(
            config,
            config_receiver,
            config.len(),
            &mut colors_overrides,
        )?;
        parsed |= apply_lua_config_colors_color_spec_mutation_overrides(
            config,
            config_receiver,
            config.len(),
            overrides,
        )?;
        apply_lua_config_colors_color_spec_mutation_overrides(
            config,
            config_receiver,
            config.len(),
            &mut colors_overrides,
        )?;
        overrides.colors = Some(Box::write(
            Box::new_uninit(),
            native_palette_from_overrides(&colors_overrides),
        ));
    }
    if let Some(notification_handling) =
        lua_config_string_assignment_from_query(config, "notification_handling")
    {
        overrides.notification_handling =
            Some(NativeNotificationHandling::parse(&notification_handling)?);
        parsed = true;
    }
    if let Some(environment) = lua_config_table_map_assignment_with_field_mutations_from_query(
        config,
        "set_environment_variables",
    ) {
        overrides.set_environment_variables =
            Some(split_lua_table_environment_from_query_with_static_source(
                Some(LuaStaticSource {
                    source: config,
                    max_start: environment.max_start,
                }),
                &environment.value,
            )?);
        parsed = true;
    }
    Some(parsed)
}

#[expect(
    clippy::too_many_lines,
    reason = "each ordered compatibility parser group keeps a coherent Lua configuration domain"
)]
fn parse_native_config_group_4(
    config: &str,
    _config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(font_config) = lua_config_font_assignment_from_query(config, "font") {
        let mut font_families = font_config.families.into_iter();
        overrides.font = font_families.next();
        overrides.font_fallbacks = Some(font_families.collect());
        overrides.font_attributes = Some(font_config.attributes);
        parsed = true;
    }
    if let Some(font_rules) = lua_config_font_rules_assignment_from_query(config) {
        overrides.font_rules = Some(font_rules);
        parsed = true;
    }
    if let Some(font_size) = lua_config_f32_assignment_from_query(config, "font_size") {
        overrides.font_size = Some(native_font_size_from_points(font_size)?);
        parsed = true;
    }
    if let Some(cell_width) = lua_config_f32_assignment_from_query(config, "cell_width") {
        overrides.cell_width = Some(native_cell_width_from_ratio(cell_width)?);
        parsed = true;
    }
    if let Some(cell_widths) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "cell_widths",
        )
    {
        overrides.cell_widths = Some(native_cell_widths_lua_table_from_query(
            config,
            &cell_widths.value,
            cell_widths.max_start,
        )?);
        parsed = true;
    }
    if let Some(line_height) = lua_config_f32_assignment_from_query(config, "line_height") {
        overrides.line_height = Some(native_line_height_from_ratio(line_height)?);
        parsed = true;
    }
    if let Some(font_antialias) = lua_config_string_assignment_from_query(config, "font_antialias")
    {
        overrides.font_antialias = Some(NativeFontAntialias::parse(&font_antialias)?);
        parsed = true;
    }
    if let Some(font_hinting) = lua_config_string_assignment_from_query(config, "font_hinting") {
        overrides.font_hinting = Some(NativeFontHinting::parse(&font_hinting)?);
        parsed = true;
    }
    if let Some(font_rasterizer) =
        lua_config_string_assignment_from_query(config, "font_rasterizer")
    {
        overrides.font_rasterizer = Some(NativeFontRasterizer::parse(&font_rasterizer)?);
        parsed = true;
    }
    if let Some(font_colr_rasterizer) =
        lua_config_string_assignment_from_query(config, "font_colr_rasterizer")
    {
        overrides.font_colr_rasterizer = Some(NativeFontRasterizer::parse(&font_colr_rasterizer)?);
        parsed = true;
    }
    if let Some(font_shaper) = lua_config_string_assignment_from_query(config, "font_shaper") {
        overrides.font_shaper = Some(NativeFontShaper::parse(&font_shaper)?);
        parsed = true;
    }
    if let Some(harfbuzz_features) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "harfbuzz_features",
        )
    {
        overrides.harfbuzz_features = Some(split_lua_table_string_array_with_static_source(
            Some(LuaStaticSource {
                source: config,
                max_start: harfbuzz_features.max_start,
            }),
            &harfbuzz_features.value,
        )?);
        parsed = true;
    }
    if let Some(font_dirs) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "font_dirs",
        )
    {
        overrides.font_dirs = Some(split_lua_table_string_array_with_static_source(
            Some(LuaStaticSource {
                source: config,
                max_start: font_dirs.max_start,
            }),
            &font_dirs.value,
        )?);
        parsed = true;
    }
    if let Some(font_locator) = lua_config_string_assignment_from_query(config, "font_locator") {
        overrides.font_locator = Some(NativeFontLocator::parse(&font_locator)?);
        parsed = true;
    }
    if let Some(ignore_svg_fonts) =
        lua_config_bool_assignment_from_query(config, "ignore_svg_fonts")
    {
        overrides.ignore_svg_fonts = Some(ignore_svg_fonts);
        parsed = true;
    }
    if let Some(sort_fallback_fonts_by_coverage) =
        lua_config_bool_assignment_from_query(config, "sort_fallback_fonts_by_coverage")
    {
        overrides.sort_fallback_fonts_by_coverage = Some(sort_fallback_fonts_by_coverage);
        parsed = true;
    }
    if let Some(search_font_dirs_for_fallback) =
        lua_config_bool_assignment_from_query(config, "search_font_dirs_for_fallback")
    {
        overrides.search_font_dirs_for_fallback = Some(search_font_dirs_for_fallback);
        parsed = true;
    }
    if let Some(custom_block_glyphs) =
        lua_config_bool_assignment_from_query(config, "custom_block_glyphs")
    {
        overrides.custom_block_glyphs = Some(custom_block_glyphs);
        parsed = true;
    }
    if let Some(anti_alias_custom_block_glyphs) =
        lua_config_bool_assignment_from_query(config, "anti_alias_custom_block_glyphs")
    {
        overrides.anti_alias_custom_block_glyphs = Some(anti_alias_custom_block_glyphs);
        parsed = true;
    }
    if let Some(allow_square_glyphs_to_overflow_width) =
        lua_config_string_assignment_from_query(config, "allow_square_glyphs_to_overflow_width")
    {
        overrides.allow_square_glyphs_to_overflow_width = Some(NativeSquareGlyphOverflow::parse(
            &allow_square_glyphs_to_overflow_width,
        )?);
        parsed = true;
    }
    if let Some(freetype_load_target) =
        lua_config_string_assignment_from_query(config, "freetype_load_target")
    {
        overrides.freetype_load_target = Some(NativeFreetypeTarget::parse(&freetype_load_target)?);
        parsed = true;
    }
    if let Some(freetype_render_target) =
        lua_config_string_assignment_from_query(config, "freetype_render_target")
    {
        overrides.freetype_render_target =
            Some(NativeFreetypeTarget::parse(&freetype_render_target)?);
        parsed = true;
    }
    if let Some(freetype_load_flags) =
        lua_config_string_assignment_from_query(config, "freetype_load_flags")
    {
        overrides.freetype_load_flags = Some(NativeFreetypeLoadFlags::parse(&freetype_load_flags)?);
        parsed = true;
    }
    if let Some(freetype_interpreter_version) =
        lua_config_usize_assignment_from_query(config, "freetype_interpreter_version")
    {
        let Ok(freetype_interpreter_version) = u32::try_from(freetype_interpreter_version) else {
            return None;
        };
        overrides.freetype_interpreter_version = Some(freetype_interpreter_version);
        parsed = true;
    }
    if let Some(freetype_pcf_long_family_names) =
        lua_config_bool_assignment_from_query(config, "freetype_pcf_long_family_names")
    {
        overrides.freetype_pcf_long_family_names = Some(freetype_pcf_long_family_names);
        parsed = true;
    }
    if let Some(display_pixel_geometry) =
        lua_config_string_assignment_from_query(config, "display_pixel_geometry")
    {
        overrides.display_pixel_geometry =
            Some(NativeDisplayPixelGeometry::parse(&display_pixel_geometry)?);
        parsed = true;
    }
    if let Some(foreground_text_hsb) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "foreground_text_hsb",
        )
        .and_then(|foreground_text_hsb| {
            native_hsb_lua_table_from_query(
                config,
                &foreground_text_hsb.value,
                Some(foreground_text_hsb.max_start),
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "foreground_text_hsb")
                .and_then(|foreground_text_hsb| {
                    native_hsb_lua_table_from_query(
                        config,
                        foreground_text_hsb,
                        lua_source_slice_start_offset(config, foreground_text_hsb),
                    )
                })
        })
    {
        overrides.foreground_text_hsb = Some(foreground_text_hsb);
        parsed = true;
    }
    if let Some(inactive_pane_hsb) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "inactive_pane_hsb",
        )
        .and_then(|inactive_pane_hsb| {
            native_hsb_lua_table_from_query(
                config,
                &inactive_pane_hsb.value,
                Some(inactive_pane_hsb.max_start),
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "inactive_pane_hsb")
                .and_then(|inactive_pane_hsb| {
                    native_hsb_lua_table_from_query(
                        config,
                        inactive_pane_hsb,
                        lua_source_slice_start_offset(config, inactive_pane_hsb),
                    )
                })
        })
    {
        overrides.inactive_pane_hsb = Some(inactive_pane_hsb);
        parsed = true;
    }
    if let Some(bold_brightens_ansi_colors) =
        lua_config_string_assignment_from_query(config, "bold_brightens_ansi_colors")
    {
        overrides.bold_brightens_ansi_colors = Some(NativeBoldBrightensAnsiColors::parse(
            &bold_brightens_ansi_colors,
        )?);
        parsed = true;
    }
    Some(parsed)
}

#[expect(
    clippy::too_many_lines,
    reason = "each ordered compatibility parser group keeps a coherent Lua configuration domain"
)]
fn parse_native_config_group_5(
    config: &str,
    _config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(text_background_opacity) =
        lua_config_f32_assignment_from_query(config, "text_background_opacity")
    {
        overrides.text_background_opacity = Some(native_text_background_opacity_from_alpha(
            text_background_opacity,
        )?);
        parsed = true;
    }
    if let Some(window_background_opacity) =
        lua_config_f32_assignment_from_query(config, "window_background_opacity")
    {
        overrides.window_background_opacity = Some(native_text_background_opacity_from_alpha(
            window_background_opacity,
        )?);
        parsed = true;
    }
    let window_background_image_hsb = if let Some(window_background_image_hsb) =
        lua_config_table_or_static_variable_assignment_from_query(
            config,
            "window_background_image_hsb",
        ) {
        parsed = true;
        Some(native_hsb_lua_table_from_query(
            config,
            window_background_image_hsb,
            lua_source_slice_start_offset(config, window_background_image_hsb),
        )?)
    } else {
        None
    };
    overrides.window_background_image_hsb = window_background_image_hsb;
    if let Some(window_background_image) =
        lua_config_string_assignment_from_query(config, "window_background_image")
    {
        let data = fs::read(Path::new(&window_background_image)).ok()?;
        overrides.window_background_image = Some(window_background_image);
        let image = NativeWindowBackgroundImage {
            data,
            opacity_alpha: overrides
                .window_background_opacity
                .unwrap_or(DEFAULT_WINDOW_BACKGROUND_OPACITY)
                .as_alpha(),
            hsb: window_background_image_hsb.unwrap_or_else(native_identity_hsb),
            animation_speed_millis: 1_000,
            attachment: RenderBackgroundImageAttachment::Fixed,
            layout: NativeWindowBackgroundImageLayout {
                width: RenderBackgroundImageDimension::Percent(10_000),
                height: RenderBackgroundImageDimension::Percent(10_000),
                ..NativeWindowBackgroundImageLayout::default()
            },
        };
        overrides.background = Some(vec![NativeWindowBackgroundVisualLayer::Image(
            image.clone(),
        )]);
        overrides.window_background_images = Some(vec![image]);
        parsed = true;
    }
    if let Some(window_background_gradient) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "window_background_gradient",
        )
    {
        let mut gradient = native_window_background_gradient_lua_table_from_query(
            config,
            &window_background_gradient.value,
            window_background_gradient.max_start,
        )?;
        gradient.opacity_alpha = overrides
            .window_background_opacity
            .unwrap_or(DEFAULT_WINDOW_BACKGROUND_OPACITY)
            .as_alpha();
        gradient.hsb = overrides
            .window_background_image_hsb
            .unwrap_or_else(native_identity_hsb);
        overrides.background = Some(vec![NativeWindowBackgroundVisualLayer::Gradient(
            gradient.clone(),
        )]);
        overrides.window_background_gradient = Some(gradient);
        parsed = true;
    }
    if let Some(background) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "background",
        )
    {
        parsed |= apply_lua_background_table_overrides(
            config,
            &background.value,
            background.max_start,
            overrides,
        )?;
    }
    if let Some(kde_window_background_blur) =
        lua_config_bool_assignment_from_query(config, "kde_window_background_blur")
    {
        overrides.kde_window_background_blur = Some(kde_window_background_blur);
        parsed = true;
    }
    if let Some(macos_window_background_blur) =
        lua_config_usize_assignment_from_query(config, "macos_window_background_blur")
    {
        let Ok(macos_window_background_blur) = u32::try_from(macos_window_background_blur) else {
            return None;
        };
        overrides.macos_window_background_blur = Some(macos_window_background_blur);
        parsed = true;
    }
    if let Some(win32_system_backdrop) =
        lua_config_string_assignment_from_query(config, "win32_system_backdrop")
    {
        overrides.win32_system_backdrop =
            Some(NativeWin32SystemBackdrop::parse(&win32_system_backdrop)?);
        parsed = true;
    }
    if let Some(win32_acrylic_accent_color) =
        lua_config_opaque_color_assignment_from_query(config, "win32_acrylic_accent_color")
    {
        overrides.win32_acrylic_accent_color = Some(win32_acrylic_accent_color);
        parsed = true;
    }
    if let Some(window_decorations) =
        lua_config_string_assignment_from_query(config, "window_decorations")
    {
        overrides.window_decorations = Some(NativeWindowDecorations::parse(&window_decorations)?);
        parsed = true;
    }
    if let Some(window_frame_appearance) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "window_frame",
        )
        .and_then(|window_frame| {
            let static_source = Some(LuaStaticSource {
                source: config,
                max_start: window_frame.max_start,
            });
            native_window_frame_appearance_lua_table_from_query(
                config,
                &window_frame.value,
                static_source,
            )
            .flatten()
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "window_frame")
                .and_then(|window_frame| {
                    let static_source =
                        lua_source_slice_start_offset(config, window_frame).map(|max_start| {
                            LuaStaticSource {
                                source: config,
                                max_start,
                            }
                        });
                    native_window_frame_appearance_lua_table_from_query(
                        config,
                        window_frame,
                        static_source,
                    )
                    .flatten()
                })
        })
    {
        overrides.window_frame_appearance = Some(window_frame_appearance);
        parsed = true;
    }
    if let Some(integrated_title_buttons) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "integrated_title_buttons",
        )
    {
        let buttons = split_lua_table_string_array_with_static_source(
            Some(LuaStaticSource {
                source: config,
                max_start: integrated_title_buttons.max_start,
            }),
            &integrated_title_buttons.value,
        )?;
        overrides.integrated_title_buttons =
            Some(native_integrated_title_buttons_from_strings(buttons)?);
        parsed = true;
    }
    if let Some(integrated_title_button_alignment) =
        lua_config_string_assignment_from_query(config, "integrated_title_button_alignment")
    {
        overrides.integrated_title_button_alignment = Some(
            NativeIntegratedTitleButtonAlignment::parse(&integrated_title_button_alignment)?,
        );
        parsed = true;
    }
    if let Some(integrated_title_button_color) =
        lua_config_integrated_title_button_color_assignment_from_query(config)
    {
        overrides.integrated_title_button_color = Some(integrated_title_button_color);
        parsed = true;
    }
    if let Some(integrated_title_button_style) =
        lua_config_string_assignment_from_query(config, "integrated_title_button_style")
    {
        overrides.integrated_title_button_style = Some(NativeIntegratedTitleButtonStyle::parse(
            &integrated_title_button_style,
        )?);
        parsed = true;
    }
    if let Some(initial_cols) = lua_config_usize_assignment_from_query(config, "initial_cols") {
        overrides.initial_cols = Some(u16::try_from(initial_cols).ok()?);
        parsed = true;
    }
    if let Some(initial_rows) = lua_config_usize_assignment_from_query(config, "initial_rows") {
        overrides.initial_rows = Some(u16::try_from(initial_rows).ok()?);
        parsed = true;
    }
    if let Some(adjust_window_size_when_changing_font_size) =
        lua_config_bool_assignment_from_query(config, "adjust_window_size_when_changing_font_size")
    {
        overrides.adjust_window_size_when_changing_font_size =
            Some(adjust_window_size_when_changing_font_size);
        parsed = true;
    }
    if let Some(cursor_blink_rate) =
        lua_config_usize_assignment_from_query(config, "cursor_blink_rate")
    {
        overrides.cursor_blink_rate_ms = Some(u64::try_from(cursor_blink_rate).ok()?);
        parsed = true;
    }
    if let Some(cursor_blink_ease_in) =
        lua_config_easing_assignment_from_query(config, "cursor_blink_ease_in")
    {
        overrides.cursor_blink_ease_in = Some(cursor_blink_ease_in);
        parsed = true;
    }
    Some(parsed)
}

#[expect(
    clippy::too_many_lines,
    reason = "each ordered compatibility parser group keeps a coherent Lua configuration domain"
)]
fn parse_native_config_group_6(
    config: &str,
    _config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(cursor_blink_ease_out) =
        lua_config_easing_assignment_from_query(config, "cursor_blink_ease_out")
    {
        overrides.cursor_blink_ease_out = Some(cursor_blink_ease_out);
        parsed = true;
    }
    if let Some(text_blink_rate) = lua_config_usize_assignment_from_query(config, "text_blink_rate")
    {
        overrides.text_blink_rate_ms = Some(u64::try_from(text_blink_rate).ok()?);
        parsed = true;
    }
    if let Some(text_blink_rate_rapid) =
        lua_config_usize_assignment_from_query(config, "text_blink_rate_rapid")
    {
        overrides.text_blink_rate_rapid_ms = Some(u64::try_from(text_blink_rate_rapid).ok()?);
        parsed = true;
    }
    if let Some(text_blink_ease_in) =
        lua_config_easing_assignment_from_query(config, "text_blink_ease_in")
    {
        overrides.text_blink_ease_in = Some(text_blink_ease_in);
        parsed = true;
    }
    if let Some(text_blink_ease_out) =
        lua_config_easing_assignment_from_query(config, "text_blink_ease_out")
    {
        overrides.text_blink_ease_out = Some(text_blink_ease_out);
        parsed = true;
    }
    if let Some(text_blink_rapid_ease_in) =
        lua_config_easing_assignment_from_query(config, "text_blink_rapid_ease_in")
    {
        overrides.text_blink_rapid_ease_in = Some(text_blink_rapid_ease_in);
        parsed = true;
    }
    if let Some(text_blink_rapid_ease_out) =
        lua_config_easing_assignment_from_query(config, "text_blink_rapid_ease_out")
    {
        overrides.text_blink_rapid_ease_out = Some(text_blink_rapid_ease_out);
        parsed = true;
    }
    if let Some(default_cursor_style) =
        lua_config_string_assignment_from_query(config, "default_cursor_style")
    {
        overrides.default_cursor_style = Some(NativeCursorStyle::parse(&default_cursor_style)?);
        parsed = true;
    }
    if let Some(cursor_thickness) =
        lua_config_dimension_assignment_from_query(config, "cursor_thickness")
    {
        overrides.cursor_thickness = Some(NativeCursorThickness::parse(&cursor_thickness)?);
        parsed = true;
    }
    if let Some(underline_thickness) =
        lua_config_dimension_assignment_from_query(config, "underline_thickness")
    {
        overrides.underline_thickness =
            Some(NativeUnderlineThickness::parse(&underline_thickness)?);
        parsed = true;
    }
    if let Some(underline_position) =
        lua_config_dimension_assignment_from_query(config, "underline_position")
    {
        overrides.underline_position = Some(NativeUnderlinePosition::parse(&underline_position)?);
        parsed = true;
    }
    if let Some(strikethrough_position) =
        lua_config_dimension_assignment_from_query(config, "strikethrough_position")
    {
        overrides.strikethrough_position =
            Some(NativeStrikethroughPosition::parse(&strikethrough_position)?);
        parsed = true;
    }
    if let Some(force_reverse_video_cursor) =
        lua_config_bool_assignment_from_query(config, "force_reverse_video_cursor")
    {
        overrides.force_reverse_video_cursor = Some(force_reverse_video_cursor);
        parsed = true;
    }
    if let Some(reverse_video_cursor_min_contrast) =
        lua_config_f32_assignment_from_query(config, "reverse_video_cursor_min_contrast")
    {
        overrides.reverse_video_cursor_min_contrast =
            NativeContrastRatio::from_f32(reverse_video_cursor_min_contrast);
        parsed = true;
    }
    if let Some(text_min_contrast_ratio) =
        lua_config_f32_assignment_from_query(config, "text_min_contrast_ratio")
    {
        overrides.text_min_contrast_ratio =
            NativeTextMinContrastRatio::from_f32(text_min_contrast_ratio);
        parsed = true;
    }
    if let Some(status_update_interval) =
        lua_config_usize_assignment_from_query(config, "status_update_interval")
    {
        overrides.status_update_interval_ms = Some(u64::try_from(status_update_interval).ok()?);
        parsed = true;
    }
    if let Some(max_fps) = lua_config_usize_assignment_from_query(config, "max_fps") {
        overrides.max_fps = Some(max_fps);
        parsed = true;
    }
    if let Some(animation_fps) = lua_config_usize_assignment_from_query(config, "animation_fps") {
        overrides.animation_fps = Some(animation_fps);
        parsed = true;
    }
    if let Some(front_end) = lua_config_string_assignment_from_query(config, "front_end") {
        overrides.front_end = Some(NativeRenderFrontEnd::parse(&front_end)?);
        parsed = true;
    }
    if let Some(webgpu_power_preference) =
        lua_config_string_assignment_from_query(config, "webgpu_power_preference")
    {
        overrides.webgpu_power_preference = Some(NativeWebGpuPowerPreference::parse(
            &webgpu_power_preference,
        )?);
        parsed = true;
    }
    if let Some(webgpu_force_fallback_adapter) =
        lua_config_bool_assignment_from_query(config, "webgpu_force_fallback_adapter")
    {
        overrides.webgpu_force_fallback_adapter = Some(webgpu_force_fallback_adapter);
        parsed = true;
    }
    if let Some(webgpu_preferred_adapter) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "webgpu_preferred_adapter",
        )
        .and_then(|webgpu_preferred_adapter| {
            native_webgpu_preferred_adapter_lua_table_from_query(
                config,
                &webgpu_preferred_adapter.value,
                Some(webgpu_preferred_adapter.max_start),
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(
                config,
                "webgpu_preferred_adapter",
            )
            .and_then(|webgpu_preferred_adapter| {
                native_webgpu_preferred_adapter_lua_table_from_query(
                    config,
                    webgpu_preferred_adapter,
                    lua_source_slice_start_offset(config, webgpu_preferred_adapter),
                )
            })
        })
    {
        overrides.webgpu_preferred_adapter = Some(webgpu_preferred_adapter);
        parsed = true;
    }
    if let Some(prefer_egl) = lua_config_bool_assignment_from_query(config, "prefer_egl") {
        overrides.prefer_egl = Some(prefer_egl);
        parsed = true;
    }
    if let Some(enable_wayland) = lua_config_bool_assignment_from_query(config, "enable_wayland") {
        overrides.enable_wayland = Some(enable_wayland);
        parsed = true;
    }
    if let Some(enable_zwlr_output_manager) =
        lua_config_bool_assignment_from_query(config, "enable_zwlr_output_manager")
    {
        overrides.enable_zwlr_output_manager = Some(enable_zwlr_output_manager);
        parsed = true;
    }
    if let Some(use_box_model_render) =
        lua_config_bool_assignment_from_query(config, "use_box_model_render")
    {
        overrides.use_box_model_render = Some(use_box_model_render);
        parsed = true;
    }
    if let Some(experimental_pixel_positioning) =
        lua_config_bool_assignment_from_query(config, "experimental_pixel_positioning")
    {
        overrides.experimental_pixel_positioning = Some(experimental_pixel_positioning);
        parsed = true;
    }
    if let Some(shape_cache_size) =
        lua_config_usize_assignment_from_query(config, "shape_cache_size")
    {
        overrides.shape_cache_size = Some(shape_cache_size);
        parsed = true;
    }
    if let Some(line_state_cache_size) =
        lua_config_usize_assignment_from_query(config, "line_state_cache_size")
    {
        overrides.line_state_cache_size = Some(line_state_cache_size);
        parsed = true;
    }
    if let Some(line_quad_cache_size) =
        lua_config_usize_assignment_from_query(config, "line_quad_cache_size")
    {
        overrides.line_quad_cache_size = Some(line_quad_cache_size);
        parsed = true;
    }
    if let Some(line_to_ele_shape_cache_size) =
        lua_config_usize_assignment_from_query(config, "line_to_ele_shape_cache_size")
    {
        overrides.line_to_ele_shape_cache_size = Some(line_to_ele_shape_cache_size);
        parsed = true;
    }
    if let Some(glyph_cache_image_cache_size) =
        lua_config_usize_assignment_from_query(config, "glyph_cache_image_cache_size")
    {
        overrides.glyph_cache_image_cache_size = Some(glyph_cache_image_cache_size);
        parsed = true;
    }
    if let Some(command_palette_rows) =
        lua_config_usize_assignment_from_query(config, "command_palette_rows")
    {
        overrides.command_palette_rows = Some(command_palette_rows);
        parsed = true;
    }
    if let Some(command_palette_font) =
        lua_config_font_assignment_from_query(config, "command_palette_font")
    {
        overrides.command_palette_font = Some(command_palette_font);
        parsed = true;
    }
    if let Some(command_palette_font_size) =
        lua_config_f32_assignment_from_query(config, "command_palette_font_size")
    {
        overrides.command_palette_font_size =
            Some(native_font_size_from_points(command_palette_font_size)?);
        parsed = true;
    }
    if let Some(command_palette_bg_color) =
        lua_config_color_assignment_from_query(config, "command_palette_bg_color")
    {
        overrides.command_palette_bg_color = Some(command_palette_bg_color);
        parsed = true;
    }
    Some(parsed)
}

#[expect(
    clippy::too_many_lines,
    reason = "each ordered compatibility parser group keeps a coherent Lua configuration domain"
)]
fn parse_native_config_group_7(
    config: &str,
    _config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(command_palette_fg_color) =
        lua_config_color_assignment_from_query(config, "command_palette_fg_color")
    {
        overrides.command_palette_fg_color = Some(command_palette_fg_color);
        parsed = true;
    }
    if let Some(char_select_bg_color) =
        lua_config_color_assignment_from_query(config, "char_select_bg_color")
    {
        overrides.char_select_bg_color = Some(char_select_bg_color);
        parsed = true;
    }
    if let Some(char_select_fg_color) =
        lua_config_color_assignment_from_query(config, "char_select_fg_color")
    {
        overrides.char_select_fg_color = Some(char_select_fg_color);
        parsed = true;
    }
    if let Some(char_select_font) =
        lua_config_font_assignment_from_query(config, "char_select_font")
    {
        overrides.char_select_font = Some(char_select_font);
        parsed = true;
    }
    if let Some(char_select_font_size) =
        lua_config_f32_assignment_from_query(config, "char_select_font_size")
    {
        overrides.char_select_font_size =
            Some(native_font_size_from_points(char_select_font_size)?);
        parsed = true;
    }
    if let Some(pane_select_font) =
        lua_config_font_assignment_from_query(config, "pane_select_font")
    {
        overrides.pane_select_font = Some(pane_select_font);
        parsed = true;
    }
    if let Some(pane_select_font_size) =
        lua_config_f32_assignment_from_query(config, "pane_select_font_size")
    {
        overrides.pane_select_font_size =
            Some(native_font_size_from_points(pane_select_font_size)?);
        parsed = true;
    }
    if let Some(pane_select_bg_color) =
        lua_config_color_assignment_from_query(config, "pane_select_bg_color")
    {
        overrides.pane_select_bg_color = Some(pane_select_bg_color);
        parsed = true;
    }
    if let Some(pane_select_fg_color) =
        lua_config_color_assignment_from_query(config, "pane_select_fg_color")
    {
        overrides.pane_select_fg_color = Some(pane_select_fg_color);
        parsed = true;
    }
    if let Some(use_cap_height_to_scale_fallback_fonts) =
        lua_config_bool_assignment_from_query(config, "use_cap_height_to_scale_fallback_fonts")
    {
        overrides.use_cap_height_to_scale_fallback_fonts =
            Some(use_cap_height_to_scale_fallback_fonts);
        parsed = true;
    }
    if let Some(launcher_alphabet) =
        lua_config_string_assignment_from_query(config, "launcher_alphabet")
    {
        overrides.launcher_alphabet =
            Some(non_empty_spawn_command_option_value(&launcher_alphabet).ok()?);
        parsed = true;
    }
    if let Some(quick_select_alphabet) =
        lua_config_string_assignment_from_query(config, "quick_select_alphabet")
    {
        overrides.quick_select_alphabet =
            Some(non_empty_spawn_command_option_value(&quick_select_alphabet).ok()?);
        parsed = true;
    }
    if let Some(quick_select_patterns) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "quick_select_patterns",
        )
    {
        overrides.quick_select_patterns = Some(split_lua_table_string_array_with_static_source(
            Some(LuaStaticSource {
                source: config,
                max_start: quick_select_patterns.max_start,
            }),
            &quick_select_patterns.value,
        )?);
        parsed = true;
    }
    if let Some(disable_default_quick_select_patterns) =
        lua_config_bool_assignment_from_query(config, "disable_default_quick_select_patterns")
    {
        overrides.disable_default_quick_select_patterns =
            Some(disable_default_quick_select_patterns);
        parsed = true;
    }
    if let Some(quick_select_remove_styling) =
        lua_config_bool_assignment_from_query(config, "quick_select_remove_styling")
    {
        overrides.quick_select_remove_styling = Some(quick_select_remove_styling);
        parsed = true;
    }
    let mut parsed_hyperlink_rules = false;
    if let Some(table) = lua_config_static_return_table_from_query(config) {
        let max_start = lua_source_slice_start_offset(config, table)?;
        if let Some(default_rules) =
            lua_config_hyperlink_rules_default_rules_with_static_inserts(config, max_start)
        {
            overrides.hyperlink_rules = Some(default_rules);
            parsed = true;
            parsed_hyperlink_rules = true;
        } else if lua_config_hyperlink_rules_static_default_alias_before_offset(config, max_start)
            .unwrap_or(false)
        {
            overrides.hyperlink_rules = Some(default_hyperlink_rules());
            parsed = true;
            parsed_hyperlink_rules = true;
        }
    }
    if !parsed_hyperlink_rules
        && let Some(default_rules) =
            lua_config_hyperlink_rules_default_rules_with_config_inserts(config, config.len())
    {
        overrides.hyperlink_rules = Some(default_rules);
        parsed = true;
        parsed_hyperlink_rules = true;
    }
    if !parsed_hyperlink_rules
        && let Some(hyperlink_rules) =
            lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
                config,
                "hyperlink_rules",
            )
    {
        let parsed_rules = native_hyperlink_rules_lua_table_from_query(
            config,
            &hyperlink_rules.value,
            hyperlink_rules.max_start,
        );
        if let Some(default_rules) = lua_config_hyperlink_rules_default_rules_with_static_inserts(
            config,
            hyperlink_rules.max_start,
        ) {
            overrides.hyperlink_rules = Some(default_rules);
        } else if let Some(default_rules) =
            lua_config_hyperlink_rules_default_rules_with_config_inserts(
                config,
                hyperlink_rules.max_start,
            )
        {
            overrides.hyperlink_rules = Some(default_rules);
        } else if lua_config_hyperlink_rules_extends_default_rules_before_offset(
            config,
            hyperlink_rules.max_start,
        )
        .unwrap_or(false)
        {
            let mut rules = parsed_rules?;
            let mut defaults = default_hyperlink_rules();
            defaults.append(&mut rules);
            overrides.hyperlink_rules = Some(defaults);
        } else {
            overrides.hyperlink_rules = Some(parsed_rules?);
        }
        parsed = true;
        parsed_hyperlink_rules = true;
    }
    if !parsed_hyperlink_rules
        && lua_config_hyperlink_rules_static_default_alias_before_offset(config, config.len())
            .unwrap_or(false)
    {
        overrides.hyperlink_rules = Some(default_hyperlink_rules());
        parsed = true;
        parsed_hyperlink_rules = true;
    }
    if !parsed_hyperlink_rules
        && lua_config_hyperlink_rules_returned_table_default_value_before_offset(
            config,
            config.len(),
        )
        .unwrap_or(false)
    {
        overrides.hyperlink_rules = Some(default_hyperlink_rules());
        parsed = true;
    }
    if !parsed_hyperlink_rules
        && lua_config_hyperlink_rules_direct_default_assignment_before_offset(config, config.len())
            .unwrap_or(false)
    {
        overrides.hyperlink_rules = Some(default_hyperlink_rules());
        parsed = true;
    }
    if let Some(selection_word_boundary) =
        lua_config_string_assignment_from_query(config, "selection_word_boundary")
    {
        overrides.selection_word_boundary =
            Some(non_empty_spawn_command_option_value(&selection_word_boundary).ok()?);
        parsed = true;
    }
    if let Some(enq_answerback) = lua_config_string_assignment_from_query(config, "enq_answerback")
    {
        overrides.enq_answerback = Some(enq_answerback);
        parsed = true;
    }
    if let Some(canonicalize_pasted_newlines) =
        lua_config_string_assignment_from_query(config, "canonicalize_pasted_newlines")
    {
        overrides.canonicalize_pasted_newlines = Some(NativeCanonicalizePastedNewlines::parse(
            &canonicalize_pasted_newlines,
        )?);
        parsed = true;
    } else if let Some(canonicalize_pasted_newlines) =
        lua_config_bool_assignment_from_query(config, "canonicalize_pasted_newlines")
    {
        overrides.canonicalize_pasted_newlines = Some(if canonicalize_pasted_newlines {
            NativeCanonicalizePastedNewlines::CarriageReturnAndLineFeed
        } else {
            NativeCanonicalizePastedNewlines::None
        });
        parsed = true;
    }
    if let Some(quote_dropped_files) =
        lua_config_string_assignment_from_query(config, "quote_dropped_files")
    {
        overrides.quote_dropped_files = Some(NativeQuoteDroppedFiles::parse(&quote_dropped_files)?);
        parsed = true;
    }
    Some(parsed)
}

#[expect(
    clippy::too_many_lines,
    reason = "each ordered compatibility parser group keeps a coherent Lua configuration domain"
)]
fn parse_native_config_group_8(
    config: &str,
    _config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(key_map_preference) =
        lua_config_string_assignment_from_query(config, "key_map_preference")
    {
        overrides.key_map_preference = Some(NativeKeyMapPreference::parse(&key_map_preference)?);
        parsed = true;
    }
    if let Some(ui_key_cap_rendering) =
        lua_config_string_assignment_from_query(config, "ui_key_cap_rendering")
    {
        overrides.ui_key_cap_rendering =
            Some(NativeUiKeyCapRendering::parse(&ui_key_cap_rendering)?);
        parsed = true;
    }
    if let Some(swap_backspace_and_delete) =
        lua_config_bool_assignment_from_query(config, "swap_backspace_and_delete")
    {
        overrides.swap_backspace_and_delete = Some(swap_backspace_and_delete);
        parsed = true;
    }
    if let Some(enable_kitty_graphics) =
        lua_config_bool_assignment_from_query(config, "enable_kitty_graphics")
    {
        overrides.enable_kitty_graphics = Some(enable_kitty_graphics);
        parsed = true;
    }
    if let Some(enable_checksum_rectangular_area) =
        lua_config_bool_assignment_from_query(config, "enable_checksum_rectangular_area")
    {
        overrides.enable_checksum_rectangular_area = Some(enable_checksum_rectangular_area);
        parsed = true;
    }
    if let Some(enable_title_reporting) =
        lua_config_bool_assignment_from_query(config, "enable_title_reporting")
    {
        overrides.enable_title_reporting = Some(enable_title_reporting);
        parsed = true;
    }
    if let Some(enable_csi_u_key_encoding) =
        lua_config_bool_assignment_from_query(config, "enable_csi_u_key_encoding")
    {
        overrides.enable_csi_u_key_encoding = Some(enable_csi_u_key_encoding);
        parsed = true;
    }
    if let Some(enable_kitty_keyboard) =
        lua_config_bool_assignment_from_query(config, "enable_kitty_keyboard")
    {
        overrides.enable_kitty_keyboard = Some(enable_kitty_keyboard);
        parsed = true;
    }
    if let Some(allow_download_protocols) =
        lua_config_bool_assignment_from_query(config, "allow_download_protocols")
    {
        overrides.allow_download_protocols = Some(allow_download_protocols);
        parsed = true;
    }
    if let Some(xcursor_theme) = lua_config_string_assignment_from_query(config, "xcursor_theme") {
        overrides.xcursor_theme = Some(xcursor_theme);
        parsed = true;
    }
    if let Some(xcursor_size) = lua_config_usize_assignment_from_query(config, "xcursor_size") {
        let Ok(xcursor_size) = u32::try_from(xcursor_size) else {
            return None;
        };
        overrides.xcursor_size = Some(xcursor_size);
        parsed = true;
    }
    if let Some(palette_max_key_assigments_for_action) =
        lua_config_usize_assignment_from_query(config, "palette_max_key_assigments_for_action")
    {
        overrides.palette_max_key_assigments_for_action =
            Some(palette_max_key_assigments_for_action);
        parsed = true;
    }
    if let Some(allow_win32_input_mode) =
        lua_config_bool_assignment_from_query(config, "allow_win32_input_mode")
    {
        overrides.allow_win32_input_mode = Some(allow_win32_input_mode);
        parsed = true;
    }
    if let Some(treat_left_ctrlalt_as_altgr) =
        lua_config_bool_assignment_from_query(config, "treat_left_ctrlalt_as_altgr")
    {
        overrides.treat_left_ctrlalt_as_altgr = Some(treat_left_ctrlalt_as_altgr);
        parsed = true;
    }
    if let Some(send_composed_key_when_left_alt_is_pressed) =
        lua_config_bool_assignment_from_query(config, "send_composed_key_when_left_alt_is_pressed")
    {
        overrides.send_composed_key_when_left_alt_is_pressed =
            Some(send_composed_key_when_left_alt_is_pressed);
        parsed = true;
    }
    if let Some(send_composed_key_when_right_alt_is_pressed) =
        lua_config_bool_assignment_from_query(config, "send_composed_key_when_right_alt_is_pressed")
    {
        overrides.send_composed_key_when_right_alt_is_pressed =
            Some(send_composed_key_when_right_alt_is_pressed);
        parsed = true;
    }
    if let Some(treat_east_asian_ambiguous_width_as_wide) =
        lua_config_bool_assignment_from_query(config, "treat_east_asian_ambiguous_width_as_wide")
    {
        overrides.treat_east_asian_ambiguous_width_as_wide =
            Some(treat_east_asian_ambiguous_width_as_wide);
        parsed = true;
    }
    if let Some(normalize_output_to_unicode_nfc) =
        lua_config_bool_assignment_from_query(config, "normalize_output_to_unicode_nfc")
    {
        overrides.normalize_output_to_unicode_nfc = Some(normalize_output_to_unicode_nfc);
        parsed = true;
    }
    if let Some(unicode_version) = lua_config_usize_assignment_from_query(config, "unicode_version")
    {
        let Ok(unicode_version) = u32::try_from(unicode_version) else {
            return None;
        };
        overrides.unicode_version = Some(unicode_version);
        parsed = true;
    }
    if let Some(bidi_enabled) = lua_config_bool_assignment_from_query(config, "bidi_enabled") {
        overrides.bidi_enabled = Some(bidi_enabled);
        parsed = true;
    }
    if let Some(bidi_direction) = lua_config_string_assignment_from_query(config, "bidi_direction")
    {
        overrides.bidi_direction = Some(NativeBidiDirection::parse(&bidi_direction)?);
        parsed = true;
    }
    if let Some(use_ime) = lua_config_bool_assignment_from_query(config, "use_ime") {
        overrides.use_ime = Some(use_ime);
        parsed = true;
    }
    if let Some(use_dead_keys) = lua_config_bool_assignment_from_query(config, "use_dead_keys") {
        overrides.use_dead_keys = Some(use_dead_keys);
        parsed = true;
    }
    if let Some(ime_preedit_rendering) =
        lua_config_string_assignment_from_query(config, "ime_preedit_rendering")
    {
        overrides.ime_preedit_rendering =
            Some(NativeImePreeditRendering::parse(&ime_preedit_rendering)?);
        parsed = true;
    }
    if let Some(macos_forward_to_ime_modifier_mask) =
        lua_config_string_assignment_from_query(config, "macos_forward_to_ime_modifier_mask")
    {
        overrides.macos_forward_to_ime_modifier_mask = Some(
            native_modifiers_from_wezterm_lua_config(&macos_forward_to_ime_modifier_mask)?,
        );
        parsed = true;
    }
    if let Some(xim_im_name) = lua_config_string_assignment_from_query(config, "xim_im_name") {
        overrides.xim_im_name = Some(xim_im_name);
        parsed = true;
    }
    if let Some(detect_password_input) =
        lua_config_bool_assignment_from_query(config, "detect_password_input")
    {
        overrides.detect_password_input = Some(detect_password_input);
        parsed = true;
    }
    if let Some(disable_default_key_bindings) =
        lua_config_bool_assignment_from_query(config, "disable_default_key_bindings")
    {
        overrides.disable_default_key_bindings = Some(disable_default_key_bindings);
        parsed = true;
    }
    if let Some(disable_default_mouse_bindings) =
        lua_config_bool_assignment_from_query(config, "disable_default_mouse_bindings")
    {
        overrides.disable_default_mouse_bindings = Some(disable_default_mouse_bindings);
        parsed = true;
    }
    if let Some(hide_mouse_cursor_when_typing) =
        lua_config_bool_assignment_from_query(config, "hide_mouse_cursor_when_typing")
    {
        overrides.hide_mouse_cursor_when_typing = Some(hide_mouse_cursor_when_typing);
        parsed = true;
    }
    if let Some(pane_focus_follows_mouse) =
        lua_config_bool_assignment_from_query(config, "pane_focus_follows_mouse")
    {
        overrides.pane_focus_follows_mouse = Some(pane_focus_follows_mouse);
        parsed = true;
    }
    if let Some(swallow_mouse_click_on_pane_focus) =
        lua_config_bool_assignment_from_query(config, "swallow_mouse_click_on_pane_focus")
    {
        overrides.swallow_mouse_click_on_pane_focus = Some(swallow_mouse_click_on_pane_focus);
        parsed = true;
    }
    if let Some(swallow_mouse_click_on_window_focus) =
        lua_config_bool_assignment_from_query(config, "swallow_mouse_click_on_window_focus")
    {
        overrides.swallow_mouse_click_on_window_focus = Some(swallow_mouse_click_on_window_focus);
        parsed = true;
    }
    if let Some(bypass_mouse_reporting_modifiers) =
        lua_config_string_assignment_from_query(config, "bypass_mouse_reporting_modifiers")
    {
        overrides.bypass_mouse_reporting_modifiers = Some(
            native_modifiers_from_wezterm_lua_config(&bypass_mouse_reporting_modifiers)?,
        );
        parsed = true;
    }
    if let Some(automatically_reload_config) =
        lua_config_bool_assignment_from_query(config, "automatically_reload_config")
    {
        overrides.automatically_reload_config = Some(automatically_reload_config);
        parsed = true;
    }
    if let Some(check_for_updates) =
        lua_config_bool_assignment_from_query(config, "check_for_updates")
    {
        overrides.check_for_updates = Some(check_for_updates);
        parsed = true;
    }
    if let Some(check_for_updates_interval_seconds) =
        lua_config_usize_assignment_from_query(config, "check_for_updates_interval_seconds")
    {
        overrides.check_for_updates_interval_seconds =
            Some(u64::try_from(check_for_updates_interval_seconds).ok()?);
        parsed = true;
    }
    if let Some(show_update_window) =
        lua_config_bool_assignment_from_query(config, "show_update_window")
    {
        overrides.show_update_window = Some(show_update_window);
        parsed = true;
    }
    Some(parsed)
}

#[expect(
    clippy::too_many_lines,
    reason = "each ordered compatibility parser group keeps a coherent Lua configuration domain"
)]
fn parse_native_config_group_9(
    config: &str,
    _config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(native_macos_fullscreen_mode) =
        lua_config_bool_assignment_from_query(config, "native_macos_fullscreen_mode")
    {
        overrides.native_macos_fullscreen_mode = Some(native_macos_fullscreen_mode);
        parsed = true;
    }
    if let Some(macos_fullscreen_extend_behind_notch) =
        lua_config_bool_assignment_from_query(config, "macos_fullscreen_extend_behind_notch")
    {
        overrides.macos_fullscreen_extend_behind_notch = Some(macos_fullscreen_extend_behind_notch);
        parsed = true;
    }
    if let Some(use_resize_increments) =
        lua_config_bool_assignment_from_query(config, "use_resize_increments")
    {
        overrides.use_resize_increments = Some(use_resize_increments);
        parsed = true;
    }
    if let Some(debug_key_events) =
        lua_config_bool_assignment_from_query(config, "debug_key_events")
    {
        overrides.debug_key_events = Some(debug_key_events);
        parsed = true;
    }
    if let Some(log_unknown_escape_sequences) =
        lua_config_bool_assignment_from_query(config, "log_unknown_escape_sequences")
    {
        overrides.log_unknown_escape_sequences = Some(log_unknown_escape_sequences);
        parsed = true;
    }
    if let Some(warn_about_missing_glyphs) =
        lua_config_bool_assignment_from_query(config, "warn_about_missing_glyphs")
    {
        overrides.warn_about_missing_glyphs = Some(warn_about_missing_glyphs);
        parsed = true;
    }
    if let Some(scroll_to_bottom_on_input) =
        lua_config_bool_assignment_from_query(config, "scroll_to_bottom_on_input")
    {
        overrides.scroll_to_bottom_on_input = Some(scroll_to_bottom_on_input);
        parsed = true;
    }
    if let Some(alternate_buffer_wheel_scroll_speed) =
        lua_config_usize_assignment_from_query(config, "alternate_buffer_wheel_scroll_speed")
    {
        overrides.alternate_buffer_wheel_scroll_speed = Some(alternate_buffer_wheel_scroll_speed);
        parsed = true;
    }
    if let Some(scrollback_lines) =
        lua_config_usize_assignment_from_query(config, "scrollback_lines")
    {
        overrides.scrollback_lines = Some(scrollback_lines);
        parsed = true;
    }
    if let Some(enable_scroll_bar) =
        lua_config_bool_assignment_from_query(config, "enable_scroll_bar")
    {
        overrides.enable_scroll_bar = Some(enable_scroll_bar);
        parsed = true;
    }
    if let Some(min_scroll_bar_height) =
        lua_config_string_assignment_from_query(config, "min_scroll_bar_height")
    {
        overrides.min_scroll_bar_height =
            Some(NativeScrollBarHeight::parse(&min_scroll_bar_height)?);
        parsed = true;
    } else if let Some(min_scroll_bar_height) =
        lua_config_usize_assignment_from_query(config, "min_scroll_bar_height")
    {
        overrides.min_scroll_bar_height = Some(NativeScrollBarHeight::Pixels(
            u32::try_from(min_scroll_bar_height).ok()?,
        ));
        parsed = true;
    }
    if let Some(window_padding) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "window_padding",
        )
        .and_then(|window_padding| {
            native_window_padding_lua_table_from_query(
                config,
                &window_padding.value,
                Some(window_padding.max_start),
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(config, "window_padding")
                .and_then(|window_padding| {
                    native_window_padding_lua_table_from_query(
                        config,
                        window_padding,
                        lua_source_slice_start_offset(config, window_padding),
                    )
                })
        })
    {
        overrides.window_padding = Some(window_padding);
        parsed = true;
    }
    if let Some(window_content_alignment) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "window_content_alignment",
        )
        .and_then(|window_content_alignment| {
            native_window_content_alignment_lua_table_from_query(
                config,
                &window_content_alignment.value,
                Some(window_content_alignment.max_start),
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(
                config,
                "window_content_alignment",
            )
            .and_then(|window_content_alignment| {
                native_window_content_alignment_lua_table_from_query(
                    config,
                    window_content_alignment,
                    lua_source_slice_start_offset(config, window_content_alignment),
                )
            })
        })
    {
        overrides.window_content_alignment = Some(window_content_alignment);
        parsed = true;
    }
    if let Some(enable_tab_bar) = lua_config_bool_assignment_from_query(config, "enable_tab_bar") {
        overrides.enable_tab_bar = Some(enable_tab_bar);
        parsed = true;
    }
    if let Some(hide_tab_bar_if_only_one_tab) =
        lua_config_bool_assignment_from_query(config, "hide_tab_bar_if_only_one_tab")
    {
        overrides.hide_tab_bar_if_only_one_tab = Some(hide_tab_bar_if_only_one_tab);
        parsed = true;
    }
    if let Some(use_fancy_tab_bar) =
        lua_config_bool_assignment_from_query(config, "use_fancy_tab_bar")
    {
        overrides.use_fancy_tab_bar = Some(use_fancy_tab_bar);
        parsed = true;
    }
    if let Some(unzoom_on_switch_pane) =
        lua_config_bool_assignment_from_query(config, "unzoom_on_switch_pane")
    {
        overrides.unzoom_on_switch_pane = Some(unzoom_on_switch_pane);
        parsed = true;
    }
    if let Some(tab_bar_at_bottom) =
        lua_config_bool_assignment_from_query(config, "tab_bar_at_bottom")
    {
        overrides.tab_bar_at_bottom = Some(tab_bar_at_bottom);
        parsed = true;
    }
    if let Some(tab_and_split_indices_are_zero_based) =
        lua_config_bool_assignment_from_query(config, "tab_and_split_indices_are_zero_based")
    {
        overrides.tab_and_split_indices_are_zero_based = Some(tab_and_split_indices_are_zero_based);
        parsed = true;
    }
    if let Some(mouse_wheel_scrolls_tabs) =
        lua_config_bool_assignment_from_query(config, "mouse_wheel_scrolls_tabs")
    {
        overrides.mouse_wheel_scrolls_tabs = Some(mouse_wheel_scrolls_tabs);
        parsed = true;
    }
    if let Some(switch_to_last_active_tab_when_closing_tab) =
        lua_config_bool_assignment_from_query(config, "switch_to_last_active_tab_when_closing_tab")
    {
        overrides.switch_to_last_active_tab_when_closing_tab =
            Some(switch_to_last_active_tab_when_closing_tab);
        parsed = true;
    }
    if let Some(tab_shortcut_style) =
        lua_config_string_assignment_from_query(config, "tab_shortcut_style")
    {
        overrides.tab_shortcut_style = Some(NativeTabShortcutStyle::parse(&tab_shortcut_style)?);
        parsed = true;
    }
    if let Some(closed_tab_history_size) =
        lua_config_usize_assignment_from_query(config, "closed_tab_history_size")
    {
        overrides.closed_tab_history_size = Some(closed_tab_history_size);
        parsed = true;
    }
    if let Some(close_tab_selection) =
        lua_config_string_assignment_from_query(config, "close_tab_selection")
    {
        overrides.close_tab_selection = Some(match close_tab_selection.trim() {
            "adjacent" => CloseTabSelection::Adjacent,
            "last-active" => CloseTabSelection::LastActive,
            "left" => CloseTabSelection::Left,
            _ => return None,
        });
        parsed = true;
    }
    if let Some(tab_bar_wheel_behavior) =
        lua_config_string_assignment_from_query(config, "tab_bar_wheel_behavior")
    {
        overrides.tab_bar_wheel_behavior =
            Some(NativeTabBarWheelBehavior::parse(&tab_bar_wheel_behavior)?);
        parsed = true;
    }
    if let Some(quit_when_all_windows_are_closed) =
        lua_config_bool_assignment_from_query(config, "quit_when_all_windows_are_closed")
    {
        overrides.quit_when_all_windows_are_closed = Some(quit_when_all_windows_are_closed);
        parsed = true;
    }
    if let Some(window_close_confirmation) =
        lua_config_string_assignment_from_query(config, "window_close_confirmation")
    {
        overrides.window_close_confirmation = Some(NativeWindowCloseConfirmation::parse(
            &window_close_confirmation,
        )?);
        parsed = true;
    }
    if let Some(show_close_tab_button_in_tabs) =
        lua_config_bool_assignment_from_query(config, "show_close_tab_button_in_tabs")
    {
        overrides.show_close_tab_button_in_tabs = Some(show_close_tab_button_in_tabs);
        parsed = true;
    }
    if let Some(show_new_tab_button_in_tab_bar) =
        lua_config_bool_assignment_from_query(config, "show_new_tab_button_in_tab_bar")
    {
        overrides.show_new_tab_button_in_tab_bar = Some(show_new_tab_button_in_tab_bar);
        parsed = true;
    }
    if let Some(show_tab_index_in_tab_bar) =
        lua_config_bool_assignment_from_query(config, "show_tab_index_in_tab_bar")
    {
        overrides.show_tab_index_in_tab_bar = Some(show_tab_index_in_tab_bar);
        parsed = true;
    }
    if let Some(show_tabs_in_tab_bar) =
        lua_config_bool_assignment_from_query(config, "show_tabs_in_tab_bar")
    {
        overrides.show_tabs_in_tab_bar = Some(show_tabs_in_tab_bar);
        parsed = true;
    }
    if let Some(skip_close_confirmation_for_processes_named) =
        lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "skip_close_confirmation_for_processes_named",
        )
    {
        overrides.skip_close_confirmation_for_processes_named =
            Some(split_lua_table_string_array_with_static_source(
                Some(LuaStaticSource {
                    source: config,
                    max_start: skip_close_confirmation_for_processes_named.max_start,
                }),
                &skip_close_confirmation_for_processes_named.value,
            )?);
        parsed = true;
    }
    if let Some(exit_behavior) = lua_config_string_assignment_from_query(config, "exit_behavior") {
        overrides.exit_behavior = Some(NativeExitBehavior::parse(&exit_behavior)?);
        parsed = true;
    }
    Some(parsed)
}

fn parse_native_config_group_10(
    config: &str,
    _config_receiver: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    if let Some(clean_exit_codes) =
        lua_config_u32_array_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "clean_exit_codes",
        )
    {
        overrides.clean_exit_codes = Some(split_lua_table_u32_array(&clean_exit_codes.value)?);
        parsed = true;
    }
    if let Some(exit_behavior_messaging) =
        lua_config_string_assignment_from_query(config, "exit_behavior_messaging")
    {
        overrides.exit_behavior_messaging = Some(NativeExitBehaviorMessaging::parse(
            &exit_behavior_messaging,
        )?);
        parsed = true;
    }
    if let Some(keys) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(config, "keys")
    {
        overrides.key_assignments = Some(native_key_assignments_lua_table_from_query(
            Some(LuaStaticSource {
                source: config,
                max_start: keys.max_start,
            }),
            &keys.value,
        )?);
        parsed = true;
    }
    if let Some(key_tables) =
        lua_config_key_tables_assignment_with_insert_appends_with_max_start_from_query(config)
    {
        overrides.key_tables = Some(native_key_tables_lua_table_from_query(
            Some(LuaStaticSource {
                source: config,
                max_start: key_tables.max_start,
            }),
            &key_tables.value,
        )?);
        parsed = true;
    }
    if let Some(mouse_bindings) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "mouse_bindings",
        )
    {
        overrides.mouse_assignments = Some(native_mouse_assignments_lua_table_from_query(
            Some(LuaStaticSource {
                source: config,
                max_start: mouse_bindings.max_start,
            }),
            &mouse_bindings.value,
        )?);
        parsed = true;
    }
    if let Some(leader) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(config, "leader")
    {
        overrides.leader = Some(native_leader_lua_table_from_query(
            config,
            &leader.value,
            leader.max_start,
        )?);
        parsed = true;
    }
    if let Some(launch_menu) =
        lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
            config,
            "launch_menu",
        )
    {
        overrides.launch_menu = Some(native_launch_menu_lua_table_from_query(
            config,
            &launch_menu.value,
            launch_menu.max_start,
        )?);
        parsed = true;
    }

    Some(parsed)
}

include!("window_parts/part02.rs");
include!("window_parts/part03.rs");
include!("window_parts/part04.rs");
include!("window_parts/part05.rs");
include!("window_parts/part06.rs");
include!("window_parts/part06b.rs");
include!("window_parts/part07.rs");
include!("window_parts/diagnostics.rs");
include!("window_parts/part08.rs");
include!("window_parts/part09.rs");
include!("window_parts/part10.rs");
include!("window_parts/part11.rs");
include!("window_parts/part12.rs");
include!("window_parts/part13.rs");
include!("window_parts/part14.rs");
include!("window_parts/part15.rs");
include!("window_parts/tab_session.rs");
include!("window_parts/runtime_helpers.rs");
include!("window_parts/functional_observer.rs");

#[cfg(all(test, feature = "ssh"))]
mod ssh_gui_startup_contract_tests {
    use super::*;
    use rssh_diagnostics::DiagnosticGpuBackend;

    fn gui_options_with_size(columns: u16, rows: u16) -> SshOptions {
        let command = crate::cli::parse_args([
            "rssh",
            "ssh",
            "--gui",
            "--host",
            "example.test",
            "--user",
            "alice",
            "--cols",
            &columns.to_string(),
            "--rows",
            &rows.to_string(),
        ])
        .expect("SSH GUI arguments should parse");
        let crate::cli::AppCommand::Ssh(options) = command else {
            panic!("expected SSH command");
        };
        options
    }

    #[test]
    fn ssh_gui_initial_size_configures_terminal_and_window() {
        let options = gui_options_with_size(132, 43);
        let mut app = NativeWindowApp::new_with_workspace_class_position_and_osc52_policy(
            None,
            options.osc52_policy,
            PtyCommand::default_shell(),
            None,
            None,
            None,
        );

        configure_ssh_gui_initial_size(&mut app, &options);

        let expected = TerminalSize::new(132, 43);
        assert_eq!(app.runtime.terminal().grid().size(), expected);
        assert_eq!(app.initial_cols, expected.columns);
        assert_eq!(app.initial_rows, expected.rows);
        assert_eq!(app.initial_frame_size(), app.frame_size_for_terminal_size(expected));
    }

    #[test]
    fn openssh_gui_initial_size_configures_terminal_and_window() {
        let command = crate::cli::parse_args([
            "rssh", "ssh", "--gui", "--target", "prod", "--cols", "101", "--rows", "37",
        ])
        .expect("OpenSSH GUI arguments should parse");
        let crate::cli::AppCommand::Ssh(options) = command else {
            panic!("expected SSH command");
        };
        let mut app = NativeWindowApp::new_with_workspace_class_position_and_osc52_policy(
            None,
            options.osc52_policy,
            PtyCommand::default_shell(),
            None,
            None,
            None,
        );

        configure_ssh_gui_initial_size(&mut app, &options);

        let expected = TerminalSize::new(101, 37);
        assert_eq!(app.runtime.terminal().grid().size(), expected);
        assert_eq!(app.initial_frame_size(), app.frame_size_for_terminal_size(expected));
    }

    #[test]
    fn diagnostic_gpu_backend_defaults_to_none_and_setter_stores_selection() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.startup.diagnostic_gpu_backend, None);
        app.set_diagnostic_gpu_backend(Some(DiagnosticGpuBackend::Dx12));
        assert_eq!(app.startup.diagnostic_gpu_backend, Some(DiagnosticGpuBackend::Dx12));
    }

    #[test]
    fn diagnostic_gpu_ready_extra_contains_only_safe_adapter_identity() {
        let mut metrics = GpuPresentationMetrics::uninitialized();
        metrics.backend = "Vulkan".to_owned();
        metrics.adapter_name = "fixture-adapter".to_owned();
        metrics.adapter_vendor_id = 0x10de;
        metrics.adapter_device_id = 0x2684;
        metrics.adapter_type = "discrete-gpu".to_owned();

        let extra = gpu_ready_extra(&metrics);

        assert_eq!(
            extra.keys().map(String::as_str).collect::<HashSet<_>>(),
            HashSet::from([
                "gpu_backend",
                "gpu_adapter_name",
                "gpu_adapter_vendor_id",
                "gpu_adapter_device_id",
                "gpu_adapter_type",
            ])
        );
        assert_eq!(extra["gpu_backend"], serde_json::json!("Vulkan"));
        assert_eq!(
            extra["gpu_adapter_name"],
            serde_json::json!("fixture-adapter")
        );
        assert_eq!(extra["gpu_adapter_vendor_id"], serde_json::json!(0x10de));
        assert_eq!(extra["gpu_adapter_device_id"], serde_json::json!(0x2684));
        assert_eq!(
            extra["gpu_adapter_type"],
            serde_json::json!("discrete-gpu")
        );
    }

    #[test]
    fn product_gui_probe_accepts_the_closed_v1_descriptor() {
        let probe = parse_product_gui_probe(
            r#"{"schema":"rssh.stage7/product-gui-probe/v1","run_id":"stage7-product-42","scenario":"ssh1","hold_ms":38000}"#,
        )
        .expect("closed product GUI descriptor");

        assert_eq!(probe.run_id, "stage7-product-42");
        assert_eq!(probe.scenario, DiagnosticScenario::Ssh1);
        assert_eq!(probe.hold_duration, Duration::from_millis(38_000));
    }

    #[test]
    fn product_gui_probe_rejects_unknown_or_secret_fields() {
        for descriptor in [
            r#"{"schema":"rssh.stage7/product-gui-probe/v1","run_id":"stage7-product-42","scenario":"ssh1","hold_ms":38000,"secret":"forbidden"}"#,
            r#"{"schema":"rssh.stage7/product-gui-probe/v1","run_id":"not a stable id","scenario":"ssh1","hold_ms":38000}"#,
            r#"{"schema":"rssh.stage7/product-gui-probe/v1","run_id":"stage7-product-42","scenario":"local","hold_ms":38000}"#,
            r#"{"schema":"rssh.stage7/product-gui-probe/v1","run_id":"stage7-product-42","scenario":"ssh1","hold_ms":4999}"#,
            r#"{"schema":"rssh.stage7/product-gui-probe/v1","run_id":"stage7-product-42","scenario":"ssh1","hold_ms":300001}"#,
        ] {
            assert!(
                parse_product_gui_probe(descriptor).is_err(),
                "accepted invalid product GUI descriptor: {descriptor}"
            );
        }
    }
}
