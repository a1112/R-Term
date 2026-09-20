impl NativeWindowApp {
    fn native_resolved_palette(&self) -> NativeResolvedPalette {
        let (ansi, brights) =
            native_split_ansi_palette(self.ansi_palette.unwrap_or(DEFAULT_ANSI_PALETTE_COLORS));
        NativeResolvedPalette {
            foreground: self.foreground_color,
            background: self.background_color,
            cursor_fg: self.cursor_fg_color,
            cursor_bg: self.cursor_bg_color,
            cursor_border: self.cursor_border_color,
            selection_fg: self.selection_fg_color,
            selection_bg: self.selection_bg_color,
            ansi,
            brights,
            indexed: self.indexed_palette.unwrap_or([None; 256]),
            tab_bar_background: self.tab_bar_background_color,
            tab_bar_inactive_tab_edge: self.tab_bar_inactive_tab_edge_color,
            tab_bar_active_tab: self.tab_bar_active_tab_colors,
            tab_bar_inactive_tab: self.tab_bar_inactive_tab_colors,
            tab_bar_inactive_tab_hover: self.tab_bar_inactive_tab_hover_colors,
            tab_bar_new_tab: self.tab_bar_new_tab_colors,
            tab_bar_new_tab_hover: self.tab_bar_new_tab_hover_colors,
            scrollbar_thumb: self.scrollbar_thumb_color,
            split: self.split_color,
            visual_bell: self.visual_bell_color,
            compose_cursor: self.compose_cursor_color,
            copy_mode_active_highlight_fg: self.copy_mode_active_highlight_fg,
            copy_mode_active_highlight_bg: self.copy_mode_active_highlight_bg,
            copy_mode_inactive_highlight_fg: self.copy_mode_inactive_highlight_fg,
            copy_mode_inactive_highlight_bg: self.copy_mode_inactive_highlight_bg,
            quick_select_label_fg: self.quick_select_label_fg,
            quick_select_label_bg: self.quick_select_label_bg,
            quick_select_match_fg: self.quick_select_match_fg,
            quick_select_match_bg: self.quick_select_match_bg,
            input_selector_label_fg: self.input_selector_label_fg,
            input_selector_label_bg: self.input_selector_label_bg,
            launcher_label_fg: self.launcher_label_fg,
            launcher_label_bg: self.launcher_label_bg,
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn native_effective_config(&self) -> NativeConfigView {
        NativeConfigView {
    dpi: self.window_dpi,
    dpi_by_screen: self.dpi_by_screen.clone(),
    tab_max_width: self.tab_max_width,
    tab_min_width: self.tab_min_width,
    status_update_interval: u64::try_from(self.status_update_interval.as_millis())
                    .unwrap_or(u64::MAX),
    status_update_interval_ms: u64::try_from(self.status_update_interval.as_millis())
                    .unwrap_or(u64::MAX),
    max_fps: self.max_fps,
    animation_fps: self.animation_fps,
    front_end: self.front_end,
    webgpu_power_preference: self.webgpu_power_preference,
    webgpu_force_fallback_adapter: self.webgpu_force_fallback_adapter,
    webgpu_preferred_adapter: self.webgpu_preferred_adapter.clone(),
    prefer_egl: self.prefer_egl,
    enable_wayland: self.enable_wayland,
    enable_zwlr_output_manager: self.enable_zwlr_output_manager,
    use_box_model_render: self.use_box_model_render,
    experimental_pixel_positioning: self.experimental_pixel_positioning,
    shape_cache_size: self.shape_cache_size,
    line_state_cache_size: self.line_state_cache_size,
    line_quad_cache_size: self.line_quad_cache_size,
    line_to_ele_shape_cache_size: self.line_to_ele_shape_cache_size,
    glyph_cache_image_cache_size: self.glyph_cache_image_cache_size,
    cursor_blink_rate: u64::try_from(self.cursor_blink_rate.as_millis())
                    .unwrap_or(u64::MAX),
    cursor_blink_rate_ms: u64::try_from(self.cursor_blink_rate.as_millis())
                    .unwrap_or(u64::MAX),
    cursor_blink_ease_in: self.cursor_blink_ease_in,
    cursor_blink_ease_out: self.cursor_blink_ease_out,
    text_blink_rate: u64::try_from(self.text_blink_rate.as_millis()).unwrap_or(u64::MAX),
    text_blink_rate_ms: u64::try_from(self.text_blink_rate.as_millis()).unwrap_or(u64::MAX),
    text_blink_rate_rapid: u64::try_from(self.text_blink_rate_rapid.as_millis())
                    .unwrap_or(u64::MAX),
    text_blink_rate_rapid_ms: u64::try_from(self.text_blink_rate_rapid.as_millis())
                    .unwrap_or(u64::MAX),
    text_blink_ease_in: self.text_blink_ease_in,
    text_blink_ease_out: self.text_blink_ease_out,
    text_blink_rapid_ease_in: self.text_blink_rapid_ease_in,
    text_blink_rapid_ease_out: self.text_blink_rapid_ease_out,
    font: self.font.clone(),
    font_fallbacks: self.font_fallbacks.clone(),
    font_attributes: self.font_attributes.clone(),
    font_rules: self.font_rules.clone(),
    font_size: self.font_size,
    cell_width: self.cell_width,
    cell_widths: self.cell_widths.clone(),
    line_height: self.line_height,
    font_antialias: self.font_antialias,
    font_hinting: self.font_hinting,
    font_rasterizer: self.font_rasterizer,
    font_colr_rasterizer: self.font_colr_rasterizer,
    font_shaper: self.font_shaper,
    harfbuzz_features: self.harfbuzz_features.clone(),
    font_dirs: self.font_dirs.clone(),
    font_locator: self.font_locator,
    use_cap_height_to_scale_fallback_fonts: self.use_cap_height_to_scale_fallback_fonts,
    ignore_svg_fonts: self.ignore_svg_fonts,
    sort_fallback_fonts_by_coverage: self.sort_fallback_fonts_by_coverage,
    search_font_dirs_for_fallback: self.search_font_dirs_for_fallback,
    next: NativeConfigView1 {
        custom_block_glyphs: self.custom_block_glyphs,
        anti_alias_custom_block_glyphs: self.anti_alias_custom_block_glyphs,
        allow_square_glyphs_to_overflow_width: self.allow_square_glyphs_to_overflow_width,
        freetype_load_target: self.freetype_load_target,
        freetype_render_target: self.freetype_render_target,
        freetype_load_flags: self.effective_freetype_load_flags(),
        freetype_interpreter_version: self.freetype_interpreter_version,
        freetype_pcf_long_family_names: self.freetype_pcf_long_family_names,
        display_pixel_geometry: self.display_pixel_geometry,
        foreground_text_hsb: self.foreground_text_hsb,
        bold_brightens_ansi_colors: self.bold_brightens_ansi_colors,
        text_min_contrast_ratio: self.text_min_contrast_ratio,
        text_background_opacity: self.text_background_opacity,
        window_background_opacity: self.window_background_opacity,
        background: self.background.clone(),
        window_background_image: self.window_background_image.clone(),
        window_background_image_hsb: self.window_background_image_hsb,
        window_background_gradient: self.window_background_gradient.clone(),
        window_background_images: self.window_background_images.clone(),
        window_background_layers: self.window_background_layers.clone(),
        kde_window_background_blur: self.kde_window_background_blur,
        macos_window_background_blur: self.macos_window_background_blur,
        win32_system_backdrop: self.win32_system_backdrop,
        win32_acrylic_accent_color: self.win32_acrylic_accent_color,
        window_decorations: self.window_decorations,
        window_frame: self.window_frame_appearance.clone(),
        window_frame_appearance: self.window_frame_appearance.clone(),
        integrated_title_buttons: self.integrated_title_buttons.clone(),
        integrated_title_button_alignment: self.integrated_title_button_alignment,
        integrated_title_button_color: self.integrated_title_button_color,
        integrated_title_button_style: self.integrated_title_button_style,
        default_cursor_style: self.default_cursor_style,
        cursor_thickness: self.cursor_thickness,
        underline_thickness: self.underline_thickness,
        underline_position: self.underline_position,
        strikethrough_position: self.strikethrough_position,
        force_reverse_video_cursor: self.force_reverse_video_cursor,
        reverse_video_cursor_min_contrast: self.reverse_video_cursor_min_contrast,
        window_padding: self.window_padding,
        window_content_alignment: self.window_content_alignment,
        initial_cols: self.initial_cols,
        initial_rows: self.initial_rows,
        inactive_pane_hsb: self.inactive_pane_hsb,
        command_palette_rows: self.command_palette_rows,
        command_palette_font: self.effective_overlay_font(&self.command_palette_font),
        command_palette_font_size: self.command_palette_font_size,
        command_palette_bg_color: self.command_palette_bg_color,
        command_palette_fg_color: self.command_palette_fg_color,
        char_select_font: self.effective_overlay_font(&self.char_select_font),
        char_select_font_size: self.char_select_font_size,
        char_select_bg_color: self.char_select_bg_color,
        char_select_fg_color: self.char_select_fg_color,
        pane_select_font: self.effective_overlay_font(&self.pane_select_font),
        next: NativeConfigView2 {
            pane_select_font_size: self.pane_select_font_size,
            pane_select_bg_color: self.pane_select_bg_color,
            pane_select_fg_color: self.pane_select_fg_color,
            launcher_alphabet: self.launcher_alphabet.clone(),
            quick_select_alphabet: self.quick_select_alphabet.clone(),
            quick_select_patterns: self.quick_select_patterns.clone(),
            disable_default_quick_select_patterns: self.disable_default_quick_select_patterns,
            quick_select_remove_styling: self.quick_select_remove_styling,
            hyperlink_rules: self.hyperlink_rules.clone(),
            copy_mode_active_highlight_bg: self.copy_mode_active_highlight_bg,
            copy_mode_active_highlight_fg: self.copy_mode_active_highlight_fg,
            copy_mode_inactive_highlight_bg: self.copy_mode_inactive_highlight_bg,
            copy_mode_inactive_highlight_fg: self.copy_mode_inactive_highlight_fg,
            quick_select_label_bg: self.quick_select_label_bg,
            quick_select_label_fg: self.quick_select_label_fg,
            quick_select_match_bg: self.quick_select_match_bg,
            quick_select_match_fg: self.quick_select_match_fg,
            input_selector_label_bg: self.input_selector_label_bg,
            input_selector_label_fg: self.input_selector_label_fg,
            launcher_label_bg: self.launcher_label_bg,
            launcher_label_fg: self.launcher_label_fg,
            selection_word_boundary: self.selection_word_boundary.clone(),
            term: self.term.clone(),
            enq_answerback: self.enq_answerback.clone(),
            audible_bell: self.audible_bell,
            visual_bell: self.visual_bell,
            colors: self.colors.clone(),
            color_scheme: self.color_scheme.clone(),
            color_scheme_dirs: self.color_scheme_dirs.clone(),
            color_schemes: self.color_schemes.clone(),
            resolved_palette: self.native_resolved_palette(),
            foreground_color: self.foreground_color,
            background_color: self.background_color,
            ansi_palette: self.ansi_palette,
            indexed_palette: self.indexed_palette,
            selection_fg_color: self.selection_fg_color,
            selection_bg_color: self.selection_bg_color,
            cursor_bg_color: self.cursor_bg_color,
            cursor_border_color: self.cursor_border_color,
            cursor_fg_color: self.cursor_fg_color,
            compose_cursor_color: self.compose_cursor_color,
            split_color: self.split_color,
            scrollbar_thumb_color: self.scrollbar_thumb_color,
            tab_bar_background_color: self.tab_bar_background_color,
            tab_bar_inactive_tab_edge_color: self.tab_bar_inactive_tab_edge_color,
            tab_bar_active_tab_colors: self.tab_bar_active_tab_colors,
            tab_bar_inactive_tab_colors: self.tab_bar_inactive_tab_colors,
            tab_bar_inactive_tab_hover_colors: self.tab_bar_inactive_tab_hover_colors,
            tab_bar_new_tab_colors: self.tab_bar_new_tab_colors,
            tab_bar_new_tab_hover_colors: self.tab_bar_new_tab_hover_colors,
            tab_bar_style: self.tab_bar_style.clone(),
            visual_bell_color: self.visual_bell_color,
            notification_handling: self.notification_handling,
            next: NativeConfigView3 {
                default_prog: self.default_prog.clone(),
                default_gui_startup_args: self.default_gui_startup_args.clone(),
                default_domain: self.default_domain.clone(),
                default_workspace: self.default_workspace.clone(),
                prefer_to_spawn_tabs: self.prefer_to_spawn_tabs,
                automatically_reload_config: self.automatically_reload_config,
                check_for_updates: self.check_for_updates,
                check_for_updates_interval_seconds: self.check_for_updates_interval_seconds,
                show_update_window: self.show_update_window,
                native_macos_fullscreen_mode: self.native_macos_fullscreen_mode,
                macos_fullscreen_extend_behind_notch: self.macos_fullscreen_extend_behind_notch,
                use_resize_increments: self.use_resize_increments,
                debug_key_events: self.debug_key_events,
                log_unknown_escape_sequences: self.log_unknown_escape_sequences,
                warn_about_missing_glyphs: self.warn_about_missing_glyphs,
                default_cwd: self.default_cwd.clone(),
                default_ssh_auth_sock: self.default_ssh_auth_sock.clone(),
                default_mux_server_domain: self.default_mux_server_domain.clone(),
                daemon_options: self.daemon_options.clone(),
                exec_domains: self.exec_domains.clone(),
                wsl_domains: self.wsl_domains.clone(),
                unix_domains: self.unix_domains.clone(),
                ssh_domains: self.ssh_domains.clone(),
                tls_servers: self.tls_servers.clone(),
                tls_clients: self.tls_clients.clone(),
                serial_ports: self.serial_ports.clone(),
                mux_enable_ssh_agent: self.mux_enable_ssh_agent,
                ssh_backend: self.ssh_backend,
                ratelimit_mux_line_prefetches_per_second: self.ratelimit_mux_line_prefetches_per_second,
                mux_output_parser_buffer_size: self.mux_output_parser_buffer_size,
                mux_output_parser_coalesce_delay_ms: self.mux_output_parser_coalesce_delay_ms,
                periodic_stat_logging: self.periodic_stat_logging,
                ulimit_nofile: self.ulimit_nofile,
                ulimit_nproc: self.ulimit_nproc,
                mux_env_remove: self.mux_env_remove.clone(),
                tiling_desktop_environments: self.tiling_desktop_environments.clone(),
                set_environment_variables: self.set_environment_variables.clone(),
                launch_menu: self.launch_menu.clone(),
                leader: self.leader.clone(),
                keys: self.key_assignments.clone(),
                key_tables: self.key_tables.clone(),
                mouse_bindings: self.mouse_assignments.clone(),
                key_map_preference: self.key_map_preference,
                ui_key_cap_rendering: self.ui_key_cap_rendering,
                swap_backspace_and_delete: self.swap_backspace_and_delete,
                enable_kitty_graphics: self.enable_kitty_graphics,
                enable_checksum_rectangular_area: self.enable_checksum_rectangular_area,
                enable_title_reporting: self.enable_title_reporting,
                enable_csi_u_key_encoding: self.enable_csi_u_key_encoding,
                enable_kitty_keyboard: self.enable_kitty_keyboard,
                allow_download_protocols: self.allow_download_protocols,
                xcursor_theme: self.xcursor_theme.clone(),
                xcursor_size: self.xcursor_size,
                next: NativeConfigView4 {
                    palette_max_key_assigments_for_action: self.palette_max_key_assigments_for_action,
                    allow_win32_input_mode: self.allow_win32_input_mode,
                    treat_left_ctrlalt_as_altgr: self.treat_left_ctrlalt_as_altgr,
                    send_composed_key_when_left_alt_is_pressed: self
                                    .send_composed_key_when_left_alt_is_pressed,
                    send_composed_key_when_right_alt_is_pressed: self
                                    .send_composed_key_when_right_alt_is_pressed,
                    treat_east_asian_ambiguous_width_as_wide: self.treat_east_asian_ambiguous_width_as_wide,
                    normalize_output_to_unicode_nfc: self.normalize_output_to_unicode_nfc,
                    unicode_version: self.unicode_version,
                    bidi_enabled: self.bidi_enabled,
                    bidi_direction: self.bidi_direction,
                    use_ime: self.use_ime,
                    use_dead_keys: self.use_dead_keys,
                    ime_preedit_rendering: self.ime_preedit_rendering,
                    macos_forward_to_ime_modifier_mask: self.macos_forward_to_ime_modifier_mask,
                    xim_im_name: self.xim_im_name.clone(),
                    detect_password_input: self.detect_password_input,
                    scroll_to_bottom_on_input: self.scroll_to_bottom_on_input,
                    adjust_window_size_when_changing_font_size: self
                                    .adjust_window_size_when_changing_font_size,
                    canonicalize_pasted_newlines: self.canonicalize_pasted_newlines,
                    quote_dropped_files: self.quote_dropped_files,
                    disable_default_key_bindings: self.disable_default_key_bindings,
                    disable_default_mouse_bindings: self.disable_default_mouse_bindings,
                    hide_mouse_cursor_when_typing: self.hide_mouse_cursor_when_typing,
                    alternate_buffer_wheel_scroll_speed: self.alternate_buffer_wheel_scroll_speed,
                    pane_focus_follows_mouse: self.pane_focus_follows_mouse,
                    swallow_mouse_click_on_pane_focus: self.swallow_mouse_click_on_pane_focus,
                    swallow_mouse_click_on_window_focus: self.swallow_mouse_click_on_window_focus,
                    bypass_mouse_reporting_modifiers: self.bypass_mouse_reporting_modifiers,
                    enable_scroll_bar: self.enable_scroll_bar,
                    scrollback_lines: self.scrollback_lines,
                    min_scroll_bar_height: self.min_scroll_bar_height,
                    enable_tab_bar: self.enable_tab_bar,
                    hide_tab_bar_if_only_one_tab: self.hide_tab_bar_if_only_one_tab,
                    use_fancy_tab_bar: self.use_fancy_tab_bar,
                    unzoom_on_switch_pane: self.unzoom_on_switch_pane,
                    tab_bar_at_bottom: self.tab_bar_at_bottom,
                    tab_and_split_indices_are_zero_based: self.tab_and_split_indices_are_zero_based,
                    mouse_wheel_scrolls_tabs: self.mouse_wheel_scrolls_tabs,
                    switch_to_last_active_tab_when_closing_tab: self
                                    .switch_to_last_active_tab_when_closing_tab,
                    tab_shortcut_style: self.tab_shortcut_style,
                    closed_tab_history_size: self.closed_tab_history_size,
                    close_tab_selection: self.close_tab_selection,
                    tab_bar_wheel_behavior: self.tab_bar_wheel_behavior,
                    quit_when_all_windows_are_closed: self.quit_when_all_windows_are_closed,
                    window_close_confirmation: self.window_close_confirmation,
                    exit_behavior: self.exit_behavior,
                    clean_exit_codes: self.clean_exit_codes.clone(),
                    exit_behavior_messaging: self.exit_behavior_messaging,
                    skip_close_confirmation_for_processes_named: self
                                    .skip_close_confirmation_for_processes_named
                                    .clone(),
                    show_close_tab_button_in_tabs: self.show_close_tab_button_in_tabs,
                    show_new_tab_button_in_tab_bar: self.show_new_tab_button_in_tab_bar,
                    show_tab_index_in_tab_bar: self.show_tab_index_in_tab_bar,
                    show_tabs_in_tab_bar: self.show_tabs_in_tab_bar,
                },
            },
        },
    },
}
    }

    fn effective_freetype_load_flags(&self) -> NativeFreetypeLoadFlags {
        self.freetype_load_flags
            .unwrap_or_else(|| default_freetype_load_flags_for_dpi(self.window_dpi))
    }

    #[expect(
        clippy::ref_option,
        reason = "borrowed optional value matches surrounding compatibility helper interfaces"
    )]
    fn effective_overlay_font(&self, font: &Option<NativeFontConfig>) -> Option<NativeFontConfig> {
        font.clone().or_else(|| {
            self.window_frame_appearance
                .font
                .as_deref()
                .map(native_font_config)
        })
    }

    #[allow(dead_code)]
    fn get_config_overrides(&self) -> NativeConfigSnapshot {
        self.config_overrides.as_ref().clone()
    }

    #[cfg(test)]
    fn base_config_generation_for_test(&self) -> u64 {
        self.base_config_generation
    }

    #[allow(dead_code)]
    fn set_config_overrides(&mut self, overrides: NativeConfigSnapshot) {
        #[cfg(test)]
        let clear_implicit_integrated_buttons = overrides.integrated_title_buttons.is_none()
            && overrides != NativeConfigSnapshot::default();
        self.base_config_overrides = Arc::new(overrides.with_refreshed_effective());
        self.apply_effective_config(ReloadDisposition::ReloadAttempt);
        #[cfg(test)]
        if clear_implicit_integrated_buttons {
            self.integrated_title_buttons.clear();
        }
    }

    #[cfg(test)]
    fn apply_config_overrides_silently(&mut self, overrides: NativeConfigSnapshot) {
        self.base_config_overrides = Arc::new(overrides.with_refreshed_effective());
        self.apply_effective_config(ReloadDisposition::SilentStartup);
    }

    fn set_base_config(&mut self, config: &EffectiveNativeConfig, disposition: ReloadDisposition) {
        self.base_config_overrides = Arc::clone(&config.config);
        self.base_config_generation = config.generation;
        self.base_config_source.clone_from(&config.source);
        self.derived_config_environment = config.publication.variables().clone();
        self.apply_effective_config(disposition);
        #[cfg(test)]
        if let Some(observer) = self.base_config_apply_observer.as_mut() {
            observer(config.generation);
        }
    }

    fn set_window_config_overrides(
        &mut self,
        overrides: Option<NativeWindowConfigPatch>,
        disposition: ReloadDisposition,
    ) {
        self.window_config_overrides = overrides;
        self.apply_effective_config(disposition);
    }

    fn apply_effective_config(&mut self, disposition: ReloadDisposition) {
        let overrides = if let Some(window_overrides) = self.window_config_overrides.clone() {
            let mut overrides = self.base_config_overrides.as_ref().clone();
            window_overrides.apply_to_native_config_overrides(&mut overrides);
            overrides.refresh_effective_config();
            Arc::new(overrides)
        } else {
            Arc::clone(&self.base_config_overrides)
        };
        self.apply_config_overrides(&overrides, disposition);
    }

    fn apply_config_overrides(
        &mut self,
        overrides: &Arc<NativeConfigSnapshot>,
        disposition: ReloadDisposition,
    ) {
        let previous_palette = self.native_resolved_palette();
        let previous_terminal_line_palette = previous_palette.terminal_line_palette();
        self.apply_core_config_overrides(overrides);
        self.apply_selector_config_overrides(overrides);
        self.apply_palette_config_overrides(overrides);
        self.apply_launch_domain_config_overrides(overrides);
        self.apply_protocol_config_overrides(overrides);
        let palette = self.native_resolved_palette();
        if palette.terminal_line_palette() != previous_terminal_line_palette {
            self.runtime.mark_all_lines_changed();
            for runtime in self.pane_runtimes.values_mut() {
                runtime.runtime.mark_all_lines_changed();
                runtime.reconcile_terminal_mutation();
            }
        }
        if palette != previous_palette {
            self.refresh_snapshot();
        }
        self.apply_window_title();
        if disposition == ReloadDisposition::ReloadAttempt {
            self.reload_configuration();
        }
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn apply_core_config_overrides(&mut self, overrides: &Arc<NativeConfigSnapshot>) {
        let mut installed = overrides.as_ref().clone();
        let mut effective = Arc::unwrap_or_clone(installed.effective_config());
        effective.reuse_equal_subtrees_from(&self.config_overrides.effective);
        installed.effective = Arc::new(effective);
        self.config_overrides = Arc::new(installed);
        self.configured_dpi = overrides.dpi;
        self.dpi_by_screen = overrides.dpi_by_screen.clone().unwrap_or_default();
        self.apply_effective_window_dpi();
        self.tab_max_width = overrides.tab_max_width.unwrap_or(if self.modern_tab_bar_brand {
            MODERN_DEFAULT_TAB_MAX_WIDTH
        } else {
            DEFAULT_TAB_MAX_WIDTH
        });
        self.tab_min_width = overrides
            .tab_min_width
            .unwrap_or(DEFAULT_TAB_MIN_WIDTH)
            .clamp(1, self.tab_max_width);
        self.apply_status_update_interval_override(overrides.status_update_interval_ms);
        self.lua_tab_title.clone_from(&overrides.lua_tab_title);
        self.lua_window_title
            .clone_from(&overrides.lua_window_title);
        self.lua_update_status
            .clone_from(&overrides.lua_update_status);
        self.lua_update_status_config_overrides
            .clone_from(&overrides.lua_update_status_config_overrides);
        self.lua_bell.clone_from(&overrides.lua_bell);
        self.lua_focus_changed
            .clone_from(&overrides.lua_focus_changed);
        self.lua_resized.clone_from(&overrides.lua_resized);
        self.lua_config_reloaded
            .clone_from(&overrides.lua_config_reloaded);
        self.lua_user_var_changed
            .clone_from(&overrides.lua_user_var_changed);
        self.lua_open_uri.clone_from(&overrides.lua_open_uri);
        self.lua_new_tab_button_click = overrides.lua_new_tab_button_click;
        self.lua_command_palette_entries = overrides
            .lua_command_palette_entries
            .clone()
            .unwrap_or_default();
        self.lua_emit_event_handlers = overrides
            .lua_emit_event_handlers
            .clone()
            .unwrap_or_default();
        self.max_fps = overrides
            .max_fps
            .filter(|fps| *fps > 0)
            .unwrap_or(DEFAULT_MAX_FPS);
        self.animation_fps = overrides
            .animation_fps
            .filter(|fps| *fps > 0)
            .unwrap_or(DEFAULT_ANIMATION_FPS);
        self.last_redraw_request_at = None;
        self.last_animation_redraw_request_at = None;
        self.front_end = overrides.front_end.unwrap_or(DEFAULT_RENDER_FRONT_END);
        self.webgpu_power_preference = overrides
            .webgpu_power_preference
            .unwrap_or(DEFAULT_WEBGPU_POWER_PREFERENCE);
        self.webgpu_force_fallback_adapter = overrides
            .webgpu_force_fallback_adapter
            .unwrap_or(DEFAULT_WEBGPU_FORCE_FALLBACK_ADAPTER);
        self.webgpu_preferred_adapter
            .clone_from(&overrides.webgpu_preferred_adapter);
        self.prefer_egl = overrides.prefer_egl.unwrap_or(DEFAULT_PREFER_EGL);
        self.enable_wayland = overrides.enable_wayland.unwrap_or(DEFAULT_ENABLE_WAYLAND);
        self.enable_zwlr_output_manager = overrides
            .enable_zwlr_output_manager
            .unwrap_or(DEFAULT_ENABLE_ZWLR_OUTPUT_MANAGER);
        self.use_box_model_render = overrides
            .use_box_model_render
            .unwrap_or(DEFAULT_USE_BOX_MODEL_RENDER);
        self.experimental_pixel_positioning = overrides
            .experimental_pixel_positioning
            .unwrap_or(DEFAULT_EXPERIMENTAL_PIXEL_POSITIONING);
        self.shape_cache_size = overrides
            .shape_cache_size
            .unwrap_or(DEFAULT_SHAPE_CACHE_SIZE);
        self.line_state_cache_size = overrides
            .line_state_cache_size
            .unwrap_or(DEFAULT_LINE_STATE_CACHE_SIZE);
        self.line_quad_cache_size = overrides
            .line_quad_cache_size
            .unwrap_or(DEFAULT_LINE_QUAD_CACHE_SIZE);
        self.line_to_ele_shape_cache_size = overrides
            .line_to_ele_shape_cache_size
            .unwrap_or(DEFAULT_LINE_TO_ELE_SHAPE_CACHE_SIZE);
        self.glyph_cache_image_cache_size = overrides
            .glyph_cache_image_cache_size
            .unwrap_or(DEFAULT_GLYPH_CACHE_IMAGE_CACHE_SIZE);
        self.apply_cursor_blink_overrides(
            overrides.cursor_blink_rate_ms,
            overrides.cursor_blink_ease_in,
            overrides.cursor_blink_ease_out,
        );
        self.apply_text_blink_overrides(
            overrides.text_blink_rate_ms,
            overrides.text_blink_rate_rapid_ms,
            overrides.text_blink_ease_in,
            overrides.text_blink_ease_out,
            overrides.text_blink_rapid_ease_in,
            overrides.text_blink_rapid_ease_out,
        );
        self.font = overrides.font.clone().filter(|font| !font.is_empty());
        self.font_fallbacks = overrides.font_fallbacks.clone().unwrap_or_default();
        self.font_attributes = overrides.font_attributes.clone().unwrap_or_default();
        self.font_rules = overrides.font_rules.clone().unwrap_or_default();
        self.font_size = overrides.font_size.unwrap_or(if self.modern_tab_bar_brand {
            MODERN_DEFAULT_FONT_SIZE
        } else {
            DEFAULT_FONT_SIZE
        });
        self.cell_width = overrides.cell_width.unwrap_or(DEFAULT_CELL_WIDTH);
        self.cell_widths = overrides.cell_widths.clone().unwrap_or_default();
        self.line_height = overrides.line_height.unwrap_or(DEFAULT_LINE_HEIGHT);
        self.font_antialias = overrides.font_antialias.unwrap_or(DEFAULT_FONT_ANTIALIAS);
        self.font_hinting = overrides.font_hinting.unwrap_or(DEFAULT_FONT_HINTING);
        self.font_rasterizer = overrides.font_rasterizer.unwrap_or(DEFAULT_FONT_RASTERIZER);
        self.font_colr_rasterizer = overrides
            .font_colr_rasterizer
            .unwrap_or(DEFAULT_FONT_COLR_RASTERIZER);
        self.font_shaper = overrides.font_shaper.unwrap_or(DEFAULT_FONT_SHAPER);
        self.harfbuzz_features = overrides.harfbuzz_features.clone().unwrap_or_default();
        self.font_dirs = overrides.font_dirs.clone().unwrap_or_default();
        self.font_locator = overrides.font_locator.or(DEFAULT_FONT_LOCATOR);
        self.use_cap_height_to_scale_fallback_fonts = overrides
            .use_cap_height_to_scale_fallback_fonts
            .unwrap_or(DEFAULT_USE_CAP_HEIGHT_TO_SCALE_FALLBACK_FONTS);
        self.ignore_svg_fonts = overrides
            .ignore_svg_fonts
            .unwrap_or(DEFAULT_IGNORE_SVG_FONTS);
        self.sort_fallback_fonts_by_coverage = overrides
            .sort_fallback_fonts_by_coverage
            .unwrap_or(DEFAULT_SORT_FALLBACK_FONTS_BY_COVERAGE);
        self.search_font_dirs_for_fallback = overrides
            .search_font_dirs_for_fallback
            .unwrap_or(DEFAULT_SEARCH_FONT_DIRS_FOR_FALLBACK);
        self.custom_block_glyphs = overrides
            .custom_block_glyphs
            .unwrap_or(DEFAULT_CUSTOM_BLOCK_GLYPHS);
        self.anti_alias_custom_block_glyphs = overrides
            .anti_alias_custom_block_glyphs
            .unwrap_or(DEFAULT_ANTI_ALIAS_CUSTOM_BLOCK_GLYPHS);
        self.allow_square_glyphs_to_overflow_width = overrides
            .allow_square_glyphs_to_overflow_width
            .unwrap_or(DEFAULT_ALLOW_SQUARE_GLYPHS_TO_OVERFLOW_WIDTH);
        self.freetype_load_target = overrides
            .freetype_load_target
            .unwrap_or(DEFAULT_FREETYPE_LOAD_TARGET);
        self.freetype_render_target = overrides
            .freetype_render_target
            .unwrap_or(self.freetype_load_target);
        self.freetype_load_flags = overrides.freetype_load_flags;
        self.freetype_interpreter_version = overrides.freetype_interpreter_version;
        self.freetype_pcf_long_family_names = overrides
            .freetype_pcf_long_family_names
            .unwrap_or(DEFAULT_FREETYPE_PCF_LONG_FAMILY_NAMES);
        self.display_pixel_geometry = overrides
            .display_pixel_geometry
            .unwrap_or(DEFAULT_DISPLAY_PIXEL_GEOMETRY);
        self.foreground_text_hsb = overrides
            .foreground_text_hsb
            .unwrap_or(DEFAULT_FOREGROUND_TEXT_HSB);
        self.apply_bold_brightens_ansi_colors_override(overrides.bold_brightens_ansi_colors);
        self.text_min_contrast_ratio = overrides.text_min_contrast_ratio;
        self.text_background_opacity = overrides
            .text_background_opacity
            .unwrap_or(DEFAULT_TEXT_BACKGROUND_OPACITY);
        self.window_background_opacity = overrides
            .window_background_opacity
            .unwrap_or(DEFAULT_WINDOW_BACKGROUND_OPACITY);
        self.background = overrides.background.clone().unwrap_or_default();
        self.window_background_image
            .clone_from(&overrides.window_background_image);
        self.window_background_image_hsb = overrides.window_background_image_hsb;
        self.window_background_gradient
            .clone_from(&overrides.window_background_gradient);
        self.renderer.set_default_background_gradient(
            self.window_background_gradient
                .as_ref()
                .map(NativeWindowBackgroundGradient::to_render),
        );
        self.window_background_images = overrides
            .window_background_images
            .clone()
            .unwrap_or_default();
        self.renderer.set_default_background_images(
            self.window_background_images
                .iter()
                .map(NativeWindowBackgroundImage::to_render)
                .collect(),
        );
        self.window_background_layers = overrides
            .window_background_layers
            .clone()
            .unwrap_or_default();
        self.renderer.set_default_background_layers(
            self.window_background_layers
                .iter()
                .map(NativeWindowBackgroundVisualLayer::to_render)
                .collect(),
        );
        self.kde_window_background_blur = overrides
            .kde_window_background_blur
            .unwrap_or(DEFAULT_KDE_WINDOW_BACKGROUND_BLUR);
        self.macos_window_background_blur = overrides
            .macos_window_background_blur
            .unwrap_or(DEFAULT_MACOS_WINDOW_BACKGROUND_BLUR);
        self.win32_system_backdrop = overrides
            .win32_system_backdrop
            .unwrap_or(DEFAULT_WIN32_SYSTEM_BACKDROP);
        self.win32_acrylic_accent_color = overrides.win32_acrylic_accent_color;
        self.window_decorations = overrides
            .window_decorations
            .unwrap_or(DEFAULT_WINDOW_DECORATIONS);
        self.window_frame_appearance = overrides
            .window_frame_appearance
            .clone()
            .unwrap_or_default();
        self.integrated_title_buttons = overrides
            .integrated_title_buttons
            .clone()
            .unwrap_or_else(default_integrated_title_buttons);
        self.integrated_title_button_alignment = overrides
            .integrated_title_button_alignment
            .unwrap_or(DEFAULT_INTEGRATED_TITLE_BUTTON_ALIGNMENT);
        self.integrated_title_button_color = overrides
            .integrated_title_button_color
            .unwrap_or(DEFAULT_INTEGRATED_TITLE_BUTTON_COLOR);
        self.integrated_title_button_style = overrides
            .integrated_title_button_style
            .unwrap_or(DEFAULT_INTEGRATED_TITLE_BUTTON_STYLE);
        self.apply_default_cursor_style_override(overrides.default_cursor_style);
        self.apply_cursor_thickness_override(overrides.cursor_thickness);
        self.apply_underline_thickness_override(overrides.underline_thickness);
        self.apply_underline_position_override(overrides.underline_position);
        self.apply_strikethrough_position_override(overrides.strikethrough_position);
        self.apply_force_reverse_video_cursor_override(overrides.force_reverse_video_cursor);
        self.reverse_video_cursor_min_contrast = overrides
            .reverse_video_cursor_min_contrast
            .unwrap_or(DEFAULT_REVERSE_VIDEO_CURSOR_MIN_CONTRAST);
        self.renderer.set_reverse_video_cursor_min_contrast(Some(
            self.reverse_video_cursor_min_contrast.as_f64(),
        ));
        self.apply_window_padding_override(overrides.window_padding);
        self.window_content_alignment = overrides
            .window_content_alignment
            .unwrap_or(DEFAULT_WINDOW_CONTENT_ALIGNMENT);
        self.apply_tab_bar_config_overrides(overrides);
        self.apply_input_config_overrides(overrides);
    }

    fn apply_selector_config_overrides(&mut self, overrides: &Arc<NativeConfigSnapshot>) {
        self.initial_cols = overrides
            .initial_cols
            .filter(|columns| *columns > 0)
            .unwrap_or(DEFAULT_INITIAL_COLS);
        self.initial_rows = overrides
            .initial_rows
            .filter(|rows| *rows > 0)
            .unwrap_or(DEFAULT_INITIAL_ROWS);
        self.inactive_pane_hsb = overrides
            .inactive_pane_hsb
            .unwrap_or(DEFAULT_INACTIVE_PANE_HSB);
        self.command_palette_rows = overrides.command_palette_rows.filter(|rows| *rows > 0);
        self.command_palette_font
            .clone_from(&overrides.command_palette_font);
        self.command_palette_font_size = overrides
            .command_palette_font_size
            .unwrap_or(DEFAULT_COMMAND_PALETTE_FONT_SIZE);
        self.command_palette_bg_color = Some(
            overrides
                .command_palette_bg_color
                .unwrap_or(DEFAULT_COMMAND_PALETTE_BG_COLOR),
        );
        self.command_palette_fg_color = Some(
            overrides
                .command_palette_fg_color
                .unwrap_or(DEFAULT_COMMAND_PALETTE_FG_COLOR),
        );
        self.char_select_font
            .clone_from(&overrides.char_select_font);
        self.char_select_font_size = overrides
            .char_select_font_size
            .unwrap_or(DEFAULT_CHAR_SELECT_FONT_SIZE);
        self.char_select_bg_color = Some(
            overrides
                .char_select_bg_color
                .unwrap_or(DEFAULT_CHAR_SELECT_BG_COLOR),
        );
        self.char_select_fg_color = Some(
            overrides
                .char_select_fg_color
                .unwrap_or(DEFAULT_CHAR_SELECT_FG_COLOR),
        );
        self.pane_select_font
            .clone_from(&overrides.pane_select_font);
        self.pane_select_font_size = overrides
            .pane_select_font_size
            .unwrap_or(DEFAULT_PANE_SELECT_FONT_SIZE);
        self.pane_select_bg_color = Some(
            overrides
                .pane_select_bg_color
                .unwrap_or(DEFAULT_PANE_SELECT_BG_COLOR),
        );
        self.pane_select_fg_color = Some(
            overrides
                .pane_select_fg_color
                .unwrap_or(DEFAULT_PANE_SELECT_FG_COLOR),
        );
        self.launcher_alphabet = overrides
            .launcher_alphabet
            .clone()
            .filter(|alphabet| !alphabet.is_empty())
            .unwrap_or_else(|| DEFAULT_LAUNCHER_ALPHABET.to_owned());
        self.quick_select_alphabet = overrides
            .quick_select_alphabet
            .clone()
            .filter(|alphabet| !alphabet.is_empty())
            .unwrap_or_else(|| DEFAULT_QUICK_SELECT_ALPHABET.to_owned());
        self.quick_select_patterns = overrides
            .quick_select_patterns
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|pattern| !pattern.is_empty())
            .collect();
        self.disable_default_quick_select_patterns = overrides
            .disable_default_quick_select_patterns
            .unwrap_or(false);
        self.quick_select_remove_styling = overrides.quick_select_remove_styling.unwrap_or(false);
        self.hyperlink_rules = overrides
            .hyperlink_rules
            .clone()
            .unwrap_or_else(default_hyperlink_rules);
        self.copy_mode_active_highlight_bg = overrides.copy_mode_active_highlight_bg;
        self.copy_mode_active_highlight_fg = overrides.copy_mode_active_highlight_fg;
        self.copy_mode_inactive_highlight_bg = overrides.copy_mode_inactive_highlight_bg;
        self.copy_mode_inactive_highlight_fg = overrides.copy_mode_inactive_highlight_fg;
        self.quick_select_label_bg = overrides.quick_select_label_bg;
        self.quick_select_label_fg = overrides.quick_select_label_fg;
        self.quick_select_match_bg = overrides.quick_select_match_bg;
        self.quick_select_match_fg = overrides.quick_select_match_fg;
        self.input_selector_label_bg = overrides.input_selector_label_bg;
        self.input_selector_label_fg = overrides.input_selector_label_fg;
        self.launcher_label_bg = overrides.launcher_label_bg;
        self.launcher_label_fg = overrides.launcher_label_fg;
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn apply_palette_config_overrides(&mut self, overrides: &Arc<NativeConfigSnapshot>) {
        self.selection_word_boundary = overrides
            .selection_word_boundary
            .clone()
            .unwrap_or_else(|| DEFAULT_SELECTION_WORD_BOUNDARY.to_owned());
        self.term = overrides
            .term
            .clone()
            .unwrap_or_else(|| DEFAULT_TERM.to_owned());
        self.enq_answerback = overrides
            .enq_answerback
            .clone()
            .unwrap_or_else(|| DEFAULT_ENQ_ANSWERBACK.to_owned());
        self.apply_terminal_identity_config_to_runtimes();
        self.audible_bell = overrides.audible_bell.unwrap_or(DEFAULT_AUDIBLE_BELL);
        self.visual_bell = overrides.visual_bell.unwrap_or_default();
        self.colors.clone_from(&overrides.colors);
        self.color_scheme.clone_from(&overrides.color_scheme);
        self.color_scheme_dirs = overrides.color_scheme_dirs.clone().unwrap_or_default();
        self.color_schemes = overrides.color_schemes.clone().unwrap_or_default();
        #[cfg(test)]
        let default_foreground = if self.legacy_test_geometry {
            LEGACY_TEST_FOREGROUND_COLOR
        } else {
            DEFAULT_FOREGROUND_COLOR
        };
        #[cfg(not(test))]
        let default_foreground = DEFAULT_FOREGROUND_COLOR;
        #[cfg(test)]
        let default_background = if self.legacy_test_geometry {
            LEGACY_TEST_BACKGROUND_COLOR
        } else {
            DEFAULT_BACKGROUND_COLOR
        };
        #[cfg(not(test))]
        let default_background = DEFAULT_BACKGROUND_COLOR;
        self.foreground_color = overrides
            .foreground_color
            .unwrap_or(default_foreground);
        self.renderer.set_default_foreground(color_to_rgba(
            self.foreground_color,
            DEFAULT_RENDER_FOREGROUND_RGBA,
        ));
        self.background_color = overrides
            .background_color
            .unwrap_or(default_background);
        self.renderer.set_default_background(color_to_rgba(
            self.background_color,
            DEFAULT_RENDER_BACKGROUND_RGBA,
        ));
        self.ansi_palette = overrides.ansi_palette;
        self.renderer
            .set_ansi_palette(self.ansi_palette.map(native_ansi_palette_to_rgba));
        self.indexed_palette = overrides.indexed_palette;
        self.renderer
            .set_indexed_palette(self.indexed_palette.map(native_indexed_palette_to_rgba));
        self.selection_fg_color = overrides.selection_fg_color;
        let color_scheme_selected = overrides.color_scheme.is_some() || overrides.colors.is_some();
        #[cfg(test)]
        let default_selection_bg = if self.legacy_test_geometry {
            None
        } else {
            Some(DEFAULT_SELECTION_BG_COLOR)
        };
        #[cfg(not(test))]
        let default_selection_bg = Some(DEFAULT_SELECTION_BG_COLOR);
        self.selection_bg_color = overrides
            .selection_bg_color
            .or_else(|| (!color_scheme_selected).then_some(default_selection_bg).flatten());
        #[cfg(test)]
        let default_cursor_bg = if self.legacy_test_geometry {
            LEGACY_TEST_CURSOR_BG_COLOR
        } else if color_scheme_selected {
            LEGACY_COLOR_SCHEME_CURSOR_BG_COLOR
        } else {
            DEFAULT_CURSOR_BG_COLOR
        };
        #[cfg(not(test))]
        let default_cursor_bg = if color_scheme_selected {
            LEGACY_COLOR_SCHEME_CURSOR_BG_COLOR
        } else {
            DEFAULT_CURSOR_BG_COLOR
        };
        self.cursor_bg_color = overrides
            .cursor_bg_color
            .unwrap_or(default_cursor_bg);
        self.renderer.set_default_cursor_color(color_to_rgba(
            self.cursor_bg_color,
            DEFAULT_RENDER_FOREGROUND_RGBA,
        ));
        self.cursor_border_color = overrides.cursor_border_color;
        self.renderer.set_default_cursor_border(
            self.cursor_border_color
                .map(|color| color_to_rgba(color, DEFAULT_RENDER_FOREGROUND_RGBA)),
        );
        self.cursor_fg_color = overrides
            .cursor_fg_color
            .or_else(|| {
                #[cfg(test)]
                if self.legacy_test_geometry {
                    return LEGACY_TEST_CURSOR_FG_COLOR;
                }
                (!color_scheme_selected).then_some(DEFAULT_CURSOR_FG_COLOR)
            });
        self.renderer.set_default_cursor_foreground(
            self.cursor_fg_color
                .map(|color| color_to_rgba(color, DEFAULT_RENDER_FOREGROUND_RGBA)),
        );
        self.compose_cursor_color = overrides.compose_cursor_color;
        self.split_color = overrides.split_color;
        self.scrollbar_thumb_color = overrides.scrollbar_thumb_color;
        #[cfg(test)]
        let default_tab_bar_background = if self.legacy_test_geometry {
            None
        } else {
            Some(DEFAULT_TAB_BAR_BACKGROUND_COLOR)
        };
        #[cfg(not(test))]
        let default_tab_bar_background = Some(DEFAULT_TAB_BAR_BACKGROUND_COLOR);
        #[cfg(test)]
        let default_tab_active = if self.legacy_test_geometry {
            NativeTabBarItemColors::default()
        } else {
            DEFAULT_TAB_BAR_ACTIVE_TAB_COLORS
        };
        #[cfg(not(test))]
        let default_tab_active = DEFAULT_TAB_BAR_ACTIVE_TAB_COLORS;
        #[cfg(test)]
        let default_tab_inactive = if self.legacy_test_geometry {
            NativeTabBarItemColors::default()
        } else {
            DEFAULT_TAB_BAR_INACTIVE_TAB_COLORS
        };
        #[cfg(not(test))]
        let default_tab_inactive = DEFAULT_TAB_BAR_INACTIVE_TAB_COLORS;
        #[cfg(test)]
        let default_tab_hover = if self.legacy_test_geometry {
            NativeTabBarItemColors::default()
        } else {
            DEFAULT_TAB_BAR_INACTIVE_TAB_HOVER_COLORS
        };
        #[cfg(not(test))]
        let default_tab_hover = DEFAULT_TAB_BAR_INACTIVE_TAB_HOVER_COLORS;
        #[cfg(test)]
        let default_tab_new = if self.legacy_test_geometry {
            NativeTabBarItemColors::default()
        } else {
            DEFAULT_TAB_BAR_NEW_TAB_COLORS
        };
        #[cfg(not(test))]
        let default_tab_new = DEFAULT_TAB_BAR_NEW_TAB_COLORS;
        #[cfg(test)]
        let default_tab_new_hover = if self.legacy_test_geometry {
            NativeTabBarItemColors::default()
        } else {
            DEFAULT_TAB_BAR_NEW_TAB_HOVER_COLORS
        };
        #[cfg(not(test))]
        let default_tab_new_hover = DEFAULT_TAB_BAR_NEW_TAB_HOVER_COLORS;
        self.tab_bar_background_color = overrides
            .tab_bar_background_color
            .or(default_tab_bar_background);
        self.tab_bar_inactive_tab_edge_color = overrides.tab_bar_inactive_tab_edge_color;
        self.tab_bar_active_tab_colors = native_tab_bar_item_colors_with_overrides(
            default_tab_active,
            overrides.tab_bar_active_tab_colors,
        );
        self.tab_bar_inactive_tab_colors = native_tab_bar_item_colors_with_overrides(
            default_tab_inactive,
            overrides.tab_bar_inactive_tab_colors,
        );
        self.tab_bar_inactive_tab_hover_colors = native_tab_bar_item_colors_with_overrides(
            default_tab_hover,
            overrides.tab_bar_inactive_tab_hover_colors,
        );
        self.tab_bar_new_tab_colors = native_tab_bar_item_colors_with_overrides(
            default_tab_new,
            overrides.tab_bar_new_tab_colors,
        );
        self.tab_bar_new_tab_hover_colors = native_tab_bar_item_colors_with_overrides(
            default_tab_new_hover,
            overrides.tab_bar_new_tab_hover_colors,
        );
        self.tab_bar_style = overrides.tab_bar_style.clone();
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn apply_launch_domain_config_overrides(&mut self, overrides: &Arc<NativeConfigSnapshot>) {
        self.visual_bell_color = overrides.visual_bell_color;
        self.notification_handling = overrides
            .notification_handling
            .unwrap_or(DEFAULT_NOTIFICATION_HANDLING);
        self.default_prog = overrides
            .default_prog
            .clone()
            .filter(|prog| !prog.is_empty());
        self.default_gui_startup_args = overrides
            .default_gui_startup_args
            .clone()
            .filter(|args| !args.is_empty())
            .unwrap_or_else(default_gui_startup_args);
        self.default_domain = default_domain_from_override(overrides.default_domain.clone());
        self.default_workspace = overrides
            .default_workspace
            .clone()
            .filter(|workspace| !workspace.is_empty())
            .unwrap_or_else(|| DEFAULT_WORKSPACE_NAME.to_owned());
        self.prefer_to_spawn_tabs = overrides
            .prefer_to_spawn_tabs
            .unwrap_or(DEFAULT_PREFER_TO_SPAWN_TABS);
        self.automatically_reload_config = overrides
            .automatically_reload_config
            .unwrap_or(DEFAULT_AUTOMATICALLY_RELOAD_CONFIG);
        self.check_for_updates = overrides
            .check_for_updates
            .unwrap_or(DEFAULT_CHECK_FOR_UPDATES);
        self.check_for_updates_interval_seconds = overrides
            .check_for_updates_interval_seconds
            .filter(|seconds| *seconds > 0)
            .unwrap_or(DEFAULT_CHECK_FOR_UPDATES_INTERVAL_SECONDS);
        self.show_update_window = overrides
            .show_update_window
            .unwrap_or(DEFAULT_SHOW_UPDATE_WINDOW);
        self.native_macos_fullscreen_mode = overrides
            .native_macos_fullscreen_mode
            .unwrap_or(DEFAULT_NATIVE_MACOS_FULLSCREEN_MODE);
        self.macos_fullscreen_extend_behind_notch = overrides
            .macos_fullscreen_extend_behind_notch
            .unwrap_or(DEFAULT_MACOS_FULLSCREEN_EXTEND_BEHIND_NOTCH);
        self.use_resize_increments = overrides
            .use_resize_increments
            .unwrap_or(DEFAULT_USE_RESIZE_INCREMENTS);
        self.apply_window_resize_increments();
        self.debug_key_events = overrides
            .debug_key_events
            .unwrap_or(DEFAULT_DEBUG_KEY_EVENTS);
        self.log_unknown_escape_sequences = overrides
            .log_unknown_escape_sequences
            .unwrap_or(DEFAULT_LOG_UNKNOWN_ESCAPE_SEQUENCES);
        self.warn_about_missing_glyphs = overrides
            .warn_about_missing_glyphs
            .unwrap_or(DEFAULT_WARN_ABOUT_MISSING_GLYPHS);
        let previous_default_ssh_auth_sock = self.default_ssh_auth_sock.clone();
        self.default_cwd.clone_from(&overrides.default_cwd);
        self.default_ssh_auth_sock = overrides
            .default_ssh_auth_sock
            .clone()
            .filter(|ssh_auth_sock| !ssh_auth_sock.is_empty());
        self.default_mux_server_domain = overrides
            .default_mux_server_domain
            .clone()
            .filter(|default_mux_server_domain| !default_mux_server_domain.is_empty());
        self.daemon_options = overrides.daemon_options.clone().unwrap_or_default();
        self.exec_domains = overrides.exec_domains.clone().unwrap_or_default();
        self.wsl_domains = overrides.wsl_domains.clone().unwrap_or_default();
        self.unix_domains = overrides
            .unix_domains
            .clone()
            .unwrap_or_else(default_native_unix_domains);
        self.ssh_domains = overrides.ssh_domains.clone().unwrap_or_default();
        self.tls_servers = overrides.tls_servers.clone().unwrap_or_default();
        self.tls_clients = overrides.tls_clients.clone().unwrap_or_default();
        self.serial_ports = overrides.serial_ports.clone().unwrap_or_default();
        self.mux_enable_ssh_agent = overrides
            .mux_enable_ssh_agent
            .unwrap_or(DEFAULT_MUX_ENABLE_SSH_AGENT);
        self.ssh_backend = overrides.ssh_backend.unwrap_or(NativeSshBackend::LibSsh);
        self.ratelimit_mux_line_prefetches_per_second = overrides
            .ratelimit_mux_line_prefetches_per_second
            .unwrap_or(DEFAULT_RATELIMIT_MUX_LINE_PREFETCHES_PER_SECOND);
        self.mux_output_parser_buffer_size = overrides
            .mux_output_parser_buffer_size
            .unwrap_or(DEFAULT_MUX_OUTPUT_PARSER_BUFFER_SIZE);
        self.mux_output_parser_coalesce_delay_ms = overrides
            .mux_output_parser_coalesce_delay_ms
            .unwrap_or(DEFAULT_MUX_OUTPUT_PARSER_COALESCE_DELAY_MS);
        self.periodic_stat_logging = overrides
            .periodic_stat_logging
            .unwrap_or(DEFAULT_PERIODIC_STAT_LOGGING);
        self.ulimit_nofile = overrides.ulimit_nofile.unwrap_or(DEFAULT_ULIMIT_NOFILE);
        self.ulimit_nproc = overrides.ulimit_nproc.unwrap_or(DEFAULT_ULIMIT_NPROC);
        self.mux_env_remove = overrides
            .mux_env_remove
            .clone()
            .unwrap_or_else(default_mux_env_remove);
        self.tiling_desktop_environments = overrides
            .tiling_desktop_environments
            .clone()
            .filter(|environments| !environments.is_empty())
            .unwrap_or_else(default_tiling_desktop_environments);
        self.set_environment_variables = overrides
            .set_environment_variables
            .clone()
            .unwrap_or_default();
        self.apply_startup_default_workspace_before_spawn();
        self.apply_startup_default_prog_before_spawn();
        self.apply_startup_default_ssh_auth_sock_before_spawn(
            previous_default_ssh_auth_sock.as_deref(),
        );
        self.launch_menu = overrides.launch_menu.clone().unwrap_or_default();
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn apply_protocol_config_overrides(&mut self, overrides: &Arc<NativeConfigSnapshot>) {
        self.key_map_preference = overrides.key_map_preference.unwrap_or_default();
        self.ui_key_cap_rendering = overrides
            .ui_key_cap_rendering
            .unwrap_or(DEFAULT_UI_KEY_CAP_RENDERING);
        self.swap_backspace_and_delete = overrides.swap_backspace_and_delete.unwrap_or(false);
        self.enable_kitty_graphics = overrides
            .enable_kitty_graphics
            .unwrap_or(DEFAULT_ENABLE_KITTY_GRAPHICS);
        self.enable_checksum_rectangular_area = overrides
            .enable_checksum_rectangular_area
            .unwrap_or(DEFAULT_ENABLE_CHECKSUM_RECTANGULAR_AREA);
        self.enable_title_reporting = overrides
            .enable_title_reporting
            .unwrap_or(DEFAULT_ENABLE_TITLE_REPORTING);
        self.enable_csi_u_key_encoding = overrides
            .enable_csi_u_key_encoding
            .unwrap_or(DEFAULT_ENABLE_CSI_U_KEY_ENCODING);
        self.enable_kitty_keyboard = overrides
            .enable_kitty_keyboard
            .unwrap_or(DEFAULT_ENABLE_KITTY_KEYBOARD);
        self.allow_download_protocols = overrides
            .allow_download_protocols
            .unwrap_or(DEFAULT_ALLOW_DOWNLOAD_PROTOCOLS);
        self.xcursor_theme = overrides
            .xcursor_theme
            .clone()
            .filter(|xcursor_theme| !xcursor_theme.is_empty());
        self.xcursor_size = overrides.xcursor_size;
        self.palette_max_key_assigments_for_action = overrides
            .palette_max_key_assigments_for_action
            .unwrap_or(DEFAULT_PALETTE_MAX_KEY_ASSIGMENTS_FOR_ACTION);
        self.allow_win32_input_mode = overrides
            .allow_win32_input_mode
            .unwrap_or(DEFAULT_ALLOW_WIN32_INPUT_MODE);
        self.treat_left_ctrlalt_as_altgr = overrides
            .treat_left_ctrlalt_as_altgr
            .unwrap_or(DEFAULT_TREAT_LEFT_CTRLALT_AS_ALTGR);
        self.send_composed_key_when_left_alt_is_pressed = overrides
            .send_composed_key_when_left_alt_is_pressed
            .unwrap_or(DEFAULT_SEND_COMPOSED_KEY_WHEN_LEFT_ALT_IS_PRESSED);
        self.send_composed_key_when_right_alt_is_pressed = overrides
            .send_composed_key_when_right_alt_is_pressed
            .unwrap_or(DEFAULT_SEND_COMPOSED_KEY_WHEN_RIGHT_ALT_IS_PRESSED);
        self.treat_east_asian_ambiguous_width_as_wide = overrides
            .treat_east_asian_ambiguous_width_as_wide
            .unwrap_or(DEFAULT_TREAT_EAST_ASIAN_AMBIGUOUS_WIDTH_AS_WIDE);
        self.normalize_output_to_unicode_nfc = overrides
            .normalize_output_to_unicode_nfc
            .unwrap_or(DEFAULT_NORMALIZE_OUTPUT_TO_UNICODE_NFC);
        self.unicode_version = overrides.unicode_version.unwrap_or(DEFAULT_UNICODE_VERSION);
        self.bidi_enabled = overrides.bidi_enabled.unwrap_or(DEFAULT_BIDI_ENABLED);
        self.bidi_direction = overrides.bidi_direction.unwrap_or(DEFAULT_BIDI_DIRECTION);
        self.use_ime = overrides.use_ime.unwrap_or(DEFAULT_USE_IME);
        self.use_dead_keys = overrides.use_dead_keys.unwrap_or(DEFAULT_USE_DEAD_KEYS);
        if !self.use_dead_keys {
            self.dead_key_active = false;
            self.dead_key_text = None;
        }
        self.ime_preedit_rendering = overrides
            .ime_preedit_rendering
            .unwrap_or(DEFAULT_IME_PREEDIT_RENDERING);
        self.macos_forward_to_ime_modifier_mask = overrides
            .macos_forward_to_ime_modifier_mask
            .unwrap_or(DEFAULT_MACOS_FORWARD_TO_IME_MODIFIER_MASK);
        if let Some(window) = self.window.as_ref() {
            window.set_ime_allowed(self.use_ime);
        }
        self.last_ime_cursor_area.set(None);
        self.update_ime_cursor_area();
        if !self.use_ime || self.ime_preedit_rendering != NativeImePreeditRendering::Builtin {
            self.ime_preedit = None;
        }
        self.xim_im_name = overrides
            .xim_im_name
            .clone()
            .filter(|xim_im_name| !xim_im_name.is_empty());
        self.detect_password_input = overrides
            .detect_password_input
            .unwrap_or(DEFAULT_DETECT_PASSWORD_INPUT);
        self.apply_keyboard_protocol_config_to_runtimes();
        self.apply_character_width_config_to_runtimes();
        self.apply_unicode_normalization_config_to_runtimes();
        self.apply_unicode_version_config_to_runtimes();
        self.leader = overrides
            .leader
            .clone()
            .filter(|leader| !leader.keys.is_empty());
        self.adjust_window_size_when_changing_font_size = overrides
            .adjust_window_size_when_changing_font_size
            .unwrap_or(DEFAULT_ADJUST_WINDOW_SIZE_WHEN_CHANGING_FONT_SIZE);
        self.key_assignments = overrides
            .key_assignments
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|assignment| !assignment.keys.is_empty())
            .collect();
        self.key_tables = overrides
            .key_tables
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(name, assignments)| {
                let name = name.trim();
                if name.is_empty() {
                    return None;
                }
                let assignments = assignments
                    .into_iter()
                    .filter(|assignment| !assignment.keys.is_empty())
                    .collect::<Vec<_>>();
                (!assignments.is_empty()).then(|| (name.to_owned(), assignments))
            })
            .collect();
        self.mouse_assignments = overrides.mouse_assignments.clone().unwrap_or_default();
        self.enable_scroll_bar = overrides
            .enable_scroll_bar
            .unwrap_or(DEFAULT_ENABLE_SCROLL_BAR);
        let scrollback_lines = overrides
            .scrollback_lines
            .unwrap_or(DEFAULT_SCROLLBACK_LIMIT);
        if self.scrollback_lines != scrollback_lines {
            self.scrollback_lines = scrollback_lines;
            self.apply_scrollback_limit_to_runtimes();
        }
        self.min_scroll_bar_height = overrides
            .min_scroll_bar_height
            .or(DEFAULT_MIN_SCROLL_BAR_HEIGHT);
    }

    fn apply_input_config_overrides(&mut self, overrides: &NativeConfigSnapshot) {
        self.scroll_to_bottom_on_input = overrides
            .scroll_to_bottom_on_input
            .unwrap_or(DEFAULT_SCROLL_TO_BOTTOM_ON_INPUT);
        self.canonicalize_pasted_newlines = overrides
            .canonicalize_pasted_newlines
            .unwrap_or(DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        self.quote_dropped_files = overrides
            .quote_dropped_files
            .unwrap_or(DEFAULT_QUOTE_DROPPED_FILES);
        self.disable_default_key_bindings = overrides
            .disable_default_key_bindings
            .unwrap_or(DEFAULT_DISABLE_DEFAULT_KEY_BINDINGS);
        self.disable_default_mouse_bindings = overrides
            .disable_default_mouse_bindings
            .unwrap_or(DEFAULT_DISABLE_DEFAULT_MOUSE_BINDINGS);
        self.hide_mouse_cursor_when_typing = overrides
            .hide_mouse_cursor_when_typing
            .unwrap_or(DEFAULT_HIDE_MOUSE_CURSOR_WHEN_TYPING);
        if !self.hide_mouse_cursor_when_typing {
            self.set_mouse_cursor_visible(true);
        }
        self.alternate_buffer_wheel_scroll_speed = overrides
            .alternate_buffer_wheel_scroll_speed
            .unwrap_or(DEFAULT_ALTERNATE_BUFFER_WHEEL_SCROLL_SPEED);
        self.pane_focus_follows_mouse = overrides
            .pane_focus_follows_mouse
            .unwrap_or(DEFAULT_PANE_FOCUS_FOLLOWS_MOUSE);
        self.swallow_mouse_click_on_pane_focus = overrides
            .swallow_mouse_click_on_pane_focus
            .unwrap_or(DEFAULT_SWALLOW_MOUSE_CLICK_ON_PANE_FOCUS);
        self.swallow_mouse_click_on_window_focus = overrides
            .swallow_mouse_click_on_window_focus
            .unwrap_or(DEFAULT_SWALLOW_MOUSE_CLICK_ON_WINDOW_FOCUS);
        self.bypass_mouse_reporting_modifiers = overrides
            .bypass_mouse_reporting_modifiers
            .unwrap_or(DEFAULT_BYPASS_MOUSE_REPORTING_MODIFIERS);
    }

    fn apply_keyboard_protocol_config_to_runtimes(&mut self) {
        let config = Arc::clone(&self.applied_config);
        self.runtime
            .set_enable_kitty_graphics(config.enable_kitty_graphics);
        self.runtime
            .set_enable_checksum_rectangular_area(config.enable_checksum_rectangular_area);
        self.runtime
            .set_enable_title_reporting(config.enable_title_reporting);
        self.runtime
            .set_enable_kitty_keyboard(config.enable_kitty_keyboard);
        self.runtime
            .set_allow_win32_input_mode(config.allow_win32_input_mode);
        for runtime in self.pane_runtimes.values_mut() {
            runtime
                .runtime
                .set_enable_kitty_graphics(config.enable_kitty_graphics);
            runtime
                .runtime
                .set_enable_checksum_rectangular_area(config.enable_checksum_rectangular_area);
            runtime
                .runtime
                .set_enable_title_reporting(config.enable_title_reporting);
            runtime
                .runtime
                .set_enable_kitty_keyboard(config.enable_kitty_keyboard);
            runtime
                .runtime
                .set_allow_win32_input_mode(config.allow_win32_input_mode);
        }
    }

    fn apply_character_width_config_to_runtimes(&mut self) {
        let cell_width_overrides = self.terminal_cell_width_overrides();
        let ambiguous_width_is_wide = self.treat_east_asian_ambiguous_width_as_wide;
        self.runtime.set_treat_east_asian_ambiguous_width_as_wide(
            ambiguous_width_is_wide,
        );
        self.runtime
            .set_cell_width_overrides(cell_width_overrides.clone());
        for runtime in self.pane_runtimes.values_mut() {
            runtime
                .runtime
                .set_treat_east_asian_ambiguous_width_as_wide(
                    ambiguous_width_is_wide,
                );
            runtime
                .runtime
                .set_cell_width_overrides(cell_width_overrides.clone());
        }
    }

    fn apply_unicode_normalization_config_to_runtimes(&mut self) {
        let normalize = self.normalize_output_to_unicode_nfc;
        self.runtime
            .set_normalize_output_to_unicode_nfc(normalize);
        for runtime in self.pane_runtimes.values_mut() {
            runtime
                .runtime
                .set_normalize_output_to_unicode_nfc(normalize);
        }
    }

    fn apply_unicode_version_config_to_runtimes(&mut self) {
        let unicode_version = self.unicode_version;
        self.runtime.set_unicode_version(unicode_version);
        for runtime in self.pane_runtimes.values_mut() {
            runtime.runtime.set_unicode_version(unicode_version);
        }
    }

    fn terminal_cell_width_overrides(&self) -> Vec<CellWidthOverride> {
        self.cell_widths
            .iter()
            .copied()
            .map(NativeCellWidthOverride::to_terminal)
            .collect()
    }

    fn apply_terminal_identity_config_to_runtimes(&mut self) {
        let term = self.term.clone();
        let enq_answerback = self.enq_answerback.clone();
        self.runtime.set_terminal_name(term.clone());
        self.runtime.set_enq_answerback(enq_answerback.clone());
        for runtime in self.pane_runtimes.values_mut() {
            runtime.runtime.set_terminal_name(term.clone());
            runtime
                .runtime
                .set_enq_answerback(enq_answerback.clone());
        }
    }

    fn apply_tab_bar_config_overrides(&mut self, overrides: &NativeConfigSnapshot) {
        self.enable_tab_bar = overrides.enable_tab_bar.unwrap_or(DEFAULT_ENABLE_TAB_BAR);
        self.hide_tab_bar_if_only_one_tab = overrides
            .hide_tab_bar_if_only_one_tab
            .unwrap_or(DEFAULT_HIDE_TAB_BAR_IF_ONLY_ONE_TAB);
        self.use_fancy_tab_bar = overrides
            .use_fancy_tab_bar
            .unwrap_or(DEFAULT_USE_FANCY_TAB_BAR);
        self.unzoom_on_switch_pane = overrides
            .unzoom_on_switch_pane
            .unwrap_or(DEFAULT_UNZOOM_ON_SWITCH_PANE);
        self.tab_bar_at_bottom = overrides
            .tab_bar_at_bottom
            .unwrap_or(DEFAULT_TAB_BAR_AT_BOTTOM);
        self.tab_and_split_indices_are_zero_based = overrides
            .tab_and_split_indices_are_zero_based
            .unwrap_or(DEFAULT_TAB_AND_SPLIT_INDICES_ARE_ZERO_BASED);
        self.mouse_wheel_scrolls_tabs = overrides
            .mouse_wheel_scrolls_tabs
            .unwrap_or(DEFAULT_MOUSE_WHEEL_SCROLLS_TABS);
        self.switch_to_last_active_tab_when_closing_tab = overrides
            .switch_to_last_active_tab_when_closing_tab
            .unwrap_or(DEFAULT_SWITCH_TO_LAST_ACTIVE_TAB_WHEN_CLOSING_TAB);
        self.tab_shortcut_style = overrides
            .tab_shortcut_style
            .unwrap_or(NativeTabShortcutStyle::Terminal);
        self.closed_tab_history_size = overrides
            .closed_tab_history_size
            .unwrap_or(DEFAULT_CLOSED_TAB_HISTORY_SIZE);
        self.close_tab_selection = overrides.close_tab_selection.unwrap_or_else(|| {
            match overrides.switch_to_last_active_tab_when_closing_tab {
                Some(true) => CloseTabSelection::LastActive,
                Some(false) => CloseTabSelection::Left,
                None => CloseTabSelection::Adjacent,
            }
        });
        self.tab_bar_wheel_behavior = overrides.tab_bar_wheel_behavior.unwrap_or_else(|| {
            match overrides.mouse_wheel_scrolls_tabs {
                Some(true) => NativeTabBarWheelBehavior::Switch,
                Some(false) => NativeTabBarWheelBehavior::Disabled,
                None => NativeTabBarWheelBehavior::Scroll,
            }
        });
        if let Ok(mut history) = self.closed_tab_history.lock() {
            history.set_capacity(self.closed_tab_history_size);
        }
        self.quit_when_all_windows_are_closed = overrides
            .quit_when_all_windows_are_closed
            .unwrap_or(DEFAULT_QUIT_WHEN_ALL_WINDOWS_ARE_CLOSED);
        self.window_close_confirmation = overrides
            .window_close_confirmation
            .unwrap_or(DEFAULT_WINDOW_CLOSE_CONFIRMATION);
        self.exit_behavior = overrides.exit_behavior.unwrap_or(DEFAULT_EXIT_BEHAVIOR);
        self.clean_exit_codes = overrides
            .clean_exit_codes
            .clone()
            .unwrap_or_else(|| DEFAULT_CLEAN_EXIT_CODES.to_vec());
        self.exit_behavior_messaging = overrides
            .exit_behavior_messaging
            .unwrap_or(DEFAULT_EXIT_BEHAVIOR_MESSAGING);
        self.skip_close_confirmation_for_processes_named = overrides
            .skip_close_confirmation_for_processes_named
            .clone()
            .map_or_else(
                default_skip_close_confirmation_for_processes_named,
                |processes| {
                    processes
                        .into_iter()
                        .map(|process| process.trim().to_owned())
                        .filter(|process| !process.is_empty())
                        .collect()
                },
            );
        self.show_close_tab_button_in_tabs = overrides
            .show_close_tab_button_in_tabs
            .unwrap_or(DEFAULT_SHOW_CLOSE_TAB_BUTTON_IN_TABS);
        self.show_new_tab_button_in_tab_bar = overrides
            .show_new_tab_button_in_tab_bar
            .unwrap_or(DEFAULT_SHOW_NEW_TAB_BUTTON_IN_TAB_BAR);
        self.show_tab_index_in_tab_bar = overrides
            .show_tab_index_in_tab_bar
            .unwrap_or(DEFAULT_SHOW_TAB_INDEX_IN_TAB_BAR);
        self.show_tabs_in_tab_bar = overrides
            .show_tabs_in_tab_bar
            .unwrap_or(DEFAULT_SHOW_TABS_IN_TAB_BAR);
    }

    fn apply_cursor_blink_rate_override(&mut self, cursor_blink_rate_ms: Option<u64>) {
        self.cursor_blink_rate = Duration::from_millis(cursor_blink_rate_ms.unwrap_or_else(|| {
            u64::try_from(DEFAULT_CURSOR_BLINK_RATE.as_millis()).unwrap_or(u64::MAX)
        }));
        self.last_cursor_blink_at = None;
        if self.cursor_blink_rate.is_zero() {
            self.apply_cursor_blink_opacity(u8::MAX);
        }
    }

    fn apply_status_update_interval_override(&mut self, status_update_interval_ms: Option<u64>) {
        self.status_update_interval =
            Duration::from_millis(status_update_interval_ms.unwrap_or_else(|| {
                u64::try_from(DEFAULT_STATUS_UPDATE_INTERVAL.as_millis()).unwrap_or(u64::MAX)
            }));
    }

    fn apply_cursor_blink_overrides(
        &mut self,
        cursor_blink_rate_ms: Option<u64>,
        cursor_blink_ease_in: Option<NativeEasingFunction>,
        cursor_blink_ease_out: Option<NativeEasingFunction>,
    ) {
        self.apply_cursor_blink_rate_override(cursor_blink_rate_ms);
        self.apply_cursor_blink_easing_overrides(cursor_blink_ease_in, cursor_blink_ease_out);
    }

    fn apply_cursor_blink_easing_overrides(
        &mut self,
        cursor_blink_ease_in: Option<NativeEasingFunction>,
        cursor_blink_ease_out: Option<NativeEasingFunction>,
    ) {
        self.cursor_blink_ease_in = cursor_blink_ease_in.unwrap_or(DEFAULT_CURSOR_BLINK_EASE_IN);
        self.cursor_blink_ease_out = cursor_blink_ease_out.unwrap_or(DEFAULT_CURSOR_BLINK_EASE_OUT);
        self.last_cursor_blink_at = None;
        self.apply_cursor_blink_opacity(u8::MAX);
    }

    fn apply_text_blink_overrides(
        &mut self,
        text_blink_rate_ms: Option<u64>,
        text_blink_rate_rapid_ms: Option<u64>,
        text_blink_ease_in: Option<NativeEasingFunction>,
        text_blink_ease_out: Option<NativeEasingFunction>,
        text_blink_rapid_ease_in: Option<NativeEasingFunction>,
        text_blink_rapid_ease_out: Option<NativeEasingFunction>,
    ) {
        self.text_blink_rate = Duration::from_millis(text_blink_rate_ms.unwrap_or_else(|| {
            u64::try_from(DEFAULT_TEXT_BLINK_RATE.as_millis()).unwrap_or(u64::MAX)
        }));
        self.text_blink_rate_rapid =
            Duration::from_millis(text_blink_rate_rapid_ms.unwrap_or_else(|| {
                u64::try_from(DEFAULT_TEXT_BLINK_RATE_RAPID.as_millis()).unwrap_or(u64::MAX)
            }));
        self.text_blink_ease_in = text_blink_ease_in.unwrap_or(DEFAULT_TEXT_BLINK_EASE_IN);
        self.text_blink_ease_out = text_blink_ease_out.unwrap_or(DEFAULT_TEXT_BLINK_EASE_OUT);
        self.text_blink_rapid_ease_in =
            text_blink_rapid_ease_in.unwrap_or(DEFAULT_TEXT_BLINK_RAPID_EASE_IN);
        self.text_blink_rapid_ease_out =
            text_blink_rapid_ease_out.unwrap_or(DEFAULT_TEXT_BLINK_RAPID_EASE_OUT);
        self.last_text_blink_at = None;
        self.last_rapid_text_blink_at = None;
        self.apply_text_blink_opacity(u8::MAX);
        self.apply_rapid_text_blink_opacity(u8::MAX);
    }

    fn apply_default_cursor_style_override(
        &mut self,
        default_cursor_style: Option<NativeCursorStyle>,
    ) {
        self.default_cursor_style = default_cursor_style.unwrap_or(DEFAULT_CURSOR_STYLE);
        self.apply_default_cursor_style_to_runtimes();
    }

    fn apply_default_cursor_style_to_runtimes(&mut self) {
        let cursor_style = CursorStyle::from(self.default_cursor_style);
        self.runtime.set_default_cursor_style(cursor_style);
        self.snapshot
            .update_cursor_from_terminal(self.runtime.terminal(), self.current_scrollback_offset());
        for pane_runtime in self.pane_runtimes.values_mut() {
            pane_runtime.runtime.set_default_cursor_style(cursor_style);
            pane_runtime.snapshot.update_cursor_from_terminal(
                pane_runtime.runtime.terminal(),
                pane_runtime
                    .ui
                    .stable_viewport
                    .scrollback_offset(pane_runtime.runtime.terminal()),
            );
        }
        self.frame_needs_full_repaint = true;
    }

    fn apply_bold_brightens_ansi_colors_override(
        &mut self,
        bold_brightens_ansi_colors: Option<NativeBoldBrightensAnsiColors>,
    ) {
        self.bold_brightens_ansi_colors =
            bold_brightens_ansi_colors.unwrap_or(DEFAULT_BOLD_BRIGHTENS_ANSI_COLORS);
        self.renderer
            .set_bold_brightens_ansi_colors(self.bold_brightens_ansi_colors.into());
        self.frame_needs_full_repaint = true;
    }

    fn apply_cursor_thickness_override(&mut self, cursor_thickness: Option<NativeCursorThickness>) {
        self.cursor_thickness = cursor_thickness.or(DEFAULT_CURSOR_THICKNESS);
        self.renderer
            .set_cursor_thickness(self.cursor_thickness.map(RenderCursorThickness::from));
        self.frame_needs_full_repaint = true;
    }

    fn apply_underline_thickness_override(
        &mut self,
        underline_thickness: Option<NativeUnderlineThickness>,
    ) {
        self.underline_thickness = underline_thickness.or(DEFAULT_UNDERLINE_THICKNESS);
        self.renderer
            .set_underline_thickness(self.underline_thickness.map(RenderUnderlineThickness::from));
        self.frame_needs_full_repaint = true;
    }

    fn apply_underline_position_override(
        &mut self,
        underline_position: Option<NativeUnderlinePosition>,
    ) {
        self.underline_position = underline_position.or(DEFAULT_UNDERLINE_POSITION);
        self.renderer
            .set_underline_position(self.underline_position.map(RenderUnderlinePosition::from));
        self.frame_needs_full_repaint = true;
    }

    fn apply_strikethrough_position_override(
        &mut self,
        strikethrough_position: Option<NativeStrikethroughPosition>,
    ) {
        self.strikethrough_position = strikethrough_position.or(DEFAULT_STRIKETHROUGH_POSITION);
        self.renderer.set_strikethrough_position(
            self.strikethrough_position
                .map(RenderStrikethroughPosition::from),
        );
        self.frame_needs_full_repaint = true;
    }

    fn apply_force_reverse_video_cursor_override(
        &mut self,
        force_reverse_video_cursor: Option<bool>,
    ) {
        self.force_reverse_video_cursor =
            force_reverse_video_cursor.unwrap_or(DEFAULT_FORCE_REVERSE_VIDEO_CURSOR);
        self.renderer
            .set_force_reverse_video_cursor(self.force_reverse_video_cursor);
        self.frame_needs_full_repaint = true;
    }

    fn apply_window_padding_override(&mut self, window_padding: Option<NativeWindowPadding>) {
        #[cfg(test)]
        if self.legacy_test_geometry && window_padding.is_none() {
            self.window_padding = NativeWindowPadding::default();
            self.frame_needs_full_repaint = true;
            return;
        }
        self.window_padding = window_padding.unwrap_or(if self.modern_tab_bar_brand {
            MODERN_DEFAULT_WINDOW_PADDING
        } else {
            DEFAULT_WINDOW_PADDING
        });
        self.frame_needs_full_repaint = true;
    }

    fn apply_scrollback_limit_to_runtimes(&mut self) {
        let scrollback_lines = self.scrollback_lines;
        self.runtime.set_scrollback_limit(scrollback_lines);

        for pane_runtime in self.pane_runtimes.values_mut() {
            pane_runtime
                .runtime
                .set_scrollback_limit(scrollback_lines);
            pane_runtime.reconcile_terminal_mutation();
        }

        self.reconcile_active_terminal_mutation();
        self.refresh_snapshot();
    }

    fn native_tab_information(
        &self,
        position: usize,
        tab: &rssh_core::app_shell::Tab,
        tab_title: Option<String>,
    ) -> NativeTabInformation {
        let panes = self.native_pane_information_for_tab(tab);
        let active_pane = panes
            .iter()
            .find(|pane| pane.pane_id == tab.active_pane_id())
            .cloned()
            .unwrap_or_else(|| {
                panes
                    .first()
                    .cloned()
                    .expect("tab information requires at least one pane")
            });

        NativeTabInformation {
            tab_id: tab.id(),
            tab_index: position,
            is_active: tab.id() == self.app_shell.active_tab_id(),
            is_last_active: Some(tab.id()) == self.app_shell.last_active_tab_id(),
            active_pane,
            panes,
            window_id: self.app_window_id,
            window_title: self.window_title.clone(),
            tab_title,
        }
    }

    fn native_window_tab_information(&self) -> Vec<NativeTabInformation> {
        self.app_shell
            .active_workspace()
            .tabs()
            .iter()
            .enumerate()
            .map(|(position, tab)| {
                self.native_tab_information(position, tab, tab.title().map(str::to_owned))
            })
            .collect()
    }

    fn native_pane_information_for_tab(
        &self,
        tab: &rssh_core::app_shell::Tab,
    ) -> Vec<NativePaneInformation> {
        let layout = self.pane_render_layout_for_tab(tab);
        tab.panes()
            .iter()
            .enumerate()
            .map(|(pane_index, pane)| {
                let rect = layout
                    .panes
                    .iter()
                    .find(|rect| rect.pane_id == pane.id())
                    .copied()
                    .unwrap_or(PaneRenderRect {
                        pane_id: pane.id(),
                        row: self.terminal_frame_row_offset(),
                        column: 0,
                        rows: 0,
                        columns: 0,
                    });
                NativePaneInformation {
                    pane_id: pane.id(),
                    pane_index,
                    is_active: pane.id() == tab.active_pane_id(),
                    is_zoomed: tab.zoomed_pane_id() == Some(pane.id()),
                    left: rect.column,
                    top: rect.row,
                    width: rect.columns,
                    height: rect.rows,
                    pixel_width: u32::from(rect.columns) * self.cell_width(),
                    pixel_height: u32::from(rect.rows) * self.cell_height(),
                    title: self.pane_title(pane.id()),
                    foreground_process_name: pane_launch_display_program(pane.launch()).to_owned(),
                    current_working_dir: pane.launch().cwd().map(str::to_owned),
                    has_unseen_output: pane.has_unseen_output(),
                    domain_name: pane_launch_domain_name(pane.launch()).to_owned(),
                    tty_name: self.pane_tty_name(pane.id()).map(str::to_owned),
                    user_vars: pane.user_vars().clone(),
                    progress: pane.progress(),
                }
            })
            .collect()
    }

    fn tab_title_for_tab(&self, tab: &rssh_core::app_shell::Tab) -> Option<String> {
        if let Some(title) = tab.title().map(str::trim).filter(|title| !title.is_empty()) {
            return Some(title.to_owned());
        }

        self.pane_title(tab.active_pane_id())
    }

    fn pane_title(&self, pane: rssh_core::PaneId) -> Option<String> {
        let title = if pane == self.app_shell.active_pane_id() {
            self.runtime.terminal().title()
        } else {
            self.pane_runtimes
                .get(&pane)
                .and_then(|runtime| runtime.runtime.terminal().title())
        };

        title
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(str::to_owned)
    }

    fn pane_inspection_lines(&self, pane_id: rssh_core::PaneId) -> Option<Vec<String>> {
        let workspace = self.app_shell.active_workspace();
        let workspace_index = self
            .app_shell
            .workspaces()
            .iter()
            .position(|candidate| candidate.id() == workspace.id())?
            .saturating_add(1);
        let tab = workspace
            .tabs()
            .iter()
            .find(|candidate| candidate.id() == self.app_shell.active_tab_id())?;
        let tab_index = workspace
            .tabs()
            .iter()
            .position(|candidate| candidate.id() == tab.id())?
            .saturating_add(1);
        let pane_index = tab
            .panes()
            .iter()
            .position(|candidate| candidate.id() == pane_id)?
            .saturating_add(1);
        let pane = tab.panes().get(pane_index.saturating_sub(1))?;
        let size = self.pane_runtime_ref(pane_id)?.terminal().grid().size();
        let launch = pane.launch();
        let title = self
            .pane_title(pane_id)
            .unwrap_or_else(|| "unavailable".to_owned());
        let pid = self
            .pane_process_id(pane_id)
            .map_or_else(|| "unavailable".to_owned(), |pid| pid.to_string());
        let cwd = launch.cwd().unwrap_or("unavailable");
        let args = if launch.args().is_empty() {
            match launch.domain() {
                PaneLaunchDomain::Local => "none".to_owned(),
                PaneLaunchDomain::Ssh(ssh) if ssh.remote_command().is_empty() => "none".to_owned(),
                PaneLaunchDomain::Ssh(ssh) => ssh.remote_command().join(" "),
            }
        } else {
            launch.args().join(" ")
        };
        let environment_count = launch.environment().len();
        let environment_label = if environment_count == 1 {
            "variable"
        } else {
            "variables"
        };

        Some(vec![
            format!("Pane {}", pane_id.get()),
            format!("workspace: {} ({workspace_index})", workspace.name()),
            format!("tab: {tab_index}"),
            format!("pane: {pane_index}"),
            format!("title: {title}"),
            format!("dimensions: {}x{}", size.columns, size.rows),
            format!("pid: {pid}"),
            format!("cwd: {cwd}"),
            format!("program: {}", pane_launch_display_program(launch)),
            format!("args: {args}"),
            format!("domain: {}", pane_launch_domain_name(launch)),
            format!("environment: {environment_count} {environment_label}"),
        ])
    }

}

impl NativeWindowApp {
    fn pane_process_id(&self, pane: rssh_core::PaneId) -> Option<u32> {
        if pane == self.app_shell.active_pane_id() {
            self.session_process_id
        } else {
            self.pane_runtimes
                .get(&pane)
                .and_then(|runtime| runtime.session_process_id)
        }
    }

    fn pane_tty_name(&self, pane: rssh_core::PaneId) -> Option<&str> {
        if pane == self.app_shell.active_pane_id() {
            self.session_tty_name.as_deref()
        } else {
            self.pane_runtimes
                .get(&pane)
                .and_then(|runtime| runtime.session_tty_name.as_deref())
        }
    }

    fn pane_terminal_icon_title(&self, pane: rssh_core::PaneId) -> Option<String> {
        let title = if pane == self.app_shell.active_pane_id() {
            self.runtime.terminal().icon_title()
        } else {
            self.pane_runtimes
                .get(&pane)
                .and_then(|runtime| runtime.runtime.terminal().icon_title())
        };

        title
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(str::to_owned)
    }

    fn pane_terminal_window_title(&self, pane: rssh_core::PaneId) -> Option<String> {
        let title = if pane == self.app_shell.active_pane_id() {
            self.runtime.terminal().window_title()
        } else {
            self.pane_runtimes
                .get(&pane)
                .and_then(|runtime| runtime.runtime.terminal().window_title())
        };

        title
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(str::to_owned)
    }

    fn pane_last_command(&self, pane: rssh_core::PaneId) -> Option<String> {
        if pane == self.app_shell.active_pane_id() {
            return last_command_from_terminal(self.runtime.terminal());
        }

        self.pane_runtimes
            .get(&pane)
            .and_then(|runtime| last_command_from_terminal(runtime.runtime.terminal()))
    }

    fn record_reported_iterm_mouse_info(
        &mut self,
        mouse_cell: PaneMouseCell,
        kind: WindowMouseEventKind,
        modifiers: ModifiersState,
    ) {
        let source_row = self.pane_mouse_source_row(mouse_cell.pane_id, mouse_cell.row);
        self.last_mouse_info = iterm_mouse_info_for_event(
            mouse_cell,
            source_row,
            kind,
            modifiers,
            ITERM_MOUSE_REPORT_SIDE_EFFECT,
        );
    }

    fn pane_mouse_source_row(&self, pane: rssh_core::PaneId, row: u16) -> usize {
        let (history_len, scrollback_offset) = if pane == self.app_shell.active_pane_id() {
            (
                self.runtime.terminal().scrollback().len(),
                self.current_scrollback_offset(),
            )
        } else {
            self.pane_runtimes.get(&pane).map_or((0, 0), |runtime| {
                (
                    runtime.runtime.terminal().scrollback().len(),
                    runtime
                        .ui
                        .stable_viewport
                        .scrollback_offset(runtime.runtime.terminal()),
                )
            })
        };
        copy_mode_viewport_top(history_len, scrollback_offset).saturating_add(usize::from(row))
    }

    fn update_active_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        if window_mouse_button_code(button).is_none() {
            return;
        }

        match state {
            ElementState::Pressed => self.active_mouse_button = Some(button),
            ElementState::Released if self.active_mouse_button == Some(button) => {
                self.active_mouse_button = None;
            }
            ElementState::Released => {}
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn handle_selection_mouse_input(&mut self, state: ElementState, button: MouseButton) -> bool {
        if button != MouseButton::Left {
            return false;
        }

        match state {
            ElementState::Pressed => {
                let Some(cell) = self.selection_source_cell_from_mouse_position() else {
                    return false;
                };
                self.active_ui.exit_overlay();
                if self.modifiers == ModifiersState::SHIFT
                    && self.active_ui.ordinary_selection.is_some()
                {
                    return self.extend_selection_to_mouse_cursor(WindowMouseSelectionMode::Cell);
                }
                if self.modifiers == (ModifiersState::ALT | ModifiersState::SHIFT)
                    && self.active_ui.ordinary_selection.is_some()
                {
                    return self.extend_selection_to_mouse_cursor(WindowMouseSelectionMode::Block);
                }
                if self.modifiers == ModifiersState::ALT {
                    self.set_ordinary_selection(StableOrdinarySelection::rectangular(
                        cell,
                        cell,
                        self.runtime.terminal().current_seqno(),
                    ));
                    self.selecting = true;
                    self.last_left_click = None;
                    self.refresh_snapshot();
                    self.apply_window_title();
                    return true;
                }
                let click = self.next_left_click(cell, Instant::now());
                if click.count >= 3 {
                    let selection = self.line_source_selection_at_cell(cell);
                    self.set_ordinary_selection(StableOrdinarySelection::new(
                        selection.anchor,
                        selection.focus,
                        self.runtime.terminal().current_seqno(),
                    ));
                    self.selecting = false;
                    self.last_left_click = Some(click);
                    self.refresh_snapshot();
                    self.apply_window_title();
                    return true;
                }
                if click.count == 2
                    && let Some(selection) = self.double_click_word_selection(cell)
                {
                    self.set_ordinary_selection(StableOrdinarySelection::new(
                        selection.anchor,
                        selection.focus,
                        self.runtime.terminal().current_seqno(),
                    ));
                    self.selecting = false;
                    self.last_left_click = Some(click);
                    self.refresh_snapshot();
                    self.apply_window_title();
                    return true;
                }
                self.set_ordinary_selection(StableOrdinarySelection::new(
                    cell,
                    cell,
                    self.runtime.terminal().current_seqno(),
                ));
                self.selecting = true;
                self.last_left_click = Some(click);
                self.refresh_snapshot();
                self.apply_window_title();
                true
            }
            ElementState::Released => {
                if !self.selecting {
                    if (self.modifiers == ModifiersState::SHIFT
                        || self.modifiers == (ModifiersState::ALT | ModifiersState::SHIFT))
                        && self
                            .active_ui
                            .ordinary_selection
                            .is_some_and(|selection| !selection.is_single_cell())
                    {
                        let _ = self.copy_selection_to_clipboard_and_primary_selection();
                        self.refresh_snapshot();
                        self.apply_window_title();
                        return true;
                    }
                    if self.modifiers.is_empty()
                        && self.last_left_click.is_some_and(|click| click.count >= 2)
                        && self
                            .active_ui
                            .ordinary_selection
                            .is_some_and(|selection| !selection.is_single_cell())
                    {
                        let _ = self.copy_selection_to_clipboard_and_primary_selection();
                        self.refresh_snapshot();
                        self.apply_window_title();
                        return true;
                    }
                    return false;
                }
                self.selecting = false;
                if self
                    .active_ui
                    .ordinary_selection
                    .is_some_and(StableOrdinarySelection::is_single_cell)
                {
                    self.clear_ordinary_selection();
                    if self.modifiers.is_empty() || self.modifiers == ModifiersState::SHIFT {
                        let _ = self.open_link_at_mouse_cursor();
                    }
                } else {
                    let _ = self.copy_selection_to_clipboard_and_primary_selection();
                }
                self.refresh_snapshot();
                self.apply_window_title();
                true
            }
        }
    }

    fn select_text_at_mouse_cursor(&mut self, mode: WindowMouseSelectionMode) -> bool {
        let Some(cell) = self.selection_source_cell_from_mouse_position() else {
            return false;
        };

        let selection = match mode {
            WindowMouseSelectionMode::Cell => Some(WindowSourceSelection::new(cell, cell)),
            WindowMouseSelectionMode::Word => self.word_source_selection_at_cell(cell),
            WindowMouseSelectionMode::Line => Some(self.line_source_selection_at_cell(cell)),
            WindowMouseSelectionMode::Block => Some(WindowSourceSelection::rectangular(cell, cell)),
            WindowMouseSelectionMode::SemanticZone => {
                copy_mode_semantic_zone_source_selection(self.runtime.terminal(), cell)
            }
        };
        let Some(selection) = selection else {
            return false;
        };

        self.active_ui.exit_overlay();
        let stable = if selection.rectangular {
            StableOrdinarySelection::rectangular(
                selection.anchor,
                selection.focus,
                self.runtime.terminal().current_seqno(),
            )
        } else {
            StableOrdinarySelection::new(
                selection.anchor,
                selection.focus,
                self.runtime.terminal().current_seqno(),
            )
        };
        self.set_ordinary_selection(stable);
        self.selecting = false;
        self.last_left_click = None;
        self.refresh_snapshot();
        self.apply_window_title();
        true
    }

    fn extend_selection_to_mouse_cursor(&mut self, mode: WindowMouseSelectionMode) -> bool {
        let Some(current) = self.active_ui.ordinary_selection else {
            return false;
        };
        let Some(cell) = self.selection_source_cell_from_mouse_position() else {
            return false;
        };

        let target = match mode {
            WindowMouseSelectionMode::Cell => Some(WindowSourceSelection::new(cell, cell)),
            WindowMouseSelectionMode::Word => self.word_source_selection_at_cell(cell),
            WindowMouseSelectionMode::Line => Some(self.line_source_selection_at_cell(cell)),
            WindowMouseSelectionMode::Block => {
                self.set_ordinary_selection(StableOrdinarySelection::rectangular(
                    current.anchor,
                    cell,
                    self.runtime.terminal().current_seqno(),
                ));
                self.active_ui.exit_overlay();
                self.selecting = false;
                self.last_left_click = None;
                self.refresh_snapshot();
                self.apply_window_title();
                return true;
            }
            WindowMouseSelectionMode::SemanticZone => {
                copy_mode_semantic_zone_source_selection(self.runtime.terminal(), cell)
            }
        };
        let Some(target) = target else {
            return false;
        };

        let focus = stable_selection_focus_for_extension(current, target);
        self.active_ui.exit_overlay();
        self.set_ordinary_selection(StableOrdinarySelection::new(
            current.anchor,
            focus,
            self.runtime.terminal().current_seqno(),
        ));
        self.selecting = false;
        self.last_left_click = None;
        self.refresh_snapshot();
        self.apply_window_title();
        true
    }

    fn handle_hyperlink_mouse_input(&mut self, state: ElementState, button: MouseButton) -> bool {
        if state != ElementState::Pressed
            || button != MouseButton::Left
            || !window_hyperlink_activation_modifiers(self.modifiers)
        {
            return false;
        }

        self.open_link_at_mouse_cursor()
    }

    fn complete_selection_or_open_link_at_mouse_cursor(&mut self) -> bool {
        self.complete_selection_or_open_link_at_mouse_cursor_to(
            WindowCopyDestination::ClipboardAndPrimarySelection,
        )
    }

    fn complete_selection_or_open_link_at_mouse_cursor_to(
        &mut self,
        destination: WindowCopyDestination,
    ) -> bool {
        if self.selecting {
            return self.complete_selection_to(destination);
        }

        self.open_link_at_mouse_cursor()
    }

    fn complete_selection_to_clipboard_and_primary_selection(&mut self) -> bool {
        self.complete_selection_to(WindowCopyDestination::ClipboardAndPrimarySelection)
    }

    fn complete_selection_to(&mut self, destination: WindowCopyDestination) -> bool {
        self.selecting = false;
        if self
            .active_ui
            .ordinary_selection
            .is_some_and(StableOrdinarySelection::is_single_cell)
        {
            self.clear_ordinary_selection();
            self.refresh_snapshot();
            self.apply_window_title();
            return false;
        }
        let copied = self.copy_selection_to(destination);
        self.apply_window_title();
        copied
    }

    fn open_link_at_mouse_cursor(&mut self) -> bool {
        let Some(url) = self.hyperlink_at_mouse_position() else {
            return false;
        };

        self.open_uri(&url)
    }

    fn open_uri(&mut self, uri: &str) -> bool {
        let pane_id = self.app_shell.active_pane_id();
        self.open_uri_for_pane(pane_id, uri)
    }

    fn open_uri_for_target(&mut self, target: WheelTarget, uri: &str) -> bool {
        self.open_uri_in_context(target.pane_id, Some(target), uri)
    }

    fn open_uri_for_pane(&mut self, pane_id: rssh_core::PaneId, uri: &str) -> bool {
        self.open_uri_in_context(pane_id, None, uri)
    }

    fn open_uri_in_context(
        &mut self,
        pane_id: rssh_core::PaneId,
        target: Option<WheelTarget>,
        uri: &str,
    ) -> bool {
        let event = NativeWindowOpenUri {
            window_id: self.app_window_id,
            pane: pane_id,
            uri: uri.to_owned(),
        };
        if self.dispatch_open_uri_in_context(&event, target) {
            (self.hyperlink_opener)(uri);
        }
        true
    }

    fn hyperlink_at_mouse_position(&self) -> Option<Arc<str>> {
        let mouse_cell = self.mouse_cell_for_active_pane()?;
        let snapshot = self.pane_snapshot(mouse_cell.pane_id)?;
        if let Some(hyperlink) = snapshot
            .iter_cells()
            .find(|cell| cell.row == mouse_cell.row && cell.column == mouse_cell.column)
            .and_then(|cell| cell.hyperlink.clone())
        {
            return Some(hyperlink);
        }

        hyperlink_rule_at_cell(
            snapshot,
            mouse_cell.row,
            mouse_cell.column,
            &self.hyperlink_rules,
        )
    }

    fn update_selection_from_mouse_position(&mut self) -> bool {
        let Some(cell) = self.selection_source_cell_from_mouse_position() else {
            return false;
        };
        let Some(selection) = self.active_ui.ordinary_selection else {
            return false;
        };

        if self.active_mouse_button == Some(MouseButton::Left)
            && self.modifiers.is_empty()
            && let Some(click) = self.last_left_click
            && click.count >= 2
        {
            let target = if click.count == 2 {
                self.word_source_selection_at_cell(cell)
            } else {
                Some(self.line_source_selection_at_cell(cell))
            };
            let Some(target) = target else {
                return false;
            };
            let focus = stable_selection_focus_for_extension(selection, target);
            if selection.focus == focus {
                return false;
            }

            self.set_ordinary_selection(StableOrdinarySelection::new(
                selection.anchor,
                focus,
                self.runtime.terminal().current_seqno(),
            ));
            self.selecting = true;
            self.refresh_snapshot();
            return true;
        }

        if !self.selecting {
            return false;
        }
        let sequence = self.runtime.terminal().current_seqno();
        let Some(selection) = self.active_ui.ordinary_selection.as_mut() else {
            return false;
        };
        if selection.focus == cell {
            return false;
        }

        selection.set_focus(cell);
        selection.sequence = sequence;
        self.last_left_click = None;
        self.update_selection_projection();
        self.refresh_snapshot();
        true
    }

    fn selection_cell_from_mouse_position(&self) -> Option<SelectionCell> {
        let PaneMouseCell { column, row, .. } = self.mouse_cell_for_active_pane()?;
        let size = self.runtime.terminal().grid().size();
        if row >= size.rows || column >= size.columns {
            return None;
        }

        Some(SelectionCell { row, column })
    }

    fn selection_source_cell_from_mouse_position(&self) -> Option<SelectionSourceCell> {
        self.selection_cell_from_mouse_position()
            .map(|cell| self.stable_source_cell_for_viewport_cell(cell))
    }

    fn next_left_click(&self, cell: SelectionSourceCell, time: Instant) -> WindowClick {
        let count = self
            .last_left_click
            .and_then(|last_click| {
                let elapsed = time.checked_duration_since(last_click.time)?;
                (elapsed <= DOUBLE_CLICK_MAX_INTERVAL
                    && last_click.cell.domain == cell.domain
                    && last_click.cell.row == cell.row)
                    .then_some(last_click.count.saturating_add(1))
            })
            .unwrap_or(1);

        WindowClick { cell, time, count }
    }

    fn double_click_word_selection(
        &self,
        cell: SelectionSourceCell,
    ) -> Option<WindowSourceSelection> {
        let last_click = self.last_left_click?;
        let selection = self.word_source_selection_at_cell(cell)?;
        let (start, end) = selection.normalized();
        if last_click.cell.domain == start.domain
            && compare_selection_source_cell(last_click.cell, start) != std::cmp::Ordering::Less
            && compare_selection_source_cell(last_click.cell, end) != std::cmp::Ordering::Greater
        {
            Some(selection)
        } else {
            None
        }
    }

    fn line_source_selection_at_cell(&self, cell: SelectionSourceCell) -> WindowSourceSelection {
        let size = self.runtime.terminal().grid().size();
        WindowSourceSelection::new(
            SelectionSourceCell { column: 0, ..cell },
            SelectionSourceCell {
                column: usize::from(size.columns.saturating_sub(1)),
                ..cell
            },
        )
    }

    fn word_source_selection_at_cell(
        &self,
        cell: SelectionSourceCell,
    ) -> Option<WindowSourceSelection> {
        copy_mode_word_source_selection(
            self.runtime.terminal(),
            cell,
            &self.selection_word_boundary,
        )
    }

    fn handle_scrollback_shortcut(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.default_assignment_disabled_for_key(key, modifiers) {
            return false;
        }

        if modifiers != ModifiersState::SHIFT {
            return false;
        }

        let Key::Named(named) = key else {
            return false;
        };

        match named {
            NamedKey::PageUp => {
                self.scroll_viewport_lines(self.viewport_page_rows());
                true
            }
            NamedKey::PageDown => {
                self.scroll_viewport_lines(-self.viewport_page_rows());
                true
            }
            _ => false,
        }
    }

    fn viewport_page_rows(&self) -> isize {
        isize::try_from(i32::from(self.runtime.terminal().grid().size().rows)).unwrap_or(isize::MAX)
    }

    fn sync_window_title_from_runtime(&mut self) {
        let Some(title) = self.runtime.terminal().title().map(str::to_owned) else {
            return;
        };

        if self.window_title == title {
            return;
        }

        self.window_title = title;
        self.apply_window_title();
    }

    fn sync_active_pane_current_working_dir_from_runtime(&mut self) {
        let pane = self.app_shell.active_pane_id();
        if let PaneRuntimeCwdUpdate::Resolved(cwd) = pane_runtime_current_working_dir_if_due(
            &mut self.runtime,
            self.session_process_id,
            Instant::now(),
        ) {
            self.sync_pane_current_working_dir_from_value(pane, cwd);
        }
    }

    fn sync_active_pane_user_vars_from_runtime(&mut self) {
        let pane = self.app_shell.active_pane_id();
        let user_vars = self
            .runtime
            .terminal()
            .user_vars()
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        self.sync_pane_user_vars_from_pairs(pane, user_vars);
    }

    fn sync_active_pane_badge_format_from_runtime(&mut self) {
        let pane = self.app_shell.active_pane_id();
        let badge_format = self.runtime.terminal().badge_format().map(str::to_owned);
        self.sync_pane_badge_format_from_value(pane, badge_format);
    }

    fn sync_active_pane_progress_from_runtime(&mut self) {
        let pane = self.app_shell.active_pane_id();
        self.sync_pane_progress_from_value(pane, self.runtime.progress());
    }

    fn sync_pane_progress_from_value(
        &mut self,
        pane: rssh_core::PaneId,
        progress: TerminalProgress,
    ) {
        let progress = pane_progress_from_terminal_progress(progress);
        if self.pane_progress(pane) == Some(progress) {
            return;
        }

        if let Err(error) = self
            .app_shell
            .apply_action(AppAction::SetPaneProgress { pane, progress })
        {
            eprintln!("failed to sync pane progress: {error:?}");
        }
    }

    fn sync_pane_has_unseen_output_from_value(
        &mut self,
        pane: rssh_core::PaneId,
        has_unseen_output: bool,
    ) {
        if self.pane_has_unseen_output(pane) == Some(has_unseen_output) {
            return;
        }

        if let Err(error) = self
            .app_shell
            .apply_action(AppAction::SetPaneHasUnseenOutput {
                pane,
                has_unseen_output,
            })
        {
            eprintln!("failed to sync pane unseen-output state: {error:?}");
        }
    }

    fn sync_pane_badge_format_from_value(
        &mut self,
        pane: rssh_core::PaneId,
        badge_format: Option<String>,
    ) {
        if self.pane_badge_format(pane) == badge_format.as_deref() {
            return;
        }

        if let Err(error) = self
            .app_shell
            .apply_action(AppAction::SetPaneBadgeFormat { pane, badge_format })
        {
            eprintln!("failed to sync pane badge format: {error:?}");
        }
    }

    fn sync_pane_user_vars_from_pairs(
        &mut self,
        pane: rssh_core::PaneId,
        user_vars: Vec<(String, String)>,
    ) {
        for (name, value) in user_vars {
            if self.pane_user_var(pane, &name) == Some(value.as_str()) {
                continue;
            }

            let change = NativeWindowUserVarChange {
                window_id: self.app_window_id,
                pane,
                name: name.clone(),
                value: value.clone(),
            };
            if let Err(error) =
                self.app_shell
                    .apply_action(AppAction::SetPaneUserVar { pane, name, value })
            {
                eprintln!("failed to sync pane user var: {error:?}");
                continue;
            }

            self.dispatch_user_var_change(&change);
        }
    }

    fn sync_pane_current_working_dir_from_value(
        &mut self,
        pane: rssh_core::PaneId,
        cwd: Option<String>,
    ) {
        if self.pane_launch_current_working_dir(pane) == cwd.as_deref() {
            return;
        }

        if let Err(error) = self
            .app_shell
            .apply_action(AppAction::SetPaneCurrentWorkingDir { pane, cwd })
        {
            eprintln!("failed to sync pane current working directory: {error:?}");
        }
    }

    fn pane_launch_current_working_dir(&self, pane: rssh_core::PaneId) -> Option<&str> {
        self.app_shell
            .workspaces()
            .iter()
            .flat_map(rssh_core::app_shell::Workspace::tabs)
            .flat_map(rssh_core::app_shell::Tab::panes)
            .find(|candidate| candidate.id() == pane)
            .and_then(|candidate| candidate.launch().cwd())
    }

    fn pane_user_var(&self, pane: rssh_core::PaneId, name: &str) -> Option<&str> {
        self.app_shell
            .workspaces()
            .iter()
            .flat_map(rssh_core::app_shell::Workspace::tabs)
            .flat_map(rssh_core::app_shell::Tab::panes)
            .find(|candidate| candidate.id() == pane)
            .and_then(|candidate| candidate.user_vars().get(name))
            .map(String::as_str)
    }

    fn pane_badge_format(&self, pane: rssh_core::PaneId) -> Option<&str> {
        self.app_shell
            .workspaces()
            .iter()
            .flat_map(rssh_core::app_shell::Workspace::tabs)
            .flat_map(rssh_core::app_shell::Tab::panes)
            .find(|candidate| candidate.id() == pane)
            .and_then(rssh_core::app_shell::Pane::badge_format)
    }

    fn pane_progress(&self, pane: rssh_core::PaneId) -> Option<PaneProgress> {
        self.app_shell
            .workspaces()
            .iter()
            .flat_map(rssh_core::app_shell::Workspace::tabs)
            .flat_map(rssh_core::app_shell::Tab::panes)
            .find(|candidate| candidate.id() == pane)
            .map(rssh_core::app_shell::Pane::progress)
    }

    fn pane_has_unseen_output(&self, pane: rssh_core::PaneId) -> Option<bool> {
        self.app_shell
            .workspaces()
            .iter()
            .flat_map(rssh_core::app_shell::Workspace::tabs)
            .flat_map(rssh_core::app_shell::Tab::panes)
            .find(|candidate| candidate.id() == pane)
            .map(rssh_core::app_shell::Pane::has_unseen_output)
    }

    fn higher_level_ui_suppresses_pane_overlay(&self) -> bool {
        self.command_palette.is_some()
            || self.char_select.is_some()
            || self.pane_select.is_some()
            || self.tab_navigator.is_some()
            || self.prompt_input_line.is_some()
            || self.input_selector.is_some()
            || self.confirmation.is_some()
            || self.close_confirmation.is_some()
    }

    fn higher_level_ui_blocks_pane_surface_mouse(&self) -> bool {
        self.debug_overlay_active || self.higher_level_ui_suppresses_pane_overlay()
    }

    fn restore_active_pane_presentation_after_higher_level_ui(&mut self) {
        self.update_selection_projection();
        self.rebuild_snapshot();
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn effective_window_title(&self) -> String {
        let mut title = self.window_title.clone();
        title.push_str(&self.app_shell_state_id_suffix());

        if let PaneLaunchDomain::Ssh(ssh) = self.app_shell.active_pane().launch().domain() {
            title.push_str(" - SSH ");
            title.push_str(ssh.target());
            title.push_str(" [");
            title.push_str(connection_metric_name(self.ssh_connection_state_for_pane(
                self.app_shell.active_pane_id(),
            )));
            title.push(']');
        }

        #[cfg(feature = "ssh")]
        let active_pane_id = self.app_shell.active_pane_id();
        #[cfg(feature = "ssh")]
        if let Some(prompt) = self.ssh_secret_prompts.get(&active_pane_id) {
            title.push_str(" - SSH ");
            title.push_str(match prompt.prompt.kind {
                SecretPromptKind::Password => "password",
                SecretPromptKind::PrivateKeyPassphrase => "private-key passphrase",
            });
            title.push_str(" (masked)");
        } else if let Some((challenge, _)) = self.ssh_host_key_prompts.get(&active_pane_id)
        {
            title.push_str(" - SSH host key ");
            title.push_str(&challenge.host);
            title.push(':');
            title.push_str(&challenge.port.to_string());
            title.push(' ');
            title.push_str(&challenge.fingerprint);
            if let Some(path) = &challenge.known_hosts_path {
                title.push_str(" [");
                title.push_str(&path.to_string_lossy());
                title.push(']');
            }
            match challenge.status {
                rssh_ssh::HostKeyStatus::Changed => {
                    title.push_str(" BLOCKED [Esc] cancel");
                }
                rssh_ssh::HostKeyStatus::Unknown => {
                    title.push_str(" [1] once [2] store [Esc] cancel");
                }
                rssh_ssh::HostKeyStatus::Known => {
                    title.push_str(" [Esc] cancel");
                }
            }
        }

        if !self.higher_level_ui_suppresses_pane_overlay() {
            match self.active_ui.copy_search_mode() {
                Some(WindowCopySearchMode::Search) => {
                    let search = self
                        .active_ui
                        .search()
                        .expect("Search mode always owns Search state");
                    title.push_str(" - ");
                    title.push_str(&search_status(search));
                }
                Some(WindowCopySearchMode::Copy) => {
                    let copy_mode = self
                        .active_ui
                        .copy_mode()
                        .expect("Copy mode always owns Copy state");
                    title.push_str(" - ");
                    title.push_str(&Self::copy_mode_status(copy_mode));
                }
                None => {}
            }

            if let Some(quick_select) = self.active_ui.quick_select() {
                title.push_str(" - ");
                title.push_str(&Self::quick_select_status(quick_select));
            }
        }

        if let Some(pane_select) = &self.pane_select {
            title.push_str(" - ");
            title.push_str(&Self::pane_select_status(pane_select));
        }

        if let Some(char_select) = &self.char_select {
            title.push_str(" - ");
            title.push_str(&Self::char_select_status(char_select));
        }

        if let Some(tab_navigator) = &self.tab_navigator {
            title.push_str(" - ");
            title.push_str(&Self::tab_navigator_status(tab_navigator));
        }

        if let Some(prompt_input_line) = &self.prompt_input_line {
            title.push_str(" - ");
            title.push_str(&Self::prompt_input_line_status(prompt_input_line));
        }

        if let Some(input_selector) = &self.input_selector {
            title.push_str(" - ");
            title.push_str(&Self::input_selector_status(input_selector));
        }

        if let Some(confirmation) = &self.confirmation {
            title.push_str(" - ");
            title.push_str(&Self::confirmation_status(confirmation));
        }

        if let Some(close_confirmation) = &self.close_confirmation {
            title.push_str(" - ");
            title.push_str(&Self::close_confirmation_status(close_confirmation));
        }

        if let Some(key_table) = self.key_table_stack.last() {
            title.push_str(" - KeyTable: ");
            title.push_str(&key_table.name);
        }

        if let Some(command_palette) = &self.command_palette {
            title.push_str(" - ");
            title.push_str(&self.command_palette_status(command_palette));
        }

        if let Some(notification) = &self.latest_notification {
            title.push_str(" - ");
            title.push_str(&notification_status(notification));
        }

        let default_title = title;
        let tabs = self.native_window_tab_information();
        let active_tab_info = tabs
            .iter()
            .find(|tab| tab.tab_id == self.app_shell.active_tab_id())
            .cloned()
            .unwrap_or_else(|| {
                tabs.first()
                    .cloned()
                    .expect("window title formatting requires at least one tab")
            });
        let active_pane_info = active_tab_info.active_pane.clone();
        let panes = active_tab_info.panes.clone();
        let title_format = NativeWindowTitleFormat {
            default_title: default_title.clone(),
            active_tab: self.app_shell.active_tab_id(),
            active_pane: self.app_shell.active_pane_id(),
            active_key_table: self
                .key_table_stack
                .last()
                .map(|activation| activation.name.clone()),
            tab_count: self.app_shell.active_workspace().tabs().len(),
            pane_count: self.app_shell.active_tab().panes().len(),
            config: self.native_effective_config(),
            active_tab_info,
            active_pane_info,
            tabs,
            panes,
        };

        let lua_window_title = self
            .lua_window_title
            .as_ref()
            .and_then(|title| title.resolve(&title_format));

        (self.window_title_formatter)(&title_format)
            .or(lua_window_title)
            .unwrap_or(default_title)
    }

    fn scrollback_scrollbar(&self) -> Option<ScrollbackScrollbar> {
        if !self.enable_scroll_bar {
            return None;
        }

        let history_len = self.runtime.terminal().scrollback().len();
        let rows = self.runtime.terminal().grid().size().rows;
        let mut scrollbar =
            ScrollbackScrollbar::new(history_len, rows, self.current_scrollback_offset())?;
        if let Some(thumb_color) = self.scrollbar_thumb_color {
            scrollbar = scrollbar.with_thumb_color(color_to_rgba(
                thumb_color,
                rterm_render_core::SCROLLBAR_THUMB_COLOR,
            ));
        }
        Some(match self.min_scroll_bar_height {
            Some(min_thumb_height) => {
                scrollbar.with_min_thumb_height(RenderScrollbarThumbSize::from(min_thumb_height))
            }
            None => scrollbar,
        })
    }

    fn apply_window_title(&self) {
        let title = self.effective_window_title();
        #[cfg(test)]
        if self.applied_window_titles.borrow().is_some() {
            self.applied_window_titles
                .borrow_mut()
                .as_mut()
                .expect("checked title observer")
                .push(title.clone());
        }
        if let Some(window) = &self.window {
            let changed = self
                .applied_window_title
                .borrow()
                .as_deref()
                != Some(title.as_str());
            if changed {
                window.set_title(&title);
                *self.applied_window_title.borrow_mut() = Some(title);
            }
        }
    }

    #[cfg(test)]
    fn clear_applied_window_titles_for_test(&self) {
        *self.applied_window_titles.borrow_mut() = Some(Vec::new());
    }

    #[cfg(test)]
    fn applied_window_titles_for_test(&self) -> Ref<'_, Vec<String>> {
        Ref::map(self.applied_window_titles.borrow(), |titles| {
            titles.as_ref().expect("title observer must be enabled")
        })
    }

    fn spawn_pty(&mut self) -> Result<(), Box<dyn Error>> {
        self.restart_missing_transferred_local_panes()?;
        if self.active_pane_transport_is_started() {
            return Ok(());
        }

        self.mark_transport_start_requested();

        let runtime = self.spawn_pane_runtime_for_active_pane()?;
        self.install_active_runtime(runtime);
        Ok(())
    }

    fn restart_missing_transferred_local_panes(&mut self) -> Result<(), Box<dyn Error>> {
        let worker = self.runtime.worker();
        let panes = self
            .app_shell
            .pane_ids()
            .into_iter()
            .filter(|pane_id| {
                let transport = if *pane_id == self.app_shell.active_pane_id() {
                    self.active_runtime_transport
                } else {
                    self.pane_runtimes
                        .get(pane_id)
                        .and_then(|runtime| runtime.transport)
                };
                transport == Some(PaneRuntimeTransportKind::LocalPty)
                    && worker.is_none_or(|worker| !worker.contains_pane(*pane_id))
            })
            .collect::<Vec<_>>();
        for pane_id in panes {
            self.restart_transferred_local_pane(pane_id)?;
        }
        Ok(())
    }

    fn restart_transferred_local_pane(
        &mut self,
        pane_id: rssh_core::PaneId,
    ) -> Result<(), Box<dyn Error>> {
        let event_proxy = self.event_proxy.clone().ok_or_else(|| {
            Box::new(io::Error::other("window event proxy is not configured")) as Box<dyn Error>
        })?;
        let (tab_id, launch) = self
            .app_shell
            .workspaces()
            .iter()
            .flat_map(rssh_core::app_shell::Workspace::tabs)
            .find_map(|tab| {
                tab.panes()
                    .iter()
                    .find(|pane| pane.id() == pane_id)
                    .map(|pane| (tab.id(), pane.launch().clone()))
            })
            .ok_or_else(|| {
                Box::new(io::Error::other(format!(
                    "pane {} has no launch metadata",
                    pane_id.get()
                ))) as Box<dyn Error>
            })?;
        if !matches!(launch.domain(), PaneLaunchDomain::Local) {
            return Err(Box::new(io::Error::other(
                "transferred local restart received a non-local launch",
            )));
        }
        let term_session_id =
            iterm_session_termid(self.app_window_id.get(), tab_id.get(), pane_id.get());
        let environment = self.pane_environment_variables();
        let command = pty_command_from_pane_launch_with_term_session_id(
            &launch,
            &self.term,
            &environment,
            self.default_cwd.as_deref(),
            &term_session_id,
        );
        let size = self
            .pane_runtime_ref(pane_id)
            .ok_or_else(|| {
                Box::new(io::Error::other(format!(
                    "pane {} has no runtime owner",
                    pane_id.get()
                ))) as Box<dyn Error>
            })?
            .terminal()
            .grid()
            .size();
        let pty_size = PtySize::try_new(size.columns, size.rows)?;
        crate::stage7_attribution::audit_product_service_start(
            crate::stage7_attribution::ProductServiceEntry::LocalPty,
        )?;
        self.metrics.start_spawn_timer();
        let session = PtySession::spawn(&command, pty_size)?;
        let process_id = session.process_id();
        let tty_name = session.tty_name();
        let transport = LocalPtyTransport::from_session(session)?;
        let wake_proxy = event_proxy.clone();
        let window_id = self.app_window_id;
        let notice_waker: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = wake_proxy.send_event(WindowUserEvent::RuntimeWakeWindow { window_id });
        });
        self.install_transferred_local_transport(
            pane_id,
            transport,
            process_id,
            tty_name,
            notice_waker,
        )
    }

    fn install_transferred_local_transport<T: SessionTransport>(
        &mut self,
        pane_id: rssh_core::PaneId,
        transport: T,
        process_id: Option<u32>,
        tty_name: Option<String>,
        notice_waker: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<(), Box<dyn Error>> {
        let terminal = self
            .pane_runtime_ref(pane_id)
            .ok_or_else(|| {
                Box::new(io::Error::other(format!(
                    "pane {} has no runtime owner",
                    pane_id.get()
                ))) as Box<dyn Error>
            })?
            .terminal()
            .clone();
        let size = terminal.grid().size();
        let worker_runtime = self
            .configured_pane_terminal_runtime_from_terminal(terminal)
            .inner;
        let capture = PaneCapturePolicy {
            host_stream: self.metrics.pty_linkage_enabled,
            visible_output: self.session_log.is_some(),
        };
        let token = if let Some(worker) = self.runtime.worker_mut() {
            let config = PaneWorkerConfig {
                size,
                capture_host_stream: capture.host_stream,
                capture_visible_output: capture.visible_output,
                ..PaneWorkerConfig::default()
            };
            worker.add_transport(pane_id, transport, config, worker_runtime)?
        } else {
            let worker = WindowPaneRuntime::open_transport(
                PaneRuntimeRoute {
                    window: self.app_window_id,
                    pane: pane_id,
                },
                transport,
                size,
                worker_runtime,
                capture,
                notice_waker,
            )?;
            let token = worker.token_for_pane(pane_id).ok_or_else(|| {
                Box::new(io::Error::other("new local worker omitted its pane token"))
                    as Box<dyn Error>
            })?;
            self.runtime.install_worker(Some(worker));
            token
        };
        let is_active = pane_id == self.app_shell.active_pane_id();
        if is_active
            && let Some(worker) = self.runtime.worker_mut()
        {
            worker.activate(token)?;
        }
        let runtime_generation = self.allocate_pane_runtime_generation();
        if is_active {
            self.session = None;
            self.session_process_id = process_id;
            self.session_tty_name = tty_name;
            self.writer = None;
            self.reader_thread = None;
            self.writer_thread = None;
            self.active_runtime_generation = runtime_generation;
            self.active_runtime_transport = Some(PaneRuntimeTransportKind::LocalPty);
        } else {
            let runtime = self.pane_runtimes.get_mut(&pane_id).ok_or_else(|| {
                Box::new(io::Error::other(format!(
                    "inactive pane {} has no runtime owner",
                    pane_id.get()
                ))) as Box<dyn Error>
            })?;
            runtime.session = None;
            runtime.session_process_id = process_id;
            runtime.session_tty_name = tty_name;
            runtime.writer = None;
            runtime.reader_thread = None;
            runtime.writer_thread = None;
            runtime.runtime_generation = runtime_generation;
            runtime.transport = Some(PaneRuntimeTransportKind::LocalPty);
        }
        Ok(())
    }

    fn restart_pane_runtime(&mut self, pane_id: rssh_core::PaneId) -> Result<(), Box<dyn Error>> {
        self.restart_pane_runtime_with(pane_id, Self::spawn_pane_runtime_for_pane)
    }

    fn restart_pane_runtime_with<F>(
        &mut self,
        pane_id: rssh_core::PaneId,
        spawn: F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce(&mut Self, rssh_core::PaneId) -> Result<PaneRuntime, Box<dyn Error>>,
    {
        let is_active = pane_id == self.app_shell.active_pane_id();
        let runtime_cwd = if is_active {
            Some(pane_runtime_current_working_dir(
                &self.runtime,
                self.session_process_id,
            ))
        } else {
            self.pane_runtimes.get(&pane_id).map(|runtime| {
                pane_runtime_current_working_dir(&runtime.runtime, runtime.session_process_id)
            })
        };
        let Some(runtime_cwd) = runtime_cwd else {
            return Err(Box::new(io::Error::other(format!(
                "pane {} has no runtime owner",
                pane_id.get()
            ))));
        };

        if let Some(cwd) = runtime_cwd {
            self.sync_pane_current_working_dir_from_value(pane_id, Some(cwd));
        }

        let mut previous_runtime = if is_active {
            self.take_active_runtime()
        } else {
            self.pane_runtimes
                .remove(&pane_id)
                .expect("validated inactive pane runtime must exist")
        };
        let preserve_ssh_presentation =
            previous_runtime.transport == Some(PaneRuntimeTransportKind::NativeSsh);
        self.cancel_ssh_runtime(pane_id);
        let previous_size = previous_runtime.runtime.terminal().grid().size();
        let cleanup = previous_runtime.close();
        report_pane_pty_cleanup("pane restart PTY cleanup", &cleanup);

        if preserve_ssh_presentation {
            previous_runtime.runtime_generation = self.allocate_pane_runtime_generation();
            self.install_pane_runtime(pane_id, is_active, previous_runtime);
            self.clear_pane_restart_state(pane_id, is_active);
            self.apply_window_title();

            let mut runtime = spawn(self, pane_id)?;
            if runtime.runtime_generation == 0 {
                runtime.runtime_generation = self.allocate_pane_runtime_generation();
            }
            if runtime.transport.is_none() {
                runtime.transport = Some(PaneRuntimeTransportKind::NativeSsh);
            }
            let preserved_runtime = if is_active {
                self.take_active_runtime()
            } else {
                self.pane_runtimes
                    .remove(&pane_id)
                    .expect("SSH retry presentation owner must remain installed")
            };
            runtime.runtime = preserved_runtime.runtime;
            runtime.snapshot = preserved_runtime.snapshot;
            runtime.ui = preserved_runtime.ui;
            self.install_pane_runtime(pane_id, is_active, runtime);
            return Ok(());
        }

        let mut blank_runtime = self.new_inactive_pane_runtime();
        blank_runtime.runtime.resize(previous_size);
        blank_runtime.snapshot =
            terminal_runtime_snapshot(&blank_runtime.runtime, blank_runtime.ui.stable_viewport);
        blank_runtime.runtime_generation = self.allocate_pane_runtime_generation();
        self.install_pane_runtime(pane_id, is_active, blank_runtime);
        self.clear_pane_restart_state(pane_id, is_active);
        self.app_shell
            .reset_pane_runtime_projection(pane_id)
            .map_err(|error| {
                Box::new(io::Error::other(format!(
                    "failed to reset pane runtime projection: {error:?}"
                ))) as Box<dyn Error>
            })?;
        self.apply_window_title();

        let mut runtime = spawn(self, pane_id)?;
        if runtime.runtime_generation == 0 {
            runtime.runtime_generation = self.allocate_pane_runtime_generation();
        }
        self.install_pane_runtime(pane_id, is_active, runtime);
        Ok(())
    }

    fn install_pane_runtime(
        &mut self,
        pane_id: rssh_core::PaneId,
        is_active: bool,
        runtime: PaneRuntime,
    ) {
        if is_active {
            self.install_active_runtime(runtime);
        } else {
            self.pane_runtimes.insert(pane_id, runtime);
        }
    }

    fn clear_pane_restart_state(&mut self, pane_id: rssh_core::PaneId, is_active: bool) {
        if is_active {
            self.end_pointer_modes_for_pane_change();
            self.current_mouse_wheel_delta = None;
            self.last_mouse_info = None;
            self.deferred_wheel_context = None;
            self.ui_left_release_pending = false;
            self.pressed_pane_close_button = None;
            self.ime_preedit = None;
            self.dead_key_active = false;
            self.dead_key_text = None;
            self.selection = None;
            DEFAULT_WINDOW_TITLE.clone_into(&mut self.window_title);
        }
        self.pane_bell_counts.remove(&pane_id);
        self.visual_bell_started_at.remove(&pane_id);
        self.frame_needs_full_repaint = true;
        self.pending_frame_damage.clear();
    }

    #[expect(
        clippy::unused_self,
        reason = "method shape is retained for compatibility call-site consistency"
    )]
    fn allocate_pane_runtime_generation(&mut self) -> u64 {
        allocate_pane_runtime_token_from(&NEXT_PANE_RUNTIME_TOKEN)
    }

    fn pane_runtime_generation_matches(
        &self,
        pane_id: rssh_core::PaneId,
        runtime_generation: u64,
    ) -> bool {
        if pane_id == self.app_shell.active_pane_id() {
            return self.active_runtime_generation == runtime_generation;
        }
        self.pane_runtimes
            .get(&pane_id)
            .is_some_and(|runtime| runtime.runtime_generation == runtime_generation)
    }

    fn pane_runtime_transport_kind(
        &self,
        pane_id: rssh_core::PaneId,
    ) -> Option<PaneRuntimeTransportKind> {
        if pane_id == self.app_shell.active_pane_id() {
            return self.active_runtime_transport;
        }
        self.pane_runtimes
            .get(&pane_id)
            .and_then(|runtime| runtime.transport)
    }

    fn preserve_ssh_pane_after_transport_error(
        &mut self,
        pane_id: rssh_core::PaneId,
        error: &str,
    ) -> bool {
        eprintln!("SSH transport error: {error}");
        self.cancel_ssh_runtime(pane_id);
        if pane_id == self.app_shell.active_pane_id() {
            let cleanup = stop_pty_lifecycle(
                &mut self.session,
                &mut self.session_process_id,
                &mut self.session_tty_name,
                &mut self.writer,
                &mut self.reader_thread,
                &mut self.writer_thread,
            );
            report_pane_pty_cleanup("SSH transport error cleanup", &cleanup);
        } else if let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) {
            let cleanup = runtime.close();
            report_pane_pty_cleanup("inactive SSH transport error cleanup", &cleanup);
        }
        self.handle_ssh_state(pane_id, ConnectionState::Failed);
        self.clear_pane_inspection_if_invalid();
        false
    }

    fn handle_pane_runtime_read_error(&mut self, pane_id: rssh_core::PaneId, error: &str) -> bool {
        if self.pane_runtime_transport_kind(pane_id)
            == Some(PaneRuntimeTransportKind::NativeSsh)
        {
            return self.preserve_ssh_pane_after_transport_error(pane_id, error);
        }
        let close_window = if pane_id == self.app_shell.active_pane_id() {
            eprintln!("PTY read error: {error}");
            if let Err(error) = self.finish_active_pane_output() {
                eprintln!("active pane terminal finish after read error failed: {error}");
            }
            self.stop_active_runtime();
            true
        } else {
            self.finish_inactive_runtime_after_error(
                pane_id,
                "inactive pane read-error cleanup",
            );
            false
        };
        self.clear_pane_inspection_if_invalid();
        close_window
    }

    fn handle_pane_runtime_write_error(&mut self, pane_id: rssh_core::PaneId, error: &str) -> bool {
        if self.pane_runtime_transport_kind(pane_id)
            == Some(PaneRuntimeTransportKind::NativeSsh)
        {
            return self.preserve_ssh_pane_after_transport_error(pane_id, error);
        }
        let close_window = if pane_id == self.app_shell.active_pane_id() {
            eprintln!("PTY write error: {error}");
            if let Err(error) = self.finish_active_pane_output() {
                eprintln!("active pane terminal finish after write error failed: {error}");
            }
            self.stop_active_runtime();
            true
        } else {
            self.finish_inactive_runtime_after_error(
                pane_id,
                "inactive pane write-error cleanup",
            );
            false
        };
        self.clear_pane_inspection_if_invalid();
        close_window
    }

    fn finish_inactive_runtime_after_error(
        &mut self,
        pane_id: rssh_core::PaneId,
        cleanup_context: &str,
    ) {
        self.cancel_ssh_runtime(pane_id);
        let Some(mut runtime) = self.pane_runtimes.remove(&pane_id) else {
            return;
        };
        if let Err(error) = self.finish_inactive_pane_output(pane_id, &mut runtime) {
            eprintln!("inactive pane terminal finish failed: {error}");
        }
        let cleanup = runtime.close();
        report_pane_pty_cleanup(cleanup_context, &cleanup);
        if let Err(error) = self.dispatch_close_pane_action(pane_id) {
            eprintln!("inactive pane error close action failed: {error:?}");
            self.pane_runtimes.insert(pane_id, runtime);
        }
    }

    fn spawn_pane_runtime_for_active_pane(&mut self) -> Result<PaneRuntime, Box<dyn Error>> {
        self.spawn_pane_runtime_for_pane(self.app_shell.active_pane_id())
    }

    #[allow(clippy::too_many_lines)]
    fn spawn_pane_runtime_for_pane(
        &mut self,
        pane_id: rssh_core::PaneId,
    ) -> Result<PaneRuntime, Box<dyn Error>> {
        #[cfg(test)]
        if let Some(observer) = &self.pty_spawn_observer {
            observer.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        let Some(event_proxy) = self.event_proxy.clone() else {
            return Err(Box::new(io::Error::other(
                "window event proxy is not configured",
            )));
        };

        let runtime_generation = self.allocate_pane_runtime_generation();
        let (tab_id, launch) = self
            .app_shell
            .workspaces()
            .iter()
            .flat_map(rssh_core::app_shell::Workspace::tabs)
            .find_map(|tab| {
                tab.panes()
                    .iter()
                    .find(|pane| pane.id() == pane_id)
                    .map(|pane| (tab.id(), pane.launch().clone()))
            })
            .ok_or_else(|| {
                Box::new(io::Error::other(format!(
                    "pane {} has no launch metadata",
                    pane_id.get()
                ))) as Box<dyn Error>
            })?;

        if let PaneLaunchDomain::Ssh(ssh_launch) = launch.domain() {
            crate::stage7_attribution::audit_product_service_start(
                crate::stage7_attribution::ProductServiceEntry::NativeSsh,
            )?;
            return self.spawn_native_ssh_runtime(
                pane_id,
                ssh_launch,
                runtime_generation,
                event_proxy,
            );
        }

        crate::stage7_attribution::audit_product_service_start(
            crate::stage7_attribution::ProductServiceEntry::LocalPty,
        )?;

        let term_session_id =
            iterm_session_termid(self.app_window_id.get(), tab_id.get(), pane_id.get());
        let environment = self.pane_environment_variables();
        let command = pty_command_from_pane_launch_with_term_session_id(
            &launch,
            &self.term,
            &environment,
            self.default_cwd.as_deref(),
            &term_session_id,
        );

        let (pty_size, runtime) = self.prepare_pane_spawn_runtime(pane_id)?;
        self.metrics.start_spawn_timer();
        let session = PtySession::spawn(&command, pty_size)?;
        let size = runtime.terminal().grid().size();
        let worker_runtime = runtime.inner;
        let runtime = self.configured_pane_terminal_runtime(size);
        let capture = PaneCapturePolicy {
            host_stream: self.metrics.pty_linkage_enabled,
            visible_output: self.session_log.is_some(),
        };
        let (session_process_id, session_tty_name) =
            if let Some(v2_runtime) = self.runtime.worker_mut() {
                let (token, process_id, tty_name) = v2_runtime.adopt_additional_local_session(
                    pane_id,
                    session,
                    size,
                    worker_runtime,
                    capture,
                )?;
                v2_runtime.activate(token)?;
                (process_id, tty_name)
            } else {
                let wake_proxy = event_proxy.clone();
                let window_id = self.app_window_id;
                let notice_waker: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
                    let _ = wake_proxy
                        .send_event(WindowUserEvent::RuntimeWakeWindow { window_id });
                });
                let composition = self.runtime.composition();
                let (v2_runtime, process_id, tty_name) = composition.adopt_local_session(
                    PaneRuntimeRoute {
                        window: self.app_window_id,
                        pane: pane_id,
                    },
                    session,
                    size,
                    worker_runtime,
                    capture,
                    notice_waker,
                )?;
                self.runtime.install_worker(Some(v2_runtime));
                (process_id, tty_name)
            };
        let snapshot = terminal_runtime_snapshot(&runtime, PaneStableViewport::default());
        Ok(PaneRuntime {
            runtime,
            transport: Some(PaneRuntimeTransportKind::LocalPty),
            session: None,
            session_process_id,
            session_tty_name,
            writer: None,
            reader_thread: None,
            writer_thread: None,
            runtime_generation,
            snapshot,
            ui: PaneUiState::default(),
        })
    }

    fn prepare_pane_spawn_runtime(
        &self,
        pane_id: rssh_core::PaneId,
    ) -> Result<(PtySize, TerminalRuntime), Box<dyn Error>> {
        let size = self
            .pane_runtime_ref(pane_id)
            .map(|runtime| runtime.terminal().grid().size())
            .ok_or_else(|| {
                Box::new(io::Error::other(format!(
                    "pane {} has no runtime owner",
                    pane_id.get()
                ))) as Box<dyn Error>
            })?;
        let pty_size = PtySize::try_new(size.columns, size.rows)?;
        Ok((pty_size, self.configured_pane_terminal_runtime(size)))
    }

    #[cfg(test)]
    fn prepare_pane_spawn_runtime_for_test(
        &self,
        pane_id: rssh_core::PaneId,
    ) -> Result<(PtySize, TerminalRuntime), Box<dyn Error>> {
        self.prepare_pane_spawn_runtime(pane_id)
    }

    fn write_pty_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        let active = self.app_shell.active_pane_id();
        let active_is_local = self.active_runtime_transport
            == Some(PaneRuntimeTransportKind::LocalPty)
            || self
                .runtime
                .worker()
                .is_some_and(|runtime| runtime.contains_pane(active));
        if active_is_local {
            let started = Instant::now();
            self.runtime
                .worker_mut()
                .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?
                .submit_input_to_pane(active, bytes)?;
            self.metrics
                .record_input_write(bytes.len(), started.elapsed());
        } else {
            let worker_reports_completion = self.writer_thread.is_some();
            let Some(writer) = self.writer.as_mut() else {
                return Ok(());
            };

            let started = Instant::now();
            writer.write_all(bytes)?;
            writer.flush()?;
            if !worker_reports_completion {
                self.metrics
                    .record_input_write(bytes.len(), started.elapsed());
            }
        }
        if self.scroll_to_bottom_on_input && !bytes.is_empty() {
            self.set_scrollback_offset(0);
        }

        Ok(())
    }

    fn write_pty_bytes_to_pane(
        &mut self,
        pane_id: rssh_core::PaneId,
        bytes: &[u8],
    ) -> io::Result<()> {
        if pane_id == self.app_shell.active_pane_id() {
            return self.write_pty_bytes(bytes);
        }

        let scroll_to_bottom_on_input = self.scroll_to_bottom_on_input;
        let is_local = self
            .pane_runtimes
            .get(&pane_id)
            .is_some_and(|runtime| {
                runtime.transport == Some(PaneRuntimeTransportKind::LocalPty)
            })
            || self
                .runtime
                .worker()
                .is_some_and(|runtime| runtime.contains_pane(pane_id));
        if is_local {
            let started = Instant::now();
            self.runtime
                .worker_mut()
                .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?
                .submit_input_to_pane(pane_id, bytes)?;
            self.metrics
                .record_input_write(bytes.len(), started.elapsed());
            if scroll_to_bottom_on_input && !bytes.is_empty() {
                self.scroll_inactive_pane_to_bottom_after_input(pane_id);
            }
            return Ok(());
        }

        let Some(runtime) = self.interaction_state.host_state.pane_runtimes.get_mut(&pane_id) else {
            return Ok(());
        };
        let worker_reports_completion = runtime.writer_thread.is_some();
        let Some(writer) = runtime.writer.as_mut() else {
            return Ok(());
        };
        let started = Instant::now();
        writer.write_all(bytes)?;
        writer.flush()?;
        if !worker_reports_completion {
            self.interaction_state.host_state.metrics
                .record_input_write(bytes.len(), started.elapsed());
        }
        if scroll_to_bottom_on_input && !bytes.is_empty() {
            self.scroll_inactive_pane_to_bottom_after_input(pane_id);
        }
        Ok(())
    }

    fn scroll_inactive_pane_to_bottom_after_input(&mut self, pane_id: rssh_core::PaneId) {
        let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) else {
            return;
        };
        runtime.ui.stable_viewport.main_top = None;
        runtime
            .ui
            .stable_viewport
            .clamp_main(runtime.runtime.terminal());
        runtime
            .ui
            .refresh_search_match_cache(runtime.runtime.terminal());
        runtime.snapshot =
            terminal_runtime_snapshot(&runtime.runtime, runtime.ui.stable_viewport);
        self.metrics.record_snapshot_rebuild();
        self.frame_needs_full_repaint = true;
    }

    fn write_pty_bytes_to_pane_for_wheel(
        &mut self,
        pane_id: rssh_core::PaneId,
        bytes: &[u8],
    ) -> io::Result<()> {
        self.write_pty_bytes_to_pane(pane_id, bytes)
    }

    fn handle_pane_input_write_completed(&mut self, byte_count: usize, elapsed: Duration) {
        self.metrics.record_input_write(byte_count, elapsed);
    }

    fn metrics_snapshot(&self) -> WindowMetricsSnapshot {
        let direct_text = self.gpu_owners.active.as_ref().and_then(|gpu| gpu.direct_text_metrics());
        let gpu = self
            .gpu_owners.active
            .as_ref()
            .map_or_else(GpuPresentationMetrics::uninitialized, |gpu| {
                gpu.metrics().clone()
            });
        let text_backend = if self.gpu_owners.active.is_some() {
            "shaped-gpu-atlas"
        } else {
            "bitmap-emergency"
        };
        let mut snapshot = self
            .metrics
            .snapshot_with_gpu(&gpu, text_backend, direct_text);
        for quarantined in &self.gpu_owners.quarantined {
            snapshot.gpu_abandoned_lost_surfaces = snapshot.gpu_abandoned_lost_surfaces
                .saturating_add(quarantined.metrics().abandoned_lost_surfaces);
        }
        "v2-runtime-hub".clone_into(&mut snapshot.runtime_api);
        snapshot.runtime_live_threads = self.runtime.worker().map_or_else(
            || {
                usize::from(
                    self.reader_thread
                        .as_ref()
                        .is_some_and(|thread| !thread.is_finished()),
                ) + usize::from(
                    self.writer_thread
                        .as_ref()
                        .is_some_and(|thread| !thread.is_finished()),
                )
            },
            WindowPaneRuntime::live_thread_count_for_metrics,
        );
        snapshot
    }

    fn shutdown_gpu_for_window_close(&mut self) {
        for gpu in self.gpu_owners.active.iter_mut().chain(self.gpu_owners.quarantined.iter_mut()) {
            gpu.shutdown_for_window_close();
        }
    }

    fn shutdown_gpu_after_native_window_close(&mut self) {
        for gpu in self.gpu_owners.active.iter_mut().chain(self.gpu_owners.quarantined.iter_mut()) {
            gpu.shutdown_after_native_window_close();
        }
    }

    #[cfg(test)]
    fn metrics_report(&self) -> String {
        self.metrics_snapshot().report()
    }

    #[cfg(test)]
    fn metrics_json_report(&self) -> Result<String, serde_json::Error> {
        self.metrics_snapshot().json_report()
    }

}

