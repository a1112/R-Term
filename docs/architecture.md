# R-Term Native Architecture

Current repository ownership and optional SSH boundaries are described in
[repository independence](formal-split.md). Historical implementation notes below
record behavior at their original checkpoints.

R-Term is an independent native Rust graphical terminal at the **production-parity
foundation** stage. The design separates terminal state, connection I/O,
rendering, and product UI so each layer can be tested and replaced
independently. The current foundation is bounded and evidence-driven; it is not
a 100% WezTerm parity claim.

## Verification Status

At source commit `83ade73a` on 2026-08-02, the Rust 1.89 workspace gates and
focused native GUI, real PTY, SSH/OpenSSH, and unpacked-package checks are
**verified locally on Windows x64**. The six-target native matrices are
**defined in hosted workflow but not run in this local session**. The
fixed-runner performance, signing/notarization/attestation, and hardware
certification work do not share one evidence state: release-integrity work
**requires protected/self-hosted environment**, while hardware/IME/RDP/DPI
certification is **not yet evidenced** and still needs a protected hardware
environment and result workflow. The other five native targets and complete
WezTerm compatibility are also **not yet evidenced**.

Commit `74f78bab` adds the guarded Linux `openssh-server` dependency required by
the isolated real-`sshd` contract. That workflow change passed local static and
cleanup-semantics verification, but no exact-commit hosted Linux run is linked.
Commit `f02acff6` extends the same guard to unsigned and protected release
package-smoke jobs; only its local contract evidence is available here.

The detailed boundary is recorded in the
[production-parity verification record](production-parity-verification.md),
[release contract](release-console.md),
[performance baseline](performance-baseline.md),
[approved design](plans/2026-07-28-production-parity-design.md),
[implementation plan](plans/2026-07-28-production-parity-implementation.md),
and [WezTerm parity gap tracker](research/wezterm-parity-gap.md).

## Goals

- Provide an XShell-like desktop SSH client with native performance.
- Keep the terminal core independent from SSH, local PTY, and rendering.
- Use GPU rendering for dense terminal output and large scrollback.
- Support Windows first, without blocking later Linux and macOS support.
- Store session metadata locally and secrets through OS-backed secure storage.

## Non-Goals

- Do not build the native desktop terminal view on Electron, WebView, or
  `xterm.js`. A separately packaged browser or Tauri client may use xterm.js
  while the Rust backend retains PTY/session ownership.
- Do not shell out to `ssh.exe` as the primary connection engine.
- Do not vendor third-party terminal projects into this repository.
- Do not implement every XShell feature before the terminal and SSH loop is
  stable.

## Layers

```text
Application shell
  Tabs, panes, session tree, command palette, settings, logs

Session runtime
  Owns terminal instances, connection tasks, resize events, lifecycle, replay

Connection layer
  SSH channel, local PTY, future serial/Telnet, SFTP, tunnels

Terminal core
  VT parser, terminal grid, scrollback, selection, hyperlinks, mouse modes

Renderer
  Font shaping, glyph atlas, damage tracking, GPU draw batches, presentation

Storage and security
  Planned SQLite metadata and OS-backed secret storage; known_hosts and logs
  remain explicit runtime boundaries
```

## Primary Data Flow

```text
SSH or PTY byte stream
  -> terminal parser
  -> terminal grid and scrollback mutation
  -> damage regions
  -> renderer batches
  -> GPU presentation

keyboard, mouse, paste, resize
  -> input encoder
  -> active connection channel
  -> remote shell or local PTY
```

## Crate Boundaries

- `rterm-core`: shared domain types, terminal session and application-shell state
  (`Workspace`, `Tab`, `Pane`, actions, IDs) used before runtime ownership.
- `rterm-terminal`: VT parser boundary, grid, scrollback, selection, and input
  encoding.
- `rterm-renderer`: deterministic CPU/offscreen rendering plus direct `wgpu`
  surface presentation and bounded `glyphon` GPU text state.
- `rterm-fonts`: `cosmic-text` shaping, configured font fallback, color-glyph
  metadata, and deterministic bundled test fonts.
- `rterm-ssh`: native `russh` sessions, authentication, full-duplex channels,
  status propagation, and forwarding lifecycles. System OpenSSH SSH/SFTP/SCP
  execution is integrated at the application boundary.
- `rterm-pty`: local shell support through Windows ConPTY and Unix PTY.
- `rterm-test-support`: hermetic SSH/SFTP fixtures and bounded process-tree test
  support shared by integration and package smokes.
- `rterm-app`: desktop entry point, native window, and user-facing action dispatch
  for app-shell state transitions.

## App Shell Status

- Completed in v1: deterministic shell state model in `rterm-core` for one startup
  workspace, tab, and pane with typed IDs and tab/pane/workspace actions.
- Completed in v1: `rterm-app` startup initializes this model from the local PTY
  command and updates the native window title with workspace/tab/pane state.
- Completed in v1: native PTY command construction honors the WezTerm-style
  `term` effective-config value for `TERM`, defaulting to `xterm-256color` and
  allowing overrides such as `wezterm` for newly spawned panes/windows. It also
  honors native `default_prog` and `set_environment_variables` overrides for
  local-domain PTY launches and uses native `default_cwd`, then the user home
  directory, as fallback cwd values when a pane launch has no explicit cwd or
  process-tree cwd. Native pane metadata prefers OSC 7/current-dir cwd and
  falls back to the local session process tree cwd, preferring child processes,
  when the PTY backend exposes a pid. Native new-tab/split/spawn-window
  actions without an explicit launch program use `default_prog` while preserving
  inherited cwd, omitted-spawn `SwitchToWorkspace`/new-workspace creation also
  uses `default_prog`, and no-program `SpawnWindow` requests inherit the active
  pane launch/cwd when no `default_prog` override is active; WezTerm-style Lua
  `default_prog` command arrays parse inline, through top-level static table
  variables with pre/post-assignment `table.insert` appends, or through
  `table.insert(config.default_prog, ...)` appends, and
  `set_environment_variables` maps parse inline, through top-level static
  table variables with pre/post-assignment field mutations including config
  table initializer aliases, through static field names in returned config
  table initializers, direct return-table configs, and top-level
  `config[static_name] = { ... }` assignments, or through top-level
  `config.set_environment_variables.NAME`, `config[static_name].NAME`, and
  `config[static_name]['NAME']` field mutations; native
  `default_workspace` names the initial default workspace before spawn when no
  explicit startup workspace is present; native `default_domain` is retained in
  the effective config and `SpawnTab(DefaultDomain)` uses the local spawn path
  only while the configured default domain is `local`; native
  `prefer_to_spawn_tabs` is retained with WezTerm's default `false` and, when
  enabled, routes unpositioned same-process `SpawnWindow` requests into a new
  tab while preserving positioned spawn-window requests as detached windows;
  static WezTerm-style Lua return-table configs and returned config variables
  are treated as the final returned config table, so earlier assignments to
  unreturned config variables do not leak into launch overrides when the file
  returns `{ ... }` or `return cfg`, and helper-function-local static table
  variables do not leak into returned config parsing; supported direct field
  assignments and whole-table assignments on that returned config use the latest
  static value by source order, and duplicate fields inside static config table
  constructors use the later entry;
  local/window/start/console CLI startup also accepts WezTerm-style `--cwd` for
  the initial child process, native window startup accepts WezTerm-style `start`
  as an alias for `window` and `-e` as an initial program alias, and native
  window startup accepts WezTerm-style
  `--workspace` to name the initial workspace, `--class CLASS` to request the
  native window class name on Windows, and
  `--position X,Y`/`screen:X,Y`/`main:X,Y`/`active:X,Y`/`<monitor>:X,Y` to
  request an initial native window screen position. The requested class is
  retained for additional native windows spawned by the same app process.
  `main:` is relative to the primary monitor origin, `active:` is relative to
  the active monitor when the platform exposes one and otherwise falls back to
  the primary monitor origin, and named monitor forms are relative to the
  matching monitor origin. Native
  window startup also accepts WezTerm startup compatibility flags `--no-auto-connect`,
  `--always-new-process`, and `--new-tab` as current no-ops because there is no
  GUI daemon or auto-connected mux domain yet. `--domain local` selects the
  current local PTY domain and `--attach` is accepted as a no-op until mux
  attachment exists. WezTerm's X11/Wayland class/app-id application,
  remote/named mux domains, and initial CLI startup default-prog selection
  remain future parity work.
- Completed in v1: the native window renders a basic configurable tab bar plus
  right/down split panes, with process-local per-pane runtime storage,
  WezTerm-style `foreground_text_hsb` transforms for terminal foreground and
  underline colors parsed inline or through top-level static table variables,
  with hue/saturation/brightness fields also parsed through top-level static
  number variables or through top-level
  `config[static_name].hue`/`saturation`/`brightness` field mutations where
  `static_name` resolves to `foreground_text_hsb`,
  WezTerm-style `bold_brightens_ansi_colors` handling for
  bold ANSI 0-7 foreground colors, WezTerm-style `underline_thickness` overrides for terminal
  text underline decorations and horizontal split dividers, WezTerm-style `underline_position` overrides for
  terminal text underline placement, WezTerm-style `strikethrough_position`
  overrides for terminal text strikethrough decorations,
  WezTerm-style `text_min_contrast_ratio` foreground adjustment for textual
  terminal cells whose foreground/background contrast falls below the configured
  threshold,
  `text_background_opacity` alpha transforms for non-default terminal
  background cells, `window_background_opacity` alpha transforms for default
  terminal background cells, basic `window_background_gradient`
  Horizontal/Vertical/Linear-angle/Radial color-list fills with colorgrad
  interpolation/blend/segment controls, `wezterm.color.gradient(...)` and legacy
  `wezterm.gradient_colors(...)` color helpers through direct calls or
  top-level static helper aliases, and bounded-static WezTerm Color object
  evaluation backed by `wezterm-color-types 0.3.0`. The evaluator covers
  `wezterm.color.parse(...)`, `wezterm.color.from_hsla(...)`, all 20 pinned
  `ColorWrap` methods, colon-method chains without intermediate 8-bit
  quantization, color/number/bool/string results, tuple-returning
  `triad`/`square`/`srgba_u8`/`linear_rgba`/`hsla`/`laba`, multi-target
  assignments, equality, and `tostring(Color)`. Direct, module, namespace,
  static-key, and constructor aliases feed the same typed resolver before
  adapting Color results once for scalar,
  palette, indexed, tab-bar, visual-bell, selector overlay colors,
  `win32_acrylic_accent_color`, `integrated_title_button_color`,
  `config.background` Color sources, `window_background_gradient.colors`
  entries, and ColorSpec `colors.*` fields plus `window_frame`
  titlebar/button/border color fields, static `config.colors` variable
  mutations, and static `config.color_schemes` entry mutations. Dynamic Lua,
  method extraction/dot-call rebinding, tuple table expansion, and unprovable
  identity/API mutations remain fail-closed. Colorgrad
  preset fills for default window backgrounds,
  and WezTerm `config.background` Color layers composed with source-over alpha
  plus single Gradient layers inline or through top-level
  static layer tables, with Color sources from inline or top-level static source
  tables plus explicit-color and preset Gradient sources mapped onto the same
  native background color/gradient paths with layer opacity and inline or
  static-variable layer `hsb` transforms, while retaining the parsed
  Color/Gradient/File layer stack in the native effective `background`
  snapshot. Multi-layer `config.background` stacks with Color layers below a
  final explicit-color or preset Gradient layer are composed into native
  background color/gradient overrides, explicit
  color-list `window_background_gradient` values can be treated as WezTerm's
  implicit prepended layer below a following Color-only `config.background`
  stack and retained as the first effective `background` layer, legacy
  `window_background_image` files render as 100%-sized background image layers
  with `window_background_image_hsb` and `window_background_opacity`
  and can serve as WezTerm's implicit prepended layer below a following
  Color-only `config.background` stack while being retained as the first
  effective `background` layer; the legacy path and optional HSB transform are
  also retained in the native effective config snapshot, and
  static ordered Gradient/File visual layer stacks, including Color-below-File,
  Color-over-File, multiple File layers, and Gradient-over-File, render through
  the native background layer path from string File sources or File source
  tables with `path` and animated image `speed`, using WezTerm's default
  cover-style sizing, Contain sizing, px/percentage/cell width and height,
  Repeat/Mirror/NoRepeat tiling with optional repeat sizes, offsets,
  Left/Center/Right plus Top/Middle/Bottom alignment, source-over opacity, and
  layer `hsb` transforms, with File layer `attachment` Fixed/Scroll/Parallax
  affecting vertical scrollback sampling, plus
  retained platform backdrop settings
  `kde_window_background_blur`, `macos_window_background_blur`,
  `win32_system_backdrop`, and `win32_acrylic_accent_color`; actual
  KDE/macOS/Windows backdrop and accent application remains platform-adapter work. It also supports
  `inactive_pane_hsb` color transforms for inactive pane Default/Indexed/RGB/RGBA
  cells parsed inline or through top-level static table variables, with
  hue/saturation/brightness fields also parsed through top-level static number
  variables or through top-level
  `config[static_name].hue`/`saturation`/`brightness` field mutations where
  `static_name` resolves to `inactive_pane_hsb`,
  click-to-focus with optional first-click swallowing honoring
  `swallow_mouse_click_on_pane_focus`, configurable window-focus click
  swallowing honoring `swallow_mouse_click_on_window_focus`, configurable
  focus-follows-mouse honoring `pane_focus_follows_mouse`, configurable
  mouse-reporting bypass via
  `bypass_mouse_reporting_modifiers`, and bounded vertical wheel routing over
  an inactive visible pane using hovered-pane local cell and pixel coordinates,
  runtime, UI state, writer, and target-aware bindings without implicit focus
  transfer. Tab-bar wheel switching, the active-pane window-right scrollbar,
  click/move focus, and explicit focus or pane-creation commands remain
  distinct; the broader non-wheel mouse, App Shell v2, font-shaping, and general
  WezTerm parity limits are described below.
- Completed after v1: ordinary selection is owned per pane. Focus and tab
  changes save and restore each pane's independent selection, active and
  inactive panes render their selections simultaneously, and a selection-free
  base snapshot rebuild from inactive PTY output does not by itself
  unconditionally erase ordinary selection because the overlay is applied at
  presentation before `inactive_pane_hsb`. The selection is still cleared when
  its selected stable rows intersect visible rows changed after its stored
  sequence. `ClearSelection`, selected-text extraction, and copy actions
  address only the active pane; closing a pane retires only that pane's
  selection. `MovePaneToNewTab` preserves selection inside the same GUI window,
  while
  `MovePaneToNewWindow` clears GUI selection at the window boundary without
  discarding terminal runtime, scrollback, or viewport state.
- Completed after v1: the stable-selection and dirty-invalidation slice gives
  terminal rows stable scrollback identity with strict stable/physical
  conversion and tracks a terminal sequence number (`seqno`) plus per-row
  sequence values. Ordinary selection and each pane viewport use stable rows;
  selected-text extraction works while rows are offscreen; and scrollback
  pruning preserves surviving row identities instead of retargeting a
  selection. For both active and inactive panes, an ordinary selection is
  cleared only when its selected stable rows intersect visible rows changed
  after the ordinary selection's stored sequence number, captured when the
  selection was established or last extended. While Search, Copy, or Quick
  Select is active, only ordinary-selection dirty-row retirement is exempted:
  PTY processing and base-snapshot repaint continue normally, and accumulated
  row changes are evaluated at the next base-snapshot presentation after the
  overlay exits. Effective palette changes mark every active-domain line dirty.
  Screen-domain switches and height
  changes synchronously retire GUI selection, drag/multi-click state, and
  transient modes. Lua pane dimensions and cursor coordinates expose stable
  rows.
- Completed after v1: bounded width-resize parity is verified against pinned
  WezTerm `093bf6bf2b82b929ed80c04fd54ebc80464f715e`. A main screen, including
  a dormant main screen behind the alternate screen, reflows logical lines with
  its scrollback; the alternate screen receives only physical resize and never
  reflows. Main reflow retires unsafe terminal coordinate metadata. When main
  is visible, its viewport resets/clamps, ordinary selection is cleared, and
  Copy/Search/Quick retain configuration while rebuilding coordinate-derived
  presentation. When alternate is visible, it is only physically resized; the
  saved main reflows in the background and alternate ordinary/overlay UI state
  is preserved. This is not a claim of general selection, rendering, or image
  parity beyond that tested metadata retirement.
- Completed after v1: cell-level `DECLRMM`/`DECSLRM` bounded-margin semantics
  cover `SU`/`SD`, LF/IND/NEL/RI, `IL`/`DL`, `ICH`/`DCH`, right-edge printing,
  insert mode, and auto-wrap while preserving exterior cells and avoiding
  partial-width history/stable-row churn. `ECH` remains physical-right-edge
  erase. Declared-cell inline-image placements now persist one
  `CellAttachment` per covered cell; bounded transforms move or delete those
  attachments with the corresponding cells, so images can split at an LR
  boundary while exterior cells remain unchanged. The renderer resolves the
  attachments at runtime geometry. Character edits transform
  `kitty_placeholder_cells`, `last_kitty_placeholder`, and
  `pending_kitty_placeholder` with their rendered coordinates, including
  relative and high-byte identities, without changing stored payloads or
  explicit-delete semantics. A malformed or footprint-less placement retains
  the conservative retirement fallback. A
  narrow per-placement marker selects the live attachment origin only when it
  conflicts with residual virtual-cache state; resize, rebase, alternate-screen,
  and delete lifecycle paths retire that marker with its placement.
- Completed in v1: vertical wheel input over an inactive visible pane uses that
  pane's local cell and pixel coordinates, runtime, UI state, writer, and
  target-aware binding context without implicit focus transfer. Tab-bar wheel
  switching, the window-right active-pane scrollbar, click/move focus behavior,
  and explicit focus or pane-creation commands remain distinct paths.
- This bounded work does not claim general mouse-event routing, full selection
  parity, full App Shell v2, font-shaping parity, or general WezTerm parity.
  Richer pane focus visuals, arbitrary Lua callbacks, external CLI title
  control, and a real mux/window registry with domain, protocol, and renderer
  parity remain open.
- Completed in v1: tab bar entries include a configurable clickable close
  marker honoring `show_close_tab_button_in_tabs` that closes non-final tabs or
  requests native-window shutdown when the final tab is closed.
- Completed in v1: the tab bar includes configurable tab index visibility
  honoring `show_tab_index_in_tab_bar`, configurable tab-label visibility
  honoring `show_tabs_in_tab_bar`, configurable mouse-wheel tab switching
  honoring `mouse_wheel_scrolls_tabs`, configurable visibility honoring
  `enable_tab_bar` and `hide_tab_bar_if_only_one_tab`, configurable top/bottom
  placement honoring `tab_bar_at_bottom`, configurable zero-based tab index
  labels honoring `tab_and_split_indices_are_zero_based`, configurable active-tab
  close selection honoring `switch_to_last_active_tab_when_closing_tab` for
  default close-tab shortcuts, tab-bar close clicks, and Close Current Tab
  command/confirmation paths, plus a
  configurable clickable new-tab button that reuses the app-shell `NewTab`
  action path and honors the native `show_new_tab_button_in_tab_bar`
  effective-config field. Native config also retains `use_fancy_tab_bar` with
  WezTerm's `true` default; the current renderer remains the retro tab strip,
  so proportional-font native fancy tab rendering is still open. Retro tab
  labels honor `tab_bar_style` edge `wezterm.format` items, and the new-tab
  button honors the current WezTerm `tab_bar_style.new_tab`/`new_tab_hover`
  full-button format fields while retaining the legacy left/right edge fields.
  Static `colors.tab_bar` item styles parse `fg_color`/`bg_color`,
  intensity, italic, strikethrough, and underline values inline or through
  top-level static value variables, including underline
  `None`/`Single`/`Double`/`Curly`/`Dotted`/`Dashed`.
  These fields parse inline or through top-level static table variables,
  including top-level
  `config[static_name].active_tab_left`/`active_tab_right`/`inactive_tab_left`/
  `inactive_tab_right`/`new_tab_left`/`new_tab_right` mutations where
  `static_name` resolves to `tab_bar_style`. When `window_decorations` includes
  WezTerm-style `INTEGRATED_BUTTONS`, the retro tab bar also renders configured
  integrated title buttons in configured order and left/right alignment, applies
  Auto/custom button color, uses WezTerm's retro defaults `.`, `-`, and `X`,
  honors `tab_bar_style.window_hide`/`window_maximize`/`window_close`
  `wezterm.format` label overrides, dispatches Hide/Maximize/Close clicks
  through the native window action paths, and honors `MacOsNative` style in a
  top retro tab bar by reserving native button space instead of drawing text
  buttons.
- Completed in v1: tab state can carry an explicit title, and tab bar labels
  prefer that explicit title before falling back to each tab's active-pane
  terminal title when OSC 0/1/2 or Sun OSC L/l title state is available.
- Completed in v1: the command palette includes Rename Tab with
  quote-aware `rename tab <title>` and action-name `renametab <title>` query
  input, writing explicit titles for the active tab. Static `config.keys`
  `RenameTab` action calls resolve top-level string variables for the title.
- Completed in v1: the command palette includes Rename Workspace with
  quote-aware `rename workspace <name>` and action-name
  `renameworkspace <name>` query input, naming the active workspace from
  user-entered text. Static `config.keys` `RenameWorkspace` action calls
  resolve top-level string variables for the name.
- Completed in v1: app-shell state tracks the last active tab and command
  palette dispatch exposes Activate Last Tab, no-oping when no previous active
  tab exists.
- Completed in v1: app-shell state exposes WezTerm-style zero-based
  `ActivateTab` index semantics through `ActivateTabIndex`, including negative
  indices for right-to-left tab selection, and command-palette Activate Tab
  1 through 9 entries plus `activate tab <index>`/`activate tab index <index>`
  and `activatetab <index>` queries route through the same action; action-name
  `activatelasttab` and `activatetab1` through `activatetab9` queries dispatch
  the corresponding fixed entries. Native `WindowCommand::ActivateTab(index)`
  payloads dispatch arbitrary positive or negative indices through that same
  app-shell path. The default
  `Ctrl+Shift+1..9` and `Super+1..9` key-assignment entries expose
  `ActivateTab(0..7/-1)` payloads while retaining numbered `Activate Tab 1..9`
  launcher labels. Static `config.keys` action calls resolve top-level signed
  integer variables for `ActivateTab`.
- Implemented in v1: native `ShowTabNavigator` opens a tab-list overlay with
  the active tab initially selected and activates the selected tab on Enter;
  action-name `showtabnavigator` queries dispatch the same command.
- Completed in v1: app-shell state exposes WezTerm-style `ActivateTabRelative`
  wrapping plus `ActivateTabRelativeNoWrap` clamping, and the command palette
  includes wrapping and no-wrap Next/Previous Tab entries. Native
  `WindowCommand::ActivateTabRelative(offset)` and
  `WindowCommand::ActivateTabRelativeNoWrap(offset)` payloads dispatch arbitrary
  relative offsets through the same app-shell paths; structured queries accept
  `activate tab relative <offset>` / `activatetabrelative <offset>` and
  `activate tab relative no wrap <offset>` /
  `activatetabrelativenowrap <offset>`; action-name `nexttab`,
  `previoustab`, `nexttabnowrap`, and `previoustabnowrap` queries dispatch the
  corresponding fixed entries. The default `Ctrl+Tab`, `Ctrl+Shift+Tab`,
  `Ctrl+PageUp`, `Ctrl+PageDown`, and `Super+Shift+[/]` key-assignment entries
  expose `ActivateTabRelative` payloads while the command palette keeps
  Next/Previous Tab aliases. Static `config.keys` action calls resolve
  top-level signed integer variables for `ActivateTabRelative` and
  `ActivateTabRelativeNoWrap`.
- Completed in v1: app-shell `MoveTabRelative` reorders the active tab within
  the current workspace while keeping that tab active, with command-palette
  Move Tab Relative Left/Right entries for one-step movement and native
  `WindowCommand::MoveTabRelative(offset)` payloads for arbitrary relative
  offsets, plus `move tab relative <offset>` /
  `movetabrelative <offset>` structured queries and action-name
  `movetabrelativeleft` / `movetabrelativeright` fixed entries. Static
  `config.keys` action calls resolve top-level signed integer variables for
  `MoveTabRelative`.
- Completed in v1: app-shell `MoveTab` reorders the active tab to a zero-based
  absolute tab index and returns a typed out-of-range error, with command-palette
  entries for Move Tab To 1 through 8, `move tab <index>`/`move tab to <index>`
  / `movetab <index>` queries, action-name `movetabto1` through `movetabto8`
  fixed entries, and native `WindowCommand::MoveTab(index)` payloads for
  arbitrary zero-based indices. Static `config.keys` action calls resolve
  top-level unsigned integer variables for `MoveTab`.
- Implemented in v1: app-shell `Nop` is a true no-effect action and preserves
  active IDs, workspace/tab/pane collections, and active-pane unseen-output
  state. Native `WindowCommand::Nop` maps to the same no-effect action for
  window-level action payloads, and structured command-palette `nop` queries
  dispatch that payload directly.
