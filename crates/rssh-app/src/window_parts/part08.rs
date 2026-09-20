fn has_redundant_trailing_path_separator(path: &str) -> bool {
    if !path.ends_with(['/', '\\']) {
        return false;
    }

    let trimmed = path.trim_end_matches(['/', '\\']);
    !trimmed.is_empty() && !trimmed.ends_with(':')
}

fn effective_force_fallback_adapter(configured: bool, software_front_end: bool) -> bool {
    let configured = configured || software_front_end;
    #[cfg(debug_assertions)]
    {
        configured
            || std::env::var_os("RSSH_TEST_FORCE_FALLBACK_ADAPTER").as_deref()
                == Some(std::ffi::OsStr::new("1"))
    }
    #[cfg(not(debug_assertions))]
    {
        configured
    }
}

fn terminal_runtime_snapshot(
    runtime: &TerminalRuntime,
    stable_viewport: PaneStableViewport,
) -> TerminalRenderSnapshot {
    let scrollback_offset = stable_viewport.scrollback_offset(runtime.terminal());
    TerminalRenderSnapshot::from_terminal_viewport(runtime.terminal(), scrollback_offset)
        .with_cursor_color(runtime.cursor_color_override())
}

#[derive(Clone, Copy)]
struct NativeFrameContentPlacement {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl NativeFrameContentPlacement {
    fn is_full_frame(self, geometry: RenderGeometry) -> bool {
        self.x == 0
            && self.y == 0
            && self.width == geometry.target_width
            && self.height == geometry.target_height
    }
}

#[allow(clippy::too_many_arguments)]
fn render_framebuffer_with_state(
    renderer: &PixelRenderer,
    snapshot: &TerminalRenderSnapshot,
    scrollbar: Option<ScrollbackScrollbar>,
    pending_frame_damage: &mut Vec<DamageRegion>,
    frame_needs_full_repaint: &mut bool,
    frame: &mut [u8],
    geometry: RenderGeometry,
    damage_row_offset: u16,
    placement: NativeFrameContentPlacement,
    background: [u8; 4],
) -> FrameRenderMode {
    if !placement.is_full_frame(geometry) {
        render_aligned_framebuffer(
            renderer,
            snapshot,
            scrollbar,
            frame,
            geometry,
            placement,
            damage_row_offset,
            background,
        );
        pending_frame_damage.clear();
        *frame_needs_full_repaint = false;
        return FrameRenderMode::Full;
    }

    if *frame_needs_full_repaint || pending_frame_damage.is_empty() {
        renderer.render(
            snapshot,
            frame,
            geometry.target_width,
            geometry.target_height,
            geometry.cell_width,
            geometry.cell_height,
        );
        paint_frame_border(frame, geometry, geometry.frame_border_color);
        if let Some(scrollbar) = scrollbar {
            renderer.render_scrollbar(scrollbar, frame, geometry);
            redraw_frame_ui_rows(renderer, snapshot, frame, geometry, damage_row_offset);
        }
        paint_frame_separator(frame, geometry, geometry.frame_separator);
        pending_frame_damage.clear();
        *frame_needs_full_repaint = false;
        return FrameRenderMode::Full;
    }

    let damage = offset_damage_regions(std::mem::take(pending_frame_damage), damage_row_offset);
    renderer.render_damage(snapshot, &damage, frame, geometry);
    if let Some(scrollbar) = scrollbar {
        renderer.render_scrollbar(scrollbar, frame, geometry);
        redraw_frame_ui_rows(renderer, snapshot, frame, geometry, damage_row_offset);
    }
    paint_frame_separator(frame, geometry, geometry.frame_separator);
    FrameRenderMode::Damage
}

#[allow(clippy::too_many_arguments)]
fn render_aligned_framebuffer(
    renderer: &PixelRenderer,
    snapshot: &TerminalRenderSnapshot,
    scrollbar: Option<ScrollbackScrollbar>,
    frame: &mut [u8],
    geometry: RenderGeometry,
    placement: NativeFrameContentPlacement,
    damage_row_offset: u16,
    background: [u8; 4],
) {
    fill_framebuffer(frame, background);
    paint_frame_border(frame, geometry, geometry.frame_border_color);
    if placement.width == 0 || placement.height == 0 {
        return;
    }

    let Some(content_len) = usize::try_from(
        u64::from(placement.width)
            .saturating_mul(u64::from(placement.height))
            .saturating_mul(4),
    )
    .ok() else {
        return;
    };
    let mut content = vec![0; content_len];
    let content_geometry = RenderGeometry::new(
        placement.width,
        placement.height,
        geometry.cell_width,
        geometry.cell_height,
    );
    renderer.render(
        snapshot,
        &mut content,
        placement.width,
        placement.height,
        geometry.cell_width,
        geometry.cell_height,
    );
    if let Some(scrollbar) = scrollbar {
        renderer.render_scrollbar(scrollbar, &mut content, content_geometry);
        redraw_frame_ui_rows(
            renderer,
            snapshot,
            &mut content,
            content_geometry,
            damage_row_offset,
        );
    }
    blit_framebuffer(
        &content,
        placement.width,
        placement.height,
        frame,
        geometry.target_width,
        geometry.target_height,
        placement.x,
        placement.y,
    );
    paint_frame_border(frame, geometry, geometry.frame_border_color);
    paint_frame_separator(frame, geometry, geometry.frame_separator);
}

fn fill_framebuffer(frame: &mut [u8], color: [u8; 4]) {
    for pixel in frame.chunks_exact_mut(4) {
        pixel.copy_from_slice(&color);
    }
}

fn paint_frame_border(
    frame: &mut [u8],
    geometry: RenderGeometry,
    color: Option<[u8; 4]>,
) {
    let Some(color) = color else {
        return;
    };
    if geometry.target_width == 0 || geometry.target_height == 0 {
        return;
    }
    let width = usize::try_from(geometry.target_width).unwrap_or(0);
    let height = usize::try_from(geometry.target_height).unwrap_or(0);
    if width == 0 || height == 0 {
        return;
    }
    let set_pixel = |frame: &mut [u8], x: usize, y: usize| {
        let index = (y * width + x) * 4;
        if let Some(pixel) = frame.get_mut(index..index.saturating_add(4)) {
            pixel.copy_from_slice(&color);
        }
    };
    if width < 3 || height < 3 {
        for x in 0..width {
            set_pixel(frame, x, 0);
            set_pixel(frame, x, height.saturating_sub(1));
        }
        for y in 1..height.saturating_sub(1) {
            set_pixel(frame, 0, y);
            set_pixel(frame, width.saturating_sub(1), y);
        }
        return;
    }

    // Leave the four outermost pixels untouched so the 1px chrome reads as a
    // subtle rounded frame instead of a hard square corner.  The framebuffer
    // is already filled with the active background before this overlay runs.
    for x in 1..width.saturating_sub(1) {
        set_pixel(frame, x, 0);
        set_pixel(frame, x, height.saturating_sub(1));
    }
    for y in 1..height.saturating_sub(1) {
        set_pixel(frame, 0, y);
        set_pixel(frame, width.saturating_sub(1), y);
    }
}

fn paint_frame_separator(
    frame: &mut [u8],
    geometry: RenderGeometry,
    separator: Option<(u32, [u8; 4])>,
) {
    let Some((y, color)) = separator else {
        return;
    };
    if y >= geometry.target_height || geometry.target_width <= 2 {
        return;
    }
    let width = usize::try_from(geometry.target_width).unwrap_or(0);
    let y = usize::try_from(y).unwrap_or(usize::MAX);
    let start = (y * width + 1) * 4;
    let length = usize::try_from(geometry.target_width.saturating_sub(2)).unwrap_or(0) * 4;
    if let Some(row) = frame.get_mut(start..start.saturating_add(length)) {
        for pixel in row.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn blit_framebuffer(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    target: &mut [u8],
    target_width: u32,
    target_height: u32,
    target_x: u32,
    target_y: u32,
) {
    let copy_width = source_width.min(target_width.saturating_sub(target_x));
    let copy_height = source_height.min(target_height.saturating_sub(target_y));
    let (
        Ok(source_width),
        Ok(copy_width),
        Ok(copy_height),
        Ok(target_width),
        Ok(target_x),
        Ok(target_y),
    ) = (
        usize::try_from(source_width),
        usize::try_from(copy_width),
        usize::try_from(copy_height),
        usize::try_from(target_width),
        usize::try_from(target_x),
        usize::try_from(target_y),
    )
    else {
        return;
    };
    for row in 0..copy_height {
        let source_start = row.saturating_mul(source_width).saturating_mul(4);
        let source_end = source_start.saturating_add(copy_width.saturating_mul(4));
        let target_start = target_y
            .saturating_add(row)
            .saturating_mul(target_width)
            .saturating_add(target_x)
            .saturating_mul(4);
        let target_end = target_start.saturating_add(copy_width.saturating_mul(4));
        let Some(source_row) = source.get(source_start..source_end) else {
            return;
        };
        let Some(target_row) = target.get_mut(target_start..target_end) else {
            return;
        };
        target_row.copy_from_slice(source_row);
    }
}

fn redraw_frame_ui_rows(
    renderer: &PixelRenderer,
    snapshot: &TerminalRenderSnapshot,
    frame: &mut [u8],
    geometry: RenderGeometry,
    rows: u16,
) {
    if rows == 0 || geometry.cell_width == 0 {
        return;
    }

    let columns =
        u16::try_from((geometry.target_width / geometry.cell_width).min(u32::from(u16::MAX)))
            .unwrap_or(u16::MAX);
    renderer.render_damage(
        snapshot,
        &[DamageRegion::new(0, 0, columns, rows)],
        frame,
        geometry,
    );
}

fn offset_damage_regions(damage: Vec<DamageRegion>, row_offset: u16) -> Vec<DamageRegion> {
    if row_offset == 0 {
        return damage;
    }

    damage
        .into_iter()
        .map(|region| {
            DamageRegion::new(
                region.x,
                region.y.saturating_add(row_offset),
                region.width,
                region.height,
            )
        })
        .collect()
}

impl NativeWindowApp {
    #[cfg(test)]
    fn new(frame_limit: Option<u64>) -> Self {
        let mut app = Self::new_with_visual_defaults(frame_limit);
        app.reset_test_geometry_to_legacy();
        app
    }

    #[cfg(test)]
    fn new_with_visual_defaults(frame_limit: Option<u64>) -> Self {
        *Self::new_with_workspace_class_position_and_osc52_policy(
            frame_limit,
            Osc52Policy::default(),
            PtyCommand::default_shell(),
            None,
            None,
            None,
        )
    }

    #[cfg(test)]
    fn new_with_osc52_policy(frame_limit: Option<u64>, osc52_policy: Osc52Policy) -> Self {
        let mut app = *Self::new_with_command_and_osc52_policy(
            frame_limit,
            osc52_policy,
            PtyCommand::default_shell(),
        );
        app.reset_test_geometry_to_legacy();
        app
    }

    #[cfg(test)]
    fn new_with_command(frame_limit: Option<u64>, startup_command: PtyCommand) -> Self {
        let mut app = *Self::new_with_command_and_osc52_policy(
            frame_limit,
            Osc52Policy::default(),
            startup_command,
        );
        app.reset_test_geometry_to_legacy();
        app
    }

    #[allow(clippy::too_many_lines, unused_mut)]
    fn new_with_command_and_osc52_policy(
        frame_limit: Option<u64>,
        osc52_policy: Osc52Policy,
        startup_command: PtyCommand,
    ) -> Box<Self> {
        let mut app = Self::new_with_workspace_class_position_and_osc52_policy(
            frame_limit,
            osc52_policy,
            startup_command,
            None,
            None,
            None,
        );
        #[cfg(test)]
        app.reset_test_geometry_to_legacy();
        app
    }

    #[cfg(test)]
    fn new_with_workspace(
        frame_limit: Option<u64>,
        startup_command: PtyCommand,
        startup_workspace: Option<&str>,
    ) -> Self {
        Self::new_with_workspace_and_osc52_policy(
            frame_limit,
            Osc52Policy::default(),
            startup_command,
            startup_workspace,
        )
    }

    #[cfg(test)]
    fn new_with_window_position(
        frame_limit: Option<u64>,
        startup_command: PtyCommand,
        initial_window_position: Option<WindowPosition>,
    ) -> Self {
        let mut app = *Self::new_with_workspace_class_position_and_osc52_policy(
            frame_limit,
            Osc52Policy::default(),
            startup_command,
            None,
            None,
            initial_window_position,
        );
        app.reset_test_geometry_to_legacy();
        app
    }

    #[cfg(test)]
    fn new_with_window_class(
        frame_limit: Option<u64>,
        startup_command: PtyCommand,
        initial_window_class: Option<String>,
    ) -> Self {
        let mut app = *Self::new_with_workspace_class_position_and_osc52_policy(
            frame_limit,
            Osc52Policy::default(),
            startup_command,
            None,
            initial_window_class,
            None,
        );
        app.reset_test_geometry_to_legacy();
        app
    }

    #[allow(clippy::too_many_lines)]
    #[cfg(test)]
    fn new_with_workspace_and_osc52_policy(
        frame_limit: Option<u64>,
        osc52_policy: Osc52Policy,
        startup_command: PtyCommand,
        startup_workspace: Option<&str>,
    ) -> Self {
        let mut app = *Self::new_with_workspace_class_position_and_osc52_policy(
            frame_limit,
            osc52_policy,
            startup_command,
            startup_workspace,
            None,
            None,
        );
        app.reset_test_geometry_to_legacy();
        app
    }

    #[cfg(test)]
    fn reset_test_geometry_to_legacy(&mut self) {
        self.legacy_test_geometry = true;
        self.window_padding = NativeWindowPadding::default();
        self.frame_width = FRAME_WIDTH;
        self.frame_height = FRAME_HEIGHT;
        self.modern_tab_bar_brand = false;
        // Keep the synthetic test window on the pre-modern tab-bar geometry.
        // Production Windows windows intentionally default to integrated title
        // buttons, but legacy layout tests must not inherit those right-edge
        // controls unless they opt in through an explicit override.
        self.window_decorations.integrated_buttons = false;
        self.window_frame
            .set_size(PhysicalSize::new(FRAME_WIDTH, FRAME_HEIGHT));
        self.font_size = DEFAULT_FONT_SIZE;
        self.tab_max_width = DEFAULT_TAB_MAX_WIDTH;
        self.tab_min_width = DEFAULT_TAB_MIN_WIDTH;
        self.foreground_color = LEGACY_TEST_FOREGROUND_COLOR;
        self.background_color = LEGACY_TEST_BACKGROUND_COLOR;
        self.selection_bg_color = None;
        self.cursor_bg_color = LEGACY_TEST_CURSOR_BG_COLOR;
        self.cursor_fg_color = LEGACY_TEST_CURSOR_FG_COLOR;
        self.tab_bar_background_color = None;
        self.tab_bar_active_tab_colors = NativeTabBarItemColors::default();
        self.tab_bar_inactive_tab_colors = NativeTabBarItemColors::default();
        self.tab_bar_inactive_tab_hover_colors = NativeTabBarItemColors::default();
        self.tab_bar_new_tab_colors = NativeTabBarItemColors::default();
        self.tab_bar_new_tab_hover_colors = NativeTabBarItemColors::default();
        self.renderer.set_default_foreground(color_to_rgba(
            self.foreground_color,
            DEFAULT_RENDER_FOREGROUND_RGBA,
        ));
        self.renderer.set_default_background(color_to_rgba(
            self.background_color,
            DEFAULT_RENDER_BACKGROUND_RGBA,
        ));
        self.renderer.set_default_cursor_color(color_to_rgba(
            self.cursor_bg_color,
            DEFAULT_RENDER_FOREGROUND_RGBA,
        ));
        self.renderer
            .set_default_cursor_foreground(self.cursor_fg_color.map(|color| {
                color_to_rgba(color, DEFAULT_RENDER_FOREGROUND_RGBA)
            }));
    }

}

impl NativeWindowApp {
    #[allow(clippy::too_many_lines)]
    fn new_with_workspace_class_position_and_osc52_policy(
        frame_limit: Option<u64>,
        osc52_policy: Osc52Policy,
        startup_command: PtyCommand,
        startup_workspace: Option<&str>,
        initial_window_class: Option<String>,
        initial_window_position: Option<WindowPosition>,
    ) -> Box<Self> {
        let runtime = TerminalRuntime::new(TerminalSize::new(TERMINAL_COLUMNS, TERMINAL_ROWS));
        let snapshot = terminal_runtime_snapshot(&runtime, PaneStableViewport::default());
        let startup_uses_default_shell = pty_command_matches_default_shell(&startup_command);
        let startup_workspace_was_explicit = startup_workspace.is_some();
        let app_shell = app_shell_from_pty_command(&startup_command, startup_workspace);
        let default_config = Arc::new(NativeConfigSnapshot::default());

        Box::write(
            Box::new_uninit(),
            Self {
                app_window_id: rssh_core::WindowId::new(1),
                tab_transfer_targets: Vec::new(),
                window_close_requested: false,
                window_drag_requested: false,
                activate_window_request: None,
                window_hide_requested: false,
                application_hide_requested: false,
                application_quit_requested: false,
                window_level: NativeWindowLevel::Normal,
                full_screen: false,
                window_maximized: false,
                font_size_scale: DEFAULT_FONT_SIZE_SCALE,
                debug_overlay_active: false,
                debug_key_event_logs: Vec::new(),
                unknown_escape_sequence_warnings: Vec::new(),
                missing_glyph_warnings: Vec::new(),
                missing_glyph_warning_codepoints: HashSet::new(),
                char_select: None,
                char_select_recently_used: Vec::new(),
                char_select_recently_used_sequence: 0,
                char_select_recently_used_path: None,
                window_focused: false,
                mouse_click_may_focus_window: false,
                window: None,
                bootstrap_surface: None,
                bootstrap_frame: Vec::new(),
                // Existing window launches retain their synchronous GPU path;
                // the SSH GUI entry point overrides this with the requested
                // hybrid/cpu mode before the event loop starts.
                renderer_mode: RendererMode::Gpu,
                presentation_owner: PresentationOwner::Bootstrap,
                deferred_gpu_generation: 0,
                startup: NativeStartupState {
                    mode: NativeStartupMode::Normal,
                    diagnostic_gpu_backend: None,
                },
                transport_start_requested: false,
                #[cfg(feature = "ssh")]
                ssh_host_key_prompts: HashMap::new(),
                #[cfg(feature = "ssh")]
                ssh_secret_prompts: HashMap::new(),
                ssh_connection_states: HashMap::new(),
                gpu_owners: crate::window_gpu::WindowGpuOwners::default(),
                renderer: {
                    let mut renderer = GpuFramePlanner::new(PixelRenderer::new());
                    renderer.set_reverse_video_cursor_min_contrast(Some(
                        DEFAULT_REVERSE_VIDEO_CURSOR_MIN_CONTRAST.as_f64(),
                    ));
                    renderer
                },
                configured_dpi: None,
                dpi_by_screen: BTreeMap::new(),
                detected_window_dpi: DEFAULT_WINDOW_DPI,
                window_dpi: DEFAULT_WINDOW_DPI,
                runtime: ActiveWindowRuntime::new(runtime),
                snapshot,
                window_title: DEFAULT_WINDOW_TITLE.to_owned(),
                modern_tab_bar_brand: true,
                frame_width: MODERN_FRAME_WIDTH + MODERN_WINDOW_PADDING_HORIZONTAL_PIXELS,
                frame_height: MODERN_FRAME_HEIGHT + MODERN_WINDOW_PADDING_VERTICAL_PIXELS,
                window_frame: NativeWindowFrame {
                    x: 0,
                    y: 0,
                    width: MODERN_FRAME_WIDTH + MODERN_WINDOW_PADDING_HORIZONTAL_PIXELS,
                    height: MODERN_FRAME_HEIGHT + MODERN_WINDOW_PADDING_VERTICAL_PIXELS,
                },
                frame_limit: frame_limit.or_else(test_ssh_gui_frame_limit),
                initial_window_class,
                initial_window_position,
                startup_command,
                startup_uses_default_shell,
                startup_workspace_was_explicit,
                rendered_frames: 0,
                animation_started_at: Instant::now(),
                event_proxy: None,
                reload_request_sender: None,
                session: None,
                session_process_id: None,
                session_tty_name: None,
                writer: None,
                ssh_writer_senders: HashMap::new(),
                ssh_writer_cancellations: HashMap::new(),
                #[cfg(feature = "ssh")]
                ssh_connection_cancellations: HashMap::new(),
                session_log: None,
                reader_thread: None,
                writer_thread: None,
                interaction_state: NativeWindowInteractionState {
                    active_runtime_generation: 0,
                    active_runtime_transport: None,
                modifiers: ModifiersState::empty(),
                left_alt_pressed: false,
                right_alt_pressed: false,
                active_ui: PaneUiState::default(),
                mouse_pixel_position: None,
                rendered_tab_bar_layout: RefCell::new(None),
                rendered_tab_bar_generation: Cell::new(0),
                mouse_position: None,
                current_mouse_wheel_delta: None,
                mouse_cursor_visible: true,
                mouse_cursor_icon: CursorIcon::Default,
                active_mouse_button: None,
                last_mouse_info: None,
                selection: None,
                selecting: false,
                scrollbar_dragging: false,
                split_resize_dragging: None,
                tab_bar_drag: None,
                tab_bar_scroll_position: 0,
                ui_left_release_pending: false,
                pressed_pane_close_button: None,
                pane_inspection: None,
                ui_key_release_pending: None,
                last_mouse_assignment_click: None,
                last_left_click: None,
                command_palette: None,
                command_palette_frecency: HashMap::new(),
                command_palette_frecency_sequence: 0,
                command_palette_frecency_path: None,
                pane_select: None,
                pending_window_positions: HashMap::new(),
                tab_navigator: None,
                prompt_input_line: None,
                input_selector: None,
                confirmation: None,
                deferred_wheel_context: None,
                close_confirmation: None,
                key_table_stack: Vec::new(),
                visual_bell_started_at: HashMap::new(),
                #[cfg(test)]
                visual_bell_test_now: None,
                ime_preedit: None,
                last_ime_cursor_area: Cell::new(None),
                dead_key_active: false,
                dead_key_text: None,
                leader_active_since: None,
                base_config_overrides: Arc::clone(&default_config),
                base_config_generation: 0,
                base_config_source: None,
                window_config_overrides: None,
                #[cfg(test)]
                base_config_apply_observer: None,
                #[cfg(test)]
                pty_spawn_observer: None,
                config_overrides: default_config,
                host_state: NativeWindowHostState {
                latest_notification: None,
                left_status: String::new(),
                right_status: String::new(),
                lua_tab_title: None,
                lua_window_title: None,
                lua_update_status: None,
                lua_update_status_config_overrides: None,
                lua_bell: None,
                lua_focus_changed: None,
                lua_resized: None,
                lua_config_reloaded: None,
                lua_user_var_changed: None,
                lua_open_uri: None,
                lua_new_tab_button_click: None,
                lua_command_palette_entries: Vec::new(),
                lua_emit_event_handlers: BTreeMap::new(),
                last_redraw_request_at: None,
                last_animation_redraw_request_at: None,
                last_status_update_at: None,
                #[cfg(test)]
                legacy_test_geometry: false,
                cursor_blink_visible: true,
                cursor_blink_opacity_alpha: u8::MAX,
                last_cursor_blink_at: None,
                text_blink_opacity_alpha: u8::MAX,
                rapid_text_blink_opacity_alpha: u8::MAX,
                last_text_blink_at: None,
                last_rapid_text_blink_at: None,
                osc52_policy,
                clipboard_writer: Box::new(write_window_clipboard_text),
                clipboard_reader: Box::new(read_window_clipboard_text),
                primary_selection_writer: Box::new(write_window_primary_selection_text),
                primary_selection_reader: Box::new(read_window_primary_selection_text),
                hyperlink_opener: Box::new(open_window_hyperlink),
                open_uri_handler: Box::new(dispatch_window_open_uri),
                new_tab_button_click_handler: Box::new(dispatch_window_new_tab_button_click),
                tab_title_formatter: Box::new(format_tab_title),
                window_title_formatter: Box::new(format_window_title),
                #[cfg(test)]
                applied_window_titles: RefCell::new(None),
                applied_window_title: RefCell::new(None),
                update_status_handler: Box::new(dispatch_window_update_status),
                update_right_status_handler: Box::new(dispatch_window_update_right_status),
                notification_handler: Box::new(show_window_notification),
                audible_bell_handler: Box::new(ring_window_audible_bell),
                bell_handler: Box::new(dispatch_window_bell),
                focus_change_handler: Box::new(dispatch_window_focus_change),
                resize_handler: Box::new(dispatch_window_resize),
                user_var_change_handler: Box::new(dispatch_window_user_var_change),
                config_reloaded_handler: Box::new(dispatch_window_config_reloaded),
                command_palette_augmenter: Box::new(dispatch_command_palette_augment),
                prompt_input_line_handler: Box::new(dispatch_prompt_input_line),
                input_selector_handler: Box::new(dispatch_input_selector),
                confirmation_handler: Box::new(dispatch_confirmation),
                emit_event_handler: Box::new(dispatch_emit_event),
                metrics: WindowMetrics::new(),
                pending_frame_damage: Vec::new(),
                frame_needs_full_repaint: true,
                app_shell,
                closed_tab_history: Arc::new(Mutex::new(ClosedTabHistory::new(25))),
                pane_runtimes: HashMap::new(),
                pane_bell_counts: HashMap::new(),
                applied_config: Arc::new(NativeAppliedConfig::default()),
                },
                },
            },
        )
    }

    #[cfg(test)]
    fn startup_command(&self) -> &PtyCommand {
        &self.startup_command
    }

    fn set_renderer_mode(&mut self, mode: RendererMode) {
        self.renderer_mode = mode;
        self.presentation_owner = if mode == RendererMode::Gpu {
            PresentationOwner::GpuInitializing
        } else {
            PresentationOwner::Bootstrap
        };
    }

    fn set_benchmark_startup(&mut self, enabled: bool) {
        self.startup.mode = if enabled {
            NativeStartupMode::Benchmark
        } else {
            NativeStartupMode::Normal
        };
    }

    fn set_initial_pane_launch(&mut self, launch: PaneLaunch) {
        if self.session.is_some() {
            return;
        }
        let workspace = self.app_shell.active_workspace().name().to_owned();
        self.app_shell = AppShell::new_with_workspace_name(launch, workspace);
        self.ssh_connection_states.clear();
        if matches!(
            self.app_shell.active_pane().launch().domain(),
            PaneLaunchDomain::Ssh(_)
        ) {
            self.ssh_connection_states.insert(
                self.app_shell.active_pane_id(),
                ConnectionState::Pending,
            );
            self.metrics.mark_connection_state(ConnectionState::Pending);
        }
        self.startup_uses_default_shell = false;
        self.startup_workspace_was_explicit = true;
        self.transport_start_requested = false;
        self.frame_needs_full_repaint = true;
        self.pending_frame_damage.clear();
    }

    #[cfg(test)]
    fn initial_window_position(&self) -> Option<WindowPosition> {
        self.initial_window_position.clone()
    }

    #[cfg(test)]
    fn initial_window_class(&self) -> Option<&str> {
        self.initial_window_class.as_deref()
    }

    #[cfg(test)]
    fn active_workspace_id(&self) -> rssh_core::WorkspaceId {
        self.app_shell.active_workspace_id()
    }

    #[cfg(test)]
    fn active_tab_id(&self) -> rssh_core::TabId {
        self.app_shell.active_tab_id()
    }

    #[cfg(test)]
    fn active_pane_id(&self) -> rssh_core::PaneId {
        self.app_shell.active_pane_id()
    }

    #[cfg(test)]
    fn app_window_id_for_test(&self) -> rssh_core::WindowId {
        self.app_window_id
    }

    #[cfg(test)]
    fn active_key_table_for_test(&self) -> Option<&str> {
        self.key_table_stack
            .last()
            .map(|activation| activation.name.as_str())
    }

    #[cfg(test)]
    fn window_close_requested_for_test(&self) -> bool {
        self.window_close_requested
    }

    #[cfg(test)]
    fn window_drag_requested_for_test(&self) -> bool {
        self.window_drag_requested
    }

    #[cfg(test)]
    fn take_activate_window_request_for_test(&mut self) -> Option<WindowActivateWindowRequest> {
        self.take_activate_window_request()
    }

    #[cfg(test)]
    fn window_hide_requested_for_test(&self) -> bool {
        self.window_hide_requested
    }

    #[cfg(test)]
    fn application_quit_requested_for_test(&self) -> bool {
        self.application_quit_requested
    }

    #[cfg(test)]
    fn font_size_scale_for_test(&self) -> f64 {
        self.font_size_scale
    }

    #[cfg(test)]
    fn frame_size_for_test(&self) -> (u32, u32) {
        (self.frame_width, self.frame_height)
    }

    #[cfg(test)]
    fn debug_overlay_active_for_test(&self) -> bool {
        self.debug_overlay_active
    }

    #[cfg(test)]
    fn debug_key_event_logs_for_test(&self) -> &[String] {
        &self.debug_key_event_logs
    }

    #[cfg(test)]
    fn unknown_escape_sequence_warnings_for_test(&self) -> &[String] {
        &self.unknown_escape_sequence_warnings
    }

    #[cfg(test)]
    fn missing_glyph_warnings_for_test(&self) -> &[String] {
        &self.missing_glyph_warnings
    }

    #[cfg(test)]
    fn char_select_active_for_test(&self) -> bool {
        self.char_select.is_some()
    }

    #[cfg(test)]
    fn char_select_for_test(&self) -> Option<&WindowCharSelect> {
        self.char_select.as_ref()
    }

    #[cfg(test)]
    fn full_screen_for_test(&self) -> bool {
        self.full_screen
    }

    #[cfg(test)]
    fn window_maximized_for_test(&self) -> bool {
        self.window_maximized
    }

    #[cfg(test)]
    fn window_level_for_test(&self) -> NativeWindowLevel {
        self.window_level
    }

    fn dispatch_app_action(&mut self, action: AppAction) -> Result<(), AppShellError> {
        match action {
            AppAction::CloseTab {
                tab,
                switch_to_last_active,
            } => return self.dispatch_close_tab_action(tab, switch_to_last_active),
            AppAction::CloseTabWithSelection { tab, selection } => {
                return self.dispatch_close_tab_with_selection(tab, selection);
            }
            AppAction::ClosePane { pane } => return self.dispatch_close_pane_action(pane),
            _ => {}
        }

        let closes_source_after_move = matches!(
            &action,
            AppAction::MoveTabToNewWindow { tab }
                if *tab == self.app_shell.active_tab_id()
                    && self.app_shell.active_workspace().tabs().len() == 1
        );
        let action = self.apply_default_prog_to_action(action);
        self.dispatch_shell_action(action)?;
        if closes_source_after_move {
            self.request_window_close();
        }
        Ok(())
    }

    fn apply_default_prog_to_action(&self, action: AppAction) -> AppAction {
        match action {
            AppAction::NewTab { launch: None } => AppAction::NewTab {
                launch: self.implicit_child_launch(self.app_shell.active_pane_id()),
            },
            AppAction::SpawnWindow { launch: None } => AppAction::SpawnWindow {
                launch: self.implicit_child_launch(self.app_shell.active_pane_id()),
            },
            AppAction::SplitPane {
                pane,
                direction,
                launch: None,
            } => AppAction::SplitPane {
                pane,
                direction,
                launch: self.implicit_child_launch(pane),
            },
            AppAction::SplitPaneWithSize {
                pane,
                direction,
                launch: None,
                source_size_delta,
            } => AppAction::SplitPaneWithSize {
                pane,
                direction,
                launch: self.implicit_child_launch(pane),
                source_size_delta,
            },
            AppAction::NewWorkspace { name, launch: None } => AppAction::NewWorkspace {
                name,
                launch: self.default_prog_launch(),
            },
            AppAction::SwitchToWorkspace { name, launch: None } => AppAction::SwitchToWorkspace {
                name,
                launch: self.default_prog_launch(),
            },
            action => action,
        }
    }

    fn implicit_child_launch(&self, source_pane: rssh_core::PaneId) -> Option<PaneLaunch> {
        let source_launch = self
            .app_shell
            .workspaces()
            .iter()
            .flat_map(rssh_core::app_shell::Workspace::tabs)
            .flat_map(rssh_core::app_shell::Tab::panes)
            .find(|pane| pane.id() == source_pane)
            .map(rssh_core::app_shell::Pane::launch);
        if let Some(source_launch) = source_launch
            && matches!(source_launch.domain(), PaneLaunchDomain::Ssh(_))
        {
            return Some(source_launch.for_child_pane());
        }
        self.default_prog_launch()
    }

    fn default_prog_launch(&self) -> Option<PaneLaunch> {
        let (program, args) = self.default_prog.as_ref()?.split_first()?;
        if program.is_empty() {
            return None;
        }

        let mut launch = PaneLaunch::local(program.clone()).with_args(args.iter().cloned());
        if let Some(cwd) = self
            .app_shell
            .active_pane()
            .launch()
            .cwd()
            .or(self.default_cwd.as_deref())
        {
            launch = launch.with_cwd(cwd.to_owned());
        }
        Some(launch)
    }

    fn apply_startup_default_prog_before_spawn(&mut self) {
        if self.session.is_some()
            || !self.startup_uses_default_shell
            || !self.app_shell_is_initial_startup_shape()
        {
            return;
        }
        let Some(command) = startup_command_from_default_prog(
            self.default_prog.as_deref(),
            self.startup_command.cwd(),
        ) else {
            return;
        };
        let startup_workspace = self.app_shell.active_workspace().name().to_owned();
        self.startup_command = command;
        self.app_shell =
            app_shell_from_pty_command(&self.startup_command, Some(&startup_workspace));
    }

    fn apply_startup_default_ssh_auth_sock_before_spawn(
        &mut self,
        previous_default_ssh_auth_sock: Option<&str>,
    ) {
        if self.session.is_some() || !self.app_shell_is_initial_startup_shape() {
            return;
        }
        if matches!(
            self.app_shell.active_pane().launch().domain(),
            PaneLaunchDomain::Ssh(_)
        ) {
            return;
        }
        let mut command = self.startup_command.clone();
        if let Some(previous_default_ssh_auth_sock) = previous_default_ssh_auth_sock
            && command.env_value(SSH_AUTH_SOCK_ENV) == Some(previous_default_ssh_auth_sock)
        {
            command = command.without_env(SSH_AUTH_SOCK_ENV);
        }
        if self.mux_enable_ssh_agent
            && let Some(default_ssh_auth_sock) = self
                .default_ssh_auth_sock
                .as_deref()
                .filter(|ssh_auth_sock| !ssh_auth_sock.is_empty())
        {
            command = command.with_env(SSH_AUTH_SOCK_ENV, default_ssh_auth_sock);
        }
        let startup_workspace = self.app_shell.active_workspace().name().to_owned();
        self.startup_command = command;
        self.app_shell =
            app_shell_from_pty_command(&self.startup_command, Some(&startup_workspace));
    }

    fn apply_startup_default_workspace_before_spawn(&mut self) {
        if self.session.is_some()
            || self.startup_workspace_was_explicit
            || !self.app_shell_is_initial_startup_shape()
            || self.app_shell.active_workspace().name() != DEFAULT_WORKSPACE_NAME
            || self.default_workspace == DEFAULT_WORKSPACE_NAME
        {
            return;
        }

        let workspace = self.app_shell.active_workspace_id();
        let name = self.default_workspace.clone();
        let _ = self.app_shell.apply_action(AppAction::RenameWorkspace {
            workspace,
            name,
        });
    }

    fn app_shell_is_initial_startup_shape(&self) -> bool {
        self.app_shell.workspaces().len() == 1
            && self.app_shell.active_workspace().tabs().len() == 1
            && self.app_shell.active_tab().panes().len() == 1
            && self.app_shell.pending_windows().is_empty()
    }

    fn dispatch_shell_action(&mut self, action: AppAction) -> Result<(), AppShellError> {
        if self.should_block_zoomed_pane_direction_switch(&action) {
            return Ok(());
        }

        let moves_inspected_pane = self
            .pane_inspection
            .is_some_and(|pane_id| Self::app_action_moves_pane_ownership(&action, pane_id));
        let previous_active_pane = self.app_shell.active_pane_id();
        let previous_shell = self.app_shell.clone();
        let pointer_transient = self.pointer_transient_state();
        let preserve_split_resize_drag = matches!(&action, AppAction::ResizePane { .. });
        self.end_pointer_modes_for_pane_change();
        let previous_runtime = self.take_active_runtime();
        if let Err(error) = self.app_shell.apply_action(action) {
            self.app_shell = previous_shell;
            self.install_active_runtime(previous_runtime);
            self.restore_pointer_transient_state(pointer_transient);
            self.apply_window_title();
            return Err(error);
        }
        if self.app_shell.active_pane_id() == previous_active_pane && preserve_split_resize_drag {
            self.restore_split_resize_pointer_state(pointer_transient);
        }
        self.sync_pane_runtimes(previous_active_pane, previous_runtime);
        if moves_inspected_pane {
            self.cancel_pane_inspection();
        } else {
            self.clear_pane_inspection_if_invalid();
        }
        self.apply_window_title();
        Ok(())
    }

    fn app_action_moves_pane_ownership(action: &AppAction, pane_id: rssh_core::PaneId) -> bool {
        match action {
            AppAction::MovePaneToNewTab { pane } | AppAction::MovePaneToNewWindow { pane } => {
                *pane == pane_id
            }
            AppAction::Multiple { actions } => actions
                .iter()
                .any(|action| Self::app_action_moves_pane_ownership(action, pane_id)),
            _ => false,
        }
    }

    fn should_block_zoomed_pane_direction_switch(&self, action: &AppAction) -> bool {
        matches!(action, AppAction::ActivatePaneDirection { .. })
            && !self.unzoom_on_switch_pane
            && self.app_shell.active_tab().zoomed_pane_id().is_some()
    }

    fn request_window_close(&mut self) {
        self.window_close_requested = true;
    }

    fn handle_window_close_requested(&mut self) {
        if self.window_close_confirmation == NativeWindowCloseConfirmation::NeverPrompt {
            self.request_window_close();
            return;
        }

        self.request_close_confirmation_or_close(WindowCloseTarget::Window);
    }

    fn start_window_drag(&mut self) {
        self.window_drag_requested = true;
        if let Some(window) = &self.window
            && let Err(error) = window.drag_window()
        {
            eprintln!("start window drag failed: {error}");
        }
    }

    fn request_activate_window(&mut self, index: usize) {
        self.activate_window_request = Some(WindowActivateWindowRequest::Index(index));
    }

    fn request_activate_window_relative(&mut self, offset: isize, wrap: bool) {
        self.activate_window_request = Some(WindowActivateWindowRequest::Relative { offset, wrap });
    }

    fn take_activate_window_request(&mut self) -> Option<WindowActivateWindowRequest> {
        self.activate_window_request.take()
    }

    fn request_application_quit(&mut self) {
        self.application_quit_requested = true;
    }

    fn hide_window(&mut self) {
        self.window_hide_requested = true;
        if let Some(window) = &self.window {
            window.set_minimized(true);
        }
    }

    fn show_window(&mut self) {
        self.window_hide_requested = false;
        if let Some(window) = &self.window {
            window.set_visible(true);
            window.set_minimized(false);
            window.focus_window();
        }
    }

    fn hide_application(&mut self) {
        self.application_hide_requested = true;
    }

    fn take_application_hide_request(&mut self) -> bool {
        std::mem::take(&mut self.application_hide_requested)
    }

    fn toggle_window_maximized(&mut self) {
        self.window_maximized = !self.window_maximized;
        if let Some(window) = &self.window {
            window.set_maximized(self.window_maximized);
        }
    }

    fn adjust_font_size(&mut self, action: WindowFontSizeAction) {
        let terminal_size = self.runtime.terminal().grid().size();
        let window_size = PhysicalSize::new(self.window_frame.width, self.window_frame.height);
        match action {
            WindowFontSizeAction::Decrease => {
                self.font_size_scale /= FONT_SIZE_STEP;
            }
            WindowFontSizeAction::Increase => {
                self.font_size_scale *= FONT_SIZE_STEP;
            }
            WindowFontSizeAction::Reset => {
                self.font_size_scale = DEFAULT_FONT_SIZE_SCALE;
            }
        }
        self.apply_window_resize_increments();
        let requested_size = if self.adjust_window_size_when_changing_font_size {
            self.frame_size_for_terminal_size(terminal_size)
        } else {
            window_size
        };
        let frame_size = if self.adjust_window_size_when_changing_font_size {
            self.window
                .as_ref()
                .and_then(|window| window.request_inner_size(requested_size))
                .unwrap_or(requested_size)
        } else {
            requested_size
        };
        if let Err(error) = self.handle_window_resize(frame_size) {
            eprintln!("font size resize failed: {error}");
        }
    }

    fn reset_font_and_window_size(&mut self) {
        self.font_size_scale = DEFAULT_FONT_SIZE_SCALE;
        self.apply_window_resize_increments();
        let requested_size = self.initial_frame_size();
        let frame_size = self
            .window
            .as_ref()
            .and_then(|window| window.request_inner_size(requested_size))
            .unwrap_or(requested_size);
        if let Err(error) = self.handle_window_resize(frame_size) {
            eprintln!("reset font and window size failed: {error}");
        }
    }

    fn initial_frame_size(&self) -> PhysicalSize<u32> {
        let terminal_width = u32::from(self.initial_cols) * self.cell_width();
        let terminal_height = u32::from(self.initial_rows) * self.cell_height();
        let padding = window_padding_pixels_for_terminal_size(
            self.window_padding,
            terminal_width,
            terminal_height,
            self.cell_width(),
            self.cell_height(),
            self.window_dpi,
        );
        PhysicalSize::new(
            terminal_width.saturating_add(padding.horizontal()),
            u32::from(self.initial_rows.saturating_add(TAB_BAR_ROWS))
                .saturating_mul(self.cell_height())
                .saturating_add(padding.vertical()),
        )
    }

    fn frame_size_for_terminal_size(&self, terminal_size: TerminalSize) -> PhysicalSize<u32> {
        let terminal_width = u32::from(terminal_size.columns) * self.cell_width();
        let terminal_height = u32::from(terminal_size.rows) * self.cell_height();
        let padding = window_padding_pixels_for_terminal_size(
            self.window_padding,
            terminal_width,
            terminal_height,
            self.cell_width(),
            self.cell_height(),
            self.window_dpi,
        );
        PhysicalSize::new(
            terminal_width.saturating_add(padding.horizontal()),
            u32::from(terminal_size.rows.saturating_add(TAB_BAR_ROWS))
                .saturating_mul(self.cell_height())
                .saturating_add(padding.vertical()),
        )
    }

    fn window_resize_increment_cell_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.cell_width().max(1), self.cell_height().max(1))
    }

    fn window_resize_increments(&self) -> Option<PhysicalSize<u32>> {
        (self.use_resize_increments && native_window_resize_increments_supported())
            .then(|| self.window_resize_increment_cell_size())
    }

    fn apply_window_resize_increments(&self) {
        if let Some(window) = &self.window {
            window.set_resize_increments(self.window_resize_increments());
        }
    }

    fn cell_width(&self) -> u32 {
        let dpi_scale = if self.modern_tab_bar_brand {
            self.window_dpi_scale()
        } else {
            1.0
        };
        scaled_cell_dimension(
            if self.modern_tab_bar_brand {
                MODERN_CELL_WIDTH
            } else {
                CELL_WIDTH
            },
            dpi_scale
                * self.font_size_scale_against_default()
                * self.font_size_scale
                * self.cell_width.as_f64(),
        )
    }

    fn cell_height(&self) -> u32 {
        let dpi_scale = if self.modern_tab_bar_brand {
            self.window_dpi_scale()
        } else {
            1.0
        };
        scaled_cell_dimension(
            if self.modern_tab_bar_brand {
                MODERN_CELL_HEIGHT
            } else {
                CELL_HEIGHT
            },
            dpi_scale
                * self.font_size_scale_against_default()
                * self.font_size_scale
                * self.line_height.as_f64(),
        )
    }

    fn window_dpi_scale(&self) -> f64 {
        f64::from(self.window_dpi.max(1)) / f64::from(DEFAULT_WINDOW_DPI)
    }

    fn gpu_dpi_scale(&self) -> f32 {
        #[allow(clippy::cast_possible_truncation)]
        let scale = self.window_dpi_scale() as f32;
        if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        }
    }

