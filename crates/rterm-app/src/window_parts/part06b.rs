macro_rules! native_window_config_patch_values_are_empty {
    ($value:ident; $($field:ident),+ $(,)?) => {
        true $(&& $value.$field.is_none())+
    };
}
macro_rules! merge_native_window_config_patch_values {
    ($target:ident, $update:ident; $($field:ident),+ $(,)?) => {
        $(if $update.$field.is_some() {
            $target.$field = $update.$field;
        })+
    };
}

macro_rules! apply_native_window_config_patch_values {
    ($values:ident, $overrides:ident; $($field:ident),+ $(,)?) => {
        $(if let Some(value) = $values.$field {
            $overrides.$field = Some(value);
        })+
    };
}
impl NativeWindowConfigPatchValues {
    fn is_empty(&self) -> bool {
        native_window_config_patch_values_are_empty!(self; dpi, dpi_by_screen, font, font_fallbacks, font_attributes, font_rules, font_size, cell_width, cell_widths, line_height, font_antialias, font_hinting, font_rasterizer, font_colr_rasterizer, font_shaper, harfbuzz_features, font_dirs, font_locator, use_cap_height_to_scale_fallback_fonts, ignore_svg_fonts, sort_fallback_fonts_by_coverage, search_font_dirs_for_fallback, custom_block_glyphs, anti_alias_custom_block_glyphs, allow_square_glyphs_to_overflow_width, freetype_load_target, freetype_render_target, freetype_load_flags, freetype_interpreter_version, freetype_pcf_long_family_names, display_pixel_geometry, foreground_text_hsb, text_background_opacity, window_background_opacity, background, window_background_image, window_background_image_hsb, window_background_gradient, window_background_images, window_background_layers, kde_window_background_blur, macos_window_background_blur, win32_system_backdrop) && self.next.is_empty()
    }

    fn merge(&mut self, update: Self) {
        merge_native_window_config_patch_values!(self, update; dpi, dpi_by_screen, font, font_fallbacks, font_attributes, font_rules, font_size, cell_width, cell_widths, line_height, font_antialias, font_hinting, font_rasterizer, font_colr_rasterizer, font_shaper, harfbuzz_features, font_dirs, font_locator, use_cap_height_to_scale_fallback_fonts, ignore_svg_fonts, sort_fallback_fonts_by_coverage, search_font_dirs_for_fallback, custom_block_glyphs, anti_alias_custom_block_glyphs, allow_square_glyphs_to_overflow_width, freetype_load_target, freetype_render_target, freetype_load_flags, freetype_interpreter_version, freetype_pcf_long_family_names, display_pixel_geometry, foreground_text_hsb, text_background_opacity, window_background_opacity, background, window_background_image, window_background_image_hsb, window_background_gradient, window_background_images, window_background_layers, kde_window_background_blur, macos_window_background_blur, win32_system_backdrop);
        self.next.merge(update.next);
    }

    fn apply_to_native_config_overrides(self, overrides: &mut NativeConfigSnapshot) {
        apply_native_window_config_patch_values!(self, overrides; dpi, dpi_by_screen, font, font_fallbacks, font_attributes, font_rules, font_size, cell_width, cell_widths, line_height, font_antialias, font_hinting, font_rasterizer, font_colr_rasterizer, font_shaper, harfbuzz_features, font_dirs, font_locator, use_cap_height_to_scale_fallback_fonts, ignore_svg_fonts, sort_fallback_fonts_by_coverage, search_font_dirs_for_fallback, custom_block_glyphs, anti_alias_custom_block_glyphs, allow_square_glyphs_to_overflow_width, freetype_load_target, freetype_render_target, freetype_load_flags, freetype_interpreter_version, freetype_pcf_long_family_names, display_pixel_geometry, foreground_text_hsb, text_background_opacity, window_background_opacity, background, window_background_image, window_background_image_hsb, window_background_gradient, window_background_images, window_background_layers, kde_window_background_blur, macos_window_background_blur, win32_system_backdrop);
        self.next.apply_to_native_config_overrides(overrides);
    }
}

