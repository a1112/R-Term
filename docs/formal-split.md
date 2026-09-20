# Repository split — 2026-09-20

Ownership: local terminal, rterm command and all graphical components.
Validated extraction baseline: `3aa55d2e2111c1fe064a0a1594753c47be67d4d8`.

The user authorized the formal repository cutover and publication to
`a1112/R-SSH` and `a1112/R-Term`. Existing Git ancestry is retained; the split
changes the current trees and future ownership, not historical commits.
R-Term defaults to a local terminal with SSH disabled. Its optional adapter
consumes an immutable Git revision from the public R-SSH repository.

## Rollback and evidence

Before-cutover source: `c96972ad` on `codex/trial-split-gui`.
Previous published R-SSH main: `a24308fc4cab0b6c60aa8722c193f9778b3a898c`.
Local complete source backup: `H:/project/R-SSH-before-split-20260920`.
The remote `archive/pre-split-20260920` branch preserves the source snapshot.
Restore files through a new commit if rollback is needed; do not rewrite shared history.

The extraction and optional-SSH Windows matrices passed before promotion.
Cutover validation receipts are in `H:/project/R-SSH/evidence/formal-split-20260920`.
New Windows CI validates these repository boundaries; it does not substitute
for protected cross-platform certification. Linux/macOS native certification
and Stage 7 certification remain outstanding. A formal split does not turn
those previous NO-GO findings into passing evidence.
