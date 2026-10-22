#!/usr/bin/env bash
set -euo pipefail

profile=release
warmups=5
samples=30
output_directory=artifacts/stage0-diagnostics
skip_build=0

while (($#)); do
  case "$1" in
    --profile) profile=$2; shift 2 ;;
    --warmups) warmups=$2; shift 2 ;;
    --samples) samples=$2; shift 2 ;;
    --output-directory) output_directory=$2; shift 2 ;;
    --skip-build) skip_build=1; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [[ "$profile" != debug && "$profile" != release ]]; then
  echo "--profile must be debug or release" >&2
  exit 2
fi
if ((warmups < 0 || samples < 1)); then
  echo "warmups must be non-negative and samples must be positive" >&2
  exit 2
fi

profile_directory=$profile
app="target/$profile_directory/rterm"
launcher="target/$profile_directory/rterm-bench-launcher"
if ((skip_build == 0)); then
  profile_args=()
  if [[ "$profile" == release ]]; then profile_args+=(--release); fi
  cargo build --locked -p rterm-app --features developer-full "${profile_args[@]}"
  cargo build --locked -p rterm-diagnostics --bin rterm-bench-launcher "${profile_args[@]}"
fi
[[ -x "$app" ]] || { echo "missing app executable: $app" >&2; exit 1; }
[[ -x "$launcher" ]] || { echo "missing launcher executable: $launcher" >&2; exit 1; }

raw_directory="$output_directory/raw"
mkdir -p "$raw_directory"
export RTERM_BENCHMARK_WINDOW_SCALE_FACTOR=1

run_one() {
  local scenario=$1
  "$launcher" \
    --app "$app" \
    --scenario "$scenario" \
    --stabilization-ms 5000 \
    --sample-interval-ms 100 \
    --sample-count 10 \
    --cols 80 \
    --rows 24 \
    --json
}

for scenario in empty-window ssh1; do
  for ((index=0; index<warmups; index++)); do
    run_one "$scenario" >/dev/null
  done
  for ((index=1; index<=samples; index++)); do
    path=$(printf '%s/%s-%02d.json' "$raw_directory" "$scenario" "$index")
    run_one "$scenario" >"$path"
  done
done

python - "$output_directory" "$profile" "$warmups" "$samples" <<'PY'
import json
import math
import pathlib
import sys

output = pathlib.Path(sys.argv[1])
profile = sys.argv[2]
warmups = int(sys.argv[3])
runs = int(sys.argv[4])
targets = {"empty-window": 45 * 1024 * 1024, "ssh1": 60 * 1024 * 1024}
scenarios = {}
for scenario, target in targets.items():
    records = [json.loads(path.read_text(encoding="utf-8")) for path in sorted((output / "raw").glob(f"{scenario}-*.json"))]
    if len(records) != runs:
        raise SystemExit(f"expected {runs} {scenario} records, observed {len(records)}")
    values = sorted(sample["bytes"] for record in records for sample in record["memory"]["samples"])
    p95 = values[max(0, math.ceil(0.95 * len(values)) - 1)]
    scenarios[scenario] = {
        "measured_runs": runs,
        "samples_per_run": 10,
        "memory_metric": records[0]["memory"]["metric"],
        "memory_p95_bytes": p95,
        "report_only_target_bytes": target,
        "report_only_target_met": p95 <= target,
    }
    if p95 > target:
        print(f"warning: Stage 0 report-only memory observation: {scenario} p95={p95} target={target}", file=sys.stderr)

aggregate = {
    "schema": "rterm.diagnostics/aggregate-v1",
    "profile": profile,
    "warmups": warmups,
    "measured_runs": runs,
    "columns": 80,
    "rows": 24,
    "scale_factor": 1.0,
    "thresholds": "report-only",
    "scenarios": scenarios,
}
(output / "aggregate.json").write_text(json.dumps(aggregate, indent=2) + "\n", encoding="utf-8")
print(json.dumps(aggregate, separators=(",", ":")))
PY