    fn font_size_scale_against_default(&self) -> f64 {
        if self.modern_tab_bar_brand {
            f64::from(self.font_size.millipoints)
                / f64::from(MODERN_DEFAULT_FONT_SIZE.millipoints)
        } else {
            self.font_size.scale_against_default()
        }
    }

    fn show_debug_overlay(&mut self) {
        self.cancel_pane_inspection();
        self.debug_overlay_active = true;
    }

    fn exit_debug_overlay(&mut self) {
        self.debug_overlay_active = false;
    }

    fn handle_debug_overlay_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if !self.debug_overlay_active {
            return false;
        }

        match key.as_ref() {
            Key::Named(NamedKey::Escape) if modifiers.is_empty() => {
                self.exit_debug_overlay();
                true
            }
            _ => false,
        }
    }

    fn enter_char_select_mode(&mut self) {
        self.enter_char_select_mode_with_options(WindowCharSelectOptions::default());
    }

    fn enter_char_select_mode_with_options(&mut self, mut options: WindowCharSelectOptions) {
        self.cancel_pane_inspection();
        self.deferred_wheel_context = None;
        self.command_palette = None;
        self.pane_select = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;
        if options.group.is_none() {
            options.group = Some(self.default_char_select_group().to_owned());
        }
        self.char_select = Some(WindowCharSelect::from_options(
            options,
            &self.char_select_recently_used,
        ));
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn default_char_select_group(&self) -> &'static str {
        if self.char_select_recently_used.is_empty() {
            DEFAULT_CHAR_SELECT_GROUP
        } else {
            RECENTLY_USED_CHAR_SELECT_GROUP
        }
    }

    fn set_char_select_recently_used_path(&mut self, path: Option<PathBuf>) {
        self.char_select_recently_used_path = path;
        self.load_char_select_recently_used();
    }

    #[cfg(test)]
    fn set_char_select_recently_used_path_for_test(&mut self, path: Option<PathBuf>) {
        self.set_char_select_recently_used_path(path);
    }

    fn load_char_select_recently_used(&mut self) {
        let Some(path) = self.char_select_recently_used_path.clone() else {
            return;
        };
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return,
            Err(error) => {
                eprintln!(
                    "failed to read char-select recently-used state {}: {error}",
                    path.display()
                );
                return;
            }
        };
        let store = match serde_json::from_str::<WindowCharSelectRecentStore>(&contents) {
            Ok(store) => store,
            Err(error) => {
                eprintln!(
                    "failed to parse char-select recently-used state {}: {error}",
                    path.display()
                );
                return;
            }
        };
        let mut seen = HashSet::new();
        let entry_count = u64::try_from(store.entries.len()).unwrap_or_default();
        self.char_select_recently_used = store
            .entries
            .into_iter()
            .enumerate()
            .filter_map(|(index, mut recent)| {
                if recent.text.is_empty()
                    || recent.selections == 0
                    || !seen.insert(recent.text.clone())
                {
                    return None;
                }
                if recent.last_used == 0 {
                    let index = u64::try_from(index).unwrap_or_default();
                    recent.last_used = entry_count.saturating_sub(index);
                }
                Some(recent)
            })
            .collect();
        self.char_select_recently_used_sequence = self
            .char_select_recently_used
            .iter()
            .map(|recent| recent.last_used)
            .max()
            .unwrap_or_default();
    }

    fn persist_char_select_recently_used(&self) {
        let Some(path) = self.char_select_recently_used_path.as_ref() else {
            return;
        };
        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            eprintln!(
                "failed to create char-select recently-used state directory {}: {error}",
                parent.display()
            );
            return;
        }
        let store = WindowCharSelectRecentStore {
            entries: self.char_select_recently_used.clone(),
        };
        let contents = match serde_json::to_string_pretty(&store) {
            Ok(contents) => contents,
            Err(error) => {
                eprintln!("failed to serialize char-select recently-used state: {error}");
                return;
            }
        };
        if let Err(error) = fs::write(path, contents) {
            eprintln!(
                "failed to write char-select recently-used state {}: {error}",
                path.display()
            );
        }
    }

    fn record_char_select_recently_used(&mut self, text: &str) {
        self.char_select_recently_used_sequence =
            self.char_select_recently_used_sequence.saturating_add(1);
        let selections = self
            .char_select_recently_used
            .iter()
            .find(|recent| recent.text == text)
            .map_or(1, |recent| recent.selections.saturating_add(1));
        self.char_select_recently_used
            .retain(|recent| recent.text != text);
        self.char_select_recently_used.insert(
            0,
            WindowCharSelectRecent {
                text: text.to_owned(),
                selections,
                last_used: self.char_select_recently_used_sequence,
            },
        );
        self.persist_char_select_recently_used();
    }

    fn exit_char_select_mode(&mut self) {
        self.char_select = None;
        self.deferred_wheel_context = None;
        self.restore_active_pane_presentation_after_higher_level_ui();
    }

    fn handle_char_select_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.char_select.is_none() {
            return false;
        }

        if self.handle_char_select_navigation_key(key, modifiers) {
            return true;
        }

        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.exit_char_select_mode();
                true
            }
            Key::Named(NamedKey::Enter) | Key::Character("\r") if modifiers.is_empty() => {
                if let Some((text, copy_destination)) =
                    self.char_select.as_ref().and_then(|char_select| {
                        char_select.selected_text().map(|text| {
                            (
                                text,
                                char_select.copy_on_select.then_some(char_select.copy_to),
                            )
                        })
                    })
                {
                    let target = self.deferred_wheel_context;
                    let pane_id = target
                        .map_or_else(|| self.app_shell.active_pane_id(), |target| target.pane_id);
                    if let Some(destination) = copy_destination {
                        self.write_text_to_copy_destination(&text, destination);
                    }
                    let write_result = if self.pane_runtime_ref(pane_id).is_none() {
                        Err(io::Error::other(format!(
                            "char-select target missing: {:?}",
                            AppShellError::InvalidPane(pane_id)
                        )))
                    } else {
                        self.write_pty_bytes_to_pane(pane_id, text.as_bytes())
                    };
                    if let Err(error) = write_result {
                        eprintln!("failed to write char-select text: {error}");
                    }
                    self.record_char_select_recently_used(&text);
                    self.exit_char_select_mode();
                }
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && !modifiers.shift_key()
                    && text.eq_ignore_ascii_case("g") =>
            {
                self.exit_char_select_mode();
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && !modifiers.shift_key()
                    && text.eq_ignore_ascii_case("u") =>
            {
                if let Some(char_select) = self.char_select.as_mut() {
                    char_select.input.clear();
                    char_select.refresh_matches();
                    self.apply_window_title();
                }
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && text.eq_ignore_ascii_case("r") =>
            {
                let forward = !modifiers.shift_key();
                if let Some(char_select) = self.char_select.as_mut() {
                    char_select.cycle_group(forward);
                    self.apply_window_title();
                }
                true
            }
            Key::Named(NamedKey::Backspace) if modifiers.is_empty() => {
                if let Some(char_select) = self.char_select.as_mut() {
                    char_select.input.pop();
                    char_select.refresh_matches();
                    self.apply_window_title();
                }
                true
            }
            Key::Character(text)
                if !modifiers.control_key() && !modifiers.alt_key() && !modifiers.super_key() =>
            {
                if let Some(char_select) = self.char_select.as_mut() {
                    char_select.input.push_str(text);
                    char_select.refresh_matches();
                    self.apply_window_title();
                }
                true
            }
            _ => true,
        }
    }

    fn handle_char_select_navigation_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        let delta = match key.as_ref() {
            Key::Named(NamedKey::ArrowDown) if modifiers.is_empty() => Some(1),
            Key::Named(NamedKey::ArrowUp) if modifiers.is_empty() => Some(-1),
            _ => None,
        };
        let Some(delta) = delta else {
            return false;
        };

        if let Some(char_select) = self.char_select.as_mut() {
            char_select.move_selection(delta);
        }
        true
    }

    fn toggle_full_screen(&mut self) {
        self.full_screen = !self.full_screen;
        if let Some(window) = &self.window {
            apply_native_fullscreen(
                window,
                native_fullscreen_request(
                    self.full_screen,
                    self.native_macos_fullscreen_mode,
                    self.macos_fullscreen_extend_behind_notch,
                    current_native_fullscreen_platform(),
                ),
            );
        }

        let resize = self.native_window_resize_event(
            self.frame_width,
            self.frame_height,
            self.runtime.terminal().grid().size(),
        );
        self.dispatch_resize(&resize);
    }

    fn set_window_level(&mut self, level: NativeWindowLevel) {
        self.window_level = level;
        if let Some(window) = &self.window {
            window.set_window_level(winit_window_level_for_native(level));
        }
    }

    fn toggle_window_level(&mut self, level: NativeWindowLevel) {
        let next_level = if self.window_level == level {
            NativeWindowLevel::Normal
        } else {
            level
        };
        self.set_window_level(next_level);
    }

    fn take_window_close_request(&mut self) -> bool {
        let requested = self.window_close_requested;
        self.window_close_requested = false;
        requested
    }

    const fn event_loop_exit_requested(&self) -> bool {
        self.window_close_requested || self.application_quit_requested
    }

    fn take_application_quit_request(&mut self) -> bool {
        let requested = self.application_quit_requested;
        self.application_quit_requested = false;
        requested
    }

    #[allow(dead_code)]
    #[expect(
        clippy::too_many_lines,
        reason = "pending window creation explicitly transfers every live pane runtime"
    )]
    fn take_next_pending_window_app(&mut self) -> Option<Box<Self>> {
        let pending_window = self.app_shell.take_next_pending_window()?;
        let app_window_id = pending_window.id();
        let initial_window_position = self.pending_window_positions.remove(&app_window_id);
        let active_pane = pending_window.active_pane_id();
        let pending_panes = pending_window
            .tab()
            .panes()
            .iter()
            .map(rssh_core::app_shell::Pane::id)
            .collect::<Vec<_>>();
        let pending_local_worker_panes = self.runtime.worker().map_or_else(Vec::new, |worker| {
            pending_panes
                .iter()
                .copied()
                .filter(|pane| worker.contains_pane(*pane))
                .collect::<Vec<_>>()
        });
        let startup_command = self.pending_window_startup_command(&pending_window)?;
        let mut runtime = self
            .pane_runtimes
            .remove(&active_pane)
            .unwrap_or_else(|| self.new_inactive_pane_runtime());
        runtime.ui.prepare_for_new_window();
        let mut inactive_pane_runtimes = Vec::new();
        let mut pending_bell_counts = Vec::new();
        let mut pending_ssh_auxiliary = Vec::new();
        for pane_id in pending_panes {
            if pane_id != active_pane
                && let Some(mut inactive_runtime) = self.pane_runtimes.remove(&pane_id)
            {
                inactive_runtime.ui.prepare_for_new_window();
                inactive_pane_runtimes.push((pane_id, inactive_runtime));
            }
            if let Some(bell_count) = self.pane_bell_counts.remove(&pane_id) {
                pending_bell_counts.push((pane_id, bell_count));
            }
            pending_ssh_auxiliary.push((
                pane_id,
                self.take_ssh_pane_auxiliary_state(pane_id),
            ));
        }
        let app_shell = AppShell::from_pending_window(pending_window);
        let mut detached_app = Self::new_with_command_and_osc52_policy(
            self.frame_limit,
            self.osc52_policy,
            startup_command,
        );
        detached_app.app_window_id = app_window_id;
        detached_app.initial_window_position = initial_window_position;
        detached_app
            .initial_window_class
            .clone_from(&self.initial_window_class);
        detached_app.event_proxy.clone_from(&self.event_proxy);
        detached_app
            .reload_request_sender
            .clone_from(&self.reload_request_sender);
        detached_app.app_shell = app_shell;
        detached_app.closed_tab_history = Arc::clone(&self.closed_tab_history);
        detached_app.startup_workspace_was_explicit = true;
        detached_app.config_overrides = self.config_overrides.clone();
        detached_app.applied_config = Arc::clone(&self.applied_config);
        detached_app.dpi_by_screen.clone_from(&self.dpi_by_screen);
        detached_app.renderer.set_default_foreground(color_to_rgba(
            self.foreground_color,
            DEFAULT_RENDER_FOREGROUND_RGBA,
        ));
        detached_app.renderer.set_default_background(color_to_rgba(
            self.background_color,
            DEFAULT_RENDER_BACKGROUND_RGBA,
        ));
        detached_app.renderer.set_default_background_gradient(
            detached_app
                .window_background_gradient
                .as_ref()
                .map(NativeWindowBackgroundGradient::to_render),
        );
        detached_app.renderer.set_default_background_images(
            detached_app
                .window_background_images
                .iter()
                .map(NativeWindowBackgroundImage::to_render)
                .collect(),
        );
        detached_app.renderer.set_default_background_layers(
            detached_app
                .window_background_layers
                .iter()
                .map(NativeWindowBackgroundVisualLayer::to_render)
                .collect(),
        );
        detached_app
            .renderer
            .set_ansi_palette(self.ansi_palette.map(native_ansi_palette_to_rgba));
        detached_app
            .renderer
            .set_indexed_palette(self.indexed_palette.map(native_indexed_palette_to_rgba));
        detached_app
            .renderer
            .set_default_cursor_color(color_to_rgba(
                self.cursor_bg_color,
                DEFAULT_RENDER_FOREGROUND_RGBA,
            ));
        detached_app.renderer.set_default_cursor_border(
            self.cursor_border_color
                .map(|color| color_to_rgba(color, DEFAULT_RENDER_FOREGROUND_RGBA)),
        );
        detached_app.renderer.set_default_cursor_foreground(
            self.cursor_fg_color
                .map(|color| color_to_rgba(color, DEFAULT_RENDER_FOREGROUND_RGBA)),
        );
        detached_app.leader_active_since = None;
        detached_app.inherit_effective_config_from(self);
        detached_app.install_active_runtime(runtime);
        for (pane_id, runtime) in inactive_pane_runtimes {
            detached_app.pane_runtimes.insert(pane_id, runtime);
        }
        for (pane_id, bell_count) in pending_bell_counts {
            detached_app.pane_bell_counts.insert(pane_id, bell_count);
        }
        for (pane_id, auxiliary) in pending_ssh_auxiliary {
            detached_app.install_ssh_pane_auxiliary_state(pane_id, auxiliary);
        }
        for pane_id in pending_local_worker_panes {
            if let Some(worker) = self.runtime.worker_mut()
                && let Err(error) = worker.retire_pane_transport(pane_id)
            {
                eprintln!("failed to retire pending-window source pane worker: {error}");
            }
        }
        detached_app.apply_window_title();
        Some(detached_app)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn inherit_effective_config_from(&mut self, source: &Self) {
        self.runtime.set_composition(source.runtime.composition());
        self.set_renderer_mode(source.renderer_mode);
        self.set_benchmark_startup(source.is_benchmark_startup());
        self.applied_config = Arc::clone(&source.applied_config);
        self.base_config_overrides
            .clone_from(&source.base_config_overrides);
        self.base_config_generation = source.base_config_generation;
        self.base_config_source
            .clone_from(&source.base_config_source);
        self.window_config_overrides
            .clone_from(&source.window_config_overrides);
        self.config_overrides.clone_from(&source.config_overrides);
        self.configured_dpi = source.configured_dpi;
        self.dpi_by_screen.clone_from(&source.dpi_by_screen);
        self.detected_window_dpi = source.detected_window_dpi;
        self.apply_effective_window_dpi();
        self.lua_tab_title.clone_from(&source.lua_tab_title);
        self.lua_window_title.clone_from(&source.lua_window_title);
        self.lua_update_status.clone_from(&source.lua_update_status);
        self.lua_update_status_config_overrides
            .clone_from(&source.lua_update_status_config_overrides);
        self.lua_bell.clone_from(&source.lua_bell);
        self.lua_focus_changed.clone_from(&source.lua_focus_changed);
        self.lua_resized.clone_from(&source.lua_resized);
        self.lua_config_reloaded
            .clone_from(&source.lua_config_reloaded);
        self.lua_user_var_changed
            .clone_from(&source.lua_user_var_changed);
        self.lua_open_uri.clone_from(&source.lua_open_uri);
        self.lua_new_tab_button_click = source.lua_new_tab_button_click;
        self.lua_command_palette_entries
            .clone_from(&source.lua_command_palette_entries);
        self.lua_emit_event_handlers
            .clone_from(&source.lua_emit_event_handlers);
        self.last_redraw_request_at = source.last_redraw_request_at;
        self.last_animation_redraw_request_at = source.last_animation_redraw_request_at;
        self.last_status_update_at = None;
        self.renderer.set_default_foreground(color_to_rgba(
            source.foreground_color,
            DEFAULT_RENDER_FOREGROUND_RGBA,
        ));
        self.renderer.set_default_background(color_to_rgba(
            source.background_color,
            DEFAULT_RENDER_BACKGROUND_RGBA,
        ));
        self.renderer.set_default_background_gradient(
            self.window_background_gradient
                .as_ref()
                .map(NativeWindowBackgroundGradient::to_render),
        );
        self.renderer.set_default_background_images(
            self.window_background_images
                .iter()
                .map(NativeWindowBackgroundImage::to_render)
                .collect(),
        );
        self.renderer.set_default_background_layers(
            self.window_background_layers
                .iter()
                .map(NativeWindowBackgroundVisualLayer::to_render)
                .collect(),
        );
        self.renderer
            .set_ansi_palette(source.ansi_palette.map(native_ansi_palette_to_rgba));
        self.renderer
            .set_indexed_palette(source.indexed_palette.map(native_indexed_palette_to_rgba));
        self.renderer.set_default_cursor_color(color_to_rgba(
            source.cursor_bg_color,
            DEFAULT_RENDER_FOREGROUND_RGBA,
        ));
        self.renderer.set_default_cursor_border(
            source
                .cursor_border_color
                .map(|color| color_to_rgba(color, DEFAULT_RENDER_FOREGROUND_RGBA)),
        );
        self.renderer.set_default_cursor_foreground(
            source
                .cursor_fg_color
                .map(|color| color_to_rgba(color, DEFAULT_RENDER_FOREGROUND_RGBA)),
        );
        self.ime_preedit.clone_from(&source.ime_preedit);
        self.dead_key_active = false;
        self.dead_key_text = None;
        self.leader_active_since = None;
        self.cursor_blink_visible = true;
        self.cursor_blink_opacity_alpha = u8::MAX;
        self.last_cursor_blink_at = None;
        self.text_blink_opacity_alpha = u8::MAX;
        self.rapid_text_blink_opacity_alpha = u8::MAX;
        self.last_text_blink_at = None;
        self.last_rapid_text_blink_at = None;
        self.renderer.set_text_blink_opacity(1.0);
        self.renderer.set_rapid_text_blink_opacity(1.0);
        self.renderer
            .set_bold_brightens_ansi_colors(self.bold_brightens_ansi_colors.into());
        self.renderer
            .set_cursor_thickness(self.cursor_thickness.map(RenderCursorThickness::from));
        self.renderer
            .set_underline_thickness(self.underline_thickness.map(RenderUnderlineThickness::from));
        self.renderer
            .set_underline_position(self.underline_position.map(RenderUnderlinePosition::from));
        self.renderer.set_strikethrough_position(
            self.strikethrough_position
                .map(RenderStrikethroughPosition::from),
        );
        self.renderer
            .set_force_reverse_video_cursor(self.force_reverse_video_cursor);
        self.renderer.set_reverse_video_cursor_min_contrast(Some(
            self.reverse_video_cursor_min_contrast.as_f64(),
        ));
        self.frame_needs_full_repaint = true;
    }

    fn pending_window_startup_command(
        &self,
        pending_window: &rssh_core::app_shell::PendingWindow,
    ) -> Option<PtyCommand> {
        let active_pane = pending_window.active_pane_id();
        let launch = pending_window
            .tab()
            .panes()
            .iter()
            .find(|pane| pane.id() == active_pane)?
            .launch();
        let term_session_id = iterm_session_termid(
            pending_window.id().get(),
            pending_window.active_tab_id().get(),
            active_pane.get(),
        );
        let environment = self.pane_environment_variables();
        Some(pty_command_from_pane_launch_with_term_session_id(
            launch,
            &self.term,
            &environment,
            self.default_cwd.as_deref(),
            &term_session_id,
        ))
    }

    fn pane_environment_variables(&self) -> BTreeMap<String, String> {
        let mut environment = self.derived_config_environment.clone();
        environment.extend(self.set_environment_variables.clone());
        for name in &self.mux_env_remove {
            environment.remove(name);
        }
        if self.mux_enable_ssh_agent
            && let Some(default_ssh_auth_sock) = self
                .default_ssh_auth_sock
                .as_deref()
                .filter(|ssh_auth_sock| !ssh_auth_sock.is_empty())
        {
            environment.insert(
                SSH_AUTH_SOCK_ENV.to_owned(),
                default_ssh_auth_sock.to_owned(),
            );
        }
        environment
    }

    fn sync_pane_runtimes(
        &mut self,
        previous_active_pane: rssh_core::PaneId,
        previous_runtime: PaneRuntime,
    ) {
        let valid_pane_ids = self.app_shell.pane_ids();
        let active_pane = self.app_shell.active_pane_id();
        let active_was_replaced = previous_active_pane != active_pane;

        if active_was_replaced {
            if valid_pane_ids.contains(&previous_active_pane) {
                self.pane_runtimes
                    .insert(previous_active_pane, previous_runtime);
            } else {
                self.stop_local_runtime_for_pane(previous_active_pane);
                self.cancel_ssh_runtime(previous_active_pane);
                self.retire_ssh_connection_state(previous_active_pane);
                let mut previous_runtime = previous_runtime;
                let cleanup = previous_runtime.close();
                report_pane_pty_cleanup("removed pane PTY cleanup", &cleanup);
            }
        } else {
            self.install_active_runtime(previous_runtime);
        }

        if active_was_replaced && valid_pane_ids.contains(&active_pane) {
            if !self.pane_runtimes.contains_key(&active_pane) {
                self.spawn_active_pane_runtime_if_needed();
            }

            if let Some(runtime) = self.pane_runtimes.remove(&active_pane) {
                self.install_active_runtime(runtime);
                self.sync_window_title_from_runtime();
            }
        } else if active_was_replaced {
            self.install_active_runtime(self.new_inactive_pane_runtime());
            self.sync_window_title_from_runtime();
        }

        let retired_panes = self
            .pane_runtimes
            .keys()
            .filter(|pane_id| !valid_pane_ids.contains(pane_id))
            .copied()
            .collect::<Vec<_>>();
        for pane_id in retired_panes {
            self.stop_local_runtime_for_pane(pane_id);
            self.cancel_ssh_runtime(pane_id);
            self.retire_ssh_connection_state(pane_id);
            if let Some(mut runtime) = self.pane_runtimes.remove(&pane_id) {
                let cleanup = runtime.close();
                report_pane_pty_cleanup("retired pane PTY cleanup", &cleanup);
            }
        }
        #[cfg(feature = "ssh")]
        let retired_ssh_panes = self
            .ssh_connection_states
            .keys()
            .chain(self.ssh_writer_senders.keys())
            .chain(self.ssh_writer_cancellations.keys())
            .chain(self.ssh_connection_cancellations.keys())
            .filter(|pane_id| !valid_pane_ids.contains(pane_id))
            .copied()
            .collect::<HashSet<_>>();
        #[cfg(feature = "ssh")]
        for pane_id in retired_ssh_panes {
            self.cancel_ssh_runtime(pane_id);
            self.retire_ssh_connection_state(pane_id);
        }
        self.pane_bell_counts
            .retain(|pane_id, _| valid_pane_ids.contains(pane_id));
    }

    fn end_pointer_modes_for_pane_change(&mut self) {
        self.selecting = false;
        self.active_mouse_button = None;
        self.scrollbar_dragging = false;
        self.split_resize_dragging = None;
        self.last_mouse_assignment_click = None;
        self.last_left_click = None;
    }

    fn pointer_transient_state(&self) -> PanePointerTransientState {
        PanePointerTransientState {
            selecting: self.selecting,
            active_mouse_button: self.active_mouse_button,
            scrollbar_dragging: self.scrollbar_dragging,
            split_resize_dragging: self.split_resize_dragging,
            last_mouse_assignment_click: self.last_mouse_assignment_click,
            last_left_click: self.last_left_click,
        }
    }

    fn restore_pointer_transient_state(&mut self, state: PanePointerTransientState) {
        self.selecting = state.selecting;
        self.active_mouse_button = state.active_mouse_button;
        self.scrollbar_dragging = state.scrollbar_dragging;
        self.split_resize_dragging = state.split_resize_dragging;
        self.last_mouse_assignment_click = state.last_mouse_assignment_click;
        self.last_left_click = state.last_left_click;
    }

    fn restore_split_resize_pointer_state(&mut self, state: PanePointerTransientState) {
        self.active_mouse_button = state.active_mouse_button;
        self.split_resize_dragging = state.split_resize_dragging;
    }

    fn clear_derived_selection_projection_for_shell_action(&mut self) {
        self.selection = None;
    }

    fn take_active_runtime(&mut self) -> PaneRuntime {
        let size = self.runtime.terminal().grid().size();
        let session = self.session.take();
        let session_process_id = self.session_process_id.take();
        let session_tty_name = self.session_tty_name.take();
        let writer = self.writer.take();
        let reader_thread = self.reader_thread.take();
        let writer_thread = self.writer_thread.take();
        let runtime_generation = std::mem::replace(&mut self.active_runtime_generation, 0);
        let transport = self.active_runtime_transport.take();
        let ui = std::mem::take(&mut self.active_ui);
        self.clear_derived_selection_projection_for_shell_action();

        let mut replacement_runtime = TerminalRuntime::new(size);
        replacement_runtime.set_terminal_name(self.term.clone());
        replacement_runtime.set_enq_answerback(self.enq_answerback.clone());
        replacement_runtime.set_enable_kitty_graphics(self.enable_kitty_graphics);
        replacement_runtime
            .set_enable_checksum_rectangular_area(self.enable_checksum_rectangular_area);
        replacement_runtime.set_enable_title_reporting(self.enable_title_reporting);
        replacement_runtime.set_enable_kitty_keyboard(self.enable_kitty_keyboard);
        replacement_runtime.set_allow_win32_input_mode(self.allow_win32_input_mode);
        replacement_runtime.set_treat_east_asian_ambiguous_width_as_wide(
            self.treat_east_asian_ambiguous_width_as_wide,
        );
        replacement_runtime
            .set_normalize_output_to_unicode_nfc(self.normalize_output_to_unicode_nfc);
        replacement_runtime.set_unicode_version(self.unicode_version);
        replacement_runtime.set_cell_width_overrides(self.terminal_cell_width_overrides());
        replacement_runtime.set_scrollback_limit(self.scrollback_lines);
        replacement_runtime.set_default_cursor_style(CursorStyle::from(self.default_cursor_style));
        let old_runtime = std::mem::replace(&mut *self.runtime, replacement_runtime);
        let old_snapshot = terminal_runtime_snapshot(&old_runtime, ui.stable_viewport);

        PaneRuntime {
            runtime: old_runtime,
            transport,
            session,
            session_process_id,
            session_tty_name,
            writer,
            reader_thread,
            writer_thread,
            runtime_generation,
            snapshot: old_snapshot,
            ui,
        }
    }

    fn new_inactive_pane_runtime(&self) -> PaneRuntime {
        let size = self.runtime.terminal().grid().size();
        let runtime = self.configured_pane_terminal_runtime(size);
        let snapshot = terminal_runtime_snapshot(&runtime, PaneStableViewport::default());
        PaneRuntime {
            runtime,
            transport: None,
            session: None,
            session_process_id: None,
            session_tty_name: None,
            writer: None,
            reader_thread: None,
            writer_thread: None,
            runtime_generation: 0,
            snapshot,
            ui: PaneUiState::default(),
        }
    }

}

