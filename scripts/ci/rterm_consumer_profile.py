"""Pure declaration checks and narrowly scoped R-Term build-profile generation.

This module does not verify Git provenance, write files, or certify compatibility.
The preparation caller must first verify immutable source/consumer trees. A
selected profile still requires compilation and real functional verification.
"""

from __future__ import annotations

import copy
import json
import re
import tomllib
from typing import Any


COMMIT = re.compile(r"[0-9a-f]{40}")
LEGACY_SELECTOR = "rterm-legacy-0-1"
FORWARDING = {
    "production-fonts": ["rterm-fonts/shared-source-ownership"],
    "diagnostic-tools": ["rterm-fonts/diagnostic-tools"],
}


def manifest_document(raw: bytes, package: str) -> dict[str, Any]:
    """Parse a manifest without accepting a different package or feature shape."""
    document = tomllib.loads(raw.decode("utf-8"))
    if not isinstance(document.get("package"), dict) or document["package"].get("name") != package:
        raise ValueError(f"manifest must identify package {package}")
    features = document.get("features", {})
    if not isinstance(features, dict) or any(
        not isinstance(values, list)
        or any(not isinstance(value, str) for value in values)
        for values in features.values()
    ):
        raise ValueError("feature capabilities must be arrays of strings")
    return document


def select_profile(source_ref: str, lkg_ref: str, fonts_manifest: bytes) -> str:
    """Choose a declared profile; source identity must be verified by the caller."""
    if any(not isinstance(ref, str) or COMMIT.fullmatch(ref) is None for ref in (source_ref, lkg_ref)):
        raise ValueError("source and LKG must be immutable lowercase commit IDs")
    document = manifest_document(fonts_manifest, "rterm-fonts")
    features = document.get("features", {})
    if source_ref == lkg_ref:
        if features:
            raise ValueError("frozen R-Term source must have its original feature-free manifest")
        return "legacy-0.1"
    if not all(name in features for name in ("diagnostic-tools", "shared-source-ownership")):
        raise ValueError("unknown R-Term capabilities; refusing implicit legacy fallback")
    return "modern"


def prepare_app_manifest(original: bytes, profile: str) -> bytes:
    """Return bytes for one profile, never editing source or dependency packages.

Legacy generation permits exactly three feature-array changes. The final parsed
document comparison prevents an edit to similarly named keys in other tables.
Unsupported layouts fail rather than being rewritten by a general TOML writer.
"""
    if profile not in ("modern", "legacy-0.1"):
        raise ValueError(f"unknown consumer profile: {profile}")
    document = manifest_document(original, "rterm-app")
    features = document.get("features", {})
    if any(LEGACY_SELECTOR in values for values in features.values()):
        raise ValueError("legacy selector is already enabled; require pristine consumer manifest")
    if profile == "modern":
        return original
    if features.get(LEGACY_SELECTOR) != []:
        raise ValueError("legacy selector must be declared and inert")
    for name, values in FORWARDING.items():
        if features.get(name) != values:
            raise ValueError(f"unexpected {name} forwarding; refusing to erase consumer features")
    production = features.get("production-gui", [])
    if "production-fonts" not in production:
        raise ValueError("production-gui must retain the production-fonts edge")

    updates = {name: [] for name in FORWARDING}
    updates["production-gui"] = [*production, LEGACY_SELECTOR]
    expected = copy.deepcopy(document)
    expected["features"].update(updates)
    prepared = original.decode("utf-8")
    for name, values in updates.items():
        pattern = re.compile(rf"^{re.escape(name)} = \[[^\r\n]*\](\r?)$", re.MULTILINE)
        prepared, count = pattern.subn(
            lambda match: f"{name} = {json.dumps(values, ensure_ascii=False)}{match.group(1)}",
            prepared,
        )
        if count != 1:
            raise ValueError(f"{name} requires exactly one unambiguous single-line feature array")
    if tomllib.loads(prepared) != expected:
        raise ValueError("manifest generation changed data outside the three allowed feature arrays")
    return prepared.encode("utf-8")
