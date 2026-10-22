#!/usr/bin/env bash
set -euo pipefail

binary=""
package_root=""
artifact_name=""
runtime_target=""
pty_backend=""
version=""
unsigned=0
while (($# > 0)); do
  case "$1" in
    --binary) binary="${2:?missing --binary value}"; shift 2 ;;
    --package-root) package_root="${2:?missing --package-root value}"; shift 2 ;;
    --artifact-name) artifact_name="${2:?missing --artifact-name value}"; shift 2 ;;
    --runtime-target) runtime_target="${2:?missing --runtime-target value}"; shift 2 ;;
    --pty-backend) pty_backend="${2:?missing --pty-backend value}"; shift 2 ;;
    --version) version="${2:?missing --version value}"; shift 2 ;;
    --unsigned) unsigned=1; shift ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done

for value in binary package_root artifact_name runtime_target pty_backend version; do
  if [[ -z "${!value}" ]]; then
    printf 'missing required --%s\n' "${value//_/-}" >&2
    exit 2
  fi
done
if [[ ! -f "$binary" ]]; then
  printf 'binary does not exist: %s\n' "$binary" >&2
  exit 1
fi
if [[ "$pty_backend" != unix-pty ]]; then
  printf 'Unix package requires unix-pty, not %s\n' "$pty_backend" >&2
  exit 1
fi

case "$runtime_target" in
  linux-x86_64) rust_target=x86_64-unknown-linux-gnu; platform=linux ;;
  linux-aarch64) rust_target=aarch64-unknown-linux-gnu; platform=linux ;;
  macos-x86_64) rust_target=x86_64-apple-darwin; platform=macos; bundle_arch=x86_64 ;;
  macos-aarch64) rust_target=aarch64-apple-darwin; platform=macos; bundle_arch=arm64 ;;
  *) printf 'unsupported Unix runtime target: %s\n' "$runtime_target" >&2; exit 1 ;;
esac
if [[ "$artifact_name" != *.tar.gz ]]; then
  printf 'Unix artifact name must end in .tar.gz\n' >&2
  exit 1
fi
if ((unsigned)) && [[ "$artifact_name" != *-unsigned.tar.gz ]]; then
  printf 'unsigned Unix artifact name must end in -unsigned.tar.gz\n' >&2
  exit 1
fi
if ((!unsigned)) && [[ "$artifact_name" == *-unsigned* ]]; then
  printf 'release-candidate artifact name must not contain -unsigned\n' >&2
  exit 1
fi
if [[ -e "$package_root" ]] && [[ -n "$(find "$package_root" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
  printf 'package root must be absent or empty: %s\n' "$package_root" >&2
  exit 1
fi
mkdir -p "$package_root/examples" "$package_root/licenses/fonts/LICENSES"

repository_root=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
binary_relative=rterm
if [[ "$platform" == macos ]]; then
  mkdir -p "$package_root/R-Term.app/Contents/MacOS"
  install -m 0755 "$binary" "$package_root/R-Term.app/Contents/MacOS/rterm"
  sed -e "s/__VERSION__/$version/g" -e "s/__ARCHITECTURE__/$bundle_arch/g" \
    "$repository_root/packaging/Info.plist" > "$package_root/R-Term.app/Contents/Info.plist"
  binary_relative=R-Term.app/Contents/MacOS/rterm
  cat > "$package_root/rterm" <<'EOF'
#!/usr/bin/env sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$root/R-Term.app/Contents/MacOS/rterm" "$@"
EOF
  chmod 0755 "$package_root/rterm"
else
  install -m 0755 "$binary" "$package_root/rterm"
fi
install -m 0755 "$repository_root/packaging/rterm-console.sh" "$package_root/rterm-console.sh"
install -m 0644 "$repository_root/README.md" "$package_root/README.md"
install -m 0644 "$repository_root/LICENSE" "$package_root/LICENSE"
install -m 0644 "$repository_root/NOTICE" "$package_root/NOTICE"
install -m 0644 "$repository_root/examples/rterm-profiles.toml" "$package_root/examples/rterm-profiles.toml"
install -m 0644 "$repository_root/tests/fixtures/fonts/"LICENSES/* "$package_root/licenses/fonts/LICENSES/"
install -m 0644 "$repository_root/tests/fixtures/fonts/MANIFEST.tsv" "$package_root/licenses/fonts/MANIFEST.tsv"

signing_status=pending-protected-signing
if ((unsigned)); then signing_status=unsigned; fi
source_commit=${GITHUB_SHA:-local}
python3 - "$package_root" "$artifact_name" "$runtime_target" "$rust_target" "$pty_backend" "$version" "$binary_relative" "$signing_status" "$source_commit" <<'PY'
import hashlib, json, pathlib, sys
root = pathlib.Path(sys.argv[1])
files = []
for path in sorted(p for p in root.rglob('*') if p.is_file()):
    relative = path.relative_to(root).as_posix()
    files.append({
        'path': relative,
        'size': path.stat().st_size,
        'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
    })
required_files = [
    sys.argv[7], 'rterm-console.sh', 'README.md', 'LICENSE', 'NOTICE',
    'examples/rterm-profiles.toml', 'licenses/fonts/MANIFEST.tsv',
]
if sys.argv[3].startswith('macos-'):
    required_files.extend(['rterm', 'R-Term.app/Contents/Info.plist'])
manifest = {
    'schema_version': 1,
    'package': {'name': 'R-Term', 'version': sys.argv[6], 'source_commit': sys.argv[9]},
    'artifact': {
        'name': sys.argv[2], 'format': 'tar.gz', 'rust_target': sys.argv[4],
        'runtime_target': sys.argv[3], 'pty_backend': sys.argv[5], 'binary': sys.argv[7],
    },
    'signing': {'status': sys.argv[8], 'unsigned': sys.argv[8] == 'unsigned'},
    'requirements': {'external_tools': [], 'optional_external_tools': {'system-openssh': ['ssh'], 'transfer-tools': ['sftp', 'scp']}},
    'required_files': required_files,
    'files': files,
}
(root / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
with (root / 'SHA256SUMS').open('w', encoding='ascii', newline='\n') as output:
    for path in sorted(p for p in root.rglob('*') if p.is_file() and p.name != 'SHA256SUMS'):
        output.write(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.relative_to(root).as_posix()}\n")
PY

package_parent=$(CDPATH= cd -- "$(dirname -- "$package_root")" && pwd)
package_name=$(basename -- "$package_root")
# BSD tar otherwise synthesizes AppleDouble ._ entries from macOS metadata,
# which are not payload files and invalidate the archive identity receipt.
COPYFILE_DISABLE=1 tar -czf "$package_parent/$artifact_name" -C "$package_parent" "$package_name"