impl NativeWindowApp {
    fn install_active_runtime(&mut self, mut runtime: PaneRuntime) {
        let applied_config = Arc::clone(&self.applied_config);
        let mut runtime_runtime = TerminalRuntime::new(self.runtime.terminal().grid().size());
        let mut runtime_snapshot =
            terminal_runtime_snapshot(&self.runtime, PaneStableViewport::default());

        std::mem::swap(&mut runtime.runtime, &mut runtime_runtime);
        std::mem::swap(&mut runtime.snapshot, &mut runtime_snapshot);

        *self.runtime = runtime_runtime;
        self.runtime
            .set_terminal_name(applied_config.term.clone());
        self.runtime
            .set_enable_kitty_keyboard(applied_config.enable_kitty_keyboard);
        self.runtime
            .set_allow_win32_input_mode(applied_config.allow_win32_input_mode);
        self.runtime.set_treat_east_asian_ambiguous_width_as_wide(
            applied_config.treat_east_asian_ambiguous_width_as_wide,
        );
        self.runtime
            .set_normalize_output_to_unicode_nfc(applied_config.normalize_output_to_unicode_nfc);
        self.runtime
            .set_unicode_version(applied_config.unicode_version);
        let cell_width_overrides = self.terminal_cell_width_overrides();
        self.runtime.set_cell_width_overrides(cell_width_overrides);
        self.snapshot = runtime_snapshot;
        self.session = runtime.session.take();
        self.session_process_id = runtime.session_process_id.take();
        self.session_tty_name = runtime.session_tty_name.take();
        self.writer = runtime.writer.take();
        self.reader_thread = runtime.reader_thread.take();
        self.writer_thread = runtime.writer_thread.take();
        self.active_runtime_generation = runtime.runtime_generation;
        self.active_runtime_transport = runtime.transport;
        self.active_ui = std::mem::take(&mut runtime.ui);
        let active = self.app_shell.active_pane_id();
        if let Some(worker) = self.runtime.worker_mut()
            && worker.contains_pane(active)
            && let Err(error) = worker.activate_pane(active)
        {
            eprintln!("runtime V2 active-pane routing error: {error}");
        }
        self.update_selection_projection();
        self.rebuild_snapshot();

        if let Some(writer) = self.writer.as_mut() {
            let _ = writer.flush();
        }

        self.frame_needs_full_repaint = true;
        self.pending_frame_damage.clear();
    }

    fn spawn_active_pane_runtime_if_needed(&mut self) {
        if self.active_pane_transport_is_started() {
            return;
        }

        if self.event_proxy.is_none() {
            self.session = None;
            self.session_process_id = None;
            self.session_tty_name = None;
            self.writer = None;
            self.reader_thread = None;
            self.writer_thread = None;
            return;
        }

        match self.spawn_pane_runtime_for_active_pane() {
            Ok(runtime) => self.install_active_runtime(runtime),
            Err(error) => {
                eprintln!("PTY spawn error while syncing pane runtime: {error}");
                self.session = None;
                self.session_process_id = None;
                self.session_tty_name = None;
                self.writer = None;
                self.reader_thread = None;
                self.writer_thread = None;
            }
        }
    }

    fn active_pane_transport_is_started(&self) -> bool {
        let active = self.app_shell.active_pane_id();
        self.session.is_some()
            || self.writer.is_some()
            || self.ssh_writer_senders.contains_key(&active)
            || self
                .runtime
                .worker()
                .is_some_and(|runtime| runtime.contains_pane(active))
    }

    fn handle_transport_start_error(&mut self, error: &dyn std::fmt::Display) -> bool {
        let active = self.app_shell.active_pane_id();
        if matches!(
            self.app_shell.active_pane().launch().domain(),
            PaneLaunchDomain::Ssh(_)
        ) {
            eprintln!("deferred SSH transport start error: {error}");
            self.cancel_ssh_runtime(active);
            self.handle_ssh_state(active, ConnectionState::Failed);
            return false;
        }
        eprintln!("deferred local transport start error: {error}");
        true
    }