impl NativeWindowConfigPatchValues1 {
    fn is_empty(&self) -> bool {
        native_window_config_patch_values_are_empty!(self; win32_acrylic_accent_color, window_frame_appearance, inactive_pane_hsb, tab_max_width, tab_min_width, status_update_interval_ms, max_fps, animation_fps, front_end, webgpu_power_preference, webgpu_force_fallback_adapter, webgpu_preferred_adapter, prefer_egl, enable_wayland, enable_zwlr_output_manager, use_box_model_render, experimental_pixel_positioning, shape_cache_size, line_state_cache_size, line_quad_cache_size, line_to_ele_shape_cache_size, glyph_cache_image_cache_size, cursor_blink_rate_ms, cursor_blink_ease_in, cursor_blink_ease_out, text_blink_rate_ms, text_blink_rate_rapid_ms, text_blink_ease_in, text_blink_ease_out, text_blink_rapid_ease_in, text_blink_rapid_ease_out, bold_brightens_ansi_colors, default_cursor_style, cursor_thickness, underline_thickness, underline_position, strikethrough_position, force_reverse_video_cursor, reverse_video_cursor_min_contrast, text_min_contrast_ratio, window_decorations, integrated_title_buttons, integrated_title_button_alignment, integrated_title_button_color) && self.next.is_empty()
    }

    fn merge(&mut self, update: Self) {
        merge_native_window_config_patch_values!(self, update; win32_acrylic_accent_color, window_frame_appearance, inactive_pane_hsb, tab_max_width, tab_min_width, status_update_interval_ms, max_fps, animation_fps, front_end, webgpu_power_preference, webgpu_force_fallback_adapter, webgpu_preferred_adapter, prefer_egl, enable_wayland, enable_zwlr_output_manager, use_box_model_render, experimental_pixel_positioning, shape_cache_size, line_state_cache_size, line_quad_cache_size, line_to_ele_shape_cache_size, glyph_cache_image_cache_size, cursor_blink_rate_ms, cursor_blink_ease_in, cursor_blink_ease_out, text_blink_rate_ms, text_blink_rate_rapid_ms, text_blink_ease_in, text_blink_ease_out, text_blink_rapid_ease_in, text_blink_rapid_ease_out, bold_brightens_ansi_colors, default_cursor_style, cursor_thickness, underline_thickness, underline_position, strikethrough_position, force_reverse_video_cursor, reverse_video_cursor_min_contrast, text_min_contrast_ratio, window_decorations, integrated_title_buttons, integrated_title_button_alignment, integrated_title_button_color);
        self.next.merge(update.next);
    }

    fn apply_to_native_config_overrides(self, overrides: &mut NativeConfigSnapshot) {
        apply_native_window_config_patch_values!(self, overrides; win32_acrylic_accent_color, window_frame_appearance, inactive_pane_hsb, tab_max_width, tab_min_width, status_update_interval_ms, max_fps, animation_fps, front_end, webgpu_power_preference, webgpu_force_fallback_adapter, webgpu_preferred_adapter, prefer_egl, enable_wayland, enable_zwlr_output_manager, use_box_model_render, experimental_pixel_positioning, shape_cache_size, line_state_cache_size, line_quad_cache_size, line_to_ele_shape_cache_size, glyph_cache_image_cache_size, cursor_blink_rate_ms, cursor_blink_ease_in, cursor_blink_ease_out, text_blink_rate_ms, text_blink_rate_rapid_ms, text_blink_ease_in, text_blink_ease_out, text_blink_rapid_ease_in, text_blink_rapid_ease_out, bold_brightens_ansi_colors, default_cursor_style, cursor_thickness, underline_thickness, underline_position, strikethrough_position, force_reverse_video_cursor, reverse_video_cursor_min_contrast, text_min_contrast_ratio, window_decorations, integrated_title_buttons, integrated_title_button_alignment, integrated_title_button_color);
        self.next.apply_to_native_config_overrides(overrides);
    }
}

