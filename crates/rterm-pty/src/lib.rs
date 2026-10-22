use std::{
    collections::VecDeque,
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use portable_pty::{
    ChildKiller, CommandBuilder, ExitStatus as PortableExitStatus, MasterPty,
    PtySize as PortablePtySize, native_pty_system,
};

#[cfg(feature = "runtime-adapter")]
mod runtime_adapter;

#[cfg(feature = "runtime-adapter")]
pub use runtime_adapter::{LocalPtyControl, LocalPtyInterrupt, LocalPtyTransport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyBackend {
    WindowsConpty,
    UnixPty,
}

impl PtyBackend {
    #[must_use]
    pub fn current_platform() -> Self {
        if cfg!(windows) {
            Self::WindowsConpty
        } else {
            Self::UnixPty
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
            mpsc,
        },
        thread,
        time::{Duration, Instant},
    };

    use super::{
        CaptureProgress, CaptureReapJob, CaptureThreadJoin, ChildKiller,
        CursorPositionQueryScanner, DefaultShellPlatform, FORCE_PROCESS_DEFER,
        FORCE_SESSION_DROP_DEFER, PtyBackend, PtyCloseIo, PtyCommand, PtyError, PtyExitStatus,
        PtyMasterClose, PtyMasterCloseStatus, PtyReaderProxy, PtySession, PtySessionInterrupt,
        PtySize, PtyWriterProxy, STREAM_ACQUISITION_FAULT, capture_cleanup_panic_count,
        capture_reaper_deferred_count, capture_reaper_error_count,
        capture_reaper_last_process_ownership, capture_reaper_retained_count,
        default_shell_program_from, defer_capture_job, join_capture_thread_before,
        observe_reaped_master_close, pending_capture_cleanup_count, pending_master_close_count,
        take_capture_reaper_errors, terminate_child_before,
    };

    static CAPTURE_REAPER_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct SharedTestWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedTestWriter {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct FailingTestWriter;

    impl Write for FailingTestWriter {
        fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected PTY write failure"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("injected PTY flush failure"))
        }
    }

    struct InterruptedThenCursor {
        interrupted: bool,
    }

    #[derive(Debug, Clone)]
    struct FailOnceKiller {
        calls: Arc<AtomicUsize>,
    }

    impl ChildKiller for FailOnceKiller {
        fn kill(&mut self) -> std::io::Result<()> {
            if self.calls.fetch_add(1, Ordering::AcqRel) == 0 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected first kill failure",
                ))
            } else {
                Ok(())
            }
        }

        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn failed_interrupt_can_be_retried_without_repeating_a_successful_kill() {
        let calls = Arc::new(AtomicUsize::new(0));
        let close_io = PtyCloseIo::new(Box::new(std::io::sink()));
        let interrupt = PtySessionInterrupt::new(
            Box::new(FailOnceKiller {
                calls: Arc::clone(&calls),
            }),
            close_io,
        );

        let first = interrupt.interrupt().expect_err("first kill must fail");
        assert_eq!(first.kind(), std::io::ErrorKind::PermissionDenied);
        interrupt.interrupt().expect("retry kill");
        interrupt
            .interrupt()
            .expect("successful kill is idempotent");
        assert_eq!(calls.load(Ordering::Acquire), 2);
    }

    impl Read for InterruptedThenCursor {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
            }
            let query = b"\x1b[6n";
            buffer[..query.len()].copy_from_slice(query);
            Ok(query.len())
        }
    }

    #[test]
    fn normal_cursor_query_is_not_replayed_when_close_begins() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let close_io = PtyCloseIo::new(Box::new(SharedTestWriter(Arc::clone(&output))));
        let mut reader = PtyReaderProxy {
            reader: Box::new(std::io::Cursor::new(b"\x1b[6n".to_vec())),
            close_io: Arc::clone(&close_io),
            scanner: CursorPositionQueryScanner::default(),
        };
        let mut buffer = [0_u8; 16];
        let mut writer = PtyWriterProxy {
            close_io: Arc::clone(&close_io),
        };

        assert_eq!(reader.read(&mut buffer).unwrap(), 4);
        writer.write_all(b"\x1b[1;").unwrap();
        writer.write_all(b"1R").unwrap();
        close_io.begin_close(true);

        assert_eq!(
            *output
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            b"\x1b[1;1R"
        );
    }

    #[test]
    fn observed_cursor_query_without_external_reply_is_answered_at_close() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let close_io = PtyCloseIo::new(Box::new(SharedTestWriter(Arc::clone(&output))));
        let mut reader = PtyReaderProxy {
            reader: Box::new(std::io::Cursor::new(b"\x1b[6n".to_vec())),
            close_io: Arc::clone(&close_io),
            scanner: CursorPositionQueryScanner::default(),
        };
        let mut buffer = [0_u8; 16];

        assert_eq!(reader.read(&mut buffer).unwrap(), 4);
        close_io.begin_close(true);

        assert_eq!(
            *output
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            b"\x1b[1;1R"
        );
    }

    #[test]
    fn live_child_close_does_not_inject_a_cursor_reply() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let close_io = PtyCloseIo::new(Box::new(SharedTestWriter(Arc::clone(&output))));

        close_io.begin_close(false);

        assert!(
            output
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty()
        );
    }

    #[test]
    fn speculative_close_reply_failure_is_not_a_cleanup_failure() {
        let close_io = PtyCloseIo::new(Box::new(FailingTestWriter));

        close_io.begin_close(true);

        assert!(close_io.error().is_none());
    }

    #[test]
    fn observed_query_reply_failure_remains_a_cleanup_failure() {
        let close_io = PtyCloseIo::new(Box::new(FailingTestWriter));
        close_io.observe_reader_queries(1);

        close_io.begin_close(true);

        assert!(close_io.error().is_some());
    }

    #[test]
    fn closing_cursor_query_is_answered_once_across_chunks() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let close_io = PtyCloseIo::new(Box::new(SharedTestWriter(Arc::clone(&output))));
        close_io.begin_close(true);
        let mut reader = PtyReaderProxy {
            reader: Box::new(std::io::Cursor::new(b"\x1b[6n".to_vec())),
            close_io: Arc::clone(&close_io),
            scanner: CursorPositionQueryScanner::default(),
        };
        let mut first = [0_u8; 2];
        let mut second = [0_u8; 2];

        assert_eq!(reader.read(&mut first).unwrap(), 2);
        assert_eq!(reader.read(&mut second).unwrap(), 2);

        assert_eq!(
            *output
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            b"\x1b[1;1R"
        );
    }

    #[test]
    fn interrupted_close_read_keeps_cursor_response_writer_open() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let close_io = PtyCloseIo::new(Box::new(SharedTestWriter(Arc::clone(&output))));
        close_io.begin_close(true);
        let mut reader = PtyReaderProxy {
            reader: Box::new(InterruptedThenCursor { interrupted: false }),
            close_io,
            scanner: CursorPositionQueryScanner::default(),
        };
        let mut buffer = [0_u8; 16];

        assert_eq!(
            reader.read(&mut buffer).unwrap_err().kind(),
            std::io::ErrorKind::Interrupted
        );
        assert_eq!(reader.read(&mut buffer).unwrap(), 4);

        assert_eq!(
            *output
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            b"\x1b[1;1R"
        );
    }

    #[test]
    fn pty_exit_status_preserves_signal() {
        let status = PtyExitStatus::from_signal("TERM");

        assert_eq!(status.exit_code(), 1);
        assert_eq!(status.signal(), Some("TERM"));
        assert!(!status.success());
    }

    #[test]
    fn selects_a_platform_backend() {
        let backend = PtyBackend::current_platform();

        assert!(matches!(
            backend,
            PtyBackend::WindowsConpty | PtyBackend::UnixPty
        ));
    }

    #[test]
    fn default_shell_uses_current_platform_command() {
        let command = PtyCommand::default_shell();

        assert!(!command.program().is_empty());
    }

    #[test]
    fn macos_default_shell_falls_back_to_zsh() {
        assert_eq!(
            default_shell_program_from(DefaultShellPlatform::Macos, None, None),
            "/bin/zsh"
        );
    }

    #[test]
    fn configured_shell_wins_on_macos() {
        assert_eq!(
            default_shell_program_from(
                DefaultShellPlatform::Macos,
                None,
                Some(std::ffi::OsStr::new("/opt/homebrew/bin/fish")),
            ),
            "/opt/homebrew/bin/fish"
        );
    }

    #[test]
    fn empty_shell_environment_uses_platform_fallback() {
        assert_eq!(
            default_shell_program_from(
                DefaultShellPlatform::Unix,
                None,
                Some(std::ffi::OsStr::new("")),
            ),
            "/bin/sh"
        );
        assert_eq!(
            default_shell_program_from(
                DefaultShellPlatform::Windows,
                Some(std::ffi::OsStr::new("")),
                None,
            ),
            "cmd.exe"
        );
    }

    #[test]
    fn pty_command_sets_default_terminal_environment() {
        let command = PtyCommand::new("shell");

        assert_eq!(command.env_value("TERM"), Some("xterm-256color"));
        assert_eq!(command.env_value("COLORTERM"), Some("truecolor"));
    }

    #[test]
    fn explicit_pty_environment_overrides_terminal_defaults() {
        let command = PtyCommand::new("shell").with_env("TERM", "vt100");

        assert_eq!(command.env_value("TERM"), Some("vt100"));
        assert_eq!(command.env_value("COLORTERM"), Some("truecolor"));
    }

    #[test]
    fn pty_command_exposes_configured_current_working_dir() {
        let command = PtyCommand::new("shell").with_cwd("/tmp/project");

        assert_eq!(
            command
                .cwd()
                .map(|path| path.to_string_lossy().into_owned()),
            Some("/tmp/project".to_owned())
        );
    }

    #[test]
    fn pty_command_builder_receives_terminal_environment() {
        let builder = PtyCommand::new("shell")
            .with_env("TERM", "vt100")
            .to_builder();

        assert_eq!(
            builder.get_env("TERM").and_then(|value| value.to_str()),
            Some("vt100")
        );
        assert_eq!(
            builder
                .get_env("COLORTERM")
                .and_then(|value| value.to_str()),
            Some("truecolor")
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_prefers_cmd_shim_over_extensionless_script() {
        let root = temporary_windows_command_dir("cmd-shim");
        std::fs::write(root.join("claude"), b"#!/bin/sh\necho unix\n").unwrap();
        std::fs::write(root.join("claude.cmd"), b"@echo off\necho windows\n").unwrap();

        let command = PtyCommand::new("claude")
            .with_args(["--version", "value with spaces"])
            .with_env("PATH", root.to_string_lossy());
        let argv = command.to_builder().get_argv().clone();

        assert_eq!(
            std::path::Path::new(&argv[0])
                .file_name()
                .and_then(|name| name.to_str()),
            Some("cmd.exe")
        );
        assert_eq!(argv[1].to_string_lossy(), "/D");
        assert_eq!(argv[2].to_string_lossy(), "/V:OFF");
        assert_eq!(argv[3].to_string_lossy(), "/S");
        assert_eq!(argv[4].to_string_lossy(), "/C");
        assert_eq!(argv[5].to_string_lossy(), "call");
        assert_eq!(
            argv[6].to_string_lossy(),
            root.join("claude.cmd").display().to_string()
        );
        assert_eq!(argv[7].to_string_lossy(), "--version");
        assert_eq!(argv[8].to_string_lossy(), "value with spaces");

        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn windows_launches_cmd_shim_through_conpty() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = temporary_windows_command_dir("cmd-shim-spawn");
        std::fs::write(root.join("rterm-shim"), b"#!/bin/sh\necho unix\n").unwrap();
        std::fs::write(
            root.join("rterm-shim.cmd"),
            b"@echo off\r\necho rterm-windows-shim-ok\r\necho [%~1]\r\necho [%~2]\r\n\
              echo [%~3]\r\necho [%~4]\r\n",
        )
        .unwrap();
        let path = std::env::join_paths([
            root.as_os_str(),
            std::env::var_os("PATH")
                .as_deref()
                .unwrap_or_else(|| std::ffi::OsStr::new("")),
        ])
        .unwrap();

        let command = PtyCommand::new("rterm-shim")
            .with_args([
                "value with spaces",
                "'single quote'",
                r"\\server\share",
                "-leading",
            ])
            .with_env("PATH", path.to_string_lossy());
        let output = PtySession::capture_output(
            &command,
            PtySize::try_new(80, 24).unwrap(),
            Duration::from_secs(5),
        )
        .unwrap();

        assert!(
            String::from_utf8_lossy(&output).contains("rterm-windows-shim-ok"),
            "captured PTY output: {:?}",
            String::from_utf8_lossy(&output)
        );
        let output = String::from_utf8_lossy(&output);
        for expected in [
            "[value with spaces]",
            "['single quote']",
            r"[\\server\share]",
            "[-leading]",
        ] {
            assert!(
                output.contains(expected),
                "missing {expected:?} in captured PTY output: {output:?}"
            );
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn windows_wraps_explicit_powershell_script() {
        let root = temporary_windows_command_dir("powershell-script");
        let script = root.join("tool.ps1");
        std::fs::write(&script, b"Write-Output ok\n").unwrap();

        let command = PtyCommand::new(script.to_string_lossy().to_string())
            .with_arg("--version")
            .to_builder();
        let argv = command.get_argv();

        assert_eq!(
            std::path::Path::new(&argv[0])
                .file_name()
                .and_then(|name| name.to_str()),
            Some("powershell.exe")
        );
        assert_eq!(argv[1].to_string_lossy(), "-NoLogo");
        assert_eq!(argv[2].to_string_lossy(), "-NoProfile");
        assert_eq!(argv[3].to_string_lossy(), "-NonInteractive");
        assert_eq!(argv[4].to_string_lossy(), "-File");
        assert_eq!(argv[5].to_string_lossy(), script.to_string_lossy());
        assert_eq!(argv[6].to_string_lossy(), "--version");

        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn windows_powershell_script_preserves_metacharacter_arguments() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = temporary_windows_command_dir("powershell-script-args");
        let script = root.join("argv.ps1");
        std::fs::write(
            &script,
            b"$args | ForEach-Object { $hex = \
              [BitConverter]::ToString([Text.Encoding]::UTF8.GetBytes([string]$_)); \
              Write-Output (\"RTERM_ARG:{0}:END\" -f $hex) }\r\n",
        )
        .unwrap();
        let arguments = [
            "%PATH%",
            "!value!",
            "a&b",
            "a|b",
            "a^b",
            "\"quoted\"",
            "'single quote'",
            "value with spaces",
            "Unicode-终端",
            r"\\server\share",
            "-leading",
            "line1\nline2",
        ];
        let command = PtyCommand::new(script.to_string_lossy())
            .with_args(arguments)
            .without_env("TERM");

        let output = PtySession::capture_output(
            &command,
            PtySize::try_new(120, 30).unwrap(),
            Duration::from_secs(10),
        )
        .unwrap();
        let output = String::from_utf8_lossy(&output);

        for argument in arguments {
            let expected = argument
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join("-");
            let marker = format!("RTERM_ARG:{expected}:END");
            assert!(
                output.contains(&marker),
                "PowerShell lost argument {argument:?}; expected {expected:?} in {output:?}"
            );
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn windows_rejects_cmd_shim_arguments_with_shell_metacharacters() {
        let root = temporary_windows_command_dir("cmd-shim-unsafe-args");
        std::fs::write(root.join("tool.cmd"), b"@echo off\r\n").unwrap();

        for argument in [
            "%PATH%",
            "!value!",
            "a&b",
            "a|b",
            "a^b",
            "<input",
            ">output",
            "(group)",
            "\"quoted\"",
            "line1\nline2",
        ] {
            let error = PtyCommand::new(root.join("tool.cmd").to_string_lossy())
                .with_arg(argument)
                .validate()
                .unwrap_err();

            assert!(
                matches!(error, PtyError::InvalidCommand(message) if message.contains("argument 0")),
                "unsafe cmd.exe argument must fail closed: {argument:?}"
            );
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn windows_accepts_cmd_shim_arguments_without_shell_metacharacters() {
        let root = temporary_windows_command_dir("cmd-shim-safe-args");
        std::fs::write(root.join("tool.cmd"), b"@echo off\r\n").unwrap();
        let command = PtyCommand::new(root.join("tool.cmd").to_string_lossy()).with_args([
            "value with spaces",
            "'single quote'",
            "Unicode-终端",
            r"\\server\share",
            "-leading",
        ]);

        command.validate().unwrap();

        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn windows_keeps_native_executable_commands_direct() {
        let command = PtyCommand::new("tool.exe")
            .with_arg("--version")
            .to_builder();
        let argv = command.get_argv();

        assert_eq!(argv[0].to_string_lossy(), "tool.exe");
        assert_eq!(argv[1].to_string_lossy(), "--version");
    }

    #[cfg(windows)]
    fn temporary_windows_command_dir(label: &str) -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("rterm-pty-{label}-{}-{suffix}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn command_validation_rejects_empty_program() {
        let command = PtyCommand::new("");

        assert!(command.validate().is_err());
    }

    #[test]
    fn pty_size_rejects_zero_dimensions() {
        assert!(PtySize::try_new(0, 24).is_err());
        assert!(PtySize::try_new(80, 0).is_err());
    }

    #[test]
    fn pty_size_accepts_columns_and_rows() {
        let size = PtySize::try_new(80, 24).unwrap();

        assert_eq!(size.columns(), 80);
        assert_eq!(size.rows(), 24);
    }

    #[test]
    fn local_pty_captures_process_output() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let marker = "rterm-pty-capture-marker";
        let command = PtyCommand::platform_echo(marker);
        let output = PtySession::capture_output(
            &command,
            PtySize::try_new(80, 24).unwrap(),
            Duration::from_secs(5),
        )
        .unwrap();

        let output = String::from_utf8_lossy(&output);

        assert!(output.contains(marker), "captured PTY output: {output:?}");
    }

    #[test]
    fn local_pty_exposes_owned_reader_and_writer() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let command = deterministic_exit_command(0);
        let mut session = PtySession::spawn(&command, PtySize::try_new(80, 24).unwrap()).unwrap();

        let _reader = session.take_reader().unwrap();
        let _writer = session.take_writer().unwrap();
        let _ = session.terminate(Duration::from_secs(2));
    }

    #[test]
    fn local_pty_exposes_child_process_metadata() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let command = deterministic_exit_command(0);
        let mut session = PtySession::spawn(&command, PtySize::try_new(80, 24).unwrap()).unwrap();

        assert!(
            session
                .process_id()
                .is_some_and(|process_id| process_id > 0),
            "PTY session should expose a positive child process id"
        );
        #[cfg(unix)]
        assert!(
            session.tty_name().is_some(),
            "Unix PTY session should expose its tty name"
        );
        #[cfg(windows)]
        assert_eq!(session.tty_name(), None);

        let _ = session.terminate(Duration::from_secs(2));
    }

    #[test]
    fn local_pty_supports_interactive_shell_roundtrip() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        default_shell_smoke_with_phase_deadlines();
    }

    #[test]
    fn local_pty_reports_child_exit_status() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let command = if cfg!(windows) {
            PtyCommand::new("cmd.exe").with_args(["/C", "exit", "7"])
        } else {
            PtyCommand::new("/bin/sh").with_args(["-lc", "exit 7"])
        };
        let mut session = PtySession::spawn(&command, PtySize::try_new(80, 24).unwrap()).unwrap();
        let mut reader = session.take_reader().unwrap();
        let mut writer = session.take_writer().unwrap();
        let io_thread = thread::spawn(move || -> std::io::Result<()> {
            let mut buffer = [0; 4096];
            let mut scanner = CursorPositionQueryScanner::default();

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => return Ok(()),
                    Ok(count) => {
                        scanner.scan(&buffer[..count], &mut writer)?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => return Ok(()),
                    Err(error) => return Err(error),
                }
            }
        });

        let timeout = Duration::from_secs(5);
        let status = session.wait_for_exit(timeout).unwrap();
        session.close_master();
        let joined = join_capture_thread_before(io_thread, Instant::now() + timeout);

        assert_eq!(status.exit_code(), 7);
        assert!(!status.success());
        assert!(matches!(joined, CaptureThreadJoin::Completed(Ok(()))));
    }

    #[cfg(windows)]
    #[test]
    #[allow(clippy::too_many_lines)]
    fn windows_master_close_allows_the_reader_to_drain_before_deadline() {
        const CHILD_ENV: &str = "RTERM_PTY_CLOSE_RED_CHILD";
        const CHILD_TEST: &str =
            "tests::windows_master_close_allows_the_reader_to_drain_before_deadline";
        const ENTERED_CLOSE: &str = "entered-close";
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if std::env::var_os(CHILD_ENV).is_some() {
            let command = PtyCommand::new("powershell.exe").with_args([
                "-NoLogo",
                "-NoProfile",
                "-Command",
                "Start-Sleep -Seconds 30",
            ]);
            let mut session =
                PtySession::spawn(&command, PtySize::try_new(80, 24).unwrap()).unwrap();
            let mut reader = session.take_reader().unwrap();
            let mut writer = session.take_writer().unwrap();
            let thread_baseline = windows_process_thread_ids(
                std::process::id(),
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap();
            let (reader_event_sender, reader_event_receiver) = mpsc::channel();
            let close_started = Arc::new(AtomicBool::new(false));
            let close_started_for_reader = Arc::clone(&close_started);
            let reader_worker = thread::spawn(move || {
                let mut scanner = CursorPositionQueryScanner::default();
                let mut buffer = [0_u8; 4096];
                loop {
                    reader_event_sender
                        .send(WindowsReaderEvent::ReadEntered)
                        .unwrap();
                    match reader.read(&mut buffer) {
                        Ok(0) => return Ok(()),
                        Ok(count) => {
                            if !close_started_for_reader.load(Ordering::Acquire)
                                && scanner.scan(&buffer[..count], writer.as_mut())?
                            {
                                reader_event_sender
                                    .send(WindowsReaderEvent::QueryAnswered)
                                    .unwrap();
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                            return Ok(());
                        }
                        Err(error) => return Err(error),
                    }
                }
            });
            let query_deadline = Instant::now() + Duration::from_secs(3);
            loop {
                let event = reader_event_receiver
                    .recv_timeout(query_deadline.saturating_duration_since(Instant::now()))
                    .expect("reader must answer the normal-phase cursor query");
                if matches!(event, WindowsReaderEvent::QueryAnswered) {
                    break;
                }
            }
            assert!(matches!(
                reader_event_receiver
                    .recv_timeout(Duration::from_secs(1))
                    .expect("reader must enter another raw read after the cursor response"),
                WindowsReaderEvent::ReadEntered
            ));
            let threads_after_spawn = windows_process_thread_ids(
                std::process::id(),
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap();
            let new_threads = threads_after_spawn
                .difference(&thread_baseline)
                .copied()
                .collect::<Vec<_>>();
            assert_eq!(
                new_threads.len(),
                1,
                "reader spawn must add exactly one fixture thread; baseline={thread_baseline:?}, \
                 after={threads_after_spawn:?}"
            );
            let reader_thread_id = new_threads[0];
            await_windows_read_wait(
                std::process::id(),
                reader_thread_id,
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap_or_else(|error| {
                panic!("reader never reached an outstanding ReadFile: {error}")
            });
            assert!(
                !reader_worker.is_finished(),
                "reader worker must retain its outstanding ReadFile before termination"
            );
            let status = session.terminate(Duration::from_secs(5)).unwrap();
            assert!(
                !status.success(),
                "terminated fixture unexpectedly succeeded"
            );
            assert!(
                session.try_wait().unwrap().is_some(),
                "fixture child was not reaped before master close"
            );
            eprintln!("{ENTERED_CLOSE}");
            std::io::stderr().flush().unwrap();

            close_started.store(true, Ordering::Release);
            let mut close = session.begin_master_close();
            assert!(matches!(
                close.finish_before(Instant::now() + Duration::from_secs(3)),
                PtyMasterCloseStatus::Completed
            ));
            reader_worker
                .join()
                .expect("reader worker must not panic")
                .expect("reader drain must finish after master close");
            return;
        }

        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD_TEST, "--nocapture"])
            .env(CHILD_ENV, "1")
            .env("RTERM_PTY_CLOSE_TRACE", "1")
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let fixture_pid = child.id();
        let stderr = child.stderr.take().expect("fixture stderr is piped");
        let (phase_sender, phase_receiver) = mpsc::channel();
        let stderr_worker = thread::spawn(move || {
            let mut reader = std::io::BufReader::new(stderr);
            let mut line = String::new();
            while std::io::BufRead::read_line(&mut reader, &mut line).unwrap_or(0) != 0 {
                let _ = phase_sender.send(line.clone());
                line.clear();
            }
        });

        let phase_deadline = Instant::now() + Duration::from_secs(12);
        let mut entered_close = false;
        let mut fixture_stderr = Vec::new();
        while Instant::now() < phase_deadline {
            let wait = phase_deadline.saturating_duration_since(Instant::now());
            match phase_receiver.recv_timeout(wait) {
                Ok(line) if line.contains(ENTERED_CLOSE) => {
                    fixture_stderr.push(line);
                    entered_close = true;
                    break;
                }
                Ok(line) => fixture_stderr.push(line),
                Err(_) => break,
            }
        }
        let owned_processes = windows_cim_process_tree(fixture_pid);
        if !entered_close {
            let cleanup = kill_and_reap_windows_fixture(child, stderr_worker, &owned_processes);
            panic!(
                "guarded fixture never entered master close; cleanup CIM survivors: {cleanup:?}; \
                 fixture stderr: {fixture_stderr:?}"
            );
        }

        let close_deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < close_deadline {
            if let Some(status) = child.try_wait().unwrap() {
                if status.success() {
                    stderr_worker.join().expect("stderr worker must finish");
                    fixture_stderr.extend(phase_receiver.try_iter());
                    for line in &fixture_stderr {
                        eprint!("{line}");
                    }
                    return;
                }
                let survivors =
                    kill_and_reap_windows_fixture(child, stderr_worker, &owned_processes);
                fixture_stderr.extend(phase_receiver.try_iter());
                panic!(
                    "guarded close fixture failed with {status}; cleanup survivors={survivors:?}; \
                     fixture stderr={fixture_stderr:?}"
                );
            }
            thread::park_timeout(Duration::from_millis(5));
        }

        let survivors = kill_and_reap_windows_fixture(child, stderr_worker, &owned_processes);
        assert!(
            survivors.is_empty(),
            "fixture process tree survived cleanup: {survivors:?}"
        );
        panic!(
            "Windows PTY master close exceeded 3s after the child was reaped and entered-close \
             was flushed; taskkill /T reaped the fixture tree and CIM survivor baseline is 0"
        );
    }

    #[cfg(windows)]
    enum WindowsReaderEvent {
        ReadEntered,
        QueryAnswered,
    }

    #[cfg(windows)]
    fn await_windows_read_wait(
        process_id: u32,
        thread_id: u32,
        deadline: Instant,
    ) -> Result<(), String> {
        const SCRIPT: &str = r"
$thread = (Get-Process -Id $env:RTERM_PTY_PROCESS_ID).Threads | Where-Object { $_.Id -eq [int]$env:RTERM_PTY_THREAD_ID }
if ($null -eq $thread) { Write-Output 'missing|missing'; exit 0 }
$state = [string]$thread.ThreadState
$reason = if ($state -eq 'Wait') { [string]$thread.WaitReason } else { 'none' }
Write-Output ($state + '|' + $reason)
";
        let mut consecutive = 0_u8;
        let mut observations = Vec::new();
        while Instant::now() < deadline {
            let output = match run_windows_powershell_before(
                SCRIPT,
                &[
                    ("RTERM_PTY_PROCESS_ID", process_id.to_string()),
                    ("RTERM_PTY_THREAD_ID", thread_id.to_string()),
                ],
                deadline,
            ) {
                Ok(output) => output,
                Err(error) => {
                    return Err(format!(
                        "{error}; prior thread-state observations={observations:?}"
                    ));
                }
            };
            let observation = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if observations.last() != Some(&observation) {
                observations.push(observation.clone());
            }
            if observation == "Wait|Executive" {
                consecutive += 1;
                if consecutive == 2 {
                    return Ok(());
                }
            } else {
                consecutive = 0;
            }
            thread::yield_now();
        }
        Err(format!(
            "thread {thread_id} did not produce two consecutive Wait|Executive samples; \
             observations={observations:?}"
        ))
    }

    #[cfg(windows)]
    fn windows_process_thread_ids(
        process_id: u32,
        deadline: Instant,
    ) -> Result<std::collections::BTreeSet<u32>, String> {
        const SCRIPT: &str = r"
(Get-Process -Id $env:RTERM_PTY_PROCESS_ID).Threads | ForEach-Object { [uint32]$_.Id }
";
        let output = run_windows_powershell_before(
            SCRIPT,
            &[("RTERM_PTY_PROCESS_ID", process_id.to_string())],
            deadline,
        )?;
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect())
    }

    #[cfg(windows)]
    fn windows_powershell_path() -> std::path::PathBuf {
        let path = std::path::PathBuf::from(
            std::env::var_os("SystemRoot").expect("SystemRoot is required on Windows"),
        )
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
        assert!(path.is_file(), "Windows PowerShell is missing at {path:?}");
        path
    }

    #[cfg(windows)]
    fn run_windows_powershell_before(
        script: &str,
        environment: &[(&str, String)],
        deadline: Instant,
    ) -> Result<std::process::Output, String> {
        let mut command = std::process::Command::new(windows_powershell_path());
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        for (key, value) in environment {
            command.env(key, value);
        }
        let mut child = command
            .spawn()
            .map_err(|error| format!("PowerShell query failed to start: {error}"))?;
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|error| format!("PowerShell query status failed: {error}"))?
            {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                child
                    .stdout
                    .take()
                    .expect("PowerShell stdout is piped")
                    .read_to_end(&mut stdout)
                    .map_err(|error| format!("PowerShell stdout failed: {error}"))?;
                child
                    .stderr
                    .take()
                    .expect("PowerShell stderr is piped")
                    .read_to_end(&mut stderr)
                    .map_err(|error| format!("PowerShell stderr failed: {error}"))?;
                if !status.success() {
                    return Err(format!(
                        "PowerShell query failed with {status}: {}",
                        String::from_utf8_lossy(&stderr)
                    ));
                }
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            if Instant::now() >= deadline {
                let _ = std::process::Command::new(windows_system32_tool("taskkill.exe"))
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .status();
                let _ = child.wait();
                return Err("PowerShell query exceeded its deadline and was reaped".to_owned());
            }
            thread::yield_now();
        }
    }

    #[cfg(windows)]
    fn windows_system32_tool(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(
            std::env::var_os("SystemRoot").expect("SystemRoot is required on Windows"),
        )
        .join("System32")
        .join(name)
    }

    #[cfg(windows)]
    fn windows_cim_process_tree(root: u32) -> Vec<u32> {
        const SCRIPT: &str = r"
$root = [uint32]$env:RTERM_PTY_FIXTURE_PID
$all = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
$queue = [System.Collections.Generic.Queue[uint32]]::new()
$seen = [System.Collections.Generic.HashSet[uint32]]::new()
$queue.Enqueue($root)
while ($queue.Count -gt 0) {
    $parent = $queue.Dequeue()
    foreach ($child in $all) {
        if ([uint32]$child.ParentProcessId -eq $parent -and $seen.Add([uint32]$child.ProcessId)) {
            [uint32]$child.ProcessId
            $queue.Enqueue([uint32]$child.ProcessId)
        }
    }
}
";
        let output = run_windows_powershell_before(
            SCRIPT,
            &[("RTERM_PTY_FIXTURE_PID", root.to_string())],
            Instant::now() + Duration::from_secs(5),
        )
        .expect("CIM process-tree query must complete before its deadline");
        let mut processes = vec![root];
        processes.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.trim().parse::<u32>().ok()),
        );
        processes.sort_unstable();
        processes.dedup();
        processes
    }

    #[cfg(windows)]
    fn windows_cim_survivors(processes: &[u32]) -> Vec<u32> {
        const SCRIPT: &str = r"
$ids = @($env:RTERM_PTY_FIXTURE_PIDS -split ',' | ForEach-Object { [uint32]$_ })
Get-CimInstance Win32_Process | Where-Object { $ids -contains [uint32]$_.ProcessId } | ForEach-Object { [uint32]$_.ProcessId }
";
        let ids = processes
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let output = run_windows_powershell_before(
            SCRIPT,
            &[("RTERM_PTY_FIXTURE_PIDS", ids)],
            Instant::now() + Duration::from_secs(5),
        )
        .expect("CIM survivor query must complete before its deadline");
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect()
    }

    #[cfg(windows)]
    fn kill_and_reap_windows_fixture(
        mut child: std::process::Child,
        stderr_worker: thread::JoinHandle<()>,
        owned_processes: &[u32],
    ) -> Vec<u32> {
        let _ = std::process::Command::new(windows_system32_tool("taskkill.exe"))
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
        for process_id in owned_processes {
            let _ = std::process::Command::new(windows_system32_tool("taskkill.exe"))
                .args(["/PID", &process_id.to_string(), "/T", "/F"])
                .status();
        }
        let _ = child.wait();
        stderr_worker.join().expect("stderr worker must finish");

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let survivors = windows_cim_survivors(owned_processes);
            if survivors.is_empty() || Instant::now() >= deadline {
                return survivors;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn local_pty_termination_reaps_child_before_deadline() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let command = deterministic_sleep_command();
        let mut session = PtySession::spawn(&command, PtySize::try_new(80, 24).unwrap()).unwrap();
        let timeout = Duration::from_secs(2);

        let status = session.terminate(timeout).unwrap();

        assert!(session.try_wait().unwrap().is_some());
        assert!(!status.success());
    }

    #[test]
    fn capture_timeout_never_detaches_cleanup() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let command = deterministic_sleep_command();
        let timeout = Duration::from_secs(1);
        let started = std::time::Instant::now();
        let baseline = pending_capture_cleanup_count();

        let error =
            PtySession::capture_output(&command, PtySize::try_new(80, 24).unwrap(), timeout)
                .unwrap_err();

        assert!(matches!(error, super::PtyError::Timeout(value) if value == timeout));
        assert!(started.elapsed() <= timeout + Duration::from_millis(250));
        let pending = pending_capture_cleanup_count();
        assert!(
            pending == baseline || pending == baseline + 1,
            "unexpected capture cleanup ownership delta: {baseline} -> {pending}"
        );
        let cleanup_deadline = Instant::now() + Duration::from_secs(2);
        while pending_capture_cleanup_count() != baseline && Instant::now() < cleanup_deadline {
            thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(pending_capture_cleanup_count(), baseline);
    }

    #[test]
    fn cursor_position_query_scanner_handles_every_split_position() {
        const QUERY: &[u8] = b"\x1b[6n";
        const RESPONSE: &[u8] = b"\x1b[1;1R";

        for split in 0..=QUERY.len() {
            let mut scanner = CursorPositionQueryScanner::default();
            let mut responses = Vec::new();
            let answered_before_split = scanner.scan(&QUERY[..split], &mut responses).unwrap();
            let answered_after_split = scanner.scan(&QUERY[split..], &mut responses).unwrap();
            assert_eq!(answered_before_split, split == QUERY.len());
            assert_eq!(answered_after_split, split != QUERY.len());
            assert_eq!(responses, RESPONSE, "split position {split}");
            assert!(scanner.buffered_len() < QUERY.len());
        }
    }

    #[test]
    fn cursor_position_query_scanner_handles_overlap_and_multiple_queries() {
        let mut scanner = CursorPositionQueryScanner::default();
        let mut responses = Vec::new();

        assert!(
            scanner
                .scan(b"noise\x1b\x1b[6nmore\x1b[6n", &mut responses)
                .unwrap()
        );

        assert_eq!(responses, b"\x1b[1;1R\x1b[1;1R");
        assert!(scanner.buffered_len() < b"\x1b[6n".len());
    }

    #[test]
    fn cursor_position_query_scanner_keeps_bounded_state_on_adversarial_input() {
        let mut scanner = CursorPositionQueryScanner::default();
        let mut responses = Vec::new();
        let adversarial = b"\x1b[6".repeat(100_000);

        assert!(!scanner.scan(&adversarial, &mut responses).unwrap());

        assert!(responses.is_empty());
        assert!(scanner.buffered_len() < b"\x1b[6n".len());
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_platform_echo_passes_untrusted_text_as_a_positional_argument() {
        let text = "'\";$HOME$(touch nope)`touch nope`\nUnicode: 世界";
        let command = PtyCommand::unix_platform_echo(text);

        assert_eq!(command.program(), "/bin/sh");
        assert_eq!(command.args(), ["-c", "printf '%s\\n' \"$1\"", "--", text]);
    }

    #[cfg(windows)]
    #[test]
    fn windows_platform_echo_passes_untrusted_text_through_environment() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let text = "literal & echo RTERM_INJECTED | %PATH% !value! ^ \"quoted\"\nUnicode: 终端";
        let command = PtyCommand::platform_echo(text);

        assert_eq!(
            command.args(),
            [
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); \
                 [Console]::Out.WriteLine($env:RTERM_PTY_ECHO)"
            ]
        );
        assert_eq!(command.env_value("RTERM_PTY_ECHO"), Some(text));

        let output = PtySession::capture_output(
            &command,
            PtySize::try_new(120, 30).unwrap(),
            Duration::from_secs(10),
        )
        .unwrap();
        let output = String::from_utf8_lossy(&output);
        let normalized_output = output.replace("\r\n", "\n");

        assert!(
            normalized_output.contains(text),
            "PowerShell must print metacharacters as data: {output:?}"
        );
    }

    #[test]
    fn slow_capture_thread_is_transferred_to_observable_reaper() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline = pending_capture_cleanup_count();
        let (release_sender, release_receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let _ = release_receiver.recv();
            Ok(())
        });

        let outcome = join_capture_thread_before(worker, Instant::now());

        assert!(matches!(outcome, CaptureThreadJoin::Deferred));
        assert_eq!(pending_capture_cleanup_count(), baseline + 1);
        release_sender.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while pending_capture_cleanup_count() != baseline && Instant::now() < deadline {
            thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(pending_capture_cleanup_count(), baseline);
    }

    #[test]
    fn capture_thread_panic_is_observable() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let worker = thread::spawn(|| -> std::io::Result<()> { panic!("capture reader panic") });

        let outcome = join_capture_thread_before(worker, Instant::now() + Duration::from_secs(1));

        assert!(matches!(outcome, CaptureThreadJoin::Panicked));
    }

    #[test]
    fn deferred_capture_thread_panic_is_observable() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline_pending = pending_capture_cleanup_count();
        let baseline_panics = capture_cleanup_panic_count();
        let (release_sender, release_receiver) = mpsc::channel();
        let worker = thread::spawn(move || -> std::io::Result<()> {
            let _ = release_receiver.recv();
            panic!("deferred capture reader panic")
        });

        assert!(matches!(
            join_capture_thread_before(worker, Instant::now()),
            CaptureThreadJoin::Deferred
        ));
        release_sender.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while pending_capture_cleanup_count() != baseline_pending && Instant::now() < deadline {
            thread::park_timeout(Duration::from_millis(2));
        }

        assert_eq!(pending_capture_cleanup_count(), baseline_pending);
        assert_eq!(capture_cleanup_panic_count(), baseline_panics + 1);
    }

    #[test]
    fn deferred_master_close_error_is_observable_after_worker_completion() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let settle_deadline = Instant::now() + Duration::from_secs(1);
        while pending_master_close_count() != 0 && Instant::now() < settle_deadline {
            thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(pending_master_close_count(), 0);
        let _ = take_capture_reaper_errors();
        let close_io = PtyCloseIo::new(Box::new(std::io::sink()));
        close_io.record_error(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "injected deferred close response failure",
        ));
        let close = PtyMasterClose {
            worker: Some(thread::spawn(|| {})),
            close_io,
            terminal: None,
        };

        drop(close);

        let deadline = Instant::now() + Duration::from_secs(1);
        while (capture_reaper_error_count() == 0 || pending_master_close_count() != 0)
            && Instant::now() < deadline
        {
            thread::park_timeout(Duration::from_millis(2));
        }
        let errors = take_capture_reaper_errors();
        assert_eq!(pending_master_close_count(), 0);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .to_string()
                .contains("injected deferred close response failure")
        );
        assert!(std::error::Error::source(&errors[0]).is_some());
    }

    #[test]
    fn grouped_cleanup_observes_master_close_failure_and_panic() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = take_capture_reaper_errors();
        let baseline_panics = capture_cleanup_panic_count();
        let close_io = PtyCloseIo::new(Box::new(std::io::sink()));
        close_io.record_error(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "injected grouped close response failure",
        ));

        observe_reaped_master_close(PtyMasterCloseStatus::Failed(
            close_io.error().expect("injected close error is present"),
        ));
        observe_reaped_master_close(PtyMasterCloseStatus::Panicked);

        let errors = take_capture_reaper_errors();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .to_string()
                .contains("injected grouped close response failure")
        );
        assert_eq!(capture_cleanup_panic_count(), baseline_panics + 1);
    }

    #[test]
    fn refused_termination_preserves_primary_error_until_deadline() {
        let timeout = Duration::from_millis(10);
        let started = Instant::now();
        let mut child = ();

        let error = terminate_child_before(
            &mut child,
            timeout,
            |()| Ok(None),
            |()| {
                Err(PtyError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "kill refused",
                )))
            },
        )
        .unwrap_err();

        assert!(
            matches!(error, PtyError::Io(ref error) if error.kind() == std::io::ErrorKind::PermissionDenied)
        );
        assert!(started.elapsed() >= timeout);
    }

    #[test]
    fn refused_termination_still_prefers_an_observed_exit() {
        let mut polls = 0_u8;

        let status = terminate_child_before(
            &mut polls,
            Duration::from_secs(1),
            |polls| {
                *polls += 1;
                Ok((*polls >= 3).then(|| PtyExitStatus::from_exit_code(9)))
            },
            |_| Err(PtyError::Backend("kill refused".to_owned())),
        )
        .unwrap();

        assert_eq!(status.exit_code(), 9);
    }

    #[test]
    fn first_status_poll_error_still_kills_and_polls_until_exit() {
        let mut polls = 0_u8;
        let mut killed = false;

        let status = terminate_child_before(
            &mut (),
            Duration::from_secs(1),
            |()| {
                polls += 1;
                match polls {
                    1 => Err(PtyError::Io(std::io::Error::other("first poll failed"))),
                    2 => Ok(None),
                    _ => Ok(Some(PtyExitStatus::from_exit_code(23))),
                }
            },
            |()| {
                killed = true;
                Ok(())
            },
        )
        .unwrap();

        assert!(killed);
        assert_eq!(polls, 3);
        assert_eq!(status.exit_code(), 23);
    }

    #[test]
    fn wait_for_exit_accepts_duration_max_without_overflowing() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let command = deterministic_sleep_command();
        let mut session = PtySession::spawn(&command, PtySize::try_new(80, 24).unwrap()).unwrap();

        let error = session.wait_for_exit(Duration::MAX).unwrap_err();

        assert!(matches!(
            error,
            PtyError::Io(ref error) if error.kind() == std::io::ErrorKind::InvalidInput
        ));
        let _ = session.terminate(Duration::from_secs(2));
    }

    #[test]
    fn stream_acquisition_failures_transfer_the_spawned_process_as_one_job() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let baseline_pending = pending_capture_cleanup_count();
        for fault in [1, 2] {
            let baseline_deferred = capture_reaper_deferred_count();
            STREAM_ACQUISITION_FAULT.with(|value| value.set(fault));
            FORCE_PROCESS_DEFER.with(|value| value.set(true));

            let error = PtySession::spawn(
                &deterministic_sleep_command(),
                PtySize::try_new(80, 24).unwrap(),
            )
            .err()
            .expect("injected stream acquisition should fail");

            assert!(matches!(error, PtyError::Backend(_)));
            assert_eq!(capture_reaper_deferred_count(), baseline_deferred + 1);
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while pending_capture_cleanup_count() != baseline_pending && Instant::now() < deadline {
            thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(pending_capture_cleanup_count(), baseline_pending);
    }

    #[test]
    fn capture_stream_acquisition_failures_transfer_the_spawned_process() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline_pending = pending_capture_cleanup_count();

        for fault in [1, 2] {
            let baseline_deferred = capture_reaper_deferred_count();
            STREAM_ACQUISITION_FAULT.with(|value| value.set(fault));
            FORCE_PROCESS_DEFER.with(|value| value.set(true));

            let error = PtySession::capture_output(
                &deterministic_sleep_command(),
                PtySize::try_new(80, 24).unwrap(),
                Duration::from_secs(1),
            )
            .unwrap_err();

            assert!(matches!(error, PtyError::Backend(_)));
            assert_eq!(capture_reaper_deferred_count(), baseline_deferred + 1);
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while pending_capture_cleanup_count() != baseline_pending && Instant::now() < deadline {
            thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(pending_capture_cleanup_count(), baseline_pending);
    }

    #[test]
    fn session_drop_failure_transfers_child_and_streams_to_the_reaper() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline_pending = pending_capture_cleanup_count();
        let baseline_deferred = capture_reaper_deferred_count();
        let session = PtySession::spawn(
            &deterministic_sleep_command(),
            PtySize::try_new(80, 24).unwrap(),
        )
        .unwrap();
        FORCE_SESSION_DROP_DEFER.with(|force| force.set(true));

        drop(session);

        assert_eq!(capture_reaper_deferred_count(), baseline_deferred + 1);
        assert_eq!(capture_reaper_last_process_ownership(), 0b1111);
        let deadline = Instant::now() + Duration::from_secs(3);
        while pending_capture_cleanup_count() != baseline_pending && Instant::now() < deadline {
            thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(pending_capture_cleanup_count(), baseline_pending);
    }

    #[test]
    fn reaper_poll_panic_is_retained_without_blocking_other_jobs() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline_panics = capture_cleanup_panic_count();
        let baseline_retained = capture_reaper_retained_count();
        let baseline_pending = pending_capture_cleanup_count();
        let completed = Arc::new(AtomicUsize::new(0));
        let keep_pending = Arc::new(AtomicBool::new(true));

        defer_capture_job(CaptureReapJob::Test(Box::new(|| {
            panic!("injected reaper poll panic")
        })));
        let panic_deadline = Instant::now() + Duration::from_secs(1);
        while capture_cleanup_panic_count() == baseline_panics && Instant::now() < panic_deadline {
            thread::park_timeout(Duration::from_millis(2));
        }

        let completed_for_job = Arc::clone(&completed);
        defer_capture_job(CaptureReapJob::Test(Box::new(move || {
            completed_for_job.fetch_add(1, Ordering::SeqCst);
            true
        })));
        let pending_for_job = Arc::clone(&keep_pending);
        defer_capture_job(CaptureReapJob::Test(Box::new(move || {
            !pending_for_job.load(Ordering::SeqCst)
        })));

        let completion_deadline = Instant::now() + Duration::from_secs(1);
        while completed.load(Ordering::SeqCst) == 0 && Instant::now() < completion_deadline {
            thread::park_timeout(Duration::from_millis(2));
        }

        assert_eq!(capture_cleanup_panic_count(), baseline_panics + 1);
        assert_eq!(capture_reaper_retained_count(), baseline_retained + 1);
        assert_eq!(completed.load(Ordering::SeqCst), 1);
        keep_pending.store(false, Ordering::SeqCst);
        let released_deadline = Instant::now() + Duration::from_secs(1);
        while pending_capture_cleanup_count() != baseline_pending + 1
            && Instant::now() < released_deadline
        {
            thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(pending_capture_cleanup_count(), baseline_pending + 1);
    }

    #[test]
    fn default_shell_reader_guard_joins_on_early_failure() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (guard, _receiver, finished) =
            DefaultShellReaderGuard::spawn(&deterministic_sleep_command(), false).unwrap();

        let result = fail_after_default_shell_reader_started(guard);

        assert!(result.is_err());
        let deadline = Instant::now() + Duration::from_secs(1);
        while !finished.load(Ordering::SeqCst) && Instant::now() < deadline {
            thread::park_timeout(Duration::from_millis(2));
        }
        assert!(finished.load(Ordering::SeqCst));
    }

    #[test]
    fn default_shell_reader_guard_observes_reader_panic() {
        let _guard = CAPTURE_REAPER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline_panics = capture_cleanup_panic_count();
        let (guard, _receiver, _finished) =
            DefaultShellReaderGuard::spawn(&deterministic_sleep_command(), true).unwrap();

        drop(guard);

        assert_eq!(capture_cleanup_panic_count(), baseline_panics + 1);
    }

    #[test]
    fn capture_cleanup_errors_do_not_replace_the_primary_timeout() {
        let timeout = Duration::from_secs(1);
        let mut progress = CaptureProgress::new("");
        progress.start_cleanup(timeout);

        progress.record_error(PtyError::Io(std::io::Error::other("secondary read error")));

        assert!(matches!(
            progress.primary_error.as_ref(),
            Some(PtyError::Timeout(value)) if *value == timeout
        ));
    }

    struct DefaultShellReaderGuard {
        session: Option<PtySession>,
        writer: Option<Box<dyn Write + Send>>,
        reader_thread: Option<thread::JoinHandle<std::io::Result<()>>>,
    }

    type DefaultShellReaderSpawn = (
        DefaultShellReaderGuard,
        mpsc::Receiver<Vec<u8>>,
        Arc<AtomicBool>,
    );

    impl DefaultShellReaderGuard {
        fn spawn(
            command: &PtyCommand,
            panic_reader: bool,
        ) -> Result<DefaultShellReaderSpawn, PtyError> {
            let mut session = PtySession::spawn(command, PtySize::try_new(80, 24).unwrap())?;
            let mut reader = session.take_reader()?;
            let writer = session.take_writer()?;
            let (sender, receiver) = mpsc::channel();
            let finished = Arc::new(AtomicBool::new(false));
            let finished_for_thread = Arc::clone(&finished);
            let reader_thread = thread::spawn(move || -> std::io::Result<()> {
                struct Finished(Arc<AtomicBool>);
                impl Drop for Finished {
                    fn drop(&mut self) {
                        self.0.store(true, Ordering::SeqCst);
                    }
                }
                let _finished = Finished(finished_for_thread);
                assert!(!panic_reader, "injected default-shell reader panic");
                let mut buffer = [0_u8; 4096];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => return Ok(()),
                        Ok(count) => {
                            if sender.send(buffer[..count].to_vec()).is_err() {
                                return Ok(());
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                            return Ok(());
                        }
                        Err(error) => return Err(error),
                    }
                }
            });
            Ok((
                Self {
                    session: Some(session),
                    writer: Some(writer),
                    reader_thread: Some(reader_thread),
                },
                receiver,
                finished,
            ))
        }

        fn session_mut(&mut self) -> &mut PtySession {
            self.session
                .as_mut()
                .expect("default-shell session is present")
        }

        fn writer_mut(&mut self) -> &mut dyn Write {
            self.writer
                .as_mut()
                .expect("default-shell writer is present")
                .as_mut()
        }

        fn cleanup(&mut self) -> Option<CaptureThreadJoin> {
            const CLEANUP_BUDGET: Duration = Duration::from_millis(500);
            let deadline = Instant::now()
                .checked_add(CLEANUP_BUDGET)
                .expect("bounded cleanup deadline fits in Instant");
            let mut session = self.session.take();
            let mut close = session.as_mut().map(PtySession::begin_master_close);
            drop(self.writer.take());
            drop(session);
            let reader = self
                .reader_thread
                .take()
                .map(|reader| join_capture_thread_before(reader, deadline));
            if let Some(close) = close.as_mut() {
                observe_reaped_master_close(close.finish_before(deadline));
            }
            reader
        }

        fn finish(mut self) -> CaptureThreadJoin {
            self.cleanup()
                .expect("default-shell reader ownership is present")
        }
    }

    impl Drop for DefaultShellReaderGuard {
        fn drop(&mut self) {
            let _ = self.cleanup();
        }
    }

    fn fail_after_default_shell_reader_started(
        _guard: DefaultShellReaderGuard,
    ) -> std::io::Result<()> {
        Err(std::io::Error::other(
            "injected default-shell early failure",
        ))
    }

    fn default_shell_smoke_with_phase_deadlines() {
        const MARKER: &str = "rterm-pty-interactive-smoke";
        const PHASE_BUDGET: Duration = Duration::from_secs(5);
        let (mut guard, receiver, _finished) =
            DefaultShellReaderGuard::spawn(&PtyCommand::default_shell(), false).unwrap();
        let mut scanner = CursorPositionQueryScanner::default();
        let mut output = Vec::new();

        receive_ready_before(
            &receiver,
            &mut scanner,
            guard.writer_mut(),
            &mut output,
            Instant::now() + PHASE_BUDGET,
        );

        let command = if cfg!(windows) {
            "echo rterm-pty-interactive-^smoke\r\nexit\r\n"
        } else {
            "printf 'rterm-pty-interactive-%s\\n' smoke\r\nexit\r\n"
        };
        let writer = guard.writer_mut();
        writer
            .write_all(command.as_bytes())
            .and_then(|()| writer.flush())
            .unwrap();
        receive_marker_before(
            &receiver,
            &mut scanner,
            guard.writer_mut(),
            &mut output,
            MARKER,
            Instant::now() + PHASE_BUDGET,
        );

        let status = guard.session_mut().wait_for_exit(PHASE_BUDGET).unwrap();
        assert!(status.success(), "default shell exit status: {status:?}");
        let joined = guard.finish();
        assert!(matches!(joined, CaptureThreadJoin::Completed(Ok(()))));
    }

    fn receive_ready_before(
        receiver: &mpsc::Receiver<Vec<u8>>,
        scanner: &mut CursorPositionQueryScanner,
        writer: &mut dyn Write,
        output: &mut Vec<u8>,
        deadline: Instant,
    ) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "default shell readiness deadline expired"
        );
        let chunk = receiver
            .recv_timeout(remaining)
            .unwrap_or_else(|error| panic!("default shell was not ready before deadline: {error}"));
        scanner.scan(&chunk, writer).unwrap();
        output.extend_from_slice(&chunk);
        assert!(
            !chunk.is_empty(),
            "default shell readiness output was empty"
        );
    }

    fn receive_marker_before(
        receiver: &mpsc::Receiver<Vec<u8>>,
        scanner: &mut CursorPositionQueryScanner,
        writer: &mut dyn Write,
        output: &mut Vec<u8>,
        marker: &str,
        deadline: Instant,
    ) {
        while !String::from_utf8_lossy(output).contains(marker) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "timed out waiting for {marker:?}");
            let chunk = receiver.recv_timeout(remaining).unwrap_or_else(|error| {
                panic!("PTY reader stopped before marker {marker:?}: {error}")
            });
            scanner.scan(&chunk, writer).unwrap();
            output.extend_from_slice(&chunk);
        }
    }

    fn deterministic_exit_command(code: u32) -> PtyCommand {
        if cfg!(windows) {
            PtyCommand::new("cmd.exe").with_args(["/D", "/C", &format!("exit {code}")])
        } else {
            PtyCommand::new("/bin/sh").with_args(["-lc", &format!("exit {code}")])
        }
    }

    fn deterministic_sleep_command() -> PtyCommand {
        if cfg!(windows) {
            PtyCommand::new("cmd.exe").with_args(["/D", "/C", "ping -n 30 127.0.0.1 >NUL"])
        } else {
            PtyCommand::new("/bin/sh").with_args(["-lc", "sleep 30"])
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DefaultShellPlatform {
    Windows,
    Macos,
    Unix,
}

impl DefaultShellPlatform {
    const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Unix
        }
    }
}

