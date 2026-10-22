# R-Term repository independence

R-Term owns the native terminal application, Web/Tauri frontends, windowing,
configuration, local PTY, terminal/runtime, rendering, fonts and test fixtures.
All current package names and crate directories use `rterm-*`; the executable
is `rterm`. The two repositories can be checked out, built and versioned
independently.

## Current boundary

- No Cargo feature, lockfile or runtime path consumes the R-SSH repository.
- `rterm-types` owns terminal dimensions and has no transport dependencies.
- Optional native SSH is owned by `crates/rterm-ssh` and depends directly on
  upstream `russh`. Its runtime adapter and hermetic fixtures live here.
- System OpenSSH remains an explicit host tool for its backend and transfers.
- R-Term owns its product name, bundle identifiers, console wrappers, profile
  filename, environment variables, browser storage and session cookie names.
- `scripts/ci/check-rterm-feature-boundary.py` rejects R-SSH dependencies,
  outside-checkout dependency paths and transport leakage into local builds.

The independence change deliberately adopts R-Term names without automatically
reading or writing old R-SSH profile files or browser storage. Existing user
profiles can be copied explicitly to `rterm-profiles.toml`. WezTerm-compatible
configuration loading and standard OpenSSH configuration are retained.

## Provenance and historical evidence

The September 2026 extraction retained Git ancestry and initially consumed an
immutable R-SSH Git dependency. The October 2026 independence change imports
that pinned MIT-licensed backend into R-Term ownership and removes the Git
source entirely. Source commits are recorded in `SOURCE-PROVENANCE.json` and
copyright attribution in `NOTICE`.

Dated plans, trial scripts and frozen Task 10 traces retain original repository
names, source paths and checksums as historical evidence. Archived workflow
templates use the current crate names for their contract regression tests.
They do not create build or runtime dependencies. Frozen trace wire formats
remain readable. New changes must not rewrite historical evidence to claim
certification.

Local builds, automated tests, native window smoke tests and CI each provide
their stated evidence. Signing/notarization, physical Windows/Linux acceptance
and protected Stage 7 performance certification are separate release gates.

Rollback uses a new commit restoring the desired previous tree; preserve shared
Git history and user configuration.

## Verification on macOS (2026-10-16)

The current worktree passed an exported-checkout build with `.git`, local build
output and Node modules excluded. All 53 local dependency paths in its Cargo
metadata were inside that export. With the dependency cache available, the
production GUI checked successfully using `--locked --offline`; the R-SSH
checkout and service were not used. The seven-package downstream consumer also
checked successfully.

The workspace and all Rust test targets compile. Four feature graph checks,
strict Clippy for the complete application profile, the 150-file architecture
policy, 356 frozen trace records, 4,137 complete-profile application unit tests
(two pre-existing ignored cases), 715 runtime/terminal tests, 129 SSH backend
tests, 11 application SSH/transfer tests, 20 Rust Web tests and the functional
harness passed. Web build, lint and unit tests passed.

Native macOS checks exercised a real PTY/window, child exit code 7 and process
cleanup. A deterministic regression also preserved that exit status after
repeated resizes before the close notice was consumed. The default local-only
unsigned bundle passed manifest/checksum, CLI,
launcher, disabled-transport and ten-frame native-window smoke checks.

These results establish local independence and the stated runtime behavior.
Hosted Windows/Linux CI, physical-device acceptance and protected performance
certification have not been executed as part of this change.
