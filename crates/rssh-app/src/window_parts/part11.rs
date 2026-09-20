impl Drop for NativeWindowApp {
    fn drop(&mut self) {
        self.stop_active_runtime();
        if let Some(mut runtime) = self.runtime.take_worker() {
            runtime.shutdown();
        }

        let inactive_panes = self.pane_runtimes.keys().copied().collect::<Vec<_>>();
        for pane_id in inactive_panes {
            self.cancel_ssh_runtime(pane_id);
            if let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) {
                let cleanup = runtime.close();
                report_pane_pty_cleanup("window drop pane PTY cleanup", &cleanup);
            }
        }

        self.pane_runtimes.clear();
    }
}
impl NativeWindowApp {
    #[cfg(feature = "ssh")]
    fn cancel_ssh_runtime(&mut self, pane_id: rssh_core::PaneId) {
        self.resolve_host_key_prompt_for_pane(pane_id, HostKeyDecision::Cancel);
        self.resolve_secret_prompt_for_pane(pane_id, None);
        if let Some(cancellation) = self.ssh_writer_cancellations.get(&pane_id) {
            cancellation.store(true, Ordering::Release);
        }
        if let Some(cancellation) = self.ssh_connection_cancellations.remove(&pane_id) {
            cancellation.cancel();
        }
        if let Some(sender) = self.ssh_writer_senders.remove(&pane_id) {
            let _ = sender.send(NativeSshCommand::Cancel);
        }
    }

    fn finish_active_runtime_after_exit(&mut self) -> Option<PtyExitStatus> {
        let active = self.app_shell.active_pane_id();
        self.cancel_ssh_runtime(active);
        let active_is_local = self.active_runtime_transport
            == Some(PaneRuntimeTransportKind::LocalPty)
            || self
                .runtime
                .worker()
                .is_some_and(|runtime| runtime.contains_pane(active));
        if active_is_local {
            if let Some(runtime) = self.runtime.worker_mut() {
                let _ = runtime.begin_close_by_pane(active, Duration::ZERO);
            }
            self.session_process_id = None;
            self.session_tty_name = None;
            self.active_runtime_generation = 0;
            self.active_runtime_transport = None;
            return None;
        }
        if let Err(error) = self.finish_active_pane_output() {
            eprintln!("active pane terminal finish failed: {error}");
        }
        let cleanup = finish_pty_lifecycle_after_exit(
            &mut self.session,
            &mut self.session_process_id,
            &mut self.session_tty_name,
            &mut self.writer,
            &mut self.reader_thread,
            &mut self.writer_thread,
        );
        report_pane_pty_cleanup("active pane exit cleanup", &cleanup);
        cleanup.status
    }

    fn stop_active_runtime(&mut self) {
        let active = self.app_shell.active_pane_id();
        self.cancel_ssh_runtime(active);
        let active_is_local = self.active_runtime_transport
            == Some(PaneRuntimeTransportKind::LocalPty)
            || self
                .runtime
                .worker()
                .is_some_and(|runtime| runtime.contains_pane(active));
        if active_is_local {
            if let Some(runtime) = self.runtime.worker_mut() {
                let _ = runtime.begin_close_by_pane(active, Duration::ZERO);
            }
            self.session_process_id = None;
            self.session_tty_name = None;
            self.active_runtime_generation = 0;
            self.active_runtime_transport = None;
            return;
        }
        let cleanup = stop_pty_lifecycle(
            &mut self.session,
            &mut self.session_process_id,
            &mut self.session_tty_name,
            &mut self.writer,
            &mut self.reader_thread,
            &mut self.writer_thread,
        );
        report_pane_pty_cleanup("active pane PTY cleanup", &cleanup);
    }

    fn stop_local_runtime_for_pane(&mut self, pane_id: rssh_core::PaneId) {
        if let Some(runtime) = self.runtime.worker_mut()
            && runtime.contains_pane(pane_id)
        {
            let _ = runtime.begin_close_by_pane(pane_id, Duration::ZERO);
        }
    }
}

#[cfg(test)]
fn encode_window_key(
    key: &Key,
    physical_key: PhysicalKey,
    text: Option<&str>,
    modifiers: ModifiersState,
    application_cursor_keys: bool,
    application_keypad: bool,
) -> Vec<u8> {
    encode_window_key_with_kitty(
        key,
        physical_key,
        text,
        modifiers,
        application_cursor_keys,
        application_keypad,
        0,
        0,
    )
}

fn window_physical_key_is_modifier(physical_key: PhysicalKey) -> bool {
    matches!(
        physical_key,
        PhysicalKey::Code(
            WinitKeyCode::ShiftLeft
                | WinitKeyCode::ShiftRight
                | WinitKeyCode::ControlLeft
                | WinitKeyCode::ControlRight
                | WinitKeyCode::AltLeft
                | WinitKeyCode::AltRight
                | WinitKeyCode::SuperLeft
                | WinitKeyCode::SuperRight
        )
    )
}

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "independent compatibility flags represent valid combinations"
)]
fn native_alt_composed_key_should_remove_alt_modifier(
    physical_key: PhysicalKey,
    text: Option<&str>,
    modifiers: ModifiersState,
    left_alt_pressed: bool,
    right_alt_pressed: bool,
    send_composed_key_when_left_alt_is_pressed: bool,
    send_composed_key_when_right_alt_is_pressed: bool,
) -> bool {
    text.is_some_and(|text| !text.is_empty())
        && modifiers.contains(ModifiersState::ALT)
        && !window_physical_key_is_modifier(physical_key)
        && ((left_alt_pressed && send_composed_key_when_left_alt_is_pressed)
            || (right_alt_pressed && send_composed_key_when_right_alt_is_pressed))
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn encode_window_key_with_kitty(
    key: &Key,
    physical_key: PhysicalKey,
    text: Option<&str>,
    modifiers: ModifiersState,
    application_cursor_keys: bool,
    application_keypad: bool,
    kitty_keyboard_flags: u16,
    modify_other_keys: u8,
) -> Vec<u8> {
    encode_window_key_with_kitty_event(
        key,
        physical_key,
        text,
        modifiers,
        application_cursor_keys,
        application_keypad,
        kitty_keyboard_flags,
        modify_other_keys,
        KittyKeyEventKind::Press,
    )
}

#[allow(clippy::too_many_arguments)]
fn encode_window_key_with_kitty_event(
    key: &Key,
    physical_key: PhysicalKey,
    text: Option<&str>,
    modifiers: ModifiersState,
    application_cursor_keys: bool,
    application_keypad: bool,
    kitty_keyboard_flags: u16,
    modify_other_keys: u8,
    key_event_kind: KittyKeyEventKind,
) -> Vec<u8> {
    let alt = modifiers.alt_key();

    // Winit reports macOS key auto-repeat as a pressed event with
    // `key.repeat = true`.  Kitty's event protocol is opt-in; when no Kitty
    // keyboard flags are active, repeat events must follow the regular
    // terminal encoding path.  Treating every non-press as a Kitty event
    // made the encoder return an empty buffer in the default configuration,
    // which stopped long-press Backspace/Delete (and repeated text) after the
    // initial key press.  Releases remain protocol-only and intentionally do
    // not emit bytes unless Kitty event reporting is enabled.
    if key_event_kind == KittyKeyEventKind::Release {
        return encode_kitty_event_window_key(
            key,
            physical_key,
            text,
            modifiers,
            kitty_keyboard_flags,
            key_event_kind,
        )
        .unwrap_or_default();
    }
    if key_event_kind == KittyKeyEventKind::Repeat
        && kitty_keyboard_flags != 0
        && let Some(bytes) = encode_kitty_event_window_key(
            key,
            physical_key,
            text,
            modifiers,
            kitty_keyboard_flags,
            key_event_kind,
        )
    {
        return bytes;
    }

    if let Some(bytes) = encode_kitty_modifier_window_key(
        physical_key,
        modifiers,
        kitty_keyboard_flags,
        key_event_kind,
    ) {
        return bytes;
    }

    if let Some(bytes) = encode_kitty_keypad_window_key(
        physical_key,
        modifiers,
        kitty_keyboard_flags,
        key_event_kind,
    ) {
        return bytes;
    }

    if let Some(bytes) =
        encode_kitty_functional_window_key(key, modifiers, kitty_keyboard_flags, key_event_kind)
    {
        return bytes;
    }

    if let Some(bytes) = encode_kitty_report_all_window_key(
        key,
        physical_key,
        text,
        modifiers,
        kitty_keyboard_flags,
        key_event_kind,
    ) {
        return bytes;
    }

    if let Some(bytes) = encode_kitty_disambiguated_window_key(
        key,
        physical_key,
        modifiers,
        kitty_keyboard_flags,
        key_event_kind,
    ) {
        return bytes;
    }

    if let Some(bytes) = encode_xterm_modify_other_window_key(key, modifiers, modify_other_keys) {
        return bytes;
    }

    if let Some(bytes) = encode_control_window_key(key, physical_key, modifiers, application_keypad)
    {
        return bytes;
    }

    if let Some(bytes) = encode_modified_window_key(key, modifiers) {
        return bytes;
    }

    if application_keypad && let Some(bytes) = encode_application_keypad_key(physical_key) {
        return bytes;
    }

    if application_cursor_keys && let Some(bytes) = encode_application_cursor_key(key) {
        return bytes;
    }

    if modifiers.shift_key() && matches!(key, Key::Named(NamedKey::Tab)) {
        return encode_terminal_key(TerminalKey::BackTab).unwrap_or_default();
    }

    if let Some(key) = named_terminal_key(key) {
        return encode_terminal_key(key).unwrap_or_default();
    }

    let mut bytes: Vec<u8> = text
        .unwrap_or_default()
        .chars()
        .filter_map(|character| encode_terminal_key(TerminalKey::Text(character)))
        .flatten()
        .collect();
    if alt && !bytes.is_empty() {
        bytes.insert(0, 0x1b);
    }

    bytes
}

fn swap_backspace_delete_key_if_needed(key: &Key, enabled: bool) -> Key {
    if !enabled {
        return key.clone();
    }

    match key {
        Key::Named(NamedKey::Backspace) => Key::Named(NamedKey::Delete),
        Key::Named(NamedKey::Delete) => Key::Named(NamedKey::Backspace),
        _ => key.clone(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KittyKeyEventKind {
    Press,
    Repeat,
    Release,
}

impl KittyKeyEventKind {
    fn from_winit_key(key: &winit::event::KeyEvent) -> Self {
        match key.state {
            ElementState::Released => Self::Release,
            ElementState::Pressed if key.repeat => Self::Repeat,
            ElementState::Pressed => Self::Press,
        }
    }
}

fn encode_win32_window_key(
    key: &Key,
    physical_key: PhysicalKey,
    text: Option<&str>,
    modifiers: ModifiersState,
    key_down: bool,
) -> Vec<u8> {
    let Some(virtual_key) = win32_virtual_key_code(key, physical_key) else {
        return Vec::new();
    };
    let unicode = if key_down {
        win32_unicode_char(key, text)
    } else {
        0
    };
    let key_down = u8::from(key_down);
    let control_key_state = win32_control_key_state(physical_key, modifiers);

    format!("\x1b[{virtual_key};0;{unicode};{key_down};{control_key_state};1_").into_bytes()
}

fn win32_unicode_char(key: &Key, text: Option<&str>) -> u32 {
    text.and_then(|text| text.chars().next())
        .or_else(|| match key.as_ref() {
            Key::Character(character) => character.chars().next(),
            Key::Named(NamedKey::Enter) => Some('\r'),
            Key::Named(NamedKey::Tab) => Some('\t'),
            Key::Named(NamedKey::Backspace) => Some('\u{8}'),
            Key::Named(NamedKey::Escape) => Some('\u{1b}'),
            _ => None,
        })
        .map_or(0, u32::from)
}

fn win32_virtual_key_code(key: &Key, physical_key: PhysicalKey) -> Option<u16> {
    if let Some(code) = win32_virtual_key_code_from_physical(physical_key) {
        return Some(code);
    }

    match key.as_ref() {
        Key::Character(character) => character
            .chars()
            .next()
            .and_then(win32_virtual_key_code_from_character),
        Key::Named(NamedKey::Backspace) => Some(0x08),
        Key::Named(NamedKey::Tab) => Some(0x09),
        Key::Named(NamedKey::Enter) => Some(0x0d),
        Key::Named(NamedKey::Escape) => Some(0x1b),
        Key::Named(NamedKey::PageUp) => Some(0x21),
        Key::Named(NamedKey::PageDown) => Some(0x22),
        Key::Named(NamedKey::End) => Some(0x23),
        Key::Named(NamedKey::Home) => Some(0x24),
        Key::Named(NamedKey::ArrowLeft) => Some(0x25),
        Key::Named(NamedKey::ArrowUp) => Some(0x26),
        Key::Named(NamedKey::ArrowRight) => Some(0x27),
        Key::Named(NamedKey::ArrowDown) => Some(0x28),
        Key::Named(NamedKey::Insert) => Some(0x2d),
        Key::Named(NamedKey::Delete) => Some(0x2e),
        Key::Named(NamedKey::Shift) => Some(0x10),
        Key::Named(NamedKey::Control) => Some(0x11),
        Key::Named(NamedKey::Alt) => Some(0x12),
        Key::Named(NamedKey::F1) => Some(0x70),
        Key::Named(NamedKey::F2) => Some(0x71),
        Key::Named(NamedKey::F3) => Some(0x72),
        Key::Named(NamedKey::F4) => Some(0x73),
        Key::Named(NamedKey::F5) => Some(0x74),
        Key::Named(NamedKey::F6) => Some(0x75),
        Key::Named(NamedKey::F7) => Some(0x76),
        Key::Named(NamedKey::F8) => Some(0x77),
        Key::Named(NamedKey::F9) => Some(0x78),
        Key::Named(NamedKey::F10) => Some(0x79),
        Key::Named(NamedKey::F11) => Some(0x7a),
        Key::Named(NamedKey::F12) => Some(0x7b),
        Key::Named(NamedKey::F13) => Some(0x7c),
        Key::Named(NamedKey::F14) => Some(0x7d),
        Key::Named(NamedKey::F15) => Some(0x7e),
        Key::Named(NamedKey::F16) => Some(0x7f),
        Key::Named(NamedKey::F17) => Some(0x80),
        Key::Named(NamedKey::F18) => Some(0x81),
        Key::Named(NamedKey::F19) => Some(0x82),
        Key::Named(NamedKey::F20) => Some(0x83),
        Key::Named(NamedKey::F21) => Some(0x84),
        Key::Named(NamedKey::F22) => Some(0x85),
        Key::Named(NamedKey::F23) => Some(0x86),
        Key::Named(NamedKey::F24) => Some(0x87),
        _ => None,
    }
}

fn win32_virtual_key_code_from_character(character: char) -> Option<u16> {
    match character {
        ' ' => return Some(0x20),
        ';' | ':' => return Some(0xba),
        '=' | '+' => return Some(0xbb),
        ',' | '<' => return Some(0xbc),
        '-' | '_' => return Some(0xbd),
        '.' | '>' => return Some(0xbe),
        '/' | '?' => return Some(0xbf),
        '`' | '~' => return Some(0xc0),
        '[' | '{' => return Some(0xdb),
        '\\' | '|' => return Some(0xdc),
        ']' | '}' => return Some(0xdd),
        '\'' | '"' => return Some(0xde),
        _ => {}
    }

    let character = character.to_ascii_uppercase();
    if character.is_ascii_alphabetic() || character.is_ascii_digit() {
        Some(character as u16)
    } else {
        None
    }
}

fn win32_virtual_key_code_from_physical(physical_key: PhysicalKey) -> Option<u16> {
    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };

    win32_alphanumeric_virtual_key_code(code)
        .or_else(|| win32_numpad_virtual_key_code(code))
        .or_else(|| win32_modifier_virtual_key_code_from_physical(code))
        .or_else(|| win32_navigation_virtual_key_code(code))
        .or_else(|| win32_function_virtual_key_code(code))
}

fn win32_alphanumeric_virtual_key_code(code: WinitKeyCode) -> Option<u16> {
    match code {
        WinitKeyCode::KeyA => Some(0x41),
        WinitKeyCode::KeyB => Some(0x42),
        WinitKeyCode::KeyC => Some(0x43),
        WinitKeyCode::KeyD => Some(0x44),
        WinitKeyCode::KeyE => Some(0x45),
        WinitKeyCode::KeyF => Some(0x46),
        WinitKeyCode::KeyG => Some(0x47),
        WinitKeyCode::KeyH => Some(0x48),
        WinitKeyCode::KeyI => Some(0x49),
        WinitKeyCode::KeyJ => Some(0x4a),
        WinitKeyCode::KeyK => Some(0x4b),
        WinitKeyCode::KeyL => Some(0x4c),
        WinitKeyCode::KeyM => Some(0x4d),
        WinitKeyCode::KeyN => Some(0x4e),
        WinitKeyCode::KeyO => Some(0x4f),
        WinitKeyCode::KeyP => Some(0x50),
        WinitKeyCode::KeyQ => Some(0x51),
        WinitKeyCode::KeyR => Some(0x52),
        WinitKeyCode::KeyS => Some(0x53),
        WinitKeyCode::KeyT => Some(0x54),
        WinitKeyCode::KeyU => Some(0x55),
        WinitKeyCode::KeyV => Some(0x56),
        WinitKeyCode::KeyW => Some(0x57),
        WinitKeyCode::KeyX => Some(0x58),
        WinitKeyCode::KeyY => Some(0x59),
        WinitKeyCode::KeyZ => Some(0x5a),
        WinitKeyCode::Digit0 => Some(0x30),
        WinitKeyCode::Digit1 => Some(0x31),
        WinitKeyCode::Digit2 => Some(0x32),
        WinitKeyCode::Digit3 => Some(0x33),
        WinitKeyCode::Digit4 => Some(0x34),
        WinitKeyCode::Digit5 => Some(0x35),
        WinitKeyCode::Digit6 => Some(0x36),
        WinitKeyCode::Digit7 => Some(0x37),
        WinitKeyCode::Digit8 => Some(0x38),
        WinitKeyCode::Digit9 => Some(0x39),
        _ => None,
    }
}

fn win32_numpad_virtual_key_code(code: WinitKeyCode) -> Option<u16> {
    match code {
        WinitKeyCode::Numpad0 => Some(0x60),
        WinitKeyCode::Numpad1 => Some(0x61),
        WinitKeyCode::Numpad2 => Some(0x62),
        WinitKeyCode::Numpad3 => Some(0x63),
        WinitKeyCode::Numpad4 => Some(0x64),
        WinitKeyCode::Numpad5 => Some(0x65),
        WinitKeyCode::Numpad6 => Some(0x66),
        WinitKeyCode::Numpad7 => Some(0x67),
        WinitKeyCode::Numpad8 => Some(0x68),
        WinitKeyCode::Numpad9 => Some(0x69),
        WinitKeyCode::NumpadMultiply => Some(0x6a),
        WinitKeyCode::NumpadAdd => Some(0x6b),
        WinitKeyCode::NumpadComma => Some(0x6c),
        WinitKeyCode::NumpadSubtract => Some(0x6d),
        WinitKeyCode::NumpadDecimal => Some(0x6e),
        WinitKeyCode::NumpadDivide => Some(0x6f),
        _ => None,
    }
}

fn win32_modifier_virtual_key_code_from_physical(code: WinitKeyCode) -> Option<u16> {
    match code {
        WinitKeyCode::ShiftLeft => Some(0xa0),
        WinitKeyCode::ShiftRight => Some(0xa1),
        WinitKeyCode::ControlLeft => Some(0xa2),
        WinitKeyCode::ControlRight => Some(0xa3),
        WinitKeyCode::AltLeft => Some(0xa4),
        WinitKeyCode::AltRight => Some(0xa5),
        _ => None,
    }
}

fn win32_navigation_virtual_key_code(code: WinitKeyCode) -> Option<u16> {
    match code {
        WinitKeyCode::Enter | WinitKeyCode::NumpadEnter => Some(0x0d),
        WinitKeyCode::Tab => Some(0x09),
        WinitKeyCode::Backspace => Some(0x08),
        WinitKeyCode::Escape => Some(0x1b),
        WinitKeyCode::PageUp => Some(0x21),
        WinitKeyCode::PageDown => Some(0x22),
        WinitKeyCode::End => Some(0x23),
        WinitKeyCode::Home => Some(0x24),
        WinitKeyCode::ArrowLeft => Some(0x25),
        WinitKeyCode::ArrowUp => Some(0x26),
        WinitKeyCode::ArrowRight => Some(0x27),
        WinitKeyCode::ArrowDown => Some(0x28),
        WinitKeyCode::Insert => Some(0x2d),
        WinitKeyCode::Delete => Some(0x2e),
        _ => None,
    }
}

fn win32_function_virtual_key_code(code: WinitKeyCode) -> Option<u16> {
    match code {
        WinitKeyCode::F1 => Some(0x70),
        WinitKeyCode::F2 => Some(0x71),
        WinitKeyCode::F3 => Some(0x72),
        WinitKeyCode::F4 => Some(0x73),
        WinitKeyCode::F5 => Some(0x74),
        WinitKeyCode::F6 => Some(0x75),
        WinitKeyCode::F7 => Some(0x76),
        WinitKeyCode::F8 => Some(0x77),
        WinitKeyCode::F9 => Some(0x78),
        WinitKeyCode::F10 => Some(0x79),
        WinitKeyCode::F11 => Some(0x7a),
        WinitKeyCode::F12 => Some(0x7b),
        WinitKeyCode::F13 => Some(0x7c),
        WinitKeyCode::F14 => Some(0x7d),
        WinitKeyCode::F15 => Some(0x7e),
        WinitKeyCode::F16 => Some(0x7f),
        WinitKeyCode::F17 => Some(0x80),
        WinitKeyCode::F18 => Some(0x81),
        WinitKeyCode::F19 => Some(0x82),
        WinitKeyCode::F20 => Some(0x83),
        WinitKeyCode::F21 => Some(0x84),
        WinitKeyCode::F22 => Some(0x85),
        WinitKeyCode::F23 => Some(0x86),
        WinitKeyCode::F24 => Some(0x87),
        _ => None,
    }
}

fn win32_control_key_state(physical_key: PhysicalKey, modifiers: ModifiersState) -> u16 {
    let mut state = 0_u16;
    if modifiers.alt_key() {
        state |= match physical_key {
            PhysicalKey::Code(WinitKeyCode::AltRight) => 0x0001,
            _ => 0x0002,
        };
    }
    if modifiers.control_key() {
        state |= match physical_key {
            PhysicalKey::Code(WinitKeyCode::ControlRight) => 0x0004,
            _ => 0x0008,
        };
    }
    if modifiers.shift_key() {
        state |= 0x0010;
    }
    if modifiers.super_key() {
        state |= 0x0100;
    }
    state
}

fn encode_kitty_event_window_key(
    key: &Key,
    physical_key: PhysicalKey,
    text: Option<&str>,
    modifiers: ModifiersState,
    kitty_keyboard_flags: u16,
    key_event_kind: KittyKeyEventKind,
) -> Option<Vec<u8>> {
    encode_kitty_modifier_window_key(
        physical_key,
        modifiers,
        kitty_keyboard_flags,
        key_event_kind,
    )
    .or_else(|| {
        encode_kitty_keypad_window_key(
            physical_key,
            modifiers,
            kitty_keyboard_flags,
            key_event_kind,
        )
    })
    .or_else(|| {
        encode_kitty_functional_window_key(key, modifiers, kitty_keyboard_flags, key_event_kind)
    })
    .or_else(|| {
        encode_kitty_report_all_window_key(
            key,
            physical_key,
            text,
            modifiers,
            kitty_keyboard_flags,
            key_event_kind,
        )
    })
    .or_else(|| {
        encode_kitty_disambiguated_window_key(
            key,
            physical_key,
            modifiers,
            kitty_keyboard_flags,
            key_event_kind,
        )
    })
}

fn encode_kitty_modifier_window_key(
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
    kitty_keyboard_flags: u16,
    key_event_kind: KittyKeyEventKind,
) -> Option<Vec<u8>> {
    let event_type = kitty_window_event_type(key_event_kind, kitty_keyboard_flags);
    if kitty_keyboard_flags
        & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_REPORT_EVENTS)
        == 0
    {
        return None;
    }
    if key_event_kind == KittyKeyEventKind::Press
        && kitty_keyboard_flags & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL) == 0
    {
        return None;
    }

    let key_code = kitty_window_modifier_key_code(physical_key)?;
    Some(kitty_csi_u_key_with_event(
        key_code,
        kitty_window_modifier(modifiers),
        event_type,
        None,
    ))
}

fn encode_kitty_keypad_window_key(
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
    kitty_keyboard_flags: u16,
    key_event_kind: KittyKeyEventKind,
) -> Option<Vec<u8>> {
    let event_type = kitty_window_event_type(key_event_kind, kitty_keyboard_flags);
    if kitty_keyboard_flags
        & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_REPORT_EVENTS)
        == 0
    {
        return None;
    }
    if key_event_kind == KittyKeyEventKind::Press
        && kitty_keyboard_flags & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL) == 0
    {
        return None;
    }

    let key_code = kitty_window_keypad_code(physical_key)?;
    Some(kitty_csi_u_key_with_event(
        key_code,
        kitty_window_modifier(modifiers),
        event_type,
        None,
    ))
}

