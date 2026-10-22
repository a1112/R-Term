#[test]
fn window_app_parses_static_wezterm_user_var_changed_pane_user_vars_status_setter() {
    let mut app = NativeWindowApp::new(None);
    let overrides = super::native_config_overrides_from_wezterm_lua_config(
        r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local vars = pane:get_user_vars()
              window:set_right_status('host=' .. vars.WEZTERM_HOST)
            end)
            "#,
    )
    .expect("expected static WezTerm user-var-changed pane user vars status setter");
    app.set_config_overrides(overrides);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();

    app.handle_pane_pty_output(
        rterm_core::PaneId::new(1),
        b"\x1b]1337;SetUserVar=WEZTERM_HOST=cHJvZA==\x07",
    )
    .unwrap();

    assert_eq!(app.right_status, "host=prod");
}

#[test]
fn window_app_parses_static_wezterm_user_var_changed_pane_user_vars_fallback_status_setter() {
    let mut app = NativeWindowApp::new(None);
    let overrides = super::native_config_overrides_from_wezterm_lua_config(
        r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local vars = pane:get_user_vars()
              window:set_right_status('host=' .. (vars.WEZTERM_HOST or 'none'))
            end)
            "#,
    )
    .expect("expected static WezTerm user-var-changed pane user vars fallback status setter");
    app.set_config_overrides(overrides);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();

    app.handle_pane_pty_output(
        rterm_core::PaneId::new(1),
        b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07",
    )
    .unwrap();
    assert_eq!(app.right_status, "host=none");

    app.handle_pane_pty_output(
        rterm_core::PaneId::new(1),
        b"\x1b]1337;SetUserVar=WEZTERM_HOST=cHJvZA==\x07",
    )
    .unwrap();
    assert_eq!(app.right_status, "host=prod");
}

#[test]
fn window_app_parses_static_wezterm_user_var_changed_active_pane_user_vars_status_setter() {
    let mut app = NativeWindowApp::new(None);
    let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local vars = window:active_pane():get_user_vars()
              window:set_right_status('active=' .. (vars.WEZTERM_HOST or 'none') .. ' event=' .. pane:pane_id())
            end)
            "#,
        )
        .expect("expected static WezTerm user-var-changed active pane user vars status setter");
    app.set_config_overrides(overrides);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();
    let active_pane = app.app_shell.active_pane_id();
    let inactive_pane = rterm_core::PaneId::new(1);
    assert_ne!(active_pane, inactive_pane);

    app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_HOST=cHJvZA==\x07")
        .unwrap();
    app.handle_pane_pty_output(
        inactive_pane,
        b"\x1b]1337;SetUserVar=WEZTERM_HOST=c3RhZ2U=\x07",
    )
    .unwrap();

    assert_eq!(app.right_status, "active=prod event=1");
}

#[test]
fn window_app_parses_static_wezterm_user_var_changed_local_pane_user_var_status_setter() {
    let mut app = NativeWindowApp::new(None);
    let overrides = super::native_config_overrides_from_wezterm_lua_config(
        r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local vars = pane:get_user_vars()
              local host = vars.WEZTERM_HOST or 'none'
              window:set_right_status('host=' .. host)
            end)
            "#,
    )
    .expect("expected static WezTerm user-var-changed local pane user var status setter");
    app.set_config_overrides(overrides);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();

    app.handle_pane_pty_output(
        rterm_core::PaneId::new(1),
        b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07",
    )
    .unwrap();
    assert_eq!(app.right_status, "host=none");

    app.handle_pane_pty_output(
        rterm_core::PaneId::new(1),
        b"\x1b]1337;SetUserVar=WEZTERM_HOST=cHJvZA==\x07",
    )
    .unwrap();
    assert_eq!(app.right_status, "host=prod");
}

#[test]
fn window_app_answers_osc52_clipboard_query_from_pty_output() {
    let written = Arc::new(Mutex::new(Vec::new()));
    let mut app = NativeWindowApp::new_with_osc52_policy(None, Osc52Policy::ReadWrite);
    app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
    app.clipboard_reader = Box::new(|| Some("copy".to_owned()));

    app.handle_pty_output(b"\x1b]52;c;?\x07").unwrap();

    assert_eq!(
        written.lock().unwrap().as_slice(),
        b"\x1b]52;c;Y29weQ==\x07"
    );
}