fn default_shell_program() -> String {
    default_shell_program_from(
        DefaultShellPlatform::current(),
        std::env::var_os("COMSPEC").as_deref(),
        std::env::var_os("SHELL").as_deref(),
    )
}

fn default_shell_program_from(
    platform: DefaultShellPlatform,
    comspec: Option<&std::ffi::OsStr>,
    shell: Option<&std::ffi::OsStr>,
) -> String {
    let configured = match platform {
        DefaultShellPlatform::Windows => comspec,
        DefaultShellPlatform::Macos | DefaultShellPlatform::Unix => shell,
    };
    if let Some(configured) = configured.filter(|value| !value.is_empty()) {
        return configured.to_string_lossy().into_owned();
    }

    match platform {
        DefaultShellPlatform::Windows => "cmd.exe",
        DefaultShellPlatform::Macos => "/bin/zsh",
        DefaultShellPlatform::Unix => "/bin/sh",
    }
    .to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyCommand {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env: Vec<(String, String)>,
}

impl PtyCommand {
    #[must_use]
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: default_terminal_environment(),
        }
    }

    #[must_use]
    pub fn default_shell() -> Self {
        Self::new(default_shell_program())
    }

    #[must_use]
    pub fn platform_echo(text: impl Into<String>) -> Self {
        let text = text.into();

        #[cfg(windows)]
        {
            Self::new(windows_powershell_program().to_string_lossy())
                .with_args([
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); \
                     [Console]::Out.WriteLine($env:RTERM_PTY_ECHO)",
                ])
                .with_env("RTERM_PTY_ECHO", text)
        }

        #[cfg(not(windows))]
        {
            Self::unix_platform_echo(text)
        }
    }

    #[cfg(not(windows))]
    fn unix_platform_echo(text: impl Into<String>) -> Self {
        let text = text.into();
        Self::new("/bin/sh").with_args(["-c", "printf '%s\\n' \"$1\"", "--", text.as_str()])
    }

    #[must_use]
    pub fn platform_identity_command() -> Self {
        if cfg!(windows) {
            Self::new("whoami.exe")
        } else {
            Self::new("whoami")
        }
    }

    #[must_use]
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    #[must_use]
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    #[must_use]
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key = key.into();
        let value = value.into();
        if let Some((_, existing)) = self
            .env
            .iter_mut()
            .find(|(existing_key, _)| existing_key == &key)
        {
            *existing = value;
        } else {
            self.env.push((key, value));
        }
        self
    }

    #[must_use]
    pub fn without_env(mut self, key: &str) -> Self {
        self.env.retain(|(existing_key, _)| existing_key != key);
        self
    }

    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    #[must_use]
    pub fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    #[must_use]
    pub fn env_value(&self, key: &str) -> Option<&str> {
        self.env
            .iter()
            .find_map(|(env_key, value)| (env_key == key).then_some(value.as_str()))
    }

    /// Validate that this command can be passed to a PTY backend.
    ///
    /// # Errors
    ///
    /// Returns [`PtyError::InvalidCommand`] when the command program is empty,
    /// or on Windows when a `.cmd`/`.bat` invocation contains characters that
    /// `cmd.exe` cannot safely preserve as argument data.
    pub fn validate(&self) -> Result<(), PtyError> {
        if self.program.trim().is_empty() {
            return Err(PtyError::InvalidCommand(
                "PTY command program cannot be empty".to_owned(),
            ));
        }

        #[cfg(windows)]
        {
            let path = self.windows_path();
            let (program, kind) = find_windows_program(&self.program, path.as_deref());
            if kind == WindowsProgramKind::CmdShim {
                validate_windows_cmd_invocation(&program, &self.args)?;
            }
        }

        Ok(())
    }

    #[cfg(windows)]
    fn windows_path(&self) -> Option<std::ffi::OsString> {
        self.env
            .iter()
            .find_map(|(key, value)| key.eq_ignore_ascii_case("PATH").then_some(value))
            .map(std::ffi::OsString::from)
            .or_else(|| std::env::var_os("PATH"))
    }

    fn to_builder(&self) -> CommandBuilder {
        #[cfg(windows)]
        let (program, args) = {
            let path = self.windows_path();
            let resolved = resolve_windows_command(&self.program, &self.args, path.as_deref());
            (resolved.program, resolved.args)
        };

        #[cfg(not(windows))]
        let (program, args) = (&self.program, &self.args);

        let mut builder = CommandBuilder::new(program);
        for arg in args {
            builder.arg(arg);
        }
        for (key, value) in &self.env {
            builder.env(key, value);
        }
        if let Some(cwd) = &self.cwd {
            builder.cwd(cwd);
        }
        builder
    }
}