fn encode_kitty_functional_window_key(
    key: &Key,
    modifiers: ModifiersState,
    kitty_keyboard_flags: u16,
    key_event_kind: KittyKeyEventKind,
) -> Option<Vec<u8>> {
    let event_type = kitty_window_event_type(key_event_kind, kitty_keyboard_flags);
    if kitty_keyboard_flags
        & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_REPORT_EVENTS)
        == 0
    {
        return None;
    }
    if key_event_kind == KittyKeyEventKind::Press
        && kitty_keyboard_flags & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL) == 0
    {
        return None;
    }

    let Key::Named(named) = key.as_ref() else {
        return None;
    };
    let modifier = kitty_window_modifier(modifiers);
    match named {
        NamedKey::Escape => Some(kitty_csi_u_key_with_event(27, modifier, event_type, None)),
        NamedKey::Enter if kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_ALL != 0 => {
            let associated_text =
                associated_text_from_window_control_key(key_event_kind, kitty_keyboard_flags, 13);
            Some(kitty_csi_u_key_with_event(
                13,
                modifier,
                event_type,
                associated_text.as_deref(),
            ))
        }
        NamedKey::Tab if kitty_window_reports_canonical_tab(modifiers, kitty_keyboard_flags) => {
            Some(kitty_csi_u_key_with_event(9, modifier, event_type, None))
        }
        NamedKey::Backspace if kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_ALL != 0 => {
            Some(kitty_csi_u_key_with_event(127, modifier, event_type, None))
        }
        NamedKey::ArrowUp => Some(kitty_csi_final_key_with_event(b'A', modifier, event_type)),
        NamedKey::ArrowDown => Some(kitty_csi_final_key_with_event(b'B', modifier, event_type)),
        NamedKey::ArrowRight => Some(kitty_csi_final_key_with_event(b'C', modifier, event_type)),
        NamedKey::ArrowLeft => Some(kitty_csi_final_key_with_event(b'D', modifier, event_type)),
        NamedKey::End => Some(kitty_csi_final_key_with_event(b'F', modifier, event_type)),
        NamedKey::Home => Some(kitty_csi_final_key_with_event(b'H', modifier, event_type)),
        NamedKey::Insert => Some(kitty_csi_tilde_key_with_event(2, modifier, event_type)),
        NamedKey::Delete => Some(kitty_csi_tilde_key_with_event(3, modifier, event_type)),
        NamedKey::PageUp => Some(kitty_csi_tilde_key_with_event(5, modifier, event_type)),
        NamedKey::PageDown => Some(kitty_csi_tilde_key_with_event(6, modifier, event_type)),
        NamedKey::F1 => Some(kitty_csi_final_key_with_event(b'P', modifier, event_type)),
        NamedKey::F2 => Some(kitty_csi_final_key_with_event(b'Q', modifier, event_type)),
        NamedKey::F3 => Some(kitty_csi_tilde_key_with_event(13, modifier, event_type)),
        NamedKey::F4 => Some(kitty_csi_final_key_with_event(b'S', modifier, event_type)),
        NamedKey::F5 => Some(kitty_csi_tilde_key_with_event(15, modifier, event_type)),
        NamedKey::F6 => Some(kitty_csi_tilde_key_with_event(17, modifier, event_type)),
        NamedKey::F7 => Some(kitty_csi_tilde_key_with_event(18, modifier, event_type)),
        NamedKey::F8 => Some(kitty_csi_tilde_key_with_event(19, modifier, event_type)),
        NamedKey::F9 => Some(kitty_csi_tilde_key_with_event(20, modifier, event_type)),
        NamedKey::F10 => Some(kitty_csi_tilde_key_with_event(21, modifier, event_type)),
        NamedKey::F11 => Some(kitty_csi_tilde_key_with_event(23, modifier, event_type)),
        NamedKey::F12 => Some(kitty_csi_tilde_key_with_event(24, modifier, event_type)),
        _ => kitty_pua_function_key_code(named)
            .map(|key_code| kitty_csi_u_key_with_event(key_code, modifier, event_type, None)),
    }
}

fn kitty_window_reports_canonical_tab(
    modifiers: ModifiersState,
    kitty_keyboard_flags: u16,
) -> bool {
    if kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_ALL != 0 {
        return true;
    }
    kitty_keyboard_flags & KITTY_KEYBOARD_DISAMBIGUATE != 0
        && modifiers.control_key()
        && modifiers.shift_key()
}

fn encode_kitty_report_all_window_key(
    key: &Key,
    physical_key: PhysicalKey,
    text: Option<&str>,
    modifiers: ModifiersState,
    kitty_keyboard_flags: u16,
    key_event_kind: KittyKeyEventKind,
) -> Option<Vec<u8>> {
    let report_all = kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_ALL != 0;
    let report_text_event = kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_EVENTS != 0
        && key_event_kind != KittyKeyEventKind::Press;
    if !report_all && !report_text_event {
        return None;
    }

    let key_code = match key.as_ref() {
        Key::Character(character) => kitty_window_report_all_key_code(
            character.chars().next()?,
            physical_key,
            modifiers,
            kitty_keyboard_flags,
        ),
        Key::Named(NamedKey::Enter) if report_all => 13.to_string(),
        Key::Named(NamedKey::Tab) if report_all => 9.to_string(),
        Key::Named(NamedKey::Backspace) if report_all => 127.to_string(),
        Key::Named(NamedKey::Escape) if report_all => 27.to_string(),
        _ => return None,
    };
    Some(kitty_csi_u_key_with_event(
        key_code,
        kitty_window_modifier(modifiers),
        kitty_window_event_type(key_event_kind, kitty_keyboard_flags),
        associated_text_from_window_key(text, kitty_keyboard_flags, key_event_kind).as_deref(),
    ))
}

fn encode_kitty_disambiguated_window_key(
    key: &Key,
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
    kitty_keyboard_flags: u16,
    key_event_kind: KittyKeyEventKind,
) -> Option<Vec<u8>> {
    if kitty_keyboard_flags & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL) == 0 {
        return None;
    }
    if !(modifiers.control_key() || modifiers.alt_key() || modifiers.super_key()) {
        return None;
    }

    let Key::Character(character) = key.as_ref() else {
        return None;
    };
    let character = character.chars().next()?;
    let key_code = if kitty_keyboard_flags & KITTY_KEYBOARD_ALTERNATE_KEYS != 0 {
        kitty_window_key_code(character, physical_key, modifiers, kitty_keyboard_flags)
    } else {
        kitty_ascii_key_code(character)?.to_string()
    };
    let modifier = kitty_window_modifier(modifiers)?;
    Some(kitty_csi_u_key_with_event(
        key_code,
        Some(modifier),
        kitty_window_event_type(key_event_kind, kitty_keyboard_flags),
        None,
    ))
}

fn kitty_ascii_key_code(character: char) -> Option<u32> {
    if let Some(key_code) = kitty_unshifted_ascii_key_code(character) {
        Some(key_code)
    } else if character.is_ascii_graphic() || character == ' ' {
        Some(u32::from(character))
    } else {
        None
    }
}

fn kitty_window_report_all_key_code(
    character: char,
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
    kitty_keyboard_flags: u16,
) -> String {
    if character.is_ascii() || !matches!(physical_key, PhysicalKey::Unidentified(_)) {
        kitty_window_key_code(character, physical_key, modifiers, kitty_keyboard_flags)
    } else {
        "0".to_owned()
    }
}

fn kitty_key_code(character: char) -> u32 {
    if character.is_ascii_alphabetic() {
        u32::from(character.to_ascii_lowercase())
    } else {
        u32::from(character)
    }
}

fn kitty_unshifted_ascii_key_code(character: char) -> Option<u32> {
    let unshifted = match character {
        'A'..='Z' => character.to_ascii_lowercase(),
        '~' => '`',
        '!' => '1',
        '@' => '2',
        '#' => '3',
        '$' => '4',
        '%' => '5',
        '^' => '6',
        '&' => '7',
        '*' => '8',
        '(' => '9',
        ')' => '0',
        '_' => '-',
        '+' => '=',
        '{' => '[',
        '}' => ']',
        '|' => '\\',
        ':' => ';',
        '"' => '\'',
        '<' => ',',
        '>' => '.',
        '?' => '/',
        _ => return None,
    };
    Some(u32::from(unshifted))
}

fn kitty_window_keypad_code(physical_key: PhysicalKey) -> Option<u32> {
    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };

    match code {
        WinitKeyCode::Numpad0 => Some(57399),
        WinitKeyCode::Numpad1 => Some(57400),
        WinitKeyCode::Numpad2 => Some(57401),
        WinitKeyCode::Numpad3 => Some(57402),
        WinitKeyCode::Numpad4 => Some(57403),
        WinitKeyCode::Numpad5 => Some(57404),
        WinitKeyCode::Numpad6 => Some(57405),
        WinitKeyCode::Numpad7 => Some(57406),
        WinitKeyCode::Numpad8 => Some(57407),
        WinitKeyCode::Numpad9 => Some(57408),
        WinitKeyCode::NumpadDecimal => Some(57409),
        WinitKeyCode::NumpadDivide => Some(57410),
        WinitKeyCode::NumpadMultiply => Some(57411),
        WinitKeyCode::NumpadSubtract => Some(57412),
        WinitKeyCode::NumpadAdd => Some(57413),
        WinitKeyCode::NumpadEnter => Some(57414),
        WinitKeyCode::NumpadEqual => Some(57415),
        WinitKeyCode::NumpadComma => Some(57416),
        _ => None,
    }
}

fn kitty_window_modifier_key_code(physical_key: PhysicalKey) -> Option<u32> {
    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };

    match code {
        WinitKeyCode::ShiftLeft => Some(57441),
        WinitKeyCode::ControlLeft => Some(57442),
        WinitKeyCode::AltLeft => Some(57443),
        WinitKeyCode::SuperLeft => Some(57444),
        WinitKeyCode::ShiftRight => Some(57447),
        WinitKeyCode::ControlRight => Some(57448),
        WinitKeyCode::AltRight => Some(57449),
        WinitKeyCode::SuperRight => Some(57450),
        _ => None,
    }
}

fn kitty_window_key_code(
    character: char,
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
    kitty_keyboard_flags: u16,
) -> String {
    let base_layout = kitty_base_layout_key(physical_key);
    let primary = if modifiers.shift_key() {
        base_layout.map_or_else(
            || {
                kitty_unshifted_ascii_key_code(character)
                    .unwrap_or_else(|| kitty_key_code(character))
            },
            u32::from,
        )
    } else {
        kitty_key_code(character)
    };

    if kitty_keyboard_flags & KITTY_KEYBOARD_ALTERNATE_KEYS == 0 {
        return primary.to_string();
    }

    let shifted = modifiers
        .shift_key()
        .then_some(u32::from(character))
        .filter(|shifted| *shifted != primary);
    let base = base_layout.map(u32::from).filter(|base| *base != primary);
    match (shifted, base) {
        (Some(shifted), Some(base)) => format!("{primary}:{shifted}:{base}"),
        (Some(shifted), None) => format!("{primary}:{shifted}"),
        (None, Some(base)) => format!("{primary}::{base}"),
        _ => primary.to_string(),
    }
}

fn kitty_base_layout_key(physical_key: PhysicalKey) -> Option<char> {
    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };

    match code {
        WinitKeyCode::Backquote => Some('`'),
        WinitKeyCode::Backslash
        | WinitKeyCode::IntlBackslash
        | WinitKeyCode::IntlRo
        | WinitKeyCode::IntlYen => Some('\\'),
        WinitKeyCode::BracketLeft => Some('['),
        WinitKeyCode::BracketRight => Some(']'),
        WinitKeyCode::Comma => Some(','),
        WinitKeyCode::Digit0 => Some('0'),
        WinitKeyCode::Digit1 => Some('1'),
        WinitKeyCode::Digit2 => Some('2'),
        WinitKeyCode::Digit3 => Some('3'),
        WinitKeyCode::Digit4 => Some('4'),
        WinitKeyCode::Digit5 => Some('5'),
        WinitKeyCode::Digit6 => Some('6'),
        WinitKeyCode::Digit7 => Some('7'),
        WinitKeyCode::Digit8 => Some('8'),
        WinitKeyCode::Digit9 => Some('9'),
        WinitKeyCode::Equal => Some('='),
        WinitKeyCode::KeyA => Some('a'),
        WinitKeyCode::KeyB => Some('b'),
        WinitKeyCode::KeyC => Some('c'),
        WinitKeyCode::KeyD => Some('d'),
        WinitKeyCode::KeyE => Some('e'),
        WinitKeyCode::KeyF => Some('f'),
        WinitKeyCode::KeyG => Some('g'),
        WinitKeyCode::KeyH => Some('h'),
        WinitKeyCode::KeyI => Some('i'),
        WinitKeyCode::KeyJ => Some('j'),
        WinitKeyCode::KeyK => Some('k'),
        WinitKeyCode::KeyL => Some('l'),
        WinitKeyCode::KeyM => Some('m'),
        WinitKeyCode::KeyN => Some('n'),
        WinitKeyCode::KeyO => Some('o'),
        WinitKeyCode::KeyP => Some('p'),
        WinitKeyCode::KeyQ => Some('q'),
        WinitKeyCode::KeyR => Some('r'),
        WinitKeyCode::KeyS => Some('s'),
        WinitKeyCode::KeyT => Some('t'),
        WinitKeyCode::KeyU => Some('u'),
        WinitKeyCode::KeyV => Some('v'),
        WinitKeyCode::KeyW => Some('w'),
        WinitKeyCode::KeyX => Some('x'),
        WinitKeyCode::KeyY => Some('y'),
        WinitKeyCode::KeyZ => Some('z'),
        WinitKeyCode::Minus => Some('-'),
        WinitKeyCode::Period => Some('.'),
        WinitKeyCode::Quote => Some('\''),
        WinitKeyCode::Semicolon => Some(';'),
        WinitKeyCode::Slash => Some('/'),
        WinitKeyCode::Space => Some(' '),
        _ => None,
    }
}

fn associated_text_from_window_key(
    text: Option<&str>,
    kitty_keyboard_flags: u16,
    key_event_kind: KittyKeyEventKind,
) -> Option<String> {
    if kitty_keyboard_flags & (KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_ASSOCIATED_TEXT)
        != (KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_ASSOCIATED_TEXT)
    {
        return None;
    }
    if key_event_kind == KittyKeyEventKind::Release {
        return None;
    }

    associated_text_codepoints(text?.chars())
}

fn associated_text_from_window_control_key(
    key_event_kind: KittyKeyEventKind,
    kitty_keyboard_flags: u16,
    codepoint: u32,
) -> Option<String> {
    if kitty_keyboard_flags & (KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_ASSOCIATED_TEXT)
        != (KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_ASSOCIATED_TEXT)
    {
        return None;
    }
    if key_event_kind == KittyKeyEventKind::Release {
        return None;
    }

    Some(codepoint.to_string())
}

fn associated_text_codepoints(characters: impl IntoIterator<Item = char>) -> Option<String> {
    let mut encoded = String::new();
    for character in characters {
        if character.is_control() {
            return None;
        }
        if !encoded.is_empty() {
            encoded.push(':');
        }
        encoded.push_str(&u32::from(character).to_string());
    }

    if encoded.is_empty() {
        None
    } else {
        Some(encoded)
    }
}

fn kitty_csi_u_key_with_event(
    key_code: impl std::fmt::Display,
    modifier: Option<u8>,
    event_type: Option<u8>,
    associated_text: Option<&str>,
) -> Vec<u8> {
    let modifier = match (modifier, event_type) {
        (Some(modifier), Some(event_type)) => Some(format!("{modifier}:{event_type}")),
        (Some(modifier), None) => Some(modifier.to_string()),
        (None, Some(event_type)) => Some(format!("1:{event_type}")),
        (None, None) => None,
    };

    match (modifier, associated_text) {
        (Some(modifier), Some(text)) => format!("\x1b[{key_code};{modifier};{text}u").into_bytes(),
        (Some(modifier), None) => format!("\x1b[{key_code};{modifier}u").into_bytes(),
        (None, Some(text)) => format!("\x1b[{key_code};;{text}u").into_bytes(),
        (None, None) => format!("\x1b[{key_code}u").into_bytes(),
    }
}

fn kitty_csi_final_key_with_event(
    final_byte: u8,
    modifier: Option<u8>,
    event_type: Option<u8>,
) -> Vec<u8> {
    match modifier {
        Some(modifier) => match event_type {
            Some(event_type) => {
                format!("\x1b[1;{}:{}{}", modifier, event_type, final_byte as char).into_bytes()
            }
            None => format!("\x1b[1;{}{}", modifier, final_byte as char).into_bytes(),
        },
        None => match event_type {
            Some(event_type) => {
                format!("\x1b[1;1:{}{}", event_type, final_byte as char).into_bytes()
            }
            None => vec![0x1b, b'[', final_byte],
        },
    }
}

fn kitty_csi_tilde_key_with_event(
    number: u8,
    modifier: Option<u8>,
    event_type: Option<u8>,
) -> Vec<u8> {
    match modifier {
        Some(modifier) => match event_type {
            Some(event_type) => format!("\x1b[{number};{modifier}:{event_type}~").into_bytes(),
            None => format!("\x1b[{number};{modifier}~").into_bytes(),
        },
        None => match event_type {
            Some(event_type) => format!("\x1b[{number};1:{event_type}~").into_bytes(),
            None => format!("\x1b[{number}~").into_bytes(),
        },
    }
}

fn kitty_window_event_type(
    key_event_kind: KittyKeyEventKind,
    kitty_keyboard_flags: u16,
) -> Option<u8> {
    if kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_EVENTS == 0 {
        return None;
    }

    match key_event_kind {
        KittyKeyEventKind::Press => None,
        KittyKeyEventKind::Repeat => Some(2),
        KittyKeyEventKind::Release => Some(3),
    }
}

fn kitty_pua_function_key_code(named: NamedKey) -> Option<u32> {
    match named {
        NamedKey::CapsLock => return Some(57358),
        NamedKey::ScrollLock => return Some(57359),
        NamedKey::NumLock => return Some(57360),
        NamedKey::PrintScreen => return Some(57361),
        NamedKey::Pause => return Some(57362),
        NamedKey::ContextMenu => return Some(57363),
        NamedKey::MediaPlay => return Some(57428),
        NamedKey::MediaPause => return Some(57429),
        NamedKey::MediaPlayPause => return Some(57430),
        NamedKey::MediaRewind => return Some(57434),
        NamedKey::MediaStop => return Some(57432),
        NamedKey::MediaFastForward => return Some(57433),
        NamedKey::MediaTrackNext => return Some(57435),
        NamedKey::MediaTrackPrevious => return Some(57436),
        NamedKey::MediaRecord => return Some(57437),
        NamedKey::AudioVolumeDown => return Some(57438),
        NamedKey::AudioVolumeUp => return Some(57439),
        NamedKey::AudioVolumeMute => return Some(57440),
        _ => {}
    }

    let offset = match named {
        NamedKey::F13 => 0,
        NamedKey::F14 => 1,
        NamedKey::F15 => 2,
        NamedKey::F16 => 3,
        NamedKey::F17 => 4,
        NamedKey::F18 => 5,
        NamedKey::F19 => 6,
        NamedKey::F20 => 7,
        NamedKey::F21 => 8,
        NamedKey::F22 => 9,
        NamedKey::F23 => 10,
        NamedKey::F24 => 11,
        NamedKey::F25 => 12,
        NamedKey::F26 => 13,
        NamedKey::F27 => 14,
        NamedKey::F28 => 15,
        NamedKey::F29 => 16,
        NamedKey::F30 => 17,
        NamedKey::F31 => 18,
        NamedKey::F32 => 19,
        NamedKey::F33 => 20,
        NamedKey::F34 => 21,
        NamedKey::F35 => 22,
        _ => return None,
    };
    Some(57376 + offset)
}

