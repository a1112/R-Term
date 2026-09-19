# R-Term — complete graphical product trial

This repository owns the native graphical application, window handling, terminal
emulation and runtime, CPU/GPU renderers, fonts, clipboard/input, local PTY,
Web UI, Tauri application, graphical configuration and graphical diagnostics.
Existing `rssh-*` GUI package names are retained for this first trial.

`cargo build --locked -p rssh-app` builds the native graphical application.
The Web and Tauri sources remain in `web/` and `tauri/`.

SSH transport comes from the separate R-SSH repository, pinned to an immutable
local Git commit in Cargo.toml/Cargo.lock. There are no sibling path dependencies.
The GUI-owned `rterm-ssh-adapter` bridges SSH sessions to the terminal runtime.
Relocation to another machine requires making that Git source available or
replacing its local URL with an approved repository URL while retaining the SHA.

Historical documentation and scripts still describe the monolith. Historical CI
workflows are archived under `docs/trial/legacy-workflows`; they are not evidence
that this trial is certified. Stage 7 remains NO-GO. No remote is configured.
The original source checkout remains the release source and rollback point.