#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsCommandResolution {
    program: PathBuf,
    args: Vec<String>,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsProgramKind {
    Native,
    CmdShim,
    PowerShellScript,
}

#[cfg(windows)]
// Windows CreateProcess cannot launch the extensionless Unix shim that npm
// places next to its `.cmd` and `.ps1` wrappers. Resolve those wrappers before
// handing the command to portable-pty so ConPTY receives a real executable.
fn resolve_windows_command(
    program: &str,
    args: &[String],
    path: Option<&std::ffi::OsStr>,
) -> WindowsCommandResolution {
    let (program_path, kind) = find_windows_program(program, path);
    let mut resolved_args = Vec::new();

    match kind {
        WindowsProgramKind::Native => resolved_args.extend(args.iter().cloned()),
        WindowsProgramKind::CmdShim => {
            resolved_args.extend([
                "/D".to_owned(),
                "/V:OFF".to_owned(),
                "/S".to_owned(),
                "/C".to_owned(),
                "call".to_owned(),
            ]);
            resolved_args.push(program_path.to_string_lossy().into_owned());
            resolved_args.extend(args.iter().cloned());
        }
        WindowsProgramKind::PowerShellScript => {
            resolved_args.extend([
                "-NoLogo".to_owned(),
                "-NoProfile".to_owned(),
                "-NonInteractive".to_owned(),
                "-File".to_owned(),
                program_path.to_string_lossy().into_owned(),
            ]);
            resolved_args.extend(args.iter().cloned());
        }
    }

    let resolved_program = match kind {
        WindowsProgramKind::Native => program_path,
        WindowsProgramKind::CmdShim => windows_system32_program("cmd.exe"),
        WindowsProgramKind::PowerShellScript => windows_powershell_program(),
    };

    WindowsCommandResolution {
        program: resolved_program,
        args: resolved_args,
    }
}

