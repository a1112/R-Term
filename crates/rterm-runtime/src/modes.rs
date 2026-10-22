use crate::queries::{
    KeyModifierOptions, KittyKeyboardApplyMode, KittyKeyboardMode, KittyKeyboardOperation,
    PrivateModeSequence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalModeChange {
    ApplicationCursorKeys(bool),
    ApplicationKeypad(bool),
    BracketedPaste(bool),
    Mouse(MouseInputMode),
    Focus(bool),
    SynchronizedOutput(bool),
    KittyKeyboardFlags(u16),
    ModifyOtherKeys(u8),
    Win32InputMode(bool),
}

pub const KITTY_KEYBOARD_DISAMBIGUATE: u16 = 1;
pub const KITTY_KEYBOARD_REPORT_EVENTS: u16 = 1 << 1;
pub const KITTY_KEYBOARD_ALTERNATE_KEYS: u16 = 1 << 2;
pub const KITTY_KEYBOARD_REPORT_ALL: u16 = 1 << 3;
pub const KITTY_KEYBOARD_ASSOCIATED_TEXT: u16 = 1 << 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MouseProtocolMode {
    #[default]
    X10,
    Utf8,
    Sgr,
    Urxvt,
    SgrPixels,
}

impl MouseProtocolMode {
    const fn bits(self) -> u8 {
        match self {
            Self::X10 => 0,
            Self::Utf8 => 1,
            Self::Sgr => 2,
            Self::Urxvt => 3,
            Self::SgrPixels => 4,
        }
    }

    const fn from_bits(bits: u8) -> Self {
        match bits {
            1 => Self::Utf8,
            2 => Self::Sgr,
            3 => Self::Urxvt,
            4 => Self::SgrPixels,
            _ => Self::X10,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MouseReportingMode {
    #[default]
    None,
    Normal,
    ButtonEvent,
    AnyEvent,
}

impl MouseReportingMode {
    const fn bits(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Normal => 1,
            Self::ButtonEvent => 2,
            Self::AnyEvent => 3,
        }
    }

    const fn from_bits(bits: u8) -> Self {
        match bits {
            1 => Self::Normal,
            2 => Self::ButtonEvent,
            3 => Self::AnyEvent,
            _ => Self::None,
        }
    }

    #[must_use]
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MouseInputMode {
    reporting: MouseReportingMode,
    protocol: MouseProtocolMode,
}

impl MouseInputMode {
    const PROTOCOL_SHIFT: u8 = 2;
    const PROTOCOL_MASK: u8 = 0b111;

    #[must_use]
    pub const fn new(reporting: MouseReportingMode, protocol: MouseProtocolMode) -> Self {
        Self {
            reporting,
            protocol,
        }
    }

    #[must_use]
    pub const fn bits(self) -> u8 {
        self.reporting.bits() | (self.protocol.bits() << Self::PROTOCOL_SHIFT)
    }

    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self {
            reporting: MouseReportingMode::from_bits(bits & 0b11),
            protocol: MouseProtocolMode::from_bits(
                (bits >> Self::PROTOCOL_SHIFT) & Self::PROTOCOL_MASK,
            ),
        }
    }

    #[must_use]
    pub const fn reporting(self) -> MouseReportingMode {
        self.reporting
    }

    #[must_use]
    pub const fn protocol(self) -> MouseProtocolMode {
        self.protocol
    }

    #[must_use]
    pub const fn reporting_enabled(self) -> bool {
        self.reporting.is_enabled()
    }

    #[must_use]
    pub const fn with_reporting(self, reporting: MouseReportingMode) -> Self {
        Self { reporting, ..self }
    }
}

pub struct TerminalModeTracker {
    pending: Vec<u8>,
    #[cfg(test)]
    copied_bytes: u64,
    #[cfg(test)]
    growths: u64,
    mouse_modes: MouseModes,
    kitty_keyboard_modes: KittyKeyboardModes,
    modify_other_keys: u8,
    allow_win32_input_mode: bool,
    tracked_modes: TrackedTerminalModes,
}

impl Default for TerminalModeTracker {
    fn default() -> Self {
        Self {
            pending: Vec::new(),
            #[cfg(test)]
            copied_bytes: 0,
            #[cfg(test)]
            growths: 0,
            mouse_modes: MouseModes::default(),
            kitty_keyboard_modes: KittyKeyboardModes::default(),
            modify_other_keys: 0,
            allow_win32_input_mode: true,
            tracked_modes: TrackedTerminalModes::default(),
        }
    }
}

impl TerminalModeTracker {
    const APPLICATION_KEYPAD_PREFIX: &'static [u8] = b"\x1b=";
    const CSI_MODE_PREFIX: &'static [u8] = b"\x1b[";
    const CSI_PRIVATE_MODE_PREFIX: &'static [u8] = b"\x1b[?";
    const C1_CSI_MODE_PREFIX: &'static [u8] = b"\x9b";
    const C1_CSI_PRIVATE_MODE_PREFIX: &'static [u8] = b"\x9b?";
    const UTF8_C1_CSI_MODE_PREFIX: &'static [u8] = b"\xc2\x9b";
    const UTF8_C1_CSI_PRIVATE_MODE_PREFIX: &'static [u8] = b"\xc2\x9b?";
    const NUMERIC_KEYPAD_PREFIX: &'static [u8] = b"\x1b>";
    const RESET_PREFIX: &'static [u8] = b"\x1bc";
    const SOFT_RESET_PREFIX: &'static [u8] = b"\x1b[!p";
    const C1_SOFT_RESET_PREFIX: &'static [u8] = b"\x9b!p";
    const UTF8_C1_SOFT_RESET_PREFIX: &'static [u8] = b"\xc2\x9b!p";

    /// Applies a mode transition produced by the authoritative worker to a
    /// presentation-only compatibility mirror.
    pub fn install_change(&mut self, change: TerminalModeChange) {
        match change {
            TerminalModeChange::ApplicationCursorKeys(enabled) => {
                self.tracked_modes
                    .set(TrackedTerminalModes::APPLICATION_CURSOR_KEYS, enabled);
            }
            TerminalModeChange::ApplicationKeypad(enabled) => {
                self.tracked_modes
                    .set(TrackedTerminalModes::APPLICATION_KEYPAD, enabled);
            }
            TerminalModeChange::BracketedPaste(enabled) => {
                self.tracked_modes
                    .set(TrackedTerminalModes::BRACKETED_PASTE, enabled);
            }
            TerminalModeChange::Mouse(mode) => self.mouse_modes.install(mode),
            TerminalModeChange::Focus(enabled) => {
                self.tracked_modes.set(TrackedTerminalModes::FOCUS, enabled);
            }
            TerminalModeChange::SynchronizedOutput(enabled) => {
                self.tracked_modes
                    .set(TrackedTerminalModes::SYNCHRONIZED_OUTPUT, enabled);
            }
            TerminalModeChange::KittyKeyboardFlags(flags) => {
                self.kitty_keyboard_modes.flags = flags;
                self.kitty_keyboard_modes.stack.clear();
            }
            TerminalModeChange::ModifyOtherKeys(value) => self.modify_other_keys = value,
            TerminalModeChange::Win32InputMode(enabled) => {
                self.tracked_modes
                    .set(TrackedTerminalModes::WIN32_INPUT_MODE, enabled);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn process(&mut self, bytes: &[u8], mut emit: impl FnMut(TerminalModeChange)) {
        if self.pending.is_empty() && !bytes.iter().copied().any(is_mode_candidate_start) {
            return;
        }
        #[cfg(test)]
        let capacity = self.pending.capacity();
        self.pending.extend_from_slice(bytes);
        #[cfg(test)]
        {
            self.copied_bytes = self
                .copied_bytes
                .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
            if self.pending.capacity() != capacity {
                self.growths = self.growths.saturating_add(1);
            }
        }

        loop {
            let Some(start) = Self::find_next_mode_start(&self.pending) else {
                self.retain_possible_prefix();
                return;
            };
            if let Some(control) = control_string_containing(&self.pending, start.index) {
                if let Some(end) = control.end {
                    self.pending.drain(..end);
                    continue;
                }
                if control.start > 0 {
                    self.pending.drain(..control.start);
                }
                return;
            }
            if start.index > 0 {
                self.pending.drain(..start.index);
            }

            match start.sequence {
                ModeSequence::ApplicationKeypad(enabled) => {
                    self.set_application_keypad(enabled, &mut emit);
                    self.pending.drain(..2);
                }
                ModeSequence::Reset => {
                    self.reset(&mut emit);
                    self.pending.drain(..2);
                }
                ModeSequence::SoftReset { prefix_len } => {
                    self.soft_reset(&mut emit);
                    self.pending.drain(..prefix_len);
                }
                ModeSequence::CsiMode { prefix_len } => {
                    match Self::parse_mode_sequence(&self.pending, prefix_len) {
                        ModeParse::Complete {
                            modes,
                            enabled,
                            consumed,
                        } => {
                            for mode in modes {
                                self.apply_ansi_mode(mode, enabled);
                            }
                            self.pending.drain(..consumed);
                        }
                        ModeParse::Incomplete => return,
                        ModeParse::Invalid => {
                            self.pending.drain(..1);
                        }
                    }
                }
                ModeSequence::CsiPrivateMode { prefix_len } => {
                    match Self::parse_mode_sequence(&self.pending, prefix_len) {
                        ModeParse::Complete {
                            modes,
                            enabled,
                            consumed,
                        } => {
                            let before_mouse = self.mouse_input_mode();
                            let mut saw_mouse_mode = false;
                            for mode in modes {
                                if self.mouse_modes.set(mode, enabled) {
                                    saw_mouse_mode = true;
                                } else {
                                    self.apply_mode(mode, enabled, &mut emit);
                                }
                            }
                            if saw_mouse_mode {
                                self.emit_mouse_change(before_mouse, &mut emit);
                            }
                            self.pending.drain(..consumed);
                        }
                        ModeParse::Incomplete => return,
                        ModeParse::Invalid => {
                            self.pending.drain(..1);
                        }
                    }
                }
            }
        }
    }
    pub fn process_without_emitting(&mut self, bytes: &[u8]) {
        self.process(bytes, |_| {});
    }

    pub fn apply_private_mode_sequence(
        &mut self,
        sequence: &PrivateModeSequence,
        mut emit: impl FnMut(TerminalModeChange),
    ) {
        let before_mouse = self.mouse_input_mode();
        let mut saw_mouse_mode = false;
        for &mode in &sequence.modes {
            if self.mouse_modes.set(mode, sequence.enabled) {
                saw_mouse_mode = true;
            } else {
                self.apply_mode(mode, sequence.enabled, &mut emit);
            }
        }
        if saw_mouse_mode {
            self.emit_mouse_change(before_mouse, &mut emit);
        }
    }

    pub fn apply_kitty_keyboard_sequence(
        &mut self,
        sequence: KittyKeyboardMode,
        mut emit: impl FnMut(TerminalModeChange),
    ) {
        self.apply_kitty_keyboard_mode(
            sequence.operation,
            sequence.value,
            sequence.apply_mode,
            &mut emit,
        );
    }

    pub fn apply_key_modifier_options_sequence(
        &mut self,
        sequence: KeyModifierOptions,
        mut emit: impl FnMut(TerminalModeChange),
    ) {
        self.apply_key_modifier_options(sequence.resource, sequence.value, &mut emit);
    }

    pub fn clear_kitty_keyboard_flags(&mut self) {
        self.kitty_keyboard_modes = KittyKeyboardModes::default();
    }

    pub fn finish(&mut self, mut emit: impl FnMut(TerminalModeChange)) {
        self.pending.clear();
        if self
            .tracked_modes
            .set(TrackedTerminalModes::SYNCHRONIZED_OUTPUT, false)
        {
            emit(TerminalModeChange::SynchronizedOutput(false));
        }
    }

    pub fn set_allow_win32_input_mode(&mut self, allowed: bool) {
        self.allow_win32_input_mode = allowed;
        if !allowed {
            self.tracked_modes
                .set(TrackedTerminalModes::WIN32_INPUT_MODE, false);
        }
    }

    fn find_next_mode_start(bytes: &[u8]) -> Option<ModeSequenceStart> {
        [
            Self::find_csi_private_mode_start(bytes),
            Self::find_soft_reset_start(bytes),
            Self::find_csi_mode_start(bytes),
            Self::find_simple_escape_start(bytes),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|start| start.index)
    }

    fn find_csi_private_mode_start(bytes: &[u8]) -> Option<ModeSequenceStart> {
        Self::find_mode_start_with_prefixes(
            bytes,
            [
                (
                    Self::CSI_PRIVATE_MODE_PREFIX,
                    ModeSequence::CsiPrivateMode {
                        prefix_len: Self::CSI_PRIVATE_MODE_PREFIX.len(),
                    },
                ),
                (
                    Self::C1_CSI_PRIVATE_MODE_PREFIX,
                    ModeSequence::CsiPrivateMode {
                        prefix_len: Self::C1_CSI_PRIVATE_MODE_PREFIX.len(),
                    },
                ),
                (
                    Self::UTF8_C1_CSI_PRIVATE_MODE_PREFIX,
                    ModeSequence::CsiPrivateMode {
                        prefix_len: Self::UTF8_C1_CSI_PRIVATE_MODE_PREFIX.len(),
                    },
                ),
            ],
        )
    }

    fn find_soft_reset_start(bytes: &[u8]) -> Option<ModeSequenceStart> {
        Self::find_mode_start_with_prefixes(
            bytes,
            [
                (
                    Self::SOFT_RESET_PREFIX,
                    ModeSequence::SoftReset {
                        prefix_len: Self::SOFT_RESET_PREFIX.len(),
                    },
                ),
                (
                    Self::C1_SOFT_RESET_PREFIX,
                    ModeSequence::SoftReset {
                        prefix_len: Self::C1_SOFT_RESET_PREFIX.len(),
                    },
                ),
                (
                    Self::UTF8_C1_SOFT_RESET_PREFIX,
                    ModeSequence::SoftReset {
                        prefix_len: Self::UTF8_C1_SOFT_RESET_PREFIX.len(),
                    },
                ),
            ],
        )
    }

    fn find_csi_mode_start(bytes: &[u8]) -> Option<ModeSequenceStart> {
        Self::find_mode_start_with_prefixes(
            bytes,
            [
                (
                    Self::CSI_MODE_PREFIX,
                    ModeSequence::CsiMode {
                        prefix_len: Self::CSI_MODE_PREFIX.len(),
                    },
                ),
                (
                    Self::C1_CSI_MODE_PREFIX,
                    ModeSequence::CsiMode {
                        prefix_len: Self::C1_CSI_MODE_PREFIX.len(),
                    },
                ),
                (
                    Self::UTF8_C1_CSI_MODE_PREFIX,
                    ModeSequence::CsiMode {
                        prefix_len: Self::UTF8_C1_CSI_MODE_PREFIX.len(),
                    },
                ),
            ],
        )
    }

    fn find_simple_escape_start(bytes: &[u8]) -> Option<ModeSequenceStart> {
        Self::find_mode_start_with_prefixes(
            bytes,
            [
                (
                    Self::APPLICATION_KEYPAD_PREFIX,
                    ModeSequence::ApplicationKeypad(true),
                ),
                (
                    Self::NUMERIC_KEYPAD_PREFIX,
                    ModeSequence::ApplicationKeypad(false),
                ),
                (Self::RESET_PREFIX, ModeSequence::Reset),
            ],
        )
    }

    fn find_mode_start_with_prefixes<const N: usize>(
        bytes: &[u8],
        prefixes: [(&'static [u8], ModeSequence); N],
    ) -> Option<ModeSequenceStart> {
        prefixes
            .into_iter()
            .filter_map(|(prefix, sequence)| {
                find_subslice(bytes, prefix).map(|index| ModeSequenceStart { index, sequence })
            })
            .min_by_key(|start| start.index)
    }

    fn parse_mode_sequence(bytes: &[u8], prefix_len: usize) -> ModeParse {
        let mut cursor = prefix_len;
        let mut modes = Vec::new();

        loop {
            if cursor >= bytes.len() {
                return ModeParse::Incomplete;
            }

            let start = cursor;
            let mut mode = 0u16;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                mode = mode
                    .saturating_mul(10)
                    .saturating_add(u16::from(bytes[cursor] - b'0'));
                cursor += 1;
            }

            if cursor == start {
                return ModeParse::Invalid;
            }
            modes.push(mode);

            if cursor >= bytes.len() {
                return ModeParse::Incomplete;
            }

            match bytes[cursor] {
                b';' => cursor += 1,
                b'h' | b'l' => {
                    return ModeParse::Complete {
                        modes,
                        enabled: bytes[cursor] == b'h',
                        consumed: cursor + 1,
                    };
                }
                _ => return ModeParse::Invalid,
            }
        }
    }

    fn apply_mode(&mut self, mode: u16, enabled: bool, emit: &mut impl FnMut(TerminalModeChange)) {
        if let Some(tracked_mode) = TrackedTerminalModes::private_mode_bit(mode) {
            self.tracked_modes.set(tracked_mode, enabled);
            return;
        }

        match mode {
            1 => {
                if self
                    .tracked_modes
                    .set(TrackedTerminalModes::APPLICATION_CURSOR_KEYS, enabled)
                {
                    emit(TerminalModeChange::ApplicationCursorKeys(enabled));
                }
            }
            1004 => {
                if self.tracked_modes.set(TrackedTerminalModes::FOCUS, enabled) {
                    emit(TerminalModeChange::Focus(enabled));
                }
            }
            2004 => {
                if self
                    .tracked_modes
                    .set(TrackedTerminalModes::BRACKETED_PASTE, enabled)
                {
                    emit(TerminalModeChange::BracketedPaste(enabled));
                }
            }
            2026 => {
                if self
                    .tracked_modes
                    .set(TrackedTerminalModes::SYNCHRONIZED_OUTPUT, enabled)
                {
                    emit(TerminalModeChange::SynchronizedOutput(enabled));
                }
            }
            9001 => {
                if self.allow_win32_input_mode
                    && self
                        .tracked_modes
                        .set(TrackedTerminalModes::WIN32_INPUT_MODE, enabled)
                {
                    emit(TerminalModeChange::Win32InputMode(enabled));
                }
            }
            _ => {}
        }
    }

    fn apply_kitty_keyboard_mode(
        &mut self,
        operation: KittyKeyboardOperation,
        value: u16,
        apply_mode: KittyKeyboardApplyMode,
        emit: &mut impl FnMut(TerminalModeChange),
    ) {
        let before = self.kitty_keyboard_flags();
        match operation {
            KittyKeyboardOperation::Push => self.kitty_keyboard_modes.push(value),
            KittyKeyboardOperation::Pop => self.kitty_keyboard_modes.pop(value),
            KittyKeyboardOperation::Apply => self.kitty_keyboard_modes.apply(value, apply_mode),
        }
        let after = self.kitty_keyboard_flags();
        if before != after {
            emit(TerminalModeChange::KittyKeyboardFlags(after));
        }
    }

    fn apply_key_modifier_options(
        &mut self,
        resource: Option<u16>,
        value: Option<u16>,
        emit: &mut impl FnMut(TerminalModeChange),
    ) {
        if resource.is_none() || resource == Some(4) {
            let next = value
                .and_then(|value| u8::try_from(value).ok())
                .filter(|value| *value <= 2)
                .unwrap_or(0);
            if self.modify_other_keys != next {
                self.modify_other_keys = next;
                emit(TerminalModeChange::ModifyOtherKeys(next));
            }
        }
    }

    fn apply_ansi_mode(&mut self, mode: u16, enabled: bool) {
        match mode {
            4 => {
                self.tracked_modes
                    .set(TrackedTerminalModes::INSERT_MODE, enabled);
            }
            8 => {
                self.tracked_modes
                    .set(TrackedTerminalModes::BIDIRECTIONAL_SUPPORT, enabled);
            }
            20 => {
                self.tracked_modes
                    .set(TrackedTerminalModes::AUTOMATIC_NEWLINE, enabled);
            }
            _ => {}
        }
    }

    fn soft_reset(&mut self, emit: &mut impl FnMut(TerminalModeChange)) {
        if self
            .tracked_modes
            .set(TrackedTerminalModes::APPLICATION_CURSOR_KEYS, false)
        {
            emit(TerminalModeChange::ApplicationCursorKeys(false));
        }
        if self.modify_other_keys != 0 {
            self.modify_other_keys = 0;
            emit(TerminalModeChange::ModifyOtherKeys(0));
        }
        if self
            .tracked_modes
            .set(TrackedTerminalModes::APPLICATION_KEYPAD, false)
        {
            emit(TerminalModeChange::ApplicationKeypad(false));
        }
        self.tracked_modes
            .set(TrackedTerminalModes::ORIGIN_MODE, false);
        self.tracked_modes
            .set(TrackedTerminalModes::AUTO_WRAP, true);
        self.tracked_modes
            .set(TrackedTerminalModes::CURSOR_VISIBLE, true);
        self.tracked_modes
            .set(TrackedTerminalModes::INSERT_MODE, false);
        self.tracked_modes
            .set(TrackedTerminalModes::LEFT_RIGHT_MARGIN_MODE, false);
        self.tracked_modes
            .set(TrackedTerminalModes::BIDIRECTIONAL_SUPPORT, false);
        self.tracked_modes
            .set(TrackedTerminalModes::REVERSE_WRAP, false);
        self.tracked_modes
            .set(TrackedTerminalModes::SCREEN_REVERSE, false);
    }

    fn reset(&mut self, emit: &mut impl FnMut(TerminalModeChange)) {
        let application_cursor_keys = self.application_cursor_keys();
        let application_keypad = self.application_keypad();
        let bracketed_paste = self.bracketed_paste();
        let mouse_input_mode = self.mouse_input_mode();
        let focus_reporting = self.focus_reporting();
        let synchronized_output = self.synchronized_output();
        let kitty_keyboard_flags = self.kitty_keyboard_flags();
        let modify_other_keys = self.modify_other_keys();
        let win32_input_mode = self.win32_input_mode();

        self.mouse_modes = MouseModes::default();
        self.kitty_keyboard_modes = KittyKeyboardModes::default();
        self.modify_other_keys = 0;
        self.tracked_modes = TrackedTerminalModes::default();

        if application_cursor_keys {
            emit(TerminalModeChange::ApplicationCursorKeys(false));
        }
        if application_keypad {
            emit(TerminalModeChange::ApplicationKeypad(false));
        }
        if bracketed_paste {
            emit(TerminalModeChange::BracketedPaste(false));
        }
        if mouse_input_mode != MouseInputMode::default() {
            emit(TerminalModeChange::Mouse(MouseInputMode::default()));
        }
        if focus_reporting {
            emit(TerminalModeChange::Focus(false));
        }
        if synchronized_output {
            emit(TerminalModeChange::SynchronizedOutput(false));
        }
        if kitty_keyboard_flags != 0 {
            emit(TerminalModeChange::KittyKeyboardFlags(0));
        }
        if modify_other_keys != 0 {
            emit(TerminalModeChange::ModifyOtherKeys(0));
        }
        if win32_input_mode {
            emit(TerminalModeChange::Win32InputMode(false));
        }
    }

    fn set_application_keypad(&mut self, enabled: bool, emit: &mut impl FnMut(TerminalModeChange)) {
        if self
            .tracked_modes
            .set(TrackedTerminalModes::APPLICATION_KEYPAD, enabled)
        {
            emit(TerminalModeChange::ApplicationKeypad(enabled));
        }
    }

    fn emit_mouse_change(&self, before: MouseInputMode, emit: &mut impl FnMut(TerminalModeChange)) {
        let after = self.mouse_input_mode();
        if before != after && (before.reporting_enabled() || after.reporting_enabled()) {
            emit(TerminalModeChange::Mouse(after));
        }
    }

    #[must_use]
    pub fn application_cursor_keys(&self) -> bool {
        self.tracked_modes
            .enabled(TrackedTerminalModes::APPLICATION_CURSOR_KEYS)
    }

    #[must_use]
    pub fn application_keypad(&self) -> bool {
        self.tracked_modes
            .enabled(TrackedTerminalModes::APPLICATION_KEYPAD)
    }

    #[must_use]
    pub fn focus_reporting(&self) -> bool {
        self.tracked_modes.enabled(TrackedTerminalModes::FOCUS)
    }

    #[must_use]
    pub fn bracketed_paste(&self) -> bool {
        self.tracked_modes
            .enabled(TrackedTerminalModes::BRACKETED_PASTE)
    }

    #[must_use]
    pub fn synchronized_output(&self) -> bool {
        self.tracked_modes
            .enabled(TrackedTerminalModes::SYNCHRONIZED_OUTPUT)
    }

    #[must_use]
    pub fn mouse_input_mode(&self) -> MouseInputMode {
        self.mouse_modes.input_mode()
    }

    #[must_use]
    pub fn win32_input_mode(&self) -> bool {
        self.tracked_modes
            .enabled(TrackedTerminalModes::WIN32_INPUT_MODE)
    }

    #[must_use]
    pub fn kitty_keyboard_flags(&self) -> u16 {
        self.kitty_keyboard_modes.flags()
    }

    #[must_use]
    pub const fn modify_other_keys(&self) -> u8 {
        self.modify_other_keys
    }

    #[must_use]
    pub fn private_mode_report_value(&self, mode: u16) -> u8 {
        match mode {
            1 => mode_report_value(self.application_cursor_keys()),
            2 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::DEC_ANSI_MODE),
            ),
            3 => mode_report_value(false),
            5 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::SCREEN_REVERSE),
            ),
            6 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::ORIGIN_MODE),
            ),
            7 => mode_report_value(self.tracked_modes.enabled(TrackedTerminalModes::AUTO_WRAP)),
            12 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::CURSOR_BLINKING),
            ),
            25 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::CURSOR_VISIBLE),
            ),
            45 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::REVERSE_WRAP),
            ),
            69 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::LEFT_RIGHT_MARGIN_MODE),
            ),
            80 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::SIXEL_DISPLAY_MODE),
            ),
            8452 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::SIXEL_SCROLLS_RIGHT),
            ),
            1000 | 1002 | 1003 | 1005 | 1006 | 1015 | 1016 => {
                self.mouse_modes.report_value(mode).unwrap_or(0)
            }
            1004 => mode_report_value(self.focus_reporting()),
            1034 => mode_report_value(self.tracked_modes.enabled(TrackedTerminalModes::META_KEY)),
            1070 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::PRIVATE_COLOR_REGISTERS),
            ),
            9001 => mode_report_value(self.win32_input_mode()),
            2004 => mode_report_value(self.bracketed_paste()),
            2026 => mode_report_value(self.synchronized_output()),
            2027 => 3,
            _ => 0,
        }
    }

    #[must_use]
    pub fn ansi_mode_report_value(&self, mode: u16) -> u8 {
        match mode {
            4 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::INSERT_MODE),
            ),
            8 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::BIDIRECTIONAL_SUPPORT),
            ),
            20 => mode_report_value(
                self.tracked_modes
                    .enabled(TrackedTerminalModes::AUTOMATIC_NEWLINE),
            ),
            _ => 0,
        }
    }

    fn retain_possible_prefix(&mut self) {
        let retained = [
            Self::CSI_MODE_PREFIX,
            Self::CSI_PRIVATE_MODE_PREFIX,
            Self::C1_CSI_MODE_PREFIX,
            Self::C1_CSI_PRIVATE_MODE_PREFIX,
            Self::UTF8_C1_CSI_MODE_PREFIX,
            Self::UTF8_C1_CSI_PRIVATE_MODE_PREFIX,
            Self::APPLICATION_KEYPAD_PREFIX,
            Self::NUMERIC_KEYPAD_PREFIX,
            Self::RESET_PREFIX,
            Self::SOFT_RESET_PREFIX,
            Self::C1_SOFT_RESET_PREFIX,
            Self::UTF8_C1_SOFT_RESET_PREFIX,
        ]
        .into_iter()
        .map(|prefix| suffix_len_matching_prefix(&self.pending, prefix))
        .max()
        .unwrap_or(0);
        let retained = retained
            .max(incomplete_osc_control_sequence_suffix_len(&self.pending))
            .max(incomplete_st_control_sequence_suffix_len(&self.pending));
        let writable = self.pending.len().saturating_sub(retained);
        if writable > 0 {
            self.pending.drain(..writable);
        }
    }
}

