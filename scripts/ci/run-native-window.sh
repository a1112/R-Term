#!/usr/bin/env bash
set -euo pipefail

profile="debug"
expected_target=""
expected_pty_backend=""
harness_self_test=0

while (($# > 0)); do
  case "$1" in
    --harness-self-test)
      harness_self_test=1
      shift
      ;;
    --profile)
      profile="${2:?missing value for --profile}"
      shift 2
      ;;
    --expected-target)
      expected_target="${2:?missing value for --expected-target}"
      shift 2
      ;;
    --expected-pty-backend)
      expected_pty_backend="${2:?missing value for --expected-pty-backend}"
      shift 2
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

if [[ "$profile" != "debug" && "$profile" != "release" ]]; then
  printf 'invalid profile: %s\n' "$profile" >&2
  exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/process-harness.sh"

run_harness_self_test
if ((harness_self_test)); then
  exit 0
fi
if [[ -z "$expected_target" || -z "$expected_pty_backend" ]]; then
  printf '%s\n' '--expected-target and --expected-pty-backend are required' >&2
  exit 2
fi

repository_root="$(cd -- "$script_dir/../.." && pwd)"
target_directory="${CARGO_TARGET_DIR:-$repository_root/target}"
profile_arguments=()
profile_directory="debug"
if [[ "$profile" == "release" ]]; then
  profile_arguments+=(--release)
  profile_directory="release"
fi
binary="$target_directory/$profile_directory/rterm"

cd "$repository_root"
run_bounded "native E2E build ($profile)" 1200 \
  cargo build --locked -p rterm-app --bin rterm --test openssh_loopback --test native_window_e2e "${profile_arguments[@]}"

version_capture="$(mktemp)"
run_bounded "version identity" 30 "$binary" version --json >"$version_capture"
cat "$version_capture"
python3 - "$version_capture" "$expected_target" "$expected_pty_backend" <<'PY'
import json
import pathlib
import sys

report = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
if report.get("target") != sys.argv[2]:
    raise SystemExit(
        f"version target mismatch: observed {report.get('target')!r}, expected {sys.argv[2]!r}"
    )
if report.get("pty_backend") != sys.argv[3]:
    raise SystemExit(
        "PTY backend mismatch: "
        f"observed {report.get('pty_backend')!r}, expected {sys.argv[3]!r}"
    )
PY

run_bounded "OpenSSH client probe" 15 ssh -V
run_bounded "hermetic native SSH tests ($profile)" 300 \
  cargo test --locked -p rterm-ssh --all-targets "${profile_arguments[@]}" -- --nocapture
run_bounded "system OpenSSH interoperability ($profile)" 300 \
  env RTERM_REQUIRE_OPENSSH=1 \
  cargo test --locked -p rterm-app --test openssh_loopback "${profile_arguments[@]}" \
  -- --nocapture

run_bounded "native ten-frame E2E ($profile)" 180 \
  cargo test --locked -p rterm-app --test native_window_e2e "${profile_arguments[@]}" \
  native_window_e2e_presents_ten_frames_from_a_real_pty -- --exact --nocapture

for scenario in \
  native_window_e2e_preserves_gpu_text_at_scale_100 \
  native_window_e2e_preserves_gpu_text_at_scale_125 \
  native_window_e2e_preserves_gpu_text_at_scale_150 \
  native_window_e2e_preserves_gpu_text_at_scale_200 \
  native_window_local_pane_v2_writes_visible_session_log
do
  run_bounded "native E2E scenario $scenario ($profile)" 300 \
    cargo test --locked -p rterm-app --test native_window_e2e "${profile_arguments[@]}" \
    "$scenario" -- --exact --ignored --nocapture
done
