# R-Term

This repository owns the native graphical application, window handling, terminal
emulation and runtime, CPU/GPU renderers, fonts, clipboard/input, local PTY,
Web UI, Tauri application, graphical configuration and graphical diagnostics.
The executable is `rterm`. Internal `rssh-*` crate names are retained for source compatibility.

## Local terminal (default, SSH disabled)

```powershell
cargo build --locked
cargo run --locked -- local -- powershell.exe
cargo run --locked -- window -- cmd.exe
```

To install the command into Cargo's bin directory (which must be on `PATH`):

```powershell
cargo install --locked --path crates/rssh-app --bin rterm
rterm local -- powershell.exe
```

Running `rterm` without arguments opens the local graphical terminal. `rterm local`
or `rterm console` runs the local shell in the current console; `--cwd` and
`-- <program> [args...]` select a working directory and command. `rterm version
--json` reports `native_ssh_backend: "disabled"` in this build. SSH, SCP and SFTP
requests, including remote profiles, fail before launching any transport.

Both the default build and `cargo build --locked --no-default-features` exclude
the SSH backend, adapter, protocol types and password-input library from the
production dependency graph. Terminal dimensions are owned by R-Term itself.

## Optional SSH extension

```powershell
cargo build --locked --features ssh
cargo run --locked --features ssh -- ssh --native user@host
cargo run --locked --features ssh -- ssh --gui user@host
```

`--features transfer-tools` also enables SSH and the SCP/SFTP command entries.
`--features developer-full` includes SSH, transfer tools, diagnostic commands and
the extended image decoders. SSH is a compile-time extension, not a dynamic plugin.

For explicit package selection use `-p rssh-app --bin rterm`. The workspace's
default member is the terminal application; `--workspace` additionally builds
the adapter and other auxiliary packages, so it is not the SSH-free build command.

## Web and Tauri

The Web and Tauri sources remain in `web/` and `tauri/`. Before checking/building
the entire workspace, run `npm --prefix web ci` and `npm --prefix web run build`;
Tauri embeds the resulting Web assets.

When enabled, SSH transport comes from the separate R-SSH repository, pinned to an immutable
GitHub commit in Cargo.toml/Cargo.lock. There are no sibling path dependencies.
The GUI-owned `rterm-ssh-adapter` bridges SSH sessions to the terminal runtime.
Cargo may resolve/fetch the pinned optional Git source for its lockfile/cache,
even when the extension is disabled. No local trial directory is required.

## Validation

```powershell
python scripts/ci/check-rterm-feature-boundary.py
cargo test --locked -p rssh-app --test feature_boundary --test local_pty --test native_window_e2e
cargo test --locked -p rssh-app --features developer-full --bin rterm --test feature_boundary --test openssh_loopback
```

The graph check covers default, no-default-features and SSH-enabled builds.
See [Windows validation](docs/trial/2026-09-20-optional-ssh.md) for the trial results.

Historical monolith workflows remain archived under `docs/trial/legacy-workflows`.
Active Windows CI validates the split repository. Cross-platform protected CI
certification remains outstanding. See [formal split record](docs/formal-split.md)
for provenance, ownership and rollback.
