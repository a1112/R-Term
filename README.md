# R-Term

R-Term is an independent graphical terminal with local PTY sessions, terminal
emulation, CPU/GPU rendering, fonts, clipboard/input, a Web UI and a Tauri app.
The executable is `rterm`; all workspace packages and runtime identifiers belong
to R-Term. Building any feature requires no R-SSH checkout, binary, Git source
or service.

## Local terminal

```sh
cargo build --locked
cargo run --locked
cargo run --locked -- local
cargo run --locked -- window --cwd /path/to/project
cargo install --locked --path crates/rterm-app --bin rterm
```

Running `rterm` without arguments opens the graphical terminal. `rterm local`
or `rterm console` runs the local shell in the current console. `--cwd` and
`-- <program> [args...]` select a directory and command, for example
`rterm local -- powershell.exe` on Windows or `rterm local -- /bin/zsh` on macOS.

Default and `--no-default-features` production builds exclude the native SSH
backend and password-input library. `rterm version --json` reports the compiled
capabilities. Remote commands and profiles fail before starting a transport
when SSH is disabled.

## Optional SSH

```sh
cargo build --locked --features ssh
cargo run --locked --features ssh -- ssh --native user@host
cargo run --locked --features ssh -- ssh --gui user@host
```

`--features transfer-tools` adds SCP/SFTP; `--features developer-full` adds SSH,
transfers, diagnostics and extended image decoders. The optional `rterm-ssh`
crate is maintained here and uses upstream `russh` directly. System OpenSSH
commands use the installed `ssh`/`scp`/`sftp` tools. Neither backend calls R-SSH.
`rterm-types` has no dependencies or transport features.

Use `-p rterm-app --bin rterm` for explicit application selection. The default
workspace member is the application; `--workspace` also builds auxiliary
packages including the optional backend and Tauri.

GUI SSH supports `--renderer auto`, `--renderer cpu` and `--renderer gpu`.
Use `--benchmark-startup` to report startup metrics. A remote profile can set:

```toml
[profiles.remote]
kind = "ssh"
target = "user@host"
gui = true
renderer = "auto"
host_key_policy = "prompt"
```

GUI SSH does not support forwarding. GUI SSH does not support `--no-shell`.
GUI SSH does not support OpenSSH passthrough options.

## Package ownership

`rterm-types` owns neutral values; `rterm-domain` owns application state;
`rterm-terminal` owns emulation; `rterm-fonts` owns font resources. The
rterm-core compatibility facade re-exports these boundaries for existing
internal consumers.

`rterm-runtime` is transport-neutral. The `runtime-adapter` interfaces in
`rterm-pty` and `rterm-ssh` connect local and remote sessions to that runtime.
`rterm-render-core`, `rterm-render-cpu` and `rterm-render-wgpu` own rendering;
the rterm-renderer compatibility facade remains available to existing consumers.

## Web and Tauri

```sh
npm --prefix web ci
# Start the Rust backend and Vite in separate terminals:
cargo run --locked -p rterm-web
npm --prefix web run dev
# Or build the Tauri application:
npm --prefix tauri ci
npm --prefix tauri run build
```

The Web page and Tauri bundle use the R-Term identity and isolated browser
storage/cookies. Before a Rust workspace-wide check, run
`npm --prefix web run build`; Tauri embeds `web/dist`.
Native unsigned archives are created by `scripts/ci/package-native.sh` or
`scripts/ci/package-native.ps1`. See [console packaging](docs/release-console.md).

## Validation

```sh
python3 scripts/ci/test_rterm_independence.py
python3 scripts/ci/check-rterm-feature-boundary.py
cargo fmt --all --check
cargo test --locked -p rterm-app --test feature_boundary --test local_pty --test native_window_e2e
cargo test --locked -p rterm-app --features developer-full --test feature_boundary --test openssh_loopback --test transfer_loopback
cargo test --locked -p rterm-ssh
```

The boundary check covers every local manifest, both lockfiles, outside-checkout
paths, dependency-free terminal types and four production feature graphs.
CI runs repository checks and local/SSH tests on Windows, macOS and Linux;
Web tests run separately. Historical split/performance records and frozen test
fixtures retain their original provenance; they are not current certification.
See [repository independence](docs/formal-split.md) and
[source provenance](SOURCE-PROVENANCE.json).

Developer diagnostics can be collected with `scripts/ci/run-stage0-diagnostics.sh`
or `scripts/ci/run-stage0-diagnostics.ps1`. Their metric definitions and historical
gates are documented in [Stage 0 metric schema](docs/benchmarks/stage0-schema-v2.md).
