# Native Release Packages

This document defines the six target native terminal packages and their release
workflow contract for R-Term. It does not record six-platform certification.

## Verification status (2026-08-02)

The local package checkout used code baseline
`83ade73a9d11e165dc66e82e8f6ca1b910c2946c`; current package evidence is:

- **verified locally on Windows x64**: a freshly assembled, separately extracted
  Windows x64 unsigned package passed the packaged-binary smoke checks. Its
  manifest records `source_commit=local`, so it is local evidence rather than
  an exact-commit provenance claim;
- **defined in hosted workflow but not run in this local session**: the six-target
  unsigned build/package/smoke matrix in
  [`.github/workflows/release.yml`](../.github/workflows/release.yml). Commit
  `f02acff6` guards both Linux package-smoke installation points against
  starting the system OpenSSH service; its local static contract passed, but
  neither hosted release job ran in this session;
- **requires protected/self-hosted environment**: fixed performance, Windows
  signing, Linux cosign, macOS signing/notarization/stapling, SBOM, provenance
  attestation, and publication jobs, together with their environments, secrets,
  reviewers, and `v*` tag ruleset prerequisites;
- **not yet evidenced**: package-smoke results for Windows arm64, Linux x64,
  Linux arm64, macOS x64, or macOS arm64, and any signed, notarized, attested, or
  published artifact produced from exact commit
  `83ade73a9d11e165dc66e82e8f6ca1b910c2946c`.

No manual unsigned workflow run or protected tag-release DAG was executed in
this local session. See the cross-requirement evidence ledger in
[`production-parity-verification.md`](production-parity-verification.md) and the
performance qualification in
[`performance-baseline.md`](performance-baseline.md).

## Package

When its protected fixed-performance prerequisite succeeds, the release
workflow is configured to build and smoke these native artifacts:

- `R-Term-windows-x64.zip`
- `R-Term-windows-arm64.zip`
- `R-Term-linux-x64.tar.gz`
- `R-Term-linux-arm64.tar.gz`
- `R-Term-macos-x64.tar.gz`
- `R-Term-macos-arm64.tar.gz`

Every unpacked package contains:

- the native executable and platform console launcher
- `README.md`
- `LICENSE`
- `examples/rterm-profiles.toml`
- `licenses/fonts/`, with the licenses and provenance manifest for the embedded
  deterministic Noto subsets
- `manifest.json`, recording package/version, artifact format, Rust and runtime
  targets, PTY backend, executable path, signing state, required files, and each
  payload file's relative path, size, and SHA-256
- `SHA256SUMS`, covering every package file except itself

Windows packages provide `rterm.exe` and `rterm-console.cmd`. Linux packages
provide `rterm` and `rterm-console.sh`. macOS packages provide
`R-Term.app/Contents/MacOS/rterm`, a root CLI launcher, and
`rterm-console.sh`; the bundle `Info.plist` names the same executable and version.

## Release Triggers

- Manual run: GitHub Actions `Release` workflow through `workflow_dispatch`
  on the repository default branch.
- Versioned release: push a tag that starts with `v`, for example `v0.1.0`.

When the full protected tag-release DAG succeeds, it attaches all six stable
artifact names. Manual artifacts use an
`-unsigned` filename suffix and set `manifest.json`'s `signing.unsigned` to
`true`. They are local/CI test artifacts and are not releasable; renaming an
unsigned archive does not make it eligible for publication.

Release configuration must require designated reviewers on the protected
`performance` environment. A repository ruleset must restrict `v*` tag creation
to authorized release maintainers and require the tagged commit to be reachable
from the protected default branch. These external GitHub settings are release
prerequisites, not facts proved by this repository. The workflow defaults to
read-only contents permission;
fixed-runner checkout does not persist credentials, and only the isolated
tag-publishing job receives `contents: write`.

## Verification Gates

The workflow contract prevents publication until all configured gates pass.
These gates are **defined in hosted workflow but not run in this local session**
for exact commit `83ade73a9d11e165dc66e82e8f6ca1b910c2946c`; protected performance
and release-integrity jobs **require protected/self-hosted environment**:

- protected fixed-runner performance certification with two warmups, seven
  measured samples, approved absolute budgets, and the 10% same-machine
  median-regression rule described in `performance-baseline.md`
- `cargo fmt --all -- --check`
- `cargo test --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo build --locked --release -p rterm-app --all-targets`
- unpacked `manifest.json`, required-file, executable-mode, and `SHA256SUMS`
  validation where the platform exposes executable modes
- packaged `version --json`, `doctor --json`, `self-test --json`, and
  deterministic benchmark gates
- a real packaged-binary OpenSSH loopback and ten-frame native GPU/PTY E2E,
  selected through `RTERM_TEST_APP_EXECUTABLE`
- packaged platform launcher preflight
- protected signing, macOS notarization/stapling, SBOM generation, provenance
  attestation, and a final signed-package smoke