- Implemented in v1: native user `key_assignments` match regular key presses
  before built-in default shortcuts and dispatch the configured native
  `WindowCommand` subset, so user bindings can override defaults while
  `DisableDefaultAssignment` remains the opt-out path below. Native key strings
  accept WezTerm-style `|` modifier grouping, such as `CTRL|ALT+D` and
  `LEADER|SHIFT+|`, in addition to the existing `+`-separated shorthand, and
  honor the documented `SUPER`/`CMD`/`WIN` plus `ALT`/`OPT`/`META` modifier
  aliases. The native key matcher also recognizes WezTerm-style `F1` through
  `F24` function-key identifiers, physical `Numpad0` through `Numpad9` and
  numpad operator identifiers, browser navigation identifiers, and native named
  identifiers for lock keys, `PrintScreen`, `Pause`, `Menu`/`ContextMenu`,
  media transport keys, and audio volume keys including WezTerm's documented
  `MediaNextTrack`/`MediaPrevTrack` and `Volume*` aliases. Explicit
  WezTerm-style `phys:` and `mapped:` key prefixes are supported for native key
  assignments so physical letter/digit positions can be distinguished from
  mapped layout output, and `raw:<decimal>` matches platform-native
  unidentified physical keycodes. Native `key_map_preference` supports the
  default `Mapped` behavior and `Physical` matching for unprefixed
  letter/digit user assignments and default app-shell/window shortcuts. Native
  `enable_csi_u_key_encoding` defaults to false and, when enabled, routes
  modified ASCII keys through CSI-u encoding while preserving the default legacy
  key encoding path. Native `enable_kitty_keyboard` defaults to false and, when
  enabled, honors kitty keyboard protocol negotiation sequences and flags
  queries. Native `enable_kitty_graphics` defaults to true and, when disabled,
  suppresses Kitty graphics query responses, uploads, placements, and rendered
  inline images. Native `enable_checksum_rectangular_area` defaults to false,
  consumes DECRQCRA requests without replying by default, and emits WezTerm-style
  DCS checksum responses for visible terminal cells when enabled. Native
  `enable_title_reporting` defaults to false, consumes window-title reports
  without replying by default, and replies to `CSI 21t` with a WezTerm-style Sun
  window-title OSC when enabled. Native `enq_answerback` defaults to an empty
  string and, when configured, replies to top-level ENQ (`0x05`) controls with
  that answerback while ignoring ENQ bytes inside OSC/DCS strings. Native
  `allow_download_protocols` defaults to true, while `xcursor_theme`,
  `xcursor_size`, and `palette_max_key_assigments_for_action` are retained with
  WezTerm defaults for future download-protocol gating, platform cursor theme/
  size application, and palette assignment limiting. Native
  `allow_win32_input_mode` defaults to true, tracks ConPTY
  `CSI ? 9001 h/l` mode requests, and makes native-window and local console
  input emit Win32 key records for that mode before CSI-u/kitty encoding.
  Static Lua snippets for `key_map_preference`, `swap_backspace_and_delete`,
  `ui_key_cap_rendering`, `enable_csi_u_key_encoding`,
  `enable_kitty_graphics`, `enable_checksum_rectangular_area`, `enable_title_reporting`,
  `enable_kitty_keyboard`, `allow_download_protocols`, `xcursor_theme`,
  `xcursor_size`, `palette_max_key_assigments_for_action`,
  `allow_win32_input_mode`, `treat_left_ctrlalt_as_altgr`,
  `send_composed_key_when_left_alt_is_pressed`,
  `send_composed_key_when_right_alt_is_pressed`,
  `treat_east_asian_ambiguous_width_as_wide`,
  `normalize_output_to_unicode_nfc`, `unicode_version`, `bidi_enabled`,
  `bidi_direction`, `use_ime`, `use_dead_keys`, `ime_preedit_rendering`,
  `macos_forward_to_ime_modifier_mask`, and
  `xim_im_name` parse into the native override path, including returned config
  table initializers and direct
  `return { [static_name] = ... }` config tables whose static key variables
  resolve to those field names. Static Lua snippets for
  `default_gui_startup_args`, `ssh_backend`,
  `tiling_desktop_environments`, `default_mux_server_domain`,
  `daemon_options`,
  `exec_domains`,
  `wsl_domains`,
  `unix_domains`,
  `ssh_domains`,
  `tls_servers`,
  `tls_clients`,
  `serial_ports`,
  `ratelimit_mux_line_prefetches_per_second`,
  `mux_output_parser_buffer_size`,
  `mux_output_parser_coalesce_delay_ms`, `periodic_stat_logging`,
  `ulimit_nofile`, and `ulimit_nproc` are also retained in effective config
  with WezTerm defaults; actual GUI startup-arg consumption, SSH backend
  selection, tiling desktop-environment behavior, mux line-prefetch limiting,
  output-parser buffering, daemon pid/log redirection, periodic stat logging,
  and rlimit application remain future parity work. `ui_key_cap_rendering`
  controls native command-palette key-assignment
  display labels with UnixLong, Emacs, AppleSymbols, WindowsLong, and
  WindowsSymbols styles. `treat_east_asian_ambiguous_width_as_wide` updates
  terminal character width calculation for ambiguous East Asian width characters
  in active and new panes. Static numeric `cell_widths` override tables parse
  inline, through top-level static table variables, or through
  `table.insert(config.cell_widths, { ... })` appends from WezTerm-style Lua,
  including `config[static_name]` assignments and
  `table.insert(config[static_name], { ... })` or
  `config[static_name][#config[static_name] + 1] = { ... }` appends where
  `static_name` resolves to `cell_widths`, plus table-constructor
  `[static_name] = { ... }` fields for directly returned or returned-variable
  config tables. Entries parse inline or through top-level static table
  variables. Entry
  `first`/`last`/`width` fields parse inline or through top-level static
  number variables, and take priority over the ambiguous-width setting in
  active and new panes.
  `normalize_output_to_unicode_nfc` applies NFC
  normalization to contiguous ordinary terminal output runs before rendering,
  including leading combining marks that arrive in the next PTY chunk when
  they compose with the prior cell without changing display width.
  `unicode_version` is retained with WezTerm's default `9` in effective config,
  static numeric Lua snippets, including static field-name config table
  initializers, configure active and new pane runtimes, and OSC 1337
  `UnicodeVersion` set/push/pop sequences, including labeled entries, update
  terminal state. Unicode 8-or-earlier runtimes keep WezTerm's
  `WidenedIn9` characters narrow, Unicode 9+ widens them, and Unicode 14+
  emoji/text presentation selectors adjust the prior cell width for FE0F/FE0E
  sequences. `bidi_enabled` and `bidi_direction` are retained with WezTerm's
  `false` and `LeftToRight` defaults and parse static Lua snippets into
  effective config; actual bidi shaping/rendering remains future parity work.
  `treat_left_ctrlalt_as_altgr` makes Ctrl+Alt text key events use the
  AltGr text path rather than Ctrl+Alt key bindings. Left/right Alt side state
  honors WezTerm's composed-key defaults: left Alt keeps Meta behavior, while
  right Alt writes composed text without an ESC Meta prefix unless a raw Alt key
  assignment matches first. Native winit IME preedit text renders through the
  Builtin overlay path at the active pane cursor, and
  static Lua `colors.compose_cursor` overrides the cursor color while Builtin
  preedit text, the leader modifier, or a dead key is active. The documented
  `window:composition_status()` status shape exposes Builtin preedit or current
  dead-key text to static status callbacks. The `use_dead_keys` override
  disables the native dead-key pending-composition state while retaining
  WezTerm's default `true`. Platform IME/XIM connection,
  broader dead-key expansion, and broader dynamic `cell_widths` Lua
  parity remain future parity work.
- Implemented in v1: native `leader` overrides expose a WezTerm-style
  `LEADER` modal modifier subset for native user `key_assignments`. Pressing
  the configured leader key arms the virtual modifier until the next key press
  or `timeout_milliseconds`; while active, only `LEADER` assignments are
  matched and unmatched keys are swallowed before returning to the normal input
  path. Static Lua `config.keys` and `config.key_tables` action fields also
  accept `wezterm.action_callback(...)` values as no-op native placeholders so
  official callback-shaped bindings can load, static table variable assignments
  such as `config.keys = user_keys` or
  `config.key_tables = user_key_tables`, or `config.leader = user_leader`.
  Top-level `config.keys` and `config.key_tables` assignment `key` and `mods`
  fields parse inline or through top-level static string variables.
  Top-level `config.keys` assignment `action` fields parse inline or through
  top-level static action variables, including top-level static
  `local <alias> = wezterm.action` aliases for dot and string-index action
  constructors, and static action variables inside `act.Multiple { ... }`,
  `act.QuickSelectArgs { action = ... }`, and
  `act.Confirmation { action = ..., cancel = ... }` tables.
  Static `config.key_tables` `CopyMode` action payloads resolve top-level
  string variables for assignment names, `SetSelectionMode`, and semantic-zone
  type fields, top-level bool variables for nested `prev_char`, nested jump
  option table variables, and top-level number variables for `MoveByPage`, plus
  parenthesized static assignment table variables.
  `config.key_tables = { [name] = ... }` key-table names and nested insert
  targets such as `config.key_tables[name]` also resolve top-level static
  string variables.
  Static `config.keys = user_keys` assignments also merge top-level
  `table.insert(user_keys, { ... })` appends and indexed assignments such as
  `user_keys[1] = { ... }` or `user_keys[#user_keys + 1] = { ... }`, plus
  direct indexed field mutations on existing binding tables such as
  `user_keys[1].key = 'H'`, `user_keys[1].mods = 'CTRL|SHIFT'`, and
  `user_keys[1].action = act.SendString '...'`, before the config assignment
  and while tracking the same variable after `config.keys = user_keys`.
  Leader `key`, `mods`, and `timeout_milliseconds` fields parse inline or
  through top-level static scalar variables, direct top-level config field
  mutations such as `config.leader.key = 'a'`,
  `config.leader.mods = 'CTRL'`, and
  `config.leader.timeout_milliseconds = 1000`, and
  `config.leader = user_leader` also merges post-assignment top-level field
  mutations such as `user_leader.key = 'a'`, `user_leader.mods = 'CTRL'`, and
  `user_leader.timeout_milliseconds = 1000`. Static return-table fields such as
  `return { keys = user_keys }` or
  `return { key_tables = user_key_tables }`, and top-level static
  `table.insert(config.keys, { ... })` appends, static item table variables
  used as `config.keys = { binding }` or
  `table.insert(config.keys, binding)` with pre-use field mutations such as
  `binding.key = 'K'`, `binding.mods = 'CTRL|SHIFT'`, and
  `binding.action = act.SendString '...'`, and direct indexed assignments such
  as `config.keys[index] = { ... }`, `config.keys[index] = binding`, or
  `config.keys[#config.keys + 1] = { ... }`, plus direct indexed field
  mutations on existing binding tables such as `config.keys[1].key = 'K'`,
  `config.keys[1].mods = 'CTRL|SHIFT'`,
  `config.keys[1].action = act.SendString '...'`, or the same
  `config[static_name][1].field` form where `static_name` resolves to `keys`, plus
  `table.insert(config.key_tables.<name>, { ... })` nested appends and
  static table variables such as
  `table.insert(config.key_tables.<name>, item)` or
  `table.insert(config.key_tables.<name>, index, item)`, plus
  `table.insert(config.key_tables.<name>, index, { ... })` numeric-position
  inserts and direct indexed assignments such as
  `config.key_tables.<name>[index] = { ... }` or
  `config.key_tables.<name>[#config.key_tables.<name> + 1] = { ... }`, plus
  direct indexed field mutations such as
  `config.key_tables.<name>[1].key = 'h'` and
  `config.key_tables.<name>[1].action = act.SendString '...'`, parse into
  native key assignments, with bracket field selectors such as
  `config['key_tables']` and `config[static_name]` supported for nested inserts
  plus indexed or length-append assignments/field mutations where `static_name`
  resolves to `key_tables`. Static
  `config.key_tables = user_key_tables` assignments also merge top-level nested
  inserts such as `table.insert(user_key_tables.resize_pane, { ... })` and
  static field assignments such as `user_key_tables.resize_pane = { ... }`, as
  well as indexed assignments such as `user_key_tables.resize_pane[1] = { ... }`
  or length appends such as
  `user_key_tables.resize_pane[#user_key_tables.resize_pane + 1] = { ... }`,
  plus direct indexed field mutations such as
  `user_key_tables.resize_pane[1].key = 'h'` and
  `user_key_tables.resize_pane[1].action = act.SendString '...'`,
  before the config assignment, and post-assignment top-level nested inserts
  such as `table.insert(user_key_tables.resize_pane, { ... })` plus indexed
  assignments such as `user_key_tables.resize_pane[1] = { ... }` and field
  assignments such as `user_key_tables.resize_pane = { ... }`, plus direct
  indexed field mutations such as `user_key_tables.resize_pane[1].key = 'h'`;
  parsed `leader`, `keys`, and `key_tables` entries are retained in native
  effective config snapshots;
  actual Lua callback execution, arbitrary Lua config evaluation, and default
  key-table merging remain future config parity work.
- Implemented in v1: native `WindowCommand::DisableDefaultAssignment` can be
  used in user key assignments to suppress matching built-in app-shell,
  window-level, and scrollback shortcuts, leaving the key available for the
  later input path. Structured command-palette `disabledefaultassignment`
  queries parse to the same native action payload for command/payload coverage.
- Implemented in v1: native `WindowCommand::SendString` writes the provided
  string bytes directly to the active PTY input path as typed input, without
  bracketed-paste wrapping. Structured command-palette `send string <text>` and
  action-name `sendstring <text>` queries dispatch the same typed payload path,
  and WezTerm-style Lua `SendString { ... }` / `SendString({ ... })` table
  queries tolerate trailing comma fields, with the table-call `string` field
  inline or through a top-level static string variable. Parenthesized
  `SendString(send_opts)` calls also accept top-level static options table
  variables.
- Implemented in v1: native `WindowCommand::SendKey` encodes the specified key
  and modifiers through the active terminal input mode and writes the resulting
  bytes directly to the active PTY input path without re-matching key
  assignments. Structured command-palette `send key <mods+key>` and action-name
  `sendkey <mods+key>` queries dispatch the same typed payload path, and
  WezTerm-style Lua `SendKey { ... }` / `SendKey({ ... })` table queries
  tolerate trailing comma fields, with `key` and `mods` fields inline or
  through top-level static string variables. Parenthesized
  `SendKey(send_key_opts)` calls also accept top-level static options table
  variables.
- Implemented in v1: app-shell `Multiple` sequences already implemented
  `AppAction` values in order.
- Implemented in v1: app-shell exposes a named WezTerm-style
  `SwitchToWorkspace` subset. Existing named workspaces become active without
  duplication, and missing named workspaces are created with the requested spawn
  command and selected. Native `SwitchToWorkspaceArgs` payloads expose that
  spawn command path through `rterm-app`, while existing named workspaces keep
  their current pane launch. Missing workspaces created without an explicit
  spawn command use native `default_prog` for the new pane when configured.
  Structured workspace spawn queries can omit the program and apply supported
  `--domain`/`--cwd`/`--env` options to the default-prog/inherited launch path.
  Omitted-name actions create randomly named workspaces. Native
  `SwitchWorkspaceRelative` payloads switch by arbitrary
  signed offsets using the same sorted workspace order as Next/Previous
  Workspace, with static `config.keys` action calls resolving top-level
  numeric offset variables. The native command palette exposes the same path
  through
  `Switch To Workspace`, `switch workspace <name>`, and action-name
  `switchtoworkspace <name>` queries. Native
  `ShowLauncher`
  opens the default Launcher Menu for local-domain spawning plus native
  launch-menu items, and action-name `showlauncher` queries dispatch that
  default launcher command. Native `ShowLauncherArgs` accepts WezTerm-style
  pipe-separated flags through `show launcher <FLAGS>` and action-name
  `showlauncherargs <FLAGS>` / `showlauncher <FLAGS>` /
  `showlauncherargs flags=<FLAGS>` queries, accepting case-insensitive flag aliases and
  `_`/`-`/compact spellings for multi-word flags. `FUZZY` and,
  with `COMMANDS`, `DOMAINS`,
  `KEY_ASSIGNMENTS`, `LAUNCH_MENU_ITEMS`, `TABS`, and/or `WORKSPACES`, opens a
  launcher-scoped palette for built-in commands, the local domain spawn entry,
  native default plus override key assignment entries, native launch-menu items,
  active-workspace tabs, and existing workspaces. `FUZZY` without item flags
  opens an empty launcher. Native `ShowLauncherArgs` also carries an `alphabet`
  subset through `show launcher <FLAGS> alphabet <chars>` and
  `showlauncherargs <FLAGS> alphabet <chars>` / `showlauncher <FLAGS>
  alphabet <chars>` queries; non-`FUZZY`
  launcher mode treats configured one- or two-key shortcuts as direct entry
  execution, falling back to the native `launcher_alphabet` effective-config
  value when the action omits `alphabet`, and handles `j`/`k` selection
  movement plus `/` fuzzy filtering. Native payloads also carry `help_text`
  for default launcher mode and `fuzzy_help_text` for fuzzy filtering prompts,
  including quote-aware `show launcher <FLAGS> help_text <text>
  fuzzy_help_text <text>` / `showlauncherargs <FLAGS> help_text <text>
  fuzzy_help_text <text>` / `showlauncher <FLAGS> help_text <text>
  fuzzy_help_text <text>` query fields plus `help text`/`fuzzy help text`
  and hyphenated field-key aliases for alphabet/title/help strings in both
  `field <text>` and `field=<text>` forms, and fall back to WezTerm's
  documented single-space default prompt strings when omitted. Structured
  `show launcher <FLAGS>` queries reject unknown top-level fields instead of
  silently discarding them.
  Static WezTerm-style `config.launch_menu` snippets feed native launch-menu
  entries for the implemented `SpawnCommand` subset, including top-level
  launch item `label`, `args` entries, `cwd`, `domain`, and environment values
  supplied inline or through top-level static string variables,
  static `table.insert(config.launch_menu, { ... })` append entries and
  `table.insert(config.launch_menu, index, { ... })` numeric-position inserts,
  with bracket field selectors such as `config['launch_menu']` and static
  item/menu table variables such as `config.launch_menu = { item }`,
  `table.insert(config.launch_menu, item)`,
  `table.insert(config.launch_menu, index, item)`, or post-assignment
  `table.insert(menu, item)` supported. Parsed launch-menu entries are also
  retained in native effective config snapshots.
  Static WezTerm-style `config.keys` actions can also carry table payloads for
  `SwitchToWorkspace`, with `name` plus nested spawn `args` and `cwd` fields
  supplied inline, through top-level static variables, or through
  parenthesized top-level static options table variables, and
  `ShowLauncherArgs`, with `flags`, `title`, `alphabet`, `help_text`, and
  `fuzzy_help_text` supplied inline or through top-level static string
  variables plus parenthesized calls that pass a top-level static args table
  variable. Remote/mux domains, richer default-mode UI styling, broader Lua key
  assignment/config parsing, broader dynamic Lua `launch_menu` construction, Lua
  `PromptInputLine` callback wiring, and Lua event/config wiring remain future
  parity work.
- Completed in v1: app-shell CloseTab handling can select either the default
  left-neighbor tab or the previous active tab, matching WezTerm's
  close-tab selection policy surface.
- Completed in v1: a native WezTerm-style `PromptInputLine` action payload
  carries `description`, optional `prompt`, and optional `initial_value`, opens
  a modal line-input overlay, uses WezTerm's `"> "` default prompt when omitted,
  submits `Some(line)` to a typed native handler on Enter, and submits `None`
  on Escape or `Ctrl+C`. Structured command-palette query fields for
  `prompt input line ...` and action-name `promptinputline ...` `description`,
  `prompt`, and `initial_value` use quote-aware text parsing; `initial_value`,
  `initial value`, and `initial-value` field keys are accepted in both
  `field <text>` and `field=<text>` forms. Static Lua
  `wezterm.format { { Text = ... } }` values for `description` and `prompt`
  are reduced to their visible text for the native overlay. WezTerm-style Lua
  table calls also skip trailing-comma fields and parse `description`, `prompt`,
  and `initial_value` fields inline or through top-level static string
  variables, with `description`/`prompt` also accepting top-level static
  `wezterm.format` text variables and text variables or inline values built
  from top-level static `wezterm.format` aliases, and parenthesized calls
  accepting top-level static options table variables. Static
  `action = wezterm.action_callback(...)` fields, top-level static callback
  variables, and callbacks built from top-level static
  `wezterm.action_callback` aliases are accepted as native-handler
  placeholders. The supported static callback subset maps submitted text through
  documented rename-tab and `SwitchToWorkspace { name = line }` callbacks, plus
  `pane:send_text(line)` through native `SendString` and `pane:send_paste(line)`
  through native `SendPaste` with paste newline and bracketed-paste encoding;
  styled prompt-line rendering and actual Lua callback wiring remain future
  parity work.
- Completed in v1: a native WezTerm-style `InputSelector` action payload carries
  `title`, `choices`, optional `alphabet`, optional `description`, optional
  `fuzzy_description`, and `fuzzy`; it opens a modal selector with default-mode
  alphabet shortcuts plus WezTerm's `1`-`9` direct choice selection, `/` fuzzy
  filtering, `j`/`k` plus arrow/Ctrl movement, Enter or left-click row
  selection, and Escape/`Ctrl+C`/`Ctrl+G` cancellation. The typed native handler
  receives selected `id`/`label` or `None` values on cancel. Structured
  command-palette query fields for `input selector ...` and
  action-name `inputselector ...` title/alphabet/description/fuzzy_description use
  quote-aware parsing; `fuzzy_description`, `fuzzy description`, and
  `fuzzy-description` field keys are accepted, selector fields support both
  `field <text>` and `field=<text>` forms, and `fuzzy=true|false` is accepted
  alongside `fuzzy true|false`. Choice lists split only on unquoted semicolon
  separators, including compact `id=label;id=label` forms, so quoted labels can
  include semicolons. Known fields following `choices` are treated as the earliest
  structured boundary. WezTerm-style Lua table choices with `{ label = ..., id = ... }`
  entries are accepted, and static `wezterm.format { { Text = ... } }` label
  values are reduced to their text for native selector labels. WezTerm-style Lua
  table calls skip trailing-comma fields and parse `title`, string `choices`,
  `alphabet`, `description`, and `fuzzy_description` fields inline or through
  top-level static string variables, table `choices` inline or through
  top-level static table variables whose entries can resolve static string
  labels, top-level static `wezterm.format` label variables, and label
  variables built from top-level static `wezterm.format` aliases, plus
  `fuzzy` inline or through a top-level static bool variable. Parenthesized
  `InputSelector(input_opts)` calls also accept top-level static options table
  variables. Static
  `action = wezterm.action_callback(...)` fields, top-level static callback
  variables, and callbacks built from top-level static
  `wezterm.action_callback` aliases are accepted as native-handler
  placeholders. Static callback bodies that send the selected choice through
  `pane:send_text(id)` / `pane:send_text(label)` map onto the native
  `SendString` path, while `pane:send_paste(id)` / `pane:send_paste(label)`
  map onto the native `SendPaste` path with paste newline and bracketed-paste
  encoding.
  Duplicate `fuzzy` fields are rejected instead of silently overriding them;
  actual Lua `wezterm.action_callback` wiring remains future parity work.
- Completed in v1: a native WezTerm-style `Confirmation` action payload carries
  a message string, required Yes action, and optional No/cancel action. It opens
  a modal confirmation overlay, dispatches a typed native handler with
  `accepted = true` on Enter/`Y`/Space before running the Yes action, and
  dispatches `accepted = false` on Escape/`N`/`Ctrl+C`/`Ctrl+G` before running
  the optional cancel action. Structured command-palette `confirmation message
  ...` queries and action-name `confirmationmessage ...` aliases use
  quote-aware message parsing and accept `message`/`action`/`cancel` fields in
  both `field <text>` and `field=<text>` forms; omitted messages default to
  WezTerm's ` Really continue?` prompt. Static Lua
  `wezterm.format { { Text = ... } }` values for `message` are reduced to
  their visible text for the native overlay, and static `action`/`cancel =
  wezterm.action_callback(...)` fields are accepted as native-handler
  placeholders. WezTerm-style Lua table calls skip trailing-comma fields and
  parse `message` inline or through top-level static string variables or
  top-level static `wezterm.format` text variables, plus `action`/`cancel`
  inline or through top-level static action variables, including variables built
  from top-level static `wezterm.action` aliases. Parenthesized
  `Confirmation(confirm_opts)` calls also accept top-level static options table
  variables. Static callback bodies that call
  `window:perform_action(<implemented action>, pane)`, including the documented
  multi-line `act.SpawnCommandInNewWindow { args = { 'htop' } }` confirmation
  shape, run the nested native command, while `pane:send_text(<static-text>)` and
  `pane:send_paste(<static-text>)` map onto native `SendString` and `SendPaste`
  paths for accept or cancel callbacks. Styled confirmation rendering and
  actual Lua callback wiring remain future parity work.
- Completed in v1: a native WezTerm-style `EmitEvent` action payload carries a
  custom event name and dispatches it through a typed native handler with the
  active window id and pane id. Structured command-palette `emit event <name>`
  and action-name `emitevent <name>` queries use the same quote-aware event-name
  parsing. WezTerm-style Lua `EmitEvent { ... }` / `EmitEvent({ ... })` table
  queries tolerate trailing comma fields, with the table-call `name` field
  inline or through a top-level static string variable. Parenthesized
  `EmitEvent(event_opts)` calls also accept top-level static options table
  variables. Static `wezterm.on('<custom-event>', function(window, pane) ... end)`
  handlers with one or more top-level
  `window:perform_action(<implemented action>, pane)` calls, including
  callback-local static action variables that resolve through top-level
  `wezterm.action` aliases, or parenthesized and single-string
  `pane:send_text(<static-text>)` / `pane:send_paste(<static-text>)` calls,
  with `send_paste` using native paste newline and bracketed-paste encoding,
  run those native commands in order after typed native event dispatch, and
  top-level `wezterm.emit(<static-event-name>, window, pane)` calls, including
  through a top-level static `local <alias> = wezterm.emit` and callback-local or
  top-level static string event-name variables, re-enter matching static
  handlers. Static `return false` stops later static handlers for that event.
  Bounded-static `wezterm.on`/`wezterm.emit` wiring is implemented in v1; arbitrary
  Lua event execution remains future parity work.