// Keyboard and IME event translation has its own implementation boundary.
impl NativeWindowApp {
    fn handle_keyboard_input(&mut self, key: &winit::event::KeyEvent) -> io::Result<()> {
        let key_event_kind = KittyKeyEventKind::from_winit_key(key);
        self.handle_keyboard_input_event(
            &key.logical_key,
            key.physical_key,
            key.text.as_deref(),
            key.state,
            key_event_kind,
        )
    }

    fn handle_ime_commit(&mut self, text: &str) -> io::Result<()> {
        self.ime_preedit = None;
        self.dead_key_active = false;
        self.dead_key_text = None;
        if self.pane_inspection_input_barrier_active() {
            return Ok(());
        }
        if !self.use_ime || text.is_empty() {
            return Ok(());
        }

        self.hide_mouse_cursor_for_typing_if_needed();
        self.write_pty_bytes(text.as_bytes())
    }

    fn handle_ime_preedit(&mut self, text: &str) {
        if self.pane_inspection_input_barrier_active() {
            self.ime_preedit = None;
            return;
        }
        if !self.use_ime
            || self.ime_preedit_rendering != NativeImePreeditRendering::Builtin
            || text.is_empty()
        {
            self.ime_preedit = None;
            return;
        }

        self.ime_preedit = Some(text.to_owned());
    }

