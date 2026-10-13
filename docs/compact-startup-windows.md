# Compact startup windows

Desktop windows start hidden and restore their geometry before becoming visible. No first-start maximize/fullscreen action is introduced.

| Entry | Previous default / minimum | New default / minimum (logical pixels) |
| --- | --- | --- |
| `tauri/src-tauri/tauri.conf.json` | 1200 × 760 / 640 × 360 | 1000 × 660 / 640 × 360 |

First use caps the window to 85% of the available desktop work area after a 24 logical pixel edge margin. If that cap would cut into the configured minimum, controls take priority and the minimum is used up to the available work area. Native title bars and borders are included. Monitor coordinates may be negative; the target monitor scale converts content dimensions to physical pixels.

Normal window geometry is saved atomically in the existing application config directory, per window label. Content dimensions are logical pixels; desktop positions and their original scale are recorded separately. Valid saved geometry, including windows placed against a desktop edge or larger than the new default, is retained. On a removed or smaller monitor it is fitted to the monitor with the largest remaining overlap, then the current/primary monitor. Maximized, fullscreen and minimized windows do not overwrite the saved normal geometry. Explicit immersive modes remain available. Mobile geometry is excluded.

Validation on macOS arm64: all new geometry modules and the explicit Search Workbench surface switch were compiled against the real Tauri 2.12.1 APIs in an isolated harness; seven geometry/state regression cases cover first use, user placement, removed monitors, small work areas, configured minimums and corrupt saved data. `git diff --check` passed. This is API/module validation, not a full application build or native-window visual acceptance.

The separate native winit terminal already defaults to 80 columns × 24 rows (approximately 800 × 525 before padding). It is not changed by this Tauri shell patch; native monitor fitting and native geometry persistence remain outside this change.

Limits: full per-application Cargo checks and physical window tests were not completed for this batch. Windows/Linux, multi-monitor hot unplug and physical DPI transitions were not visually tested. The existing Tauri 2.11.5 source cache failed internally (`UnexpectedMenuKind` is referenced but missing from its Error enum); no shared cache or lockfile was patched. The successful harness uses a separate compatible dependency resolution.
