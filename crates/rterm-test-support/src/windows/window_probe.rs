use std::{
    fmt,
    process::Command,
    time::{Duration, Instant},
};

use crate::ChildGuard;

const PROBE_GRACE: Duration = Duration::from_secs(2);
const POWERSHELL_PROBE: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public sealed class RtermOwnedWindowObservation {
    public long Hwnd;
    public long Style;
    public long ExStyle;
    public int WindowLeft;
    public int WindowTop;
    public int WindowRight;
    public int WindowBottom;
    public int ClientLeft;
    public int ClientTop;
    public int ClientRight;
    public int ClientBottom;
    public int ClientOriginX;
    public int ClientOriginY;
    public uint Dpi;
    public string Title = "";
    public bool Visible;
    public bool FrameValid;
    public int FrameError;
}

public static class RtermOwnedWindowProbe {
    [StructLayout(LayoutKind.Sequential)] private struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [StructLayout(LayoutKind.Sequential)] private struct POINT {
        public int X;
        public int Y;
    }

    private delegate bool EnumWindowsCallback(IntPtr hwnd, IntPtr parameter);

    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindowsCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", SetLastError = true)] private static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
    [DllImport("user32.dll", SetLastError = true)] private static extern bool GetClientRect(IntPtr hwnd, out RECT rect);
    [DllImport("user32.dll", SetLastError = true)] private static extern bool ClientToScreen(IntPtr hwnd, ref POINT point);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")] private static extern IntPtr GetWindowLongPtr(IntPtr hwnd, int index);
    [DllImport("user32.dll")] private static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int count);

    public static RtermOwnedWindowObservation[] Enumerate(uint wantedProcessId) {
        var observations = new List<RtermOwnedWindowObservation>();
        EnumWindows(delegate(IntPtr hwnd, IntPtr parameter) {
            uint processId;
            GetWindowThreadProcessId(hwnd, out processId);
            if (processId != wantedProcessId) {
                return true;
            }

            var observation = new RtermOwnedWindowObservation();
            observation.Hwnd = hwnd.ToInt64();
            observation.Visible = IsWindowVisible(hwnd);
            observation.Style = GetWindowLongPtr(hwnd, -16).ToInt64();
            observation.ExStyle = GetWindowLongPtr(hwnd, -20).ToInt64();
            observation.Dpi = GetDpiForWindow(hwnd);
            var title = new StringBuilder(512);
            GetWindowText(hwnd, title, title.Capacity);
            observation.Title = title.ToString();

            RECT windowRect;
            RECT clientRect;
            var clientOrigin = new POINT();
            if (GetWindowRect(hwnd, out windowRect)
                && GetClientRect(hwnd, out clientRect)
                && ClientToScreen(hwnd, ref clientOrigin)) {
                observation.WindowLeft = windowRect.Left;
                observation.WindowTop = windowRect.Top;
                observation.WindowRight = windowRect.Right;
                observation.WindowBottom = windowRect.Bottom;
                observation.ClientLeft = clientRect.Left;
                observation.ClientTop = clientRect.Top;
                observation.ClientRight = clientRect.Right;
                observation.ClientBottom = clientRect.Bottom;
                observation.ClientOriginX = clientOrigin.X;
                observation.ClientOriginY = clientOrigin.Y;
                observation.FrameValid = true;
            } else {
                observation.FrameError = Marshal.GetLastWin32Error();
            }
            observations.Add(observation);
            return true;
        }, IntPtr.Zero);
        return observations.ToArray();
    }
}
'@