    fn effective_kitty_keyboard_flags(&self) -> u16 {
        let mut flags = self.runtime.kitty_keyboard_flags();
        if self.enable_csi_u_key_encoding {
            flags |= KITTY_KEYBOARD_DISAMBIGUATE;
        }
        flags
    }

    fn effective_keyboard_modifiers(
        &self,
        physical_key: PhysicalKey,
        text: Option<&str>,
    ) -> ModifiersState {
        let mut modifiers = self.modifiers;
        if self.treat_left_ctrlalt_as_altgr
            && text.is_some_and(|text| !text.is_empty())
            && modifiers.contains(ModifiersState::CONTROL)
            && modifiers.contains(ModifiersState::ALT)
            && !window_physical_key_is_modifier(physical_key)
        {
            modifiers.remove(ModifiersState::CONTROL | ModifiersState::ALT);
        }
        modifiers
    }

    fn update_alt_modifier_side_state(&mut self, physical_key: PhysicalKey, state: ElementState) {
        let pressed = state != ElementState::Released;
        match physical_key {
            PhysicalKey::Code(WinitKeyCode::AltLeft) => self.left_alt_pressed = pressed,
            PhysicalKey::Code(WinitKeyCode::AltRight) => self.right_alt_pressed = pressed,
            _ => {}
        }
    }

