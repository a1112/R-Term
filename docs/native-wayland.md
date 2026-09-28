# Native Linux terminal

The production build keeps SSH disabled. Build the locked default profile with
`cargo build --locked --release -p rssh-app --bin rterm`.

`rterm window` and `rterm start` accept `--renderer auto|cpu|gpu` and
`--display-backend auto|wayland|x11`. Both default to `auto`. Renderer selection
is retained by newly created windows and configuration reloads. A forced Linux
display backend fails if its connection fails; it never retries the other
backend. Explicit `wayland`/`x11` selections are rejected on other platforms.

Example for a software-rendered desktop terminal:

```sh
rterm window --renderer cpu --display-backend wayland --class org.forge.RTerm
```

`--class` sets the Wayland application ID and X11 WM_CLASS. Use the same value
as the desktop file basename for taskbar grouping. `--cwd` and command arguments
remain distinct arguments; launchers should never concatenate them into shell
code.

The GUI owns a lazily initialized clipboard for the lifetime of its UI thread.
Copy and paste access it only on request. Linux builds enable arboard's Wayland
data-control support; the compositor must support that protocol. No clipboard
history or periodic clipboard reads are implemented.

Linux CI uses a headless Sway compositor with `DISPLAY` unset to exercise CPU
presentation and clipboard ownership. This is a protocol smoke check, not KWin
or Fcitx acceptance. In the target KWin VM also verify Fcitx preedit, candidate
placement, commits and Enter, focus changes, Chinese wide characters, copy/paste
in both directions, and 100/150/200 percent scaling. Do not promote the terminal
to desktop default until those checks and the desktop stability checklist pass.

The standalone tests are:

```sh
cargo test --locked -p rssh-app --test native_wayland
# In a data-control-capable Wayland session with wl-paste installed:
env -u DISPLAY cargo test --locked -p rssh-app --test native_wayland native_wayland_cpu_window -- --ignored --exact
env -u DISPLAY cargo test --locked -p rssh-app --bin rterm native_wayland_clipboard_survives_copy_callback -- --ignored
```