impl NativeWindowConfigPatchValues2 {
    fn is_empty(&self) -> bool {
        native_window_config_patch_values_are_empty!(self; integrated_title_button_style, window_padding, window_content_alignment, initial_cols, initial_rows, adjust_window_size_when_changing_font_size, command_palette_rows, command_palette_font, command_palette_font_size, command_palette_bg_color, command_palette_fg_color, char_select_font, char_select_font_size, char_select_bg_color, char_select_fg_color, pane_select_font, pane_select_font_size, pane_select_bg_color, pane_select_fg_color, launcher_alphabet, quick_select_alphabet, quick_select_patterns, disable_default_quick_select_patterns, quick_select_remove_styling, hyperlink_rules, selection_word_boundary, default_prog, default_domain, prefer_to_spawn_tabs, set_environment_variables, default_gui_startup_args, default_workspace, native_macos_fullscreen_mode, macos_fullscreen_extend_behind_notch, use_resize_increments, default_cwd, default_ssh_auth_sock, default_mux_server_domain, daemon_options, exec_domains, wsl_domains, unix_domains, ssh_domains) && self.next.is_empty()
    }

    fn merge(&mut self, update: Self) {
        merge_native_window_config_patch_values!(self, update; integrated_title_button_style, window_padding, window_content_alignment, initial_cols, initial_rows, adjust_window_size_when_changing_font_size, command_palette_rows, command_palette_font, command_palette_font_size, command_palette_bg_color, command_palette_fg_color, char_select_font, char_select_font_size, char_select_bg_color, char_select_fg_color, pane_select_font, pane_select_font_size, pane_select_bg_color, pane_select_fg_color, launcher_alphabet, quick_select_alphabet, quick_select_patterns, disable_default_quick_select_patterns, quick_select_remove_styling, hyperlink_rules, selection_word_boundary, default_prog, default_domain, prefer_to_spawn_tabs, set_environment_variables, default_gui_startup_args, default_workspace, native_macos_fullscreen_mode, macos_fullscreen_extend_behind_notch, use_resize_increments, default_cwd, default_ssh_auth_sock, default_mux_server_domain, daemon_options, exec_domains, wsl_domains, unix_domains, ssh_domains);
        self.next.merge(update.next);
    }

    fn apply_to_native_config_overrides(self, overrides: &mut NativeConfigSnapshot) {
        apply_native_window_config_patch_values!(self, overrides; integrated_title_button_style, window_padding, window_content_alignment, initial_cols, initial_rows, adjust_window_size_when_changing_font_size, command_palette_rows, command_palette_font, command_palette_font_size, command_palette_bg_color, command_palette_fg_color, char_select_font, char_select_font_size, char_select_bg_color, char_select_fg_color, pane_select_font, pane_select_font_size, pane_select_bg_color, pane_select_fg_color, launcher_alphabet, quick_select_alphabet, quick_select_patterns, disable_default_quick_select_patterns, quick_select_remove_styling, hyperlink_rules, selection_word_boundary, default_prog, default_domain, prefer_to_spawn_tabs, set_environment_variables, default_gui_startup_args, default_workspace, native_macos_fullscreen_mode, macos_fullscreen_extend_behind_notch, use_resize_increments, default_cwd, default_ssh_auth_sock, default_mux_server_domain, daemon_options, exec_domains, wsl_domains, unix_domains, ssh_domains);
        self.next.apply_to_native_config_overrides(overrides);
    }
}

impl NativeWindowConfigPatchValues3 {
    fn is_empty(&self) -> bool {
        native_window_config_patch_values_are_empty!(self; tls_servers, tls_clients, serial_ports, mux_enable_ssh_agent, ssh_backend, ratelimit_mux_line_prefetches_per_second, mux_output_parser_buffer_size, mux_output_parser_coalesce_delay_ms, mux_env_remove, periodic_stat_logging, ulimit_nofile, ulimit_nproc, tiling_desktop_environments, launch_menu, term, enq_answerback, audible_bell, visual_bell, visual_bell_color, notification_handling, colors, color_scheme, color_scheme_dirs, color_schemes, foreground_color, background_color, ansi_palette, indexed_palette, selection_fg_color, selection_bg_color, cursor_bg_color, cursor_border_color, cursor_fg_color, compose_cursor_color, split_color, scrollbar_thumb_color, tab_bar_background_color, tab_bar_inactive_tab_edge_color, tab_bar_active_tab_colors, tab_bar_inactive_tab_colors, tab_bar_inactive_tab_hover_colors, tab_bar_new_tab_colors, tab_bar_new_tab_hover_colors) && self.next.is_empty()
    }

