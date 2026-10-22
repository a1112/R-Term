use super::*;

fn assert_direct_ascii_matches_decoded(bytes: &[u8], size: TerminalSize) {
    let seed = Terminal::new(size);
    let mut direct = seed.clone();
    let mut decoded = seed;

    direct.advance_seqno();
    assert!(direct.try_feed_direct_ascii_at_current_seqno(bytes));
    decoded.advance_seqno();
    decoded.feed_decoded_at_current_seqno(bytes);

    // Scratch capacity and fixture identities are deliberately not terminal
    // semantics. Everything else, including pending parser state, damage,
    // scrollback, metadata, and screen state, must be byte-for-byte identical.
    direct.feed_scratch = TerminalFeedScratch::default();
    decoded.feed_scratch = TerminalFeedScratch::default();
    direct.fixture_trace = FixtureTraceIdentity::default();
    decoded.fixture_trace = FixtureTraceIdentity::default();
    assert_eq!(format!("{direct:#?}"), format!("{decoded:#?}"));
}

#[test]
fn plain_ascii_feed_does_not_grow_utf8_decode_storage() {
    let mut terminal = Terminal::new(TerminalSize::new(80, 24));
    let record = b"bench line 00000000 ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789\r\n";

    terminal.feed(record);

    assert_eq!(terminal.feed_storage_counters().growths(), 0);
}

#[test]
fn direct_ascii_feed_matches_decoded_printable_controls_and_scrollback() {
    let mut input = (0_u8..=0x7f)
        .filter(|byte| *byte != 0x1b)
        .collect::<Vec<_>>();
    for line in 0..96 {
        input.extend_from_slice(
            format!("bench line {line:08} ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789\r\n").as_bytes(),
        );
    }

    assert_direct_ascii_matches_decoded(&input, TerminalSize::new(37, 7));
}

#[test]
fn direct_ascii_feed_matches_decoded_deterministic_chunk_records() {
    let mut state = 0x6d2b_79f5_u32;
    let mut input = Vec::with_capacity(32 * 1024);
    for index in 0..32 * 1024 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let byte = match index % 97 {
            0 => b'\r',
            1 => b'\n',
            2 => b'\t',
            3 => 0x08,
            _ => 0x20 + ((state >> 24) as u8 % 0x5f),
        };
        input.push(byte);
    }

    for chunk in input.chunks(8192) {
        assert_direct_ascii_matches_decoded(chunk, TerminalSize::new(80, 24));
    }
}

#[test]
fn direct_ascii_feed_rejects_escape_unicode_nfc_and_pending_state() {
    let mut terminal = Terminal::new(TerminalSize::new(80, 24));
    assert!(!terminal.try_feed_direct_ascii_at_current_seqno(b"plain\x1b[31mred"));
    assert!(!terminal.try_feed_direct_ascii_at_current_seqno("café".as_bytes()));

    terminal.set_normalize_output_to_unicode_nfc(true);
    assert!(!terminal.try_feed_direct_ascii_at_current_seqno(b"plain"));
    terminal.set_normalize_output_to_unicode_nfc(false);

    terminal.pending_utf8.push(0xc3);
    assert!(!terminal.try_feed_direct_ascii_at_current_seqno(b"plain"));
    terminal.pending_utf8.clear();

    terminal.pending_control.push('\x1b');
    assert!(!terminal.try_feed_direct_ascii_at_current_seqno(b"plain"));
}

#[test]
fn ascii_starters_never_require_previous_grapheme_extension() {
    for byte in 0_u8..=0x7f {
        assert!(!may_extend_previous_grapheme("A", char::from(byte)));
    }
    assert!(may_extend_previous_grapheme("A", '\u{301}'));
    assert!(may_extend_previous_grapheme("A", '\u{200d}'));
    assert!(may_extend_previous_grapheme("\u{600}", 'A'));
}
