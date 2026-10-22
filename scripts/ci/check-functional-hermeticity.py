#!/usr/bin/env python3
"""Fail when functional-test runtime assets can address the public network."""

from __future__ import annotations

import ipaddress
import re
import sys
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[2]
SCAN_ROOTS = (
    ROOT / "crates" / "rterm-functional-tests" / "src",
    ROOT / "functional-tests",
    ROOT / "scripts" / "functional",
    ROOT / "web" / "tests",
    ROOT / "tauri" / "src-tauri" / "src",
)
TEXT_SUFFIXES = {".py", ".ps1", ".rs", ".sh", ".swift", ".toml", ".ts"}
URL = re.compile(r"https?://[^\s\"'<>`)]+")
IPV4 = re.compile(r"(?<![\d.])(?:\d{1,3}\.){3}\d{1,3}(?![\d.])")


def is_loopback_host(host: str | None) -> bool:
    if host is None:
        return False
    host = host.strip("[]").rstrip(".").lower()
    if host == "localhost" or host.endswith(".localhost"):
        return True
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


def check_file(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    try:
        label = path.relative_to(ROOT)
    except ValueError:
        label = path
    errors: list[str] = []
    for match in URL.finditer(text):
        raw = match.group(0).replace("*", "1")
        if not is_loopback_host(urlsplit(raw).hostname):
            errors.append(f"{label}: external URL {match.group(0)!r}")
    for match in IPV4.finditer(text):
        try:
            address = ipaddress.ip_address(match.group(0))
        except ValueError:
            continue
        if not address.is_loopback:
            errors.append(f"{label}: non-loopback address {address}")
    return errors


def main() -> int:
    errors: list[str] = []
    files = 0
    for root in SCAN_ROOTS:
        for path in sorted(root.rglob("*")):
            if path.is_file() and path.suffix.lower() in TEXT_SUFFIXES:
                files += 1
                errors.extend(check_file(path))
    runner = (ROOT / "crates/rterm-functional-tests/src/runner.rs").read_text(encoding="utf-8")
    transport = (ROOT / "crates/rterm-functional-tests/src/transport_driver.rs").read_text(
        encoding="utf-8"
    )
    if "Command::new(app)" in runner or "Command::new(app)" in transport:
        errors.append("application entrypoints must use hermetic_app_command")
    if errors:
        for error in sorted(set(errors)):
            print(f"FUNCTIONAL_HERMETICITY: {error}", file=sys.stderr)
        return 1
    print(f'{{"ok":true,"files_checked":{files},"network_policy":"loopback-only"}}')
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
