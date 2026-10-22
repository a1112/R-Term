use std::process::Command;

const MARKER_ENV: &str = "RTERM_TEST_MARKER";

/// Builds a native command that writes `marker` as exact UTF-8 bytes to stdout.
///
/// The marker is passed through the child environment rather than interpolated
/// into a shell program, avoiding platform quoting and command injection.
#[must_use]
pub fn platform_marker_command(marker: &str) -> Command {
    platform_marker_command_with_delay(marker, None)
}

/// Builds a native command that writes `marker` as exact UTF-8 bytes to stdout
/// and remains alive briefly before exiting.
///
/// Native-window frame probes use this variant so their rendering assertions do
/// not race a deliberately short-lived marker process during window startup,
/// while preserving the normal PTY close-and-reap lifecycle.
#[must_use]
pub fn platform_marker_command_for_window_frames(marker: &str) -> Command {
    platform_marker_command_with_delay(marker, Some(5))
}

fn platform_marker_command_with_delay(marker: &str, delay_seconds: Option<u64>) -> Command {
    #[cfg(windows)]
    {
        let sleep = delay_seconds.map_or_else(String::new, |seconds| {
            format!("; Start-Sleep -Seconds {seconds}")
        });
        // ConPTY decodes raw console writes using the console output code page.
        // Match it to the bytes below, including on non-UTF-8 Windows locales.
        let script = format!(
            "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); \
             $bytes=[Text.Encoding]::UTF8.GetBytes($env:RTERM_TEST_MARKER); \
             $stdout=[Console]::OpenStandardOutput(); \
             $stdout.Write($bytes,0,$bytes.Length){sleep}"
        );
        let mut command = Command::new("powershell.exe");
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &script,
            ])
            .env(MARKER_ENV, marker);
        command
    }

    #[cfg(not(windows))]
    {
        let program = delay_seconds.map_or_else(
            || "printf '%s' \"$RTERM_TEST_MARKER\"".to_owned(),
            |seconds| format!("printf '%s' \"$RTERM_TEST_MARKER\"; sleep {seconds}"),
        );
        let mut command = Command::new("/bin/sh");
        command.args(["-c", &program]).env(MARKER_ENV, marker);
        command
    }
}