const fn is_mode_candidate_start(byte: u8) -> bool {
    matches!(byte, 0x1b | 0x90 | 0x98 | 0x9b | 0x9d | 0x9e | 0x9f | 0xc2)
}

pub(crate) fn framed_control_may_change_modes(bytes: &[u8]) -> bool {
    if matches!(bytes, b"\x1b=" | b"\x1b>" | b"\x1bc") {
        return true;
    }
    let body = bytes
        .strip_prefix(b"\x1b[")
        .or_else(|| bytes.strip_prefix(b"\x9b"))
        .or_else(|| bytes.strip_prefix(b"\xc2\x9b"));
    body.is_some_and(|body| body == b"!p" || matches!(body.last(), Some(b'h' | b'l')))
}

#[derive(Default)]
struct KittyKeyboardModes {
    flags: u16,
    stack: Vec<u16>,
}

impl KittyKeyboardModes {
    const MAX_STACK: usize = 32;

    fn flags(&self) -> u16 {
        self.flags
    }

    fn push(&mut self, flags: u16) {
        if self.stack.len() == Self::MAX_STACK {
            self.stack.remove(0);
        }
        self.stack.push(self.flags);
        self.flags = flags;
    }

    fn pop(&mut self, count: u16) {
        let count = if count == 0 { 1 } else { count };
        for _ in 0..count {
            let Some(flags) = self.stack.pop() else {
                self.flags = 0;
                return;
            };
            self.flags = flags;
        }
    }