fn encode_xterm_modify_other_window_key(
    key: &Key,
    modifiers: ModifiersState,
    modify_other_keys: u8,
) -> Option<Vec<u8>> {
    if modify_other_keys == 0 {
        return None;
    }
    let modifier = xterm_window_modifier(modifiers)?;
    let key_code = match key.as_ref() {
        Key::Character(character) => u32::from(character.chars().next()?),
        Key::Named(NamedKey::Enter) => 13,
        Key::Named(NamedKey::Tab) => 9,
        Key::Named(NamedKey::Backspace) => 127,
        Key::Named(NamedKey::Escape) => 27,
        _ => return None,
    };

    Some(format!("\x1b[27;{modifier};{key_code}~").into_bytes())
}

fn scrollback_lines_from_mouse_delta(delta: MouseScrollDelta) -> isize {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => signed_scroll_lines(f64::from(y)),
        MouseScrollDelta::PixelDelta(position) => {
            signed_scroll_lines(position.y / f64::from(CELL_HEIGHT))
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "finite deltas are truncated deliberately and saturated before isize conversion"
)]
fn signed_scroll_lines(value: f64) -> isize {
    if !value.is_finite() {
        return 0;
    }
    if value == 0.0 {
        return 0;
    }

    let direction = if value.is_sign_negative() { -1 } else { 1 };
    let value = value.abs().trunc();
    let lines = if value == 0.0 {
        1
    } else {
        isize::try_from(value as i64).unwrap_or(isize::MAX)
    };
    lines.saturating_mul(direction)
}

fn copy_mode_viewport_top(history_len: usize, scrollback_offset: usize) -> usize {
    history_len.saturating_sub(scrollback_offset.min(history_len))
}

fn apply_isize_delta_to_usize(current: usize, delta: isize, max: usize) -> usize {
    if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta.unsigned_abs()).min(max)
    }
}

fn copy_mode_viewport_cell_for_source_position(
    source_row: usize,
    source_column: usize,
    current_offset: usize,
    history_len: usize,
    size: TerminalSize,
) -> Option<(usize, SelectionCell)> {
    let current_viewport_top = copy_mode_viewport_top(history_len, current_offset);
    if let Some(cell) =
        copy_mode_cell_for_source_position(source_row, source_column, current_viewport_top, size)
    {
        return Some((current_offset.min(history_len), cell));
    }

    let target_offset = if source_row < history_len {
        history_len.saturating_sub(source_row)
    } else {
        0
    };
    let target_viewport_top = copy_mode_viewport_top(history_len, target_offset);
    let cell =
        copy_mode_cell_for_source_position(source_row, source_column, target_viewport_top, size)?;

    Some((target_offset, cell))
}

fn copy_mode_cell_for_source_position(
    source_row: usize,
    source_column: usize,
    viewport_top: usize,
    size: TerminalSize,
) -> Option<SelectionCell> {
    let row = source_row.checked_sub(viewport_top)?;
    if row >= usize::from(size.rows) || source_column >= usize::from(size.columns) {
        return None;
    }

    Some(SelectionCell {
        row: u16::try_from(row).ok()?,
        column: u16::try_from(source_column).ok()?,
    })
}

fn copy_mode_semantic_zone_type_for_key(character: &str) -> Option<SemanticType> {
    if character.eq_ignore_ascii_case("z") || character.eq_ignore_ascii_case("o") {
        Some(SemanticType::Output)
    } else if character.eq_ignore_ascii_case("p") {
        Some(SemanticType::Prompt)
    } else if character.eq_ignore_ascii_case("i") {
        Some(SemanticType::Input)
    } else {
        None
    }
}

fn copy_mode_line_content_bounds(
    terminal: &Terminal,
    domain: TerminalScreenDomain,
    source_row: StableRowIndex,
) -> Option<(usize, usize)> {
    let columns = usize::from(terminal.grid().size().columns);
    if columns == 0 {
        return None;
    }

    let line = copy_mode_source_line(
        terminal,
        SelectionSourceCell {
            domain,
            row: source_row,
            column: 0,
        },
    )?;
    let mut bounds = None;
    for (column, character) in line.chars().enumerate() {
        if character != ' ' {
            bounds = Some(match bounds {
                Some((start, _)) => (start, column),
                None => (column, column),
            });
        }
    }

    Some(bounds.unwrap_or((0, 0)))
}

fn copy_mode_jump_target(
    terminal: &Terminal,
    cursor: SelectionSourceCell,
    jump: WindowCopyJump,
    repeat: bool,
) -> Option<SelectionSourceCell> {
    let columns = usize::from(terminal.grid().size().columns);
    if columns == 0 {
        return None;
    }

    let line = copy_mode_source_line(terminal, cursor)?;
    let mut candidates = line
        .chars()
        .enumerate()
        .filter_map(|(column, character)| (character == jump.target).then_some(column))
        .collect::<Vec<_>>();
    if !jump.forward {
        candidates.reverse();
    }

    let cursor_column = match (jump.prev_char && repeat, jump.forward) {
        (false, _) => cursor.column,
        (true, true) => cursor.column.saturating_add(1),
        (true, false) => cursor.column.saturating_sub(1),
    };

    let target = candidates.into_iter().find(|column| {
        if jump.forward {
            *column > cursor_column
        } else {
            *column < cursor_column
        }
    })?;

    let target_column = match (jump.prev_char, jump.forward) {
        (false, _) => target,
        (true, true) => target.saturating_sub(1),
        (true, false) => target.saturating_add(1),
    }
    .min(columns.saturating_sub(1));

    Some(SelectionSourceCell {
        domain: cursor.domain,
        row: cursor.row,
        column: target_column,
    })
}

fn copy_mode_word_target(
    terminal: &Terminal,
    cursor: SelectionSourceCell,
    movement: WindowCopyWordMovement,
) -> Option<SelectionSourceCell> {
    let columns = usize::from(terminal.grid().size().columns);
    if columns == 0 {
        return None;
    }

    let retained = terminal.retained_stable_range();
    if cursor.domain != terminal.stable_dimensions().domain
        || cursor.row < retained.start
        || cursor.row >= retained.end
    {
        return None;
    }

    match movement {
        WindowCopyWordMovement::Backward => copy_mode_backward_word_target(terminal, cursor),
        WindowCopyWordMovement::Forward => copy_mode_forward_word_target(terminal, cursor),
        WindowCopyWordMovement::End => copy_mode_forward_word_end_target(terminal, cursor),
    }
}

fn copy_mode_backward_word_target(
    terminal: &Terminal,
    cursor: SelectionSourceCell,
) -> Option<SelectionSourceCell> {
    if cursor.column == 0 && cursor.row > 0 {
        let previous_line = copy_mode_source_line(
            terminal,
            SelectionSourceCell {
                domain: cursor.domain,
                row: cursor.row.saturating_sub(1),
                column: 0,
            },
        )?;
        let previous_column = previous_line.chars().count().saturating_sub(1);
        return copy_mode_backward_word_target(
            terminal,
            SelectionSourceCell {
                domain: cursor.domain,
                row: cursor.row.saturating_sub(1),
                column: previous_column,
            },
        );
    }

    let line = copy_mode_source_line(terminal, cursor)?;
    let line_len = line.chars().count();
    if line_len == 0 {
        return None;
    }

    let cursor_column = cursor.column.min(line_len.saturating_sub(1));
    let segments = copy_mode_word_segments(&line);
    let mut target_column = cursor_column;
    let mut last_was_whitespace = false;

    for (index, segment) in copy_mode_prefix_word_segments(&segments, cursor_column)
        .into_iter()
        .rev()
        .enumerate()
    {
        let width = segment.end.saturating_sub(segment.start);
        if width == 0 {
            continue;
        }

        if segment.is_whitespace {
            target_column = target_column.saturating_sub(width);
            last_was_whitespace = true;
            continue;
        }

        last_was_whitespace = false;
        if index == 0 && width == 1 {
            target_column = target_column.saturating_sub(width);
            continue;
        }

        target_column = target_column.saturating_sub(width.saturating_sub(1));
        break;
    }

    if last_was_whitespace && cursor.row > 0 {
        let previous_line = copy_mode_source_line(
            terminal,
            SelectionSourceCell {
                domain: cursor.domain,
                row: cursor.row.saturating_sub(1),
                column: 0,
            },
        )?;
        let previous_column = previous_line.chars().count().saturating_sub(1);
        return copy_mode_backward_word_target(
            terminal,
            SelectionSourceCell {
                domain: cursor.domain,
                row: cursor.row.saturating_sub(1),
                column: previous_column,
            },
        );
    }

    Some(SelectionSourceCell {
        domain: cursor.domain,
        row: cursor.row,
        column: target_column,
    })
}

fn copy_mode_forward_word_target(
    terminal: &Terminal,
    cursor: SelectionSourceCell,
) -> Option<SelectionSourceCell> {
    let line = copy_mode_source_line(terminal, cursor)?;
    let line_len = line.chars().count();
    if line_len == 0 {
        return copy_mode_next_line_content_target(terminal, cursor.domain, cursor.row);
    }

    let cursor_column = cursor.column.min(line_len);
    let mut target_column = cursor_column;
    let suffix = copy_mode_suffix_word_segments(&copy_mode_word_segments(&line), cursor_column);
    let mut segments = suffix.into_iter();

    if let Some(segment) = segments.next() {
        target_column = target_column.saturating_add(segment.end.saturating_sub(cursor_column));
        if !segment.is_whitespace
            && let Some(next_segment) = segments.next()
            && next_segment.is_whitespace
        {
            target_column = target_column.saturating_add(next_segment.end - next_segment.start);
        }
    }

    if target_column >= line_len {
        return copy_mode_next_line_content_target(terminal, cursor.domain, cursor.row).or(Some(
            SelectionSourceCell {
                domain: cursor.domain,
                row: cursor.row,
                column: line_len.saturating_sub(1),
            },
        ));
    }

    Some(SelectionSourceCell {
        domain: cursor.domain,
        row: cursor.row,
        column: target_column,
    })
}

fn copy_mode_forward_word_end_target(
    terminal: &Terminal,
    cursor: SelectionSourceCell,
) -> Option<SelectionSourceCell> {
    let line = copy_mode_source_line(terminal, cursor)?;
    let line_len = line.chars().count();
    if line_len == 0 {
        return copy_mode_next_line_content_target(terminal, cursor.domain, cursor.row);
    }

    let cursor_column = cursor.column.min(line_len.saturating_sub(1));
    if cursor_column >= line_len.saturating_sub(1) {
        return copy_mode_next_line_first_word_end_target(terminal, cursor.domain, cursor.row).or(
            Some(SelectionSourceCell {
                domain: cursor.domain,
                row: cursor.row,
                column: line_len.saturating_sub(1),
            }),
        );
    }

    let suffix = copy_mode_suffix_word_segments(&copy_mode_word_segments(&line), cursor_column);
    let mut segments = suffix.into_iter();
    let first_segment = segments.next()?;
    let mut word_end = first_segment.end;

    if !first_segment.is_whitespace && cursor_column == word_end.saturating_sub(1) {
        for next_segment in segments.by_ref() {
            word_end = next_segment.end;
            if !next_segment.is_whitespace {
                break;
            }
        }
    }

    for next_segment in segments {
        if next_segment.is_whitespace {
            break;
        }
        word_end = next_segment.end;
    }

    Some(SelectionSourceCell {
        domain: cursor.domain,
        row: cursor.row,
        column: word_end.saturating_sub(1),
    })
}

fn copy_mode_next_line_content_target(
    terminal: &Terminal,
    domain: TerminalScreenDomain,
    source_row: StableRowIndex,
) -> Option<SelectionSourceCell> {
    let next_row = source_row.checked_add(1)?;
    let retained = terminal.retained_stable_range();
    if domain != terminal.stable_dimensions().domain || next_row >= retained.end {
        return None;
    }

    let (column, _) = copy_mode_line_content_bounds(terminal, domain, next_row)?;
    Some(SelectionSourceCell {
        domain,
        row: next_row,
        column,
    })
}

fn copy_mode_next_line_first_word_end_target(
    terminal: &Terminal,
    domain: TerminalScreenDomain,
    source_row: StableRowIndex,
) -> Option<SelectionSourceCell> {
    let target = copy_mode_next_line_content_target(terminal, domain, source_row)?;
    copy_mode_forward_word_end_target(terminal, target)
}

fn copy_mode_source_line(terminal: &Terminal, source: SelectionSourceCell) -> Option<String> {
    let columns = usize::from(terminal.grid().size().columns);
    if columns == 0 {
        return None;
    }

    terminal.text_from_stable_selection(StableSelectionRange {
        start: StableSelectionCoordinate {
            domain: source.domain,
            row: source.row,
            column: 0,
        },
        end: StableSelectionCoordinate {
            domain: source.domain,
            row: source.row,
            column: columns.saturating_sub(1),
        },
        rectangular: false,
    })
}

fn copy_mode_word_segments(line: &str) -> Vec<WindowCopyWordSegment> {
    let mut column = 0_usize;
    line.split_word_bounds()
        .filter_map(|word| {
            let width = word.chars().count();
            if width == 0 {
                return None;
            }

            let segment = WindowCopyWordSegment {
                start: column,
                end: column.saturating_add(width),
                is_whitespace: is_copy_mode_whitespace_word(word),
            };
            column = segment.end;
            Some(segment)
        })
        .collect()
}

fn copy_mode_prefix_word_segments(
    segments: &[WindowCopyWordSegment],
    cursor_column: usize,
) -> Vec<WindowCopyWordSegment> {
    segments
        .iter()
        .filter_map(|segment| {
            if segment.start > cursor_column {
                return None;
            }

            Some(WindowCopyWordSegment {
                start: segment.start,
                end: segment.end.min(cursor_column.saturating_add(1)),
                is_whitespace: segment.is_whitespace,
            })
            .filter(|segment| segment.start < segment.end)
        })
        .collect()
}

fn copy_mode_suffix_word_segments(
    segments: &[WindowCopyWordSegment],
    cursor_column: usize,
) -> Vec<WindowCopyWordSegment> {
    segments
        .iter()
        .filter_map(|segment| {
            if segment.end <= cursor_column {
                return None;
            }

            Some(WindowCopyWordSegment {
                start: segment.start.max(cursor_column),
                end: segment.end,
                is_whitespace: segment.is_whitespace,
            })
            .filter(|segment| segment.start < segment.end)
        })
        .collect()
}

fn is_copy_mode_whitespace_word(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_whitespace)
}

fn copy_mode_source_selection(
    copy_mode: &WindowCopyMode,
    terminal: &Terminal,
    word_boundary: &str,
) -> Option<WindowSourceSelection> {
    let size = terminal.grid().size();
    match copy_mode.selection_mode {
        WindowCopySelectionMode::None => None,
        WindowCopySelectionMode::Cell => copy_mode
            .source_anchor
            .map(|anchor| WindowSourceSelection::new(anchor, copy_mode.source_cursor)),
        WindowCopySelectionMode::Word => {
            copy_mode_word_source_selection(terminal, copy_mode.source_cursor, word_boundary)
        }
        WindowCopySelectionMode::Block => copy_mode
            .source_anchor
            .map(|anchor| WindowSourceSelection::rectangular(anchor, copy_mode.source_cursor)),
        WindowCopySelectionMode::Line => {
            if size.columns == 0 {
                return None;
            }

            Some(WindowSourceSelection::new(
                SelectionSourceCell {
                    domain: copy_mode.source_cursor.domain,
                    row: copy_mode.source_cursor.row,
                    column: 0,
                },
                SelectionSourceCell {
                    domain: copy_mode.source_cursor.domain,
                    row: copy_mode.source_cursor.row,
                    column: usize::from(size.columns.saturating_sub(1)),
                },
            ))
        }
        WindowCopySelectionMode::SemanticZone => {
            copy_mode_semantic_zone_source_selection(terminal, copy_mode.source_cursor)
        }
    }
}

fn copy_mode_word_source_selection(
    terminal: &Terminal,
    cursor: SelectionSourceCell,
    word_boundary: &str,
) -> Option<WindowSourceSelection> {
    let columns = usize::from(terminal.grid().size().columns);
    if columns == 0 {
        return None;
    }

    let line = copy_mode_source_line(terminal, cursor)?;
    let characters = line.chars().collect::<Vec<_>>();
    if characters.is_empty() {
        return None;
    }

    let column = cursor.column.min(columns.saturating_sub(1));
    let character = *characters.get(column)?;
    if !is_word_selection_character(character, word_boundary) {
        return None;
    }

    let mut start_column = column;
    while start_column > 0
        && characters
            .get(start_column.saturating_sub(1))
            .is_some_and(|character| is_word_selection_character(*character, word_boundary))
    {
        start_column = start_column.saturating_sub(1);
    }

    let mut end_column = column;
    while end_column + 1 < columns
        && characters
            .get(end_column + 1)
            .is_some_and(|character| is_word_selection_character(*character, word_boundary))
    {
        end_column += 1;
    }

    Some(WindowSourceSelection::new(
        SelectionSourceCell {
            domain: cursor.domain,
            row: cursor.row,
            column: start_column,
        },
        SelectionSourceCell {
            domain: cursor.domain,
            row: cursor.row,
            column: end_column,
        },
    ))
}