#[cfg(windows)]
fn validate_windows_cmd_invocation(program: &Path, args: &[String]) -> Result<(), PtyError> {
    if let Some(character) = unsupported_windows_cmd_character(&program.to_string_lossy()) {
        return Err(PtyError::InvalidCommand(format!(
            "cmd.exe cannot safely launch a .cmd/.bat path containing {character:?}; use a path \
             without command-shell metacharacters"
        )));
    }

    for (index, argument) in args.iter().enumerate() {
        if let Some(character) = unsupported_windows_cmd_character(argument) {
            return Err(PtyError::InvalidCommand(format!(
                "cmd.exe cannot safely pass argument {index} containing {character:?} to a \
                 .cmd/.bat program; use a native executable or PowerShell script"
            )));
        }
    }

    Ok(())
}

#[cfg(windows)]
fn unsupported_windows_cmd_character(value: &str) -> Option<char> {
    value.chars().find(|character| {
        matches!(
            character,
            '%' | '!' | '&' | '|' | '^' | '<' | '>' | '(' | ')' | '"' | '\r' | '\n' | '\0'
        )
    })
}

#[cfg(windows)]
fn windows_system32_program(name: &str) -> PathBuf {
    std::env::var_os("SystemRoot").map_or_else(
        || PathBuf::from(name),
        |root| PathBuf::from(root).join("System32").join(name),
    )
}