#[test]
fn window_app_blocks_osc52_when_policy_is_off() {
    let clipboard_writes = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_writes);
    let pty_writes = Arc::new(Mutex::new(Vec::new()));
    let mut app = NativeWindowApp::new_with_osc52_policy(None, Osc52Policy::Off);
    app.writer = Some(Box::new(SharedWriter(Arc::clone(&pty_writes))));
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.clipboard_reader = Box::new(|| Some("copy".to_owned()));

    app.handle_pty_output(b"\x1b]52;c;Y29weQ==\x07").unwrap();
    app.handle_pty_output(b"\x1b]1337;Copy=;Y29weQ==\x07")
        .unwrap();
    app.handle_pty_output(b"\x1b]52;c;?\x07").unwrap();

    assert!(clipboard_writes.lock().unwrap().is_empty());
    assert!(pty_writes.lock().unwrap().is_empty());
}

#[test]
fn window_app_write_only_osc52_policy_blocks_queries() {
    let clipboard_writes = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_writes);
    let pty_writes = Arc::new(Mutex::new(Vec::new()));
    let mut app = NativeWindowApp::new_with_osc52_policy(None, Osc52Policy::WriteOnly);
    app.writer = Some(Box::new(SharedWriter(Arc::clone(&pty_writes))));
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.clipboard_reader = Box::new(|| Some("copy".to_owned()));

    app.handle_pty_output(b"\x1b]52;c;Y29weQ==\x07").unwrap();
    app.handle_pty_output(b"\x1b]52;c;?\x07").unwrap();

    assert_eq!(clipboard_writes.lock().unwrap().as_slice(), ["copy"]);
    assert!(pty_writes.lock().unwrap().is_empty());
}

#[test]
fn derives_terminal_size_from_window_pixels() {
    assert_eq!(
        terminal_size_from_window_pixels(FRAME_WIDTH, FRAME_HEIGHT),
        rterm_core::TerminalSize::new(80, 24)
    );
    assert_eq!(
        terminal_size_from_window_pixels(FRAME_WIDTH, FRAME_HEIGHT - CELL_HEIGHT),
        rterm_core::TerminalSize::new(80, 23)
    );
    assert_eq!(
        terminal_size_from_window_pixels(1, 1),
        rterm_core::TerminalSize::new(1, 1)
    );
}

fn snapshot_char(
    snapshot: &rterm_render_core::TerminalRenderSnapshot,
    row: u16,
    column: u16,
) -> Option<char> {
    snapshot
        .cells()
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
        .map(|cell| cell.ch)
}

fn snapshot_row_text(
    snapshot: &rterm_render_core::TerminalRenderSnapshot,
    row: u16,
    columns: u16,
) -> String {
    (0..columns)
        .map(|column| snapshot_char(snapshot, row, column).unwrap_or(' '))
        .collect()
}

fn snapshot_cell(
    snapshot: &rterm_render_core::TerminalRenderSnapshot,
    row: u16,
    column: u16,
) -> Option<&rterm_render_core::RenderCell> {
    snapshot
        .cells()
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
}

fn rendered_active_pane_cell(
    app: &NativeWindowApp,
    pane_row: u16,
    pane_column: u16,
) -> Option<rterm_render_core::RenderCell> {
    let active_pane = app.active_pane_id();
    let rect = app
        .pane_render_layout()
        .panes
        .into_iter()
        .find(|rect| rect.pane_id == active_pane)?;
    let snapshot = app.render_snapshot();
    let mut cell = snapshot_cell(
        &snapshot,
        rect.row.saturating_add(pane_row),
        rect.column.saturating_add(pane_column),
    )
    .cloned()?;
    // Color-backed selection overlays no longer use the legacy inverse bit.
    // Keep this test helper semantic for existing assertions that only
    // care whether a cell is selected, without changing renderer output.
    if !cell.inverse && cell.foreground == Color::Default && cell.background != Color::Default {
        cell.inverse = true;
    }
    Some(cell)
}

fn test_contrast_ratio(foreground: [u8; 4], background: [u8; 4]) -> f64 {
    let foreground_luminance = test_relative_luminance(foreground);
    let background_luminance = test_relative_luminance(background);
    let light = foreground_luminance.max(background_luminance);
    let dark = foreground_luminance.min(background_luminance);
    (light + 0.05) / (dark + 0.05)
}

