#!/usr/bin/env bash
set -euo pipefail

if (($# == 0)); then
  echo "usage: run-wayland-seat.sh COMMAND [ARG ...]" >&2
  exit 2
fi
runtime="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/rterm-functional-wayland-$RANDOM"
mkdir -m 700 "$runtime"
export XDG_RUNTIME_DIR="$runtime"
export DISPLAY=:98
export WAYLAND_DISPLAY=wayland-rterm-functional
export RTERM_FUNCTIONAL_XDOTOOL="${RTERM_FUNCTIONAL_XDOTOOL:-$(command -v xdotool)}"
weston_shell="${RTERM_FUNCTIONAL_WESTON_SHELL:-kiosk-shell.so}"

dump_startup_logs() {
  for log in xvfb weston; do
    if [[ -s "$runtime/$log.log" ]]; then
      echo "--- $log startup log ---" >&2
      cat "$runtime/$log.log" >&2
    fi
  done
}

require_live_process() {
  local pid="$1" name="$2"
  if kill -0 "$pid" 2>/dev/null; then
    return 0
  fi
  echo "$name exited before the nested Wayland seat became ready" >&2
  dump_startup_logs
  return 1
}

wait_for_x11_display() {
  for _ in {1..100}; do
    if xdotool getmouselocation --shell >/dev/null 2>&1; then
      return 0
    fi
    require_live_process "$xvfb_pid" Xvfb || return
    sleep 0.05
  done
  echo "Xvfb display $DISPLAY did not become ready" >&2
  dump_startup_logs
  return 1
}

wait_for_weston_socket() {
  for _ in {1..300}; do
    if [[ -S "$runtime/$WAYLAND_DISPLAY" ]]; then
      return 0
    fi
    require_live_process "$weston_pid" Weston || return
    sleep 0.05
  done
  echo "Weston socket $WAYLAND_DISPLAY did not become ready" >&2
  dump_startup_logs
  return 1
}

wait_for_weston_window() {
  local window
  for _ in {1..300}; do
    window="$(xdotool search --onlyvisible --class weston 2>/dev/null | head -n1 || true)"
    if [[ -n "$window" ]]; then
      export RTERM_FUNCTIONAL_WESTON_WINDOW="$window"
      return 0
    fi
    require_live_process "$weston_pid" Weston || return
    sleep 0.05
  done
  echo "Weston created its socket but never mapped an X11 window" >&2
  dump_startup_logs
  return 1
}

Xvfb "$DISPLAY" -screen 0 1280x800x24 >"$runtime/xvfb.log" 2>&1 &
xvfb_pid=$!
weston_pid=""
cleanup() {
  status=$?
  trap - EXIT
  if [[ -n "$weston_pid" ]]; then
    kill -TERM "$weston_pid" 2>/dev/null || true
    wait "$weston_pid" 2>/dev/null || true
  fi
  kill -TERM "$xvfb_pid" 2>/dev/null || true
  wait "$xvfb_pid" 2>/dev/null || true
  if ! rm -rf -- "$runtime"; then
    echo "warning: could not remove every private Wayland runtime entry" >&2
  fi
  exit "$status"
}
trap cleanup EXIT
wait_for_x11_display
weston --backend=x11-backend.so --shell="$weston_shell" --socket="$WAYLAND_DISPLAY" --idle-time=0 >"$runtime/weston.log" 2>&1 &
weston_pid=$!
wait_for_weston_socket
export RTERM_FUNCTIONAL_WESTON_BACKEND=x11
export RTERM_FUNCTIONAL_COMPOSITOR_LOG="$runtime/weston.log"
wait_for_weston_window
"$@"