$wantedProcessId = [uint32]$env:RTERM_WINDOW_PROBE_PID
$timeoutMs = [int]$env:RTERM_WINDOW_PROBE_TIMEOUT_MS
$deadline = [DateTime]::UtcNow.AddMilliseconds($timeoutMs)
$last = @()
do {
    $last = @([RtermOwnedWindowProbe]::Enumerate($wantedProcessId))
    $match = $last |
        Where-Object { $_.Visible -and $_.FrameValid } |
        Sort-Object { ($_.WindowRight - $_.WindowLeft) * ($_.WindowBottom - $_.WindowTop) } -Descending |
        Select-Object -First 1
    if ($null -ne $match) {
        $title = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($match.Title))
        [Console]::Out.WriteLine([string]::Join("`t", @(
            'OK', $wantedProcessId, $match.Hwnd, $match.Style, $match.ExStyle,
            $match.WindowLeft, $match.WindowTop, $match.WindowRight, $match.WindowBottom,
            $match.ClientLeft, $match.ClientTop, $match.ClientRight, $match.ClientBottom,
            $match.ClientOriginX, $match.ClientOriginY, $match.Dpi, $title
        )))
        exit 0
    }
    Start-Sleep -Milliseconds 25
} while ([DateTime]::UtcNow -lt $deadline)

$summary = if ($last.Count -eq 0) {
    'no candidates'
} else {
    ($last | ForEach-Object {
        'hwnd=0x{0:x} visible={1} frame-valid={2} frame-error={3} title={4}' -f `
            $_.Hwnd, $_.Visible, $_.FrameValid, $_.FrameError, $_.Title
    }) -join '; '
}
[Console]::Error.WriteLine(
    ('process {0} did not expose an observable window before the deadline; last enumeration: {1}' -f `
        $wantedProcessId, $summary)
)
exit 2
"#;

/// A screen-space rectangle returned by the window frame probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl WindowRect {
    #[must_use]
    pub fn width(self) -> i32 {
        self.right - self.left
    }

    #[must_use]
    pub fn height(self) -> i32 {
        self.bottom - self.top
    }
}

/// A screen-space point returned by the window frame probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowPoint {
    pub x: i32,
    pub y: i32,
}

/// A successful observation of a visible top-level window owned by a process.
#[derive(Debug, PartialEq, Eq)]
pub struct WindowFrameObservation {
    pub process_id: u32,
    pub hwnd: i64,
    pub style: i64,
    pub ex_style: i64,
    pub window_rect: WindowRect,
    pub client_rect: WindowRect,
    pub client_origin: WindowPoint,
    pub dpi: u32,
    pub title: String,
}

impl WindowFrameObservation {
    /// Returns whether the client area replaces the native frame.
    ///
    /// Winit deliberately retains native caption style bits so Windows keeps
    /// snap, animation, and system-menu behavior. It removes the actual frame
    /// through `WM_NCCALCSIZE`; when undecorated shadow is enabled, winit
    /// documents a one-physical-pixel line at the top. Geometry, not the
    /// retained style bits, is therefore the observable borderless contract.
    #[must_use]
    pub fn has_borderless_client_area(&self) -> bool {
        let top_offset = self.client_origin.y - self.window_rect.top;
        self.window_rect.width() > 0
            && self.window_rect.height() > 0
            && self.client_rect.left == 0
            && self.client_rect.top == 0
            && self.client_origin.x == self.window_rect.left
            && matches!(top_offset, 0 | 1)
            && self.client_rect.width() == self.window_rect.width()
            && self.client_rect.height() == self.window_rect.height()
    }
}

/// A failure to launch, execute, or decode the external Win32 probe.
#[derive(Debug, PartialEq, Eq)]
pub struct WindowProbeError {
    diagnostic: String,
}

impl WindowProbeError {
    fn new(diagnostic: impl Into<String>) -> Self {
        Self {
            diagnostic: diagnostic.into(),
        }
    }
}

impl fmt::Display for WindowProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.diagnostic)
    }
}

impl std::error::Error for WindowProbeError {}

