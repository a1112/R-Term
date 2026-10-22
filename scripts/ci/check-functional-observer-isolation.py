#!/usr/bin/env python3
"""Fail closed if a production R-Term artifact contains a functional observer."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

PROTOCOL_MARKERS = (
    b"rterm-functional-observer-v1",
    b"__RTERM_FUNCTIONAL_SNAPSHOT__",
    b"RTERM_FUNCTIONAL_OBSERVER_ENDPOINT",
)


def endpoint_path_hash(path: Path) -> int:
    value = 0xCBF29CE484222325
    for byte in str(path).encode("utf-8"):
        value = ((value ^ byte) * 0x00000100000001B3) & 0xFFFFFFFFFFFFFFFF
    return value


def observer_endpoint_exists(requested_path: Path) -> bool:
    if os.name != "nt":
        return requested_path.exists()

    import ctypes
    from ctypes import wintypes

    pipe = rf"\\.\pipe\rterm-functional-{endpoint_path_hash(requested_path):016x}"
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    create_file = kernel32.CreateFileW
    create_file.argtypes = (
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.LPVOID,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.HANDLE,
    )
    create_file.restype = wintypes.HANDLE
    handle = create_file(pipe, 0, 0, None, 3, 0, None)
    invalid_handle = wintypes.HANDLE(-1).value
    if handle != invalid_handle:
        kernel32.CloseHandle(handle)
        return True
    return ctypes.get_last_error() == 231  # ERROR_PIPE_BUSY


def run(command: list[str], *, cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        cwd=cwd,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=120,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--web-dist", type=Path)
    parser.add_argument("--package", default="rterm-app")
    parser.add_argument(
        "--startup-probe",
        choices=("desktop-cli", "web-server", "gui"),
        default="desktop-cli",
    )
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"production binary is absent: {binary}")

    tree = run(
        ["cargo", "tree", "--locked", "-p", args.package, "--edges", "normal", "--prefix", "none"],
        cwd=root,
    )
    if tree.returncode != 0:
        raise SystemExit(f"cargo tree failed: {tree.stderr.decode(errors='replace')}")
    if b"rterm-functional-tests" in tree.stdout:
        raise SystemExit("production cargo tree contains rterm-functional-tests")

    binary_bytes = binary.read_bytes()
    for marker in PROTOCOL_MARKERS:
        if marker in binary_bytes:
            raise SystemExit(f"production binary contains observer marker {marker!r}")
    if args.web_dist:
        for path in sorted(args.web_dist.rglob("*")):
            if path.is_file():
                contents = path.read_bytes()
                for marker in PROTOCOL_MARKERS:
                    if marker in contents:
                        raise SystemExit(f"production web asset {path} contains observer marker {marker!r}")

    with tempfile.TemporaryDirectory(prefix="rterm-production-observer-probe-") as directory:
        endpoint = Path(directory) / "must-not-exist.sock"
        env = os.environ.copy()
        env["RTERM_FUNCTIONAL_OBSERVER_ENDPOINT"] = str(endpoint)
        env["RTERM_FUNCTIONAL_OBSERVER_TOKEN"] = "00" * 32
        if args.startup_probe == "desktop-cli":
            probe = run([str(binary), "version", "--json"], cwd=root, env=env)
            if probe.returncode != 0:
                raise SystemExit(f"production startup probe failed: {probe.stderr.decode(errors='replace')}")
            try:
                json.loads(probe.stdout)
            except json.JSONDecodeError as error:
                raise SystemExit(f"production version --json was invalid: {error}") from error
        else:
            command = [str(binary)]
            if args.startup_probe == "web-server":
                if not args.web_dist:
                    raise SystemExit("web-server startup probe requires --web-dist")
                command.extend(["--listen", "127.0.0.1:0", "--web-root", str(args.web_dist.resolve())])
            process = subprocess.Popen(
                command,
                cwd=root,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            try:
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    if observer_endpoint_exists(endpoint):
                        raise SystemExit("production startup created a functional observer endpoint")
                    if process.poll() is not None:
                        stdout, stderr = process.communicate()
                        raise SystemExit(
                            "production startup probe exited early: "
                            + (stderr or stdout).decode(errors="replace")
                        )
                    time.sleep(0.05)
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=10)
        if observer_endpoint_exists(endpoint):
            raise SystemExit("production startup created a functional observer endpoint")

    print(json.dumps({"ok": True, "binary": str(binary), "markers": 0, "observer_endpoint": False}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
