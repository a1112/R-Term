use std::{
    error::Error,
    io::{Read, Write},
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use rterm_pty::{PtySession, PtySize};
use serde::Serialize;

use crate::cli::SelfTestOptions;

const SELF_TEST_MARKER: &str = "rterm-self-test";
const SELF_TEST_COLUMNS: u16 = 80;
const SELF_TEST_ROWS: u16 = 24;
const SELF_TEST_TIMEOUT: Duration = Duration::from_secs(5);
const OPENSSH_TOOL_TIMEOUT_DETAIL: &str = "tool did not produce expected startup output";

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct SelfTestReport {
    pub ok: bool,
    pub elapsed_ms: u128,
    pub checks: Vec<SelfTestCheck>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct SelfTestCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
    pub output_bytes: usize,
}

pub fn print_self_test(options: &SelfTestOptions) -> Result<(), Box<dyn Error>> {
    let report = run_self_test();

    if options.json {
        println!("{}", self_test_json(&report)?);
    } else {
        for line in self_test_text_lines(&report) {
            println!("{line}");
        }
    }

    if report.ok {
        Ok(())
    } else {
        Err("self-test failed".into())
    }
}

fn run_self_test() -> SelfTestReport {
    let started = Instant::now();
    let size = PtySize::try_new(SELF_TEST_COLUMNS, SELF_TEST_ROWS)
        .expect("self-test PTY dimensions are non-zero");
    let output = capture_local_pty_self_test_output(size, SELF_TEST_TIMEOUT);
    let elapsed_ms = started.elapsed().as_millis();

    let local_pty_check = match output {
        Ok(output) => local_pty_self_test_report_from_output(&output, elapsed_ms),
        Err(error) => SelfTestReport {
            ok: false,
            elapsed_ms,
            checks: vec![SelfTestCheck {
                name: "local-pty".to_owned(),
                ok: false,
                detail: error.to_string(),
                output_bytes: 0,
            }],
        },
    };
    let mut checks = local_pty_check.checks;
    checks.extend(run_openssh_tool_self_tests());

    SelfTestReport {
        ok: checks.iter().all(|check| check.ok),
        elapsed_ms,
        checks,
    }
}

fn self_test_shell_input() -> String {
    format!("echo {SELF_TEST_MARKER}\r\n")
}

fn capture_local_pty_self_test_output(
    size: PtySize,
    timeout: Duration,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut session = PtySession::spawn(&rterm_pty::PtyCommand::default_shell(), size)?;
    let mut reader = session.take_reader()?;
    let mut writer = session.take_writer()?;
    let (sender, receiver) = mpsc::channel();

    let reader_thread = thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    let _ = sender.send(Ok(Vec::new()));
                    return;
                }
                Ok(count) => {
                    if sender.send(Ok(buffer[..count].to_vec())).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(error));
                    return;
                }
            }
        }
    });

    thread::sleep(Duration::from_millis(300));
    writer.write_all(self_test_shell_input().as_bytes())?;
    writer.flush()?;

    let started = Instant::now();
    let mut output = Vec::new();
    while started.elapsed() < timeout {
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(chunk)) => {
                if chunk.is_empty() {
                    break;
                }
                output.extend_from_slice(&chunk);
                if contains_cursor_position_query(&output) {
                    writer.write_all(b"\x1b[1;1R")?;
                    writer.flush()?;
                }
                if String::from_utf8_lossy(&output).contains(SELF_TEST_MARKER) {
                    break;
                }
            }
            Ok(Err(error)) => return Err(Box::new(error)),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    if !String::from_utf8_lossy(&output).contains(SELF_TEST_MARKER) {
        let _ = session.kill();
        return Err("PTY marker missing".into());
    }

    let _ = writer.write_all(b"exit\r\n");
    let _ = writer.flush();
    drop(writer);
    // The marker capture proves PTY I/O. Some ConPTY hosts report control-exit
    // status during teardown, so cleanup should not override a captured marker.
    let _ = wait_for_self_test_shell_exit(&mut session, timeout);
    drop(session);
    let _ = reader_thread.join();

    Ok(output)
}

fn wait_for_self_test_shell_exit(
    session: &mut PtySession,
    timeout: Duration,
) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if let Some(status) = session.try_wait()? {
            if status.success() {
                return Ok(());
            }
            return Err(format!("self-test shell exited with code {}", status.exit_code()).into());
        }
        thread::sleep(Duration::from_millis(20));
    }

    let _ = session.kill();
    Err("self-test shell did not exit before timeout".into())
}

fn contains_cursor_position_query(bytes: &[u8]) -> bool {
    bytes
        .windows(b"\x1b[6n".len())
        .any(|window| window == b"\x1b[6n")
}

pub fn self_test_json(report: &SelfTestReport) -> Result<String, Box<dyn Error>> {
    Ok(serde_json::to_string(report)?)
}