/// Waits for the largest visible top-level window owned by `process_id`.
///
/// The Win32 FFI is isolated in a short-lived PowerShell/C# probe so the Rust
/// workspace can retain `unsafe_code = "forbid"` without weakening the policy
/// for production or test-support crates.
///
/// # Errors
///
/// Returns a diagnostic containing the process ID and last enumeration when no
/// matching window is observable before `deadline`, or when the helper cannot
/// be launched or decoded.
pub fn wait_for_owned_window_frame(
    process_id: u32,
    deadline: Instant,
) -> Result<WindowFrameObservation, WindowProbeError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(WindowProbeError::new(format!(
            "process {process_id} did not expose an observable window before the deadline; last enumeration: not started"
        )));
    }

    let timeout_ms = remaining.as_millis().clamp(1, i32::MAX as u128);
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            POWERSHELL_PROBE,
        ])
        .env("RTERM_WINDOW_PROBE_PID", process_id.to_string())
        .env("RTERM_WINDOW_PROBE_TIMEOUT_MS", timeout_ms.to_string());
    let output = ChildGuard::spawn(command, remaining.saturating_add(PROBE_GRACE))
        .map_err(|error| WindowProbeError::new(format!("launch window probe: {error}")))?
        .wait()
        .map_err(|error| WindowProbeError::new(format!("execute window probe: {error}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(WindowProbeError::new(if stderr.is_empty() {
            format!(
                "process {process_id} did not expose an observable window before the deadline; last enumeration: helper exited {}",
                output.status
            )
        } else {
            stderr
        }));
    }

    parse_observation(&output.stdout)
}

fn parse_observation(stdout: &[u8]) -> Result<WindowFrameObservation, WindowProbeError> {
    let stdout = std::str::from_utf8(stdout).map_err(|error| {
        WindowProbeError::new(format!("window probe emitted non-UTF-8 output: {error}"))
    })?;
    let fields = stdout.trim().split('\t').collect::<Vec<_>>();
    if fields.len() != 17 || fields.first() != Some(&"OK") {
        return Err(WindowProbeError::new(format!(
            "window probe emitted an invalid record: {stdout:?}"
        )));
    }

    let parse = |index: usize, name: &str| -> Result<i64, WindowProbeError> {
        fields[index].parse::<i64>().map_err(|error| {
            WindowProbeError::new(format!(
                "window probe field {name} was invalid ({:?}): {error}",
                fields[index]
            ))
        })
    };
    let parse_i32 = |index: usize, name: &str| -> Result<i32, WindowProbeError> {
        i32::try_from(parse(index, name)?).map_err(|error| {
            WindowProbeError::new(format!(
                "window probe field {name} was out of range: {error}"
            ))
        })
    };

    Ok(WindowFrameObservation {
        process_id: u32::try_from(parse(1, "process_id")?).map_err(|error| {
            WindowProbeError::new(format!("window probe process ID was out of range: {error}"))
        })?,
        hwnd: parse(2, "hwnd")?,
        style: parse(3, "style")?,
        ex_style: parse(4, "ex_style")?,
        window_rect: WindowRect {
            left: parse_i32(5, "window_left")?,
            top: parse_i32(6, "window_top")?,
            right: parse_i32(7, "window_right")?,
            bottom: parse_i32(8, "window_bottom")?,
        },
        client_rect: WindowRect {
            left: parse_i32(9, "client_left")?,
            top: parse_i32(10, "client_top")?,
            right: parse_i32(11, "client_right")?,
            bottom: parse_i32(12, "client_bottom")?,
        },
        client_origin: WindowPoint {
            x: parse_i32(13, "client_origin_x")?,
            y: parse_i32(14, "client_origin_y")?,
        },
        dpi: u32::try_from(parse(15, "dpi")?).map_err(|error| {
            WindowProbeError::new(format!("window probe DPI was out of range: {error}"))
        })?,
        title: decode_title(fields[16])?,
    })
}

fn decode_title(encoded: &str) -> Result<String, WindowProbeError> {
    let bytes = decode_base64(encoded)?;
    String::from_utf8(bytes).map_err(|error| {
        WindowProbeError::new(format!("window probe title was not UTF-8: {error}"))
    })
}

fn decode_base64(encoded: &str) -> Result<Vec<u8>, WindowProbeError> {
    use base64::Engine as _;

    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| {
            WindowProbeError::new(format!("window probe title was not base64: {error}"))
        })
}
