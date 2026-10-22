# R-SSH — SSH-only trial

This workspace owns SSH authentication, host-key verification, channels, remote
shell/commands and forwarding primitives. It has no window, renderer, font,
terminal emulator, Web or Tauri dependency, including optional features.

`cargo build --locked -p rssh-cli` builds `rssh`. Run `rssh --help` for the initial
command-line interface. This is an experimental native client, not an OpenSSH
compatible replacement: OpenSSH configuration, scp/sftp commands and forwarding
CLI options are not yet implemented. Forwarding remains available in the library.

Unknown and changed host keys are rejected. Supply a previously verified
known_hosts file. Passwords are requested through a hidden terminal prompt, never
through command-line arguments.

`cargo test --locked --workspace --all-targets` runs the SSH and fixture tests.
The graphical product and terminal adapter live in the separate R-Term trial.

This repository retains the original monolith ancestry for local traceability.
Only the current tree is SSH-only. History has not been filtered or audited for
publication. No remote is configured and Stage 7 remains NO-GO.
