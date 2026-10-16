# Compact startup windows

Desktop windows start hidden and restore their geometry before becoming visible. No first-start maximize/fullscreen action is introduced.

| Entry | Previous default / minimum | New default / minimum (logical pixels) |
| --- | --- | --- |
| `tauri/src-tauri/tauri.conf.json` | 1200 × 760 / 640 × 360 | 1000 × 660 / 640 × 360 |

First use caps the window to 85% of the available desktop work area after a 24 logical pixel edge margin. If that cap would cut into the configured minimum, controls take priority and the minimum is used up to the available work area. Native title bars and borders are included. Monitor coordinates may be negative; the target monitor scale converts content dimensions to physical pixels.

Normal window geometry is saved atomically in the existing application config directory, per window label. Content dimensions are logical pixels; desktop positions and their original scale are recorded separately. Valid saved geometry, including windows placed against a desktop edge or larger than the new default, is retained. On a removed or smaller monitor it is fitted to the monitor with the largest remaining overlap, then the current/primary monitor. Maximized, fullscreen and minimized windows do not overwrite the saved normal geometry. Explicit immersive modes remain available. Mobile geometry is excluded.

Validation on macOS arm64: all new geometry modules and the explicit Search Workbench surface switch were compiled against the real Tauri 2.12.1 APIs in an isolated harness; seven geometry/state regression cases cover first use, user placement, removed monitors, small work areas, configured minimums and corrupt saved data. `git diff --check` passed. This is API/module validation, not a full application build or native-window visual acceptance.

The separate native winit terminal retains its 80 column × 24 row default (800 × 525 grid, 828 × 545 with the modern default padding at 1×). It now starts hidden, fits the normal startup rectangle before showing, and stores logical content dimensions plus physical position/scale in `native-window-geometry.txt` under the existing platform R-SSH state directory. Explicit initial grid/CLI position settings retain priority. Normal move/resize/approved close/application shutdown save atomically; maximized/fullscreen/minimized states do not replace the saved normal rectangle. Diagnostic and frame-limited sessions do not read or write real user geometry. DPI changes fit only when the target display cannot contain the logical dimensions.

Native macOS work areas are measured from `NSScreen.visibleFrame`, matching `NSScreenNumber` to winit's native display ID. Frame-relative logical insets are mapped to the target winit monitor's physical origin and scale, accounting for Cocoa's bottom origin, negative origins, Retina and varying Dock sizes/positions. Native Windows uses winit's HMONITOR and `GetMonitorInfoW.rcWork` in physical coordinates. Platforms without these adapters fall back to reserved top/bottom/side space; Wayland still delegates unsupported global placement to the compositor.

The Windows read-only OS query is the sole function-local unsafe exception, with its handle/pointer safety basis documented. This crate's unsafe lint is `deny`; the rest of the workspace retains `forbid`, and all original clippy warning levels are retained. Only the adapter function permits unsafe code.

Thirteen synthetic geometry/persistence tests passed against the actual winit 0.30.13 and safe AppKit APIs, including dynamic Dock dimensions, mixed-screen coordinates, Windows work rectangles, defaults, DPI, saved edges, storage corruption and atomic replacement. The final native `rssh-app` application `cargo check --locked --offline` passed on macOS arm64 with the Xcode macOS 26.5 SDK. Windows bindings were type-checked by an isolated Mac-host API shim; no Windows-target application build or real Windows window was tested. A read-only Mac screen query returned frame `(0,0,1470,956)` and visible frame `(0,90,1470,833)`; this confirms API availability, not visual window acceptance.

Limits: the Tauri shell full Cargo build and physical window tests were not completed for this batch. The native rterm check is separate from both the Tauri 98-test harness and visual acceptance. Windows/Linux, multi-monitor hot unplug and physical DPI transitions were not visually tested. The existing Tauri 2.11.5 source cache failed internally (`UnexpectedMenuKind` is referenced but missing from its Error enum); no shared cache or lockfile was patched. The successful harness uses a separate compatible dependency resolution.

Review follow-up: the native minimum is expressed in the target monitor’s physical pixels, so moving a saved window from a display with a different scale cannot expand a valid small window. The added regression case covers a saved 640 × 400 window at 1× and 2×.

MacBook Air follow-up: deterministic geometry tests cover logical work areas from 1024 × 640 through 2560 × 1400, including 1280 × 720/800, 1440 × 800 and 1470 × 833, at 1× and 2×. First-use and oversized restore stay within the available work area; a valid 640 × 400 user rectangle remains unchanged and the runtime minimum never exceeds the fitted content. These are simulations, not actual-device visual acceptance.
