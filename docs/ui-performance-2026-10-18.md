# Compact title bar and local performance measurements

The approved [interface reference](design/rterm-ui-main-2026-10-18-v2.png)
places session tabs in one top row without an application-name block or a
session sidebar. Native and Web/Tauri retain actual window controls, session
switching and terminal input. Saved configuration remains compatible.

## Native idle CPU and hyperlink rules

A native window trace identified repeated `regex::Regex::new` calls in the
per-row hyperlink matching path. The window now retains one compiled rule set
in its host state. Rendering and mouse hit testing share it, invalid patterns
are skipped without repeated compilation, and configuration updates invalidate
the cache. The cache does not accumulate configurations or use a global map.

The default unoptimized native build was measured with the same command before
and after this change:

```sh
rterm -n window -- /bin/sh -c 'printf "https://example.invalid/docs\n"; sleep 30'
```

After a three-second warmup, process CPU time was sampled for approximately
five seconds with `ps -o time=`. This short local observation was:

| Variant | Process CPU time | Wall time | CPU utilization |
| --- | ---: | ---: | ---: |
| Before rule caching | 5.10 s | 5.04 s | 101.2% |
| After rule caching | 0.12 s | 5.07 s | 2.4% |

Raw results and executable hashes are in
`evidence/ui-performance-2026-10-18/native-idle-before.json` and
`native-idle-after.json`; the sampled stack is retained in
`native-before-sample.txt`. CPU percentage uses process CPU seconds divided
by wall seconds, so one fully occupied core is approximately 100%. This is a
single-machine, short debug-build observation with a fixed idle terminal, not
an idle-CPU guarantee for every workload or platform.

Both runs logged a successful GPU first present in under one second, before
the sampling window started. The follow-up native interaction check created
two local sessions, entered distinct commands, switched the active top tab,
and opened the existing launcher from the plus button.

## Runtime polling

`WindowPaneRuntime::poll` now iterates pending queues directly and skips empty
queues before looking up or cloning a pane handle. It no longer allocates a
temporary pane-ID vector on every poll. Queued input still flushes in FIFO
order, and a closed worker retains its empty queue until its close notice is
consumed, preserving the child's exit status during resize races.

The manual benchmark uses eight real scripted pane workers, 1,000 warmup
polls, and nine samples of 20,000 polls. Five before/after process pairs were
run in alternating order on this macOS machine, with the Rust test profile
(`unoptimized`, `debug=1`). The median of process medians was:

| Workload | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| Eight idle pane workers, one poll | 3,234 ns | 320 ns | 90.1% |

Reproduce the current implementation with:

```sh
cargo test --locked -p rterm-app --features developer-full --bin rterm \
  runtime_composition::tests::idle_multi_pane_poll_benchmark \
  -- --ignored --exact --nocapture
```

## Incremental render snapshots

`TerminalSnapshotCache::canonicalize` recognizes unchanged rows by their
shared `Arc` identity before scanning row contents. Changed rows continue
through the existing exact comparison; snapshot content and memory budgets
are unchanged.

The existing optimized `snapshot_memory` benchmark ran five processes per
variant. Each process builds 200 full snapshots and performs 1,000 damage
updates at each terminal size. Medians of the per-process means were:

| Terminal size | Damage update before | Damage update after | Reduction |
| --- | ---: | ---: | ---: |
| 80 × 24 | 10,253 ns | 8,091 ns | 21.1% |
| 200 × 60 | 17,990 ns | 12,057 ns | 33.0% |

Full-build medians remained approximately 80 µs and 208 µs, within 0.6% of
the respective baselines. Retained snapshot/image bytes and row-reuse metrics
were identical. The benchmark checks the final incremental snapshot against
a complete rebuild.

```sh
cargo bench --locked -p rterm-render-core --bench snapshot_memory
```

Raw local samples are in
`evidence/ui-performance-2026-10-18/idle-poll-ab.json` and
`evidence/ui-performance-2026-10-18/snapshot-cache-ab.json`; `evidence/` is
intentionally ignored by Git. These are synthetic CPU microbenchmarks on one
machine, not measurements of total application CPU, startup time, GPU present
latency, or protected cross-platform performance certification.

## Validation

- Complete `developer-full` application unit suite: 4,157 passed, zero failed;
  three ignored cases include the manual polling benchmark.
- Native window suite: 13 passed, six dedicated-runner cases ignored. Real
  PTY/GPU presentation, exit code 7 and child cleanup remained correct.
- Strict application Clippy (`--all-targets -- -D warnings`), formatting,
  architecture policy and four feature/dependency graphs passed.
- Frozen Task 10 provenance remained unchanged: 356 tests and 356 traces.
- Web lint, unit tests, production and functional builds passed. Real Chrome
  end-to-end checks exercised multi-session input, background-tab closure,
  resize, exit and restart against the Rust PTY backend.
- The final Web distribution is a production build without the functional
  snapshot hook.

Actual screenshots are stored locally at
`evidence/ui-2026-10-18/native-titlebar.png` and
`evidence/ui-2026-10-18/web-titlebar.png`. The latest temporary native preview bundle
is `target/workspace-preview/R-Term Workspace.app`; it is an unsigned local development build.
Windows/Linux runtime and hosted CI were not exercised during this change.

## Native maximized workspace follow-up

A real 3456 × 2014 three-pane window exposed repeated text preparation even
when its terminal content was unchanged. The initial row-cache attempt did
not improve the paired measurement: 102.642% versus 102.332% of one CPU core.
Short-lived diagnostics then showed that regular/rapid text-blink opacity
changes invalidated the complete cache, including frames without blinking
cells.

Unchanged full frames now reuse prepared rows. Style, palette, geometry, DPI,
zoom and font-catalog changes still invalidate the required work; regular and
rapid blink changes rebuild their affected rows. Retained missing-glyph
metadata still reaches font fallback. Reports separately expose preparation
work and glyphs retained for drawing, including partial updates and clearing.
This keeps native presentation diagnostics correct on cache hits. The existing
full-composition rule for split panes remains in place.

Two intermediate five-second samples at the same captured window/pane geometry
measured 0.0% and 0.4% of one core. A three-second process profile showed waiting
stacks rather than the previous text-preparation hotspot. These intermediate
samples enabled temporary trace logging. A non-instrumented build then
measured 0.199% of one core over 5.022 seconds (0.010 seconds of process CPU)
at the same captured 3456 × 2014 geometry and 94 × 33 active pane. Its
three-second profile contained no text-preparation hotspot. This is an idle
macOS development-build measurement, not a throughput or release benchmark. Raw comparison evidence is under
`evidence/workspace-console-2026-10-18/native-maximized-*`.
