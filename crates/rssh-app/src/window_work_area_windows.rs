//! Narrow Win32 work-area query. This is the native shell's only unsafe boundary.
use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO},
};

/// Returns the OS work rectangle in physical desktop coordinates.
#[allow(unsafe_code)]
pub(super) fn query(monitor: HMONITOR) -> Option<(i32, i32, u32, u32)> {
    let empty = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut info = MONITORINFO {
        cbSize: u32::try_from(std::mem::size_of::<MONITORINFO>()).ok()?,
        rcMonitor: empty,
        rcWork: empty,
        dwFlags: 0,
    };
    // SAFETY: winit supplies a live HMONITOR. `info` is a correctly sized,
    // writable MONITORINFO for this synchronous, read-only OS query. No handle
    // is owned or retained; fields are inspected only after a successful BOOL.
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    let width = u32::try_from(i64::from(info.rcWork.right) - i64::from(info.rcWork.left)).ok()?;
    let height = u32::try_from(i64::from(info.rcWork.bottom) - i64::from(info.rcWork.top)).ok()?;
    (width > 0 && height > 0).then_some((info.rcWork.left, info.rcWork.top, width, height))
}
