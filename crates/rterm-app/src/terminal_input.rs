#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKey {
    Text(char),
    Control(char),
    Enter,
    Backspace,
    Tab,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Delete,
    Insert,
    PageUp,
    PageDown,
    BackTab,
    Menu,
    Function(u8),
}

pub fn encode_terminal_key(key: TerminalKey) -> Option<Vec<u8>> {
    match key {
        TerminalKey::Text(character) => {
            let mut bytes = Vec::new();
            let mut buffer = [0; 4];
            bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
            Some(bytes)
        }
        TerminalKey::Control(character) => encode_control_char(character),
        TerminalKey::Enter => Some(b"\r".to_vec()),
        TerminalKey::Backspace => Some(vec![0x7f]),
        TerminalKey::Tab => Some(b"\t".to_vec()),
        TerminalKey::Escape => Some(vec![0x1b]),
        TerminalKey::Left => Some(b"\x1b[D".to_vec()),
        TerminalKey::Right => Some(b"\x1b[C".to_vec()),
        TerminalKey::Up => Some(b"\x1b[A".to_vec()),
        TerminalKey::Down => Some(b"\x1b[B".to_vec()),
        TerminalKey::Home => Some(b"\x1b[H".to_vec()),
        TerminalKey::End => Some(b"\x1b[F".to_vec()),
        TerminalKey::Delete => Some(b"\x1b[3~".to_vec()),
        TerminalKey::Insert => Some(b"\x1b[2~".to_vec()),
        TerminalKey::PageUp => Some(b"\x1b[5~".to_vec()),
        TerminalKey::PageDown => Some(b"\x1b[6~".to_vec()),
        TerminalKey::BackTab => Some(b"\x1b[Z".to_vec()),
        TerminalKey::Menu => Some(b"\x1b[29~".to_vec()),
        TerminalKey::Function(key) => encode_function_key(key),
    }
}

fn encode_control_char(character: char) -> Option<Vec<u8>> {
    let lower = character.to_ascii_lowercase();

    let byte = match lower {
        ' ' | '@' | '2' => 0,
        'a'..='z' => lower as u8 - b'a' + 1,
        '0' => b'0',
        '1' => b'1',
        '3' | '[' => 0x1b,
        '4' | '\\' => 0x1c,
        '5' | ']' => 0x1d,
        '6' | '^' | '~' => 0x1e,
        '7' | '/' | '_' => 0x1f,
        '8' | '?' => 0x7f,
        '9' => b'9',
        _ => return None,
    };

    Some(vec![byte])
}

fn encode_function_key(key: u8) -> Option<Vec<u8>> {
    let bytes = match key {
        1 => b"\x1bOP".as_slice(),
        2 => b"\x1bOQ".as_slice(),
        3 => b"\x1bOR".as_slice(),
        4 => b"\x1bOS".as_slice(),
        5 => b"\x1b[15~".as_slice(),
        6 => b"\x1b[17~".as_slice(),
        7 => b"\x1b[18~".as_slice(),
        8 => b"\x1b[19~".as_slice(),
        9 => b"\x1b[20~".as_slice(),
        10 => b"\x1b[21~".as_slice(),
        11 => b"\x1b[23~".as_slice(),
        12 => b"\x1b[24~".as_slice(),
        _ => return None,
    };

    Some(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::{TerminalKey, encode_terminal_key};

    #[test]
    fn encodes_text_as_utf8() {
        assert_eq!(
            encode_terminal_key(TerminalKey::Text('中')).unwrap(),
            "中".as_bytes()
        );
    }

    #[test]
    fn encodes_control_c() {
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('c')).unwrap(),
            vec![3]
        );
    }

    #[test]
    fn encodes_navigation_keys() {
        assert_eq!(encode_terminal_key(TerminalKey::Enter).unwrap(), b"\r");
        assert_eq!(encode_terminal_key(TerminalKey::Backspace).unwrap(), [0x7f]);
        assert_eq!(encode_terminal_key(TerminalKey::Up).unwrap(), b"\x1b[A");
        assert_eq!(
            encode_terminal_key(TerminalKey::Delete).unwrap(),
            b"\x1b[3~"
        );
    }

    #[test]
    fn encodes_extended_control_keys() {
        assert_eq!(
            encode_terminal_key(TerminalKey::Control(' ')).unwrap(),
            vec![0]
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('0')).unwrap(),
            b"0"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('1')).unwrap(),
            b"1"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('2')).unwrap(),
            vec![0]
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('3')).unwrap(),
            vec![0x1b]
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('8')).unwrap(),
            vec![0x7f]
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('9')).unwrap(),
            b"9"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('/')).unwrap(),
            vec![0x1f]
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('[')).unwrap(),
            vec![0x1b]
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('\\')).unwrap(),
            vec![0x1c]
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Control('~')).unwrap(),
            vec![0x1e]
        );
    }

    #[test]
    fn encodes_backtab_and_function_keys() {
        assert_eq!(
            encode_terminal_key(TerminalKey::BackTab).unwrap(),
            b"\x1b[Z"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Function(1)).unwrap(),
            b"\x1bOP"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Function(12)).unwrap(),
            b"\x1b[24~"
        );
    }
}
