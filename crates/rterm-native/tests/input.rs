use rterm_native::input::{
    CanonicalizePastedNewlines, PendingPaneCommand, PendingPaneCommandQueue, encode_paste,
};
use rterm_runtime::SubmitResult;
use rterm_types::TerminalSize;

#[test]
fn paste_encoding_preserves_bracketed_input_and_canonicalizes_plain_input() {
    for line in include_str!("fixtures/paste_encoding.tsv").lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5, "malformed paste case: {line}");
        let name = fields[0];
        let bracketed = fields[1].parse::<bool>().expect("boolean bracketed flag");
        let canonicalization = match fields[2] {
            "none" => CanonicalizePastedNewlines::None,
            "cr" => CanonicalizePastedNewlines::CarriageReturn,
            "crlf" => CanonicalizePastedNewlines::CarriageReturnAndLineFeed,
            value => panic!("unknown canonicalization {value} in case {name}"),
        };
        let input = decode_hex(fields[3]);
        let expected = decode_hex(fields[4]);
        let input = String::from_utf8(input).expect("fixture input is UTF-8");
        assert_eq!(
            encode_paste(&input, bracketed, canonicalization),
            expected,
            "paste fixture {name}"
        );
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "hex fixture has an odd length");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("hex digits are ASCII");
            u8::from_str_radix(pair, 16).expect("valid hex byte")
        })
        .collect()
}

#[test]
fn pending_input_retries_in_order_after_runtime_backpressure() {
    let mut pending = PendingPaneCommandQueue::new();
    pending
        .submit_input(b"first", |_| SubmitResult::Backpressured {
            retry_after: std::time::Duration::from_millis(1),
        })
        .expect("queue first input");
    pending
        .submit_input(b"second", |_| SubmitResult::Accepted)
        .expect("queue behind pending input");

    let mut delivered = Vec::new();
    pending
        .flush(|command| {
            let PendingPaneCommand::Input(bytes) = command else {
                panic!("unexpected resize")
            };
            delivered.push(bytes);
            SubmitResult::Accepted
        })
        .expect("flush pending input");

    assert_eq!(delivered, [b"first".to_vec(), b"second".to_vec()]);
    assert!(pending.is_empty());
}

#[test]
fn pending_resizes_coalesce_without_reordering_input() {
    let mut pending = PendingPaneCommandQueue::new();
    pending
        .submit_input(b"input", |_| SubmitResult::Backpressured {
            retry_after: std::time::Duration::from_millis(1),
        })
        .unwrap();
    pending
        .submit_resize(TerminalSize::new(80, 24), |_| SubmitResult::Accepted)
        .unwrap();
    pending
        .submit_resize(TerminalSize::new(120, 40), |_| SubmitResult::Accepted)
        .unwrap();

    let mut delivered = Vec::new();
    pending
        .flush(|command| {
            delivered.push(match command {
                PendingPaneCommand::Input(bytes) => format!("input:{}", bytes.len()),
                PendingPaneCommand::Resize(size) => {
                    format!("resize:{}x{}", size.columns, size.rows)
                }
            });
            SubmitResult::Accepted
        })
        .unwrap();
    assert_eq!(delivered, ["input:5", "resize:120x40"]);
}

#[test]
fn oversized_input_is_chunked_without_spinning_the_caller() {
    let mut pending = PendingPaneCommandQueue::new();
    let input = vec![b'x'; PendingPaneCommandQueue::MAX_INPUT_CHUNK_BYTES * 2 + 17];
    let mut attempts = 0;
    pending
        .submit_input(&input, |_| {
            attempts += 1;
            SubmitResult::Backpressured {
                retry_after: std::time::Duration::from_millis(1),
            }
        })
        .expect("oversized input is retained for asynchronous retry");

    assert_eq!(attempts, 1, "the UI thread must never spin or sleep");
    let mut delivered = Vec::new();
    pending
        .flush(|command| {
            let PendingPaneCommand::Input(bytes) = command else {
                panic!("unexpected resize")
            };
            assert!(bytes.len() <= PendingPaneCommandQueue::MAX_INPUT_CHUNK_BYTES);
            delivered.extend(bytes);
            SubmitResult::Accepted
        })
        .expect("asynchronous poll drains every chunk");
    assert_eq!(delivered, input);
    assert!(pending.is_empty());
}