    fn apply(&mut self, flags: u16, mode: KittyKeyboardApplyMode) {
        match mode {
            KittyKeyboardApplyMode::Replace => self.flags = flags,
            KittyKeyboardApplyMode::Set => self.flags |= flags,
            KittyKeyboardApplyMode::Reset => self.flags &= !flags,
        }
    }
}

#[derive(Default)]
struct MouseModes(u8);

impl MouseModes {
    const NORMAL: u8 = 1;
    const BUTTON_EVENT: u8 = 1 << 1;
    const ANY_EVENT: u8 = 1 << 2;
    const UTF8_PROTOCOL: u8 = 1 << 3;
    const SGR_PROTOCOL: u8 = 1 << 4;
    const URXVT_PROTOCOL: u8 = 1 << 5;
    const SGR_PIXELS_PROTOCOL: u8 = 1 << 6;
    const PROTOCOL_MASK: u8 =
        Self::UTF8_PROTOCOL | Self::SGR_PROTOCOL | Self::URXVT_PROTOCOL | Self::SGR_PIXELS_PROTOCOL;

    fn install(&mut self, mode: MouseInputMode) {
        *self = Self::default();
        let reporting = match mode.reporting() {
            MouseReportingMode::None => None,
            MouseReportingMode::Normal => Some(1000),
            MouseReportingMode::ButtonEvent => Some(1002),
            MouseReportingMode::AnyEvent => Some(1003),
        };
        if let Some(reporting) = reporting {
            self.set(reporting, true);
        }
        let protocol = match mode.protocol() {
            MouseProtocolMode::X10 => None,
            MouseProtocolMode::Utf8 => Some(1005),
            MouseProtocolMode::Sgr => Some(1006),
            MouseProtocolMode::Urxvt => Some(1015),
            MouseProtocolMode::SgrPixels => Some(1016),
        };
        if let Some(protocol) = protocol {
            self.set(protocol, true);
        }
    }