- Completed in v1: native WezTerm-style `ActivateKeyTable`, `PopKeyTable`, and
  `ClearKeyTableStack` action payloads maintain a per-window key-table
  activation stack, expose the active table in native window status and the
  typed title-formatting snapshot, and clear the stack when configuration is
  reloaded. Timed activations expire from the stack via
  `timeout_milliseconds`, matching native key-table assignments reset that
  timeout, and one-shot activations pop on the next native key press.
  `prevent_fallback` activations consume unmatched native key presses so they
  do not fall through to default shortcuts or PTY input, while `until_unknown`
  activations pop when an unmatched native key press is seen. Structured
  `activate key table <name>` queries and action-name `activatekeytable <name>`
  aliases reject duplicate `timeout`/`timeout_milliseconds`/
  `timeout-milliseconds`, `one shot`/`one_shot`/`one-shot`, `replace current`/
  `replace_current`/`replace-current`, `until unknown`/`until_unknown`/
  `until-unknown`, and `prevent fallback`/`prevent_fallback`/
  `prevent-fallback` fields instead of silently overriding them, and accept
  `timeout=<ms>` plus single-token boolean assignment forms such as
  `one_shot=false` and `prevent-fallback=true`. WezTerm-style Lua
  `ActivateKeyTable { ... }` / `ActivateKeyTable({ ... })` table queries
  tolerate trailing comma fields, with table-call `name` string,
  `timeout_milliseconds` number, and boolean option fields inline or through
  top-level static variables. Parenthesized `ActivateKeyTable(key_table_opts)`
  calls also accept top-level static options table variables. Action-name
  `popkeytable` and `clearkeytablestack` aliases dispatch the same stack
  mutation payloads as `pop key table` and `clear key table stack`, as do
  WezTerm-style bare and zero-argument Lua action forms such as
  `wezterm.action.PopKeyTable` and `act.ClearKeyTableStack()`, plus empty-table
  wrappers such as `wezterm.action { PopKeyTable = {} }`. Static
  `config.keys` action wrappers also resolve parenthesized top-level static
  wrapper table variables such as `local wrapper = { SendString = "text" }`
  followed by `action = act(wrapper)`.
  Native `key_tables` overrides now match table entries from the activation
  stack top downward and execute the matched native action. Lua `key_tables`
  parsing remains future parity work.
- Completed in v1: split separators can be dragged with the mouse to update the
  same app-shell resize deltas used by keyboard and command-palette pane resize
  actions.
- Completed in v1: WezTerm-style `AdjustPaneSize` actions are represented in
  app-shell state and routed from native-window keyboard shortcuts and command
  palette Adjust Pane Size entries. Native
  `WindowCommand::AdjustPaneSize { direction, amount }` payloads dispatch the
  same active-pane resize path with arbitrary cell amounts, and structured
  command-palette queries accept both `adjust pane size <direction> <amount>`
  and `adjustpanesize <direction> <amount>`, plus field forms such as
  `adjustpanesize direction=<direction> amount=<cells>`. WezTerm-style
  `AdjustPaneSize { '<direction>', <cells> }` Lua table actions and
  `wezterm.action { AdjustPaneSize = { '<direction>', <cells> } }` wrappers
  dispatch the same payload, including trailing comma fields. Static
  `config.keys` action tables resolve top-level string variables for direction
  and top-level integer variables for amount, plus parenthesized top-level
  static resize table variables.
- Completed in v1: pane zoom state is represented in app-shell state and
  rendered by the native window as a full-tab pane until toggled off through
  WezTerm-style `TogglePaneZoomState`, explicitly zoomed/unzoomed through
  WezTerm-style `SetPaneZoomState`, native
  `WindowCommand::SetPaneZoomState(bool)` payloads, or unzoomed before
  directional pane switching when `unzoom_on_switch_pane` is true. The default
  `Ctrl+Shift+Z` key-assignment entry exposes `TogglePaneZoomState`, while
  `TogglePaneZoom` remains a native compatibility alias. Action-name
  `togglepanezoomstate`, `togglepanezoom`, `zoompane`, and `unzoompane`
  queries dispatch the corresponding no-argument zoom commands. Static
  `config.keys` action calls resolve top-level bool variables for
  `SetPaneZoomState`.
- Completed in v1: WezTerm-style `PaneSelect` default Activate mode renders
  labels over pane regions from the command palette entry `Pane Select`, accepts
  action-name `enterpaneselect` queries, accepts label input to focus a pane,
  honors the native effective
  `quick_select_alphabet` value for label generation, accepts quote-aware
  command-palette queries `pane select alphabet <chars>` and
  `pane select activate alphabet <chars>` plus action-name `paneselect ...`
  aliases for the native Activate plus per-action alphabet subset, and supports
  `Esc`/`Ctrl+g` cancellation. Pane labels honor WezTerm's
  `pane_select_bg_color`/`pane_select_fg_color` defaults plus configured RGBA
  alpha, and `pane_select_font_size` is retained in effective config.
- Completed in v1: `Pane Select Show Pane IDs` exposes a native
  `show_pane_ids=true` subset by rendering pane-select labels as
  `label:pane_id` while retaining default Activate behavior, and the
  action-name `enterpaneselectshowpaneids` dispatches that default
  show-pane-ids entry. The
  quote-aware command-palette queries `pane select show pane ids alphabet
  <chars>`/`show-pane-ids alphabet <chars>` and `pane select activate show pane
  ids alphabet <chars>`/`show-pane-ids alphabet <chars>` combine Activate,
  `show_pane_ids=true`, and a per-action alphabet, with `alphabet=<chars>`
  assignment forms accepted for the same alphabet field. The implemented non-default
  mode queries (`swap`, `swap keep focus`, `move to new tab`, and `move to new
  window`) can include `show pane ids`, `show_pane_ids`, or `show-pane-ids`, and
  may append quote-aware `alphabet <chars>` to combine mode, `show_pane_ids=true`,
  and a per-action alphabet. The structured command-palette query `pane select
  mode <mode> [show_pane_ids true|false] [show_pane_ids=true|false] [alphabet
  <chars>|alphabet=<chars>]` / `mode=<mode>` and action-name `paneselect ...` aliases map
  WezTerm-style option names to the native `PaneSelect { mode, show_pane_ids,
  alphabet }` payload and reject duplicate structured fields. WezTerm-style
  `wezterm.action.PaneSelect { mode = ..., show_pane_ids = ..., alphabet = ... }`
  and parenthesized table-call queries dispatch the same native field subset,
  including long-bracket table keys and trailing-comma table fields, and parse
  `mode`/`alphabet` through top-level static string variables plus
  `show_pane_ids` through top-level static bool variables, with parenthesized
  calls also accepting top-level static options table variables inside static
  WezTerm-style `config.keys`; arbitrary dynamic Lua construction remains
  future parity work.
- Completed in v1: WezTerm-style `ActivatePaneDirection` routes
  `Ctrl+Shift+Arrow` and command-palette Activate Pane Direction Left/Right/
  Up/Down/Next/Previous entries to directional pane focus changes. Native
  `WindowCommand::ActivatePaneDirection(direction)` payloads dispatch through
  the same path, and structured command-palette queries accept both
  `activate pane direction <direction>` and
  `activatepanedirection <direction>`; action-name `activatepaneleft`,
  `activatepaneright`, `activatepaneup`, `activatepanedown`, `nextpane`, and
  `previouspane` queries dispatch the corresponding no-argument entries. Static
  `config.keys` action calls resolve top-level string variables for
  `ActivatePaneDirection`.
- Completed in v1: app-shell state exposes WezTerm-style `ActivatePaneByIndex`,
  with command-palette Activate Pane By Index entries for pane indices 1
  through 8, `activate pane <index>`/`activate pane by index <index>` queries,
  `activatepanebyindex <index>` queries, action-name `activatepane1` through
  `activatepane8` entries, and native `WindowCommand::ActivatePaneByIndex(index)`
  payloads for arbitrary zero-based pane indices. Static `config.keys` action
  calls resolve top-level unsigned integer variables for `ActivatePaneByIndex`.
- Completed in v1: app-shell state exposes WezTerm-style `RotatePanes` for
  clockwise/counter-clockwise pane identity rotation while preserving split
  positions and size deltas, with command-palette entries for both directions
  and quoted or unquoted `rotate panes <direction>` /
  `rotatepanes <direction>` queries mapping to native
  `WindowCommand::RotatePanes(direction)` payloads. Static `config.keys` action
  calls resolve top-level string variables for `RotatePanes`.
- Completed in v1: pane-select swap mode entries `Pane Select Swap With Active` and
  `Pane Select Swap With Active Keep Focus` exchange active/selected pane layout
  positions and support both selected-pane focus and keep-active-focus behavior.
  Action-name `enterpaneswap` and `enterpaneswapkeepfocus` queries dispatch
  the corresponding default mode entries.
- Completed in v1: pane-select MoveToNewTab mode moves the selected pane into a
  new tab in the same workspace and activates that tab. Action-name
  `enterpanemovetonewtab` queries dispatch that default mode entry.
- Completed in v1: pane-select MoveToNewWindow mode removes the selected pane
  from the current split layout and records a pending native-window request with
  its own tab and active pane. Action-name `enterpanemovetonewwindow` queries
  dispatch that default mode entry.
- Completed in v1: pending MoveToNewWindow requests can be consumed into an
  independent app-shell/native-window app state while transferring the detached
  pane runtime snapshot.
- Completed in v1: the native window entry point now runs through a
  multi-window manager that materializes detached MoveToNewWindow app states as
  additional OS windows.
- Completed in v1: WezTerm-style `SpawnWindow` creates a pending native-window
  app with a fresh default-launch tab and pane. The default `Ctrl+Shift+N`
  and `Super+N` shortcuts plus command-palette `Spawn Window` entry route
  through this path, action-name `spawnwindow` queries dispatch the same
  command, and the multi-window manager materializes spawned windows as
  additional OS windows.
- Completed in v1: the command palette includes native
  `SpawnCommandInNewTab`/`SpawnCommandInNewWindow` query subsets. `new tab
  <program> [args...]` creates and activates a tab with an explicit
  `PaneLaunch`, while `spawn window <program> [args...]` records a pending
  native window with that explicit launch. The WezTerm action-name aliases
  `spawncommandinnewtab <program> [args...]` and
  `spawncommandinnewwindow <program> [args...]` route through the same parser.
  `new tab --domain current-pane-domain --cwd <path> --env NAME=VALUE`,
  `new tab --set-environment-variables NAME=VALUE`, and
  `spawn window --cwd <path> --env NAME=VALUE --position <position>` also work
  without an explicit program by applying those options to the existing
  default-prog/inherited launch path. Split query subsets also route
  explicit launches through the same app-shell split action: `split horizontal
  <program> [args...]` / `split right <program> [args...]` create right-side
  splits, and `split vertical <program> [args...]` / `split down <program>
  [args...]` create downward splits. The WezTerm action-name forms
  `splitpane <right|down|left|up> ...` and
  `splitpane direction <right|down|left|up> ...` build the same native payload.
  Split queries accept `--percent N`/`--percent=N`, `--cells N`/`--cells=N`,
  `--top-level`/`--top-level=true|false`, and supported
  `--domain`/`--cwd`/`--env`/`--set-environment-variables`/
  `--set_environment_variables` options in any order before the optional launch
  command. They can also omit the program when those supported spawn options
  are present, applying those fields to the existing default-prog/inherited
  launch path. Native
  `SplitPane` action payloads support
  Left/Right/Up/Down directions, the local-domain subset `CurrentPaneDomain`,
  `DefaultDomain`, and `DomainName("local")`, optional Percent/Cells sizing,
  and `top_level=true` root-region splits that compress the existing tab layout
  into the source side. The default `Ctrl+Shift+Alt+\"` and
  `Ctrl+Shift+Alt+%` key-assignment entries now expose WezTerm-style
  `SplitVertical={domain="CurrentPaneDomain"}` and
  `SplitHorizontal={domain="CurrentPaneDomain"}` payloads while the command
  palette keeps the shorter `SplitVertical`/`SplitHorizontal` aliases, with
  action-name `splitvertical` and `splithorizontal` queries dispatching those
  default split directions. Static WezTerm-style `config.keys` actions can
  carry `SpawnCommandInNewTab`/`SpawnCommandInNewWindow` table payloads with
  `args`, `cwd`, `domain`, `set_environment_variables`, and window `position`
  fields supplied inline or through top-level static variables, plus
  parenthesized static options table variables for those spawn-command
  payloads, including option-only tables that omit `args` and apply supported
  options to the default-program/inherited launch path.
  `SplitPane`/`SplitHorizontal`/`SplitVertical` table payloads carry
  `direction`, `domain`, nested `command` spawn fields, Percent/Cells `size`,
  `top_level`, and parenthesized static options table variables supplied inline
  or through top-level static variables.
  Broader dynamic Lua table evaluation remains future config parity work.
- Completed in v1: app-shell keyboard routing includes WezTerm-style `Super`
  aliases for native tab actions: `Super+T` `SpawnTab(CurrentPaneDomain)`
  new tab, `Super+Shift+T` `SpawnTab(DefaultDomain)` with configured
  `default_domain` validation,
  `Super+W` `CloseCurrentTab(confirm=true)` close-tab confirmation,
  `Super+1..9` indexed tab activation, and
  `Super+Shift+[` / `Super+Shift+]` relative tab activation. Native `SpawnTab`
  action payloads cover the local-domain subset by mapping `CurrentPaneDomain`,
  `DefaultDomain`, and `DomainName("local")` to the native `NewTab` launch path
  when they resolve to local. Structured command-palette `spawn tab current
  pane domain`, `spawn tab default domain`, and `spawn tab domain <name>`
  queries plus action-name `spawntab ...` aliases dispatch the same native
  payload subset with quote-aware domain-name parsing; no-argument
  `spawntab` dispatches the current-pane-domain default. Static `config.keys`
  `SpawnTab` action calls resolve top-level string variables for string
  arguments, `DomainName` table fields, and parenthesized top-level static
  domain table variables. Remote/mux named
  domain spawning remains future mux/domain parity work.
- Completed in v1: WezTerm-style `AttachDomain` and `DetachDomain` action
  parsing recognizes string, function-call, and `DomainName` table forms,
  including static `config.keys` top-level string variables, `AttachDomain` and
  `DetachDomain` table fields for `DomainName`, and `DetachDomain` parenthesized
  static domain `DomainId` table variables. `AttachDomain` now dispatches to local
  tab creation for `local`, `CurrentPaneDomain`, and local default-domain cases
  (for example `DefaultDomain` when the configured default is local); non-local
  `AttachDomain` values still return unsupported-action.
- Completed in v1: WezTerm-style `ToggleFullScreen` routes the default
  `Alt+Enter` shortcut and command-palette `Toggle Full Screen` entry through
  the native window fullscreen state, then dispatches the typed resize hook
  with fullscreen dimension metadata. Action-name `togglefullscreen` queries
  dispatch the same command. Arbitrary Lua callback execution beyond this bounded
  static surface remains future parity work.
- Completed in v1: WezTerm-style `StartWindowDrag` routes command-palette
  dispatch plus the default `SUPER` + left drag and `CTRL|SHIFT` + left drag
  bindings to the native drag-to-move request path, calling the platform
  window backend when a native window exists. Action-name `startwindowdrag`
  queries dispatch the same command. Static WezTerm-style
  `config.mouse_bindings` now parses the native `Down`/`Up`/`Drag` plus
  `Left`/`Middle`/`Right` buttons with non-zero streak values plus vertical
  `WheelUp`/`WheelDown` `streak = 1` with `mods`, `mouse_reporting`,
  `alt_screen`, implemented native `action` payloads, top-level static `mods`
  string, `mouse_reporting` bool, and `alt_screen` bool/string variables, and
  top-level static `table.insert(config.mouse_bindings, { ... })` appends plus static item
  variables such as `config.mouse_bindings = { binding }` with pre-use field
  mutations, and parsed mouse bindings are retained in native effective config
  snapshots. Mouse binding `event` payloads parse inline or through top-level
  static event table variables, with nested `Down`/`Up`/`Drag` payloads inline
  or through top-level static table variables, `WheelUp`/`WheelDown` button
  tables inline or through top-level static table variables with amount fields
  inline or through top-level static number variables, and `button`/`streak`
  fields inline or through top-level static variables. `action` payloads parse
  inline or through top-level static action variables, so bindings such as
  `ALT` + left drag can route to `StartWindowDrag`, middle-button release can
  route to `PastePrimarySelection`, `CTRL` + wheel-up can route to
  `IncreaseFontSize`, double-left-down can route to a custom action, and
  non-left button streaks are tracked for user mouse bindings. Matching user
  mouse bindings suppress the implemented default mouse assignment for the same
  button, streak, modifiers, mouse-reporting state, and alternate-screen state;
  `DisableDefaultAssignment` mouse bindings participate in that suppression
  without consuming the event, matching WezTerm's opt-out semantics rather than
  `Nop`. Default mouse assignments are skipped while the pane has captured mouse
  reporting unless the configured bypass modifier is held; drag bindings
  classify that bypassed path as `mouse_reporting = false`, and bypassed wheel
  input routes through native scroll handling instead of SGR mouse reporting.
  Wheel bindings can route `ScrollByCurrentEventWheelDelta` using the current
  vertical wheel delta; Lua `window:current_event()` object exposure remains future parity work. Native
  `disable_default_mouse_bindings` defaults to false and suppresses the
  implemented default mouse-assignment subset when true, including the
  built-in wheel scroll/alternate-screen arrow fallback when no user wheel
  binding matched.
- Completed in v1: native `hide_mouse_cursor_when_typing` defaults to true,
  hides the OS mouse cursor on key press while the cursor is inside the
  native window, and restores it on mouse motion or cursor leave.
- Completed in v1: WezTerm-style `ActivateWindow`, `ActivateWindowRelative`,
  and `ActivateWindowRelativeNoWrap` action payloads route through a native
  manager focus request. Materialized OS windows are ordered by app window id;
  `ActivateWindow` uses zero-based absolute indexes, the wrapping relative
  variant cycles through that order, and the no-wrap variant stops at the
  first/last window. Structured command-palette queries accept
  `activate window <index>`, `activate window index <index>`,
  `activatewindow <index>`, `activate window relative <offset>`,
  `activatewindowrelative <offset>`,
  `activate window relative no wrap <offset>`, and
  `activatewindowrelativenowrap <offset>` for those same payloads. Static
  `config.keys` action calls resolve top-level numeric variables for the
  absolute index and relative offsets.
- Completed in v1: native WezTerm-style `SetWindowLevel` action payloads accept
  `AlwaysOnBottom`, `Normal`, and `AlwaysOnTop`, updating the app's remembered
  window level and applying it to the platform window through winit's
  `WindowLevel` API when backend support exists. Structured command-palette
  `set window level <value>` and `setwindowlevel <value>` queries use
  quote-aware value parsing. Static `config.keys` action calls resolve
  top-level string variables for `SetWindowLevel`.
- Completed in v1: native WezTerm-style `ToggleAlwaysOnTop` and
  `ToggleAlwaysOnBottom` action payloads and command-palette entries toggle the
  remembered window level between the requested z-order and `Normal`.
  Action-name `togglealwaysontop` and `togglealwaysonbottom` queries dispatch
  the corresponding commands.
- Completed in v1: WezTerm-style `Show` routes command-palette and native
  action payload dispatch to native show/unminimize/focus behavior for the
  current window, clearing a prior hide request. Action-name `show` queries
  dispatch the same command.
- Completed in v1: WezTerm-style `Hide` routes the default `Super+M` shortcut
  and command-palette `Hide` entry to native hide/minimize state, minimizing the
  platform window when available. Action-name `hide` queries dispatch the same
  command.
- Completed in v1: WezTerm-style `HideApplication` routes the macOS-default
  `Super+H` shortcut, command-palette `Hide Application` entry, and
  action-name `hideapplication` query to a manager-consumed, single-use
  application-hide request. On macOS the manager calls the locked winit native
  application-hide API; `cfg(not(target_os = "macos"))` builds use a platform
  no-op. This action neither minimizes the current window nor synthesizes focus;
  subsequent OS focus events remain authoritative. The default
  `KEY_ASSIGNMENTS` list includes `Super+H` only on macOS, matching WezTerm's
  platform-specific default.
- Completed in v1: WezTerm-style `QuitApplication` is exposed through the
  command-palette `Quit Application` entry and action-name `quitapplication`
  query. It requests whole-application shutdown, drops pending native-window
  apps, and preserves final metrics.
- Completed in v1: WezTerm-style `DecreaseFontSize`, `IncreaseFontSize`,
  `ResetFontSize`, and command-palette `ResetFontAndWindowSize` route font-size
  actions into native-window logical font-size scale updates. Action-name
  `decreasefontsize`, `increasefontsize`, `resetfontsize`, and
  `resetfontandwindowsize` queries dispatch the same commands. Native `font_size`
  defaults to WezTerm's `12.0` points and scales the fixed native base cell
  metrics. Native `cell_width` defaults to WezTerm's `1.0` ratio and further
  scales horizontal cell geometry, while native `line_height` defaults to
  WezTerm's `1.0` ratio and further scales vertical cell geometry used for
  rendering, hit testing, terminal size calculation, and frame sizing; shortcut
  zoom remains an additional scale over that configured baseline. Native
  `adjust_window_size_when_changing_font_size` defaults to the non-tiling
  WezTerm effective behavior of true, preserving terminal rows/columns by
  resizing the native frame and requesting the matching OS-window inner size
  when a native window exists; setting it false keeps the current window size
  and recomputes terminal rows/columns from the scaled cell size. Reset Font
  And Window Size also restores the native frame to the configured initial rows
  and columns. Native config overrides expose `font_size`, `cell_width`,
  `cell_widths`,
  `line_height`, deprecated WezTerm-compatible `font_antialias`/`font_hinting`,
  `font_rasterizer`, `font_colr_rasterizer`, `font_shaper`,
  `font_dirs`, `font_locator`, `use_cap_height_to_scale_fallback_fonts`,
  `ignore_svg_fonts`, `sort_fallback_fonts_by_coverage`,
  `search_font_dirs_for_fallback`, `custom_block_glyphs`,
  `anti_alias_custom_block_glyphs`, `allow_square_glyphs_to_overflow_width`,
  `freetype_load_target`, `freetype_render_target`, `freetype_load_flags`,
  `freetype_interpreter_version`, `freetype_pcf_long_family_names`,
  `display_pixel_geometry`, `dpi`, `dpi_by_screen`, `initial_cols`, `initial_rows`, and
  `adjust_window_size_when_changing_font_size`; `freetype_render_target`
  defaults to the effective load target when unset, while `dpi` overrides the
  detected window DPI for renderer state and FreeType defaults until cleared,
  `dpi_by_screen` retains static screen-name DPI maps for future per-display
  matching,
  `freetype_load_flags` defaults to `DEFAULT` below 100 DPI and
  `NO_HINTING` at 100 DPI or higher, and static Lua `font_dirs` tables parse
  inline, through top-level static table variables, or through
  `table.insert(config.font_dirs, ...)` appends.
  `config.font` accepts static `wezterm.font` and `wezterm.font_with_fallback`
  calls, including top-level static helper aliases and static font value
  variables, retaining the primary family, fallback families, and supported
  font attributes in effective config.
  Direct or `table.insert`-appended `config.font_rules[*]` entries use the same
  static variable expansion for font values and matcher fields so rule-specific
  attributes are retained.
  `window_frame.font` also accepts top-level static `wezterm.font` helper
  aliases, static `wezterm.font(...)` value variables, and
  `wezterm.font { family = ... }` static family variables for retained native
  titlebar font settings.
  Custom block glyph,
  square-glyph overflow, COLR font rasterizer, SVG-font ignore, fallback-font
  coverage sorting and font-directory fallback-search, FreeType
  interpreter-version, PCF long-family-name, display pixel-geometry,
  font-directory, font-locator, font shaper, and fallback-font cap-height
  scaling options are retained in
  effective config with WezTerm defaults, but actual renderer glyph strategy,
  configured font-directory scanning, font-locator application, COLR rasterizer
  selection, SVG font filtering, fallback-font coverage sorting,
  font-directory fallback search, shaping-engine application, fallback-font cap-height scaling, FreeType interpreter
  application, subpixel geometry application, and PCF font-resolution changes
  remain future parity work.
- Completed in v1: WezTerm-style `ShowDebugOverlay` routes the default
  `Ctrl+Shift+L` shortcut, command-palette `Show Debug Overlay` entry, and
  action-name `showdebugoverlay` query into native-window debug-overlay state
  and renders a visible native diagnostic overlay with current
  window/tab/pane/workspace and runtime state plus recent native diagnostic log
  lines from key-event, unknown-escape, and missing-glyph warnings. Bare `Esc`
  closes the overlay without forwarding input to the PTY; Lua REPL support and
  full external log-source integration remain future parity work.
