#!/usr/bin/env bash
set -euo pipefail

package_root=""
expected_target=""
expected_pty_backend=""
expected_artifact_name=""
expected_unsigned=0
while (($# > 0)); do
  case "$1" in
    --package-root) package_root="${2:?missing --package-root value}"; shift 2 ;;
    --expected-target) expected_target="${2:?missing --expected-target value}"; shift 2 ;;
    --expected-pty-backend) expected_pty_backend="${2:?missing --expected-pty-backend value}"; shift 2 ;;
    --expected-artifact-name) expected_artifact_name="${2:?missing --expected-artifact-name value}"; shift 2 ;;
    --expected-unsigned) expected_unsigned=1; shift ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done
for value in package_root expected_target expected_pty_backend expected_artifact_name; do
  if [[ -z "${!value}" ]]; then printf 'missing required --%s\n' "${value//_/-}" >&2; exit 2; fi
done
if ((expected_unsigned)) && [[ "$expected_artifact_name" != *-unsigned* ]]; then
  printf 'expected unsigned artifact name must contain -unsigned\n' >&2
  exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(CDPATH= cd -- "$script_dir/../.." && pwd)
if [[ ! -f "$script_dir/process-harness.sh" ]]; then
  printf 'shared process harness is missing: %s\n' "$script_dir/process-harness.sh" >&2
  exit 1
fi
# shellcheck source=process-harness.sh
source "$script_dir/process-harness.sh"
package_root=$(CDPATH= cd -- "$package_root" && pwd)
temporary=$(mktemp -d)
cleanup_package_smoke() {
  cleanup
  rm -rf -- "$temporary"
}
trap cleanup_package_smoke EXIT

for required in manifest.json SHA256SUMS README.md LICENSE examples/rterm-profiles.toml; do
  if [[ ! -f "$package_root/$required" ]]; then printf 'unpacked package is missing %s\n' "$required" >&2; exit 1; fi
done

validation_program=$(cat <<'PY'
import hashlib, json, pathlib, re, sys
root = pathlib.Path(sys.argv[1]).resolve()
target, pty, artifact, unsigned = sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5] == '1'
manifest = json.loads((root / 'manifest.json').read_text(encoding='utf-8'))
assert manifest['schema_version'] == 1
assert manifest['artifact']['name'] == artifact
assert manifest['artifact']['runtime_target'] == target
assert manifest['artifact']['pty_backend'] == pty
assert manifest['signing']['unsigned'] is unsigned
assert manifest['requirements']['external_tools'] == []
for relative in manifest['required_files']:
    candidate = (root / relative).resolve()
    assert candidate.is_relative_to(root) and candidate.is_file(), relative
manifest_files = set()
for entry in manifest['files']:
    relative = pathlib.PurePosixPath(entry['path'])
    assert not relative.is_absolute() and '..' not in relative.parts
    assert relative.as_posix() not in manifest_files
    manifest_files.add(relative.as_posix())
    candidate = (root / pathlib.Path(*relative.parts)).resolve()
    assert candidate.is_relative_to(root) and candidate.is_file()
    assert candidate.stat().st_size == entry['size']
    assert hashlib.sha256(candidate.read_bytes()).hexdigest() == entry['sha256']
payload = {p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() and p.name not in ('manifest.json', 'SHA256SUMS')}
assert manifest_files == payload, f'unmanifested files: {sorted(payload - manifest_files)}; stale entries: {sorted(manifest_files - payload)}'
listed = set()
for line in (root / 'SHA256SUMS').read_text(encoding='ascii').splitlines():
    match = re.fullmatch(r'([0-9a-fA-F]{64})  (.+)', line)
    assert match, line
    relative = pathlib.PurePosixPath(match.group(2))
    assert not relative.is_absolute() and '..' not in relative.parts
    assert relative.as_posix() not in listed
    listed.add(relative.as_posix())
    candidate = (root / pathlib.Path(*relative.parts)).resolve()
    assert candidate.is_relative_to(root) and candidate.is_file()
    assert hashlib.sha256(candidate.read_bytes()).hexdigest() == match.group(1).lower()
