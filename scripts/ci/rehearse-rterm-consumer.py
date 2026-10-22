#!/usr/bin/env python3
"""Rehearse candidate and rollback R-Term sources in clean consumer clones."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import runpy
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path, PurePosixPath
from typing import Any, Iterable


SHA1 = re.compile(r"^[0-9a-f]{40}$")
FORBIDDEN_PRODUCT_ROOTS = (
    "crates/rterm-app",
    "crates/rterm-config",
    "crates/rterm-core",
    "crates/rterm-diagnostics",
    "crates/rterm-domain",
    "crates/rterm-functional-tests",
    "crates/rterm-native",
    "crates/rterm-pty",
    "crates/rterm-renderer",
    "crates/rterm-ssh",
    "crates/rterm-test-support",
    "crates/rterm-web",
    "tauri",
)


class RehearsalError(RuntimeError):
    pass


def git(repo: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )


def resolve_commit(repo: Path, reference: str) -> str:
    result = git(repo, "rev-parse", "--verify", f"{reference}^{{commit}}")
    commit = result.stdout.strip()
    if result.returncode != 0 or SHA1.fullmatch(commit) is None:
        raise RehearsalError(f"cannot resolve immutable commit for {reference}: {result.stderr.strip()}")
    return commit


def validate_overlay_path(value: str) -> str:
    path = PurePosixPath(value)
    if (
        not value
        or path.is_absolute()
        or value in (".", "..")
        or ".." in path.parts
        or "\\" in value
    ):
        raise RehearsalError(f"refusing overlay path outside repository: {value}")
    normalized = path.as_posix()
    if any(
        normalized == root or normalized.startswith(f"{root}/")
        for root in FORBIDDEN_PRODUCT_ROOTS
    ):
        raise RehearsalError(f"refusing overlay path owned by R-Term product code: {value}")
    return normalized


def contract_overlay_paths(contract: dict[str, Any]) -> list[str]:
    paths: list[str] = []
    for section in ("packages", "vendor_trees"):
        entries = contract.get(section)
        if not isinstance(entries, list):
            raise RehearsalError(f"{section} must be a list")
        for entry in entries:
            if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
                raise RehearsalError(f"{section} entries must declare a path")
            normalized = validate_overlay_path(entry["path"])
            if normalized not in paths:
                paths.append(normalized)
    for index, left in enumerate(paths):
        for right in paths[index + 1 :]:
            if left.startswith(f"{right}/") or right.startswith(f"{left}/"):
                raise RehearsalError(f"refusing overlapping overlay paths: {left}, {right}")
    return paths


def command_list(value: Any, label: str) -> list[str]:
    if not isinstance(value, list) or not value or not all(isinstance(item, str) and item for item in value):
        raise RehearsalError(f"{label} must be a nonempty argument list")
    return list(value)


def clone_at(source: Path, destination: Path, commit: str) -> None:
    if destination.exists():
        raise RehearsalError(f"refusing existing rehearsal checkout: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    cloned = subprocess.run(
        ["git", "clone", "--no-local", "--no-checkout", "--quiet", str(source), str(destination)],
        text=True,
        capture_output=True,
        check=False,
    )
    if cloned.returncode != 0:
        raise RehearsalError(f"clean clone failed: {cloned.stderr.strip()}")
    checked_out = git(destination, "checkout", "--detach", "--quiet", commit)
    if checked_out.returncode != 0:
        raise RehearsalError(f"checkout failed for {commit}: {checked_out.stderr.strip()}")


def overlay_paths(source: Path, consumer: Path, paths: Iterable[str]) -> None:
    for relative in paths:
        source_path = source / relative
        destination = consumer / relative
        if not source_path.exists():
            raise RehearsalError(f"overlay source path is missing: {relative}")
        if destination.is_dir():
            shutil.rmtree(destination)
        elif destination.exists():
            destination.unlink()
        destination.parent.mkdir(parents=True, exist_ok=True)
        if source_path.is_dir():
            shutil.copytree(source_path, destination, symlinks=True)
        else:
            shutil.copy2(source_path, destination)


def run_command(
    arguments: list[str], cwd: Path, environment: dict[str, str], kind: str
) -> dict[str, Any]:
    started = time.perf_counter()
    result = subprocess.run(
        arguments,
        cwd=cwd,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    return {
        "kind": kind,
        "argv": arguments,
        "cwd": str(cwd),
        "returncode": result.returncode,
        "elapsed_ms": round((time.perf_counter() - started) * 1000, 3),
        "stdout": result.stdout[-16_384:],
        "stderr": result.stderr[-16_384:],
    }


def write_json_atomic(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(f"{path.suffix}.tmp")
    temporary.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    os.replace(temporary, path)


def remove_readonly_tree(path: Path) -> None:
    def retry(function: Any, failed_path: str, _error: Any) -> None:
        os.chmod(failed_path, stat.S_IWRITE)
        function(failed_path)

    shutil.rmtree(path, onerror=retry)


def artifact_specs(contract: dict[str, Any]) -> list[dict[str, Any]]:
    specs = contract.get("consumer_artifacts", [])
    if not isinstance(specs, list):
        raise RehearsalError("consumer_artifacts must be a list")
    seen = set()
    for spec in specs:
        if not isinstance(spec, dict) or set(spec) != {"after_command", "path"}:
            raise RehearsalError("artifact requires after_command and path")
        index, path = spec["after_command"], spec["path"]
        if type(index) is not int or not 0 <= index < len(contract.get("consumer_commands", [])):
            raise RehearsalError("artifact command index is out of range")
        if not isinstance(path, str):
            raise RehearsalError("artifact path must be a string")
        resolved = path.replace("{exe_suffix}", ".exe" if os.name == "nt" else "")
        if (not resolved or any(c in resolved for c in "\\:{}")
                or PurePosixPath(resolved).is_absolute()
                or any(p in ("", ".", "..") for p in resolved.split("/"))
                or resolved in seen):
            raise RehearsalError("artifact path must be unique and contained in Cargo target")
        seen.add(resolved)
    return specs


def artifact_identity(target: Path, spec: dict[str, Any]) -> dict[str, Any]:
    relative = spec["path"].replace("{exe_suffix}", ".exe" if os.name == "nt" else "")
    path = target / relative
    for component in (path, *path.parents):
        info = component.lstat()
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise ValueError(f"artifact path contains a link or reparse point: {component}")
    if not path.is_file():
        raise ValueError(f"artifact is not a regular file: {path}")
    digest, size = hashlib.sha256(), 0
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
            size += len(chunk)
    if not size:
        raise ValueError(f"artifact is empty: {path}")
    return {"after_command": spec["after_command"], "path": relative,
            "sha256": digest.hexdigest(), "size_bytes": size}


def artifact_failure(error: Exception) -> dict[str, Any]:
    return {"kind": "artifact-identity", "argv": [], "returncode": 1,
            "stdout": "", "stderr": str(error)}


def verify_recovery_path(work: Path, backup: Path) -> None:
    for component in (backup, *backup.parents):
        try:
            info = component.lstat()
        except FileNotFoundError:
            continue
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise ValueError(f"recovery path contains a link or reparse point: {component}")
    if not backup.resolve().is_relative_to(work.resolve()):
        raise ValueError("recovery path escapes owned work directory")


def run_mode(
    *,
    mode: str,
    repo: Path,
    work: Path,
    output: Path,
    contract: dict[str, Any],
    source_commit: str,
    consumer_commit: str,
    candidate_probe: Path | None,
    overlay: list[str],
    contract_path: Path | None = None,
) -> tuple[dict[str, Any], Path]:
    package_mode = contract.get("consumer_package")
    if package_mode not in (None, "native-unsigned-v1"):
        raise RehearsalError("unknown consumer package mode")
    package_tests = contract.get("consumer_package_tests")
    if package_tests not in (None, "native-gui-ssh-gpu-v1") or (package_tests and not package_mode):
        raise RehearsalError("package scenarios require the native package contract")
    specs = artifact_specs(contract)
    artifacts: list[dict[str, Any]] = []
    prior_artifacts: list[dict[str, Any]] = []
    source = work / f"{mode}-rterm"
    consumer = work / f"{mode}-consumer"
    clone_at(repo, source, source_commit)
    verified = contract.get("consumer_preparation") == "verified-dual-adapter-v1"
    prepared_root = work / f"{mode}-prepared"
    if verified:
        consumer = prepared_root / "consumer"
    else:
        clone_at(repo, consumer, consumer_commit)
        overlay_paths(source, consumer, overlay)

    probe = contract.get("standalone_probe")
    if not isinstance(probe, dict) or not isinstance(probe.get("path"), str):
        raise RehearsalError("standalone_probe must declare a path and command")
    probe_relative = PurePosixPath(probe["path"])
    if probe_relative.is_absolute() or ".." in probe_relative.parts:
        raise RehearsalError(f"invalid standalone probe path: {probe_relative}")
    probe_root = source / probe_relative.as_posix()
    copied_probe = False
    if not probe_root.exists():
        if candidate_probe is None or not candidate_probe.exists():
            raise RehearsalError(f"standalone probe is missing: {probe_relative}")
        probe_root.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(candidate_probe, probe_root)
        copied_probe = True

    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(work / "cargo-target"))
    target = Path(environment["CARGO_TARGET_DIR"])
    if not target.is_absolute():
        target = consumer / target
    target = Path(os.path.abspath(target))
    environment["CARGO_TARGET_DIR"] = str(target)
    environment.update(
        {
            "RTERM_REHEARSAL_MODE": mode,
            "RTERM_REHEARSAL_SOURCE_REF": source_commit,
            "RTERM_REHEARSAL_CONSUMER_REF": consumer_commit,
        }
    )
    commands: list[dict[str, Any]] = []
    if copied_probe and probe.get("rollback_prepare_command") is not None:
        command = command_list(
            probe["rollback_prepare_command"], "rollback_prepare_command"
        )
        commands.append(run_command(command, probe_root, environment, "probe-prepare"))
    if not commands or commands[-1]["returncode"] == 0:
        commands.append(
            run_command(
                command_list(probe.get("command"), "standalone_probe command"),
                probe_root,
                environment,
                "standalone-probe",
            )
        )

    consumer_prepare = contract.get("consumer_prepare_command")
    preparation = None
    if verified and all(command["returncode"] == 0 for command in commands):
        if contract_path is None or consumer_prepare != ["cargo", "generate-lockfile"]:
            raise RehearsalError("verified preparation requires the immutable contract and cargo generate-lockfile")
        command = run_command([
            sys.executable, str(Path(__file__).with_name("prepare-rterm-consumer.py")),
            "--repo", str(repo), "--source-ref", source_commit,
            "--consumer-ref", consumer_commit, "--contract", str(contract_path),
            "--output-dir", str(prepared_root),
        ], repo, environment, "verified-consumer-prepare")
        commands.append(command)
        receipt_path = prepared_root / "preparation.json"
        preparation = json.loads(receipt_path.read_text()) if receipt_path.exists() else {
            "ok": False, "error": command["stderr"],
        }
        if command["returncode"] == 0 and preparation.get("ok") is not True:
            command["returncode"] = 1
            command["stderr"] = "preparation did not return a successful receipt"
    if (
        all(command["returncode"] == 0 for command in commands)
        and consumer_prepare is not None
        and not verified
    ):
        commands.append(
            run_command(
                command_list(consumer_prepare, "consumer_prepare_command"),
                consumer,
                environment,
                "consumer-prepare",
            )
        )
    consumer_commands = contract.get("consumer_commands")
    if not isinstance(consumer_commands, list) or not consumer_commands:
        raise RehearsalError("consumer_commands must be a nonempty list")
    for index, value in enumerate(consumer_commands):
        if any(command["returncode"] != 0 for command in commands):
            break
        try:
            for spec in specs:
                if spec["after_command"] != index:
                    continue
                relative = spec["path"].replace("{exe_suffix}", ".exe" if os.name == "nt" else "")
                prior = target / relative
                if prior.exists() or prior.is_symlink():
                    old_identity = artifact_identity(target, spec)
                    # Move only the checked regular file, never directories.
                    # Retain it on failure, without discarding Cargo's deps cache.
                    backup = work / f"{mode}-prior-artifacts" / relative
                    verify_recovery_path(work, backup)
                    backup.parent.mkdir(parents=True, exist_ok=True)
                    verify_recovery_path(work, backup)
                    if backup.exists():
                        raise ValueError(f"refusing existing prior artifact backup: {backup}")
                    shutil.move(str(prior), str(backup))
                    prior_artifacts.append({**old_identity, "backup_path": str(backup)})
        except (OSError, ValueError) as error:
            commands.append(artifact_failure(error))
            break
        commands.append(
            run_command(
                command_list(value, f"consumer command {index}"),
                consumer,
                environment,
                "consumer",
            )
        )
        if commands[-1]["returncode"] == 0:
            try:
                for spec in specs:
                    if spec["after_command"] == index:
                        artifacts.append(artifact_identity(target, spec))
            except (OSError, ValueError) as error:
                commands.append(artifact_failure(error))

    package = None
    if package_mode and all(command["returncode"] == 0 for command in commands):
        if not verified or len(artifacts) != 1:
            commands.append(artifact_failure(ValueError("native package requires one verified executable")))
        else:
            packager = runpy.run_path(str(Path(__file__).with_name("rterm_package_evidence.py")))
            package = packager["assemble_package"](consumer, target, artifacts[0], preparation,
                output, mode, environment, run_command, verify_recovery_path, test_mode=package_tests)
            commands.extend(package["commands"])
            if not package["ok"]:
                commands.append(artifact_failure(ValueError(package["error"])))

    # Keep the original hash even on failure; later tests may have rebuilt or
    # modified the executable. Record each mode before the next overwrites it.
    try:
        for artifact in artifacts:
            if artifact_identity(target, artifact) != artifact:
                raise ValueError(f"artifact changed after its build command: {artifact['path']}")
    except (OSError, ValueError) as error:
        commands.append(artifact_failure(error))

    identity = None
    if preparation is not None and preparation.get("ok") is True:
        try:
            verify_prepared_identity(repo, prepared_root, preparation, contract_path)
            identity = {"ok": True}
        except (OSError, ValueError, KeyError) as error:
            identity = {"ok": False, "error": str(error)}
    evidence = {
        "schema_version": 1,
        "mode": mode,
        "ok": bool(commands) and all(command["returncode"] == 0 for command in commands)
        and (not verified or identity == {"ok": True}),
        "source_commit": source_commit,
        "consumer_commit": consumer_commit,
        "overlay_paths": overlay,
        "probe_copied_from_candidate": copied_probe,
        "commands": commands,
        "artifacts": artifacts,
        "prior_artifacts": prior_artifacts,
        "package": package,
    }
    if verified:
        evidence.update(preparation=preparation, post_consumer_identity=identity,
                        prepared_root=str(prepared_root))
    write_json_atomic(output / f"{mode}.json", evidence)
    if not evidence["ok"]:
        failed = next((command for command in commands if command["returncode"] != 0), identity)
        # Include bounded, JSON-escaped command output in the job log as well
        # as the artifact, so failed hosted jobs remain diagnosable.
        print(json.dumps({"mode": mode, "failed_command": failed}, sort_keys=True), file=sys.stderr)
    return evidence, probe_root


def verify_prepared_identity(repo: Path, root: Path, receipt: dict, contract_path: Path) -> None:
    """Check actual checkout bytes again, never Git index flags or a success marker."""
    preparer = runpy.run_path(str(Path(__file__).with_name("prepare-rterm-consumer.py")))
    source, consumer = receipt["source_commit"], receipt["consumer_commit"]
    _, overlay, digest = preparer["validated_contract"](repo, consumer, contract_path)
    if digest != receipt["contract_sha256"]:
        raise ValueError("preparation contract identity changed")
    expected = preparer["expected_files"](repo, source, consumer, overlay)
    manifest = preparer["APP_MANIFEST"]
    original = preparer["git"](repo, "show", f"{consumer}:{manifest}")
    generated = preparer["prepare_app_manifest"](original, receipt["profile"])
    lock = (root / "consumer/Cargo.lock").read_bytes()
    if hashlib.sha256(lock).hexdigest() != receipt["lockfile_sha256"]:
        raise ValueError("prepared lockfile changed during consumer commands")
    preparer["verify_checkout"](root / "source", preparer["tree_files"](repo, source), {})
    preparer["verify_checkout"](root / "consumer", expected, {manifest: generated, "Cargo.lock": lock})


def verify_retained_packages(output: Path, modes: list[dict[str, Any]]) -> bool:
    valid = True
    for evidence in modes:
        package = evidence.get("package")
        if package is None:
            continue
        try:
            current = artifact_identity(output, {"path": package["path"], "after_command": 0})
            if any(current[key] != package[key] for key in ("sha256", "size_bytes")):
                raise ValueError("retained package changed after mode completion")
        except (OSError, ValueError, KeyError) as error:
            evidence["ok"] = package["ok"] = False
            package["error"] = str(error)
            write_json_atomic(output / f"{evidence['mode']}.json", evidence)
            print(json.dumps({"mode": evidence["mode"], "package_error": str(error)}), file=sys.stderr)
            valid = False
    return valid


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--candidate-ref", required=True)
    parser.add_argument("--consumer-ref", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args()

    try:
        repo = arguments.repo.resolve()
        output = arguments.output_dir.resolve()
        contract = json.loads(arguments.contract.read_text(encoding="utf-8"))
        candidate_commit = resolve_commit(repo, arguments.candidate_ref)
        consumer_commit = resolve_commit(repo, arguments.consumer_ref)
        committed_contract = subprocess.run(
            ["git", "-C", str(repo), "show", f"{consumer_commit}:scripts/ci/rterm-release-contract.json"],
            capture_output=True, check=False,
        )
        if committed_contract.returncode == 0:
            # The immutable product contract, not an untrusted selector, decides
            # which preparation policy applies. Legacy generic fixtures without
            # a product contract retain their explicit external-contract support.
            if arguments.contract.read_bytes() != committed_contract.stdout:
                raise RehearsalError("contract must match the immutable consumer commit")
            contract = json.loads(committed_contract.stdout)
        preparation_mode = contract.get("consumer_preparation")
        if preparation_mode not in (None, "verified-dual-adapter-v1"):
            raise RehearsalError("unknown consumer preparation mode")
        overlay = contract_overlay_paths(contract)
        if preparation_mode is not None:
            # Validate before executing even the standalone probe: its command
            # is contract-controlled, too. The preparer validates again later.
            preparer = runpy.run_path(str(Path(__file__).with_name("prepare-rterm-consumer.py")))
            contract, overlay, _ = preparer["validated_contract"](
                repo, consumer_commit, arguments.contract.resolve()
            )
        rollback_ref = contract.get("last_known_good_rterm_ref")
        if not isinstance(rollback_ref, str) or SHA1.fullmatch(rollback_ref) is None:
            raise RehearsalError("last_known_good_rterm_ref must be an immutable commit")
        rollback_commit = resolve_commit(repo, rollback_ref)
        work = output / "work"
        if work.exists() or any((output / f"{mode}.json").exists() for mode in ("candidate", "rollback")):
            raise RehearsalError(f"refusing existing rehearsal work directory: {work}")
        if preparation_mode is not None:
            # Owned temporary checkout root; receipts remain in output on cleanup.
            # The preparer deliberately rejects directories inside the source repo.
            # macOS exposes its system temporary directory through /var ->
            # /private/var. Canonicalize this newly created, owned directory;
            # downstream checks must still reject links in supplied paths.
            work = Path(tempfile.mkdtemp(prefix="rterm-verified-rehearsal-")).resolve()

        candidate, candidate_probe = run_mode(
            mode="candidate",
            repo=repo,
            work=work,
            output=output,
            contract=contract,
            source_commit=candidate_commit,
            consumer_commit=consumer_commit,
            candidate_probe=None,
            overlay=overlay,
            contract_path=arguments.contract.resolve(),
        )
        if not candidate["ok"]:
            return 1
        rollback, _ = run_mode(
            mode="rollback",
            repo=repo,
            work=work,
            output=output,
            contract=contract,
            source_commit=rollback_commit,
            consumer_commit=consumer_commit,
            candidate_probe=candidate_probe,
            overlay=overlay,
            contract_path=arguments.contract.resolve(),
        )
        if not rollback["ok"]:
            return 1
        if not verify_retained_packages(output, [candidate, rollback]):
            return 1
        remove_readonly_tree(work)
        print(
            json.dumps(
                {
                    "ok": True,
                    "candidate_commit": candidate_commit,
                    "consumer_commit": consumer_commit,
                    "rollback_commit": rollback_commit,
                },
                sort_keys=True,
                separators=(",", ":"),
            )
        )
        return 0
    except (OSError, ValueError, RehearsalError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
