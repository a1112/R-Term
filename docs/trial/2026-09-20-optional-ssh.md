# R-Term optional SSH: Windows trial validation (2026-09-20)

## Product boundary

The application binary and command identity are `rterm`. The internal Cargo
package remains `rssh-app`; the workspace defaults to that package. Default
features provide a native local terminal, with SSH disabled. `--features ssh`
adds the SSH extension; `transfer-tools` enables SSH plus SCP/SFTP.

The local build rejects remote commands and remote profiles before starting a
transport. Profile initialization produces valid local-only examples. Local PTY,
window startup, rendering and process cleanup remain available. `rterm-types`
owns terminal dimensions and only converts to R-SSH types when SSH is enabled.

## Reproduce

Run from the R-Term repository, on a configured Windows Rust/MSVC environment:

```powershell
cargo build --locked -j1
cargo run --locked -- local -- powershell.exe
cargo run --locked -- window -- cmd.exe
cargo install --locked --path crates/rssh-app --bin rterm
python scripts/ci/check-rterm-feature-boundary.py
cargo test --locked -p rssh-app --no-default-features --bin rterm -j1
cargo test --locked -p rssh-app --test feature_boundary --test local_pty --test native_window_e2e -j1
cargo test --locked -p rssh-app --features developer-full --bin rterm --test feature_boundary --test openssh_loopback -j1
cargo test --locked -p rterm-ssh-adapter -j1
cargo clippy --locked -p rssh-app --no-default-features --all-targets -j1 -- -D warnings
cargo clippy --locked -p rssh-app --features developer-full --all-targets -j1 -- -D warnings
cargo fmt --all --check
```

Tests use `RUST_TEST_THREADS=4`. The successful graph check excludes all five
SSH packages (`russh`, `rssh-ssh`, `rssh-types`, `rterm-ssh-adapter`, `rpassword`)
from both default and no-default-features production graphs, and requires them
when SSH is enabled. Dev-only loopback fixtures are outside that graph.

## Results

| Validation | Result |
| --- | --- |
| SSH disabled, no-default-features unit tests | 3,913 passed; 3 existing ignored |
| Default local command / PTY / native window integration | 25 passed; 6 existing ignored |
| developer-full unit tests | 4,145 passed; 3 existing ignored |
| developer-full command identity | 1 passed |
| OpenSSH loopback integration | 7 passed |
| SSH runtime adapter tests | 5 passed |
| Clippy, all targets, both minimal and developer-full | Passed with warnings denied |
| Standalone ssh feature compilation | Passed |
| Production dependency graph, three profiles | Passed |
| Rust formatting | Passed |

No failing tests remain in this selected Windows validation matrix. Ignored
tests retain their pre-existing dedicated-runner/release requirements.

## Evidence and artifacts

Evidence directory: `H:/project/R-SSH/evidence/optional-ssh-20260920`.
Final results are recorded in `results.json`; intermediate failure logs are kept
alongside the final logs. Feature-dependent tests are gated by their owning
feature; test deadlines and failure assertions were not relaxed.

Trial debug executables are saved separately:

- `dist/optional-ssh/local/rterm.exe`: default local build, SSH disabled.
- `dist/optional-ssh/with-ssh/rterm.exe`: developer-full build, SSH enabled.

`dist/optional-ssh/receipt.json` records hashes, command identity and source commit.
These ignored artifacts are machine-local, not release packages.

## Limits

This validates the native Windows application boundary. Linux/macOS native
execution and protected CI certification remain unavailable; Stage 7 stays
NO-GO. This does not certify the Web/Tauri product or historical CI workflows.
The optional R-SSH Git source remains pinned to the local trial repository;
Cargo may need that source to resolve its lockfile/cache even for a disabled
feature. Relocating/publishing the repository needs a reachable immutable source
URL. The original split receipt `../trial.json` remains historical; this work
is recorded separately and does not rewrite its original certification claims.