    fn set(&mut self, mode: u16, enabled: bool) -> bool {
        let Some(mask) = Self::mask(mode) else {
            return false;
        };

        if Self::is_protocol_mode(mode) {
            self.0 &= !Self::PROTOCOL_MASK;
            if enabled {
                self.0 |= mask;
            }
        } else if enabled {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }

        true
    }

    fn input_mode(&self) -> MouseInputMode {
        let reporting = if self.0 & Self::ANY_EVENT != 0 {
            MouseReportingMode::AnyEvent
        } else if self.0 & Self::BUTTON_EVENT != 0 {
            MouseReportingMode::ButtonEvent
        } else if self.0 & Self::NORMAL != 0 {
            MouseReportingMode::Normal
        } else {
            MouseReportingMode::None
        };
        let protocol = if self.0 & Self::SGR_PIXELS_PROTOCOL != 0 {
            MouseProtocolMode::SgrPixels
        } else if self.0 & Self::SGR_PROTOCOL != 0 {
            MouseProtocolMode::Sgr
        } else if self.0 & Self::URXVT_PROTOCOL != 0 {
            MouseProtocolMode::Urxvt
        } else if self.0 & Self::UTF8_PROTOCOL != 0 {
            MouseProtocolMode::Utf8
        } else {
            MouseProtocolMode::X10
        };

        MouseInputMode::new(reporting, protocol)
    }

