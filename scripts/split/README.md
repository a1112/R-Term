# SSH-only / full-GUI local trial

This trial follows the 2026-09-19 ownership change: **R-SSH becomes the SSH
connection library and command-line client; R-Term owns the entire graphical
product**, not just the seven original terminal libraries.

Run from a committed source tree with Rust 1.89, Git and Python available:

```powershell
$env:CARGO_TARGET_DIR = 'D:/codex-builds/rssh-trial'
python scripts/split/create-trial.py --output H:/project/my-new-trial
python scripts/split/check-boundary.py --ssh H:/project/my-new-trial/R-SSH --gui H:/project/my-new-trial/R-Term
```

The output directory must not already exist and must be outside the source tree.
The script creates independent Git clones without hardlinks, retains ancestry,
removes remote configuration, commits both trial trees and writes `trial.json`.
It never filters the original object database, publishes a repository, switches a
remote default branch or changes the Stage 7 contract. A failed attempt is left
for diagnosis; choose a new output directory when rerunning.

| Owner | Current-tree contents |
| --- | --- |
| R-SSH | `rssh-ssh` backend, dependency-free `rssh-types`, `rssh-cli`, SSH/process test support, isolated OpenSSH fixture script |
| R-Term | Native app/window/input/clipboard, renderers/fonts, terminal/runtime/local PTY, GUI configuration/domain, Web/Tauri, graphical diagnostics and tests |
| R-Term adapter | Existing `crates/rssh-ssh` path becomes `rterm-ssh-adapter`; it reexports the external SSH API and implements the terminal runtime bridge |

R-Term obtains `rssh-ssh` and `rssh-types` from one full immutable local Git SHA.
`rterm-types::TerminalSize` reexports `rssh-types::TerminalSize`, preserving type
identity without making the SSH backend depend on the terminal runtime. Other
graphical crate names remain compatible during the trial.

The boundary checker resolves all features, rejects graphical dependencies in
R-SSH, verifies the immutable source identity, and rejects external path
dependencies. It is a structural trial check, not the Stage 7 certification gate.

## Local validation

In the SSH output:

```powershell
$env:RUST_TEST_THREADS = '4'
cargo test --workspace --all-targets --locked -j1
cargo clippy --workspace --all-targets --locked -j1 -- -D warnings
cargo build --locked -p rssh-cli
```

In the R-Term output:

```powershell
npm --prefix web ci
npm --prefix web run build
npm --prefix web run lint
npm --prefix web test
cargo check --workspace --all-targets --locked -j1
cargo test --locked -p rterm-ssh-adapter -p rterm-types -j1
cargo test --locked -p rssh-app --test native_window_e2e --test openssh_loopback -j1
```

The native tests build the actual graphical executable. Set
`RSSH_REQUIRE_OPENSSH=1` when OpenSSH tools are installed to require their probes.
Tauri needs the generated Web assets before workspace compilation.

## Limits and rollback

The CLI supports agent/private-key/password authentication, strict known-hosts
verification, shell/remote command startup, bidirectional byte streams and remote
exit status. Interactive input uses cooked console mode. OpenSSH configuration,
raw terminal handling/resize, forwarding CLI flags and scp/sftp commands remain
future work; backend forwarding support is retained. This is not an OpenSSH
replacement release.

The trial's Git ancestry still contains the monolith. This is not a filtered,
security-audited history extraction. Local Git URLs must be replaced by approved
repository URLs before remote CI or distribution. Legacy single-repository CI is
archived in R-Term and historical Rust contract checks point to that archive.

The original main branch remains the release/rollback source. To stop using the
trial, continue building from that original checkout; no production migration is
required. Do not call the trial `split-complete`: native Linux/macOS certification,
protected CI, performance gates, complete regression/release/rollback rehearsal
and final history extraction remain separate work.
