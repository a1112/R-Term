#!/usr/bin/env bash
set -euo pipefail

if (($# < 2)); then
  echo "usage: x11-xtest-input.sh PID ACTION [ARG ...]" >&2
  exit 2
fi
pid="$1"
action="$2"
window_title="${RTERM_FUNCTIONAL_X11_WINDOW_TITLE:-}"
shift 2

process_tree() {
  local -a processes=("$pid")
  local index=0 child existing
  while ((index < ${#processes[@]})); do
    while IFS= read -r child; do
      [[ -n "$child" ]] || continue
      existing=false
      for known in "${processes[@]}"; do
        [[ "$known" == "$child" ]] && existing=true && break
      done
      [[ "$existing" == true ]] || processes+=("$child")
    done < <(pgrep -P "${processes[$index]}" 2>/dev/null || true)
    ((index += 1))
  done
  printf '%s\n' "${processes[@]}"
}

find_visible_window() {
  local process window actual_title
  local -a matches=()
  for _ in {1..600}; do
    matches=()
    while IFS= read -r process; do
      while IFS= read -r window; do
        [[ -n "$window" ]] && matches+=("$window")
      done < <(xdotool search --onlyvisible --pid "$process" 2>/dev/null || true)
    done < <(process_tree)
    if ((${#matches[@]} == 0)) && [[ -n "$window_title" ]]; then
      while IFS= read -r window; do
        [[ -n "$window" ]] || continue
        actual_title="$(xdotool getwindowname "$window" 2>/dev/null || true)"
        [[ "$actual_title" == "$window_title" ]] && matches+=("$window")
      done < <(xdotool search --onlyvisible --name "$window_title" 2>/dev/null || true)
    fi
    if ((${#matches[@]} == 1)); then
      printf '%s\n' "${matches[0]}"
      return 0
    fi
    if ((${#matches[@]} > 1)); then
      echo "expected one visible X11 window for PID tree $pid; observed ${#matches[@]}" >&2
      return 3
    fi
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.05
  done
  echo "expected one visible X11 window for PID tree $pid; observed 0" >&2
  return 3
}

window="$(find_visible_window)"
xdotool windowactivate --sync "$window"

case "$action" in
  focus) ;;
  type) xdotool type --clearmodifiers --delay 0 -- "$*" ;;
  key)
    case "$*" in
      Enter|enter) key=Return ;;
      *) key="$*" ;;
    esac
    xdotool key --clearmodifiers "$key"
    ;;
  click)
    xdotool mousemove --window "$window" "$1" "$2"
    case "$3" in left) button=1;; middle) button=2;; right) button=3;; *) exit 2;; esac
    xdotool click "$button"
    ;;
  drag)
    case "$5" in left) button=1;; middle) button=2;; right) button=3;; *) exit 2;; esac
    xdotool mousemove --window "$window" "$1" "$2"
    xdotool mousedown "$button"
    xdotool mousemove --window "$window" "$3" "$4"
    xdotool mouseup "$button"
    ;;
  wheel)
    x="$1"; y="$2"
    for ((i=0; i<${y#-}; i++)); do ((y < 0)) && button=5 || button=4; xdotool click "$button"; done
    for ((i=0; i<${x#-}; i++)); do ((x < 0)) && button=7 || button=6; xdotool click "$button"; done
    ;;
  paste)
    command -v xclip >/dev/null
    printf %s "$*" | xclip -selection clipboard
    xdotool key --clearmodifiers ctrl+v
    ;;
  resize) xdotool windowsize "$window" "$1" "$2" ;;
  window)
    case "$1" in
      minimize) xdotool windowminimize "$window" ;;
      maximize|restore) xdotool key --clearmodifiers alt+F10 ;;
      close) xdotool windowclose "$window" ;;
      *) exit 2 ;;
    esac
    ;;
  *) echo "unsupported XTEST action: $action" >&2; exit 2 ;;
esac

# Let the target client consume the injected events before the next helper
# process can move focus or enqueue another action.
sleep 0.1
