"""Assemble existing native packages and bind archive bytes to a verified binary."""

import hashlib
import json
import os
import platform
import re
import shutil
import stat
import tarfile
import tomllib
import zipfile
from pathlib import Path, PurePosixPath


def digest_file(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify_archive(archive, payload, expected_binary_hash, consumer_commit):
    expected = {}
    expected_modes = {}
    for parent, directories, files in os.walk(payload):
        for name in directories + files:
            info = (Path(parent) / name).lstat()
            if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
                raise ValueError("package payload contains a link or reparse point")
        for name in files:
            path = Path(parent) / name
            expected["payload/" + path.relative_to(payload).as_posix()] = digest_file(path)
            expected_modes["payload/" + path.relative_to(payload).as_posix()] = stat.S_IMODE(path.stat().st_mode)
    actual = {}
    archive_modes = {}
    with (zipfile.ZipFile(archive) if archive.suffix == ".zip" else tarfile.open(archive)) as bundle:
        entries = bundle.infolist() if archive.suffix == ".zip" else bundle.getmembers()
        for entry in entries:
            name = entry.filename if archive.suffix == ".zip" else entry.name
            if ("\\" in name or ":" in name or PurePosixPath(name).is_absolute()
                    or ".." in PurePosixPath(name).parts):
                raise ValueError("unsafe package archive member")
            if archive.suffix == ".zip":
                if stat.S_ISLNK(entry.external_attr >> 16):
                    raise ValueError("linked package archive member")
                if entry.is_dir():
                    continue
                stream = bundle.open(entry)
            else:
                if entry.isdir():
                    continue
                if not entry.isfile():
                    raise ValueError("non-regular package archive member")
                archive_modes[name] = entry.mode
                stream = bundle.extractfile(entry)
            if name in actual:
                stream.close()
                raise ValueError("duplicate package archive member")
            with stream:
                actual[name] = hashlib.file_digest(stream, "sha256").hexdigest()
    if actual != expected:
        raise ValueError("package archive differs from assembled payload")
    manifest = json.loads((payload / "manifest.json").read_text(encoding="utf-8"))
    if (not isinstance(manifest, dict)
            or any(not isinstance(manifest.get(key), dict) for key in ("artifact", "package", "signing"))
            or not isinstance(manifest["artifact"].get("binary"), str)):
        raise ValueError("malformed native package manifest")
    binary = manifest["artifact"]["binary"]
    if archive.suffix != ".zip":
        if os.name != "nt" and archive_modes != expected_modes:
            raise ValueError("package archive permissions differ from payload")
        for relative in (binary, "rterm-console.sh", "rterm"):
            member = "payload/" + relative
            if member in actual and not archive_modes[member] & 0o111:
                raise ValueError("packaged executable lacks execute permission")
    if actual.get("payload/" + binary) != expected_binary_hash:
        raise ValueError("packaged executable differs from verified build")
    if manifest["package"]["source_commit"] != consumer_commit or manifest["signing"]["unsigned"] is not True:
        raise ValueError("package provenance or unsigned status mismatch")
    return binary


def run_package_tests(consumer, binary, environment, run_command):
    scenarios = [
        ("openssh_loopback", "rterm_app_native_ssh_disconnects_and_reconnects_with_closed_lifecycle", False),
        ("native_window_e2e", "native_window_e2e_presents_ten_frames_from_a_real_pty", False),
        ("native_window_e2e", "native_window_e2e_preserves_gpu_text_at_scale_100", True),
    ]
    result = {"ok": False, "binary": str(binary), "binary_sha256": digest_file(binary),
              "cargo_target_dir": environment["CARGO_TARGET_DIR"], "commands": []}
    env = {**environment, "RTERM_TEST_APP_EXECUTABLE": str(binary), "RTERM_REQUIRE_OPENSSH": "1"}
    for suite, scenario, ignored in scenarios:
        command = ["cargo", "test", "--locked", "-p", "rterm-app",
                   "--no-default-features", "--features", "production-gui,transfer-tools",
                   "--test", suite, scenario, "--", "--exact", "--nocapture", "--test-threads=1"]
        if ignored:
            command.append("--ignored")
        record = run_command(command, consumer, env, "packaged-functional")
        result["commands"].append(record)
        if record["returncode"] != 0:
            return result
        if not re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;", record["stdout"]):
            record["returncode"] = 1
            record["stderr"] += "\nrequired package scenario did not execute exactly one passing test"
            return result
    result["ok"] = True
    return result


def assemble_package(consumer, target, artifact, receipt, output, mode, environment, run_command, verify_path, test_mode=None):
    result = {"ok": False, "commands": []}
    try:
        host = {"Windows": "windows", "Linux": "linux", "Darwin": "macos"}[platform.system()]
        arch = {"AMD64": "x86_64", "x86_64": "x86_64", "ARM64": "aarch64", "arm64": "aarch64", "aarch64": "aarch64"}[platform.machine()]
        runtime = f"{host}-{arch}"
        pty = "windows-conpty" if host == "windows" else "unix-pty"
        extension = "zip" if host == "windows" else "tar.gz"
        name = f"rterm-{mode}-{runtime}-unsigned.{extension}"
        package_output = output / f"{mode}-package"
        verify_path(output, package_output)
        package_output.mkdir(parents=True, exist_ok=False)
        verify_path(output, package_output)
        payload = package_output / "payload"
        archive = package_output / name
        app = tomllib.loads((consumer / "crates/rterm-app/Cargo.toml").read_text())["package"]
        version = app["version"]
        if isinstance(version, dict):
            version = tomllib.loads((consumer / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        values = [str(target / artifact["path"]), str(payload), name, runtime, pty, version]
        if host == "windows":
            command = [shutil.which("pwsh") or "powershell.exe", "-NoProfile", "-NonInteractive", "-File", str(consumer / "scripts/ci/package-native.ps1")]
            flags = ["-Binary", "-PackageRoot", "-ArtifactName", "-RuntimeTarget", "-PtyBackend", "-Version"]
            unsigned = "-Unsigned"
        else:
            command = ["bash", str(consumer / "scripts/ci/package-native.sh")]
            flags = ["--binary", "--package-root", "--artifact-name", "--runtime-target", "--pty-backend", "--version"]
            unsigned = "--unsigned"
        for flag, value in zip(flags, values):
            command.extend([flag, value])
        command.append(unsigned)
        result["commands"].append(run_command(command, consumer, {**environment, "GITHUB_SHA": receipt["consumer_commit"]}, "native-package"))
        if result["commands"][-1]["returncode"] != 0:
            raise ValueError("native packaging command failed")
        verify_path(output, archive)
        verify_path(output, payload)
        binary = verify_archive(archive, payload, artifact["sha256"], receipt["consumer_commit"])
        result.update(ok=True, path=archive.relative_to(output).as_posix(),
                      sha256=digest_file(archive), size_bytes=archive.stat().st_size,
                      binary=binary, binary_sha256=artifact["sha256"],
                      runtime_target=runtime, profile=receipt["profile"],
                      consumer_commit=receipt["consumer_commit"], source_commit=receipt["source_commit"])
        if test_mode:
            # Integration tests implicitly rebuild the app with dev-dependency
            # features, even when its explicit features match the production
            # build. Keep that output separate from the attested executable.
            test_target = target / "packaged-functional"
            verify_path(target, test_target)
            test_target.mkdir(parents=True, exist_ok=True)
            verify_path(target, test_target)
            test_environment = {**environment, "CARGO_TARGET_DIR": str(test_target)}
            result["tests"] = run_package_tests(consumer, payload / binary, test_environment, run_command)
            result["commands"].extend(result["tests"]["commands"])
            if not result["tests"]["ok"]:
                raise ValueError("packaged functional scenario failed or was not executed")
            verify_path(output, archive)
            verify_path(output, payload)
            verify_archive(archive, payload, artifact["sha256"], receipt["consumer_commit"])
    except (OSError, ValueError, KeyError, zipfile.BadZipFile, tarfile.TarError) as error:
        result["ok"] = False
        result["error"] = str(error)
    return result
