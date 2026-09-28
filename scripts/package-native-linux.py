#!/usr/bin/env python3
"""Package a tested native Linux binary from a digest-pinned source export.

Build from the supplied source tar with --locked -p rssh-app --bin rterm and
default production-gui features. This command does not publish or install.
"""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile
import tomllib


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def run(repo, *args):
    return subprocess.check_output(args, cwd=repo, text=True).strip()


def verify_source(repo, archive, commit):
    """Check a git archive without extracting any archive-controlled paths."""
    with tarfile.open(archive, "r:") as stream:
        if stream.pax_headers.get("comment") != commit:
            raise ValueError("git archive commit does not match requested source commit")
        seen = set()
        for member in stream:
            name = PurePosixPath(member.name)
            if name.is_absolute() or ".." in name.parts or "\\" in member.name:
                raise ValueError("unsafe source archive path")
            if member.isdir():
                continue
            if not member.isfile() or member.name in seen:
                raise ValueError("source archive requires unique regular files")
            seen.add(member.name)
            path = repo / member.name
            if any(p.is_symlink() for p in [path, *path.parents]) or not path.is_file():
                raise ValueError("source export has missing or linked files: " + member.name)
            if path.read_bytes() != stream.extractfile(member).read():
                raise ValueError("source export differs from archive: " + member.name)
        if not seen:
            raise ValueError("empty source archive")
        actual = {path.relative_to(repo).as_posix() for path in repo.rglob("*") if not path.is_dir()}
        if actual != seen:
            raise ValueError("source export contains files outside the git archive")


def font_inventory(repo):
    root = repo / "tests/fixtures/fonts"
    hashes = dict((name, sha) for sha, name in
                  (line.split() for line in (root / "SHA256SUMS").read_text().splitlines()))
    notices = []
    with (root / "MANIFEST.tsv").open(newline="", encoding="utf-8") as stream:
        for row in csv.DictReader(stream, delimiter="\t"):
            for key in ("file", "license_file"):
                name = PurePosixPath(row[key])
                if name.is_absolute() or ".." in name.parts:
                    raise ValueError("unsafe font manifest path")
            font = (root / row["file"]).read_bytes()
            license_bytes = (root / row["license_file"]).read_bytes()
            # Git on Windows may check text files out with CRLF. The archive
            # build uses the original LF bytes; accept neither altered content.
            license_bytes = license_bytes.replace(b"\r\n", b"\n")
            if digest(font) != hashes.get(row["file"]) or digest(license_bytes) != row["license_sha256"]:
                raise ValueError("embedded font or license digest mismatch: " + row["file"])
            notices.append({"name": row["file"], "version": row["version"],
                            "license": row["license"], "source": row["source"],
                            "sha256": digest(font), "manifest": row,
                            "notices": {row["license_file"]: license_bytes.decode("utf-8")}})
    if {item["name"] for item in notices} != set(hashes):
        raise ValueError("font manifest does not cover embedded fixture set")
    return notices


def verify_version(value, version):
    expected = {"name": "rterm", "version": version, "target": "linux-x86_64",
                "console": True, "pty_backend": "unix-pty", "native_ssh_backend": "disabled"}
    if value != expected:
        raise ValueError("built binary does not report expected local Linux release identity")


