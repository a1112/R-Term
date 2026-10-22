#[derive(Default)]
pub struct TerminalVisibleOutputFilter {
    state: VisibleOutputState,
    #[cfg(test)]
    bulk_appended_bytes: u64,
}

#[derive(Default)]
enum VisibleOutputState {
    #[default]
    Ground,
    Utf8C1Lead,
    Utf8Text {
        remaining: u8,
    },
    Escape,
    Csi,
    Osc,
    OscEscape,
    OscUtf8C1,
    StString,
    StStringEscape,
    StStringUtf8C1,
}

impl TerminalVisibleOutputFilter {
    pub fn process(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut visible = Vec::new();

        self.process_into(bytes, &mut visible);
        visible
    }

    pub fn process_into(&mut self, bytes: &[u8], visible: &mut Vec<u8>) {
        if matches!(&self.state, VisibleOutputState::Ground)
            && !bytes
                .iter()
                .copied()
                .any(byte_requires_visible_state_machine)
        {
            visible.extend_from_slice(bytes);
            #[cfg(test)]
            {
                self.bulk_appended_bytes = self
                    .bulk_appended_bytes
                    .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
            }
            return;
        }
        for byte in bytes {
            self.process_byte(*byte, visible);
        }
    }

    fn process_byte(&mut self, byte: u8, visible: &mut Vec<u8>) {
        match self.state {
            VisibleOutputState::Ground => self.process_ground_byte(byte, visible),
            VisibleOutputState::Utf8C1Lead => self.process_utf8_c1_byte(byte, visible),
            VisibleOutputState::Utf8Text { remaining } => {
                self.process_utf8_text_byte(byte, remaining, visible);
            }
            VisibleOutputState::Escape => self.process_escape_byte(byte),
            VisibleOutputState::Csi => self.process_csi_byte(byte),
            VisibleOutputState::Osc => self.process_osc_byte(byte),
            VisibleOutputState::OscEscape => self.process_osc_escape_byte(byte),
            VisibleOutputState::OscUtf8C1 => self.process_osc_utf8_c1_byte(byte),
            VisibleOutputState::StString => self.process_st_string_byte(byte),
            VisibleOutputState::StStringEscape => self.process_st_string_escape_byte(byte),
            VisibleOutputState::StStringUtf8C1 => self.process_st_string_utf8_c1_byte(byte),
        }
    }

    fn process_ground_byte(&mut self, byte: u8, visible: &mut Vec<u8>) {
        match byte {
            b'\x07' | b'\x00' | b'\x18' | b'\x1a' => {}
            b'\x1b' => self.state = VisibleOutputState::Escape,
            0xc2 => self.state = VisibleOutputState::Utf8C1Lead,
            0xc3..=0xdf => {
                visible.push(byte);
                self.state = VisibleOutputState::Utf8Text { remaining: 1 };
            }
            0xe0..=0xef => {
                visible.push(byte);
                self.state = VisibleOutputState::Utf8Text { remaining: 2 };
            }
            0xf0..=0xf4 => {
                visible.push(byte);
                self.state = VisibleOutputState::Utf8Text { remaining: 3 };
            }
            0x90 | 0x98 | 0x9e | 0x9f => self.state = VisibleOutputState::StString,
            0x9b => self.state = VisibleOutputState::Csi,
            0x9d => self.state = VisibleOutputState::Osc,
            _ => visible.push(byte),
        }
    }

    fn process_utf8_c1_byte(&mut self, byte: u8, visible: &mut Vec<u8>) {
        if (0x80..=0x9f).contains(&byte) {
            self.process_c1_control(byte);
        } else {
            visible.push(0xc2);
            self.state = VisibleOutputState::Ground;
            self.process_ground_byte(byte, visible);
        }
    }

    fn process_utf8_text_byte(&mut self, byte: u8, remaining: u8, visible: &mut Vec<u8>) {
        if is_utf8_continuation(byte) {
            visible.push(byte);
            self.state = if remaining > 1 {
                VisibleOutputState::Utf8Text {
                    remaining: remaining - 1,
                }
            } else {
                VisibleOutputState::Ground
            };
        } else {
            self.state = VisibleOutputState::Ground;
            self.process_ground_byte(byte, visible);
        }
    }

    fn process_c1_control(&mut self, byte: u8) {
        self.state = match byte {
            0x90 | 0x98 | 0x9e | 0x9f => VisibleOutputState::StString,
            0x9b => VisibleOutputState::Csi,
            0x9d => VisibleOutputState::Osc,
            _ => VisibleOutputState::Ground,
        };
    }

