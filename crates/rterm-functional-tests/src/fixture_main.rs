use std::{
    env, fs,
    io::{self, BufRead, Read, Write},
    process::{Command, ExitCode, Stdio},
    thread,
    time::Duration,
};

use rterm_functional_tests::TRANSPORT_WRITE_MARKER;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("fixture error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<u8, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "help".to_owned());
    match mode.as_str() {
        "version" => {
            println!("{{\"name\":\"rterm-app\",\"version\":\"fixture\",\"console\":true}}");
            Ok(0)
        }
        "self-test" => {
            println!("{{\"ok\":true,\"checks\":[{{\"name\":\"local-pty\",\"ok\":true}}]}}");
            Ok(0)
        }
        "local" => local_entrypoint_proxy(args),
        "echo-query" => echo_query(),
        "slow-read" => slow_read(args.next(), args.next()),
        "high-output" => high_output(args.next()),
        "osc-clipboard" => {
            print!("\x1b]52;c;ZnVuY3Rpb25hbC10ZXN0\x07");
            io::stdout().flush()?;
            Ok(0)
        }
        "mouse-focus-report" => {
            print!("\x1b[?1004h\x1b[?1006h\x1b[?1003hfixture-ready\r\n");
            io::stdout().flush()?;
            copy_stdin_to_stdout()?;
            Ok(0)
        }
        "exit-code" => Ok(args.next().unwrap_or_else(|| "0".to_owned()).parse()?),
        "hold-open" => {
            println!("fixture-hold-open");
            io::stdout().flush()?;
            let mut byte = [0_u8; 1];
            while io::stdin().read(&mut byte)? != 0 {}
            Ok(0)
        }
        "synchronized-output" => {
            print!("\x1b[?2026hfirst\r\nsecond\x1b[?2026l");
            io::stdout().flush()?;
            Ok(0)
        }
        "window-effects" => {
            print!("fixture-ready\r\n");
            io::stdout().flush()?;
            let mut input = String::new();
            loop {
                input.clear();
                if io::stdin().lock().read_line(&mut input)? == 0 {
                    break;
                }
                print!("{input}");
                let command = command_after_terminal_responses(input.trim_end());
                if command == "emit-effects" {
                    print!(
                        "\x1b[6n\x07\x1b]52;c;ZnVuY3Rpb25hbC10ZXN0\x07\x1b]777;notify;Functional;ready\x07"
                    );
                }
                io::stdout().flush()?;
                if command == "functional-exit" {
                    break;
                }
            }
            Ok(0)
        }
        "host-terminal-probe" => {
            let marker = args
                .next()
                .ok_or("host-terminal-probe requires a marker path")?;
            let title = args
                .next()
                .ok_or("host-terminal-probe requires a window title")?;
            set_host_terminal_title(&title)?;
            fs::write(&marker, "host-terminal-ready")?;
            print!("host-terminal-ready\r\n");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().lock().read_line(&mut input)?;
            fs::write(marker, input.trim_end_matches(['\r', '\n']))?;
            Ok(0)
        }
        _ => {
            eprintln!(
                "usage: rterm-functional-fixture <version|self-test|local|echo-query|slow-read|high-output|osc-clipboard|mouse-focus-report|exit-code|hold-open|synchronized-output|window-effects|host-terminal-probe>"
            );
            Ok(2)
        }
    }
}

#[cfg(windows)]
fn set_host_terminal_title(title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("cmd.exe")
        .args(["/d", "/c", "title", title])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("set console title exited {status}").into())
    }
}

#[cfg(not(windows))]
fn set_host_terminal_title(title: &str) -> Result<(), Box<dyn std::error::Error>> {
    print!("\x1b]0;{title}\x07");
    io::stdout().flush()?;
    Ok(())
}

fn command_after_terminal_responses(mut input: &str) -> &str {
    while let Some(rest) = input.strip_prefix("\x1b[") {
        let Some(response_end) = rest.find('R') else {
            break;
        };
        let response = &rest[..response_end];
        if response.is_empty()
            || !response
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b';')
        {
            break;
        }
        input = &rest[response_end + 1..];
    }
    input
}

fn local_entrypoint_proxy(
    mut args: impl Iterator<Item = String>,
) -> Result<u8, Box<dyn std::error::Error>> {
    if args.next().as_deref() != Some("--") {
        return Err("local fixture proxy requires `-- <program> [arguments...]`".into());
    }
    let program = args
        .next()
        .ok_or("local fixture proxy requires a child program")?;
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1))
}

fn echo_query() -> Result<u8, Box<dyn std::error::Error>> {
    print!("\x1b[6nfixture-ready\r\n");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let received_len = line.len();
    let line = command_after_terminal_responses(&line);
    if line.len() < received_len {
        println!("{TRANSPORT_WRITE_MARKER}");
    }
    print!("fixture-echo:{line}");
    io::stdout().flush()?;
    Ok(0)
}

fn slow_read(
    delay_ms: Option<String>,
    byte_count: Option<String>,
) -> Result<u8, Box<dyn std::error::Error>> {
    let delay = delay_ms.unwrap_or_else(|| "5".to_owned()).parse()?;
    let mut remaining = byte_count.map(|value| value.parse::<usize>()).transpose()?;
    let mut byte = [0_u8; 1];
    while io::stdin().read(&mut byte)? != 0 {
        thread::sleep(Duration::from_millis(delay));
        io::stdout().write_all(&byte)?;
        if let Some(value) = &mut remaining {
            *value = value.saturating_sub(1);
            if *value == 0 {
                break;
            }
        }
    }
    io::stdout().flush()?;
    Ok(0)
}

fn high_output(byte_count: Option<String>) -> Result<u8, Box<dyn std::error::Error>> {
    let mut remaining: usize = byte_count.unwrap_or_else(|| "1048576".to_owned()).parse()?;
    let chunk = [b'X'; 8192];
    let mut stdout = io::stdout().lock();
    while remaining > 0 {
        let count = remaining.min(chunk.len());
        stdout.write_all(&chunk[..count])?;
        remaining -= count;
    }
    stdout.flush()?;
    Ok(0)
}

fn copy_stdin_to_stdout() -> io::Result<()> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    io::copy(&mut stdin, &mut stdout)?;
    stdout.flush()
}

#[cfg(test)]
mod tests {
    use super::command_after_terminal_responses;

    #[test]
    fn window_fixture_accepts_command_after_device_status_response() {
        assert_eq!(
            command_after_terminal_responses("\x1b[24;9Rfunctional-exit"),
            "functional-exit"
        );
        assert_eq!(
            command_after_terminal_responses("\x1b[1;2R\x1b[3;4Remit-effects"),
            "emit-effects"
        );
        assert_eq!(
            command_after_terminal_responses("\x1b[1;1RR-Term 终端\r\n"),
            "R-Term 终端\r\n"
        );
    }

    #[test]
    fn window_fixture_does_not_strip_unrelated_escape_input() {
        assert_eq!(
            command_after_terminal_responses("\x1b[31mred"),
            "\x1b[31mred"
        );
        assert_eq!(command_after_terminal_responses("plain"), "plain");
    }
}