    fn merge(&mut self, update: Self) {
        merge_native_window_config_patch_values!(self, update; tls_servers, tls_clients, serial_ports, mux_enable_ssh_agent, ssh_backend, ratelimit_mux_line_prefetches_per_second, mux_output_parser_buffer_size, mux_output_parser_coalesce_delay_ms, mux_env_remove, periodic_stat_logging, ulimit_nofile, ulimit_nproc, tiling_desktop_environments, launch_menu, term, enq_answerback, audible_bell, visual_bell, visual_bell_color, notification_handling, colors, color_scheme, color_scheme_dirs, color_schemes, foreground_color, background_color, ansi_palette, indexed_palette, selection_fg_color, selection_bg_color, cursor_bg_color, cursor_border_color, cursor_fg_color, compose_cursor_color, split_color, scrollbar_thumb_color, tab_bar_background_color, tab_bar_inactive_tab_edge_color, tab_bar_active_tab_colors, tab_bar_inactive_tab_colors, tab_bar_inactive_tab_hover_colors, tab_bar_new_tab_colors, tab_bar_new_tab_hover_colors);
        self.next.merge(update.next);
    }

    fn apply_to_native_config_overrides(self, overrides: &mut NativeConfigSnapshot) {
        apply_native_window_config_patch_values!(self, overrides; tls_servers, tls_clients, serial_ports, mux_enable_ssh_agent, ssh_backend, ratelimit_mux_line_prefetches_per_second, mux_output_parser_buffer_size, mux_output_parser_coalesce_delay_ms, mux_env_remove, periodic_stat_logging, ulimit_nofile, ulimit_nproc, tiling_desktop_environments, launch_menu, term, enq_answerback, audible_bell, visual_bell, visual_bell_color, notification_handling, colors, color_scheme, color_scheme_dirs, color_schemes, foreground_color, background_color, ansi_palette, indexed_palette, selection_fg_color, selection_bg_color, cursor_bg_color, cursor_border_color, cursor_fg_color, compose_cursor_color, split_color, scrollbar_thumb_color, tab_bar_background_color, tab_bar_inactive_tab_edge_color);
        if let Some(value) = self.tab_bar_active_tab_colors {
            overrides.tab_bar_active_tab_colors = value;
        }
        if let Some(value) = self.tab_bar_inactive_tab_colors {
            overrides.tab_bar_inactive_tab_colors = value;
        }
        if let Some(value) = self.tab_bar_inactive_tab_hover_colors {
            overrides.tab_bar_inactive_tab_hover_colors = value;
        }
        if let Some(value) = self.tab_bar_new_tab_colors {
            overrides.tab_bar_new_tab_colors = value;
        }
        if let Some(value) = self.tab_bar_new_tab_hover_colors {
            overrides.tab_bar_new_tab_hover_colors = value;
        }
        self.next.apply_to_native_config_overrides(overrides);
    }
}

impl NativeWindowConfigPatchValues4 {
    fn is_empty(&self) -> bool {
        native_window_config_patch_values_are_empty!(self; tab_bar_style, copy_mode_active_highlight_fg, copy_mode_active_highlight_bg, copy_mode_inactive_highlight_fg, copy_mode_inactive_highlight_bg, quick_select_label_fg, quick_select_label_bg, quick_select_match_fg, quick_select_match_bg, input_selector_label_fg, input_selector_label_bg, launcher_label_fg, launcher_label_bg, automatically_reload_config, check_for_updates, check_for_updates_interval_seconds, show_update_window, key_map_preference, ui_key_cap_rendering, swap_backspace_and_delete, enable_kitty_graphics, enable_checksum_rectangular_area, enable_title_reporting, enable_csi_u_key_encoding, enable_kitty_keyboard, allow_download_protocols, xcursor_theme, xcursor_size, palette_max_key_assigments_for_action, allow_win32_input_mode, treat_left_ctrlalt_as_altgr, send_composed_key_when_left_alt_is_pressed, send_composed_key_when_right_alt_is_pressed, treat_east_asian_ambiguous_width_as_wide, normalize_output_to_unicode_nfc, unicode_version, bidi_enabled, bidi_direction, use_ime, use_dead_keys, ime_preedit_rendering, macos_forward_to_ime_modifier_mask, xim_im_name) && self.next.is_empty()
    }