    fn command_palette_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
        modifiers.control_key()
            && modifiers.shift_key()
            && !modifiers.alt_key()
            && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("p"))
    }

    #[cfg(test)]
    fn handle_reload_configuration_shortcut(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
    ) -> bool {
        self.handle_reload_configuration_shortcut_with_preference(
            key,
            None,
            modifiers,
            NativeKeyMapPreference::Mapped,
        )
    }

    fn handle_reload_configuration_shortcut_event(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
    ) -> bool {
        self.handle_reload_configuration_shortcut_with_preference(
            key,
            Some(physical_key),
            modifiers,
            self.key_map_preference,
        )
    }

    fn handle_reload_configuration_shortcut_with_preference(
        &mut self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
        key_map_preference: NativeKeyMapPreference,
    ) -> bool {
        if self.default_assignment_disabled_for_key_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return false;
        }

        if !window_reload_configuration_shortcut_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return false;
        }

        self.request_reload_configuration();
        true
    }

    fn handle_toggle_full_screen_shortcut(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.default_assignment_disabled_for_key(key, modifiers) {
            return false;
        }

        if !window_toggle_full_screen_shortcut(key, modifiers) {
            return false;
        }

        self.toggle_full_screen();
        true
    }

    #[cfg(test)]
    fn handle_hide_shortcut(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        self.handle_hide_shortcut_with_preference(
            key,
            None,
            modifiers,
            NativeKeyMapPreference::Mapped,
        )
    }

    fn handle_hide_shortcut_event(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
    ) -> bool {
        self.handle_hide_shortcut_with_preference(
            key,
            Some(physical_key),
            modifiers,
            self.key_map_preference,
        )
    }

    fn handle_hide_shortcut_with_preference(
        &mut self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
        key_map_preference: NativeKeyMapPreference,
    ) -> bool {
        if self.default_assignment_disabled_for_key_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return false;
        }

        if !window_hide_shortcut_with_preference(key, physical_key, modifiers, key_map_preference) {
            return false;
        }

        self.hide_window();
        true
    }

    #[cfg(test)]
    fn handle_application_hide_shortcut(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        self.handle_application_hide_shortcut_with_preference(
            key,
            None,
            modifiers,
            NativeKeyMapPreference::Mapped,
        )
    }

    fn handle_application_hide_shortcut_event(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
    ) -> bool {
        self.handle_application_hide_shortcut_with_preference(
            key,
            Some(physical_key),
            modifiers,
            self.key_map_preference,
        )
    }

    fn handle_application_hide_shortcut_with_preference(
        &mut self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
        key_map_preference: NativeKeyMapPreference,
    ) -> bool {
        if self.default_assignment_disabled_for_key_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return false;
        }

        if !window_application_hide_shortcut_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return false;
        }

        self.hide_application();
        true
    }

    fn handle_font_size_shortcut(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.default_assignment_disabled_for_key(key, modifiers) {
            return false;
        }

        let Some(action) = window_font_size_shortcut(key, modifiers) else {
            return false;
        };

        self.adjust_font_size(action);
        true
    }

    #[cfg(test)]
    fn handle_show_debug_overlay_shortcut(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        self.handle_show_debug_overlay_shortcut_with_preference(
            key,
            None,
            modifiers,
            NativeKeyMapPreference::Mapped,
        )
    }

    fn handle_show_debug_overlay_shortcut_event(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
    ) -> bool {
        self.handle_show_debug_overlay_shortcut_with_preference(
            key,
            Some(physical_key),
            modifiers,
            self.key_map_preference,
        )
    }

    fn handle_show_debug_overlay_shortcut_with_preference(
        &mut self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
        key_map_preference: NativeKeyMapPreference,
    ) -> bool {
        if self.default_assignment_disabled_for_key_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return false;
        }

        if !window_show_debug_overlay_shortcut_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return false;
        }

        self.show_debug_overlay();
        true
    }

    #[cfg(test)]
    fn handle_char_select_shortcut(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        self.handle_char_select_shortcut_with_preference(
            key,
            None,
            modifiers,
            NativeKeyMapPreference::Mapped,
        )
    }

    fn handle_char_select_shortcut_event(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
    ) -> bool {
        self.handle_char_select_shortcut_with_preference(
            key,
            Some(physical_key),
            modifiers,
            self.key_map_preference,
        )
    }

    fn handle_char_select_shortcut_with_preference(
        &mut self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
        key_map_preference: NativeKeyMapPreference,
    ) -> bool {
        if self.default_assignment_disabled_for_key_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return false;
        }

        if !window_char_select_shortcut_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return false;
        }

        self.enter_char_select_mode();
        true
    }

    fn enter_command_palette_mode(&mut self) {
        let pane_id = self.app_shell.active_pane_id();
        self.deferred_wheel_context = None;
        self.enter_command_palette_mode_for_pane(pane_id);
    }

    fn enter_command_palette_mode_for_pane(&mut self, pane_id: rssh_core::PaneId) {
        self.cancel_pane_inspection();
        self.pane_select = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;
        let event = NativeCommandPaletteAugment {
            window_id: self.app_window_id,
            pane: pane_id,
        };
        let augmented_entries = self.dispatch_command_palette_augment(&event);
        self.command_palette = Some(WindowCommandPalette {
            augmented_entries,
            ..WindowCommandPalette::default()
        });
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn enter_tab_context_menu(&mut self, tab: rssh_core::TabId) -> Result<(), AppShellError> {
        if self.app_shell.active_tab_id() != tab {
            self.dispatch_app_action(AppAction::ActivateTab { tab })?;
        }
        self.enter_command_palette_mode();
        let tab_transfer_targets = self.tab_transfer_targets.clone();
        let palette = self
            .command_palette
            .as_mut()
            .expect("entering the command palette must create its state");
        palette.context_title = Some("Tab Actions".to_owned());
        let mut entries = vec![
            WindowCommand::NewTab,
            WindowCommand::DuplicateTab,
            WindowCommand::RenameTab,
            WindowCommand::MoveTabToNewWindow,
        ]
        .into_iter()
        .map(WindowCommandPaletteEntry::BuiltIn)
        .collect::<Vec<_>>();
        entries.extend(tab_transfer_targets.into_iter().map(|window_id| {
            WindowCommandPaletteEntry::Contextual {
                command: WindowCommand::MoveTabToWindow(window_id),
                label: format!("Move Tab To Window {}", window_id.get()),
            }
        }));
        entries.extend([
            WindowCommand::CloseTab,
            WindowCommand::CloseOtherTabs,
            WindowCommand::CloseTabsToRight,
            WindowCommand::ReopenClosedTab,
        ]
        .into_iter()
        .map(WindowCommandPaletteEntry::BuiltIn));
        palette.context_entries = Some(entries);
        Ok(())
    }

    fn enter_launcher_mode_with_args(&mut self, args: WindowShowLauncherArgs) {
        self.cancel_pane_inspection();
        self.deferred_wheel_context = None;
        self.pane_select = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;
        self.command_palette = Some(WindowCommandPalette {
            launcher_args: Some(args),
            ..WindowCommandPalette::default()
        });
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn enter_launcher_mode(&mut self) {
        self.enter_launcher_mode_with_args(WindowShowLauncherArgs {
            flags: WindowShowLauncherFlags::default_launcher(),
            title: None,
            alphabet: None,
            help_text: None,
            fuzzy_help_text: None,
        });
    }

    fn exit_command_palette_mode(&mut self) {
        self.command_palette = None;
        self.deferred_wheel_context = None;
        self.restore_active_pane_presentation_after_higher_level_ui();
    }

    fn command_palette_filtered_commands(&self) -> Vec<WindowCommand> {
        let Some(palette) = self.command_palette.as_ref() else {
            return Vec::new();
        };
        if palette.launcher_args.is_some() {
            return self
                .command_palette_filtered_entries()
                .into_iter()
                .map(WindowCommandPaletteEntry::into_command)
                .collect();
        }
        if let Some(entries) = palette.context_entries.as_ref() {
            if palette.query.is_empty() {
                return entries
                    .iter()
                    .cloned()
                    .map(WindowCommandPaletteEntry::into_command)
                    .collect();
            }
            let query = palette.query.to_ascii_lowercase();
            return entries
                .iter()
                .filter(|entry| palette_match_score(entry.label(), &query).is_some())
                .cloned()
                .map(WindowCommandPaletteEntry::into_command)
                .collect();
        }
        if palette.query.is_empty() {
            let mut commands = WINDOW_COMMANDS.to_vec();
            self.sort_command_palette_commands(&mut commands, (0, 0));
            return commands;
        }

        if let Some(command) = command_palette_structured_query_command(&palette.query) {
            return vec![command];
        }

        let query = palette.query.to_ascii_lowercase();
        let mut matches = WINDOW_COMMANDS
            .iter()
            .cloned()
            .filter_map(|command| {
                palette_match_score(command.label(), &query).map(|score| (command, score))
            })
            .collect::<Vec<_>>();

        matches.sort_by_key(|(command, score)| self.command_palette_rank(command.label(), *score));
        matches.into_iter().map(|(command, _)| command).collect()
    }

    fn command_palette_filtered_entries(&self) -> Vec<WindowCommandPaletteEntry> {
        let Some(palette) = self.command_palette.as_ref() else {
            return Vec::new();
        };
        if let Some(args) = &palette.launcher_args {
            return self.launcher_filtered_entries(args, &palette.query);
        }
        if let Some(entries) = palette.context_entries.as_ref() {
            if palette.query.is_empty() {
                return entries.clone();
            }
            let query = palette.query.to_ascii_lowercase();
            return entries
                .iter()
                .filter(|entry| palette_match_score(entry.label(), &query).is_some())
                .cloned()
                .collect();
        }
        if palette.query.is_empty() {
            let mut entries = self
                .command_palette_filtered_commands()
                .into_iter()
                .map(WindowCommandPaletteEntry::BuiltIn)
                .chain(
                    palette
                        .augmented_entries
                        .iter()
                        .cloned()
                        .map(WindowCommandPaletteEntry::Augmented),
                )
                .collect::<Vec<_>>();
            self.sort_command_palette_entries(&mut entries, (0, 0));
            return entries;
        }

        if let Some(command) = command_palette_structured_query_command(&palette.query) {
            return vec![WindowCommandPaletteEntry::BuiltIn(command)];
        }

        let query = palette.query.to_ascii_lowercase();
        let built_in_entries = WINDOW_COMMANDS
            .iter()
            .cloned()
            .map(WindowCommandPaletteEntry::BuiltIn);
        let augmented_entries = palette
            .augmented_entries
            .iter()
            .cloned()
            .map(WindowCommandPaletteEntry::Augmented);
        let mut matches = built_in_entries
            .chain(augmented_entries)
            .filter_map(|entry| {
                palette_match_score(entry.label(), &query).map(|score| (entry, score))
            })
            .collect::<Vec<_>>();

        matches.sort_by_key(|(entry, score)| self.command_palette_rank(entry.label(), *score));
        matches.into_iter().map(|(entry, _)| entry).collect()
    }

    fn launcher_filtered_entries(
        &self,
        args: &WindowShowLauncherArgs,
        query: &str,
    ) -> Vec<WindowCommandPaletteEntry> {
        let entries = self.launcher_entries(args);
        if query.is_empty() {
            return entries;
        }

        let query = query.to_ascii_lowercase();
        let mut matches = entries
            .into_iter()
            .filter_map(|entry| {
                palette_match_score(entry.label(), &query).map(|score| (entry, score))
            })
            .collect::<Vec<_>>();

        matches.sort_by_key(|(entry, score)| self.command_palette_rank(entry.label(), *score));
        matches.into_iter().map(|(entry, _)| entry).collect()
    }

    fn sort_command_palette_commands(&self, commands: &mut [WindowCommand], score: (usize, usize)) {
        commands.sort_by_key(|command| self.command_palette_rank(command.label(), score));
    }

    fn sort_command_palette_entries(
        &self,
        entries: &mut [WindowCommandPaletteEntry],
        score: (usize, usize),
    ) {
        entries.sort_by_key(|entry| self.command_palette_rank(entry.label(), score));
    }

    fn command_palette_rank(
        &self,
        label: &str,
        score: (usize, usize),
    ) -> ((usize, usize), Reverse<u64>, Reverse<u64>) {
        let frecency = self.command_palette_frecency(label);
        (score, Reverse(frecency.uses), Reverse(frecency.last_used))
    }

    fn command_palette_frecency(&self, label: &str) -> WindowCommandPaletteFrecency {
        self.command_palette_frecency
            .get(label)
            .copied()
            .unwrap_or_default()
    }

    fn set_command_palette_frecency_path(&mut self, path: Option<PathBuf>) {
        self.command_palette_frecency_path = path;
        self.load_command_palette_frecency();
    }

    #[cfg(test)]
    fn set_command_palette_frecency_path_for_test(&mut self, path: Option<PathBuf>) {
        self.set_command_palette_frecency_path(path);
    }

    fn load_command_palette_frecency(&mut self) {
        let Some(path) = self.command_palette_frecency_path.clone() else {
            return;
        };
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return,
            Err(error) => {
                eprintln!(
                    "failed to read command palette frecency state {}: {error}",
                    path.display()
                );
                return;
            }
        };
        let store = match serde_json::from_str::<WindowCommandPaletteFrecencyStore>(&contents) {
            Ok(store) => store,
            Err(error) => {
                eprintln!(
                    "failed to parse command palette frecency state {}: {error}",
                    path.display()
                );
                return;
            }
        };
        let max_last_used = store
            .entries
            .values()
            .map(|entry| entry.last_used)
            .max()
            .unwrap_or_default();
        self.command_palette_frecency = store.entries.into_iter().collect();
        self.command_palette_frecency_sequence = store.sequence.max(max_last_used);
    }

    fn persist_command_palette_frecency(&self) {
        let Some(path) = self.command_palette_frecency_path.as_ref() else {
            return;
        };
        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            eprintln!(
                "failed to create command palette frecency state directory {}: {error}",
                parent.display()
            );
            return;
        }
        let store = WindowCommandPaletteFrecencyStore {
            sequence: self.command_palette_frecency_sequence,
            entries: self
                .command_palette_frecency
                .iter()
                .map(|(label, frecency)| (label.clone(), *frecency))
                .collect(),
        };
        let contents = match serde_json::to_string_pretty(&store) {
            Ok(contents) => contents,
            Err(error) => {
                eprintln!("failed to serialize command palette frecency state: {error}");
                return;
            }
        };
        if let Err(error) = fs::write(path, contents) {
            eprintln!(
                "failed to write command palette frecency state {}: {error}",
                path.display()
            );
        }
    }

    fn record_command_palette_label(&mut self, label: &str) {
        self.command_palette_frecency_sequence =
            self.command_palette_frecency_sequence.saturating_add(1);
        let last_used = self.command_palette_frecency_sequence;
        let entry = self
            .command_palette_frecency
            .entry(label.to_owned())
            .or_default();
        entry.uses = entry.uses.saturating_add(1);
        entry.last_used = last_used;
        self.persist_command_palette_frecency();
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn launcher_entries(&self, args: &WindowShowLauncherArgs) -> Vec<WindowCommandPaletteEntry> {
        fn add_domain_entry(
            entries: &mut Vec<WindowCommandPaletteEntry>,
            added_domains: &mut HashSet<String>,
            name: &str,
            action: WindowCommand,
        ) {
            let normalized_name = name.to_ascii_lowercase();
            if !added_domains.insert(normalized_name) {
                return;
            }
            let doc = match action {
                WindowCommand::AttachDomain(_) => {
                    if is_local_domain_name(name) {
                        None
                    } else {
                        Some("Attach Domain actions are currently unsupported".to_owned())
                    }
                }
                _ => None,
            };
            entries.push(WindowCommandPaletteEntry::Augmented(
                NativeCommandPaletteEntry {
                    brief: format!("Spawn In Domain: {name}"),
                    doc,
                    icon: None,
                    key_assignment: None,
                    action,
                },
            ));
        }

        let mut entries = Vec::new();
        if args.flags.commands {
            entries.extend(
                WINDOW_COMMANDS
                    .iter()
                    .cloned()
                    .map(WindowCommandPaletteEntry::BuiltIn),
            );
        }
        if args.flags.domains {
            let mut added_domains = HashSet::new();
            entries.push(WindowCommandPaletteEntry::Augmented(
                NativeCommandPaletteEntry {
                    brief: "Spawn In Domain: local".to_owned(),
                    doc: None,
                    icon: None,
                    key_assignment: None,
                    action: WindowCommand::NewTab,
                },
            ));
            added_domains.insert(DEFAULT_DOMAIN_NAME.to_owned());

            add_domain_entry(
                &mut entries,
                &mut added_domains,
                &self.default_domain,
                WindowCommand::AttachDomain(self.default_domain.clone()),
            );
            self.exec_domains
                .iter()
                .map(|domain| &domain.name)
                .for_each(|name| {
                    add_domain_entry(
                        &mut entries,
                        &mut added_domains,
                        name,
                        WindowCommand::AttachDomain(name.to_owned()),
                    );
                });
            self.wsl_domains
                .iter()
                .map(|domain| &domain.name)
                .for_each(|name| {
                    add_domain_entry(
                        &mut entries,
                        &mut added_domains,
                        name,
                        WindowCommand::AttachDomain(name.to_owned()),
                    );
                });
            self.unix_domains
                .iter()
                .map(|domain| &domain.name)
                .for_each(|name| {
                    add_domain_entry(
                        &mut entries,
                        &mut added_domains,
                        name,
                        WindowCommand::AttachDomain(name.to_owned()),
                    );
                });
            self.ssh_domains
                .iter()
                .map(|domain| &domain.name)
                .for_each(|name| {
                    add_domain_entry(
                        &mut entries,
                        &mut added_domains,
                        name,
                        WindowCommand::AttachDomain(name.to_owned()),
                    );
                });
            self.tls_clients
                .iter()
                .map(|domain| &domain.name)
                .for_each(|name| {
                    add_domain_entry(
                        &mut entries,
                        &mut added_domains,
                        name,
                        WindowCommand::AttachDomain(name.to_owned()),
                    );
                });
            self.serial_ports
                .iter()
                .map(|domain| &domain.name)
                .for_each(|name| {
                    add_domain_entry(
                        &mut entries,
                        &mut added_domains,
                        name,
                        WindowCommand::AttachDomain(name.to_owned()),
                    );
                });
        }
        if args.flags.key_assignments {
            entries.extend(native_window_key_assignment_entries());
            entries.extend(self.user_key_assignment_entries());
        }
        if args.flags.launch_menu_items {
            entries.extend(self.launch_menu.iter().map(|item| {
                let label = item
                    .label
                    .clone()
                    .unwrap_or_else(|| item.command.launch_menu_label());
                WindowCommandPaletteEntry::Augmented(NativeCommandPaletteEntry {
                    brief: label,
                    doc: None,
                    icon: None,
                    key_assignment: None,
                    action: item.command.window_command(),
                })
            }));
        }
        if args.flags.tabs {
            let tabs = self.app_shell.active_workspace().tabs();
            entries.extend(tabs.iter().map(|tab| {
                let title = self
                    .tab_title_for_tab(tab)
                    .unwrap_or_else(|| format!("tab {}", tab.id().get()));
                WindowCommandPaletteEntry::Augmented(NativeCommandPaletteEntry {
                    brief: format!("Activate Tab: {title}"),
                    doc: None,
                    icon: None,
                    key_assignment: None,
                    action: WindowCommand::ActivateTabId(tab.id()),
                })
            }));
        }
        if args.flags.workspaces {
            entries.extend(self.app_shell.workspaces().iter().map(|workspace| {
                let name = workspace.name().to_owned();
                WindowCommandPaletteEntry::Augmented(NativeCommandPaletteEntry {
                    brief: format!("Switch To Workspace: {name}"),
                    doc: None,
                    icon: None,
                    key_assignment: None,
                    action: WindowCommand::SwitchToWorkspaceName(name),
                })
            }));
        }
        entries
    }

    fn user_key_assignment_entries(&self) -> Vec<WindowCommandPaletteEntry> {
        self.key_assignments
            .iter()
            .map(|assignment| {
                window_key_assignment_entry(&assignment.keys, assignment.command.clone())
            })
            .collect()
    }

    fn command_palette_set_query(&mut self, query: String) {
        let Some(palette) = self.command_palette.as_mut() else {
            return;
        };
        palette.query = query;
        palette.selected = 0;
        palette.launcher_shortcut_prefix.clear();
        self.apply_window_title();
    }

    fn command_palette_move_selection(&mut self, delta: isize) {
        let entries = self.command_palette_filtered_entries();
        if entries.is_empty() {
            return;
        }

        let Some(palette) = self.command_palette.as_mut() else {
            return;
        };

        let len = isize::try_from(entries.len()).unwrap_or(1);
        let current = isize::try_from(palette.selected).unwrap_or(0);
        palette.selected = usize::try_from((current + delta).rem_euclid(len)).unwrap_or(0);
        self.apply_window_title();
    }

    fn launcher_shortcut_for_key(&self, text: &str) -> Option<WindowLauncherShortcut> {
        let palette = self.command_palette.as_ref()?;
        if !palette.query.is_empty() {
            return None;
        }

        let args = palette.launcher_args.as_ref()?;
        if args.flags.fuzzy || palette.launcher_fuzzy_filter {
            return None;
        }

        let alphabet = args.alphabet.as_deref().unwrap_or(&self.launcher_alphabet);
        let mut text_chars = text.chars();
        let target = text_chars.next()?;
        if text_chars.next().is_some() {
            return None;
        }
        let target = target.to_lowercase().to_string();
        let candidate = format!("{}{}", palette.launcher_shortcut_prefix, target);
        let entries = self.launcher_entries(args);
        let labels = quick_select_labels_for_alphabet(alphabet, entries.len());

        for (entry, label) in entries.iter().cloned().zip(labels.iter()) {
            if label == &candidate {
                return Some(WindowLauncherShortcut::Execute(Box::new(entry)));
            }
        }

        labels
            .iter()
            .any(|label| label.starts_with(&candidate))
            .then_some(WindowLauncherShortcut::Pending(candidate))
    }

    fn launcher_default_mode_vi_delta(&self, text: &str) -> Option<isize> {
        let palette = self.command_palette.as_ref()?;
        if !palette.query.is_empty() || !palette.launcher_shortcut_prefix.is_empty() {
            return None;
        }

        let args = palette.launcher_args.as_ref()?;
        if args.flags.fuzzy || palette.launcher_fuzzy_filter {
            return None;
        }

        match text {
            "j" => Some(1),
            "k" => Some(-1),
            _ => None,
        }
    }

    fn launcher_should_enter_fuzzy_filter_mode(&self, text: &str) -> bool {
        let Some(palette) = self.command_palette.as_ref() else {
            return false;
        };
        let Some(args) = palette.launcher_args.as_ref() else {
            return false;
        };

        text == "/"
            && !args.flags.fuzzy
            && !palette.launcher_fuzzy_filter
            && palette.query.is_empty()
            && palette.launcher_shortcut_prefix.is_empty()
    }

    #[allow(clippy::too_many_lines)]
    fn command_palette_apply_command(
        &mut self,
        command: WindowCommand,
    ) -> Result<(), AppShellError> {
        if self.dispatch_tab_command(&command)? {
            return Ok(());
        }

        if let WindowCommand::SpawnCommandInNewWindow(spawn_command) = &command {
            let window_position = spawn_command.window_position.clone();
            let launch = self.supported_pane_launch(spawn_command.clone())?;
            self.dispatch_spawn_window_or_preferred_tab(Some(launch), window_position)?;
            return Ok(());
        }

        if let WindowCommand::SpawnCommandOptionsInNewWindow(spawn_options) = &command {
            let window_position = spawn_options.window_position.clone();
            let launch = self.default_pane_launch_with_options(spawn_options.clone())?;
            self.dispatch_spawn_window_or_preferred_tab(Some(launch), window_position)?;
            return Ok(());
        }

        if command == WindowCommand::SpawnWindow {
            let palette_query = self
                .command_palette
                .as_ref()
                .map(|palette| palette.query.as_str());
            let spawn_command = palette_query.and_then(spawn_command_in_new_window_from_query);
            let spawn_options =
                palette_query.and_then(spawn_command_options_in_new_window_from_query);
            let window_position = spawn_command
                .as_ref()
                .and_then(|command| command.window_position.clone())
                .or_else(|| {
                    spawn_options
                        .as_ref()
                        .and_then(|options| options.window_position.clone())
                });
            let launch = match (spawn_command, spawn_options) {
                (Some(command), _) => Some(self.supported_pane_launch(command)?),
                (None, Some(options)) => Some(self.default_pane_launch_with_options(options)?),
                (None, None) => None,
            };
            self.dispatch_spawn_window_or_preferred_tab(launch, window_position)?;
            return Ok(());
        }

        let action = match command {
            WindowCommand::ActivateCommandPalette => {
                self.enter_command_palette_mode();
                return Ok(());
            }
            WindowCommand::ActivateCopyMode | WindowCommand::EnterCopyMode => {
                self.enter_copy_mode();
                return Ok(());
            }
            WindowCommand::CopyMode(assignment) => {
                self.perform_copy_mode_assignment(assignment);
                return Ok(());
            }
            WindowCommand::EnterQuickSelect => {
                let query = self
                    .command_palette
                    .as_ref()
                    .map(|palette| palette.query.clone());
                let options = query
                    .as_deref()
                    .map(quick_select_options_from_query)
                    .unwrap_or_default();
                self.enter_quick_select_mode_with_options(options);
                return Ok(());
            }
            WindowCommand::QuickSelect(options) | WindowCommand::QuickSelectArgs(options) => {
                self.enter_quick_select_mode_with_options(options);
                return Ok(());
            }
            WindowCommand::ShowLauncherArgs(args) => {
                self.enter_launcher_mode_with_args(args);
                return Ok(());
            }
            WindowCommand::ShowLauncher => {
                self.enter_launcher_mode();
                return Ok(());
            }
            WindowCommand::PaneSelect(options) => {
                self.enter_pane_select_mode_with_action(options);
                return Ok(());
            }
            WindowCommand::PromptInputLine(options) => {
                self.enter_prompt_input_line_mode(options);
                return Ok(());
            }
            WindowCommand::InputSelector(options) => {
                self.enter_input_selector_mode(options);
                return Ok(());
            }
            WindowCommand::Confirmation(options) => {
                self.enter_confirmation_mode(options);
                return Ok(());
            }
            WindowCommand::EmitEvent(event) => {
                self.emit_event(event);
                return Ok(());
            }
            WindowCommand::ActivateKeyTable(key_table) => {
                self.activate_key_table(key_table);
                return Ok(());
            }
            WindowCommand::PopKeyTable => {
                self.pop_key_table();
                return Ok(());
            }
            WindowCommand::ClearKeyTableStack => {
                self.clear_key_table_stack();
                return Ok(());
            }
            WindowCommand::Multiple(commands) => {
                for command in commands {
                    self.command_palette_apply_command(command)?;
                }
                return Ok(());
            }
            WindowCommand::DisableDefaultAssignment | WindowCommand::Nop => AppAction::Nop,
            WindowCommand::EnterPaneSelect => {
                let alphabet = self.command_palette.as_ref().and_then(|palette| {
                    pane_select_alphabet_from_query(&palette.query)
                        .or_else(|| pane_select_activate_alphabet_from_query(&palette.query))
                });
                if let Some(alphabet) = alphabet {
                    self.enter_pane_select_mode_with_alphabet(
                        WindowPaneSelectMode::Activate,
                        false,
                        &alphabet,
                    );
                } else {
                    self.enter_pane_select_mode();
                }
                return Ok(());
            }
            WindowCommand::EnterPaneSelectShowPaneIds => {
                let alphabet = self.command_palette.as_ref().and_then(|palette| {
                    pane_select_show_pane_ids_alphabet_from_query(&palette.query).or_else(|| {
                        pane_select_activate_show_pane_ids_alphabet_from_query(&palette.query)
                    })
                });
                if let Some(alphabet) = alphabet {
                    self.enter_pane_select_mode_with_alphabet(
                        WindowPaneSelectMode::Activate,
                        true,
                        &alphabet,
                    );
                } else {
                    self.enter_pane_select_mode_with_options(WindowPaneSelectMode::Activate, true);
                }
                return Ok(());
            }
            WindowCommand::EnterPaneSwap => {
                let pane_select_show_ids_query = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| pane_select_mode_show_pane_ids_from_query(&palette.query))
                    .filter(|query| query.command == WindowCommand::EnterPaneSwap);
                if let Some(query) = pane_select_show_ids_query {
                    if let Some(alphabet) = query.alphabet {
                        self.enter_pane_select_mode_with_alphabet(query.mode, true, &alphabet);
                    } else {
                        self.enter_pane_select_mode_with_options(query.mode, true);
                    }
                    return Ok(());
                }
                let pane_select_query = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| pane_select_mode_alphabet_from_query(&palette.query))
                    .filter(|query| query.command == WindowCommand::EnterPaneSwap);
                if let Some(pane_select_query) = pane_select_query {
                    self.enter_pane_select_mode_with_alphabet(
                        pane_select_query.mode,
                        false,
                        &pane_select_query.alphabet,
                    );
                } else {
                    self.enter_pane_select_mode_with_mode(WindowPaneSelectMode::SwapWithActive);
                }
                return Ok(());
            }
            WindowCommand::EnterPaneSwapKeepFocus => {
                let pane_select_show_ids_query = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| pane_select_mode_show_pane_ids_from_query(&palette.query))
                    .filter(|query| query.command == WindowCommand::EnterPaneSwapKeepFocus);
                if let Some(query) = pane_select_show_ids_query {
                    if let Some(alphabet) = query.alphabet {
                        self.enter_pane_select_mode_with_alphabet(query.mode, true, &alphabet);
                    } else {
                        self.enter_pane_select_mode_with_options(query.mode, true);
                    }
                    return Ok(());
                }
                let pane_select_query = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| pane_select_mode_alphabet_from_query(&palette.query))
                    .filter(|query| query.command == WindowCommand::EnterPaneSwapKeepFocus);
                if let Some(pane_select_query) = pane_select_query {
                    self.enter_pane_select_mode_with_alphabet(
                        pane_select_query.mode,
                        false,
                        &pane_select_query.alphabet,
                    );
                } else {
                    self.enter_pane_select_mode_with_mode(
                        WindowPaneSelectMode::SwapWithActiveKeepFocus,
                    );
                }
                return Ok(());
            }
            WindowCommand::EnterPaneMoveToNewTab => {
                let pane_select_show_ids_query = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| pane_select_mode_show_pane_ids_from_query(&palette.query))
                    .filter(|query| query.command == WindowCommand::EnterPaneMoveToNewTab);
                if let Some(query) = pane_select_show_ids_query {
                    if let Some(alphabet) = query.alphabet {
                        self.enter_pane_select_mode_with_alphabet(query.mode, true, &alphabet);
                    } else {
                        self.enter_pane_select_mode_with_options(query.mode, true);
                    }
                    return Ok(());
                }
                let pane_select_query = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| pane_select_mode_alphabet_from_query(&palette.query))
                    .filter(|query| query.command == WindowCommand::EnterPaneMoveToNewTab);
                if let Some(pane_select_query) = pane_select_query {
                    self.enter_pane_select_mode_with_alphabet(
                        pane_select_query.mode,
                        false,
                        &pane_select_query.alphabet,
                    );
                } else {
                    self.enter_pane_select_mode_with_mode(WindowPaneSelectMode::MoveToNewTab);
                }
                return Ok(());
            }
            WindowCommand::EnterPaneMoveToNewWindow => {
                let pane_select_show_ids_query = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| pane_select_mode_show_pane_ids_from_query(&palette.query))
                    .filter(|query| query.command == WindowCommand::EnterPaneMoveToNewWindow);
                if let Some(query) = pane_select_show_ids_query {
                    if let Some(alphabet) = query.alphabet {
                        self.enter_pane_select_mode_with_alphabet(query.mode, true, &alphabet);
                    } else {
                        self.enter_pane_select_mode_with_options(query.mode, true);
                    }
                    return Ok(());
                }
                let pane_select_query = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| pane_select_mode_alphabet_from_query(&palette.query))
                    .filter(|query| query.command == WindowCommand::EnterPaneMoveToNewWindow);
                if let Some(pane_select_query) = pane_select_query {
                    self.enter_pane_select_mode_with_alphabet(
                        pane_select_query.mode,
                        false,
                        &pane_select_query.alphabet,
                    );
                } else {
                    self.enter_pane_select_mode_with_mode(WindowPaneSelectMode::MoveToNewWindow);
                }
                return Ok(());
            }
            command => return self.command_palette_apply_command_part2(command),
        };

        self.dispatch_app_action(action)
    }

