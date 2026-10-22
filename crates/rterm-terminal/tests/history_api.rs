use rterm_terminal::{ScrollbackLine, Terminal};
use rterm_types::TerminalSize;

#[test]
fn public_history_api_is_logically_indexed_and_iterable() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"aa\r\nbb\r\ncc");
    let history = terminal.scrollback();

    assert_eq!(history.len(), 1);
    let indexed: &ScrollbackLine = &history[0];
    assert!(std::ptr::eq(history.get(0).unwrap(), indexed));
    assert_eq!(history.iter().count(), history.len());
    assert_eq!(history.range(..1).count(), 1);
    assert_eq!(history.into_iter().count(), history.len());
    assert_eq!(history.last(), history.get(history.len() - 1));
}