def inventory(repo):
    metadata = json.loads(run(repo, "cargo", "metadata", "--locked", "--offline", "--format-version", "1",
                              "--filter-platform", "x86_64-unknown-linux-gnu"))
    tree = run(repo, "cargo", "tree", "--locked", "--offline", "-p", "rssh-app",
               "--target", "x86_64-unknown-linux-gnu", "--edges", "normal,build", "--prefix", "none", "--format", "{p}")
    identities = {tuple(line.split()[:2]) for line in tree.splitlines() if line.strip()}
    forbidden = {"russh", "rssh-ssh", "rssh-types", "rterm-ssh-adapter", "rpassword"}
    if any(name in forbidden for name, _ in identities):
        raise ValueError("production dependency graph contains SSH")
    notices = []
    for package in metadata["packages"]:
        if (package["name"], "v" + package["version"]) not in identities:
            continue
        root = Path(package["manifest_path"]).parent
        files = {}
        for pattern in ("LICENSE*", "COPYING*", "COPYRIGHT*", "NOTICE*", "UNLICENSE*"):
            for path in sorted(root.glob(pattern)):
                if path.is_file() and not path.is_symlink():
                    files[path.name] = path.read_text(errors="replace")
        if package.get("license_file"):
            path = root / package["license_file"]
            files[path.name] = path.read_text(errors="replace")
        source = package["source"]
        if source is None:
            relative = root.relative_to(repo).as_posix()
            source = "source-export:" + relative
        notices.append({"name": package["name"], "version": package["version"],
                        "source": source, "license": package["license"] or "LicenseRef-bundled-notice",
                        "notices": files})
    if len(notices) != len(identities):
        raise ValueError("dependency inventory is incomplete")
    return sorted(notices + font_inventory(repo), key=lambda p: (p["name"], p["version"], p["source"]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--target-dir", required=True, type=Path,
                        help="external Cargo build directory on the application build disk")
    parser.add_argument("--source-archive", required=True, type=Path)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_commit):
        parser.error("source commit must be an immutable full Git commit")
    if args.output.exists() or args.output.is_symlink():
        raise FileExistsError(args.output)
    verify_source(repo, args.source_archive, args.source_commit)
    target = args.target_dir.resolve()
    if target.is_relative_to(repo):
        raise ValueError("build output must be outside the immutable source export")
    workspace = tomllib.loads((repo / "Cargo.toml").read_text())
    toolchain = {"rustc": run(repo, "rustc", "--version"), "cargo": run(repo, "cargo", "--version")}
    subprocess.run(["cargo", "build", "--locked", "--offline", "--release",
                    "--no-default-features", "--features", "production-gui", "-p", "rssh-app",
                    "--bin", "rterm", "--target", "x86_64-unknown-linux-gnu", "--target-dir", str(target)],
                   cwd=repo, check=True)
    # Packaging always consumes the binary produced by this controlled build.
    verify_source(repo, args.source_archive, args.source_commit)
    binary_path = target / "x86_64-unknown-linux-gnu/release/rterm"
    binary = binary_path.read_bytes()
    if len(binary) < 64 or binary[:7] != b"\x7fELF\x02\x01\x01" or binary[18:20] != b"\x3e\x00":
        raise ValueError("binary is not x86_64 ELF")
    version = json.loads(run(repo, str(binary_path), "version", "--json"))
    verify_version(version, workspace["workspace"]["package"]["version"])
    payloads = {
        "usr/bin/rterm": (binary, 0o755),
        "usr/share/licenses/rterm/LICENSE": ((repo / "LICENSE").read_bytes(), 0o644),
        "usr/share/licenses/rterm/NOTICE": ((repo / "NOTICE").read_bytes(), 0o644),
        "usr/share/rterm/THIRD-PARTY.json": ((json.dumps(inventory(repo), ensure_ascii=False,
             sort_keys=True, indent=2) + "\n").encode(), 0o644),
    }
    receipt = {"schemaVersion": 1, "kind": "rterm-native", "target": "x86_64-unknown-linux-gnu",
               "version": workspace["workspace"]["package"]["version"],
               "sourceCommit": args.source_commit, "sourceSha256": digest(args.source_archive.read_bytes()),
               "cargoLockSha256": digest((repo / "Cargo.lock").read_bytes()),
               "toolchain": toolchain,
               "features": ["production-gui"], "files": {
                   name: {"sha256": digest(data), "size": len(data), "mode": mode}
                   for name, (data, mode) in payloads.items()}}
    # The new output is never reused on failure, preserving evidence.
    args.output.mkdir(parents=True, exist_ok=False)
    for name, (data, mode) in payloads.items():
        path = args.output / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("xb") as stream:
            stream.write(data)
        path.chmod(mode)
    raw = (json.dumps(receipt, sort_keys=True, separators=(",", ":")) + "\n").encode()
    with (args.output / "bundle.json").open("xb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    print(json.dumps({"bundleSha256": digest(raw), "receipt": receipt}, sort_keys=True))


if __name__ == "__main__":
    main()
