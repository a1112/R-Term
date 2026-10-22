#!/usr/bin/env python3
"""Check repository independence and the actual local/SSH production graphs."""

import argparse
import json
from pathlib import Path
import subprocess
import tomllib


SSH_PACKAGES = {"russh", "rterm-ssh", "rpassword"}


def check_sources(root: Path) -> list[str]:
    """Inspect optional and target-specific dependencies as well as the lockfile."""
    violations = []
    manifests = [root / "Cargo.toml"]
    for directory in ("crates", "tauri/src-tauri", "contracts"):
        manifests.extend((root / directory).rglob("Cargo.toml"))
    for manifest in manifests:
        if "target" in manifest.relative_to(root).parts:
            continue
        document = tomllib.loads(manifest.read_text(encoding="utf-8"))
        package = document.get("package", {}).get("name", "")
        if package and not package.startswith("rterm-"):
            violations.append(f"{manifest.relative_to(root)}: foreign package identity {package}")

        def inspect(table):
            if not isinstance(table, dict):
                return
            for key, value in table.items():
                if key == "git" and "r-ssh" in str(value).lower():
                    violations.append(f"{manifest.relative_to(root)}: R-SSH Git dependency")
                if key == "path" and isinstance(value, str):
                    path = (manifest.parent / value).resolve()
                    if not path.is_relative_to(root):
                        violations.append(f"{manifest.relative_to(root)}: external path {value}")
                    elif not path.exists():
                        violations.append(f"{manifest.relative_to(root)}: missing path {value}")
                if isinstance(value, dict):
                    inspect(value)

        inspect(document)
    for lock in (root / "Cargo.lock", root / "contracts/rterm-consumer/Cargo.lock"):
        if not lock.exists():
            continue
        for package in tomllib.loads(lock.read_text(encoding="utf-8")).get("package", []):
            if package["name"].startswith("rssh-") or "r-ssh" in package.get("source", "").lower():
                violations.append(f"{lock.relative_to(root)}: R-SSH package {package['name']}")
    types = tomllib.loads((root / "crates/rterm-types/Cargo.toml").read_text(encoding="utf-8"))
    if types.get("dependencies") or types.get("features"):
        violations.append("rterm-types must remain dependency-free and transport-independent")
    return violations


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--source-only", action="store_true")
    arguments = parser.parse_args()
    root = arguments.root.resolve()
    violations = check_sources(root)
    results = []
    if not arguments.source_only and not violations:
        for name, flags, enabled in (
            ("default-local", [], False),
            ("no-default-features", ["--no-default-features"], False),
            ("ssh-extension", ["--features", "ssh"], True),
            ("developer-full", ["--features", "developer-full"], True),
        ):
            command = ["cargo", "tree", "--locked", "-p", "rterm-app", "--edges", "normal,build",
                       "--prefix", "none", "--format", "{p}", *flags]
            output = subprocess.check_output(command, cwd=root, text=True)
            packages = {line.split()[0] for line in output.splitlines() if line.strip()}
            present = packages & SSH_PACKAGES
            passed = present == SSH_PACKAGES if enabled else not present
            results.append({"profile": name, "passed": passed, "ssh_packages": sorted(present),
                            "package_count": len(packages)})
    passed = not violations and all(item["passed"] for item in results)
    print(json.dumps({"passed": passed, "violations": violations, "profiles": results}, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