    fn terminal_keyboard_modifiers(
        &self,
        physical_key: PhysicalKey,
        text: Option<&str>,
        modifiers: ModifiersState,
    ) -> ModifiersState {
        let mut modifiers = modifiers;
        if native_alt_composed_key_should_remove_alt_modifier(
            physical_key,
            text,
            modifiers,
            self.left_alt_pressed,
            self.right_alt_pressed,
            self.send_composed_key_when_left_alt_is_pressed,
            self.send_composed_key_when_right_alt_is_pressed,
        ) {
            modifiers.remove(ModifiersState::ALT);
        }
        modifiers
    }

    #[allow(clippy::too_many_lines)]
    fn handle_keyboard_input_event(
        &mut self,
        logical_key: &Key,
        physical_key: PhysicalKey,
        text: Option<&str>,
        state: ElementState,
        key_event_kind: KittyKeyEventKind,
    ) -> io::Result<()> {
        self.update_alt_modifier_side_state(physical_key, state);
        self.record_debug_key_event(logical_key, physical_key, text, state, key_event_kind);
        let modifiers = self.effective_keyboard_modifiers(physical_key, text);

        if self.handle_pane_inspection_key_event(logical_key, state) {
            return Ok(());
        }

        // winit's macOS backend already asks the IME to interpret the event
        // before delivering KeyboardInput. An event delivered here was not
        // consumed (or was explicitly forwarded by the IME). Dropping it by
        // modifier mask loses Shift punctuation and native shortcuts.

        if state != ElementState::Pressed {
            self.handle_keyboard_release_event(
                logical_key,
                physical_key,
                text,
                modifiers,
                key_event_kind,
            )?;
            return Ok(());
        }

        if self.handle_ssh_prompt_key_event(logical_key, text) {
            return Ok(());
        }

        let is_dead_key = self.use_dead_keys && matches!(logical_key, Key::Dead(_));
        if !is_dead_key {
            self.dead_key_active = false;
            self.dead_key_text = None;
        }

        self.hide_mouse_cursor_for_typing_if_needed();

        if self.close_confirmation.is_some() {
            if self.handle_close_confirmation_key(logical_key, modifiers) {
                return Ok(());
            }
            return Ok(());
        }

        if self.confirmation.is_some() {
            if self.handle_confirmation_key(logical_key, modifiers) {
                return Ok(());
            }
            return Ok(());
        }

        if self.prompt_input_line.is_some() {
            if self.handle_prompt_input_line_key(logical_key, modifiers) {
                return Ok(());
            }
            return Ok(());
        }

        if self.input_selector.is_some() {
            if self.handle_input_selector_key(logical_key, modifiers) {
                return Ok(());
            }
            return Ok(());
        }

        if self.command_palette.is_some() {
            if self.handle_command_palette_logical_key(logical_key, modifiers) {
                return Ok(());
            }
            return Ok(());
        }

        if self.pane_select.is_some() {
            if self.handle_pane_select_key(logical_key, modifiers) {
                return Ok(());
            }
            return Ok(());
        }

        if self.tab_navigator.is_some() {
            if self.handle_tab_navigator_key(logical_key, modifiers) {
                return Ok(());
            }
            return Ok(());
        }

        if self.char_select.is_some() {
            if self.handle_char_select_key(logical_key, modifiers) {
                return Ok(());
            }
            return Ok(());
        }

        if self.active_ui.quick_select().is_some() {
            if self.handle_quick_select_logical_key(logical_key, modifiers) {
                return Ok(());
            }
            return Ok(());
        }

        match self.active_ui.copy_search_mode() {
            Some(WindowCopySearchMode::Search) => {
                self.handle_search_key(logical_key, modifiers);
                return Ok(());
            }
            Some(WindowCopySearchMode::Copy) => {
                if self.handle_copy_mode_key(logical_key, modifiers) {
                    return Ok(());
                }
                return Ok(());
            }
            None => {}
        }

        if self.handle_debug_overlay_key(logical_key, modifiers) {
            return Ok(());
        }

        if self.handle_active_key_table_assignment_key_press(logical_key, physical_key, modifiers) {
            return Ok(());
        }

        if self.handle_unmatched_active_key_table_key_press() {
            return Ok(());
        }

        let now = Instant::now();
        if self.handle_leader_key_press(logical_key, physical_key, modifiers, now) {
            return Ok(());
        }

        if self.handle_user_key_assignment_key_press(logical_key, physical_key, modifiers) {
            return Ok(());
        }

        let default_assignment_disabled = self.default_assignment_disabled_for_key_with_preference(
            logical_key,
            Some(physical_key),
            modifiers,
            self.key_map_preference,
        );

        if !default_assignment_disabled && window_clear_scrollback_shortcut(logical_key, modifiers)
        {
            self.clear_scrollback();
            return Ok(());
        }

        if self.handle_toggle_full_screen_shortcut(logical_key, modifiers) {
            return Ok(());
        }

        if self.handle_hide_shortcut_event(logical_key, physical_key, modifiers) {
            return Ok(());
        }

        if self.handle_application_hide_shortcut_event(logical_key, physical_key, modifiers) {
            return Ok(());
        }

        if self.handle_font_size_shortcut(logical_key, modifiers) {
            return Ok(());
        }

        if self.handle_show_debug_overlay_shortcut_event(logical_key, physical_key, modifiers) {
            return Ok(());
        }

        if self.handle_char_select_shortcut_event(logical_key, physical_key, modifiers) {
            return Ok(());
        }

        if self.handle_reload_configuration_shortcut_event(logical_key, physical_key, modifiers) {
            return Ok(());
        }

        if self.handle_browser_tab_shortcut_event(
            logical_key,
            physical_key,
            modifiers,
            default_assignment_disabled,
        ) {
            return Ok(());
        }

        if self.handle_default_close_current_tab_shortcut_event(
            logical_key,
            physical_key,
            modifiers,
            default_assignment_disabled,
        ) {
            return Ok(());
        }

        if let Some(action) =
            self.app_shell_action_for_key_event(logical_key, physical_key, modifiers)
        {
            if let Err(error) = self.dispatch_app_action(action) {
                eprintln!("app shell action error: {error:?}");
            }
            return Ok(());
        }

        if !default_assignment_disabled && Self::command_palette_shortcut(logical_key, modifiers) {
            self.enter_command_palette_mode();
            return Ok(());
        }

        if !default_assignment_disabled && window_quick_select_shortcut(logical_key, modifiers) {
            self.enter_quick_select_mode();
            return Ok(());
        }

        if !default_assignment_disabled && window_copy_mode_shortcut(logical_key, modifiers) {
            self.enter_copy_mode();
            return Ok(());
        }

        if !default_assignment_disabled && window_search_shortcut(logical_key, modifiers) {
            self.enter_search_mode_with_query(&WindowSearchCommandQuery::Pattern {
                pattern: String::new(),
                match_type: WindowSearchMatchType::CaseSensitive,
            });
            return Ok(());
        }

        if !default_assignment_disabled
            && let Some(destination) = window_copy_destination_for_shortcut(logical_key, modifiers)
        {
            self.copy_selection_to(destination);
            return Ok(());
        }

        if !default_assignment_disabled
            && let Some(source) = window_paste_source_for_shortcut(logical_key, modifiers)
        {
            self.handle_window_paste_from(source)?;
            return Ok(());
        }

        if self.handle_scrollback_shortcut(logical_key, modifiers) {
            return Ok(());
        }

        if is_dead_key {
            self.dead_key_active = true;
            self.dead_key_text = match logical_key {
                Key::Dead(Some(dead_key)) => Some(dead_key.to_string()),
                Key::Dead(None) => Some(String::new()),
                _ => None,
            };
            return Ok(());
        }

        let encoded_key =
            swap_backspace_delete_key_if_needed(logical_key, self.swap_backspace_and_delete);
        let terminal_modifiers = self.terminal_keyboard_modifiers(physical_key, text, modifiers);
        let bytes = if self.runtime.win32_input_mode() {
            encode_win32_window_key(&encoded_key, physical_key, text, terminal_modifiers, true)
        } else {
            encode_window_key_with_kitty_event(
                &encoded_key,
                physical_key,
                text,
                terminal_modifiers,
                self.runtime.application_cursor_keys(),
                self.runtime.application_keypad(),
                self.effective_kitty_keyboard_flags(),
                self.runtime.modify_other_keys(),
                key_event_kind,
            )
        };
        if !bytes.is_empty() {
            self.write_pty_bytes(&bytes)?;
        }

        Ok(())
    }

