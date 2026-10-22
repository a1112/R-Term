# Historical R-Term 0.1 extraction contract

This document records the pre-extraction Stage 6/7 procedure and its original
commit identities. R-Term is now independent; current build commands and
boundaries are in [README](../../README.md) and
[repository independence](../formal-split.md). New source paths use `rterm-*`.
The standalone `contracts/rterm-consumer` probe remains supported.

R-Term remains a logical package family inside the R-SSH repository during
Stage 6. This contract validates a future repository boundary without moving
code or rewriting history. A physical extraction is Stage 7 work and is not
authorized by this contract.

## Compatibility policy

All public R-Term packages currently publish the `0.1` compatibility line.
Patch releases may add backward-compatible APIs and fix behavior. Removing or
changing a public API requires the next minor line, beginning with `0.2`.
Downstream rehearsals always identify both candidate and last-known-good inputs
with immutable 40-character Git commits; branches and tags are not release
evidence.

The package list, owned paths, internal dependency edges, last-known-good commit,
and vendored dependency trees are defined in
`scripts/ci/rterm-release-contract.json`. The contract forbids every dependency
from an `rterm-*` package to an `rssh-*` package.

The initial LKG is the verified Task 0 boundary commit
`0e8ebd5de22758275cbb6a849c19c032268d7fac`. The earlier Stage 5 merge is not a
valid rollback source for this contract because its runtime package still had
reverse dependencies on product crates.

## Consumer and vendor policy

The standalone consumer compiles against the seven public `rterm-*` packages at
version `0.1.0`. The real R-SSH consumer rehearsal overlays only contract-owned
R-Term paths into an independent clean checkout.

`glyphon` and `gpu-allocator` remain repository patches. A future consumer must
declare the same patches in its root manifest using the
`consumer-root-path-patch` strategy; transitive package manifests must not try to
own those patches. Each patch is pinned by its Git tree identity, so content
drift fails the contract even when the directory name is unchanged.

The production contract selects `verified-dual-adapter-v1`. Both rehearsal
modes invoke `prepare-rterm-consumer.py` against immutable source/consumer refs
and the committed contract. Modern feature forwarding is preserved; the exact
frozen source uses the app-owned legacy selector and allowlisted feature-array
edits. The preparation receipt (including generated lockfile hash and source
tree identities) is embedded in each candidate/rollback result. Preparation is
not itself a compatibility certificate.

Prepared source and consumer checkouts live under an owned external temporary
directory. Build outputs use `CARGO_TARGET_DIR` (or a separate directory there),
not either verified checkout. Standalone probe overlays remain in a separate
probe checkout, never in the verified frozen source. After consumer commands,
the rehearsal verifies the source tree and retained product bytes again and
rejects changes outside the original manifest/lock allowlist, including a
changed generated lockfile. Existing consumer commands still must all succeed.
Failures retain receipts, command diagnostics and temporary checkouts; success
removes owned temporary checkouts but preserves the evidence JSON.

`consumer_artifacts` binds required Cargo-target-relative files to a zero-based
consumer command index. The production contract requires the executable from
command 4 (`cargo build`), with `{exe_suffix}` expanded for the host platform.
Before that command, an existing declared regular file is moved to the owned
temporary recovery directory and recorded in `prior_artifacts`. Cargo must
recreate the declared path; an old profile's output or a redirected target
cannot silently satisfy the requirement. The dependency cache remains in place.
Failure retains the recovery path; successful cleanup removes these old copies.
Immediately after that command succeeds, each mode records the file's SHA-256,
byte size, relative path and command index in `artifacts`. Missing, empty or
linked files fail the rehearsal. The recorded identity is checked again after
all consumer commands; later mutation fails while preserving the original hash.
Candidate evidence is written before rollback can overwrite a shared target.
The six original consumer commands are unchanged. These records describe the
tested executable, not a retained package archive or a performance certificate;
archive identities use the separate `package` record described below.

The production contract now selects `consumer_package: native-unsigned-v1`.
After the unchanged consumer commands, the rehearsal invokes the existing
`package-native.ps1` or `package-native.sh` from the verified consumer checkout.
Each mode owns a fresh package directory and an unsigned native archive. The
packager receives the exact consumer commit for manifest provenance. Archive
members are read without extraction and compared with the assembled payload;
the packaged executable must match the earlier build hash. The retained
`package` record binds archive SHA-256/size, binary SHA-256, runtime target,
profile and source/consumer commits. CI uploads both mode archives alongside
the evidence JSON. Failure cannot become a successful rehearsal, and source
and original executable identity are rechecked after packaging.
Before overall success and temporary-checkout cleanup, both retained archives
are checked again for their original size/hash and non-linked paths. A later
mode cannot modify an earlier archive without invalidating its evidence.
Malformed package manifests produce structured failure evidence as well.

These are unsigned rehearsal packages built by the existing debug-profile
consumer command, not signed release packages or fixed-runner performance
evidence. Package assembly alone does not claim that packaged scenarios ran.

`consumer_package_tests: native-gui-ssh-gpu-v1` additionally requires three
existing Rust scenarios for each profile: native SSH disconnect/reconnect,
ten GPU-presented frames from a real local PTY, and GPU text at 100% scale. Each
command uses the packaged executable via `RSSH_TEST_APP_EXECUTABLE`, requires
OpenSSH tools, and retains its result under `package.tests`. Cargo uses the same
production GUI/transfer feature set as the original build. The GPU scale test
is explicitly selected with `--ignored`; every exact selection must report one
passed, zero failed and zero ignored tests. Missing/filtered/ignored scenarios
are failures, not successful skips. Archive/payload identity is checked again
after testing, followed by the existing source and final retained-archive checks.
Tests execute the verified staging payload. For Unix archives, regular-file
permissions must also match the payload, and the binary/launchers must retain
execute bits; byte equality alone cannot establish an executable package.

Hosted Linux runs these scenarios under Xvfb/X11 with Vulkan support. Software
GPU adapters on hosted runners establish functional rendering only, not physical
GPU or performance certification. These three scenarios do not cover every SSH
GUI prompt/authentication combination or replace the fixed-runner gates.

## History extraction

`docs/release/rterm-history-paths.txt` is the reviewed old-to-current path map for
future history extraction. It is evidence and planning input only: Stage 6 does
not run `filter-repo`, create a second repository, publish crates, or change the
authorized single-repository topology.