pub fn self_test_text_lines(report: &SelfTestReport) -> Vec<String> {
    report
        .checks
        .iter()
        .map(|check| {
            if check.ok {
                format!("ok\t{}\t{}", check.name, check.detail)
            } else {
                format!("missing\t{}", check.name)
            }
        })
        .collect()
}

pub fn local_pty_self_test_report_from_output(output: &[u8], elapsed_ms: u128) -> SelfTestReport {
    let text = String::from_utf8_lossy(output);
    let ok = text.contains(SELF_TEST_MARKER);
    let detail = if ok {
        "captured PTY marker".to_owned()
    } else {
        "PTY marker missing".to_owned()
    };

    SelfTestReport {
        ok,
        elapsed_ms,
        checks: vec![SelfTestCheck {
            name: "local-pty".to_owned(),
            ok,
            detail,
            output_bytes: output.len(),
        }],
    }
}

fn run_openssh_tool_self_tests() -> Vec<SelfTestCheck> {
    [
        ("openssh-ssh", "ssh", &["-V"][..], "OpenSSH"),
        ("openssh-sftp", "sftp", &["-h"][..], "usage:"),
        ("openssh-scp", "scp", &["-h"][..], "usage:"),
    ]
    .into_iter()
    .map(|(name, program, args, expected)| run_openssh_tool_check(name, program, args, expected))
    .collect()
}

fn run_openssh_tool_check(
    name: &str,
    program: &str,
    args: &[&str],
    expected_output: &str,
) -> SelfTestCheck {
    match Command::new(program).args(args).output() {
        Ok(output) => openssh_tool_check_from_output(
            name,
            program,
            args,
            output.status.code(),
            &output.stdout,
            &output.stderr,
            expected_output,
        ),
        Err(error) => SelfTestCheck {
            name: name.to_owned(),
            ok: false,
            detail: error.to_string(),
            output_bytes: 0,
        },
    }
}

pub fn openssh_tool_check_from_output(
    name: &str,
    program: &str,
    args: &[&str],
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
    expected_output: &str,
) -> SelfTestCheck {
    let mut output = Vec::with_capacity(stdout.len() + stderr.len());
    output.extend_from_slice(stdout);
    output.extend_from_slice(stderr);
    let output_text = String::from_utf8_lossy(&output);
    let ok = output_text.contains(expected_output);
    let detail = if ok {
        format!(
            "{} {} launched with exit_code={}",
            program,
            args.join(" "),
            exit_code.map_or_else(|| "unknown".to_owned(), |code| code.to_string())
        )
    } else {
        format!(
            "{} {} did not report {}",
            program,
            args.join(" "),
            expected_output
        )
    };

    SelfTestCheck {
        name: name.to_owned(),
        ok,
        detail: if detail.trim().is_empty() {
            OPENSSH_TOOL_TIMEOUT_DETAIL.to_owned()
        } else {
            detail
        },
        output_bytes: output.len(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn self_test_json_reports_local_pty_marker_capture() {
        let report =
            super::local_pty_self_test_report_from_output(b"prompt\r\nrterm-self-test\r\n", 12);
        let json = super::self_test_json(&report).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["checks"][0]["name"], "local-pty");
        assert_eq!(value["checks"][0]["ok"], true);
        assert_eq!(value["checks"][0]["output_bytes"], 25);
        assert_eq!(value["elapsed_ms"], 12);
    }

    #[test]
    fn self_test_text_reports_failed_local_pty_marker_capture() {
        let report = super::local_pty_self_test_report_from_output(b"prompt\r\n", 9);
        let lines = super::self_test_text_lines(&report);

        assert!(!report.ok);
        assert!(lines.iter().any(|line| line == "missing\tlocal-pty"));
    }

    #[test]
    fn self_test_shell_input_waits_for_marker_before_exit() {
        let input = super::self_test_shell_input();

        assert_eq!(input, "echo rterm-self-test\r\n");
    }

    #[test]
    fn detects_cursor_position_query_in_pty_output() {
        assert!(super::contains_cursor_position_query(b"before\x1b[6nafter"));
        assert!(!super::contains_cursor_position_query(b"before-after"));
    }

    #[test]
    fn openssh_tool_check_accepts_expected_output_even_with_usage_exit_code() {
        let check = super::openssh_tool_check_from_output(
            "openssh-sftp",
            "sftp",
            &["-h"],
            Some(1),
            b"",
            b"usage: sftp destination",
            "usage:",
        );

        assert!(check.ok);
        assert_eq!(check.name, "openssh-sftp");
        assert_eq!(check.output_bytes, 23);
    }

    #[test]
    fn openssh_tool_check_rejects_missing_expected_output() {
        let check = super::openssh_tool_check_from_output(
            "openssh-ssh",
            "ssh",
            &["-V"],
            Some(0),
            b"",
            b"unexpected output",
            "OpenSSH",
        );

        assert!(!check.ok);
        assert_eq!(check.detail, "ssh -V did not report OpenSSH");
    }
}