    const fn mask(mode: u16) -> Option<u8> {
        match mode {
            1000 => Some(Self::NORMAL),
            1002 => Some(Self::BUTTON_EVENT),
            1003 => Some(Self::ANY_EVENT),
            1005 => Some(Self::UTF8_PROTOCOL),
            1006 => Some(Self::SGR_PROTOCOL),
            1015 => Some(Self::URXVT_PROTOCOL),
            1016 => Some(Self::SGR_PIXELS_PROTOCOL),
            _ => None,
        }
    }

    const fn is_protocol_mode(mode: u16) -> bool {
        matches!(mode, 1005 | 1006 | 1015 | 1016)
    }

    fn report_value(&self, mode: u16) -> Option<u8> {
        Self::mask(mode).map(|mask| mode_report_value(self.0 & mask != 0))
    }
}

const fn mode_report_value(enabled: bool) -> u8 {
    if enabled { 1 } else { 2 }
}

#[derive(Clone, Copy)]
struct TrackedTerminalModes(u32);

impl TrackedTerminalModes {
    const APPLICATION_CURSOR_KEYS: u32 = 1;
    const APPLICATION_KEYPAD: u32 = 1 << 1;
    const BRACKETED_PASTE: u32 = 1 << 2;
    const FOCUS: u32 = 1 << 3;
    const SYNCHRONIZED_OUTPUT: u32 = 1 << 4;
    const ORIGIN_MODE: u32 = 1 << 5;
    const AUTO_WRAP: u32 = 1 << 6;
    const CURSOR_VISIBLE: u32 = 1 << 7;
    const CURSOR_BLINKING: u32 = 1 << 8;
    const ALTERNATE_SCREEN_47: u32 = 1 << 9;
    const ALTERNATE_SCREEN_1047: u32 = 1 << 10;
    const ALTERNATE_SCREEN_1049: u32 = 1 << 11;
    const PRIVATE_CURSOR_SAVE: u32 = 1 << 12;
    const INSERT_MODE: u32 = 1 << 13;
    const META_KEY: u32 = 1 << 14;
    const LEFT_RIGHT_MARGIN_MODE: u32 = 1 << 15;
    const SIXEL_DISPLAY_MODE: u32 = 1 << 16;
    const SIXEL_SCROLLS_RIGHT: u32 = 1 << 17;
    const REVERSE_WRAP: u32 = 1 << 18;
    const SCREEN_REVERSE: u32 = 1 << 19;
    const WIN32_INPUT_MODE: u32 = 1 << 20;
    const AUTOMATIC_NEWLINE: u32 = 1 << 21;
    const BIDIRECTIONAL_SUPPORT: u32 = 1 << 22;
    const PRIVATE_COLOR_REGISTERS: u32 = 1 << 23;
    const DEC_ANSI_MODE: u32 = 1 << 24;

    fn private_mode_bit(mode: u16) -> Option<u32> {
        match mode {
            2 => Some(Self::DEC_ANSI_MODE),
            5 => Some(Self::SCREEN_REVERSE),
            6 => Some(Self::ORIGIN_MODE),
            7 => Some(Self::AUTO_WRAP),
            12 => Some(Self::CURSOR_BLINKING),
            25 => Some(Self::CURSOR_VISIBLE),
            45 => Some(Self::REVERSE_WRAP),
            47 => Some(Self::ALTERNATE_SCREEN_47),
            69 => Some(Self::LEFT_RIGHT_MARGIN_MODE),
            80 => Some(Self::SIXEL_DISPLAY_MODE),
            1034 => Some(Self::META_KEY),
            1047 => Some(Self::ALTERNATE_SCREEN_1047),
            1048 => Some(Self::PRIVATE_CURSOR_SAVE),
            1049 => Some(Self::ALTERNATE_SCREEN_1049),
            1070 => Some(Self::PRIVATE_COLOR_REGISTERS),
            8452 => Some(Self::SIXEL_SCROLLS_RIGHT),
            _ => None,
        }
    }