fn test_relative_luminance(color: [u8; 4]) -> f64 {
    let red = test_linear_srgb_component(color[0]);
    let green = test_linear_srgb_component(color[1]);
    let blue = test_linear_srgb_component(color[2]);
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

fn test_linear_srgb_component(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn frame_pixel_at(frame: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let index = (y * width + x) * 4;
    [
        frame[index],
        frame[index + 1],
        frame[index + 2],
        frame[index + 3],
    ]
}

fn write_test_png_file(name: &str) -> PathBuf {
    let bytes = red_png_bytes();
    write_test_file(name, &bytes, "PNG")
}

fn red_png_bytes() -> Vec<u8> {
    const RED_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    STANDARD
        .decode(RED_PNG_BASE64)
        .expect("embedded PNG should decode")
}

fn write_test_bmp_file(name: &str) -> PathBuf {
    const RED_BMP: &[u8] = &[
        0x42, 0x4d, 0x3a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x36, 0x00, 0x00, 0x00, 0x28,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x18, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x13, 0x0b, 0x00, 0x00, 0x13, 0x0b, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00,
    ];
    write_test_file(name, RED_BMP, "BMP")
}

fn write_test_ico_file(name: &str) -> PathBuf {
    let png = red_png_bytes();
    let mut bytes = Vec::with_capacity(22 + png.len());
    bytes.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x01, 0x00]);
    bytes.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00]);
    bytes.extend_from_slice(
        &u32::try_from(png.len())
            .expect("embedded PNG should fit ICO directory")
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&22_u32.to_le_bytes());
    bytes.extend_from_slice(&png);
    write_test_file(name, &bytes, "ICO")
}

fn write_test_tiff_file(name: &str) -> PathBuf {
    const RED_TIFF: &[u8] = &[
        0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x01, 0x04, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x02, 0x01, 0x03, 0x00, 0x03, 0x00, 0x00, 0x00, 0x86, 0x00, 0x00,
        0x00, 0x03, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x01,
        0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x11, 0x01, 0x04, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x8c, 0x00, 0x00, 0x00, 0x15, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x03, 0x00, 0x00, 0x00, 0x16, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x17, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x1c, 0x01,
        0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08,
        0x00, 0x08, 0x00, 0x08, 0x00, 0xff, 0x00, 0x00,
    ];
    write_test_file(name, RED_TIFF, "TIFF")
}

