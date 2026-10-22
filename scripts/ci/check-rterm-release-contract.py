#!/usr/bin/env python3
"""Validate the versioned R-Term release boundary and immutable inputs."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


SHA1 = re.compile(r"^[0-9a-f]{40}$")
EXPECTED_PACKAGES = {
    "rterm-types": "crates/rterm-types",
    "rterm-terminal": "crates/rterm-terminal",
    "rterm-runtime": "crates/rterm-runtime",
    "rterm-fonts": "crates/rterm-fonts",
    "rterm-render-core": "crates/rterm-render-core",
    "rterm-render-cpu": "crates/rterm-render-cpu",
    "rterm-render-wgpu": "crates/rterm-render-wgpu",
}
EXPECTED_VENDORS = {
    "glyphon": "vendor/glyphon-0.12.0",
    "gpu-allocator": "vendor/gpu-allocator-0.28.0",
}
REQUIRED_HISTORY_COMPONENTS = {
    "terminal",
    "runtime",
    "fonts",
    "render-core",
    "render-cpu",
    "render-wgpu",
}


def run_git(repo_root: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo_root), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )


def cargo_packages(repo_root: Path) -> dict[str, dict[str, Any]]:
    result = subprocess.run(
        [
            "cargo",
            "metadata",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        ],
        cwd=repo_root,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"cargo metadata failed: {result.stderr.strip()}")
    metadata = json.loads(result.stdout)
    return {package["name"]: package for package in metadata["packages"]}


def relative_manifest_directory(repo_root: Path, package: dict[str, Any]) -> str:
    manifest = Path(package["manifest_path"]).resolve()
    return manifest.parent.relative_to(repo_root.resolve()).as_posix()


def parse_history_map(path: Path, violations: list[str]) -> list[tuple[str, str, str]]:
    if not path.is_file():
        violations.append(f"missing history map: {path}")
        return []
    rows: list[tuple[str, str, str]] = []
    for number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        fields = tuple(field.strip() for field in line.split("|"))
        if len(fields) != 3 or not all(fields):
            violations.append(f"invalid history map row {number}")
            continue
        rows.append(fields)  # type: ignore[arg-type]
    return rows


def validate_contract(
    repo_root: Path, contract_path: Path, checked_ref: str = "HEAD"
) -> dict[str, Any]:
    violations: list[str] = []
    try:
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return {
            "ok": False,
            "checked_ref": checked_ref,
            "packages": [],
            "vendor_trees": [],
            "violations": [f"cannot read contract: {error}"],
        }

    if contract.get("schema_version") != 1:
        violations.append("schema_version must be 1")
    if contract.get("api_compatibility_line") != "0.1":
        violations.append("api_compatibility_line must be 0.1")
    if contract.get("vendor_patch_strategy") != "consumer-root-path-patch":
        violations.append("vendor patch strategy must be consumer-root-path-patch")

    lkg = contract.get("last_known_good_rterm_ref")
    if not isinstance(lkg, str) or SHA1.fullmatch(lkg) is None:
        violations.append(
            "last_known_good_rterm_ref must be an immutable 40-character lowercase commit"
        )
    elif run_git(repo_root, "cat-file", "-e", f"{lkg}^{{commit}}").returncode != 0:
        violations.append(f"last-known-good commit is unavailable: {lkg}")

    try:
        metadata = cargo_packages(repo_root)
    except RuntimeError as error:
        metadata = {}
        violations.append(str(error))

    entries = contract.get("packages")
    if not isinstance(entries, list):
        entries = []
        violations.append("packages must be a list")
    declared_names = {
        entry.get("name") for entry in entries if isinstance(entry, dict)
    }
    if declared_names != set(EXPECTED_PACKAGES):
        violations.append("contract must declare exactly the seven public R-Term packages")

    for entry in entries:
        if not isinstance(entry, dict):
            violations.append("package entries must be objects")
            continue
        name = entry.get("name")
        path = entry.get("path")
        version = entry.get("version")
        dependencies = entry.get("dependencies", [])
        if name not in EXPECTED_PACKAGES:
            continue
        if path != EXPECTED_PACKAGES[name]:
            violations.append(f"{name} path must be {EXPECTED_PACKAGES[name]}")
        if not isinstance(path, str) or not (repo_root / path).is_dir():
            violations.append(f"missing package path for {name}: {path}")
        if version != "0.1.0":
            violations.append(f"{name} version must be 0.1.0")
        if not isinstance(dependencies, list):
            violations.append(f"{name} dependencies must be a list")
            dependencies = []
        for dependency in dependencies:
            if isinstance(dependency, str) and dependency.startswith("rssh-"):
                violations.append(f"{name} declares reverse dependency {dependency}")

        actual = metadata.get(name)
        if actual is None:
            violations.append(f"cargo metadata is missing {name}")
            continue
        if str(actual["version"]) != version:
            violations.append(
                f"{name} manifest version {actual['version']} does not match contract version {version}"
            )
        if isinstance(path, str) and relative_manifest_directory(repo_root, actual) != path:
            violations.append(f"{name} Cargo package path does not match contract")
        actual_rterm_dependencies = sorted(
            dependency["name"]
            for dependency in actual["dependencies"]
            if dependency["name"].startswith("rterm-")
        )
        if sorted(dependencies) != actual_rterm_dependencies:
            violations.append(f"{name} R-Term dependencies do not match Cargo metadata")
        for dependency in actual["dependencies"]:
            if dependency["name"].startswith("rssh-"):
                violations.append(
                    f"{name} Cargo metadata has reverse dependency {dependency['name']}"
                )

    vendors = contract.get("vendor_trees")
    if not isinstance(vendors, list):
        vendors = []
        violations.append("vendor_trees must be a list")
    vendor_names = {
        entry.get("name") for entry in vendors if isinstance(entry, dict)
    }
    if vendor_names != set(EXPECTED_VENDORS):
        violations.append("contract must declare glyphon and gpu-allocator vendor trees")
    for entry in vendors:
        if not isinstance(entry, dict):
            continue
        name = entry.get("name")
        path = entry.get("path")
        expected_tree = entry.get("tree")
        if name not in EXPECTED_VENDORS:
            continue
        if path != EXPECTED_VENDORS[name]:
            violations.append(f"{name} vendor path must be {EXPECTED_VENDORS[name]}")
        if not isinstance(expected_tree, str) or SHA1.fullmatch(expected_tree) is None:
            violations.append(f"{name} vendor tree must be a 40-character lowercase object ID")
            continue
        observed = run_git(repo_root, "rev-parse", f"{checked_ref}:{path}")
        if observed.returncode != 0:
            violations.append(f"missing vendor path at {checked_ref}: {path}")
        elif observed.stdout.strip() != expected_tree:
            violations.append(
                f"vendor tree drift for {name}: expected {expected_tree}, observed {observed.stdout.strip()}"
            )

    history_path_value = contract.get("history_map_file")
    if not isinstance(history_path_value, str):
        violations.append("history_map_file must be a repository-relative path")
        history_rows: list[tuple[str, str, str]] = []
    else:
        history_rows = parse_history_map(repo_root / history_path_value, violations)
    components = {row[0] for row in history_rows}
    missing_components = sorted(REQUIRED_HISTORY_COMPONENTS - components)
    if missing_components:
        violations.append(
            "history map is missing components: " + ", ".join(missing_components)
        )
    for component, historical, current in history_rows:
        if not (repo_root / current).exists():
            violations.append(f"history map current path is missing for {component}: {current}")
        history = run_git(repo_root, "log", "--all", "--format=%H", "--", historical)
        if history.returncode != 0 or not history.stdout.strip():
            violations.append(
                f"history map has no Git history for {component}: {historical}"
            )

    return {
        "ok": not violations,
        "checked_ref": checked_ref,
        "packages": sorted(
            name for name in declared_names if isinstance(name, str)
        ),
        "vendor_trees": sorted(
            name for name in vendor_names if isinstance(name, str)
        ),
        "violations": violations,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--ref", default="HEAD")
    arguments = parser.parse_args()

    report = validate_contract(
        arguments.repo_root.resolve(), arguments.contract.resolve(), arguments.ref
    )
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