fn copy_mode_semantic_zone_source_selection(
    terminal: &Terminal,
    cursor: SelectionSourceCell,
) -> Option<WindowSourceSelection> {
    let size = terminal.grid().size();
    let retained = terminal.retained_stable_range();
    if cursor.domain != terminal.stable_dimensions().domain
        || cursor.row < retained.start
        || cursor.row >= retained.end
        || cursor.column >= usize::from(size.columns)
    {
        return None;
    }

    let zone = terminal.stable_semantic_zone_at(cursor.column, cursor.row)?;
    Some(WindowSourceSelection::new(
        SelectionSourceCell {
            domain: cursor.domain,
            row: zone.start_y,
            column: zone.start_x,
        },
        SelectionSourceCell {
            domain: cursor.domain,
            row: zone.end_y,
            column: zone.end_x,
        },
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionSourceCell {
    domain: TerminalScreenDomain,
    row: StableRowIndex,
    column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowSourceSelection {
    anchor: SelectionSourceCell,
    focus: SelectionSourceCell,
    rectangular: bool,
}

impl WindowSourceSelection {
    const fn new(anchor: SelectionSourceCell, focus: SelectionSourceCell) -> Self {
        Self {
            anchor,
            focus,
            rectangular: false,
        }
    }

    const fn rectangular(anchor: SelectionSourceCell, focus: SelectionSourceCell) -> Self {
        Self {
            anchor,
            focus,
            rectangular: true,
        }
    }

    fn text_from_terminal(self, terminal: &Terminal) -> Option<String> {
        terminal.text_from_stable_selection(StableSelectionRange {
            start: StableSelectionCoordinate {
                domain: self.anchor.domain,
                row: self.anchor.row,
                column: self.anchor.column,
            },
            end: StableSelectionCoordinate {
                domain: self.focus.domain,
                row: self.focus.row,
                column: self.focus.column,
            },
            rectangular: self.rectangular,
        })
    }

    fn viewport_selection(
        self,
        domain: TerminalScreenDomain,
        viewport_top: StableRowIndex,
        size: TerminalSize,
    ) -> Option<WindowSelection> {
        if size.rows == 0
            || size.columns == 0
            || self.anchor.domain != domain
            || self.focus.domain != domain
        {
            return None;
        }

        let (start, end) = self.normalized();
        let viewport_bottom = viewport_top.saturating_add(
            StableRowIndex::try_from(size.rows.saturating_sub(1)).unwrap_or(StableRowIndex::MAX),
        );
        let first_row = start.row.max(viewport_top);
        let last_row = end.row.min(viewport_bottom);
        if first_row > last_row {
            return None;
        }

        let visible_column_end = usize::from(size.columns);
        let first_column = if first_row == start.row {
            start.column
        } else {
            0
        };
        let last_column = if last_row == end.row {
            end.column.min(visible_column_end.saturating_sub(1))
        } else {
            visible_column_end.saturating_sub(1)
        };

        if self.rectangular {
            let (start, end) = self.normalized_rectangular();
            let first_row = start.row.max(viewport_top);
            let last_row = end.row.min(viewport_bottom);
            if first_row > last_row || start.column >= visible_column_end {
                return None;
            }
            let first_column = start.column;
            let last_column = end.column.min(visible_column_end.saturating_sub(1));

            return Some(WindowSelection::rectangular(
                SelectionCell {
                    row: u16::try_from(first_row.saturating_sub(viewport_top)).ok()?,
                    column: u16::try_from(first_column).ok()?,
                },
                SelectionCell {
                    row: u16::try_from(last_row.saturating_sub(viewport_top)).ok()?,
                    column: u16::try_from(last_column).ok()?,
                },
            ));
        }

        if first_row == last_row && first_column > last_column {
            return None;
        }

        Some(WindowSelection::new(
            SelectionCell {
                row: u16::try_from(first_row.saturating_sub(viewport_top)).ok()?,
                column: u16::try_from(first_column).ok()?,
            },
            SelectionCell {
                row: u16::try_from(last_row.saturating_sub(viewport_top)).ok()?,
                column: u16::try_from(last_column).ok()?,
            },
        ))
    }

    const fn normalized(self) -> (SelectionSourceCell, SelectionSourceCell) {
        if self.anchor.row < self.focus.row
            || (self.anchor.row == self.focus.row && self.anchor.column <= self.focus.column)
        {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    const fn normalized_rectangular(self) -> (SelectionSourceCell, SelectionSourceCell) {
        let start_row = if self.anchor.row <= self.focus.row {
            self.anchor.row
        } else {
            self.focus.row
        };
        let end_row = if self.anchor.row >= self.focus.row {
            self.anchor.row
        } else {
            self.focus.row
        };
        let start_column = if self.anchor.column <= self.focus.column {
            self.anchor.column
        } else {
            self.focus.column
        };
        let end_column = if self.anchor.column >= self.focus.column {
            self.anchor.column
        } else {
            self.focus.column
        };

        (
            SelectionSourceCell {
                domain: self.anchor.domain,
                row: start_row,
                column: start_column,
            },
            SelectionSourceCell {
                domain: self.anchor.domain,
                row: end_row,
                column: end_column,
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionCell {
    row: u16,
    column: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowMouseSelectionMode {
    Cell,
    Word,
    Line,
    Block,
    SemanticZone,
}

#[derive(Clone, Copy, Debug)]
struct WindowClick {
    cell: SelectionSourceCell,
    time: Instant,
    count: u8,
}

#[derive(Clone, Copy, Debug)]
struct WindowMouseAssignmentClick {
    button: MouseButton,
    modifiers: ModifiersState,
    mouse_reporting: bool,
    alternate_screen_active: bool,
    time: Instant,
    count: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowSelection {
    anchor: SelectionCell,
    focus: SelectionCell,
    rectangular: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StableOrdinarySelection {
    anchor: SelectionSourceCell,
    focus: SelectionSourceCell,
    rectangular: bool,
    sequence: SequenceNo,
}

impl StableOrdinarySelection {
    const fn new(
        anchor: SelectionSourceCell,
        focus: SelectionSourceCell,
        sequence: SequenceNo,
    ) -> Self {
        Self {
            anchor,
            focus,
            rectangular: false,
            sequence,
        }
    }

    const fn rectangular(
        anchor: SelectionSourceCell,
        focus: SelectionSourceCell,
        sequence: SequenceNo,
    ) -> Self {
        Self {
            anchor,
            focus,
            rectangular: true,
            sequence,
        }
    }

    const fn source_selection(self) -> WindowSourceSelection {
        WindowSourceSelection {
            anchor: self.anchor,
            focus: self.focus,
            rectangular: self.rectangular,
        }
    }

    fn viewport_selection(
        self,
        domain: TerminalScreenDomain,
        viewport_top: StableRowIndex,
        size: TerminalSize,
    ) -> Option<WindowSelection> {
        self.source_selection()
            .viewport_selection(domain, viewport_top, size)
    }

    fn text_from_terminal(self, terminal: &Terminal) -> Option<String> {
        self.source_selection().text_from_terminal(terminal)
    }

    fn set_focus(&mut self, focus: SelectionSourceCell) {
        self.focus = focus;
    }

    fn is_single_cell(self) -> bool {
        self.anchor.domain == self.focus.domain
            && self.anchor.row == self.focus.row
            && self.anchor.column == self.focus.column
    }
}

fn ordinary_selection_is_invalidated_by_visible_dirty_rows(
    terminal: &Terminal,
    viewport_top: Option<StableRowIndex>,
    ordinary: Option<StableOrdinarySelection>,
) -> bool {
    let Some(ordinary) = ordinary else {
        return false;
    };
    let dimensions = terminal.stable_dimensions();
    if ordinary.anchor.domain != dimensions.domain || ordinary.focus.domain != dimensions.domain {
        return false;
    }

    let visible_rows = terminal.viewport_stable_range(viewport_top);
    if visible_rows.is_empty() {
        return false;
    }
    let (selection_start, selection_end) = ordinary.source_selection().normalized();
    let selected_rows = selection_start.row..selection_end.row.saturating_add(1);
    terminal
        .changed_stable_rows_since(visible_rows, ordinary.sequence)
        .into_iter()
        .any(|row| selected_rows.contains(&row))
}

impl WindowSelection {
    const fn new(anchor: SelectionCell, focus: SelectionCell) -> Self {
        Self {
            anchor,
            focus,
            rectangular: false,
        }
    }

    const fn rectangular(anchor: SelectionCell, focus: SelectionCell) -> Self {
        Self {
            anchor,
            focus,
            rectangular: true,
        }
    }

    fn contains(self, row: u16, column: u16, size: TerminalSize) -> bool {
        if row >= size.rows || column >= size.columns {
            return false;
        }

        if self.rectangular {
            let (start, end) = self.normalized_rectangular();
            return row >= start.row
                && row <= end.row
                && column >= start.column
                && column <= end.column;
        }

        let (start, end) = self.normalized();
        if row < start.row || row > end.row {
            return false;
        }

        if start.row == end.row {
            return column >= start.column && column <= end.column;
        }

        if row == start.row {
            column >= start.column
        } else if row == end.row {
            column <= end.column
        } else {
            true
        }
    }

    #[cfg(test)]
    fn text_from_snapshot(self, snapshot: &TerminalRenderSnapshot, size: TerminalSize) -> String {
        if size.columns == 0 || size.rows == 0 {
            return String::new();
        }

        if self.rectangular {
            let (start, end) = self.normalized_rectangular();
            let mut lines = Vec::new();
            for row in start.row..=end.row.min(size.rows.saturating_sub(1)) {
                let mut line = String::new();
                for column in start.column..=end.column.min(size.columns.saturating_sub(1)) {
                    line.push(snapshot_character(snapshot, row, column));
                }
                trim_trailing_spaces(&mut line);
                lines.push(line);
            }
            return lines.join("\n");
        }

        let (start, end) = self.normalized();
        let mut lines = Vec::new();
        for row in start.row..=end.row.min(size.rows.saturating_sub(1)) {
            let first_column = if row == start.row { start.column } else { 0 };
            let last_column = if row == end.row {
                end.column.min(size.columns.saturating_sub(1))
            } else {
                size.columns.saturating_sub(1)
            };
            if first_column > last_column {
                lines.push(String::new());
                continue;
            }

            let mut line = String::new();
            for column in first_column..=last_column {
                line.push(snapshot_character(snapshot, row, column));
            }
            trim_trailing_spaces(&mut line);
            lines.push(line);
        }

        lines.join("\n")
    }

    const fn normalized(self) -> (SelectionCell, SelectionCell) {
        if self.anchor.row < self.focus.row
            || (self.anchor.row == self.focus.row && self.anchor.column <= self.focus.column)
        {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    const fn normalized_rectangular(self) -> (SelectionCell, SelectionCell) {
        let start_row = if self.anchor.row <= self.focus.row {
            self.anchor.row
        } else {
            self.focus.row
        };
        let end_row = if self.anchor.row >= self.focus.row {
            self.anchor.row
        } else {
            self.focus.row
        };
        let start_column = if self.anchor.column <= self.focus.column {
            self.anchor.column
        } else {
            self.focus.column
        };
        let end_column = if self.anchor.column >= self.focus.column {
            self.anchor.column
        } else {
            self.focus.column
        };

        (
            SelectionCell {
                row: start_row,
                column: start_column,
            },
            SelectionCell {
                row: end_row,
                column: end_column,
            },
        )
    }
}

fn compare_selection_source_cell(
    left: SelectionSourceCell,
    right: SelectionSourceCell,
) -> std::cmp::Ordering {
    debug_assert_eq!(left.domain, right.domain);
    (left.row, left.column).cmp(&(right.row, right.column))
}

fn stable_selection_focus_for_extension(
    current: StableOrdinarySelection,
    target: WindowSourceSelection,
) -> SelectionSourceCell {
    let (target_start, target_end) = target.normalized();
    match (
        compare_selection_source_cell(current.anchor, target_start),
        compare_selection_source_cell(current.anchor, target_end),
    ) {
        (std::cmp::Ordering::Greater, _) => target_start,
        (_, std::cmp::Ordering::Less) => target_end,
        _ if compare_selection_source_cell(current.focus, current.anchor)
            == std::cmp::Ordering::Less =>
        {
            target_start
        }
        _ => target_end,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowCopySelectionMode {
    None,
    Cell,
    Word,
    Block,
    Line,
    SemanticZone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowCopyDestination {
    Clipboard,
    PrimarySelection,
    ClipboardAndPrimarySelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowPasteSource {
    Clipboard,
    PrimarySelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowFontSizeAction {
    Decrease,
    Increase,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowCopyMode {
    cursor: SelectionCell,
    source_cursor: SelectionSourceCell,
    pending_jump: Option<WindowCopyPendingJump>,
    last_jump: Option<WindowCopyJump>,
    search_direction: Option<SearchDirection>,
    selection_mode: WindowCopySelectionMode,
    anchor: Option<SelectionCell>,
    source_anchor: Option<SelectionSourceCell>,
}

mod pane_transient_overlay {
    use std::cell::{Ref, RefCell};

    use super::{
        PaneStableViewport, SelectionCell, SelectionSourceCell, StableOrdinarySelection,
        StableRowIndex, Terminal, TerminalScreenDomain, WindowCopyMode, WindowQuickSelect,
        WindowSearch, WindowSearchMatch, WindowSearchMatchType,
        ordinary_selection_is_invalidated_by_visible_dirty_rows, window_search_matches_with_type,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum WindowCopySearchMode {
        Search,
        Copy,
    }

    #[derive(Debug)]
    struct WindowCopySearchController {
        mode: WindowCopySearchMode,
        copy_mode_retained: bool,
        copy_mode: WindowCopyMode,
        search: Option<WindowSearch>,
    }

    #[derive(Debug)]
    enum PaneTransientOverlay {
        CopySearch(WindowCopySearchController),
        QuickSelect(WindowQuickSelect),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct WindowSearchMatchCacheKey {
        query: String,
        match_type: WindowSearchMatchType,
        terminal_sequence: usize,
        owner_terminal_epoch: u64,
        screen_identity_generation: usize,
        domain: TerminalScreenDomain,
        retained_start: StableRowIndex,
        retained_end: StableRowIndex,
        rows: u16,
        columns: u16,
    }

    impl WindowSearchMatchCacheKey {
        fn new(
            terminal: &Terminal,
            owner_terminal_epoch: u64,
            query: &str,
            match_type: WindowSearchMatchType,
        ) -> Self {
            let dimensions = terminal.stable_dimensions();
            let retained = terminal.retained_stable_range();
            let size = terminal.grid().size();
            Self {
                query: query.to_owned(),
                match_type,
                terminal_sequence: terminal.current_seqno(),
                owner_terminal_epoch,
                screen_identity_generation: terminal.screen_identity_generation(),
                domain: dimensions.domain,
                retained_start: retained.start,
                retained_end: retained.end,
                rows: size.rows,
                columns: size.columns,
            }
        }

        fn matches(
            &self,
            terminal: &Terminal,
            owner_terminal_epoch: u64,
            search: &WindowSearch,
        ) -> bool {
            let dimensions = terminal.stable_dimensions();
            let retained = terminal.retained_stable_range();
            let size = terminal.grid().size();
            self.query == search.query
                && self.match_type == search.match_type
                && self.terminal_sequence == terminal.current_seqno()
                && self.owner_terminal_epoch == owner_terminal_epoch
                && self.screen_identity_generation == terminal.screen_identity_generation()
                && self.domain == dimensions.domain
                && self.retained_start == retained.start
                && self.retained_end == retained.end
                && self.rows == size.rows
                && self.columns == size.columns
        }
    }

    #[cfg(test)]
    #[derive(Debug, PartialEq)]
    pub(super) struct SearchMatchCacheKeyForTest(WindowSearchMatchCacheKey);

    #[cfg(test)]
    pub(super) fn search_match_cache_key_for_test(
        terminal: &Terminal,
        owner_terminal_epoch: u64,
        query: &str,
        match_type: WindowSearchMatchType,
    ) -> SearchMatchCacheKeyForTest {
        SearchMatchCacheKeyForTest(WindowSearchMatchCacheKey::new(
            terminal,
            owner_terminal_epoch,
            query,
            match_type,
        ))
    }

    #[derive(Debug, Default)]
    struct WindowSearchMatchCache {
        key: Option<WindowSearchMatchCacheKey>,
        matches: Vec<WindowSearchMatch>,
        #[cfg(test)]
        recompute_count: usize,
    }

    impl WindowSearchMatchCache {
        fn clear(&mut self) {
            self.key = None;
            self.matches.clear();
        }
    }

    #[derive(Debug)]
    pub(super) struct PaneUiState {
        pub(super) stable_viewport: PaneStableViewport,
        pub(super) ordinary_selection: Option<StableOrdinarySelection>,
        overlay: Option<PaneTransientOverlay>,
        owner_terminal_epoch: u64,
        search_match_cache: RefCell<WindowSearchMatchCache>,
    }

    impl Default for PaneUiState {
        fn default() -> Self {
            Self {
                stable_viewport: PaneStableViewport::default(),
                ordinary_selection: None,
                overlay: None,
                owner_terminal_epoch: 0,
                search_match_cache: RefCell::new(WindowSearchMatchCache::default()),
            }
        }
    }

    impl PaneUiState {
        pub(super) fn reset_after_main_screen_reflow(&mut self) {
            self.stable_viewport = PaneStableViewport::default();
            self.ordinary_selection = None;
            self.search_match_cache.get_mut().clear();
        }

        pub(super) fn reconcile_after_main_screen_reflow(&mut self, terminal: &Terminal) {
            self.reset_after_main_screen_reflow();

            match self.overlay.as_mut() {
                Some(PaneTransientOverlay::CopySearch(controller)) => {
                    let dimensions = terminal.stable_dimensions();
                    controller.copy_mode.cursor = SelectionCell { row: 0, column: 0 };
                    controller.copy_mode.source_cursor = SelectionSourceCell {
                        domain: dimensions.domain,
                        row: dimensions.physical_top,
                        column: 0,
                    };
                    controller.copy_mode.anchor = None;
                    controller.copy_mode.source_anchor = None;
                    controller.copy_mode.pending_jump = None;
                    controller.copy_mode.last_jump = None;
                    controller.copy_mode.search_direction = None;
                    if let Some(search) = controller.search.as_mut() {
                        search.current = None;
                    }
                }
                Some(PaneTransientOverlay::QuickSelect(quick_select)) => {
                    quick_select.rebuild_after_main_screen_reflow(terminal);
                }
                None => {}
            }

            self.refresh_search_match_cache(terminal);
        }

        pub(super) fn enter_search(
            &mut self,
            initial_copy_mode: WindowCopyMode,
            mut requested: WindowSearch,
        ) {
            requested.editing = true;
            if let Some(PaneTransientOverlay::CopySearch(controller)) = self.overlay.as_mut() {
                match controller.search.as_mut() {
                    Some(retained)
                        if retained.query == requested.query
                            && retained.match_type == requested.match_type =>
                    {
                        retained.editing = true;
                    }
                    Some(_) | None => {
                        requested.current = None;
                        controller.search = Some(requested);
                    }
                }
                controller.mode = WindowCopySearchMode::Search;
            } else {
                requested.current = None;
                self.overlay = Some(PaneTransientOverlay::CopySearch(
                    WindowCopySearchController {
                        mode: WindowCopySearchMode::Search,
                        copy_mode_retained: true,
                        copy_mode: initial_copy_mode,
                        search: Some(requested),
                    },
                ));
            }
        }

        pub(super) fn enter_copy_mode(&mut self, initial_copy_mode: WindowCopyMode) {
            match self.overlay.as_mut() {
                Some(PaneTransientOverlay::CopySearch(controller)) => {
                    if let Some(search) = controller.search.as_mut() {
                        search.editing = false;
                    }
                    controller.mode = WindowCopySearchMode::Copy;
                    controller.copy_mode_retained = true;
                }
                _ => {
                    self.overlay = Some(PaneTransientOverlay::CopySearch(
                        WindowCopySearchController {
                            mode: WindowCopySearchMode::Copy,
                            copy_mode_retained: true,
                            copy_mode: initial_copy_mode,
                            search: None,
                        },
                    ));
                }
            }
        }

        pub(super) fn enter_quick_select(&mut self, quick_select: WindowQuickSelect) {
            self.overlay = Some(PaneTransientOverlay::QuickSelect(quick_select));
            self.search_match_cache.get_mut().clear();
        }

        pub(super) fn exit_overlay(&mut self) {
            self.overlay = None;
            self.search_match_cache.get_mut().clear();
        }

        pub(super) fn prepare_for_new_window(&mut self) {
            self.ordinary_selection = None;
            self.overlay = None;
            self.search_match_cache.get_mut().clear();
        }

        pub(super) fn retire_terminal_identity(&mut self) {
            self.ordinary_selection = None;
            self.overlay = None;
            self.owner_terminal_epoch = self.owner_terminal_epoch.saturating_add(1);
            self.search_match_cache.get_mut().clear();
        }

        #[expect(
            clippy::too_many_lines,
            reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
        )]
        pub(super) fn reconcile_stable_coordinates(&mut self, terminal: &Terminal) {
            self.stable_viewport.clamp_main(terminal);
            let dimensions = terminal.stable_dimensions();
            let retained = terminal.retained_stable_range();
            let viewport_driver_row = match self.overlay.as_ref() {
                Some(PaneTransientOverlay::CopySearch(controller)) => match controller.mode {
                    WindowCopySearchMode::Search => controller
                        .search
                        .as_ref()
                        .and_then(|search| search.current)
                        .filter(|matched| matched.is_retained(terminal))
                        .map(|matched| matched.source_row),
                    WindowCopySearchMode::Copy
                        if controller.copy_mode_retained
                            && source_cell_is_retained(
                                controller.copy_mode.source_cursor,
                                dimensions.domain,
                                &retained,
                            ) =>
                    {
                        Some(controller.copy_mode.source_cursor.row)
                    }
                    WindowCopySearchMode::Copy => None,
                },
                _ => None,
            };
            if let Some(row) = viewport_driver_row {
                self.stable_viewport.ensure_main_row_visible(terminal, row);
            }
            let viewport_top = self
                .stable_viewport
                .active_top(terminal)
                .unwrap_or(dimensions.physical_top);
            let size = terminal.grid().size();

            let retire_overlay = match self.overlay.as_mut() {
                Some(PaneTransientOverlay::CopySearch(controller)) => {
                    let copy_retained = !controller.copy_mode_retained
                        || (source_cell_is_retained(
                            controller.copy_mode.source_cursor,
                            dimensions.domain,
                            &retained,
                        ) && controller.copy_mode.source_anchor.is_none_or(|anchor| {
                            source_cell_is_retained(anchor, dimensions.domain, &retained)
                        }));
                    if copy_retained {
                        if let Some(search) = controller.search.as_mut()
                            && search
                                .current
                                .is_some_and(|matched| !matched.is_retained(terminal))
                        {
                            search.current = None;
                        }
                        if controller.copy_mode_retained {
                            let cursor = (size.columns > 0).then(|| SelectionSourceCell {
                                column: controller
                                    .copy_mode
                                    .source_cursor
                                    .column
                                    .min(usize::from(size.columns.saturating_sub(1))),
                                ..controller.copy_mode.source_cursor
                            });
                            let cursor = cursor.and_then(|cursor| {
                                viewport_cell_for_source(
                                    cursor,
                                    dimensions.domain,
                                    viewport_top,
                                    size,
                                )
                            });
                            let anchor = controller.copy_mode.source_anchor.and_then(|anchor| {
                                viewport_cell_for_source(
                                    anchor,
                                    dimensions.domain,
                                    viewport_top,
                                    size,
                                )
                            });
                            if let Some(cursor) = cursor {
                                controller.copy_mode.cursor = cursor;
                                controller.copy_mode.anchor = anchor;
                            } else {
                                controller.copy_mode.cursor = SelectionCell { row: 0, column: 0 };
                                controller.copy_mode.anchor = None;
                            }
                            false
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                }
                Some(PaneTransientOverlay::QuickSelect(quick_select)) => {
                    if quick_select.matches.len() == quick_select.labels.len() {
                        let current_match = quick_select.current_match();
                        let retained_pairs = quick_select
                            .matches
                            .iter()
                            .copied()
                            .zip(quick_select.labels.iter().cloned())
                            .filter(|(matched, _)| matched.is_retained(terminal))
                            .collect::<Vec<_>>();
                        quick_select.matches =
                            retained_pairs.iter().map(|(matched, _)| *matched).collect();
                        quick_select.labels =
                            retained_pairs.into_iter().map(|(_, label)| label).collect();
                        match current_match {
                            Some(current_match) => {
                                if let Some(current) = quick_select
                                    .matches
                                    .iter()
                                    .position(|matched| *matched == current_match)
                                {
                                    quick_select.current = current;
                                    false
                                } else {
                                    true
                                }
                            }
                            None => true,
                        }
                    } else {
                        true
                    }
                }
                None => false,
            };
            if retire_overlay {
                self.overlay = None;
                self.search_match_cache.get_mut().clear();
            }
        }

        pub(super) fn reconcile_terminal_mutation(&mut self, terminal: &Terminal) {
            self.reconcile_terminal_resize(terminal, false);
        }

        pub(super) fn reconcile_terminal_resize(
            &mut self,
            terminal: &Terminal,
            preserve_ordinary_selection: bool,
        ) {
            self.reconcile_stable_coordinates(terminal);
            if !preserve_ordinary_selection
                && !self.overlay_active()
                && ordinary_selection_is_invalidated_by_visible_dirty_rows(
                    terminal,
                    self.stable_viewport.active_top(terminal),
                    self.ordinary_selection,
                )
            {
                self.ordinary_selection = None;
            }
            self.refresh_search_match_cache(terminal);
        }

        pub(super) fn copy_search_mode(&self) -> Option<WindowCopySearchMode> {
            self.copy_search().map(|controller| controller.mode)
        }

        pub(super) fn copy_mode(&self) -> Option<&WindowCopyMode> {
            let controller = self.copy_search()?;
            (controller.mode == WindowCopySearchMode::Copy && controller.copy_mode_retained)
                .then_some(&controller.copy_mode)
        }

        pub(super) fn copy_mode_mut(&mut self) -> Option<&mut WindowCopyMode> {
            let controller = self.copy_search_mut()?;
            (controller.mode == WindowCopySearchMode::Copy && controller.copy_mode_retained)
                .then_some(&mut controller.copy_mode)
        }

        pub(super) fn retained_copy_mode(&self) -> Option<&WindowCopyMode> {
            let controller = self.copy_search()?;
            controller
                .copy_mode_retained
                .then_some(&controller.copy_mode)
        }

        pub(super) fn retained_copy_mode_mut(&mut self) -> Option<&mut WindowCopyMode> {
            let controller = self.copy_search_mut()?;
            controller
                .copy_mode_retained
                .then_some(&mut controller.copy_mode)
        }

        pub(super) fn search(&self) -> Option<&WindowSearch> {
            let controller = self.copy_search()?;
            (controller.mode == WindowCopySearchMode::Search).then(|| {
                controller
                    .search
                    .as_ref()
                    .expect("search mode always owns search state")
            })
        }

        pub(super) fn retained_search(&self) -> Option<&WindowSearch> {
            self.copy_search()?.search.as_ref()
        }

        pub(super) fn refresh_search_match_cache(&self, terminal: &Terminal) -> bool {
            let Some(search) = self
                .retained_search()
                .filter(|search| !search.query.is_empty())
            else {
                self.search_match_cache.borrow_mut().clear();
                return false;
            };
            {
                let cache = self.search_match_cache.borrow();
                if cache
                    .key
                    .as_ref()
                    .is_some_and(|key| key.matches(terminal, self.owner_terminal_epoch, search))
                {
                    return false;
                }
            }

            let key = WindowSearchMatchCacheKey::new(
                terminal,
                self.owner_terminal_epoch,
                &search.query,
                search.match_type,
            );
            let matches =
                window_search_matches_with_type(terminal, &search.query, search.match_type);
            let mut cache = self.search_match_cache.borrow_mut();
            cache.key = Some(key);
            cache.matches = matches;
            #[cfg(test)]
            {
                cache.recompute_count = cache.recompute_count.saturating_add(1);
            }
            true
        }

        pub(super) fn cached_search_matches<'a>(
            &'a self,
            terminal: &Terminal,
        ) -> Option<Ref<'a, [WindowSearchMatch]>> {
            self.refresh_search_match_cache(terminal);
            let search = self
                .retained_search()
                .filter(|search| !search.query.is_empty())?;
            let cache = self.search_match_cache.borrow();
            if !cache
                .key
                .as_ref()
                .is_some_and(|key| key.matches(terminal, self.owner_terminal_epoch, search))
            {
                return None;
            }
            Some(Ref::map(cache, |cache| cache.matches.as_slice()))
        }

        #[cfg(test)]
        pub(super) fn search_match_cache_recompute_count(&self) -> usize {
            self.search_match_cache.borrow().recompute_count
        }

        #[cfg(test)]
        pub(super) fn reset_search_match_cache_recompute_count(&self) {
            self.search_match_cache.borrow_mut().recompute_count = 0;
        }

        pub(super) fn set_search_current(&mut self, current: Option<WindowSearchMatch>) -> bool {
            let Some(search) = self
                .copy_search_mut()
                .and_then(|controller| controller.search.as_mut())
            else {
                return false;
            };
            search.current = current;
            true
        }

        pub(super) fn replace_search_pattern(
            &mut self,
            query: String,
            match_type: WindowSearchMatchType,
        ) -> Option<bool> {
            let controller = self.copy_search_mut()?;
            if controller
                .search
                .as_ref()
                .is_some_and(|search| search.query == query && search.match_type == match_type)
            {
                return Some(false);
            }
            controller.search = Some(WindowSearch {
                query,
                current: None,
                match_type,
                editing: controller.mode == WindowCopySearchMode::Search,
            });
            self.search_match_cache.get_mut().clear();
            Some(true)
        }

        pub(super) fn set_search_editing(&mut self, editing: bool) -> bool {
            let Some(controller) = self.copy_search_mut() else {
                return false;
            };
            let Some(search) = controller.search.as_mut() else {
                return false;
            };
            controller.mode = if editing {
                WindowCopySearchMode::Search
            } else {
                WindowCopySearchMode::Copy
            };
            search.editing = editing;
            true
        }

        pub(super) fn quick_select(&self) -> Option<&WindowQuickSelect> {
            match self.overlay.as_ref()? {
                PaneTransientOverlay::QuickSelect(quick_select) => Some(quick_select),
                PaneTransientOverlay::CopySearch(_) => None,
            }
        }

        pub(super) fn quick_select_mut(&mut self) -> Option<&mut WindowQuickSelect> {
            match self.overlay.as_mut()? {
                PaneTransientOverlay::QuickSelect(quick_select) => Some(quick_select),
                PaneTransientOverlay::CopySearch(_) => None,
            }
        }

        pub(super) fn overlay_active(&self) -> bool {
            self.overlay.is_some()
        }

        fn copy_search(&self) -> Option<&WindowCopySearchController> {
            match self.overlay.as_ref()? {
                PaneTransientOverlay::CopySearch(controller) => Some(controller),
                PaneTransientOverlay::QuickSelect(_) => None,
            }
        }

        fn copy_search_mut(&mut self) -> Option<&mut WindowCopySearchController> {
            match self.overlay.as_mut()? {
                PaneTransientOverlay::CopySearch(controller) => Some(controller),
                PaneTransientOverlay::QuickSelect(_) => None,
            }
        }
    }

    fn source_cell_is_retained(
        cell: SelectionSourceCell,
        domain: super::TerminalScreenDomain,
        retained: &std::ops::Range<super::StableRowIndex>,
    ) -> bool {
        cell.domain == domain && retained.contains(&cell.row)
    }

    fn viewport_cell_for_source(
        source: SelectionSourceCell,
        domain: super::TerminalScreenDomain,
        viewport_top: super::StableRowIndex,
        size: super::TerminalSize,
    ) -> Option<SelectionCell> {
        if source.domain != domain || source.column >= usize::from(size.columns) {
            return None;
        }
        let row = source.row.checked_sub(viewport_top)?;
        let row = u16::try_from(row).ok()?;
        if row >= size.rows {
            return None;
        }
        Some(SelectionCell {
            row,
            column: u16::try_from(source.column).ok()?,
        })
    }
}

use pane_transient_overlay::{PaneUiState, WindowCopySearchMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowCopyPendingJump {
    forward: bool,
    prev_char: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowCopyJump {
    forward: bool,
    prev_char: bool,
    target: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowCopyWordMovement {
    Backward,
    Forward,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowCopyModeAssignment {
    AcceptPattern,
    Close,
    ClearPattern,
    ClearSelectionMode,
    CycleMatchType,
    EditPattern,
    JumpAgain,
    JumpReverse,
    StartJump {
        forward: bool,
        prev_char: bool,
    },
    MoveBackwardSemanticZone,
    MoveSemanticZoneOfType {
        delta: isize,
        semantic_type: SemanticType,
    },
    MoveBackwardWord,
    MoveDown,
    MoveForwardSemanticZone,
    MoveForwardWord,
    MoveForwardWordEnd,
    MoveLeft,
    MoveRight,
    MoveToEndOfLineContent,
    MoveToScrollbackBottom,
    MoveToScrollbackTop,
    MoveToSelectionOtherEnd,
    MoveToSelectionOtherEndHoriz,
    MoveToStartOfLine,
    MoveToStartOfLineContent,
    MoveToStartOfNextLine,
    MoveToViewportBottom,
    MoveToViewportMiddle,
    MoveToViewportTop,
    MoveUp,
    MoveByPage(WindowScrollByPageAmount),
    PageDown,
    PageUp,
    NextMatch,
    NextMatchPage,
    PriorMatch,
    PriorMatchPage,
    SetSelectionMode(WindowCopySelectionMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowCopyWordSegment {
    start: usize,
    end: usize,
    is_whitespace: bool,
}

#[derive(Debug, Default, Clone)]
struct WindowQuickSelect {
    current: usize,
    matches: Vec<WindowSearchMatch>,
    labels: Vec<String>,
    input: String,
    reflow_config: Option<WindowQuickSelectReflowConfig>,
    action_label: Option<String>,
    action: WindowQuickSelectAction,
    skip_action_on_paste: bool,
}

#[derive(Debug, Clone)]
struct WindowQuickSelectReflowConfig {
    alphabet: String,
    patterns: Vec<(String, Option<usize>)>,
    scope_lines: usize,
}

impl WindowQuickSelect {
    fn current_match(&self) -> Option<WindowSearchMatch> {
        self.matches.get(self.current).copied()
    }

    fn match_for_label(&self, input: &str) -> Option<WindowSearchMatch> {
        let input = input.to_ascii_lowercase();
        self.labels
            .iter()
            .position(|label| label == &input)
            .and_then(|index| self.matches.get(index))
            .copied()
    }

    fn has_label_prefix(&self, input: &str) -> bool {
        let input = input.to_ascii_lowercase();
        self.labels.iter().any(|label| label.starts_with(&input))
    }

    fn rebuild_after_main_screen_reflow(&mut self, terminal: &Terminal) {
        let Some(config) = self.reflow_config.as_ref() else {
            self.current = 0;
            self.matches.clear();
            self.labels.clear();
            return;
        };
        let patterns = config
            .patterns
            .iter()
            .map(|(regex, capture)| WindowQuickSelectPatternRef {
                regex,
                capture: *capture,
            })
            .collect::<Vec<_>>();
        let (row_start, row_end) = quick_select_source_row_scope(terminal, 0, config.scope_lines);
        self.current = 0;
        self.matches =
            find_window_quick_select_matches_with_patterns(terminal, &patterns, row_start, row_end);
        self.labels =
            quick_select_labels_for_alphabet_by_match(&config.alphabet, self.matches.len());
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum WindowQuickSelectAction {
    #[default]
    Copy,
    CopyTo(WindowCopyDestination),
    OpenUri,
    Nop,
    PasteFrom(WindowPasteSource),
    SendString(String),
    SendSelectedText,
    PasteSelectedText,
    SendKey(WindowSendKey),
    EmitEvent(WindowEmitEvent),
    Multiple(Vec<WindowCommand>),
    ActivateKeyTable(WindowActivateKeyTable),
    PopKeyTable,
    ClearKeyTableStack,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct WindowPaneSelect {
    labels: Vec<WindowPaneSelectLabel>,
    input: String,
    mode: WindowPaneSelectMode,
    show_pane_ids: bool,
}

impl WindowPaneSelect {
    fn from_panes(
        panes: &[rssh_core::app_shell::Pane],
        mode: WindowPaneSelectMode,
        show_pane_ids: bool,
        alphabet: &str,
    ) -> Self {
        Self {
            labels: pane_select_labels(panes, alphabet),
            input: String::new(),
            mode,
            show_pane_ids,
        }
    }

    fn pane_for_label(&self, input: &str) -> Option<rssh_core::PaneId> {
        self.labels
            .iter()
            .find(|label| label.label == input)
            .map(|label| label.pane_id)
    }

    fn has_label_prefix(&self, input: &str) -> bool {
        self.labels
            .iter()
            .any(|label| label.label.starts_with(input))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowTabNavigator {
    tabs: Vec<WindowTabNavigatorEntry>,
    selected: usize,
}

impl WindowTabNavigator {
    fn from_tabs(tabs: &[rssh_core::app_shell::Tab], active_tab: rssh_core::TabId) -> Self {
        let entries = tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| WindowTabNavigatorEntry {
                tab_id: tab.id(),
                title: tab
                    .title()
                    .map_or_else(|| format!("Tab {}", index + 1), str::to_owned),
            })
            .collect::<Vec<_>>();
        let selected = entries
            .iter()
            .position(|entry| entry.tab_id == active_tab)
            .unwrap_or_default();

        Self {
            tabs: entries,
            selected,
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.tabs.is_empty() {
            self.selected = 0;
            return;
        }

        let max = self.tabs.len() - 1;
        self.selected = self.selected.saturating_add_signed(delta).min(max);
    }

    fn selected_tab(&self) -> Option<rssh_core::TabId> {
        self.tabs.get(self.selected).map(|entry| entry.tab_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowTabNavigatorEntry {
    tab_id: rssh_core::TabId,
    title: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum WindowPaneSelectMode {
    #[default]
    Activate,
    SwapWithActive,
    SwapWithActiveKeepFocus,
    MoveToNewTab,
    MoveToNewWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowPaneSelectLabel {
    pane_id: rssh_core::PaneId,
    label: String,
}

fn pane_select_labels(
    panes: &[rssh_core::app_shell::Pane],
    alphabet: &str,
) -> Vec<WindowPaneSelectLabel> {
    let alphabet = alphabet.chars().collect::<Vec<_>>();
    panes
        .iter()
        .enumerate()
        .filter_map(|(index, pane)| {
            pane_select_label_for_index(index, &alphabet).map(|label| WindowPaneSelectLabel {
                pane_id: pane.id(),
                label,
            })
        })
        .collect()
}

fn pane_select_label_for_index(index: usize, alphabet: &[char]) -> Option<String> {
    if alphabet.is_empty() {
        return None;
    }

    if let Some(ch) = alphabet.get(index) {
        return Some(ch.to_string());
    }

    let two_char_index = index.saturating_sub(alphabet.len());
    let first = two_char_index / alphabet.len();
    let second = two_char_index % alphabet.len();
    Some(format!("{}{}", alphabet.get(first)?, alphabet.get(second)?))
}

#[cfg(test)]
fn trim_trailing_spaces(text: &mut String) {
    while text.ends_with(' ') {
        text.pop();
    }
}

#[cfg(test)]
fn snapshot_character(snapshot: &TerminalRenderSnapshot, row: u16, column: u16) -> char {
    snapshot
        .iter_cells()
        .find(|cell| cell.row == row && cell.column == column)
        .map_or(' ', |cell| cell.ch)
}

fn is_word_selection_character(character: char, word_boundary: &str) -> bool {
    !word_boundary.contains(character)
}

fn command_palette_structured_query_command(query: &str) -> Option<WindowCommand> {
    if let Some(command) = wezterm_action_table_wrapper_command(query) {
        return Some(command);
    }
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };
    command_palette_structured_query_command_inner(query)
}

fn strip_wezterm_action_prefix(query: &str) -> Option<&str> {
    let query = query.trim_start();
    ["wezterm.action", "act"].into_iter().find_map(|prefix| {
        let candidate = query.get(..prefix.len())?;
        if !candidate.eq_ignore_ascii_case(prefix) {
            return None;
        }
        let rest = lua_trim_start_comments(query.get(prefix.len()..)?)?;
        let rest = rest.strip_prefix('.')?;
        lua_trim_start_comments(rest)
    })
}

fn strip_wezterm_action_index_prefix(query: &str) -> Option<String> {
    ["wezterm.action", "act"].into_iter().find_map(|prefix| {
        let candidate = query.get(..prefix.len())?;
        if !candidate.eq_ignore_ascii_case(prefix) {
            return None;
        }
        let rest = lua_trim_start_comments(query.get(prefix.len()..)?)?;
        let index = rest.strip_prefix('[')?;
        let indexed = lua_trim_start_comments(index)?;
        let (name, tail) = if let Some(literal) = lua_quoted_string_literal_from_query(indexed)
            .or_else(|| lua_long_bracket_literal_from_query(indexed))
        {
            let close = lua_trim_start_comments(indexed.get(literal.len()..)?)?;
            let tail = lua_trim_start_comments(close.strip_prefix(']')?)?;
            (parse_maybe_quoted_query_text(literal)?, tail)
        } else {
            let end = index.find(']')?;
            let tail = lua_trim_start_comments(index.get(end + 1..)?)?;
            (parse_maybe_quoted_query_text(index[..end].trim())?, tail)
        };
        if name.is_empty() {
            return None;
        }
        if tail.starts_with('"')
            || tail.starts_with('\'')
            || tail.starts_with('[')
            || tail.starts_with('{')
        {
            Some(format!("{name} {tail}"))
        } else {
            Some(format!("{name}{tail}"))
        }
    })
}

fn wezterm_action_table_wrapper_command(query: &str) -> Option<WindowCommand> {
    wezterm_action_table_wrapper_command_with_static_source(None, query)
}

fn wezterm_action_table_wrapper_command_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowCommand> {
    let resolved_table;
    let table = if let Some(table) = strip_wezterm_action_table_wrapper_from_query(query) {
        table
    } else {
        let static_source = static_source?;
        let argument = strip_wezterm_action_table_wrapper_argument_from_query(query)?;
        resolved_table = lua_table_map_value_table_string_from_query(
            static_source.source,
            argument,
            static_source.max_start,
        )?;
        resolved_table
            .trim()
            .strip_prefix('{')?
            .strip_suffix('}')?
            .trim()
    };
    let mut fields = split_lua_table_top_level_fields(table)?
        .into_iter()
        .map(str::trim)
        .filter(|field| !field.is_empty());
    let field = fields.next()?;
    if fields.next().is_some() {
        return None;
    }

    let (name, value) = split_lua_table_assignment_from_field(field)?;
    let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if is_empty_lua_table_query(value) {
        return command_palette_structured_query_command_inner(&name);
    }

    if let Some(static_source) = static_source {
        let static_source = LuaStaticSource {
            source: static_source.source,
            max_start: static_source.source.len(),
        };
        let name = name.to_ascii_lowercase();
        if name == "attachdomain" {
            return attach_domain_lua_table_from_query_with_static_source(
                Some(static_source),
                value,
            )
            .map(WindowCommand::AttachDomain)
            .or_else(|| {
                named_domain_from_query_with_static_source(Some(static_source), value)
                    .map(WindowCommand::AttachDomain)
            });
        }
        if name == "detachdomain" {
            return window_domain_selector_lua_table_from_query_with_static_source(
                Some(static_source),
                value,
            )
            .map(WindowCommand::DetachDomain)
            .or_else(|| {
                window_domain_selector_from_query_with_static_source(Some(static_source), value)
                    .map(WindowCommand::DetachDomain)
            });
        }
    }

    command_palette_structured_query_command_inner(&format!("{name}={value}"))
}

fn is_empty_lua_table_query(value: &str) -> bool {
    value
        .trim()
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .and_then(lua_trim_start_comments)
        .and_then(lua_trim_end_comments)
        .is_some_and(str::is_empty)
}

fn strip_wezterm_action_table_wrapper_from_query(query: &str) -> Option<&str> {
    let query = query.trim();
    ["wezterm.action", "act"].into_iter().find_map(|prefix| {
        let candidate = query.get(..prefix.len())?;
        if !candidate.eq_ignore_ascii_case(prefix) {
            return None;
        }

        let rest = lua_trim_end_comments(lua_trim_start_comments(query.get(prefix.len()..)?)?)?;
        if let Some(table) = rest
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
        {
            return Some(table.trim());
        }
        let table = rest.strip_prefix('(')?.trim().strip_suffix(')')?.trim();
        lua_trim_end_comments(lua_trim_start_comments(table)?)?
            .strip_prefix('{')?
            .strip_suffix('}')
            .map(str::trim)
    })
}

fn strip_wezterm_action_table_wrapper_argument_from_query(query: &str) -> Option<&str> {
    let query = query.trim();
    ["wezterm.action", "act"].into_iter().find_map(|prefix| {
        let candidate = query.get(..prefix.len())?;
        if !candidate.eq_ignore_ascii_case(prefix) {
            return None;
        }

        let rest = lua_trim_end_comments(lua_trim_start_comments(query.get(prefix.len()..)?)?)?;
        let argument = rest.strip_prefix('(')?.trim().strip_suffix(')')?.trim();
        lua_trim_end_comments(lua_trim_start_comments(argument)?)
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn command_palette_structured_query_command_inner(query: &str) -> Option<WindowCommand> {
    if let Some(command) = command_palette_basic_structured_query_command(query) {
        return Some(command);
    }
    if let Some(command) = activate_window_command_from_query(query) {
        return Some(command);
    }
    if let Some(assignment) = copy_mode_assignment_from_query(query) {
        return Some(WindowCommand::CopyMode(assignment));
    }
    if let Some(offset) = activate_tab_relative_no_wrap_from_query(query) {
        return Some(WindowCommand::ActivateTabRelativeNoWrap(offset));
    }
    if let Some(offset) = activate_tab_relative_from_query(query) {
        return Some(WindowCommand::ActivateTabRelative(offset));
    }
    if let Some(index) = activate_tab_from_query(query) {
        return Some(WindowCommand::ActivateTab(index));
    }
    if let Some(offset) = move_tab_relative_from_query(query) {
        return Some(WindowCommand::MoveTabRelative(offset));
    }
    if let Some(index) = move_tab_from_query(query) {
        return Some(WindowCommand::MoveTab(index));
    }
    if let Some(window_id) = move_tab_to_window_from_query(query) {
        return Some(WindowCommand::MoveTabToWindow(window_id));
    }
    if let Some(direction) = activate_pane_direction_from_query(query) {
        return Some(WindowCommand::ActivatePaneDirection(direction));
    }
    if let Some(index) = activate_pane_by_index_from_query(query) {
        return Some(WindowCommand::ActivatePaneByIndex(index));
    }
    if let Some((direction, amount)) = adjust_pane_size_from_query(query) {
        return Some(WindowCommand::AdjustPaneSize { direction, amount });
    }
    if let Some(amount) = scroll_by_page_from_query(query) {
        return Some(WindowCommand::ScrollByPage(amount));
    }
    if let Some(amount) = scroll_by_line_from_query(query) {
        return Some(WindowCommand::ScrollByLine(amount));
    }
    if let Some(amount) = scroll_to_prompt_from_query(query) {
        return Some(WindowCommand::ScrollToPrompt(amount));
    }
    if let Some(zoomed) = set_pane_zoom_state_from_query(query) {
        return Some(WindowCommand::SetPaneZoomState(zoomed));
    }
    if let Some(direction) = rotate_panes_from_query(query) {
        return Some(WindowCommand::RotatePanes(direction));
    }
    if let Some(level) = set_window_level_from_query(query) {
        return Some(WindowCommand::SetWindowLevel(level));
    }
    if let Some(command) = complete_selection_command_from_query(query) {
        return Some(command);
    }
    if let Some(command) = mouse_selection_command_from_query(query) {
        return Some(command);
    }
    if let Some(uri) = open_uri_from_query(query) {
        return Some(WindowCommand::OpenUri(uri));
    }
    if let Some((text, destination)) = copy_text_to_from_query(query) {
        return Some(WindowCommand::CopyTextTo { text, destination });
    }
    if let Some(split_pane) = split_pane_table_action_from_query(query) {
        return Some(WindowCommand::SplitPane(split_pane));
    }
    if split_horizontal_options_from_query(query).is_some() {
        return Some(WindowCommand::SplitRight);
    }
    if split_vertical_options_from_query(query).is_some() {
        return Some(WindowCommand::SplitDown);
    }
    if let Some(split_pane) = split_pane_action_name_options_from_query(query) {
        return Some(WindowCommand::SplitPane(split_pane));
    }
    if let Some(split_pane) =
        split_left_options_from_query(query).or_else(|| split_up_options_from_query(query))
    {
        return Some(WindowCommand::SplitPane(split_pane));
    }
    if let Some(options) = pane_select_options_from_query(query) {
        return Some(WindowCommand::PaneSelect(options));
    }
    if let Some(args) = show_launcher_args_from_query(query) {
        return Some(WindowCommand::ShowLauncherArgs(args));
    }
    if let Some(options) = char_select_options_from_query(query) {
        return Some(WindowCommand::CharSelectArgs(options));
    }
    if let Some(query) = pane_select_mode_show_pane_ids_from_query(query) {
        return Some(query.command);
    }
    if let Some(query) = pane_select_mode_alphabet_from_query(query) {
        return Some(query.command);
    }
    if pane_select_show_pane_ids_alphabet_from_query(query).is_some()
        || pane_select_activate_show_pane_ids_alphabet_from_query(query).is_some()
    {
        return Some(WindowCommand::EnterPaneSelectShowPaneIds);
    }
    if pane_select_alphabet_from_query(query).is_some()
        || pane_select_activate_alphabet_from_query(query).is_some()
    {
        return Some(WindowCommand::EnterPaneSelect);
    }
    if search_query_from_query(query).is_some() {
        return Some(WindowCommand::EnterSearch);
    }
    if let Some(options) = quick_select_lua_table_from_query(query) {
        return Some(WindowCommand::QuickSelectArgs(options));
    }
    if quick_select_patterns_from_query(query).is_some()
        || quick_select_pattern_from_query(query).is_some()
        || quick_select_alphabet_from_query(query).is_some()
        || quick_select_label_from_query(query).is_some()
        || quick_select_action_from_query(query).is_some()
        || quick_select_scope_lines_from_query(query).is_some()
    {
        return Some(WindowCommand::EnterQuickSelect);
    }
    command_palette_normalized_no_arg_query_command(query)
}

fn command_palette_basic_structured_query_command(query: &str) -> Option<WindowCommand> {
    let query = query.trim();
    let action_name = query.to_ascii_lowercase();
    if let Some(command) = basic_no_arg_action_name_command(&action_name) {
        return Some(command);
    }
    if let Some(action_name) = strip_zero_arg_lua_function_call_from_query(query)
        && let Some(command) =
            basic_no_arg_action_name_command(&normalized_action_name_query(action_name))
    {
        return Some(command);
    }

    if rename_tab_title_from_query(query).is_some() {
        return Some(WindowCommand::RenameTab);
    }
    if rename_workspace_name_from_query(query).is_some() {
        return Some(WindowCommand::RenameWorkspace);
    }
    if let Some(offset) = switch_workspace_relative_from_query(query) {
        return Some(WindowCommand::SwitchWorkspaceRelative(offset));
    }
    if switch_workspace_options_from_query(query).is_some() {
        return Some(WindowCommand::SwitchToWorkspace);
    }
    if switch_workspace_name_from_query(query).is_some() {
        return Some(WindowCommand::SwitchToWorkspace);
    }
    if let Some(domain) = spawn_tab_domain_from_query(query) {
        return Some(WindowCommand::SpawnTab(domain));
    }
    if let Some(domain) = attach_domain_from_query(query) {
        return Some(WindowCommand::AttachDomain(domain));
    }
    if let Some(domain) = detach_domain_from_query(query) {
        return Some(WindowCommand::DetachDomain(domain));
    }
    if spawn_command_in_new_tab_from_query(query).is_some() {
        return Some(WindowCommand::NewTab);
    }
    if spawn_command_options_in_new_tab_from_query(query).is_some() {
        return Some(WindowCommand::NewTab);
    }
    if spawn_command_in_new_window_from_query(query).is_some() {
        return Some(WindowCommand::SpawnWindow);
    }
    if spawn_command_options_in_new_window_from_query(query).is_some() {
        return Some(WindowCommand::SpawnWindow);
    }
    if let Some(mode) = clear_scrollback_mode_from_query(query) {
        return Some(WindowCommand::ClearScrollback(mode));
    }
    if let Some(destination) = copy_destination_command_from_query(query) {
        return Some(WindowCommand::CopyTo(destination));
    }
    if let Some(source) = paste_source_command_from_query(query) {
        return Some(WindowCommand::PasteFrom(source));
    }
    if let Some(command) = close_current_command_from_query(query) {
        return Some(command);
    }
    if let Some(options) = prompt_input_line_options_from_query(query) {
        return Some(WindowCommand::PromptInputLine(options));
    }
    if let Some(options) = input_selector_options_from_query(query) {
        return Some(WindowCommand::InputSelector(options));
    }
    if let Some(options) = confirmation_options_from_query(query) {
        return Some(WindowCommand::Confirmation(options));
    }
    if let Some(commands) = multiple_commands_from_query(query) {
        return Some(WindowCommand::Multiple(commands));
    }
    if let Some(event) = emit_event_from_query(query) {
        return Some(WindowCommand::EmitEvent(event));
    }
    if let Some(value) = send_string_from_query(query) {
        return Some(WindowCommand::SendString(value));
    }
    if let Some(send_key) = send_key_from_query(query) {
        return Some(WindowCommand::SendKey(send_key));
    }
    if let Some(key_table) = activate_key_table_from_query(query) {
        return Some(WindowCommand::ActivateKeyTable(key_table));
    }
    if let Some(command) = key_table_stack_command_from_query(query) {
        return Some(command);
    }

    None
}

fn strip_zero_arg_lua_function_call_from_query(query: &str) -> Option<&str> {
    let query = lua_trim_end_comments(query.trim())?;
    let without_close = query.strip_suffix(')')?;
    let (name, args) = without_close.split_once('(')?;
    let name = name.trim();
    (!name.is_empty()
        && (args.trim().is_empty()
            || lua_trim_start_comments(args)
                .and_then(lua_trim_end_comments)
                .is_some_and(str::is_empty)
            || is_empty_lua_table_query(args.trim())))
    .then_some(name)
}

fn command_palette_normalized_no_arg_query_command(query: &str) -> Option<WindowCommand> {
    let query = query.trim();
    let action_name = query.to_ascii_lowercase();
    let normalized_action_name = normalized_action_name_query(query);
    (normalized_action_name != action_name)
        .then(|| basic_no_arg_action_name_command(&normalized_action_name))
        .flatten()
}

fn normalized_action_name_query(query: &str) -> String {
    query
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn basic_no_arg_action_name_command(action_name: &str) -> Option<WindowCommand> {
    match action_name {
        "nop" => return Some(WindowCommand::Nop),
        "disabledefaultassignment" => return Some(WindowCommand::DisableDefaultAssignment),
        "hideapplication" => return Some(WindowCommand::HideApplication),
        "quitapplication" => return Some(WindowCommand::QuitApplication),
        "decreasefontsize" => return Some(WindowCommand::DecreaseFontSize),
        "increasefontsize" => return Some(WindowCommand::IncreaseFontSize),
        "resetfontsize" => return Some(WindowCommand::ResetFontSize),
        "resetfontandwindowsize" => return Some(WindowCommand::ResetFontAndWindowSize),
        "showdebugoverlay" => return Some(WindowCommand::ShowDebugOverlay),
        "activatecopymode" => return Some(WindowCommand::ActivateCopyMode),
        "entercopymode" => return Some(WindowCommand::EnterCopyMode),
        "spawntab" => {
            return Some(WindowCommand::SpawnTab(
                WindowSpawnTabDomain::CurrentPaneDomain,
            ));
        }
        "spawnwindow" => return Some(WindowCommand::SpawnWindow),
        "switchtoworkspace" => return Some(WindowCommand::SwitchToWorkspace),
        "splithorizontal" => return Some(WindowCommand::SplitHorizontal),
        "splitvertical" => return Some(WindowCommand::SplitVertical),
        "clearselection" => return Some(WindowCommand::ClearSelection),
        "completeselection" => return Some(WindowCommand::CompleteSelection),
        "openlinkatmousecursor" => return Some(WindowCommand::OpenLinkAtMouseCursor),
        "completeselectionoropenlinkatmousecursor" => {
            return Some(WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor);
        }
        "copytoclipboard" => return Some(WindowCommand::CopyToClipboard),
        "copytoprimaryselection" => return Some(WindowCommand::CopyToPrimarySelection),
        "copytoclipboardandprimaryselection" => {
            return Some(WindowCommand::CopyToClipboardAndPrimarySelection);
        }
        "copy" => return Some(WindowCommand::Copy),
        "pastefromclipboard" => return Some(WindowCommand::PasteFromClipboard),
        "pastefromprimaryselection" => return Some(WindowCommand::PasteFromPrimarySelection),
        "paste" => return Some(WindowCommand::Paste),
        "pasteprimaryselection" => return Some(WindowCommand::PastePrimarySelection),
        "clearscrollbackandviewport" => return Some(WindowCommand::ClearScrollbackAndViewport),
        "scrolltotop" => return Some(WindowCommand::ScrollToTop),
        "scrolltobottom" => return Some(WindowCommand::ScrollToBottom),
        "scrollpageup" => return Some(WindowCommand::ScrollPageUp),
        "scrollpagedown" => return Some(WindowCommand::ScrollPageDown),
        "scrolllineup" => return Some(WindowCommand::ScrollLineUp),
        "scrolllinedown" => return Some(WindowCommand::ScrollLineDown),
        "scrollbycurrenteventwheeldelta" => {
            return Some(WindowCommand::ScrollByCurrentEventWheelDelta);
        }
        "scrolltopreviousprompt" => return Some(WindowCommand::ScrollToPreviousPrompt),
        "scrolltonextprompt" => return Some(WindowCommand::ScrollToNextPrompt),
        "activatepaneleft" => return Some(WindowCommand::ActivatePaneLeft),
        "activatepaneright" => return Some(WindowCommand::ActivatePaneRight),
        "activatepaneup" => return Some(WindowCommand::ActivatePaneUp),
        "activatepanedown" => return Some(WindowCommand::ActivatePaneDown),
        "nextpane" => return Some(WindowCommand::NextPane),
        "previouspane" => return Some(WindowCommand::PreviousPane),
        "activatepane1" => return Some(WindowCommand::ActivatePane1),
        "activatepane2" => return Some(WindowCommand::ActivatePane2),
        "activatepane3" => return Some(WindowCommand::ActivatePane3),
        "activatepane4" => return Some(WindowCommand::ActivatePane4),
        "activatepane5" => return Some(WindowCommand::ActivatePane5),
        "activatepane6" => return Some(WindowCommand::ActivatePane6),
        "activatepane7" => return Some(WindowCommand::ActivatePane7),
        "activatepane8" => return Some(WindowCommand::ActivatePane8),
        "activatelasttab" => return Some(WindowCommand::ActivateLastTab),
        "activatetab1" => return Some(WindowCommand::ActivateTab1),
        "activatetab2" => return Some(WindowCommand::ActivateTab2),
        "activatetab3" => return Some(WindowCommand::ActivateTab3),
        "activatetab4" => return Some(WindowCommand::ActivateTab4),
        "activatetab5" => return Some(WindowCommand::ActivateTab5),
        "activatetab6" => return Some(WindowCommand::ActivateTab6),
        "activatetab7" => return Some(WindowCommand::ActivateTab7),
        "activatetab8" => return Some(WindowCommand::ActivateTab8),
        "activatetab9" => return Some(WindowCommand::ActivateTab9),
        "nexttab" => return Some(WindowCommand::NextTab),
        "previoustab" => return Some(WindowCommand::PreviousTab),
        "nexttabnowrap" => return Some(WindowCommand::NextTabNoWrap),
        "previoustabnowrap" => return Some(WindowCommand::PreviousTabNoWrap),
        "movetabrelativeleft" => return Some(WindowCommand::MoveTabRelativeLeft),
        "movetabrelativeright" => return Some(WindowCommand::MoveTabRelativeRight),
        "movetabto1" => return Some(WindowCommand::MoveTabTo1),
        "movetabto2" => return Some(WindowCommand::MoveTabTo2),
        "movetabto3" => return Some(WindowCommand::MoveTabTo3),
        "movetabto4" => return Some(WindowCommand::MoveTabTo4),
        "movetabto5" => return Some(WindowCommand::MoveTabTo5),
        "movetabto6" => return Some(WindowCommand::MoveTabTo6),
        "movetabto7" => return Some(WindowCommand::MoveTabTo7),
        "movetabto8" => return Some(WindowCommand::MoveTabTo8),
        "togglepanezoomstate" => return Some(WindowCommand::TogglePaneZoomState),
        "togglepanezoom" => return Some(WindowCommand::TogglePaneZoom),
        "zoompane" => return Some(WindowCommand::ZoomPane),
        "unzoompane" => return Some(WindowCommand::UnzoomPane),
        "restartpane" => return Some(WindowCommand::RestartPane),
        "inspectpane" => return Some(WindowCommand::InspectPane),
        "reloadconfiguration" => return Some(WindowCommand::ReloadConfiguration),
        "activatecommandpalette" => return Some(WindowCommand::ActivateCommandPalette),
        "togglefullscreen" => return Some(WindowCommand::ToggleFullScreen),
        "startwindowdrag" => return Some(WindowCommand::StartWindowDrag),
        "togglealwaysontop" => return Some(WindowCommand::ToggleAlwaysOnTop),
        "togglealwaysonbottom" => return Some(WindowCommand::ToggleAlwaysOnBottom),
        "show" => return Some(WindowCommand::Show),
        "hide" => return Some(WindowCommand::Hide),
        "resetterminal" => return Some(WindowCommand::ResetTerminal),
        "showtabnavigator" => return Some(WindowCommand::ShowTabNavigator),
        "showlauncher" => return Some(WindowCommand::ShowLauncher),
        "charselect" => return Some(WindowCommand::CharSelect),
        "quickselect" | "quickselectargs" | "enterquickselect" => {
            return Some(WindowCommand::EnterQuickSelect);
        }
        "enterpaneselect" => return Some(WindowCommand::EnterPaneSelect),
        "enterpaneselectshowpaneids" => return Some(WindowCommand::EnterPaneSelectShowPaneIds),
        "enterpaneswap" => return Some(WindowCommand::EnterPaneSwap),
        "enterpaneswapkeepfocus" => return Some(WindowCommand::EnterPaneSwapKeepFocus),
        "enterpanemovetonewtab" => return Some(WindowCommand::EnterPaneMoveToNewTab),
        "enterpanemovetonewwindow" => return Some(WindowCommand::EnterPaneMoveToNewWindow),
        "closepane" => return Some(WindowCommand::ClosePane),
        "closetab" => return Some(WindowCommand::CloseTab),
        "duplicatetab" => return Some(WindowCommand::DuplicateTab),
        "reopenclosedtab" => return Some(WindowCommand::ReopenClosedTab),
        "closeothertabs" => return Some(WindowCommand::CloseOtherTabs),
        "closetabstoright" => return Some(WindowCommand::CloseTabsToRight),
        "movetabtonewwindow" => return Some(WindowCommand::MoveTabToNewWindow),
        _ => {}
    }
    None
}

fn clear_scrollback_mode_from_query(query: &str) -> Option<WindowClearScrollbackMode> {
    clear_scrollback_mode_from_query_with_static_source(None, query)
}

fn clear_scrollback_mode_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowClearScrollbackMode> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(rest) = strip_lua_function_call_from_query(query, "clearscrollback") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            return clear_scrollback_lua_table_from_query_with_static_source(static_source, rest);
        }
        if static_source.is_some()
            && let Some(mode) =
                clear_scrollback_lua_table_from_query_with_static_source(static_source, rest)
        {
            return Some(mode);
        }
        let mode = parse_maybe_static_query_text(static_source, rest)?;
        return clear_scrollback_mode_from_query(&format!("clearscrollback {mode}"));
    }

    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "clearscrollback=")
        && rest.trim_start().starts_with('{')
    {
        return clear_scrollback_lua_table_from_query_with_static_source(static_source, rest);
    }

    let mode = strip_query_prefix_from_any(
        query,
        &[
            "clear scrollback=",
            "clear scrollback ",
            "clearscrollback=",
            "clearscrollback ",
        ],
    )?;
    let mode = strip_query_prefix_from_any(mode, &["mode=", "mode "]).unwrap_or(mode);
    let mode = parse_maybe_static_query_text(static_source, mode)?;
    let normalized = mode
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "only" | "scrollbackonly" => Some(WindowClearScrollbackMode::ScrollbackOnly),
        "viewport" | "andviewport" | "scrollbackviewport" | "scrollbackandviewport" => {
            Some(WindowClearScrollbackMode::ScrollbackAndViewport)
        }
        _ => None,
    }
}

fn clear_scrollback_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowClearScrollbackMode> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut mode = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = parse_maybe_static_query_text(static_source, value)?;
        match name.to_ascii_lowercase().as_str() {
            "mode" => {
                if mode.is_some() || value.is_empty() {
                    return None;
                }
                mode = Some(value);
            }
            _ => return None,
        }
    }

    clear_scrollback_mode_from_query(&format!("clearscrollback {}", mode?))
}

fn copy_destination_command_from_query(query: &str) -> Option<WindowCopyDestination> {
    copy_destination_command_from_query_with_static_source(None, query)
}

fn copy_destination_command_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowCopyDestination> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(destination) = strip_lua_function_call_from_query(query, "copyto") {
        return copy_destination_from_query_with_static_source(static_source, destination);
    }

    let destination =
        strip_query_prefix_from_any(query, &["copy to=", "copy to ", "copyto=", "copyto "])?;
    let destination = strip_query_prefix_from_any(destination, &["destination=", "destination "])
        .unwrap_or(destination);
    copy_destination_from_query_with_static_source(static_source, destination)
}

fn paste_source_command_from_query(query: &str) -> Option<WindowPasteSource> {
    paste_source_command_from_query_with_static_source(None, query)
}

fn paste_source_command_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowPasteSource> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(source) = strip_lua_function_call_from_query(query, "pastefrom") {
        return paste_source_from_query_with_static_source(static_source, source);
    }

    let source = strip_query_prefix_from_any(
        query,
        &["paste from=", "paste from ", "pastefrom=", "pastefrom "],
    )?;
    let source = strip_query_prefix_from_any(source, &["source=", "source "]).unwrap_or(source);
    paste_source_from_query_with_static_source(static_source, source)
}

fn copy_mode_assignment_from_query(query: &str) -> Option<WindowCopyModeAssignment> {
    copy_mode_assignment_from_query_with_static_source(None, query)
}

fn copy_mode_assignment_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowCopyModeAssignment> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(value) = strip_lua_function_call_from_query(query, "copymode") {
        let value = value.trim();
        if value.starts_with('{') {
            return copy_mode_assignment_lua_table_from_query_with_static_source(
                static_source,
                value,
            );
        }
        if static_source.is_some()
            && let Some(assignment) =
                copy_mode_assignment_lua_table_from_query_with_static_source(static_source, value)
        {
            return Some(assignment);
        }
        return copy_mode_assignment_name_from_query_with_static_source(static_source, value);
    }

    let value = strip_query_prefix_from_any(
        query,
        &["copy mode=", "copy mode ", "copymode=", "copymode "],
    )?;
    if value.trim_start().starts_with('{') {
        return copy_mode_assignment_lua_table_from_query_with_static_source(static_source, value);
    }
    let value = strip_query_prefix_from_any(value, &["assignment=", "assignment "])
        .or_else(|| strip_query_prefix_from_any(value, &["action=", "action "]))
        .unwrap_or(value);
    copy_mode_assignment_name_from_query_with_static_source(static_source, value)
}

fn copy_mode_assignment_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowCopyModeAssignment> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut fields = split_lua_table_top_level_fields(table)?
        .into_iter()
        .map(str::trim)
        .filter(|field| !field.is_empty());
    let field = fields.next()?;
    if fields.next().is_some() {
        return None;
    }

    if let Some((name, value)) = split_lua_table_assignment_from_field(field) {
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        match normalized_action_name_query(&name).as_str() {
            "movebypage" => parse_maybe_static_query_f64(static_source, value.trim())
                .and_then(scroll_by_page_amount_from_f64)
                .map(WindowCopyModeAssignment::MoveByPage),
            "jumpforward" => copy_mode_jump_assignment_lua_table_from_query_with_static_source(
                static_source,
                value,
                true,
            ),
            "jumpbackward" => copy_mode_jump_assignment_lua_table_from_query_with_static_source(
                static_source,
                value,
                false,
            ),
            "moveforwardsemanticzoneoftype" => {
                copy_mode_semantic_zone_type_from_query_with_static_source(static_source, value)
                    .map(
                        |semantic_type| WindowCopyModeAssignment::MoveSemanticZoneOfType {
                            delta: 1,
                            semantic_type,
                        },
                    )
            }
            "moveforwardzoneoftype" => {
                copy_mode_semantic_zone_type_from_query_with_static_source(static_source, value)
                    .map(
                        |semantic_type| WindowCopyModeAssignment::MoveSemanticZoneOfType {
                            delta: 1,
                            semantic_type,
                        },
                    )
            }
            "movebackwardsemanticzoneoftype" => {
                copy_mode_semantic_zone_type_from_query_with_static_source(static_source, value)
                    .map(
                        |semantic_type| WindowCopyModeAssignment::MoveSemanticZoneOfType {
                            delta: -1,
                            semantic_type,
                        },
                    )
            }
            "movebackwardzoneoftype" => {
                copy_mode_semantic_zone_type_from_query_with_static_source(static_source, value)
                    .map(
                        |semantic_type| WindowCopyModeAssignment::MoveSemanticZoneOfType {
                            delta: -1,
                            semantic_type,
                        },
                    )
            }
            "setselectionmode" => {
                copy_mode_selection_mode_from_query_with_static_source(static_source, value)
                    .map(WindowCopyModeAssignment::SetSelectionMode)
            }
            _ => None,
        }
    } else {
        copy_mode_assignment_name_from_query_with_static_source(static_source, field)
    }
}

fn copy_mode_jump_assignment_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    forward: bool,
) -> Option<WindowCopyModeAssignment> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut prev_char = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        match normalized_action_name_query(&name).as_str() {
            "prevchar" => {
                if prev_char.is_some() {
                    return None;
                }
                prev_char = Some(parse_maybe_static_query_bool(static_source, value.trim())?);
            }
            _ => return None,
        }
    }

    Some(WindowCopyModeAssignment::StartJump {
        forward,
        prev_char: prev_char.unwrap_or(false),
    })
}

fn copy_mode_semantic_zone_type_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<SemanticType> {
    let value = parse_maybe_static_query_text(static_source, value)?;
    match normalized_action_name_query(&value).as_str() {
        "input" => Some(SemanticType::Input),
        "output" => Some(SemanticType::Output),
        "prompt" => Some(SemanticType::Prompt),
        _ => None,
    }
}

fn copy_mode_assignment_name_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowCopyModeAssignment> {
    let value = parse_maybe_static_query_text(static_source, value)?;
    match normalized_action_name_query(&value).as_str() {
        "acceptpattern" => Some(WindowCopyModeAssignment::AcceptPattern),
        "close" => Some(WindowCopyModeAssignment::Close),
        "clearpattern" => Some(WindowCopyModeAssignment::ClearPattern),
        "clearselectionmode" => Some(WindowCopyModeAssignment::ClearSelectionMode),
        "cyclematchtype" => Some(WindowCopyModeAssignment::CycleMatchType),
        "editpattern" => Some(WindowCopyModeAssignment::EditPattern),
        "jumpagain" => Some(WindowCopyModeAssignment::JumpAgain),
        "jumpreverse" => Some(WindowCopyModeAssignment::JumpReverse),
        "movebackwardsemanticzone" => Some(WindowCopyModeAssignment::MoveBackwardSemanticZone),
        "movebackwardword" => Some(WindowCopyModeAssignment::MoveBackwardWord),
        "movedown" => Some(WindowCopyModeAssignment::MoveDown),
        "moveforwardsemanticzone" => Some(WindowCopyModeAssignment::MoveForwardSemanticZone),
        "moveforwardword" => Some(WindowCopyModeAssignment::MoveForwardWord),
        "moveforwardwordend" => Some(WindowCopyModeAssignment::MoveForwardWordEnd),
        "moveleft" => Some(WindowCopyModeAssignment::MoveLeft),
        "moveright" => Some(WindowCopyModeAssignment::MoveRight),
        "movetoendoflinecontent" => Some(WindowCopyModeAssignment::MoveToEndOfLineContent),
        "movetoscrollbackbottom" | "scrolltobottom" => {
            Some(WindowCopyModeAssignment::MoveToScrollbackBottom)
        }
        "movetoscrollbacktop" | "scrolltotop" => {
            Some(WindowCopyModeAssignment::MoveToScrollbackTop)
        }
        "movetoselectionotherend" => Some(WindowCopyModeAssignment::MoveToSelectionOtherEnd),
        "movetoselectionotherendhoriz" => {
            Some(WindowCopyModeAssignment::MoveToSelectionOtherEndHoriz)
        }
        "movetostartofline" => Some(WindowCopyModeAssignment::MoveToStartOfLine),
        "movetostartoflinecontent" => Some(WindowCopyModeAssignment::MoveToStartOfLineContent),
        "movetostartofnextline" => Some(WindowCopyModeAssignment::MoveToStartOfNextLine),
        "movetoviewportbottom" => Some(WindowCopyModeAssignment::MoveToViewportBottom),
        "movetoviewportmiddle" => Some(WindowCopyModeAssignment::MoveToViewportMiddle),
        "movetoviewporttop" => Some(WindowCopyModeAssignment::MoveToViewportTop),
        "moveup" => Some(WindowCopyModeAssignment::MoveUp),
        "pagedown" => Some(WindowCopyModeAssignment::PageDown),
        "pageup" => Some(WindowCopyModeAssignment::PageUp),
        "nextmatch" => Some(WindowCopyModeAssignment::NextMatch),
        "nextmatchpage" => Some(WindowCopyModeAssignment::NextMatchPage),
        "priormatch" => Some(WindowCopyModeAssignment::PriorMatch),
        "priormatchpage" => Some(WindowCopyModeAssignment::PriorMatchPage),
        _ => None,
    }
}

fn copy_mode_selection_mode_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowCopySelectionMode> {
    let value = parse_maybe_static_query_text(static_source, value)?;
    match normalized_action_name_query(&value).as_str() {
        "cell" => Some(WindowCopySelectionMode::Cell),
        "word" => Some(WindowCopySelectionMode::Word),
        "line" => Some(WindowCopySelectionMode::Line),
        "block" => Some(WindowCopySelectionMode::Block),
        "semanticzone" => Some(WindowCopySelectionMode::SemanticZone),
        _ => None,
    }
}

fn close_current_command_from_query(query: &str) -> Option<WindowCommand> {
    close_current_command_from_query_with_static_source(None, query)
}

fn close_current_command_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowCommand> {
    close_current_pane_confirm_from_query_with_static_source(static_source, query)
        .map(|confirm| WindowCommand::CloseCurrentPane { confirm })
        .or_else(|| {
            close_current_tab_confirm_from_query_with_static_source(static_source, query)
                .map(|confirm| WindowCommand::CloseCurrentTab { confirm })
        })
}

fn close_current_pane_confirm_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<bool> {
    let query = strip_wezterm_action_prefix(query).unwrap_or(query);
    if let Some(rest) = strip_lua_function_call_from_query(query, "closecurrentpane") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            return close_current_confirm_lua_table_from_query_with_static_source(
                static_source,
                rest,
            );
        }
        if static_source.is_some()
            && let Some(confirm) =
                close_current_confirm_lua_table_from_query_with_static_source(static_source, rest)
        {
            return Some(confirm);
        }
    }

    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "closecurrentpane=")
        && rest.trim_start().starts_with('{')
    {
        return close_current_confirm_lua_table_from_query_with_static_source(static_source, rest);
    }

    if static_source.is_some() {
        return close_current_pane_confirm_from_query_with_static_source(None, query);
    }

    bool_query_value_from_prefixes(
        query,
        &[
            "close current pane confirm ",
            "closecurrentpane confirm ",
            "closecurrentpaneconfirm ",
        ],
    )
}

fn close_current_tab_confirm_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<bool> {
    let query = strip_wezterm_action_prefix(query).unwrap_or(query);
    if let Some(rest) = strip_lua_function_call_from_query(query, "closecurrenttab") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            return close_current_confirm_lua_table_from_query_with_static_source(
                static_source,
                rest,
            );
        }
        if static_source.is_some()
            && let Some(confirm) =
                close_current_confirm_lua_table_from_query_with_static_source(static_source, rest)
        {
            return Some(confirm);
        }
    }

    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "closecurrenttab=")
        && rest.trim_start().starts_with('{')
    {
        return close_current_confirm_lua_table_from_query_with_static_source(static_source, rest);
    }

    if static_source.is_some() {
        return close_current_tab_confirm_from_query_with_static_source(None, query);
    }

    bool_query_value_from_prefixes(
        query,
        &[
            "close current tab confirm ",
            "closecurrenttab confirm ",
            "closecurrenttabconfirm ",
        ],
    )
}

fn close_current_confirm_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<bool> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut confirm = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        match name.to_ascii_lowercase().as_str() {
            "confirm" => {
                if confirm.is_some() {
                    return None;
                }
                confirm = Some(parse_maybe_static_query_bool(static_source, value.trim())?);
            }
            _ => return None,
        }
    }

    confirm
}

fn bool_query_value_from_prefixes(query: &str, prefixes: &[&str]) -> Option<bool> {
    strip_query_prefix_from_any(query, prefixes)
        .or_else(|| {
            prefixes.iter().find_map(|prefix| {
                let equals_prefix = prefix.trim_end().to_owned() + "=";
                strip_query_prefix_from_any(query, &[equals_prefix.as_str()])
            })
        })
        .and_then(parse_single_query_value)
        .and_then(bool_from_query)
}

fn prompt_input_line_options_from_query(query: &str) -> Option<WindowPromptInputLineOptions> {
    prompt_input_line_options_from_query_with_static_source(None, query)
}

fn prompt_input_line_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowPromptInputLineOptions> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(rest) = strip_lua_function_call_from_query(query, "promptinputline") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            return prompt_input_line_lua_table_from_query_with_static_source(static_source, rest);
        }
        if static_source.is_some()
            && let Some(options) =
                prompt_input_line_lua_table_from_query_with_static_source(static_source, rest)
        {
            return Some(options);
        }
    }

    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "promptinputline=")
        && rest.trim_start().starts_with('{')
    {
        return prompt_input_line_lua_table_from_query_with_static_source(static_source, rest);
    }

    let rest = strip_query_prefix_from_any(
        query,
        &[
            "prompt input line=",
            "prompt input line ",
            "promptinputline=",
            "promptinputline ",
        ],
    )?;
    let (options, _) = prompt_input_line_fields_from_query_with_static_source(
        static_source,
        rest,
        WindowPromptInputLineOptions::default(),
    )?;
    Some(options)
}

fn prompt_input_line_fields_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    rest: &str,
    options: WindowPromptInputLineOptions,
) -> Option<(WindowPromptInputLineOptions, usize)> {
    let rest = rest.trim();
    if rest.is_empty() {
        return (!options.description.is_empty()).then_some((options, 0));
    }

    if let Some(value) = prompt_input_line_strip_field_key(rest, "description") {
        return prompt_input_line_field_splits(value)
            .filter_map(|(description, remaining)| {
                if description.is_empty() || !options.description.is_empty() {
                    return None;
                }
                let description =
                    modal_display_text_from_query_with_static_source(static_source, description)?;
                let mut options = options.clone();
                let description_len = description.len();
                options.description = description;
                let (options, score) = prompt_input_line_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    options,
                )?;
                Some((options, score + 1, description_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(options, score, _)| (options, score));
    }

    if let Some(value) = prompt_input_line_strip_field_key(rest, "prompt") {
        return prompt_input_line_field_splits(value)
            .filter_map(|(prompt, remaining)| {
                if options.prompt.is_some() {
                    return None;
                }
                let prompt =
                    modal_display_text_from_query_with_static_source(static_source, prompt)?;
                let mut options = options.clone();
                let prompt_len = prompt.len();
                options.prompt = Some(prompt);
                let (options, score) = prompt_input_line_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    options,
                )?;
                Some((options, score + 1, prompt_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(options, score, _)| (options, score));
    }

    if let Some(value) = prompt_input_line_strip_field_key(rest, "initial_value")
        .or_else(|| prompt_input_line_strip_field_key(rest, "initial-value"))
        .or_else(|| prompt_input_line_strip_field_key(rest, "initial value"))
    {
        return prompt_input_line_field_splits(value)
            .filter_map(|(initial_value, remaining)| {
                if options.initial_value.is_some() {
                    return None;
                }
                let initial_value = parse_maybe_static_query_text(static_source, initial_value)?;
                let mut options = options.clone();
                let initial_value_len = initial_value.len();
                options.initial_value = Some(initial_value);
                let (options, score) = prompt_input_line_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    options,
                )?;
                Some((options, score + 1, initial_value_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(options, score, _)| (options, score));
    }

    None
}

fn prompt_input_line_field_splits(rest: &str) -> impl Iterator<Item = (&str, &str)> {
    let mut offsets = prompt_input_line_next_field_offsets(rest);
    offsets.reverse();
    offsets.push(rest.len());
    offsets
        .into_iter()
        .map(|offset| {
            let (value, remaining) = rest.split_at(offset);
            (value.trim(), remaining.trim_start())
        })
        .filter(|(value, _)| !value.is_empty())
}

fn prompt_input_line_strip_field_key<'a>(rest: &'a str, key: &str) -> Option<&'a str> {
    let key_prefix = rest.get(..key.len())?;
    let remaining = rest.get(key.len()..)?;
    key_prefix
        .eq_ignore_ascii_case(key)
        .then_some(remaining)
        .and_then(|remaining| {
            remaining.strip_prefix('=').or_else(|| {
                remaining
                    .starts_with(char::is_whitespace)
                    .then_some(remaining)
            })
        })
        .map(str::trim_start)
}

fn prompt_input_line_next_field_offsets(rest: &str) -> Vec<usize> {
    let lowercase_rest = rest.to_ascii_lowercase();
    let mut offsets = [
        " description ",
        " description=",
        " prompt ",
        " prompt=",
        " initial_value ",
        " initial_value=",
        " initial-value ",
        " initial-value=",
        " initial value ",
        " initial value=",
    ]
    .into_iter()
    .flat_map(|needle| {
        lowercase_rest
            .match_indices(needle)
            .map(|(index, _)| index + 1)
    })
    .collect::<Vec<_>>();
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

fn prompt_input_line_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowPromptInputLineOptions> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut options = WindowPromptInputLineOptions::default();
    let mut parsed_description = false;
    let mut parsed_prompt = false;
    let mut parsed_initial_value = false;
    let mut parsed_action = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        match name.to_ascii_lowercase().as_str() {
            "description" => {
                let value =
                    modal_display_text_from_query_with_static_source(static_source, value.trim())?;
                if parsed_description || value.is_empty() {
                    return None;
                }
                options.description = value;
                parsed_description = true;
            }
            "prompt" => {
                let value =
                    modal_display_text_from_query_with_static_source(static_source, value.trim())?;
                if parsed_prompt {
                    return None;
                }
                options.prompt = Some(value);
                parsed_prompt = true;
            }
            "initial_value" | "initial-value" => {
                let value = parse_maybe_static_query_text(static_source, value)?;
                if parsed_initial_value {
                    return None;
                }
                options.initial_value = Some(value);
                parsed_initial_value = true;
            }
            "action" => {
                if parsed_action {
                    return None;
                }
                options.action = prompt_input_line_action_from_lua_action_with_static_source(
                    static_source,
                    value.trim(),
                );
                if options.action.is_none()
                    && !lua_action_callback_from_query_with_static_source(
                        static_source,
                        value.trim(),
                    )
                {
                    return None;
                }
                parsed_action = true;
            }
            _ => return None,
        }
    }

    parsed_description.then_some(options)
}

fn prompt_input_line_action_from_lua_action_callback_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowPromptInputLineAction> {
    prompt_input_line_action_from_lua_action_callback_with_static_source_and_depth(
        static_source,
        value,
        0,
    )
}

fn prompt_input_line_action_from_lua_action_callback_with_static_source_and_depth(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    depth: usize,
) -> Option<WindowPromptInputLineAction> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }
    if let Some(action) = prompt_input_line_action_from_lua_action_callback_query(value) {
        return Some(action);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_wezterm_action_callback_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return prompt_input_line_action_from_lua_action_callback_query(&value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        if let Some(value) = lua_static_wezterm_action_callback_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        ) {
            return prompt_input_line_action_from_lua_action_callback_query(&value);
        }
        return prompt_input_line_action_from_lua_action_callback_with_static_source_and_depth(
            Some(static_source),
            value,
            depth + 1,
        );
    }
    None
}

fn prompt_input_line_action_from_lua_action_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowPromptInputLineAction> {
    prompt_input_line_action_from_lua_action_callback_with_static_source(static_source, value)
        .or_else(|| {
            command_palette_structured_query_command(value)
                .map(Box::new)
                .map(WindowPromptInputLineAction::Command)
        })
        .or_else(|| {
            let static_source = static_source?;
            let value = lua_static_expression_assignment_value_before_offset_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )?;
            prompt_input_line_action_from_lua_action_with_static_source(Some(static_source), value)
        })
}

fn prompt_input_line_action_from_lua_action_callback_query(
    value: &str,
) -> Option<WindowPromptInputLineAction> {
    let callback = strip_lua_function_call_from_query(value, "wezterm.action_callback")
        .or_else(|| strip_lua_function_call_from_query(value, "action_callback"))?;
    let (body, window_param, pane_param, line_param) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(callback)?;
    let line_param = line_param?;
    prompt_input_line_action_from_lua_action_callback_body(
        body,
        window_param,
        pane_param,
        line_param,
    )
}

fn prompt_input_line_action_from_lua_action_callback_body(
    body: &str,
    window_param: &str,
    pane_param: &str,
    line_param: &str,
) -> Option<WindowPromptInputLineAction> {
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if let Some(action) = prompt_input_line_callback_statement_sends_pane_input(
            statement, pane_param, line_param,
        )? {
            return Some(action);
        }
        if prompt_input_line_callback_statement_renames_active_tab(
            statement,
            window_param,
            line_param,
        )? {
            return Some(WindowPromptInputLineAction::RenameActiveTab);
        }
        if prompt_input_line_callback_statement_switches_to_workspace_name(
            statement,
            window_param,
            pane_param,
            line_param,
        )? {
            return Some(WindowPromptInputLineAction::SwitchToWorkspaceName);
        }
        if let Some((branches, rest)) =
            lua_static_if_condition_and_body_branches_from_statement(statement)
        {
            let [(condition, if_body)] = branches.as_slice() else {
                continue;
            };
            if lua_callback_line_condition_from_expression(condition, line_param)?
                && lua_trim_end_statement_separator(rest).trim().is_empty()
            {
                if let Some(action) = prompt_input_line_callback_body_sends_pane_input(
                    if_body, pane_param, line_param,
                )? {
                    return Some(action);
                }
                if prompt_input_line_callback_body_renames_active_tab(
                    if_body,
                    window_param,
                    line_param,
                )? {
                    return Some(WindowPromptInputLineAction::RenameActiveTab);
                }
                if prompt_input_line_callback_body_switches_to_workspace_name(
                    if_body,
                    window_param,
                    pane_param,
                    line_param,
                )? {
                    return Some(WindowPromptInputLineAction::SwitchToWorkspaceName);
                }
            }
        }
    }
    None
}

fn prompt_input_line_callback_body_renames_active_tab(
    body: &str,
    window_param: &str,
    line_param: &str,
) -> Option<bool> {
    let mut found = false;
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if !prompt_input_line_callback_statement_renames_active_tab(
            statement,
            window_param,
            line_param,
        )? {
            return Some(false);
        }
        found = true;
    }
    Some(found)
}

fn prompt_input_line_callback_body_switches_to_workspace_name(
    body: &str,
    window_param: &str,
    pane_param: &str,
    line_param: &str,
) -> Option<bool> {
    let mut found = false;
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if !prompt_input_line_callback_statement_switches_to_workspace_name(
            statement,
            window_param,
            pane_param,
            line_param,
        )? {
            return Some(false);
        }
        found = true;
    }
    Some(found)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn prompt_input_line_callback_body_sends_pane_input(
    body: &str,
    pane_param: &str,
    line_param: &str,
) -> Option<Option<WindowPromptInputLineAction>> {
    let mut found = None;
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some(action) = prompt_input_line_callback_statement_sends_pane_input(
            statement, pane_param, line_param,
        )?
        else {
            return Some(None);
        };
        if found.as_ref().is_some_and(|existing| existing != &action) {
            return None;
        }
        found = Some(action);
    }
    Some(found)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn prompt_input_line_callback_statement_sends_pane_input(
    statement: &str,
    pane_param: &str,
    line_param: &str,
) -> Option<Option<WindowPromptInputLineAction>> {
    let statement = lua_trim_start_comments(statement)?;
    let Some(rest) = statement.strip_prefix(pane_param) else {
        return Some(None);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(None);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let (command_name, action) = if rest.starts_with("send_text")
        && lua_config_assignment_field_has_boundaries(rest, 0, "send_text")
    {
        ("send_text", WindowPromptInputLineAction::SendLineText)
    } else if rest.starts_with("send_paste")
        && lua_config_assignment_field_has_boundaries(rest, 0, "send_paste")
    {
        ("send_paste", WindowPromptInputLineAction::SendLinePaste)
    } else {
        return Some(None);
    };
    let rest = lua_trim_start_comments(rest.get(command_name.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [argument] = arguments.as_slice() else {
        return Some(None);
    };
    let argument = lua_trim_start_comments(argument.trim())?;
    let name = lua_identifier_literal_from_query(argument)?;
    if name != line_param
        || !lua_static_identifier_value_rest_is_statement_end(argument.get(name.len()..)?)
        || !lua_trim_end_statement_separator(rest).trim().is_empty()
    {
        return Some(None);
    }
    Some(Some(action))
}

fn lua_callback_line_condition_from_expression(condition: &str, line_param: &str) -> Option<bool> {
    let condition = lua_trim_start_comments(condition.trim())?;
    let name = lua_identifier_literal_from_query(condition)?;
    Some(
        name == line_param
            && lua_static_identifier_value_rest_is_statement_end(condition.get(name.len()..)?),
    )
}

fn prompt_input_line_callback_statement_renames_active_tab(
    statement: &str,
    window_param: &str,
    line_param: &str,
) -> Option<bool> {
    let statement = lua_trim_start_comments(statement)?;
    let Some(rest) = statement.strip_prefix(window_param) else {
        return Some(false);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("active_tab")
        || !lua_config_assignment_field_has_boundaries(rest, 0, "active_tab")
    {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest.get("active_tab".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !arguments.trim().is_empty() {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("set_title")
        || !lua_config_assignment_field_has_boundaries(rest, 0, "set_title")
    {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest.get("set_title".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [argument] = arguments.as_slice() else {
        return Some(false);
    };
    let argument = lua_trim_start_comments(argument.trim())?;
    let name = lua_identifier_literal_from_query(argument)?;
    if name != line_param
        || !lua_static_identifier_value_rest_is_statement_end(argument.get(name.len()..)?)
    {
        return Some(false);
    }
    Some(lua_trim_end_statement_separator(rest).trim().is_empty())
}

fn prompt_input_line_callback_statement_switches_to_workspace_name(
    statement: &str,
    window_param: &str,
    pane_param: &str,
    line_param: &str,
) -> Option<bool> {
    let statement = lua_trim_start_comments(statement)?;
    let Some(rest) = statement.strip_prefix(window_param) else {
        return Some(false);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("perform_action")
        || !lua_config_assignment_field_has_boundaries(rest, 0, "perform_action")
    {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest.get("perform_action".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [action, pane] = arguments.as_slice() else {
        return Some(false);
    };
    let pane = lua_trim_start_comments(pane.trim())?;
    let name = lua_identifier_literal_from_query(pane)?;
    if name != pane_param
        || !lua_static_identifier_value_rest_is_statement_end(pane.get(name.len()..)?)
        || !lua_trim_end_statement_separator(rest).trim().is_empty()
    {
        return Some(false);
    }
    prompt_input_line_switch_to_workspace_action_uses_line(action.trim(), line_param)
}

fn prompt_input_line_switch_to_workspace_action_uses_line(
    action: &str,
    line_param: &str,
) -> Option<bool> {
    let indexed_action;
    let action = if let Some(action) = strip_wezterm_action_prefix(action) {
        action
    } else if let Some(action) = strip_wezterm_action_index_prefix(action) {
        indexed_action = action;
        indexed_action.as_str()
    } else {
        action
    };
    let action = action.trim();
    let action_name = lua_identifier_literal_from_query(action)?;
    if normalized_action_name_query(action_name) != "switchtoworkspace" {
        return Some(false);
    }
    let rest = lua_trim_start_comments(action.get(action_name.len()..)?)?;
    let table = if rest.starts_with('{') {
        rest.strip_prefix('{')?.strip_suffix('}')?.trim()
    } else if rest.starts_with('(') {
        let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
        let (arguments, after) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_end_statement_separator(after).trim().is_empty() {
            return Some(false);
        }
        let arguments = split_lua_top_level_arguments(arguments)?;
        let [table] = arguments.as_slice() else {
            return Some(false);
        };

        table.trim().strip_prefix('{')?.strip_suffix('}')?.trim()
    } else {
        return Some(false);
    };
    let mut found_name = false;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        if !key.eq_ignore_ascii_case("name") {
            return Some(false);
        }
        if found_name {
            return Some(false);
        }
        let value = lua_trim_start_comments(value.trim().trim_end_matches(',').trim())?;
        let name = lua_identifier_literal_from_query(value)?;
        if name != line_param
            || !lua_static_identifier_value_rest_is_statement_end(value.get(name.len()..)?)
        {
            return Some(false);
        }
        found_name = true;
    }
    Some(found_name)
}

fn lua_action_callback_from_query(value: &str) -> bool {
    strip_lua_function_call_from_query(value, "wezterm.action_callback").is_some()
        || strip_lua_function_call_from_query(value, "action_callback").is_some()
}

fn lua_action_callback_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> bool {
    if lua_action_callback_from_query(value) {
        return true;
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_wezterm_action_callback_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return lua_action_callback_from_query(&value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        if let Some(value) = lua_static_wezterm_action_callback_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        ) {
            return lua_action_callback_from_query(&value);
        }
        return lua_action_callback_from_query(value);
    }
    false
}

fn modal_display_text_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_string_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return modal_display_text_from_query(value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return wezterm_format_visible_text_from_query_with_static_source(
            Some(static_source),
            value,
        );
    }

    if let Some(value) =
        wezterm_format_visible_text_from_query_with_static_source(static_source, value)
    {
        return Some(value);
    }

    parse_maybe_quoted_query_text(value)
}

fn modal_display_text_from_query(value: &str) -> Option<String> {
    wezterm_format_visible_text_from_query(value).or_else(|| parse_maybe_quoted_query_text(value))
}

fn wezterm_format_visible_text_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    native_format_items_from_wezterm_format_query_with_static_sources(static_source, None, value)
        .map(|items| native_format_items_plain_text(&items))
}

fn wezterm_format_visible_text_from_query(value: &str) -> Option<String> {
    native_format_items_from_wezterm_format_query(value)
        .map(|items| native_format_items_plain_text(&items))
}

fn native_format_items_plain_text(items: &[NativeFormatItem]) -> String {
    items
        .iter()
        .filter_map(|item| match item {
            NativeFormatItem::Text(text) => Some(tab_bar_ansi_plain_text(text)),
            NativeFormatItem::Foreground(_)
            | NativeFormatItem::Background(_)
            | NativeFormatItem::Attribute(_)
            | NativeFormatItem::ResetAttributes => None,
        })
        .collect::<String>()
}

fn wezterm_format_status_text_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    native_format_items_from_wezterm_format_query_with_static_sources(
        static_source,
        outer_static_source,
        value,
    )
    .map(|items| native_format_items_status_ansi_text(&items))
}

fn native_format_items_status_ansi_text(items: &[NativeFormatItem]) -> String {
    let mut text = String::new();
    for item in items {
        match item {
            NativeFormatItem::Text(value) => text.push_str(value),
            NativeFormatItem::Foreground(color) => {
                push_native_format_color_status_sgr(&mut text, 38, *color);
            }
            NativeFormatItem::Background(color) => {
                push_native_format_color_status_sgr(&mut text, 48, *color);
            }
            NativeFormatItem::Attribute(attribute) => {
                push_native_format_attribute_status_sgr(&mut text, attribute);
            }
            NativeFormatItem::ResetAttributes => text.push_str("\x1b[0m"),
        }
    }
    text
}

fn push_native_format_color_status_sgr(text: &mut String, target: u16, color: Color) {
    match color {
        Color::Default => match target {
            38 => text.push_str("\x1b[39m"),
            48 => text.push_str("\x1b[49m"),
            _ => {}
        },
        Color::Indexed(index) => {
            std::fmt::Write::write_fmt(text, format_args!("\x1b[{target};5;{index}m"))
                .expect("writing to a String cannot fail");
        }
        Color::Rgb(red, green, blue) | Color::Rgba(red, green, blue, _) => {
            std::fmt::Write::write_fmt(text, format_args!("\x1b[{target};2;{red};{green};{blue}m"))
                .expect("writing to a String cannot fail");
        }
    }
}

fn native_lua_color_config_text(color: Color) -> String {
    match color {
        Color::Default => "Default".to_owned(),
        Color::Indexed(index) => index.to_string(),
        Color::Rgb(red, green, blue) => format!("#{red:02x}{green:02x}{blue:02x}"),
        Color::Rgba(red, green, blue, alpha) => {
            format!("rgba({red},{green},{blue},{alpha})")
        }
    }
}

fn push_native_format_attribute_status_sgr(text: &mut String, attribute: &NativeFormatAttribute) {
    match attribute {
        NativeFormatAttribute::Intensity(NativeFormatIntensity::Normal) => {
            text.push_str("\x1b[22m");
        }
        NativeFormatAttribute::Intensity(NativeFormatIntensity::Bold) => {
            text.push_str("\x1b[1m");
        }
        NativeFormatAttribute::Intensity(NativeFormatIntensity::Half) => {
            text.push_str("\x1b[2m");
        }
        NativeFormatAttribute::Italic(true) => text.push_str("\x1b[3m"),
        NativeFormatAttribute::Italic(false) => text.push_str("\x1b[23m"),
        NativeFormatAttribute::Underline(NativeFormatUnderline::None) => text.push_str("\x1b[24m"),
        NativeFormatAttribute::Underline(NativeFormatUnderline::Single) => {
            text.push_str("\x1b[4m");
        }
        NativeFormatAttribute::Underline(NativeFormatUnderline::Double) => {
            text.push_str("\x1b[4:2m");
        }
        NativeFormatAttribute::Underline(NativeFormatUnderline::Curly) => {
            text.push_str("\x1b[4:3m");
        }
        NativeFormatAttribute::Underline(NativeFormatUnderline::Dotted) => {
            text.push_str("\x1b[4:4m");
        }
        NativeFormatAttribute::Underline(NativeFormatUnderline::Dashed) => {
            text.push_str("\x1b[4:5m");
        }
    }
}

fn input_selector_options_from_query(query: &str) -> Option<WindowInputSelectorOptions> {
    input_selector_options_from_query_with_static_source(None, query)
}

fn input_selector_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowInputSelectorOptions> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(rest) = strip_lua_function_call_from_query(query, "inputselector") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            return input_selector_lua_table_from_query_with_static_source(static_source, rest);
        }
        if static_source.is_some()
            && let Some(options) =
                input_selector_lua_table_from_query_with_static_source(static_source, rest)
        {
            return Some(options);
        }
    }

    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "inputselector=")
        && rest.trim_start().starts_with('{')
    {
        return input_selector_lua_table_from_query_with_static_source(static_source, rest);
    }

    let rest = strip_query_prefix_from_any(
        query,
        &[
            "input selector=",
            "input selector ",
            "inputselector=",
            "inputselector ",
        ],
    )?;
    let (options, _) = input_selector_fields_from_query_with_static_source(
        static_source,
        rest,
        WindowInputSelectorOptions::default(),
        false,
    )?;
    Some(options)
}

fn input_selector_choices_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<WindowInputSelectorChoice>> {
    let value = parse_maybe_static_query_text(static_source, value)?;

    let choices = split_unquoted_query_semicolons(&value)
        .into_iter()
        .map(str::trim)
        .filter(|choice| !choice.is_empty())
        .map(|choice| {
            let (id, label) = choice
                .split_once('=')
                .map_or((None, choice), |(id, label)| {
                    (Some(id.trim()), label.trim())
                });
            let label = parse_maybe_quoted_query_text(label)?;
            Some(WindowInputSelectorChoice {
                label,
                id: id.filter(|id| !id.is_empty()).map(str::to_owned),
            })
        })
        .collect::<Option<Vec<_>>>()?;

    (!choices.is_empty()).then_some(choices)
}

fn input_selector_choices_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<WindowInputSelectorChoice>> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut choices = Vec::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        choices.push(
            input_selector_choice_lua_table_from_query_with_static_source(static_source, field)?,
        );
    }

    (!choices.is_empty()).then_some(choices)
}

fn input_selector_choice_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowInputSelectorChoice> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut label = None;
    let mut id = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        match name.to_ascii_lowercase().as_str() {
            "label" => {
                let value = input_selector_choice_label_from_query_with_static_source(
                    static_source,
                    value.trim(),
                )?;
                if label.is_some() || value.is_empty() {
                    return None;
                }
                label = Some(value);
            }
            "id" => {
                let value = parse_maybe_static_query_text(static_source, value.trim())?;
                if id.is_some() {
                    return None;
                }
                id = (!value.is_empty()).then_some(value);
            }
            _ => return None,
        }
    }

    Some(WindowInputSelectorChoice { label: label?, id })
}

fn input_selector_choice_label_from_query(value: &str) -> Option<String> {
    input_selector_choice_label_from_query_with_static_source(None, value)
}

fn input_selector_choice_label_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_string_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return input_selector_choice_label_from_query(value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return wezterm_format_visible_text_from_query_with_static_source(
            Some(static_source),
            value,
        )
        .or_else(|| input_selector_choice_label_from_query(value));
    }

    wezterm_format_visible_text_from_query_with_static_source(static_source, value)
        .or_else(|| parse_maybe_quoted_query_text(value))
}

fn split_unquoted_query_semicolons(query: &str) -> Vec<&str> {
    let mut values = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;

    for (index, character) in query.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match quote {
            Some(_) if character == '\\' => {
                escaped = true;
            }
            Some(active_quote) if character == active_quote => {
                quote = None;
            }
            None if character == '"' || character == '\'' => {
                quote = Some(character);
            }
            None if character == ';' => {
                values.push(&query[start..index]);
                start = index + character.len_utf8();
            }
            Some(_) | None => {}
        }
    }

    values.push(&query[start..]);
    values
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn input_selector_fields_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    rest: &str,
    options: WindowInputSelectorOptions,
    fuzzy_seen: bool,
) -> Option<(WindowInputSelectorOptions, usize)> {
    let rest = rest.trim();
    if rest.is_empty() {
        return (!options.title.is_empty() && !options.choices.is_empty()).then_some((options, 0));
    }

    if let Some(value) = input_selector_strip_field_key(rest, "title") {
        return input_selector_field_splits(value)
            .filter_map(|(title, remaining)| {
                if title.is_empty() || !options.title.is_empty() {
                    return None;
                }
                let title = parse_maybe_static_query_text(static_source, title)?;
                let mut options = options.clone();
                let title_len = title.len();
                options.title = title;
                let (options, score) = input_selector_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    options,
                    fuzzy_seen,
                )?;
                Some((options, score + 1, title_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(options, score, _)| (options, score));
    }

    if let Some(value) = input_selector_strip_field_key(rest, "choices") {
        return input_selector_choices_splits(value)
            .filter_map(|(choices, remaining)| {
                if !options.choices.is_empty() {
                    return None;
                }
                let mut options = options.clone();
                options.choices =
                    input_selector_choices_from_query_with_static_source(static_source, choices)?;
                let (options, score) = input_selector_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    options,
                    fuzzy_seen,
                )?;
                Some((options, score + 1, choices.len()))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(options, score, _)| (options, score));
    }

    if let Some(value) = input_selector_strip_field_key(rest, "alphabet") {
        return input_selector_one_word_splits(value)
            .filter_map(|(alphabet, remaining)| {
                if options.alphabet.is_some() {
                    return None;
                }
                let alphabet = parse_maybe_static_query_text(static_source, alphabet)?;
                let mut options = options.clone();
                options.alphabet = Some(alphabet);
                let (options, score) = input_selector_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    options,
                    fuzzy_seen,
                )?;
                Some((options, score + 1))
            })
            .max_by_key(|(_, score)| *score);
    }

    if let Some(value) = input_selector_strip_field_key(rest, "description") {
        return input_selector_field_splits(value)
            .filter_map(|(description, remaining)| {
                if options.description.is_some() {
                    return None;
                }
                let description = parse_maybe_static_query_text(static_source, description)?;
                let mut options = options.clone();
                let description_len = description.len();
                options.description = Some(description);
                let (options, score) = input_selector_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    options,
                    fuzzy_seen,
                )?;
                Some((options, score + 1, description_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(options, score, _)| (options, score));
    }

    if let Some(value) = input_selector_strip_field_key(rest, "fuzzy_description")
        .or_else(|| input_selector_strip_field_key(rest, "fuzzy-description"))
        .or_else(|| input_selector_strip_field_key(rest, "fuzzy description"))
    {
        return input_selector_field_splits(value)
            .filter_map(|(fuzzy_description, remaining)| {
                if options.fuzzy_description.is_some() {
                    return None;
                }
                let fuzzy_description =
                    parse_maybe_static_query_text(static_source, fuzzy_description)?;
                let mut options = options.clone();
                let fuzzy_description_len = fuzzy_description.len();
                options.fuzzy_description = Some(fuzzy_description);
                let (options, score) = input_selector_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    options,
                    fuzzy_seen,
                )?;
                Some((options, score + 1, fuzzy_description_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(options, score, _)| (options, score));
    }

    if let Some(value) = input_selector_strip_fuzzy_field_key(rest) {
        if fuzzy_seen {
            return None;
        }
        return input_selector_one_word_splits(value)
            .filter_map(|(fuzzy, remaining)| {
                let mut options = options.clone();
                options.fuzzy = parse_maybe_static_query_bool(static_source, fuzzy)?;
                let (options, score) = input_selector_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    options,
                    true,
                )?;
                Some((options, score + 1))
            })
            .max_by_key(|(_, score)| *score);
    }

    None
}

fn input_selector_one_word_splits(rest: &str) -> impl Iterator<Item = (&str, &str)> {
    let (value, remaining) = rest
        .split_once(char::is_whitespace)
        .map_or((rest, ""), |(value, remaining)| {
            (value, remaining.trim_start())
        });
    std::iter::once((value, remaining))
}

fn input_selector_field_splits(rest: &str) -> impl Iterator<Item = (&str, &str)> {
    let mut offsets = input_selector_next_field_offsets(rest);
    offsets.reverse();
    offsets.push(rest.len());
    offsets
        .into_iter()
        .map(|offset| {
            let (value, remaining) = rest.split_at(offset);
            (value.trim(), remaining.trim_start())
        })
        .filter(|(value, _)| !value.is_empty())
}

fn input_selector_choices_splits(rest: &str) -> impl Iterator<Item = (&str, &str)> {
    let offset = input_selector_next_field_offsets(rest)
        .into_iter()
        .next()
        .unwrap_or(rest.len());
    let (value, remaining) = rest.split_at(offset);
    std::iter::once((value.trim(), remaining.trim_start())).filter(|(value, _)| !value.is_empty())
}

fn input_selector_strip_field_key<'a>(rest: &'a str, key: &str) -> Option<&'a str> {
    let key_prefix = rest.get(..key.len())?;
    let remaining = rest.get(key.len()..)?;
    key_prefix
        .eq_ignore_ascii_case(key)
        .then_some(remaining)
        .and_then(|remaining| {
            remaining.strip_prefix('=').or_else(|| {
                remaining
                    .starts_with(char::is_whitespace)
                    .then_some(remaining)
            })
        })
        .map(str::trim_start)
}

fn input_selector_strip_fuzzy_field_key(rest: &str) -> Option<&str> {
    input_selector_strip_field_key(rest, "fuzzy")
}

fn input_selector_next_field_offsets(rest: &str) -> Vec<usize> {
    let lowercase_rest = rest.to_ascii_lowercase();
    let mut offsets = [
        " title ",
        " title=",
        " choices ",
        " choices=",
        " alphabet ",
        " alphabet=",
        " description ",
        " description=",
        " fuzzy_description ",
        " fuzzy_description=",
        " fuzzy-description ",
        " fuzzy-description=",
        " fuzzy description ",
        " fuzzy description=",
        " fuzzy ",
        " fuzzy=",
    ]
    .into_iter()
    .flat_map(|needle| {
        lowercase_rest
            .match_indices(needle)
            .map(|(index, _)| index + 1)
    })
    .collect::<Vec<_>>();
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn input_selector_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowInputSelectorOptions> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut options = WindowInputSelectorOptions::default();
    let mut parsed_title = false;
    let mut parsed_choices = false;
    let mut parsed_alphabet = false;
    let mut parsed_description = false;
    let mut parsed_fuzzy_description = false;
    let mut parsed_fuzzy = false;
    let mut parsed_action = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let raw_value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "title" => {
                let value = parse_maybe_static_query_text(static_source, raw_value)?;
                if parsed_title || value.is_empty() {
                    return None;
                }
                options.title = value;
                parsed_title = true;
            }
            "choices" => {
                if parsed_choices {
                    return None;
                }
                options.choices = if raw_value.starts_with('{') {
                    input_selector_choices_lua_table_from_query_with_static_source(
                        static_source,
                        raw_value,
                    )?
                } else if let Some(choices) =
                    input_selector_choices_lua_table_from_query_with_static_source(
                        static_source,
                        raw_value,
                    )
                {
                    choices
                } else {
                    input_selector_choices_from_query_with_static_source(static_source, raw_value)?
                };
                parsed_choices = true;
            }
            "alphabet" => {
                let value = parse_maybe_static_query_text(static_source, raw_value)?;
                if parsed_alphabet {
                    return None;
                }
                options.alphabet = Some(value);
                parsed_alphabet = true;
            }
            "description" => {
                let value = parse_maybe_static_query_text(static_source, raw_value)?;
                if parsed_description {
                    return None;
                }
                options.description = Some(value);
                parsed_description = true;
            }
            "fuzzy_description" | "fuzzy-description" => {
                let value = parse_maybe_static_query_text(static_source, raw_value)?;
                if parsed_fuzzy_description {
                    return None;
                }
                options.fuzzy_description = Some(value);
                parsed_fuzzy_description = true;
            }
            "fuzzy" => {
                if parsed_fuzzy {
                    return None;
                }
                options.fuzzy = parse_maybe_static_query_bool(static_source, raw_value)?;
                parsed_fuzzy = true;
            }
            "action" => {
                if parsed_action {
                    return None;
                }
                options.action = input_selector_action_from_lua_action_with_static_source(
                    static_source,
                    raw_value,
                );
                if options.action.is_none()
                    && !lua_action_callback_from_query_with_static_source(static_source, raw_value)
                {
                    return None;
                }
                parsed_action = true;
            }
            _ => return None,
        }
    }

    (parsed_title && parsed_choices).then_some(options)
}

fn input_selector_action_from_lua_action_callback_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowInputSelectorAction> {
    input_selector_action_from_lua_action_callback_with_static_source_and_depth(
        static_source,
        value,
        0,
    )
}

fn input_selector_action_from_lua_action_callback_with_static_source_and_depth(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    depth: usize,
) -> Option<WindowInputSelectorAction> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }
    if let Some(action) = input_selector_action_from_lua_action_callback_query(value) {
        return Some(action);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_wezterm_action_callback_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return input_selector_action_from_lua_action_callback_query(&value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        if let Some(value) = lua_static_wezterm_action_callback_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        ) {
            return input_selector_action_from_lua_action_callback_query(&value);
        }
        return input_selector_action_from_lua_action_callback_with_static_source_and_depth(
            Some(static_source),
            value,
            depth + 1,
        );
    }
    None
}

fn input_selector_action_from_lua_action_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowInputSelectorAction> {
    input_selector_action_from_lua_action_callback_with_static_source(static_source, value)
        .or_else(|| {
            command_palette_structured_query_command(value)
                .map(Box::new)
                .map(WindowInputSelectorAction::Command)
        })
        .or_else(|| {
            let static_source = static_source?;
            let value = lua_static_expression_assignment_value_before_offset_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )?;
            input_selector_action_from_lua_action_with_static_source(Some(static_source), value)
        })
}