- Completed in v1: WezTerm-style `CharSelect` routes the default
  `Ctrl+Shift+U` shortcut and command-palette `Char Select` entry into native
  character-selection mode, closing other active overlays. Native
  `CharSelectArgs` payloads carry `copy_on_select`, `copy_to`, and `group`
  into the overlay state, and structured command-palette `char select` plus
  WezTerm-style action-name `charselect` default and argument queries use the same
  quote-aware parsing as other palette command options plus `field=value`
  assignment forms including `copy-on-select=false`, `copy-to=<destination>` /
  `copy-to="primary selection"`, and `group=<name>` /
  `group="<name with spaces>"`, so quoted group values with spaces do not retain
  their quotes. The native query subset rejects duplicate
  `copy_on_select`, `copy_to`, and `group` fields instead of silently overriding
  them. WezTerm-style Lua `CharSelect { ... }` / `CharSelect({ ... })` table
  queries tolerate trailing comma fields and parse `copy_to`/`group` through
  top-level static string variables plus `copy_on_select` through top-level
  static bool variables, with parenthesized calls also accepting top-level
  static options table variables inside static WezTerm-style `config.keys`. The modal handles `Esc` /
  `Ctrl+G` cancellation
  plus typed text input, Backspace editing, `Ctrl+U` input clearing,
  `Ctrl+R` / `Ctrl+Shift+R` group cycling, and Enter acceptance for raw, `U+`,
  and `0x` hex Unicode codepoint input without forwarding those keys to the PTY. Accepted
  codepoints insert into the active pane and honor `copy_on_select` /
  `copy_to` for Clipboard, PrimarySelection, or both configured copy targets.
  Standard Unicode character-name input such as `grinning face`, plus fuzzy
  token queries such as `grin face`, resolve through the same Enter acceptance
  path. Window title/status text shows `Char Select`, includes the requested
  group when present, surfaces the current text input, renders a visible
  candidate overlay for name/codepoint matches, RecentlyUsed entries, and
  initial built-in category candidates including NerdFonts private-use glyphs;
  normal rows honor WezTerm's `char_select_bg_color`/`char_select_fg_color`
  defaults plus configured RGBA alpha, and `char_select_font_size` uses
  WezTerm's current 18pt default in effective config;
  typed fuzzy queries and hex codepoint input also match the built-in NerdFonts
  names. ArrowUp/ArrowDown moves the selected candidate before Enter acceptance
  while scrolling the overlay past the first visible rows.
  RecentlyUsed candidates use persisted JSON selection counts plus a last-used
  sequence across app instances. Rendering the full categorized picker/database
  plus exact WezTerm frecency scoring remains future parity work.
- Completed in v1: PTY reader events carry the app-shell `WindowId` plus
  `PaneId`, so independent windows can route events without relying on globally
  unique pane IDs. PTY EOF handling now waits for the process status and honors
  native `exit_behavior` overrides for `Close`, `Hold`, and
  `CloseOnCleanExit`, including configured `clean_exit_codes` for non-zero
  statuses that should count as clean. Native `exit_behavior_messaging` controls
  held-pane status text verbosity, with `None` suppressing the message and
  verbose text reporting the actual `exit_behavior` value that kept the pane
  open. Static Lua config parsing covers `exit_behavior`,
  `exit_behavior_messaging`, and `clean_exit_codes` inline, through top-level
  static table variables with pre/post-assignment `table.insert` appends, or
  through static field names in returned config table initializers and direct
  return-table configs, `table.insert(config.clean_exit_codes, ...)` appends,
  or `table.insert(config[static_name], ...)` and
  `config[static_name][#config[static_name] + 1] = ...` appends where
  `static_name` resolves to `clean_exit_codes`;
  exact message text parity remains future work.
- Completed in v1: ClosePane follows WezTerm-style lifecycle cascading by
  closing a single-pane tab when another tab exists, while final tab/pane close
  actions request native-window shutdown from the window manager. The manager
  honors WezTerm's `quit_when_all_windows_are_closed=true` default and keeps
  the process alive after the last window closes when the native override is
  false. Native
  WezTerm-style `CloseCurrentPane { confirm = false }` and
  `CloseCurrentTab { confirm = false }` payloads route through the same
  immediate-close path; action-name `closepane` and `closetab` queries dispatch
  the no-argument immediate-close aliases. `confirm = true` opens a native
  confirmation overlay that accepts Enter/Y and cancels with Esc/N/Ctrl-C/Ctrl-G
  before dispatching the captured pane/tab close action. Structured
  command-palette queries accept
  both `close current pane confirm true|false` /
  `close current pane confirm=true|false` /
  `close current tab confirm true|false` /
  `close current tab confirm=true|false` and the WezTerm-style
  `closecurrentpane confirm true|false` /
  `closecurrentpane confirm=true|false` /
  `closecurrenttab confirm true|false` /
  `closecurrenttab confirm=true|false` action-name spelling. WezTerm-style Lua
  `CloseCurrentPane { ... }` / `CloseCurrentPane({ ... })` and
  `CloseCurrentTab { ... }` / `CloseCurrentTab({ ... })` table queries tolerate
  trailing comma fields, and static WezTerm-style `config.keys` actions resolve
  top-level static bool variables for the `confirm` field and parenthesized
  static options table variables.
- Completed after v1: native multi-window focus is coordinated by the window
  manager so at most one managed window owns focus. Apps start unfocused until
  the OS reports focus, and duplicate or out-of-order focus events suppress
  repeated Lua/native handlers, status refreshes, and terminal focus-reporting
  bytes. Removing a window clears stale manager ownership. Startup and, after an
  entire pending batch materializes successfully, only its final window request
  show/unminimize/focus; `ActivateWindow*` reuses that same request path.
  Observable focus still changes only when the OS confirms it. `HideApplication`
  is consumed once by the manager and dispatches winit's native application hide
  on macOS without synthesizing focus changes. `rterm-app` app/unit tests on the
  Windows host/build verify the logical lifecycle; neither Windows nor macOS has
  a real native multi-window focus-event end-to-end run in this slice. The macOS
  call was checked against the locked winit API type and signature, while the
  cross-target build is blocked by a third-party C-toolchain dependency.
- Completed after v1: every pane owns at most one current transient overlay
  slot, either `CopySearch` or `QuickSelect`. Search and Copy Mode are modes of
  the shared `CopySearch` controller; Quick Select replaces that slot and exit
  does not implicitly restore the replaced controller. Pane, tab, and workspace
  focus changes save and restore the owner state within one native window, and
  same-window `MovePaneToNewTab` preserves it. Input, titles, copy, and
  selection actions route only through the active pane, while every visible
  pane renders its own selection-free base snapshot plus owner-local overlay.
  Active and inactive terminal mutation paths reconcile stable coordinates;
  Quick matches and labels prune deterministically without retargeting.
  Moving a pane across a native-window boundary preserves terminal runtime,
  scrollback, and viewport but clears GUI selection and overlay state.
  Immediate pane/tab close cleanup removes only the closed owner state as an
  intentional R-Term robustness enhancement over pinned WezTerm retention.
- Completed after v1: bounded width-resize parity is verified against pinned
  WezTerm `093bf6bf2b82b929ed80c04fd54ebc80464f715e`. Main scrollback and the
  main screen, including a dormant main screen, reflow by logical lines;
  alternate-screen resize remains physical and non-reflowing. Unsafe
  main-coordinate metadata is retired. With main visible, its viewport
  resets/clamps, ordinary selection is cleared, and Copy/Search/Quick retain
  configuration while rebuilding coordinate-derived presentation. With
  alternate visible, its UI state is preserved while saved main reflows in the
  background and the alternate is only physically resized. This scope makes no
  general selection, rendering, or image parity claim beyond the tested
  metadata retirement.
- Cell-level horizontal bounded-margin scrolling is complete: `SU`/`SD`, line
   editing/control scrolling, bounded character edits, and in-margin
   right-edge writing preserve exterior cells. Declared-cell graphics use
   persistent `CellAttachment` state, so the same bounded transforms move or
   delete only their covered cells and may split a graphic at an LR boundary;
   malformed or footprint-less placements retain conservative retirement.
   This bounded behavior is not a claim of general graphics, renderer, or
   protocol parity.
   Vertical wheel routing over an inactive visible pane is complete within its
   bounded scope: it uses hovered-pane cell and pixel coordinates, runtime, UI,
   writer, and target-aware bindings without implicit focus transfer. Tab-bar
   wheel switching, the active-pane window-right scrollbar, click/move focus,
   and explicit focus or pane-creation commands remain distinct. Subsequent
   backlog includes general non-wheel mouse routing, richer pane focus visuals
   and split-drag affordances, arbitrary Lua callbacks including full
   arbitrary-Lua/custom tab-formatting parity, external CLI title control, font
   shaping parity, and a real mux/window registry with domain, protocol, and
   renderer parity. The bounded-static `format-tab-title` surface is already
   implemented. The pinned WezTerm contract uses one
  window-right scrollbar bound to the active pane, which is already the
  implemented layout; per-pane scrollbar tracks are not a parity requirement.
  The completed stable-row,
  per-row-`seqno`, ordinary-selection selected/visible-changed-row intersection
  rule, overlay accumulated-row-change evaluation, and stable Lua coordinate
  work does not imply full selection parity, full App Shell v2, or general
  WezTerm parity.
- Implemented in v1: minimal `Ctrl+Shift+P` command palette dispatch for
  tab/pane/window/workspace actions including `Spawn Window`,
  `Toggle Full Screen`, Split Horizontal, Split Vertical, Close Current Tab,
  and Close Current Pane, plus WezTerm-style `ActivateCommandPalette` as a
  discoverable command that reopens the palette after command execution.
  Action-name `activatecommandpalette` queries dispatch the same command.
  Native default key-assignment entries include the implemented WezTerm
  defaults for tab navigation/movement, split creation, pane focus, and pane
  resize so `ShowLauncherArgs { flags = KEY_ASSIGNMENTS }` can surface those
  bindings alongside user overrides. Native `disable_default_key_bindings`
  defaults to false and suppresses the implemented built-in default key
  assignments when true.
  Native `new tab <program> [args...]`, `spawn window <program> [args...]`,
  `split horizontal <program> [args...]`, and `split vertical <program>
  [args...]` query subsets route explicit launch commands through the same
  app-shell actions. The native command palette renders a visible candidate
  overlay whose row count honors `command_palette_rows`, falling back to a
  terminal-height-based default when unset, and normal rows honor WezTerm's
  `command_palette_bg_color`/`command_palette_fg_color` defaults plus
  configured RGBA alpha. Executed command labels update command-palette frecency
  in memory and persist it to a JSON state file so later app instances can
  promote frequently and recently used entries.
- Implemented in v1 against pinned WezTerm
  `093bf6bf2b82b929ed80c04fd54ebc80464f715e`: native window startup has a
  file-backed WezTerm configuration lifecycle for the strict production
  grammar. Source discovery follows explicit `--config-file`,
  `WEZTERM_CONFIG_FILE`, Windows portable executable-adjacent config,
  `$HOME/.wezterm.lua`, `$XDG_CONFIG_HOME/wezterm/wezterm.lua` or, when
  `XDG_CONFIG_HOME` is unset, `$HOME/.config/wezterm/wezterm.lua`, then Unix
  `$XDG_CONFIG_DIRS` candidates;
  `--skip-config` disables discovery, repeated
  `--config name=value` values apply last and above the file, and invalid CLI
  overrides fail before app construction. Every startup, manual reload, and
  automatic reload attempt rediscovers the source. Successful startup installs
  generation 1 before the first PTY spawn; a file read/parse/validation failure
  starts generation 0 defaults with a diagnostic so the attempted source can
  recover later. Derived `WEZTERM_CONFIG_FILE`/`WEZTERM_CONFIG_DIR` values are
  published only to child launch environments without mutating process-global
  environment state.
  The strict production grammar is intentionally bounded to a direct static
  return table or the supported `wezterm.config_builder()` plus direct static
  assignments and a final return. Unsupported statements, dynamic expressions,
  and registry-excluded top-level or nested fields fail with explicit source
  diagnostics; this does not provide an arbitrary Lua VM, general `require`
  evaluation, dynamic helper execution, or config dependency/helper-file
  watching.
- Implemented in v1: command-palette `ReloadConfiguration`, action-name
  `reloadconfiguration`, and the default `Ctrl+Shift+R` shortcut run one
  manager-owned reload transaction across startup, pending, detached, and
  materialized windows. On success the generation advances exactly once, all
  apps consume the new base before any `window-config-reloaded` callback, and
  per-window overrides remain the highest-precedence layer. On failure the
  last-known-good generation and effective state remain installed, while every
  app is still notified exactly once and the latest diagnostic is retained.
  Future pending/detached window materialization inherits the current base
  generation and its window layer; existing PTYs are not respawned.
- Implemented in v1: when the manager-owned base-plus-CLI effective
  `automatically_reload_config` policy is enabled, one native watcher tracks
  each attempted source and eligible parent and coalesces relevant
  create/modify/remove events using a fixed 200 ms first-event debounce before
  invoking the same manager reload transaction. Failed attempted sources seed
  the watcher; successful rediscovery accumulates normalized file and parent
  paths so atomic replacement and source movement remain observable.
  The home config file is watched without registering the noisy home parent.
  Once created, the watcher and accumulated paths remain active if a later
  generation disables automatic reload, but that disabled generation adds no
  new paths. Per-window overrides never control this global policy.
  A typed native `set_config_overrides`/`get_config_overrides` subset stores
  per-window overrides for implemented effective-config fields (`dpi`, `tab_max_width`,
  `status_update_interval`, `max_fps`, `animation_fps`, `front_end`,
  `webgpu_power_preference`, `webgpu_force_fallback_adapter`,
  `webgpu_preferred_adapter`, `prefer_egl`, `enable_wayland`,
  `enable_zwlr_output_manager`, `use_box_model_render`,
  `experimental_pixel_positioning`, `shape_cache_size`,
  `line_state_cache_size`, `line_quad_cache_size`,
  `line_to_ele_shape_cache_size`, `glyph_cache_image_cache_size`,
  `cursor_blink_rate`, `cursor_blink_ease_in`,
  `cursor_blink_ease_out`, `text_blink_rate`, `text_blink_rate_rapid`,
  `text_blink_ease_in`, `text_blink_ease_out`, `text_blink_rapid_ease_in`,
  `text_blink_rapid_ease_out`, `font_size`, `cell_width`, `cell_widths`, `line_height`,
  `font_antialias`, `font_hinting`, `font_rasterizer`,
  `font_colr_rasterizer`, `font_shaper`, `font_dirs`, `font_locator`,
  `use_cap_height_to_scale_fallback_fonts`, `ignore_svg_fonts`,
  `sort_fallback_fonts_by_coverage`, `search_font_dirs_for_fallback`,
  `custom_block_glyphs`,
  `anti_alias_custom_block_glyphs`,
  `allow_square_glyphs_to_overflow_width`, `freetype_load_target`,
  `freetype_render_target`, `freetype_load_flags`,
  `freetype_interpreter_version`, `freetype_pcf_long_family_names`, `display_pixel_geometry`, `dpi`, `dpi_by_screen`, `bold_brightens_ansi_colors`, `default_cursor_style`, `cursor_thickness`, `underline_thickness`, `underline_position`, `strikethrough_position`, `window_padding`, `window_content_alignment`, `kde_window_background_blur`, `macos_window_background_blur`, `win32_system_backdrop`, `win32_acrylic_accent_color`, `window_decorations`, `integrated_title_buttons`, `integrated_title_button_alignment`, `integrated_title_button_color`, `integrated_title_button_style`,
  `force_reverse_video_cursor`, `reverse_video_cursor_min_contrast`,
  `initial_cols`, `initial_rows`,
  `adjust_window_size_when_changing_font_size`, `command_palette_rows`,
  `command_palette_font_size`, `command_palette_bg_color`,
  `command_palette_fg_color`, `char_select_font_size`,
  `char_select_bg_color`, `char_select_fg_color`, `pane_select_font_size`,
  `pane_select_bg_color`, `pane_select_fg_color`, `launcher_alphabet`, `quick_select_alphabet`,
  `quick_select_patterns`, `disable_default_quick_select_patterns`,
  `quick_select_remove_styling`, `hyperlink_rules`, `selection_word_boundary`, `term`, `enq_answerback`, `audible_bell`, `visual_bell`, `colors`, `color_scheme`, `color_schemes`, `color_scheme_dirs`, `resolved_palette`, `foreground_color`, `background_color`, `ansi_palette`, `indexed_palette`, `selection_fg_color`, `selection_bg_color`, `cursor_bg_color`, `cursor_border_color`, `cursor_fg_color`, `compose_cursor_color`, `visual_bell_color`, `notification_handling`, `default_prog`,
  `default_domain`, `default_workspace`, `prefer_to_spawn_tabs`, `automatically_reload_config`, `check_for_updates`, `check_for_updates_interval_seconds`, `show_update_window`, `native_macos_fullscreen_mode`, `macos_fullscreen_extend_behind_notch`, `use_resize_increments`, `debug_key_events`, `log_unknown_escape_sequences`, `warn_about_missing_glyphs`, `default_cwd`, `default_ssh_auth_sock`, `default_gui_startup_args`, `default_mux_server_domain`, `daemon_options`, `exec_domains`, `wsl_domains`, `unix_domains`, `ssh_domains`, `tls_servers`, `tls_clients`, `serial_ports`, `mux_enable_ssh_agent`, `ssh_backend`, `ratelimit_mux_line_prefetches_per_second`, `mux_output_parser_buffer_size`, `mux_output_parser_coalesce_delay_ms`, `mux_env_remove`, `tiling_desktop_environments`, `periodic_stat_logging`, `ulimit_nofile`, `ulimit_nproc`, `detect_password_input`, `set_environment_variables`, `launch_menu`, `leader`, `keys`, `key_tables`, `mouse_bindings`, `key_map_preference`,
  `ui_key_cap_rendering`, `swap_backspace_and_delete`, `enable_csi_u_key_encoding`,
  `enable_kitty_graphics`, `enable_checksum_rectangular_area`, `enable_title_reporting`,
  `enable_kitty_keyboard`, `allow_download_protocols`, `xcursor_theme`,
  `xcursor_size`, `palette_max_key_assigments_for_action`,
  `allow_win32_input_mode`,
  `treat_left_ctrlalt_as_altgr`,
  `send_composed_key_when_left_alt_is_pressed`,
  `send_composed_key_when_right_alt_is_pressed`,
  `treat_east_asian_ambiguous_width_as_wide`,
  `normalize_output_to_unicode_nfc`, `unicode_version`, `bidi_enabled`,
  `bidi_direction`, `use_ime`, `use_dead_keys`, `ime_preedit_rendering`,
  `macos_forward_to_ime_modifier_mask`,
  `xim_im_name`,
  `scroll_to_bottom_on_input`,
  `alternate_buffer_wheel_scroll_speed`,
  `canonicalize_pasted_newlines`, `quote_dropped_files`,
  `disable_default_key_bindings`,
  `disable_default_mouse_bindings`,
  `hide_mouse_cursor_when_typing`,
  `pane_focus_follows_mouse`, `swallow_mouse_click_on_pane_focus`,
  `swallow_mouse_click_on_window_focus`, `bypass_mouse_reporting_modifiers`,
  `enable_scroll_bar`, `min_scroll_bar_height`, `enable_tab_bar`, `hide_tab_bar_if_only_one_tab`,
  `use_fancy_tab_bar`, `unzoom_on_switch_pane`,
  `tab_bar_at_bottom`, `tab_and_split_indices_are_zero_based`,
  `mouse_wheel_scrolls_tabs`,
  `switch_to_last_active_tab_when_closing_tab`,
  `quit_when_all_windows_are_closed`,
  `window_close_confirmation`,
  `exit_behavior`,
  `clean_exit_codes`,
  `exit_behavior_messaging`,
  `skip_close_confirmation_for_processes_named`,
  `show_close_tab_button_in_tabs`,
  `show_new_tab_button_in_tab_bar`, `show_tab_index_in_tab_bar`, and
  `show_tabs_in_tab_bar`) and emits
  `window-config-reloaded` on every set. `automatically_reload_config` is
  stored with WezTerm's default `true`, parses inline or through top-level
  static bool variables, and is included in effective config snapshots.
  `check_for_updates` is stored with WezTerm's default `true`,
  `check_for_updates_interval_seconds` with the default `86400`, and
  `show_update_window` with the compatibility default `false`; actual update
  checks and update-window UI remain future parity work.
  `native_macos_fullscreen_mode` is stored with WezTerm's default `false`,
  parses inline or through top-level static bool variables, and is included in
  effective config snapshots; on macOS, `ToggleFullScreen` uses simple fullscreen
  by default and the native macOS Space transition when this option is `true`.
  `macos_fullscreen_extend_behind_notch` is stored with WezTerm's default
  `false` and selects the simple-fullscreen behind-notch request path only when
  `native_macos_fullscreen_mode` is `false`.
  Non-macOS platforms keep the existing borderless fullscreen behavior.
  `max_fps` is stored
  with WezTerm's default `60` and parses inline or through top-level static
  number variables before throttling native redraw requests from `about_to_wait`
  to the configured frame interval. `animation_fps` is stored with the default
  `10`, parses inline or through top-level static number variables, and drives
  dedicated redraw scheduling for active cursor/text blink easing, visual bell
  fade, and animated inline-image frames,
  while still respecting the global `max_fps` ceiling. `front_end` is stored
  with WezTerm's current default `OpenGL`,
  `webgpu_power_preference` with `LowPower`, `webgpu_force_fallback_adapter`
  with `false`, optional static `webgpu_preferred_adapter` tables inline or
  through top-level static table variables with fields inline or through
  top-level static string/integer variables, plus top-level
  `config[static_name].backend`/`device`/`device_type`/`driver`/
  `driver_info`/`name`/`vendor` mutations where `static_name` resolves to
  `webgpu_preferred_adapter`, `prefer_egl`/`enable_wayland` with `true`,
  `enable_zwlr_output_manager`, `use_box_model_render`, and
  `experimental_pixel_positioning` with `false`, render cache sizes
  `shape_cache_size`, `line_state_cache_size`, `line_quad_cache_size`, and
  `line_to_ele_shape_cache_size` with `1024`, plus
  `glyph_cache_image_cache_size` with `256`; actual renderer front-end,
  WebGPU adapter, EGL, box-model render path, pixel-positioning behavior,
  cache sizing application, zwlr output-manager integration, and Wayland/X11
  startup selection remain future parity work.
  `use_resize_increments` is
  stored with WezTerm's default `false`, parses inline or through top-level
  static bool variables, and is included in effective config snapshots; when
  enabled on X11/Wayland/macOS-capable builds, native windows
  set their resize increments to the current terminal cell size and refresh the
  hint after font, `cell_width`, or `line_height` changes. Unsupported
  platforms keep the WezTerm-style no-op behavior. `debug_key_events` and
  `log_unknown_escape_sequences` are
  stored with WezTerm's default `false`, parse inline or through top-level
  static bool variables, and are included in effective config snapshots.
  Startup/SSH/environment compatibility fields are retained in the native config
  snapshot: `default_gui_startup_args` defaults to `["start"]`,
  `ssh_backend` to `LibSsh`, and `tiling_desktop_environments` to WezTerm's
  X11 tiling WM list; actual GUI startup-arg consumption, SSH backend
  selection, and resize behavior based on desktop environment remain future
  parity work. Mux/resource compatibility fields are retained in the native
  config snapshot:
  `default_mux_server_domain` defaults to `None`,
  `daemon_options` stores optional `pid_file`, `stdout`, and `stderr` paths and
  defaults each to `None`,
  `exec_domains` stores static exec domain `name`, generated or explicit
  `fixup_command`, and optional string/function label event names and defaults
  to an empty list,
  `wsl_domains` stores explicit WSL domain `name`, `distribution`,
  `username`, `default_cwd`, and `default_prog` entries and defaults to an
  empty list,
  `unix_domains` stores Unix domain `name`, optional `socket_path`,
  `connect_automatically`, `no_serve_automatically`, optional `serve_command`
  and `proxy_command` arrays, `skip_permissions_check`, `read_timeout_ms`,
  `write_timeout_ms`, optional `local_echo_threshold_ms`, and
  `overlay_lag_indicator` entries and defaults to WezTerm's `unix` domain with
  60000ms read/write timeouts,
  `ssh_domains` stores explicit SSH domain `name`, `remote_address`,
  `no_agent_auth`, optional `username`, `connect_automatically`,
  `timeout_ms`, optional `local_echo_threshold_ms`, `overlay_lag_indicator`,
  optional `remote_wezterm_path`, optional `override_proxy_command`, optional
  per-domain `ssh_backend`, `multiplexing`, `ssh_option` string map, optional
  `default_prog`, and `assume_shell` entries and defaults to an empty explicit
  list while WezTerm's dynamic SSH-config host enumeration remains future work,
  `tls_servers` stores TLS server `bind_address`, optional `pem_private_key`,
  `pem_cert`, `pem_ca`, and `pem_root_certs` entries and defaults to an empty
  list,
  `tls_clients` stores TLS client `name`, optional `bootstrap_via_ssh`,
  `remote_address`, optional PEM key/cert/CA paths, `pem_root_certs`,
  `accept_invalid_hostnames`, optional `expected_cn`, `connect_automatically`,
  `read_timeout_ms`, `write_timeout_ms`, optional `local_echo_threshold_ms`,
  optional `remote_wezterm_path`, and `overlay_lag_indicator` entries and
  defaults to an empty list,
  `serial_ports` stores optional serial domain `name`, `port`, and `baud`
  entries and defaults to an empty list,
  `ratelimit_mux_line_prefetches_per_second` to `50`,
  `mux_output_parser_buffer_size` to `131072`,
  `mux_output_parser_coalesce_delay_ms` to `3`, `periodic_stat_logging` to
  `0`, and both `ulimit_nofile` and `ulimit_nproc` to `2048`; applying those
  values to a real mux server/parser, daemon pid/log redirection, WSL default
  discovery, SSH config host enumeration, exec/WSL/Unix/SSH/TLS/serial domain
  connections, Unix socket/proxy timeout application, SSH timeout/backend/
  multiplexing application, TLS certificate loading/validation and timeout
  application, or process rlimits remains future parity work.
  `treat_left_ctrlalt_as_altgr` is stored with WezTerm's default
  `false`; when enabled, Ctrl+Alt text key events are routed as AltGr text
  input instead of matching Ctrl+Alt key bindings.
  `send_composed_key_when_left_alt_is_pressed` is stored with WezTerm's default
  `false`, `send_composed_key_when_right_alt_is_pressed` with WezTerm's default
  `true`, and native window input tracks left/right Alt key state so composed
  text from right Alt is written without an ESC Meta prefix while raw Alt key
  assignments retain precedence.
  `treat_east_asian_ambiguous_width_as_wide` is stored with WezTerm's
  default `false` and, when enabled, makes the terminal runtime treat East
  Asian ambiguous-width characters as two cells wide; static numeric
  `cell_widths` override tables are stored and applied with higher priority
  than that ambiguous-width setting, with `first`/`last`/`width` fields inline
  or through top-level static number variables, while dynamic Lua/exact nightly
  parity remains future work. `normalize_output_to_unicode_nfc` is stored with
  WezTerm's default `false` and, when enabled, normalizes contiguous ordinary
  terminal output runs to Unicode NFC before the cells are written, including
  leading combining marks that arrive in the next PTY chunk when they compose
  with the prior cell without changing display width. `unicode_version` is
  retained with WezTerm's default `9`, parses static numeric Lua snippets into
  effective config, applies that setting to active and new pane runtimes, and
  tracks OSC 1337 `UnicodeVersion` set/push/pop terminal state, including
  labeled stack entries. Unicode 8-or-earlier runtimes keep WezTerm's
  `WidenedIn9` characters narrow, Unicode 9+ widens them, and Unicode 14+
  emoji/text presentation selectors adjust the prior cell width for FE0F/FE0E
  sequences. `bidi_enabled` and `bidi_direction` are retained with WezTerm's
  `false` and `LeftToRight` defaults in effective config and parse static Lua
  snippets; actual bidi shaping/rendering remains future parity work. `use_ime`
  and `use_dead_keys` are stored with WezTerm's current default `true`,
  `ime_preedit_rendering` with WezTerm's `Builtin` default,
  `macos_forward_to_ime_modifier_mask` with WezTerm's `SHIFT` default, and
  `xim_im_name` is retained as an optional XIM server name for X11-style IME
  configuration.
  Native winit IME commit text is written to the active pane when `use_ime` is
  enabled and ignored when disabled; native winit IME preedit text is rendered
  as a Builtin overlay at the active pane cursor and suppressed for `System` or
  disabled IME, with commit/empty preedit clearing the overlay. On macOS with
  `use_ime` enabled, modified key presses whose modifiers intersect
  `macos_forward_to_ime_modifier_mask` are left for IME routing before native
  shortcut/input handling. Static Lua
  `colors.compose_cursor` overrides the cursor color while Builtin preedit text
  or the leader modifier is active, and dead-key input uses the same cursor
  override while composition is pending unless `use_dead_keys=false`. Deeper
  platform IME/XIM setup remains future parity work.
  `detect_password_input` is stored
  with WezTerm's default `true`;
  actual Unix local-pane termios probing and lock-cursor rendering remain
  future parity work. `warn_about_missing_glyphs` is
  stored with WezTerm's default `true` and included in effective config
  snapshots. Missing glyph codepoints detected in rendered cells are emitted
  once per native window as stderr `CONFIG ERROR missing glyph ...`
  diagnostics when `warn_about_missing_glyphs` is enabled; setting it false
  suppresses those diagnostics. Unknown ESC/CSI sequences are recorded by the
  terminal runtime and emitted as native stderr warnings when
  `log_unknown_escape_sequences` is enabled. Native key events are emitted as
  stderr `INFO key_event` diagnostics when `debug_key_events` is enabled; full
  WezTerm-style configuration error window UI, arbitrary Lua config execution,
  dependency/helper-file watching, Lua `window:set_config_overrides` wiring,
  and broader registry coverage remain future parity work.
