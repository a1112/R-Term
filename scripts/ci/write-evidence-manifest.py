#!/usr/bin/env python3
"""Write a commit-bound CI evidence manifest after a gate succeeds."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import platform
import subprocess


def tool_version(command: list[str]) -> str | None:
    try:
        result = subprocess.run(
            command,
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (FileNotFoundError, subprocess.SubprocessError):
        return None
    return (result.stdout or result.stderr).strip().splitlines()[0]


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=pathlib.Path)
    parser.add_argument("--gate", action="append", required=True)
    parser.add_argument("--command", action="append", default=[])
    args = parser.parse_args()

    lockfiles = {}
    for name in ("Cargo.lock", "web/package-lock.json", "tauri/package-lock.json"):
        path = pathlib.Path(name)
        if path.is_file():
            lockfiles[name] = sha256(path)

    manifest = {
        "schema": "rterm-ci-evidence-v1",
        "commit": os.environ.get("GITHUB_SHA") or tool_version(["git", "rev-parse", "HEAD"]),
        "repository": os.environ.get("GITHUB_REPOSITORY"),
        "ref": os.environ.get("GITHUB_REF"),
        "run": {
            "id": os.environ.get("GITHUB_RUN_ID"),
            "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
            "workflow": os.environ.get("GITHUB_WORKFLOW"),
            "job": os.environ.get("GITHUB_JOB"),
        },
        "runner": {
            "os": os.environ.get("RUNNER_OS") or platform.system(),
            "arch": os.environ.get("RUNNER_ARCH") or platform.machine(),
            "name": os.environ.get("RUNNER_NAME"),
        },
        "generated_at": dt.datetime.now(dt.UTC).isoformat(),
        "successful_gates": args.gate,
        "commands": args.command,
        "tool_versions": {
            "rustc": tool_version(["rustc", "--version"]),
            "cargo": tool_version(["cargo", "--version"]),
            "node": tool_version(["node", "--version"]),
            "npm": tool_version(["npm", "--version"]),
        },
        "lockfile_sha256": lockfiles,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
