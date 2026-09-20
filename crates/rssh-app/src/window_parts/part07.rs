impl NativeWindowConfigPatch {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn merge(&mut self, update: Self) {
        self.0.merge(*update.0);
    }

    fn apply_to_native_config_overrides(self, overrides: &mut NativeConfigSnapshot) {
        self.0.apply_to_native_config_overrides(overrides);
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaUserVarChanged {
    left_status: Option<NativeLuaUserVarChangedStatusText>,
    right_status: Option<NativeLuaUserVarChangedStatusText>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaUserVarChangedStatusText {
    parts: Vec<NativeLuaUserVarChangedStatusPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaUserVarChangedPaneUserVarSource {
    EventPane,
    ActivePane,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaUserVarChangedStatusPart {
    Static(String),
    WindowId,
    PaneId,
    PaneUserVar {
        source: NativeLuaUserVarChangedPaneUserVarSource,
        name: String,
        fallback: String,
    },
    Name,
    Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaWindowStatusText {
    Static(String),
    ActiveWorkspace,
    WindowId {
        prefix: String,
        suffix: String,
    },
    WindowPane {
        parts: Vec<NativeLuaWindowPaneStatusPart>,
    },
    ActiveKeyTable {
        prefix: String,
        fallback: String,
    },
    CompositionStatus {
        prefix: String,
        fallback: String,
    },
    Leader {
        active: String,
        inactive: String,
    },
    Focus {
        focused: String,
        unfocused: String,
    },
    PaneAltScreen {
        active: String,
        inactive: String,
    },
    PaneHasUnseenOutput {
        unseen: String,
        seen: String,
    },
    WindowDimensions {
        parts: Vec<NativeLuaWindowDimensionsStatusPart>,
    },
    WindowEffectiveConfig {
        parts: Vec<NativeLuaWindowEffectiveConfigStatusPart>,
    },
    PaneDimensions {
        parts: Vec<NativeLuaPaneDimensionsStatusPart>,
    },
    PaneCursorPosition {
        parts: Vec<NativeLuaPaneCursorPositionStatusPart>,
    },
    PaneUserVars {
        parts: Vec<NativeLuaPaneUserVarsStatusPart>,
    },
    PaneProgress {
        none: String,
        percentage_prefix: String,
        error_prefix: String,
        indeterminate: String,
    },
    KeyboardModifiers {
        parts: Vec<NativeLuaKeyboardModifiersStatusPart>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaWindowPaneStatusPart {
    Static(String),
    ActiveWorkspace,
    WindowId,
    ActiveTabId,
    ActiveTabTitle,
    PaneId,
    PaneTitle,
    PaneDomainName,
    PaneCurrentWorkingDir,
    PaneForegroundProcessName,
    PaneTtyName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaWindowDimensionsStatusPart {
    Static(String),
    Field(NativeLuaWindowDimensionsField),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaWindowEffectiveConfigStatusPart {
    Static(String),
    Field(NativeLuaWindowEffectiveConfigField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaWindowDimensionsField {
    PixelWidth,
    PixelHeight,
    Dpi,
    IsFullScreen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaWindowEffectiveConfigField {
    FontSize,
    DefaultWorkspace,
    DefaultProg(usize),
    DefaultGuiStartupArg(usize),
    DefaultCwd,
    DefaultDomain,
    PreferToSpawnTabs,
    SshBackend,
    StatusUpdateInterval,
    TabMaxWidth,
    Dpi,
    DpiByScreen(String),
    ResolvedPalette(NativeLuaResolvedPaletteField),
    VisualBell(NativeLuaVisualBellField),
    ColorScheme,
    ForegroundColor,
    BackgroundColor,
    MaxFps,
    AnimationFps,
    FrontEnd,
    WebGpuPowerPreference,
    WebGpuForceFallbackAdapter,
    WebGpuPreferredAdapter(NativeLuaWebGpuPreferredAdapterField),
    PreferEgl,
    EnableWayland,
    EnableZwlrOutputManager,
    UseBoxModelRender,
    ExperimentalPixelPositioning,
    IgnoreSvgFonts,
    BidiEnabled,
    BidiDirection,
    CellWidth,
    LineHeight,
    FontAntialias,
    FontHinting,
    FontRasterizer,
    FontColrRasterizer,
    FontShaper,
    HarfbuzzFeature(usize),
    FontDir(usize),
    CellWidths(usize, NativeLuaCellWidthOverrideField),
    FontLocator,
    UseCapHeightToScaleFallbackFonts,
    SortFallbackFontsByCoverage,
    SearchFontDirsForFallback,
    FreetypeLoadTarget,
    FreetypeRenderTarget,
    FreetypeLoadFlags,
    FreetypeInterpreterVersion,
    FreetypePcfLongFamilyNames,
    BoldBrightensAnsiColors,
    AllowSquareGlyphsToOverflowWidth,
    DisplayPixelGeometry,
    TextBackgroundOpacity,
    WindowBackgroundOpacity,
    ForegroundTextHsbHue,
    ForegroundTextHsbSaturation,
    ForegroundTextHsbBrightness,
    InactivePaneHsbHue,
    InactivePaneHsbSaturation,
    InactivePaneHsbBrightness,
    ShapeCacheSize,
    LineStateCacheSize,
    LineQuadCacheSize,
    LineToEleShapeCacheSize,
    GlyphCacheImageCacheSize,
    CursorBlinkRate,
    CursorBlinkEaseIn,
    CursorBlinkEaseOut,
    TextBlinkRate,
    TextBlinkRateRapid,
    TextBlinkEaseIn,
    TextBlinkEaseOut,
    TextBlinkRapidEaseIn,
    TextBlinkRapidEaseOut,
    CursorThickness,
    UnderlineThickness,
    UnderlinePosition,
    StrikethroughPosition,
    HideMouseCursorWhenTyping,
    DefaultMuxServerDomain,
    DaemonOption(NativeLuaDaemonOptionsField),
    RatelimitMuxLinePrefetchesPerSecond,
    MuxOutputParserBufferSize,
    MuxOutputParserCoalesceDelayMs,
    MuxEnvRemove(usize),
    SetEnvironmentVariable(String),
    PeriodicStatLogging,
    UlimitNofile,
    UlimitNproc,
    ScrollToBottomOnInput,
    UseIme,
    XimImName,
    ImePreeditRendering,
    MacosForwardToImeModifierMask,
    NotificationHandling,
    UseDeadKeys,
    AudibleBell,
    LaunchMenu(usize, NativeLuaLaunchMenuField),
    AutomaticallyReloadConfig,
    CheckForUpdates,
    ShowUpdateWindow,
    CheckForUpdatesIntervalSeconds,
    EnableKittyGraphics,
    EnableChecksumRectangularArea,
    EnableTitleReporting,
    EnableCsiUKeyEncoding,
    EnableKittyKeyboard,
    AllowDownloadProtocols,
    XcursorTheme,
    XcursorSize,
    PaletteMaxKeyAssigmentsForAction,
    AllowWin32InputMode,
    TreatLeftCtrlAltAsAltGr,
    SendComposedKeyWhenLeftAltIsPressed,
    SendComposedKeyWhenRightAltIsPressed,
    TreatEastAsianAmbiguousWidthAsWide,
    NormalizeOutputToUnicodeNfc,
    UnicodeVersion,
    WindowCloseConfirmation,
    EnableTabBar,
    UseFancyTabBar,
    TabBarAtBottom,
    MouseWheelScrollsTabs,
    ShowCloseTabButtonInTabs,
    ShowNewTabButtonInTabBar,
    ShowTabIndexInTabBar,
    ShowTabsInTabBar,
    TabAndSplitIndicesAreZeroBased,
    HideTabBarIfOnlyOneTab,
    WarnAboutMissingGlyphs,
    PaneFocusFollowsMouse,
    SwallowMouseClickOnPaneFocus,
    SwallowMouseClickOnWindowFocus,
    BypassMouseReportingModifiers,
    UnzoomOnSwitchPane,
    QuitWhenAllWindowsAreClosed,
    DefaultCursorStyle,
    ForceReverseVideoCursor,
    ReverseVideoCursorMinContrast,
    TextMinContrastRatio,
    CommandPaletteRows,
    CommandPaletteFontSize,
    CommandPaletteBgColor,
    CommandPaletteFgColor,
    CharSelectFontSize,
    CharSelectBgColor,
    CharSelectFgColor,
    PaneSelectFontSize,
    PaneSelectBgColor,
    PaneSelectFgColor,
    LauncherAlphabet,
    QuickSelectAlphabet,
    QuickSelectPattern(usize),
    HyperlinkRule(usize, NativeLuaHyperlinkRuleField),
    ColorSchemeDir(usize),
    CleanExitCode(usize),
    DisableDefaultQuickSelectPatterns,
    QuickSelectRemoveStyling,
    CanonicalizePastedNewlines,
    QuoteDroppedFiles,
    DisableDefaultKeyBindings,
    DisableDefaultMouseBindings,
    DebugKeyEvents,
    KeyMapPreference,
    UiKeyCapRendering,
    SwapBackspaceAndDelete,
    LogUnknownEscapeSequences,
    DefaultSshAuthSock,
    MuxEnableSshAgent,
    DetectPasswordInput,
    EnableScrollBar,
    MinScrollBarHeight,
    CustomBlockGlyphs,
    AntiAliasCustomBlockGlyphs,
    WindowPaddingLeft,
    WindowPaddingRight,
    WindowPaddingTop,
    WindowPaddingBottom,
    WindowContentAlignmentHorizontal,
    WindowContentAlignmentVertical,
    KdeWindowBackgroundBlur,
    MacosWindowBackgroundBlur,
    Win32SystemBackdrop,
    Win32AcrylicAccentColor,
    WindowDecorations,
    IntegratedTitleButton(usize),
    IntegratedTitleButtonAlignment,
    IntegratedTitleButtonColor,
    IntegratedTitleButtonStyle,
    NativeMacosFullscreenMode,
    MacosFullscreenExtendBehindNotch,
    SelectionWordBoundary,
    Term,
    EnqAnswerback,
    InitialCols,
    InitialRows,
    ScrollbackLines,
    SwitchToLastActiveTabWhenClosingTab,
    ExitBehavior,
    ExitBehaviorMessaging,
    AdjustWindowSizeWhenChangingFontSize,
    TilingDesktopEnvironment(usize),
    SkipCloseConfirmationProcess(usize),
    UseResizeIncrements,
    AlternateBufferWheelScrollSpeed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaWebGpuPreferredAdapterField {
    Backend,
    Device,
    DeviceType,
    Driver,
    DriverInfo,
    Name,
    Vendor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaVisualBellField {
    FadeInDurationMs,
    FadeOutDurationMs,
    FadeInFunction,
    FadeOutFunction,
    Target,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaDaemonOptionsField {
    PidFile,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaCellWidthOverrideField {
    First,
    Last,
    Width,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaHyperlinkRuleField {
    Regex,
    Format,
    Highlight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaLaunchMenuField {
    Label,
    Arg(usize),
    Cwd,
    Domain,
    SetEnvironmentVariable(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaResolvedPaletteField {
    Foreground,
    Background,
    CursorBg,
    CursorFg,
    CursorBorder,
    SelectionFg,
    SelectionBg,
    ComposeCursor,
    VisualBell,
    Ansi(usize),
    Bright(usize),
    Indexed(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaPaneDimensionsStatusPart {
    Static(String),
    Field(NativeLuaPaneDimensionsField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaPaneDimensionsField {
    Cols,
    ViewportRows,
    ScrollbackRows,
    PhysicalTop,
    ScrollbackTop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaPaneCursorPositionStatusPart {
    Static(String),
    Field(NativeLuaPaneCursorPositionField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaPaneCursorPositionField {
    X,
    Y,
    Shape,
    Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaPaneUserVarsStatusPart {
    Static(String),
    UserVar { name: String, fallback: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaKeyboardModifiersStatusPart {
    Static(String),
    Modifiers,
    Leds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaOpenUri {
    Sequence(Vec<NativeLuaOpenUri>),
    Static {
        allow_default: bool,
    },
    UriPrefix {
        prefix: String,
        allow_default: bool,
        action: Option<NativeLuaOpenUriAction>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaOpenUriAction {
    SpawnCommandInNewWindow { args: Vec<NativeLuaOpenUriArg> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaOpenUriArg {
    Static(String),
    UriSuffix,
}

impl NativeLuaOpenUri {
    fn allows_default(&self, event: &NativeWindowOpenUri) -> bool {
        match self {
            Self::Sequence(handlers) => {
                handlers.iter().all(|handler| handler.allows_default(event))
            }
            Self::Static { allow_default } => *allow_default,
            Self::UriPrefix {
                prefix,
                allow_default,
                ..
            } => {
                if event.uri.starts_with(prefix) {
                    *allow_default
                } else {
                    true
                }
            }
        }
    }

    fn command_for_event(&self, event: &NativeWindowOpenUri) -> Option<WindowCommand> {
        match self {
            Self::Sequence(handlers) => {
                for handler in handlers {
                    if let Some(command) = handler.command_for_event(event) {
                        return Some(command);
                    }
                    if !handler.allows_default(event) {
                        return None;
                    }
                }
                None
            }
            Self::UriPrefix {
                prefix,
                action: Some(action),
                ..
            } if event.uri.starts_with(prefix) => action.command_for_uri(&event.uri, prefix),
            Self::Static { .. } | Self::UriPrefix { .. } => None,
        }
    }
}

impl NativeLuaOpenUriAction {
    fn command_for_uri(&self, uri: &str, prefix: &str) -> Option<WindowCommand> {
        match self {
            Self::SpawnCommandInNewWindow { args } => {
                let mut args = args
                    .iter()
                    .map(|arg| arg.value_for_uri(uri, prefix))
                    .collect::<Option<Vec<_>>>()?;
                if args.is_empty() {
                    return None;
                }
                let program = args.remove(0);
                Some(WindowCommand::SpawnCommandInNewWindow(
                    WindowSpawnCommandQuery {
                        label: None,
                        program,
                        args,
                        cwd: None,
                        environment: BTreeMap::new(),
                        domain: None,
                        window_position: None,
                    },
                ))
            }
        }
    }
}

impl NativeLuaOpenUriArg {
    fn value_for_uri(&self, uri: &str, prefix: &str) -> Option<String> {
        match self {
            Self::Static(value) => Some(value.clone()),
            Self::UriSuffix => uri.strip_prefix(prefix).map(str::to_owned),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeLuaNewTabButtonClick {
    allow_default: NativeLuaNewTabButtonClickAllowDefault,
    perform_default_action: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaNewTabButtonClickAllowDefault {
    Static(bool),
    ButtonConditions {
        defaults: NativeLuaNewTabButtonClickButtonDefaults,
    },
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent compatibility flags represent valid combinations"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeLuaNewTabButtonClickButtonDefaults {
    left_allow_default: bool,
    right_allow_default: bool,
    middle_allow_default: bool,
    other_allow_default: bool,
}

impl NativeLuaNewTabButtonClick {
    fn allows_default(self, event: &NativeWindowNewTabButtonClick) -> bool {
        self.allow_default.allows_default(event)
    }

    const fn performs_default_action(self) -> bool {
        self.perform_default_action
    }
}

impl NativeLuaNewTabButtonClickAllowDefault {
    fn allows_default(self, event: &NativeWindowNewTabButtonClick) -> bool {
        match self {
            Self::Static(allow_default) => allow_default,
            Self::ButtonConditions { defaults } => defaults.allows_default(event.button),
        }
    }
}

impl NativeLuaNewTabButtonClickButtonDefaults {
    fn from_lua_branches(
        branches: &[LuaNewTabButtonClickButtonBranch],
        fallback_allow_default: bool,
    ) -> Self {
        Self {
            left_allow_default: Self::allow_default_for_button(
                branches,
                fallback_allow_default,
                MouseButton::Left,
            ),
            right_allow_default: Self::allow_default_for_button(
                branches,
                fallback_allow_default,
                MouseButton::Right,
            ),
            middle_allow_default: Self::allow_default_for_button(
                branches,
                fallback_allow_default,
                MouseButton::Middle,
            ),
            other_allow_default: Self::allow_default_for_button(
                branches,
                fallback_allow_default,
                MouseButton::Other(0),
            ),
        }
    }

    fn allow_default_for_button(
        branches: &[LuaNewTabButtonClickButtonBranch],
        fallback_allow_default: bool,
        button: MouseButton,
    ) -> bool {
        branches
            .iter()
            .find_map(|branch| branch.matches(button).then_some(branch.allow_default))
            .unwrap_or(fallback_allow_default)
    }

    const fn allows_default(self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.left_allow_default,
            MouseButton::Right => self.right_allow_default,
            MouseButton::Middle => self.middle_allow_default,
            _ => self.other_allow_default,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeWindowLevel {
    AlwaysOnBottom,
    Normal,
    AlwaysOnTop,
}

const fn winit_window_level_for_native(level: NativeWindowLevel) -> WinitWindowLevel {
    match level {
        NativeWindowLevel::AlwaysOnBottom => WinitWindowLevel::AlwaysOnBottom,
        NativeWindowLevel::Normal => WinitWindowLevel::Normal,
        NativeWindowLevel::AlwaysOnTop => WinitWindowLevel::AlwaysOnTop,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowActivateWindowRequest {
    Index(usize),
    Relative { offset: isize, wrap: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowFocusTransitions<Id> {
    blur: Option<Id>,
    focus: Option<Id>,
}

impl<Id> Default for WindowFocusTransitions<Id> {
    fn default() -> Self {
        Self {
            blur: None,
            focus: None,
        }
    }
}

#[derive(Debug)]
struct WindowFocusCoordinator<Id> {
    focused: Option<Id>,
}

impl<Id> Default for WindowFocusCoordinator<Id> {
    fn default() -> Self {
        Self { focused: None }
    }
}

impl<Id: Copy + Eq> WindowFocusCoordinator<Id> {
    #[cfg(test)]
    const fn focused(&self) -> Option<Id> {
        self.focused
    }

    fn apply(&mut self, id: Id, focused: bool) -> WindowFocusTransitions<Id> {
        if focused {
            if self.focused == Some(id) {
                return WindowFocusTransitions::default();
            }
            let blur = self.focused.replace(id);
            return WindowFocusTransitions {
                blur,
                focus: Some(id),
            };
        }

        if self.focused == Some(id) {
            self.focused = None;
            return WindowFocusTransitions {
                blur: Some(id),
                focus: None,
            };
        }

        WindowFocusTransitions::default()
    }

    fn remove(&mut self, id: Id) -> bool {
        if self.focused != Some(id) {
            return false;
        }
        self.focused = None;
        true
    }
}

fn apply_focus_transitions<Id: Copy + Eq>(
    focus: &mut WindowFocusCoordinator<Id>,
    id: Id,
    focused: bool,
) -> WindowFocusTransitions<Id> {
    focus.apply(id, focused)
}

fn dispatch_window_focus_changed<Id: Copy + Eq + std::hash::Hash>(
    focus: &mut WindowFocusCoordinator<Id>,
    apps: &mut HashMap<Id, Box<NativeWindowApp>>,
    id: Id,
    focused: bool,
) -> io::Result<()> {
    if !apps.contains_key(&id) {
        return Ok(());
    }

    let transitions = apply_focus_transitions(focus, id, focused);
    if let Some(blur) = transitions.blur
        && let Some(app) = apps.get_mut(&blur)
    {
        let _ = app.handle_focus_changed(false)?;
        if let Some(window) = &app.window {
            window.request_redraw();
        }
    }
    if let Some(focus) = transitions.focus
        && let Some(app) = apps.get_mut(&focus)
    {
        let _ = app.handle_focus_changed(true)?;
        if let Some(window) = &app.window {
            window.request_redraw();
        }
    }
    Ok(())
}

type TabTitleFormatter = dyn Fn(&NativeTabTitleFormat) -> Option<NativeTabTitle> + Send;
type WindowTitleFormatter = dyn Fn(&NativeWindowTitleFormat) -> Option<String> + Send;
type WindowStatusUpdateHandler =
    dyn FnMut(&NativeWindowStatusUpdateEvent) -> NativeWindowStatusUpdate + Send;
type WindowRightStatusUpdateHandler =
    dyn FnMut(&NativeWindowStatusUpdateEvent) -> Option<String> + Send;
type CommandPaletteAugmenter =
    dyn FnMut(&NativeCommandPaletteAugment) -> Vec<NativeCommandPaletteEntry> + Send;

struct NativeWindowStartup {
    command: PtyCommand,
    workspace: Option<String>,
    window_class: Option<String>,
    position: Option<WindowPosition>,
}

impl NativeWindowStartup {
    fn from_options(options: &WindowOptions) -> Self {
        Self {
            command: options.command.clone(),
            workspace: options.workspace.clone(),
            window_class: options.window_class.clone(),
            position: options.position.clone(),
        }
    }
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
#[derive(Clone)]
struct NativeAppliedPaletteConfig {
    selection_word_boundary: String,
    term: String,
    enq_answerback: String,
    audible_bell: NativeAudibleBell,
    visual_bell: NativeVisualBell,
    colors: Option<Box<NativePalette>>,
    color_scheme: Option<String>,
    color_scheme_dirs: Vec<String>,
    color_schemes: HashMap<String, NativeResolvedPalette>,
    foreground_color: Color,
    background_color: Color,
    ansi_palette: Option<[Color; 16]>,
    indexed_palette: Option<[Option<Color>; 256]>,
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
    backend: NativeAppliedBackendConfig,
}

impl Default for NativeAppliedPaletteConfig {
    fn default() -> Self {
        Self {
            selection_word_boundary: DEFAULT_SELECTION_WORD_BOUNDARY.to_owned(),
            term: DEFAULT_TERM.to_owned(),
            enq_answerback: DEFAULT_ENQ_ANSWERBACK.to_owned(),
            audible_bell: DEFAULT_AUDIBLE_BELL,
            visual_bell: NativeVisualBell::default(),
            colors: None,
            color_scheme: None,
            color_scheme_dirs: Vec::new(),
            color_schemes: HashMap::new(),
            foreground_color: DEFAULT_FOREGROUND_COLOR,
            background_color: DEFAULT_BACKGROUND_COLOR,
            ansi_palette: None,
            indexed_palette: None,
            selection_fg_color: None,
            selection_bg_color: Some(DEFAULT_SELECTION_BG_COLOR),
            cursor_bg_color: DEFAULT_CURSOR_BG_COLOR,
            cursor_border_color: None,
            cursor_fg_color: Some(DEFAULT_CURSOR_FG_COLOR),
            compose_cursor_color: None,
            split_color: None,
            scrollbar_thumb_color: None,
            tab_bar_background_color: Some(DEFAULT_TAB_BAR_BACKGROUND_COLOR),
            tab_bar_inactive_tab_edge_color: None,
            tab_bar_active_tab_colors: DEFAULT_TAB_BAR_ACTIVE_TAB_COLORS,
            tab_bar_inactive_tab_colors: DEFAULT_TAB_BAR_INACTIVE_TAB_COLORS,
            tab_bar_inactive_tab_hover_colors: DEFAULT_TAB_BAR_INACTIVE_TAB_HOVER_COLORS,
            tab_bar_new_tab_colors: DEFAULT_TAB_BAR_NEW_TAB_COLORS,
            tab_bar_new_tab_hover_colors: DEFAULT_TAB_BAR_NEW_TAB_HOVER_COLORS,
            tab_bar_style: NativeTabBarStyle::default(),
            visual_bell_color: None,
            notification_handling: DEFAULT_NOTIFICATION_HANDLING,
            backend: NativeAppliedBackendConfig::default(),
        }
    }
}

impl Deref for NativeAppliedPaletteConfig {
    type Target = NativeAppliedBackendConfig;

    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}

impl DerefMut for NativeAppliedPaletteConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backend
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
struct NativeAppliedRenderConfig {
    initial_cols: u16,
    initial_rows: u16,
    foreground_text_hsb: NativeInactivePaneHsb,
    bold_brightens_ansi_colors: NativeBoldBrightensAnsiColors,
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
    window_frame_appearance: NativeWindowFrameAppearance,
    integrated_title_buttons: Vec<NativeIntegratedTitleButton>,
    integrated_title_button_alignment: NativeIntegratedTitleButtonAlignment,
    integrated_title_button_color: NativeIntegratedTitleButtonColor,
    integrated_title_button_style: NativeIntegratedTitleButtonStyle,
    inactive_pane_hsb: NativeInactivePaneHsb,
    tab_max_width: usize,
    tab_min_width: usize,
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
    palette: NativeAppliedPaletteConfig,
}

impl Default for NativeAppliedRenderConfig {
    fn default() -> Self {
        Self {
            initial_cols: DEFAULT_INITIAL_COLS,
            initial_rows: DEFAULT_INITIAL_ROWS,
            foreground_text_hsb: DEFAULT_FOREGROUND_TEXT_HSB,
            bold_brightens_ansi_colors: DEFAULT_BOLD_BRIGHTENS_ANSI_COLORS,
            text_background_opacity: DEFAULT_TEXT_BACKGROUND_OPACITY,
            window_background_opacity: DEFAULT_WINDOW_BACKGROUND_OPACITY,
            background: Vec::new(),
            window_background_image: None,
            window_background_image_hsb: None,
            window_background_gradient: None,
            window_background_images: Vec::new(),
            window_background_layers: Vec::new(),
            kde_window_background_blur: DEFAULT_KDE_WINDOW_BACKGROUND_BLUR,
            macos_window_background_blur: DEFAULT_MACOS_WINDOW_BACKGROUND_BLUR,
            win32_system_backdrop: DEFAULT_WIN32_SYSTEM_BACKDROP,
            win32_acrylic_accent_color: None,
            window_decorations: DEFAULT_WINDOW_DECORATIONS,
            window_frame_appearance: NativeWindowFrameAppearance::default(),
            integrated_title_buttons: default_integrated_title_buttons(),
            integrated_title_button_alignment: DEFAULT_INTEGRATED_TITLE_BUTTON_ALIGNMENT,
            integrated_title_button_color: DEFAULT_INTEGRATED_TITLE_BUTTON_COLOR,
            integrated_title_button_style: DEFAULT_INTEGRATED_TITLE_BUTTON_STYLE,
            inactive_pane_hsb: DEFAULT_INACTIVE_PANE_HSB,
            tab_max_width: MODERN_DEFAULT_TAB_MAX_WIDTH,
            tab_min_width: DEFAULT_TAB_MIN_WIDTH,
            command_palette_rows: None,
            command_palette_font: None,
            command_palette_font_size: DEFAULT_COMMAND_PALETTE_FONT_SIZE,
            command_palette_bg_color: Some(DEFAULT_COMMAND_PALETTE_BG_COLOR),
            command_palette_fg_color: Some(DEFAULT_COMMAND_PALETTE_FG_COLOR),
            char_select_font: None,
            char_select_font_size: DEFAULT_CHAR_SELECT_FONT_SIZE,
            char_select_bg_color: Some(DEFAULT_CHAR_SELECT_BG_COLOR),
            char_select_fg_color: Some(DEFAULT_CHAR_SELECT_FG_COLOR),
            pane_select_font: None,
            pane_select_font_size: DEFAULT_PANE_SELECT_FONT_SIZE,
            pane_select_bg_color: Some(DEFAULT_PANE_SELECT_BG_COLOR),
            pane_select_fg_color: Some(DEFAULT_PANE_SELECT_FG_COLOR),
            launcher_alphabet: DEFAULT_LAUNCHER_ALPHABET.to_owned(),
            quick_select_alphabet: DEFAULT_QUICK_SELECT_ALPHABET.to_owned(),
            quick_select_patterns: Vec::new(),
            disable_default_quick_select_patterns: false,
            quick_select_remove_styling: false,
            hyperlink_rules: default_hyperlink_rules(),
            copy_mode_active_highlight_bg: None,
            copy_mode_active_highlight_fg: None,
            copy_mode_inactive_highlight_bg: None,
            copy_mode_inactive_highlight_fg: None,
            quick_select_label_bg: None,
            quick_select_label_fg: None,
            quick_select_match_bg: None,
            quick_select_match_fg: None,
            input_selector_label_bg: None,
            input_selector_label_fg: None,
            launcher_label_bg: None,
            launcher_label_fg: None,
            palette: NativeAppliedPaletteConfig::default(),
        }
    }
}

impl Deref for NativeAppliedRenderConfig {
    type Target = NativeAppliedPaletteConfig;

    fn deref(&self) -> &Self::Target {
        &self.palette
    }
}

impl DerefMut for NativeAppliedRenderConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.palette
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
struct NativeAppliedInputConfig {
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
    leader: Option<NativeLeaderKey>,
    key_assignments: Vec<NativeUserKeyAssignment>,
    key_tables: BTreeMap<String, Vec<NativeUserKeyAssignment>>,
    mouse_assignments: Vec<NativeUserMouseAssignment>,
    scroll_to_bottom_on_input: bool,
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
    render: NativeAppliedRenderConfig,
}

impl Default for NativeAppliedInputConfig {
    fn default() -> Self {
        Self {
            key_map_preference: NativeKeyMapPreference::Mapped,
            ui_key_cap_rendering: DEFAULT_UI_KEY_CAP_RENDERING,
            swap_backspace_and_delete: false,
            enable_kitty_graphics: DEFAULT_ENABLE_KITTY_GRAPHICS,
            enable_checksum_rectangular_area: DEFAULT_ENABLE_CHECKSUM_RECTANGULAR_AREA,
            enable_title_reporting: DEFAULT_ENABLE_TITLE_REPORTING,
            enable_csi_u_key_encoding: DEFAULT_ENABLE_CSI_U_KEY_ENCODING,
            enable_kitty_keyboard: DEFAULT_ENABLE_KITTY_KEYBOARD,
            allow_download_protocols: DEFAULT_ALLOW_DOWNLOAD_PROTOCOLS,
            xcursor_theme: None,
            xcursor_size: None,
            palette_max_key_assigments_for_action:
                DEFAULT_PALETTE_MAX_KEY_ASSIGMENTS_FOR_ACTION,
            allow_win32_input_mode: DEFAULT_ALLOW_WIN32_INPUT_MODE,
            treat_left_ctrlalt_as_altgr: DEFAULT_TREAT_LEFT_CTRLALT_AS_ALTGR,
            send_composed_key_when_left_alt_is_pressed:
                DEFAULT_SEND_COMPOSED_KEY_WHEN_LEFT_ALT_IS_PRESSED,
            send_composed_key_when_right_alt_is_pressed:
                DEFAULT_SEND_COMPOSED_KEY_WHEN_RIGHT_ALT_IS_PRESSED,
            treat_east_asian_ambiguous_width_as_wide:
                DEFAULT_TREAT_EAST_ASIAN_AMBIGUOUS_WIDTH_AS_WIDE,
            normalize_output_to_unicode_nfc: DEFAULT_NORMALIZE_OUTPUT_TO_UNICODE_NFC,
            unicode_version: DEFAULT_UNICODE_VERSION,
            bidi_enabled: DEFAULT_BIDI_ENABLED,
            bidi_direction: DEFAULT_BIDI_DIRECTION,
            use_ime: DEFAULT_USE_IME,
            use_dead_keys: DEFAULT_USE_DEAD_KEYS,
            ime_preedit_rendering: DEFAULT_IME_PREEDIT_RENDERING,
            macos_forward_to_ime_modifier_mask: DEFAULT_MACOS_FORWARD_TO_IME_MODIFIER_MASK,
            xim_im_name: None,
            detect_password_input: DEFAULT_DETECT_PASSWORD_INPUT,
            leader: None,
            key_assignments: Vec::new(),
            key_tables: BTreeMap::new(),
            mouse_assignments: Vec::new(),
            scroll_to_bottom_on_input: DEFAULT_SCROLL_TO_BOTTOM_ON_INPUT,
            canonicalize_pasted_newlines: DEFAULT_CANONICALIZE_PASTED_NEWLINES,
            quote_dropped_files: DEFAULT_QUOTE_DROPPED_FILES,
            disable_default_key_bindings: DEFAULT_DISABLE_DEFAULT_KEY_BINDINGS,
            disable_default_mouse_bindings: DEFAULT_DISABLE_DEFAULT_MOUSE_BINDINGS,
            hide_mouse_cursor_when_typing: DEFAULT_HIDE_MOUSE_CURSOR_WHEN_TYPING,
            alternate_buffer_wheel_scroll_speed: DEFAULT_ALTERNATE_BUFFER_WHEEL_SCROLL_SPEED,
            pane_focus_follows_mouse: DEFAULT_PANE_FOCUS_FOLLOWS_MOUSE,
            swallow_mouse_click_on_pane_focus: DEFAULT_SWALLOW_MOUSE_CLICK_ON_PANE_FOCUS,
            swallow_mouse_click_on_window_focus: DEFAULT_SWALLOW_MOUSE_CLICK_ON_WINDOW_FOCUS,
            bypass_mouse_reporting_modifiers: DEFAULT_BYPASS_MOUSE_REPORTING_MODIFIERS,
            enable_scroll_bar: DEFAULT_ENABLE_SCROLL_BAR,
            scrollback_lines: DEFAULT_SCROLLBACK_LIMIT,
            min_scroll_bar_height: DEFAULT_MIN_SCROLL_BAR_HEIGHT,
            enable_tab_bar: DEFAULT_ENABLE_TAB_BAR,
            hide_tab_bar_if_only_one_tab: DEFAULT_HIDE_TAB_BAR_IF_ONLY_ONE_TAB,
            use_fancy_tab_bar: DEFAULT_USE_FANCY_TAB_BAR,
            unzoom_on_switch_pane: DEFAULT_UNZOOM_ON_SWITCH_PANE,
            tab_bar_at_bottom: DEFAULT_TAB_BAR_AT_BOTTOM,
            tab_and_split_indices_are_zero_based: DEFAULT_TAB_AND_SPLIT_INDICES_ARE_ZERO_BASED,
            mouse_wheel_scrolls_tabs: DEFAULT_MOUSE_WHEEL_SCROLLS_TABS,
            switch_to_last_active_tab_when_closing_tab:
                DEFAULT_SWITCH_TO_LAST_ACTIVE_TAB_WHEN_CLOSING_TAB,
            quit_when_all_windows_are_closed: DEFAULT_QUIT_WHEN_ALL_WINDOWS_ARE_CLOSED,
            window_close_confirmation: DEFAULT_WINDOW_CLOSE_CONFIRMATION,
            exit_behavior: DEFAULT_EXIT_BEHAVIOR,
            clean_exit_codes: DEFAULT_CLEAN_EXIT_CODES.to_vec(),
            exit_behavior_messaging: DEFAULT_EXIT_BEHAVIOR_MESSAGING,
            skip_close_confirmation_for_processes_named:
                default_skip_close_confirmation_for_processes_named(),
            show_close_tab_button_in_tabs: DEFAULT_SHOW_CLOSE_TAB_BUTTON_IN_TABS,
            show_new_tab_button_in_tab_bar: DEFAULT_SHOW_NEW_TAB_BUTTON_IN_TAB_BAR,
            show_tab_index_in_tab_bar: DEFAULT_SHOW_TAB_INDEX_IN_TAB_BAR,
            show_tabs_in_tab_bar: DEFAULT_SHOW_TABS_IN_TAB_BAR,
            render: NativeAppliedRenderConfig::default(),
        }
    }
}

impl Deref for NativeAppliedInputConfig {
    type Target = NativeAppliedRenderConfig;

    fn deref(&self) -> &Self::Target {
        &self.render
    }
}

impl DerefMut for NativeAppliedInputConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
struct NativeAppliedDomainConfig {
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
    derived_config_environment: BTreeMap<String, String>,
    set_environment_variables: BTreeMap<String, String>,
    launch_menu: Vec<NativeLaunchMenuItem>,
    tab_shortcut_style: NativeTabShortcutStyle,
    closed_tab_history_size: usize,
    close_tab_selection: CloseTabSelection,
    tab_bar_wheel_behavior: NativeTabBarWheelBehavior,
    input: NativeAppliedInputConfig,
}

impl Default for NativeAppliedDomainConfig {
    fn default() -> Self {
        Self {
            default_prog: None,
            default_gui_startup_args: default_gui_startup_args(),
            default_domain: DEFAULT_DOMAIN_NAME.to_owned(),
            default_workspace: DEFAULT_WORKSPACE_NAME.to_owned(),
            prefer_to_spawn_tabs: DEFAULT_PREFER_TO_SPAWN_TABS,
            automatically_reload_config: DEFAULT_AUTOMATICALLY_RELOAD_CONFIG,
            check_for_updates: DEFAULT_CHECK_FOR_UPDATES,
            check_for_updates_interval_seconds: DEFAULT_CHECK_FOR_UPDATES_INTERVAL_SECONDS,
            show_update_window: DEFAULT_SHOW_UPDATE_WINDOW,
            native_macos_fullscreen_mode: DEFAULT_NATIVE_MACOS_FULLSCREEN_MODE,
            macos_fullscreen_extend_behind_notch: DEFAULT_MACOS_FULLSCREEN_EXTEND_BEHIND_NOTCH,
            use_resize_increments: DEFAULT_USE_RESIZE_INCREMENTS,
            debug_key_events: DEFAULT_DEBUG_KEY_EVENTS,
            log_unknown_escape_sequences: DEFAULT_LOG_UNKNOWN_ESCAPE_SEQUENCES,
            warn_about_missing_glyphs: DEFAULT_WARN_ABOUT_MISSING_GLYPHS,
            default_cwd: None,
            default_ssh_auth_sock: None,
            default_mux_server_domain: None,
            daemon_options: NativeDaemonOptions::default(),
            exec_domains: Vec::new(),
            wsl_domains: Vec::new(),
            unix_domains: default_native_unix_domains(),
            ssh_domains: Vec::new(),
            tls_servers: Vec::new(),
            tls_clients: Vec::new(),
            serial_ports: Vec::new(),
            mux_enable_ssh_agent: DEFAULT_MUX_ENABLE_SSH_AGENT,
            ssh_backend: NativeSshBackend::LibSsh,
            ratelimit_mux_line_prefetches_per_second:
                DEFAULT_RATELIMIT_MUX_LINE_PREFETCHES_PER_SECOND,
            mux_output_parser_buffer_size: DEFAULT_MUX_OUTPUT_PARSER_BUFFER_SIZE,
            mux_output_parser_coalesce_delay_ms: DEFAULT_MUX_OUTPUT_PARSER_COALESCE_DELAY_MS,
            periodic_stat_logging: DEFAULT_PERIODIC_STAT_LOGGING,
            ulimit_nofile: DEFAULT_ULIMIT_NOFILE,
            ulimit_nproc: DEFAULT_ULIMIT_NPROC,
            mux_env_remove: default_mux_env_remove(),
            tiling_desktop_environments: default_tiling_desktop_environments(),
            derived_config_environment: BTreeMap::new(),
            set_environment_variables: BTreeMap::new(),
            launch_menu: Vec::new(),
            tab_shortcut_style: NativeTabShortcutStyle::Terminal,
            closed_tab_history_size: DEFAULT_CLOSED_TAB_HISTORY_SIZE,
            close_tab_selection: CloseTabSelection::Adjacent,
            tab_bar_wheel_behavior: NativeTabBarWheelBehavior::Scroll,
            input: NativeAppliedInputConfig::default(),
        }
    }
}

impl Deref for NativeAppliedDomainConfig {
    type Target = NativeAppliedInputConfig;

    fn deref(&self) -> &Self::Target {
        &self.input
    }
}

impl DerefMut for NativeAppliedDomainConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
struct NativeAppliedConfig {
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
    custom_block_glyphs: bool,
    anti_alias_custom_block_glyphs: bool,
    allow_square_glyphs_to_overflow_width: NativeSquareGlyphOverflow,
    freetype_load_target: NativeFreetypeTarget,
    freetype_render_target: NativeFreetypeTarget,
    freetype_load_flags: Option<NativeFreetypeLoadFlags>,
    freetype_interpreter_version: Option<u32>,
    freetype_pcf_long_family_names: bool,
    display_pixel_geometry: NativeDisplayPixelGeometry,
    adjust_window_size_when_changing_font_size: bool,
    domain: NativeAppliedDomainConfig,
}

impl Default for NativeAppliedConfig {
    fn default() -> Self {
        Self {
            font: None,
            font_fallbacks: Vec::new(),
            font_attributes: NativeFontAttributes::default(),
            font_rules: Vec::new(),
            font_size: MODERN_DEFAULT_FONT_SIZE,
            cell_width: DEFAULT_CELL_WIDTH,
            cell_widths: Vec::new(),
            line_height: DEFAULT_LINE_HEIGHT,
            font_antialias: DEFAULT_FONT_ANTIALIAS,
            font_hinting: DEFAULT_FONT_HINTING,
            font_rasterizer: DEFAULT_FONT_RASTERIZER,
            font_colr_rasterizer: DEFAULT_FONT_COLR_RASTERIZER,
            font_shaper: DEFAULT_FONT_SHAPER,
            harfbuzz_features: Vec::new(),
            font_dirs: Vec::new(),
            font_locator: DEFAULT_FONT_LOCATOR,
            use_cap_height_to_scale_fallback_fonts:
                DEFAULT_USE_CAP_HEIGHT_TO_SCALE_FALLBACK_FONTS,
            ignore_svg_fonts: DEFAULT_IGNORE_SVG_FONTS,
            sort_fallback_fonts_by_coverage: DEFAULT_SORT_FALLBACK_FONTS_BY_COVERAGE,
            search_font_dirs_for_fallback: DEFAULT_SEARCH_FONT_DIRS_FOR_FALLBACK,
            custom_block_glyphs: DEFAULT_CUSTOM_BLOCK_GLYPHS,
            anti_alias_custom_block_glyphs: DEFAULT_ANTI_ALIAS_CUSTOM_BLOCK_GLYPHS,
            allow_square_glyphs_to_overflow_width:
                DEFAULT_ALLOW_SQUARE_GLYPHS_TO_OVERFLOW_WIDTH,
            freetype_load_target: DEFAULT_FREETYPE_LOAD_TARGET,
            freetype_render_target: DEFAULT_FREETYPE_LOAD_TARGET,
            freetype_load_flags: None,
            freetype_interpreter_version: None,
            freetype_pcf_long_family_names: DEFAULT_FREETYPE_PCF_LONG_FAMILY_NAMES,
            display_pixel_geometry: DEFAULT_DISPLAY_PIXEL_GEOMETRY,
            adjust_window_size_when_changing_font_size:
                DEFAULT_ADJUST_WINDOW_SIZE_WHEN_CHANGING_FONT_SIZE,
            domain: NativeAppliedDomainConfig::default(),
        }
    }
}

impl Deref for NativeAppliedConfig {
    type Target = NativeAppliedDomainConfig;

    fn deref(&self) -> &Self::Target {
        &self.domain
    }
}

impl DerefMut for NativeAppliedConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.domain
    }
}

struct NativeDiagnosticGuiState {
    markers: DiagnosticMarkerHandle,
    scenario: DiagnosticScenario,
    hold_duration: Duration,
    hold_deadline: Option<Instant>,
    absolute_deadline: Instant,
    pending_secret: Option<String>,
    secret_prompt_presented: bool,
    font_mode: Option<rssh_diagnostics::DiagnosticFontMode>,
    font_specimen: Option<rssh_diagnostics::DiagnosticFontSpecimen>,
    scenario_ready_requires_gpu: bool,
}

enum NativeStartupMode {
    Normal,
    Benchmark,
    Diagnostic(NativeDiagnosticGuiState),
}

struct NativeStartupState {
    mode: NativeStartupMode,
    diagnostic_gpu_backend: Option<rssh_diagnostics::DiagnosticGpuBackend>,
}

#[allow(clippy::struct_excessive_bools)]
struct NativeWindowApp {
    app_window_id: rssh_core::WindowId,
    tab_transfer_targets: Vec<rssh_core::WindowId>,
    window_close_requested: bool,
    window_drag_requested: bool,
    activate_window_request: Option<WindowActivateWindowRequest>,
    window_hide_requested: bool,
    application_hide_requested: bool,
    application_quit_requested: bool,
    window_level: NativeWindowLevel,
    full_screen: bool,
    window_maximized: bool,
    font_size_scale: f64,
    debug_overlay_active: bool,
    debug_key_event_logs: Vec<String>,
    unknown_escape_sequence_warnings: Vec<String>,
    missing_glyph_warnings: Vec<String>,
    missing_glyph_warning_codepoints: HashSet<char>,
    char_select: Option<WindowCharSelect>,
    char_select_recently_used: Vec<WindowCharSelectRecent>,
    char_select_recently_used_sequence: u64,
    char_select_recently_used_path: Option<PathBuf>,
    window_focused: bool,
    mouse_click_may_focus_window: bool,
    window: Option<Arc<Window>>,
    bootstrap_surface: Option<WindowBootstrapSurface>,
    bootstrap_frame: Vec<u8>,
    renderer_mode: RendererMode,
    presentation_owner: PresentationOwner,
    deferred_gpu_generation: u64,
    startup: NativeStartupState,
    transport_start_requested: bool,
    // Prompts are keyed by pane so a slow host-key or secret decision in one
    // SSH pane cannot overwrite another pane's independent connection.
    #[cfg(feature = "ssh")]
    ssh_host_key_prompts: HashMap<
        rssh_core::PaneId,
        (HostKeyChallenge, mpsc::SyncSender<HostKeyDecision>),
    >,
    #[cfg(feature = "ssh")]
    ssh_secret_prompts: HashMap<rssh_core::PaneId, SshSecretPromptState>,
    ssh_connection_states: HashMap<rssh_core::PaneId, ConnectionState>,
    gpu_owners: crate::window_gpu::WindowGpuOwners,
    renderer: GpuFramePlanner,
    configured_dpi: Option<u32>,
    dpi_by_screen: BTreeMap<String, u32>,
    detected_window_dpi: u32,
    window_dpi: u32,
    runtime: ActiveWindowRuntime,
    snapshot: TerminalRenderSnapshot,
    window_title: String,
    modern_tab_bar_brand: bool,
    frame_width: u32,
    frame_height: u32,
    window_frame: NativeWindowFrame,
    frame_limit: Option<u64>,
    initial_window_class: Option<String>,
    initial_window_position: Option<WindowPosition>,
    #[allow(dead_code)]
    startup_command: PtyCommand,
    startup_uses_default_shell: bool,
    startup_workspace_was_explicit: bool,
    rendered_frames: u64,
    animation_started_at: Instant,
    event_proxy: Option<EventLoopProxy<WindowUserEvent>>,
    reload_request_sender: Option<Arc<dyn Fn(WindowUserEvent) -> bool + Send + Sync>>,
    session: Option<PtySession>,
    session_process_id: Option<u32>,
    session_tty_name: Option<String>,
    writer: Option<Box<dyn Write + Send>>,
    ssh_writer_senders: HashMap<rssh_core::PaneId, mpsc::Sender<NativeSshCommand>>,
    ssh_writer_cancellations:
        HashMap<rssh_core::PaneId, Arc<std::sync::atomic::AtomicBool>>,
    #[cfg(feature = "ssh")]
    ssh_connection_cancellations:
        HashMap<rssh_core::PaneId, rssh_ssh::RusshConnectionCancellation>,
    session_log: Option<Box<dyn Write + Send>>,
    reader_thread: Option<thread::JoinHandle<()>>,
    writer_thread: Option<thread::JoinHandle<()>>,
    interaction_state: NativeWindowInteractionState,
}

#[allow(clippy::struct_excessive_bools)]
struct NativeWindowInteractionState {
    active_runtime_generation: u64,
    active_runtime_transport: Option<PaneRuntimeTransportKind>,
    modifiers: ModifiersState,
    left_alt_pressed: bool,
    right_alt_pressed: bool,
    active_ui: PaneUiState,
    mouse_pixel_position: Option<PhysicalPosition<f64>>,
    rendered_tab_bar_layout: RefCell<Option<TabBarVisibleLayout>>,
    rendered_tab_bar_generation: Cell<u64>,
    mouse_position: Option<(u16, u16)>,
    current_mouse_wheel_delta: Option<MouseScrollDelta>,
    mouse_cursor_visible: bool,
    mouse_cursor_icon: CursorIcon,
    active_mouse_button: Option<MouseButton>,
    last_mouse_info: Option<ItermMouseInfo>,
    selection: Option<WindowSelection>,
    selecting: bool,
    scrollbar_dragging: bool,
    split_resize_dragging: Option<PaneSplitResizeDrag>,
    tab_bar_drag: Option<TabBarDrag>,
    tab_bar_scroll_position: usize,
    ui_left_release_pending: bool,
    pressed_pane_close_button: Option<rssh_core::PaneId>,
    pane_inspection: Option<rssh_core::PaneId>,
    ui_key_release_pending: Option<UiKeyReleasePending>,
    last_mouse_assignment_click: Option<WindowMouseAssignmentClick>,
    last_left_click: Option<WindowClick>,
    command_palette: Option<WindowCommandPalette>,
    command_palette_frecency: HashMap<String, WindowCommandPaletteFrecency>,
    command_palette_frecency_sequence: u64,
    command_palette_frecency_path: Option<PathBuf>,
    pane_select: Option<WindowPaneSelect>,
    pending_window_positions: HashMap<rssh_core::WindowId, WindowPosition>,
    tab_navigator: Option<WindowTabNavigator>,
    prompt_input_line: Option<WindowPromptInputLine>,
    input_selector: Option<WindowInputSelector>,
    confirmation: Option<WindowConfirmation>,
    deferred_wheel_context: Option<WheelTarget>,
    close_confirmation: Option<WindowCloseConfirmation>,
    key_table_stack: Vec<WindowActiveKeyTable>,
    visual_bell_started_at: HashMap<rssh_core::PaneId, Instant>,
    #[cfg(test)]
    visual_bell_test_now: Option<Instant>,
    ime_preedit: Option<String>,
    last_ime_cursor_area: Cell<Option<(u32, u32, u32, u32)>>,
    dead_key_active: bool,
    dead_key_text: Option<String>,
    leader_active_since: Option<Instant>,
    base_config_overrides: Arc<NativeConfigSnapshot>,
    base_config_generation: u64,
    base_config_source: Option<PathBuf>,
    window_config_overrides: Option<NativeWindowConfigPatch>,
    #[cfg(test)]
    base_config_apply_observer: Option<Box<dyn FnMut(u64) + Send>>,
    #[cfg(test)]
    pty_spawn_observer: Option<Arc<std::sync::atomic::AtomicUsize>>,
    #[allow(dead_code)]
    config_overrides: Arc<NativeConfigSnapshot>,
    host_state: NativeWindowHostState,
}

#[allow(clippy::struct_excessive_bools)]
struct NativeWindowHostState {
    latest_notification: Option<TerminalNotification>,
    left_status: String,
    right_status: String,
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
    lua_command_palette_entries: Vec<NativeCommandPaletteEntry>,
    lua_emit_event_handlers: BTreeMap<String, Vec<NativeLuaEmitEventHandler>>,
    last_redraw_request_at: Option<Instant>,
    last_animation_redraw_request_at: Option<Instant>,
    last_status_update_at: Option<Instant>,
    #[cfg(test)]
    legacy_test_geometry: bool,
    cursor_blink_visible: bool,
    cursor_blink_opacity_alpha: u8,
    last_cursor_blink_at: Option<Instant>,
    text_blink_opacity_alpha: u8,
    rapid_text_blink_opacity_alpha: u8,
    last_text_blink_at: Option<Instant>,
    last_rapid_text_blink_at: Option<Instant>,
    osc52_policy: Osc52Policy,
    clipboard_writer: Box<dyn FnMut(&str) -> bool + Send>,
    clipboard_reader: Box<dyn FnMut() -> Option<String> + Send>,
    primary_selection_writer: Box<dyn FnMut(&str) -> bool + Send>,
    primary_selection_reader: Box<dyn FnMut() -> Option<String> + Send>,
    hyperlink_opener: Box<dyn FnMut(&str) -> bool + Send>,
    open_uri_handler: Box<dyn FnMut(&NativeWindowOpenUri) -> bool + Send>,
    new_tab_button_click_handler: Box<dyn FnMut(&NativeWindowNewTabButtonClick) -> bool + Send>,
    tab_title_formatter: Box<TabTitleFormatter>,
    window_title_formatter: Box<WindowTitleFormatter>,
    #[cfg(test)]
    applied_window_titles: RefCell<Option<Vec<String>>>,
    applied_window_title: RefCell<Option<String>>,
    update_status_handler: Box<WindowStatusUpdateHandler>,
    update_right_status_handler: Box<WindowRightStatusUpdateHandler>,
    notification_handler: Box<dyn FnMut(&TerminalNotification) -> bool + Send>,
    audible_bell_handler: Box<dyn FnMut(&NativeWindowBell) -> bool + Send>,
    bell_handler: Box<dyn FnMut(&NativeWindowBell) -> bool + Send>,
    focus_change_handler: Box<dyn FnMut(&NativeWindowFocusChange) -> bool + Send>,
    resize_handler: Box<dyn FnMut(&NativeWindowResize) -> bool + Send>,
    user_var_change_handler: Box<dyn FnMut(&NativeWindowUserVarChange) -> bool + Send>,
    config_reloaded_handler: Box<dyn FnMut(&NativeWindowConfigReloaded) -> bool + Send>,
    command_palette_augmenter: Box<CommandPaletteAugmenter>,
    prompt_input_line_handler: Box<dyn FnMut(&NativePromptInputLine) -> bool + Send>,
    input_selector_handler: Box<dyn FnMut(&NativeInputSelector) -> bool + Send>,
    confirmation_handler: Box<dyn FnMut(&NativeConfirmation) -> bool + Send>,
    emit_event_handler: Box<dyn FnMut(&NativeWindowEmitEvent) -> bool + Send>,
    metrics: WindowMetrics,
    pending_frame_damage: Vec<DamageRegion>,
    frame_needs_full_repaint: bool,
    app_shell: AppShell,
    closed_tab_history: Arc<Mutex<ClosedTabHistory>>,
    pane_runtimes: HashMap<rssh_core::PaneId, PaneRuntime>,
    pane_bell_counts: HashMap<rssh_core::PaneId, u64>,
    applied_config: Arc<NativeAppliedConfig>,
}

impl Deref for NativeWindowApp {
    type Target = NativeWindowInteractionState;

    fn deref(&self) -> &Self::Target {
        &self.interaction_state
    }
}

impl DerefMut for NativeWindowApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.interaction_state
    }
}

impl Deref for NativeWindowInteractionState {
    type Target = NativeWindowHostState;

    fn deref(&self) -> &Self::Target {
        &self.host_state
    }
}

impl DerefMut for NativeWindowInteractionState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.host_state
    }
}

impl Deref for NativeWindowHostState {
    type Target = NativeAppliedConfig;

    fn deref(&self) -> &Self::Target {
        &self.applied_config
    }
}

impl DerefMut for NativeWindowHostState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.applied_config)
    }
}

#[cfg(target_os = "macos")]
fn hide_native_application(event_loop: &ActiveEventLoop) {
    use winit::platform::macos::ActiveEventLoopExtMacOS;

    event_loop.hide_application();
}

#[cfg(not(target_os = "macos"))]
fn hide_native_application(_event_loop: &ActiveEventLoop) {}

struct NativeWindowManager {
    startup_app: Option<Box<NativeWindowApp>>,
    #[allow(dead_code)]
    config_lifecycle: Option<Box<NativeConfigLifecycle>>,
    deferred_config_pending: bool,
    deferred_config_receiver: Option<
        mpsc::Receiver<
            Result<
                crate::config_lifecycle::NativeConfigLoadAttempt<NativeConfigSnapshot>,
                String,
            >,
        >,
    >,
    deferred_config_thread: Option<thread::JoinHandle<()>>,
    windows: HashMap<winit::window::WindowId, Box<NativeWindowApp>>,
    pending_apps: Vec<Box<NativeWindowApp>>,
    retired_apps: Vec<Box<NativeWindowApp>>,
    pane_event_routes: HashMap<(rssh_core::WindowId, rssh_core::PaneId), PaneEventRoute>,
    focus: WindowFocusCoordinator<winit::window::WindowId>,
    last_metrics: Option<WindowMetricsSnapshot>,
    closed_gpu_abandonments: u64,
    quit_when_all_windows_are_closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PaneEventRoute {
    window_id: rssh_core::WindowId,
    pane_id: rssh_core::PaneId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedWindowAppLocation {
    Startup,
    Window(winit::window::WindowId),
    Pending(usize),
}

const fn should_spawn_transport_during_materialization(
    renderer_mode: RendererMode,
    benchmark_startup: bool,
    suppress_transport_start: bool,
) -> bool {
    matches!(renderer_mode, RendererMode::Gpu)
        && !benchmark_startup
        && !suppress_transport_start
}

impl NativeWindowManager {
    fn new(startup_app: impl Into<Box<NativeWindowApp>>) -> Self {
        let startup_app = startup_app.into();
        let quit_when_all_windows_are_closed = startup_app.quit_when_all_windows_are_closed;
        Self {
            startup_app: Some(startup_app),
            config_lifecycle: None,
            deferred_config_pending: false,
            deferred_config_receiver: None,
            deferred_config_thread: None,
            windows: HashMap::new(),
            pending_apps: Vec::new(),
            retired_apps: Vec::new(),
            pane_event_routes: HashMap::new(),
            focus: WindowFocusCoordinator::default(),
            last_metrics: None,
            closed_gpu_abandonments: 0,
            quit_when_all_windows_are_closed,
        }
    }

    fn with_config_lifecycle(mut self, lifecycle: Box<NativeConfigLifecycle>) -> Self {
        #[cfg(feature = "functional-test-observer")]
        crate::functional_observer::record_config_lifecycle(
            lifecycle.effective().generation,
            lifecycle.latest_diagnostic().is_some(),
        );
        self.config_lifecycle = Some(lifecycle);
        self
    }

    fn shutdown_runtime_owners(&mut self) {
        for app in self
            .startup_app
            .iter_mut()
            .chain(self.windows.values_mut())
            .chain(self.pending_apps.iter_mut())
            .chain(self.retired_apps.iter_mut())
        {
            app.stop_active_runtime();
            if let Some(mut worker) = app.runtime.take_worker() {
                worker.shutdown();
            }
            for runtime in app.pane_runtimes.values_mut() {
                let cleanup = runtime.close();
                report_pane_pty_cleanup("event-loop runtime shutdown", &cleanup);
            }
        }
    }

    fn with_deferred_config(mut self) -> Self {
        self.deferred_config_pending = true;
        self
    }

    fn metrics_app(&self) -> Option<&NativeWindowApp> {
        self.windows
            .values()
            .next()
            .or(self.startup_app.as_ref())
            .or_else(|| self.pending_apps.first())
            .map(Box::as_ref)
    }

    fn metrics_report(&self) -> String {
        self.aggregated_metrics_snapshot().report()
    }

    fn metrics_json_report(&self) -> Result<String, serde_json::Error> {
        self.aggregated_metrics_snapshot().json_report()
    }

    fn aggregated_metrics_snapshot(&self) -> WindowMetricsSnapshot {
        let live_gpu_abandonments = self
            .startup_app
            .iter()
            .chain(self.windows.values())
            .chain(self.pending_apps.iter())
            .map(|app| app.metrics_snapshot().gpu_abandoned_lost_surfaces)
            .fold(0_u64, u64::saturating_add);
        let total_gpu_abandonments = self
            .closed_gpu_abandonments
            .saturating_add(live_gpu_abandonments);
        if let Some(app) = self.metrics_app() {
            let mut snapshot = app.metrics_snapshot();
            snapshot.gpu_abandoned_lost_surfaces = total_gpu_abandonments;
            snapshot
        } else {
            let mut snapshot = self
                .last_metrics
                .clone()
                .unwrap_or_else(|| WindowMetrics::new().snapshot());
            snapshot.gpu_abandoned_lost_surfaces = total_gpu_abandonments;
            snapshot
        }
    }

    fn retain_closed_window_metrics(&mut self, snapshot: WindowMetricsSnapshot) {
        self.closed_gpu_abandonments = self
            .closed_gpu_abandonments
            .saturating_add(snapshot.gpu_abandoned_lost_surfaces);
        self.last_metrics = Some(snapshot);
    }

    fn materialize_startup_app(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn Error>> {
        let Some(app) = self.startup_app.take() else {
            return Ok(());
        };
        let window_id = self.materialize_app(event_loop, app)?;
        self.request_window_focus(window_id);
        Ok(())
    }

    fn materialize_pending_apps(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn Error>> {
        let pending = std::mem::take(&mut self.pending_apps);
        let pending_len = pending.len();
        let mut materialized_window_ids = Vec::with_capacity(pending_len);
        for app in pending {
            let window_id = self.materialize_app(event_loop, app)?;
            materialized_window_ids.push(window_id);
        }
        let focus_window_id =
            materialized_window_ids
                .into_iter()
                .enumerate()
                .find_map(|(index, window_id)| {
                    should_focus_materialized_window(index, pending_len).then_some(window_id)
                });
        if let Some(window_id) = focus_window_id {
            self.request_window_focus(window_id);
        }
        Ok(())
    }

    fn materialize_app(
        &mut self,
        event_loop: &ActiveEventLoop,
        mut app: Box<NativeWindowApp>,
    ) -> Result<winit::window::WindowId, Box<dyn Error>> {
        self.refresh_app_to_current_base(&mut app);
        app.create_window(event_loop)?;
        if should_spawn_transport_during_materialization(
            app.renderer_mode,
            app.is_benchmark_startup(),
            app.suppresses_transport_start(),
        ) {
            app.spawn_pty()?;
        }
        let Some(window_id) = app.window_id() else {
            return Err(Box::new(io::Error::other("window was not created")));
        };
        if let Some(window) = &app.window {
            window.request_redraw();
        }
        self.windows.insert(window_id, app);
        Ok(window_id)
    }

    fn refresh_app_to_current_base(&self, app: &mut NativeWindowApp) {
        if let Some(config) = self
            .config_lifecycle
            .as_ref()
            .map(|lifecycle| lifecycle.effective().clone())
        {
            app.set_base_config(&config, ReloadDisposition::SilentStartup);
        }
    }

    fn collect_pending_window_apps_from_app(&mut self, app: &mut NativeWindowApp) {
        let source_window_id = app.app_window_id;
        while let Some(mut detached_app) = app.take_next_pending_window_app() {
            self.refresh_app_to_current_base(&mut detached_app);
            let destination_window_id = detached_app.app_window_id;
            for pane_id in detached_app.app_shell.pane_ids() {
                for ((_, routed_pane_id), target) in &mut self.pane_event_routes {
                    if *routed_pane_id == pane_id && target.window_id == source_window_id {
                        target.window_id = destination_window_id;
                    }
                }
                self.pane_event_routes
                    .insert(
                        (source_window_id, pane_id),
                        PaneEventRoute {
                            window_id: destination_window_id,
                            pane_id,
                        },
                    );
            }
            self.pending_apps.push(detached_app);
        }
    }

    fn refresh_tab_transfer_targets(&mut self) {
        let window_ids = self
            .all_app_locations()
            .into_iter()
            .filter_map(|location| self.app_at_location(location).map(|app| app.app_window_id))
            .collect::<Vec<_>>();
        for location in self.all_app_locations() {
            if let Some(app) = self.app_at_location_mut(location) {
                app.tab_transfer_targets = window_ids
                    .iter()
                    .copied()
                    .filter(|window_id| *window_id != app.app_window_id)
                    .collect();
            }
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "tab runtime transfer keeps rollback and event-route updates in one transaction"
    )]
    fn transfer_tab_between_windows(
        &mut self,
        source_window_id: rssh_core::WindowId,
        target_window_id: rssh_core::WindowId,
        tab: rssh_core::TabId,
        target_index: usize,
    ) -> Result<(), AppShellError> {
        if source_window_id == target_window_id {
            return Err(AppShellError::UnsupportedAction);
        }
        let source_location = self
            .owned_app_location_for_window(source_window_id)
            .ok_or(AppShellError::UnsupportedAction)?;
        let mut source = self
            .take_app_at_location(source_location)
            .ok_or(AppShellError::UnsupportedAction)?;
        let target_location = self
            .owned_app_location_for_window(target_window_id)
            .ok_or(AppShellError::UnsupportedAction)?;
        let Some(mut target) = self.take_app_at_location(target_location) else {
            self.restore_app_at_location(source_location, source);
            return Err(AppShellError::UnsupportedAction);
        };

        let source_transfers_final_tab = source
            .app_shell
            .active_workspace()
            .tabs()
            .iter()
            .any(|candidate| candidate.id() == tab)
            && source.app_shell.active_workspace().tabs().len() == 1;

        let moved_pane_ids = source
            .app_shell
            .active_workspace()
            .tabs()
            .iter()
            .find(|candidate| candidate.id() == tab)
            .map(|candidate| {
                candidate
                    .panes()
                    .iter()
                    .map(rssh_core::app_shell::Pane::id)
                    .collect::<Vec<_>>()
            })
            .ok_or(AppShellError::InvalidTab(tab));
        let moved_pane_ids = match moved_pane_ids {
            Ok(panes) => panes,
            Err(error) => {
                self.restore_app_at_location(target_location, target);
                self.restore_app_at_location(source_location, source);
                return Err(error);
            }
        };
        let moved_local_worker_panes = source.runtime.worker().map_or_else(Vec::new, |worker| {
            moved_pane_ids
                .iter()
                .copied()
                .filter(|pane| worker.contains_pane(*pane))
                .collect::<Vec<_>>()
        });

        let source_active_pane = source.app_shell.active_pane_id();
        let mut moved_runtimes = HashMap::new();
        let mut moved_bells = HashMap::new();
        // Disabled SSH has no auxiliary payload; preserve the shared transfer flow.
        #[cfg_attr(not(feature = "ssh"), allow(clippy::zero_sized_map_values))]
        let mut moved_ssh_auxiliary = HashMap::new();
        for pane_id in &moved_pane_ids {
            let runtime = if *pane_id == source_active_pane {
                Some(source.take_active_runtime())
            } else {
                source.pane_runtimes.remove(pane_id)
            };
            if let Some(runtime) = runtime {
                moved_runtimes.insert(*pane_id, runtime);
            }
            if let Some(bell_count) = source.pane_bell_counts.remove(pane_id) {
                moved_bells.insert(*pane_id, bell_count);
            }
            moved_ssh_auxiliary.insert(
                *pane_id,
                source.take_ssh_pane_auxiliary_state(*pane_id),
            );
        }

        let detached = match if source_transfers_final_tab {
            source.app_shell.clone_tab_for_transfer(tab)
        } else {
            source.app_shell.detach_tab(tab)
        } {
            Ok(detached) => detached,
            Err(error) => {
                if moved_pane_ids.contains(&source_active_pane)
                    && let Some(runtime) = moved_runtimes.remove(&source_active_pane)
                {
                    source.install_active_runtime(runtime);
                }
                for (pane_id, runtime) in moved_runtimes {
                    source.pane_runtimes.insert(pane_id, runtime);
                }
                source.pane_bell_counts.extend(moved_bells);
                for (pane_id, auxiliary) in moved_ssh_auxiliary {
                    source.install_ssh_pane_auxiliary_state(pane_id, auxiliary);
                }
                self.restore_app_at_location(target_location, target);
                self.restore_app_at_location(source_location, source);
                return Err(error);
            }
        };

        if moved_pane_ids.contains(&source_active_pane) && !source_transfers_final_tab {
            let replacement = source.new_inactive_pane_runtime();
            source.sync_pane_runtimes(source_active_pane, replacement);
        }

        let target_active_pane = target.app_shell.active_pane_id();
        let target_previous_runtime = target.take_active_runtime();
        let imported = match target.app_shell.import_detached_tab(detached, target_index) {
            Ok(imported) => imported,
            Err(error) => {
                target.install_active_runtime(target_previous_runtime);
                self.restore_app_at_location(target_location, target);
                self.restore_app_at_location(source_location, source);
                return Err(error);
            }
        };
        for pane_id in &moved_local_worker_panes {
            if let Some(worker) = source.runtime.worker_mut()
                && let Err(error) = worker.retire_pane_transport(*pane_id)
            {
                eprintln!("failed to retire transferred source pane worker: {error}");
            }
        }
        let mut transferred_local_target_panes = Vec::new();
        for (source_pane, target_pane) in imported.pane_id_map() {
            if let Some(runtime) = moved_runtimes.remove(source_pane) {
                target.pane_runtimes.insert(*target_pane, runtime);
            }
            if let Some(bell_count) = moved_bells.remove(source_pane) {
                target.pane_bell_counts.insert(*target_pane, bell_count);
            }
            if let Some(auxiliary) = moved_ssh_auxiliary.remove(source_pane) {
                target.install_ssh_pane_auxiliary_state(*target_pane, auxiliary);
            }
            if moved_local_worker_panes.contains(source_pane) {
                transferred_local_target_panes.push(*target_pane);
            }
            for route in self.pane_event_routes.values_mut() {
                if route.window_id == source_window_id && route.pane_id == *source_pane {
                    route.window_id = target_window_id;
                    route.pane_id = *target_pane;
                }
            }
            self.pane_event_routes.insert(
                (source_window_id, *source_pane),
                PaneEventRoute {
                    window_id: target_window_id,
                    pane_id: *target_pane,
                },
            );
        }
        target.sync_pane_runtimes(target_active_pane, target_previous_runtime);
        if target.event_proxy.is_some() {
            for pane_id in transferred_local_target_panes {
                if let Err(error) = target.restart_transferred_local_pane(pane_id) {
                    eprintln!("failed to restart transferred target pane worker: {error}");
                }
            }
        }
        source.apply_window_title();
        target.apply_window_title();
        self.restore_app_at_location(target_location, target);
        self.restore_app_at_location(source_location, source);
        if source_transfers_final_tab {
            self.finalize_app_close_at_location(source_location)
                .expect("moved final tab source remains manager-owned until retirement");
        }
        Ok(())
    }

    fn reload_configuration_attempt(&mut self) -> bool {
        if self.config_lifecycle.is_none() {
            return false;
        }
        for app in self
            .startup_app
            .iter_mut()
            .chain(self.pending_apps.iter_mut())
            .chain(self.windows.values_mut())
        {
            app.metrics.mark_config_started();
        }
        let attempt = self
            .config_lifecycle
            .as_ref()
            .expect("configuration lifecycle remains installed")
            .attempt_reload();
        self.install_configuration_attempt(attempt)
    }

    fn install_configuration_attempt(
        &mut self,
        attempt: crate::config_lifecycle::NativeConfigLoadAttempt<NativeConfigSnapshot>,
    ) -> bool {
        let Some(lifecycle) = self.config_lifecycle.as_mut() else {
            return false;
        };
        let succeeded = lifecycle.install_runtime_attempt(attempt);
        let effective = lifecycle.effective().clone();
        let diagnostic = lifecycle
            .latest_diagnostic()
            .map(std::string::ToString::to_string);
        #[cfg(feature = "functional-test-observer")]
        crate::functional_observer::record_config_lifecycle(
            lifecycle.effective().generation,
            lifecycle.latest_diagnostic().is_some(),
        );

        if let Some(app) = self.startup_app.as_mut() {
            app.set_base_config(&effective, ReloadDisposition::SilentStartup);
        }
        for app in &mut self.pending_apps {
            app.set_base_config(&effective, ReloadDisposition::SilentStartup);
        }
        for app in self.windows.values_mut() {
            app.set_base_config(&effective, ReloadDisposition::SilentStartup);
        }

        if let Some(app) = self.startup_app.as_mut() {
            app.reload_configuration();
        }
        for app in &mut self.pending_apps {
            app.reload_configuration();
        }
        for app in self.windows.values_mut() {
            app.reload_configuration();
            app.metrics.mark_config_finished();
        }

        if let Some(app) = self.startup_app.as_mut() {
            app.metrics.mark_config_finished();
        }
        for app in &mut self.pending_apps {
            app.metrics.mark_config_finished();
        }

        if let Some(diagnostic) = diagnostic {
            eprintln!("configuration reload failed: {diagnostic}");
        }
        succeeded
    }

    fn start_deferred_config_if_ready(&mut self) {
        if !self.deferred_config_pending || self.deferred_config_receiver.is_some() {
            return;
        }
        let Some(app) = self
            .windows
            .values()
            .find(|app| app.rendered_frames > 0)
        else {
            return;
        };
        if app.is_benchmark_startup() {
            self.deferred_config_pending = false;
            return;
        }

        self.deferred_config_pending = false;
        let Some(lifecycle) = self.config_lifecycle.as_ref() else {
            self.finish_deferred_startup_after_config();
            return;
        };
        if crate::stage7_attribution::audit_product_service_start(
            crate::stage7_attribution::ProductServiceEntry::DeferredConfig,
        )
        .is_err()
        {
            return;
        }
        let task = lifecycle.reload_task();
        let event_proxy = self
            .windows
            .values()
            .find_map(|app| app.event_proxy.clone())
            .or_else(|| {
                self.startup_app
                    .as_ref()
                    .and_then(|app| app.event_proxy.clone())
            });
        for app in self
            .startup_app
            .iter_mut()
            .chain(self.pending_apps.iter_mut())
            .chain(self.windows.values_mut())
        {
            app.metrics.mark_config_started();
        }
        let (sender, receiver) = mpsc::channel();
        match thread::Builder::new()
            .name("rssh-deferred-config".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(task))
                    .map_err(|_| "deferred configuration worker panicked".to_owned());
                let _ = sender.send(result);
                if let Some(event_proxy) = event_proxy {
                    let _ = event_proxy.send_event(WindowUserEvent::DeferredConfigReady);
                }
            })
        {
            Ok(worker) => {
                self.deferred_config_receiver = Some(receiver);
                self.deferred_config_thread = Some(worker);
            }
            Err(error) => {
                eprintln!("failed to start deferred configuration worker: {error}");
                for app in self
                    .startup_app
                    .iter_mut()
                    .chain(self.pending_apps.iter_mut())
                    .chain(self.windows.values_mut())
                {
                    app.metrics.mark_config_finished();
                }
                self.finish_deferred_startup_after_config();
            }
        }
    }

    fn finish_deferred_config_if_ready(&mut self) -> bool {
        let received = {
            let Some(receiver) = self.deferred_config_receiver.as_ref() else {
                return false;
            };
            match receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return false,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Err("deferred configuration worker disconnected".to_owned())
                }
            }
        };
        self.deferred_config_receiver = None;
        if let Some(worker) = self.deferred_config_thread.take()
            && worker.join().is_err()
        {
            eprintln!("deferred configuration worker panicked after reporting completion");
        }
        match received {
            Ok(attempt) => {
                self.install_configuration_attempt(attempt);
            }
            Err(error) => {
                eprintln!("deferred configuration load failed: {error}");
                for app in self
                    .startup_app
                    .iter_mut()
                    .chain(self.pending_apps.iter_mut())
                    .chain(self.windows.values_mut())
                {
                    app.metrics.mark_config_finished();
                }
            }
        }
        for app in self
            .startup_app
            .iter_mut()
            .chain(self.pending_apps.iter_mut())
            .chain(self.windows.values_mut())
        {
            app.mark_diagnostic_config_ready();
        }
        self.finish_deferred_startup_after_config();
        true
    }

    fn finish_deferred_startup_after_config(&mut self) {

        // The bootstrap frame is intentionally independent from the full
        // configuration.  Local PTYs must not observe the default projection
        // and then get resized/reconfigured underneath them; start them only
        // after the first configuration attempt has completed.  SSH GUI
        // panes are the exception: their native transport is allowed to run
        // in parallel with configuration and is already marked as started by
        // the first CPU frame.
        for app in self
            .windows
            .values_mut()
            .filter(|app| {
                app.rendered_frames > 0
                    && !app.is_benchmark_startup()
                    && !app.suppresses_transport_start()
            })
        {
            if app.transport_start_requested {
                continue;
            }
            if let Err(error) = app.spawn_pty() {
                eprintln!("deferred transport start error: {error}");
            }
        }

        // Diagnostics own a hermetic, one-shot configuration lifecycle.  A
        // filesystem watcher would make the measured process depend on ambient
        // user state and would outlive the bounded scenario contract.
        if self
            .windows
            .values()
            .any(|app| app.has_diagnostic_gui())
            || self
                .startup_app
                .as_ref()
                .is_some_and(|app| app.has_diagnostic_gui())
        {
            return;
        }

        let event_proxy = self
            .windows
            .values()
            .find_map(|app| app.event_proxy.clone())
            .or_else(|| {
                self.startup_app
                    .as_ref()
                    .and_then(|app| app.event_proxy.clone())
            });
        let Some(event_proxy) = event_proxy else {
            return;
        };
        if crate::stage7_attribution::audit_product_service_start(
            crate::stage7_attribution::ProductServiceEntry::ConfigWatcher,
        )
        .is_err()
        {
            return;
        }
        if let Some(lifecycle) = self.config_lifecycle.as_mut()
            && let Err(diagnostic) = lifecycle.install_watcher_sink(Arc::new(move || {
                event_proxy
                    .send_event(WindowUserEvent::ConfigFileChanged)
                    .is_ok()
            }))
        {
            eprintln!(
                "failed to initialize deferred WezTerm config watcher: {}",
                diagnostic.detail
            );
        }
    }

    fn handle_manager_user_event(&mut self, event: &WindowUserEvent) -> bool {
        if matches!(event, WindowUserEvent::DeferredConfigReady) {
            self.finish_deferred_config_if_ready();
            return true;
        }
        if matches!(
            event,
            WindowUserEvent::ReloadConfigurationRequested | WindowUserEvent::ConfigFileChanged
        ) {
            self.reload_configuration_attempt();
            return true;
        }
        if let WindowUserEvent::MoveTabToWindow {
            source_window_id,
            target_window_id,
            tab,
            target_index,
        } = event
        {
            if let Err(error) = self.transfer_tab_between_windows(
                *source_window_id,
                *target_window_id,
                *tab,
                *target_index,
            ) {
                eprintln!("cross-window tab transfer failed: {error:?}");
            }
            return true;
        }
        false
    }

    fn app_owns_pane(app: &NativeWindowApp, pane_id: rssh_core::PaneId) -> bool {
        app.app_shell.pane_ids().contains(&pane_id)
    }

    fn user_event_owner_location_and_pane(
        &self,
        event: &WindowUserEvent,
    ) -> Option<(ManagedWindowAppLocation, rssh_core::PaneId)> {
        let (declared_window_id, pane_id) = event.pane_identity()?;
        if let Some(location) =
            self.owned_app_location_for_window_and_pane(declared_window_id, pane_id)
        {
            return Some((location, pane_id));
        }

        let mut routed_identity = (declared_window_id, pane_id);
        let mut visited = HashSet::new();
        let mut saw_route = false;
        while visited.insert(routed_identity) {
            let Some(next) = self
                .pane_event_routes
                .get(&routed_identity)
                .copied()
            else {
                break;
            };
            saw_route = true;
            routed_identity = (next.window_id, next.pane_id);
            if let Some(location) =
                self.owned_app_location_for_window_and_pane(routed_identity.0, routed_identity.1)
            {
                return Some((location, routed_identity.1));
            }
        }
        if saw_route {
            return None;
        }

        None
    }

    fn owned_app_location_for_window_and_pane(
        &self,
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
    ) -> Option<ManagedWindowAppLocation> {
        self.all_app_locations().into_iter().find(|location| {
            self.app_at_location(*location).is_some_and(|app| {
                app.app_window_id == window_id && Self::app_owns_pane(app, pane_id)
            })
        })
    }

    fn owned_app_location_for_window(
        &self,
        window_id: rssh_core::WindowId,
    ) -> Option<ManagedWindowAppLocation> {
        self.all_app_locations().into_iter().find(|location| {
            self.app_at_location(*location)
                .is_some_and(|app| app.app_window_id == window_id)
        })
    }

    fn all_app_locations(&self) -> Vec<ManagedWindowAppLocation> {
        let mut locations = Vec::with_capacity(
            usize::from(self.startup_app.is_some()) + self.windows.len() + self.pending_apps.len(),
        );
        if self.startup_app.is_some() {
            locations.push(ManagedWindowAppLocation::Startup);
        }
        locations.extend(
            self.windows
                .keys()
                .copied()
                .map(ManagedWindowAppLocation::Window),
        );
        locations.extend((0..self.pending_apps.len()).map(ManagedWindowAppLocation::Pending));
        locations
    }

    fn app_at_location(&self, location: ManagedWindowAppLocation) -> Option<&NativeWindowApp> {
        match location {
            ManagedWindowAppLocation::Startup => self.startup_app.as_deref(),
            ManagedWindowAppLocation::Window(window_id) => {
                self.windows.get(&window_id).map(Box::as_ref)
            }
            ManagedWindowAppLocation::Pending(index) => {
                self.pending_apps.get(index).map(Box::as_ref)
            }
        }
    }

    fn app_at_location_mut(
        &mut self,
        location: ManagedWindowAppLocation,
    ) -> Option<&mut NativeWindowApp> {
        match location {
            ManagedWindowAppLocation::Startup => self.startup_app.as_deref_mut(),
            ManagedWindowAppLocation::Window(window_id) => {
                self.windows.get_mut(&window_id).map(Box::as_mut)
            }
            ManagedWindowAppLocation::Pending(index) => {
                self.pending_apps.get_mut(index).map(Box::as_mut)
            }
        }
    }

    fn take_app_at_location(
        &mut self,
        location: ManagedWindowAppLocation,
    ) -> Option<Box<NativeWindowApp>> {
        match location {
            ManagedWindowAppLocation::Startup => self.startup_app.take(),
            ManagedWindowAppLocation::Window(window_id) => self.windows.remove(&window_id),
            ManagedWindowAppLocation::Pending(index) => {
                (index < self.pending_apps.len()).then(|| self.pending_apps.remove(index))
            }
        }
    }

    fn restore_app_at_location(
        &mut self,
        location: ManagedWindowAppLocation,
        app: Box<NativeWindowApp>,
    ) {
        match location {
            ManagedWindowAppLocation::Startup => self.startup_app = Some(app),
            ManagedWindowAppLocation::Window(window_id) => {
                self.windows.insert(window_id, app);
            }
            ManagedWindowAppLocation::Pending(index) => {
                self.pending_apps
                    .insert(index.min(self.pending_apps.len()), app);
            }
        }
    }

    fn finalize_app_close_at_location(&mut self, location: ManagedWindowAppLocation) -> Option<()> {
        let (app_window_id, quit_when_all_windows_are_closed, snapshot) = {
            let app = self.app_at_location_mut(location)?;
            app.shutdown_gpu_for_window_close();
            app.stop_active_runtime();
            if let Some(mut worker) = app.runtime.take_worker() {
                worker.shutdown();
            }
            for runtime in app.pane_runtimes.values_mut() {
                let cleanup = runtime.close();
                report_pane_pty_cleanup("window close pane PTY cleanup", &cleanup);
            }
            #[cfg(feature = "functional-test-observer")]
            {
                crate::functional_observer::publish(app.functional_observer_snapshot());
                let _ = crate::functional_observer::wait_until_current_revision_delivered(
                    Duration::from_secs(2),
                );
            }
            (
                app.app_window_id,
                app.quit_when_all_windows_are_closed,
                app.metrics_snapshot(),
            )
        };
        if let ManagedWindowAppLocation::Window(window_id) = location {
            self.focus.remove(window_id);
        }
        self.quit_when_all_windows_are_closed = quit_when_all_windows_are_closed;
        let app = self
            .take_app_at_location(location)
            .expect("closed app remains manager-owned until final removal");
        self.retain_closed_window_metrics(snapshot);
        self.retired_apps.push(app);
        self.remove_pane_event_routes_for_window(app_window_id);
        Some(())
    }

    fn reap_retired_apps(&mut self) {
        self.retired_apps.clear();
    }

    #[cfg(feature = "ssh")]
    fn redraw_after_ssh_event(app: &NativeWindowApp) {
        if let Some(window) = &app.window { window.request_redraw(); }
    }

    fn dispatch_user_event_to_app(
        app: &mut NativeWindowApp,
        event: WindowUserEvent,
        pane_identity: Option<(rssh_core::PaneId, u64)>,
    ) -> bool {
        match event {
            WindowUserEvent::ReloadConfigurationRequested
            | WindowUserEvent::ConfigFileChanged
            | WindowUserEvent::DeferredConfigReady
            | WindowUserEvent::DiagnosticShutdownRequested
            | WindowUserEvent::MoveTabToWindow { .. } => false,
            WindowUserEvent::RuntimeWakeWindow { .. } => match app.poll_active_v2_runtime() {
                Ok(Some(close_window)) => close_window,
                Ok(None) => false,
                Err(error) => {
                    eprintln!("runtime V2 host error: {error}");
                    true
                }
            },
            WindowUserEvent::DeferredGpuInitialized {
                generation,
                outcome,
                ..
            } => {
                app.handle_deferred_gpu_initialized(generation, outcome);
                false
            }
            WindowUserEvent::Output { bytes, .. } => {
                let (pane_id, _) = pane_identity.expect("pane output carries a pane identity");
                if let Err(error) = app.handle_pane_pty_output(pane_id, &bytes) {
                    eprintln!("PTY write error: {error}");
                    true
                } else {
                    if pane_id == app.app_shell.active_pane_id()
                        && let Some(window) = &app.window
                    {
                        window.request_redraw();
                    }
                    false
                }
            }
            WindowUserEvent::Exited { .. } => {
                let (pane_id, runtime_generation) =
                    pane_identity.expect("pane exit carries a pane identity");
                let status = app.finish_pane_runtime_after_exit(pane_id, runtime_generation);
                #[cfg(feature = "functional-test-observer")]
                {
                    crate::functional_observer::publish(app.functional_observer_snapshot());
                    let _ = crate::functional_observer::wait_until_current_revision_delivered(
                        Duration::from_millis(250),
                    );
                }
                let close_window = app.apply_pane_exit_behavior_after_exit(pane_id, status);
                app.defer_automatic_close_for_frame_limit(close_window)
            }
            WindowUserEvent::ReadError { error, .. } => {
                let (pane_id, _) = pane_identity.expect("pane error carries a pane identity");
                app.handle_pane_runtime_read_error(pane_id, &error)
            }
            WindowUserEvent::WriteCompleted {
                byte_count,
                elapsed,
                ..
            } => {
                app.handle_pane_input_write_completed(byte_count, elapsed);
                false
            }
            WindowUserEvent::WriteError { error, .. } => {
                let (pane_id, _) = pane_identity.expect("pane error carries a pane identity");
                app.handle_pane_runtime_write_error(pane_id, &error)
            }
            #[cfg(feature = "ssh")]
            WindowUserEvent::SshState { state, .. } => {
                let (pane_id, _) = pane_identity.expect("SSH state carries a pane identity");
                app.handle_ssh_state(pane_id, state);
                Self::redraw_after_ssh_event(app);
                false
            }
            #[cfg(feature = "ssh")]
            WindowUserEvent::HostKeyPrompt {
                challenge,
                decision,
                ..
            } => {
                let (pane_id, _) = pane_identity.expect("host-key prompt carries a pane identity");
                app.handle_host_key_prompt(pane_id, challenge, decision);
                Self::redraw_after_ssh_event(app);
                false
            }
            #[cfg(feature = "ssh")]
            WindowUserEvent::SecretPrompt {
                prompt,
                response,
                ..
            } => {
                let (pane_id, _) = pane_identity.expect("secret prompt carries a pane identity");
                app.handle_secret_prompt(pane_id, prompt, response);
                Self::redraw_after_ssh_event(app);
                false
            }
        }
    }

    fn dispatch_user_event_to_owner(&mut self, event: WindowUserEvent) -> Option<bool> {
        let (location, pane_identity) = match &event {
            WindowUserEvent::RuntimeWakeWindow { window_id }
            | WindowUserEvent::DeferredGpuInitialized { window_id, .. } => {
                (self.owned_app_location_for_window(*window_id)?, None)
            }
            _ => {
                let (location, pane_id) = self.user_event_owner_location_and_pane(&event)?;
                let runtime_generation = event.runtime_generation()?;
                (location, Some((pane_id, runtime_generation)))
            }
        };
        let event_is_exit = matches!(&event, WindowUserEvent::Exited { .. });
        let mut app = self.take_app_at_location(location)?;
        if let Some((pane_id, runtime_generation)) = pane_identity
            && !app.pane_runtime_generation_matches(pane_id, runtime_generation)
        {
            self.restore_app_at_location(location, app);
            return Some(false);
        }
        let owner_window_id = app.app_window_id;
        let close_window = Self::dispatch_user_event_to_app(&mut app, event, pane_identity);

        self.collect_pending_window_apps_from_app(&mut app);
        if close_window {
            self.restore_app_at_location(location, app);
            self.finalize_app_close_at_location(location)
                .expect("event owner remains manager-owned until final removal");
        } else {
            let owner_still_has_pane = pane_identity
                .is_none_or(|(pane_id, _)| Self::app_owns_pane(&app, pane_id));
            self.restore_app_at_location(location, app);
            if event_is_exit && !owner_still_has_pane {
                let (pane_id, _) = pane_identity.expect("pane exit carries a pane identity");
                self.remove_pane_event_routes_for_owner(owner_window_id, pane_id);
            }
        }
        Some(close_window)
    }

    fn remove_pane_event_routes_for_owner(
        &mut self,
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
    ) {
        self.pane_event_routes.retain(|(source, pane), target| {
            !((*source == window_id && *pane == pane_id)
                || (target.window_id == window_id && target.pane_id == pane_id))
        });
    }

    fn remove_pane_event_routes_for_window(&mut self, window_id: rssh_core::WindowId) {
        self.pane_event_routes
            .retain(|_, target| target.window_id != window_id);
    }

    fn activate_window_relative_from(
        &self,
        current_window_id: winit::window::WindowId,
        request: WindowActivateWindowRequest,
    ) -> bool {
        let mut window_order = self
            .windows
            .iter()
            .map(|(window_id, app)| (*window_id, app.app_window_id))
            .collect::<Vec<_>>();
        window_order.sort_unstable_by_key(|(_, app_window_id)| app_window_id.get());

        let Some(current_index) = window_order
            .iter()
            .position(|(window_id, _)| *window_id == current_window_id)
        else {
            return false;
        };
        let target_index = match request {
            WindowActivateWindowRequest::Index(index) => {
                activate_window_absolute_index(index, window_order.len())
            }
            WindowActivateWindowRequest::Relative { offset, wrap } => {
                activate_window_relative_index(current_index, window_order.len(), offset, wrap)
            }
        };
        let Some(target_index) = target_index else {
            return false;
        };
        let Some((target_window_id, _)) = window_order.get(target_index) else {
            return false;
        };
        self.request_window_focus(*target_window_id)
    }

    fn request_window_focus(&self, window_id: winit::window::WindowId) -> bool {
        let Some(window) = self
            .windows
            .get(&window_id)
            .and_then(|app| app.window.as_ref())
        else {
            return false;
        };
        window.set_visible(true);
        window.set_minimized(false);
        window.focus_window();
        true
    }

    fn handle_window_platform_event(
        &mut self,
        window_id: winit::window::WindowId,
        event: &rssh_native::PlatformEvent,
    ) -> io::Result<()> {
        let rssh_native::PlatformEvent::Focused(focused) = *event else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "manager platform adapter expected a focus event",
            ));
        };
        dispatch_window_focus_changed(&mut self.focus, &mut self.windows, window_id, focused)
    }

    fn close_window(&mut self, window_id: winit::window::WindowId) -> bool {
        let closes_last_window = self.quit_when_all_windows_are_closed
            && self.windows.len() == 1
            && self.startup_app.is_none()
            && self.pending_apps.is_empty();
        if let Some(app) = self.windows.get_mut(&window_id) {
            app.handle_window_close_requested();
            if !app.take_window_close_request() {
                return false;
            }
            if closes_last_window {
                app.shutdown_gpu_after_native_window_close();
            }
            self.finalize_app_close_at_location(ManagedWindowAppLocation::Window(window_id))
                .expect("approved window remains manager-owned until final removal");
        }
        self.focus.remove(window_id);
        self.should_exit_when_idle()
    }

    fn quit_application_from_app(&mut self, mut app: Box<NativeWindowApp>) {
        app.shutdown_gpu_for_window_close();
        for window in self.windows.values_mut() {
            window.shutdown_gpu_for_window_close();
        }
        if let Some(startup) = self.startup_app.as_mut() {
            startup.shutdown_gpu_for_window_close();
        }
        for pending in &mut self.pending_apps {
            pending.shutdown_gpu_for_window_close();
        }
        let app_snapshot = app.metrics_snapshot();
        let additional_abandonments = self
            .windows
            .values()
            .chain(self.startup_app.iter())
            .chain(self.pending_apps.iter())
            .map(|window| window.metrics_snapshot().gpu_abandoned_lost_surfaces)
            .fold(0_u64, u64::saturating_add);
        self.closed_gpu_abandonments = self
            .closed_gpu_abandonments
            .saturating_add(app_snapshot.gpu_abandoned_lost_surfaces)
            .saturating_add(additional_abandonments);
        self.last_metrics = Some(app_snapshot);
        self.retired_apps.push(app);
        self.retired_apps
            .extend(self.windows.drain().map(|(_, app)| app));
        self.focus = WindowFocusCoordinator::default();
        if let Some(startup) = self.startup_app.take() {
            self.retired_apps.push(startup);
        }
        self.retired_apps.append(&mut self.pending_apps);
        self.pane_event_routes.clear();
    }

    fn should_exit_when_idle(&self) -> bool {
        self.quit_when_all_windows_are_closed
            && self.windows.is_empty()
            && self.startup_app.is_none()
            && self.pending_apps.is_empty()
    }

    fn shutdown_gpu_for_application_exit(&mut self) {
        if let Some(startup) = self.startup_app.as_mut() {
            startup.shutdown_gpu_after_native_window_close();
        }
        for app in self.windows.values_mut() {
            app.shutdown_gpu_after_native_window_close();
        }
        for app in &mut self.pending_apps {
            app.shutdown_gpu_after_native_window_close();
        }
        for app in &mut self.retired_apps {
            app.shutdown_gpu_after_native_window_close();
        }
    }

    #[cfg(test)]
    fn new_for_test(startup_app: impl Into<Box<NativeWindowApp>>) -> Self {
        Self::new(startup_app)
    }

    #[cfg(test)]
    fn collect_pending_window_apps_from_primary_for_test(&mut self) {
        let Some(mut app) = self.startup_app.take() else {
            return;
        };
        self.collect_pending_window_apps_from_app(&mut app);
        self.startup_app = Some(app);
    }

    #[cfg(test)]
    fn primary_app_mut_for_test(&mut self) -> &mut NativeWindowApp {
        self.startup_app
            .as_mut()
            .expect("test manager should still own startup app")
    }

    #[cfg(test)]
    fn pending_app_count_for_test(&self) -> usize {
        self.pending_apps.len()
    }

    #[cfg(test)]
    fn retired_app_count_for_test(&self) -> usize {
        self.retired_apps.len()
    }

    #[cfg(test)]
    fn startup_app_count_for_test(&self) -> usize {
        usize::from(self.startup_app.is_some())
    }

    #[cfg(test)]
    fn last_metrics_for_test(&self) -> Option<WindowMetricsSnapshot> {
        self.last_metrics.clone()
    }

    #[cfg(test)]
    const fn closed_gpu_abandonments_for_test(&self) -> u64 {
        self.closed_gpu_abandonments
    }

    #[cfg(test)]
    fn discard_startup_app_for_test(&mut self) {
        if let Some(app) = self.startup_app.take() {
            self.quit_when_all_windows_are_closed = app.quit_when_all_windows_are_closed;
            self.last_metrics = Some(app.metrics_snapshot());
            drop(app);
        }
    }

    #[cfg(test)]
    fn should_exit_when_idle_for_test(&self) -> bool {
        self.should_exit_when_idle()
    }

    #[cfg(test)]
    fn quit_application_from_primary_for_test(&mut self) {
        let Some(app) = self.startup_app.take() else {
            return;
        };
        self.quit_application_from_app(app);
    }

    #[cfg(test)]
    fn pending_app_for_test(&self, index: usize) -> Option<&NativeWindowApp> {
        self.pending_apps.get(index).map(Box::as_ref)
    }

    #[cfg(test)]
    fn all_apps_for_test(&self) -> Vec<&NativeWindowApp> {
        self.startup_app
            .iter()
            .chain(self.pending_apps.iter())
            .chain(self.windows.values())
            .map(Box::as_ref)
            .collect()
    }

    #[cfg(test)]
    fn all_apps_mut_for_test(&mut self) -> Vec<&mut NativeWindowApp> {
        self.startup_app
            .iter_mut()
            .chain(self.pending_apps.iter_mut())
            .chain(self.windows.values_mut())
            .map(Box::as_mut)
            .collect()
    }

    #[cfg(test)]
    fn config_generation_for_test(&self) -> Option<u64> {
        self.config_lifecycle
            .as_ref()
            .map(|lifecycle| lifecycle.effective().generation)
    }

    #[cfg(test)]
    fn install_lifecycle_attempt_without_fanout_for_test(&mut self) -> bool {
        let lifecycle = self
            .config_lifecycle
            .as_mut()
            .expect("test manager should own a configuration lifecycle");
        let attempt = lifecycle.attempt_reload();
        lifecycle.install_runtime_attempt(attempt)
    }

    #[cfg(test)]
    fn refresh_pending_app_before_spawn_for_test(&mut self, index: usize) {
        let mut app = self.pending_apps.remove(index);
        self.refresh_app_to_current_base(&mut app);
        self.pending_apps.insert(index, app);
    }

    #[cfg(test)]
    fn install_config_watcher_for_test(
        &mut self,
        debounce: Duration,
        sink: crate::config_lifecycle::ConfigFileChangedSink,
    ) -> Result<(), crate::config_lifecycle::NativeConfigWatchDiagnostic> {
        self.config_lifecycle
            .as_mut()
            .expect("test manager should own a configuration lifecycle")
            .install_watcher_sink_for_test(debounce, sink, None)
    }

    #[cfg(test)]
    fn watched_config_paths_for_test(&self) -> std::collections::BTreeSet<PathBuf> {
        self.config_lifecycle
            .as_ref()
            .expect("test manager should own a configuration lifecycle")
            .watched_paths_for_test()
    }

    #[cfg(test)]
    fn config_watcher_exists_for_test(&self) -> bool {
        self.config_lifecycle
            .as_ref()
            .expect("test manager should own a configuration lifecycle")
            .watcher_exists_for_test()
    }

    #[cfg(test)]
    fn enqueue_config_watch_burst_for_test(&mut self, count: usize) {
        self.config_lifecycle
            .as_mut()
            .expect("test manager should own a configuration lifecycle")
            .enqueue_watcher_relevant_burst_for_test(count);
    }
}

#[allow(dead_code)]
enum NativeSshCommand {
    #[cfg(feature = "ssh")]
    Attach(Box<dyn SshShellWriter>),
    Data(Vec<u8>),
    Resize(TerminalSize),
    Cancel,
}

#[cfg(feature = "ssh")]
struct SshSecretPromptState {
    prompt: SecretPrompt,
    response: mpsc::SyncSender<Option<String>>,
    input: String,
}

#[derive(Default)]
#[cfg(feature = "ssh")]
struct SshPaneAuxiliaryState {
    writer_sender: Option<mpsc::Sender<NativeSshCommand>>,
    writer_cancellation: Option<Arc<std::sync::atomic::AtomicBool>>,
    connection_cancellation: Option<rssh_ssh::RusshConnectionCancellation>,
    connection_state: Option<ConnectionState>,
    host_key_prompt: Option<(HostKeyChallenge, mpsc::SyncSender<HostKeyDecision>)>,
    secret_prompt: Option<SshSecretPromptState>,
}

struct NativeSshWriter {
    sender: mpsc::Sender<NativeSshCommand>,
    connected: Arc<std::sync::atomic::AtomicBool>,
}

impl Write for NativeSshWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.connected.load(Ordering::Acquire) {
            return Ok(bytes.len());
        }
        let count = bytes.len();
        self.sender
            .send(NativeSshCommand::Data(bytes.to_vec()))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "SSH writer is closed"))?;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) enum DeferredGpuInitialization {
    Ready(Box<WindowGpu>),
    Failed(String),
}

impl fmt::Debug for DeferredGpuInitialization {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready(_) => formatter.write_str("Ready(..)"),
            Self::Failed(error) => formatter.debug_tuple("Failed").field(error).finish(),
        }
    }
}

fn spawn_deferred_gpu_task(
    name: String,
    task: impl FnOnce() + Send + 'static,
) -> io::Result<()> {
    thread::Builder::new().name(name).spawn(task).map(drop)
}

#[derive(Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum WindowUserEvent {
    ReloadConfigurationRequested,
    ConfigFileChanged,
    DeferredConfigReady,
    DiagnosticShutdownRequested,
    MoveTabToWindow {
        source_window_id: rssh_core::WindowId,
        target_window_id: rssh_core::WindowId,
        tab: rssh_core::TabId,
        target_index: usize,
    },
    RuntimeWakeWindow {
        window_id: rssh_core::WindowId,
    },
    DeferredGpuInitialized {
        window_id: rssh_core::WindowId,
        generation: u64,
        outcome: DeferredGpuInitialization,
    },
    Output {
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
        runtime_generation: u64,
        bytes: Vec<u8>,
    },
    Exited {
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
        runtime_generation: u64,
    },
    ReadError {
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
        runtime_generation: u64,
        error: String,
    },
    WriteCompleted {
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
        runtime_generation: u64,
        byte_count: usize,
        elapsed: Duration,
    },
    WriteError {
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
        runtime_generation: u64,
        error: String,
    },
    #[cfg(feature = "ssh")]
    SshState {
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
        runtime_generation: u64,
        state: ConnectionState,
    },
    #[cfg(feature = "ssh")]
    HostKeyPrompt {
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
        runtime_generation: u64,
        challenge: HostKeyChallenge,
        decision: mpsc::SyncSender<HostKeyDecision>,
    },
    #[cfg(feature = "ssh")]
    SecretPrompt {
        window_id: rssh_core::WindowId,
        pane_id: rssh_core::PaneId,
        runtime_generation: u64,
        prompt: SecretPrompt,
        response: mpsc::SyncSender<Option<String>>,
    },
}

impl WindowUserEvent {
    const fn pane_identity(&self) -> Option<(rssh_core::WindowId, rssh_core::PaneId)> {
        match self {
            Self::ReloadConfigurationRequested
            | Self::ConfigFileChanged
            | Self::DeferredConfigReady
            | Self::DiagnosticShutdownRequested
            | Self::MoveTabToWindow { .. }
            | Self::RuntimeWakeWindow { .. }
            | Self::DeferredGpuInitialized { .. } => None,
            Self::Output {
                window_id, pane_id, ..
            }
            | Self::Exited {
                window_id, pane_id, ..
            }
            | Self::ReadError {
                window_id, pane_id, ..
            }
            | Self::WriteCompleted {
                window_id, pane_id, ..
            }
            | Self::WriteError {
                window_id, pane_id, ..
            }
            => Some((*window_id, *pane_id)),
            #[cfg(feature = "ssh")]
            Self::SshState {
                window_id, pane_id, ..
            }
            | Self::HostKeyPrompt {
                window_id, pane_id, ..
            }
            | Self::SecretPrompt {
                window_id, pane_id, ..
            } => Some((*window_id, *pane_id)),
        }
    }

    const fn runtime_generation(&self) -> Option<u64> {
        match self {
            Self::ReloadConfigurationRequested
            | Self::ConfigFileChanged
            | Self::DeferredConfigReady
            | Self::DiagnosticShutdownRequested
            | Self::MoveTabToWindow { .. }
            | Self::RuntimeWakeWindow { .. }
            | Self::DeferredGpuInitialized { .. } => None,
            Self::Output {
                runtime_generation, ..
            }
            | Self::Exited {
                runtime_generation, ..
            }
            | Self::ReadError {
                runtime_generation, ..
            }
            | Self::WriteCompleted {
                runtime_generation, ..
            }
            | Self::WriteError {
                runtime_generation, ..
            }
            => Some(*runtime_generation),
            #[cfg(feature = "ssh")]
            Self::SshState {
                runtime_generation, ..
            }
            | Self::HostKeyPrompt {
                runtime_generation, ..
            }
            | Self::SecretPrompt {
                runtime_generation, ..
            } => Some(*runtime_generation),
        }
    }
}

#[cfg(any(test, target_os = "windows"))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct NativeWindowChromePolicy {
    undecorated_shadow: bool,
    rounded_corners: bool,
}

#[cfg(any(test, target_os = "windows"))]
fn native_window_chrome_policy_for_platform(
    platform: &str,
    decorations: NativeWindowDecorations,
) -> NativeWindowChromePolicy {
    let integrated_windows_chrome = platform == "windows" && decorations.integrated_buttons;
    NativeWindowChromePolicy {
        undecorated_shadow: integrated_windows_chrome,
        rounded_corners: integrated_windows_chrome,
    }
}

#[cfg(target_os = "windows")]
fn native_window_chrome_policy(
    decorations: NativeWindowDecorations,
) -> NativeWindowChromePolicy {
    native_window_chrome_policy_for_platform(std::env::consts::OS, decorations)
}

#[cfg(any(test, target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeMacosWindowChromePolicy {
    unified_titlebar: bool,
    has_shadow: bool,
}

#[cfg(any(test, target_os = "macos"))]
fn native_macos_window_chrome_policy_for_platform(
    platform: &str,
    decorations: NativeWindowDecorations,
) -> NativeMacosWindowChromePolicy {
    let unified_titlebar = platform == "macos"
        && decorations.title
        && decorations.integrated_buttons;
    let has_shadow = if decorations.macos_force_disable_shadow {
        false
    } else {
        decorations.macos_force_enable_shadow || decorations.resize || decorations.title
    };
    NativeMacosWindowChromePolicy {
        unified_titlebar,
        has_shadow,
    }
}

#[cfg(target_os = "macos")]
fn native_macos_window_chrome_policy(
    decorations: NativeWindowDecorations,
) -> NativeMacosWindowChromePolicy {
    native_macos_window_chrome_policy_for_platform(std::env::consts::OS, decorations)
}

#[cfg(test)]
struct QueuedPaneWriter {
    sender: mpsc::SyncSender<Vec<u8>>,
}

#[cfg(test)]
struct PaneInputWorkerContext {
    event_proxy: EventLoopProxy<WindowUserEvent>,
    window_id: rssh_core::WindowId,
    pane_id: rssh_core::PaneId,
    runtime_generation: u64,
}

#[cfg(test)]
impl Write for QueuedPaneWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.sender
            .send(bytes.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "pane PTY writer stopped"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
fn start_pane_input_queue(
    writer: &mut Option<Box<dyn Write + Send>>,
    writer_thread: &mut Option<thread::JoinHandle<()>>,
    context: Option<PaneInputWorkerContext>,
) -> io::Result<()> {
    if writer.is_none() || writer_thread.is_some() {
        return Ok(());
    }

    let mut blocking_writer = writer
        .take()
        .expect("pane writer presence was checked before queue startup");
    let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(64);
    *writer_thread = Some(
        thread::Builder::new()
            .name("rssh-pane-pty-writer".to_owned())
            .spawn(move || {
                while let Ok(bytes) = receiver.recv() {
                    let started = Instant::now();
                    if let Err(error) = blocking_writer
                        .write_all(&bytes)
                        .and_then(|()| blocking_writer.flush())
                    {
                        if let Some(context) = &context {
                            let _ = context.event_proxy.send_event(WindowUserEvent::WriteError {
                                window_id: context.window_id,
                                pane_id: context.pane_id,
                                runtime_generation: context.runtime_generation,
                                error: error.to_string(),
                            });
                        } else {
                            eprintln!("PTY write error: {error}");
                        }
                        break;
                    }
                    if let Some(context) = &context
                        && context
                            .event_proxy
                            .send_event(WindowUserEvent::WriteCompleted {
                                window_id: context.window_id,
                                pane_id: context.pane_id,
                                runtime_generation: context.runtime_generation,
                                byte_count: bytes.len(),
                                elapsed: started.elapsed(),
                            })
                            .is_err()
                    {
                        break;
                    }
                }
            })?,
    );
    *writer = Some(Box::new(QueuedPaneWriter { sender }));
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneRuntimeTransportKind {
    LocalPty,
    NativeSsh,
}

struct PaneRuntime {
    runtime: TerminalRuntime,
    transport: Option<PaneRuntimeTransportKind>,
    session: Option<PtySession>,
    session_process_id: Option<u32>,
    session_tty_name: Option<String>,
    writer: Option<Box<dyn Write + Send>>,
    reader_thread: Option<thread::JoinHandle<()>>,
    writer_thread: Option<thread::JoinHandle<()>>,
    runtime_generation: u64,
    snapshot: TerminalRenderSnapshot,
    ui: PaneUiState,
}

enum ActiveV2Close {
    Open,
    Closed {
        pane: rterm_runtime::PaneToken,
        exit: Option<rterm_runtime::SessionExit>,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PaneStableViewport {
    main_top: Option<StableRowIndex>,
}

impl PaneStableViewport {
    fn normalized_main_top(
        terminal: &Terminal,
        requested: Option<StableRowIndex>,
    ) -> Option<StableRowIndex> {
        let dimensions = terminal.stable_dimensions();
        if dimensions.domain != TerminalScreenDomain::Main || dimensions.viewport_rows == 0 {
            return None;
        }
        let requested = requested?;
        if requested >= dimensions.physical_top {
            return None;
        }
        Some(requested.max(dimensions.scrollback_top))
    }

    fn active_top(self, terminal: &Terminal) -> Option<StableRowIndex> {
        if terminal.stable_dimensions().domain != TerminalScreenDomain::Main {
            return None;
        }
        Self::normalized_main_top(terminal, self.main_top)
    }

    fn clamp_main(&mut self, terminal: &Terminal) {
        if terminal.stable_dimensions().domain == TerminalScreenDomain::Main {
            self.main_top = Self::normalized_main_top(terminal, self.main_top);
        }
    }

    fn ensure_main_row_visible(&mut self, terminal: &Terminal, row: StableRowIndex) {
        let dimensions = terminal.stable_dimensions();
        if dimensions.domain != TerminalScreenDomain::Main
            || dimensions.viewport_rows == 0
            || !terminal.retained_stable_range().contains(&row)
        {
            return;
        }

        self.clamp_main(terminal);
        let top = self.active_top(terminal).unwrap_or(dimensions.physical_top);
        let bottom = top.saturating_add(
            StableRowIndex::try_from(dimensions.viewport_rows).unwrap_or(StableRowIndex::MAX),
        );
        let requested = if row < top {
            Some(row.max(dimensions.scrollback_top))
        } else if row >= bottom {
            let last_viewport_offset =
                StableRowIndex::try_from(dimensions.viewport_rows.saturating_sub(1))
                    .unwrap_or(StableRowIndex::MAX);
            Some(
                row.saturating_sub(last_viewport_offset)
                    .max(dimensions.scrollback_top),
            )
        } else {
            return;
        };
        self.main_top = Self::normalized_main_top(terminal, requested);
    }

    fn scrollback_offset(self, terminal: &Terminal) -> usize {
        let dimensions = terminal.stable_dimensions();
        if dimensions.domain != TerminalScreenDomain::Main {
            return 0;
        }
        self.active_top(terminal)
            .and_then(|top| dimensions.physical_top.checked_sub(top))
            .and_then(|offset| usize::try_from(offset).ok())
            .unwrap_or(0)
            .min(
                dimensions
                    .scrollback_rows
                    .saturating_sub(dimensions.viewport_rows),
            )
    }

    fn set_scrollback_offset(&mut self, terminal: &Terminal, offset: usize) {
        let dimensions = terminal.stable_dimensions();
        if dimensions.domain != TerminalScreenDomain::Main || dimensions.viewport_rows == 0 {
            return;
        }
        let history_len = dimensions
            .scrollback_rows
            .saturating_sub(dimensions.viewport_rows);
        let offset = offset.min(history_len);
        self.main_top = if offset == 0 {
            None
        } else {
            dimensions
                .physical_top
                .checked_sub(StableRowIndex::try_from(offset).unwrap_or(StableRowIndex::MAX))
        };
        self.clamp_main(terminal);
    }
}

impl PaneRuntime {
    fn reconcile_terminal_mutation(&mut self) {
        self.ui.reconcile_terminal_mutation(self.runtime.terminal());
        self.snapshot = terminal_runtime_snapshot(&self.runtime, self.ui.stable_viewport);
    }

    fn reconcile_terminal_resize(&mut self, preserve_ordinary_selection: bool) {
        self.ui
            .reconcile_terminal_resize(self.runtime.terminal(), preserve_ordinary_selection);
        self.snapshot = terminal_runtime_snapshot(&self.runtime, self.ui.stable_viewport);
    }

    fn rebuild_snapshot_after_main_screen_reflow(&mut self) {
        self.snapshot = terminal_runtime_snapshot(&self.runtime, self.ui.stable_viewport);
    }

    fn reconcile_after_main_screen_reflow(&mut self) {
        self.ui
            .reconcile_after_main_screen_reflow(self.runtime.terminal());
        self.rebuild_snapshot_after_main_screen_reflow();
    }

    fn close(&mut self) -> PanePtyCleanupOutcome {
        stop_pty_lifecycle(
            &mut self.session,
            &mut self.session_process_id,
            &mut self.session_tty_name,
            &mut self.writer,
            &mut self.reader_thread,
            &mut self.writer_thread,
        )
    }

    fn finish_after_exit(&mut self) -> PanePtyCleanupOutcome {
        finish_pty_lifecycle_after_exit(
            &mut self.session,
            &mut self.session_process_id,
            &mut self.session_tty_name,
            &mut self.writer,
            &mut self.reader_thread,
            &mut self.writer_thread,
        )
    }
}

trait PanePtySessionLifecycle: Send + 'static {
    type MasterClose: PanePtyMasterCloseLifecycle;

    fn stop_before(&mut self, timeout: Duration) -> Result<PtyExitStatus, String>;

    fn finish_before(&mut self, timeout: Duration) -> Result<PtyExitStatus, String>;

    fn begin_master_close(&mut self) -> Self::MasterClose;

    fn reap_until_exit(&mut self);
}

impl PanePtySessionLifecycle for PtySession {
    type MasterClose = PtyMasterClose;

    fn stop_before(&mut self, timeout: Duration) -> Result<PtyExitStatus, String> {
        self.terminate(timeout).map_err(|error| error.to_string())
    }

    fn finish_before(&mut self, timeout: Duration) -> Result<PtyExitStatus, String> {
        self.wait_for_exit(timeout)
            .map_err(|error| error.to_string())
    }

    fn begin_master_close(&mut self) -> Self::MasterClose {
        PtySession::begin_master_close(self)
    }

    fn reap_until_exit(&mut self) {
        const REAP_ATTEMPT: Duration = Duration::from_secs(2);
        loop {
            match self.terminate(REAP_ATTEMPT) {
                Ok(_) => return,
                Err(error) => {
                    eprintln!(
                        "pane PTY reaper is retaining a child after cleanup failure: {error}"
                    );
                    thread::park_timeout(Duration::from_millis(50));
                }
            }
        }
    }
}

enum PanePtyMasterCloseOutcome {
    Completed,
    Deferred,
    Failed(String),
    Panicked,
    Retained,
}

trait PanePtyMasterCloseLifecycle: Send + 'static {
    fn finish_before(&mut self, deadline: Instant) -> PanePtyMasterCloseOutcome;
}

impl PanePtyMasterCloseLifecycle for PtyMasterClose {
    fn finish_before(&mut self, deadline: Instant) -> PanePtyMasterCloseOutcome {
        match PtyMasterClose::finish_before(self, deadline) {
            PtyMasterCloseStatus::Completed => PanePtyMasterCloseOutcome::Completed,
            PtyMasterCloseStatus::Deferred => PanePtyMasterCloseOutcome::Deferred,
            PtyMasterCloseStatus::Failed(error) => {
                PanePtyMasterCloseOutcome::Failed(error.to_string())
            }
            PtyMasterCloseStatus::Panicked => PanePtyMasterCloseOutcome::Panicked,
            PtyMasterCloseStatus::Retained => PanePtyMasterCloseOutcome::Retained,
        }
    }
}

struct PanePtyOwnership<S: PanePtySessionLifecycle> {
    session: Option<S>,
    writer: Option<Box<dyn Write + Send>>,
    master_close: Option<S::MasterClose>,
    reader_thread: Option<thread::JoinHandle<()>>,
    writer_thread: Option<thread::JoinHandle<()>>,
}

impl<S: PanePtySessionLifecycle> PanePtyOwnership<S> {
    fn reap(mut self) {
        if let Some(session) = self.session.as_mut() {
            loop {
                let reaped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    session.reap_until_exit();
                }));
                if reaped.is_ok() {
                    break;
                }
                eprintln!("pane PTY reaper session cleanup panicked; retaining ownership");
                thread::park_timeout(Duration::from_millis(50));
            }
        }
        if self.master_close.is_none()
            && let Some(session) = self.session.as_mut()
        {
            self.master_close = Some(session.begin_master_close());
        }
        drop(self.writer.take());

        let mut issues = Vec::new();
        loop {
            let complete = poll_pane_pty_io(&mut self, &mut issues);
            for issue in issues.drain(..) {
                report_pane_pty_reaper_issue(&issue);
            }
            if complete {
                break;
            }
            thread::park_timeout(Duration::from_millis(5));
        }
    }
}

fn report_pane_pty_reaper_issue(issue: &str) {
    eprintln!("pane PTY reaper cleanup issue: {issue}");
    #[cfg(test)]
    lock_pane_pty_reaper_reported_issues().push(issue.to_owned());
}

#[cfg(test)]
fn pane_pty_reaper_reported_issues() -> &'static Mutex<Vec<String>> {
    static REPORTED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    REPORTED.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(test)]
fn lock_pane_pty_reaper_reported_issues() -> std::sync::MutexGuard<'static, Vec<String>> {
    pane_pty_reaper_reported_issues()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
fn take_pane_pty_reaper_reported_issues() -> Vec<String> {
    std::mem::take(&mut *lock_pane_pty_reaper_reported_issues())
}

fn poll_pane_pty_io<S: PanePtySessionLifecycle>(
    ownership: &mut PanePtyOwnership<S>,
    issues: &mut Vec<String>,
) -> bool {
    let close_status = ownership
        .master_close
        .as_mut()
        .map(|master_close| master_close.finish_before(Instant::now()));
    match close_status {
        Some(PanePtyMasterCloseOutcome::Completed) => drop(ownership.master_close.take()),
        Some(PanePtyMasterCloseOutcome::Failed(error)) => {
            issues.push(format!("pane PTY master close failed: {error}"));
            drop(ownership.master_close.take());
        }
        Some(PanePtyMasterCloseOutcome::Panicked) => {
            issues.push("pane PTY master close worker panicked".to_owned());
            drop(ownership.master_close.take());
        }
        Some(PanePtyMasterCloseOutcome::Retained) => {
            issues.push("pane PTY master close ownership was retained".to_owned());
            drop(ownership.master_close.take());
        }
        Some(PanePtyMasterCloseOutcome::Deferred) | None => {}
    }

    if ownership
        .reader_thread
        .as_ref()
        .is_some_and(thread::JoinHandle::is_finished)
        && ownership
            .reader_thread
            .take()
            .is_some_and(|reader_thread| reader_thread.join().is_err())
    {
        issues.push("pane PTY reader thread panicked".to_owned());
    }

    if ownership
        .writer_thread
        .as_ref()
        .is_some_and(thread::JoinHandle::is_finished)
        && ownership
            .writer_thread
            .take()
            .is_some_and(|writer_thread| writer_thread.join().is_err())
    {
        issues.push("pane PTY writer thread panicked".to_owned());
    }

    ownership.master_close.is_none()
        && ownership.reader_thread.is_none()
        && ownership.writer_thread.is_none()
}

#[derive(Clone, Copy)]
enum PanePtyCleanupOperation {
    Stop,
    Finish,
}

struct PanePtyCleanupOutcome {
    status: Option<PtyExitStatus>,
    issue: Option<String>,
    transferred_to_reaper: bool,
}

impl PanePtyCleanupOutcome {
    fn complete(status: Option<PtyExitStatus>) -> Self {
        Self {
            status,
            issue: None,
            transferred_to_reaper: false,
        }
    }
}

fn pane_pty_reaper_threads() -> &'static Mutex<Vec<thread::JoinHandle<()>>> {
    PANE_PTY_REAPER_THREADS.get_or_init(|| Mutex::new(Vec::new()))
}

fn lock_pane_pty_reaper_threads() -> std::sync::MutexGuard<'static, Vec<thread::JoinHandle<()>>> {
    pane_pty_reaper_threads()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn collect_finished_pane_pty_reapers() {
    let mut threads = lock_pane_pty_reaper_threads();
    let mut index = 0;
    while index < threads.len() {
        if threads[index].is_finished() {
            let handle = threads.swap_remove(index);
            if handle.join().is_err() {
                eprintln!("pane PTY ownership reaper panicked");
            }
        } else {
            index += 1;
        }
    }
}

#[cfg(test)]
fn pane_pty_reaper_pending() -> usize {
    collect_finished_pane_pty_reapers();
    PANE_PTY_REAPER_PENDING.load(Ordering::Acquire)
}

fn transfer_pane_pty_ownership_to_reaper<S: PanePtySessionLifecycle>(
    ownership: PanePtyOwnership<S>,
) -> Result<(), String> {
    struct PendingGuard;

    impl Drop for PendingGuard {
        fn drop(&mut self) {
            PANE_PTY_REAPER_PENDING.fetch_sub(1, Ordering::AcqRel);
        }
    }

    collect_finished_pane_pty_reapers();
    let ownership = Arc::new(Mutex::new(Some(ownership)));
    let ownership_for_thread = Arc::clone(&ownership);
    PANE_PTY_REAPER_PENDING.fetch_add(1, Ordering::AcqRel);
    match thread::Builder::new()
        .name("rssh-pane-pty-reaper".to_owned())
        .spawn(move || {
            let _pending = PendingGuard;
            let ownership = ownership_for_thread
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take();
            if let Some(ownership) = ownership {
                ownership.reap();
            }
        }) {
        Ok(handle) => {
            lock_pane_pty_reaper_threads().push(handle);
            Ok(())
        }
        Err(error) => {
            PANE_PTY_REAPER_PENDING.fetch_sub(1, Ordering::AcqRel);
            PANE_PTY_REAPER_RETAINED
                .get_or_init(|| Mutex::new(Vec::new()))
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(Box::new(ownership));
            Err(format!(
                "failed to start pane PTY reaper ({error}); ownership retained for process lifetime"
            ))
        }
    }
}

fn cleanup_pane_pty_ownership<S: PanePtySessionLifecycle>(
    mut ownership: PanePtyOwnership<S>,
    operation: PanePtyCleanupOperation,
    deadline: Instant,
) -> PanePtyCleanupOutcome {
    let status = if let Some(session) = ownership.session.as_mut() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let result = match operation {
            PanePtyCleanupOperation::Stop => session.stop_before(remaining),
            PanePtyCleanupOperation::Finish => session.finish_before(remaining),
        };
        match result {
            Ok(status) => {
                ownership.master_close = Some(session.begin_master_close());
                Some(status)
            }
            Err(error) => {
                let action = match operation {
                    PanePtyCleanupOperation::Stop => "terminate",
                    PanePtyCleanupOperation::Finish => "wait",
                };
                let (transferred_to_reaper, transfer_note) =
                    match transfer_pane_pty_ownership_to_reaper(ownership) {
                        Ok(()) => (true, "transferred to reaper".to_owned()),
                        Err(error) => (false, error),
                    };
                return PanePtyCleanupOutcome {
                    status: None,
                    issue: Some(format!(
                        "pane PTY {action} failed: {error}; {transfer_note}"
                    )),
                    transferred_to_reaper,
                };
            }
        }
    } else {
        None
    };
    drop(ownership.writer.take());

    let mut issues = Vec::new();
    while !poll_pane_pty_io(&mut ownership, &mut issues) {
        let now = Instant::now();
        if now >= deadline {
            let (transferred_to_reaper, transfer_note) =
                match transfer_pane_pty_ownership_to_reaper(ownership) {
                    Ok(()) => (true, "transferred to reaper".to_owned()),
                    Err(error) => (false, error),
                };
            let timeout_issue = format!(
                "pane PTY close or reader drain did not finish before cleanup deadline; \
                 {transfer_note}"
            );
            let issue = if issues.is_empty() {
                timeout_issue
            } else {
                format!("{}; {timeout_issue}", issues.join("; "))
            };
            return PanePtyCleanupOutcome {
                status,
                issue: Some(issue),
                transferred_to_reaper,
            };
        }
        thread::park_timeout((deadline - now).min(Duration::from_millis(5)));
    }

    if !issues.is_empty() {
        return PanePtyCleanupOutcome {
            status,
            issue: Some(issues.join("; ")),
            transferred_to_reaper: false,
        };
    }
    PanePtyCleanupOutcome::complete(status)
}

fn report_pane_pty_cleanup(context: &str, outcome: &PanePtyCleanupOutcome) {
    if let Some(issue) = &outcome.issue {
        let ownership = if outcome.transferred_to_reaper {
            " (ownership retained by the process-lifetime reaper)"
        } else {
            ""
        };
        eprintln!("{context}: {issue}{ownership}");
    }
}

fn stop_pty_lifecycle(
    session: &mut Option<PtySession>,
    session_process_id: &mut Option<u32>,
    session_tty_name: &mut Option<String>,
    writer: &mut Option<Box<dyn Write + Send>>,
    reader_thread: &mut Option<thread::JoinHandle<()>>,
    writer_thread: &mut Option<thread::JoinHandle<()>>,
) -> PanePtyCleanupOutcome {
    const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);
    let deadline = Instant::now() + CLOSE_TIMEOUT;
    let ownership = PanePtyOwnership {
        session: session.take(),
        writer: writer.take(),
        master_close: None,
        reader_thread: reader_thread.take(),
        writer_thread: writer_thread.take(),
    };
    *session_process_id = None;
    *session_tty_name = None;
    cleanup_pane_pty_ownership(ownership, PanePtyCleanupOperation::Stop, deadline)
}

fn finish_pty_lifecycle_after_exit(
    session: &mut Option<PtySession>,
    session_process_id: &mut Option<u32>,
    session_tty_name: &mut Option<String>,
    writer: &mut Option<Box<dyn Write + Send>>,
    reader_thread: &mut Option<thread::JoinHandle<()>>,
    writer_thread: &mut Option<thread::JoinHandle<()>>,
) -> PanePtyCleanupOutcome {
    const FINISH_TIMEOUT: Duration = Duration::from_secs(2);
    let deadline = Instant::now() + FINISH_TIMEOUT;
    let ownership = PanePtyOwnership {
        session: session.take(),
        writer: writer.take(),
        master_close: None,
        reader_thread: reader_thread.take(),
        writer_thread: writer_thread.take(),
    };
    *session_process_id = None;
    *session_tty_name = None;
    cleanup_pane_pty_ownership(ownership, PanePtyCleanupOperation::Finish, deadline)
}

fn allocate_pane_runtime_token_from(next_token: &AtomicU64) -> u64 {
    let mut current = next_token.load(Ordering::Relaxed);
    loop {
        assert_ne!(current, 0, "pane runtime token allocator reached zero");
        let next = current
            .checked_add(1)
            .expect("pane runtime token space exhausted");
        match next_token.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return current,
            Err(observed) => current = observed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PaneRenderRect {
    pane_id: rssh_core::PaneId,
    row: u16,
    column: u16,
    rows: u16,
    columns: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PaneSeparator {
    row: u16,
    column: u16,
    rows: u16,
    columns: u16,
    direction: SplitDirection,
    source_pane: rssh_core::PaneId,
    new_pane: rssh_core::PaneId,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PaneRenderLayout {
    panes: Vec<PaneRenderRect>,
    separators: Vec<PaneSeparator>,
}

struct TabBadgeContext<'a> {
    iterm2_pid: u32,
    window_id: u64,
    window_style: &'static str,
    tab_id: u64,
    current_session_id: u64,
    current_session_name: Option<String>,
    current_session_process_id: Option<u32>,
    current_session_tty_name: Option<String>,
    current_session_job_name: Option<&'a str>,
    current_session_command_line: Option<String>,
    current_session_last_command: Option<String>,
    current_session_terminal_icon_name: Option<String>,
    current_session_terminal_window_name: Option<String>,
    current_session_path: Option<&'a str>,
    current_session_profile_name: Option<&'a str>,
    current_session_mouse_reporting_mode: i16,
    current_session_mouse_info: Option<ItermMouseInfo>,
    current_session_application_keypad: bool,
    current_session_bell_count: u64,
    current_session_columns: u16,
    current_session_rows: u16,
    current_session_selection: Option<String>,
    title: Option<String>,
    title_override: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PaneMouseCell {
    pane_id: rssh_core::PaneId,
    row: u16,
    column: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(not(test), allow(dead_code))]
struct WheelTarget {
    pane_id: rssh_core::PaneId,
    rect: PaneRenderRect,
    cell: PaneMouseCell,
    pixel_position: PhysicalPosition<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(not(test), allow(dead_code))]
enum WheelHitTarget {
    PaneSurface(WheelTarget),
    ActiveScrollbar { pane_id: rssh_core::PaneId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeWindowFrame {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl NativeWindowFrame {
    fn set_position(&mut self, position: PhysicalPosition<i32>) {
        self.x = position.x;
        self.y = position.y;
    }

    fn set_size(&mut self, size: PhysicalSize<u32>) {
        self.width = size.width;
        self.height = size.height;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ItermMouseInfo {
    pane_id: rssh_core::PaneId,
    x: u16,
    y: usize,
    button: u16,
    click_count: u16,
    modifier_mask: u8,
    side_effects: u16,
    event_type: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PaneSplitResizeDrag {
    pane_id: rssh_core::PaneId,
    direction: SplitDirection,
    last_row: u16,
    last_column: u16,
}

#[derive(Debug, Clone, Copy)]
struct PanePointerTransientState {
    selecting: bool,
    active_mouse_button: Option<MouseButton>,
    scrollbar_dragging: bool,
    split_resize_dragging: Option<PaneSplitResizeDrag>,
    last_mouse_assignment_click: Option<WindowMouseAssignmentClick>,
    last_left_click: Option<WindowClick>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiKeyRelease {
    Escape,
    Enter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiKeyReleasePending {
    FullBarrier(UiKeyRelease),
    MatchingReleaseOnly(UiKeyRelease),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneInspectionRequestSource {
    Direct,
    CommandPaletteExecute,
}

type FrameRenderMode = rssh_native::RenderMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresentationOwner {
    Bootstrap,
    GpuInitializing,
    GpuActive,
    CpuFallback,
}

const fn deferred_gpu_initialization_owner(succeeded: bool) -> PresentationOwner {
    if succeeded {
        PresentationOwner::GpuInitializing
    } else {
        PresentationOwner::CpuFallback
    }
}

fn resize_deferred_gpu_candidate_for_install<E>(
    current_size: PhysicalSize<u32>,
    resize_surface: impl FnOnce(PhysicalSize<u32>) -> Result<(), E>,
) -> Result<PresentationOwner, E> {
    resize_surface(current_size)?;
    Ok(deferred_gpu_initialization_owner(true))
}

const fn presentation_owner_after_gpu_frame(
    current: PresentationOwner,
    status: GpuFrameStatus,
) -> PresentationOwner {
    if matches!(
        (current, status),
        (PresentationOwner::GpuInitializing, GpuFrameStatus::Presented)
    ) {
        PresentationOwner::GpuActive
    } else {
        current
    }
}

fn release_bootstrap_staging_after_gpu_activation(bootstrap_frame: &mut Vec<u8>) {
    *bootstrap_frame = Vec::new();
}

fn visible_snapshot_cell_count(snapshot: &TerminalRenderSnapshot) -> usize {
    snapshot
        .iter_cells()
        .filter(|cell| !cell.text.trim().is_empty())
        .count()
}

const fn diagnostic_connection_state(state: ConnectionState) -> DiagnosticConnectionState {
    match state {
        ConnectionState::NotStarted => DiagnosticConnectionState::NotStarted,
        ConnectionState::Pending => DiagnosticConnectionState::Pending,
        ConnectionState::Connecting => DiagnosticConnectionState::Connecting,
        ConnectionState::AwaitingSecret => DiagnosticConnectionState::AwaitingSecret,
        ConnectionState::AwaitingHostKey => DiagnosticConnectionState::AwaitingHostKey,
        ConnectionState::Connected => DiagnosticConnectionState::Connected,
        ConnectionState::Disconnected => DiagnosticConnectionState::Disconnected,
        ConnectionState::Failed => DiagnosticConnectionState::Failed,
    }
}

fn finalize_native_gpu_frame<E>(
    outcome: Result<GpuFrameStatus, E>,
    pending_damage: &mut Vec<DamageRegion>,
    needs_full_repaint: &mut bool,
) -> Result<bool, E> {
    match outcome {
        Ok(GpuFrameStatus::Presented) => {
            pending_damage.clear();
            *needs_full_repaint = false;
            Ok(true)
        }
        Ok(GpuFrameStatus::Skipped) => {
            *needs_full_repaint = true;
            Ok(false)
        }
        Err(error) => {
            *needs_full_repaint = true;
            Err(error)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct WindowMetricsSnapshot {
    runtime_api: String,
    runtime_live_threads: usize,
    last_exit_code: Option<u32>,
    process_to_first_present_ms: Option<u128>,
    config_duration_ms: Option<u128>,
    gpu_duration_ms: Option<u128>,
    ssh_duration_ms: Option<u128>,
    first_frame_private_bytes: Option<u64>,
    final_renderer: Option<RendererKind>,
    connection_state: ConnectionState,
    first_pty_byte_ms: Option<u128>,
    first_rendered_cell_ms: Option<u128>,
    pty_chunks: u64,
    pty_bytes: u64,
    pty_linkage_found: bool,
    pty_linkage_digest: Option<String>,
    terminal_linkage_nonce_found: bool,
    terminal_snapshot_content_digest: Option<String>,
    pty_chunk_process_p95_us: u128,
    damage_regions: u64,
    damaged_cells: u64,
    snapshot_damage_updates: u64,
    snapshot_rebuilds: u64,
    render_frames: u64,
    full_render_frames: u64,
    dirty_render_frames: u64,
    render_frame_p95_us: u128,
    gpu_backend: String,
    gpu_adapter_name: String,
    gpu_adapter_vendor_id: u32,
    gpu_adapter_device_id: u32,
    gpu_adapter_type: String,
    gpu_software_adapter: bool,
    gpu_surface_format: Option<String>,
    gpu_present_mode: Option<String>,
    gpu_surface_width: Option<u32>,
    gpu_surface_height: Option<u32>,
    gpu_rendered_frames: u64,
    gpu_presented_frames: u64,
    gpu_surface_reconfigurations: u64,
    gpu_surface_recreations: u64,
    gpu_surface_timeouts: u64,
    gpu_surface_occlusions: u64,
    gpu_surface_validation_errors: u64,
    gpu_compatibility_frame_uploads: u64,
    gpu_uncaptured_errors: u64,
    gpu_device_losses: u64,
    gpu_device_recoveries: u64,
    gpu_device_recovery_failures: u64,
    gpu_abandoned_lost_surfaces: u64,
    text_backend: String,
    gpu_text_prepared_glyphs: usize,
    gpu_text_mask_glyphs: usize,
    gpu_text_color_glyphs: usize,
    gpu_text_block_glyphs: usize,
    gpu_text_content_digest: Option<String>,
    gpu_text_rendered_frames: u64,
    input_writes: u64,
    input_bytes: u64,
    input_write_p95_us: u128,
    bells: u64,
}

impl WindowMetricsSnapshot {
    #[allow(clippy::too_many_lines)]
    fn report(self) -> String {
        format!(
            "\
R-SSH metrics
runtime_api={}
runtime_live_threads={}
last_exit_code={}
process_to_first_present_ms={}
config_duration_ms={}
gpu_duration_ms={}
ssh_duration_ms={}
first_frame_private_bytes={}
final_renderer={}
connection_state={}
first_pty_byte_ms={}
first_rendered_cell_ms={}
pty_chunks={}
pty_bytes={}
pty_linkage_found={}
pty_linkage_digest={}
terminal_linkage_nonce_found={}
terminal_snapshot_content_digest={}
pty_chunk_process_p95_us={}
damage_regions={}
damaged_cells={}
snapshot_damage_updates={}
snapshot_rebuilds={}
render_frames={}
full_render_frames={}
dirty_render_frames={}
render_frame_p95_us={}
gpu_backend={}
gpu_adapter_name={}
gpu_adapter_vendor_id={}
gpu_adapter_device_id={}
gpu_adapter_type={}
gpu_software_adapter={}
gpu_surface_format={}
gpu_present_mode={}
gpu_surface_width={}
gpu_surface_height={}
gpu_rendered_frames={}
gpu_presented_frames={}
gpu_surface_reconfigurations={}
gpu_surface_recreations={}
gpu_surface_timeouts={}
gpu_surface_occlusions={}
gpu_surface_validation_errors={}
gpu_compatibility_frame_uploads={}
gpu_uncaptured_errors={}
gpu_device_losses={}
gpu_device_recoveries={}
gpu_device_recovery_failures={}
gpu_abandoned_lost_surfaces={}
text_backend={}
gpu_text_prepared_glyphs={}
gpu_text_mask_glyphs={}
gpu_text_color_glyphs={}
gpu_text_block_glyphs={}
gpu_text_content_digest={}
gpu_text_rendered_frames={}
input_writes={}
input_bytes={}
input_write_p95_us={}
bells={}
",
            self.runtime_api,
            self.runtime_live_threads,
            metric_option_u32(self.last_exit_code),
            metric_option(self.process_to_first_present_ms),
            metric_option(self.config_duration_ms),
            metric_option(self.gpu_duration_ms),
            metric_option(self.ssh_duration_ms),
            metric_option_u64(self.first_frame_private_bytes),
            renderer_metric_name(self.final_renderer),
            connection_metric_name(self.connection_state),
            metric_option(self.first_pty_byte_ms),
            metric_option(self.first_rendered_cell_ms),
            self.pty_chunks,
            self.pty_bytes,
            self.pty_linkage_found,
            metric_option_string(self.pty_linkage_digest.as_deref()),
            self.terminal_linkage_nonce_found,
            metric_option_string(self.terminal_snapshot_content_digest.as_deref()),
            self.pty_chunk_process_p95_us,
            self.damage_regions,
            self.damaged_cells,
            self.snapshot_damage_updates,
            self.snapshot_rebuilds,
            self.render_frames,
            self.full_render_frames,
            self.dirty_render_frames,
            self.render_frame_p95_us,
            self.gpu_backend,
            self.gpu_adapter_name,
            self.gpu_adapter_vendor_id,
            self.gpu_adapter_device_id,
            self.gpu_adapter_type,
            self.gpu_software_adapter,
            metric_option_string(self.gpu_surface_format.as_deref()),
            metric_option_string(self.gpu_present_mode.as_deref()),
            metric_option_u32(self.gpu_surface_width),
            metric_option_u32(self.gpu_surface_height),
            self.gpu_rendered_frames,
            self.gpu_presented_frames,
            self.gpu_surface_reconfigurations,
            self.gpu_surface_recreations,
            self.gpu_surface_timeouts,
            self.gpu_surface_occlusions,
            self.gpu_surface_validation_errors,
            self.gpu_compatibility_frame_uploads,
            self.gpu_uncaptured_errors,
            self.gpu_device_losses,
            self.gpu_device_recoveries,
            self.gpu_device_recovery_failures,
            self.gpu_abandoned_lost_surfaces,
            self.text_backend,
            self.gpu_text_prepared_glyphs,
            self.gpu_text_mask_glyphs,
            self.gpu_text_color_glyphs,
            self.gpu_text_block_glyphs,
            metric_option_string(self.gpu_text_content_digest.as_deref()),
            self.gpu_text_rendered_frames,
            self.input_writes,
            self.input_bytes,
            self.input_write_p95_us,
            self.bells
        )
    }

    fn json_report(self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug)]
struct WindowMetrics {
    spawn_started_at: Instant,
    startup_trace: StartupTrace,
    first_pty_byte: Option<Duration>,
    first_rendered_cell: Option<Duration>,
    pty_chunks: u64,
    pty_bytes: u64,
    pty_linkage_enabled: bool,
    pty_content_probe: Vec<u8>,
    pty_linkage_payload: Option<Vec<u8>>,
    terminal_linkage_nonce_found: bool,
    terminal_snapshot_content_digest: Option<rterm_render_core::TerminalContentDigest>,
    pty_chunk_process_times: Vec<Duration>,
    damage_regions: u64,
    damaged_cells: u64,
    snapshot_damage_updates: u64,
    snapshot_rebuilds: u64,
    render_frame_times: Vec<Duration>,
    full_render_frames: u64,
    dirty_render_frames: u64,
    input_writes: u64,
    input_bytes: u64,
    input_write_times: Vec<Duration>,
    bells: u64,
    last_exit_code: Option<u32>,
    observed_pane_exit_statuses: HashMap<(rssh_core::PaneId, u64), PtyExitStatus>,
}

impl WindowMetrics {
    fn new() -> Self {
        let pty_linkage_enabled =
            std::env::var_os("RSSH_TEST_PTY_LINKAGE").as_deref() == Some(std::ffi::OsStr::new("1"));
        Self {
            spawn_started_at: Instant::now(),
            startup_trace: StartupTrace::new(),
            first_pty_byte: None,
            first_rendered_cell: None,
            pty_chunks: 0,
            pty_bytes: 0,
            pty_linkage_enabled,
            pty_content_probe: Vec::new(),
            pty_linkage_payload: None,
            terminal_linkage_nonce_found: false,
            terminal_snapshot_content_digest: None,
            pty_chunk_process_times: Vec::new(),
            damage_regions: 0,
            damaged_cells: 0,
            snapshot_damage_updates: 0,
            snapshot_rebuilds: 0,
            render_frame_times: Vec::new(),
            full_render_frames: 0,
            dirty_render_frames: 0,
            input_writes: 0,
            input_bytes: 0,
            input_write_times: Vec::new(),
            bells: 0,
            last_exit_code: None,
            observed_pane_exit_statuses: HashMap::new(),
        }
    }

    fn mark_config_started(&mut self) {
        self.startup_trace.mark_config_started();
    }

    fn mark_config_finished(&mut self) {
        self.startup_trace.mark_config_finished();
    }

    fn mark_gpu_started(&mut self) {
        self.startup_trace.mark_gpu_started();
    }

    fn mark_gpu_finished(&mut self) {
        self.startup_trace.mark_gpu_finished();
    }

    fn mark_renderer(&mut self, renderer: RendererKind) {
        self.startup_trace.mark_renderer(renderer);
    }

    fn mark_ssh_started(&mut self) {
        self.startup_trace.mark_ssh_started();
    }

    fn mark_ssh_connected(&mut self) {
        self.startup_trace.mark_ssh_connected();
    }

    fn mark_connection_state(&mut self, state: ConnectionState) {
        self.startup_trace.mark_connection_state(state);
    }

    fn record_first_present(&mut self, renderer: RendererKind) -> bool {
        self.startup_trace.mark_renderer(renderer);
        self.startup_trace.mark_first_present_to(&mut io::stderr())
            .unwrap_or(false)
    }

    fn record_first_frame_private_bytes(&mut self, private_bytes: u64) {
        let _ = self
            .startup_trace
            .mark_first_frame_private_bytes_to(&mut io::stderr(), private_bytes);
    }

    fn start_spawn_timer(&mut self) {
        self.spawn_started_at = Instant::now();
        self.first_pty_byte = None;
        self.first_rendered_cell = None;
    }

    fn record_pty_chunk(&mut self, bytes: &[u8]) {
        self.record_first_pty_byte();
        self.pty_chunks = self.pty_chunks.saturating_add(1);
        self.pty_bytes = self
            .pty_bytes
            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
    }

    fn record_first_pty_byte(&mut self) {
        if self.first_pty_byte.is_none() {
            self.first_pty_byte = Some(self.spawn_started_at.elapsed());
        }
    }

    fn record_first_pty_byte_at(&mut self, observed_at: Instant) {
        if self.first_pty_byte.is_none() {
            self.first_pty_byte = Some(
                observed_at
                    .checked_duration_since(self.spawn_started_at)
                    .unwrap_or(Duration::ZERO),
            );
        }
    }

    fn record_active_pty_content(&mut self, bytes: &[u8]) {
        const BEGIN: &[u8] = b"RSSH-LINK-BEGIN|";
        const END: &[u8] = b"|RSSH-LINK-END";
        const MAX_PROBE_BYTES: usize = 64 * 1024;

        if !self.pty_linkage_enabled || self.pty_linkage_payload.is_some() {
            return;
        }
        self.pty_content_probe.extend_from_slice(bytes);
        let Some(begin) = find_bytes(&self.pty_content_probe, BEGIN) else {
            let retained = BEGIN.len().saturating_sub(1);
            if self.pty_content_probe.len() > retained {
                let discard = self.pty_content_probe.len() - retained;
                self.pty_content_probe.drain(..discard);
            }
            return;
        };
        let payload_start = begin.saturating_add(BEGIN.len());
        if let Some(end) = find_bytes(&self.pty_content_probe[payload_start..], END) {
            let payload_end = payload_start.saturating_add(end);
            self.pty_linkage_payload =
                Some(self.pty_content_probe[payload_start..payload_end].to_vec());
            self.pty_content_probe.clear();
            return;
        }
        if begin > 0 {
            self.pty_content_probe.drain(..begin);
        }
        if self.pty_content_probe.len() > MAX_PROBE_BYTES {
            self.pty_content_probe.clear();
        }
    }

    fn record_terminal_linkage_snapshot(&mut self, snapshot: &TerminalRenderSnapshot) {
        if !self.pty_linkage_enabled {
            return;
        }
        self.terminal_snapshot_content_digest =
            Some(rterm_render_core::terminal_snapshot_content_digest(snapshot));
        let Some(payload) = self.pty_linkage_payload.as_deref() else {
            self.terminal_linkage_nonce_found = false;
            return;
        };
        let nonce = payload
            .split(|byte| *byte == b'|')
            .next()
            .unwrap_or_default();
        let ordered_text = snapshot
            .iter_cells()
            .filter(|cell| !cell.continuation)
            .flat_map(|cell| cell.text.bytes())
            .collect::<Vec<_>>();
        self.terminal_linkage_nonce_found =
            !nonce.is_empty() && find_bytes(&ordered_text, nonce).is_some();
    }

    fn record_first_rendered_cell(&mut self, snapshot_is_empty: bool) {
        if self.first_rendered_cell.is_none() && !snapshot_is_empty {
            self.first_rendered_cell = Some(self.spawn_started_at.elapsed());
        }
    }

    fn record_pty_chunk_process(&mut self, duration: Duration) {
        self.pty_chunk_process_times.push(duration);
    }

    fn record_damage(&mut self, damage: &[DamageRegion]) {
        self.damage_regions = self
            .damage_regions
            .saturating_add(u64::try_from(damage.len()).unwrap_or(u64::MAX));
        let cells = damage.iter().fold(0_u64, |total, region| {
            total.saturating_add(damage_region_cells(*region))
        });
        self.damaged_cells = self.damaged_cells.saturating_add(cells);
    }

    fn record_snapshot_damage_update(&mut self) {
        self.snapshot_damage_updates = self.snapshot_damage_updates.saturating_add(1);
    }

    fn record_snapshot_rebuild(&mut self) {
        self.snapshot_rebuilds = self.snapshot_rebuilds.saturating_add(1);
    }

    fn record_render_frame(&mut self, duration: Duration) {
        self.render_frame_times.push(duration);
    }

    fn record_frame_render_mode(&mut self, mode: FrameRenderMode) {
        match mode {
            FrameRenderMode::Full => {
                self.full_render_frames = self.full_render_frames.saturating_add(1);
            }
            FrameRenderMode::Damage => {
                self.dirty_render_frames = self.dirty_render_frames.saturating_add(1);
            }
        }
    }

    fn record_input_write(&mut self, byte_count: usize, duration: Duration) {
        self.input_writes = self.input_writes.saturating_add(1);
        self.input_bytes = self
            .input_bytes
            .saturating_add(u64::try_from(byte_count).unwrap_or(u64::MAX));
        self.input_write_times.push(duration);
    }

    fn record_bells(&mut self, count: u64) {
        self.bells = self.bells.saturating_add(count);
    }

    fn record_exit_status(&mut self, status: &PtyExitStatus) {
        self.last_exit_code = Some(status.exit_code());
    }

    fn snapshot(&self) -> WindowMetricsSnapshot {
        self.snapshot_with_gpu(
            &GpuPresentationMetrics::uninitialized(),
            "bitmap-emergency",
            None,
        )
    }

    fn snapshot_with_gpu(
        &self,
        gpu: &GpuPresentationMetrics,
        text_backend: &str,
        direct_text: Option<(&rterm_render_wgpu::gpu::GpuTextPrepareReport, u64)>,
    ) -> WindowMetricsSnapshot {
        let (direct_report, direct_rendered_frames) =
            direct_text.map_or((None, 0), |(report, frames)| (Some(report), frames));
        let startup = self.startup_trace.snapshot();
        WindowMetricsSnapshot {
            runtime_api: "v2-runtime-hub".to_owned(),
            runtime_live_threads: 0,
            last_exit_code: self.last_exit_code,
            process_to_first_present_ms: startup.process_to_first_present_ms,
            config_duration_ms: startup.config_duration_ms,
            gpu_duration_ms: startup.gpu_duration_ms,
            ssh_duration_ms: startup.ssh_duration_ms,
            first_frame_private_bytes: startup.first_frame_private_bytes,
            final_renderer: startup.final_renderer,
            connection_state: startup.connection_state,
            first_pty_byte_ms: self.first_pty_byte.map(|duration| duration.as_millis()),
            first_rendered_cell_ms: self
                .first_rendered_cell
                .map(|duration| duration.as_millis()),
            pty_chunks: self.pty_chunks,
            pty_bytes: self.pty_bytes,
            pty_linkage_found: self.pty_linkage_payload.is_some(),
            pty_linkage_digest: self
                .pty_linkage_payload
                .as_deref()
                .map(rterm_render_core::terminal_bytes_content_digest)
                .map(content_digest_hex),
            terminal_linkage_nonce_found: self.terminal_linkage_nonce_found,
            terminal_snapshot_content_digest: self
                .terminal_snapshot_content_digest
                .map(content_digest_hex),
            pty_chunk_process_p95_us: p95_us(&self.pty_chunk_process_times),
            damage_regions: self.damage_regions,
            damaged_cells: self.damaged_cells,
            snapshot_damage_updates: self.snapshot_damage_updates,
            snapshot_rebuilds: self.snapshot_rebuilds,
            render_frames: u64::try_from(self.render_frame_times.len()).unwrap_or(u64::MAX),
            full_render_frames: self.full_render_frames,
            dirty_render_frames: self.dirty_render_frames,
            render_frame_p95_us: p95_us(&self.render_frame_times),
            gpu_backend: gpu.backend.clone(),
            gpu_adapter_name: gpu.adapter_name.clone(),
            gpu_adapter_vendor_id: gpu.adapter_vendor_id,
            gpu_adapter_device_id: gpu.adapter_device_id,
            gpu_adapter_type: gpu.adapter_type.clone(),
            gpu_software_adapter: gpu.software_adapter,
            gpu_surface_format: gpu.surface_format.clone(),
            gpu_present_mode: gpu.present_mode.clone(),
            gpu_surface_width: gpu.surface_width,
            gpu_surface_height: gpu.surface_height,
            gpu_rendered_frames: gpu.rendered_frames,
            gpu_presented_frames: gpu.presented_frames,
            gpu_surface_reconfigurations: gpu.surface_reconfigurations,
            gpu_surface_recreations: gpu.surface_recreations,
            gpu_surface_timeouts: gpu.surface_timeouts,
            gpu_surface_occlusions: gpu.surface_occlusions,
            gpu_surface_validation_errors: gpu.surface_validation_errors,
            gpu_compatibility_frame_uploads: gpu.compatibility_frame_uploads,
            gpu_uncaptured_errors: gpu.uncaptured_errors,
            gpu_device_losses: gpu.device_losses,
            gpu_device_recoveries: gpu.device_recoveries,
            gpu_device_recovery_failures: gpu.device_recovery_failures,
            gpu_abandoned_lost_surfaces: gpu.abandoned_lost_surfaces,
            text_backend: text_backend.to_owned(),
            gpu_text_prepared_glyphs: direct_report.map_or(0, |report| report.shaped_glyphs),
            gpu_text_mask_glyphs: direct_report.map_or(0, |report| report.mask_glyphs),
            gpu_text_color_glyphs: direct_report.map_or(0, |report| report.color_glyphs),
            gpu_text_block_glyphs: direct_report.map_or(0, |report| report.custom_block_glyphs),
            // A process-cold GPU can finish the configured frame budget before
            // the PTY output event is dispatched. Once the linkage probe has
            // observed the active terminal payload, expose that same digest
            // instead of retaining a stale pre-output GPU report.
            gpu_text_content_digest: if self.terminal_linkage_nonce_found {
                self.terminal_snapshot_content_digest
                    .map(content_digest_hex)
            } else {
                direct_report.map(|report| content_digest_hex(report.content_digest))
            },
            gpu_text_rendered_frames: direct_rendered_frames,
            input_writes: self.input_writes,
            input_bytes: self.input_bytes,
            input_write_p95_us: p95_us(&self.input_write_times),
            bells: self.bells,
        }
    }

    #[cfg(test)]
    fn connection_state(&self) -> ConnectionState {
        self.startup_trace.snapshot().connection_state
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn content_digest_hex(digest: rterm_render_core::TerminalContentDigest) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn p95_us(samples: &[Duration]) -> u128 {
    if samples.is_empty() {
        return 0;
    }

    let mut values = samples
        .iter()
        .map(std::time::Duration::as_micros)
        .collect::<Vec<_>>();
    values.sort_unstable();
    let index = values
        .len()
        .saturating_mul(95)
        .saturating_add(99)
        .saturating_div(100)
        .saturating_sub(1);

    values[index]
}

fn damage_region_cells(region: DamageRegion) -> u64 {
    u64::from(region.width).saturating_mul(u64::from(region.height))
}

fn padding_dimension_to_pixels(
    dimension: NativeWindowPaddingDimension,
    cell_pixels: u32,
    axis_pixels: u32,
    dpi: u32,
) -> u32 {
    match dimension {
        NativeWindowPaddingDimension::Pixels(pixels) => {
            let scaled = u64::from(pixels)
                .saturating_mul(u64::from(dpi.max(1)))
                .saturating_add(u64::from(DEFAULT_WINDOW_DPI / 2))
                / u64::from(DEFAULT_WINDOW_DPI);
            u32::try_from(scaled).unwrap_or(u32::MAX)
        }
        NativeWindowPaddingDimension::Points(points) => {
            points.saturating_mul(dpi).saturating_add(36) / 72
        }
        NativeWindowPaddingDimension::Percent(percent) => axis_pixels.saturating_mul(percent) / 100,
        NativeWindowPaddingDimension::CellFractionPerMille(per_mille) => {
            cell_pixels.saturating_mul(per_mille) / 1_000
        }
    }
}

fn padding_dimension_to_cells(
    dimension: NativeWindowPaddingDimension,
    cell_pixels: u32,
    axis_pixels: u32,
    dpi: u32,
) -> u16 {
    if let NativeWindowPaddingDimension::CellFractionPerMille(per_mille) = dimension {
        return u16::try_from(per_mille.saturating_add(999) / 1_000).unwrap_or(u16::MAX);
    }
    let pixels = padding_dimension_to_pixels(dimension, cell_pixels, axis_pixels, dpi);
    padding_pixels_to_cells(pixels, cell_pixels)
}

fn padding_pixels_to_cells(pixels: u32, cell_pixels: u32) -> u16 {
    if pixels == 0 || cell_pixels == 0 {
        return 0;
    }

    u16::try_from(pixels.div_ceil(cell_pixels)).unwrap_or(u16::MAX)
}

fn window_padding_pixels_for_terminal_size(
    padding: NativeWindowPadding,
    terminal_width: u32,
    terminal_height: u32,
    cell_width: u32,
    cell_height: u32,
    dpi: u32,
) -> NativeWindowPaddingPixels {
    NativeWindowPaddingPixels {
        left: padding_dimension_to_pixels(padding.left, cell_width, terminal_width, dpi),
        right: padding_dimension_to_pixels(padding.right, cell_width, terminal_width, dpi),
        top: padding_dimension_to_pixels(padding.top, cell_height, terminal_height, dpi),
        bottom: padding_dimension_to_pixels(padding.bottom, cell_height, terminal_height, dpi),
    }
}

fn content_axis_pixels_from_window_pixels(
    total_pixels: u32,
    reserved_pixels: u32,
    cell_pixels: u32,
    start: NativeWindowPaddingDimension,
    end: NativeWindowPaddingDimension,
    dpi: u32,
) -> u32 {
    let available = total_pixels.saturating_sub(reserved_pixels);
    let fixed_start = match start {
        NativeWindowPaddingDimension::Percent(_) => 0,
        dimension => padding_dimension_to_pixels(dimension, cell_pixels, 0, dpi),
    };
    let fixed_end = match end {
        NativeWindowPaddingDimension::Percent(_) => 0,
        dimension => padding_dimension_to_pixels(dimension, cell_pixels, 0, dpi),
    };
    let fixed = fixed_start.saturating_add(fixed_end);
    let remaining = available.saturating_sub(fixed);
    let percent = u64::from(match start {
        NativeWindowPaddingDimension::Percent(percent) => percent,
        _ => 0,
    })
    .saturating_add(u64::from(match end {
        NativeWindowPaddingDimension::Percent(percent) => percent,
        _ => 0,
    }));
    if percent == 0 {
        return remaining;
    }

    // Percent padding is relative to the terminal content. Solve
    // content + content*(left+right)/100 = remaining directly instead of
    // iterating, which can oscillate for large percentages.
    let denominator = 100_u64.saturating_add(percent);
    u32::try_from(u64::from(remaining).saturating_mul(100) / denominator)
        .unwrap_or(u32::MAX)
}

fn terminal_size_from_window_pixels_with_padding(
    width: u32,
    height: u32,
    cell_width: u32,
    cell_height: u32,
    padding: NativeWindowPadding,
    dpi: u32,
) -> TerminalSize {
    let cell_width = cell_width.max(1);
    let cell_height = cell_height.max(1);
    let content_width = content_axis_pixels_from_window_pixels(
        width,
        0,
        cell_width,
        padding.left,
        padding.right,
        dpi,
    );
    let content_height = content_axis_pixels_from_window_pixels(
        height,
        u32::from(TAB_BAR_ROWS).saturating_mul(cell_height),
        cell_height,
        padding.top,
        padding.bottom,
        dpi,
    );
    let columns = u16::try_from((content_width / cell_width).clamp(1, u32::from(u16::MAX)))
        .expect("column count is clamped to u16");
    let rows = u16::try_from((content_height / cell_height).clamp(1, u32::from(u16::MAX)))
        .expect("row count is clamped to u16");
    TerminalSize::new(columns, rows)
}

fn metric_option(value: Option<u128>) -> String {
    value.map_or_else(|| "NA".to_owned(), |value| value.to_string())
}

fn metric_option_string(value: Option<&str>) -> &str {
    value.unwrap_or("NA")
}

fn metric_option_u32(value: Option<u32>) -> String {
    value.map_or_else(|| "NA".to_owned(), |value| value.to_string())
}

fn metric_option_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "NA".to_owned(), |value| value.to_string())
}

fn current_process_private_bytes() -> u64 {
    let Ok(pid) = get_current_pid() else {
        return 0;
    };
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid).map_or(0, sysinfo::Process::virtual_memory)
}

const fn renderer_metric_name(value: Option<RendererKind>) -> &'static str {
    match value {
        Some(RendererKind::Cpu) => "cpu",
        Some(RendererKind::Gpu) => "gpu",
        None => "NA",
    }
}

const fn connection_metric_name(value: ConnectionState) -> &'static str {
    match value {
        ConnectionState::NotStarted => "not_started",
        ConnectionState::Pending => "pending",
        ConnectionState::Connecting => "connecting",
        ConnectionState::AwaitingSecret => "awaiting_secret",
        ConnectionState::AwaitingHostKey => "awaiting_host_key",
        ConnectionState::Connected => "connected",
        ConnectionState::Disconnected => "disconnected",
        ConnectionState::Failed => "failed",
    }
}

fn default_skip_close_confirmation_for_processes_named() -> Vec<String> {
    DEFAULT_SKIP_CLOSE_CONFIRMATION_FOR_PROCESSES_NAMED
        .iter()
        .map(|process| (*process).to_owned())
        .collect()
}

fn process_file_name(process: &str) -> &str {
    process.rsplit(['/', '\\']).next().unwrap_or(process).trim()
}

fn pane_runtime_current_working_dir(
    runtime: &TerminalRuntime,
    session_process_id: Option<u32>,
) -> Option<String> {
    runtime
        .terminal()
        .current_working_dir()
        .map(str::to_owned)
        .or_else(|| session_process_id.and_then(process_current_working_dir))
}

/// Reports `Skipped` when the process fallback is throttled, and `Resolved`
/// when the terminal source or a due process probe produced the authoritative
/// result (including an authoritative missing value).
enum PaneRuntimeCwdUpdate {
    Skipped,
    Resolved(Option<String>),
}

fn pane_runtime_current_working_dir_if_due(
    runtime: &mut TerminalRuntime,
    session_process_id: Option<u32>,
    now: Instant,
) -> PaneRuntimeCwdUpdate {
    if let Some(cwd) = runtime.terminal().current_working_dir().map(str::to_owned) {
        runtime.reset_process_cwd_probe();
        return PaneRuntimeCwdUpdate::Resolved(Some(cwd));
    }

    let Some(process_id) = session_process_id else {
        return PaneRuntimeCwdUpdate::Resolved(None);
    };
    if !runtime.should_probe_process_cwd(process_id, now) {
        return PaneRuntimeCwdUpdate::Skipped;
    }

    let cwd = process_current_working_dir(process_id);
    runtime.complete_process_cwd_probe(process_id, Instant::now());
    PaneRuntimeCwdUpdate::Resolved(cwd)
}

fn process_current_working_dir(process_id: u32) -> Option<String> {
    #[cfg(test)]
    PROCESS_CWD_PROBE_COUNT.with(|count| count.set(count.get().saturating_add(1)));
    let pid = sysinfo::Pid::from_u32(process_id);
    let refreshes = sysinfo::RefreshKind::nothing().with_processes(
        sysinfo::ProcessRefreshKind::nothing()
            .with_cwd(sysinfo::UpdateKind::OnlyIfNotSet)
            .without_tasks(),
    );
    let mut system = sysinfo::System::new_with_specifics(refreshes);
    let _ = system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let candidates = system
        .processes()
        .iter()
        .map(|(pid, process)| ProcessCwdCandidate {
            pid: *pid,
            parent: process.parent(),
            start_time: process.start_time(),
            cwd: process.cwd(),
        })
        .collect::<Vec<_>>();

    process_tree_current_working_dir(&candidates, pid).map(process_cwd_to_string)
}

#[cfg(test)]
fn reset_process_cwd_probe_count() {
    PROCESS_CWD_PROBE_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
fn process_cwd_probe_count() -> u64 {
    PROCESS_CWD_PROBE_COUNT.with(Cell::get)
}

#[derive(Clone, Copy)]
struct ProcessCwdCandidate<'a> {
    pid: sysinfo::Pid,
    parent: Option<sysinfo::Pid>,
    start_time: u64,
    cwd: Option<&'a Path>,
}

fn process_tree_current_working_dir<'a>(
    processes: &[ProcessCwdCandidate<'a>],
    root: sysinfo::Pid,
) -> Option<&'a Path> {
    processes
        .iter()
        .filter_map(|process| {
            let depth = process_descendant_depth(processes, process.pid, root)?;
            if depth == 0 || process.cwd.is_none() {
                return None;
            }
            Some((depth, process.start_time, process.pid.as_u32(), process))
        })
        .max_by_key(|(depth, start_time, pid, _)| (*depth, *start_time, *pid))
        .and_then(|(_, _, _, process)| process.cwd)
        .or_else(|| {
            processes
                .iter()
                .find(|process| process.pid == root)
                .and_then(|process| process.cwd)
        })
}

fn process_descendant_depth(
    processes: &[ProcessCwdCandidate<'_>],
    pid: sysinfo::Pid,
    root: sysinfo::Pid,
) -> Option<usize> {
    let mut depth = 0;
    let mut current = pid;
    let mut seen = HashSet::new();
    while current != root {
        if !seen.insert(current) {
            return None;
        }
        current = processes
            .iter()
            .find(|process| process.pid == current)?
            .parent?;
        depth += 1;
    }
    Some(depth)
}

fn process_cwd_to_string(cwd: &Path) -> String {
    let mut value = cwd.to_string_lossy().into_owned();
    while has_redundant_trailing_path_separator(&value) {
        value.pop();
    }
    value
}