The packaged `version --json` smoke test proves that the downloaded executable
can identify its app version, target, console mode, PTY backend, and native SSH
backend. The packaged `doctor --json` smoke test proves that it can start as a
console app and report terminal size, child terminal environment, default shell,
and OpenSSH tool availability. The packaged `self-test --json` smoke test proves
that the downloaded executable can open a real local PTY, spawn a child command,
capture its output, and launch the OpenSSH `ssh`, `sftp`, and `scp` console
tools without opening a network connection. The packaged `bench --json` smoke
test proves that the downloaded executable can run the deterministic
terminal-runtime benchmark and emit parser throughput, p95 chunk latency,
response count, visible output bytes, bell count, scrollback lines, and final
cursor position as machine-readable metrics, plus offscreen renderer p95 frame
time, rendered pixels, rendered pixel throughput, idle CPU usage, process
resident memory, virtual memory, and accumulated CPU time. It also exercises
the deterministic algorithmic gates and fails the package smoke if
`threshold_violations` is non-empty. Absolute timing, idle CPU, and RSS budgets
are enforced only by the fixed-runner median job; the current 16 ms render
budget is an offscreen `PixelRenderer` proxy rather than a GPU-present claim.
The packaged launcher preflight proves that each platform launcher resolves only
the executable inside the same unpacked package.

## Reproducible local assembly

The assembly scripts accept an already-built native binary, create the package
directory, generate `manifest.json` and `SHA256SUMS`, and write the complete
archive beside that directory. A local Windows x64 example is:

```powershell
cargo build --locked --release -p rterm-app --all-targets
./scripts/ci/package-native.ps1 `
  -Binary ./target/release/rterm.exe `
  -PackageRoot ./dist/R-Term-windows-x64-unsigned `
  -ArtifactName R-Term-windows-x64-unsigned.zip `
  -RuntimeTarget windows-x86_64 `
  -PtyBackend windows-conpty `
  -Version 0.1.0 `
  -Unsigned
New-Item -ItemType Directory ./dist/local-smoke
Expand-Archive `
  -LiteralPath ./dist/R-Term-windows-x64-unsigned.zip `
  -DestinationPath ./dist/local-smoke
./scripts/ci/package-smoke.ps1 `
  -PackageRoot ./dist/local-smoke/R-Term-windows-x64-unsigned `
  -ExpectedTarget windows-x86_64 `
  -ExpectedPtyBackend windows-conpty `
  -ExpectedArtifactName R-Term-windows-x64-unsigned.zip `
  -ExpectedUnsigned
```

The POSIX scripts expose the corresponding `--binary`, `--package-root`,
`--artifact-name`, `--runtime-target`, `--pty-backend`, `--version`,
`--unsigned`, and `--expected-*` flags. Smoke scripts never execute a workspace
replacement binary or extract an archive: `PackageRoot`/`--package-root` must
identify the actual unpacked artifact being certified. Cargo may compile the
Rust integration-test harness, but `RTERM_TEST_APP_EXECUTABLE` forces every
tested app launch to use the validated executable inside that unpacked artifact.

## Console Startup After Download

After extracting the zip:

```powershell
.\rterm.exe doctor
.\rterm.exe version --json
.\rterm.exe self-test --json
.\rterm.exe bench --json --render-frames 30 --idle-ms 200
.\rterm.exe bench --json --workload ansi-scroll-query --bytes 1048576 --chunk-size 8192 --render-frames 30 --idle-ms 1000 --min-throughput-bytes-per-sec 1048576 --max-chunk-p95-us 5000 --max-render-frame-p95-us 16000 --max-idle-cpu-percent 3 --max-process-memory-bytes 268435456
.\rterm.exe console
.\rterm-console.cmd
.\rterm.exe local
.\rterm.exe ssh ops@example.com
.\rterm.exe ssh -p 2222 -i C:\Users\ops\.ssh\id_ed25519 -l ops example.com
.\rterm.exe ssh ops@example.com uptime -p
.\rterm.exe ssh --target prod --preflight
.\rterm.exe sftp ops@example.com
.\rterm.exe sftp -P 2222 -i C:\Users\ops\.ssh\id_ed25519 ops@example.com
.\rterm.exe sftp -F C:\Users\ops\.ssh\prod_config -o ProxyJump=bastion prod
.\rterm.exe sftp -J bastion -C -vv prod
.\rterm.exe sftp -l 4096 prod
.\rterm.exe sftp -b batch.txt -B 32768 -R 64 prod
.\rterm.exe scp local.txt ops@example.com:/tmp/remote.txt
.\rterm.exe scp app.log audit.log ops@example.com:/tmp/logs/
.\rterm.exe scp -l 4096 local.txt prod:/tmp/remote.txt
.\rterm.exe scp -p local.txt prod:/tmp/remote.txt
.\rterm.exe scp -R -s local.txt prod:/tmp/remote.txt
.\rterm.exe scp -P 2222 -i C:\Users\ops\.ssh\id_ed25519 -r logs ops@example.com:/tmp/logs
.\rterm.exe scp -F C:\Users\ops\.ssh\prod_config -o ProxyJump=bastion local.txt prod:/tmp/remote.txt
.\rterm.exe scp -J bastion -C -vv local.txt prod:/tmp/remote.txt
.\rterm.exe scp -O -T -B local.txt prod:/tmp/remote.txt
.\rterm.exe scp ops@example.com:/tmp/remote.txt local.txt
.\rterm.exe scp ops@example.com:/var/log/app.log ops@example.com:/var/log/audit.log logs
.\rterm.exe scp ops@example.com --upload local.txt /tmp/remote.txt
.\rterm.exe ssh -F C:\Users\ops\.ssh\prod_config -o ProxyJump=bastion prod
.\rterm.exe ssh -J bastion -C -vv prod
.\rterm.exe ssh -T -W db.internal:5432 prod
.\rterm.exe ssh -E ssh-debug.log -Q cipher prod
.\rterm.exe ssh -L 127.0.0.1:15432:db.internal:5432 -D 127.0.0.1:1080 -N prod
.\rterm.exe ssh --native --trust-on-first-use --host example.com --user ops --agent
```