#[cfg(windows)]
fn windows_powershell_program() -> PathBuf {
    std::env::var_os("SystemRoot").map_or_else(
        || PathBuf::from("powershell.exe"),
        |root| {
            PathBuf::from(root)
                .join("System32")
                .join("WindowsPowerShell")
                .join("v1.0")
                .join("powershell.exe")
        },
    )
}

#[cfg(windows)]
fn find_windows_program(
    program: &str,
    path: Option<&std::ffi::OsStr>,
) -> (PathBuf, WindowsProgramKind) {
    let input = Path::new(program);

    if let Some(kind) = windows_program_kind(input) {
        return (input.to_owned(), kind);
    }

    let has_directory = input.is_absolute()
        || input
            .parent()
            .is_some_and(|parent| !parent.as_os_str().is_empty());
    if has_directory {
        if let Some((path, kind)) = windows_sidecar(input) {
            return (path, kind);
        }
        return (input.to_owned(), WindowsProgramKind::Native);
    }

    if let Some(path) = path {
        for directory in std::env::split_paths(path) {
            let candidate = directory.join(input);
            if let Some((path, kind)) = windows_sidecar(&candidate) {
                return (path, kind);
            }
        }

        for directory in std::env::split_paths(path) {
            let candidate = directory.join(input);
            if candidate.is_file() {
                return (candidate, WindowsProgramKind::Native);
            }
        }
    }

    if let Some((path, kind)) = windows_sidecar(input) {
        return (path, kind);
    }

    (input.to_owned(), WindowsProgramKind::Native)
}

#[cfg(windows)]
fn windows_sidecar(base: &Path) -> Option<(PathBuf, WindowsProgramKind)> {
    [
        ("com", WindowsProgramKind::Native),
        ("exe", WindowsProgramKind::Native),
        ("bat", WindowsProgramKind::CmdShim),
        ("cmd", WindowsProgramKind::CmdShim),
        ("ps1", WindowsProgramKind::PowerShellScript),
    ]
    .into_iter()
    .map(|(extension, kind)| (base.with_extension(extension), kind))
    .find(|(candidate, _)| candidate.is_file())
}

#[cfg(windows)]
fn windows_program_kind(program: &Path) -> Option<WindowsProgramKind> {
    match program.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "bat" | "cmd" => Some(WindowsProgramKind::CmdShim),
        "ps1" => Some(WindowsProgramKind::PowerShellScript),
        _ => Some(WindowsProgramKind::Native),
    }
}

fn default_terminal_environment() -> Vec<(String, String)> {
    vec![
        ("TERM".to_owned(), "xterm-256color".to_owned()),
        ("COLORTERM".to_owned(), "truecolor".to_owned()),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySize {
    columns: u16,
    rows: u16,
}

impl PtySize {
    /// Create a PTY size in character cells.
    ///
    /// # Errors
    ///
    /// Returns [`PtyError::InvalidSize`] when either dimension is zero.
    pub fn try_new(columns: u16, rows: u16) -> Result<Self, PtyError> {
        if columns == 0 || rows == 0 {
            return Err(PtyError::InvalidSize { columns, rows });
        }

        Ok(Self { columns, rows })
    }

    #[must_use]
    pub const fn columns(self) -> u16 {
        self.columns
    }

    #[must_use]
    pub const fn rows(self) -> u16 {
        self.rows
    }

    const fn to_portable(self) -> PortablePtySize {
        PortablePtySize {
            rows: self.rows,
            cols: self.columns,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyExitStatus {
    code: u32,
    signal: Option<String>,
}

impl PtyExitStatus {
    #[must_use]
    pub const fn from_exit_code(code: u32) -> Self {
        Self { code, signal: None }
    }

    #[must_use]
    /// Projects a signal name into the portable app status representation.
    ///
    /// Rich SSH signal metadata remains in the SSH session result; this type
    /// intentionally carries only the conventional failure code and name.
    pub fn from_signal(signal: impl Into<String>) -> Self {
        Self {
            code: 1,
            signal: Some(signal.into()),
        }
    }

    #[must_use]
    pub const fn success(&self) -> bool {
        self.code == 0 && self.signal.is_none()
    }

    #[must_use]
    pub const fn exit_code(&self) -> u32 {
        self.code
    }

    #[must_use]
    pub fn signal(&self) -> Option<&str> {
        self.signal.as_deref()
    }
}

impl From<PortableExitStatus> for PtyExitStatus {
    fn from(status: PortableExitStatus) -> Self {
        Self {
            code: status.exit_code(),
            signal: status.signal().map(str::to_owned),
        }
    }
}

pub struct PtySession {
    master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    reader: Option<Box<dyn Read + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    close_io: Arc<PtyCloseIo>,
    interrupt: PtySessionInterrupt,
}

/// Cloneable out-of-band termination handle for a live PTY child.
#[derive(Clone)]
pub struct PtySessionInterrupt {
    state: Arc<PtySessionInterruptState>,
}

struct PtySessionInterruptState {
    requested: AtomicBool,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    close_io: Arc<PtyCloseIo>,
}

impl fmt::Debug for PtySessionInterrupt {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PtySessionInterrupt")
            .field("requested", &self.state.requested.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl PtySessionInterrupt {
    fn new(killer: Box<dyn ChildKiller + Send + Sync>, close_io: Arc<PtyCloseIo>) -> Self {
        Self {
            state: Arc::new(PtySessionInterruptState {
                requested: AtomicBool::new(false),
                killer: Mutex::new(killer),
                close_io,
            }),
        }
    }

    /// Requests child termination and makes later writer calls fail closed.
    ///
    /// Repeated calls are idempotent.
    ///
    /// # Errors
    ///
    /// Returns the platform process error from the first termination request.
    pub fn interrupt(&self) -> io::Result<()> {
        self.state.close_io.closing.store(true, Ordering::Release);
        if self.state.requested.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut killer = self
            .state
            .killer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.state.requested.load(Ordering::Acquire) {
            return Ok(());
        }
        let result = match killer.kill() {
            #[cfg(windows)]
            Err(error) if error.raw_os_error() == Some(6) => Ok(()),
            result => result,
        };
        if result.is_ok() {
            self.state.requested.store(true, Ordering::Release);
        }
        result
    }
}

struct PtyCloseIo {
    state: Mutex<PtyCloseState>,
    closing: AtomicBool,
    error: Mutex<Option<Arc<io::Error>>>,
}

struct PtyCloseState {
    writer: Option<Box<dyn Write + Send>>,
    reply_scanner: CursorPositionReplyScanner,
    queries_seen: usize,
    replies_seen: usize,
    speculative_reply_sent: bool,
    allow_internal_reply: bool,
    reader_alive: bool,
}

impl PtyCloseIo {
    fn new(writer: Box<dyn Write + Send>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(PtyCloseState {
                writer: Some(writer),
                reply_scanner: CursorPositionReplyScanner::default(),
                queries_seen: 0,
                replies_seen: 0,
                speculative_reply_sent: false,
                allow_internal_reply: false,
                reader_alive: true,
            }),
            closing: AtomicBool::new(false),
            error: Mutex::new(None),
        })
    }

    fn begin_close(&self, child_reaped: bool) {
        self.closing.store(true, Ordering::Release);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.allow_internal_reply |= child_reaped;
        #[cfg(test)]
        pty_close_trace(&format!(
            "begin-close writer-present={} child-reaped={child_reaped}",
            state.writer.is_some()
        ));
        if !state.reader_alive {
            drop(state.writer.take());
            return;
        }
        if state.allow_internal_reply {
            self.supplement_cursor_responses(&mut state, true);
        }
    }

    fn close_writer(&self) {
        drop(
            self.state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .writer
                .take(),
        );
    }

    #[cfg(test)]
    fn writer_present(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .writer
            .is_some()
    }

    fn observe_reader_queries(&self, count: usize) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.queries_seen = state.queries_seen.saturating_add(count);
        if self.closing.load(Ordering::Acquire) && state.allow_internal_reply {
            self.supplement_cursor_responses(&mut state, false);
        }
    }

    fn reader_finished(&self) {
        let writer = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.reader_alive = false;
            state.writer.take()
        };
        drop(writer);
    }

    fn supplement_cursor_responses(&self, state: &mut PtyCloseState, allow_speculative: bool) {
        const RESPONSE: &[u8] = b"\x1b[1;1R";
        let missing = state.queries_seen.saturating_sub(state.replies_seen);
        let speculative = missing == 0
            && allow_speculative
            && state.queries_seen == 0
            && state.replies_seen == 0
            && !state.speculative_reply_sent;
        let count = if missing > 0 {
            missing
        } else if speculative {
            state.speculative_reply_sent = true;
            1
        } else {
            0
        };
        if count == 0 {
            return;
        }
        #[cfg(test)]
        pty_close_trace(&format!("cursor-response-request count={count}"));
        let result = (|| {
            let writer = state
                .writer
                .as_mut()
                .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "PTY writer is closed"))?;
            for _ in 0..count {
                writer.write_all(RESPONSE)?;
            }
            writer.flush()
        })();
        #[cfg(test)]
        pty_close_trace(&format!("cursor-response-result={:?}", result.as_ref()));
        match result {
            Ok(()) => state.replies_seen = state.replies_seen.saturating_add(count),
            Err(_) if speculative => {}
            Err(error) => self.record_error(error),
        }
    }

    fn record_error(&self, error: io::Error) {
        self.error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_insert_with(|| Arc::new(error));
    }

    fn error(&self) -> Option<PtyMasterCloseError> {
        self.error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .map(|source| PtyMasterCloseError { source })
    }
}

struct PtyWriterProxy {
    close_io: Arc<PtyCloseIo>,
}

impl Write for PtyWriterProxy {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.close_io.closing.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "PTY writer is closing",
            ));
        }
        let mut state = self
            .close_io
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.close_io.closing.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "PTY writer is closing",
            ));
        }
        let count = state
            .writer
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "PTY writer is closed"))?
            .write(buffer)?;
        let replies = state.reply_scanner.observe(&buffer[..count]);
        state.replies_seen = state.replies_seen.saturating_add(replies);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.close_io.closing.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "PTY writer is closing",
            ));
        }
        let mut state = self
            .close_io
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.close_io.closing.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "PTY writer is closing",
            ));
        }
        state
            .writer
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "PTY writer is closed"))?
            .flush()
    }
}

impl Drop for PtyWriterProxy {
    fn drop(&mut self) {
        if !self.close_io.closing.load(Ordering::Acquire) {
            self.close_io.close_writer();
        }
    }
}

struct PtyReaderProxy {
    reader: Box<dyn Read + Send>,
    close_io: Arc<PtyCloseIo>,
    scanner: CursorPositionQueryScanner,
}

impl Read for PtyReaderProxy {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        #[cfg(test)]
        pty_close_trace(&format!(
            "reader-read-enter closing={}",
            self.close_io.closing.load(Ordering::Acquire)
        ));
        let result = self.reader.read(buffer);
        #[cfg(test)]
        pty_close_trace(&format!(
            "reader-read-return result={:?} closing={} writer-present={}",
            result.as_ref().copied(),
            self.close_io.closing.load(Ordering::Acquire),
            self.close_io.writer_present()
        ));
        match result {
            Ok(count) if count > 0 => {
                let queries = self.scanner.observe(&buffer[..count]);
                #[cfg(test)]
                pty_close_trace(&format!(
                    "reader-scan count={count} queries={queries} buffered={}",
                    self.scanner.buffered_len()
                ));
                self.close_io.observe_reader_queries(queries);
            }
            Ok(0) => self.close_io.reader_finished(),
            Err(ref error) if error.kind() != io::ErrorKind::Interrupted => {
                self.close_io.reader_finished();
            }
            _ => {}
        }
        result
    }
}

impl Drop for PtyReaderProxy {
    fn drop(&mut self) {
        self.close_io.reader_finished();
    }
}

/// A structured failure observed while answering a terminal query during PTY close.
///
/// The original [`io::Error`] remains available through [`Error::source`].
#[derive(Debug, Clone)]
pub struct PtyMasterCloseError {
    source: Arc<io::Error>,
}

impl Display for PtyMasterCloseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "PTY close response failed: {}", self.source)
    }
}

impl Error for PtyMasterCloseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// The observable state of an owned PTY master-close operation.
#[derive(Debug, Clone)]
pub enum PtyMasterCloseStatus {
    /// The master-close worker finished and no close-time I/O error was observed.
    Completed,
    /// The deadline expired. The [`PtyMasterClose`] token still owns the worker.
    Deferred,
    /// The master closed, but close-time cursor-response I/O failed.
    Failed(PtyMasterCloseError),
    /// The named master-close worker panicked and was joined.
    Panicked,
    /// Worker startup failed and process-lifetime retained storage took ownership.
    ///
    /// The token no longer owns the master handle in this state.
    Retained,
}

/// Owns a potentially asynchronous PTY master-close operation.
///
/// Dropping an unfinished token transfers its worker and structured error state
/// to the process-lifetime cleanup reaper; it never synchronously joins or
/// detaches the worker.
pub struct PtyMasterClose {
    worker: Option<thread::JoinHandle<()>>,
    close_io: Arc<PtyCloseIo>,
    terminal: Option<PtyMasterCloseStatus>,
}