- Implemented in v1: native `window_close_confirmation` defaults to
  `AlwaysPrompt` for window-manager/decorations close requests, showing the
  same confirmation overlay style as tab/pane close confirmations and only
  closing the window after acceptance. Setting it to `NeverPrompt` requests the
  close immediately. Native `skip_close_confirmation_for_processes_named`
  stores WezTerm's default stateless-process list or a custom override, and
  close-window, close-tab, and close-pane confirmation targets skip the overlay
  when every affected pane's known local launch-program basename matches the
  list. Static Lua config parsing covers `window_close_confirmation` and
  `skip_close_confirmation_for_processes_named` inline, through top-level
  static table variables, through static field names in returned config table
  initializers and direct return-table configs, or through
  `table.insert(config.skip_close_confirmation_for_processes_named, ...)`
  appends, plus `table.insert(config[static_name], ...)` and
  `config[static_name][#config[static_name] + 1] = ...` appends where
  `static_name` resolves to `skip_close_confirmation_for_processes_named`.
  Full child process tree inspection and
  `mux-is-process-stateful` remain future parity work.
- Implemented in v1: opening the command palette dispatches a typed native
  `augment-command-palette` hook with the window id and active pane id. Returned
  entries join the same fuzzy filtering, palette status, selection, and
  execution flow using the implemented `WindowCommand` action subset, with
  optional `doc` text and known Nerd Font `icon` names, including the official
  `md_rename_box`, `fa_clock_o`, and `cod_github` examples, shown in the visible
  candidate row. Bounded-static event callbacks are implemented; arbitrary Lua
  callbacks, the full Nerd Font icon catalog, and the full WezTerm action surface
  remain future parity work.
- Implemented in v1: native WezTerm-style `Multiple` action payloads can
  sequence implemented `WindowCommand` values, applying each command in order
  and stopping on the first failure. Structured `multiple <command> ; <command>`
  command-palette queries split only on unquoted separators, so quoted
  `send string` payloads can contain ` ; ` text. Static `config.keys`
  `Multiple` action calls also accept parenthesized top-level static action
  table variables.
- Implemented in v1: command-palette `ClearSelection` plus structured
  `clearselection` action-name queries clear active-window selection state and
  refresh rendered selection highlights.
- Implemented in v1: command-palette and native action payload
  `SelectTextAtMouseCursor` and `ExtendSelectionToMouseCursor` cover Cell,
  Word, Line, and Block modes using the current mouse cell. Structured
  command-palette queries accept `select text at mouse cursor <mode>` /
  `selecttextatmousecursor <mode>` and `extend selection to mouse cursor
  <mode>` / `extendselectiontomousecursor <mode>` action-name forms.
  Default left-mouse selection follows WezTerm's Cell/Word/Line click streaks,
  `SHIFT` left click extends the active selection, `ALT` left drag creates a
  rectangular block selection, and `ALT|SHIFT` left click extends the active
  selection as a rectangular block. Releasing a non-empty left-drag selection
  or modified extension copies it to ClipboardAndPrimarySelection, while
  NONE/SHIFT single-click release can open the OSC 8 hyperlink under the mouse.
  Double/triple-click drag extends by Word/Line boundaries, and
  double/triple-click release completes the selected word or line to
  ClipboardAndPrimarySelection.
  Command-palette and native `SelectTextAtMouseCursor` also cover SemanticZone
  selection for the OSC 133 semantic zone under the mouse. Static `config.keys`
  action calls resolve top-level string variables for
  `SelectTextAtMouseCursor` and `ExtendSelectionToMouseCursor` modes.
- Implemented in v1: command-palette Clear Scrollback and native
  `ClearScrollback('ScrollbackOnly')` action payloads clear active-pane history
  on the output side while preserving the viewport; structured
  `clear scrollback <mode>` and `clearscrollback <mode>` queries accept quoted
  or unquoted modes, and WezTerm-style Lua `ClearScrollback { ... }` /
  `ClearScrollback({ ... })` table queries tolerate trailing comma fields.
  Static `config.keys` action tables also resolve top-level string variables
  for the `mode` field and parenthesized static options table variables.
- Implemented in v1: command-palette Clear Scrollback And Viewport and native
  `ClearScrollback('ScrollbackAndViewport')` action payloads clear active-pane
  history plus the viewport while preserving the prompt/cursor row as the new
  first visible line. Action-name `clearscrollbackandviewport` queries dispatch
  the no-argument compatibility command.
- Implemented in v1: command-palette Copy To Clipboard and native
  `CopyTo('Clipboard')` action payloads map the active selection to WezTerm's
  clipboard behavior; the default `Super+C` and `Ctrl+Shift+C` shortcuts plus
  the dedicated `Copy` key map to the same clipboard destination. Action-name
  `copytoclipboard` queries route through the same command.
- Implemented in v1: command-palette Copy To Primary Selection and Copy To
  Clipboard And Primary Selection plus native action payloads map to
  WezTerm-style `CopyTo('PrimarySelection')` and
  `CopyTo('ClipboardAndPrimarySelection')` routing, with quoted or unquoted
  `copy to <destination>` and `copyto <destination>` queries plus action-name
  `copytoprimaryselection` and `copytoclipboardandprimaryselection` queries.
  Static `config.keys` action calls resolve top-level string variables for
  `CopyTo` destinations. Native `CopyTextTo { text, destination }` payloads
  write explicit text to Clipboard, PrimarySelection, or both copy targets
  without requiring an active selection, including WezTerm-style
  `wezterm.action.CopyTextTo { text = ..., destination = ... }` and
  `wezterm.action { CopyTextTo = { ... } }` query/config forms with top-level
  static string variables for both fields.
  The native platform PrimarySelection backend remains a later
  platform-adapter task.
- Implemented in v1: command-palette Paste From Clipboard and native
  `PasteFrom('Clipboard')` action payloads map the configured clipboard reader
  into the active pane; the default `Super+V` and `Ctrl+Shift+V` shortcuts plus
  the dedicated `Paste` key map to the same clipboard source while unmodified
  `Ctrl+V` remains available to the PTY. Action-name `pastefromclipboard`
  queries route through the same command. Native
  `canonicalize_pasted_newlines` normalizes non-bracketed paste newlines to
  `None`, `LineFeed`, `CarriageReturn`, or `CarriageReturnAndLineFeed`, while
  bracketed paste sends the original text inside bracketed-paste markers.
- Implemented in v1: native file-drop events write the dropped path to the
  active pane using WezTerm-style `quote_dropped_files` modes: `None`,
  `SpacesOnly`, `Posix`, `Windows`, and `WindowsAlwaysQuoted`. Defaults match
  WezTerm's platform split: `Windows` on Windows and `SpacesOnly` elsewhere.
- Implemented in v1: command-palette Paste From Primary Selection and native
  `PasteFrom('PrimarySelection')` action payloads route primary-selection text
  to the active pane, quoted or unquoted `paste from <source>` and
  `pastefrom <source>` queries plus action-name
  `pastefromprimaryselection` map to the same native payloads, and default
  `Ctrl+Insert`/`Shift+Insert` shortcut classification plus default unmodified
  middle-click paste now match
  WezTerm's PrimarySelection defaults. Static `config.keys` action calls
  resolve top-level string variables for `PasteFrom` sources.
- Implemented in v1: deprecated native WezTerm aliases `Copy`, `Paste`, and
  `PastePrimarySelection` are accepted for older action payload compatibility
  and route to `CopyTo('Clipboard')`, `PasteFrom('Clipboard')`, and
  `PasteFrom('PrimarySelection')` respectively. Action-name `copy`, `paste`,
  and `pasteprimaryselection` queries dispatch those aliases directly.
- Implemented in v1: default `Super+R`, `Super+K`/`Ctrl+Shift+K`, and
  `Super+F` plus `Ctrl+Shift+F` shortcuts route to the same
  reload-configuration, clear-scrollback, and
  `Search(CaseSensitiveString="")` paths as their implemented WezTerm-style
  counterparts while plain `Ctrl+F` remains available to the active PTY.
- Implemented in v1: OSC 52 and iTerm2 `OSC 1337;Copy=;base64` clipboard writes
  are extracted from ESC plus UTF-8 C1 OSC/ST active and inactive pane output,
  with legacy raw C1 compatibility, and routed through the same clipboard
  writer/policy path used for OSC52 clipboard writes. The default OSC52 policy
  is WezTerm-style write-only; read queries require explicit `--osc52
  read-write`.
- Implemented in v1: WezTerm-documented OSC 9 notification text and OSC 777
  `notify` title/body events are extracted from ESC plus UTF-8 C1 OSC/ST active
  and inactive pane output, with legacy raw C1 compatibility, and routed through
  the native-window notification handler. Native per-window
  `notification_handling` defaults to `AlwaysShow` and can suppress all
  notifications, notifications from the focused pane, notifications from the
  focused tab, or notifications while the window is focused before handler
  dispatch and title-status updates. The native window title shows the latest
  notification as a status suffix; native OS toast integration remains future
  platform-adapter work.
- Implemented in v1: ConEmu-style `OSC 9;4;st;pr` progress reports update
  terminal-runtime progress state as None, percentage, error, or indeterminate
  from ESC plus UTF-8 C1 OSC/ST forms, are not misrouted as OSC 9 notifications,
  sync into active/inactive app-shell pane metadata, and mark native tab bar
  entries as `N%`, `err:N%`, or `~`. Static `update-status` callbacks can read
  active-pane `pane:get_progress()` or
  `window:active_pane():get_progress()` through documented `Percentage`,
  `Error`, and `Indeterminate` branches; arbitrary Lua pane API execution and
  broader configurable status formatting remain future parity work.
- Implemented in v1: ASCII BEL from active and inactive pane output is counted
  in metrics and dispatched through a typed native-window bell hook with the
  window id and originating pane id. Native per-window `audible_bell` overrides
  support `SystemBeep` and `Disabled`; disabling the audible bell suppresses
  only the system-beep path. Native per-window `visual_bell` overrides support
  WezTerm's zero-duration default no-op plus `BackgroundColor` pane flashes and
  `CursorColor` cursor-color flashes derived from the active rendered foreground
  color with a default text-foreground fallback, with native `visual_bell_color`
  overrides standing in for WezTerm `colors.visual_bell`; background flashes
  include blank cells and blend over existing background/cursor colors across
  the configured fade-in/fade-out durations using the native
  Constant/Linear/Ease/EaseIn/EaseOut/EaseInOut/CubicBezier easing subset;
  `CursorColor` fades return to the current rendered cursor color, including
  `force_reverse_video_cursor` cursor-cell foreground behavior. Static
  WezTerm-style Lua `config.visual_bell` snippets parse inline or through
  top-level static table variables, and its duration, easing, and target fields
  parse inline, through top-level static scalar/table variables, or through
  top-level `config[static_name].fade_in_duration_ms`/`fade_out_duration_ms`/
  `fade_in_function`/`fade_out_function`/`target` mutations where
  `static_name` resolves to `visual_bell`. Native
  `foreground_color`, `background_color`, `ansi_palette`, `indexed_palette`,
  `selection_fg_color`, `selection_bg_color`, `cursor_bg_color`,
  `cursor_border_color`, and `cursor_fg_color` overrides stand in for WezTerm
  `colors.foreground`, `colors.background`, `colors.ansi`, `colors.brights`,
  `colors.indexed`, `colors.selection_fg`, `colors.selection_bg`,
  `colors.cursor_bg`, `colors.cursor_border`, `colors.cursor_fg`, and
  `colors.compose_cursor`; static `config.color_schemes` entries can define
  custom in-file schemes inline, through whole-table assignments such as
  `config.color_schemes = schemes` including top-level
  `schemes['Name'] = { ... }` entries and their supported static field
  mutations assigned before that reference, or through static top-level Lua
  table variables assigned before their reference with their own supported
  static field mutations, and static top-level
  `config.color_schemes['Name'] = scheme` or
  `config.color_schemes.Name = scheme` assignments can append or replace named
  schemes after initialization. Selected custom scheme entries also support
  static top-level field mutations such as
  `config.color_schemes['Name'].background = '#101010'` and bracket-key
  variants such as `config.color_schemes['Name']['cursor_bg'] = '#101010'`,
  indexed slot mutations such as
  `config.color_schemes['Name'].indexed[136] = '#101010'`, plus ANSI/bright
  slot mutations such as
  `config.color_schemes['Name'].ansi[2] = '#101010'` and tab-bar nested
  top-level mutations such as
  `config.color_schemes['Name'].tab_bar.active_tab.bg_color = '#101010'`,
  and ColorSpec nested mutations such as
  `config.color_schemes['Name'].quick_select_match_fg.Color = '#101010'`.
  Helper-function-local assignments and mutations are ignored. Mutations are
  applied after the final selected static scheme definition, so later full
  `config.color_schemes['Name'] = { ... }` assignments replace earlier entry
  mutations. Returned static config variables such as `return cfg`
  also carry `cfg.color_schemes['Name']` entry assignments and selected-scheme
  mutations while unreturned `config.color_schemes` assignments are ignored.
  `config.color_scheme`, inline or through a top-level static string variable,
  selects one before `config.colors` applies overriding fields. The selected
  scheme name, custom `color_schemes` map, and explicit `colors` palette are
  retained in effective config; a WezTerm-style `resolved_palette` snapshot is
  derived from the final applied native color fields, and static
  `config.color_scheme_dirs` lists, inline, through top-level static table
  variables, or through `table.insert(config.color_scheme_dirs, ...)` appends,
  are retained in effective config and scan configured directories for matching
  TOML scheme files. External TOML schemes
  load when `[metadata].name` or the file stem matches `config.color_scheme`
  and reuse the same implemented color fields before `config.colors` applies
  overriding fields. Static `wezterm.color.load_scheme(...)` paths may be
  inline quoted or long-bracket literals, or statically evaluable top-level
  string expressions composed of quoted/long-bracket literals, identifier
  variable chains, and pure `..` concatenation. Each identifier resolves to its
  latest top-level binding at that evaluation point; an assigned variable
  evaluates its right-hand side at the assignment offset, while the call
  argument evaluates at the original call offset, preserving assignment-time
  capture and isolating the loaded path from rebinding after the call. One
  resolver is shared by `config.colors` and selected
  `config.color_schemes['Name']` entries and normalizes canonical direct calls,
  module aliases, direct or parenthesized `require('wezterm')` receivers,
  static-key access, and function aliases, including supported Lua comments
  between helper-path segments or before call arguments. Loaded colors can flow
  directly, through static variables whose supported mutations are applied, or
  through the first returned variable from `local colors, metadata = ...` or
  `colors, metadata = ...` assignments. This remains bounded static
  interpretation: paths derived from dynamic helpers, branches, or
  environment-backed values, arbitrary Lua
  execution, and downstream consumption of returned `metadata` remain open.
  `wezterm.config_dir` (including `local wt = require 'wezterm'` alias forms) resolves
  from `WEZTERM_CONFIG_DIR` in static path composition.
  Static `load_scheme` result-variable references resolve to the latest top-level
  binding before the `config.colors` assignment and ignore
  helper-function-local bindings/mutations plus later rebinding, including
  top-level static mutations such as
  `colors.background = '#101010'` and bracket-key variants such as
  `colors['background'] = '#101010'`, indexed slot mutations such as
  `colors.indexed[136] = '#101010'`, ANSI/bright slot mutations such as
  `colors.ansi[2] = '#101010'`, tab-bar nested mutations such as
  `colors.tab_bar.active_tab.bg_color = '#101010'`, static-key config
  mutations such as `config[colors_field].tab_bar[tab_field].bg_color` where
  the keys resolve to `colors` and a supported tab-bar item, direct ColorSpec
  nested mutations such as
  `config.colors.quick_select_match_bg.Color = '#101010'` and
  `colors.quick_select_match_bg.Color = '#101010'`, or multiline table
  mutations such as `colors.ansi = { ... }` before assignment. When complete
  `config.colors`
  table assignments, static table-variable `config.colors = colors`
  assignments, and load-scheme-backed `config.colors = colors` assignments
  appear together, the static parser chooses the later source before
  applying supported mutations; a top-level `return { colors = ... }` table or
  returned static config variable such as `return cfg` is treated as the
  returned config and wins over earlier unreturned `config.colors`
  assignments. When no in-file or configured-dir
  scheme matches, the default WezTerm custom scheme directories are also
  searched: `$HOME/.config/wezterm/colors` on POSIX and `colors` next to the
  executable on Windows. These color overrides drive the default text
  foreground, framebuffer background, ANSI 0-15 palette, indexed 16-255 palette
  overrides, selected text foreground/background, cursor fill, block-cursor
  border, line-cursor color, and block-cursor text foreground for full and
  damage renders. Static `wezterm.color.get_builtin_schemes()['Name']` and
  legacy `wezterm.get_builtin_color_schemes()['Name']` lookups now feed
  `config.colors` directly or via supported variables, and also populate
  `config.color_schemes['Name']` table entries (inline or direct assignment)
  before selected-entry mutations. Built-in lookup also accepts top-level static
  helper aliases, parenthesized `require('wezterm')` receivers, and canonical
  static-key access, following the same `load_scheme`-shape restrictions.
  Bounded-static `wezterm.color.get_default_colors()` is also a shared palette
  source for direct or returned `config.colors`, intermediate palette
  variables, and inline/direct `config.color_schemes['Name']` entries. Exact
  zero-argument calls accept the same statically proven module, color-namespace,
  function, and key aliases; the existing reducer then replays supported
  mutations in source order, including fresh-call rebinding. Its palette is
  pinned to WezTerm `093bf6b`, including alpha-bearing selection colors, all 16
  ANSI colors, and every indexed color from 16 through 255. Arguments,
  expression tails, dynamic keys or rebinding, escaped identities, and
  arbitrary Lua execution remain unsupported. Arbitrary Lua event execution and richer
  dynamic `load_scheme` composition remain future parity work.
- Implemented in v1: native window focus changes still write CSI focus-reporting
  sequences to the PTY when requested and now dispatch a typed focus-change hook
  with the window id, active pane id, and focused/unfocused state. Lua event
  wiring on static event names is implemented; arbitrary Lua event execution
  remains future parity work.
- Implemented in v1: successful native window resizes and fullscreen/windowed
  transitions dispatch a typed resize hook with the window id, active pane id,
  pixel size, terminal rows/columns, and `is_full_screen` state so native
  handlers receive the fullscreen dimension metadata exposed by WezTerm's
  window dimensions APIs. Static event wiring is implemented; arbitrary Lua
  event execution remains future parity work.
