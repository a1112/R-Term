//! Normal native window geometry; no renderer, terminal or immersive-mode ownership.
//!
//! macOS queries each target NSScreen.visibleFrame through safe AppKit APIs.
//! Logical frame-relative insets are applied to winit's physical monitor origin,
//! so Retina scales and negative/mixed-screen origins are never conflated.
//! Platforms lacking a safe work-area adapter use a documented reserve fallback.
use std::{fs, path::Path};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::InnerSizeWriter,
    monitor::MonitorHandle,
    window::Window,
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Geometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f64,
}

#[derive(Clone, Copy, Debug)]
struct Area {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f64,
}

fn usable_area(position: PhysicalPosition<i32>, size: PhysicalSize<u32>, scale: f64) -> Area {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let width = f64::from(size.width) / scale;
    let height = f64::from(size.height) / scale;
    let side = 24.0_f64.min(width / 10.0);
    let top = 32.0_f64.min(height / 10.0);
    let bottom = 96.0_f64.min(height / 5.0);
    Area {
        x: f64::from(position.x) + side * scale,
        y: f64::from(position.y) + top * scale,
        width: (width - side * 2.0).max(1.0),
        height: (height - top - bottom).max(1.0),
        scale,
    }
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn visible_area(
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    scale: f64,
    frame: Rect,
    visible: Rect,
) -> Option<Area> {
    if ![
        scale,
        frame.x,
        frame.y,
        frame.width,
        frame.height,
        visible.x,
        visible.y,
        visible.width,
        visible.height,
    ]
    .iter()
    .all(|v| v.is_finite())
        || scale <= 0.0
        || frame.width <= 0.0
        || frame.height <= 0.0
        || visible.width <= 0.0
        || visible.height <= 0.0
    {
        return None;
    }
    let left = (visible.x - frame.x).max(0.0);
    let right = ((frame.x + frame.width) - (visible.x + visible.width)).max(0.0);
    let bottom = (visible.y - frame.y).max(0.0);
    let top = ((frame.y + frame.height) - (visible.y + visible.height)).max(0.0);
    let width = f64::from(size.width) / scale - left - right;
    let height = f64::from(size.height) / scale - top - bottom;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some(Area {
        x: f64::from(position.x) + left * scale,
        y: f64::from(position.y) + top * scale,
        width,
        height,
        scale,
    })
}

#[cfg(target_os = "macos")]
fn platform_area(monitor: &MonitorHandle) -> Option<Area> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;
    use objc2_foundation::{NSNumber, NSString};
    use winit::platform::macos::MonitorHandleExtMacOS;
    let mtm = MainThreadMarker::new()?;
    let key = NSString::from_str("NSScreenNumber");
    let screens = NSScreen::screens(mtm);
    for index in 0..screens.count() {
        let screen = screens.objectAtIndex(index);
        let description = screen.deviceDescription();
        let Some(value) = description.objectForKey(&key) else {
            continue;
        };
        let Some(number) = value.downcast_ref::<NSNumber>() else {
            continue;
        };
        if number.unsignedIntValue() != monitor.native_id() {
            continue;
        }
        let frame = screen.frame();
        let visible = screen.visibleFrame();
        return visible_area(
            monitor.position(),
            monitor.size(),
            monitor.scale_factor(),
            Rect {
                x: frame.origin.x,
                y: frame.origin.y,
                width: frame.size.width,
                height: frame.size.height,
            },
            Rect {
                x: visible.origin.x,
                y: visible.origin.y,
                width: visible.size.width,
                height: visible.size.height,
            },
        );
    }
    None
}

#[cfg(target_os = "windows")]
#[path = "window_work_area_windows.rs"]
mod windows_work_area;

#[cfg(target_os = "windows")]
fn platform_area(monitor: &MonitorHandle) -> Option<Area> {
    use winit::platform::windows::MonitorHandleExtWindows;
    let (x, y, width, height) = windows_work_area::query(monitor.hmonitor())?;
    let scale = monitor.scale_factor();
    Some(physical_work_area(x, y, width, height, scale))
}