    fn process_escape_byte(&mut self, byte: u8) {
        self.state = match byte {
            b'[' => VisibleOutputState::Csi,
            b']' => VisibleOutputState::Osc,
            b'P' | b'X' | b'^' | b'_' => VisibleOutputState::StString,
            _ => VisibleOutputState::Ground,
        };
    }

    fn process_csi_byte(&mut self, byte: u8) {
        if byte == b'\x18' || byte == b'\x1a' || (0x40..=0x7e).contains(&byte) {
            self.state = VisibleOutputState::Ground;
        }
    }

    fn process_osc_byte(&mut self, byte: u8) {
        match byte {
            b'\x07' | 0x9c | b'\x18' | b'\x1a' => self.state = VisibleOutputState::Ground,
            b'\x1b' => self.state = VisibleOutputState::OscEscape,
            0xc2 => self.state = VisibleOutputState::OscUtf8C1,
            _ => {}
        }
    }

    fn process_osc_escape_byte(&mut self, byte: u8) {
        if byte == b'\\' {
            self.state = VisibleOutputState::Ground;
        } else {
            self.state = VisibleOutputState::Osc;
            self.process_osc_byte(byte);
        }
    }

    fn process_osc_utf8_c1_byte(&mut self, byte: u8) {
        if byte == 0x9c {
            self.state = VisibleOutputState::Ground;
        } else {
            self.state = VisibleOutputState::Osc;
            self.process_osc_byte(byte);
        }
    }

    fn process_st_string_byte(&mut self, byte: u8) {
        match byte {
            0x9c | b'\x18' | b'\x1a' => self.state = VisibleOutputState::Ground,
            b'\x1b' => self.state = VisibleOutputState::StStringEscape,
            0xc2 => self.state = VisibleOutputState::StStringUtf8C1,
            _ => {}
        }
    }

    fn process_st_string_escape_byte(&mut self, byte: u8) {
        if byte == b'\\' {
            self.state = VisibleOutputState::Ground;
        } else {
            self.state = VisibleOutputState::StString;
            self.process_st_string_byte(byte);
        }
    }

    fn process_st_string_utf8_c1_byte(&mut self, byte: u8) {
        if byte == 0x9c {
            self.state = VisibleOutputState::Ground;
        } else {
            self.state = VisibleOutputState::StString;
            self.process_st_string_byte(byte);
        }
    }
}

fn is_utf8_continuation(byte: u8) -> bool {
    byte & 0b1100_0000 == 0b1000_0000
}

const fn byte_requires_visible_state_machine(byte: u8) -> bool {
    matches!(
        byte,
        b'\x00'
            | b'\x07'
            | b'\x18'
            | b'\x1a'
            | b'\x1b'
            | 0x90
            | 0x98
            | 0x9b
            | 0x9d
            | 0x9e
            | 0x9f
            | 0xc2..=0xf4
    )
}

#[cfg(test)]
mod tests {
    use super::TerminalVisibleOutputFilter;

    fn visible_for_split(input: &[u8], split: usize) -> Vec<u8> {
        let mut filter = TerminalVisibleOutputFilter::default();
        let mut visible = Vec::new();
        filter.process_into(&input[..split], &mut visible);
        filter.process_into(&input[split..], &mut visible);
        visible
    }

    fn assert_visible_for_every_split(input: &[u8], expected: &[u8]) {
        for split in 0..=input.len() {
            assert_eq!(
                visible_for_split(input, split),
                expected,
                "input={input:?}, split={split}"
            );
        }
    }

    #[test]
    fn control_string_escape_boundaries_preserve_following_visible_bytes() {
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

        for (start, osc) in families {
            for ending in [
                b"\x1b\x1b\\after".as_slice(),
                b"\x1b\x18after".as_slice(),
                b"\x1b\x1aafter".as_slice(),
                b"\x1b\x9cafter".as_slice(),
                b"\x1b\xc2\x9cafter".as_slice(),
                b"\xc2\x9cafter".as_slice(),
                b"\xc2\x18after".as_slice(),
            ] {
                let input = [start, b"hidden", ending].concat();
                assert_visible_for_every_split(&input, b"after");
            }

            let input = if osc {
                [start, b"hidden\x1b\x07after"].concat()
            } else {
                [start, b"hidden\x1b\x07still-hidden\x1b\\after"].concat()
            };
            assert_visible_for_every_split(&input, b"after");
        }
    }

    #[test]
    fn fragmented_plain_ascii_uses_bulk_visible_output_appends() {
        let input = vec![b'x'; 1024 * 1024];
        let mut filter = TerminalVisibleOutputFilter::default();
        let mut visible = Vec::with_capacity(input.len());

        for chunk in input.chunks(8192) {
            filter.process_into(chunk, &mut visible);
        }

        assert_eq!(visible, input);
        assert_eq!(filter.bulk_appended_bytes, 1024 * 1024);
    }
}