    fn set(&mut self, mode: u32, enabled: bool) -> bool {
        let before = self.0;
        if enabled {
            self.0 |= mode;
        } else {
            self.0 &= !mode;
        }
        self.0 != before
    }

    const fn enabled(self, mode: u32) -> bool {
        self.0 & mode != 0
    }
}

impl Default for TrackedTerminalModes {
    fn default() -> Self {
        Self(Self::AUTO_WRAP | Self::CURSOR_VISIBLE)
    }
}

enum ModeParse {
    Complete {
        modes: Vec<u16>,
        enabled: bool,
        consumed: usize,
    },
    Incomplete,
    Invalid,
}

#[derive(Clone, Copy)]
struct ModeSequenceStart {
    index: usize,
    sequence: ModeSequence,
}

#[derive(Clone, Copy)]
enum ModeSequence {
    CsiPrivateMode { prefix_len: usize },
    CsiMode { prefix_len: usize },
    ApplicationKeypad(bool),
    Reset,
    SoftReset { prefix_len: usize },
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    haystack
        .windows(needle.len())
        .enumerate()
        .find_map(|(index, window)| {
            (window == needle && !raw_c1_prefix_is_utf8_continuation(haystack, index, needle))
                .then_some(index)
        })
}

fn raw_c1_prefix_is_utf8_continuation(bytes: &[u8], index: usize, prefix: &[u8]) -> bool {
    prefix
        .first()
        .is_some_and(|byte| is_raw_c1_control_byte(*byte))
        && is_utf8_continuation_in_potential_sequence(bytes, index)
}

fn is_raw_c1_control_byte(byte: u8) -> bool {
    (0x80..=0x9f).contains(&byte)
}

fn is_utf8_continuation_in_potential_sequence(bytes: &[u8], index: usize) -> bool {
    if index == 0
        || bytes
            .get(index)
            .is_none_or(|byte| !is_utf8_continuation(*byte))
    {
        return false;
    }

    let mut start = index;
    while start > 0 && is_utf8_continuation(bytes[start]) {
        start -= 1;
    }
    if start == index {
        return false;
    }

    let Some(expected_len) = utf8_sequence_len(bytes[start]) else {
        return false;
    };

    index < start + expected_len
        && bytes[start + 1..=index]
            .iter()
            .all(|byte| is_utf8_continuation(*byte))
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

#[derive(Clone, Copy)]
enum ControlStringKind {
    Osc,
    St,
}

#[derive(Clone, Copy)]
struct ControlStringStart {
    index: usize,
    prefix_len: usize,
    kind: ControlStringKind,
}

struct ControlStringSpan {
    start: usize,
    end: Option<usize>,
}

fn control_string_containing(bytes: &[u8], index: usize) -> Option<ControlStringSpan> {
    let mut offset = 0;
    while offset < bytes.len() {
        let start = find_next_control_string_start(&bytes[offset..])?;
        let absolute_start = offset + start.index;
        if absolute_start > index {
            return None;
        }
        let content_start = absolute_start + start.prefix_len;
        let terminator = find_control_string_terminator(&bytes[content_start..], start.kind);
        let end = terminator.map(|terminator| content_start + terminator.index + terminator.length);
        if end.is_none_or(|end| index < end) {
            return Some(ControlStringSpan {
                start: absolute_start,
                end,
            });
        }
        offset = end.expect("complete control string has an end");
    }
    None
}

fn incomplete_osc_control_sequence_suffix_len(bytes: &[u8]) -> usize {
    incomplete_control_string_suffix_len(bytes, ControlStringKind::Osc)
        .max(suffix_len_matching_prefix(bytes, b"\x1b]"))
}

fn incomplete_st_control_sequence_suffix_len(bytes: &[u8]) -> usize {
    incomplete_control_string_suffix_len(bytes, ControlStringKind::St)
        .max(suffix_len_matching_prefix(bytes, b"\x1bP"))
        .max(suffix_len_matching_prefix(bytes, b"\x1bX"))
        .max(suffix_len_matching_prefix(bytes, b"\x1b^"))
        .max(suffix_len_matching_prefix(bytes, b"\x1b_"))
}

fn incomplete_control_string_suffix_len(bytes: &[u8], expected_kind: ControlStringKind) -> usize {
    let mut offset = 0;
    while offset < bytes.len() {
        let Some(start) = find_next_control_string_start(&bytes[offset..]) else {
            return 0;
        };
        let absolute_start = offset + start.index;
        let content_start = absolute_start + start.prefix_len;
        let Some(terminator) = find_control_string_terminator(&bytes[content_start..], start.kind)
        else {
            return if control_string_kinds_match(expected_kind, start.kind) {
                bytes.len() - absolute_start
            } else {
                0
            };
        };
        offset = content_start + terminator.index + terminator.length;
    }
    0
}

const fn control_string_kinds_match(left: ControlStringKind, right: ControlStringKind) -> bool {
    matches!(
        (left, right),
        (ControlStringKind::Osc, ControlStringKind::Osc)
            | (ControlStringKind::St, ControlStringKind::St)
    )
}

fn find_next_control_string_start(bytes: &[u8]) -> Option<ControlStringStart> {
    let osc = find_next_osc_start(bytes).map(|(index, prefix_len)| ControlStringStart {
        index,
        prefix_len,
        kind: ControlStringKind::Osc,
    });
    let st =
        find_next_st_control_string_start(bytes).map(|(index, prefix_len)| ControlStringStart {
            index,
            prefix_len,
            kind: ControlStringKind::St,
        });
    [osc, st]
        .into_iter()
        .flatten()
        .min_by_key(|start| start.index)
}

fn find_next_osc_start(bytes: &[u8]) -> Option<(usize, usize)> {
    [
        (b"\x1b]".as_slice(), 2),
        (b"\xc2\x9d".as_slice(), 2),
        (b"\x9d".as_slice(), 1),
    ]
    .into_iter()
    .filter_map(|(prefix, prefix_len)| {
        find_subslice(bytes, prefix).map(|index| (index, prefix_len))
    })
    .min_by_key(|(index, _)| *index)
}

fn find_next_st_control_string_start(bytes: &[u8]) -> Option<(usize, usize)> {
    [
        (b"\x1bP".as_slice(), 2),
        (b"\x1bX".as_slice(), 2),
        (b"\x1b^".as_slice(), 2),
        (b"\x1b_".as_slice(), 2),
        (b"\xc2\x90".as_slice(), 2),
        (b"\xc2\x98".as_slice(), 2),
        (b"\xc2\x9e".as_slice(), 2),
        (b"\xc2\x9f".as_slice(), 2),
        (b"\x90".as_slice(), 1),
        (b"\x98".as_slice(), 1),
        (b"\x9e".as_slice(), 1),
        (b"\x9f".as_slice(), 1),
    ]
    .into_iter()
    .filter_map(|(prefix, prefix_len)| {
        find_subslice(bytes, prefix).map(|index| (index, prefix_len))
    })
    .min_by_key(|(index, _)| *index)
}

fn find_control_string_terminator(
    bytes: &[u8],
    kind: ControlStringKind,
) -> Option<ControlStringTerminator> {
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\x18' | b'\x1a' => {
                return Some(ControlStringTerminator { index, length: 1 });
            }
            b'\x07' if matches!(kind, ControlStringKind::Osc) => {
                return Some(ControlStringTerminator { index, length: 1 });
            }
            0x9c if !is_utf8_continuation_in_potential_sequence(bytes, index) => {
                return Some(ControlStringTerminator { index, length: 1 });
            }
            0xc2 if bytes.get(index + 1) == Some(&0x9c) => {
                return Some(ControlStringTerminator { index, length: 2 });
            }
            b'\x1b' => match bytes.get(index + 1) {
                Some(b'\\') => {
                    return Some(ControlStringTerminator { index, length: 2 });
                }
                Some(_) => index += 1,
                None => return None,
            },
            _ => index += 1,
        }
    }
    None
}