#[expect(
    clippy::too_many_lines,
    reason = "the command dispatcher preserves explicit compatibility priority"
)]
#[expect(
    clippy::needless_return,
    reason = "the command dispatcher exits each selected compatibility branch immediately"
)]
fn command_palette_apply_command_part2(
        &mut self,
        command: WindowCommand,
    ) -> Result<(), AppShellError> {
        match command {
            WindowCommand::ShowTabNavigator => {
                self.enter_tab_navigator_mode();
                return Ok(());
            }
            WindowCommand::Search(search_query) => {
                self.enter_search_mode_with_query(&search_query);
                return Ok(());
            }
            WindowCommand::EnterSearch => {
                let search_query = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| search_query_from_query(&palette.query));
                if let Some(search_query) = search_query {
                    self.enter_search_mode_with_query(&search_query);
                } else {
                    self.enter_search_mode();
                }
                return Ok(());
            }
            WindowCommand::CharSelect => {
                self.enter_char_select_mode();
                return Ok(());
            }
            WindowCommand::CharSelectArgs(options) => {
                self.enter_char_select_mode_with_options(options);
                return Ok(());
            }
            WindowCommand::ClearScrollback(mode) => {
                match mode {
                    WindowClearScrollbackMode::ScrollbackOnly => self.clear_scrollback(),
                    WindowClearScrollbackMode::ScrollbackAndViewport => {
                        self.clear_scrollback_and_viewport();
                    }
                }
                return Ok(());
            }
            WindowCommand::ClearScrollbackAndViewport => {
                self.clear_scrollback_and_viewport();
                return Ok(());
            }
            WindowCommand::ClearSelection => {
                self.clear_selection();
                return Ok(());
            }
            WindowCommand::SelectTextAtMouseCursorCell => {
                self.select_text_at_mouse_cursor(WindowMouseSelectionMode::Cell);
                return Ok(());
            }
            WindowCommand::SelectTextAtMouseCursorWord => {
                self.select_text_at_mouse_cursor(WindowMouseSelectionMode::Word);
                return Ok(());
            }
            WindowCommand::SelectTextAtMouseCursorLine => {
                self.select_text_at_mouse_cursor(WindowMouseSelectionMode::Line);
                return Ok(());
            }
            WindowCommand::SelectTextAtMouseCursorBlock => {
                self.select_text_at_mouse_cursor(WindowMouseSelectionMode::Block);
                return Ok(());
            }
            WindowCommand::SelectTextAtMouseCursorSemanticZone => {
                self.select_text_at_mouse_cursor(WindowMouseSelectionMode::SemanticZone);
                return Ok(());
            }
            WindowCommand::SelectTextAtMouseCursor(mode) => {
                self.select_text_at_mouse_cursor(mode);
                return Ok(());
            }
            WindowCommand::ExtendSelectionToMouseCursorCell => {
                self.extend_selection_to_mouse_cursor(WindowMouseSelectionMode::Cell);
                return Ok(());
            }
            WindowCommand::ExtendSelectionToMouseCursorWord => {
                self.extend_selection_to_mouse_cursor(WindowMouseSelectionMode::Word);
                return Ok(());
            }
            WindowCommand::ExtendSelectionToMouseCursorLine => {
                self.extend_selection_to_mouse_cursor(WindowMouseSelectionMode::Line);
                return Ok(());
            }
            WindowCommand::ExtendSelectionToMouseCursorBlock => {
                self.extend_selection_to_mouse_cursor(WindowMouseSelectionMode::Block);
                return Ok(());
            }
            WindowCommand::ExtendSelectionToMouseCursorSemanticZone => {
                self.extend_selection_to_mouse_cursor(WindowMouseSelectionMode::SemanticZone);
                return Ok(());
            }
            WindowCommand::ExtendSelectionToMouseCursor(mode) => {
                self.extend_selection_to_mouse_cursor(mode);
                return Ok(());
            }
            WindowCommand::CompleteSelection => {
                self.complete_selection_to_clipboard_and_primary_selection();
                return Ok(());
            }
            WindowCommand::CompleteSelectionTo(destination) => {
                self.complete_selection_to(destination);
                return Ok(());
            }
            WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor => {
                self.complete_selection_or_open_link_at_mouse_cursor();
                return Ok(());
            }
            WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(destination) => {
                self.complete_selection_or_open_link_at_mouse_cursor_to(destination);
                return Ok(());
            }
            WindowCommand::OpenLinkAtMouseCursor => {
                self.open_link_at_mouse_cursor();
                return Ok(());
            }
            WindowCommand::OpenUri(uri) => {
                self.open_uri(&uri);
                return Ok(());
            }
            WindowCommand::CopyToClipboard => {
                self.copy_selection_to_clipboard();
                return Ok(());
            }
            WindowCommand::CopyToPrimarySelection => {
                self.copy_selection_to_primary_selection();
                return Ok(());
            }
            WindowCommand::CopyToClipboardAndPrimarySelection => {
                self.copy_selection_to_clipboard_and_primary_selection();
                return Ok(());
            }
            WindowCommand::CopyTo(destination) => {
                self.copy_selection_to(destination);
                return Ok(());
            }
            WindowCommand::CopyTextTo { text, destination } => {
                self.write_text_to_copy_destination(&text, destination);
                return Ok(());
            }
            WindowCommand::Copy => {
                self.copy_selection_to(WindowCopyDestination::Clipboard);
                return Ok(());
            }
            WindowCommand::PasteFromClipboard => {
                if let Err(error) = self.handle_window_paste() {
                    eprintln!("paste from clipboard failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::PasteFromPrimarySelection => {
                if let Err(error) = self.handle_window_primary_selection_paste() {
                    eprintln!("paste from primary selection failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::PasteFrom(source) => {
                if let Err(error) = self.handle_window_paste_from(source) {
                    eprintln!("paste from source failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::Paste => {
                if let Err(error) = self.handle_window_paste_from(WindowPasteSource::Clipboard) {
                    eprintln!("paste failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::PastePrimarySelection => {
                if let Err(error) =
                    self.handle_window_paste_from(WindowPasteSource::PrimarySelection)
                {
                    eprintln!("paste primary selection failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::SendString(value) => {
                if let Err(error) = self.write_pty_bytes(value.as_bytes()) {
                    eprintln!("send string failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::SendPaste(value) => {
                let bytes = encode_window_paste(
                    &value,
                    self.runtime.bracketed_paste(),
                    self.canonicalize_pasted_newlines,
                );
                if let Err(error) = self.write_pty_bytes(&bytes) {
                    eprintln!("send paste failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::SendKey(send_key) => {
                if let Err(error) = self.send_key_to_active_pane(&send_key) {
                    eprintln!("send key failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::RestartPane => {
                let pane_id = self.app_shell.active_pane_id();
                if let Err(error) = self.restart_pane_runtime(pane_id) {
                    eprintln!("restart pane failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::InspectPane => {
                self.request_pane_inspection(self.app_shell.active_pane_id());
                return Ok(());
            }
            WindowCommand::ReloadConfiguration => {
                self.request_reload_configuration();
                return Ok(());
            }
            WindowCommand::ToggleFullScreen => {
                self.toggle_full_screen();
                return Ok(());
            }
            WindowCommand::StartWindowDrag => {
                self.start_window_drag();
                return Ok(());
            }
            WindowCommand::ActivateWindow(index) => {
                self.request_activate_window(index);
                return Ok(());
            }
            WindowCommand::ActivateWindowRelative(offset) => {
                self.request_activate_window_relative(offset, true);
                return Ok(());
            }
            WindowCommand::ActivateWindowRelativeNoWrap(offset) => {
                self.request_activate_window_relative(offset, false);
                return Ok(());
            }
            WindowCommand::SetWindowLevel(level) => {
                self.set_window_level(level);
                return Ok(());
            }
            WindowCommand::ToggleAlwaysOnTop => {
                self.toggle_window_level(NativeWindowLevel::AlwaysOnTop);
                return Ok(());
            }
            WindowCommand::ToggleAlwaysOnBottom => {
                self.toggle_window_level(NativeWindowLevel::AlwaysOnBottom);
                return Ok(());
            }
            WindowCommand::Show => {
                self.show_window();
                return Ok(());
            }
            WindowCommand::Hide => {
                self.hide_window();
                return Ok(());
            }
            WindowCommand::HideApplication => {
                self.hide_application();
                return Ok(());
            }
            WindowCommand::QuitApplication => {
                self.request_application_quit();
                return Ok(());
            }
            WindowCommand::DecreaseFontSize => {
                self.adjust_font_size(WindowFontSizeAction::Decrease);
                return Ok(());
            }
            WindowCommand::IncreaseFontSize => {
                self.adjust_font_size(WindowFontSizeAction::Increase);
                return Ok(());
            }
            WindowCommand::ResetFontSize => {
                self.adjust_font_size(WindowFontSizeAction::Reset);
                return Ok(());
            }
            WindowCommand::ResetFontAndWindowSize => {
                self.reset_font_and_window_size();
                return Ok(());
            }
            WindowCommand::ShowDebugOverlay => {
                self.show_debug_overlay();
                return Ok(());
            }
            command => return self.command_palette_apply_command_part3(command),
        }
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the command dispatcher preserves explicit compatibility priority"
)]
fn command_palette_apply_command_part3(
        &mut self,
        command: WindowCommand,
    ) -> Result<(), AppShellError> {
        let action = match command {
            WindowCommand::ResetTerminal => {
                if let Err(error) = self.reset_terminal() {
                    eprintln!("reset terminal failed: {error}");
                }
                return Ok(());
            }
            WindowCommand::ScrollToTop => {
                self.set_scrollback_offset(self.runtime.terminal().scrollback().len());
                return Ok(());
            }
            WindowCommand::ScrollToBottom => {
                self.set_scrollback_offset(0);
                return Ok(());
            }
            WindowCommand::ScrollByPage(amount) => {
                self.scroll_viewport_lines(amount.viewport_lines(self.viewport_page_rows()));
                return Ok(());
            }
            WindowCommand::ScrollByLine(amount) => {
                self.scroll_viewport_lines(amount.saturating_neg());
                return Ok(());
            }
            WindowCommand::ScrollByCurrentEventWheelDelta => {
                if let Some(delta) = self.current_mouse_wheel_delta {
                    self.handle_mouse_wheel(delta);
                }
                return Ok(());
            }
            WindowCommand::ScrollToPrompt(amount) => {
                self.scroll_to_prompt(amount);
                return Ok(());
            }
            WindowCommand::ScrollPageUp => {
                self.scroll_viewport_lines(self.viewport_page_rows());
                return Ok(());
            }
            WindowCommand::ScrollPageDown => {
                self.scroll_viewport_lines(-self.viewport_page_rows());
                return Ok(());
            }
            WindowCommand::ScrollLineUp => {
                self.scroll_viewport_lines(1);
                return Ok(());
            }
            WindowCommand::ScrollLineDown => {
                self.scroll_viewport_lines(-1);
                return Ok(());
            }
            WindowCommand::ScrollToPreviousPrompt => {
                self.scroll_to_prompt(-1);
                return Ok(());
            }
            WindowCommand::ScrollToNextPrompt => {
                self.scroll_to_prompt(1);
                return Ok(());
            }
            WindowCommand::NewTab => {
                let palette_query = self
                    .command_palette
                    .as_ref()
                    .map(|palette| palette.query.as_str());
                let spawn_command = palette_query.and_then(spawn_command_in_new_tab_from_query);
                let spawn_options =
                    palette_query.and_then(spawn_command_options_in_new_tab_from_query);
                let launch = match (spawn_command, spawn_options) {
                    (Some(command), _) => Some(self.supported_pane_launch(command)?),
                    (None, Some(options)) => Some(self.default_pane_launch_with_options(options)?),
                    (None, None) => None,
                };
                AppAction::NewTab { launch }
            }
            WindowCommand::SpawnTab(domain) => {
                if !domain.is_supported_local_domain(&self.default_domain) {
                    return Err(AppShellError::UnsupportedAction);
                }
                AppAction::NewTab { launch: None }
            }
            WindowCommand::AttachDomain(domain) => {
                if !is_attach_domain_supported_locally(&domain, &self.default_domain) {
                    return Err(AppShellError::UnsupportedAction);
                }
                AppAction::NewTab { launch: None }
            }
            WindowCommand::DetachDomain(selector) => {
                if selector.is_supported_local_domain(&self.default_domain) {
                    return Ok(());
                }
                return Err(AppShellError::UnsupportedAction);
            }
            WindowCommand::SpawnCommandInNewTab(spawn_command) => AppAction::NewTab {
                launch: Some(self.supported_pane_launch(spawn_command)?),
            },
            WindowCommand::SpawnCommandOptionsInNewTab(spawn_options) => AppAction::NewTab {
                launch: Some(self.default_pane_launch_with_options(spawn_options)?),
            },
            WindowCommand::SpawnWindow => {
                let spawn_command = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| spawn_command_in_new_window_from_query(&palette.query));
                let has_window_position = spawn_command
                    .as_ref()
                    .is_some_and(|command| command.window_position.is_some());
                let launch = spawn_command
                    .map(|command| self.supported_pane_launch(command))
                    .transpose()?;
                self.spawn_window_or_preferred_tab_action(launch, has_window_position)
            }
            WindowCommand::SpawnCommandOptionsInNewWindow(spawn_options) => {
                let has_window_position = spawn_options.window_position.is_some();
                let launch = Some(self.default_pane_launch_with_options(spawn_options)?);
                self.spawn_window_or_preferred_tab_action(launch, has_window_position)
            }
            WindowCommand::SpawnCommandInNewWindow(spawn_command) => {
                let has_window_position = spawn_command.window_position.is_some();
                let launch = Some(self.supported_pane_launch(spawn_command)?);
                self.spawn_window_or_preferred_tab_action(launch, has_window_position)
            }
            WindowCommand::ActivateLastTab => AppAction::ActivateLastTab,
            WindowCommand::CloseCurrentTab { confirm: true } => {
                self.request_close_confirmation_or_close(WindowCloseTarget::Tab(
                    self.app_shell.active_tab_id(),
                ));
                return Ok(());
            }
            WindowCommand::CloseCurrentPane { confirm: true } => {
                self.request_close_confirmation_or_close(WindowCloseTarget::Pane(
                    self.app_shell.active_pane_id(),
                ));
                return Ok(());
            }
            WindowCommand::CloseCurrentTab { confirm: false } | WindowCommand::CloseTab => {
                AppAction::CloseTab {
                    tab: self.app_shell.active_tab_id(),
                    switch_to_last_active: self.switch_to_last_active_tab_when_closing_tab,
                }
            }
            WindowCommand::ActivateTab1 => AppAction::ActivateTabIndex { index: 0 },
            WindowCommand::ActivateTab2 => AppAction::ActivateTabIndex { index: 1 },
            WindowCommand::ActivateTab3 => AppAction::ActivateTabIndex { index: 2 },
            WindowCommand::ActivateTab4 => AppAction::ActivateTabIndex { index: 3 },
            WindowCommand::ActivateTab5 => AppAction::ActivateTabIndex { index: 4 },
            WindowCommand::ActivateTab6 => AppAction::ActivateTabIndex { index: 5 },
            WindowCommand::ActivateTab7 => AppAction::ActivateTabIndex { index: 6 },
            WindowCommand::ActivateTab8 => AppAction::ActivateTabIndex { index: 7 },
            WindowCommand::ActivateTab9 => AppAction::ActivateTabIndex { index: -1 },
            WindowCommand::ActivateTab(index) => AppAction::ActivateTabIndex { index },
            WindowCommand::ActivateTabRelative(offset) => AppAction::ActivateTabRelative { offset },
            WindowCommand::ActivateTabRelativeNoWrap(offset) => {
                AppAction::ActivateTabRelativeNoWrap { offset }
            }
            WindowCommand::NextTabNoWrap => AppAction::ActivateTabRelativeNoWrap { offset: 1 },
            WindowCommand::PreviousTabNoWrap => AppAction::ActivateTabRelativeNoWrap { offset: -1 },
            WindowCommand::NextTab => AppAction::ActivateTabRelative { offset: 1 },
            WindowCommand::PreviousTab => AppAction::ActivateTabRelative { offset: -1 },
            WindowCommand::MoveTabRelative(offset) => AppAction::MoveTabRelative { offset },
            WindowCommand::MoveTabRelativeLeft => AppAction::MoveTabRelative { offset: -1 },
            WindowCommand::MoveTabRelativeRight => AppAction::MoveTabRelative { offset: 1 },
            WindowCommand::MoveTab(index) => AppAction::MoveTab { index },
            WindowCommand::MoveTabTo1 => AppAction::MoveTab { index: 0 },
            WindowCommand::MoveTabTo2 => AppAction::MoveTab { index: 1 },
            WindowCommand::MoveTabTo3 => AppAction::MoveTab { index: 2 },
            WindowCommand::MoveTabTo4 => AppAction::MoveTab { index: 3 },
            WindowCommand::MoveTabTo5 => AppAction::MoveTab { index: 4 },
            WindowCommand::MoveTabTo6 => AppAction::MoveTab { index: 5 },
            WindowCommand::MoveTabTo7 => AppAction::MoveTab { index: 6 },
            WindowCommand::MoveTabTo8 => AppAction::MoveTab { index: 7 },
            WindowCommand::RotatePanes(direction) => AppAction::RotatePanes { direction },
            WindowCommand::RotatePanesClockwise => AppAction::RotatePanes {
                direction: PaneRotationDirection::Clockwise,
            },
            WindowCommand::RotatePanesCounterClockwise => AppAction::RotatePanes {
                direction: PaneRotationDirection::CounterClockwise,
            },
            WindowCommand::SplitRight | WindowCommand::SplitHorizontal => {
                if let Some(split_pane) = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| split_horizontal_options_from_query(&palette.query))
                {
                    return self
                        .split_pane_app_action(split_pane)
                        .and_then(|action| self.dispatch_app_action(action));
                }
                let launch = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| split_horizontal_command_from_query(&palette.query))
                    .map(|command| self.supported_pane_launch(command))
                    .transpose()?;
                AppAction::SplitPane {
                    pane: self.app_shell.active_pane_id(),
                    direction: SplitDirection::Right,
                    launch,
                }
            }
            WindowCommand::SplitDown | WindowCommand::SplitVertical => {
                if let Some(split_pane) = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| split_vertical_options_from_query(&palette.query))
                {
                    return self
                        .split_pane_app_action(split_pane)
                        .and_then(|action| self.dispatch_app_action(action));
                }
                let launch = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| split_vertical_command_from_query(&palette.query))
                    .map(|command| self.supported_pane_launch(command))
                    .transpose()?;
                AppAction::SplitPane {
                    pane: self.app_shell.active_pane_id(),
                    direction: SplitDirection::Down,
                    launch,
                }
            }
            command => return self.command_palette_apply_command_part4(command),
        };

        self.dispatch_app_action(action)
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the command dispatcher preserves explicit compatibility priority"
)]
fn command_palette_apply_command_part4(
        &mut self,
        command: WindowCommand,
    ) -> Result<(), AppShellError> {
        let action = match command {
            WindowCommand::SplitPane(split_pane) => self.split_pane_app_action(split_pane)?,
            WindowCommand::CloseCurrentPane { confirm: false } | WindowCommand::ClosePane => {
                AppAction::ClosePane {
                    pane: self.app_shell.active_pane_id(),
                }
            }
            WindowCommand::ActivatePaneLeft => AppAction::ActivatePaneDirection {
                direction: PaneDirection::Left,
            },
            WindowCommand::ActivatePaneRight => AppAction::ActivatePaneDirection {
                direction: PaneDirection::Right,
            },
            WindowCommand::ActivatePaneUp => AppAction::ActivatePaneDirection {
                direction: PaneDirection::Up,
            },
            WindowCommand::ActivatePaneDown => AppAction::ActivatePaneDirection {
                direction: PaneDirection::Down,
            },
            WindowCommand::ActivatePaneDirection(direction) => {
                AppAction::ActivatePaneDirection { direction }
            }
            WindowCommand::ActivateTabId(tab) => AppAction::ActivateTab { tab },
            WindowCommand::ActivatePaneByIndex(index) => AppAction::ActivatePaneByIndex { index },
            WindowCommand::ActivatePane1 => AppAction::ActivatePaneByIndex { index: 0 },
            WindowCommand::ActivatePane2 => AppAction::ActivatePaneByIndex { index: 1 },
            WindowCommand::ActivatePane3 => AppAction::ActivatePaneByIndex { index: 2 },
            WindowCommand::ActivatePane4 => AppAction::ActivatePaneByIndex { index: 3 },
            WindowCommand::ActivatePane5 => AppAction::ActivatePaneByIndex { index: 4 },
            WindowCommand::ActivatePane6 => AppAction::ActivatePaneByIndex { index: 5 },
            WindowCommand::ActivatePane7 => AppAction::ActivatePaneByIndex { index: 6 },
            WindowCommand::ActivatePane8 => AppAction::ActivatePaneByIndex { index: 7 },
            WindowCommand::NextPane => AppAction::ActivatePaneDirection {
                direction: PaneDirection::Next,
            },
            WindowCommand::PreviousPane => AppAction::ActivatePaneDirection {
                direction: PaneDirection::Previous,
            },
            WindowCommand::AdjustPaneSize { direction, amount } => AppAction::ResizePane {
                pane: self.app_shell.active_pane_id(),
                direction,
                amount,
            },
            WindowCommand::ResizePaneLeft => AppAction::ResizePane {
                pane: self.app_shell.active_pane_id(),
                direction: ResizeDirection::Left,
                amount: 1,
            },
            WindowCommand::ResizePaneRight => AppAction::ResizePane {
                pane: self.app_shell.active_pane_id(),
                direction: ResizeDirection::Right,
                amount: 1,
            },
            WindowCommand::ResizePaneUp => AppAction::ResizePane {
                pane: self.app_shell.active_pane_id(),
                direction: ResizeDirection::Up,
                amount: 1,
            },
            WindowCommand::ResizePaneDown => AppAction::ResizePane {
                pane: self.app_shell.active_pane_id(),
                direction: ResizeDirection::Down,
                amount: 1,
            },
            WindowCommand::TogglePaneZoom | WindowCommand::TogglePaneZoomState => {
                AppAction::TogglePaneZoom {
                    pane: self.app_shell.active_pane_id(),
                }
            }
            WindowCommand::SetPaneZoomState(zoomed) => AppAction::SetPaneZoomState {
                pane: self.app_shell.active_pane_id(),
                zoomed,
            },
            WindowCommand::ZoomPane => AppAction::SetPaneZoomState {
                pane: self.app_shell.active_pane_id(),
                zoomed: true,
            },
            WindowCommand::UnzoomPane => AppAction::SetPaneZoomState {
                pane: self.app_shell.active_pane_id(),
                zoomed: false,
            },
            WindowCommand::NewWorkspace => AppAction::NewWorkspace {
                name: format!("workspace-{}", self.app_shell.workspaces().len() + 1),
                launch: None,
            },
            WindowCommand::CloseWorkspace => AppAction::CloseWorkspace {
                workspace: self.app_shell.active_workspace_id(),
            },
            WindowCommand::RenameWorkspace => {
                let active_workspace = self.app_shell.active_workspace();
                let explicit_name = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| rename_workspace_name_from_query(&palette.query));
                AppAction::RenameWorkspace {
                    workspace: self.app_shell.active_workspace_id(),
                    name: explicit_name
                        .unwrap_or_else(|| format!("{} (renamed)", active_workspace.name())),
                }
            }
            WindowCommand::RenameWorkspaceTo(name) => AppAction::RenameWorkspace {
                workspace: self.app_shell.active_workspace_id(),
                name,
            },
            WindowCommand::RenameTab => {
                let active_tab = self.app_shell.active_tab();
                let explicit_title = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| rename_tab_title_from_query(&palette.query));
                let title = explicit_title.unwrap_or_else(|| {
                    let title = self
                        .tab_title_for_tab(active_tab)
                        .unwrap_or_else(|| format!("tab {}", active_tab.id().get()));
                    format!("{title} (renamed)")
                });
                AppAction::SetTabTitle {
                    tab: active_tab.id(),
                    title,
                }
            }
            WindowCommand::RenameTabTo(title) => AppAction::SetTabTitle {
                tab: self.app_shell.active_tab_id(),
                title,
            },
            WindowCommand::NextWorkspace => AppAction::SwitchWorkspaceRelative { offset: 1 },
            WindowCommand::PreviousWorkspace => AppAction::SwitchWorkspaceRelative { offset: -1 },
            WindowCommand::SwitchWorkspaceRelative(offset) => {
                AppAction::SwitchWorkspaceRelative { offset }
            }
            WindowCommand::SwitchToWorkspace => {
                let args = self
                    .command_palette
                    .as_ref()
                    .and_then(|palette| switch_workspace_options_from_query(&palette.query))
                    .unwrap_or(WindowSwitchToWorkspaceOptions {
                        name: None,
                        command: None,
                        command_options: None,
                    });
                AppAction::SwitchToWorkspace {
                    name: args.name,
                    launch: self.switch_to_workspace_launch(args.command, args.command_options)?,
                }
            }
            WindowCommand::SwitchToWorkspaceArgs(args) => AppAction::SwitchToWorkspace {
                name: args.name,
                launch: self.switch_to_workspace_launch(args.command, args.command_options)?,
            },
            WindowCommand::SwitchToWorkspaceName(name) => AppAction::SwitchToWorkspace {
                name: Some(name),
                launch: None,
            },
            _ => unreachable!("command was routed to the wrong palette reducer stage"),
        };

        self.dispatch_app_action(action)
    }


}

impl NativeWindowApp {
    fn supported_pane_launch(
        &self,
        command: WindowSpawnCommandQuery,
    ) -> Result<PaneLaunch, AppShellError> {
        command.into_supported_pane_launch(&self.default_domain)
    }

    fn default_pane_launch_with_options(
        &self,
        options: WindowSpawnCommandQueryOptions,
    ) -> Result<PaneLaunch, AppShellError> {
        if let Some(domain) = &options.domain
            && !domain.is_supported_local_domain(&self.default_domain)
        {
            return Err(AppShellError::UnsupportedAction);
        }

        let inherited = self.app_shell.active_pane().launch().for_child_pane();
        let mut launch = if options.domain.is_none()
            && matches!(inherited.domain(), PaneLaunchDomain::Ssh(_))
        {
            inherited
        } else {
            self.default_prog_launch().unwrap_or(inherited)
        };
        if let Some(cwd) = options.cwd {
            launch = launch.with_cwd(cwd);
        }
        Ok(launch.with_environment(options.environment))
    }

    fn switch_to_workspace_launch(
        &self,
        command: Option<WindowSpawnCommandQuery>,
        command_options: Option<WindowSpawnCommandQueryOptions>,
    ) -> Result<Option<PaneLaunch>, AppShellError> {
        match (command, command_options) {
            (Some(command), _) => self.supported_pane_launch(command).map(Some),
            (None, Some(options)) => self.default_pane_launch_with_options(options).map(Some),
            (None, None) => Ok(None),
        }
    }

    fn prefers_spawn_window_as_tab(&self, has_window_position: bool) -> bool {
        self.prefer_to_spawn_tabs && !has_window_position
    }

    fn spawn_window_or_preferred_tab_action(
        &self,
        launch: Option<PaneLaunch>,
        has_window_position: bool,
    ) -> AppAction {
        if self.prefers_spawn_window_as_tab(has_window_position) {
            AppAction::NewTab { launch }
        } else {
            AppAction::SpawnWindow { launch }
        }
    }

    fn dispatch_spawn_window_or_preferred_tab(
        &mut self,
        launch: Option<PaneLaunch>,
        window_position: Option<WindowPosition>,
    ) -> Result<(), AppShellError> {
        let has_window_position = window_position.is_some();
        let route_to_tab = self.prefers_spawn_window_as_tab(has_window_position);
        let action = self.spawn_window_or_preferred_tab_action(launch, has_window_position);
        self.dispatch_app_action(action)?;
        if !route_to_tab {
            self.record_latest_pending_window_position(window_position);
        }
        Ok(())
    }

    fn record_latest_pending_window_position(&mut self, position: Option<WindowPosition>) {
        let Some(position) = position else {
            return;
        };
        if let Some(pending_window_id) = self
            .app_shell
            .pending_windows()
            .last()
            .map(rssh_core::app_shell::PendingWindow::id)
        {
            self.pending_window_positions
                .insert(pending_window_id, position);
        }
    }

    fn split_pane_app_action(
        &self,
        split_pane: WindowSplitPaneOptions,
    ) -> Result<AppAction, AppShellError> {
        if let Some(domain) = &split_pane.domain
            && !domain.is_supported_local_domain(&self.default_domain)
        {
            return Err(AppShellError::UnsupportedAction);
        }
        let source_size_delta = split_pane
            .size
            .map(|size| {
                self.split_pane_source_size_delta_for_active_pane(split_pane.direction, size)
            })
            .unwrap_or_default();
        let launch = match split_pane.command {
            Some(mut command) => {
                if command.domain.is_none() {
                    command.domain = split_pane.domain;
                }
                Some(self.supported_pane_launch(command)?)
            }
            None => split_pane
                .command_options
                .map(|options| self.default_pane_launch_with_options(options))
                .transpose()?,
        };
        if split_pane.top_level {
            Ok(AppAction::SplitTopLevelPaneWithSize {
                direction: split_pane.direction,
                launch,
                source_size_delta,
            })
        } else {
            Ok(AppAction::SplitPaneWithSize {
                pane: self.app_shell.active_pane_id(),
                direction: split_pane.direction,
                launch,
                source_size_delta,
            })
        }
    }

    fn command_palette_execute(&mut self, command: WindowCommand) -> bool {
        let target = self.deferred_wheel_context.take();
        let command = if target.is_some() {
            self.resolve_wheel_palette_command(command)
        } else {
            command
        };
        let frecency_label = match &command {
            WindowCommand::ActivateCommandPalette => None,
            command => Some(command.label().to_owned()),
        };
        if command == WindowCommand::InspectPane && self.command_palette.is_some() {
            let pane_id =
                target.map_or_else(|| self.app_shell.active_pane_id(), |target| target.pane_id);
            let succeeded = self.request_pane_inspection_from(
                pane_id,
                PaneInspectionRequestSource::CommandPaletteExecute,
            );
            if succeeded {
                self.exit_command_palette_mode();
                if let Some(frecency_label) = frecency_label.as_deref() {
                    self.record_command_palette_label(frecency_label);
                }
            } else {
                self.deferred_wheel_context = target;
            }
            return succeeded;
        }
        let succeeded = match command {
            WindowCommand::ShowLauncherArgs(args) => {
                self.enter_launcher_mode_with_args(args);
                self.deferred_wheel_context = target;
                true
            }
            WindowCommand::ShowLauncher => {
                self.enter_launcher_mode();
                self.deferred_wheel_context = target;
                true
            }
            WindowCommand::ActivateCommandPalette => {
                self.exit_command_palette_mode();
                self.enter_command_palette_mode();
                self.deferred_wheel_context = target;
                true
            }
            command => match if let Some(target) = target {
                self.apply_command_for_target_context(target, command)
                    .map_err(|error| error.to_string())
            } else {
                self.command_palette_apply_command(command)
                    .map_err(|error| format!("{error:?}"))
            } {
                Ok(()) => {
                    if self.command_palette.is_some() {
                        self.exit_command_palette_mode();
                    }
                    true
                }
                Err(error) => {
                    if self.command_palette.is_some() {
                        self.deferred_wheel_context = target;
                    }
                    eprintln!("command palette action failed: {error}");
                    false
                }
            },
        };
        if succeeded && let Some(frecency_label) = frecency_label.as_deref() {
            self.record_command_palette_label(frecency_label);
        }
        succeeded
    }

    fn resolve_wheel_palette_command(&self, command: WindowCommand) -> WindowCommand {
        let Some(query) = self
            .command_palette
            .as_ref()
            .map(|palette| palette.query.as_str())
        else {
            return command;
        };
        match command {
            WindowCommand::EnterQuickSelect => {
                WindowCommand::QuickSelectArgs(quick_select_options_from_query(query))
            }
            WindowCommand::Search(_) | WindowCommand::EnterSearch => search_query_from_query(query)
                .map(WindowCommand::Search)
                .unwrap_or(command),
            WindowCommand::SpawnWindow => spawn_command_in_new_window_from_query(query)
                .map(WindowCommand::SpawnCommandInNewWindow)
                .or_else(|| {
                    spawn_command_options_in_new_window_from_query(query)
                        .map(WindowCommand::SpawnCommandOptionsInNewWindow)
                })
                .unwrap_or(WindowCommand::SpawnWindow),
            WindowCommand::NewTab | WindowCommand::SpawnTab(_) => {
                spawn_command_in_new_tab_from_query(query)
                    .map(WindowCommand::SpawnCommandInNewTab)
                    .or_else(|| {
                        spawn_command_options_in_new_tab_from_query(query)
                            .map(WindowCommand::SpawnCommandOptionsInNewTab)
                    })
                    .unwrap_or(command)
            }
            WindowCommand::SplitRight | WindowCommand::SplitHorizontal => {
                split_horizontal_options_from_query(query)
                    .map(WindowCommand::SplitPane)
                    .or_else(|| {
                        split_horizontal_command_from_query(query).map(|command| {
                            WindowCommand::SplitPane(WindowSplitPaneOptions {
                                direction: SplitDirection::Right,
                                domain: None,
                                command: Some(command),
                                command_options: None,
                                size: None,
                                top_level: false,
                            })
                        })
                    })
                    .unwrap_or(command)
            }
            WindowCommand::SplitDown | WindowCommand::SplitVertical => {
                split_vertical_options_from_query(query)
                    .map(WindowCommand::SplitPane)
                    .or_else(|| {
                        split_vertical_command_from_query(query).map(|command| {
                            WindowCommand::SplitPane(WindowSplitPaneOptions {
                                direction: SplitDirection::Down,
                                domain: None,
                                command: Some(command),
                                command_options: None,
                                size: None,
                                top_level: false,
                            })
                        })
                    })
                    .unwrap_or(command)
            }
            WindowCommand::SwitchToWorkspace => switch_workspace_options_from_query(query).map_or(
                WindowCommand::SwitchToWorkspace,
                WindowCommand::SwitchToWorkspaceArgs,
            ),
            WindowCommand::EnterPaneSwap | WindowCommand::EnterPaneSwapKeepFocus => {
                pane_select_mode_show_pane_ids_from_query(query)
                    .map(|parsed| {
                        WindowCommand::PaneSelect(WindowPaneSelectOptions {
                            mode: parsed.mode,
                            show_pane_ids: true,
                            alphabet: parsed.alphabet,
                        })
                    })
                    .or_else(|| {
                        pane_select_mode_alphabet_from_query(query).map(|parsed| {
                            WindowCommand::PaneSelect(WindowPaneSelectOptions {
                                mode: parsed.mode,
                                show_pane_ids: false,
                                alphabet: Some(parsed.alphabet),
                            })
                        })
                    })
                    .unwrap_or(command)
            }
            _ => command,
        }
    }

    fn command_palette_execute_entry(&mut self, entry: WindowCommandPaletteEntry) -> bool {
        match entry {
            WindowCommandPaletteEntry::BuiltIn(command) => self.command_palette_execute(command),
            WindowCommandPaletteEntry::Contextual { command, .. } => {
                self.command_palette_execute(command)
            }
            WindowCommandPaletteEntry::Augmented(entry) => {
                let frecency_label = entry.brief;
                let succeeded = self.command_palette_execute(entry.action);
                if succeeded {
                    self.record_command_palette_label(&frecency_label);
                }
                succeeded
            }
        }
    }

    fn command_palette_status(&self, palette: &WindowCommandPalette) -> String {
        let entries = self.command_palette_filtered_entries();
        let title = palette.title();
        let help_prefix = palette.help_text().map(|help_text| {
            if help_text.ends_with(char::is_whitespace) {
                help_text.to_owned()
            } else {
                format!("{help_text} ")
            }
        });
        if entries.is_empty() {
            if palette.query.is_empty() {
                if let Some(help_prefix) = help_prefix.as_deref() {
                    return format!("{title}: {help_prefix}no commands");
                }
                return format!("{title}: no commands");
            }
            if let Some(help_prefix) = help_prefix.as_deref() {
                return format!("{title}: {help_prefix}\"{}\" (no match)", palette.query);
            }
            return format!("{title}: \"{}\" (no match)", palette.query);
        }

        let selected = palette.selected.min(entries.len().saturating_sub(1));
        let entry = &entries[selected];

        if palette.query.is_empty() {
            if let Some(help_prefix) = help_prefix.as_deref() {
                return format!(
                    "{}: {}[{} / {}] {}",
                    title,
                    help_prefix,
                    selected + 1,
                    entries.len(),
                    entry.label()
                );
            }
            format!(
                "{}: [{} / {}] {}",
                title,
                selected + 1,
                entries.len(),
                entry.label()
            )
        } else {
            if let Some(help_prefix) = help_prefix.as_deref() {
                return format!(
                    "{}: {}\"{}\" [{} / {}] {}",
                    title,
                    help_prefix,
                    palette.query,
                    selected + 1,
                    entries.len(),
                    entry.label()
                );
            }
            format!(
                "{}: \"{}\" [{} / {}] {}",
                title,
                palette.query,
                selected + 1,
                entries.len(),
                entry.label()
            )
        }
    }

    fn quick_select_status(quick_select: &WindowQuickSelect) -> String {
        let action_label = quick_select
            .action_label
            .as_ref()
            .map_or(String::new(), |label| format!(" {label}"));
        if quick_select.matches.is_empty() {
            return format!("Quick Select{action_label}: no match");
        }

        if !quick_select.input.is_empty() {
            return format!(
                "Quick Select{}: \"{}\" [{} / {}]",
                action_label,
                quick_select.input,
                quick_select.current + 1,
                quick_select.matches.len()
            );
        }

        format!(
            "Quick Select{}: [{} / {}]",
            action_label,
            quick_select.current + 1,
            quick_select.matches.len()
        )
    }

    fn clear_selection(&mut self) {
        self.clear_ordinary_selection();
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn pane_select_status(pane_select: &WindowPaneSelect) -> String {
        format!("Pane Select: [{} panes]", pane_select.labels.len())
    }

    fn char_select_status(char_select: &WindowCharSelect) -> String {
        let mut status = char_select.group.as_ref().map_or_else(
            || "Char Select".to_owned(),
            |group| format!("Char Select: {group}"),
        );
        if !char_select.input.is_empty() {
            status.push_str(" [");
            status.push_str(&char_select.input);
            status.push(']');
        }
        status
    }

    fn prompt_input_line_status(prompt_input_line: &WindowPromptInputLine) -> String {
        if prompt_input_line.description.is_empty() {
            return format!("{}{}", prompt_input_line.prompt, prompt_input_line.input);
        }

        format!(
            "{}: {}{}",
            prompt_input_line.description, prompt_input_line.prompt, prompt_input_line.input
        )
    }

    fn input_selector_status(input_selector: &WindowInputSelector) -> String {
        let choices = Self::input_selector_filtered_choices(input_selector);
        let title = &input_selector.title;
        let help_text = if input_selector.fuzzy {
            &input_selector.fuzzy_description
        } else {
            &input_selector.description
        };
        let help_prefix = if help_text.ends_with(char::is_whitespace) {
            help_text.to_owned()
        } else {
            format!("{help_text} ")
        };

        if choices.is_empty() {
            if input_selector.fuzzy {
                return format!(
                    "{}: {}\"{}\" (no match)",
                    title, help_prefix, input_selector.query
                );
            }
            return format!("{title}: {help_prefix}no choices");
        }

        let selected = input_selector.selected.min(choices.len().saturating_sub(1));
        let choice = &choices[selected];
        if input_selector.fuzzy {
            return format!(
                "{}: {}\"{}\" [{} / {}] {}",
                title,
                help_prefix,
                input_selector.query,
                selected + 1,
                choices.len(),
                choice.label
            );
        }

        format!(
            "{}: {}[{} / {}] {}",
            title,
            help_prefix,
            selected + 1,
            choices.len(),
            choice.label
        )
    }

    fn confirmation_status(confirmation: &WindowConfirmation) -> String {
        format!("{} Enter/Y=yes Esc/N=no", confirmation.message)
    }

    fn close_confirmation_status(close_confirmation: &WindowCloseConfirmation) -> String {
        format!("{}? Enter/Y=yes Esc/N=no", close_confirmation.label())
    }

    fn request_close_confirmation_or_close(&mut self, target: WindowCloseTarget) {
        if self.should_skip_close_confirmation(&target) {
            self.close_target_without_confirmation(target);
        } else {
            self.enter_close_confirmation_mode(target);
        }
    }

    fn should_skip_close_confirmation(&self, target: &WindowCloseTarget) -> bool {
        let processes = self.close_target_process_names(target);
        !processes.is_empty()
            && processes
                .iter()
                .all(|process| self.is_skip_close_confirmation_process(process))
    }

    fn close_target_process_names(&self, target: &WindowCloseTarget) -> Vec<&str> {
        match target {
            WindowCloseTarget::Window => self
                .app_shell
                .active_workspace()
                .tabs()
                .iter()
                .flat_map(rssh_core::app_shell::Tab::panes)
                .map(|pane| pane_launch_display_program(pane.launch()))
                .collect(),
            WindowCloseTarget::Pane(pane_id) => self
                .app_shell
                .active_workspace()
                .tabs()
                .iter()
                .flat_map(rssh_core::app_shell::Tab::panes)
                .find(|pane| pane.id() == *pane_id)
                .map(|pane| vec![pane.launch().program()])
                .unwrap_or_default(),
            WindowCloseTarget::Tab(tab_id) => self
                .app_shell
                .active_workspace()
                .tabs()
                .iter()
                .find(|tab| tab.id() == *tab_id)
                .map(|tab| {
                    tab.panes()
                        .iter()
                        .map(|pane| pane_launch_display_program(pane.launch()))
                        .collect()
                })
                .unwrap_or_default(),
            WindowCloseTarget::Tabs(tab_ids) => self
                .app_shell
                .active_workspace()
                .tabs()
                .iter()
                .filter(|tab| tab_ids.contains(&tab.id()))
                .flat_map(rssh_core::app_shell::Tab::panes)
                .map(|pane| pane.launch().program())
                .collect(),
        }
    }

    fn is_skip_close_confirmation_process(&self, process: &str) -> bool {
        let process = process_file_name(process);
        self.skip_close_confirmation_for_processes_named
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(process))
    }

    fn close_target_without_confirmation(&mut self, target: WindowCloseTarget) {
        match target {
            WindowCloseTarget::Window => self.request_window_close(),
            WindowCloseTarget::Pane(pane) => {
                if let Err(error) = self.dispatch_app_action(AppAction::ClosePane { pane }) {
                    eprintln!("close action failed: {error:?}");
                }
            }
            WindowCloseTarget::Tab(tab) => {
                if let Err(error) = self.dispatch_app_action(AppAction::CloseTab {
                    tab,
                    switch_to_last_active: self.switch_to_last_active_tab_when_closing_tab,
                }) {
                    eprintln!("close action failed: {error:?}");
                }
            }
            WindowCloseTarget::Tabs(tabs) => {
                if let Err(error) = self.dispatch_close_tab_set_without_confirmation(tabs) {
                    eprintln!("batch tab close action failed: {error:?}");
                }
            }
        }
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn enter_close_confirmation_mode(&mut self, target: WindowCloseTarget) {
        self.cancel_pane_inspection();
        self.deferred_wheel_context = None;
        self.command_palette = None;
        self.pane_select = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.selection = None;
        self.close_confirmation = Some(WindowCloseConfirmation { target });
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn exit_close_confirmation_mode(&mut self) {
        self.close_confirmation = None;
        self.restore_active_pane_presentation_after_higher_level_ui();
    }

    fn accept_close_confirmation(&mut self) {
        let Some(close_confirmation) = self.close_confirmation.take() else {
            return;
        };
        match close_confirmation.target {
            WindowCloseTarget::Tabs(tabs) => {
                if let Err(error) = self.dispatch_close_tab_set_without_confirmation(tabs) {
                    eprintln!("batch tab close confirmation action failed: {error:?}");
                }
            }
            WindowCloseTarget::Window => self.request_window_close(),
            target => {
                let confirmation = WindowCloseConfirmation { target };
                if let Some(action) =
                    confirmation.action(self.switch_to_last_active_tab_when_closing_tab)
                    && let Err(error) = self.dispatch_app_action(action)
                {
                    eprintln!("close confirmation action failed: {error:?}");
                }
            }
        }
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn handle_close_confirmation_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.close_confirmation.is_none() {
            return false;
        }

        match key.as_ref() {
            Key::Named(NamedKey::Enter) | Key::Character("\r") if modifiers.is_empty() => {
                self.accept_close_confirmation();
                true
            }
            Key::Character(text)
                if modifiers.is_empty()
                    && (text.eq_ignore_ascii_case("y") || text.eq_ignore_ascii_case(" ")) =>
            {
                self.accept_close_confirmation();
                true
            }
            Key::Named(NamedKey::Escape) => {
                self.exit_close_confirmation_mode();
                true
            }
            Key::Character(text) if modifiers.is_empty() && text.eq_ignore_ascii_case("n") => {
                self.exit_close_confirmation_mode();
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && (text.eq_ignore_ascii_case("c") || text.eq_ignore_ascii_case("g")) =>
            {
                self.exit_close_confirmation_mode();
                true
            }
            _ => true,
        }
    }

    fn enter_confirmation_mode(&mut self, options: WindowConfirmationOptions) {
        self.cancel_pane_inspection();
        self.deferred_wheel_context = None;
        self.command_palette = None;
        self.pane_select = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;
        self.selection = None;
        self.confirmation = Some(WindowConfirmation::from_options(options));
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn exit_confirmation_mode(&mut self) {
        self.confirmation = None;
        self.deferred_wheel_context = None;
        self.restore_active_pane_presentation_after_higher_level_ui();
    }

    fn submit_confirmation(&mut self, accepted: bool) {
        let target = self.deferred_wheel_context;
        let pane_id =
            target.map_or_else(|| self.app_shell.active_pane_id(), |target| target.pane_id);
        let action = self.confirmation.as_ref().and_then(|confirmation| {
            if accepted {
                Some((*confirmation.action).clone())
            } else {
                confirmation.cancel.as_deref().cloned()
            }
        });
        let event = NativeConfirmation {
            window_id: self.app_window_id,
            pane: pane_id,
            accepted,
        };
        self.dispatch_confirmation(&event);
        self.exit_confirmation_mode();
        if let Some(action) = action {
            let result = if let Some(target) = target {
                self.apply_command_for_target_context(target, action)
                    .map_err(|error| error.to_string())
            } else {
                self.command_palette_apply_command(action)
                    .map_err(|error| format!("{error:?}"))
            };
            if let Err(error) = result {
                eprintln!("confirmation action failed: {error}");
            }
        }
    }

    fn dispatch_confirmation(&mut self, event: &NativeConfirmation) -> bool {
        (self.confirmation_handler)(event)
    }

    fn emit_event(&mut self, event: WindowEmitEvent) -> bool {
        let pane_id = self.app_shell.active_pane_id();
        self.emit_event_for_pane(pane_id, event)
    }

    fn emit_event_for_target(&mut self, target: WheelTarget, event: WindowEmitEvent) -> bool {
        self.emit_event_in_context(target.pane_id, Some(target), event)
    }

    fn emit_event_for_pane(&mut self, pane_id: rssh_core::PaneId, event: WindowEmitEvent) -> bool {
        self.emit_event_in_context(pane_id, None, event)
    }

    fn emit_event_in_context(
        &mut self,
        pane_id: rssh_core::PaneId,
        target: Option<WheelTarget>,
        event: WindowEmitEvent,
    ) -> bool {
        let event = NativeWindowEmitEvent {
            window_id: self.app_window_id,
            pane: pane_id,
            name: event.name,
        };
        let handled = (self.emit_event_handler)(&event);
        if let Some(handlers) = self.lua_emit_event_handlers.get(&event.name).cloned() {
            for handler in handlers {
                if let Some(command) = handler.command {
                    let result = if let Some(target) = target {
                        self.apply_command_for_target_context(target, command)
                    } else {
                        self.command_palette_apply_command(command)
                            .map_err(|error| io::Error::other(format!("{error:?}")))
                    };
                    if let Err(error) = result {
                        eprintln!("emit event action failed: {error}");
                    }
                }
                if handler.stop_propagation {
                    break;
                }
            }
        }
        handled
    }

    fn activate_key_table(&mut self, key_table: WindowActivateKeyTable) {
        if key_table.replace_current {
            self.key_table_stack.pop();
        }
        self.key_table_stack
            .push(WindowActiveKeyTable::from(key_table));
        self.apply_window_title();
    }

    fn pop_key_table(&mut self) {
        self.key_table_stack.pop();
        self.apply_window_title();
    }

    fn clear_key_table_stack(&mut self) {
        self.key_table_stack.clear();
        self.apply_window_title();
    }

    fn expire_key_table_stack_if_due(&mut self, now: Instant) -> bool {
        let before = self.key_table_stack.len();
        self.key_table_stack
            .retain(|activation| !activation.is_expired(now));
        if self.key_table_stack.len() == before {
            return false;
        }

        self.apply_window_title();
        true
    }

    fn handle_active_key_table_assignment_key_press(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
    ) -> bool {
        let mut halt_fallback = false;
        let matched =
            self.key_table_stack
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, activation)| {
                    let command = self
                        .key_tables
                        .get(&activation.name)
                        .and_then(|assignments| {
                            assignments
                                .iter()
                                .find(|assignment| {
                                    window_key_assignment_matches_key_event(
                                        &assignment.keys,
                                        key,
                                        physical_key,
                                        modifiers,
                                        self.key_map_preference,
                                    )
                                })
                                .map(|assignment| assignment.command.clone())
                        });

                    if let Some(command) = command {
                        Some((index, activation.clone(), command))
                    } else {
                        if activation.prevent_fallback {
                            halt_fallback = true;
                        }
                        None
                    }
                });

        let Some((index, activation, command)) = matched else {
            return halt_fallback;
        };

        if let Err(error) = self.command_palette_apply_command(command) {
            eprintln!("key table action failed: {error:?}");
        }

        if activation.one_shot
            && self
                .key_table_stack
                .get(index)
                .is_some_and(|current| current == &activation)
        {
            self.key_table_stack.remove(index);
            self.apply_window_title();
        } else if self
            .key_table_stack
            .get(index)
            .is_some_and(|current| current == &activation)
        {
            self.key_table_stack[index].activated_at = Instant::now();
        }

        true
    }

    fn handle_unmatched_active_key_table_key_press(&mut self) -> bool {
        let Some(activation) = self.key_table_stack.last() else {
            return false;
        };
        let prevent_fallback = activation.prevent_fallback;
        let should_pop = activation.one_shot || activation.until_unknown;

        if should_pop {
            self.key_table_stack.pop();
            self.apply_window_title();
        }

        prevent_fallback
    }

    fn handle_user_key_assignment_key_press(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
    ) -> bool {
        self.handle_user_key_assignment_key_press_with_leader(key, physical_key, modifiers, false)
    }

    fn handle_user_key_assignment_key_press_with_leader(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
        leader_active: bool,
    ) -> bool {
        let command = self
            .key_assignments
            .iter()
            .find(|assignment| {
                assignment.command != WindowCommand::DisableDefaultAssignment
                    && window_key_assignment_matches_with_leader(
                        &assignment.keys,
                        key,
                        Some(physical_key),
                        modifiers,
                        self.key_map_preference,
                        leader_active,
                    )
            })
            .map(|assignment| assignment.command.clone());

        let Some(command) = command else {
            return false;
        };

        if let Err(error) = self.command_palette_apply_command(command) {
            eprintln!("key assignment action failed: {error:?}");
        }

        true
    }

    fn handle_leader_key_press(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
        now: Instant,
    ) -> bool {
        self.expire_leader_key_if_due(now);

        if self.leader_active_since.is_some() {
            self.leader_active_since = None;
            self.handle_user_key_assignment_key_press_with_leader(
                key,
                physical_key,
                modifiers,
                true,
            );
            self.apply_window_title();
            return true;
        }

        let Some(leader) = &self.leader else {
            return false;
        };

        if !window_key_assignment_matches_key_event(
            &leader.keys,
            key,
            physical_key,
            modifiers,
            self.key_map_preference,
        ) {
            return false;
        }

        self.leader_active_since = Some(now);
        self.apply_window_title();
        true
    }

    fn expire_leader_key_if_due(&mut self, now: Instant) -> bool {
        let Some(active_since) = self.leader_active_since else {
            return false;
        };

        if now.duration_since(active_since) < self.leader_timeout() {
            return false;
        }

        self.leader_active_since = None;
        self.apply_window_title();
        true
    }

    fn leader_timeout(&self) -> Duration {
        self.leader
            .as_ref()
            .and_then(|leader| leader.timeout_milliseconds)
            .map_or(DEFAULT_LEADER_TIMEOUT, Duration::from_millis)
    }

    fn handle_confirmation_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.confirmation.is_none() {
            return false;
        }

        match key.as_ref() {
            Key::Named(NamedKey::Enter) | Key::Character("\r") if modifiers.is_empty() => {
                self.submit_confirmation(true);
                true
            }
            Key::Character(text)
                if modifiers.is_empty()
                    && (text.eq_ignore_ascii_case("y") || text.eq_ignore_ascii_case(" ")) =>
            {
                self.submit_confirmation(true);
                true
            }
            Key::Named(NamedKey::Escape) => {
                self.submit_confirmation(false);
                true
            }
            Key::Character(text) if modifiers.is_empty() && text.eq_ignore_ascii_case("n") => {
                self.submit_confirmation(false);
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && (text.eq_ignore_ascii_case("c") || text.eq_ignore_ascii_case("g")) =>
            {
                self.submit_confirmation(false);
                true
            }
            _ => true,
        }
    }

    fn enter_input_selector_mode(&mut self, options: WindowInputSelectorOptions) {
        self.cancel_pane_inspection();
        self.deferred_wheel_context = None;
        self.command_palette = None;
        self.pane_select = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.confirmation = None;
        self.close_confirmation = None;
        self.selection = None;
        self.input_selector = Some(WindowInputSelector::from_options(options));
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn exit_input_selector_mode(&mut self) {
        self.input_selector = None;
        self.deferred_wheel_context = None;
        self.restore_active_pane_presentation_after_higher_level_ui();
    }

    fn dispatch_input_selector(&mut self, event: &NativeInputSelector) -> bool {
        (self.input_selector_handler)(event)
    }

    fn submit_input_selector(&mut self, choice: Option<WindowInputSelectorChoice>) {
        let target = self.deferred_wheel_context;
        let pane_id =
            target.map_or_else(|| self.app_shell.active_pane_id(), |target| target.pane_id);
        let action = self
            .input_selector
            .as_ref()
            .and_then(|input_selector| input_selector.action.clone());
        let event = NativeInputSelector {
            window_id: self.app_window_id,
            pane: pane_id,
            id: choice.as_ref().and_then(|choice| choice.id.clone()),
            label: choice.map(|choice| choice.label),
        };
        self.dispatch_input_selector(&event);
        self.exit_input_selector_mode();
        if let Some(action) = action {
            self.perform_input_selector_action(target, action, &event);
        }
    }

    fn perform_input_selector_action(
        &mut self,
        target: Option<WheelTarget>,
        action: WindowInputSelectorAction,
        event: &NativeInputSelector,
    ) {
        let command = match action {
            WindowInputSelectorAction::SendIdText => {
                let Some(value) =
                    Self::input_selector_event_value(event, WindowInputSelectorValueParam::Id)
                else {
                    return;
                };
                WindowCommand::SendString(value)
            }
            WindowInputSelectorAction::SendIdPaste => {
                let Some(value) =
                    Self::input_selector_event_value(event, WindowInputSelectorValueParam::Id)
                else {
                    return;
                };
                WindowCommand::SendPaste(value)
            }
            WindowInputSelectorAction::SendLabelText => {
                let Some(value) =
                    Self::input_selector_event_value(event, WindowInputSelectorValueParam::Label)
                else {
                    return;
                };
                WindowCommand::SendString(value)
            }
            WindowInputSelectorAction::SendLabelPaste => {
                let Some(value) =
                    Self::input_selector_event_value(event, WindowInputSelectorValueParam::Label)
                else {
                    return;
                };
                WindowCommand::SendPaste(value)
            }
            WindowInputSelectorAction::SwitchToWorkspace { name, cwd } => {
                let Some(name) = Self::input_selector_event_value(event, name) else {
                    return;
                };
                let command_options = cwd
                    .and_then(|source| Self::input_selector_event_value(event, source))
                    .map(|cwd| WindowSpawnCommandQueryOptions {
                        cwd: Some(cwd),
                        ..WindowSpawnCommandQueryOptions::default()
                    });
                WindowCommand::SwitchToWorkspaceArgs(WindowSwitchToWorkspaceOptions {
                    name: Some(name),
                    command: None,
                    command_options,
                })
            }
            WindowInputSelectorAction::Command(command) => *command,
        };
        let result = if let Some(target) = target {
            self.apply_command_for_target_context(target, command)
                .map_err(|error| error.to_string())
        } else {
            self.command_palette_apply_command(command)
                .map_err(|error| format!("{error:?}"))
        };
        if let Err(error) = result {
            eprintln!("input selector action failed: {error}");
        }
    }

    fn input_selector_event_value(
        event: &NativeInputSelector,
        source: WindowInputSelectorValueParam,
    ) -> Option<String> {
        match source {
            WindowInputSelectorValueParam::Id => event.id.clone(),
            WindowInputSelectorValueParam::Label => event.label.clone(),
        }
    }

    fn input_selector_filtered_choices(
        input_selector: &WindowInputSelector,
    ) -> Vec<WindowInputSelectorChoice> {
        if input_selector.query.is_empty() {
            return input_selector.choices.clone();
        }

        let query = input_selector.query.to_ascii_lowercase();
        let mut matches = input_selector
            .choices
            .iter()
            .cloned()
            .filter_map(|choice| {
                palette_match_score(&choice.label, &query).map(|score| (choice, score))
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|(_, score)| *score);
        matches.into_iter().map(|(choice, _)| choice).collect()
    }

    fn move_input_selector_selection(&mut self, delta: isize) -> bool {
        let Some(input_selector) = self.input_selector.as_ref() else {
            return false;
        };
        let choices = Self::input_selector_filtered_choices(input_selector);
        if choices.is_empty() {
            return true;
        }

        let Some(input_selector) = self.input_selector.as_mut() else {
            return false;
        };
        let len = isize::try_from(choices.len()).unwrap_or(1);
        let current = isize::try_from(input_selector.selected).unwrap_or(0);
        input_selector.selected = usize::try_from((current + delta).rem_euclid(len)).unwrap_or(0);
        self.apply_window_title();
        true
    }

    fn input_selector_shortcut_for_key(&self, text: &str) -> Option<WindowInputSelectorShortcut> {
        let input_selector = self.input_selector.as_ref()?;
        if input_selector.fuzzy || !input_selector.query.is_empty() {
            return None;
        }

        let mut text_chars = text.chars();
        let target = text_chars.next()?;
        if text_chars.next().is_some() {
            return None;
        }
        let choices = Self::input_selector_filtered_choices(input_selector);
        if input_selector.shortcut_prefix.is_empty()
            && let Some(index) = target
                .to_digit(10)
                .filter(|digit| (1..=9).contains(digit))
                .and_then(|digit| usize::try_from(digit - 1).ok())
            && let Some(choice) = choices.get(index)
        {
            return Some(WindowInputSelectorShortcut::Execute(choice.clone()));
        }
        let target = target.to_lowercase().to_string();
        let candidate = format!("{}{}", input_selector.shortcut_prefix, target);
        let labels = quick_select_labels_for_alphabet(&input_selector.alphabet, choices.len());

        for (choice, label) in choices.iter().cloned().zip(labels.iter()) {
            if label == &candidate {
                return Some(WindowInputSelectorShortcut::Execute(choice));
            }
        }

        labels
            .iter()
            .any(|label| label.starts_with(&candidate))
            .then_some(WindowInputSelectorShortcut::Pending(candidate))
    }

    fn submit_current_input_selector_choice(&mut self) {
        let choice = self.input_selector.as_ref().and_then(|input_selector| {
            let choices = Self::input_selector_filtered_choices(input_selector);
            choices
                .get(input_selector.selected.min(choices.len().saturating_sub(1)))
                .cloned()
        });
        self.submit_input_selector(choice);
    }

    fn input_selector_choice_at_mouse_position(&self) -> Option<WindowInputSelectorChoice> {
        let input_selector = self.input_selector.as_ref()?;
        let choices = Self::input_selector_filtered_choices(input_selector);
        if choices.is_empty() {
            return None;
        }

        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 || size.columns == 0 {
            return None;
        }

        let (_, row) = self.mouse_position?;
        let frame_row = row.checked_add(self.terminal_frame_row_offset())?;
        let first_row = if self.tab_bar_is_visible() && !self.tab_bar_at_bottom {
            TAB_BAR_ROWS
        } else {
            0
        };
        let visible_rows = choices.len().min(usize::from(size.rows));
        let selected = input_selector.selected.min(choices.len().saturating_sub(1));
        let start = selected.saturating_add(1).saturating_sub(visible_rows);
        let visible_index = usize::from(frame_row.checked_sub(first_row)?);
        if visible_index >= visible_rows {
            return None;
        }

        choices.get(start + visible_index).cloned()
    }

    fn handle_input_selector_mouse_input(
        &mut self,
        state: ElementState,
        button: MouseButton,
    ) -> bool {
        if state != ElementState::Pressed || button != MouseButton::Left {
            return false;
        }

        let Some(choice) = self.input_selector_choice_at_mouse_position() else {
            return false;
        };
        self.submit_input_selector(Some(choice));
        true
    }

    fn handle_user_mouse_assignment(
        &mut self,
        kind: NativeMouseAssignmentEventKind,
        button: NativeMouseAssignmentButton,
        streak: u8,
        mouse_reporting: bool,
        alternate_screen_active: bool,
    ) -> bool {
        let Some(command) = self
            .mouse_assignments
            .iter()
            .find(|assignment| {
                assignment.event.kind == kind
                    && assignment.event.button == button
                    && assignment.event.streak == streak
                    && assignment.modifiers == self.modifiers
                    && assignment.mouse_reporting == mouse_reporting
                    && assignment.alt_screen.matches(alternate_screen_active)
                    && assignment.command != WindowCommand::DisableDefaultAssignment
            })
            .map(|assignment| assignment.command.clone())
        else {
            return false;
        };

        if kind == NativeMouseAssignmentEventKind::Drag {
            self.clear_ordinary_selection();
            self.selecting = false;
        }

        if let Err(error) = self.command_palette_apply_command(command) {
            eprintln!("mouse binding failed: {error:?}");
            return false;
        }
        true
    }

    fn user_mouse_assignment_overrides_default_for_button(
        &self,
        button: NativeMouseAssignmentButton,
        streak: u8,
        mouse_reporting: bool,
        alternate_screen_active: bool,
    ) -> bool {
        self.mouse_assignments.iter().any(|assignment| {
            assignment.event.button == button
                && assignment.event.streak == streak
                && assignment.modifiers == self.modifiers
                && assignment.mouse_reporting == mouse_reporting
                && assignment.alt_screen.matches(alternate_screen_active)
        })
    }

    fn has_mouse_assignment_candidate_for_button(
        &self,
        button: MouseButton,
        mouse_reporting: bool,
        alternate_screen_active: bool,
    ) -> bool {
        self.mouse_assignments.iter().any(|assignment| {
            assignment.event.button == NativeMouseAssignmentButton::Mouse(button)
                && assignment.modifiers == self.modifiers
                && assignment.mouse_reporting == mouse_reporting
                && assignment.alt_screen.matches(alternate_screen_active)
        })
    }

    fn mouse_assignment_streak(
        &mut self,
        state: ElementState,
        button: MouseButton,
        mouse_reporting: bool,
        alternate_screen_active: bool,
    ) -> u8 {
        match state {
            ElementState::Pressed => {
                if !self.has_mouse_assignment_candidate_for_button(
                    button,
                    mouse_reporting,
                    alternate_screen_active,
                ) {
                    self.last_mouse_assignment_click = None;
                    return 1;
                }

                let time = Instant::now();
                let count = self
                    .last_mouse_assignment_click
                    .and_then(|last_click| {
                        let elapsed = time.checked_duration_since(last_click.time)?;
                        (last_click.button == button
                            && last_click.modifiers == self.modifiers
                            && last_click.mouse_reporting == mouse_reporting
                            && last_click.alternate_screen_active == alternate_screen_active
                            && elapsed <= DOUBLE_CLICK_MAX_INTERVAL)
                            .then_some(last_click.count.saturating_add(1))
                    })
                    .unwrap_or(1);
                self.last_mouse_assignment_click = Some(WindowMouseAssignmentClick {
                    button,
                    modifiers: self.modifiers,
                    mouse_reporting,
                    alternate_screen_active,
                    time,
                    count,
                });
                count
            }
            ElementState::Released => self
                .last_mouse_assignment_click
                .filter(|click| {
                    click.button == button
                        && click.modifiers == self.modifiers
                        && click.mouse_reporting == mouse_reporting
                        && click.alternate_screen_active == alternate_screen_active
                })
                .map_or(1, |click| click.count),
        }
    }

    fn active_mouse_assignment_streak(
        &self,
        button: MouseButton,
        mouse_reporting: bool,
        alternate_screen_active: bool,
    ) -> u8 {
        self.last_mouse_assignment_click
            .filter(|click| {
                click.button == button
                    && click.modifiers == self.modifiers
                    && click.mouse_reporting == mouse_reporting
                    && click.alternate_screen_active == alternate_screen_active
            })
            .map_or(1, |click| click.count)
    }

    fn handle_input_selector_backspace(&mut self) -> bool {
        if let Some(input_selector) = self.input_selector.as_mut() {
            if input_selector.query.pop().is_none()
                && input_selector.fuzzy
                && !input_selector.started_fuzzy
            {
                input_selector.fuzzy = false;
            }
            input_selector.selected = 0;
            input_selector.shortcut_prefix.clear();
        }
        self.apply_window_title();
        true
    }

    fn handle_input_selector_text(&mut self, text: &str) -> bool {
        if text == "/"
            && let Some(input_selector) = self.input_selector.as_mut()
            && !input_selector.fuzzy
            && input_selector.query.is_empty()
            && input_selector.shortcut_prefix.is_empty()
        {
            input_selector.fuzzy = true;
            self.apply_window_title();
            return true;
        }

        if let Some(shortcut) = self.input_selector_shortcut_for_key(text) {
            match shortcut {
                WindowInputSelectorShortcut::Execute(choice) => {
                    self.submit_input_selector(Some(choice));
                    return true;
                }
                WindowInputSelectorShortcut::Pending(prefix) => {
                    if let Some(input_selector) = self.input_selector.as_mut() {
                        input_selector.shortcut_prefix = prefix;
                    }
                    return true;
                }
            }
        }

        if let Some(input_selector) = self.input_selector.as_mut() {
            if !input_selector.fuzzy && input_selector.query.is_empty() {
                match text {
                    "j" => {
                        return self.move_input_selector_selection(1);
                    }
                    "k" => {
                        return self.move_input_selector_selection(-1);
                    }
                    _ => {}
                }
            }
            if !input_selector.fuzzy {
                return true;
            }
            input_selector.query.push_str(text);
            input_selector.selected = 0;
            input_selector.shortcut_prefix.clear();
        }
        self.apply_window_title();
        true
    }

    fn handle_input_selector_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.input_selector.is_none() {
            return false;
        }

        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.submit_input_selector(None);
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && (text.eq_ignore_ascii_case("c") || text.eq_ignore_ascii_case("g")) =>
            {
                self.submit_input_selector(None);
                true
            }
            Key::Named(NamedKey::Enter) | Key::Character("\r") if modifiers.is_empty() => {
                self.submit_current_input_selector_choice();
                true
            }
            Key::Named(NamedKey::ArrowDown) => self.move_input_selector_selection(1),
            Key::Named(NamedKey::ArrowUp) => self.move_input_selector_selection(-1),
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && (text.eq_ignore_ascii_case("n") || text.eq_ignore_ascii_case("j")) =>
            {
                self.move_input_selector_selection(1)
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && (text.eq_ignore_ascii_case("p") || text.eq_ignore_ascii_case("k")) =>
            {
                self.move_input_selector_selection(-1)
            }
            Key::Named(NamedKey::Backspace) if modifiers.is_empty() => {
                self.handle_input_selector_backspace()
            }
            Key::Character(text) if !modifiers.control_key() && !modifiers.alt_key() => {
                self.handle_input_selector_text(text)
            }
            _ => true,
        }
    }

    fn enter_prompt_input_line_mode(&mut self, options: WindowPromptInputLineOptions) {
        self.cancel_pane_inspection();
        self.deferred_wheel_context = None;
        self.command_palette = None;
        self.pane_select = None;
        self.tab_navigator = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;
        self.selection = None;
        self.prompt_input_line = Some(WindowPromptInputLine::from_options(options));
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn exit_prompt_input_line_mode(&mut self) {
        self.prompt_input_line = None;
        self.deferred_wheel_context = None;
        self.restore_active_pane_presentation_after_higher_level_ui();
    }

    fn submit_prompt_input_line(&mut self, line: Option<String>) {
        let target = self.deferred_wheel_context;
        let pane_id =
            target.map_or_else(|| self.app_shell.active_pane_id(), |target| target.pane_id);
        let action = self
            .prompt_input_line
            .as_ref()
            .and_then(|prompt| prompt.action.clone());
        let event = NativePromptInputLine {
            window_id: self.app_window_id,
            pane: pane_id,
            line,
        };
        self.dispatch_prompt_input_line(&event);
        self.exit_prompt_input_line_mode();
        if let (Some(action), Some(line)) = (action, event.line.clone()) {
            self.perform_prompt_input_line_action(target, action, line);
        }
    }

    fn dispatch_prompt_input_line(&mut self, event: &NativePromptInputLine) -> bool {
        (self.prompt_input_line_handler)(event)
    }

    fn perform_prompt_input_line_action(
        &mut self,
        target: Option<WheelTarget>,
        action: WindowPromptInputLineAction,
        line: String,
    ) {
        let command = match action {
            WindowPromptInputLineAction::RenameActiveTab => WindowCommand::RenameTabTo(line),
            WindowPromptInputLineAction::SwitchToWorkspaceName => {
                WindowCommand::SwitchToWorkspaceName(line)
            }
            WindowPromptInputLineAction::SendLineText => WindowCommand::SendString(line),
            WindowPromptInputLineAction::SendLinePaste => WindowCommand::SendPaste(line),
            WindowPromptInputLineAction::Command(command) => *command,
        };
        let result = if let Some(target) = target {
            self.apply_command_for_target_context(target, command)
                .map_err(|error| error.to_string())
        } else {
            self.command_palette_apply_command(command)
                .map_err(|error| format!("{error:?}"))
        };
        if let Err(error) = result {
            eprintln!("prompt input line action failed: {error}");
        }
    }

    fn handle_prompt_input_line_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.prompt_input_line.is_none() {
            return false;
        }

        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.submit_prompt_input_line(None);
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && text.eq_ignore_ascii_case("c") =>
            {
                self.submit_prompt_input_line(None);
                true
            }
            Key::Named(NamedKey::Enter) | Key::Character("\r") if modifiers.is_empty() => {
                let line = self
                    .prompt_input_line
                    .as_ref()
                    .map(|prompt| prompt.input.clone())
                    .unwrap_or_default();
                self.submit_prompt_input_line(Some(line));
                true
            }
            Key::Named(NamedKey::Backspace) if modifiers.is_empty() => {
                if let Some(prompt) = self.prompt_input_line.as_mut() {
                    prompt.input.pop();
                }
                self.apply_window_title();
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && text.eq_ignore_ascii_case("u") =>
            {
                if let Some(prompt) = self.prompt_input_line.as_mut() {
                    prompt.input.clear();
                }
                self.apply_window_title();
                true
            }
            Key::Character(text) if !modifiers.control_key() && !modifiers.alt_key() => {
                if let Some(prompt) = self.prompt_input_line.as_mut() {
                    prompt.input.push_str(text);
                }
                self.apply_window_title();
                true
            }
            _ => true,
        }
    }

    fn enter_pane_select_mode(&mut self) {
        self.enter_pane_select_mode_with_mode(WindowPaneSelectMode::Activate);
    }

    fn enter_pane_select_mode_with_mode(&mut self, mode: WindowPaneSelectMode) {
        self.enter_pane_select_mode_with_options(mode, false);
    }

    fn enter_pane_select_mode_with_options(
        &mut self,
        mode: WindowPaneSelectMode,
        show_pane_ids: bool,
    ) {
        let alphabet = self.quick_select_alphabet.clone();
        self.enter_pane_select_mode_with_alphabet(mode, show_pane_ids, &alphabet);
    }

    fn enter_pane_select_mode_with_action(&mut self, options: WindowPaneSelectOptions) {
        if let Some(alphabet) = options.alphabet {
            self.enter_pane_select_mode_with_alphabet(
                options.mode,
                options.show_pane_ids,
                &alphabet,
            );
        } else {
            self.enter_pane_select_mode_with_options(options.mode, options.show_pane_ids);
        }
    }

    fn enter_pane_select_mode_with_alphabet(
        &mut self,
        mode: WindowPaneSelectMode,
        show_pane_ids: bool,
        alphabet: &str,
    ) {
        self.cancel_pane_inspection();
        self.deferred_wheel_context = None;
        self.command_palette = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;
        self.selection = None;
        self.pane_select = Some(WindowPaneSelect::from_panes(
            self.app_shell.active_tab().panes(),
            mode,
            show_pane_ids,
            alphabet,
        ));
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    fn exit_pane_select_mode(&mut self) {
        self.pane_select = None;
        self.deferred_wheel_context = None;
        self.restore_active_pane_presentation_after_higher_level_ui();
    }

    fn tab_navigator_status(tab_navigator: &WindowTabNavigator) -> String {
        format!("Tab Navigator: [{} tabs]", tab_navigator.tabs.len())
    }

    fn enter_tab_navigator_mode(&mut self) {
        self.cancel_pane_inspection();
        self.deferred_wheel_context = None;
        self.command_palette = None;
        self.pane_select = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;
        self.selection = None;
        self.tab_navigator = Some(WindowTabNavigator::from_tabs(
            self.app_shell.active_workspace().tabs(),
            self.app_shell.active_tab_id(),
        ));
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

}

impl NativeWindowApp {
    fn exit_tab_navigator_mode(&mut self) {
        self.tab_navigator = None;
        self.restore_active_pane_presentation_after_higher_level_ui();
    }

    fn handle_tab_navigator_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        let Some(tab_navigator) = self.tab_navigator.as_mut() else {
            return false;
        };

        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.exit_tab_navigator_mode();
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && text.eq_ignore_ascii_case("g") =>
            {
                self.exit_tab_navigator_mode();
                true
            }
            Key::Named(NamedKey::ArrowUp) | Key::Character("k" | "p") if modifiers.is_empty() => {
                tab_navigator.move_selection(-1);
                self.frame_needs_full_repaint = true;
                self.apply_window_title();
                true
            }
            Key::Named(NamedKey::ArrowDown) | Key::Character("j" | "n") if modifiers.is_empty() => {
                tab_navigator.move_selection(1);
                self.frame_needs_full_repaint = true;
                self.apply_window_title();
                true
            }
            Key::Named(NamedKey::Enter) | Key::Character("\r") if modifiers.is_empty() => {
                let Some(tab) = tab_navigator.selected_tab() else {
                    self.exit_tab_navigator_mode();
                    return true;
                };
                if let Err(error) = self.dispatch_app_action(AppAction::ActivateTab { tab }) {
                    eprintln!("tab navigator action failed: {error:?}");
                }
                self.exit_tab_navigator_mode();
                true
            }
            _ => true,
        }
    }

    fn handle_pane_select_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.pane_select.is_none() {
            return false;
        }

        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.exit_pane_select_mode();
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && text.eq_ignore_ascii_case("g") =>
            {
                self.exit_pane_select_mode();
                true
            }
            Key::Character(text) if !modifiers.control_key() && !modifiers.alt_key() => {
                let Some(pane_select) = self.pane_select.as_ref() else {
                    return false;
                };
                let mut input = pane_select.input.clone();
                input.push_str(&text.to_ascii_lowercase());

                if let Some(pane) = pane_select.pane_for_label(&input) {
                    let mode = pane_select.mode;
                    let source = self
                        .deferred_wheel_context
                        .map_or_else(|| self.app_shell.active_pane_id(), |target| target.pane_id);
                    let action = match mode {
                        WindowPaneSelectMode::Activate => AppAction::ActivatePane { pane },
                        WindowPaneSelectMode::SwapWithActive => AppAction::SwapPanes {
                            active: source,
                            selected: pane,
                            keep_focus: false,
                        },
                        WindowPaneSelectMode::SwapWithActiveKeepFocus => AppAction::SwapPanes {
                            active: source,
                            selected: pane,
                            keep_focus: true,
                        },
                        WindowPaneSelectMode::MoveToNewTab => AppAction::MovePaneToNewTab { pane },
                        WindowPaneSelectMode::MoveToNewWindow => {
                            AppAction::MovePaneToNewWindow { pane }
                        }
                    };
                    if let Err(error) = self.dispatch_app_action(action) {
                        eprintln!("pane select action failed: {error:?}");
                    }
                    self.exit_pane_select_mode();
                    return true;
                }

                if let Some(pane_select) = self.pane_select.as_mut() {
                    if pane_select.has_label_prefix(&input) {
                        pane_select.input = input;
                    } else {
                        pane_select.input.clear();
                    }
                }
                self.apply_window_title();
                true
            }
            _ => true,
        }
    }

    fn quick_select_step(&mut self, direction: SearchDirection) -> bool {
        let Some(quick_select) = self.active_ui.quick_select_mut() else {
            return false;
        };

        if quick_select.matches.is_empty() {
            return false;
        }

        let len = quick_select.matches.len();
        let current = quick_select.current;
        quick_select.current = match direction {
            SearchDirection::Next => (current + 1) % len,
            SearchDirection::Previous => {
                if current == 0 {
                    len - 1
                } else {
                    current - 1
                }
            }
        };

        let Some(active) = quick_select.current_match() else {
            return false;
        };

        self.apply_quick_select_match(active);
        true
    }

    fn quick_select_step_page(&mut self, direction: SearchDirection) -> bool {
        let viewport_rows = usize::from(self.runtime.terminal().grid().size().rows);
        let viewport_top = self.current_viewport_stable_top();
        let viewport_rows_stable =
            StableRowIndex::try_from(viewport_rows).unwrap_or(StableRowIndex::MAX);

        let Some(active) = self.active_ui.quick_select_mut().and_then(|quick_select| {
            if quick_select.matches.is_empty() || viewport_rows == 0 {
                return None;
            }

            let current = quick_select.current;
            let last = quick_select.matches.len().saturating_sub(1);
            let target = match direction {
                SearchDirection::Next => {
                    let bottom = viewport_top.saturating_add(viewport_rows_stable);
                    quick_select
                        .matches
                        .iter()
                        .position(|candidate| candidate.source_row >= bottom)
                        .unwrap_or_else(|| current.min(last))
                }
                SearchDirection::Previous => {
                    let top = viewport_top;
                    let prior = top.saturating_sub(viewport_rows_stable);
                    quick_select
                        .matches
                        .iter()
                        .position(|candidate| {
                            let row = candidate.source_row;
                            row > prior && row < top
                        })
                        .unwrap_or_else(|| current.saturating_sub(1))
                }
            };
            quick_select.current = target;
            quick_select.current_match()
        }) else {
            return false;
        };

        self.apply_quick_select_match(active);
        true
    }

    fn apply_quick_select_match(&mut self, selection: WindowSearchMatch) {
        self.apply_search_match(selection, true);
        self.apply_window_title();
    }

    fn handle_quick_select_logical_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.active_ui.quick_select().is_none() {
            return false;
        }

        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.exit_quick_select_mode();
                true
            }
            Key::Named(NamedKey::Tab) => {
                if modifiers.shift_key() {
                    self.quick_select_step(SearchDirection::Previous);
                } else {
                    self.quick_select_step(SearchDirection::Next);
                }
                true
            }
            Key::Named(NamedKey::ArrowDown | NamedKey::ArrowRight) => {
                self.quick_select_step(SearchDirection::Next);
                true
            }
            Key::Named(NamedKey::ArrowUp | NamedKey::ArrowLeft | NamedKey::Enter) => {
                self.quick_select_step(SearchDirection::Previous);
                true
            }
            Key::Named(NamedKey::PageDown) if modifiers.is_empty() => {
                self.quick_select_step_page(SearchDirection::Next);
                true
            }
            Key::Named(NamedKey::PageUp) if modifiers.is_empty() => {
                self.quick_select_step_page(SearchDirection::Previous);
                true
            }
            Key::Named(NamedKey::Backspace) if modifiers.is_empty() => {
                if let Some(quick_select) = self.active_ui.quick_select_mut() {
                    quick_select.input.pop();
                }
                self.apply_window_title();
                true
            }
            Key::Character(text)
                if modifiers == ModifiersState::CONTROL && text.eq_ignore_ascii_case("n") =>
            {
                self.quick_select_step(SearchDirection::Next);
                true
            }
            Key::Character(text)
                if modifiers == ModifiersState::CONTROL && text.eq_ignore_ascii_case("p") =>
            {
                self.quick_select_step(SearchDirection::Previous);
                true
            }
            Key::Character(text)
                if modifiers.control_key()
                    && !modifiers.alt_key()
                    && text.eq_ignore_ascii_case("u") =>
            {
                if let Some(quick_select) = self.active_ui.quick_select_mut() {
                    quick_select.input.clear();
                }
                self.apply_window_title();
                true
            }
            Key::Character(text) if !modifiers.control_key() && !modifiers.alt_key() => {
                let Some((input, matched)) = self.active_ui.quick_select().map(|quick_select| {
                    let mut input = quick_select.input.clone();
                    input.push_str(text);
                    let matched = quick_select.match_for_label(&input);
                    (input, matched)
                }) else {
                    return false;
                };

                if let Some(matched) = matched {
                    if let Some(quick_select) = self.active_ui.quick_select_mut()
                        && let Some(current) = quick_select
                            .matches
                            .iter()
                            .position(|candidate| *candidate == matched)
                    {
                        quick_select.current = current;
                    }
                    self.apply_quick_select_match(matched);
                    self.accept_quick_select_match(input != input.to_ascii_lowercase());
                    return true;
                }

                if let Some(quick_select) = self.active_ui.quick_select_mut() {
                    if quick_select.has_label_prefix(&input) {
                        quick_select.input = input;
                    } else {
                        quick_select.input.clear();
                    }
                }
                self.apply_window_title();
                true
            }
            _ => false,
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn accept_quick_select_match(&mut self, paste: bool) {
        let source_pane_id = self.app_shell.active_pane_id();
        let Some((action, skip_action_on_paste)) =
            self.active_ui.quick_select().map(|quick_select| {
                (
                    quick_select.action.clone(),
                    quick_select.skip_action_on_paste,
                )
            })
        else {
            return;
        };
        let selected_text = self.selected_text();
        self.active_ui.exit_overlay();
        self.selection = None;
        self.refresh_snapshot();
        self.apply_window_title();

        match action {
            WindowQuickSelectAction::Copy => {
                if paste {
                    if let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                    {
                        eprintln!("quick-select paste failed: {error}");
                    }
                } else if let Some(text) = selected_text.as_deref() {
                    self.write_text_to_copy_destination(
                        text,
                        WindowCopyDestination::ClipboardAndPrimarySelection,
                    );
                }
            }
            WindowQuickSelectAction::CopyTo(destination) => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                if let Some(text) = selected_text.as_deref() {
                    self.write_text_to_copy_destination(text, destination);
                }
            }
            WindowQuickSelectAction::OpenUri => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                if let Some(uri) = selected_text.as_deref() {
                    self.open_uri(uri);
                }
            }
            WindowQuickSelectAction::Nop => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
            }
            WindowQuickSelectAction::PasteFrom(source) => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                if let Err(error) = self.handle_window_paste_from(source) {
                    eprintln!("quick-select paste-from failed: {error}");
                }
            }
            WindowQuickSelectAction::SendString(value) => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                if let Err(error) = self.write_pty_bytes(value.as_bytes()) {
                    eprintln!("quick-select send-string failed: {error}");
                }
            }
            WindowQuickSelectAction::SendSelectedText => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                if let Some(text) = selected_text.as_deref()
                    && let Err(error) = self.write_pty_bytes(text.as_bytes())
                {
                    eprintln!("quick-select send-selected-text failed: {error}");
                }
            }
            WindowQuickSelectAction::PasteSelectedText => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                if let Some(text) = selected_text.as_deref() {
                    let bytes = encode_window_paste(
                        text,
                        self.runtime.bracketed_paste(),
                        self.canonicalize_pasted_newlines,
                    );
                    if let Err(error) = self.write_pty_bytes(&bytes) {
                        eprintln!("quick-select paste-selected-text failed: {error}");
                    }
                }
            }
            WindowQuickSelectAction::SendKey(send_key) => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                if let Err(error) = self.send_key_to_active_pane(&send_key) {
                    eprintln!("quick-select send-key failed: {error}");
                }
            }
            WindowQuickSelectAction::EmitEvent(event) => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                self.emit_event(event);
            }
            WindowQuickSelectAction::Multiple(commands) => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                if let Err(error) = self.apply_quick_select_source_bound_commands(
                    source_pane_id,
                    selected_text.as_deref(),
                    commands,
                ) {
                    eprintln!("quick-select multiple failed: {error:?}");
                }
            }
            WindowQuickSelectAction::ActivateKeyTable(key_table) => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                self.activate_key_table(key_table);
            }
            WindowQuickSelectAction::PopKeyTable => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                self.pop_key_table();
            }
            WindowQuickSelectAction::ClearKeyTableStack => {
                if paste
                    && let Err(error) =
                        self.paste_captured_selected_text_to_pane(selected_text.as_deref())
                {
                    eprintln!("quick-select paste failed: {error}");
                }
                if paste && skip_action_on_paste {
                    return;
                }
                self.clear_key_table_stack();
            }
        }
    }

    fn apply_quick_select_source_bound_commands(
        &mut self,
        source_pane_id: rssh_core::PaneId,
        selected_text: Option<&str>,
        commands: Vec<WindowCommand>,
    ) -> Result<(), AppShellError> {
        for command in commands {
            self.apply_quick_select_source_bound_command(source_pane_id, selected_text, command)?;
        }
        Ok(())
    }

    fn apply_quick_select_source_bound_command(
        &mut self,
        source_pane_id: rssh_core::PaneId,
        selected_text: Option<&str>,
        command: WindowCommand,
    ) -> Result<(), AppShellError> {
        match command {
            WindowCommand::Multiple(commands) => self.apply_quick_select_source_bound_commands(
                source_pane_id,
                selected_text,
                commands,
            ),
            WindowCommand::ClearSelection => {
                self.clear_ordinary_selection_for_pane(source_pane_id);
                Ok(())
            }
            WindowCommand::Copy | WindowCommand::CopyToClipboard => {
                if let Some(text) = selected_text {
                    self.write_text_to_copy_destination(text, WindowCopyDestination::Clipboard);
                }
                Ok(())
            }
            WindowCommand::CopyToPrimarySelection => {
                if let Some(text) = selected_text {
                    self.write_text_to_copy_destination(
                        text,
                        WindowCopyDestination::PrimarySelection,
                    );
                }
                Ok(())
            }
            WindowCommand::CopyToClipboardAndPrimarySelection
            | WindowCommand::CompleteSelection
            | WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor => {
                if let Some(text) = selected_text {
                    self.write_text_to_copy_destination(
                        text,
                        WindowCopyDestination::ClipboardAndPrimarySelection,
                    );
                }
                Ok(())
            }
            WindowCommand::CopyTo(destination)
            | WindowCommand::CompleteSelectionTo(destination)
            | WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(destination) => {
                if let Some(text) = selected_text {
                    self.write_text_to_copy_destination(text, destination);
                }
                Ok(())
            }
            WindowCommand::PasteFromClipboard | WindowCommand::Paste => {
                let text = self.read_clipboard_text();
                self.paste_text_to_pane(source_pane_id, text.as_deref(), "clipboard");
                Ok(())
            }
            WindowCommand::PasteFromPrimarySelection | WindowCommand::PastePrimarySelection => {
                let text = self.read_primary_selection_text();
                self.paste_text_to_pane(source_pane_id, text.as_deref(), "primary selection");
                Ok(())
            }
            WindowCommand::PasteFrom(source) => {
                let text = match source {
                    WindowPasteSource::Clipboard => self.read_clipboard_text(),
                    WindowPasteSource::PrimarySelection => self.read_primary_selection_text(),
                };
                self.paste_text_to_pane(source_pane_id, text.as_deref(), "configured source");
                Ok(())
            }
            WindowCommand::SendString(value) => {
                if let Err(error) = self.write_pty_bytes_to_pane(source_pane_id, value.as_bytes()) {
                    eprintln!("quick-select source-bound send string failed: {error}");
                }
                Ok(())
            }
            WindowCommand::SendPaste(value) => {
                let bytes = encode_window_paste(
                    &value,
                    self.pane_bracketed_paste(source_pane_id),
                    self.canonicalize_pasted_newlines,
                );
                if let Err(error) = self.write_pty_bytes_to_pane(source_pane_id, &bytes) {
                    eprintln!("quick-select source-bound send paste failed: {error}");
                }
                Ok(())
            }
            WindowCommand::SendKey(send_key) => {
                if let Err(error) = self.send_key_to_pane(source_pane_id, &send_key) {
                    eprintln!("quick-select source-bound send key failed: {error}");
                }
                Ok(())
            }
            command => self.command_palette_apply_command(command),
        }
    }

    fn clear_ordinary_selection_for_pane(&mut self, pane_id: rssh_core::PaneId) {
        if pane_id == self.app_shell.active_pane_id() {
            self.clear_selection();
            return;
        }
        if let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) {
            runtime.ui.ordinary_selection = None;
            self.frame_needs_full_repaint = true;
        }
    }

    fn paste_text_to_pane(
        &mut self,
        pane_id: rssh_core::PaneId,
        text: Option<&str>,
        source_label: &str,
    ) {
        let Some(text) = text.filter(|text| !text.is_empty()) else {
            return;
        };
        let bytes = encode_window_paste(
            text,
            self.pane_bracketed_paste(pane_id),
            self.canonicalize_pasted_newlines,
        );
        if let Err(error) = self.write_pty_bytes_to_pane(pane_id, &bytes) {
            eprintln!("quick-select source-bound paste from {source_label} failed: {error}");
        }
    }

    fn pane_bracketed_paste(&self, pane_id: rssh_core::PaneId) -> bool {
        if pane_id == self.app_shell.active_pane_id() {
            return self.runtime.bracketed_paste();
        }
        self.pane_runtimes
            .get(&pane_id)
            .is_some_and(|runtime| runtime.runtime.bracketed_paste())
    }

    fn send_key_to_pane(
        &mut self,
        pane_id: rssh_core::PaneId,
        send_key: &WindowSendKey,
    ) -> io::Result<()> {
        let (application_cursor_keys, application_keypad, kitty_keyboard_flags, modify_other_keys) =
            if pane_id == self.app_shell.active_pane_id() {
                (
                    self.runtime.application_cursor_keys(),
                    self.runtime.application_keypad(),
                    self.runtime.kitty_keyboard_flags(),
                    self.runtime.modify_other_keys(),
                )
            } else {
                let Some(runtime) = self.pane_runtimes.get(&pane_id) else {
                    return Ok(());
                };
                (
                    runtime.runtime.application_cursor_keys(),
                    runtime.runtime.application_keypad(),
                    runtime.runtime.kitty_keyboard_flags(),
                    runtime.runtime.modify_other_keys(),
                )
            };
        let text = send_key.text();
        let bytes = encode_window_key_with_kitty_event(
            &send_key.key,
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            text.as_deref(),
            send_key.modifiers,
            application_cursor_keys,
            application_keypad,
            kitty_keyboard_flags
                | (u16::from(self.enable_csi_u_key_encoding) * KITTY_KEYBOARD_DISAMBIGUATE),
            modify_other_keys,
            KittyKeyEventKind::Press,
        );
        self.write_pty_bytes_to_pane(pane_id, &bytes)
    }

    fn send_key_to_active_pane(&mut self, send_key: &WindowSendKey) -> io::Result<()> {
        let text = send_key.text();
        let bytes = encode_window_key_with_kitty_event(
            &send_key.key,
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            text.as_deref(),
            send_key.modifiers,
            self.runtime.application_cursor_keys(),
            self.runtime.application_keypad(),
            self.effective_kitty_keyboard_flags(),
            self.runtime.modify_other_keys(),
            KittyKeyEventKind::Press,
        );
        self.write_pty_bytes(&bytes)
    }

    fn enter_quick_select_mode(&mut self) {
        let alphabet = self.quick_select_alphabet.clone();
        self.enter_quick_select_mode_with_alphabet_and_scope_lines(
            &alphabet,
            DEFAULT_QUICK_SELECT_SCOPE_LINES,
            None,
            None,
            false,
        );
    }

    fn enter_quick_select_mode_with_alphabet_and_scope_lines(
        &mut self,
        alphabet: &str,
        scope_lines: usize,
        action_label: Option<String>,
        action: Option<WindowQuickSelectAction>,
        skip_action_on_paste: bool,
    ) {
        let mut pattern_specs = Vec::new();
        if !self.disable_default_quick_select_patterns {
            pattern_specs.extend(
                QUICK_SELECT_PATTERNS
                    .iter()
                    .map(|pattern| (pattern.regex.to_owned(), pattern.capture)),
            );
        }
        pattern_specs.extend(
            self.quick_select_patterns
                .iter()
                .map(|pattern| (pattern.clone(), None)),
        );
        let patterns = pattern_specs
            .iter()
            .map(|(regex, capture)| WindowQuickSelectPatternRef {
                regex,
                capture: *capture,
            })
            .collect::<Vec<_>>();
        self.enter_quick_select_mode_with_alphabet_and_patterns(
            alphabet,
            &patterns,
            scope_lines,
            action_label,
            action.unwrap_or_default(),
            skip_action_on_paste,
        );
    }

    fn enter_quick_select_mode_with_options(&mut self, options: WindowQuickSelectOptions) {
        let scope_lines = options
            .scope_lines
            .unwrap_or(DEFAULT_QUICK_SELECT_SCOPE_LINES);
        let alphabet = options
            .alphabet
            .unwrap_or_else(|| self.quick_select_alphabet.clone());
        if let Some(patterns) = options.patterns {
            let patterns = patterns
                .iter()
                .map(String::as_str)
                .map(WindowQuickSelectPatternRef::whole)
                .collect::<Vec<_>>();
            self.enter_quick_select_mode_with_alphabet_and_patterns(
                &alphabet,
                &patterns,
                scope_lines,
                options.label,
                options.action.unwrap_or_default(),
                options.skip_action_on_paste,
            );
        } else {
            self.enter_quick_select_mode_with_alphabet_and_scope_lines(
                &alphabet,
                scope_lines,
                options.label,
                options.action,
                options.skip_action_on_paste,
            );
        }
    }

    fn enter_quick_select_mode_with_alphabet_and_patterns(
        &mut self,
        alphabet: &str,
        patterns: &[WindowQuickSelectPatternRef<'_>],
        scope_lines: usize,
        action_label: Option<String>,
        action: WindowQuickSelectAction,
        skip_action_on_paste: bool,
    ) {
        self.cancel_pane_inspection();
        self.command_palette = None;
        self.pane_select = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;

        let (row_start, row_end) = quick_select_source_row_scope(
            self.runtime.terminal(),
            self.current_scrollback_offset(),
            scope_lines,
        );
        let matches = find_window_quick_select_matches_with_patterns(
            self.runtime.terminal(),
            patterns,
            row_start,
            row_end,
        );
        let labels = quick_select_labels_for_alphabet_by_match(alphabet, matches.len());
        let quick_select = WindowQuickSelect {
            current: 0,
            matches,
            labels,
            input: String::new(),
            reflow_config: Some(WindowQuickSelectReflowConfig {
                alphabet: alphabet.to_owned(),
                patterns: patterns
                    .iter()
                    .map(|pattern| (pattern.regex.to_owned(), pattern.capture))
                    .collect(),
                scope_lines,
            }),
            action_label,
            action,
            skip_action_on_paste,
        };
        let current = quick_select.current_match();
        self.active_ui.enter_quick_select(quick_select);

        if let Some(active) = current {
            self.apply_quick_select_match(active);
        } else {
            self.selection = None;
            self.refresh_snapshot();
            self.apply_window_title();
        }
    }

    fn exit_quick_select_mode(&mut self) {
        self.active_ui.exit_overlay();
        self.selection = None;
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn handle_command_palette_logical_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        let Some(palette_query) = self
            .command_palette
            .as_ref()
            .map(|palette| palette.query.clone())
        else {
            return false;
        };
        let palette_selected = self
            .command_palette
            .as_ref()
            .map_or(0, |palette| palette.selected);

        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.exit_command_palette_mode();
                true
            }
            Key::Named(NamedKey::Enter) => {
                let entries = self.command_palette_filtered_entries();
                if let Some(entry) =
                    entries.get(palette_selected.min(entries.len().saturating_sub(1)))
                {
                    self.command_palette_execute_entry(entry.clone())
                } else {
                    false
                }
            }
            Key::Named(NamedKey::ArrowDown) => {
                self.command_palette_move_selection(1);
                false
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.command_palette_move_selection(-1);
                false
            }
            Key::Named(NamedKey::Backspace) => {
                let mut query = palette_query;
                if query.pop().is_some() {
                    self.command_palette_set_query(query);
                } else {
                    self.command_palette_set_query(String::new());
                }
                false
            }
            Key::Character(text)
                if modifiers.control_key() && text == "u" && !modifiers.alt_key() =>
            {
                self.command_palette_set_query(String::new());
                false
            }
            Key::Character(text)
                if !modifiers.control_key() && !modifiers.alt_key() && !text.is_empty() =>
            {
                if self.launcher_should_enter_fuzzy_filter_mode(text) {
                    if let Some(palette) = self.command_palette.as_mut() {
                        palette.launcher_fuzzy_filter = true;
                    }
                    return true;
                }
                if let Some(shortcut) = self.launcher_shortcut_for_key(text) {
                    match shortcut {
                        WindowLauncherShortcut::Execute(entry) => {
                            return self.command_palette_execute_entry(*entry);
                        }
                        WindowLauncherShortcut::Pending(prefix) => {
                            if let Some(palette) = self.command_palette.as_mut() {
                                palette.launcher_shortcut_prefix = prefix;
                            }
                            return true;
                        }
                    }
                }
                if let Some(delta) = self.launcher_default_mode_vi_delta(text) {
                    self.command_palette_move_selection(delta);
                    return false;
                }
                let mut query = palette_query;
                query.push_str(text);
                self.command_palette_set_query(query);
                false
            }
            _ => false,
        }
    }

    fn app_shell_state_id_suffix(&self) -> String {
        format!(
            " [workspace:{} tab:{} pane:{}]",
            self.app_shell.active_workspace_id().get(),
            self.app_shell.active_tab_id().get(),
            self.app_shell.active_pane_id().get()
        )
    }

    fn app_shell_action_for_key(&self, key: &Key, modifiers: ModifiersState) -> Option<AppAction> {
        self.app_shell_action_for_key_with_preference(
            key,
            None,
            modifiers,
            NativeKeyMapPreference::Mapped,
        )
    }

    fn app_shell_action_for_key_event(
        &self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
    ) -> Option<AppAction> {
        self.app_shell_action_for_key_with_preference(
            key,
            Some(physical_key),
            modifiers,
            self.key_map_preference,
        )
    }

    fn handle_default_close_current_tab_shortcut_event(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
        default_assignment_disabled: bool,
    ) -> bool {
        if default_assignment_disabled {
            return false;
        }

        let matches_close_current_tab = ["CTRL+SHIFT+W", "SUPER+W"].iter().any(|keys| {
            window_key_assignment_matches_key_event(
                keys,
                key,
                physical_key,
                modifiers,
                self.key_map_preference,
            )
        });
        if !matches_close_current_tab {
            return false;
        }

        self.request_close_confirmation_or_close(WindowCloseTarget::Tab(
            self.app_shell.active_tab_id(),
        ));
        true
    }

    fn handle_browser_tab_shortcut_event(
        &mut self,
        key: &Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
        default_assignment_disabled: bool,
    ) -> bool {
        if self.tab_shortcut_style != NativeTabShortcutStyle::Browser
            || default_assignment_disabled
            || !modifiers.control_key()
            || modifiers.alt_key()
            || modifiers.super_key()
        {
            return false;
        }

        let key_matches = |token: &str| {
            window_key_assignment_matches_key_event(
                token,
                key,
                physical_key,
                modifiers,
                self.key_map_preference,
            )
        };

        if modifiers.shift_key() {
            if key_matches("CTRL+SHIFT+T") {
                if let Err(error) = self.dispatch_reopen_closed_tab() {
                    eprintln!("browser reopen closed tab failed: {error:?}");
                }
                return true;
            }
            return false;
        }

        if key_matches("CTRL+T") {
            self.enter_launcher_mode();
            return true;
        }
        if key_matches("CTRL+W") {
            self.request_close_confirmation_or_close(WindowCloseTarget::Tab(
                self.app_shell.active_tab_id(),
            ));
            return true;
        }

        for index in 0..8 {
            let key = format!("CTRL+{}", index + 1);
            if key_matches(&key) {
                if let Err(error) = self.dispatch_app_action(AppAction::ActivateTabIndex { index }) {
                    eprintln!("browser activate tab failed: {error:?}");
                }
                return true;
            }
        }
        if key_matches("CTRL+9") {
            if let Err(error) = self.dispatch_app_action(AppAction::ActivateTabIndex { index: -1 }) {
                eprintln!("browser activate last tab failed: {error:?}");
            }
            return true;
        }
        false
    }

    #[allow(clippy::too_many_lines)]
    fn app_shell_action_for_key_with_preference(
        &self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
        key_map_preference: NativeKeyMapPreference,
    ) -> Option<AppAction> {
        if self.default_assignment_disabled_for_key_with_preference(
            key,
            physical_key,
            modifiers,
            key_map_preference,
        ) {
            return None;
        }

        let key_matches = |token: &str| {
            window_key_assignment_key_matches(token, key, physical_key, key_map_preference)
        };
        let map_index = || window_default_tab_index_for_key(key, physical_key, key_map_preference);

        if modifiers.super_key() && !modifiers.control_key() && !modifiers.alt_key() {
            if !modifiers.shift_key() {
                if let Some(index) = map_index() {
                    return Some(AppAction::ActivateTabIndex { index });
                }

                if key_matches("N") {
                    return Some(AppAction::SpawnWindow { launch: None });
                }
                if key_matches("T") {
                    return Some(AppAction::NewTab { launch: None });
                }
                if key_matches("W") {
                    return Some(AppAction::CloseTab {
                        tab: self.app_shell.active_tab_id(),
                        switch_to_last_active: self.switch_to_last_active_tab_when_closing_tab,
                    });
                }
                return None;
            }

            if key_matches("T") {
                return is_local_domain_name(&self.default_domain)
                    .then_some(AppAction::NewTab { launch: None });
            }
            return match key {
                Key::Character(character) if character == "]" || character == "}" => {
                    Some(AppAction::ActivateTabRelative { offset: 1 })
                }
                Key::Character(character) if character == "[" || character == "{" => {
                    Some(AppAction::ActivateTabRelative { offset: -1 })
                }
                _ => None,
            };
        }

        if !modifiers.control_key() {
            return None;
        }

        if modifiers.alt_key() {
            if modifiers.shift_key() {
                match key {
                    Key::Named(NamedKey::ArrowLeft) => {
                        return Some(AppAction::ResizePane {
                            pane: self.app_shell.active_pane_id(),
                            direction: ResizeDirection::Left,
                            amount: 1,
                        });
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        return Some(AppAction::ResizePane {
                            pane: self.app_shell.active_pane_id(),
                            direction: ResizeDirection::Right,
                            amount: 1,
                        });
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        return Some(AppAction::ResizePane {
                            pane: self.app_shell.active_pane_id(),
                            direction: ResizeDirection::Up,
                            amount: 1,
                        });
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        return Some(AppAction::ResizePane {
                            pane: self.app_shell.active_pane_id(),
                            direction: ResizeDirection::Down,
                            amount: 1,
                        });
                    }
                    Key::Character(character) if character == "\"" => {
                        return Some(AppAction::SplitPane {
                            pane: self.app_shell.active_pane_id(),
                            direction: SplitDirection::Down,
                            launch: None,
                        });
                    }
                    Key::Character(character) if character == "%" => {
                        return Some(AppAction::SplitPane {
                            pane: self.app_shell.active_pane_id(),
                            direction: SplitDirection::Right,
                            launch: None,
                        });
                    }
                    _ => return None,
                }
            }

            return None;
        }

        if !modifiers.shift_key() {
            match key {
                Key::Named(NamedKey::Tab | NamedKey::PageDown) => {
                    return Some(AppAction::ActivateTabRelative { offset: 1 });
                }
                Key::Named(NamedKey::PageUp) => {
                    return Some(AppAction::ActivateTabRelative { offset: -1 });
                }
                _ => return None,
            }
        }

        if let Some(index) = map_index() {
            return Some(AppAction::ActivateTabIndex { index });
        }

        if key_matches("N") {
            return Some(AppAction::SpawnWindow { launch: None });
        }
        if key_matches("T") {
            return Some(AppAction::NewTab { launch: None });
        }
        if key_matches("Z") {
            return Some(AppAction::TogglePaneZoom {
                pane: self.app_shell.active_pane_id(),
            });
        }
        if key_matches("W") {
            return Some(AppAction::CloseTab {
                tab: self.app_shell.active_tab_id(),
                switch_to_last_active: self.switch_to_last_active_tab_when_closing_tab,
            });
        }

        match key {
            Key::Named(NamedKey::Tab) if modifiers.shift_key() && !modifiers.alt_key() => {
                Some(AppAction::ActivateTabRelative { offset: -1 })
            }
            Key::Named(NamedKey::Tab) => Some(AppAction::ActivateTabRelative { offset: 1 }),
            Key::Named(NamedKey::ArrowLeft) => Some(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Left,
            }),
            Key::Named(NamedKey::ArrowRight) => Some(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Right,
            }),
            Key::Named(NamedKey::ArrowUp) => Some(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Up,
            }),
            Key::Named(NamedKey::ArrowDown) => Some(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Down,
            }),
            Key::Named(NamedKey::PageUp) => Some(AppAction::MoveTabRelative { offset: -1 }),
            Key::Named(NamedKey::PageDown) => Some(AppAction::MoveTabRelative { offset: 1 }),
            _ => None,
        }
    }

    fn default_assignment_disabled_for_key(&self, key: &Key, modifiers: ModifiersState) -> bool {
        self.default_assignment_disabled_for_key_with_preference(
            key,
            None,
            modifiers,
            NativeKeyMapPreference::Mapped,
        )
    }

    fn default_assignment_disabled_for_key_with_preference(
        &self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
        key_map_preference: NativeKeyMapPreference,
    ) -> bool {
        if self.disable_default_key_bindings {
            return true;
        }

        self.key_assignments.iter().any(|assignment| {
            assignment.command == WindowCommand::DisableDefaultAssignment
                && window_key_assignment_matches_with_leader(
                    &assignment.keys,
                    key,
                    physical_key,
                    modifiers,
                    key_map_preference,
                    false,
                )
        })
    }

    #[cfg(test)]
    fn new_with_session_log(
        frame_limit: Option<u64>,
        session_log: impl Write + Send + 'static,
    ) -> Self {
        let mut app = Self::new(frame_limit);
        app.session_log = Some(Box::new(session_log));
        app
    }

    #[expect(
        clippy::too_many_lines,
        reason = "native window creation keeps platform setup and presentation ownership atomic"
    )]
    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let initial_size = self.initial_frame_size();
        #[cfg(target_os = "windows")]
        let chrome_policy = native_window_chrome_policy(self.window_decorations);
        let mut window_attributes = Window::default_attributes()
            .with_title(self.window_title.clone())
            .with_inner_size(initial_size)
            .with_resizable(self.window_decorations.resize)
            .with_decorations(self.window_decorations.winit_decorations_enabled());
        #[cfg(target_os = "windows")]
        {
            window_attributes =
                window_attributes.with_undecorated_shadow(chrome_policy.undecorated_shadow);
        }
        #[cfg(target_os = "macos")]
        {
            let chrome_policy = native_macos_window_chrome_policy(self.window_decorations);
            if chrome_policy.unified_titlebar {
                window_attributes = window_attributes
                    .with_titlebar_transparent(true)
                    .with_title_hidden(true)
                    .with_fullsize_content_view(true)
                    // The terminal body must remain selectable. Window drag
                    // starts only from non-interactive tab-bar cells below.
                    .with_movable_by_window_background(false);
            }
            window_attributes = window_attributes.with_has_shadow(chrome_policy.has_shadow);
        }
        if let Some(position) = self.initial_window_position.as_ref() {
            let primary_monitor_position = event_loop
                .primary_monitor()
                .map(|monitor| monitor.position());
            let monitor_positions = match &position.origin {
                WindowPositionOrigin::Monitor(_) => event_loop
                    .available_monitors()
                    .map(|monitor| NativeMonitorPosition {
                        name: monitor.name(),
                        position: monitor.position(),
                    })
                    .collect::<Vec<_>>(),
                WindowPositionOrigin::Screen
                | WindowPositionOrigin::Main
                | WindowPositionOrigin::Active => Vec::new(),
            };
            let Some(resolved_position) = resolve_initial_window_position(
                position,
                primary_monitor_position,
                primary_monitor_position,
                &monitor_positions,
            ) else {
                let WindowPositionOrigin::Monitor(name) = &position.origin else {
                    unreachable!("only named monitor positions can fail to resolve");
                };
                return Err(format!("monitor not found for --position: {name}").into());
            };
            window_attributes = window_attributes.with_position(resolved_position);
        }
        if let Some(resize_increments) = self.window_resize_increments() {
            window_attributes = window_attributes.with_resize_increments(resize_increments);
        }
        window_attributes =
            window_attributes_with_class(window_attributes, self.initial_window_class.as_deref());

        let window = Arc::new(event_loop.create_window(window_attributes)?);
        // winit keeps IME delivery disabled by default.  Without explicitly
        // enabling it, macOS can show the candidate UI but no Commit/Preedit
        // events reach the terminal input path.
        window.set_ime_allowed(self.use_ime);
        #[cfg(target_os = "windows")]
        if chrome_policy.rounded_corners {
            window.set_corner_preference(CornerPreference::Round);
        }
        self.apply_window_scale_factor(startup_window_scale_factor(
            self.diagnostic_scale_override_enabled(),
            window.scale_factor(),
        ));
        let size = window
            .request_inner_size(self.initial_frame_size())
            .unwrap_or_else(|| window.inner_size());
        self.frame_width = size.width;
        self.frame_height = size.height;
        self.window = Some(window);
        if self.renderer_mode == RendererMode::Gpu {
            self.presentation_owner = PresentationOwner::GpuInitializing;
            self.metrics.mark_gpu_started();
            let gpu = self.initialize_gpu(event_loop)?;
            self.gpu_owners.active = Some(Box::new(gpu));
            self.presentation_owner = PresentationOwner::GpuActive;
            self.metrics.mark_gpu_finished();
        } else {
            let bootstrap = WindowBootstrapSurface::new(
                event_loop,
                self.window
                    .as_ref()
                    .expect("window is installed before bootstrap surface")
                    .clone(),
                size,
            )?;
            self.bootstrap_surface = Some(bootstrap);
            self.presentation_owner = PresentationOwner::Bootstrap;
        }
        self.last_ime_cursor_area.set(None);
        if let Some(window) = &self.window {
            window.set_cursor_visible(self.mouse_cursor_visible);
        }
        self.refresh_window_frame_from_window();
        self.update_ime_cursor_area();
        self.mark_diagnostic_window_created();

        Ok(())
    }

    fn initialize_gpu(&mut self, event_loop: &ActiveEventLoop) -> Result<WindowGpu, Box<dyn Error>> {
        let window = self
            .window
            .clone()
            .ok_or_else(|| io::Error::other("window is not available for GPU initialization"))?;
        let size = window.inner_size();
        let high_performance = matches!(
            self.webgpu_power_preference,
            NativeWebGpuPowerPreference::HighPerformance
        );
        let force_fallback_adapter = effective_force_fallback_adapter(
            self.webgpu_force_fallback_adapter,
            matches!(self.front_end, NativeRenderFrontEnd::Software),
        );
        let (font_mode, font_specimen) = self.diagnostic_font_options();
        if self.startup.diagnostic_gpu_backend.is_some() || font_mode.is_some() {
            pollster::block_on(WindowGpu::new_with_diagnostic_options(
                event_loop.owned_display_handle(),
                window,
                size,
                high_performance,
                force_fallback_adapter,
                self.startup.diagnostic_gpu_backend,
                font_mode,
                font_specimen,
            ))
        } else {
            pollster::block_on(WindowGpu::new(
                event_loop.owned_display_handle(),
                window,
                size,
                high_performance,
                force_fallback_adapter,
            ))
        }
    }

    fn initialize_deferred_gpu(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer_mode != RendererMode::Auto
            || self.presentation_owner != PresentationOwner::Bootstrap
            || self.rendered_frames == 0
            || self.gpu_owners.active.is_some()
        {
            return;
        }

        if let Err(error) = crate::stage7_attribution::audit_product_service_start(
            crate::stage7_attribution::ProductServiceEntry::PostReadyCoordinator,
        ) {
            eprintln!("deferred GPU initialization blocked by scheduling audit: {error}");
            self.activate_cpu_fallback();
            return;
        }
        self.presentation_owner = PresentationOwner::GpuInitializing;
        self.deferred_gpu_generation = self.deferred_gpu_generation.saturating_add(1);
        let generation = self.deferred_gpu_generation;
        self.metrics.mark_gpu_started();
        let Some(window) = self.window.clone() else {
            self.metrics.mark_gpu_finished();
            eprintln!("deferred GPU initialization failed; using CPU renderer: window closed");
            self.activate_cpu_fallback();
            return;
        };
        let Some(event_proxy) = self.event_proxy.clone() else {
            self.metrics.mark_gpu_finished();
            eprintln!(
                "deferred GPU initialization failed; using CPU renderer: event proxy unavailable"
            );
            self.activate_cpu_fallback();
            return;
        };
        let display = event_loop.owned_display_handle();
        let surface_size = window.inner_size();
        let high_performance = matches!(
            self.webgpu_power_preference,
            NativeWebGpuPowerPreference::HighPerformance
        );
        let force_fallback_adapter = effective_force_fallback_adapter(
            self.webgpu_force_fallback_adapter,
            matches!(self.front_end, NativeRenderFrontEnd::Software),
        );
        let (font_mode, font_specimen) = self.diagnostic_font_options();
        let prepared_gpu = match if self.startup.diagnostic_gpu_backend.is_some() || font_mode.is_some() {
            WindowGpu::prepare_with_diagnostic_options(
                display,
                window,
                surface_size,
                high_performance,
                force_fallback_adapter,
                self.startup.diagnostic_gpu_backend,
                font_mode,
                font_specimen,
            )
        } else {
            WindowGpu::prepare(
                display,
                window,
                surface_size,
                high_performance,
                force_fallback_adapter,
            )
        } {
            Ok(prepared) => prepared,
            Err(error) => {
                self.metrics.mark_gpu_finished();
                eprintln!("deferred GPU initialization failed; using CPU renderer: {error}");
                self.activate_cpu_fallback();
                return;
            }
        };
        let window_id = self.app_window_id;
        let task_name = format!("rssh-gpu-init-{window_id:?}");
        if let Err(error) = spawn_deferred_gpu_task(task_name, move || {
            let outcome = if test_deferred_gpu_init_failure() {
                DeferredGpuInitialization::Failed(
                    "injected deferred GPU initialization failure".to_owned(),
                )
            } else {
                match pollster::block_on(WindowGpu::finish_prepared(prepared_gpu)) {
                    Ok(gpu) => DeferredGpuInitialization::Ready(Box::new(gpu)),
                    Err(error) => DeferredGpuInitialization::Failed(error.to_string()),
                }
            };
            let _ = event_proxy.send_event(WindowUserEvent::DeferredGpuInitialized {
                window_id,
                generation,
                outcome,
            });
        }) {
            self.metrics.mark_gpu_finished();
            eprintln!("deferred GPU initialization failed; using CPU renderer: {error}");
            self.activate_cpu_fallback();
        }
    }

    fn handle_deferred_gpu_initialized(
        &mut self,
        generation: u64,
        outcome: DeferredGpuInitialization,
    ) -> bool {
        if generation != self.deferred_gpu_generation
            || self.renderer_mode != RendererMode::Auto
            || self.presentation_owner != PresentationOwner::GpuInitializing
        {
            return false;
        }

        self.metrics.mark_gpu_finished();
        match outcome {
            DeferredGpuInitialization::Ready(mut gpu) => {
                let Some(current_size) = self.window.as_ref().map(|window| window.inner_size())
                else {
                    eprintln!(
                        "deferred GPU initialization finished after the window closed; using CPU renderer"
                    );
                    self.activate_cpu_fallback();
                    return true;
                };
                match resize_deferred_gpu_candidate_for_install(current_size, |size| {
                    gpu.resize_surface(size)
                }) {
                    Ok(owner) => {
                        self.gpu_owners.active = Some(gpu);
                        self.presentation_owner = owner;
                        self.pending_frame_damage.clear();
                        self.frame_needs_full_repaint = true;
                        if let Some(window) = self.window.as_ref() {
                            window.request_redraw();
                        }
                    }
                    Err(error) => {
                        eprintln!(
                            "deferred GPU surface resize failed; using CPU renderer: {error}"
                        );
                        self.activate_cpu_fallback();
                    }
                }
            }
            DeferredGpuInitialization::Failed(error) => {
                eprintln!("deferred GPU initialization failed; using CPU renderer: {error}");
                self.activate_cpu_fallback();
            }
        }
        true
    }

    fn activate_cpu_fallback(&mut self) {
        self.gpu_owners.quarantine_active();
        self.metrics.mark_renderer(RendererKind::Cpu);
        self.presentation_owner = deferred_gpu_initialization_owner(false);
        self.pending_frame_damage.clear();
        self.frame_needs_full_repaint = true;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn window_id(&self) -> Option<winit::window::WindowId> {
        self.window.as_ref().map(|window| window.id())
    }

    fn apply_window_scale_factor(&mut self, scale_factor: f64) {
        self.detected_window_dpi = window_dpi_from_scale_factor(scale_factor);
        self.apply_effective_window_dpi();
    }

    fn apply_effective_window_dpi(&mut self) {
        let window_dpi = self.configured_dpi.unwrap_or(self.detected_window_dpi);
        self.window_dpi = window_dpi;
        self.renderer.set_window_dpi(window_dpi);
        self.apply_window_resize_increments();
        if let Some(window) = self.window.as_ref() {
            let terminal_size = self.runtime.terminal().grid().size();
            let requested_size = self.frame_size_for_terminal_size(terminal_size);
            if window.inner_size() != requested_size {
                let _ = window.request_inner_size(requested_size);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_frame(&mut self, event_loop: &ActiveEventLoop) {
        self.refresh_renderer_animation_clock();
        self.update_ime_cursor_area();
        let scrollbar = self.scrollback_scrollbar();
        let surface_geometry = self.render_geometry();
        let placement = self.frame_content_placement();
        let geometry = self.frame_render_geometry(surface_geometry, placement);
        let snapshot = self.with_diagnostic_font_specimen(self.render_snapshot());
        self.metrics.record_terminal_linkage_snapshot(&snapshot);
        if self.final_linkage_frame_is_reserved() {
            return;
        }
        if self.frame_limit_reached() {
            // A frame limit is an exact presentation contract.  When a probe
            // also asks for PTY linkage, later output may still need to reach
            // the terminal snapshot after the final frame.  Observe that
            // state, but never prepare or present an additional frame while
            // waiting for it.
            if self.frame_limit_probe_ready() {
                event_loop.exit();
            }
            return;
        }
        let gpu_ready_to_present = self.presentation_owner == PresentationOwner::GpuActive
            || (self.presentation_owner == PresentationOwner::GpuInitializing
                && self.gpu_owners.active.is_some());
        if !gpu_ready_to_present {
            if test_ssh_gui_frame_limit().is_some()
                && self.presentation_owner == PresentationOwner::GpuInitializing
            {
                return;
            }
            self.draw_cpu_frame(event_loop, &snapshot, scrollbar, geometry, placement);
            return;
        }
        let damage_row_offset = self.terminal_frame_row_offset();
        if self.has_visible_split_layout() {
            self.frame_needs_full_repaint = true;
        }
        let started = Instant::now();
        let mode = if self.presentation_owner == PresentationOwner::GpuInitializing
            || self.frame_needs_full_repaint
            || self.pending_frame_damage.is_empty()
        {
            FrameRenderMode::Full
        } else {
            FrameRenderMode::Damage
        };
        let damage = if mode == FrameRenderMode::Damage {
            offset_damage_regions(self.pending_frame_damage.clone(), damage_row_offset)
        } else {
            Vec::new()
        };
        let graph =
            self.renderer
                .prepare_gpu_frame(&snapshot, geometry, scrollbar, damage_row_offset);
        let gpu_dpi_scale = self.gpu_dpi_scale();

        let outcome = if let (Some(gpu), Some(window)) = (self.gpu_owners.active.as_mut(), self.window.as_ref()) {
            gpu.present(
                window,
                &snapshot,
                geometry,
                &damage,
                &self.renderer.text_paint_config(),
                &graph,
                mode,
                gpu_dpi_scale,
            )
        } else {
            Ok(GpuFrameStatus::Skipped)
        };
        let frame_status = outcome
            .as_ref()
            .copied()
            .unwrap_or(GpuFrameStatus::Skipped);
        let host_state = &mut self.interaction_state.host_state;
        let presented = match finalize_native_gpu_frame(
            outcome,
            &mut host_state.pending_frame_damage,
            &mut host_state.frame_needs_full_repaint,
        ) {
            Ok(presented) => presented,
            Err(error) => {
                eprintln!("render error: {error}");
                if self.bootstrap_surface.is_some() {
                    self.activate_cpu_fallback();
                } else {
                    event_loop.exit();
                }
                return;
            }
        };

        if presented && self.rendered_frames == 0 {
            self.record_first_present(RendererKind::Gpu, &snapshot);
        }
        if presented {
            self.advance_ssh1_diagnostic_after_present(DiagnosticRendererKind::Gpu, &snapshot);
        }

        if presented {
            let missing_glyphs = self
                .gpu_owners.active
                .as_ref()
                .and_then(|gpu| gpu.direct_text_metrics())
                .map(|(report, _)| report.missing_glyphs.clone())
                .unwrap_or_default();
            self.record_missing_glyph_codepoints(missing_glyphs);
        }

        self.metrics.record_render_frame(started.elapsed());
        if !presented {
            return;
        }
        self.metrics.record_frame_render_mode(mode);
        self.rendered_frames = self.rendered_frames.saturating_add(1);
        let previous_owner = self.presentation_owner;
        self.presentation_owner =
            presentation_owner_after_gpu_frame(self.presentation_owner, frame_status);
        if previous_owner != PresentationOwner::GpuActive
            && self.presentation_owner == PresentationOwner::GpuActive
        {
            self.metrics.mark_renderer(RendererKind::Gpu);
            self.mark_diagnostic_gpu_ready();
        }
        if self.presentation_owner == PresentationOwner::GpuActive {
            // Keep the softbuffer surface (but release the RGBA staging
            // vector) as a recoverable presentation path.  A later device or
            // surface failure can therefore switch back to CPU without a
            // white frame or an event-loop restart.
            release_bootstrap_staging_after_gpu_activation(&mut self.bootstrap_frame);
        }
        if self.rendered_frames == 1 && self.is_benchmark_startup() {
            event_loop.exit();
            return;
        }
        if self.rendered_frames == 1
            && let Some(size) = test_resize_after_first_present()
            && let Some(window) = self.window.as_ref()
        {
            let _ = window.request_inner_size(size);
            window.request_redraw();
        }
        if self.frame_limit_probe_ready() {
            event_loop.exit();
        }
    }

    fn draw_cpu_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        snapshot: &TerminalRenderSnapshot,
        scrollbar: Option<ScrollbackScrollbar>,
        geometry: RenderGeometry,
        placement: NativeFrameContentPlacement,
    ) {
        if self.bootstrap_surface.is_none() {
            eprintln!("CPU renderer has no bootstrap surface");
            event_loop.exit();
            return;
        }
        let Some(frame_len) = usize::try_from(geometry.target_width)
            .ok()
            .and_then(|width| {
                usize::try_from(geometry.target_height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
        else {
            eprintln!("CPU renderer frame size overflow");
            event_loop.exit();
            return;
        };
        let mut bootstrap_frame = std::mem::take(&mut self.bootstrap_frame);
        bootstrap_frame.resize(frame_len, 0);
        let damage_row_offset = self.terminal_frame_row_offset();
        let mut pending_frame_damage = std::mem::take(&mut self.pending_frame_damage);
        let mut frame_needs_full_repaint =
            self.frame_needs_full_repaint || self.has_visible_split_layout();
        let background_color = self.background_color;
        let started = Instant::now();
        let mode = render_framebuffer_with_state(
            &self.renderer,
            snapshot,
            scrollbar,
            &mut pending_frame_damage,
            &mut frame_needs_full_repaint,
            &mut bootstrap_frame,
            geometry,
            damage_row_offset,
            placement,
            color_to_rgba(background_color, DEFAULT_RENDER_BACKGROUND_RGBA),
        );
        self.pending_frame_damage = pending_frame_damage;
        self.frame_needs_full_repaint = frame_needs_full_repaint;
        self.bootstrap_frame = bootstrap_frame;
        self.metrics.record_terminal_linkage_snapshot(snapshot);
        let Some(surface) = self.bootstrap_surface.as_mut() else {
            eprintln!("CPU renderer lost bootstrap surface");
            event_loop.exit();
            return;
        };
        if surface.size() != PhysicalSize::new(geometry.target_width, geometry.target_height)
            && let Err(error) = surface.resize(PhysicalSize::new(
                geometry.target_width,
                geometry.target_height,
            ))
        {
            eprintln!("CPU surface resize failed: {error}");
            event_loop.exit();
            return;
        }
        if let Err(error) = surface.present_rgba(&self.bootstrap_frame) {
            eprintln!("CPU surface present failed: {error}");
            self.presentation_owner = PresentationOwner::CpuFallback;
            event_loop.exit();
            return;
        }
        if self.rendered_frames == 0 {
            self.record_first_present(RendererKind::Cpu, snapshot);
        }
        self.advance_ssh1_diagnostic_after_present(DiagnosticRendererKind::Cpu, snapshot);
        self.metrics.record_render_frame(started.elapsed());
        self.metrics.record_frame_render_mode(mode);
        self.rendered_frames = self.rendered_frames.saturating_add(1);
        if self.rendered_frames == 1 {
            if self.is_benchmark_startup() {
                event_loop.exit();
                return;
            }
            if let Some(size) = test_resize_after_first_present()
                && let Some(window) = self.window.as_ref()
            {
                let _ = window.request_inner_size(size);
                window.request_redraw();
            }
        }
        // Native SSH connects as soon as the first CPU frame is visible so
        // network setup can overlap deferred configuration/GPU work.  Local
        // PTYs are started by the manager after that configuration attempt;
        // keeping them out of this branch avoids spawning against defaults.
        if !self.suppresses_transport_start()
            && !self.transport_start_requested
            && matches!(
                self.app_shell.active_pane().launch().domain(),
                PaneLaunchDomain::Ssh(_)
            )
            && let Err(error) = self.spawn_pty()
            && self.handle_transport_start_error(error.as_ref())
        {
            event_loop.exit();
            return;
        }
        self.initialize_deferred_gpu(event_loop);
        if self.frame_limit_probe_ready() {
            event_loop.exit();
        }
    }

    /// Keeps the native IME candidate window anchored to the active terminal
    /// cell.  winit expects physical client-area coordinates and converts them
    /// to the platform's logical coordinate system (including macOS's Retina
    /// scale) before asking `AppKit` for the candidate rectangle.
    fn update_ime_cursor_area(&self) {
        if !self.use_ime {
            self.last_ime_cursor_area.set(None);
            return;
        }
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let layout = self.pane_render_layout();
        let active_pane = self.app_shell.active_pane_id();
        let Some(rect) = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == active_pane)
            .copied()
        else {
            return;
        };
        if rect.rows == 0 || rect.columns == 0 {
            return;
        }

        let (cursor_row, cursor_column) = self.runtime.terminal().cursor();
        let Some((position, size)) = Self::ime_cursor_area_pixels(
            self.frame_content_pixel_left(),
            self.frame_content_pixel_top(),
            self.cell_width(),
            self.cell_height(),
            rect,
            cursor_row,
            cursor_column,
        ) else {
            return;
        };
        let key = (position.x, position.y, size.width, size.height);
        if self.last_ime_cursor_area.get() == Some(key) {
            return;
        }
        window.set_ime_cursor_area(position, size);
        self.last_ime_cursor_area.set(Some(key));
    }

    fn ime_cursor_area_pixels(
        frame_content_pixel_left: u32,
        frame_content_pixel_top: u32,
        cell_width: u32,
        cell_height: u32,
        rect: PaneRenderRect,
        cursor_row: u16,
        cursor_column: u16,
    ) -> Option<(PhysicalPosition<u32>, PhysicalSize<u32>)> {
        if rect.rows == 0 || rect.columns == 0 || cell_width == 0 || cell_height == 0 {
            return None;
        }
        let row = cursor_row.min(rect.rows.saturating_sub(1));
        let column = cursor_column.min(rect.columns.saturating_sub(1));
        let x = frame_content_pixel_left
            .saturating_add(u32::from(rect.column).saturating_mul(cell_width))
            .saturating_add(u32::from(column).saturating_mul(cell_width));
        let y = frame_content_pixel_top
            .saturating_add(u32::from(rect.row).saturating_mul(cell_height))
            .saturating_add(u32::from(row).saturating_mul(cell_height));
        Some((
            PhysicalPosition::new(x, y),
            PhysicalSize::new(cell_width, cell_height),
        ))
    }

    #[cfg(test)]
    fn render_framebuffer(&mut self, frame: &mut [u8]) -> FrameRenderMode {
        self.refresh_renderer_animation_clock();
        let scrollbar = self.scrollback_scrollbar();
        let placement = self.frame_content_placement();
        let geometry = self.frame_render_geometry(self.render_geometry(), placement);
        let mut pending_frame_damage = std::mem::take(&mut self.pending_frame_damage);
        let mut frame_needs_full_repaint =
            self.frame_needs_full_repaint || self.has_visible_split_layout();
        let damage_row_offset = self.terminal_frame_row_offset();
        let background_color = self.background_color;
        let snapshot = self.render_snapshot();
        let missing_glyphs = snapshot.missing_glyphs().clone();
        let mode = render_framebuffer_with_state(
            &self.renderer,
            &snapshot,
            scrollbar,
            &mut pending_frame_damage,
            &mut frame_needs_full_repaint,
            frame,
            geometry,
            damage_row_offset,
            placement,
            color_to_rgba(background_color, DEFAULT_RENDER_BACKGROUND_RGBA),
        );
        drop(snapshot);
        self.pending_frame_damage = pending_frame_damage;
        self.frame_needs_full_repaint = frame_needs_full_repaint;
        self.record_missing_glyph_codepoints(missing_glyphs);
        self.metrics.record_frame_render_mode(mode);
        mode
    }

    fn record_missing_glyph_codepoints<I>(&mut self, glyphs: I)
    where
        I: IntoIterator<Item = char>,
    {
        if !self.warn_about_missing_glyphs {
            return;
        }

        for glyph in glyphs {
            if !self.missing_glyph_warning_codepoints.insert(glyph) {
                continue;
            }

            let warning = missing_glyph_warning(glyph);
            eprintln!("{warning}");
            self.missing_glyph_warnings.push(warning);
        }
    }

    fn refresh_renderer_animation_clock(&mut self) -> u64 {
        let elapsed_ms =
            u64::try_from(self.animation_started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.renderer.set_animation_elapsed_ms(elapsed_ms);
        elapsed_ms
    }

    fn redraw_request_interval(&self) -> Duration {
        let fps = u32::try_from(self.max_fps.max(1)).unwrap_or(u32::MAX);
        Duration::from_secs_f64(1.0 / f64::from(fps))
    }

    fn animation_redraw_request_interval(&self) -> Duration {
        let fps = u32::try_from(self.animation_fps.max(1)).unwrap_or(u32::MAX);
        Duration::from_secs_f64(1.0 / f64::from(fps))
    }

    fn should_request_redraw_at(&mut self, now: Instant) -> bool {
        if self.last_redraw_request_at.is_some_and(|last| {
            now.saturating_duration_since(last) < self.redraw_request_interval()
        }) {
            return false;
        }

        self.last_redraw_request_at = Some(now);
        true
    }

    fn should_request_animation_redraw_at(&mut self, now: Instant) -> bool {
        if !self.has_active_animation_at(now) {
            return false;
        }

        if self.last_animation_redraw_request_at.is_some_and(|last| {
            now.saturating_duration_since(last) < self.animation_redraw_request_interval()
        }) {
            return false;
        }

        if !self.should_request_redraw_at(now) {
            return false;
        }

        self.last_animation_redraw_request_at = Some(now);
        true
    }

    fn request_redraw_if_due(&mut self, now: Instant) {
        if !self.should_request_redraw_at(now) {
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn request_animation_redraw_if_due(&mut self, now: Instant) {
        if !self.should_request_animation_redraw_at(now) {
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn has_active_animation_at(&self, now: Instant) -> bool {
        self.has_active_cursor_blink_animation()
            || self.has_active_text_blink_animation()
            || self.has_active_visual_bell_at(now)
            || self.has_active_inline_image_animation()
    }

    fn has_active_cursor_blink_animation(&self) -> bool {
        !self.cursor_blink_rate.is_zero()
            && self.snapshot.cursor().is_some_and(|cursor| cursor.blinking)
    }

}
