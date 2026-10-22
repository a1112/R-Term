# Contributing to R-Term

Use Rust 1.89.0 and Node 24. Keep changes focused and add regression tests at
the boundary being changed.

Before opening a pull request, run:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
npm --prefix web ci
npm --prefix web run lint
npm --prefix web test
npm --prefix web run build
```

Security and coverage checks mirror CI:

```sh
cargo audit
cargo deny check
cargo llvm-cov --locked --workspace --fail-under-lines 35
npm --prefix web audit --audit-level=high
npm --prefix tauri audit --audit-level=high
```

Do not commit generated build output, credentials, bootstrap URLs, private host
names, terminal transcripts, or unredacted CI logs. Update documentation when a
CLI default or security boundary changes.