#[cfg(windows)]
type PtyMasterOwner = Arc<Mutex<Option<Box<dyn MasterPty + Send>>>>;
type PtyReaderOwner = Arc<Mutex<Option<Box<dyn Read + Send>>>>;

impl PtyMasterClose {
    fn start(master: Option<Box<dyn MasterPty + Send>>, close_io: Arc<PtyCloseIo>) -> Self {
        let Some(master) = master else {
            return Self {
                worker: None,
                close_io,
                terminal: None,
            };
        };
        #[cfg(not(windows))]
        {
            drop(master);
            Self {
                worker: None,
                close_io,
                terminal: None,
            }
        }
        #[cfg(windows)]
        {
            let owner = Arc::new(Mutex::new(Some(master)));
            let owner_for_worker = Arc::clone(&owner);
            let worker = if let Ok(worker) = thread::Builder::new()
                .name("rterm-pty-master-close".to_owned())
                .spawn(move || {
                    #[cfg(test)]
                    pty_close_trace("master-close-worker-enter");
                    drop(
                        owner_for_worker
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .take(),
                    );
                    #[cfg(test)]
                    pty_close_trace("master-close-worker-return");
                }) {
                Some(worker)
            } else {
                retain_capture_job(CaptureReapJob::MasterOwner(owner));
                return Self {
                    worker: None,
                    close_io,
                    terminal: Some(PtyMasterCloseStatus::Retained),
                };
            };
            Self {
                worker,
                close_io,
                terminal: None,
            }
        }
    }

    /// Observe completion before an absolute deadline.
    ///
    /// [`PtyMasterCloseStatus::Deferred`] leaves ownership in this token so the
    /// caller may retry or move it into a larger grouped cleanup owner.
    #[must_use]
    pub fn finish_before(&mut self, deadline: Instant) -> PtyMasterCloseStatus {
        if let Some(status) = self.terminal.clone() {
            return status;
        }
        let Some(worker) = self.worker.as_ref() else {
            let status = self.close_io.error().map_or(
                PtyMasterCloseStatus::Completed,
                PtyMasterCloseStatus::Failed,
            );
            self.terminal = Some(status.clone());
            return status;
        };
        while !worker.is_finished() && Instant::now() < deadline {
            thread::park_timeout(
                deadline
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(2)),
            );
        }
        if !worker.is_finished() {
            #[cfg(test)]
            pty_close_trace("master-close-finish=Deferred");
            return PtyMasterCloseStatus::Deferred;
        }
        let status = if self
            .worker
            .take()
            .is_some_and(|worker| worker.join().is_err())
        {
            PtyMasterCloseStatus::Panicked
        } else {
            self.close_io.error().map_or(
                PtyMasterCloseStatus::Completed,
                PtyMasterCloseStatus::Failed,
            )
        };
        self.terminal = Some(status.clone());
        #[cfg(test)]
        pty_close_trace(&format!("master-close-finish={status:?}"));
        status
    }
}

#[cfg(test)]
fn pty_close_trace(message: &str) {
    if std::env::var_os("RTERM_PTY_CLOSE_TRACE").is_some() {
        eprintln!("rterm-pty-close-trace {message}");
    }
}

impl Drop for PtyMasterClose {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            defer_capture_job(CaptureReapJob::MasterClose(PtyMasterCloseReap {
                worker: Some(worker),
                close_io: Arc::clone(&self.close_io),
            }));
        }
    }
}

struct PtyOwnedProcess {
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    master: Option<Box<dyn MasterPty + Send>>,
    reader: Option<Box<dyn Read + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    kill_sent: bool,
    close_io: Option<Arc<PtyCloseIo>>,
    master_close: Option<PtyMasterClose>,
    reader_close: Option<CaptureReaderThread>,
    reader_owner: Option<PtyReaderOwner>,
}

enum CaptureReadEvent {
    Chunk(Vec<u8>),
    Eof,
    Error(io::Error),
}

struct CaptureIo {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    receiver: mpsc::Receiver<CaptureReadEvent>,
    reader_thread: Option<CaptureReaderThread>,
    close_io: Arc<PtyCloseIo>,
    master_close: Option<PtyMasterClose>,
}

impl CaptureIo {
    fn begin_master_close(&mut self) {
        if self.master_close.is_some() {
            return;
        }
        self.close_io.begin_close(true);
        drop(self.writer.take());
        self.master_close = Some(PtyMasterClose::start(
            self.master.take(),
            Arc::clone(&self.close_io),
        ));
    }

    fn poll_master_close(&mut self) -> bool {
        let Some(close) = self.master_close.as_mut() else {
            return true;
        };
        let status = close.finish_before(Instant::now());
        if matches!(status, PtyMasterCloseStatus::Deferred) {
            return false;
        }
        observe_reaped_master_close(status);
        drop(self.master_close.take());
        true
    }
}

type CaptureReaderThread = thread::JoinHandle<io::Result<()>>;

enum CaptureThreadJoin {
    Completed(io::Result<()>),
    Panicked,
    Deferred,
}

enum CaptureReapJob {
    Reader(Option<CaptureReaderThread>),
    Io(CaptureIo),
    Process(PtyOwnedProcess),
    MasterClose(PtyMasterCloseReap),
    #[cfg(windows)]
    MasterOwner(PtyMasterOwner),
    #[cfg(test)]
    Test(Box<dyn FnMut() -> bool + Send>),
}

impl CaptureReapJob {
    fn counts_as_capture_cleanup(&self) -> bool {
        match self {
            Self::MasterClose(_) => false,
            #[cfg(windows)]
            Self::MasterOwner(_) => false,
            _ => true,
        }
    }

    fn counts_as_master_close(&self) -> bool {
        matches!(self, Self::MasterClose(_))
    }
}

struct PtyMasterCloseReap {
    worker: Option<thread::JoinHandle<()>>,
    close_io: Arc<PtyCloseIo>,
}

enum CaptureReaperInitialization {
    Ready(mpsc::Sender<CaptureReapJob>),
    Failed,
}

static CAPTURE_REAPER: OnceLock<CaptureReaperInitialization> = OnceLock::new();
static CAPTURE_REAPER_RETAINED: OnceLock<Mutex<Vec<CaptureReapJob>>> = OnceLock::new();
static CAPTURE_REAPER_PENDING: AtomicUsize = AtomicUsize::new(0);
static MASTER_CLOSE_REAPER_PENDING: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_REAPER_PANICS: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_REAPER_ERRORS: OnceLock<Mutex<VecDeque<PtyMasterCloseError>>> = OnceLock::new();
#[cfg(test)]
static CAPTURE_REAPER_DEFERRED: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static CAPTURE_REAPER_LAST_PROCESS_OWNERSHIP: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
std::thread_local! {
    static STREAM_ACQUISITION_FAULT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    static FORCE_PROCESS_DEFER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FORCE_SESSION_DROP_DEFER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn observe_reaped_master_close(status: PtyMasterCloseStatus) {
    match status {
        PtyMasterCloseStatus::Completed | PtyMasterCloseStatus::Deferred => {}
        PtyMasterCloseStatus::Failed(error) => {
            eprintln!("deferred PTY master close observed an I/O error: {error}");
            CAPTURE_REAPER_ERRORS
                .get_or_init(|| Mutex::new(VecDeque::new()))
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push_back(error);
        }
        PtyMasterCloseStatus::Panicked => {
            CAPTURE_REAPER_PANICS.fetch_add(1, Ordering::SeqCst);
        }
        PtyMasterCloseStatus::Retained => {
            // `PtyMasterClose::start` already transferred the master owner to
            // `CAPTURE_REAPER_RETAINED`; keep that process-lifetime ownership.
            eprintln!("PTY master close retained its master for process lifetime");
        }
    }
}

impl PtyOwnedProcess {
    fn new(
        child: Box<dyn portable_pty::Child + Send + Sync>,
        master: Box<dyn MasterPty + Send>,
    ) -> Self {
        Self {
            child: Some(child),
            master: Some(master),
            reader: None,
            writer: None,
            kill_sent: false,
            close_io: None,
            master_close: None,
            reader_close: None,
            reader_owner: None,
        }
    }

    fn begin_master_close(&mut self) -> bool {
        if self.master_close.is_some() {
            return true;
        }
        let had_shared_close_io = self.close_io.is_some();
        if !had_shared_close_io && (self.reader.is_none() || self.writer.is_none()) {
            // A real backend stream-acquisition failure can leave the master as
            // the only owner of an inaccessible pipe endpoint. Retain the whole
            // process group instead of starting a close that cannot be drained.
            return false;
        }
        let close_io = Arc::clone(self.close_io.get_or_insert_with(|| {
            PtyCloseIo::new(
                self.writer
                    .take()
                    .expect("complete acquired streams include a writer"),
            )
        }));
        if self.reader_close.is_none() && (self.reader.is_some() || self.reader_owner.is_some()) {
            let owner = self.reader_owner.take().unwrap_or_else(|| {
                let reader = self.reader.take().expect("reader ownership is present");
                let reader: Box<dyn Read + Send> = if had_shared_close_io {
                    reader
                } else {
                    Box::new(PtyReaderProxy {
                        reader,
                        close_io: Arc::clone(&close_io),
                        scanner: CursorPositionQueryScanner::default(),
                    })
                };
                Arc::new(Mutex::new(Some(reader)))
            });
            let owner_for_worker = Arc::clone(&owner);
            if let Ok(worker) = thread::Builder::new()
                .name("rterm-pty-close-drain".to_owned())
                .spawn(move || {
                    let Some(mut reader) = owner_for_worker
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .take()
                    else {
                        return Ok(());
                    };
                    io::copy(&mut reader, &mut io::sink()).map(|_| ())
                })
            {
                self.reader_close = Some(worker);
            } else {
                self.reader_owner = Some(owner);
                return false;
            }
        }
        close_io.begin_close(true);
        drop(self.writer.take());
        self.master_close = Some(PtyMasterClose::start(self.master.take(), close_io));
        true
    }

    fn poll_master_close(&mut self) -> bool {
        let Some(close) = self.master_close.as_mut() else {
            return true;
        };
        let status = close.finish_before(Instant::now());
        if matches!(status, PtyMasterCloseStatus::Deferred) {
            return false;
        }
        observe_reaped_master_close(status);
        drop(self.master_close.take());
        true
    }

    fn poll_cleanup(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else {
            if !self.begin_master_close() {
                return false;
            }
            let close_finished = self.poll_master_close();
            let reader_finished = self
                .reader_close
                .as_ref()
                .is_none_or(thread::JoinHandle::is_finished);
            if close_finished
                && reader_finished
                && let Some(reader) = self.reader_close.take()
            {
                observe_reaped_capture_thread(reader);
            }
            return close_finished && reader_finished;
        };
        if let Ok(Some(_)) = child.try_wait() {
            drop(self.child.take());
            if !self.begin_master_close() {
                return false;
            }
            let close_finished = self.poll_master_close();
            let reader_finished = self
                .reader_close
                .as_ref()
                .is_none_or(thread::JoinHandle::is_finished);
            if close_finished
                && reader_finished
                && let Some(reader) = self.reader_close.take()
            {
                observe_reaped_capture_thread(reader);
            }
            close_finished && reader_finished
        } else {
            if !self.kill_sent {
                self.kill_sent = child.kill().is_ok();
            }
            false
        }
    }
}

fn acquire_pty_streams(
    mut owned: PtyOwnedProcess,
) -> Result<PtyOwnedProcess, Box<(PtyError, PtyOwnedProcess)>> {
    #[cfg(test)]
    let fault = STREAM_ACQUISITION_FAULT.with(|fault| fault.replace(0));
    #[cfg(not(test))]
    let fault = 0;

    let reader = match owned
        .master
        .as_ref()
        .expect("PTY master is present during stream acquisition")
        .try_clone_reader()
    {
        Ok(reader) => reader,
        Err(error) => return Err(Box::new((PtyError::Backend(error.to_string()), owned))),
    };
    owned.reader = Some(reader);
    let writer = match owned
        .master
        .as_ref()
        .expect("PTY master is present during stream acquisition")
        .take_writer()
    {
        Ok(writer) => writer,
        Err(error) => return Err(Box::new((PtyError::Backend(error.to_string()), owned))),
    };
    owned.writer = Some(writer);

    let injected_error = match fault {
        1 => Some("injected PTY reader acquisition failure"),
        2 => Some("injected PTY writer acquisition failure"),
        _ => None,
    };
    if let Some(message) = injected_error {
        return Err(Box::new((PtyError::Backend(message.to_owned()), owned)));
    }
    Ok(owned)
}

type PtyAcquiredStreams = (Box<dyn Read + Send>, Box<dyn Write + Send>);

fn take_acquired_streams(owned: &mut PtyOwnedProcess) -> Result<PtyAcquiredStreams, PtyError> {
    let reader = owned
        .reader
        .take()
        .ok_or_else(|| PtyError::Backend("PTY reader ownership is missing".to_owned()))?;
    let Some(writer) = owned.writer.take() else {
        owned.reader = Some(reader);
        return Err(PtyError::Backend(
            "PTY writer ownership is missing".to_owned(),
        ));
    };
    Ok((reader, writer))
}

fn settle_failed_stream_acquisition(mut owned: PtyOwnedProcess) {
    const CLEANUP_BUDGET: Duration = Duration::from_millis(500);
    let cleanup_deadline = Instant::now()
        .checked_add(CLEANUP_BUDGET)
        .expect("bounded cleanup deadline fits in Instant");
    #[cfg(test)]
    if FORCE_PROCESS_DEFER.with(|force| force.replace(false)) {
        defer_capture_job(CaptureReapJob::Process(owned));
        return;
    }

    let result = terminate_child_before(
        owned
            .child
            .as_mut()
            .expect("spawned PTY child ownership is present")
            .as_mut(),
        cleanup_deadline.saturating_duration_since(Instant::now()),
        |child| {
            child
                .try_wait()
                .map(|status| status.map(PtyExitStatus::from))
                .map_err(PtyError::Io)
        },
        |child| child.kill().map_err(PtyError::Io),
    );
    if result.is_ok() {
        drop(owned.child.take());
        owned.begin_master_close();
        let mut cleanup_finished = owned.poll_cleanup();
        while !cleanup_finished && Instant::now() < cleanup_deadline {
            thread::park_timeout(
                cleanup_deadline
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(2)),
            );
            cleanup_finished = owned.poll_cleanup();
        }
        if !cleanup_finished {
            defer_capture_job(CaptureReapJob::Process(owned));
        }
    } else {
        defer_capture_job(CaptureReapJob::Process(owned));
    }
}

struct CaptureProgress {
    bytes: Vec<u8>,
    query_scanner: CursorPositionQueryScanner,
    pending_input: Option<Vec<u8>>,
    primary_error: Option<PtyError>,
    child_exited: bool,
    reader_finished: bool,
    phase: CapturePhase,
}

enum CapturePhase {
    Operating,
    Cleanup { kill_sent: bool },
}

impl CaptureProgress {
    fn new(input: &str) -> Self {
        Self {
            bytes: Vec::new(),
            query_scanner: CursorPositionQueryScanner::default(),
            pending_input: (!input.is_empty()).then(|| input.as_bytes().to_vec()),
            primary_error: None,
            child_exited: false,
            reader_finished: false,
            phase: CapturePhase::Operating,
        }
    }

    fn record_error(&mut self, error: PtyError) {
        self.primary_error.get_or_insert(error);
        if matches!(self.phase, CapturePhase::Operating) {
            self.phase = CapturePhase::Cleanup { kill_sent: false };
        }
    }

