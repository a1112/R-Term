#!/usr/bin/env python3
"""Prepare a verified disposable R-Term consumer; not a compatibility certificate.

Only committed input is exported. The source worktree (including local edits)
is never rewritten. Output must be a new directory outside that worktree; all
created checkouts, lock-command diagnostics and receipts remain for inspection.
Requires Python 3.11+, Git and Cargo.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import runpy
import stat
import subprocess
import sys

from rterm_consumer_profile import COMMIT, prepare_app_manifest, select_profile


SCRIPTS = Path(__file__).resolve().parent
REHEARSAL = runpy.run_path(str(SCRIPTS / "rehearse-rterm-consumer.py"))
BOUNDARY = runpy.run_path(str(SCRIPTS / "check-rterm-release-contract.py"))
APP_MANIFEST = "crates/rterm-app/Cargo.toml"
FONT_MANIFEST = "crates/rterm-fonts/Cargo.toml"


def git(repo: Path, *args: str) -> bytes:
    result = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, check=False)
    if result.returncode:
        raise ValueError(f"Git command failed: {result.stderr.decode(errors='replace').strip()}")
    return result.stdout


def full_commit(repo: Path, value: str) -> str:
    if COMMIT.fullmatch(value) is None:
        raise ValueError("require an immutable lowercase commit ID")
    if git(repo, "rev-parse", "--verify", f"{value}^{{commit}}").decode().strip() != value:
        raise ValueError("commit identity mismatch")
    return value


def safe_relative(value: str) -> None:
    path = PurePosixPath(value)
    if (
        not value or path.is_absolute() or "\\" in value or ":" in value
        or path.as_posix() != value
        or any(part in (".", "..") or part.lower() == ".git" or PureWindowsPath(part).is_reserved() for part in path.parts)
    ):
        raise ValueError(f"unsafe repository path: {value}")


def tree_files(repo: Path, commit: str) -> dict[str, tuple[str, str]]:
    result = {}
    for entry in git(repo, "ls-tree", "-r", "-z", commit).split(b"\0"):
        if not entry:
            continue
        metadata, raw_path = entry.split(b"\t", 1)
        mode, kind, blob = metadata.decode().split()
        path = raw_path.decode("utf-8")
        safe_relative(path)
        if kind != "blob" or mode not in ("100644", "100755"):
            raise ValueError(f"non-regular source entry: {path}")
        result[path] = (mode, blob)
    return result


def expected_files(repo: Path, source: str, consumer: str, overlay: list[str]) -> dict[str, tuple[str, str]]:
    def overlaid(path):
        return any(path.startswith(root + "/") for root in overlay)

    # Check every source entry before checkout, even ones not in the overlay.
    source_files = tree_files(repo, source)
    consumer_files = tree_files(repo, consumer)
    return {
        **{path: value for path, value in consumer_files.items() if not overlaid(path)},
        **{path: value for path, value in source_files.items() if overlaid(path)},
    }


def reject_link(path: Path) -> None:
    info = path.lstat()
    if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0):
        raise ValueError(f"symlink or reparse point is not allowed: {path}")


def blob_id(data: bytes) -> str:
    return hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest()


def verify_checkout(checkout: Path, expected: dict[str, tuple[str, str]], overrides: dict[str, bytes]) -> None:
    """Verify actual bytes, including ignored files, independently of Git index flags."""
    reject_link(checkout)
    permitted = dict(expected)
    for path, data in overrides.items():
        if path not in (APP_MANIFEST, "Cargo.lock"):
            raise ValueError("override is outside the preparation allowlist")
        permitted[path] = ("100644", blob_id(data))
    seen = set()
    for parent, dirs, files in os.walk(checkout, followlinks=False):
        for name in dirs + files:
            reject_link(Path(parent) / name)
        if Path(parent) == checkout:
            dirs[:] = [name for name in dirs if name != ".git"]
        for name in files:
            path = Path(parent) / name
            relative = path.relative_to(checkout).as_posix()
            if relative not in permitted:
                raise ValueError(f"unexpected checkout file (including ignored files): {relative}")
            mode, expected_blob = permitted[relative]
            if blob_id(path.read_bytes()) != expected_blob:
                raise ValueError(f"checkout source mismatch: {relative}")
            if os.name != "nt" and bool(path.stat().st_mode & stat.S_IXUSR) != (mode == "100755"):
                raise ValueError(f"checkout file mode mismatch: {relative}")
            seen.add(relative)
    if seen != set(permitted):
        raise ValueError("checkout is missing expected source files")


def validated_contract(repo: Path, consumer: str, contract_path: Path) -> tuple[dict, list[str], str]:
    relative = contract_path.resolve().relative_to(repo).as_posix()
    safe_relative(relative)
    raw = contract_path.read_bytes()
    if raw != git(repo, "show", f"{consumer}:{relative}"):
        raise ValueError("contract must match the immutable consumer commit")
    contract = json.loads(raw)
    if contract.get("schema_version") != 1:
        raise ValueError("unsupported contract schema")
    for key, expected in (("packages", BOUNDARY["EXPECTED_PACKAGES"]), ("vendor_trees", BOUNDARY["EXPECTED_VENDORS"])):
        entries = contract.get(key, [])
        if not isinstance(entries, list) or any(not isinstance(entry, dict) for entry in entries):
            raise ValueError(f"invalid {key} boundary")
        if len(entries) != len(expected) or {entry.get("name"): entry.get("path") for entry in entries} != expected:
            raise ValueError(f"unexpected {key} overlay boundary")
    return contract, REHEARSAL["contract_overlay_paths"](contract), hashlib.sha256(raw).hexdigest()


def prepare(repo: Path, source_ref: str, consumer_ref: str, contract_path: Path, output: Path) -> dict:
    repo = repo.resolve()
    output = Path(os.path.abspath(output))
    for path in (output, *output.parents):
        if path.exists() or path.is_symlink():
            reject_link(path)
    if output.exists() or output.is_relative_to(repo):
        raise ValueError("output must be a new directory outside the source worktree")
    source = full_commit(repo, source_ref)
    consumer = full_commit(repo, consumer_ref)
    contract, overlay, contract_digest = validated_contract(repo, consumer, contract_path)
    lkg = full_commit(repo, contract["last_known_good_rterm_ref"])
    trees = {}
    for path in overlay:
        if git(repo, "cat-file", "-t", f"{source}:{path}").strip() != b"tree":
            raise ValueError(f"overlay must be a source tree: {path}")
        trees[path] = git(repo, "rev-parse", f"{source}:{path}").decode().strip()
    for vendor in contract["vendor_trees"]:
        if trees[vendor["path"]] != vendor.get("tree"):
            raise ValueError(f"vendor tree drift: {vendor['name']}")
    expected = expected_files(repo, source, consumer, overlay)
    profile = select_profile(source, lkg, git(repo, "show", f"{source}:{FONT_MANIFEST}"))
    original = git(repo, "show", f"{consumer}:{APP_MANIFEST}")
    generated = prepare_app_manifest(original, profile)

    # No writes until every input/profile check above has succeeded. mkdir also
    # prevents reusing output created by another invocation since preflight.
    output.mkdir(parents=True, exist_ok=False)
    receipt = {
        "schema_version": 1, "ok": False, "compatibility_verified": False,
        "profile": profile, "source_commit": source, "consumer_commit": consumer,
        "last_known_good_rterm_ref": lkg, "source_trees": trees,
        "manifest_before_sha256": hashlib.sha256(original).hexdigest(),
        "manifest_after_sha256": hashlib.sha256(generated).hexdigest(),
        "contract_sha256": contract_digest,
    }
    try:
        source_checkout, checkout = output / "source", output / "consumer"
        REHEARSAL["clone_at"](repo, source_checkout, source)
        REHEARSAL["clone_at"](repo, checkout, consumer)
        verify_checkout(source_checkout, tree_files(repo, source), {})
        verify_checkout(checkout, tree_files(repo, consumer), {})
        REHEARSAL["overlay_paths"](source_checkout, checkout, overlay)
        verify_checkout(checkout, expected, {})
        (checkout / APP_MANIFEST).write_bytes(generated)
        overrides = {APP_MANIFEST: generated}
        verify_checkout(checkout, expected, overrides)
        receipt["lock_command"] = REHEARSAL["run_command"](
            ["cargo", "generate-lockfile"], checkout, os.environ.copy(), "consumer-prepare"
        )
        if receipt["lock_command"]["returncode"] != 0:
            raise ValueError("Cargo lock generation failed; see retained lock_command")
        lock = (checkout / "Cargo.lock").read_bytes()
        overrides["Cargo.lock"] = lock
        verify_checkout(checkout, expected, overrides)
        receipt["lockfile_sha256"] = hashlib.sha256(lock).hexdigest()
        receipt["ok"] = True
    except (OSError, ValueError, REHEARSAL["RehearsalError"]) as error:
        receipt["error"] = str(error)
    REHEARSAL["write_json_atomic"](output / "preparation.json", receipt)
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--source-ref", required=True)
    parser.add_argument("--consumer-ref", required=True)
    parser.add_argument("--contract", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()
    try:
        receipt = prepare(args.repo, args.source_ref, args.consumer_ref, args.contract, args.output_dir)
        print(json.dumps(receipt, sort_keys=True), file=sys.stdout if receipt["ok"] else sys.stderr)
        return 0 if receipt["ok"] else 1
    except (OSError, ValueError, KeyError, REHEARSAL["RehearsalError"]) as error:
        print(json.dumps({"ok": False, "error": str(error)}), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