    fn handle_keyboard_release_event(
        &mut self,
        logical_key: &Key,
        physical_key: PhysicalKey,
        text: Option<&str>,
        modifiers: ModifiersState,
        key_event_kind: KittyKeyEventKind,
    ) -> io::Result<()> {
        if key_event_kind != KittyKeyEventKind::Release {
            return Ok(());
        }
        let encoded_key =
            swap_backspace_delete_key_if_needed(logical_key, self.swap_backspace_and_delete);
        let terminal_modifiers =
            self.terminal_keyboard_modifiers(physical_key, text, modifiers);
        let bytes = if self.runtime.win32_input_mode() {
            encode_win32_window_key(
                &encoded_key,
                physical_key,
                text,
                terminal_modifiers,
                false,
            )
        } else {
            encode_window_key_with_kitty_event(
                &encoded_key,
                physical_key,
                text,
                terminal_modifiers,
                self.runtime.application_cursor_keys(),
                self.runtime.application_keypad(),
                self.effective_kitty_keyboard_flags(),
                self.runtime.modify_other_keys(),
                key_event_kind,
            )
        };
        if !bytes.is_empty() {
            self.write_pty_bytes(&bytes)?;
        }
        Ok(())
    }

}

impl NativeWindowApp {
    fn record_debug_key_event(
        &mut self,
        logical_key: &Key,
        physical_key: PhysicalKey,
        text: Option<&str>,
        state: ElementState,
        key_event_kind: KittyKeyEventKind,
    ) {
        if !self.debug_key_events {
            return;
        }

        let log = format!(
            "INFO key_event KeyEvent {{ key: {logical_key:?}, physical_key: {physical_key:?}, modifiers: {:?}, state: {state:?}, kind: {key_event_kind:?}, text: {text:?} }}",
            self.modifiers
        );
        eprintln!("{log}");
        self.debug_key_event_logs.push(log);
    }

    fn enter_search_mode(&mut self) {
        self.cancel_pane_inspection();
        let initial_query = self
            .ordinary_selected_text()
            .map(|text| single_line_search_query_from_selection(&text))
            .filter(|query| !query.is_empty());
        if self.command_palette.is_some() {
            self.command_palette = None;
        }
        self.pane_select = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;
        let initial_copy_mode = self.initial_copy_mode();
        self.active_ui
            .enter_search(initial_copy_mode, WindowSearch::default());
        if let Some(query) = initial_query {
            self.update_search_query(&query);
        } else {
            self.apply_window_title();
        }
    }

    fn enter_search_mode_with_query(&mut self, search_query: &WindowSearchCommandQuery) {
        match search_query {
            WindowSearchCommandQuery::Pattern {
                pattern,
                match_type,
            } => {
                self.enter_search_mode();
                self.update_search_query_with_type(
                    pattern,
                    self.initial_search_direction(),
                    *match_type,
                );
            }
            WindowSearchCommandQuery::CurrentSelectionOrEmptyString => self.enter_search_mode(),
        }
    }