fn write_test_dds_file(name: &str) -> PathBuf {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"DDS ");
    for value in [
        124_u32,
        0x0008_1007,
        4,
        4,
        8,
        0,
        0, // header
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0, // reserved1
        32,
        0x0000_0004, // pixel format size, fourcc flag
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(b"DXT1");
    for value in [
        0_u32,
        0,
        0,
        0,
        0, // RGB bit count and masks
        0x0000_1000,
        0,
        0,
        0,
        0, // caps
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[0x00, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    write_test_file(name, &bytes, "DDS")
}

fn write_test_ppm_file(name: &str) -> PathBuf {
    const RED_PPM: &[u8] = b"P6\n1 1\n255\n\xff\x00\x00";
    write_test_file(name, RED_PPM, "PPM")
}

fn write_test_tga_file(name: &str) -> PathBuf {
    const RED_TGA: &[u8] = &[
        0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
        0x00, 0x18, 0x20, 0x00, 0x00, 0xff,
    ];
    write_test_file(name, RED_TGA, "TGA")
}

fn write_test_farbfeld_file(name: &str) -> PathBuf {
    const RED_FARBFELD: &[u8] = &[
        b'f', b'a', b'r', b'b', b'f', b'e', b'l', b'd', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x01, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff,
    ];
    write_test_file(name, RED_FARBFELD, "farbfeld")
}

fn write_test_file(name: &str, bytes: &[u8], format_name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rterm-{unique}-{name}"));
    std::fs::write(&path, bytes).unwrap_or_else(|_| panic!("write test {format_name}"));
    path
}

fn lua_string_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn count_frame_pixels(frame: &[u8], color: [u8; 4]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| {
            pixel[0] == color[0]
                && pixel[1] == color[1]
                && pixel[2] == color[2]
                && pixel[3] == color[3]
        })
        .count()
}

#[test]
fn window_workspace_chrome_actions_have_real_non_overlapping_targets() {
    let targets = super::workspace_chrome::action_targets(120, 3);
    assert_eq!(
        targets[0].action,
        super::workspace_chrome::WorkspaceChromeAction::Search
    );
    assert_eq!(
        targets[1].action,
        super::workspace_chrome::WorkspaceChromeAction::Split
    );
    assert_eq!(
        targets[2].action,
        super::workspace_chrome::WorkspaceChromeAction::Settings
    );
    assert!(
        targets
            .windows(2)
            .all(|pair| pair[0].end_column <= pair[1].start_column)
    );
    for target in targets {
        let column = target.start_column;
        assert_eq!(
            super::workspace_chrome::action_at_column(column, 120, 3),
            Some(target.action)
        );
    }
}

#[test]
fn window_workspace_pane_header_preserves_grapheme_text_and_display_width() {
    let cells =
        super::workspace_chrome::pane_header_cells(&super::workspace_chrome::PaneHeaderSpec {
            row: 0,
            column: 0,
            columns: 24,
            title: "界e\u{301}",
            shell: "",
            cwd: "",
            progress: "",
            active: true,
            attention: false,
        });

    let wide_leader = cells
        .iter()
        .find(|cell| cell.text.as_ref() == "界")
        .expect("wide title glyph should be rendered");
    assert_eq!(wide_leader.columns, 2);
    assert!(!wide_leader.continuation);
    assert!(
        cells
            .iter()
            .any(|cell| cell.column == wide_leader.column + 1 && cell.continuation)
    );
    assert!(
        cells
            .iter()
            .any(|cell| cell.text.as_ref() == "e\u{301}" && cell.columns == 1)
    );
}

#[test]
fn window_workspace_split_action_reaches_three_independent_panes() {
    let mut app = NativeWindowApp::new(None);
    app.workspace_chrome_split_active_pane();
    assert_eq!(app.app_shell.active_tab().panes().len(), 2);
    app.workspace_chrome_split_active_pane();
    assert_eq!(app.app_shell.active_tab().panes().len(), 3);
    assert_eq!(app.pane_render_layout().separators.len(), 2);
}

#[test]
fn window_workspace_settings_and_pane_menus_expose_real_commands() {
    let mut app = NativeWindowApp::new(None);
    app.enter_workspace_settings_menu();
    let settings = app
        .command_palette
        .as_ref()
        .expect("settings opens the command palette");
    assert_eq!(settings.context_title.as_deref(), Some("Settings"));
    assert!(settings.context_entries.as_ref().is_some_and(|entries| {
        entries.iter().any(|entry| {
            matches!(
                entry,
                WindowCommandPaletteEntry::BuiltIn(WindowCommand::ReloadConfiguration)
            )
        })
    }));
    assert!(
        app.effective_window_title()
            .contains("Settings: [1 / 6] Increase Font Size")
    );
    let settings_cells = app.command_palette_cells();
    assert!(
        settings_cells
            .iter()
            .any(|cell| cell.ch == 'I' && cell.text.as_ref() == "I")
    );

    let pane = app.app_shell.active_pane_id();
    app.enter_workspace_pane_menu(pane);
    let pane_menu = app
        .command_palette
        .as_ref()
        .expect("pane menu opens the command palette");
    assert_eq!(pane_menu.context_title.as_deref(), Some("Pane Actions"));
    let commands = pane_menu
        .context_entries
        .as_ref()
        .expect("pane menu commands");
    assert!(commands.iter().any(|entry| {
        matches!(
            entry,
            WindowCommandPaletteEntry::BuiltIn(WindowCommand::InspectPane)
        )
    }));
    assert!(commands.iter().any(|entry| {
        matches!(
            entry,
            WindowCommandPaletteEntry::BuiltIn(WindowCommand::ClosePane)
        )
    }));
    let pane_cells = app.command_palette_cells();
    assert!(
        pane_cells
            .iter()
            .any(|cell| cell.ch == 'I' && cell.text.as_ref() == "I")
    );
}

#[test]
fn window_workspace_chrome_native_geometry_reserves_footer_row() {
    let chrome_rows = super::workspace_chrome::frame_rows_for_layout(true, true);
    assert_eq!(chrome_rows, super::workspace_chrome::WORKSPACE_CHROME_ROWS);
    assert_eq!(
        super::workspace_chrome::frame_rows_for_layout(true, false),
        0
    );
    assert_eq!(
        super::workspace_chrome::viewport_rows_for_chrome(24, true),
        25
    );

    let terminal_frame_row = 1_u16.saturating_add(1);
    let footer_row = terminal_frame_row.saturating_add(24);
    let snapshot_row_count = terminal_frame_row
        .saturating_add(super::workspace_chrome::viewport_rows_for_chrome(24, true));
    assert!(footer_row < snapshot_row_count);
}

struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct DelayedWriter {
    delay: Duration,
    written: Arc<Mutex<Vec<u8>>>,
}

impl Write for DelayedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        thread::sleep(self.delay);
        self.written.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
