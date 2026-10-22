#!/usr/bin/env bash
set -euo pipefail

if (($# == 0)); then
  echo "usage: run-x11-seat.sh COMMAND [ARG ...]" >&2
  exit 2
fi

runtime="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/rterm-functional-x11-$RANDOM"
mkdir -m 700 "$runtime"
export XDG_RUNTIME_DIR="$runtime"
export GDK_BACKEND=x11
export NO_AT_BRIDGE=1
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export RTERM_FUNCTIONAL_XDOTOOL="${RTERM_FUNCTIONAL_XDOTOOL:-$(command -v xdotool)}"

cleanup() {
  status=$?
  trap - EXIT
  if ((status != 0)) && [[ -s "$runtime/openbox.log" ]]; then
    echo "--- openbox startup log ---" >&2
    sed -n '1,200p' "$runtime/openbox.log" >&2
  fi
  if ! rm -rf -- "$runtime"; then
    echo "warning: could not remove every private X11 runtime entry" >&2
  fi
  exit "$status"
}
trap cleanup EXIT

xvfb-run --auto-servernum \
  --server-args="-screen 0 1280x800x24 -nolisten tcp" \
  dbus-run-session -- bash -c '
    wait_for_openbox() {
      local openbox_pid=$1
      for _ in {1..300}; do
        if ! kill -0 "$openbox_pid" 2>/dev/null; then
          wait "$openbox_pid" || true
          echo "Openbox exited before publishing its X11 window-manager property" >&2
          return 1
        fi
        if xprop -root _NET_SUPPORTING_WM_CHECK 2>/dev/null | grep -q "window id #"; then
          return 0
        fi
        sleep 0.05
      done
      echo "Openbox did not publish its X11 window-manager property within 15 seconds" >&2
      return 1
    }

    openbox >"$XDG_RUNTIME_DIR/openbox.log" 2>&1 &
    openbox_pid=$!
    wait_for_openbox "$openbox_pid"
    exec "$@"
  ' bash "$@"