    fn clear_scrollback(&mut self) {
        self.active_ui.stable_viewport = PaneStableViewport::default();
        if let Err(error) = self.handle_active_pane_output(b"\x1b[3J") {
            eprintln!("clear scrollback command failed: {error}");
        }
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn clear_scrollback_and_viewport(&mut self) {
        self.active_ui.stable_viewport = PaneStableViewport::default();
        self.retire_active_terminal_identity_state();
        let damage = self.runtime.erase_scrollback_and_viewport();
        self.reconcile_active_terminal_mutation();
        self.metrics.record_damage(&damage);
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn reset_terminal(&mut self) -> io::Result<()> {
        self.retire_active_terminal_identity_state();
        self.handle_active_pane_output(b"\x1bc")
    }

    fn copy_mode_status(copy_mode: &WindowCopyMode) -> String {
        match copy_mode.selection_mode {
            WindowCopySelectionMode::Cell => "Copy Mode: Cell".to_owned(),
            WindowCopySelectionMode::Word => "Copy Mode: Word".to_owned(),
            WindowCopySelectionMode::Block => "Copy Mode: Block".to_owned(),
            WindowCopySelectionMode::Line => "Copy Mode: Line".to_owned(),
            WindowCopySelectionMode::SemanticZone => "Copy Mode: SemanticZone".to_owned(),
            WindowCopySelectionMode::None => "Copy Mode".to_owned(),
        }
    }

    fn initial_copy_mode(&self) -> WindowCopyMode {
        let size = self.runtime.terminal().grid().size();
        let (row, column) = self.runtime.terminal().cursor();
        let terminal = self.runtime.terminal();
        let dimensions = terminal.stable_dimensions();
        let row = if size.rows == 0 {
            0
        } else {
            row.min(size.rows.saturating_sub(1))
        };
        let column = if size.columns == 0 {
            0
        } else {
            column.min(size.columns.saturating_sub(1))
        };
        let source_row = dimensions
            .physical_top
            .saturating_add(StableRowIndex::try_from(row).unwrap_or(StableRowIndex::MAX));
        WindowCopyMode {
            cursor: SelectionCell { row, column },
            source_cursor: SelectionSourceCell {
                domain: dimensions.domain,
                row: source_row,
                column: usize::from(column),
            },
            pending_jump: None,
            last_jump: None,
            search_direction: None,
            selection_mode: WindowCopySelectionMode::None,
            anchor: None,
            source_anchor: None,
        }
    }

    fn enter_copy_mode(&mut self) {
        self.cancel_pane_inspection();
        self.command_palette = None;
        self.pane_select = None;
        self.tab_navigator = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;
        self.close_confirmation = None;

        let size = self.runtime.terminal().grid().size();
        if size.columns == 0 || size.rows == 0 {
            self.active_ui.exit_overlay();
            self.selection = None;
            self.refresh_snapshot();
            self.apply_window_title();
            return;
        }

        let initial_copy_mode = self.initial_copy_mode();
        self.active_ui.enter_copy_mode(initial_copy_mode);
        self.selection = None;
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn exit_copy_mode(&mut self) {
        self.active_ui.exit_overlay();
        self.selection = None;
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn scroll_to_bottom_and_exit_copy_mode(&mut self) {
        self.active_ui.stable_viewport = PaneStableViewport::default();
        self.exit_copy_mode();
    }

    #[cfg(test)]
    fn set_copy_mode_selection_mode(&mut self, mode: WindowCopySelectionMode) -> bool {
        let Some(copy_mode) = self.active_ui.copy_mode_mut() else {
            return false;
        };

        copy_mode.selection_mode = mode;
        match mode {
            WindowCopySelectionMode::None
            | WindowCopySelectionMode::Word
            | WindowCopySelectionMode::Line
            | WindowCopySelectionMode::SemanticZone => {
                copy_mode.anchor = None;
                copy_mode.source_anchor = None;
            }
            WindowCopySelectionMode::Cell | WindowCopySelectionMode::Block => {
                copy_mode.anchor = Some(copy_mode.cursor);
                copy_mode.source_anchor = Some(copy_mode.source_cursor);
            }
        }
        self.apply_copy_mode_selection();
        true
    }

    fn perform_copy_mode_assignment(&mut self, assignment: WindowCopyModeAssignment) -> bool {
        let handled = Self::perform_copy_mode_assignment_for_owner(
            self.runtime.terminal(),
            &mut self.interaction_state.active_ui,
            assignment,
        );
        if !handled {
            return false;
        }
        if assignment == WindowCopyModeAssignment::Close {
            self.selection = None;
            self.selecting = false;
            self.active_mouse_button = None;
            self.last_left_click = None;
            self.last_mouse_assignment_click = None;
        } else {
            self.update_selection_projection();
        }
        self.refresh_snapshot();
        self.apply_window_title();
        true
    }

    #[allow(clippy::too_many_lines)]
    fn handle_copy_mode_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.active_ui.copy_search_mode() == Some(WindowCopySearchMode::Search) {
            return self.handle_search_key(key, modifiers);
        }

        let pending_jump = {
            let Some(copy_mode) = self.active_ui.copy_mode_mut() else {
                return false;
            };
            copy_mode.pending_jump.take()
        };
        if let Some(pending_jump) = pending_jump {
            return self.complete_copy_mode_jump(key, modifiers, pending_jump);
        }

        let in_copy_mode_search = self
            .active_ui
            .copy_mode()
            .is_some_and(|copy_mode| copy_mode.search_direction.is_some());

        if in_copy_mode_search && Self::copy_mode_search_key_table_handles_key(key, modifiers) {
            return self.handle_copy_mode_search_key(key, modifiers);
        }

        if Self::command_palette_shortcut(key, modifiers) {
            self.enter_command_palette_mode();
            return true;
        }

        if self.handle_copy_mode_app_shell_fallback(key, modifiers) {
            return true;
        }

        if in_copy_mode_search {
            return self.handle_copy_mode_search_key(key, modifiers);
        }

        let Some(copy_mode) = self.active_ui.copy_mode_mut() else {
            return false;
        };

        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.scroll_to_bottom_and_exit_copy_mode();
                true
            }
            Key::Character("\u{1b}") if modifiers.is_empty() => {
                self.scroll_to_bottom_and_exit_copy_mode();
                true
            }
            Key::Character(character) if character.eq_ignore_ascii_case("q") => {
                self.scroll_to_bottom_and_exit_copy_mode();
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("c") =>
            {
                self.scroll_to_bottom_and_exit_copy_mode();
                true
            }
            Key::Named(NamedKey::Enter) => {
                self.move_copy_mode_cursor_by_lines(1);
                self.move_copy_mode_to_line_start()
            }
            Key::Character("\r") if modifiers.is_empty() => {
                self.move_copy_mode_cursor_by_lines(1);
                self.move_copy_mode_to_line_start()
            }
            Key::Named(NamedKey::PageUp) => {
                let page = isize::try_from(self.runtime.terminal().grid().size().rows).unwrap_or(0);
                self.move_copy_mode_cursor_by_lines(-page)
            }
            Key::Named(NamedKey::PageDown) => {
                let page = isize::try_from(self.runtime.terminal().grid().size().rows).unwrap_or(0);
                self.move_copy_mode_cursor_by_lines(page)
            }
            Key::Character(character) if character.eq_ignore_ascii_case("y") => {
                self.copy_selection_to_clipboard_and_primary_selection();
                self.scroll_to_bottom_and_exit_copy_mode();
                true
            }
            Key::Character(character)
                if (modifiers.shift_key() && character.eq_ignore_ascii_case("v"))
                    || (modifiers.is_empty() && character == "V") =>
            {
                copy_mode.selection_mode = WindowCopySelectionMode::Line;
                copy_mode.anchor = None;
                copy_mode.source_anchor = None;
                self.apply_copy_mode_selection();
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("v") =>
            {
                copy_mode.selection_mode = WindowCopySelectionMode::Block;
                copy_mode.anchor = Some(copy_mode.cursor);
                copy_mode.source_anchor = Some(copy_mode.source_cursor);
                self.apply_copy_mode_selection();
                true
            }
            Key::Character(" ") if modifiers.is_empty() => {
                copy_mode.selection_mode = WindowCopySelectionMode::Cell;
                copy_mode.anchor = Some(copy_mode.cursor);
                copy_mode.source_anchor = Some(copy_mode.source_cursor);
                self.apply_copy_mode_selection();
                true
            }
            Key::Character(character) if character.eq_ignore_ascii_case("v") => {
                copy_mode.selection_mode = WindowCopySelectionMode::Cell;
                copy_mode.anchor = Some(copy_mode.cursor);
                copy_mode.source_anchor = Some(copy_mode.source_cursor);
                self.apply_copy_mode_selection();
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("g") =>
            {
                self.scroll_to_bottom_and_exit_copy_mode();
                true
            }
            Key::Character("/") if modifiers.is_empty() => {
                self.start_copy_mode_search(SearchDirection::Next)
            }
            Key::Character("?") if modifiers.is_empty() || modifiers == ModifiersState::SHIFT => {
                self.start_copy_mode_search(SearchDirection::Previous)
            }
            Key::Character("o") if modifiers.is_empty() => {
                self.move_copy_mode_to_selection_other_end()
            }
            Key::Character("O") if modifiers.is_empty() || modifiers == ModifiersState::SHIFT => {
                self.move_copy_mode_to_selection_other_end_horiz()
            }
            Key::Character(";") if modifiers.is_empty() => self.repeat_copy_mode_jump(false),
            Key::Character(",") if modifiers.is_empty() => self.repeat_copy_mode_jump(true),
            Key::Character("f") if modifiers.is_empty() => self.start_copy_mode_jump(true, false),
            Key::Character("t") if modifiers.is_empty() => self.start_copy_mode_jump(true, true),
            Key::Character("F") if modifiers.is_empty() || modifiers == ModifiersState::SHIFT => {
                self.start_copy_mode_jump(false, false)
            }
            Key::Character("T") if modifiers.is_empty() || modifiers == ModifiersState::SHIFT => {
                self.start_copy_mode_jump(false, true)
            }
            Key::Named(NamedKey::Tab) if modifiers == ModifiersState::SHIFT => {
                self.move_copy_mode_by_word(WindowCopyWordMovement::Backward)
            }
            Key::Named(NamedKey::Tab) if modifiers.is_empty() => {
                self.move_copy_mode_by_word(WindowCopyWordMovement::Forward)
            }
            Key::Named(NamedKey::ArrowLeft) if modifiers == ModifiersState::ALT => {
                self.move_copy_mode_by_word(WindowCopyWordMovement::Backward)
            }
            Key::Named(NamedKey::ArrowRight) if modifiers == ModifiersState::ALT => {
                self.move_copy_mode_by_word(WindowCopyWordMovement::Forward)
            }
            Key::Character(character)
                if !modifiers.control_key()
                    && character.eq_ignore_ascii_case("b")
                    && (modifiers.is_empty() || modifiers == ModifiersState::ALT) =>
            {
                self.move_copy_mode_by_word(WindowCopyWordMovement::Backward)
            }
            Key::Character(character)
                if !modifiers.control_key()
                    && character.eq_ignore_ascii_case("w")
                    && modifiers.is_empty() =>
            {
                self.move_copy_mode_by_word(WindowCopyWordMovement::Forward)
            }
            Key::Character(character)
                if !modifiers.control_key()
                    && character.eq_ignore_ascii_case("f")
                    && modifiers == ModifiersState::ALT =>
            {
                self.move_copy_mode_by_word(WindowCopyWordMovement::Forward)
            }
            Key::Character(character)
                if !modifiers.control_key()
                    && character.eq_ignore_ascii_case("e")
                    && modifiers.is_empty() =>
            {
                self.move_copy_mode_by_word(WindowCopyWordMovement::End)
            }
            Key::Character("m") if modifiers == ModifiersState::ALT => {
                self.move_copy_mode_to_line_content_start()
            }
            Key::Character(character) if modifiers.alt_key() => {
                if let Some(semantic_type) = copy_mode_semantic_zone_type_for_key(character) {
                    let delta = if modifiers.shift_key() { 1 } else { -1 };
                    self.move_copy_mode_by_semantic_zone(delta, Some(semantic_type))
                } else {
                    false
                }
            }
            Key::Character(character)
                if modifiers.shift_key() && character.eq_ignore_ascii_case("z") =>
            {
                self.move_copy_mode_by_semantic_zone(1, None)
            }
            Key::Character("z") => self.move_copy_mode_by_semantic_zone(-1, None),
            Key::Character("G") if modifiers.is_empty() || modifiers == ModifiersState::SHIFT => {
                self.move_copy_mode_to_scrollback_bottom()
            }
            Key::Character("H") if modifiers.is_empty() || modifiers == ModifiersState::SHIFT => {
                self.move_copy_mode_to_viewport_top()
            }
            Key::Character("M") if modifiers.is_empty() || modifiers == ModifiersState::SHIFT => {
                self.move_copy_mode_to_viewport_middle()
            }
            Key::Character("L") if modifiers.is_empty() || modifiers == ModifiersState::SHIFT => {
                self.move_copy_mode_to_viewport_bottom()
            }
            Key::Named(NamedKey::ArrowLeft) => self.move_copy_mode_cursor(0, -1),
            Key::Named(NamedKey::ArrowRight) => self.move_copy_mode_cursor(0, 1),
            Key::Named(NamedKey::ArrowUp) => self.move_copy_mode_cursor(-1, 0),
            Key::Named(NamedKey::ArrowDown) => self.move_copy_mode_cursor(1, 0),
            Key::Character(character) if character.eq_ignore_ascii_case("h") => {
                self.move_copy_mode_cursor(0, -1)
            }
            Key::Character(character) if character.eq_ignore_ascii_case("l") => {
                self.move_copy_mode_cursor(0, 1)
            }
            Key::Character(character) if character.eq_ignore_ascii_case("k") => {
                self.move_copy_mode_cursor(-1, 0)
            }
            Key::Character(character) if character.eq_ignore_ascii_case("j") => {
                self.move_copy_mode_cursor(1, 0)
            }
            Key::Named(NamedKey::Home) | Key::Character("0") => self.move_copy_mode_to_line_start(),
            Key::Character("^") => self.move_copy_mode_to_line_content_start(),
            Key::Named(NamedKey::End) | Key::Character("$") => {
                self.move_copy_mode_to_line_content_end()
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("b") =>
            {
                let page = isize::try_from(self.runtime.terminal().grid().size().rows).unwrap_or(0);
                self.move_copy_mode_cursor_by_lines(-page)
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("f") =>
            {
                let page = isize::try_from(self.runtime.terminal().grid().size().rows).unwrap_or(0);
                self.move_copy_mode_cursor_by_lines(page)
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("u") =>
            {
                let half_page =
                    isize::try_from(self.runtime.terminal().grid().size().rows / 2).unwrap_or(0);
                self.move_copy_mode_cursor_by_lines(-half_page)
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("d") =>
            {
                let half_page =
                    isize::try_from(self.runtime.terminal().grid().size().rows / 2).unwrap_or(0);
                self.move_copy_mode_cursor_by_lines(half_page)
            }
            Key::Character("g") => self.move_copy_mode_to_scrollback_top(),
            _ => false,
        }
    }

    fn handle_copy_mode_app_shell_fallback(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
    ) -> bool {
        let Some(action) = self.app_shell_action_for_key(key, modifiers) else {
            return false;
        };

        if let Err(error) = self.dispatch_app_action(action) {
            eprintln!("copy-mode fallback app shell action error: {error:?}");
        }
        true
    }

    fn apply_copy_mode_selection(&mut self) {
        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return;
        };

        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 || size.columns == 0 {
            return;
        }

        let dimensions = self.runtime.terminal().stable_dimensions();
        let viewport_top = self.current_viewport_stable_top();
        self.selection = copy_mode_source_selection(
            copy_mode,
            self.runtime.terminal(),
            &self.selection_word_boundary,
        )
        .and_then(|selection| selection.viewport_selection(dimensions.domain, viewport_top, size));
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn move_copy_mode_cursor(&mut self, row_delta: isize, col_delta: isize) -> bool {
        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 || size.columns == 0 {
            return false;
        }
        let retained = self.runtime.terminal().retained_stable_range();
        if retained.start >= retained.end {
            return false;
        }

        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };

        let min_row = retained.start;
        let max_row = retained.end.saturating_sub(1);
        let max_column = usize::from(size.columns.saturating_sub(1));
        let next_row = copy_mode
            .source_cursor
            .row
            .saturating_add(row_delta)
            .clamp(min_row, max_row);
        let next_column = apply_isize_delta_to_usize(
            copy_mode.source_cursor.column.min(max_column),
            col_delta,
            max_column,
        );

        if next_row == copy_mode.source_cursor.row && next_column == copy_mode.source_cursor.column
        {
            return false;
        }

        self.set_copy_mode_cursor_for_source_position(next_row, next_column)
    }

    fn set_copy_mode_cursor(&mut self, row: u16, column: u16) -> bool {
        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 || size.columns == 0 {
            return false;
        }
        let dimensions = self.runtime.terminal().stable_dimensions();
        let viewport_top = self.current_viewport_stable_top();
        let Some(copy_mode) = self.active_ui.copy_mode_mut() else {
            return false;
        };

        let target = SelectionCell {
            row: row.min(size.rows.saturating_sub(1)),
            column: column.min(size.columns.saturating_sub(1)),
        };

        if target == copy_mode.cursor {
            return false;
        }

        copy_mode.cursor = target;
        copy_mode.source_cursor = SelectionSourceCell {
            domain: dimensions.domain,
            row: viewport_top.saturating_add(
                StableRowIndex::try_from(target.row).unwrap_or(StableRowIndex::MAX),
            ),
            column: usize::from(target.column),
        };
        self.apply_copy_mode_selection();
        true
    }

    fn set_copy_mode_cursor_for_source_position(
        &mut self,
        source_row: StableRowIndex,
        source_column: usize,
    ) -> bool {
        let domain = self.runtime.terminal().stable_dimensions().domain;
        let source_anchor = self
            .active_ui
            .copy_mode()
            .and_then(|copy_mode| copy_mode.source_anchor);
        self.set_copy_mode_cursor_and_anchor_for_source_position(
            SelectionSourceCell {
                domain,
                row: source_row,
                column: source_column,
            },
            source_anchor,
        )
    }

    fn set_copy_mode_cursor_and_anchor_for_source_position(
        &mut self,
        source_cursor: SelectionSourceCell,
        source_anchor: Option<SelectionSourceCell>,
    ) -> bool {
        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 || size.columns == 0 {
            return false;
        }

        let terminal = self.runtime.terminal();
        let dimensions = terminal.stable_dimensions();
        if source_cursor.domain != dimensions.domain {
            return false;
        }
        let history_len = terminal.scrollback().len();
        let current_offset = self.current_scrollback_offset().min(history_len);
        let Some(source_history_row) = terminal.stable_row_to_history_index(source_cursor.row)
        else {
            return false;
        };
        let (target_offset, target_viewport_top, target) =
            if dimensions.domain == TerminalScreenDomain::Alternate {
                let Some(target) = copy_mode_cell_for_source_position(
                    source_history_row,
                    source_cursor.column,
                    0,
                    size,
                ) else {
                    return false;
                };
                (0, 0, target)
            } else {
                let Some((target_offset, target)) = copy_mode_viewport_cell_for_source_position(
                    source_history_row,
                    source_cursor.column,
                    current_offset,
                    history_len,
                    size,
                ) else {
                    return false;
                };
                (
                    target_offset,
                    copy_mode_viewport_top(history_len, target_offset),
                    target,
                )
            };
        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };

        if target_offset == current_offset
            && target == copy_mode.cursor
            && source_cursor == copy_mode.source_cursor
            && source_anchor == copy_mode.source_anchor
        {
            return false;
        }

        self.interaction_state.active_ui
            .stable_viewport
            .set_scrollback_offset(self.runtime.terminal(), target_offset);
        let anchor_cell = source_anchor.and_then(|anchor| {
            let source_row = self
                .runtime
                .terminal()
                .stable_row_to_history_index(anchor.row)?;
            copy_mode_cell_for_source_position(
                source_row,
                anchor.column,
                target_viewport_top,
                size,
            )
        });
        if let Some(copy_mode) = self.active_ui.copy_mode_mut() {
            copy_mode.cursor = target;
            copy_mode.source_cursor = source_cursor;
            copy_mode.source_anchor = source_anchor;
            copy_mode.anchor = anchor_cell;
        }
        self.apply_copy_mode_selection();
        true
    }

    fn move_copy_mode_to_selection_other_end(&mut self) -> bool {
        let Some((source_cursor, source_anchor)) =
            self.active_ui.copy_mode().and_then(|copy_mode| {
                copy_mode
                    .source_anchor
                    .map(|anchor| (copy_mode.source_cursor, anchor))
            })
        else {
            return false;
        };

        self.set_copy_mode_cursor_and_anchor_for_source_position(source_anchor, Some(source_cursor))
    }

    fn move_copy_mode_to_selection_other_end_horiz(&mut self) -> bool {
        let Some((source_cursor, source_anchor)) =
            self.active_ui.copy_mode().and_then(|copy_mode| {
                copy_mode
                    .source_anchor
                    .map(|anchor| (copy_mode.source_cursor, anchor))
            })
        else {
            return false;
        };

        self.set_copy_mode_cursor_and_anchor_for_source_position(
            SelectionSourceCell {
                domain: source_cursor.domain,
                row: source_cursor.row,
                column: source_anchor.column,
            },
            Some(SelectionSourceCell {
                domain: source_anchor.domain,
                row: source_anchor.row,
                column: source_cursor.column,
            }),
        )
    }

    fn move_copy_mode_cursor_by_lines(&mut self, line_delta: isize) -> bool {
        self.move_copy_mode_cursor(line_delta, 0)
    }

    fn move_copy_mode_by_semantic_zone(
        &mut self,
        delta: isize,
        semantic_type: Option<SemanticType>,
    ) -> bool {
        if delta == 0 {
            return false;
        }

        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };
        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 || size.columns == 0 {
            return false;
        }

        let terminal = self.runtime.terminal();
        let cursor_y = copy_mode.source_cursor.row;
        let cursor_x = copy_mode.source_cursor.column;
        let zones = terminal.stable_semantic_zones();
        let mut index = match zones.binary_search_by(|zone| match zone.start_y.cmp(&cursor_y) {
            std::cmp::Ordering::Equal => zone.start_x.cmp(&cursor_x),
            ordering => ordering,
        }) {
            Ok(index) | Err(index) => index,
        };

        let step = if delta > 0 { 1 } else { -1 };
        let mut remaining = delta;
        while remaining != 0 {
            index = if step > 0 {
                let Some(next) = index.checked_add(1) else {
                    return false;
                };
                next
            } else {
                let Some(previous) = index.checked_sub(1) else {
                    return false;
                };
                previous
            };

            let Some(zone) = zones.get(index).cloned() else {
                return false;
            };
            if semantic_type.is_some_and(|semantic_type| zone.semantic_type != semantic_type) {
                continue;
            }

            remaining -= step;
            if remaining == 0 {
                return self.set_copy_mode_cursor_for_source_position(zone.start_y, zone.start_x);
            }
        }

        false
    }

    fn move_copy_mode_by_word(&mut self, movement: WindowCopyWordMovement) -> bool {
        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };
        let Some(target) =
            copy_mode_word_target(self.runtime.terminal(), copy_mode.source_cursor, movement)
        else {
            return false;
        };

        self.set_copy_mode_cursor_for_source_position(target.row, target.column)
    }

    fn start_copy_mode_jump(&mut self, forward: bool, prev_char: bool) -> bool {
        let Some(copy_mode) = self.active_ui.copy_mode_mut() else {
            return false;
        };

        copy_mode.pending_jump = Some(WindowCopyPendingJump { forward, prev_char });
        true
    }

    fn complete_copy_mode_jump(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
        pending_jump: WindowCopyPendingJump,
    ) -> bool {
        let target = match key.as_ref() {
            Key::Character(character)
                if modifiers.is_empty() || modifiers == ModifiersState::SHIFT =>
            {
                character.chars().next()
            }
            _ => None,
        };

        let Some(target) = target else {
            return true;
        };

        let jump = WindowCopyJump {
            forward: pending_jump.forward,
            prev_char: pending_jump.prev_char,
            target,
        };
        if let Some(copy_mode) = self.active_ui.copy_mode_mut() {
            copy_mode.last_jump = Some(jump);
        }
        self.perform_copy_mode_jump(jump, false)
    }

    fn repeat_copy_mode_jump(&mut self, reverse: bool) -> bool {
        let Some(mut jump) = self
            .active_ui
            .copy_mode()
            .and_then(|copy_mode| copy_mode.last_jump)
        else {
            return false;
        };

        if reverse {
            jump.forward = !jump.forward;
        }
        self.perform_copy_mode_jump(jump, true)
    }

    fn perform_copy_mode_jump(&mut self, jump: WindowCopyJump, repeat: bool) -> bool {
        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };
        let cursor = copy_mode.source_cursor;
        let Some(target) = copy_mode_jump_target(self.runtime.terminal(), cursor, jump, repeat)
        else {
            return false;
        };

        self.set_copy_mode_cursor_for_source_position(target.row, target.column)
    }

    fn start_copy_mode_search(&mut self, direction: SearchDirection) -> bool {
        let Some(copy_mode) = self.active_ui.copy_mode_mut() else {
            return false;
        };

        copy_mode.search_direction = Some(direction);
        let initial_copy_mode = self.initial_copy_mode();
        self.active_ui
            .enter_search(initial_copy_mode, WindowSearch::default());
        self.selection = None;
        self.refresh_snapshot();
        self.apply_window_title();
        true
    }

    fn copy_mode_search_key_table_handles_key(key: &Key, modifiers: ModifiersState) -> bool {
        match key.as_ref() {
            Key::Named(NamedKey::Escape) => true,
            Key::Character("\u{1b}" | "\r")
            | Key::Named(
                NamedKey::ArrowDown
                | NamedKey::ArrowUp
                | NamedKey::Enter
                | NamedKey::PageDown
                | NamedKey::PageUp
                | NamedKey::Backspace,
            ) if modifiers.is_empty() => true,
            Key::Character(character)
                if modifiers == ModifiersState::CONTROL && character.eq_ignore_ascii_case("n") =>
            {
                true
            }
            Key::Character(character)
                if modifiers == ModifiersState::CONTROL && character.eq_ignore_ascii_case("p") =>
            {
                true
            }
            Key::Character(character)
                if modifiers == ModifiersState::CONTROL && character.eq_ignore_ascii_case("r") =>
            {
                true
            }
            Key::Character(character)
                if modifiers == ModifiersState::CONTROL && character.eq_ignore_ascii_case("u") =>
            {
                true
            }
            Key::Character(_) if !modifiers.control_key() && !modifiers.alt_key() => true,
            _ => false,
        }
    }

    fn handle_copy_mode_search_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.exit_copy_mode();
                true
            }
            Key::Character("\u{1b}") if modifiers.is_empty() => {
                self.exit_copy_mode();
                true
            }
            Key::Named(NamedKey::ArrowDown) => {
                self.step_search(SearchDirection::Next);
                true
            }
            Key::Named(NamedKey::ArrowUp | NamedKey::Enter) => {
                self.step_search(SearchDirection::Previous);
                true
            }
            Key::Character("\r") if modifiers.is_empty() => {
                self.step_search(SearchDirection::Previous);
                true
            }
            Key::Named(NamedKey::PageDown) => {
                self.step_search_page(SearchDirection::Next);
                true
            }
            Key::Named(NamedKey::PageUp) => {
                self.step_search_page(SearchDirection::Previous);
                true
            }
            Key::Named(NamedKey::Backspace) => {
                let Some(search) = self.active_ui.retained_search() else {
                    return true;
                };
                if !search.editing {
                    return true;
                }
                let mut query = search.query.clone();
                query.pop();
                let direction = self.copy_mode_search_direction();
                self.update_search_query_with_direction(&query, direction);
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("n") =>
            {
                self.step_search(SearchDirection::Next);
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("p") =>
            {
                self.step_search(SearchDirection::Previous);
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("r") =>
            {
                self.cycle_search_match_type();
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("u") =>
            {
                let Some(search) = self.active_ui.retained_search() else {
                    return true;
                };
                if !search.editing {
                    return true;
                }
                let direction = self.copy_mode_search_direction();
                self.update_search_query_with_direction("", direction);
                true
            }
            Key::Character(text) if !modifiers.control_key() && !modifiers.alt_key() => {
                let Some(search) = self.active_ui.retained_search() else {
                    return true;
                };
                if !search.editing {
                    return true;
                }
                let mut query = search.query.clone();
                query.push_str(text);
                let direction = self.copy_mode_search_direction();
                self.update_search_query_with_direction(&query, direction);
                true
            }
            _ => true,
        }
    }

    fn copy_mode_search_direction(&self) -> SearchDirection {
        self.active_ui
            .retained_copy_mode()
            .and_then(|copy_mode| copy_mode.search_direction)
            .unwrap_or(SearchDirection::Next)
    }

    fn move_copy_mode_to_viewport_top(&mut self) -> bool {
        self.set_copy_mode_cursor(0, 0)
    }

    fn move_copy_mode_to_viewport_middle(&mut self) -> bool {
        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 {
            return false;
        }

        let middle_row = size.rows / 2;
        self.set_copy_mode_cursor(middle_row, 0)
    }

    fn move_copy_mode_to_viewport_bottom(&mut self) -> bool {
        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 {
            return false;
        }

        self.set_copy_mode_cursor(size.rows.saturating_sub(1), 0)
    }

    fn move_copy_mode_to_scrollback_top(&mut self) -> bool {
        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };
        self.set_copy_mode_cursor_for_source_position(
            self.runtime.terminal().retained_stable_range().start,
            copy_mode.source_cursor.column,
        )
    }

    fn move_copy_mode_to_scrollback_bottom(&mut self) -> bool {
        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 {
            return false;
        }
        let retained = self.runtime.terminal().retained_stable_range();
        if retained.start >= retained.end {
            return false;
        }

        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };
        self.set_copy_mode_cursor_for_source_position(
            retained.end.saturating_sub(1),
            copy_mode.source_cursor.column,
        )
    }

    fn move_copy_mode_to_line_start(&mut self) -> bool {
        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };
        self.set_copy_mode_cursor(copy_mode.cursor.row, 0)
    }

    fn move_copy_mode_to_line_content_start(&mut self) -> bool {
        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };
        let source_row = copy_mode.source_cursor.row;
        let Some((content_start, _)) = copy_mode_line_content_bounds(
            self.runtime.terminal(),
            copy_mode.source_cursor.domain,
            source_row,
        ) else {
            return false;
        };
        self.set_copy_mode_cursor_for_source_position(source_row, content_start)
    }

    fn move_copy_mode_to_line_content_end(&mut self) -> bool {
        let Some(copy_mode) = self.active_ui.copy_mode() else {
            return false;
        };
        let source_row = copy_mode.source_cursor.row;
        let Some((_, content_end)) = copy_mode_line_content_bounds(
            self.runtime.terminal(),
            copy_mode.source_cursor.domain,
            source_row,
        ) else {
            return false;
        };
        self.set_copy_mode_cursor_for_source_position(source_row, content_end)
    }