actual = {p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() and p.name != 'SHA256SUMS'}
assert listed == actual, f'unchecksummed files: {sorted(actual - listed)}; stale entries: {sorted(listed - actual)}'
print(manifest['artifact']['binary'])
PY
)
run_bounded "package manifest and checksum validation" 30 python3 -c "$validation_program" "$package_root" "$expected_target" "$expected_pty_backend" "$expected_artifact_name" "$expected_unsigned" > "$temporary/binary-relative"
IFS= read -r binary_relative < "$temporary/binary-relative"
binary="$package_root/$binary_relative"
if [[ ! -x "$binary" ]]; then printf 'packaged binary is not executable: %s\n' "$binary" >&2; exit 1; fi

run_bounded "shared process harness self-test" 90 bash "$script_dir/run-native-window.sh" --harness-self-test
run_bounded "packaged version" 30 "$binary" version --json > "$temporary/version.json"
version_validation=$(cat <<'PY'
import json, sys
report = json.load(open(sys.argv[1], encoding='utf-8'))
manifest = json.load(open(sys.argv[2], encoding='utf-8'))
assert report['target'] == sys.argv[3]
assert report['pty_backend'] == sys.argv[4]
assert report['version'] == manifest['package']['version']
assert report['name'] == 'rterm'
assert report['native_ssh_backend'] in ('disabled', 'russh')
PY
)
run_bounded "validate packaged version JSON" 30 python3 -c "$version_validation" "$temporary/version.json" "$package_root/manifest.json" "$expected_target" "$expected_pty_backend"
if [[ "$expected_target" == macos-* ]]; then
  macos_cli="$package_root/rterm"
  if [[ ! -x "$macos_cli" ]]; then printf 'packaged macOS CLI launcher is missing or not executable\n' >&2; exit 1; fi
  run_bounded "packaged macOS CLI launcher" 30 "$macos_cli" version --json > "$temporary/macos-cli-version.json"
  plist_validation=$(cat <<'PY'
import json, pathlib, plistlib, sys
root = pathlib.Path(sys.argv[1])
manifest = json.loads((root / 'manifest.json').read_text(encoding='utf-8'))
with (root / 'R-Term.app/Contents/Info.plist').open('rb') as source:
    plist = plistlib.load(source)
expected_arch = {'macos-x86_64': 'x86_64', 'macos-aarch64': 'arm64'}[sys.argv[2]]
assert plist['CFBundleExecutable'] == 'rterm'
assert plist['CFBundleShortVersionString'] == manifest['package']['version']
assert plist['CFBundleVersion'] == manifest['package']['version']
assert plist['LSArchitecturePriority'] == [expected_arch]
assert plist['LSMinimumSystemVersion'] == '11.0'
assert plist['NSHighResolutionCapable'] is True
assert plist['NSSupportsAutomaticGraphicsSwitching'] is True
cli_version = json.load(open(sys.argv[3], encoding='utf-8'))
assert cli_version['target'] == sys.argv[2]
PY
)
  run_bounded "validate packaged macOS bundle identity" 30 python3 -c "$plist_validation" "$package_root" "$expected_target" "$temporary/macos-cli-version.json"
fi
launcher="$package_root/rterm-console.sh"
if [[ ! -x "$launcher" ]]; then printf 'packaged console launcher is missing or not executable\n' >&2; exit 1; fi
run_bounded "packaged launcher preflight" 30 "$launcher" --preflight -- sh -c 'printf package-launcher-smoke'

export RTERM_TEST_APP_EXECUTABLE="$binary"
cd "$repository_root"
native_backend=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["native_ssh_backend"])' "$temporary/version.json")
if [[ "$native_backend" == russh ]]; then
  export RTERM_REQUIRE_OPENSSH=1
  run_bounded "packaged OpenSSH loopback" 300 cargo test --locked -p rterm-app --features ssh --test openssh_loopback rterm_app_system_openssh_entrypoint_runs_a_real_loopback_exec -- --exact --nocapture
else
  run_bounded "packaged SSH-free profile" 30 python3 - "$binary" <<'PY_LOCAL'
import subprocess, sys
for command in ('ssh', 'scp', 'sftp'):
    result = subprocess.run([sys.argv[1], command, 'user@must-not-resolve.invalid'], capture_output=True, text=True, timeout=5)
    assert result.returncode != 0 and 'SSH support is disabled' in result.stderr, result
    assert not result.stdout
PY_LOCAL
fi
run_bounded "packaged native ten-frame E2E" 240 cargo test --locked -p rterm-app --test native_window_e2e native_window_e2e_presents_ten_frames_from_a_real_pty -- --exact --nocapture
