#!/usr/bin/env sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ -x "$root/R-Term.app/Contents/MacOS/rterm" ]; then
  executable="$root/R-Term.app/Contents/MacOS/rterm"
else
  executable="$root/rterm"
fi

exec "$executable" console "$@"