    fn exit_search_mode(&mut self) {
        self.active_ui.exit_overlay();
        self.selection = None;
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn handle_search_key(&mut self, key: &Key, modifiers: ModifiersState) -> bool {
        if self.active_ui.retained_copy_mode().is_some() {
            if Self::copy_mode_search_key_table_handles_key(key, modifiers) {
                return self.handle_copy_mode_search_key(key, modifiers);
            }
            if Self::command_palette_shortcut(key, modifiers) {
                self.enter_command_palette_mode();
                return true;
            }
            if self.handle_copy_mode_app_shell_fallback(key, modifiers) {
                return true;
            }
            return self.handle_copy_mode_search_key(key, modifiers);
        }

        match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                self.exit_search_mode();
                true
            }
            Key::Character("\u{1b}") if modifiers.is_empty() => {
                self.exit_search_mode();
                true
            }
            Key::Named(NamedKey::ArrowDown) if modifiers.is_empty() => {
                self.step_search(SearchDirection::Next);
                true
            }
            Key::Named(NamedKey::ArrowUp) if modifiers.is_empty() => {
                self.step_search(SearchDirection::Previous);
                true
            }
            Key::Named(NamedKey::PageDown) if modifiers.is_empty() => {
                self.step_search_page(SearchDirection::Next);
                true
            }
            Key::Named(NamedKey::PageUp) if modifiers.is_empty() => {
                self.step_search_page(SearchDirection::Previous);
                true
            }
            Key::Named(NamedKey::F3) if modifiers.shift_key() => {
                self.step_search(SearchDirection::Previous)
            }
            Key::Named(NamedKey::Enter | NamedKey::F3) => self.step_search(SearchDirection::Next),
            Key::Named(NamedKey::Backspace) => {
                let Some(search) = self.active_ui.retained_search() else {
                    return false;
                };
                let mut query = search.query.clone();
                if query.pop().is_none() {
                    return false;
                }
                self.update_search_query(&query);
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("n") =>
            {
                self.step_search(SearchDirection::Next);
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("p") =>
            {
                self.step_search(SearchDirection::Previous);
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("r") =>
            {
                self.cycle_search_match_type();
                true
            }
            Key::Character(character)
                if modifiers.control_key() && character.eq_ignore_ascii_case("u") =>
            {
                self.update_search_query("");
                true
            }
            Key::Character(text) if !modifiers.control_key() && !modifiers.alt_key() => {
                let Some(search) = self.active_ui.retained_search() else {
                    return false;
                };
                let mut query = search.query.clone();
                query.push_str(text);
                self.update_search_query(&query);
                true
            }
            _ => true,
        }
    }

    fn update_search_query(&mut self, query: &str) -> bool {
        self.update_search_query_with_direction(query, self.initial_search_direction())
    }

    fn initial_search_direction(&self) -> SearchDirection {
        self.active_ui
            .retained_copy_mode()
            .and_then(|copy_mode| copy_mode.search_direction)
            .unwrap_or(SearchDirection::Previous)
    }

    #[cfg(test)]
    fn set_search_pattern_editing(&mut self, editing: bool) -> bool {
        if self
            .active_ui
            .retained_search()
            .is_some_and(|search| search.editing == editing)
            && self.active_ui.copy_search_mode()
                == Some(if editing {
                    WindowCopySearchMode::Search
                } else {
                    WindowCopySearchMode::Copy
                })
        {
            return true;
        }
        if !self.active_ui.set_search_editing(editing) {
            return false;
        }
        self.reconcile_active_terminal_mutation();
        self.refresh_snapshot();
        self.apply_window_title();
        true
    }

    fn cycle_search_match_type(&mut self) -> bool {
        let Some(search) = self.active_ui.retained_search() else {
            return false;
        };
        let query = search.query.clone();
        let match_type = search.match_type.next();
        self.update_search_query_with_type(&query, SearchDirection::Next, match_type)
    }

    fn update_search_query_with_direction(
        &mut self,
        query: &str,
        direction: SearchDirection,
    ) -> bool {
        let match_type = self
            .active_ui
            .retained_search()
            .map_or(WindowSearchMatchType::CaseSensitive, |search| {
                search.match_type
            });
        self.update_search_query_with_type(query, direction, match_type)
    }

    fn update_search_query_with_type(
        &mut self,
        query: &str,
        direction: SearchDirection,
        match_type: WindowSearchMatchType,
    ) -> bool {
        if self.active_ui.retained_search().is_none() {
            let initial_copy_mode = self.initial_copy_mode();
            self.active_ui
                .enter_search(initial_copy_mode, WindowSearch::default());
        }
        let preserve_copy_state = self.active_ui.copy_search_mode()
            == Some(WindowCopySearchMode::Search)
            && self.active_ui.retained_copy_mode().is_some();
        let Some(pattern_changed) = self
            .active_ui
            .replace_search_pattern(query.to_owned(), match_type)
        else {
            return false;
        };
        if !pattern_changed {
            if query.is_empty() {
                self.selection = None;
                self.refresh_snapshot();
                self.apply_window_title();
                return false;
            }
            return self
                .active_ui
                .retained_search()
                .is_some_and(|search| search.current.is_some());
        }

        if query.is_empty() {
            self.selection = None;
            self.refresh_snapshot();
            self.apply_window_title();
            return false;
        }

        self.active_ui
            .refresh_search_match_cache(self.runtime.terminal());
        let found = self
            .active_ui
            .cached_search_matches(self.runtime.terminal())
            .and_then(|matches| find_window_search_match(&matches, None, direction));
        self.active_ui.set_search_current(found);

        let Some(found) = found else {
            self.selection = None;
            self.refresh_snapshot();
            self.apply_window_title();
            return false;
        };

        self.apply_search_match(found, preserve_copy_state);
        true
    }

    fn step_search(&mut self, direction: SearchDirection) -> bool {
        let Some((query, current)) = self
            .active_ui
            .retained_search()
            .map(|search| (search.query.clone(), search.current))
        else {
            return false;
        };
        if query.is_empty() {
            return false;
        }

        self.active_ui
            .refresh_search_match_cache(self.runtime.terminal());
        let found = self
            .active_ui
            .cached_search_matches(self.runtime.terminal())
            .and_then(|matches| find_window_search_match(&matches, current, direction));
        let Some(found) = found else {
            return false;
        };

        self.active_ui.set_search_current(Some(found));
        self.apply_search_match(found, false);
        true
    }

    fn step_search_page(&mut self, direction: SearchDirection) -> bool {
        let Some((query, current)) = self
            .active_ui
            .retained_search()
            .map(|search| (search.query.clone(), search.current))
        else {
            return false;
        };
        if query.is_empty() {
            return false;
        }

        let size = self.runtime.terminal().grid().size();
        if size.rows == 0 {
            return self.step_search(direction);
        }

        let viewport_top = self.current_viewport_stable_top();
        self.active_ui
            .refresh_search_match_cache(self.runtime.terminal());
        let retained = self.runtime.terminal().retained_stable_range();
        let found = self
            .active_ui
            .cached_search_matches(self.runtime.terminal())
            .and_then(|matches| {
                find_window_search_page_match(
                    &matches,
                    retained,
                    viewport_top,
                    usize::from(size.rows),
                    direction,
                )
                .or_else(|| find_window_search_match(&matches, current, direction))
            });
        let Some(found) = found else {
            return false;
        };

        self.active_ui.set_search_current(Some(found));
        self.apply_search_match(found, false);
        true
    }

    fn apply_search_match(&mut self, search_match: WindowSearchMatch, preserve_copy_state: bool) {
        let Some((offset, selection)) = search_match.viewport_selection(self.runtime.terminal())
        else {
            self.active_ui.set_search_current(None);
            self.selection = None;
            self.refresh_snapshot();
            self.apply_window_title();
            return;
        };
        self.interaction_state.active_ui
            .stable_viewport
            .set_scrollback_offset(self.runtime.terminal(), offset);
        self.selection = Some(selection);
        if !preserve_copy_state && let Some(copy_mode) = self.active_ui.retained_copy_mode_mut() {
            copy_mode.cursor = selection.anchor;
            copy_mode.source_cursor = SelectionSourceCell {
                domain: search_match.domain,
                row: search_match.source_row,
                column: usize::from(search_match.start_column),
            };
            copy_mode.anchor = None;
            copy_mode.source_anchor = None;
        }
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn copy_selection_to_clipboard(&mut self) -> bool {
        self.copy_selection_to(WindowCopyDestination::Clipboard)
    }

    fn copy_selection_to_primary_selection(&mut self) -> bool {
        self.copy_selection_to(WindowCopyDestination::PrimarySelection)
    }

    fn copy_selection_to_clipboard_and_primary_selection(&mut self) -> bool {
        self.copy_selection_to(WindowCopyDestination::ClipboardAndPrimarySelection)
    }

    fn copy_selection_to(&mut self, destination: WindowCopyDestination) -> bool {
        let Some(text) = self.selected_text() else {
            return false;
        };

        self.write_text_to_copy_destination(&text, destination)
    }

    fn write_text_to_copy_destination(
        &mut self,
        text: &str,
        destination: WindowCopyDestination,
    ) -> bool {
        match destination {
            WindowCopyDestination::Clipboard => self.write_clipboard_text(text),
            WindowCopyDestination::PrimarySelection => self.write_primary_selection_text(text),
            WindowCopyDestination::ClipboardAndPrimarySelection => {
                let clipboard_written = self.write_clipboard_text(text);
                let primary_written = self.write_primary_selection_text(text);
                clipboard_written || primary_written
            }
        }
    }

    fn paste_captured_selected_text_to_pane(&mut self, text: Option<&str>) -> io::Result<bool> {
        let Some(text) = text else {
            return Ok(false);
        };
        if text.is_empty() {
            return Ok(false);
        }

        let bytes = encode_window_paste(
            text,
            self.runtime.bracketed_paste(),
            self.canonicalize_pasted_newlines,
        );
        self.write_pty_bytes(&bytes)?;
        Ok(true)
    }

    fn write_clipboard_text(&mut self, text: &str) -> bool {
        (self.clipboard_writer)(text)
    }

    fn read_clipboard_text(&mut self) -> Option<String> {
        (self.clipboard_reader)()
    }

    fn write_primary_selection_text(&mut self, text: &str) -> bool {
        (self.primary_selection_writer)(text)
    }

    fn read_primary_selection_text(&mut self) -> Option<String> {
        (self.primary_selection_reader)()
    }

    fn dispatch_notification(
        &mut self,
        pane: rssh_core::PaneId,
        notification: &TerminalNotification,
    ) -> bool {
        if !self.should_show_notification_from_pane(pane) {
            return false;
        }

        self.latest_notification = Some(notification.clone());
        self.apply_window_title();
        (self.notification_handler)(notification)
    }

    fn should_show_notification_from_pane(&self, pane: rssh_core::PaneId) -> bool {
        match self.notification_handling {
            NativeNotificationHandling::AlwaysShow => true,
            NativeNotificationHandling::NeverShow => false,
            NativeNotificationHandling::SuppressFromFocusedWindow => !self.window_focused,
            NativeNotificationHandling::SuppressFromFocusedPane => {
                !self.window_focused || pane != self.app_shell.active_pane_id()
            }
            NativeNotificationHandling::SuppressFromFocusedTab => {
                !self.window_focused || !self.pane_is_in_active_tab(pane)
            }
        }
    }

    fn pane_is_in_active_tab(&self, pane: rssh_core::PaneId) -> bool {
        self.app_shell
            .active_tab()
            .panes()
            .iter()
            .any(|candidate| candidate.id() == pane)
    }

    fn dispatch_open_uri_in_context(
        &mut self,
        event: &NativeWindowOpenUri,
        target: Option<WheelTarget>,
    ) -> bool {
        if !(self.open_uri_handler)(event) {
            return false;
        }
        let Some(handler) = self.lua_open_uri.clone() else {
            return true;
        };
        if let Some(command) = handler.command_for_event(event) {
            let result = if let Some(target) = target {
                self.apply_command_for_target_context(target, command)
                    .map_err(|error| error.to_string())
            } else if self.command_palette_execute(command) {
                Ok(())
            } else {
                Err("command execution failed".to_owned())
            };
            if let Err(error) = result {
                eprintln!("open-uri action failed: {error}");
            }
        }
        handler.allows_default(event)
    }

    fn dispatch_new_tab_button_click(&mut self, event: &NativeWindowNewTabButtonClick) -> bool {
        if !(self.new_tab_button_click_handler)(event) {
            return false;
        }
        let Some(handler) = self.lua_new_tab_button_click else {
            return true;
        };
        if handler.performs_default_action()
            && let Some(command) = event.default_action.clone()
        {
            self.command_palette_execute(command);
        }
        handler.allows_default(event)
    }

    fn dispatch_update_status(&mut self) {
        let event = NativeWindowStatusUpdateEvent {
            window_id: self.app_window_id,
            pane: self.app_shell.active_pane_id(),
        };
        let update = (self.update_status_handler)(&event);
        if let Some(left_status) = update.left_status {
            self.left_status = left_status;
        }
        if let Some(right_status) = update.right_status {
            self.right_status = right_status;
        }
        if let Some(config_overrides) = self.lua_update_status_config_overrides.clone() {
            self.apply_lua_window_config_overrides(config_overrides);
        }
        if let Some(update) = self.lua_update_status.clone() {
            if let Some(left_status) = update.left_status {
                self.left_status = self.lua_window_status_text(left_status);
            }
            if let Some(right_status) = update.right_status {
                self.right_status = self.lua_window_status_text(right_status);
            }
        }
        if let Some(right_status) = (self.update_right_status_handler)(&event) {
            self.right_status = right_status;
        }
    }

    fn apply_lua_window_config_overrides(
        &mut self,
        config_overrides: NativeWindowConfigPatch,
    ) {
        self.set_window_config_overrides(Some(config_overrides), ReloadDisposition::ReloadAttempt);
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn lua_window_status_text(&self, status: NativeLuaWindowStatusText) -> String {
        match status {
            NativeLuaWindowStatusText::Static(status) => status,
            NativeLuaWindowStatusText::ActiveWorkspace => {
                self.app_shell.active_workspace().name().to_owned()
            }
            NativeLuaWindowStatusText::WindowId { prefix, suffix } => {
                format!("{prefix}{}{suffix}", self.app_window_id.get())
            }
            NativeLuaWindowStatusText::WindowPane { parts } => parts
                .into_iter()
                .map(|part| match part {
                    NativeLuaWindowPaneStatusPart::Static(text) => text,
                    NativeLuaWindowPaneStatusPart::ActiveWorkspace => {
                        self.app_shell.active_workspace().name().to_owned()
                    }
                    NativeLuaWindowPaneStatusPart::WindowId => self.app_window_id.get().to_string(),
                    NativeLuaWindowPaneStatusPart::ActiveTabId => {
                        self.app_shell.active_tab_id().get().to_string()
                    }
                    NativeLuaWindowPaneStatusPart::ActiveTabTitle => {
                        tab_title_override(self.app_shell.active_tab())
                            .unwrap_or_default()
                            .to_owned()
                    }
                    NativeLuaWindowPaneStatusPart::PaneId => {
                        self.app_shell.active_pane_id().get().to_string()
                    }
                    NativeLuaWindowPaneStatusPart::PaneTitle => self
                        .pane_title(self.app_shell.active_pane_id())
                        .unwrap_or_default(),
                    NativeLuaWindowPaneStatusPart::PaneDomainName => "local".to_owned(),
                    NativeLuaWindowPaneStatusPart::PaneCurrentWorkingDir => self
                        .app_shell
                        .active_pane()
                        .launch()
                        .cwd()
                        .unwrap_or_default()
                        .to_owned(),
                    NativeLuaWindowPaneStatusPart::PaneForegroundProcessName => {
                        self.app_shell.active_pane().launch().program().to_owned()
                    }
                    NativeLuaWindowPaneStatusPart::PaneTtyName => self
                        .pane_tty_name(self.app_shell.active_pane_id())
                        .unwrap_or_default()
                        .to_owned(),
                })
                .collect::<String>(),
            NativeLuaWindowStatusText::ActiveKeyTable { prefix, fallback } => {
                self.key_table_stack.last().map_or(fallback, |activation| {
                    format!("{prefix}{}", activation.name)
                })
            }
            NativeLuaWindowStatusText::CompositionStatus { prefix, fallback } => self
                .composition_status_text()
                .map_or(fallback, |status| format!("{prefix}{status}")),
            NativeLuaWindowStatusText::Leader { active, inactive } => {
                if self.leader_active_since.is_some() {
                    active
                } else {
                    inactive
                }
            }
            NativeLuaWindowStatusText::Focus { focused, unfocused } => {
                if self.window_focused {
                    focused
                } else {
                    unfocused
                }
            }
            NativeLuaWindowStatusText::PaneAltScreen { active, inactive } => {
                if self.runtime.terminal().alternate_screen_active() {
                    active
                } else {
                    inactive
                }
            }
            NativeLuaWindowStatusText::PaneHasUnseenOutput { unseen, seen } => {
                if self
                    .pane_has_unseen_output(self.app_shell.active_pane_id())
                    .unwrap_or(false)
                {
                    unseen
                } else {
                    seen
                }
            }
            NativeLuaWindowStatusText::WindowDimensions { parts } => parts
                .into_iter()
                .map(|part| match part {
                    NativeLuaWindowDimensionsStatusPart::Static(text) => text,
                    NativeLuaWindowDimensionsStatusPart::Field(field) => {
                        self.lua_window_dimensions_field_text(field)
                    }
                })
                .collect::<String>(),
            NativeLuaWindowStatusText::WindowEffectiveConfig { parts } => parts
                .into_iter()
                .map(|part| match part {
                    NativeLuaWindowEffectiveConfigStatusPart::Static(text) => text,
                    NativeLuaWindowEffectiveConfigStatusPart::Field(field) => {
                        self.lua_window_effective_config_field_text(field)
                    }
                })
                .collect::<String>(),
            NativeLuaWindowStatusText::PaneDimensions { parts } => parts
                .into_iter()
                .map(|part| match part {
                    NativeLuaPaneDimensionsStatusPart::Static(text) => text,
                    NativeLuaPaneDimensionsStatusPart::Field(field) => {
                        self.lua_pane_dimensions_field_text(field)
                    }
                })
                .collect::<String>(),
            NativeLuaWindowStatusText::PaneCursorPosition { parts } => parts
                .into_iter()
                .map(|part| match part {
                    NativeLuaPaneCursorPositionStatusPart::Static(text) => text,
                    NativeLuaPaneCursorPositionStatusPart::Field(field) => {
                        self.lua_pane_cursor_position_field_text(field)
                    }
                })
                .collect::<String>(),
            NativeLuaWindowStatusText::PaneUserVars { parts } => parts
                .into_iter()
                .map(|part| match part {
                    NativeLuaPaneUserVarsStatusPart::Static(text) => text,
                    NativeLuaPaneUserVarsStatusPart::UserVar { name, fallback } => {
                        self.lua_pane_user_var_text(&name, fallback)
                    }
                })
                .collect::<String>(),
            NativeLuaWindowStatusText::PaneProgress {
                none,
                percentage_prefix,
                error_prefix,
                indeterminate,
            } => {
                self.lua_pane_progress_text(none, &percentage_prefix, &error_prefix, indeterminate)
            }
            NativeLuaWindowStatusText::KeyboardModifiers { parts } => {
                let modifiers = native_lua_keyboard_modifiers_text(self.modifiers);
                let leds = String::new();
                parts
                    .into_iter()
                    .map(|part| match part {
                        NativeLuaKeyboardModifiersStatusPart::Static(text) => text,
                        NativeLuaKeyboardModifiersStatusPart::Modifiers => modifiers.clone(),
                        NativeLuaKeyboardModifiersStatusPart::Leds => leds.clone(),
                    })
                    .collect::<String>()
            }
        }
    }

}

impl NativeWindowApp {
    fn lua_window_dimensions_field_text(&self, field: NativeLuaWindowDimensionsField) -> String {
        match field {
            NativeLuaWindowDimensionsField::PixelWidth => self.window_frame.width.to_string(),
            NativeLuaWindowDimensionsField::PixelHeight => self.window_frame.height.to_string(),
            NativeLuaWindowDimensionsField::Dpi => self.window_dpi.to_string(),
            NativeLuaWindowDimensionsField::IsFullScreen => self.full_screen.to_string(),
        }
    }

    fn lua_window_effective_config_field_text(
        &self,
        field: NativeLuaWindowEffectiveConfigField,
    ) -> String {
        self.lua_window_effective_config_field_text_part1(field)
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_window_effective_config_field_text_part1(
        &self,
        field: NativeLuaWindowEffectiveConfigField,
    ) -> String {
        match field {
           NativeLuaWindowEffectiveConfigField::FontSize => {
                native_lua_font_size_points_text(self.font_size)
            }
            NativeLuaWindowEffectiveConfigField::DefaultWorkspace => self.default_workspace.clone(),
            NativeLuaWindowEffectiveConfigField::DefaultProg(index) => index
                .checked_sub(1)
                .and_then(|offset| self.default_prog.as_ref()?.get(offset))
                .cloned()
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::DefaultGuiStartupArg(index) => index
                .checked_sub(1)
                .and_then(|offset| self.default_gui_startup_args.get(offset))
                .cloned()
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::DefaultCwd => {
                self.default_cwd.clone().unwrap_or_default()
            }
            NativeLuaWindowEffectiveConfigField::DefaultDomain => self.default_domain.clone(),
            NativeLuaWindowEffectiveConfigField::PreferToSpawnTabs => {
                self.prefer_to_spawn_tabs.to_string()
            }
            NativeLuaWindowEffectiveConfigField::SshBackend => match self.ssh_backend {
                NativeSshBackend::Ssh2 => "Ssh2",
                NativeSshBackend::LibSsh => "LibSsh",
            }
            .to_owned(),
            NativeLuaWindowEffectiveConfigField::StatusUpdateInterval => {
                self.status_update_interval.as_millis().to_string()
            }
            NativeLuaWindowEffectiveConfigField::TabMaxWidth => self.tab_max_width.to_string(),
            NativeLuaWindowEffectiveConfigField::Dpi => self.window_dpi.to_string(),
            NativeLuaWindowEffectiveConfigField::DpiByScreen(name) => self
                .dpi_by_screen
                .get(&name)
                .map(std::string::ToString::to_string)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::ResolvedPalette(field) => {
                let palette = self.native_resolved_palette();
                match field {
                    NativeLuaResolvedPaletteField::Foreground => {
                        native_lua_color_config_text(palette.foreground)
                    }
                    NativeLuaResolvedPaletteField::Background => {
                        native_lua_color_config_text(palette.background)
                    }
                    NativeLuaResolvedPaletteField::CursorBg => {
                        native_lua_color_config_text(palette.cursor_bg)
                    }
                    NativeLuaResolvedPaletteField::CursorFg => palette
                        .cursor_fg
                        .map(native_lua_color_config_text)
                        .unwrap_or_default(),
                    NativeLuaResolvedPaletteField::CursorBorder => palette
                        .cursor_border
                        .map(native_lua_color_config_text)
                        .unwrap_or_default(),
                    NativeLuaResolvedPaletteField::SelectionFg => palette
                        .selection_fg
                        .flatten()
                        .map(native_lua_color_config_text)
                        .unwrap_or_default(),
                    NativeLuaResolvedPaletteField::SelectionBg => palette
                        .selection_bg
                        .map(native_lua_color_config_text)
                        .unwrap_or_default(),
                    NativeLuaResolvedPaletteField::ComposeCursor => palette
                        .compose_cursor
                        .map(native_lua_color_config_text)
                        .unwrap_or_default(),
                    NativeLuaResolvedPaletteField::VisualBell => palette
                        .visual_bell
                        .map(native_lua_color_config_text)
                        .unwrap_or_default(),
                    NativeLuaResolvedPaletteField::Ansi(index) => index
                        .checked_sub(1)
                        .and_then(|offset| palette.ansi.get(offset).copied())
                        .map(native_lua_color_config_text)
                        .unwrap_or_default(),
                    NativeLuaResolvedPaletteField::Bright(index) => index
                        .checked_sub(1)
                        .and_then(|offset| palette.brights.get(offset).copied())
                        .map(native_lua_color_config_text)
                        .unwrap_or_default(),
                    NativeLuaResolvedPaletteField::Indexed(index) => palette
                        .indexed
                        .get(index)
                        .copied()
                        .flatten()
                        .map(native_lua_color_config_text)
                        .unwrap_or_default(),
                }
            }
            NativeLuaWindowEffectiveConfigField::VisualBell(field) => match field {
                NativeLuaVisualBellField::FadeInDurationMs => {
                    self.visual_bell.fade_in_duration_ms.to_string()
                }
                NativeLuaVisualBellField::FadeOutDurationMs => {
                    self.visual_bell.fade_out_duration_ms.to_string()
                }
                NativeLuaVisualBellField::FadeInFunction => {
                    self.visual_bell.fade_in_function.config_text().to_owned()
                }
                NativeLuaVisualBellField::FadeOutFunction => {
                    self.visual_bell.fade_out_function.config_text().to_owned()
                }
                NativeLuaVisualBellField::Target => {
                    self.visual_bell.target.as_wezterm_config_value().to_owned()
                }
            },
            NativeLuaWindowEffectiveConfigField::ColorScheme => {
                self.color_scheme.clone().unwrap_or_default()
            }
            NativeLuaWindowEffectiveConfigField::ForegroundColor => {
                native_lua_color_config_text(self.foreground_color)
            }
            NativeLuaWindowEffectiveConfigField::BackgroundColor => {
                native_lua_color_config_text(self.background_color)
            }
            NativeLuaWindowEffectiveConfigField::MaxFps => self.max_fps.to_string(),
            NativeLuaWindowEffectiveConfigField::AnimationFps => self.animation_fps.to_string(),
            NativeLuaWindowEffectiveConfigField::FrontEnd => {
                self.front_end.as_wezterm_config_str().to_owned()
            }
            NativeLuaWindowEffectiveConfigField::WebGpuPowerPreference => self
                .webgpu_power_preference
                .as_wezterm_config_str()
                .to_owned(),
            NativeLuaWindowEffectiveConfigField::WebGpuForceFallbackAdapter => {
                self.webgpu_force_fallback_adapter.to_string()
            }
            NativeLuaWindowEffectiveConfigField::WebGpuPreferredAdapter(field) => self
                .webgpu_preferred_adapter
                .as_ref()
                .and_then(|adapter| match field {
                    NativeLuaWebGpuPreferredAdapterField::Backend => adapter.backend.clone(),
                    NativeLuaWebGpuPreferredAdapterField::Device => {
                        adapter.device.map(|device| device.to_string())
                    }
                    NativeLuaWebGpuPreferredAdapterField::DeviceType => adapter.device_type.clone(),
                    NativeLuaWebGpuPreferredAdapterField::Driver => adapter.driver.clone(),
                    NativeLuaWebGpuPreferredAdapterField::DriverInfo => adapter.driver_info.clone(),
                    NativeLuaWebGpuPreferredAdapterField::Name => adapter.name.clone(),
                    NativeLuaWebGpuPreferredAdapterField::Vendor => {
                        adapter.vendor.map(|vendor| vendor.to_string())
                    }
                })
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::PreferEgl => self.prefer_egl.to_string(),
            NativeLuaWindowEffectiveConfigField::EnableWayland => self.enable_wayland.to_string(),
            NativeLuaWindowEffectiveConfigField::EnableZwlrOutputManager => {
                self.enable_zwlr_output_manager.to_string()
            }
            NativeLuaWindowEffectiveConfigField::UseBoxModelRender => {
                self.use_box_model_render.to_string()
            }
            NativeLuaWindowEffectiveConfigField::ExperimentalPixelPositioning => {
                self.experimental_pixel_positioning.to_string()
            }
            NativeLuaWindowEffectiveConfigField::IgnoreSvgFonts => {
                self.ignore_svg_fonts.to_string()
            }
            NativeLuaWindowEffectiveConfigField::BidiEnabled => self.bidi_enabled.to_string(),
            NativeLuaWindowEffectiveConfigField::BidiDirection => {
                self.bidi_direction.as_wezterm_config_value().to_owned()
            }
            NativeLuaWindowEffectiveConfigField::CellWidth => {
                native_ratio_config_text(self.cell_width.as_f64())
            }
            NativeLuaWindowEffectiveConfigField::LineHeight => {
                native_ratio_config_text(self.line_height.as_f64())
            }
            NativeLuaWindowEffectiveConfigField::FontAntialias => {
                self.font_antialias.as_wezterm_config_value().to_owned()
            }
            NativeLuaWindowEffectiveConfigField::FontHinting => {
                self.font_hinting.as_wezterm_config_value().to_owned()
            }
            NativeLuaWindowEffectiveConfigField::FontRasterizer => {
                self.font_rasterizer.as_wezterm_config_value().to_owned()
            }
            NativeLuaWindowEffectiveConfigField::FontColrRasterizer => self
                .font_colr_rasterizer
                .as_wezterm_config_value()
                .to_owned(),
            NativeLuaWindowEffectiveConfigField::FontShaper => {
                self.font_shaper.as_wezterm_config_value().to_owned()
            }
            NativeLuaWindowEffectiveConfigField::HarfbuzzFeature(index) => index
                .checked_sub(1)
                .and_then(|offset| self.harfbuzz_features.get(offset))
                .cloned()
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::FontDir(index) => index
                .checked_sub(1)
                .and_then(|offset| self.font_dirs.get(offset))
                .cloned()
                .unwrap_or_default(),
            field => self.lua_window_effective_config_field_text_part2(field),
        }
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_window_effective_config_field_text_part2(
        &self,
        field: NativeLuaWindowEffectiveConfigField,
    ) -> String {
        match field {
            NativeLuaWindowEffectiveConfigField::CellWidths(index, field) => index
                .checked_sub(1)
                .and_then(|offset| self.cell_widths.get(offset))
                .map(|override_| match field {
                    NativeLuaCellWidthOverrideField::First => override_.first.to_string(),
                    NativeLuaCellWidthOverrideField::Last => override_.last.to_string(),
                    NativeLuaCellWidthOverrideField::Width => override_.width.to_string(),
                })
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::FontLocator => self
                .font_locator
                .map(|font_locator| font_locator.as_wezterm_config_value().to_owned())
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::UseCapHeightToScaleFallbackFonts => {
                self.use_cap_height_to_scale_fallback_fonts.to_string()
            }
            NativeLuaWindowEffectiveConfigField::SortFallbackFontsByCoverage => {
                self.sort_fallback_fonts_by_coverage.to_string()
            }
            NativeLuaWindowEffectiveConfigField::SearchFontDirsForFallback => {
                self.search_font_dirs_for_fallback.to_string()
            }
            NativeLuaWindowEffectiveConfigField::FreetypeLoadTarget => self
                .freetype_load_target
                .as_wezterm_config_value()
                .to_owned(),
            NativeLuaWindowEffectiveConfigField::FreetypeRenderTarget => self
                .freetype_render_target
                .as_wezterm_config_value()
                .to_owned(),
            NativeLuaWindowEffectiveConfigField::FreetypeLoadFlags => {
                self.effective_freetype_load_flags().config_text()
            }
            NativeLuaWindowEffectiveConfigField::FreetypeInterpreterVersion => self
                .freetype_interpreter_version
                .map(|version| version.to_string())
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::FreetypePcfLongFamilyNames => {
                self.freetype_pcf_long_family_names.to_string()
            }
            NativeLuaWindowEffectiveConfigField::BoldBrightensAnsiColors => self
                .bold_brightens_ansi_colors
                .as_wezterm_config_value()
                .to_owned(),
            NativeLuaWindowEffectiveConfigField::AllowSquareGlyphsToOverflowWidth => self
                .allow_square_glyphs_to_overflow_width
                .as_wezterm_config_value()
                .to_owned(),
            NativeLuaWindowEffectiveConfigField::DisplayPixelGeometry => self
                .display_pixel_geometry
                .as_wezterm_config_value()
                .to_owned(),
            NativeLuaWindowEffectiveConfigField::TextBackgroundOpacity => {
                self.text_background_opacity.config_text()
            }
            NativeLuaWindowEffectiveConfigField::WindowBackgroundOpacity => {
                self.window_background_opacity.config_text()
            }
            NativeLuaWindowEffectiveConfigField::ForegroundTextHsbHue => {
                self.foreground_text_hsb.hue.config_text()
            }
            NativeLuaWindowEffectiveConfigField::ForegroundTextHsbSaturation => {
                self.foreground_text_hsb.saturation.config_text()
            }
            NativeLuaWindowEffectiveConfigField::ForegroundTextHsbBrightness => {
                self.foreground_text_hsb.brightness.config_text()
            }
            NativeLuaWindowEffectiveConfigField::InactivePaneHsbHue => {
                self.inactive_pane_hsb.hue.config_text()
            }
            NativeLuaWindowEffectiveConfigField::InactivePaneHsbSaturation => {
                self.inactive_pane_hsb.saturation.config_text()
            }
            NativeLuaWindowEffectiveConfigField::InactivePaneHsbBrightness => {
                self.inactive_pane_hsb.brightness.config_text()
            }
            NativeLuaWindowEffectiveConfigField::ShapeCacheSize => {
                self.shape_cache_size.to_string()
            }
            NativeLuaWindowEffectiveConfigField::LineStateCacheSize => {
                self.line_state_cache_size.to_string()
            }
            NativeLuaWindowEffectiveConfigField::LineQuadCacheSize => {
                self.line_quad_cache_size.to_string()
            }
            NativeLuaWindowEffectiveConfigField::LineToEleShapeCacheSize => {
                self.line_to_ele_shape_cache_size.to_string()
            }
            NativeLuaWindowEffectiveConfigField::GlyphCacheImageCacheSize => {
                self.glyph_cache_image_cache_size.to_string()
            }
            NativeLuaWindowEffectiveConfigField::CursorBlinkRate => {
                self.cursor_blink_rate.as_millis().to_string()
            }
            NativeLuaWindowEffectiveConfigField::CursorBlinkEaseIn => {
                self.cursor_blink_ease_in.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::CursorBlinkEaseOut => {
                self.cursor_blink_ease_out.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::TextBlinkRate => {
                self.text_blink_rate.as_millis().to_string()
            }
            NativeLuaWindowEffectiveConfigField::TextBlinkRateRapid => {
                self.text_blink_rate_rapid.as_millis().to_string()
            }
            NativeLuaWindowEffectiveConfigField::TextBlinkEaseIn => {
                self.text_blink_ease_in.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::TextBlinkEaseOut => {
                self.text_blink_ease_out.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::TextBlinkRapidEaseIn => {
                self.text_blink_rapid_ease_in.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::TextBlinkRapidEaseOut => {
                self.text_blink_rapid_ease_out.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::CursorThickness => self
                .cursor_thickness
                .map(native_cursor_thickness_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::UnderlineThickness => self
                .underline_thickness
                .map(native_underline_thickness_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::UnderlinePosition => self
                .underline_position
                .map(native_underline_position_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::StrikethroughPosition => self
                .strikethrough_position
                .map(native_strikethrough_position_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::HideMouseCursorWhenTyping => {
                self.hide_mouse_cursor_when_typing.to_string()
            }
            NativeLuaWindowEffectiveConfigField::DefaultMuxServerDomain => {
                self.default_mux_server_domain.clone().unwrap_or_default()
            }
            NativeLuaWindowEffectiveConfigField::DaemonOption(field) => match field {
                NativeLuaDaemonOptionsField::PidFile => self.daemon_options.pid_file.clone(),
                NativeLuaDaemonOptionsField::Stdout => self.daemon_options.stdout.clone(),
                NativeLuaDaemonOptionsField::Stderr => self.daemon_options.stderr.clone(),
            }
            .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::RatelimitMuxLinePrefetchesPerSecond => {
                self.ratelimit_mux_line_prefetches_per_second.to_string()
            }
            NativeLuaWindowEffectiveConfigField::MuxOutputParserBufferSize => {
                self.mux_output_parser_buffer_size.to_string()
            }
            NativeLuaWindowEffectiveConfigField::MuxOutputParserCoalesceDelayMs => {
                self.mux_output_parser_coalesce_delay_ms.to_string()
            }
            NativeLuaWindowEffectiveConfigField::MuxEnvRemove(index) => index
                .checked_sub(1)
                .and_then(|offset| self.mux_env_remove.get(offset))
                .cloned()
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::SetEnvironmentVariable(name) => self
                .set_environment_variables
                .get(&name)
                .cloned()
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::PeriodicStatLogging => {
                self.periodic_stat_logging.to_string()
            }
            NativeLuaWindowEffectiveConfigField::UlimitNofile => self.ulimit_nofile.to_string(),
            NativeLuaWindowEffectiveConfigField::UlimitNproc => self.ulimit_nproc.to_string(),
            NativeLuaWindowEffectiveConfigField::ScrollToBottomOnInput => {
                self.scroll_to_bottom_on_input.to_string()
            }
            NativeLuaWindowEffectiveConfigField::UseIme => self.use_ime.to_string(),
            NativeLuaWindowEffectiveConfigField::XimImName => {
                self.xim_im_name.clone().unwrap_or_default()
            }
            NativeLuaWindowEffectiveConfigField::ImePreeditRendering => {
                match self.ime_preedit_rendering {
                    NativeImePreeditRendering::Builtin => "Builtin",
                    NativeImePreeditRendering::System => "System",
                }
                .to_owned()
            }
            NativeLuaWindowEffectiveConfigField::MacosForwardToImeModifierMask => {
                native_lua_keyboard_modifiers_text(self.macos_forward_to_ime_modifier_mask)
            }
            NativeLuaWindowEffectiveConfigField::NotificationHandling => match self
                .notification_handling
            {
                NativeNotificationHandling::AlwaysShow => "AlwaysShow",
                NativeNotificationHandling::NeverShow => "NeverShow",
                NativeNotificationHandling::SuppressFromFocusedPane => "SuppressFromFocusedPane",
                NativeNotificationHandling::SuppressFromFocusedTab => "SuppressFromFocusedTab",
                NativeNotificationHandling::SuppressFromFocusedWindow => {
                    "SuppressFromFocusedWindow"
                }
            }
            .to_owned(),
            NativeLuaWindowEffectiveConfigField::UseDeadKeys => self.use_dead_keys.to_string(),
            NativeLuaWindowEffectiveConfigField::AudibleBell => {
                self.audible_bell.as_wezterm_config_value().to_owned()
            }
            field => self.lua_window_effective_config_field_text_part3(field),
        }
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_window_effective_config_field_text_part3(
        &self,
        field: NativeLuaWindowEffectiveConfigField,
    ) -> String {
        match field {
            NativeLuaWindowEffectiveConfigField::LaunchMenu(index, field) => index
                .checked_sub(1)
                .and_then(|offset| self.launch_menu.get(offset))
                .map(|item| match field {
                    NativeLuaLaunchMenuField::Label => item.label.clone().unwrap_or_default(),
                    NativeLuaLaunchMenuField::Arg(arg_index) => match &item.command {
                        NativeLaunchMenuCommand::Command(command) => arg_index
                            .checked_sub(1)
                            .and_then(|offset| {
                                if offset == 0 {
                                    Some(command.program.as_str())
                                } else {
                                    command.args.get(offset - 1).map(String::as_str)
                                }
                            })
                            .unwrap_or_default()
                            .to_owned(),
                        NativeLaunchMenuCommand::Options(_) => match arg_index.checked_sub(1) {
                            Some(offset) => self
                                .default_prog
                                .as_ref()
                                .and_then(|default_prog| default_prog.get(offset))
                                .cloned()
                                .unwrap_or_else(|| {
                                    let launch = self.app_shell.active_pane().launch();
                                    if offset == 0 {
                                        pane_launch_display_program(launch).to_owned()
                                    } else {
                                        launch
                                            .args()
                                            .get(offset - 1)
                                            .map(std::string::ToString::to_string)
                                            .unwrap_or_default()
                                    }
                                }),
                            None => String::new(),
                        },
                    },
                    NativeLuaLaunchMenuField::Cwd => match &item.command {
                        NativeLaunchMenuCommand::Command(command) => {
                            command.cwd.clone().unwrap_or_default()
                        }
                        NativeLaunchMenuCommand::Options(options) => {
                            options.cwd.clone().unwrap_or_default()
                        }
                    },
                    NativeLuaLaunchMenuField::Domain => match &item.command {
                        NativeLaunchMenuCommand::Command(command) => command
                            .domain
                            .as_ref()
                            .map(native_spawn_domain_config_text)
                            .unwrap_or_default(),
                        NativeLaunchMenuCommand::Options(options) => options
                            .domain
                            .as_ref()
                            .map(native_spawn_domain_config_text)
                            .unwrap_or_default(),
                    },
                    NativeLuaLaunchMenuField::SetEnvironmentVariable(name) => match &item.command {
                        NativeLaunchMenuCommand::Command(command) => {
                            command.environment.get(&name).cloned().unwrap_or_default()
                        }
                        NativeLaunchMenuCommand::Options(options) => {
                            options.environment.get(&name).cloned().unwrap_or_default()
                        }
                    },
                })
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::AutomaticallyReloadConfig => {
                self.automatically_reload_config.to_string()
            }
            NativeLuaWindowEffectiveConfigField::CheckForUpdates => {
                self.check_for_updates.to_string()
            }
            NativeLuaWindowEffectiveConfigField::ShowUpdateWindow => {
                self.show_update_window.to_string()
            }
            NativeLuaWindowEffectiveConfigField::CheckForUpdatesIntervalSeconds => {
                self.check_for_updates_interval_seconds.to_string()
            }
            NativeLuaWindowEffectiveConfigField::EnableKittyGraphics => {
                self.enable_kitty_graphics.to_string()
            }
            NativeLuaWindowEffectiveConfigField::EnableChecksumRectangularArea => {
                self.enable_checksum_rectangular_area.to_string()
            }
            NativeLuaWindowEffectiveConfigField::EnableTitleReporting => {
                self.enable_title_reporting.to_string()
            }
            NativeLuaWindowEffectiveConfigField::EnableCsiUKeyEncoding => {
                self.enable_csi_u_key_encoding.to_string()
            }
            NativeLuaWindowEffectiveConfigField::EnableKittyKeyboard => {
                self.enable_kitty_keyboard.to_string()
            }
            NativeLuaWindowEffectiveConfigField::AllowDownloadProtocols => {
                self.allow_download_protocols.to_string()
            }
            NativeLuaWindowEffectiveConfigField::XcursorTheme => {
                self.xcursor_theme.clone().unwrap_or_default()
            }
            NativeLuaWindowEffectiveConfigField::XcursorSize => self
                .xcursor_size
                .map(|xcursor_size| xcursor_size.to_string())
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::PaletteMaxKeyAssigmentsForAction => {
                self.palette_max_key_assigments_for_action.to_string()
            }
            NativeLuaWindowEffectiveConfigField::AllowWin32InputMode => {
                self.allow_win32_input_mode.to_string()
            }
            NativeLuaWindowEffectiveConfigField::TreatLeftCtrlAltAsAltGr => {
                self.treat_left_ctrlalt_as_altgr.to_string()
            }
            NativeLuaWindowEffectiveConfigField::SendComposedKeyWhenLeftAltIsPressed => {
                self.send_composed_key_when_left_alt_is_pressed.to_string()
            }
            NativeLuaWindowEffectiveConfigField::SendComposedKeyWhenRightAltIsPressed => {
                self.send_composed_key_when_right_alt_is_pressed.to_string()
            }
            NativeLuaWindowEffectiveConfigField::TreatEastAsianAmbiguousWidthAsWide => {
                self.treat_east_asian_ambiguous_width_as_wide.to_string()
            }
            NativeLuaWindowEffectiveConfigField::NormalizeOutputToUnicodeNfc => {
                self.normalize_output_to_unicode_nfc.to_string()
            }
            NativeLuaWindowEffectiveConfigField::UnicodeVersion => self.unicode_version.to_string(),
            NativeLuaWindowEffectiveConfigField::WindowCloseConfirmation => {
                match self.window_close_confirmation {
                    NativeWindowCloseConfirmation::AlwaysPrompt => "AlwaysPrompt",
                    NativeWindowCloseConfirmation::NeverPrompt => "NeverPrompt",
                }
                .to_owned()
            }
            NativeLuaWindowEffectiveConfigField::EnableTabBar => self.enable_tab_bar.to_string(),
            NativeLuaWindowEffectiveConfigField::UseFancyTabBar => {
                self.use_fancy_tab_bar.to_string()
            }
            NativeLuaWindowEffectiveConfigField::TabBarAtBottom => {
                self.tab_bar_at_bottom.to_string()
            }
            NativeLuaWindowEffectiveConfigField::MouseWheelScrollsTabs => {
                self.mouse_wheel_scrolls_tabs.to_string()
            }
            NativeLuaWindowEffectiveConfigField::ShowCloseTabButtonInTabs => {
                self.show_close_tab_button_in_tabs.to_string()
            }
            NativeLuaWindowEffectiveConfigField::ShowNewTabButtonInTabBar => {
                self.show_new_tab_button_in_tab_bar.to_string()
            }
            NativeLuaWindowEffectiveConfigField::ShowTabIndexInTabBar => {
                self.show_tab_index_in_tab_bar.to_string()
            }
            NativeLuaWindowEffectiveConfigField::ShowTabsInTabBar => {
                self.show_tabs_in_tab_bar.to_string()
            }
            NativeLuaWindowEffectiveConfigField::TabAndSplitIndicesAreZeroBased => {
                self.tab_and_split_indices_are_zero_based.to_string()
            }
            NativeLuaWindowEffectiveConfigField::HideTabBarIfOnlyOneTab => {
                self.hide_tab_bar_if_only_one_tab.to_string()
            }
            NativeLuaWindowEffectiveConfigField::WarnAboutMissingGlyphs => {
                self.warn_about_missing_glyphs.to_string()
            }
            NativeLuaWindowEffectiveConfigField::PaneFocusFollowsMouse => {
                self.pane_focus_follows_mouse.to_string()
            }
            NativeLuaWindowEffectiveConfigField::SwallowMouseClickOnPaneFocus => {
                self.swallow_mouse_click_on_pane_focus.to_string()
            }
            NativeLuaWindowEffectiveConfigField::SwallowMouseClickOnWindowFocus => {
                self.swallow_mouse_click_on_window_focus.to_string()
            }
            NativeLuaWindowEffectiveConfigField::BypassMouseReportingModifiers => {
                native_lua_keyboard_modifiers_text(self.bypass_mouse_reporting_modifiers)
            }
            NativeLuaWindowEffectiveConfigField::UnzoomOnSwitchPane => {
                self.unzoom_on_switch_pane.to_string()
            }
            NativeLuaWindowEffectiveConfigField::QuitWhenAllWindowsAreClosed => {
                self.quit_when_all_windows_are_closed.to_string()
            }
            NativeLuaWindowEffectiveConfigField::DefaultCursorStyle => {
                self.default_cursor_style.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::ForceReverseVideoCursor => {
                self.force_reverse_video_cursor.to_string()
            }
            NativeLuaWindowEffectiveConfigField::ReverseVideoCursorMinContrast => {
                self.reverse_video_cursor_min_contrast.as_f64().to_string()
            }
            NativeLuaWindowEffectiveConfigField::TextMinContrastRatio => self
                .text_min_contrast_ratio
                .map(|ratio| ratio.as_f64().to_string())
                .unwrap_or_default(),
            field => self.lua_window_effective_config_field_text_part4(&field),
        }
    }

    #[expect(
    clippy::too_many_lines,
    reason = "the compatibility reducer remains linear to preserve evaluation and precedence order"
)]
    fn lua_window_effective_config_field_text_part4(
        &self,
        field: &NativeLuaWindowEffectiveConfigField,
    ) -> String {
        match field {
            NativeLuaWindowEffectiveConfigField::CommandPaletteRows => self
                .command_palette_rows
                .map(|rows| rows.to_string())
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::CommandPaletteFontSize => {
                self.command_palette_font_size.config_text()
            }
            NativeLuaWindowEffectiveConfigField::CommandPaletteBgColor => self
                .command_palette_bg_color
                .map(native_lua_color_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::CommandPaletteFgColor => self
                .command_palette_fg_color
                .map(native_lua_color_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::CharSelectFontSize => {
                self.char_select_font_size.config_text()
            }
            NativeLuaWindowEffectiveConfigField::CharSelectBgColor => self
                .char_select_bg_color
                .map(native_lua_color_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::CharSelectFgColor => self
                .char_select_fg_color
                .map(native_lua_color_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::PaneSelectFontSize => {
                self.pane_select_font_size.config_text()
            }
            NativeLuaWindowEffectiveConfigField::PaneSelectBgColor => self
                .pane_select_bg_color
                .map(native_lua_color_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::PaneSelectFgColor => self
                .pane_select_fg_color
                .map(native_lua_color_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::LauncherAlphabet => self.launcher_alphabet.clone(),
            NativeLuaWindowEffectiveConfigField::QuickSelectAlphabet => {
                self.quick_select_alphabet.clone()
            }
            NativeLuaWindowEffectiveConfigField::QuickSelectPattern(index) => index
                .checked_sub(1)
                .and_then(|offset| self.quick_select_patterns.get(offset))
                .cloned()
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::HyperlinkRule(index, field) => index
                .checked_sub(1)
                .and_then(|offset| self.hyperlink_rules.get(offset))
                .map(|rule| match field {
                    NativeLuaHyperlinkRuleField::Regex => rule.regex.clone(),
                    NativeLuaHyperlinkRuleField::Format => rule.format.clone(),
                    NativeLuaHyperlinkRuleField::Highlight => rule.highlight.to_string(),
                })
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::ColorSchemeDir(index) => index
                .checked_sub(1)
                .and_then(|offset| self.color_scheme_dirs.get(offset))
                .cloned()
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::CleanExitCode(index) => index
                .checked_sub(1)
                .and_then(|offset| self.clean_exit_codes.get(offset))
                .map(std::string::ToString::to_string)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::DisableDefaultQuickSelectPatterns => {
                self.disable_default_quick_select_patterns.to_string()
            }
            NativeLuaWindowEffectiveConfigField::QuickSelectRemoveStyling => {
                self.quick_select_remove_styling.to_string()
            }
            NativeLuaWindowEffectiveConfigField::CanonicalizePastedNewlines => {
                self.canonicalize_pasted_newlines.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::QuoteDroppedFiles => {
                self.quote_dropped_files.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::DisableDefaultKeyBindings => {
                self.disable_default_key_bindings.to_string()
            }
            NativeLuaWindowEffectiveConfigField::DisableDefaultMouseBindings => {
                self.disable_default_mouse_bindings.to_string()
            }
            NativeLuaWindowEffectiveConfigField::DebugKeyEvents => {
                self.debug_key_events.to_string()
            }
            NativeLuaWindowEffectiveConfigField::KeyMapPreference => {
                self.key_map_preference.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::UiKeyCapRendering => {
                self.ui_key_cap_rendering.config_text().to_string()
            }
            NativeLuaWindowEffectiveConfigField::SwapBackspaceAndDelete => {
                self.swap_backspace_and_delete.to_string()
            }
            NativeLuaWindowEffectiveConfigField::LogUnknownEscapeSequences => {
                self.log_unknown_escape_sequences.to_string()
            }
            NativeLuaWindowEffectiveConfigField::DefaultSshAuthSock => {
                self.default_ssh_auth_sock.clone().unwrap_or_default()
            }
            NativeLuaWindowEffectiveConfigField::MuxEnableSshAgent => {
                self.mux_enable_ssh_agent.to_string()
            }
            NativeLuaWindowEffectiveConfigField::DetectPasswordInput => {
                self.detect_password_input.to_string()
            }
            NativeLuaWindowEffectiveConfigField::EnableScrollBar => {
                self.enable_scroll_bar.to_string()
            }
            NativeLuaWindowEffectiveConfigField::MinScrollBarHeight => self
                .min_scroll_bar_height
                .map_or_else(String::new, NativeScrollBarHeight::config_text),
            NativeLuaWindowEffectiveConfigField::CustomBlockGlyphs => {
                self.custom_block_glyphs.to_string()
            }
            NativeLuaWindowEffectiveConfigField::AntiAliasCustomBlockGlyphs => {
                self.anti_alias_custom_block_glyphs.to_string()
            }
            NativeLuaWindowEffectiveConfigField::WindowPaddingLeft => {
                self.window_padding.left.config_text()
            }
            NativeLuaWindowEffectiveConfigField::WindowPaddingRight => {
                self.window_padding.right.config_text()
            }
            NativeLuaWindowEffectiveConfigField::WindowPaddingTop => {
                self.window_padding.top.config_text()
            }
            NativeLuaWindowEffectiveConfigField::WindowPaddingBottom => {
                self.window_padding.bottom.config_text()
            }
            NativeLuaWindowEffectiveConfigField::WindowContentAlignmentHorizontal => {
                match self.window_content_alignment.horizontal {
                    NativeHorizontalContentAlignment::Left => "Left",
                    NativeHorizontalContentAlignment::Center => "Center",
                    NativeHorizontalContentAlignment::Right => "Right",
                }
                .to_string()
            }
            NativeLuaWindowEffectiveConfigField::WindowContentAlignmentVertical => {
                match self.window_content_alignment.vertical {
                    NativeVerticalContentAlignment::Top => "Top",
                    NativeVerticalContentAlignment::Center => "Center",
                    NativeVerticalContentAlignment::Bottom => "Bottom",
                }
                .to_string()
            }
            NativeLuaWindowEffectiveConfigField::KdeWindowBackgroundBlur => {
                self.kde_window_background_blur.to_string()
            }
            NativeLuaWindowEffectiveConfigField::MacosWindowBackgroundBlur => {
                self.macos_window_background_blur.to_string()
            }
            NativeLuaWindowEffectiveConfigField::Win32SystemBackdrop => self
                .win32_system_backdrop
                .as_wezterm_config_value()
                .to_string(),
            NativeLuaWindowEffectiveConfigField::Win32AcrylicAccentColor => self
                .win32_acrylic_accent_color
                .map(native_lua_color_config_text)
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::WindowDecorations => {
                self.window_decorations.as_wezterm_config_value()
            }
            NativeLuaWindowEffectiveConfigField::IntegratedTitleButton(index) => index
                .checked_sub(1)
                .and_then(|offset| self.integrated_title_buttons.get(offset).copied())
                .map(|button| button.as_wezterm_config_value().to_owned())
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::IntegratedTitleButtonAlignment => self
                .integrated_title_button_alignment
                .as_wezterm_config_value()
                .to_string(),
            NativeLuaWindowEffectiveConfigField::IntegratedTitleButtonColor => {
                match self.integrated_title_button_color {
                    NativeIntegratedTitleButtonColor::Auto => "Auto".to_owned(),
                    NativeIntegratedTitleButtonColor::Color(color) => {
                        native_lua_color_config_text(color)
                    }
                }
            }
            NativeLuaWindowEffectiveConfigField::IntegratedTitleButtonStyle => self
                .integrated_title_button_style
                .as_wezterm_config_value()
                .to_string(),
            NativeLuaWindowEffectiveConfigField::NativeMacosFullscreenMode => {
                self.native_macos_fullscreen_mode.to_string()
            }
            NativeLuaWindowEffectiveConfigField::MacosFullscreenExtendBehindNotch => {
                self.macos_fullscreen_extend_behind_notch.to_string()
            }
            NativeLuaWindowEffectiveConfigField::SelectionWordBoundary => {
                self.selection_word_boundary.clone()
            }
            NativeLuaWindowEffectiveConfigField::Term => self.term.clone(),
            NativeLuaWindowEffectiveConfigField::EnqAnswerback => self.enq_answerback.clone(),
            NativeLuaWindowEffectiveConfigField::InitialCols => self.initial_cols.to_string(),
            NativeLuaWindowEffectiveConfigField::InitialRows => self.initial_rows.to_string(),
            NativeLuaWindowEffectiveConfigField::ScrollbackLines => {
                self.scrollback_lines.to_string()
            }
            NativeLuaWindowEffectiveConfigField::SwitchToLastActiveTabWhenClosingTab => {
                self.switch_to_last_active_tab_when_closing_tab.to_string()
            }
            NativeLuaWindowEffectiveConfigField::ExitBehavior => {
                self.exit_behavior.as_wezterm_config_value().to_owned()
            }
            NativeLuaWindowEffectiveConfigField::ExitBehaviorMessaging => self
                .exit_behavior_messaging
                .as_wezterm_config_value()
                .to_owned(),
            NativeLuaWindowEffectiveConfigField::AdjustWindowSizeWhenChangingFontSize => {
                self.adjust_window_size_when_changing_font_size.to_string()
            }
            NativeLuaWindowEffectiveConfigField::TilingDesktopEnvironment(index) => index
                .checked_sub(1)
                .and_then(|offset| self.tiling_desktop_environments.get(offset))
                .cloned()
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::SkipCloseConfirmationProcess(index) => index
                .checked_sub(1)
                .and_then(|offset| self.skip_close_confirmation_for_processes_named.get(offset))
                .cloned()
                .unwrap_or_default(),
            NativeLuaWindowEffectiveConfigField::UseResizeIncrements => {
                self.use_resize_increments.to_string()
            }
            NativeLuaWindowEffectiveConfigField::AlternateBufferWheelScrollSpeed => {
                self.alternate_buffer_wheel_scroll_speed.to_string()
            }
            _ => unreachable!("effective config field was routed to the wrong formatter stage"),
        }
    }

    fn lua_pane_dimensions_field_text(&self, field: NativeLuaPaneDimensionsField) -> String {
        let terminal = self.runtime.terminal();
        let dimensions = terminal.stable_dimensions();
        match field {
            NativeLuaPaneDimensionsField::Cols => terminal.grid().size().columns.to_string(),
            NativeLuaPaneDimensionsField::ViewportRows => dimensions.viewport_rows.to_string(),
            NativeLuaPaneDimensionsField::ScrollbackRows => dimensions.scrollback_rows.to_string(),
            NativeLuaPaneDimensionsField::PhysicalTop => dimensions.physical_top.to_string(),
            NativeLuaPaneDimensionsField::ScrollbackTop => dimensions.scrollback_top.to_string(),
        }
    }

    fn lua_pane_cursor_position_field_text(
        &self,
        field: NativeLuaPaneCursorPositionField,
    ) -> String {
        let terminal = self.runtime.terminal();
        let (row, column) = terminal.cursor();
        let dimensions = terminal.stable_dimensions();
        match field {
            NativeLuaPaneCursorPositionField::X => column.to_string(),
            NativeLuaPaneCursorPositionField::Y => dimensions
                .physical_top
                .saturating_add(StableRowIndex::try_from(row).unwrap_or(StableRowIndex::MAX))
                .to_string(),
            NativeLuaPaneCursorPositionField::Shape => {
                native_lua_cursor_shape_text(terminal.cursor_shape()).to_owned()
            }
            NativeLuaPaneCursorPositionField::Visibility => {
                if terminal.cursor_visible() {
                    "Visible".to_owned()
                } else {
                    "Hidden".to_owned()
                }
            }
        }
    }

    fn lua_pane_user_var_text(&self, name: &str, fallback: String) -> String {
        self.pane_user_var(self.app_shell.active_pane_id(), name)
            .map_or(fallback, ToOwned::to_owned)
    }

    fn lua_pane_progress_text(
        &self,
        none: String,
        percentage_prefix: &str,
        error_prefix: &str,
        indeterminate: String,
    ) -> String {
        match self
            .pane_progress(self.app_shell.active_pane_id())
            .unwrap_or(PaneProgress::None)
        {
            PaneProgress::None => none,
            PaneProgress::Percentage(value) => format!("{percentage_prefix}{value}"),
            PaneProgress::Error(value) => format!("{error_prefix}{value}"),
            PaneProgress::Indeterminate => indeterminate,
        }
    }

    fn composition_status_text(&self) -> Option<String> {
        if let Some(preedit) = self.ime_preedit.as_deref().filter(|text| !text.is_empty()) {
            return Some(preedit.to_owned());
        }
        if self.dead_key_active {
            return Some(self.dead_key_text.clone().unwrap_or_default());
        }
        None
    }

    #[allow(dead_code)]
    fn set_left_status(&mut self, status: String) {
        self.left_status = status;
        self.apply_window_title();
    }

    #[allow(dead_code)]
    fn set_right_status(&mut self, status: String) {
        self.right_status = status;
        self.apply_window_title();
    }

    fn dispatch_update_status_if_due(&mut self, now: Instant) -> bool {
        if self.last_status_update_at.is_some_and(|last| {
            now.checked_duration_since(last)
                .is_some_and(|elapsed| elapsed < self.status_update_interval)
        }) {
            return false;
        }

        self.dispatch_update_status();
        self.last_status_update_at = Some(now);
        true
    }

    fn update_cursor_blink_phase_if_due(&mut self, now: Instant) -> bool {
        if self.cursor_blink_rate.is_zero() {
            if self.cursor_blink_opacity_alpha != u8::MAX {
                self.apply_cursor_blink_opacity(u8::MAX);
                self.last_cursor_blink_at = None;
                return true;
            }
            self.last_cursor_blink_at = None;
            return false;
        }

        let Some(last) = self.last_cursor_blink_at else {
            self.last_cursor_blink_at = Some(now);
            return false;
        };

        let Some(elapsed) = now.checked_duration_since(last) else {
            return false;
        };
        let alpha = cursor_blink_opacity_alpha(
            elapsed,
            self.cursor_blink_rate,
            self.cursor_blink_ease_in,
            self.cursor_blink_ease_out,
        );
        if alpha == self.cursor_blink_opacity_alpha {
            return false;
        }

        self.apply_cursor_blink_opacity(alpha);
        true
    }

    fn apply_cursor_blink_opacity(&mut self, alpha: u8) {
        self.cursor_blink_opacity_alpha = alpha;
        self.cursor_blink_visible = alpha > 0;
        self.renderer
            .set_cursor_opacity(f32::from(alpha) / f32::from(u8::MAX));
        self.frame_needs_full_repaint = true;
    }

    fn update_text_blink_phase_if_due(&mut self, now: Instant) -> bool {
        let text_blink_rate = self.text_blink_rate;
        let text_blink_ease_in = self.text_blink_ease_in;
        let text_blink_ease_out = self.text_blink_ease_out;
        let text_blink_rate_rapid = self.text_blink_rate_rapid;
        let text_blink_rapid_ease_in = self.text_blink_rapid_ease_in;
        let text_blink_rapid_ease_out = self.text_blink_rapid_ease_out;
        let text_blink_opacity_alpha = self.text_blink_opacity_alpha;
        let rapid_text_blink_opacity_alpha = self.rapid_text_blink_opacity_alpha;
        let mut changed = false;
        if let Some(alpha) = blink_opacity_alpha_if_changed(
            now,
            &mut self.last_text_blink_at,
            text_blink_rate,
            text_blink_ease_in,
            text_blink_ease_out,
            text_blink_opacity_alpha,
        ) {
            self.apply_text_blink_opacity(alpha);
            changed = true;
        }
        if let Some(alpha) = blink_opacity_alpha_if_changed(
            now,
            &mut self.last_rapid_text_blink_at,
            text_blink_rate_rapid,
            text_blink_rapid_ease_in,
            text_blink_rapid_ease_out,
            rapid_text_blink_opacity_alpha,
        ) {
            self.apply_rapid_text_blink_opacity(alpha);
            changed = true;
        }
        changed
    }

    fn apply_text_blink_opacity(&mut self, alpha: u8) {
        self.text_blink_opacity_alpha = alpha;
        self.renderer
            .set_text_blink_opacity(f32::from(alpha) / f32::from(u8::MAX));
        self.frame_needs_full_repaint = true;
    }

    fn apply_rapid_text_blink_opacity(&mut self, alpha: u8) {
        self.rapid_text_blink_opacity_alpha = alpha;
        self.renderer
            .set_rapid_text_blink_opacity(f32::from(alpha) / f32::from(u8::MAX));
        self.frame_needs_full_repaint = true;
    }

    fn dispatch_bells(&mut self, pane: rssh_core::PaneId, count: u64) {
        for _ in 0..count {
            let bell = NativeWindowBell {
                window_id: self.app_window_id,
                pane,
            };
            self.record_visual_bell(pane);
            self.ring_audible_bell(bell);
            self.dispatch_bell(bell);
        }
    }

    fn record_visual_bell(&mut self, pane: rssh_core::PaneId) {
        if !self.visual_bell.is_enabled() {
            return;
        }

        self.visual_bell_started_at.insert(pane, Instant::now());
        self.frame_needs_full_repaint = true;
    }

    fn ring_audible_bell(&mut self, bell: NativeWindowBell) -> bool {
        match self.audible_bell {
            NativeAudibleBell::SystemBeep => (self.audible_bell_handler)(&bell),
            NativeAudibleBell::Disabled => false,
        }
    }

    fn dispatch_bell(&mut self, bell: NativeWindowBell) -> bool {
        let mut handled = (self.bell_handler)(&bell);
        if let Some(update) = self.lua_bell.clone() {
            if let Some(left_status) = update.left_status {
                self.left_status = self.lua_window_status_text(left_status);
                handled = true;
            }
            if let Some(right_status) = update.right_status {
                self.right_status = self.lua_window_status_text(right_status);
                handled = true;
            }
        }
        handled
    }

    fn dispatch_focus_change(&mut self, change: &NativeWindowFocusChange) -> bool {
        let mut handled = (self.focus_change_handler)(change);
        if let Some(update) = self.lua_focus_changed.clone() {
            if let Some(left_status) = update.left_status {
                self.left_status = self.lua_window_status_text(left_status);
                handled = true;
            }
            if let Some(right_status) = update.right_status {
                self.right_status = self.lua_window_status_text(right_status);
                handled = true;
            }
        }
        handled
    }

    fn dispatch_resize(&mut self, resize: &NativeWindowResize) -> bool {
        let mut handled = (self.resize_handler)(resize);
        if let Some(update) = self.lua_resized.clone() {
            if let Some(left_status) = update.left_status {
                self.left_status = self.lua_window_status_text(left_status);
                handled = true;
            }
            if let Some(right_status) = update.right_status {
                self.right_status = self.lua_window_status_text(right_status);
                handled = true;
            }
        }
        handled
    }

    fn dispatch_user_var_change(&mut self, change: &NativeWindowUserVarChange) -> bool {
        let mut handled = (self.user_var_change_handler)(change);
        if let Some(update) = self.lua_user_var_changed.clone() {
            if let Some(left_status) = update.left_status {
                self.left_status = self.lua_user_var_changed_status_text(left_status, change);
                handled = true;
            }
            if let Some(right_status) = update.right_status {
                self.right_status = self.lua_user_var_changed_status_text(right_status, change);
                handled = true;
            }
        }
        handled
    }

    fn lua_user_var_changed_status_text(
        &self,
        status: NativeLuaUserVarChangedStatusText,
        change: &NativeWindowUserVarChange,
    ) -> String {
        status
            .parts
            .into_iter()
            .map(|part| match part {
                NativeLuaUserVarChangedStatusPart::Static(text) => text,
                NativeLuaUserVarChangedStatusPart::WindowId => change.window_id.get().to_string(),
                NativeLuaUserVarChangedStatusPart::PaneId => change.pane.get().to_string(),
                NativeLuaUserVarChangedStatusPart::PaneUserVar {
                    source,
                    name,
                    fallback,
                } => {
                    let pane = match source {
                        NativeLuaUserVarChangedPaneUserVarSource::EventPane => change.pane,
                        NativeLuaUserVarChangedPaneUserVarSource::ActivePane => {
                            self.app_shell.active_pane_id()
                        }
                    };
                    self.pane_user_var(pane, &name)
                        .map_or(fallback, str::to_owned)
                }
                NativeLuaUserVarChangedStatusPart::Name => change.name.clone(),
                NativeLuaUserVarChangedStatusPart::Value => change.value.clone(),
            })
            .collect()
    }

    fn dispatch_config_reloaded(&mut self, event: &NativeWindowConfigReloaded) -> bool {
        let mut handled = (self.config_reloaded_handler)(event);
        if let Some(update) = self.lua_config_reloaded.clone() {
            if let Some(left_status) = update.left_status {
                self.left_status = self.lua_window_status_text(left_status);
                handled = true;
            }
            if let Some(right_status) = update.right_status {
                self.right_status = self.lua_window_status_text(right_status);
                handled = true;
            }
        }
        handled
    }

    fn dispatch_command_palette_augment(
        &mut self,
        event: &NativeCommandPaletteAugment,
    ) -> Vec<NativeCommandPaletteEntry> {
        let mut entries = (self.command_palette_augmenter)(event);
        entries.extend(self.lua_command_palette_entries.iter().cloned());
        entries
    }

    fn reload_configuration(&mut self) {
        self.clear_key_table_stack();
        self.leader_active_since = None;
        let event = NativeWindowConfigReloaded {
            window_id: self.app_window_id,
            pane: self.app_shell.active_pane_id(),
        };
        self.dispatch_config_reloaded(&event);
    }

    fn request_reload_configuration(&mut self) {
        if let Some(sender) = &self.reload_request_sender
            && sender(WindowUserEvent::ReloadConfigurationRequested)
        {
            return;
        }
        self.reload_configuration();
    }

    fn answer_clipboard_query(&mut self, selection: &str) -> io::Result<bool> {
        let Some(text) = self.read_clipboard_text() else {
            return Ok(false);
        };

        let response = encode_osc52_clipboard_response(selection, &text);
        self.write_pty_bytes(&response)?;
        Ok(true)
    }

    fn selected_text(&self) -> Option<String> {
        if let Some(quick_select) = self.active_ui.quick_select() {
            return quick_select.current_match().and_then(|matched| {
                let text = matched.text_from_terminal(self.runtime.terminal())?;
                (!text.is_empty()).then_some(text)
            });
        }

        match self.active_ui.copy_search_mode() {
            Some(WindowCopySearchMode::Search) => {
                return self
                    .active_ui
                    .search()
                    .and_then(|search| search.current)
                    .and_then(|matched| {
                        let text = matched.text_from_terminal(self.runtime.terminal())?;
                        (!text.is_empty()).then_some(text)
                    });
            }
            Some(WindowCopySearchMode::Copy) => {
                let copy_text = self.active_ui.copy_mode().and_then(|copy_mode| {
                    let selection = copy_mode_source_selection(
                        copy_mode,
                        self.runtime.terminal(),
                        &self.selection_word_boundary,
                    )?;
                    let text = selection.text_from_terminal(self.runtime.terminal())?;
                    (!text.is_empty()).then_some(text)
                });
                if copy_text.is_some() {
                    return copy_text;
                }
                return self
                    .active_ui
                    .retained_search()
                    .and_then(|search| search.current)
                    .and_then(|matched| {
                        let text = matched.text_from_terminal(self.runtime.terminal())?;
                        (!text.is_empty()).then_some(text)
                    });
            }
            None => {}
        }

        self.ordinary_selected_text()
    }

    fn ordinary_selected_text(&self) -> Option<String> {
        let selection = self.active_ui.ordinary_selection?;
        let text = selection.text_from_terminal(self.runtime.terminal())?;
        (!text.is_empty()).then_some(text)
    }

    fn handle_window_paste(&mut self) -> io::Result<bool> {
        self.handle_window_paste_from(WindowPasteSource::Clipboard)
    }

    fn handle_window_primary_selection_paste(&mut self) -> io::Result<bool> {
        self.handle_window_paste_from(WindowPasteSource::PrimarySelection)
    }

    fn handle_window_paste_from(&mut self, source: WindowPasteSource) -> io::Result<bool> {
        if self.pane_inspection_input_barrier_active() {
            return Ok(true);
        }
        let text = match source {
            WindowPasteSource::Clipboard => self.read_clipboard_text(),
            WindowPasteSource::PrimarySelection => self.read_primary_selection_text(),
        };
        let Some(text) = text else {
            return Ok(false);
        };
        if text.is_empty() {
            return Ok(false);
        }

        let bytes = encode_window_paste(
            &text,
            self.runtime.bracketed_paste(),
            self.canonicalize_pasted_newlines,
        );
        self.write_pty_bytes(&bytes)?;
        Ok(true)
    }

    fn handle_dropped_file_path(&mut self, path: &Path) -> io::Result<bool> {
        if self.pane_inspection_input_barrier_active() {
            return Ok(true);
        }
        let path = path.to_string_lossy();
        if path.is_empty() {
            return Ok(false);
        }

        let quoted = quote_dropped_file_name(&path, self.quote_dropped_files);
        self.write_pty_bytes(quoted.as_bytes())?;
        Ok(true)
    }

    fn handle_focus_changed(&mut self, focused: bool) -> io::Result<bool> {
        if self.window_focused == focused {
            return Ok(false);
        }

        self.window_focused = focused;
        self.mouse_click_may_focus_window = focused;
        if !focused && let Some(UiKeyReleasePending::FullBarrier(key)) = self.ui_key_release_pending
        {
            self.ui_key_release_pending = Some(UiKeyReleasePending::MatchingReleaseOnly(key));
        }
        if !focused && self.tab_bar_drag.take().is_some() {
            self.ui_left_release_pending = true;
        }

        let change = NativeWindowFocusChange {
            window_id: self.app_window_id,
            pane: self.app_shell.active_pane_id(),
            focused,
        };
        self.dispatch_focus_change(&change);
        self.dispatch_update_status();

        if let Some(bytes) = encode_window_focus_event(focused, self.runtime.focus_reporting()) {
            self.write_pty_bytes(&bytes)?;
        }

        Ok(true)
    }

    fn handle_window_moved(&mut self, position: PhysicalPosition<i32>) {
        self.window_frame.set_position(position);
    }

    fn refresh_window_frame_from_window(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if let Ok(position) = window.outer_position() {
            self.window_frame.set_position(position);
        }
        self.window_frame.set_size(window.outer_size());
    }

    fn resize_presentation_surface(
        &mut self,
        size: PhysicalSize<u32>,
    ) -> Result<(), Box<dyn Error>> {
        let gpu_resize_error = self
            .gpu_owners.active
            .as_mut()
            .and_then(|gpu| gpu.resize_surface(size).err());
        if let Some(error) = gpu_resize_error {
            if self.bootstrap_surface.is_some() {
                eprintln!("GPU surface resize failed; using CPU fallback: {error}");
                self.activate_cpu_fallback();
            } else {
                return Err(error);
            }
        }
        if let Some(surface) = self.bootstrap_surface.as_mut()
            && size.width > 0
            && size.height > 0
        {
            surface.resize(size)?;
        }
        Ok(())
    }

    fn apply_pane_exit_behavior(
        &mut self,
        pane_id: rssh_core::PaneId,
        status: &PtyExitStatus,
    ) -> bool {
        if !self.exit_behavior_closes_pane(status) {
            self.write_exit_behavior_message(pane_id, status);
            self.apply_window_title();
            return false;
        }

        self.close_pane_after_exit(pane_id)
    }

    fn apply_pane_exit_behavior_after_exit(
        &mut self,
        pane_id: rssh_core::PaneId,
        status: Option<PtyExitStatus>,
    ) -> bool {
        let Some(status) = status else {
            return if matches!(self.exit_behavior, NativeExitBehavior::Close) {
                self.close_pane_after_exit(pane_id)
            } else {
                self.apply_window_title();
                false
            };
        };

        self.metrics.record_exit_status(&status);
        self.apply_pane_exit_behavior(pane_id, &status)
    }

    fn defer_automatic_close_for_frame_limit(&mut self, close_window: bool) -> bool {
        if close_window && self.frame_limit_probe_pending() {
            // `--frames` is a bounded presentation/probe contract. A short
            // child can exit before the requested frames are presented or a
            // requested PTY linkage marker reaches the terminal snapshot, so
            // keep the last pane/window alive until both conditions hold.
            // Normal application sessions have no frame limit and keep their
            // configured exit behavior unchanged.
            let _ = self.take_window_close_request();
            return false;
        }

        close_window
    }

    fn close_pane_after_exit(&mut self, pane_id: rssh_core::PaneId) -> bool {
        if let Err(error) = self.dispatch_close_pane_action(pane_id) {
            eprintln!("pane exit close action failed: {error:?}");
        }
        self.window_close_requested
    }

    fn write_exit_behavior_message(&mut self, pane_id: rssh_core::PaneId, status: &PtyExitStatus) {
        let Some(message) = self.exit_behavior_message(pane_id, status) else {
            return;
        };

        if let Err(error) = self.handle_pane_pty_output(pane_id, message.as_bytes()) {
            eprintln!("pane exit message write failed: {error:?}");
        }
    }

    fn exit_behavior_message(
        &self,
        pane_id: rssh_core::PaneId,
        status: &PtyExitStatus,
    ) -> Option<String> {
        if matches!(
            self.exit_behavior_messaging,
            NativeExitBehaviorMessaging::None
        ) {
            return None;
        }

        let process_name = self.pane_process_name(pane_id);
        let domain_name = self
            .app_shell
            .active_workspace()
            .tabs()
            .iter()
            .flat_map(rssh_core::app_shell::Tab::panes)
            .find(|pane| pane.id() == pane_id)
            .map_or(DEFAULT_DOMAIN_NAME, |pane| pane_launch_domain_name(pane.launch()));
        let code = status.exit_code();
        let clean = self.is_clean_exit_status(status);
        let exit_behavior = self.exit_behavior.as_wezterm_config_value();
        match self.exit_behavior_messaging {
            NativeExitBehaviorMessaging::Verbose if clean => Some(format!(
                "👍 Process \"{process_name}\" in domain \"{domain_name}\" completed.\r\n\
                 This message is shown because exit_behavior=\"{exit_behavior}\"\r\n"
            )),
            NativeExitBehaviorMessaging::Verbose => Some(format!(
                "⚠️  Process \"{process_name}\" in domain \"{domain_name}\" didn't exit cleanly\r\n\
                 Exited with code {code}\r\n\
                 This message is shown because exit_behavior=\"{exit_behavior}\"\r\n"
            )),
            NativeExitBehaviorMessaging::Brief if clean => Some(format!(
                "👍 Process \"{process_name}\" in domain \"{domain_name}\" completed.\r\n"
            )),
            NativeExitBehaviorMessaging::Brief => Some(format!(
                "⚠️  Process \"{process_name}\" in domain \"{domain_name}\" didn't exit cleanly\r\n\
                 Exited with code {code}\r\n"
            )),
            NativeExitBehaviorMessaging::Terse if clean => Some("[done]\r\n".to_owned()),
            NativeExitBehaviorMessaging::Terse => Some(format!("[Exited with code {code}]\r\n")),
            NativeExitBehaviorMessaging::None => None,
        }
    }

    fn pane_process_name(&self, pane_id: rssh_core::PaneId) -> String {
        self.app_shell
            .active_workspace()
            .tabs()
            .iter()
            .flat_map(rssh_core::app_shell::Tab::panes)
            .find(|pane| pane.id() == pane_id)
            .map(|pane| pane_launch_display_program(pane.launch()).to_owned())
            .unwrap_or_default()
    }

    fn is_clean_exit_status(&self, status: &PtyExitStatus) -> bool {
        status.success() || self.clean_exit_codes.contains(&status.exit_code())
    }

    fn exit_behavior_closes_pane(&self, status: &PtyExitStatus) -> bool {
        match self.exit_behavior {
            NativeExitBehavior::Close => true,
            NativeExitBehavior::Hold => false,
            NativeExitBehavior::CloseOnCleanExit => self.is_clean_exit_status(status),
        }
    }

    fn native_window_resize_event(
        &self,
        pixel_width: u32,
        pixel_height: u32,
        terminal_size: rssh_core::TerminalSize,
    ) -> NativeWindowResize {
        NativeWindowResize {
            window_id: self.app_window_id,
            pane: self.app_shell.active_pane_id(),
            pixel_width,
            pixel_height,
            terminal_size,
            is_full_screen: self.full_screen,
        }
    }
}