## GPU backend memory matrix

The Windows GPU backend memory matrix is a report-only diagnostic. It launches
fresh 80x24 windows at the benchmark-only 100% DPI scale for the CPU, DX12,
Vulkan, and GL probes. A GPU result is accepted only when the final renderer is
GPU and the reported backend exactly matches the requested backend; unsupported
initialization and renderer/backend fallback are retained as explicit probe
failures in `aggregate.json`. The 45 MiB comparison is evidence for backend
attribution and is not a shared pull-request gate.

Keep build outputs and evidence off a space-constrained workspace drive. For
example, from the repository root on Windows:

```powershell
$env:CARGO_TARGET_DIR = 'L:\rterm-targets\stage7-release-certification'
./scripts/ci/run-gpu-backend-memory-matrix.ps1 `
  -Profile release `
  -Warmups 5 `
  -Samples 30 `
  -OutputDirectory 'L:\rterm-evidence\gpu-backend-memory-matrix'
```

The command builds with Cargo's locked dependency graph unless `-SkipBuild` is
passed. Every measured launcher process writes one JSON document beneath
`raw/<probe>-NN.json`; the deterministic summary is `aggregate.json`.

`-AppPath` and `-LauncherPath` are test-only injection points and require
`-SkipBuild`. Evidence produced with them records `binary_source = "override"`
and `certification_eligible = false`; it cannot support Stage 7 or fixed-runner
certification. Normal runs record `binary_source = "cargo-target"`, but are
eligible only for the release profile with at least 5 warmups and 30 measured
runs. Eligibility metadata does not by itself certify a run.

## Release Indicators

Use these indicators to decide whether a build is ready for a wider console
pilot:

- Package build success rate: release workflow passes on the target tag.
- Package identity: packaged `version --json` reports the expected version and
  backends.
- Startup health: packaged `doctor --json` exits successfully.
- Local PTY and tool health: packaged `self-test --json` captures the expected
  PTY marker from a child process and verifies that `ssh -V`, `sftp -h`, and
  `scp -h` can launch.
- Benchmark baseline: packaged `bench --json` reports terminal-runtime
  throughput, p95 chunk processing latency, visible output bytes, response
  count, bell count, scrollback lines, and final cursor position from a
  deterministic ANSI/CSI/OSC workload, plus offscreen render frame p95 and
  rendered pixel throughput from `PixelRenderer`, plus idle CPU usage,
  process resident memory, virtual memory, and accumulated CPU time from the
  current process resource sampler. Protected fixed-runner certification
  enforces approved absolute budgets and rejects medians that regress by more
  than 10% from a fingerprint-matched same-machine baseline.
- Profile readiness: packaged profile checks validate bundled profiles and
  verify that the native-window smoke profile resolves `--metrics-json`.
- Console coverage: explicit `console` launcher, local shell, positional
  `[USER@]HOST` SSH/SFTP/SCP launches, SSH trailing remote commands, SCP
  `HOST:PATH` single-source and multi-source upload/download operands, common
  OpenSSH short options (`ssh -p/-l/-i/-J/-F/-o/-4/-6/-A/-a/-C/-q/-v/-L/-R/-D/-N`,
  SSH control options `-B/-b/-c/-E/-e/-I/-m/-O/-P/-Q/-S/-W/-w/-f/-G/-g/-K/-k/-M/-n/-s/-T/-t/-X/-x/-Y/-y`,
  `sftp -P/-i/-J/-F/-o/-4/-6/-A/-a/-C/-q/-v/-l/-b/-B/-R/-D/-S/-s/-X/-c`, and
  `scp -P/-i/-J/-F/-o/-4/-6/-A/-a/-C/-q/-v/-l/-3/-O/-T/-B/-p/-R/-s/-D/-S/-X/-c/-r`),
  OpenSSH config targets, profiles, metrics, and native russh startup remain
  covered by workspace tests.
- Runtime observability: console sessions can emit `--metrics` or
  `--metrics-json`; native-window sessions can report PTY, render, input, and
  terminal-damage metrics, plus snapshot damage-update and full-rebuild counts;
  native-window render metrics also split full and dirty frame paths;
  non-interactive parser runs can emit `bench --json`.
- Safe startup: `--preflight` is available for console/local, SSH, SFTP, and
  SCP profile or direct launches.