#[cfg(any(target_os = "windows", test))]
fn physical_work_area(x: i32, y: i32, width: u32, height: u32, scale: f64) -> Area {
    Area {
        x: f64::from(x),
        y: f64::from(y),
        width: f64::from(width) / scale,
        height: f64::from(height) / scale,
        scale,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_area(_monitor: &MonitorHandle) -> Option<Area> {
    None
}

fn area(monitor: &MonitorHandle) -> Area {
    platform_area(monitor)
        .unwrap_or_else(|| usable_area(monitor.position(), monitor.size(), monitor.scale_factor()))
}

fn overlap(geometry: Geometry, area: Area) -> f64 {
    let right =
        (geometry.x + geometry.width * geometry.scale).min(area.x + area.width * area.scale);
    let bottom =
        (geometry.y + geometry.height * geometry.scale).min(area.y + area.height * area.scale);
    (right - geometry.x.max(area.x)).max(0.0) * (bottom - geometry.y.max(area.y)).max(0.0)
}

fn fit(mut geometry: Geometry, area: Area, first: bool, border: (f64, f64)) -> Geometry {
    let width = if first {
        (area.width * 0.85).max((640.0 + border.0).min(area.width))
    } else {
        area.width
    };
    let height = if first {
        (area.height * 0.85).max((420.0 + border.1).min(area.height))
    } else {
        area.height
    };
    geometry.width = geometry.width.clamp(1.0, (width - border.0).max(1.0));
    geometry.height = geometry.height.clamp(1.0, (height - border.1).max(1.0));
    let outer_width = (geometry.width + border.0) * area.scale;
    let outer_height = (geometry.height + border.1) * area.scale;
    geometry.x = if first {
        area.x + (area.width * area.scale - outer_width) / 2.0
    } else {
        geometry.x.clamp(
            area.x,
            (area.x + area.width * area.scale - outer_width).max(area.x),
        )
    };
    geometry.y = if first {
        area.y + (area.height * area.scale - outer_height) / 2.0
    } else {
        geometry.y.clamp(
            area.y,
            (area.y + area.height * area.scale - outer_height).max(area.y),
        )
    };
    geometry.scale = area.scale;
    geometry
}

fn parse(text: &str) -> Option<Geometry> {
    let values = text
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if values.len() != 5
        || !values.iter().all(|v| v.is_finite())
        || values[2] < 160.0
        || values[3] < 120.0
        || values[4] <= 0.0
    {
        return None;
    }
    Some(Geometry {
        x: values[0],
        y: values[1],
        width: values[2],
        height: values[3],
        scale: values[4],
    })
}

fn load(path: &Path) -> Option<Geometry> {
    if fs::metadata(path).ok()?.len() > 512 {
        return None;
    }
    parse(&fs::read_to_string(path).ok()?)
}

fn write(path: &Path, geometry: Geometry) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(
        &temporary,
        format!(
            "{} {} {} {} {}\n",
            geometry.x, geometry.y, geometry.width, geometry.height, geometry.scale
        ),
    )?;
    fs::rename(temporary, path)
}

fn snapshot(window: &Window, scale: f64) -> Option<Geometry> {
    // Wayland may reject global positions. Size fitting still works; the
    // compositor retains placement ownership on platforms without this API.
    let position = window
        .outer_position()
        .ok()
        .or_else(|| {
            window
                .current_monitor()
                .or_else(|| window.available_monitors().next())
                .map(|monitor| monitor.position())
        })
        .unwrap_or(PhysicalPosition::new(0, 0));
    let size = window.inner_size();
    if size.width == 0 || size.height == 0 || !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    Some(Geometry {
        x: f64::from(position.x),
        y: f64::from(position.y),
        width: f64::from(size.width) / scale,
        height: f64::from(size.height) / scale,
        scale,
    })
}

fn normal(window: &Window) -> bool {
    !window.is_maximized()
        && window.fullscreen().is_none()
        && !window.is_minimized().unwrap_or(false)
}

fn bounds(position: PhysicalPosition<i32>, size: PhysicalSize<u32>, scale: f64) -> Area {
    Area {
        x: f64::from(position.x),
        y: f64::from(position.y),
        width: f64::from(size.width) / scale,
        height: f64::from(size.height) / scale,
        scale,
    }
}

fn contains(geometry: Geometry, area: Area, border: (f64, f64)) -> bool {
    geometry.x >= area.x
        && geometry.y >= area.y
        && geometry.x + (geometry.width + border.0) * area.scale <= area.x + area.width * area.scale
        && geometry.y + (geometry.height + border.1) * area.scale
            <= area.y + area.height * area.scale
}

fn selected_monitor(window: &Window, geometry: Geometry) -> Option<MonitorHandle> {
    window
        .available_monitors()
        .filter(|monitor| {
            overlap(
                geometry,
                bounds(monitor.position(), monitor.size(), monitor.scale_factor()),
            ) > 0.0
        })
        .max_by(|a, b| {
            overlap(geometry, bounds(a.position(), a.size(), a.scale_factor())).total_cmp(&overlap(
                geometry,
                bounds(b.position(), b.size(), b.scale_factor()),
            ))
        })
        .or_else(|| window.current_monitor())
        .or_else(|| window.primary_monitor())
        .or_else(|| window.available_monitors().next())
}

fn set_minimum(window: &Window, geometry: Geometry) {
    window.set_min_inner_size(Some(PhysicalSize::new(
        (640.0_f64.min(geometry.width) * geometry.scale)
            .round()
            .max(1.0) as u32,
        (420.0_f64.min(geometry.height) * geometry.scale)
            .round()
            .max(1.0) as u32,
    )));
}

fn border(window: &Window, scale: f64) -> (f64, f64) {
    let inner = window.inner_size();
    let outer = window.outer_size();
    (
        f64::from(outer.width.saturating_sub(inner.width)) / scale,
        f64::from(outer.height.saturating_sub(inner.height)) / scale,
    )
}

fn physical_size(geometry: Geometry) -> PhysicalSize<u32> {
    PhysicalSize::new(
        (geometry.width * geometry.scale).round().max(1.0) as u32,
        (geometry.height * geometry.scale).round().max(1.0) as u32,
    )
}

/// Restore normal dimensions only when the user did not provide an initial grid;
/// explicit CLI position is retained. Invalid/offscreen values are fitted once.
pub(super) fn startup(
    window: &Window,
    desired: PhysicalSize<u32>,
    path: Option<&Path>,
    restore_size: bool,
    restore_position: bool,
) -> PhysicalSize<u32> {
    let scale = window.scale_factor();
    let Some(mut geometry) = snapshot(window, scale) else {
        return desired;
    };
    geometry.width = f64::from(desired.width) / scale;
    geometry.height = f64::from(desired.height) / scale;
    let stored = path.and_then(load);
    if let Some(saved) = stored {
        if restore_size {
            geometry.width = saved.width;
            geometry.height = saved.height;
        }
        if restore_position {
            geometry.x = saved.x;
            geometry.y = saved.y;
            geometry.scale = saved.scale;
        }
    }
    let Some(monitor) = selected_monitor(window, geometry) else {
        return desired;
    };
    let frame = border(window, scale);
    let full = bounds(monitor.position(), monitor.size(), monitor.scale_factor());
    // A measured work area detects Dock/menu changes. On unsupported platforms,
    // preserve a known in-bounds normal edge placement instead of guessing it invalid.
    let target = platform_area(&monitor).unwrap_or_else(|| {
        if stored.is_some() && contains(geometry, full, frame) {
            full
        } else {
            area(&monitor)
        }
    });
    let geometry = fit(
        geometry,
        target,
        stored.is_none() && restore_size && restore_position,
        frame,
    );
    // Native backends interpret physical constraints at the current monitor's
    // DPI. Release old constraints and move before applying the target minimum.
    window.set_min_inner_size(None::<PhysicalSize<u32>>);
    if window.outer_position().is_ok() {
        window.set_outer_position(PhysicalPosition::new(
            geometry.x.round() as i32,
            geometry.y.round() as i32,
        ));
    }
    set_minimum(window, geometry);
    window
        .request_inner_size(physical_size(geometry))
        .unwrap_or_else(|| window.inner_size())
}

/// Keep the normal logical dimensions through DPI changes, fitting only if the
/// new monitor cannot contain them. Fullscreen/maximized modes remain explicit.
pub(super) fn scale_changed(
    window: &Window,
    previous_scale: f64,
    scale: f64,
    writer: &mut InnerSizeWriter,
) {
    let Some(monitor) = window.current_monitor() else {
        return;
    };
    let usable = platform_area(&monitor)
        .unwrap_or_else(|| usable_area(monitor.position(), monitor.size(), scale));
    // Physical minimum constraints must track DPI even in an immersive mode.
    if !normal(window) {
        set_minimum(
            window,
            Geometry {
                x: usable.x,
                y: usable.y,
                width: usable.width,
                height: usable.height,
                scale,
            },
        );
        return;
    }
    let Some(geometry) = snapshot(window, previous_scale) else {
        return;
    };
    let frame = border(window, previous_scale);
    let full = bounds(monitor.position(), monitor.size(), scale);
    let target = platform_area(&monitor).unwrap_or_else(|| {
        if contains(geometry, full, frame) {
            full
        } else {
            usable
        }
    });
    let geometry = fit(geometry, target, false, frame);
    set_minimum(window, geometry);
    if window.outer_position().is_ok() {
        window.set_outer_position(PhysicalPosition::new(
            geometry.x.round() as i32,
            geometry.y.round() as i32,
        ));
    }
    let _ = writer.request_inner_size(physical_size(geometry));
}

pub(super) fn save(window: &Window, path: &Path) {
    if !normal(window) {
        return;
    }
    if let Some(geometry) = snapshot(window, window.scale_factor()) {
        let _ = write(path, geometry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn screen() -> Area {
        usable_area(
            PhysicalPosition::new(-2880, 0),
            PhysicalSize::new(2880, 1800),
            2.0,
        )
    }
    fn geometry() -> Geometry {
        Geometry {
            x: -2600.0,
            y: 100.0,
            width: 828.0,
            height: 545.0,
            scale: 2.0,
        }
    }
    #[test]
    fn existing_80_by_24_default_is_retained() {
        let g = fit(geometry(), screen(), true, (0.0, 0.0));
        assert_eq!((g.width, g.height), (828.0, 545.0));
    }
    #[test]
    fn valid_saved_large_window_and_edge_position_survive() {
        let a = screen();
        let g = Geometry {
            x: a.x,
            y: a.y,
            width: a.width,
            height: a.height,
            scale: a.scale,
        };
        assert_eq!(fit(g, a, false, (0.0, 0.0)), g);
    }
    #[test]
    fn small_screen_keeps_controls_accessible_without_overflow() {
        let a = usable_area(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(800, 600),
            1.0,
        );
        let g = fit(geometry(), a, true, (0.0, 24.0));
        assert!(g.width <= 752.0 && g.height <= 448.0);
        assert!(g.width >= 640.0);
        assert!(g.y + g.height + 24.0 <= a.y + a.height);
    }
    #[test]
    fn removed_monitor_is_recovered_and_negative_origin_is_supported() {
        let a = screen();
        let g = fit(
            Geometry {
                x: 8000.0,
                y: -4000.0,
                width: 1800.0,
                height: 1000.0,
                scale: 1.0,
            },
            a,
            false,
            (0.0, 24.0),
        );
        assert!(g.x >= a.x && g.y >= a.y);
        assert!(g.x + (g.width * 2.0) <= a.x + a.width * 2.0);
    }
    #[test]
    fn largest_physical_overlap_selects_target_monitor() {
        let a = screen();
        let b = usable_area(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(1920, 1080),
            1.0,
        );
        assert!(overlap(geometry(), a) > overlap(geometry(), b));
    }
    #[test]
    fn logical_size_is_preserved_across_dpi() {
        let g = fit(
            geometry(),
            usable_area(
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(1920, 1080),
                1.0,
            ),
            false,
            (0.0, 0.0),
        );
        assert_eq!(physical_size(g), PhysicalSize::new(828, 545));
        assert_eq!(physical_size(geometry()), PhysicalSize::new(1656, 1090));
    }
    #[test]
    fn saved_screen_edge_does_not_receive_first_launch_margins() {
        let full = bounds(
            PhysicalPosition::new(-2880, 0),
            PhysicalSize::new(2880, 1800),
            2.0,
        );
        let g = Geometry {
            x: full.x,
            y: full.y,
            width: 1000.0,
            height: 700.0,
            scale: 2.0,
        };
        assert!(contains(g, full, (0.0, 24.0)));
        assert_eq!(fit(g, full, false, (0.0, 24.0)), g);
    }
    #[test]
    fn mac_bottom_origin_insets_map_to_target_physical_origin() {
        let a = visible_area(
            PhysicalPosition::new(-2880, -1800),
            PhysicalSize::new(2880, 1800),
            2.0,
            Rect {
                x: -1440.0,
                y: 900.0,
                width: 1440.0,
                height: 900.0,
            },
            Rect {
                x: -1344.0,
                y: 996.0,
                width: 1344.0,
                height: 780.0,
            },
        )
        .unwrap();
        assert_eq!(
            (a.x, a.y, a.width, a.height),
            (-2688.0, -1752.0, 1344.0, 780.0)
        );
    }
    #[test]
    fn mac_dock_height_and_side_are_measured_instead_of_fixed() {
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        };
        let low = visible_area(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(1440, 900),
            1.0,
            frame,
            Rect {
                x: 0.0,
                y: 72.0,
                width: 1440.0,
                height: 804.0,
            },
        )
        .unwrap();
        let high = visible_area(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(1440, 900),
            1.0,
            frame,
            Rect {
                x: 0.0,
                y: 180.0,
                width: 1440.0,
                height: 696.0,
            },
        )
        .unwrap();
        assert_eq!((low.y, high.y), (24.0, 24.0));
        assert_eq!(low.height - high.height, 108.0);
        let right = visible_area(
            PhysicalPosition::new(2880, 0),
            PhysicalSize::new(1920, 1080),
            1.0,
            Rect {
                x: 1440.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            Rect {
                x: 1440.0,
                y: 0.0,
                width: 1800.0,
                height: 1056.0,
            },
        )
        .unwrap();
        assert_eq!((right.x, right.y, right.width), (2880.0, 24.0, 1800.0));
    }
    #[test]
    fn saved_work_area_edge_is_preserved_and_dock_change_recovers_geometry() {
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        };
        let a = visible_area(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(2880, 1800),
            2.0,
            frame,
            Rect {
                x: 0.0,
                y: 96.0,
                width: 1440.0,
                height: 780.0,
            },
        )
        .unwrap();
        let g = Geometry {
            x: a.x,
            y: a.y,
            width: 1000.0,
            height: 700.0,
            scale: 2.0,
        };
        assert_eq!(fit(g, a, false, (0.0, 24.0)), g);
        let b = visible_area(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(2880, 1800),
            2.0,
            frame,
            Rect {
                x: 0.0,
                y: 180.0,
                width: 1440.0,
                height: 696.0,
            },
        )
        .unwrap();
        assert!(fit(g, b, false, (0.0, 24.0)).height < g.height);
    }
    #[test]
    fn win32_work_rect_stays_physical_and_taskbar_size_is_variable() {
        let a = physical_work_area(-2880, 48, 2880, 1704, 2.0);
        assert_eq!(
            (a.x, a.y, a.width, a.height),
            (-2880.0, 48.0, 1440.0, 852.0)
        );
        let b = physical_work_area(-2880, 180, 2880, 1572, 2.0);
        assert_eq!(a.height - b.height, 66.0);
        let g = Geometry {
            x: a.x,
            y: a.y,
            width: 1000.0,
            height: 700.0,
            scale: 2.0,
        };
        assert_eq!(fit(g, a, false, (0.0, 24.0)), g);
    }
    #[test]
    fn macbook_air_and_large_native_work_area_matrix() {
        for (width, height) in [
            (1024.0, 640.0),
            (1280.0, 720.0),
            (1280.0, 800.0),
            (1440.0, 800.0),
            (1470.0, 833.0),
            (1920.0, 1040.0),
            (2560.0, 1400.0),
        ] {
            for scale in [1.0, 2.0] {
                let area = Area {
                    x: -width * scale,
                    y: 32.0 * scale,
                    width,
                    height,
                    scale,
                };
                let oversized = Geometry {
                    x: 9000.0,
                    y: -9000.0,
                    width: 3000.0,
                    height: 2000.0,
                    scale,
                };
                for first in [true, false] {
                    let fitted = fit(oversized, area, first, (0.0, 24.0));
                    assert!(contains(fitted, area, (0.0, 24.0)));
                    assert!(physical_size(fitted).width as f64 <= (width * scale).round());
                    assert!(physical_size(fitted).height as f64 <= (height * scale).round());
                }
                let saved = Geometry {
                    x: area.x + 20.0 * scale,
                    y: area.y + 20.0 * scale,
                    width: 640.0,
                    height: 400.0,
                    scale,
                };
                assert_eq!(fit(saved, area, false, (0.0, 24.0)), saved);
            }
        }
    }
    #[test]
    fn corrupt_storage_is_ignored() {
        for text in [
            "",
            "0 0 NaN 525 2",
            "0 0 800 525 0",
            "0 0 20 20 1",
            "0 0 800 525 1 extra",
        ] {
            assert!(parse(text).is_none());
        }
    }
    #[test]
    fn synthetic_storage_round_trip_and_atomic_replacement() {
        let dir =
            std::env::temp_dir().join(format!("rterm-native-window-test-{}", std::process::id()));
        let path = dir.join("geometry.txt");
        write(&path, geometry()).unwrap();
        assert_eq!(load(&path), Some(geometry()));
        let updated = Geometry {
            width: 1000.0,
            ..geometry()
        };
        write(&path, updated).unwrap();
        assert_eq!(load(&path), Some(updated));
        fs::remove_dir_all(dir).unwrap();
    }
}