    fn write_pending_input(&mut self, writer: &mut dyn Write) {
        let Some(input) = self.pending_input.take() else {
            return;
        };
        if let Err(error) = writer.write_all(&input).and_then(|()| writer.flush()) {
            self.record_error(PtyError::Io(error));
        }
    }

    fn receive_before(&mut self, io: &mut CaptureIo, deadline: Instant) {
        let now = Instant::now();
        if now >= deadline {
            return;
        }
        let wait = (deadline - now).min(Duration::from_millis(5));
        match io.receiver.recv_timeout(wait) {
            Ok(CaptureReadEvent::Chunk(chunk)) => {
                if let Some(writer) = io.writer.as_mut() {
                    match respond_to_cursor_position_queries(
                        &chunk,
                        &mut self.query_scanner,
                        writer.as_mut(),
                    ) {
                        Ok(true) => self.write_pending_input(writer.as_mut()),
                        Ok(false) => {}
                        Err(error) => self.record_error(PtyError::Io(error)),
                    }
                }
                self.bytes.extend_from_slice(&chunk);
            }
            Ok(CaptureReadEvent::Error(error)) => {
                self.reader_finished = true;
                self.record_error(PtyError::Io(error));
            }
            Ok(CaptureReadEvent::Eof) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.reader_finished = true;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }

    fn poll_child(&mut self, io: &mut CaptureIo) {
        if self.child_exited {
            return;
        }
        match io.child.try_wait() {
            Ok(Some(_)) => {
                self.child_exited = true;
                io.begin_master_close();
            }
            Ok(None) => {}
            Err(error) => self.record_error(PtyError::Io(error)),
        }
    }

    fn cleanup_started(&self) -> bool {
        matches!(self.phase, CapturePhase::Cleanup { .. })
    }

    fn start_cleanup(&mut self, timeout: Duration) {
        self.primary_error.get_or_insert(PtyError::Timeout(timeout));
        if matches!(self.phase, CapturePhase::Operating) {
            self.phase = CapturePhase::Cleanup { kill_sent: false };
        }
    }

    fn perform_cleanup(&mut self, io: &mut CaptureIo) {
        let CapturePhase::Cleanup { kill_sent } = &mut self.phase else {
            return;
        };
        io.close_io.begin_close(false);
        drop(io.writer.take());
        if !self.child_exited && !*kill_sent {
            if let Err(error) = io.child.kill() {
                self.primary_error.get_or_insert(PtyError::Io(error));
            }
            *kill_sent = true;
        }
    }
}

impl PtySession {
    /// Spawn a command inside a new platform PTY.
    ///
    /// # Errors
    ///
    /// Returns an error when the command is invalid, the PTY backend cannot be
    /// opened, the child process cannot be spawned, or PTY streams cannot be
    /// acquired.
    pub fn spawn(command: &PtyCommand, size: PtySize) -> Result<Self, PtyError> {
        command.validate()?;

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(size.to_portable())
            .map_err(|error| PtyError::Backend(error.to_string()))?;

        let child = pair
            .slave
            .spawn_command(command.to_builder())
            .map_err(|error| PtyError::Backend(error.to_string()))?;
        drop(pair.slave);

        let mut owned = match acquire_pty_streams(PtyOwnedProcess::new(child, pair.master)) {
            Ok(owned) => owned,
            Err(error_and_owned) => {
                let (error, owned) = *error_and_owned;
                settle_failed_stream_acquisition(owned);
                return Err(error);
            }
        };
        let Some(child) = owned.child.as_ref() else {
            settle_failed_stream_acquisition(owned);
            return Err(PtyError::Backend(
                "spawned PTY child was lost before stream acquisition".to_owned(),
            ));
        };
        let killer = child.clone_killer();
        let (reader, writer) = match take_acquired_streams(&mut owned) {
            Ok(streams) => streams,
            Err(error) => {
                settle_failed_stream_acquisition(owned);
                return Err(error);
            }
        };
        let close_io = PtyCloseIo::new(writer);
        let interrupt = PtySessionInterrupt::new(killer, Arc::clone(&close_io));
        let reader: Box<dyn Read + Send> = Box::new(PtyReaderProxy {
            reader,
            close_io: Arc::clone(&close_io),
            scanner: CursorPositionQueryScanner::default(),
        });
        let writer: Box<dyn Write + Send> = Box::new(PtyWriterProxy {
            close_io: Arc::clone(&close_io),
        });

        Ok(Self {
            master: owned.master,
            child: owned.child,
            reader: Some(reader),
            writer: Some(writer),
            close_io,
            interrupt,
        })
    }

    /// Spawn a command and collect PTY output until the child exits.
    ///
    /// # Errors
    ///
    /// Returns an error when the command cannot be spawned, PTY output cannot be
    /// read, or the operation exceeds `timeout`.
    pub fn capture_output(
        command: &PtyCommand,
        size: PtySize,
        timeout: Duration,
    ) -> Result<Vec<u8>, PtyError> {
        Self::capture_with_input(command, "", size, timeout)
    }

    /// Spawn the platform shell, write input, and collect output until exit.
    ///
    /// # Errors
    ///
    /// Returns an error when the shell cannot be spawned, writing input fails,
    /// reading output fails, or the operation exceeds `timeout`.
    pub fn capture_shell_output(
        input: &str,
        size: PtySize,
        timeout: Duration,
    ) -> Result<Vec<u8>, PtyError> {
        let command = PtyCommand::default_shell();
        Self::capture_with_input(&command, input, size, timeout)
    }

    fn capture_with_input(
        command: &PtyCommand,
        input: &str,
        size: PtySize,
        timeout: Duration,
    ) -> Result<Vec<u8>, PtyError> {
        command.validate()?;

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(size.to_portable())
            .map_err(|error| PtyError::Backend(error.to_string()))?;

        let child = pair
            .slave
            .spawn_command(command.to_builder())
            .map_err(|error| PtyError::Backend(error.to_string()))?;
        drop(pair.slave);
        let mut owned = match acquire_pty_streams(PtyOwnedProcess::new(child, pair.master)) {
            Ok(owned) => owned,
            Err(error_and_owned) => {
                let (error, owned) = *error_and_owned;
                settle_failed_stream_acquisition(owned);
                return Err(error);
            }
        };
        let (reader, writer) = match take_acquired_streams(&mut owned) {
            Ok(streams) => streams,
            Err(error) => {
                settle_failed_stream_acquisition(owned);
                return Err(error);
            }
        };
        let close_io = PtyCloseIo::new(writer);
        let reader: Box<dyn Read + Send> = Box::new(PtyReaderProxy {
            reader,
            close_io: Arc::clone(&close_io),
            scanner: CursorPositionQueryScanner::default(),
        });
        let writer: Box<dyn Write + Send> = Box::new(PtyWriterProxy {
            close_io: Arc::clone(&close_io),
        });

        let (receiver, reader_thread) = spawn_capture_reader(reader);
        run_capture_io(
            CaptureIo {
                child: owned
                    .child
                    .take()
                    .expect("capture PTY child is present after acquisition"),
                master: owned.master.take(),
                writer: Some(writer),
                receiver,
                reader_thread: Some(reader_thread),
                close_io,
                master_close: None,
            },
            input,
            timeout,
        )
    }

    /// Borrow the PTY reader stream.
    ///
    /// # Panics
    ///
    /// Panics when the reader has already been moved out with
    /// [`PtySession::take_reader`].
    pub fn reader(&mut self) -> &mut dyn Read {
        self.reader
            .as_mut()
            .expect("PTY reader was already taken")
            .as_mut()
    }

    /// Borrow the PTY writer stream.
    ///
    /// # Panics
    ///
    /// Panics when the writer has already been moved out with
    /// [`PtySession::take_writer`].
    pub fn writer(&mut self) -> &mut dyn Write {
        self.writer
            .as_mut()
            .expect("PTY writer was already taken")
            .as_mut()
    }

    /// Move the PTY reader stream out of the session.
    ///
    /// # Errors
    ///
    /// Returns [`PtyError::StreamTaken`] when the reader was already moved out.
    pub fn take_reader(&mut self) -> Result<Box<dyn Read + Send>, PtyError> {
        self.reader.take().ok_or(PtyError::StreamTaken("reader"))
    }

    /// Move the PTY writer stream out of the session.
    ///
    /// # Errors
    ///
    /// Returns [`PtyError::StreamTaken`] when the writer was already moved out.
    pub fn take_writer(&mut self) -> Result<Box<dyn Write + Send>, PtyError> {
        self.writer.take().ok_or(PtyError::StreamTaken("writer"))
    }

    /// Returns a cloneable handle that terminates the child out of band.
    #[must_use]
    pub fn interrupt_handle(&self) -> PtySessionInterrupt {
        self.interrupt.clone()
    }

    /// Begin closing every PTY master-side stream owned by this session.
    ///
    /// On Windows the potentially blocking pseudoconsole close runs on a named
    /// owned worker while an external reader can continue draining final output.
    /// The returned token owns that worker until completion or transfer.
    #[must_use]
    pub fn begin_master_close(&mut self) -> PtyMasterClose {
        let child_reaped = self
            .child
            .as_mut()
            .is_none_or(|child| matches!(child.try_wait(), Ok(Some(_))));
        self.close_io.begin_close(child_reaped);
        drop(self.writer.take());
        drop(self.reader.take());
        PtyMasterClose::start(self.master.take(), Arc::clone(&self.close_io))
    }

    /// Start closing every PTY master-side stream without synchronously waiting.
    ///
    /// Calling this method more than once is harmless. Once the child has
    /// exited, an external reader returned by [`PtySession::take_reader`] must
    /// continue draining until EOF. Use [`PtySession::begin_master_close`] when
    /// the caller needs to observe or group the owned close operation.
    pub fn close_master(&mut self) {
        drop(self.begin_master_close());
    }

    /// Resize the PTY in character cells.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend rejects the resize operation.
    pub fn resize(&mut self, size: PtySize) -> Result<(), PtyError> {
        self.master
            .as_ref()
            .ok_or_else(|| PtyError::Backend("PTY master was already closed".to_owned()))?
            .resize(size.to_portable())
            .map_err(|error| PtyError::Backend(error.to_string()))
    }

    /// Return the child process identifier when the backend exposes one.
    #[must_use]
    pub fn process_id(&self) -> Option<u32> {
        self.child.as_ref().and_then(|child| child.process_id())
    }

    /// Return the platform tty name when the backend exposes one.
    #[must_use]
    pub fn tty_name(&self) -> Option<String> {
        #[cfg(unix)]
        {
            self.master
                .as_ref()
                .and_then(|master| master.tty_name())
                .map(|path| path.to_string_lossy().into_owned())
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    /// Read one blocking chunk from the PTY reader.
    ///
    /// This compatibility API has no internal deadline and can block forever.
    /// New lifecycle code should move the reader to an owned worker and enforce
    /// an explicit shutdown deadline.
    ///
    /// # Errors
    ///
    /// Returns an error when the reader stream fails.
    pub fn read_blocking(&mut self) -> Result<Vec<u8>, PtyError> {
        let mut buffer = [0; 8192];
        let count = self.reader().read(&mut buffer)?;

        Ok(buffer[..count].to_vec())
    }

    /// Wait until the child process exits.
    ///
    /// This compatibility API is intentionally unbounded. Prefer
    /// [`PtySession::wait_for_exit`] in production lifecycle code.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend wait operation fails.
    pub fn wait(&mut self) -> Result<PtyExitStatus, PtyError> {
        self.child
            .as_mut()
            .ok_or_else(|| PtyError::Backend("PTY child ownership was transferred".to_owned()))?
            .wait()
            .map(PtyExitStatus::from)
            .map_err(|error| PtyError::Backend(error.to_string()))
    }

    /// Wait until the child exits or the supplied deadline expires.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend status check fails or the child does
    /// not exit within `timeout`.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Result<PtyExitStatus, PtyError> {
        let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
            PtyError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PTY exit timeout exceeds the platform instant range",
            ))
        })?;
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(PtyError::Timeout(timeout));
            }
            let wait = deadline
                .saturating_duration_since(now)
                .min(Duration::from_millis(5));
            thread::park_timeout(wait);
        }
    }

    /// Terminate the child process.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot terminate the child.
    pub fn kill(&mut self) -> Result<(), PtyError> {
        self.child
            .as_mut()
            .ok_or_else(|| PtyError::Backend("PTY child ownership was transferred".to_owned()))?
            .kill()
            .map_err(PtyError::Io)
    }

    /// Terminate the child and confirm that it has been reaped before the
    /// supplied deadline.
    ///
    /// A child that already exited is returned without attempting to kill it.
    /// If termination fails, an observed exit takes precedence over the kill
    /// error; otherwise the kill error is returned.
    ///
    /// # Errors
    ///
    /// Returns an error when termination or status polling fails, or when the
    /// child cannot be reaped within `timeout`.
    pub fn terminate(&mut self, timeout: Duration) -> Result<PtyExitStatus, PtyError> {
        terminate_child_before(
            self.child
                .as_mut()
                .ok_or_else(|| PtyError::Backend("PTY child ownership was transferred".to_owned()))?
                .as_mut(),
            timeout,
            |child| {
                child
                    .try_wait()
                    .map(|status| status.map(PtyExitStatus::from))
                    .map_err(PtyError::Io)
            },
            |child| child.kill().map_err(PtyError::Io),
        )
    }

    /// Check whether the child process has exited without blocking.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend status check fails.
    pub fn try_wait(&mut self) -> Result<Option<PtyExitStatus>, PtyError> {
        self.child
            .as_mut()
            .ok_or_else(|| PtyError::Backend("PTY child ownership was transferred".to_owned()))?
            .try_wait()
            .map(|status| status.map(PtyExitStatus::from))
            .map_err(PtyError::Io)
    }
}

fn terminate_child_before<T, TryWait, Kill>(
    child: &mut T,
    timeout: Duration,
    mut try_wait: TryWait,
    mut kill: Kill,
) -> Result<PtyExitStatus, PtyError>
where
    T: ?Sized,
    TryWait: FnMut(&mut T) -> Result<Option<PtyExitStatus>, PtyError>,
    Kill: FnMut(&mut T) -> Result<(), PtyError>,
{
    let deadline = Instant::now().checked_add(timeout);
    let mut primary_error = match try_wait(child) {
        Ok(Some(status)) => return Ok(status),
        Ok(None) => None,
        Err(error) => Some(error),
    };
    if let Err(error) = kill(child) {
        primary_error.get_or_insert(error);
    }
    loop {
        match try_wait(child) {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(error) => {
                primary_error.get_or_insert(error);
            }
        }

        let now = Instant::now();
        if deadline.is_some_and(|deadline| now >= deadline) {
            return Err(primary_error.unwrap_or(PtyError::Timeout(timeout)));
        }
        let wait = deadline
            .map_or(Duration::from_millis(5), |deadline| {
                deadline.saturating_duration_since(now)
            })
            .min(Duration::from_millis(5));
        thread::park_timeout(wait);
    }
}

fn spawn_capture_reader(
    mut reader: Box<dyn Read + Send>,
) -> (mpsc::Receiver<CaptureReadEvent>, CaptureReaderThread) {
    let (sender, receiver) = mpsc::channel();
    let reader_thread = thread::spawn(move || -> io::Result<()> {
        let mut buffer = [0_u8; 4096];
        loop {
            let event = match reader.read(&mut buffer) {
                Ok(0) => CaptureReadEvent::Eof,
                Ok(count) => CaptureReadEvent::Chunk(buffer[..count].to_vec()),
                Err(error) => CaptureReadEvent::Error(error),
            };
            let finished = !matches!(event, CaptureReadEvent::Chunk(_));
            if sender.send(event).is_err() || finished {
                return Ok(());
            }
        }
    });
    (receiver, reader_thread)
}