- Implemented in v1: command-palette `ReloadConfiguration` and the default
  `Ctrl+Shift+R` shortcut dispatch a typed native `window-config-reloaded` hook
  with the window id and active pane id.
  A typed native `set_config_overrides`/`get_config_overrides` subset stores
  per-window overrides for `dpi`, `tab_max_width`, `status_update_interval`,
  `max_fps`, `animation_fps`, `cursor_blink_rate`, `cursor_blink_ease_in`, `cursor_blink_ease_out`,
  `text_blink_rate`, `text_blink_rate_rapid`, `text_blink_ease_in`,
  `text_blink_ease_out`, `text_blink_rapid_ease_in`,
  `text_blink_rapid_ease_out`,
  `font_size`, `cell_width`, `cell_widths`, `line_height`, `font_antialias`, `font_hinting`, `font_rasterizer`, `font_colr_rasterizer`, `font_shaper`, `font_dirs`, `font_locator`, `use_cap_height_to_scale_fallback_fonts`, `ignore_svg_fonts`, `sort_fallback_fonts_by_coverage`, `search_font_dirs_for_fallback`, `custom_block_glyphs`, `anti_alias_custom_block_glyphs`, `allow_square_glyphs_to_overflow_width`, `freetype_load_target`, `freetype_render_target`, `freetype_load_flags`, `freetype_interpreter_version`, `freetype_pcf_long_family_names`, `display_pixel_geometry`, `dpi`, `dpi_by_screen`, `foreground_text_hsb`, `bold_brightens_ansi_colors`, `text_background_opacity`, `window_background_opacity`, `window_background_gradient`, `kde_window_background_blur`, `macos_window_background_blur`, `win32_system_backdrop`, `win32_acrylic_accent_color`, `window_decorations`, `integrated_title_buttons`, `integrated_title_button_alignment`, `integrated_title_button_color`, `integrated_title_button_style`, `default_cursor_style`, `cursor_thickness`, `underline_thickness`, `underline_position`, `strikethrough_position`, `force_reverse_video_cursor`, `reverse_video_cursor_min_contrast`, `window_content_alignment`,
  `initial_cols`, `initial_rows`, `adjust_window_size_when_changing_font_size`,
  `inactive_pane_hsb`, `command_palette_rows`, `command_palette_font_size`,
  `command_palette_bg_color`, `command_palette_fg_color`,
  `char_select_font_size`, `char_select_bg_color`, `char_select_fg_color`,
  `pane_select_font_size`, `pane_select_bg_color`, `pane_select_fg_color`,
  `launcher_alphabet`, `quick_select_alphabet`, `quick_select_patterns`,
  `disable_default_quick_select_patterns`, `quick_select_remove_styling`,
  `hyperlink_rules`, `selection_word_boundary`, `term`, `enq_answerback`,
  `audible_bell`, `visual_bell`, `colors`, `color_scheme`, `color_schemes`, `color_scheme_dirs`, `resolved_palette`, `foreground_color`, `background_color`, `ansi_palette`, `indexed_palette`, `selection_fg_color`, `selection_bg_color`, `cursor_bg_color`, `cursor_border_color`, `cursor_fg_color`, `compose_cursor_color`, `visual_bell_color`, `notification_handling`, `default_prog`,
  `default_domain`, `default_workspace`, `prefer_to_spawn_tabs`, `automatically_reload_config`, `check_for_updates`, `check_for_updates_interval_seconds`, `show_update_window`, `native_macos_fullscreen_mode`, `macos_fullscreen_extend_behind_notch`, `use_resize_increments`, `debug_key_events`, `log_unknown_escape_sequences`, `warn_about_missing_glyphs`, `default_cwd`, `default_ssh_auth_sock`, `default_gui_startup_args`, `default_mux_server_domain`, `daemon_options`, `exec_domains`, `wsl_domains`, `unix_domains`, `ssh_domains`, `tls_servers`, `tls_clients`, `serial_ports`, `mux_enable_ssh_agent`, `ssh_backend`, `ratelimit_mux_line_prefetches_per_second`, `mux_output_parser_buffer_size`, `mux_output_parser_coalesce_delay_ms`, `mux_env_remove`, `tiling_desktop_environments`, `periodic_stat_logging`, `ulimit_nofile`, `ulimit_nproc`, `detect_password_input`, `set_environment_variables`, `launch_menu`, `leader`, `keys`, `key_tables`, `mouse_bindings`, `key_map_preference`,
  `ui_key_cap_rendering`, `swap_backspace_and_delete`, `enable_csi_u_key_encoding`,
  `enable_kitty_graphics`, `enable_checksum_rectangular_area`, `enable_title_reporting`,
  `enable_kitty_keyboard`, `allow_download_protocols`, `xcursor_theme`,
  `xcursor_size`, `palette_max_key_assigments_for_action`,
  `allow_win32_input_mode`,
  `treat_left_ctrlalt_as_altgr`,
  `send_composed_key_when_left_alt_is_pressed`,
  `send_composed_key_when_right_alt_is_pressed`,
  `treat_east_asian_ambiguous_width_as_wide`,
  `normalize_output_to_unicode_nfc`, `unicode_version`, `bidi_enabled`,
  `bidi_direction`, `use_ime`, `use_dead_keys`, `ime_preedit_rendering`,
  `macos_forward_to_ime_modifier_mask`,
  `xim_im_name`,
  `scroll_to_bottom_on_input`,
  `alternate_buffer_wheel_scroll_speed`,
  `canonicalize_pasted_newlines`,
  `quote_dropped_files`,
  `disable_default_key_bindings`,
  `disable_default_mouse_bindings`,
  `hide_mouse_cursor_when_typing`,
  `pane_focus_follows_mouse`,
  `swallow_mouse_click_on_pane_focus`,
  `swallow_mouse_click_on_window_focus`,
  `bypass_mouse_reporting_modifiers`,
  `enable_scroll_bar`, `min_scroll_bar_height`,
  `enable_tab_bar`, `hide_tab_bar_if_only_one_tab`, `unzoom_on_switch_pane`,
  `use_fancy_tab_bar`, `tab_bar_at_bottom`,
  `tab_and_split_indices_are_zero_based`, `mouse_wheel_scrolls_tabs`,
  `switch_to_last_active_tab_when_closing_tab`,
  `quit_when_all_windows_are_closed`,
  `window_close_confirmation`,
  `exit_behavior`,
  `clean_exit_codes`,
  `exit_behavior_messaging`,
  `skip_close_confirmation_for_processes_named`,
  `show_close_tab_button_in_tabs`,
  `show_new_tab_button_in_tab_bar`, `show_tab_index_in_tab_bar`, and
  `show_tabs_in_tab_bar`, updates effective config snapshots, and
  emits `window-config-reloaded` on every set. `automatically_reload_config`,
  `check_for_updates`, `check_for_updates_interval_seconds`,
  `show_update_window`, `native_macos_fullscreen_mode`,
  `macos_fullscreen_extend_behind_notch`,
  `use_resize_increments`, `debug_key_events`, and
  `log_unknown_escape_sequences` are retained in effective config snapshots.
  `warn_about_missing_glyphs` is retained with WezTerm's default `true`.
  Unknown ESC/CSI sequences are recorded by the terminal runtime and emitted
  as native stderr warnings when `log_unknown_escape_sequences` is enabled.
  Native key events are emitted as stderr `INFO key_event` diagnostics when
  `debug_key_events` is enabled. Missing glyph codepoints detected in rendered
  cells are emitted once per native window as stderr `CONFIG ERROR missing
  glyph ...` diagnostics when `warn_about_missing_glyphs` is enabled.
  `use_resize_increments` applies current cell-size resize hints on
  X11/Wayland/macOS-capable builds and remains a WezTerm-style no-op on
  unsupported platforms. `native_macos_fullscreen_mode` is retained with
  WezTerm's default `false`; on macOS, `ToggleFullScreen` uses simple fullscreen
  unless this option is `true`, in which case it uses native macOS fullscreen.
  `macos_fullscreen_extend_behind_notch` is retained with WezTerm's default
  `false` and only affects the macOS simple-fullscreen request path.
  Bounded-static event wiring plus command and focus hooks are implemented. Full
  WezTerm-style configuration error window UI,
  arbitrary Lua config execution, dependency/helper-file watching, Lua
  `window:set_config_overrides` wiring, and broader registry coverage remain
  future parity work.
- Implemented in v1: opening the command palette dispatches a typed native
  `augment-command-palette` hook with the window id and active pane id. Returned
  entries can add native `WindowCommand` actions to the same fuzzy-filtered
  palette list, and optional entry `doc` text plus known Nerd Font `icon` names
  are rendered alongside the brief label. Bounded-static event callbacks are
  implemented; arbitrary Lua callbacks, full Nerd Font icon catalog coverage, and
  full action-value parity remain future work.
- Implemented in v1: native tab bar title rendering now passes the computed
  default title, tab id, active pane id, tab index, tab count, active-tab pane
  count, active state, and last-active state through a typed `format-tab-title`
  hook. Returning a string overrides the displayed tab title; returning `None`
  falls back to the default. The event also carries tab-bar hover state and
  `max_width`, and is dispatched in WezTerm-style two passes: first with
  `hover=false` and the WezTerm-default 16-cell `tab_max_width`, then with the
  computed hover state and an available-space title width. Native
  Text/Foreground/Background format items style the title segment, Text items consume embedded SGR
  presentation escapes including blink/inverse/conceal/strikethrough/overline
  while layout uses only their visible text, and
  ResetAttributes restores the tab segment style.
  Native Intensity Normal/Bold/Half toggles tab-title bold/faint rendering,
  native Italic true/false toggles tab-title italic rendering, and native
  Underline None/Single/Double/Curly/Dotted/Dashed maps to tab-title underline
  style. The typed event also carries TabInformation/PaneInformation-style
  snapshots with window id/title, all tabs in the window, explicit tab title,
  the current tab's active pane and pane entries, plus the active tab's pane
  entries for the top-level `panes` parameter. Pane snapshots include geometry,
  titles, foreground process name, current working directory, unseen-output
  state, local domain name, tty name when known, user vars, and progress. The
  typed event carries an effective config snapshot for implemented window
  options including `dpi`, `tab_max_width`, `status_update_interval`,
  `max_fps`, `animation_fps`, `cursor_blink_rate`, `cursor_blink_ease_in`, `cursor_blink_ease_out`,
  `text_blink_rate`, `text_blink_rate_rapid`, `text_blink_ease_in`,
  `text_blink_ease_out`, `text_blink_rapid_ease_in`,
  `text_blink_rapid_ease_out`,
  `font_size`, `cell_width`, `cell_widths`, `line_height`, `font_antialias`, `font_hinting`, `font_rasterizer`, `font_colr_rasterizer`, `font_shaper`, `font_dirs`, `font_locator`, `use_cap_height_to_scale_fallback_fonts`, `ignore_svg_fonts`, `sort_fallback_fonts_by_coverage`, `search_font_dirs_for_fallback`, `custom_block_glyphs`, `anti_alias_custom_block_glyphs`, `allow_square_glyphs_to_overflow_width`, `freetype_load_target`, `freetype_render_target`, `freetype_load_flags`, `freetype_interpreter_version`, `freetype_pcf_long_family_names`, `display_pixel_geometry`, `dpi`, `dpi_by_screen`, `foreground_text_hsb`, `bold_brightens_ansi_colors`, `text_background_opacity`, `window_background_opacity`, `window_background_gradient`, `kde_window_background_blur`, `macos_window_background_blur`, `win32_system_backdrop`, `win32_acrylic_accent_color`, `window_decorations`, `integrated_title_buttons`, `integrated_title_button_alignment`, `integrated_title_button_color`, `integrated_title_button_style`, `default_cursor_style`, `cursor_thickness`, `underline_thickness`, `underline_position`, `strikethrough_position`, `force_reverse_video_cursor`, `window_content_alignment`,
  `initial_cols`, `initial_rows`, `adjust_window_size_when_changing_font_size`,
  `inactive_pane_hsb`,
  `command_palette_rows`, `command_palette_font_size`,
  `command_palette_bg_color`, `command_palette_fg_color`,
  `char_select_font_size`, `char_select_bg_color`, `char_select_fg_color`,
  `pane_select_font_size`, `pane_select_bg_color`, `pane_select_fg_color`,
  `launcher_alphabet`, `quick_select_alphabet`, `quick_select_patterns`,
  `disable_default_quick_select_patterns`, `quick_select_remove_styling`,
  `hyperlink_rules`, `selection_word_boundary`, `term`, `enq_answerback`,
  `audible_bell`, `visual_bell`, `colors`, `color_scheme`, `color_schemes`, `color_scheme_dirs`, `resolved_palette`, `foreground_color`, `background_color`, `ansi_palette`, `indexed_palette`, `selection_fg_color`, `selection_bg_color`, `cursor_bg_color`, `cursor_border_color`, `cursor_fg_color`, `compose_cursor_color`, `visual_bell_color`, `notification_handling`, `default_prog`,
  `default_domain`, `default_workspace`, `prefer_to_spawn_tabs`, `automatically_reload_config`, `check_for_updates`, `check_for_updates_interval_seconds`, `show_update_window`, `native_macos_fullscreen_mode`, `macos_fullscreen_extend_behind_notch`, `use_resize_increments`, `debug_key_events`, `log_unknown_escape_sequences`, `warn_about_missing_glyphs`, `default_cwd`, `default_ssh_auth_sock`, `default_gui_startup_args`, `default_mux_server_domain`, `daemon_options`, `exec_domains`, `wsl_domains`, `unix_domains`, `ssh_domains`, `tls_servers`, `tls_clients`, `serial_ports`, `mux_enable_ssh_agent`, `ssh_backend`, `ratelimit_mux_line_prefetches_per_second`, `mux_output_parser_buffer_size`, `mux_output_parser_coalesce_delay_ms`, `mux_env_remove`, `tiling_desktop_environments`, `periodic_stat_logging`, `ulimit_nofile`, `ulimit_nproc`, `detect_password_input`, `set_environment_variables`, `launch_menu`, `leader`, `keys`, `key_tables`, `mouse_bindings`,
  `scroll_to_bottom_on_input`, `alternate_buffer_wheel_scroll_speed`,
  `canonicalize_pasted_newlines`,
  `quote_dropped_files`,
  `disable_default_key_bindings`,
  `disable_default_mouse_bindings`,
  `hide_mouse_cursor_when_typing`, `detect_password_input`,
  `pane_focus_follows_mouse`,
  `swallow_mouse_click_on_pane_focus`,
  `swallow_mouse_click_on_window_focus`,
  `bypass_mouse_reporting_modifiers`,
  `enable_scroll_bar`, `min_scroll_bar_height`,
  `enable_tab_bar`, `hide_tab_bar_if_only_one_tab`, `unzoom_on_switch_pane`,
  `use_fancy_tab_bar`, `tab_bar_at_bottom`,
  `tab_and_split_indices_are_zero_based`, `mouse_wheel_scrolls_tabs`,
  `switch_to_last_active_tab_when_closing_tab`,
  `quit_when_all_windows_are_closed`,
  `window_close_confirmation`,
  `exit_behavior`,
  `clean_exit_codes`,
  `exit_behavior_messaging`,
  `skip_close_confirmation_for_processes_named`,
  `show_close_tab_button_in_tabs`,
  `show_new_tab_button_in_tab_bar`, `show_tab_index_in_tab_bar`, and
  `show_tabs_in_tab_bar`. Static
  `wezterm.on('format-tab-title', function(...) return ... end)` string,
  inline/callback-local/top-level static FormatItem table returns, and
  `wezterm.format` returns through static aliases now map onto the same
  tab-title override path; arbitrary Lua callbacks and the full Lua config
  object remain future parity work.
- Implemented in v1: native window title recomputation now passes the computed
  default title, active tab id, active pane id, tab count, and active-tab pane
  count plus the active key-table stack top through a typed
  `format-window-title` hook, along with
  TabInformation/PaneInformation-style snapshots for the active tab, active
  pane, all tabs in the window, and panes in the active tab. Returning a string
  overrides the title; returning `None` falls back to the default. The typed
  event carries the same effective config snapshot. Static
  `wezterm.on('format-window-title', function(...) return 'title' end)` string
  returns and top-level static `local <alias> = wezterm.on` event aliases now
  map onto the same title override path; arbitrary Lua callback execution and
  the full Lua config object remain future parity work.
- Implemented in v1: native window `about_to_wait` dispatches typed
  `update-status` and deprecated `update-right-status` hooks with the window id
  and active pane id, scheduled by a WezTerm-style 1000ms
  `status_update_interval` default and immediately on focus changes.
  The handlers can update stored left and right status strings; the native tab
  bar renders left status after the workspace label, consumes SGR presentation
  escapes including blink/inverse/conceal/strikethrough/overline plus WezTerm
  underline style variants and
  ANSI/indexed/RGB foreground/background/underline color escapes in status
  strings, computes status layout from visible text, and right-aligns right
  status at the window edge, clipping over-wide right status from the left.
  Native `set_left_status` and `set_right_status` methods update the same
  tab-bar state directly. Lua-configurable `status_update_interval` parses into
  the same native update interval path, and a static
  `wezterm.on('update-status', function(window, pane) ... end)` and deprecated
  `update-right-status` static subsets map literal
  `window:set_left_status(...)` and `window:set_right_status(...)` calls plus
  the documented `window:set_right_status(window:active_workspace())` and
  `window:set_right_status(tostring(window:active_workspace()))` shapes,
  static string concatenations with `window:active_workspace()` or
  `tostring(window:active_workspace())`, callback-local aliases such as
  `local ws = window:active_workspace()` feeding static concatenations, plus
  static string concatenations
  with `window:window_id()`,
  `window:active_tab():tab_id()`/`get_title()`,
  `window:active_pane():pane_id()`/`get_title()`/`get_domain_name()`/
  `get_current_working_dir()`/`get_foreground_process_name()`/
  `get_tty_name()`, callback-local object aliases such as
  `local tab = window:active_tab()` or
  `local pane = window:active_pane()` feeding those zero-argument methods,
  callback-local method-result variables such as `local title = tab:get_title()`,
  dynamic variable fallbacks such as `title or ''` or `title or fallback`
  where `fallback` is a callback-local or top-level static string variable,
  variable fallback concat segments such as `(title or '')` or
  `(title or fallback)`, and direct dynamic method
  fallbacks such as `window:active_tab():get_title() or ''` or
  `tab:get_title() or ''`, including parenthesized fallback concat segments, or callback
  `pane:pane_id()`/`pane:get_title()`/`pane:get_domain_name()`/
  `pane:get_current_working_dir()`/`pane:get_foreground_process_name()`/
  `pane:get_tty_name()`
  plus static `pane:get_cursor_position()` or
  `window:active_pane():get_cursor_position()` field concatenations for `x`,
  `y`, `shape`, and `visibility`
  plus static direct or callback-local-aliased `pane:get_user_vars()` or
  `window:active_pane():get_user_vars()` dot-field, static string-key, or
  callback-local/top-level static string-key-variable concatenations for stored
  user-var names, including static `or` fallbacks with optional `tostring(...)`
  wrapping for direct reads and callback-local variables assigned from those
  user-var reads, including `tostring(...)` wrapping when concatenated
  plus static `pane:get_progress()` or
  `window:active_pane():get_progress()` conditional status branches for
  `Percentage`, `Error`, and `Indeterminate`
  plus static `pane:get_dimensions()` or
  `window:active_pane():get_dimensions()` field concatenations for `cols`,
  `viewport_rows`, `scrollback_rows`, `physical_top`, and `scrollback_top`
  plus static `pane:is_alt_screen_active()` or
  `window:active_pane():is_alt_screen_active()` status branches with static
  active/inactive strings
  plus static `pane:has_unseen_output()` or
  `window:active_pane():has_unseen_output()` status branches with static
  seen/unseen strings from active-pane unseen-output metadata
  plus static `window:get_dimensions()` field concatenations for
  `pixel_width`, `pixel_height`, `dpi`, and `is_full_screen`
  plus static `window:effective_config().font_size`,
  `window:effective_config().default_workspace`,
  `window:effective_config().default_prog[n]`,
  `window:effective_config().default_gui_startup_args[n]`,
  `window:effective_config().default_cwd`,
  `window:effective_config().term`,
  `window:effective_config().initial_cols`,
  `window:effective_config().initial_rows`,
  `window:effective_config().scrollback_lines`,
  `window:effective_config().exit_behavior`,
  `window:effective_config().exit_behavior_messaging`,
  `window:effective_config().set_environment_variables.NAME`,
  `window:effective_config().set_environment_variables['NAME']`,
  `window:effective_config().default_domain`,
  `window:effective_config().prefer_to_spawn_tabs`,
  `window:effective_config().ssh_backend`,
  `window:effective_config().status_update_interval`,
  `window:effective_config().tab_max_width`,
  `window:effective_config().dpi`,
  `window:effective_config().dpi_by_screen.NAME`,
  `window:effective_config().dpi_by_screen['SCREEN-NAME']`,
  `window:effective_config().color_scheme`,
  `window:effective_config().foreground_color`,
  `window:effective_config().background_color`,
  `window:effective_config().resolved_palette.foreground`,
  `window:effective_config().resolved_palette.background`,
  `window:effective_config().resolved_palette.cursor_bg`,
  `window:effective_config().resolved_palette.cursor_fg`,
  `window:effective_config().resolved_palette.cursor_border`,
  `window:effective_config().resolved_palette.selection_fg`,
  `window:effective_config().resolved_palette.selection_bg`,
  `window:effective_config().resolved_palette.compose_cursor`,
  `window:effective_config().resolved_palette.visual_bell`,
  `window:effective_config().resolved_palette.ansi[n]`,
  `window:effective_config().resolved_palette.brights[n]`,
  `window:effective_config().resolved_palette.indexed[n]`,
  `window:effective_config().max_fps`,
  `window:effective_config().animation_fps`,
  `window:effective_config().front_end`,
  `window:effective_config().webgpu_power_preference`,
  `window:effective_config().webgpu_force_fallback_adapter`,
  `window:effective_config().webgpu_preferred_adapter.backend`,
  `window:effective_config().webgpu_preferred_adapter.device`,
  `window:effective_config().webgpu_preferred_adapter.device_type`,
  `window:effective_config().webgpu_preferred_adapter.driver`,
  `window:effective_config().webgpu_preferred_adapter.driver_info`,
  `window:effective_config().webgpu_preferred_adapter.name`,
  `window:effective_config().webgpu_preferred_adapter.vendor`,
  `window:effective_config().prefer_egl`,
  `window:effective_config().enable_wayland`,
  `window:effective_config().enable_zwlr_output_manager`,
  `window:effective_config().use_box_model_render`,
  `window:effective_config().experimental_pixel_positioning`,
  `window:effective_config().cell_width`,
  `window:effective_config().cell_widths[n].first`,
  `window:effective_config().cell_widths[n].last`,
  `window:effective_config().cell_widths[n].width`,
  `window:effective_config().line_height`,
  `window:effective_config().font_antialias`,
  `window:effective_config().font_hinting`,
  `window:effective_config().font_rasterizer`,
  `window:effective_config().font_colr_rasterizer`,
  `window:effective_config().font_shaper`,
  `window:effective_config().harfbuzz_features[n]`,
  `window:effective_config().font_dirs[n]`,
  `window:effective_config().font_locator`,
  `window:effective_config().use_cap_height_to_scale_fallback_fonts`,
  `window:effective_config().sort_fallback_fonts_by_coverage`,
  `window:effective_config().search_font_dirs_for_fallback`,
  `window:effective_config().freetype_load_target`,
  `window:effective_config().freetype_render_target`,
  `window:effective_config().freetype_load_flags`,
  `window:effective_config().freetype_interpreter_version`,
  `window:effective_config().freetype_pcf_long_family_names`,
  `window:effective_config().bold_brightens_ansi_colors`,
  `window:effective_config().allow_square_glyphs_to_overflow_width`,
  `window:effective_config().display_pixel_geometry`,
  `window:effective_config().text_background_opacity`,
  `window:effective_config().window_background_opacity`,
  `window:effective_config().foreground_text_hsb.hue`,
  `window:effective_config().foreground_text_hsb.saturation`,
  `window:effective_config().foreground_text_hsb.brightness`,
  `window:effective_config().inactive_pane_hsb.hue`,
  `window:effective_config().inactive_pane_hsb.saturation`,
  `window:effective_config().inactive_pane_hsb.brightness`,
  `window:effective_config().shape_cache_size`,
  `window:effective_config().line_state_cache_size`,
  `window:effective_config().line_quad_cache_size`,
  `window:effective_config().line_to_ele_shape_cache_size`,
  `window:effective_config().glyph_cache_image_cache_size`,
  `window:effective_config().cursor_blink_rate`,
  `window:effective_config().cursor_blink_ease_in`,
  `window:effective_config().cursor_blink_ease_out`,
  `window:effective_config().text_blink_rate`,
  `window:effective_config().text_blink_rate_rapid`,
  `window:effective_config().text_blink_ease_in`,
  `window:effective_config().text_blink_ease_out`,
  `window:effective_config().text_blink_rapid_ease_in`,
  `window:effective_config().text_blink_rapid_ease_out`,
  `window:effective_config().cursor_thickness`,
  `window:effective_config().underline_thickness`,
  `window:effective_config().underline_position`,
  `window:effective_config().strikethrough_position`,
  `window:effective_config().hide_mouse_cursor_when_typing`,
  `window:effective_config().default_mux_server_domain`,
  `window:effective_config().ratelimit_mux_line_prefetches_per_second`,
  `window:effective_config().mux_output_parser_buffer_size`,
  `window:effective_config().mux_output_parser_coalesce_delay_ms`,
  `window:effective_config().mux_env_remove[n]`,
  `window:effective_config().periodic_stat_logging`,
  `window:effective_config().ulimit_nofile`,
  `window:effective_config().ulimit_nproc`,
  `window:effective_config().scroll_to_bottom_on_input`,
  `window:effective_config().use_ime`,
  `window:effective_config().xim_im_name`,
  `window:effective_config().ime_preedit_rendering`,
  `window:effective_config().macos_forward_to_ime_modifier_mask`,
  `window:effective_config().notification_handling`,
  `window:effective_config().use_dead_keys`,
  `window:effective_config().audible_bell`,
  `window:effective_config().visual_bell.fade_in_duration_ms`,
  `window:effective_config().visual_bell.fade_out_duration_ms`,
  `window:effective_config().visual_bell.fade_in_function`,
  `window:effective_config().visual_bell.fade_out_function`,
  `window:effective_config().visual_bell.target`,
  `window:effective_config().automatically_reload_config`,
  `window:effective_config().check_for_updates`,
  `window:effective_config().show_update_window`,
  `window:effective_config().check_for_updates_interval_seconds`,
  `window:effective_config().enable_kitty_graphics`,
  `window:effective_config().enable_checksum_rectangular_area`,
  `window:effective_config().enable_title_reporting`,
  `window:effective_config().enable_csi_u_key_encoding`,
  `window:effective_config().enable_kitty_keyboard`,
  `window:effective_config().allow_download_protocols`,
  `window:effective_config().xcursor_theme`,
  `window:effective_config().xcursor_size`,
  `window:effective_config().palette_max_key_assigments_for_action`,
  `window:effective_config().allow_win32_input_mode`,
  `window:effective_config().treat_left_ctrlalt_as_altgr`,
  `window:effective_config().send_composed_key_when_left_alt_is_pressed`,
  `window:effective_config().send_composed_key_when_right_alt_is_pressed`,
  `window:effective_config().treat_east_asian_ambiguous_width_as_wide`,
  `window:effective_config().normalize_output_to_unicode_nfc`,
  `window:effective_config().unicode_version`,
  `window:effective_config().window_close_confirmation`,
  `window:effective_config().enable_tab_bar`,
  `window:effective_config().use_fancy_tab_bar`,
  `window:effective_config().tab_bar_at_bottom`,
  `window:effective_config().mouse_wheel_scrolls_tabs`,
  `window:effective_config().switch_to_last_active_tab_when_closing_tab`,
  `window:effective_config().warn_about_missing_glyphs`,
  `window:effective_config().pane_focus_follows_mouse`,
  `window:effective_config().swallow_mouse_click_on_pane_focus`,
  `window:effective_config().swallow_mouse_click_on_window_focus`,
  `window:effective_config().bypass_mouse_reporting_modifiers`,
  `window:effective_config().unzoom_on_switch_pane`,
  `window:effective_config().quit_when_all_windows_are_closed`,
  `window:effective_config().default_cursor_style`,
  `window:effective_config().force_reverse_video_cursor`,
  `window:effective_config().reverse_video_cursor_min_contrast`,
  `window:effective_config().text_min_contrast_ratio`,
  `window:effective_config().command_palette_rows`,
  `window:effective_config().command_palette_font_size`,
  `window:effective_config().command_palette_bg_color`,
  `window:effective_config().command_palette_fg_color`,
  `window:effective_config().char_select_font_size`,
  `window:effective_config().char_select_bg_color`,
  `window:effective_config().char_select_fg_color`,
  `window:effective_config().pane_select_font_size`,
  `window:effective_config().pane_select_bg_color`,
  `window:effective_config().pane_select_fg_color`,
  `window:effective_config().launcher_alphabet`,
  `window:effective_config().quick_select_alphabet`,
  `window:effective_config().quick_select_patterns[n]`,
  `window:effective_config().disable_default_quick_select_patterns`,
  `window:effective_config().quick_select_remove_styling`,
  `window:effective_config().canonicalize_pasted_newlines`,
  `window:effective_config().quote_dropped_files`,
  `window:effective_config().disable_default_key_bindings`,
  `window:effective_config().disable_default_mouse_bindings`,
  `window:effective_config().debug_key_events`,
  `window:effective_config().key_map_preference`,
  `window:effective_config().ui_key_cap_rendering`,
  `window:effective_config().swap_backspace_and_delete`,
  `window:effective_config().log_unknown_escape_sequences`,
  `window:effective_config().default_ssh_auth_sock`,
  `window:effective_config().mux_enable_ssh_agent`,
  `window:effective_config().detect_password_input`,
  `window:effective_config().native_macos_fullscreen_mode`,
  `window:effective_config().macos_fullscreen_extend_behind_notch`,
  `window:effective_config().selection_word_boundary`,
  `window:effective_config().enq_answerback`,
  `window:effective_config().adjust_window_size_when_changing_font_size`,
  `window:effective_config().tiling_desktop_environments[1]`,
  `window:effective_config().use_resize_increments`,
  `window:effective_config().alternate_buffer_wheel_scroll_speed`,
  `window:effective_config().ignore_svg_fonts`,
  `window:effective_config().custom_block_glyphs`,
  `window:effective_config().anti_alias_custom_block_glyphs`,
  `window:effective_config().window_padding.left`,
  `window:effective_config().window_padding.right`,
  `window:effective_config().window_padding.top`,
  `window:effective_config().window_padding.bottom`,
  `window:effective_config().window_content_alignment.horizontal`,
  `window:effective_config().window_content_alignment.vertical`,
  `window:effective_config().kde_window_background_blur`,
  `window:effective_config().macos_window_background_blur`,
  `window:effective_config().win32_system_backdrop`,
  `window:effective_config().win32_acrylic_accent_color`,
  `window:effective_config().window_decorations`,
  `window:effective_config().integrated_title_buttons[n]`,
  `window:effective_config().integrated_title_button_alignment`,
  `window:effective_config().integrated_title_button_color`,
  `window:effective_config().integrated_title_button_style`,
  `window:effective_config().bidi_enabled`,
  `window:effective_config().bidi_direction`,
  `window:effective_config().color_scheme_dirs[n]`,
  `window:effective_config().clean_exit_codes[n]`,
  `window:effective_config().skip_close_confirmation_for_processes_named[1]`,
  `window:effective_config().show_close_tab_button_in_tabs`,
  `window:effective_config().show_new_tab_button_in_tab_bar`,
  `window:effective_config().show_tab_index_in_tab_bar`,
  `window:effective_config().show_tabs_in_tab_bar`,
  `window:effective_config().tab_and_split_indices_are_zero_based`,
  `window:effective_config().hide_tab_bar_if_only_one_tab`,
  `window:effective_config().enable_scroll_bar`,
  `window:effective_config().min_scroll_bar_height`,
  `window:effective_config().launch_menu[n].label`,
  `window:effective_config().launch_menu[n].args[k]`,
  `window:effective_config().launch_menu[n].cwd`,
  `window:effective_config().launch_menu[n].domain`, and
  `window:effective_config().launch_menu[n].set_environment_variables.NAME` field concatenations for current
  effective config values, with default-program launch-menu items exposing the
  inherited or configured default command through `launch_menu[n].args[k]`;
  direct or aliased top-level, sub-object, and nested effective-config fields
  also accept literal, callback-local, or top-level static string/numeric-variable
  bracket access such as
  `window:effective_config()['default_prog'][1]`,
  `window:effective_config().visual_bell['target']`, or
  `local env_key = 'PROJECT_MODE'; local env = window:effective_config().set_environment_variables; env[env_key]`,
  plus launch-menu environment aliases such as
  `local item_index = 1; local env = window:effective_config().launch_menu[item_index].set_environment_variables; env[env_key]`,
  plus the documented `window:active_key_table()` key-table status shape with
  its static prefix and empty-stack fallback, plus the documented
  `window:leader_is_active()` status shape with static active/inactive strings,
  plus the documented `window:is_focused()` status shape with static
  focused/unfocused strings,
  plus the documented `window:keyboard_modifiers()` status shape with current
  modifier text and empty LED status, plus the documented
  `window:composition_status()` status shape with Builtin IME preedit or
  current dead-key text and static prefix/fallback, plus inline,
  static-table-variable, or static-alias `wezterm.format`
  Text/Foreground/Background/ResetAttributes and Attribute
  Intensity/Italic/Underline item composition onto that status state, with
  static item tables resolving from callback-local or top-level scope and
  callback-local `table.insert` or `items[#items + 1] = ...` appends whose
  table or string items can resolve from callback-local or top-level static
  variables, including `table.insert(elements, accent)` or
  `table.insert(elements, reset)`, plus callback-local aliases to those
  top-level static item tables or strings.
  Arbitrary Lua status callbacks, dynamic `wezterm.format` construction, and
  remaining window status API wiring and real keyboard LED tracking remain
  future parity work.