    fn merge(&mut self, update: Self) {
        merge_native_window_config_patch_values!(self, update; tab_bar_style, copy_mode_active_highlight_fg, copy_mode_active_highlight_bg, copy_mode_inactive_highlight_fg, copy_mode_inactive_highlight_bg, quick_select_label_fg, quick_select_label_bg, quick_select_match_fg, quick_select_match_bg, input_selector_label_fg, input_selector_label_bg, launcher_label_fg, launcher_label_bg, automatically_reload_config, check_for_updates, check_for_updates_interval_seconds, show_update_window, key_map_preference, ui_key_cap_rendering, swap_backspace_and_delete, enable_kitty_graphics, enable_checksum_rectangular_area, enable_title_reporting, enable_csi_u_key_encoding, enable_kitty_keyboard, allow_download_protocols, xcursor_theme, xcursor_size, palette_max_key_assigments_for_action, allow_win32_input_mode, treat_left_ctrlalt_as_altgr, send_composed_key_when_left_alt_is_pressed, send_composed_key_when_right_alt_is_pressed, treat_east_asian_ambiguous_width_as_wide, normalize_output_to_unicode_nfc, unicode_version, bidi_enabled, bidi_direction, use_ime, use_dead_keys, ime_preedit_rendering, macos_forward_to_ime_modifier_mask, xim_im_name);
        self.next.merge(update.next);
    }

    fn apply_to_native_config_overrides(self, overrides: &mut NativeConfigSnapshot) {
        if let Some(value) = self.tab_bar_style {
            overrides.tab_bar_style = value;
        }
        apply_native_window_config_patch_values!(self, overrides; copy_mode_active_highlight_fg, copy_mode_active_highlight_bg, copy_mode_inactive_highlight_fg, copy_mode_inactive_highlight_bg, quick_select_label_fg, quick_select_label_bg, quick_select_match_fg, quick_select_match_bg, input_selector_label_fg, input_selector_label_bg, launcher_label_fg, launcher_label_bg, automatically_reload_config, check_for_updates, check_for_updates_interval_seconds, show_update_window, key_map_preference, ui_key_cap_rendering, swap_backspace_and_delete, enable_kitty_graphics, enable_checksum_rectangular_area, enable_title_reporting, enable_csi_u_key_encoding, enable_kitty_keyboard, allow_download_protocols, xcursor_theme, xcursor_size, palette_max_key_assigments_for_action, allow_win32_input_mode, treat_left_ctrlalt_as_altgr, send_composed_key_when_left_alt_is_pressed, send_composed_key_when_right_alt_is_pressed, treat_east_asian_ambiguous_width_as_wide, normalize_output_to_unicode_nfc, unicode_version, bidi_enabled, bidi_direction, use_ime, use_dead_keys, ime_preedit_rendering, macos_forward_to_ime_modifier_mask, xim_im_name);
        self.next.apply_to_native_config_overrides(overrides);
    }
}

