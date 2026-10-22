# Workspace implementation and console compatibility

The [approved design](design/rterm-ui-main-2026-10-18-v2.png) is the reference
for this change: one top tab row, a compact header for each terminal pane,
search/split/settings controls, and a bottom status bar. Terminal output in
the implementation comes from real local PTYs; example paths, Git status and
development-server output in the design are not seeded into sessions.

## Session metadata

WebSocket protocol v1 retains its existing required fields. The `opened`
response additionally includes optional `shell` and `cwd` strings. `shell`
is the basename of the actual child program; `cwd` is the directory explicitly
passed when spawning the PTY. Clients that do not recognize the fields remain
compatible. Shell title and OSC 7 events can provide later updates; an initial
directory is not a guarantee that a shell startup file has not changed it.

The authenticated WebSocket integration test starts a real PTY and checks
these fields against the spawned command and directory. Local-only binding,
authentication and the shared eight-session backend limit remain in effect.

## Workspace behavior

Both front ends retain top-level tabs and provide independent terminal panes.
The Web/Tauri client stores a split tree per tab, supports nested horizontal
and vertical splits, and offers the design's 58/42 three-pane layout. Pane
menus expose split and close actions; divider dragging updates actual PTY
dimensions. Its eight-pane budget includes every tab. Hidden tabs release
WebGL resources and disconnect resize observers; closed panes dispose their
socket, terminal and add-ons.

Web search operates on the focused xterm buffer. Match positions use terminal
cell widths rather than JavaScript string offsets, so Chinese, emoji and
combining text select the correct columns. Output, reset, reflow and scrollback
changes invalidate the result cache. Empty results clear the selection and
remain cached until the buffer or query changes. Font size, theme, renderer
and scrollback settings are stored locally. A failed GPU add-on is disposed
before falling back to CPU rendering.

The native toolbar connects to the existing search and command actions. Its
split button opens a right pane and then a lower pane; right-click exposes
direction choices. The settings menu provides font zoom/reset, configuration
reload and full-screen controls. Pane menus provide inspect, split and close.
The native layout reserves separate header/footer rows; the status bar
reports the active pane's dimensions, matching its real PTY.

The compact macOS workspace draws its traffic-light buttons in that same
strip and dispatches actual close, minimize and maximize operations. This
avoids AppKit's transparent title-bar view intercepting toolbar clicks.
Explicit button order and style remain configurable. Search displays its
query and match position in the status strip and accepts clipboard paste,
including Chinese text. Compact tab titles reserve complete CJK/emoji cell
spans, and horizontal dragging uses the same row origin as pointer motion.
Palette/search overlays suppress only the rendered
terminal cursor; copy and quick-select retain their own cursor behavior.

Full-window geometry is independent of the active pane's terminal size.
Splitting, dragging and closing panes resize their actual PTYs; new workers
start with their pane's dimensions. Pending windows retain their transport
state until their frame is known. Display-scale changes retain logical rows
and columns before changing cell dimensions. A real native-window regression
requests 120 columns and 32 rows and checks the child's `stty size` output
after six and seven seconds on the ordinary GPU path. A manual launch also
confirmed a stable 120 × 32 footer and `STABLE-GRID=32 120` from the real PTY.

The real Chrome functional test covers three independent panes, resizing,
divider dragging, pane closure, Unicode search selection, empty-result
selection clearing, search after reflow, and settings. Two production browser
tests additionally check ordinary terminal input/lifecycle and the desktop
three-pane layout without the functional observer hook.

```sh
cd web
npm run build
RTERM_BROWSER_CHANNEL=chrome npm run test:e2e:production
```

The channel variable selects an installed system Chrome on this machine.
Without it, the checked-in production configuration uses Playwright Chromium.

## Native GPU text and retained raster work

The live settings check exposed five native overlay builders that updated
`RenderCell.ch` while leaving `RenderCell.text` as a space. The GPU text path
uses `text`. Command palette, input/character selectors, debug overlay and
tab navigator now update both representations. Context menus refresh their
title after installing the settings/pane actions. Pane headers retain complete
grapheme clusters and their display width.

A foreground debug-build sample showed most main-thread time in glyph
rasterization. Invisible glyphs such as spaces previously returned no image
without retaining that result. They now occupy an `Empty` entry in the same
bounded raster cache, with accounted metadata and the same catalog, DPI,
zoom and eviction rules. The cache key distinguishes visible requests so
an empty entry cannot suppress a required visible fallback. Tests cover
repeat hits, fallback visibility, invalidation and budget eviction.

