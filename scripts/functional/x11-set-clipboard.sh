#!/usr/bin/env bash
set -euo pipefail

wayland_clipboard="${RTERM_FUNCTIONAL_WAYLAND_CLIPBOARD:-0}"
if [[ "$wayland_clipboard" == 1 && "${1:-}" == --clear ]]; then
  sleep 0.25
  wl-copy --clear
  exit 0
fi

if (($# != 1)); then
  echo "usage: x11-set-clipboard.sh TEXT" >&2
  exit 2
fi

if [[ "$wayland_clipboard" == 1 ]]; then
  printf '%s' "$1" | wl-copy
else
  printf '%s' "$1" | xclip -selection clipboard -loops 1
fi