impl NativeWindowConfigPatchValues5 {
    fn is_empty(&self) -> bool {
        native_window_config_patch_values_are_empty!(self; detect_password_input, canonicalize_pasted_newlines, quote_dropped_files, alternate_buffer_wheel_scroll_speed, bypass_mouse_reporting_modifiers, enable_scroll_bar, scrollback_lines, min_scroll_bar_height, unzoom_on_switch_pane, scroll_to_bottom_on_input, disable_default_key_bindings, disable_default_mouse_bindings, hide_mouse_cursor_when_typing, pane_focus_follows_mouse, swallow_mouse_click_on_pane_focus, swallow_mouse_click_on_window_focus, debug_key_events, log_unknown_escape_sequences, warn_about_missing_glyphs, leader, key_assignments, key_tables, mouse_assignments, enable_tab_bar, hide_tab_bar_if_only_one_tab, use_fancy_tab_bar, tab_bar_at_bottom, tab_and_split_indices_are_zero_based, mouse_wheel_scrolls_tabs, switch_to_last_active_tab_when_closing_tab, tab_shortcut_style, closed_tab_history_size, close_tab_selection, tab_bar_wheel_behavior, quit_when_all_windows_are_closed, window_close_confirmation, exit_behavior, clean_exit_codes, exit_behavior_messaging, skip_close_confirmation_for_processes_named, show_close_tab_button_in_tabs, show_new_tab_button_in_tab_bar, show_tab_index_in_tab_bar, show_tabs_in_tab_bar)
    }

    fn merge(&mut self, update: Self) {
        merge_native_window_config_patch_values!(self, update; detect_password_input, canonicalize_pasted_newlines, quote_dropped_files, alternate_buffer_wheel_scroll_speed, bypass_mouse_reporting_modifiers, enable_scroll_bar, scrollback_lines, min_scroll_bar_height, unzoom_on_switch_pane, scroll_to_bottom_on_input, disable_default_key_bindings, disable_default_mouse_bindings, hide_mouse_cursor_when_typing, pane_focus_follows_mouse, swallow_mouse_click_on_pane_focus, swallow_mouse_click_on_window_focus, debug_key_events, log_unknown_escape_sequences, warn_about_missing_glyphs, leader, key_assignments, key_tables, mouse_assignments, enable_tab_bar, hide_tab_bar_if_only_one_tab, use_fancy_tab_bar, tab_bar_at_bottom, tab_and_split_indices_are_zero_based, mouse_wheel_scrolls_tabs, switch_to_last_active_tab_when_closing_tab, tab_shortcut_style, closed_tab_history_size, close_tab_selection, tab_bar_wheel_behavior, quit_when_all_windows_are_closed, window_close_confirmation, exit_behavior, clean_exit_codes, exit_behavior_messaging, skip_close_confirmation_for_processes_named, show_close_tab_button_in_tabs, show_new_tab_button_in_tab_bar, show_tab_index_in_tab_bar, show_tabs_in_tab_bar);
    }

    fn apply_to_native_config_overrides(self, overrides: &mut NativeConfigSnapshot) {
        apply_native_window_config_patch_values!(self, overrides; detect_password_input, canonicalize_pasted_newlines, quote_dropped_files, alternate_buffer_wheel_scroll_speed, bypass_mouse_reporting_modifiers, enable_scroll_bar, scrollback_lines, min_scroll_bar_height, unzoom_on_switch_pane, scroll_to_bottom_on_input, disable_default_key_bindings, disable_default_mouse_bindings, hide_mouse_cursor_when_typing, pane_focus_follows_mouse, swallow_mouse_click_on_pane_focus, swallow_mouse_click_on_window_focus, debug_key_events, log_unknown_escape_sequences, warn_about_missing_glyphs, leader, key_assignments, key_tables, mouse_assignments, enable_tab_bar, hide_tab_bar_if_only_one_tab, use_fancy_tab_bar, tab_bar_at_bottom, tab_and_split_indices_are_zero_based, mouse_wheel_scrolls_tabs, switch_to_last_active_tab_when_closing_tab, tab_shortcut_style, closed_tab_history_size, close_tab_selection, tab_bar_wheel_behavior, quit_when_all_windows_are_closed, window_close_confirmation, exit_behavior, clean_exit_codes, exit_behavior_messaging, skip_close_confirmation_for_processes_named, show_close_tab_button_in_tabs, show_new_tab_button_in_tab_bar, show_tab_index_in_tab_bar, show_tabs_in_tab_bar);
    }
}
