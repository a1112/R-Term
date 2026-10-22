use std::io::{self, Read, Write};
use std::thread;
use std::time::{Duration, Instant};

use rterm_runtime::testing::{
    ControlCall, ExitAction, ReadAction, ScriptedTransport, VirtualClock, VirtualClockAdvanceError,
    WriteAction,
};
use rterm_runtime::{
    Clock, SessionControl, SessionExit, SessionInterrupt, SessionTransport, SystemClock,
};
use rterm_types::TerminalSize;

#[test]
fn system_clock_observes_a_real_instant() {
    let before = Instant::now();
    let observed = SystemClock.now();
    let after = Instant::now();

    assert!(observed >= before);
    assert!(observed <= after);
}

#[test]
fn virtual_clock_clones_share_explicit_monotonic_time() {
    let start = Instant::now();
    let clock = VirtualClock::new(start);
    let observer = clock.clone();

    assert_eq!(clock.now(), start);
    let advanced = clock
        .advance(Duration::from_millis(125))
        .expect("representable advance");

    assert_eq!(advanced, start + Duration::from_millis(125));
    assert_eq!(observer.now(), advanced);
    assert_eq!(clock.advance(Duration::ZERO), Ok(advanced));
}

#[test]
fn virtual_clock_rejects_overflow_without_changing_time() {
    let start = Instant::now();
    let clock = VirtualClock::new(start);

    let error = clock
        .advance(Duration::MAX)
        .expect_err("an unrepresentable instant must not wrap or panic");

    assert_eq!(error, VirtualClockAdvanceError::Overflow);
    assert_eq!(clock.now(), start);
}

#[test]
fn scripted_transport_preserves_partial_io_errors_and_control_calls() -> io::Result<()> {
    let exit = SessionExit {
        status: Some(u32::MAX),
        signal: None,
    };
    let (transport, driver) = ScriptedTransport::new(
        [
            ReadAction::bytes(b"abcdef"),
            ReadAction::error(io::ErrorKind::ConnectionReset),
            ReadAction::Eof,
        ],
        [
            WriteAction::accept(2),
            WriteAction::accept(usize::MAX),
            WriteAction::error(io::ErrorKind::BrokenPipe),
        ],
        [ExitAction::Pending, ExitAction::Exited(exit.clone())],
    );
    let mut parts = transport.split();

    let mut first = [0; 4];
    assert_eq!(parts.reader.read(&mut first)?, 4);
    assert_eq!(&first, b"abcd");
    let mut second = [0; 4];
    assert_eq!(parts.reader.read(&mut second)?, 2);
    assert_eq!(&second[..2], b"ef");
    assert_eq!(
        parts
            .reader
            .read(&mut second)
            .expect_err("scripted read error")
            .kind(),
        io::ErrorKind::ConnectionReset
    );
    assert_eq!(parts.reader.read(&mut second)?, 0);

    assert_eq!(parts.writer.write(b"hello")?, 2);
    assert_eq!(parts.writer.write(b"llo")?, 3);
    assert_eq!(
        parts
            .writer
            .write(b"!")
            .expect_err("scripted write error")
            .kind(),
        io::ErrorKind::BrokenPipe
    );
    parts.control.resize(TerminalSize::new(132, 43))?;
    assert_eq!(parts.control.poll_exit()?, None);
    assert_eq!(parts.control.poll_exit()?, Some(exit));
    parts.control.begin_close()?;
    parts.control.begin_close()?;

    assert_eq!(driver.accepted_writes(), b"hello");
    let log = driver.control_log();
    assert_eq!(log.resizes, vec![TerminalSize::new(132, 43)]);
    assert_eq!(log.poll_exit_calls, 2);
    assert_eq!(log.begin_close_calls, 2);
    assert_eq!(
        log.calls,
        vec![
            ControlCall::Resize(TerminalSize::new(132, 43)),
            ControlCall::PollExit,
            ControlCall::PollExit,
            ControlCall::BeginClose,
            ControlCall::BeginClose,
        ]
    );
    Ok(())
}

