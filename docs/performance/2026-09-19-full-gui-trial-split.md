# Full graphical product / SSH-only trial split

Date: 2026-09-19. This records a local trial, not a Stage 7 GO or a published split.

## Outputs and ownership

| Repository | Local path | Trial commit |
| --- | --- | --- |
| R-SSH | `H:/project/R-SSH-split-trial/R-SSH` | `c2c515df0e532271abe32aac63c62b91fc75864d` |
| R-Term | `H:/project/R-SSH-split-trial/R-Term` | `3ff8deb12488422669e372b25d22a5d86406883b` |

Both are derived from committed extraction source
`f79dcfd22a9f655f21f20616c11380993b02304a`; the unchanged original product main is
`a24308fc4cab0b6c60aa8722c193f9778b3a898c`. Each trial retains original Git ancestry,
has its own object database, uses branch `codex/trial-split`, and has no remote.
`H:/project/R-Term` (the pre-existing empty checkout) was not modified.
The subsequent extraction-tool fix is recorded as `extraction_fix_commit` in
`trial.json`: the Linux/OpenSSH source-contract assertions moved intact from the
graphical test into the SSH repository, and R-Term's immutable SSH pin was updated.

R-SSH's current tree owns the SSH backend, SSH PTY dimensions, native CLI,
SSH/process fixtures and the isolated OpenSSH server launcher. Graphical window
probes were removed from its fixture crate. It contains no graphical application,
renderer, font library, terminal emulator/runtime, Web or Tauri workspace member.

R-Term owns the entire existing graphical product: native window/application,
CPU/GPU renderers, fonts, terminal emulation/runtime, local PTY, GUI input and
clipboard, configuration/domain, graphical diagnostics, Web/Tauri and tests.
Existing graphical package/binary names remain compatible for this first trial.

## Dependency boundary

R-Term consumes both `rssh-ssh` and `rssh-types` from the same full local Git SHA
shown above. The former SSH crate path in R-Term is now `rterm-ssh-adapter`: a
GUI-owned facade and terminal-runtime adapter, without a duplicate SSH backend.
`rterm-types::TerminalSize` reexports the dependency-free SSH dimension type.
Consequently R-SSH does not depend on R-Term, even with every SSH feature enabled.

`scripts/split/check-boundary.py` passed on both final directories: four SSH
workspace packages, 175 resolved SSH-side packages and 561 GUI-side packages on
Windows. It checks forbidden graphical dependencies, ownership, external path
dependencies and exact Git source identity, including optional features.

Legacy monolith CI workflows are archived in R-Term under
`docs/trial/legacy-workflows`. Rust contract tests that embed those files now read
the archive. Those historical contracts are not newly configured standalone CI.

## Validation

Final evidence is under `evidence/trial-split-20260919/final/` in the original
checkout. Rust uses 1.89, `RUST_TEST_THREADS=4`, compilation `-j1`, and external
target directory `D:/codex-builds/R-SSH-windows-certification-20260919`.

- R-SSH `cargo test --workspace --all-targets --locked -j1`: **205 passed, zero failed**.
  This includes the new CLI's real SSH command/output/exit-status and unknown-key
  rejection tests, plus existing native authentication/channel/forwarding tests.
- R-Term Web: dependency install, build, lint and **3 unit tests passed**.
- R-SSH workspace/all-targets Clippy with `-D warnings`: **passed**.
- R-Term `cargo check --workspace --all-targets --locked -j1`: **passed**,
  including the native application, Web backend and Tauri application.
- R-Term SSH adapter and shared types: **6 tests passed**.
- R-Term real native-window and OpenSSH loopback suites: **21 passed, zero failed,
  6 existing dedicated/release scenarios ignored**. This builds and launches the
  actual graphical executable, exercises ten-frame PTY rendering, cleanup,
  disconnect/reconnect, authentication/host-key checks and TCP forwarding.
- Final immutable-source boundary check: **passed**.

The initial attempts exposed and corrected a missing SSH fixture script, an
explicit Rust closure type requirement, and historical workflow include paths.
Native E2E then exposed a remaining monolith path to an SSH test; its assertions
now run under the SSH owner instead of reading a nonexistent file from R-Term.
Tauri's workspace build requires `npm --prefix web run build` first; this is now
documented. Earlier attempt logs and directories are diagnostic only. The two
paths at the top are the final trial outputs.

## Scope and remaining work

The new `rssh` CLI offers agent/private-key/password authentication, strict
known-host verification, shell/remote commands, bidirectional streams and remote
exit status. It uses cooked console input and is not an OpenSSH-compatible
replacement. Raw terminal handling/resize, OpenSSH configuration, forwarding CLI
options and scp/sftp commands remain future work; backend forwarding is retained.

The trial is not a filtered history extraction: old GUI code remains reachable in
R-SSH's ancestry. Local Git source URLs also require a deliberate remote-source
transition before distribution or CI on another machine. Nothing was published,
and no remote default branch was changed.

Stage 7 remains NO-GO. This trial does not claim fixed-runner performance,
Linux/macOS execution, protected CI, complete post-split regression, signed
release or full rollback certification. The original main remains the release
source; reverting to it requires no production migration.

Reproduction and build commands: `scripts/split/README.md`.