- Implemented in v1: the tab bar `+` button dispatches a typed
  `new-tab-button-click` hook with the window id, active pane id, and mouse
  button for Left/Right/Middle clicks. Left click carries the default `NewTab`
  action in the event payload, while Right/Middle clicks have no default action;
  returning `false` suppresses any default action. The documented static
  callback shape that calls `window:perform_action(default_action, pane)` before
  returning `false` runs the native default action exactly once. Lua event
  wiring remains future parity work.
- Implemented in v1: ctrl-clicked OSC 8 hyperlinks dispatch a typed open-uri
  hook with the window id, active pane id, and URI before the default opener
  runs. Returning `false` suppresses the default opener. The command palette
  also exposes WezTerm-style `CompleteSelection`, `OpenLinkAtMouseCursor`, and
  `CompleteSelectionOrOpenLinkAtMouseCursor` behavior: active mouse selections
  are completed into ClipboardAndPrimarySelection, otherwise the OSC 8 link
  under the mouse is opened through the same open-uri hook. Structured
  `completeselection`, `openlinkatmousecursor`, and
  `completeselectionoropenlinkatmousecursor` action-name queries resolve to the
  same native behavior. Native `OpenUri(uri)` payloads and structured
  `open uri <uri>`/`openuri <uri>` queries dispatch an explicit URI through the
  same open-uri hook and default opener, including WezTerm-style
  `wezterm.action.OpenUri('<uri>')` and
  `wezterm.action { OpenUri = '<uri>' }` Lua action query forms. Native
  `CompleteSelectionTo(destination)` and
  `CompleteSelectionOrOpenLinkAtMouseCursorTo(destination)` payloads complete
  active selections into a specific implemented copy destination, with quoted or
  unquoted command-palette destination queries for both
  `complete selection to <destination>` / `completeselectionto <destination>`
  and `complete selection open link to <destination>` /
  `completeselectionoropenlinkatmousecursorto <destination>`. Static
  `config.keys` action calls resolve top-level string variables for
  `CompleteSelection` and `CompleteSelectionOrOpenLinkAtMouseCursor`
  destinations. Static WezTerm-style `config.hyperlink_rules` tables parse
  `regex`, `format`, and `highlight` fields, `table.insert` appends custom
  rules, and direct, config-assignment static-alias, return-table static-alias,
  or returned-config-variable static-alias `wezterm.default_hyperlink_rules()`
  values are accepted as explicit defaults and preserve default rules before
  appended custom rules while positioned inserts, indexed replacements, and
  indexed field mutations on the static rules variable, plus direct
  config-field inserts, replacements, and indexed field mutations, keep their
  matching priority. Static `wezterm.on('open-uri', ...)` callbacks with
  literal returns can suppress the default opener, and the documented
  `uri:find 'mailto:'` branch can spawn `SpawnCommandInNewWindow { args = {
  'mutt', recipient } }`, deriving `recipient` from `uri:sub(match_end + 1)`,
  before returning `false`. Bounded-static `wezterm.on` event callbacks are
  implemented; arbitrary Lua event execution remains future parity work.
- Implemented in v1: command-palette Reset Terminal injects RIS (`ESC c`) into
  the active pane output side, matching WezTerm-style `ResetTerminal`.
  Action-name `resetterminal` queries dispatch the same command.
- Implemented in v1: command-palette scrollback navigation covers
  WezTerm-style Scroll To Top/Bottom, Scroll By Page Up/Down, and Scroll By
  Line Up/Down actions for the active viewport, and native `ScrollByPage`,
  `ScrollByLine`, `ScrollToPrompt`, and `ScrollByCurrentEventWheelDelta`
  payloads route signed WezTerm amounts and the current vertical mouse-wheel
  event delta through the same viewport and OSC 133 prompt navigation helpers;
  action-name `scrolltotop`, `scrolltobottom`, `scrollpageup`,
  `scrollpagedown`, `scrolllineup`, `scrolllinedown`,
  `scrollbycurrenteventwheeldelta`, `scrolltopreviousprompt`, and
  `scrolltonextprompt` queries dispatch the corresponding no-argument
  commands;
  structured command-palette queries accept both spaced and action-name forms
  (`scroll by page <amount>`/`scrollbypage <amount>`,
  `scroll by line <amount>`/`scrollbyline <amount>`, and
  `scroll to prompt <amount>`/`scrolltoprompt <amount>`). Static
  `config.keys` action calls resolve top-level signed integer variables for
  `ScrollByLine`/`ScrollToPrompt` and signed number variables for
  `ScrollByPage`.
- Implemented in v1: WezTerm-style `Shift+PageUp` and `Shift+PageDown`
  shortcuts expose `ScrollByPage(-1)` and `ScrollByPage(1)` in native
  `KEY_ASSIGNMENTS` and route normal keyboard input to page-wise scrollback
  movement while unmodified PageUp/PageDown remain available to the active PTY
  application.
- Implemented in v1: native `scroll_to_bottom_on_input` defaults to true and
  resets the active scrollback viewport to the bottom when terminal input is
  written; setting it false preserves the current scrollback viewport on input.
- Implemented in v1: native `alternate_buffer_wheel_scroll_speed` defaults to
  WezTerm's `3`; when the active pane is in the alternate screen and mouse
  reporting is disabled, vertical wheel input writes repeated Up/Down arrow-key
  sequences to the PTY instead of moving scrollback.
- Implemented in v1: native `enable_scroll_bar` defaults to false, while
  `min_scroll_bar_height` retains WezTerm's `"0.5cell"` default in effective
  config and applies px, point, cell, and percent overrides to scrollbar
  rendering and hit-testing when the scrollbar is enabled.
- Implemented in v1: native `scrollback_lines` defaults to WezTerm's `3500`
  retained lines. Config overrides update active and inactive pane runtimes,
  apply to new pane/window runtimes, and prune retained history immediately when
  the limit is reduced.
- Implemented in v1: command-palette Scroll To Prompt Previous and Scroll To
  Prompt Next use OSC 133 `A`/`N`/`P` prompt row markers to jump the active
  viewport between retained prompt rows.
- Implemented in v1: `rterm-terminal` records OSC 133 Prompt/Input/Output
  semantic types on retained cells, exposes semantic zone queries over
  scrollback plus the visible grid, and handles `I` input markers as
  line-scoped input until cursor movement.
- Implemented in v1: `rterm-terminal` records WezTerm shell-integration OSC 133
  `D` command-finished metadata with retained row, exit status, and `aid`.
- Implemented in v1: `rterm-terminal` exposes retained row/column text extraction
  for semantic zones and general regions, unwrapping soft-wrapped physical rows
  into logical-line text. WezTerm-style Lua pane APIs and configurable key-table
  bindings remain future parity work.
- Implemented in v1: `rterm-terminal` records OSC 7 and iTerm2
  `OSC 1337;CurrentDir` current working directory metadata. `rterm-app` syncs
  that metadata into per-pane launch models for active and inactive panes,
  falling back to local session process tree cwd when the PTY backend exposes a
  pid and preferring child processes over the session root, so new tabs/splits
  inherit it and local PTY spawns receive a decoded filesystem cwd.
- Implemented in v1: `rterm-terminal` base64-decodes iTerm2/WezTerm
  `OSC 1337;SetUserVar` metadata into terminal user vars. `rterm-app` syncs
  those values into per-pane app-shell metadata for active and inactive pane
  runtimes and emits a typed native-window user-var change hook when a stored
  pane value changes, carrying the window id, pane id, name, and value. Static
  `update-status` callbacks can read active-pane direct or callback-local-aliased
  `pane:get_user_vars()` or `window:active_pane():get_user_vars()` dot fields,
  static string keys, and callback-local/top-level static string-key variables
  from the same stored metadata, including static `or` fallbacks with optional
  `tostring(...)` wrapping. Static `user-var-changed` callbacks can directly or
  through callback-local status variables update left or right status text from
  static strings, callback-local/top-level static string variables,
  `window:window_id()`, `pane:pane_id()`, and the event `name` and `value`
  parameters including direct use or callback-local aliases with static `or`
  fallbacks from literal, callback-local, or top-level static string values for those
  dynamic values with optional `tostring(...)` wrapping, plus callback-local `pane:get_user_vars()` reads with
  static `or` fallbacks for the triggering pane and
  `window:active_pane():get_user_vars()` reads for the active pane, including
  callback-local variables assigned from those user-var reads.
- Implemented in v1: `rterm-terminal` base64-decodes iTerm2
  `OSC 1337;SetBadgeFormat` metadata into terminal badge format state.
  `rterm-app` syncs that value into per-pane app-shell metadata for active and
  inactive pane runtimes, interpolates `\(user.NAME)` badge variables from pane
  user vars, interpolates `\(iterm2.pid)` from the current app process id,
  interpolates `\(iterm2.localhostName)` from the local host name,
  interpolates `\(iterm2.effectiveTheme)` plus
  `\(tab.iterm2.effectiveTheme)`,
  `\(tab.window.iterm2.effectiveTheme)`, and
  `\(tab.window.currentTab.iterm2.effectiveTheme)` as `dark` for the current
  fixed dark native UI,
  interpolates `\(tab.window.id)` from the native window id,
  interpolates `\(tab.window.number)` from the native window number,
  interpolates `\(tab.window.frame)` from the latest native window origin and
  pixel size as `[x, y, width, height]`,
  interpolates `\(tab.window.style)` from current normal/full-screen window
  style,
  interpolates `\(tab.window.isHotkeyWindow)` as `false` until native hotkey
  windows exist,
  interpolates `\(tab.window.titleOverrideFormat)`/
  `\(tab.window.titleOverride)` from the current base window title,
  interpolates `\(tab.window.currentTab.id)`/`\(tab.window.currentTab.title)`/
  `\(tab.window.currentTab.titleOverrideFormat)`/
  `\(tab.window.currentTab.titleOverride)` from the active tab
  id/title/explicit tab title,
  interpolates `\(tab.window.currentTab.currentSession.id)`/
  `\(tab.window.currentTab.currentSession.pid)`/
  `\(tab.window.currentTab.currentSession.jobPid)`/
  `\(tab.window.currentTab.currentSession.tty)`/
  `\(tab.window.currentTab.currentSession.autoName)`/
  `\(tab.window.currentTab.currentSession.autoNameFormat)`/
  `\(tab.window.currentTab.currentSession.name)`/
  `\(tab.window.currentTab.currentSession.presentationName)`/
  `\(tab.window.currentTab.currentSession.jobName)`/
  `\(tab.window.currentTab.currentSession.processTitle)`/
  `\(tab.window.currentTab.currentSession.commandLine)`/
  `\(tab.window.currentTab.currentSession.lastCommand)`/
  `\(tab.window.currentTab.currentSession.homeDirectory)`/
  `\(tab.window.currentTab.currentSession.sshIntegrationLevel)`/
  `\(tab.window.currentTab.currentSession.username)`/
  `\(tab.window.currentTab.currentSession.hostname)`/
  `\(tab.window.currentTab.currentSession.shell)`/
  `\(tab.window.currentTab.currentSession.uname)`/
  `\(tab.window.currentTab.currentSession.path)`/
  `\(tab.window.currentTab.currentSession.profileName)`/
  `\(tab.window.currentTab.currentSession.terminalIconName)`/
  `\(tab.window.currentTab.currentSession.terminalWindowName)`/
  `\(tab.window.currentTab.currentSession.applicationKeypad)`/
  `\(tab.window.currentTab.currentSession.bellCount)`/
  `\(tab.window.currentTab.currentSession.mouseReportingMode)`/
  `\(tab.window.currentTab.currentSession.mouseInfo)`/
  `\(tab.window.currentTab.currentSession.mouseInfo[0/1/2/3/4/5/6])`/
  `\(tab.window.currentTab.currentSession.columns)`/
  `\(tab.window.currentTab.currentSession.rows)`/
  `\(tab.window.currentTab.currentSession.selection)`/
  `\(tab.window.currentTab.currentSession.selectionLength)` from the active
  tab current pane id/process id/PTY name/title/auto-name from OSC 1 icon title
  or profile name/launch program/command line/last OSC 133 shell-integration command/local home directory/SSH-integration
  level/local user/host/shell/uname/path/profile name/OSC 1 icon title/OSC 2
  window title/application-keypad state/bell count/mouse reporting mode/latest
  mouse-info array and indexed values/size/selection,
  interpolates `\(tab.id)`/`\(tab.title)`/`\(tab.titleOverrideFormat)`/
  `\(tab.titleOverride)` from the active tab id/title/explicit tab title,
  interpolates `\(tab.currentSession.id)` from the active tab current pane id,
  interpolates `\(tab.currentSession.pid)`/`\(tab.currentSession.jobPid)`/
  `\(tab.currentSession.tty)` from the active tab current pane process id and
  PTY name,
  interpolates `\(tab.currentSession.autoName)`/
  `\(tab.currentSession.autoNameFormat)`/`\(tab.currentSession.name)`/
  `\(tab.currentSession.presentationName)` from the active tab current pane,
  with auto-name values using the current OSC 1 icon title or profile name and
  name/presentation-name values using the pane title/session name,
  interpolates `\(tab.currentSession.jobName)`/`\(tab.currentSession.processTitle)`/
  `\(tab.currentSession.commandLine)` from the active tab current pane launch
  program and command line, and interpolates
  `\(tab.currentSession.lastCommand)` from the active tab current pane's most
  recent OSC 133 shell-integration input command,
  interpolates `\(tab.currentSession.homeDirectory)`/
  `\(tab.currentSession.sshIntegrationLevel)`/
  `\(tab.currentSession.username)`/`\(tab.currentSession.hostname)`/
  `\(tab.currentSession.shell)`/`\(tab.currentSession.uname)` from the local
  host home directory, native/local SSH-integration level `0`, local user name,
  local host name, local shell, and local OS/architecture description,
  interpolates `\(tab.currentSession.path)` from the active tab current working
  directory,
  interpolates `\(tab.currentSession.profileName)` from the active tab current
  pane profile name,
  interpolates `\(tab.currentSession.terminalIconName)` from the active tab
  current OSC 1 icon title,
  interpolates `\(tab.currentSession.terminalWindowName)` from the active tab
  current OSC 2 window title,
  interpolates `\(tab.currentSession.applicationKeypad)`/
  `\(tab.currentSession.bellCount)`/
  `\(tab.currentSession.mouseReportingMode)`/
  `\(tab.currentSession.mouseInfo)`/
  `\(tab.currentSession.mouseInfo[0/1/2/3/4/5/6])`/
  `\(tab.currentSession.columns)`/`\(tab.currentSession.rows)`/
  `\(tab.currentSession.selection)`/
  `\(tab.currentSession.selectionLength)` from the active tab current pane
  keypad state, retained BEL count, iTerm2-compatible mouse reporting mode,
  latest reported mouse-info array plus x/y/button/click-count/modifier-array/
  side-effects/event-type indices using iTerm2's up/down/drag event-type values
  `0`/`1`/`2`, modifier values Control/Option/Command/Shift as `1`/`2`/`3`/`4`,
  and the drag side-effect bit, rendered pane size, and active selection text/UTF-8 byte length,
  interpolates session id/termid/pid/job-pid/tty/auto-name/name/presentation-name/job-name/process-title/command-line/last-command/home-directory/profile-name/SSH-integration-level/username/hostname/shell/uname/path/title/size/application-keypad,
  bell-count, mouse-reporting, mouse-info array/index, and selection badge variables including
  `\(session.id)`, `\(session.termid)`, `\(session.pid)`,
  `\(session.jobPid)`, `\(session.tty)`, `\(session.autoName)`,
  `\(session.autoNameFormat)`,
  `\(session.presentationName)`, `\(session.jobName)`, `\(session.processTitle)`,
  `\(session.commandLine)`, `\(session.lastCommand)`, `\(session.homeDirectory)`,
  `\(session.profileName)`,
  `\(session.sshIntegrationLevel)`, `\(session.username)`,
  `\(session.hostname)`, `\(session.shell)`, `\(session.uname)`,
  `\(session.applicationKeypad)`, `\(session.bellCount)`,
  `\(session.mouseReportingMode)`, `\(session.mouseInfo)`,
  `\(session.mouseInfo[0/1/2/3/4/5/6])`,
  and `\(session.selectionLength)`, with
  termid using the current window/tab/pane identifiers and injecting the same
  value as `TERM_SESSION_ID` for spawned PTY children, pid and job-pid using the
  live PTY child process id when available, tty using the PTY name when the
  backend exposes one, auto-name and auto-name-format using the current OSC 1
  icon title or loaded profile name, presentation-name using the pane
  title/session title source, job-name and process-title using the
  pane launch program, command-line using launch program plus args,
  home-directory using the local host home directory, profile-name using the
  loaded TOML profile name exported as `RTERM_PROFILE` when present, and SSH
  integration level currently reporting `0` for native/local sessions, username using the local
  host user name, hostname using the local host name, shell using the local host
  shell, uname using the local host OS/architecture description, and mouse-info
  array and indices using the latest app-reported mouse event for that pane with
  iTerm2 up/down/drag event-type values `0`/`1`/`2` and modifier arrays rendered
  as ordered numeric arrays such as `[2, 4]`, then renders
  non-empty badge text as a pane-local top-right overlay.
- Implemented in v1: `rterm-terminal` records iTerm2/WezTerm
  `OSC 1337;File=...` inline image metadata and decoded payload bytes at the
  current retained-history cursor position. `rterm-renderer` carries those image
  items through live, scrollback, and overlaid pane render snapshots, and draws
  PNG, JPEG, and GIF payloads into the RGBA framebuffer with delay-aware
  animated GIF frame selection by elapsed render time, cell/`px` dimensions,
  and damage-region redraw coverage. The same image
  snapshot path now covers the Kitty Graphics Protocol direct `a=T` subset for
  single-block and chunked, uncompressed and zlib-compressed raw RGB/RGBA plus
  encoded image payloads, regular-file `t=f` simple-file transfers with
  optional `O`/`S` file slicing, temporary-file `t=t` transfers with guarded
  `tty-graphics-protocol` temp-file deletion, plus minimal `a=t,i=<id>`
  stored-image transmission, `a=t,I=<number>` terminal-assigned image-number
  uploads with `i`/`I` OK responses, and `a=p` placement by image id or image
  number at the current cursor. Native `enable_kitty_graphics` defaults to true
  and, when disabled, suppresses Kitty graphics query responses, uploads,
  placements, and rendered inline images.
  Basic source rectangles (`x`/`y`/`w`/`h`) are propagated for direct and
  stored-placement image cropping, and `X`/`Y` target pixel offsets shift direct
  and stored placements relative to the placement cell; placements that specify
  only `c` or only `r` derive the other cell axis from the source image or
  source-rectangle aspect ratio. Basic `a=q` support queries return
  `OK`/`EINVAL` for supported direct, regular-file, and temporary-file
  payloads without storing/displaying the queried image, stored-image existence
  queries and stored placements return `OK` or `ENOENT` for present/missing
  image ids or image numbers, Kitty `q=1`/`q=2` response suppression is
  honored, `i`/`I` mutual exclusion is enforced, direct/stored placements
  advance the cursor by the placement cell rectangle unless `C=1` suppresses
  movement, and basic placement ids are tracked so repeated
  `(image id, placement id)` pairs replace old placements.
  Basic `a=d` deletion covers all live viewport visible Kitty placements while
  retaining scrollback placements, image-id placement deletion, image-number
  placement deletion, image-id range deletion,
  `(image id, placement id)` pair deletion, and cursor-cell, explicit-cell,
  visible-column, visible-row, z-index, and cell-plus-z-index deletion. These
  position-oriented deletes leave Unicode-placeholder-derived renders intact;
  the derived render is removed when the underlying placeholder cell is
  overwritten or erased. The
  renderer applies Kitty z-index layer ordering, drawing negative z-index images
  below text, z-index values below `i32::MIN / 2` below non-default cell
  backgrounds, and non-negative z-index images above text in ascending z order,
  with Kitty image id breaking ties for overlapping same-z images.
  Terminal erase display paths remove affected inline-image
  placements for `CSI 2J`, drop scrollback inline images for `CSI 3J`, and
  rebase retained visible image rows after scrollback clearing. Alternate-screen
  `?1049` switches isolate inline-image placements between the main and
  alternate buffers, restoring main placements on exit and discarding alternate
  placements. Scroll operations move inline-image placements with affected text
  rows and drop placements that leave the scrolled region. Basic Sixel DCS `q`
  image payloads with VT340 default palette entries, RGB plus DEC HLS hue
  palette definitions, DCS `P1` macro pixel aspect, DECGRA `Pan`/`Pad` aspect
  override plus `Ph`/`Pv` minimum background dimensions, DCS `P2`
  transparent/opaque background mode, repeat introducers, carriage returns,
  and sixel newlines are normalized into raw RGBA inline images. By default
  and after `?80l`, Sixel output starts at the text cursor and advances below
  the image while preserving the left-edge column; xterm/WezTerm `?8452h`
  moves that post-Sixel cursor to the right edge. When DECSDM `?80h` is set,
  Sixel output starts at the active graphics-page origin and keeps the text
  cursor fixed, matching WezTerm's placement behavior. WezTerm's tmux-control
  `DCS 1000 q` is ignored instead of being classified as Sixel, and supported
  Sixel images draw through the same snapshot path.
  Native window redraws advance elapsed-time GIF frames through the renderer
  animation clock. Kitty shared-memory transfers, remaining richer placement
  controls, broader query responses beyond current direct/local-file payload
  validation and stored-image existence checks, full Sixel protocol coverage,
  and remote sync remain future parity work.
