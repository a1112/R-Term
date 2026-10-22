use std::{path::PathBuf, time::Duration};

use rterm_functional_tests::{PtyFixtureDriver, PtyFixtureResult};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rterm-functional-fixture"))
}

#[test]
fn pty_driver_round_trips_unicode_and_answers_terminal_queries_without_sleep() {
    let result = PtyFixtureDriver::spawn(&fixture(), "echo-query", 80, 24, Duration::from_secs(5))
        .unwrap()
        .wait_for_output(b"fixture-ready")
        .unwrap()
        .write(b"R-Term \xe7\xbb\x88\xe7\xab\xaf\r\n")
        .unwrap()
        .finish()
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert!(
        String::from_utf8_lossy(&result.output).contains("fixture-echo:R-Term \u{7ec8}\u{7aef}")
    );
    assert!(result.terminal_query_responses >= 1);
    assert!(result.transport_write_observed());
    assert!(result.resources_zero());
}

#[test]
fn pty_driver_preserves_nonzero_exit_and_reaps_the_fixture() {
    let result = PtyFixtureDriver::spawn_with_args(
        &fixture(),
        ["exit-code", "7"],
        80,
        24,
        Duration::from_secs(5),
    )
    .unwrap()
    .finish()
    .unwrap();

    assert_eq!(result.exit_code, 7);
    assert!(result.resources_zero());
}

#[test]
fn disconnect_keeps_the_internal_writer_alive_until_master_close_begins() {
    let result = PtyFixtureDriver::spawn(&fixture(), "hold-open", 80, 24, Duration::from_secs(5))
        .unwrap()
        .wait_for_output(b"fixture-hold-open")
        .unwrap()
        .disconnect()
        .unwrap();

    assert!(result.resources_zero());
}

#[test]
fn fixture_cleanup_begins_master_close_before_dropping_the_external_writer() {
    let source = include_str!("../src/pty_driver.rs");
    let finish = source
        .split("fn finish_with_status(")
        .nth(1)
        .expect("fixture cleanup implementation")
        .split("fn drain_available(")
        .next()
        .unwrap();
    let begin_close = finish
        .find("let mut close = self.session.begin_master_close();")
        .expect("master close must begin");
    let drop_writer = finish
        .find("drop(self.writer.take());")
        .expect("external writer must be dropped");
    assert!(begin_close < drop_writer);
}

#[test]
fn normal_fixture_eof_is_not_classified_as_a_reader_failure() {
    let source = include_str!("../src/pty_driver.rs");
    assert!(source.contains("reader_eof: bool"));
    assert!(
        source.contains("Err(mpsc::RecvTimeoutError::Disconnected) if self.reader_eof => Ok(())")
    );
}

#[test]
fn fixture_catalog_covers_streaming_effect_and_lifecycle_modes() {
    for (mode, expected) in [
        ("osc-clipboard", "\u{1b}]52;c;"),
        ("synchronized-output", "\u{1b}[?2026h"),
    ] {
        let PtyFixtureResult { output, .. } =
            PtyFixtureDriver::spawn(&fixture(), mode, 80, 24, Duration::from_secs(5))
                .unwrap()
                .finish()
                .unwrap();
        assert!(String::from_utf8_lossy(&output).contains(expected));
    }
}

#[test]
fn stress_journey_exercises_backpressure_sync_nonzero_exit_and_cleanup() {
    let fixture = fixture();
    let high_output = PtyFixtureDriver::spawn_with_args(
        &fixture,
        ["high-output", "1048576"],
        80,
        24,
        Duration::from_secs(10),
    )
    .unwrap()
    .finish()
    .unwrap();
    assert!(
        high_output
            .output
            .split(|byte| *byte != b'X')
            .map(<[u8]>::len)
            .sum::<usize>()
            >= 1_048_576,
        "captured PTY stream length={} prefix={:?} suffix={:?}",
        high_output.output.len(),
        &high_output.output[..high_output.output.len().min(32)],
        &high_output.output[high_output.output.len().saturating_sub(32)..]
    );
    assert!(high_output.resources_zero());

    let synchronized = PtyFixtureDriver::spawn(
        &fixture,
        "synchronized-output",
        80,
        24,
        Duration::from_secs(5),
    )
    .unwrap()
    .finish()
    .unwrap();
    assert!(
        synchronized
            .output
            .windows(b"\x1b[?2026h".len())
            .any(|window| window == b"\x1b[?2026h")
    );
    assert!(
        synchronized
            .output
            .windows(b"\x1b[?2026l".len())
            .any(|window| window == b"\x1b[?2026l")
    );
    assert!(synchronized.resources_zero());

    let nonzero = PtyFixtureDriver::spawn_with_args(
        &fixture,
        ["exit-code", "37"],
        80,
        24,
        Duration::from_secs(5),
    )
    .unwrap()
    .finish()
    .unwrap();
    assert_eq!(nonzero.exit_code, 37);
    assert!(nonzero.resources_zero());

    let slow_read = PtyFixtureDriver::spawn_with_args(
        &fixture,
        ["slow-read", "1", "22"],
        80,
        24,
        Duration::from_secs(5),
    )
    .unwrap()
    .write(b"functional-slow-read\r\n")
    .unwrap()
    .finish()
    .unwrap();
    assert!(
        String::from_utf8_lossy(&slow_read.output).contains("functional-slow-read"),
        "{}",
        String::from_utf8_lossy(&slow_read.output)
    );
    assert!(slow_read.resources_zero());
}