#[cfg(test)]
fn pending_capture_cleanup_count() -> usize {
    CAPTURE_REAPER_PENDING.load(Ordering::SeqCst)
}

#[cfg(test)]
fn pending_master_close_count() -> usize {
    MASTER_CLOSE_REAPER_PENDING.load(Ordering::SeqCst)
}

#[cfg(test)]
fn capture_cleanup_panic_count() -> usize {
    CAPTURE_REAPER_PANICS.load(Ordering::SeqCst)
}

#[cfg(test)]
fn capture_reaper_deferred_count() -> usize {
    CAPTURE_REAPER_DEFERRED.load(Ordering::SeqCst)
}

#[cfg(test)]
fn capture_reaper_retained_count() -> usize {
    CAPTURE_REAPER_RETAINED.get().map_or(0, |jobs| {
        jobs.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    })
}

#[cfg(test)]
fn capture_reaper_last_process_ownership() -> usize {
    CAPTURE_REAPER_LAST_PROCESS_OWNERSHIP.load(Ordering::SeqCst)
}

#[cfg(test)]
fn take_capture_reaper_errors() -> Vec<PtyMasterCloseError> {
    CAPTURE_REAPER_ERRORS.get().map_or_else(Vec::new, |errors| {
        errors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .drain(..)
            .collect()
    })
}

#[cfg(test)]
fn capture_reaper_error_count() -> usize {
    CAPTURE_REAPER_ERRORS.get().map_or(0, |errors| {
        errors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    })
}

fn capture_reaper() -> &'static CaptureReaperInitialization {
    CAPTURE_REAPER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel();
        match thread::Builder::new()
            .name("rterm-pty-capture-reaper".to_owned())
            .spawn(move || run_capture_reaper(&receiver))
        {
            Ok(_) => CaptureReaperInitialization::Ready(sender),
            Err(_) => CaptureReaperInitialization::Failed,
        }
    })
}

fn defer_capture_job(job: CaptureReapJob) {
    #[cfg(test)]
    if let CaptureReapJob::Process(owned) = &job {
        let ownership = usize::from(owned.child.is_some())
            | (usize::from(owned.master.is_some()) << 1)
            | (usize::from(owned.reader.is_some()) << 2)
            | (usize::from(owned.writer.is_some()) << 3);
        CAPTURE_REAPER_LAST_PROCESS_OWNERSHIP.store(ownership, Ordering::SeqCst);
    }
    if job.counts_as_capture_cleanup() {
        CAPTURE_REAPER_PENDING.fetch_add(1, Ordering::SeqCst);
        #[cfg(test)]
        CAPTURE_REAPER_DEFERRED.fetch_add(1, Ordering::SeqCst);
    }
    if job.counts_as_master_close() {
        MASTER_CLOSE_REAPER_PENDING.fetch_add(1, Ordering::SeqCst);
    }
    let job = match capture_reaper() {
        CaptureReaperInitialization::Ready(sender) => match sender.send(job) {
            Ok(()) => return,
            Err(error) => error.0,
        },
        CaptureReaperInitialization::Failed => job,
    };

    // If the process-lifetime worker could not be created or unexpectedly
    // stopped, retain ownership for the remainder of the process. Dropping a
    // JoinHandle here would silently detach the reader thread.
    CAPTURE_REAPER_RETAINED
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(job);
}

#[cfg(windows)]
fn retain_capture_job(job: CaptureReapJob) {
    if job.counts_as_capture_cleanup() {
        CAPTURE_REAPER_PENDING.fetch_add(1, Ordering::SeqCst);
    }
    CAPTURE_REAPER_RETAINED
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(job);
}

fn run_capture_reaper(receiver: &mpsc::Receiver<CaptureReapJob>) {
    let mut active = Vec::new();
    loop {
        if active.is_empty() {
            let Ok(job) = receiver.recv() else {
                return;
            };
            active.push(job);
        }
        while let Ok(job) = receiver.try_recv() {
            active.push(job);
        }

        let mut index = active.len();
        while index > 0 {
            index -= 1;
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                poll_capture_reap_job(&mut active[index])
            })) {
                Ok(true) => {
                    let counted = active[index].counts_as_capture_cleanup();
                    let master_close = active[index].counts_as_master_close();
                    active.swap_remove(index);
                    if counted {
                        CAPTURE_REAPER_PENDING.fetch_sub(1, Ordering::SeqCst);
                    }
                    if master_close {
                        MASTER_CLOSE_REAPER_PENDING.fetch_sub(1, Ordering::SeqCst);
                    }
                }
                Ok(false) => {}
                Err(_) => {
                    CAPTURE_REAPER_PANICS.fetch_add(1, Ordering::SeqCst);
                    CAPTURE_REAPER_RETAINED
                        .get_or_init(|| Mutex::new(Vec::new()))
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(active.swap_remove(index));
                }
            }
        }
        if !active.is_empty() {
            thread::park_timeout(Duration::from_millis(5));
        }
    }
}

fn poll_capture_reap_job(job: &mut CaptureReapJob) -> bool {
    match job {
        CaptureReapJob::Reader(worker) => {
            let Some(reader_thread) = worker.as_ref() else {
                return true;
            };
            if !reader_thread.is_finished() {
                return false;
            }
            observe_capture_thread(worker.take().expect("reader thread is present"));
            true
        }
        CaptureReapJob::Io(io) => {
            io.close_io.begin_close(false);
            drop(io.writer.take());
            let child_exited = if let Ok(Some(_)) = io.child.try_wait() {
                true
            } else {
                let _ = io.child.kill();
                false
            };
            if !child_exited {
                return false;
            }
            io.begin_master_close();
            let close_finished = io.poll_master_close();
            let reader_finished = io
                .reader_thread
                .as_ref()
                .is_none_or(thread::JoinHandle::is_finished);
            if !close_finished || !reader_finished {
                return false;
            }
            if let Some(reader) = io.reader_thread.take() {
                observe_reaped_capture_thread(reader);
            }
            true
        }
        CaptureReapJob::Process(owned) => owned.poll_cleanup(),
        CaptureReapJob::MasterClose(close) => {
            let Some(handle) = close.worker.as_ref() else {
                return true;
            };
            if !handle.is_finished() {
                return false;
            }
            if close
                .worker
                .take()
                .expect("master-close worker is present")
                .join()
                .is_err()
            {
                CAPTURE_REAPER_PANICS.fetch_add(1, Ordering::SeqCst);
            } else if let Some(error) = close.close_io.error() {
                eprintln!("deferred PTY master close observed an I/O error: {error}");
                CAPTURE_REAPER_ERRORS
                    .get_or_init(|| Mutex::new(VecDeque::new()))
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push_back(error);
            }
            true
        }
        #[cfg(windows)]
        CaptureReapJob::MasterOwner(owner) => {
            let _ = Arc::strong_count(owner);
            false
        }
        #[cfg(test)]
        CaptureReapJob::Test(poll) => poll(),
    }
}

fn observe_capture_thread(worker: CaptureReaderThread) -> CaptureThreadJoin {
    if let Ok(result) = worker.join() {
        CaptureThreadJoin::Completed(result)
    } else {
        CAPTURE_REAPER_PANICS.fetch_add(1, Ordering::SeqCst);
        CaptureThreadJoin::Panicked
    }
}

fn observe_reaped_capture_thread(worker: CaptureReaderThread) {
    if let CaptureThreadJoin::Completed(Err(error)) = observe_capture_thread(worker) {
        eprintln!("deferred PTY reader cleanup failed: {error}");
    }
}

fn join_capture_thread_before(worker: CaptureReaderThread, deadline: Instant) -> CaptureThreadJoin {
    while !worker.is_finished() && Instant::now() < deadline {
        thread::park_timeout(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(2)),
        );
    }
    if worker.is_finished() {
        observe_capture_thread(worker)
    } else {
        defer_capture_job(CaptureReapJob::Reader(Some(worker)));
        CaptureThreadJoin::Deferred
    }
}

fn run_capture_io(mut io: CaptureIo, input: &str, timeout: Duration) -> Result<Vec<u8>, PtyError> {
    let started = Instant::now();
    let deadline = started.checked_add(timeout).unwrap_or(started);
    let cleanup_reserve = timeout.min(Duration::from_millis(500));
    let operation_deadline = deadline.checked_sub(cleanup_reserve).unwrap_or(started);
    let mut progress = CaptureProgress::new(input);

    if !cfg!(windows)
        && let Some(writer) = io.writer.as_mut()
    {
        progress.write_pending_input(writer.as_mut());
    }

    loop {
        progress.poll_child(&mut io);
        if progress.child_exited && progress.reader_finished {
            return finish_capture_io(io, progress, deadline, timeout);
        }

        let now = Instant::now();
        if !progress.cleanup_started() && now >= operation_deadline {
            progress.start_cleanup(timeout);
        }
        progress.perform_cleanup(&mut io);
        progress.poll_child(&mut io);
        if progress.child_exited && progress.reader_finished {
            return finish_capture_io(io, progress, deadline, timeout);
        }
        if Instant::now() >= deadline {
            progress.start_cleanup(timeout);
            progress.perform_cleanup(&mut io);
            progress.poll_child(&mut io);
            if progress.child_exited && progress.reader_finished {
                return finish_capture_io(io, progress, deadline, timeout);
            }

            let error = progress.primary_error.unwrap_or(PtyError::Timeout(timeout));
            defer_capture_job(CaptureReapJob::Io(io));
            return Err(error);
        }

        let phase_deadline = if progress.cleanup_started() {
            deadline
        } else {
            operation_deadline
        };
        progress.receive_before(&mut io, phase_deadline);
    }
}

fn finish_capture_io(
    mut io: CaptureIo,
    mut progress: CaptureProgress,
    deadline: Instant,
    timeout: Duration,
) -> Result<Vec<u8>, PtyError> {
    io.begin_master_close();
    let Some(reader_thread) = io.reader_thread.take() else {
        progress.record_error(PtyError::Backend(
            "PTY capture reader ownership was lost".to_owned(),
        ));
        return match progress.primary_error {
            Some(error) => Err(error),
            None => Ok(progress.bytes),
        };
    };
    match join_capture_thread_before(reader_thread, deadline) {
        CaptureThreadJoin::Completed(Ok(())) => {}
        CaptureThreadJoin::Completed(Err(error)) => {
            progress.record_error(PtyError::Io(error));
        }
        CaptureThreadJoin::Panicked => {
            progress.record_error(PtyError::Backend("PTY reader thread panicked".to_owned()));
        }
        CaptureThreadJoin::Deferred => progress.start_cleanup(timeout),
    }
    if let Some(close) = io.master_close.as_mut() {
        match close.finish_before(deadline) {
            PtyMasterCloseStatus::Completed => {}
            PtyMasterCloseStatus::Failed(error) => {
                progress.record_error(PtyError::Backend(error.to_string()));
            }
            PtyMasterCloseStatus::Panicked => {
                progress.record_error(PtyError::Backend(
                    "PTY master-close worker panicked".to_owned(),
                ));
            }
            PtyMasterCloseStatus::Deferred => progress.start_cleanup(timeout),
            PtyMasterCloseStatus::Retained => {
                progress.record_error(PtyError::Backend(
                    "PTY master close retained for process lifetime".to_owned(),
                ));
            }
        }
    }
    match progress.primary_error {
        Some(error) => Err(error),
        None => Ok(progress.bytes),
    }
}

#[derive(Default)]
struct CursorPositionQueryScanner {
    matched: usize,
}

impl CursorPositionQueryScanner {
    fn observe(&mut self, chunk: &[u8]) -> usize {
        const QUERY: &[u8] = b"\x1b[6n";

        let mut queries = 0;
        for &byte in chunk {
            if byte == QUERY[self.matched] {
                self.matched += 1;
                if self.matched == QUERY.len() {
                    queries += 1;
                    self.matched = 0;
                }
            } else {
                self.matched = usize::from(byte == QUERY[0]);
            }
        }
        queries
    }

    fn scan(&mut self, chunk: &[u8], writer: &mut dyn Write) -> io::Result<bool> {
        const RESPONSE: &[u8] = b"\x1b[1;1R";

        let queries = self.observe(chunk);
        for _ in 0..queries {
            writer.write_all(RESPONSE)?;
        }
        let answered = queries > 0;
        if answered {
            writer.flush()?;
        }
        Ok(answered)
    }

    #[cfg(test)]
    const fn buffered_len(&self) -> usize {
        self.matched
    }
}

#[derive(Default)]
struct CursorPositionReplyScanner {
    state: u8,
}

impl CursorPositionReplyScanner {
    fn observe(&mut self, chunk: &[u8]) -> usize {
        let mut replies = 0;
        for &byte in chunk {
            self.state = match (self.state, byte) {
                (_, b'\x1b') => 1,
                (1, b'[') => 2,
                (2 | 3, b'0'..=b'9') => 3,
                (3, b';') => 4,
                (4 | 5, b'0'..=b'9') => 5,
                (5, b'R') => {
                    replies += 1;
                    0
                }
                _ => 0,
            };
        }
        replies
    }
}

fn respond_to_cursor_position_queries(
    chunk: &[u8],
    scanner: &mut CursorPositionQueryScanner,
    writer: &mut dyn Write,
) -> io::Result<bool> {
    scanner.scan(chunk, writer)
}

impl Drop for PtySession {
    fn drop(&mut self) {
        const DROP_BUDGET: Duration = Duration::from_millis(500);
        // Use one total child-termination budget, then close master-side streams
        // in writer/reader/master order. `terminate` never starts a fresh budget
        // after a failed kill or status poll.
        if self.child.is_none() {
            self.close_master();
            return;
        }
        #[cfg(test)]
        let force_defer = FORCE_SESSION_DROP_DEFER.with(|force| force.replace(false));
        #[cfg(not(test))]
        let force_defer = false;
        if !force_defer && self.terminate(DROP_BUDGET).is_ok() {
            self.close_master();
            drop(self.child.take());
            return;
        }

        defer_capture_job(CaptureReapJob::Process(PtyOwnedProcess {
            child: self.child.take(),
            master: self.master.take(),
            reader: self.reader.take(),
            writer: self.writer.take(),
            kill_sent: false,
            close_io: Some(Arc::clone(&self.close_io)),
            master_close: None,
            reader_close: None,
            reader_owner: None,
        }));
    }
}

#[derive(Debug)]
pub enum PtyError {
    InvalidCommand(String),
    InvalidSize { columns: u16, rows: u16 },
    Io(io::Error),
    Backend(String),
    Timeout(Duration),
    StreamTaken(&'static str),
}

impl Display for PtyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommand(message) | Self::Backend(message) => formatter.write_str(message),
            Self::InvalidSize { columns, rows } => {
                write!(
                    formatter,
                    "invalid PTY size: {columns} columns, {rows} rows"
                )
            }
            Self::Io(error) => Display::fmt(error, formatter),
            Self::Timeout(timeout) => {
                write!(formatter, "PTY operation timed out after {timeout:?}")
            }
            Self::StreamTaken(stream) => write!(formatter, "PTY {stream} stream was already taken"),
        }
    }
}

impl Error for PtyError {}

impl From<io::Error> for PtyError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