- Implemented in v1: `rterm-app` responds to xterm cell/window size queries and
  WezTerm/iTerm2-compatible `OSC 1337;ReportCellSize` requests using the native
  fixed cell pixel dimensions.
- Implemented in v1: `rterm-app` answers XTGETTCAP `Sync` queries with the
  WezTerm synchronized-output terminfo template for the existing
  `ESC[?2026h/l` path.
- Implemented in v1: XTGETTCAP style/color replies include WezTerm-style
  overline, strikethrough, default-color, palette-reset, `sgr`/`sgr0`,
  standout, and conditional select-color templates in addition to italic,
  underline, underline-color, and true-color templates.
- Implemented in v1: cursor blink mode `?12` is tracked in the terminal core
  and shared mode tracker, DECRQM reports it for ESC/C1 CSI forms, and XTGETTCAP
  exposes WezTerm `civis`/`cnorm`/`cvvis` cursor visibility/blink templates.
  Renderer snapshots carry the terminal cursor blink state, and the pixel
  renderer supports hidden and interpolated-opacity blinking cursors.
  Native window overrides expose WezTerm-style `cursor_blink_rate`, including
  `0` to keep blinking cursors visible, and `cursor_blink_ease_in` /
  `cursor_blink_ease_out` for native Constant/Linear/Ease-style opacity
  interpolation. Native window overrides also expose WezTerm-style
  `text_blink_rate`, `text_blink_rate_rapid`, `text_blink_ease_in`,
  `text_blink_ease_out`, `text_blink_rapid_ease_in`, and
  `text_blink_rapid_ease_out`; SGR 5 and SGR 6 text blink use independent
  opacity phases and interpolate foreground/decorations toward the rendered
  background. Static WezTerm-style Lua snippets for the cursor/text blink
  easing fields parse string easing names and `{ CubicBezier = { ... } }`
  table easing forms inline or through top-level static variables, including
  trailing comma fields. Native overrides also expose WezTerm-style
  `bold_brightens_ansi_colors`, with the `No`, `BrightAndBold`, and
  `BrightOnly` modes applied to bold ANSI 0-7 foreground colors. Native
  overrides also expose `default_cursor_style` for
  steady/blinking block, underline, and bar cursors plus `cursor_thickness`
  overrides for underline and bar cursor glyphs using px, DPI-scaled pt,
  percent-of-default, and cell-fraction units. Native `underline_thickness`
  applies the same unit forms to terminal text underline decorations and
  horizontal split dividers. Native
  `underline_position` applies signed px, DPI-scaled pt, percent-of-default,
  and cell-fraction units to terminal text underline placement using the
  default underline row as the current baseline approximation. Native
  `strikethrough_position` applies px, DPI-scaled pt, percent-of-default, and
  cell-fraction units to terminal text strikethrough decorations. Static
  WezTerm-style Lua snippets for these decoration dimensions parse string
  dimensions with units plus bare numeric pixel values inline or through
  top-level static number variables.
  `force_reverse_video_cursor` forces native cursor
  fills to use the cursor cell's effective foreground color unless OSC 12 set
  an explicit cursor color, and OSC 112 resets that override. Native
  `reverse_video_cursor_min_contrast` defaults to WezTerm's `2.5` threshold and
  falls back to configured cursor foreground/background colors when the
  reverse-video cursor contrast is too low; `DECSCUSR 0` and full terminal
  reset restore the configured shape default.
- Implemented in v1: native `window_padding` parses WezTerm-style px and
  cell-unit side padding inline or through top-level static table variables,
  with side values also parsed through top-level static number/string
  variables, plus top-level `config[static_name].left`/`right`/`top`/`bottom`
  field mutations where `static_name` resolves to `window_padding`.
  Native `window_content_alignment` parses WezTerm-style static tables, inline
  or through top-level static table variables, for
  horizontal `Left`/`Center`/`Right` and vertical `Top`/`Center`/`Bottom`
  values, including field values supplied through top-level static string
  variables or through top-level
  `config[static_name].horizontal`/`vertical` field mutations where
  `static_name` resolves to `window_content_alignment`. When explicitly
  configured, native resize keeps the real framebuffer pixel size, renders
  terminal cells into the aligned grid, fills leftover gap pixels with the
  configured background color, and maps mouse coordinates back through the same
  pixel offset.
- Implemented in v1: Meta-key mode `?1034` is tracked in the shared runtime
  mode tracker, DECRQM reports it for ESC/C1 CSI forms, and XTGETTCAP exposes
  WezTerm `km`/`smm`/`rmm` Meta-key capabilities.
- Implemented in v1: app runtime and console output filtering recognize
  WezTerm-style UTF-8 C1 CSI (`U+009B`) terminal queries and shared mode
  tracking prefixes without leaking UTF-8 prefix bytes, while retaining legacy
  raw C1 CSI compatibility.
- Implemented in v1: app runtime and console output filtering recognize
  WezTerm-style UTF-8 C1 DCS (`U+0090`) wrappers for DECRQSS and XTGETTCAP,
  protect UTF-8 C1 DCS payloads from nested query matching, and retain legacy
  raw C1 DCS/ST compatibility.
- Implemented in v1: native `enq_answerback` defaults to WezTerm's empty
  string, static Lua `config.enq_answerback` parses into the runtime override
  path, and top-level ENQ (`0x05`) controls reply with the configured bytes
  while ENQ bytes inside OSC/DCS control strings do not answer.
- Implemented in v1: XTGETTCAP replies expose the remaining official WezTerm
  boolean names (`am`, `bce`, `ccc`, `hs`, `mc5i`, `mir`, `msgr`, `npc`, `Su`,
  `xenl`) plus `flash`, printer (`mc0`/`mc4`/`mc5`), memory-lock
  (`meml`/`memu`), and `rs1` reset templates.
- Implemented in v1: XTGETTCAP replies expose implemented tab-stop/backtab,
  erase-character, repeat-character, scroll-region, indexed scroll,
  Backspace/BackTab/keypad-enter, SGR mouse templates (`kmous`/`XM`/`xm`),
  shifted navigation/editing, and WezTerm
  `kf13`-`kf63` modified function-key templates.
- Implemented in v1: XTGETTCAP replies expose dynamic `co`/`li` and official
  WezTerm `cols`/`lines` numeric names from the current runtime size, plus
  `it=8` for the default tab interval and `pairs=32767`.
- Implemented in v1: XTGETTCAP replies expose WezTerm basic control,
  save/restore, and ACS character-set templates (`bel`, `cr`, `ind`, `ri`,
  `sc`, `rc`, `smacs`, `rmacs`) using sequences already handled by the parser.
- Implemented in v1: XTGETTCAP replies expose WezTerm cursor-position and
  device-attribute query templates (`u6`, `u7`, `u8`, `u9`) for the existing
  CSI query response paths.
- Implemented in v1: `rterm-terminal` tracks `DECSCA` protected-cell state and
  applies it to DEC selective display/line erase (`DECSED`/`DECSEL`) while
  ordinary `ED`/`EL` still clears the addressed cell range.
- Implemented in v1: XTGETTCAP replies expose WezTerm title/status-line and
  palette-initialization templates (`tsl`, `fsl`, `dsl`, `initc`) for existing
  OSC title and OSC 4 color handling paths.
- Implemented in v1: `rterm-terminal` handles DECSTR `CSI ! p` soft reset for
  insert/replace mode, cursor visibility, origin mode, scroll region, G0
  character set, and saved-cursor state without clearing cells; app runtime and
  console filtering track ESC/C1 DECSTR for mode reports and expose XTGETTCAP
  `is2`/`rs2` reset/init templates.
- Implemented in v1: XTGETTCAP replies expose WezTerm keypad transmit
  templates (`smkx`, `rmkx`) for existing application cursor-key and
  application-keypad mode tracking/input paths.
- Implemented in v1: XTGETTCAP replies expose WezTerm keypad-center `kb2`,
  and console input encodes `KeypadBegin` as `ESC O E` while preserving keypad
  digit 5 as `ESC O u` in application-keypad mode.
- Implemented in v1: `rterm-app` tracks xterm OSC 4/10/11/12 color changes and
  answers color queries, including WezTerm-style multiple OSC 4 palette
  index/color update pairs and index query pairs in one sequence, plus
  `#RGB`/`#RRGGBB` hex color specs and RGBA `OSC 10`/`11`/`12` dynamic colors.
- Implemented in v1: `rterm-terminal` records OSC 0/1/2 title updates and Sun
  OSC L/l aliases without rendering those control bytes, giving the app shell
  WezTerm-compatible active-pane title fallback state.
- Implemented in v1: `rterm-terminal` saves and restores the tracked terminal
  title through xterm title-stack `CSI 22;0;0t` and `CSI 23;0;0t`, and
  XTGETTCAP `smcup`/`rmcup` expose WezTerm's alternate-screen plus title-stack
  templates.
- Implemented in v1: `rterm-terminal` handles DECALN `ESC # 8` screen alignment
  display by filling the visible grid with `E` cells and resetting
  margins/origin mode.
- Implemented in v1: `rterm-terminal` ignores WezTerm-documented non-printing
  C0 controls while preserving BEL/BS/HT/LF/VT/FF/CR/ESC special handling.
- Implemented in v1: `rterm-terminal` matches WezTerm line-feed semantics for
  bare `LF`, `VT`, and `FF` by moving down within the active scroll context
  while preserving the current column; `CR` remains the explicit return to the
  active left margin and `NEL`/`CRLF` provide next-line-at-left-margin behavior.
- Implemented in v1: `rterm-terminal` consumes `ESC =`/`ESC >`
  application-keypad mode escapes so those non-printing DEC controls do not
  enter the rendered grid.
- Implemented in v1: `rterm-terminal` plus app/runtime output filtering consume
  standalone ST controls (`ESC \`, UTF-8 C1 `U+009C`, and legacy raw C1
  `0x9C`) as no-effect sequences so string terminators do not enter the
  rendered grid or visible output when seen outside a control string.
- Implemented in v1: `rterm-terminal` tracks reverse-wrap mode `?45`; with
  auto-wrap enabled, BS at the left boundary wraps to the previous row's right
  boundary, and shared runtime mode tracking reports `?45` through DECRQM.
- Implemented in v1: `rterm-terminal` tracks DEC screen reverse-video mode
  `?5`; `rterm-renderer` applies it as a full-viewport inverse-video overlay,
  and shared runtime mode tracking reports `?5` through DECRQM.
- Implemented in v1: `rterm-terminal` models WezTerm SGR mode 6 RGBA colors for
  foreground, background, and underline color state; `rterm-renderer` preserves
  alpha in RGBA pixel conversion, and app-shell DECRQSS SGR responses serialize
  alpha-bearing colors.
- Implemented in v1: `rterm-terminal` maps WezTerm SGR `6` rapid blink onto the
  existing blink cell attribute, sharing SGR `5` visibility behavior and SGR
  `25` reset handling.
- Implemented in v1: `rterm-terminal` models WezTerm SGR 73/74/75 vertical-align
  state for superscript, subscript, and baseline; `rterm-renderer` offsets the
  glyph baseline, and app-shell DECRQSS SGR responses serialize active 73/74
  state.
- Implemented in v1: `rterm-app` answers WezTerm-documented DECRQSS `"` `p`
  conformance-level and `s` left/right-margin queries in both native runtime and
  console output filter paths. The `s` query reports the modeled DECSLRM state,
  and DECRQM reports DECLRMM `?69` for ESC and C1 CSI forms.
- Implemented in v1: copy mode can move between semantic zones across retained
  scrollback with WezTerm-style `z`/`Shift+Z` bindings and typed
  Prompt/Input/Output filters backed by OSC 133 zones.
- Implemented in v1: command-palette Activate Copy Mode exposes WezTerm-style
  `ActivateCopyMode`, and native `WindowCommand::ActivateCopyMode` payloads
  enter the same copy-mode path. Structured `activatecopymode` and
  `entercopymode` action-name queries resolve to the same copy-mode paths. The
  default `Ctrl+Shift+X` key-assignment entry now exposes the WezTerm-style
  `ActivateCopyMode` payload while the older native `EnterCopyMode` alias
  remains accepted.
- Implemented in v1: static `config.key_tables` `CopyMode` action payloads
  resolve top-level variables for single-name assignments, `SetSelectionMode`,
  semantic-zone type fields, nested jump `prev_char` or option tables,
  `MoveByPage`, and parenthesized static assignment table variables.
- Implemented in v1: copy mode stores source-row selection anchors, so `y` can
  copy selections that span the live viewport and retained scrollback.
- Implemented in v1: mouse double-click word selection honors
  `selection_word_boundary`, including WezTerm's documented default boundary set
  and native per-window overrides.
- Implemented in v1: copy mode `y` follows WezTerm's default CopyTo
  ClipboardAndPrimarySelection, then ScrollToBottom and Close behavior.
- Implemented in v1: copy mode supports WezTerm-style Cell selection with
  Space/`v`.
- Implemented in v1: copy mode supports WezTerm-style Line selection with
  uppercase no-modifier or shifted `V`.
- Implemented in v1: copy mode supports WezTerm-style rectangular block
  selection with `Ctrl+V`.
- Implemented in v1: copy mode vertical movement and page movement use
  source-row cursor coordinates, so `j`/`k`, arrows, PageUp/PageDown,
  `Ctrl+B/F`, and `Ctrl+U/D` can traverse retained scrollback.
- Implemented in v1: copy mode supports WezTerm-style
  `MoveToStartOfNextLine` through Enter and character CR (`\r`) events.
- Implemented in v1: copy mode `g`/`Shift+G` move to scrollback top/bottom.
- Implemented in v1: copy mode `H`/`M`/`L` move to viewport top/middle/bottom
  for both shifted and uppercase no-modifier default key-table events.
- Implemented in v1: copy mode `^`/`Alt+m` and `$`/End move to the first/last
  non-space cell in the current source row, matching WezTerm-style
  content-aware line start/end.
- Implemented in v1: copy mode word movement supports WezTerm-style
  `MoveForwardWord`, `MoveBackwardWord`, and `MoveForwardWordEnd` defaults
  across retained source rows.
- Implemented in v1: copy mode jump-to-char supports WezTerm-style
  `JumpForward`, `JumpBackward`, `JumpAgain`, and `JumpReverse` defaults on the
  current source row, including optional trailing commas in nested jump option
  tables.
- Implemented in v1: copy mode supports WezTerm-style `MoveToSelectionOtherEnd`
  and `MoveToSelectionOtherEndHoriz` through `o`/`O`.
- Implemented in v1: ordinary copy-mode close follows WezTerm's
  `ScrollToBottom` then `Close` default behavior before exiting the overlay.
- Implemented in v1: copy mode and copy-mode search close on both Escape key
  events and character ESC (`\u{1b}`) events, clearing copy-mode search status
  from the window title.
- Implemented in v1: copy mode and copy-mode search allow global
  command-palette and app-shell shortcuts such as `Ctrl+Shift+P` and
  `Ctrl+Shift+T` to fall through from the overlay, matching WezTerm key-table
  fallback behavior.
- Implemented in v1: copy-mode search keeps copy mode active while entering
  `/`/`?` queries and supports WezTerm-style next/prior match navigation,
  including character CR as PriorMatch.
- Implemented in v1: copy-mode search supports WezTerm-style page-wise match
  navigation with PageDown/PageUp.
- Implemented in v1: copy-mode search supports WezTerm-style `Ctrl+R`
  match-type cycling across case-sensitive, case-insensitive, and regex search.
- Implemented in v1: default `Ctrl+Shift+F`/`Super+F` search shortcuts open
  WezTerm-style `Search(CaseSensitiveString="")`, while command-palette Search
  exposes the same search overlay with search table navigation via Down/Up,
  `Ctrl+N`/`Ctrl+P`, PageDown/PageUp, `Ctrl+R` match-type cycling, `Ctrl+U`
  clear-pattern, and character ESC close. Command-palette
  `search <pattern>`, `search regex <pattern>`,
  `search case-sensitive <pattern>`, and
  `search case-insensitive <pattern>` / `search case insensitive <pattern>`
  queries open Search with that initial typed pattern using quote-aware parsing
  and select the bottom-most initial match in the pane.
  WezTerm-style action field names `search casesensitivestring <pattern>` and
  `search caseinsensitivestring <pattern>` dispatch the same typed search
  payloads, and `search current selection or empty string` maps to WezTerm-style
  `CurrentSelectionOrEmptyString`. Native `Search` action payloads cover typed
  `Regex`, `CaseSensitiveString`, and `CaseInSensitiveString` patterns plus
  `CurrentSelectionOrEmptyString` single-line selection-prefill behavior, and
  WezTerm-style Lua `Search { ... }` / `Search({ ... })` table queries tolerate
  trailing comma fields. Static `config.keys` `Search` action calls resolve
  top-level string variables for table pattern fields, parenthesized static
  options table variables, and `CurrentSelectionOrEmptyString` string arguments.
- Implemented in v1: quick-select mode (`Ctrl+Shift+Space`) and
  command-palette Quick Select expose WezTerm-style `QuickSelect` for common
  patterns (URLs including `git@`, `git://`, `ssh://`, and `ftp://`, markdown
  URLs, diff paths, docker SHA values, paths, colors, UUID/IPFS/SHA hashes,
  IPv4/IPv6, hex addresses, long numbers, emails), quick overlay navigation including
  `Ctrl+N`/`Ctrl+P`, PageDown/PageUp page-wise movement, WezTerm's Enter
  PriorMatch binding, configurable labels honoring `quick_select_alphabet`,
  configurable `quick_select_patterns` appended to the defaults, including
  top-level static Lua table-variable assignments and
  `table.insert(config.quick_select_patterns, ...)` appends, configurable
  `disable_default_quick_select_patterns` so configured patterns become the
  full set, and `quick_select_remove_styling` stripping pane colors, text
  styling, vertical alignment, hyperlink metadata, and inverse attributes before
  quick-select match/label highlights are applied, typed quick-select label
  prefixes hide non-matching labels while keeping matching labels visible,
  and same-text quick-select candidates are de-duplicated before label
  assignment,
  `input_selector_label_bg`/`input_selector_label_fg` and
  `launcher_label_bg`/`launcher_label_fg` parsing into native/effective config
  and applying to default-mode selector/launcher shortcut labels,
  quote-aware command-palette `quick select alphabet <chars>` for the native
  `QuickSelectArgs { alphabet = ... }` subset, command-palette
  `quick select pattern <regex>` and
  `quick select patterns <regex> ; <regex>` for native
  `QuickSelectArgs { patterns = ... }` override subsets, splitting only on
  unquoted ` ; ` separators so quoted regexes can include semicolons,
  command-palette `quick select scope lines <n>` for the native
  `QuickSelectArgs { scope_lines = ... }` subset with a complete numeric value,
  command-palette
  `quick select label <text>` for the native status/overlay label subset with
  quote-aware text parsing, command-palette `quick select action open uri` with
  quoted or unquoted action names for a native open-uri action subset using the
  same open-uri hook as hyperlink clicks, command-palette
  `quick select action copy to clipboard`,
  `quick select action copy to primary selection`, and
  `quick select action copy to clipboard and primary selection` for native
  `CopyTo` action subsets with quoted or unquoted destinations, command-palette
  `quick select action open uri skip action on paste`/`skip_action_on_paste`/
  `skip-action-on-paste`, including `=true|false` suffixes, for the native
  `skip_action_on_paste` subset on valid native action paths, including
  `action=<action> skip_action_on_paste=true|false` assignment forms,
  and WezTerm-style action-name `quickselectargs pattern`/`patterns`/
  `alphabet`/`label`/`action`/`scope lines`/`scope_lines`/`scope-lines` query
  prefixes, with `pattern=<regex>`, `patterns=<regex>[;<regex>]`, `alphabet=<chars>`,
  `label=<text>`, `action=<action>`, `scope_lines=<n>`, and `scope-lines=<n>` assignment forms plus
  legacy `quickselect ...` aliases, with assignment fields combinable in the
  same query for the same implemented `QuickSelectArgs`
  fields, and WezTerm-style
  label input where lowercase labels copy the match to ClipboardAndPrimarySelection
  and uppercase labels paste it into the pane, while partial label prefixes hide
  non-matching labels. The default `Ctrl+Shift+Space`
  key-assignment entry exposes `QuickSelect` with default native args, while
  `EnterQuickSelect` remains an internal command-palette query alias and
  action-name `enterquickselect` queries dispatch that default entry. Lua
  option-table wiring accepts `pattern`, `patterns` (including top-level
  static table variables whose entries can resolve through static string
  variables), `alphabet`, `label`, `skip_action_on_paste`, and `scope_lines`
  inline or through top-level static string/bool/number variables from static
  WezTerm-style `config.keys`; parenthesized `QuickSelectArgs(quick_opts)` calls
  also accept top-level static options table variables. Exact WezTerm-style
  `QuickSelectArgs` Lua table/config forms materialize as native
  `QuickSelectArgs { patterns, alphabet, label, action, skip_action_on_paste,
  scope_lines }` payloads, while local `quick select ...` query aliases keep the
  internal command-palette entry path. It also accepts static
  `QuickSelectArgs.action = wezterm.action_callback(...)` values as
  native-handler placeholders, maps
  `window:perform_action(<implemented action>, pane)` callback bodies onto the
  corresponding native quick-select action subset, and supports
  `pane:send_text(window:get_selection_text_for_pane(pane))` /
  `pane:send_paste(window:get_selection_text_for_pane(pane))` callbacks that
  send or paste the selected match, plus official-style
  `wezterm.open_with(url)` callbacks where `url` is assigned from
  `window:get_selection_text_for_pane(pane)`, dispatching through the native
  open-uri hook. Lua table parsing skips trailing-comma fields and resolves
  top-level static action variables for `QuickSelectArgs.action` inside static
  WezTerm-style `config.keys`; arbitrary custom action execution remains open.

## Technology Choices

Current foundation stack:

- `winit` for cross-platform native windows and input.
- direct `wgpu` surfaces for GPU presentation, with `glyphon`-backed text
  batches and bounded glyph caches; the CPU/offscreen renderer remains a
  deterministic test and benchmark-proxy path.
- `rterm-fonts` with `cosmic-text` for shaping and configured fallback.
- native `russh` for in-process SSH and forwarding, alongside system OpenSSH
  for interactive compatibility plus SFTP/SCP workflows.
- platform PTY adapters for Windows ConPTY and Unix PTYs.

Planned product layers:

- SQLite-backed session and host metadata; `rusqlite` or `sqlx` remains an
  implementation choice.
- OS-backed secret storage through Windows DPAPI, macOS Keychain, or Linux
  Secret Service. Passwords and private-key passphrases must never be stored in
  plaintext or ordinary metadata storage.

## Error Handling

- Connection failures should keep the session tab visible with a clear terminal
  status line and retry action.
- Host key mismatch is a blocking security error and must never be auto-accepted.
- Parser errors should be counted and logged, but unknown escape sequences should
  not crash the session.
- Renderer device loss should rebuild renderer state from terminal grid and
  scrollback.
- Secret storage failures should degrade to "not saved" rather than plaintext
  persistence.

## Testing Strategy

- Unit-test terminal grid behavior and VT parser conformance with recorded
  byte streams.
- Use snapshot tests for terminal state after escape sequences.
- Use loopback SSH fixtures before touching real servers.
- Use local PTY integration tests gated by platform.
- Keep deterministic renderer pixel/snapshot tests, and exercise direct `wgpu`
  native-window presentation in platform E2E jobs.
- Use fuzzing for the VT parser before handling untrusted network streams at
  scale.