#[test]
fn one_interrupt_releases_blocked_reader_and_writer_without_sleep() {
    let (transport, driver) = ScriptedTransport::new(
        [ReadAction::Block],
        [WriteAction::Block],
        [ExitAction::Pending],
    );
    let parts = transport.split();
    let interrupt = parts.interrupt.clone();

    let reader = thread::spawn(move || {
        let mut reader = parts.reader;
        let mut byte = [0];
        reader.read(&mut byte).map(|_| ())
    });
    let writer = thread::spawn(move || {
        let mut writer = parts.writer;
        writer.write_all(b"blocked")
    });

    driver.wait_until_reader_blocked();
    driver.wait_until_writer_blocked();
    interrupt.interrupt().expect("first interrupt");
    interrupt.interrupt().expect("idempotent interrupt");

    assert_eq!(
        reader
            .join()
            .expect("reader thread")
            .expect_err("reader must be released")
            .kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(
        writer
            .join()
            .expect("writer thread")
            .expect_err("writer must be released")
            .kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(driver.interrupt_calls(), 2);
}

#[test]
fn blocked_reader_can_be_advanced_by_the_script_driver() {
    let (transport, driver) = ScriptedTransport::new(
        [
            ReadAction::Block,
            ReadAction::error(io::ErrorKind::ConnectionAborted),
        ],
        [WriteAction::accept(usize::MAX)],
        [ExitAction::Pending],
    );
    let parts = transport.split();
    let reader = thread::spawn(move || {
        let mut reader = parts.reader;
        let mut bytes = [0; 3];
        let count = reader.read(&mut bytes)?;
        let following_error = reader
            .read(&mut [0; 1])
            .expect_err("queued action must follow injected bytes")
            .kind();
        io::Result::Ok((count, bytes, following_error))
    });

    driver.wait_until_reader_blocked();
    driver.push_read(ReadAction::bytes(b"abc"));

    let (count, bytes, following_error) = reader
        .join()
        .expect("reader thread")
        .expect("scripted bytes");
    assert_eq!(count, 3);
    assert_eq!(&bytes, b"abc");
    assert_eq!(following_error, io::ErrorKind::ConnectionAborted);
}

#[test]
fn blocked_reader_can_be_released_to_eof_by_the_script_driver() {
    let (transport, driver) = ScriptedTransport::new(
        [ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
        [ExitAction::Pending],
    );
    let parts = transport.split();
    let reader = thread::spawn(move || {
        let mut reader = parts.reader;
        reader.read(&mut [0; 1])
    });

    driver.wait_until_reader_blocked();
    driver.push_read(ReadAction::Eof);

    assert_eq!(reader.join().expect("reader thread").expect("EOF"), 0);
}

#[test]
fn control_errors_are_one_shot_and_calls_remain_ordered() -> io::Result<()> {
    let (transport, driver) = ScriptedTransport::new(
        [ReadAction::Eof],
        [WriteAction::accept(usize::MAX)],
        [ExitAction::error(io::ErrorKind::TimedOut)],
    );
    driver.push_resize_error(io::ErrorKind::PermissionDenied);
    driver.push_close_error(io::ErrorKind::ConnectionAborted);
    let mut control = transport.split().control;
    let size = TerminalSize::new(80, 24);

    assert_eq!(
        control.resize(size).expect_err("resize error").kind(),
        io::ErrorKind::PermissionDenied
    );
    control.resize(size)?;
    assert_eq!(
        control.poll_exit().expect_err("poll error").kind(),
        io::ErrorKind::TimedOut
    );
    assert_eq!(control.poll_exit()?, None);
    assert_eq!(
        control.begin_close().expect_err("close error").kind(),
        io::ErrorKind::ConnectionAborted
    );
    control.begin_close()?;

    assert_eq!(
        driver.control_log().calls,
        vec![
            ControlCall::Resize(size),
            ControlCall::Resize(size),
            ControlCall::PollExit,
            ControlCall::PollExit,
            ControlCall::BeginClose,
            ControlCall::BeginClose,
        ]
    );
    Ok(())
}

#[test]
fn interruption_rejects_all_later_writes_without_accepting_bytes() {
    let (transport, driver) = ScriptedTransport::new(
        [ReadAction::Eof],
        [WriteAction::accept(usize::MAX)],
        [ExitAction::Pending],
    );
    let mut parts = transport.split();
    parts.interrupt.interrupt().expect("interrupt");

    assert_eq!(
        parts
            .writer
            .write(b"must not be accepted")
            .expect_err("interrupted writer")
            .kind(),
        io::ErrorKind::BrokenPipe
    );
    assert!(driver.accepted_writes().is_empty());
}
