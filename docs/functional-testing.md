# Functional testing

R-Term's functional gate is a versioned, deterministic test system. The non-published
`rterm-functional-tests` crate builds two binaries:

- `rterm-functional`: validates, shards, executes, and aggregates scenarios.
- `rterm-functional-fixture`: provides hermetic PTY/terminal fixture modes.

## Suite layout

`functional-tests/behaviors.toml` assigns stable behavior IDs to public commands,
user actions, host effects, lifecycle results, and subsystem journeys. Scenario
intent is stored in `functional-tests/scenarios/*.toml` as `ScenarioV1`; actions and
checkpoints are closed Rust enums, so scenarios cannot execute arbitrary scripts or
sleep for fixed intervals. `functional-tests/hosted-matrix.toml` is the approved
PR execution matrix; `functional-tests/matrix.toml` adds privileged macOS targets
for manual full-matrix validation. `functional-tests/evidence-map.toml` maps protocol-level
libtests and Playwright tests to behavior IDs.

Useful local commands:

```text
cargo run --locked -p rterm-functional-tests --bin rterm-functional -- validate --suite functional-tests
cargo run --locked -p rterm-functional-tests --bin rterm-functional -- list --suite functional-tests
cargo run --locked -p rterm-functional-tests --bin rterm-functional -- shard --suite functional-tests --count 4
```

A scenario run requires an explicit target, evidence directory, application path,
and every required capability. Missing capabilities are infrastructure failures,
never skips. Each run has one absolute scenario deadline and emits exactly one
terminal NDJSON event. Semantic failures, timeouts, lost input, and leaks are not
retried.

## Observer and entry paths

Native and Tauri functional builds enable `functional-test-observer`. Windows uses
a current-user-only named pipe; Unix uses a UDS in a `0700` directory. A 256-bit
one-time token permits only `hello`, `snapshot`, and `subscribe`. The channel is
read-only and excludes credentials and environment data. Keyboard, pointer,
clipboard, PTY, SSH, and WebSocket input still enters through real platform APIs.

Production builds do not enable the feature. CI checks the Cargo feature tree,
binary protocol markers, and a startup probe for native, Web, and Tauri artifacts.

Every functional child receives a loopback-only network environment, browser
contexts abort non-loopback requests, and `check-functional-hermeticity.py` rejects
public endpoints in runtime test assets. Native scenarios use an explicit temporary
configuration and SSH/SFTP/SCP use isolated homes and loopback servers.

## Evidence and CI

Success evidence contains the monotonic NDJSON stream plus each scenario's declared
stdout, stderr, final snapshot, server trace, process tree, or file digest. Failure
paths additionally capture the whole window and compositor diagnostics. Clipboard
contents are restored by RAII cleanup. Final checkpoints require owned children,
workers, readers, listeners, ports, and temporary endpoints to be gone.

`.github/workflows/functional.yml` runs fixed shards for CLI/transport on three
platforms; native Windows, X11, nested Weston/Wayland, and hosted macOS; three
Playwright engines; Tauri; and unsigned production packages. Every job has an
18-minute hard timeout. PR aggregation enforces the exact hosted matrix. Manual
dispatch additionally runs authorized self-hosted macOS input jobs and enforces the
full matrix. Both aggregates reject orphaned or declarations-only behavior coverage.

## Adding a behavior

1. Add a stable `BHV-*` row to `behaviors.toml`.
2. Add an actual scenario action/checkpoint or a precise libtest/Playwright mapping.
3. For a scenario, add it to every approved target in `matrix.toml` and to the
   corresponding workflow job.
4. Run the functional crate tests, both Python CI test modules, the hermeticity
   checker, and suite validation.
5. Do not add sleeps, arbitrary script fields, semantic retries, external network
   endpoints, or evidence inferred only from a test name.