GPU raster scale now follows configured font size and zoom as well as DPI,
so reducing font size no longer leaves oversized glyphs overlapping cells.
The retained-cache behavior and text-scale changes are verified by tests.
The unlocked desktop check verified toolbar-center clicks, visible menu text,
Chinese search paste, zoomed glyphs and actual split-pane dimensions. A
maximized-window profile exposed text-blink paint invalidating unchanged
rows. The renderer now retains unchanged rows and invalidates only affected
blink rows; retained glyph counters remain separate from preparation work.
Full native integration passed with the retained-counter fix. The final
preview and automated checks passed. After unlocking, the latest binary also
passed live checks of the 3456 × 2234 full-screen three-pane command palette,
compact Chinese tab titles, both split-divider directions, independent PTYs,
Chinese search (1/1), font zoom and settings. The two-row horizontal-divider
drag moved the header by 52 pixels, matching two 26-pixel rows. The preview
was restored to 1496 × 950 with two tabs and three panes and remains running.
Final screenshots are `native-workspace-final.jpg`, `native-search-latest.jpg`,
`native-settings-latest.jpg` and `native-large-palette-final.jpg`; process/hash
and action scope are recorded in `native-manual-acceptance-final.json`.
Background idle CPU is not evidence
of a foreground improvement. The temporary development preview is at
`target/workspace-preview/R-Term Workspace.app`.

The large-window palette check also exposed a 64 KiB instance-budget setting
in the production GPU renderer. A 91,648-byte graph exceeded that setting.
Production now uses the renderer's existing 64 MiB bounded default. Persistent
instance buffers still grow lazily, and both the budget and device limits
remain enforced. A headless 3456 × 2010 three-pane graph regression verifies
complete upload above the old limit; explicit small-budget rejection tests
remain intact.

## Evidence and scope

| Validation | Result |
| --- | --- |
| Application `developer-full` unit suite | 4,157 passed; 3 intentionally ignored |
| Native window integration suite | 13 passed; 6 dedicated-runner scenarios ignored |
| Terminal/runtime unit suites | 715 passed |
| Bounded worker/mailbox suites | 34 passed |
| Web backend | 21 passed |
| Web protocol unit suite | 4 passed |
| Real Chrome | 1 functional and 2 production tests passed |
| Font/cache suites | 52 passed |
| GPU text integration suite | 26 passed |
| GPU instance capacity/boundary regressions | 5 passed |
| Real console software | 25 scenarios passed in each of 3 runs |
| Strict Clippy, workspace/all-target compilation, formatting | Passed |
| Architecture and independent feature graphs | Passed; 151 checked files, no violations |
| Frozen provenance | 356 tests and 356 traces preserved |

Native process tests exercise real PTY/GPU frames, nonzero child exit status
and process cleanup. Manual native checks verify that the first terminal row
is below its header and `stty size` agrees with the active pane's footer.
The latest display-scale regression also checks the configured initial grid.
The full application suite passed again after the native follow-up fixes.
The production Web screenshot is `web-workspace-production-3pane.png` in the
evidence directory. `native-settings-before-text-fix.png` intentionally
records the discovered pre-fix failure, rather than a passing final screen.

The real-software matrix is registered as
`rterm-pty/tests/console_software_compatibility.rs`, behind the existing
`runtime-adapter` feature. It drives `LocalPtyTransport` and feeds received
bytes into `TerminalRuntime`, including transport replies to terminal queries.

| Area | Actual checks |
| --- | --- |
| PTY lifecycle | Exit code 7, EOF, resize from 80 x 24 to 132 x 43, Ctrl-C, Ctrl-D, interrupt and close |
| Text and display | Chinese wide cells, emoji, combining graphemes, ANSI 16/256/truecolor, cursor and scroll region |
| Interactive modes | Alternate-screen enter/leave and actual bracketed-paste payload |
| Shells | Bash/Zsh history, arrow-key editing, completion and Ctrl-C without user startup files |
| REPLs | Python and Node calculated results checked in received output and the visible grid; PowerShell with `-NoProfile` |
| Editors and pager | Vim and Nano/Pico file editing, Less full-screen display and exit |
| Local commands | An isolated Git repository/workflow, colored ripgrep, SQLite, awk and a top snapshot |
| Large output | Exactly 20,000 records, including the first and last, with no lost output |
| AI command-line tools | Codex and Claude local version output only; authentication and interactive inference are outside this smoke check |

The matrix ran three times with 25 scenarios each: 8.17 s, 7.39 s and 7.37 s.
Each run passed. Tool inventory is an observation, not a compatibility result:
an installed executable does not count as tested unless it has a scenario.
`fzf`, `tmux`, `fish`, `htop`, `timeout` and `gtimeout` were absent on this host.

```sh
cargo test --locked --offline -p rterm-pty --features runtime-adapter \
  --test console_software_compatibility -- --nocapture
```

The large-output case uses a test reader channel and checks lossless delivery;
it does not measure the production worker's bounded backpressure. Worker and
mailbox regression tests passed separately: 34 checks across `mailbox`,
`burst`, `batching` and `pane_worker`, including a 64 MiB burst. REPL result assertions
use values absent from input so terminal echo cannot satisfy the checks.

```sh
cargo test --locked --offline -p rterm-runtime --features test-support \
  --test mailbox --test burst --test batching --test pane_worker
```

Local evidence is saved under `evidence/workspace-console-2026-10-18/`.
The console tests use isolated temporary files and configuration rather than
editing the user's shell, editor or Git configuration. Results distinguish
software exercised by a real PTY from terminal-protocol regression tests and
from visual interaction checks. Missing software is reported separately.

This is a macOS development-run validation. It does not certify every command
line program, remote SSH server, Windows/Linux runtime, signed release, or
all possible fonts, input methods and accessibility configurations.
