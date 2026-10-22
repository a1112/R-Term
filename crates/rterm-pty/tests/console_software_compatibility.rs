//! Repeatable, bounded compatibility checks for local console programs.
//!
//! The suite deliberately drives the same local PTY and `TerminalRuntime` path
//! used by R-Term.  It does not use the user's shell startup files, network
//! services, or a native/web front end.  Run with `-- --nocapture` to retain
//! the per-scenario evidence lines.

#![cfg(unix)]

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use rterm_pty::{LocalPtyTransport, PtyCommand};
use rterm_runtime::{
    RuntimeBuffers, RuntimeEffectRef, SessionControl, SessionExit, SessionInterrupt,
    SessionTransport, TerminalModeChange, TerminalRuntime,
};
use rterm_types::TerminalSize;

const PTY_SIZE: TerminalSize = TerminalSize::new(80, 24);
const PTY_TIMEOUT: Duration = Duration::from_secs(8);
static TEMP_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
enum InputAction {
    Write { after: Duration, bytes: Vec<u8> },
    Resize { after: Duration, size: TerminalSize },
    Interrupt { after: Duration },
}

#[derive(Debug)]
enum ReadEvent {
    Bytes(Vec<u8>),
    Eof,
    Error(String),
}

struct PtyRun {
    output: Vec<u8>,
    runtime: TerminalRuntime,
    exit: SessionExit,
    mode_changes: Vec<TerminalModeChange>,
    elapsed: Duration,
    saw_alternate_screen: bool,
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new(label: &str) -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rterm-console-{}-{label}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create isolated console fixture directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn find_command(name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.is_absolute() {
        return candidate.exists().then(|| candidate.to_owned());
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

fn find_command_with_fallback(name: &str, fallback: &str) -> Option<PathBuf> {
    find_command(name).or_else(|| {
        let path = Path::new(fallback);
        path.is_file().then(|| path.to_owned())
    })
}

fn shell_command(script: &str, home: &Path) -> PtyCommand {
    PtyCommand::new("/bin/sh")
        .with_args(["-c", script])
        .with_env("HOME", home.to_string_lossy())
        .with_env("XDG_CONFIG_HOME", home.to_string_lossy())
        .with_env("ZDOTDIR", home.to_string_lossy())
        .with_env("GIT_CONFIG_GLOBAL", "/dev/null")
        .with_env("GIT_CONFIG_SYSTEM", "/dev/null")
}

#[allow(clippy::needless_pass_by_value)]
fn command_with_home(program: PathBuf, home: &Path) -> PtyCommand {
    PtyCommand::new(program.to_string_lossy())
        .with_env("HOME", home.to_string_lossy())
        .with_env("XDG_CONFIG_HOME", home.to_string_lossy())
        .with_env("ZDOTDIR", home.to_string_lossy())
        .with_env("GIT_CONFIG_GLOBAL", "/dev/null")
        .with_env("GIT_CONFIG_SYSTEM", "/dev/null")
}

#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
fn run_pty(
    label: &str,
    command: PtyCommand,
    size: TerminalSize,
    mut actions: Vec<InputAction>,
    timeout: Duration,
) -> PtyRun {
    actions.sort_by_key(|action| match action {
        InputAction::Write { after, .. }
        | InputAction::Resize { after, .. }
        | InputAction::Interrupt { after } => *after,
    });

    let transport = LocalPtyTransport::spawn(&command, size)
        .unwrap_or_else(|error| panic!("{label}: spawn local PTY: {error}"));
    let parts = transport.split();
    let mut control = parts.control;
    let interrupt = parts.interrupt;
    let mut writer = parts.writer;
    let mut reader = parts.reader;
    let (events_tx, events_rx) = mpsc::channel();
    let reader_thread = thread::Builder::new()
        .name(format!("rterm-compat-reader-{label}"))
        .spawn(move || {
            let mut chunk = [0_u8; 16 * 1024];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) => {
                        let _ = events_tx.send(ReadEvent::Eof);
                        break;
                    }
                    Ok(count) => {
                        if events_tx
                            .send(ReadEvent::Bytes(chunk[..count].to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = events_tx.send(ReadEvent::Error(error.to_string()));
                        break;
                    }
                }
            }
        })
        .expect("spawn bounded PTY reader thread");

    let started = Instant::now();
    let deadline = started + timeout;
    let mut runtime = TerminalRuntime::new(size);
    runtime.set_capture_host_stream(true);
    let mut buffers = RuntimeBuffers::with_capacity(16 * 1024);
    let mut output = Vec::new();
    let mut mode_changes = Vec::new();
    let mut action_index = 0;
    let mut exit = None;
    let mut exit_seen_at = None;
    let mut reader_done = false;
    let mut saw_alternate_screen = false;
    let mut timed_out = false;
    let mut reader_error = None;

    loop {
        if exit.is_none() {
            exit = control
                .poll_exit()
                .unwrap_or_else(|error| panic!("{label}: poll PTY exit: {error}"));
            if exit.is_some() {
                exit_seen_at = Some(Instant::now());
            }
        }

        let now = Instant::now();
        while exit.is_none() && action_index < actions.len() {
            let action_after = match &actions[action_index] {
                InputAction::Write { after, .. }
                | InputAction::Resize { after, .. }
                | InputAction::Interrupt { after } => *after,
            };
            if now.duration_since(started) < action_after {
                break;
            }
            match &actions[action_index] {
                InputAction::Write { bytes, .. } => {
                    writer
                        .write_all(bytes)
                        .unwrap_or_else(|error| panic!("{label}: write PTY input: {error}"));
                    writer
                        .flush()
                        .unwrap_or_else(|error| panic!("{label}: flush PTY input: {error}"));
                }
                InputAction::Resize { size, .. } => {
                    control
                        .resize(*size)
                        .unwrap_or_else(|error| panic!("{label}: resize PTY: {error}"));
                }
                InputAction::Interrupt { .. } => {
                    interrupt
                        .interrupt()
                        .unwrap_or_else(|error| panic!("{label}: interrupt PTY: {error}"));
                }
            }
            action_index += 1;
        }

        if (reader_done && exit.is_some())
            || exit_seen_at.is_some_and(|seen| seen.elapsed() >= Duration::from_secs(1))
        {
            break;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            break;
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        match events_rx.recv_timeout(remaining.min(Duration::from_millis(10))) {
            Ok(ReadEvent::Bytes(bytes)) => {
                output.extend_from_slice(&bytes);
                let delta = runtime.feed_into(&bytes, &mut buffers);
                for effect in delta.effects() {
                    match effect {
                        RuntimeEffectRef::TransportWrite(bytes) => {
                            if let Err(error) = writer.write_all(bytes) {
                                let exited = control
                                    .poll_exit()
                                    .unwrap_or_else(|poll_error| {
                                        panic!(
                                            "{label}: poll after transport response write error: {poll_error}"
                                        )
                                    })
                                    .is_some();
                                assert!(
                                    exited,
                                    "{label}: transport response write failed while process was live: {error}"
                                );
                            } else if let Err(error) = writer.flush() {
                                let exited = control
                                    .poll_exit()
                                    .unwrap_or_else(|poll_error| {
                                        panic!(
                                            "{label}: poll after transport response flush error: {poll_error}"
                                        )
                                    })
                                    .is_some();
                                assert!(
                                    exited,
                                    "{label}: transport response flush failed while process was live: {error}"
                                );
                            }
                        }
                        RuntimeEffectRef::ModeChange(change) => mode_changes.push(change),
                        _ => {}
                    }
                }
                saw_alternate_screen |= runtime.terminal().alternate_screen_active();
            }
            Ok(ReadEvent::Eof) => reader_done = true,
            Ok(ReadEvent::Error(error)) => {
                reader_error = Some(error);
                reader_done = true;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                reader_done = true;
                if exit.is_some() {
                    break;
                }
                thread::yield_now();
            }
        }
    }

    if timed_out && exit.is_none() {
        let _ = interrupt.interrupt();
        let reap_deadline = Instant::now() + Duration::from_secs(2);
        while exit.is_none() && Instant::now() < reap_deadline {
            exit = control
                .poll_exit()
                .unwrap_or_else(|error| panic!("{label}: poll interrupted PTY exit: {error}"));
            if exit.is_none() {
                thread::yield_now();
            }
        }
    }

    drop(writer);
    control
        .begin_close()
        .unwrap_or_else(|error| panic!("{label}: close PTY: {error}"));
    reader_thread
        .join()
        .unwrap_or_else(|_| panic!("{label}: PTY reader thread panicked"));

    assert!(!timed_out, "{label}: PTY exceeded {timeout:?}");
    assert!(
        reader_error.is_none() || exit.is_some(),
        "{label}: PTY reader failed: {reader_error:?}"
    );
    let exit = exit.unwrap_or_else(|| {
        panic!(
            "{label}: PTY did not report an exit status; output={:?}",
            String::from_utf8_lossy(&output)
        )
    });
    PtyRun {
        output,
        runtime,
        exit,
        mode_changes,
        elapsed: started.elapsed(),
        saw_alternate_screen,
    }
}

fn report(label: &str, run: &PtyRun) {
    println!(
        "compat case={label} elapsed_ms={} bytes={} status={:?} signal={:?} mode_changes={} alt_seen={}",
        run.elapsed.as_millis(),
        run.output.len(),
        run.exit.status,
        run.exit.signal.as_ref().map(|signal| signal.name.as_str()),
        run.mode_changes.len(),
        run.saw_alternate_screen,
    );
}

fn output_text(run: &PtyRun) -> String {
    String::from_utf8_lossy(&run.output).into_owned()
}

fn visible_grid_text(run: &PtyRun) -> String {
    let mut text = String::new();
    for row in (0..run.runtime.terminal().grid().size().rows)
        .filter_map(|row| run.runtime.terminal().grid().row(row))
    {
        for cell in row.cells() {
            if !cell.is_continuation() {
                text.push_str(cell.text());
            }
        }
        text.push('\n');
    }
    text
}

fn assert_output(run: &PtyRun, needle: &str) {
    let output = output_text(run);
    assert!(
        output.contains(needle),
        "PTY output does not contain {needle:?}: {output:?}"
    );
}

fn assert_command_status(run: &PtyRun, status: u32) {
    assert_eq!(
        run.exit.status,
        Some(status),
        "unexpected PTY exit: {:?}",
        run.exit
    );
    assert!(
        run.exit.signal.is_none(),
        "unexpected signal: {:?}",
        run.exit
    );
}

fn run_readline_shell(
    label: &str,
    shell: PathBuf,
    shell_args: &[&str],
    use_bracketed_paste: bool,
    home: &Path,
    fixture: &Path,
) -> PtyRun {
    let paste_payload = if use_bracketed_paste {
        b"\x1b[200~printf 'RTERM-SHELL-RESULT:%s\\n' \"$((40+2))\"\x1b[201~\r".to_vec()
    } else {
        b"printf 'RTERM-SHELL-RESULT:%s\\n' \"$((40+2))\"\r".to_vec()
    };
    run_pty(
        label,
        command_with_home(shell, home)
            .with_args(shell_args.iter().copied())
            .with_env("HISTFILE", "/dev/null")
            .with_env("PS1", "")
            .with_cwd(fixture),
        PTY_SIZE,
        vec![
            InputAction::Write {
                after: Duration::from_millis(200),
                bytes: paste_payload,
            },
            InputAction::Write {
                after: Duration::from_millis(450),
                bytes: b"\x1b[A\r".to_vec(),
            },
            InputAction::Write {
                after: Duration::from_millis(700),
                bytes: b"cat rterm-shell-completion-\t\r".to_vec(),
            },
            InputAction::Write {
                after: Duration::from_millis(950),
                bytes: b"sleep 5\n".to_vec(),
            },
            InputAction::Write {
                after: Duration::from_millis(1_150),
                bytes: vec![3],
            },
            InputAction::Write {
                after: Duration::from_millis(1_350),
                bytes: b"exit 0\n".to_vec(),
            },
        ],
        PTY_TIMEOUT,
    )
}

#[test]
#[allow(clippy::too_many_lines)]
fn console_software_compatibility_matrix() {
    let home = TempDirectory::new("home");
    let fixture = TempDirectory::new("fixture");

    // Keep the inventory in the test output so a report records the exact host
    // capability set used by this run rather than silently assuming tools.
    let inventory = [
        "sh", "bash", "zsh", "dash", "ksh", "git", "rg", "python3", "node", "vim", "vi", "less",
        "nano", "top", "screen", "sqlite3", "perl", "ruby", "awk", "sed", "pwsh", "fzf", "tmux",
        "fish", "htop", "timeout", "gtimeout", "codex", "claude",
    ];
    for command in inventory {
        let available = find_command(command).is_some();
        println!("compat tool={command} available={available}");
        if !available {
            println!("compat inventory-skip tool={command} reason=missing");
        }
    }

    let mut run = run_pty(
        "exit-status",
        shell_command("printf 'RTERM-EXIT\\n'; exit 7", home.path()),
        PTY_SIZE,
        Vec::new(),
        PTY_TIMEOUT,
    );
    report("exit-status", &run);
    assert_output(&run, "RTERM-EXIT");
    assert_command_status(&run, 7);

    run = run_pty(
        "resize",
        shell_command(
            "printf 'SIZE-A:%s\\n' \"$(stty size)\"; sleep 0.25; printf 'SIZE-B:%s\\n' \"$(stty size)\"",
            home.path(),
        ),
        PTY_SIZE,
        vec![InputAction::Resize {
            after: Duration::from_millis(100),
            size: TerminalSize::new(132, 43),
        }],
        PTY_TIMEOUT,
    );
    report("resize", &run);
    assert_output(&run, "SIZE-A:24 80");
    assert_output(&run, "SIZE-B:43 132");
    assert_command_status(&run, 0);

    let combining = "e\u{301}";
    let unicode_script = format!("printf 'CJK:世界 EMOJI:😀 COMB:{combining}\\n'");
    run = run_pty(
        "unicode",
        shell_command(&unicode_script, home.path()),
        PTY_SIZE,
        Vec::new(),
        PTY_TIMEOUT,
    );
    report("unicode", &run);
    let unicode_output = format!("CJK:世界 EMOJI:😀 COMB:{combining}");
    assert_output(&run, &unicode_output);
    let cells = (0..run.runtime.terminal().grid().size().rows)
        .filter_map(|row| run.runtime.terminal().grid().row(row))
        .flat_map(|row| row.cells().iter())
        .collect::<Vec<_>>();
    let cjk = cells
        .iter()
        .find(|cell| cell.text() == "世")
        .expect("CJK cell");
    assert_eq!(cjk.columns(), 2, "CJK character must occupy two columns");
    let emoji = cells
        .iter()
        .find(|cell| cell.text() == "😀")
        .expect("emoji cell");
    assert_eq!(emoji.columns(), 2, "emoji must occupy two columns");
    let combining = cells
        .iter()
        .find(|cell| cell.text() == combining)
        .expect("combining grapheme cell");
    assert_eq!(
        combining.columns(),
        1,
        "combining grapheme must stay in one column"
    );

    run = run_pty(
        "ansi-cursor-scroll",
        shell_command(
            "printf '\\033[31mRTERM-ANSI16\\033[0m\\n'; printf '\\033[38;5;196mRTERM-ANSI256\\033[0m\\n'; printf '\\033[38;2;1;2;3mRTERM-TRUECOLOR\\033[0m\\n'; printf '\\033[2;4r\\033[3;5HXY'",
            home.path(),
        ),
        PTY_SIZE,
        Vec::new(),
        PTY_TIMEOUT,
    );
    report("ansi-cursor-scroll", &run);
    assert_output(&run, "RTERM-ANSI16");
    assert_output(&run, "RTERM-ANSI256");
    assert_output(&run, "RTERM-TRUECOLOR");
    let cells = (0..run.runtime.terminal().grid().size().rows)
        .filter_map(|row| run.runtime.terminal().grid().row(row))
        .flat_map(|row| row.cells().iter())
        .collect::<Vec<_>>();
    let first = cells
        .iter()
        .find(|cell| cell.text() == "R")
        .expect("ANSI row cell");
    assert_eq!(format!("{:?}", first.foreground), "Indexed(1)");
    let ansi256 = cells
        .iter()
        .find(|cell| cell.text() == "R" && format!("{:?}", cell.foreground) == "Indexed(196)")
        .expect("256-color row cell");
    assert_eq!(format!("{:?}", ansi256.foreground), "Indexed(196)");
    let truecolor = cells
        .iter()
        .find(|cell| cell.text() == "R" && format!("{:?}", cell.foreground) == "Rgb(1, 2, 3)")
        .expect("truecolor row cell");
    assert_eq!(format!("{:?}", truecolor.foreground), "Rgb(1, 2, 3)");
    assert_eq!(run.runtime.terminal().scroll_region(), (1, 3));
    assert_eq!(run.runtime.terminal().cursor(), (2, 6));
    assert_command_status(&run, 0);

    run = run_pty(
        "alternate-bracketed-paste",
        shell_command(
            "printf '\\033[?1049hALT-SCREEN\\n'; sleep 0.2; printf '\\033[?2004hBRACKETED\\n'; sleep 0.2; printf '\\033[?1049lMAIN-SCREEN\\n'",
            home.path(),
        ),
        PTY_SIZE,
        Vec::new(),
        PTY_TIMEOUT,
    );
    report("alternate-bracketed-paste", &run);
    assert_output(&run, "ALT-SCREEN");
    assert_output(&run, "MAIN-SCREEN");
    assert!(
        run.saw_alternate_screen,
        "alternate screen was never observed"
    );
    assert!(!run.runtime.terminal().alternate_screen_active());
    assert!(
        run.runtime.bracketed_paste(),
        "bracketed paste mode was not enabled"
    );
    assert!(
        run.mode_changes
            .iter()
            .any(|change| matches!(change, TerminalModeChange::BracketedPaste(true)))
    );
    assert_command_status(&run, 0);

    if let Some(python) = find_command("python3") {
        run = run_pty(
            "ctrl-c",
            command_with_home(python.clone(), home.path()).with_args([
                "-c",
                "import signal,time,sys; signal.signal(signal.SIGINT, lambda s,f: (print('RTERM-CTRL-C', flush=True), sys.exit(130))); print('RTERM-READY', flush=True); time.sleep(10)",
            ]),
            PTY_SIZE,
            vec![InputAction::Write {
                after: Duration::from_millis(150),
                bytes: vec![3],
            }],
            PTY_TIMEOUT,
        );
        report("ctrl-c", &run);
        assert_output(&run, "RTERM-READY");
        assert_output(&run, "RTERM-CTRL-C");
        assert_command_status(&run, 130);

        run = run_pty(
            "ctrl-d-eof",
            command_with_home(python.clone(), home.path()).with_args([
                "-c",
                "import sys; print('RTERM-EOF:' + sys.stdin.read().strip())",
            ]),
            PTY_SIZE,
            vec![InputAction::Write {
                after: Duration::from_millis(100),
                bytes: b"RTERM-DATA\n\x04".to_vec(),
            }],
            PTY_TIMEOUT,
        );
        report("ctrl-d-eof", &run);
        assert_output(&run, "RTERM-EOF:RTERM-DATA");
        assert_command_status(&run, 0);

        run = run_pty(
            "python-repl",
            command_with_home(python, home.path()).with_args(["-q", "-I"]),
            PTY_SIZE,
            vec![InputAction::Write {
                after: Duration::from_millis(150),
                bytes: b"print('RTERM-PY-RESULT:' + str(40+2))\nexit()\n".to_vec(),
            }],
            PTY_TIMEOUT,
        );
        report("python-repl", &run);
        assert_output(&run, "RTERM-PY-RESULT:42");
        assert!(
            visible_grid_text(&run).contains("RTERM-PY-RESULT:42"),
            "Python result did not reach the visible grid"
        );
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=ctrl-c reason=missing-python3");
        println!("compat skip=ctrl-d-eof reason=missing-python3");
        println!("compat skip=python-repl reason=missing-python3");
    }

    if let Some(node) = find_command("node") {
        run = run_pty(
            "node-repl",
            command_with_home(node, home.path()).with_args(["--interactive"]),
            PTY_SIZE,
            vec![InputAction::Write {
                after: Duration::from_millis(150),
                bytes: b"console.log('RTERM-NODE-RESULT:' + (40+2))\n.exit\n".to_vec(),
            }],
            PTY_TIMEOUT,
        );
        report("node-repl", &run);
        assert_output(&run, "RTERM-NODE-RESULT:42");
        assert!(
            visible_grid_text(&run).contains("RTERM-NODE-RESULT:42"),
            "Node result did not reach the visible grid"
        );
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=node-repl reason=missing-node");
    }

    fs::write(
        fixture.path().join("rterm-shell-completion-marker"),
        "RTERM-TAB-RESULT:84\n",
    )
    .expect("create shell completion fixture");
    if let Some(bash) = find_command("bash") {
        // macOS Bash 3.2 does not enable bracketed paste; zsh below exercises
        // the actual wrapped payload and asserts its mode transition.
        run = run_readline_shell(
            "bash-readline",
            bash,
            &["--noprofile", "--norc", "-i"],
            false,
            home.path(),
            fixture.path(),
        );
        report("bash-readline", &run);
        let output = output_text(&run);
        assert!(
            output.matches("RTERM-SHELL-RESULT:42").count() >= 2,
            "Bash history arrow did not repeat the computed command: {output:?}"
        );
        assert_output(&run, "RTERM-TAB-RESULT:84");
        assert_output(&run, "^C");
        assert!(
            visible_grid_text(&run).contains("RTERM-SHELL-RESULT:42"),
            "Bash computed result did not reach the visible grid"
        );
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=bash-readline reason=missing-bash");
    }

    if let Some(zsh) = find_command("zsh") {
        run = run_readline_shell(
            "zsh-readline",
            zsh,
            &["-f", "-i"],
            true,
            home.path(),
            fixture.path(),
        );
        report("zsh-readline", &run);
        let output = output_text(&run);
        assert!(
            output.matches("RTERM-SHELL-RESULT:42").count() >= 2,
            "zsh history arrow did not repeat the computed command: {output:?}"
        );
        assert_output(&run, "RTERM-TAB-RESULT:84");
        assert_output(&run, "^C");
        assert!(
            run.mode_changes
                .iter()
                .any(|change| matches!(change, TerminalModeChange::BracketedPaste(true))),
            "zsh did not enable bracketed paste for the payload"
        );
        assert!(
            visible_grid_text(&run).contains("RTERM-SHELL-RESULT:42"),
            "zsh computed result did not reach the visible grid"
        );
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=zsh-readline reason=missing-zsh");
    }

    let vim_file = fixture.path().join("vim.txt");
    fs::write(&vim_file, "").expect("create Vim fixture");
    if let Some(vim) = find_command("vim") {
        run = run_pty(
            "vim-tui",
            command_with_home(vim, home.path())
                .with_args(["-Nu", "NONE", "-i", "NONE", "-n", "vim.txt"])
                .with_cwd(fixture.path()),
            PTY_SIZE,
            vec![InputAction::Write {
                after: Duration::from_millis(200),
                bytes: b"iRTERM-VIM\n\x1b:wq\r".to_vec(),
            }],
            PTY_TIMEOUT,
        );
        report("vim-tui", &run);
        assert_command_status(&run, 0);
        assert_eq!(
            fs::read_to_string(&vim_file).expect("read Vim fixture"),
            "RTERM-VIM\n\n"
        );
        assert!(
            output_text(&run).contains("\x1b[?1049h") || run.saw_alternate_screen,
            "Vim did not exercise alternate-screen TUI mode"
        );
    } else {
        println!("compat skip=vim-tui reason=missing-vim");
    }

    let less_file = fixture.path().join("less.txt");
    fs::write(&less_file, "RTERM-LESS-LINE\n").expect("create less fixture");
    if let Some(less) = find_command("less") {
        run = run_pty(
            "less-tui",
            command_with_home(less, home.path())
                .with_args(["-R", "less.txt"])
                .with_env("LESS", "")
                .with_env("LESSHISTFILE", "/dev/null")
                .with_cwd(fixture.path()),
            PTY_SIZE,
            vec![InputAction::Write {
                after: Duration::from_millis(200),
                bytes: b"q".to_vec(),
            }],
            PTY_TIMEOUT,
        );
        report("less-tui", &run);
        assert_output(&run, "RTERM-LESS-LINE");
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=less-tui reason=missing-less");
    }

    let nano_file = fixture.path().join("nano.txt");
    fs::write(&nano_file, "").expect("create nano fixture");
    if let Some(nano) = find_command("nano") {
        run = run_pty(
            "nano-editor",
            command_with_home(nano, home.path())
                // The macOS `/usr/bin/nano` may be the Pico-compatible editor,
                // which rejects GNU nano's `-I` flag.  Its isolated HOME keeps
                // the editor configuration hermetic without a vendor flag.
                .with_args(["nano.txt"])
                .with_cwd(fixture.path()),
            PTY_SIZE,
            vec![InputAction::Write {
                after: Duration::from_millis(250),
                bytes: b"RTERM-NANO\x0f\r\x18".to_vec(),
            }],
            PTY_TIMEOUT,
        );
        report("nano-editor", &run);
        assert_eq!(
            run.exit.status,
            Some(0),
            "nano exit {:?}, output={:?}",
            run.exit,
            output_text(&run)
        );
        assert_eq!(
            fs::read_to_string(&nano_file).expect("read nano fixture"),
            "RTERM-NANO\n"
        );
    } else {
        println!("compat skip=nano-editor reason=missing-nano");
    }

    if let Some(pwsh) = find_command("pwsh") {
        run = run_pty(
            "pwsh-noprofile",
            command_with_home(pwsh, home.path()).with_args([
                "-NoProfile",
                "-NoLogo",
                "-NonInteractive",
                "-Command",
                "'RTERM-PWSH-RESULT:' + (40 + 2)",
            ]),
            PTY_SIZE,
            Vec::new(),
            PTY_TIMEOUT,
        );
        report("pwsh-noprofile", &run);
        assert_output(&run, "RTERM-PWSH-RESULT:42");
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=pwsh-noprofile reason=missing-pwsh");
    }

    for (label, name, fallback) in [
        (
            "codex-basic-cli",
            "codex",
            "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
        ),
        ("claude-basic-cli", "claude", "/opt/homebrew/bin/claude"),
    ] {
        if let Some(cli) = find_command_with_fallback(name, fallback) {
            run = run_pty(
                label,
                command_with_home(cli, home.path()).with_args(["--version"]),
                PTY_SIZE,
                Vec::new(),
                PTY_TIMEOUT,
            );
            report(label, &run);
            assert!(
                !output_text(&run).trim().is_empty(),
                "{label} did not produce version output"
            );
            assert_command_status(&run, 0);
        } else {
            println!("compat skip={label} reason=missing-{name}");
        }
    }

    if let Some(git) = find_command("git") {
        fs::write(fixture.path().join("tracked.txt"), "seed\n").expect("create git fixture");
        run = run_pty(
            "git-workflow",
            command_with_home(git, home.path())
                .with_args([
                    "-c",
                    "user.email=rterm@example.invalid",
                    "-c",
                    "user.name=R-Term",
                    "init",
                    "-q",
                ])
                .with_cwd(fixture.path()),
            PTY_SIZE,
            Vec::new(),
            PTY_TIMEOUT,
        );
        report("git-init", &run);
        assert_command_status(&run, 0);
        run = run_pty(
            "git-workflow",
            shell_command(
                "git config user.email rterm@example.invalid && git config user.name R-Term && git add tracked.txt && git commit -qm seed && printf changed >> tracked.txt && git -c color.status=false status --short --untracked-files=no",
                home.path(),
            )
            .with_cwd(fixture.path()),
            PTY_SIZE,
            Vec::new(),
            PTY_TIMEOUT,
        );
        report("git-workflow", &run);
        assert_output(&run, " M tracked.txt");
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=git-workflow reason=missing-git");
    }

    let rg_file = fixture.path().join("matches.txt");
    fs::write(&rg_file, "other\nRTERM-MATCH here\n").expect("create rg fixture");
    if let Some(rg) = find_command("rg") {
        run = run_pty(
            "rg-colored",
            command_with_home(rg, home.path())
                .with_args(["--color=always", "RTERM-MATCH", "matches.txt"])
                .with_cwd(fixture.path()),
            PTY_SIZE,
            Vec::new(),
            PTY_TIMEOUT,
        );
        report("rg-colored", &run);
        assert_output(&run, "RTERM-MATCH");
        assert!(
            output_text(&run).contains("\x1b["),
            "rg did not emit requested color"
        );
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=rg-colored reason=missing-rg");
    }

    if let Some(sqlite) = find_command("sqlite3") {
        run = run_pty(
            "sqlite-query",
            command_with_home(sqlite, home.path()).with_args([
                ":memory:",
                "create table t(v); insert into t values ('RTERM-SQL'); select v from t;",
            ]),
            PTY_SIZE,
            Vec::new(),
            PTY_TIMEOUT,
        );
        report("sqlite-query", &run);
        assert_output(&run, "RTERM-SQL");
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=sqlite-query reason=missing-sqlite3");
    }

    if let Some(awk) = find_command("awk") {
        run = run_pty(
            "awk-transform",
            command_with_home(awk, home.path())
                .with_args(["BEGIN { print \"RTERM-AWK:\" (3 * 7) }"]),
            PTY_SIZE,
            Vec::new(),
            PTY_TIMEOUT,
        );
        report("awk-transform", &run);
        assert_output(&run, "RTERM-AWK:21");
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=awk-transform reason=missing-awk");
    }

    if let Some(top) = find_command("top") {
        let top_args: &[&str] = if cfg!(target_os = "macos") {
            &["-l", "1", "-n", "5"]
        } else {
            &["-b", "-n", "1", "-w", "120"]
        };
        run = run_pty(
            "top-snapshot",
            command_with_home(top, home.path()).with_args(top_args.iter().copied()),
            PTY_SIZE,
            Vec::new(),
            PTY_TIMEOUT,
        );
        report("top-snapshot", &run);
        assert_output(&run, "PID");
        assert_command_status(&run, 0);
    } else {
        println!("compat skip=top-snapshot reason=missing-top");
    }

    // This driver keeps an unbounded reader channel so the assertion can
    // prove losslessness.  It is not a production backpressure benchmark;
    // bounded worker behavior remains covered by rterm-runtime tests.
    run = run_pty(
        "large-output-lossless",
        shell_command(
            "i=0; while [ \"$i\" -lt 20000 ]; do printf 'RTERM-BURST-%05d\\n' \"$i\"; i=$((i + 1)); done",
            home.path(),
        ),
        PTY_SIZE,
        Vec::new(),
        Duration::from_secs(10),
    );
    report("large-output-lossless", &run);
    assert_output(&run, "RTERM-BURST-00000");
    assert_output(&run, "RTERM-BURST-19999");
    assert_eq!(
        output_text(&run).matches("RTERM-BURST-").count(),
        20_000,
        "large output lost or duplicated records"
    );
    assert_command_status(&run, 0);

    run = run_pty(
        "interrupt-cleanup",
        shell_command("sleep 30", home.path()),
        PTY_SIZE,
        vec![InputAction::Interrupt {
            after: Duration::from_millis(150),
        }],
        PTY_TIMEOUT,
    );
    report("interrupt-cleanup", &run);
    assert!(
        run.exit.status != Some(0),
        "interrupt must not report success"
    );
    assert!(
        run.elapsed < Duration::from_secs(3),
        "interrupt cleanup exceeded bound: {:?}",
        run.elapsed
    );

    println!(
        "compat summary=case-assertions-pass core=pty-resize,unicode,ansi,alternate,bracketed-paste-payload,signals,repl,readline,tui,cli,lossless-output,cleanup bounded-worker-backpressure=covered-by-rterm-runtime-tests inventory-is-observation-only"
    );
}