struct ControlStringTerminator {
    index: usize,
    length: usize,
}

fn suffix_len_matching_prefix(haystack: &[u8], needle: &[u8]) -> usize {
    let max = haystack.len().min(needle.len().saturating_sub(1));
    (1..=max)
        .rev()
        .find(|&length| {
            let suffix_start = haystack.len() - length;
            haystack[suffix_start..] == needle[..length]
                && !raw_c1_prefix_is_utf8_continuation(haystack, suffix_start, &needle[..length])
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{TerminalModeChange, TerminalModeTracker};
    use crate::queries::{
        KeyModifierOptions, KittyKeyboardApplyMode, KittyKeyboardMode, KittyKeyboardOperation,
        PrivateModeSequence,
    };

    fn assert_control_string_mode_parts<'a>(
        input: &[u8],
        parts: impl IntoIterator<Item = &'a [u8]>,
        partition: &str,
    ) {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();
        for part in parts {
            tracker.process(part, |change| changes.push(change));
        }
        assert!(
            !tracker.application_cursor_keys(),
            "embedded fake mode escaped its control string: input={input:?}, {partition}"
        );
        assert!(!tracker.focus_reporting(), "input={input:?}, {partition}");
        assert_eq!(
            tracker.private_mode_report_value(45),
            2,
            "input={input:?}, {partition}"
        );
        assert!(
            tracker.bracketed_paste(),
            "mode after terminator was lost: input={input:?}, {partition}"
        );
        assert_eq!(
            changes,
            vec![TerminalModeChange::BracketedPaste(true)],
            "input={input:?}, {partition}"
        );
        assert!(
            tracker.pending.is_empty(),
            "completed transcript retained bytes: input={input:?}, {partition}"
        );
    }

    fn assert_control_string_modes_for_every_split(input: &[u8]) {
        for split in 0..=input.len() {
            assert_control_string_mode_parts(
                input,
                [&input[..split], &input[split..]],
                &format!("split={split}"),
            );
        }
        assert_control_string_mode_parts(input, input.chunks(1), "bytewise");
    }

    #[test]
    fn control_string_modes_honor_utf8_c1_st_cancel_and_overlapping_escape() {
        let families = [
            (b"\x1b]".as_slice(), true),
            (b"\x1bP".as_slice(), false),
            (b"\x1b_".as_slice(), false),
            (b"\x1b^".as_slice(), false),
            (b"\x1bX".as_slice(), false),
            (b"\x9d".as_slice(), true),
            (b"\x90".as_slice(), false),
            (b"\x9f".as_slice(), false),
            (b"\x9e".as_slice(), false),
            (b"\x98".as_slice(), false),
            (b"\xc2\x9d".as_slice(), true),
            (b"\xc2\x90".as_slice(), false),
            (b"\xc2\x9f".as_slice(), false),
            (b"\xc2\x9e".as_slice(), false),
            (b"\xc2\x98".as_slice(), false),
        ];
        let fake_modes = [
            b"\x1b[?1h".as_slice(),
            b"\x9b?1h".as_slice(),
            b"\xc2\x9b?1h".as_slice(),
        ];

        for (start, osc) in families {
            let endings: &[&[u8]] = if osc {
                &[
                    b"\x1b\x1b\\",
                    b"\x18",
                    b"\x1a",
                    b"\x1b\\",
                    b"\x9c",
                    b"\xc2\x9c",
                    b"\x07",
                ]
            } else {
                &[
                    b"\x1b\x1b\\",
                    b"\x18",
                    b"\x1a",
                    b"\x1b\\",
                    b"\x9c",
                    b"\xc2\x9c",
                ]
            };
            for fake_mode in fake_modes {
                for ending in endings {
                    let input = [
                        start,
                        b"hidden",
                        fake_mode,
                        b"tail",
                        *ending,
                        b"\x1b[?2004h",
                    ]
                    .concat();
                    assert_control_string_modes_for_every_split(&input);
                }
            }

            let ending = if osc {
                b"\x07".as_slice()
            } else {
                b"\xc2\x9c".as_slice()
            };
            let multiple_fakes = [
                start,
                b"hidden\x9b?1h;middle",
                b"\xc2\x9b?1004h;tail\x9b?45h",
                ending,
                b"\x1b[?2004h",
            ]
            .concat();
            assert_control_string_modes_for_every_split(&multiple_fakes);

            let part_one = [start, b"hidden\x9b?1h"].concat();
            let part_two = b"middle\xc2\x9b?1004h\x9b?45h".to_vec();
            let part_three = [ending, b"\x1b[?2004h"].concat();
            assert_control_string_mode_parts(
                &multiple_fakes,
                [
                    part_one.as_slice(),
                    part_two.as_slice(),
                    part_three.as_slice(),
                ],
                "three chunks",
            );

            if !osc {
                let bel_is_body = [start, b"hidden\x07\x9b?1h", b"\x1b\\\x1b[?2004h"].concat();
                assert_control_string_modes_for_every_split(&bel_is_body);
            }
        }
    }

    #[test]
    fn plain_chunks_do_not_copy_or_grow_mode_candidate_storage() {
        let plain = vec![b'x'; 1024 * 1024];
        let mut tracker = TerminalModeTracker::default();

        for chunk in plain.chunks(8192) {
            tracker.process_without_emitting(chunk);
        }

        assert_eq!(tracker.copied_bytes, 0);
        assert_eq!(tracker.growths, 0);
        assert!(tracker.pending.is_empty());
    }

    #[test]
    fn every_split_preserves_modes_resets_and_ignores_embedded_control_strings() {
        fn run(parts: impl IntoIterator<Item = Vec<u8>>) -> (Vec<TerminalModeChange>, Vec<u8>) {
            let mut tracker = TerminalModeTracker::default();
            let mut changes = Vec::new();
            for part in parts {
                tracker.process(&part, |change| changes.push(change));
            }
            let final_state = vec![
                u8::from(tracker.application_cursor_keys()),
                u8::from(tracker.application_keypad()),
                u8::from(tracker.bracketed_paste()),
                tracker.private_mode_report_value(45),
                tracker.private_mode_report_value(25),
            ];
            assert!(tracker.pending.is_empty());
            (changes, final_state)
        }

        let transcript = b"plain\x1b]0;fake\x1b[?1h\x07\x9dfake\x1b[?1h\x9c\x90fake\x1b[?1h\x9c\x1b[?1h\x9b?2004h\xc2\x9b?45h\x1b=\x1b[!p\x1b[?45h\x1bc\x1b[?1h";
        let expected = run([transcript.to_vec()]);

        for split in 0..=transcript.len() {
            let actual = run([transcript[..split].to_vec(), transcript[split..].to_vec()]);
            assert_eq!(actual, expected, "split={split}");
        }
        let bytewise = run(transcript.iter().map(|byte| vec![*byte]));
        assert_eq!(bytewise, expected);
    }

    #[test]
    fn tracks_kitty_keyboard_protocol_push_pop_flags() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        assert_eq!(tracker.kitty_keyboard_flags(), 0);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Push,
                value: 1,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 1);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Push,
                value: 9,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 9);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Pop,
                value: 0,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 1);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Pop,
                value: 1,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 0);
        assert_eq!(
            changes,
            vec![
                TerminalModeChange::KittyKeyboardFlags(1),
                TerminalModeChange::KittyKeyboardFlags(9),
                TerminalModeChange::KittyKeyboardFlags(1),
                TerminalModeChange::KittyKeyboardFlags(0)
            ]
        );
    }

    #[test]
    fn tracks_kitty_keyboard_protocol_set_reset_flags() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Apply,
                value: 1,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 1);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Apply,
                value: 8,
                apply_mode: KittyKeyboardApplyMode::Set,
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 9);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Apply,
                value: 1,
                apply_mode: KittyKeyboardApplyMode::Reset,
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 8);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Apply,
                value: 8,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 8);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Apply,
                value: 0,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 0);
        assert_eq!(
            changes,
            vec![
                TerminalModeChange::KittyKeyboardFlags(1),
                TerminalModeChange::KittyKeyboardFlags(9),
                TerminalModeChange::KittyKeyboardFlags(8),
                TerminalModeChange::KittyKeyboardFlags(0)
            ]
        );
    }

    #[test]
    fn applies_kitty_keyboard_protocol_push_pop_dtos() {
        let mut tracker = TerminalModeTracker::default();

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Push,
                value: 17,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |_| {},
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 17);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Push,
                value: 3,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |_| {},
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 3);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Pop,
                value: 2,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |_| {},
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 0);
    }

    #[test]
    fn applies_kitty_keyboard_protocol_set_reset_dtos() {
        let mut tracker = TerminalModeTracker::default();

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Apply,
                value: 17,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |_| {},
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 17);

        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Apply,
                value: 1,
                apply_mode: KittyKeyboardApplyMode::Reset,
            },
            |_| {},
        );
        assert_eq!(tracker.kitty_keyboard_flags(), 16);
    }

    #[test]
    fn applies_combined_private_mode_dto_without_losing_mouse_state() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.apply_private_mode_sequence(
            &PrivateModeSequence {
                modes: vec![1000, 2026],
                enabled: true,
            },
            |change| changes.push(change),
        );

        assert!(tracker.synchronized_output());
        assert!(tracker.mouse_input_mode().reporting_enabled());
        assert!(
            changes
                .iter()
                .any(|change| matches!(change, TerminalModeChange::SynchronizedOutput(true)))
        );
        assert!(
            changes
                .iter()
                .any(|change| matches!(change, TerminalModeChange::Mouse(_)))
        );
    }

    #[test]
    fn tracks_reverse_wraparound_private_mode_status() {
        let mut tracker = TerminalModeTracker::default();

        assert_eq!(tracker.private_mode_report_value(45), 2);

        tracker.process(b"\x1b[?45h", |_| {});
        assert_eq!(tracker.private_mode_report_value(45), 1);

        tracker.process(b"\x9b?45l", |_| {});
        assert_eq!(tracker.private_mode_report_value(45), 2);
    }

    #[test]
    fn tracks_screen_reverse_private_mode_status() {
        let mut tracker = TerminalModeTracker::default();

        assert_eq!(tracker.private_mode_report_value(5), 2);

        tracker.process(b"\x1b[?5h", |_| {});
        assert_eq!(tracker.private_mode_report_value(5), 1);

        tracker.process(b"\x9b?5l", |_| {});
        assert_eq!(tracker.private_mode_report_value(5), 2);
    }

    #[test]
    fn tracks_wezterm_private_mode_reports() {
        let mut tracker = TerminalModeTracker::default();

        assert_eq!(tracker.private_mode_report_value(3), 2);
        assert_eq!(tracker.private_mode_report_value(2027), 3);
        assert_eq!(tracker.private_mode_report_value(1070), 2);

        tracker.process(b"\x1b[?2027l\x1b[?1070h", |_| {});
        assert_eq!(tracker.private_mode_report_value(2027), 3);
        assert_eq!(tracker.private_mode_report_value(1070), 1);

        tracker.process(b"\x9b?1070l", |_| {});
        assert_eq!(tracker.private_mode_report_value(1070), 2);
    }

    #[test]
    fn tracks_dec_ansi_private_mode_status() {
        let mut tracker = TerminalModeTracker::default();

        assert_eq!(tracker.private_mode_report_value(2), 2);

        tracker.process(b"\x1b[?2h", |_| {});
        assert_eq!(tracker.private_mode_report_value(2), 1);

        tracker.process(b"\x9b?2l", |_| {});
        assert_eq!(tracker.private_mode_report_value(2), 2);
    }

    #[test]
    fn tracks_utf8_c1_private_mode_status() {
        let mut tracker = TerminalModeTracker::default();
        let csi = '\u{9b}';

        assert_eq!(tracker.private_mode_report_value(45), 2);

        tracker.process(format!("{csi}?45h").as_bytes(), |_| {});
        assert_eq!(tracker.private_mode_report_value(45), 1);

        tracker.process(format!("{csi}?45l").as_bytes(), |_| {});
        assert_eq!(tracker.private_mode_report_value(45), 2);
    }

    #[test]
    fn soft_reset_restores_cursor_visibility_mode_status() {
        let mut tracker = TerminalModeTracker::default();

        tracker.process(b"\x1b[?25l", |_| {});
        assert_eq!(tracker.private_mode_report_value(25), 2);

        tracker.process(b"\x1b[!p", |_| {});

        assert_eq!(tracker.private_mode_report_value(25), 1);
    }

    #[test]
    fn tracks_xterm_modify_other_keys_mode() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        assert_eq!(tracker.modify_other_keys(), 0);

        tracker.apply_key_modifier_options_sequence(
            KeyModifierOptions {
                resource: Some(4),
                value: Some(2),
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.modify_other_keys(), 2);

        tracker.apply_key_modifier_options_sequence(
            KeyModifierOptions {
                resource: Some(4),
                value: Some(1),
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.modify_other_keys(), 1);

        tracker.apply_key_modifier_options_sequence(
            KeyModifierOptions {
                resource: Some(4),
                value: Some(0),
            },
            |change| changes.push(change),
        );
        assert_eq!(tracker.modify_other_keys(), 0);
        assert_eq!(
            changes,
            vec![
                TerminalModeChange::ModifyOtherKeys(2),
                TerminalModeChange::ModifyOtherKeys(1),
                TerminalModeChange::ModifyOtherKeys(0)
            ]
        );
    }

    #[test]
    fn tracks_win32_input_private_mode() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        assert!(!tracker.win32_input_mode());

        tracker.process(b"\x1b[?9001h", |change| changes.push(change));
        assert!(tracker.win32_input_mode());

        tracker.process(b"\x9b?9001l", |change| changes.push(change));
        assert!(!tracker.win32_input_mode());

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::Win32InputMode(true),
                TerminalModeChange::Win32InputMode(false)
            ]
        );
    }
}
