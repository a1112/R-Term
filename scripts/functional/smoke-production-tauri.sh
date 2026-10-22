#!/usr/bin/env bash
set -euo pipefail

binary=""
evidence_directory=""
input_backend=""
while (($#)); do
  case "$1" in
    --binary) binary="$2"; shift 2 ;;
    --evidence-directory) evidence_directory="$2"; shift 2 ;;
    --input-backend) input_backend="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
[[ -n "$binary" && -n "$evidence_directory" && -n "$input_backend" ]] || {
  echo "usage: smoke-production-tauri.sh --binary PATH --evidence-directory PATH --input-backend x11|macos" >&2
  exit 2
}

binary="$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary")"
mkdir -p "$evidence_directory"
evidence_directory="$(cd "$evidence_directory" && pwd)"
stdout="$evidence_directory/stdout"
stderr="$evidence_directory/stderr"
process_tree="$evidence_directory/process-tree.json"
screenshot="$evidence_directory/failure-screenshot.png"
root_pid=""
owned_pids=()
owned_start_times=()

descendants() {
  local parent child
  local -a pending=("$1")
  while ((${#pending[@]})); do
    parent="${pending[0]}"
    pending=("${pending[@]:1}")
    while IFS= read -r child; do
      [[ -n "$child" ]] || continue
      printf '%s\n' "$child"
      pending+=("$child")
    done < <(pgrep -P "$parent" 2>/dev/null || true)
  done
}

session_descendants() {
  local pid command
  while IFS= read -r pid; do
    command="$(ps -p "$pid" -o comm= 2>/dev/null || true)"
    case "$command" in
      *WebKit*|*bwrap*|*xdg-dbus-proxy*) ;;
      *) printf '%s\n' "$pid" ;;
    esac
  done < <(descendants "$1")
}

wait_condition() {
  local seconds="$1" failure="$2"
  shift 2
  local deadline=$((SECONDS + seconds))
  while ((SECONDS < deadline)); do
    "$@" && return 0
    sleep 0.05
  done
  "$@" && return 0
  echo "$failure" >&2
  return 1
}

root_alive() { kill -0 "$root_pid" 2>/dev/null; }
root_exited() { ! kill -0 "$root_pid" 2>/dev/null; }
session_started() { [[ -n "$(session_descendants "$root_pid")" ]]; }
session_stopped() { [[ -z "$(session_descendants "$root_pid")" ]]; }
process_start_time() {
  local pid="$1" stat rest state
  [[ -r "/proc/$pid/stat" ]] || return 1
  IFS= read -r stat <"/proc/$pid/stat" || return 1
  rest="${stat##*) }"
  set -- $rest
  (($# >= 20)) || return 1
  state="$1"
  [[ "$state" != Z && "$state" != X ]] || return 1
  printf '%s\n' "${20}"
}

process_is_owned_live() {
  local index="$1" current
  current="$(process_start_time "${owned_pids[$index]}")" || return 1
  [[ "$current" == "${owned_start_times[$index]}" ]]
}

all_owned_exited() {
  local index
  for index in "${!owned_pids[@]}"; do
    process_is_owned_live "$index" && return 1
  done
  return 0
}

dump_live_owned_processes() {
  local index pid
  for index in "${!owned_pids[@]}"; do
    process_is_owned_live "$index" || continue
    pid="${owned_pids[$index]}"
    ps -p "$pid" -o pid=,ppid=,stat=,comm=,args= >&2 || true
  done
}

input() {
  case "$input_backend" in
    x11)
      RTERM_FUNCTIONAL_X11_WINDOW_TITLE=R-Term \
        bash scripts/functional/x11-xtest-input.sh "$root_pid" "$@"
      ;;
    macos)
      [[ "${RTERM_FUNCTIONAL_MACOS_ACCESSIBILITY:-}" == authorized ]] || {
        echo "macOS Accessibility authorization is required" >&2
        return 3
      }
      /usr/bin/xcrun swift "${RTERM_FUNCTIONAL_MACOS_CGEVENT_HELPER:?missing CGEvent helper}" --pid "$root_pid" "$@"
      ;;
    *) echo "unsupported input backend: $input_backend" >&2; return 2 ;;
  esac
}

save_process_tree() {
  local remaining=0 index pid separator=""
  printf '{"schema":1,"root_process_id":%s,"owned_process_ids":[' "$root_pid" >"$process_tree"
  for index in "${!owned_pids[@]}"; do
    pid="${owned_pids[$index]}"
    printf '%s%s' "$separator" "$pid" >>"$process_tree"
    separator=,
    process_is_owned_live "$index" && remaining=$((remaining + 1))
  done
  root_exited || remaining=$((remaining + 1))
  printf '],"remaining_owned_processes":%s,"reaped":%s,"pty_interaction":"exit 7"}\n' \
    "$remaining" "$([[ "$remaining" == 0 ]] && echo true || echo false)" >>"$process_tree"
}

save_screenshot() {
  if [[ "$input_backend" == x11 ]] && command -v import >/dev/null; then
    import -window root "$screenshot" || true
  elif [[ "$input_backend" == macos ]] && command -v screencapture >/dev/null; then
    screencapture -x "$screenshot" || true
  fi
}

cleanup() {
  status=$?
  trap - EXIT
  if [[ -n "$root_pid" ]]; then
    root_exited || kill -TERM "$root_pid" 2>/dev/null || true
    wait "$root_pid" 2>/dev/null || true
    save_process_tree
  fi
  ((status == 0)) || save_screenshot
  exit "$status"
}
trap cleanup EXIT

"$binary" >"$stdout" 2>"$stderr" &
root_pid=$!
wait_condition 30 "production Tauri process exited before its window became interactive" root_alive
wait_condition 30 "production Tauri did not start a PTY child" session_started
while IFS= read -r owned_pid; do
  [[ -n "$owned_pid" ]] || continue
  owned_start_time="$(process_start_time "$owned_pid")" || continue
  owned_pids+=("$owned_pid")
  owned_start_times+=("$owned_start_time")
done < <(descendants "$root_pid")

input focus
input type "exit 7"
input key enter
wait_condition 15 "production Tauri PTY child did not exit after OS keyboard input" session_stopped
input window close
wait_condition 10 "production Tauri did not exit after its OS close action" root_exited
wait "$root_pid"
if ! wait_condition 45 "production Tauri left an owned helper process" all_owned_exited; then
  dump_live_owned_processes
  exit 1
fi
