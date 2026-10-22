impl NativeWindowApp {
    fn configured_pane_terminal_runtime(&self, size: TerminalSize) -> TerminalRuntime {
        self.configure_pane_terminal_runtime(TerminalRuntime::new(size))
    }

    fn configured_pane_terminal_runtime_from_terminal(
        &self,
        terminal: Terminal,
    ) -> TerminalRuntime {
        self.configure_pane_terminal_runtime(TerminalRuntime::from_terminal(terminal))
    }

    fn configure_pane_terminal_runtime(&self, mut runtime: TerminalRuntime) -> TerminalRuntime {
        runtime.set_terminal_name(self.term.clone());
        runtime.set_enq_answerback(self.enq_answerback.clone());
        runtime.set_enable_kitty_graphics(self.enable_kitty_graphics);
        runtime.set_enable_checksum_rectangular_area(self.enable_checksum_rectangular_area);
        runtime.set_enable_title_reporting(self.enable_title_reporting);
        runtime.set_enable_kitty_keyboard(self.enable_kitty_keyboard);
        runtime.set_allow_win32_input_mode(self.allow_win32_input_mode);
        runtime.set_treat_east_asian_ambiguous_width_as_wide(
            self.treat_east_asian_ambiguous_width_as_wide,
        );
        runtime.set_normalize_output_to_unicode_nfc(self.normalize_output_to_unicode_nfc);
        runtime.set_unicode_version(self.unicode_version);
        runtime.set_cell_width_overrides(self.terminal_cell_width_overrides());
        runtime.set_scrollback_limit(self.scrollback_lines);
        runtime.set_default_cursor_style(CursorStyle::from(self.default_cursor_style));
        runtime
    }
}
