import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from test_rterm_consumer_profile import APP, MODERN_FONTS, OLD_FONTS


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "scripts/ci/prepare-rterm-consumer.py"
PACKAGES = {
    "rterm-types": "crates/rterm-types",
    "rterm-terminal": "crates/rterm-terminal",
    "rterm-runtime": "crates/rterm-runtime",
    "rterm-fonts": "crates/rterm-fonts",
    "rterm-render-core": "crates/rterm-render-core",
    "rterm-render-cpu": "crates/rterm-render-cpu",
    "rterm-render-wgpu": "crates/rterm-render-wgpu",
}
VENDORS = {"glyphon": "vendor/glyphon-0.12.0", "gpu-allocator": "vendor/gpu-allocator-0.28.0"}


def git(root, *args):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, check=True)
    return result.stdout.decode().strip()


class PrepareConsumerTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "verified consumer preparer is missing")
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.repo = self.root / "repo"
        self.repo.mkdir()
        git(self.repo, "init", "-b", "main")
        git(self.repo, "config", "user.name", "Fixture")
        git(self.repo, "config", "user.email", "fixture@example.invalid")
        self.write(".gitattributes", "* text=auto eol=lf\n")
        self.write(".gitignore", "ignored.txt\n")
        self.write("Cargo.toml", '[workspace]\nmembers = ["crates/rterm-app"]\nresolver = "2"\n')
        self.write("crates/rterm-app/Cargo.toml", APP.decode())
        self.write("crates/rterm-app/src/main.rs", "fn main() {}\n")
        for name, path in PACKAGES.items():
            self.write(f"{path}/Cargo.toml", f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n')
            self.write(f"{path}/src/lib.rs", "// immutable fixture source\n")
        self.write("crates/rterm-fonts/Cargo.toml", OLD_FONTS.decode())
        for path in VENDORS.values():
            self.write(f"{path}/value.txt", "frozen vendor\n")
        self.lkg = self.commit("frozen")
        self.contract = {
            "schema_version": 1,
            "last_known_good_rterm_ref": self.lkg,
            "packages": [{"name": name, "path": path} for name, path in PACKAGES.items()],
            "vendor_trees": [{"name": name, "path": path, "tree": git(self.repo, "rev-parse", f"{self.lkg}:{path}")} for name, path in VENDORS.items()],
        }
        self.contract_path = self.repo / "scripts/ci/rterm-release-contract.json"
        self.write("scripts/ci/rterm-release-contract.json", json.dumps(self.contract))
        self.write("crates/rterm-fonts/Cargo.toml", MODERN_FONTS.decode())
        self.candidate = self.commit("consumer and modern fonts")
        self.output = self.root / "prepared"

    def write(self, relative, text):
        path = self.repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="\n")

    def commit(self, message):
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", message)
        return git(self.repo, "rev-parse", "HEAD")

    def run_prepare(self, source=None, consumer=None):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--repo", str(self.repo),
             "--source-ref", source or self.candidate, "--consumer-ref", consumer or self.candidate,
             "--contract", str(self.contract_path), "--output-dir", str(self.output)],
            capture_output=True, text=True, timeout=60,
            env={**os.environ, "CARGO_NET_OFFLINE": "true"},
        )

    def receipt(self):
        return json.loads((self.output / "preparation.json").read_text())

    def load_module(self):
        sys.path.insert(0, str(SCRIPT.parent))
        self.addCleanup(sys.path.pop, 0)
        spec = importlib.util.spec_from_file_location("prepare_consumer_test", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def directory_link(self, link, target):
        if os.name == "nt":
            subprocess.run(
                ["powershell.exe", "-NoProfile", "-NonInteractive", "-Command",
                 "New-Item -ItemType Junction -Path $env:RTERM_TEST_LINK -Target $env:RTERM_TEST_TARGET | Out-Null"],
                env={**os.environ, "RTERM_TEST_LINK": str(link), "RTERM_TEST_TARGET": str(target)},
                capture_output=True, check=True,
            )
        else:
            link.symlink_to(target, target_is_directory=True)

    def test_modern_preparation_preserves_app_and_records_real_lock_and_trees(self):
        result = self.run_prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = self.receipt()
        self.assertTrue(receipt["ok"])
        self.assertFalse(receipt["compatibility_verified"])
        self.assertEqual(receipt["profile"], "modern")
        self.assertEqual(receipt["source_commit"], self.candidate)
        self.assertEqual(receipt["consumer_commit"], self.candidate)
        self.assertEqual(len(receipt["source_trees"]), 9)
        self.assertEqual(receipt["manifest_before_sha256"], receipt["manifest_after_sha256"])
        self.assertEqual(receipt["lockfile_sha256"], hashlib.sha256((self.output / "consumer/Cargo.lock").read_bytes()).hexdigest())
        self.assertEqual((self.output / "consumer/crates/rterm-app/Cargo.toml").read_bytes(), APP)
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")

    def test_frozen_preparation_uses_same_product_and_unmodified_old_packages(self):
        result = self.run_prepare(source=self.lkg)
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = self.receipt()
        self.assertEqual(receipt["profile"], "legacy-0.1")
        self.assertEqual(receipt["source_commit"], self.lkg)
        self.assertNotEqual(receipt["manifest_before_sha256"], receipt["manifest_after_sha256"])
        self.assertEqual((self.output / "consumer/crates/rterm-fonts/Cargo.toml").read_bytes(), OLD_FONTS)
        self.assertEqual((self.output / "consumer/crates/rterm-app/src/main.rs").read_text(), "fn main() {}\n")
        self.assertEqual((self.repo / "crates/rterm-app/Cargo.toml").read_bytes(), APP)

    def test_mutable_or_unavailable_refs_fail_before_creating_output(self):
        for source in ("HEAD", "f" * 40):
            with self.subTest(source=source):
                result = self.run_prepare(source=source)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(self.output.exists())

    def test_existing_output_is_never_overwritten(self):
        self.output.mkdir()
        sentinel = self.output / "preparation.json"
        sentinel.write_text("keep")
        result = self.run_prepare()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(sentinel.read_text(), "keep")

    def test_dirty_contract_cannot_change_the_frozen_identity(self):
        self.contract["last_known_good_rterm_ref"] = self.candidate
        self.contract_path.write_text(json.dumps(self.contract))
        result = self.run_prepare()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("contract", result.stderr)
        self.assertFalse(self.output.exists())

    def test_contract_digest_is_captured_with_the_validated_input(self):
        module = self.load_module()
        expected = hashlib.sha256(self.contract_path.read_bytes()).hexdigest()
        validated = module.validated_contract(self.repo, self.candidate, self.contract_path)
        self.assertEqual(len(validated), 3, "validation must return its immutable input digest")
        self.contract_path.write_text("subsequent unrelated worktree edit")
        self.assertEqual(validated[2], expected)

    def test_source_worktree_edits_and_untracked_files_are_not_exported(self):
        self.write("crates/rterm-fonts/src/lib.rs", "dirty\n")
        self.write("crates/rterm-app/src/untracked.rs", "injected\n")
        self.write("ignored.txt", "ignored injection\n")
        result = self.run_prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.output / "consumer/crates/rterm-fonts/src/lib.rs").read_text(), "// immutable fixture source\n")
        self.assertFalse((self.output / "consumer/crates/rterm-app/src/untracked.rs").exists())
        self.assertFalse((self.output / "consumer/ignored.txt").exists())
        self.assertEqual((self.repo / "crates/rterm-fonts/src/lib.rs").read_text(), "dirty\n")

    def test_unknown_source_capabilities_fail_before_writes(self):
        self.write("crates/rterm-fonts/Cargo.toml", OLD_FONTS.decode())
        unknown = self.commit("unrecognized old-like source")
        result = self.run_prepare(source=unknown)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("capabilities", result.stderr)
        self.assertFalse(self.output.exists())

    def test_wrong_vendor_tree_fails_before_writes(self):
        self.write("vendor/glyphon-0.12.0/value.txt", "drift\n")
        source = self.commit("vendor drift")
        result = self.run_prepare(source=source)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("vendor", result.stderr)
        self.assertFalse(self.output.exists())

    def test_path_escape_and_product_overlay_are_rejected(self):
        for path in ("../escape", "crates/rterm-app", "C:/escape"):
            with self.subTest(path=path):
                self.contract["packages"][0]["path"] = path
                self.contract_path.write_text(json.dumps(self.contract))
                consumer = self.commit("bad boundary")
                result = self.run_prepare(consumer=consumer)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(self.output.exists())

    def test_lock_generation_failure_is_retained_and_not_certified(self):
        self.write("Cargo.toml", '[workspace]\nresolver = "invalid"\n')
        consumer = self.commit("invalid Cargo resolver")
        result = self.run_prepare(consumer=consumer)
        self.assertNotEqual(result.returncode, 0)
        receipt = self.receipt()
        self.assertFalse(receipt["ok"])
        self.assertFalse(receipt["compatibility_verified"])
        self.assertNotEqual(receipt["lock_command"]["returncode"], 0)
        self.assertTrue((self.output / "consumer").is_dir())

    def test_git_symlink_source_is_rejected_before_checkout(self):
        blob = subprocess.run(["git", "-C", str(self.repo), "hash-object", "-w", "--stdin"], input=b"../../outside", capture_output=True, check=True).stdout.decode().strip()
        git(self.repo, "update-index", "--add", "--cacheinfo", f"120000,{blob},crates/rterm-fonts/escape")
        git(self.repo, "commit", "-m", "symlink")
        source = git(self.repo, "rev-parse", "HEAD")
        result = self.run_prepare(source=source)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("non-regular", result.stderr)
        self.assertFalse(self.output.exists())

    def test_prepared_checkout_verifier_rejects_dirty_and_ignored_injection(self):
        result = self.run_prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        module = self.load_module()
        checkout = self.output / "consumer"
        expected = module.expected_files(self.repo, self.candidate, self.candidate, list(PACKAGES.values()) + list(VENDORS.values()))
        overrides = {"Cargo.lock": (checkout / "Cargo.lock").read_bytes()}
        module.verify_checkout(checkout, expected, overrides)
        for path in ("crates/rterm-app/src/main.rs", "crates/rterm-fonts/src/lib.rs", "ignored.txt"):
            target = checkout / path
            previous = target.read_bytes() if target.exists() else None
            target.write_text("tampered")
            with self.subTest(path=path), self.assertRaises(ValueError):
                module.verify_checkout(checkout, expected, overrides)
            if previous is None:
                target.unlink()
            else:
                target.write_bytes(previous)

    def test_output_under_junction_or_symlink_is_rejected_without_external_writes(self):
        outside = self.root / "outside"
        outside.mkdir()
        link = self.root / "linked"
        self.directory_link(link, outside)
        self.output = link / "new-output"
        result = self.run_prepare()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("reparse", result.stderr)
        self.assertFalse((outside / "new-output").exists())

    def test_checkout_verifier_rejects_directory_links_and_unapproved_overrides(self):
        module = self.load_module()
        checkout = self.root / "checkout"
        checkout.mkdir()
        outside = self.root / "outside"
        outside.mkdir()
        self.directory_link(checkout / "injected", outside)
        with self.assertRaisesRegex(ValueError, "reparse"):
            module.verify_checkout(checkout, {}, {})
        with self.assertRaisesRegex(ValueError, "allowlist"):
            module.verify_checkout(checkout, {}, {"crates/rterm-app/src/main.rs": b"changed"})

    def test_output_cannot_be_nested_in_the_source_worktree(self):
        self.output = self.repo / "disposable"
        result = self.run_prepare()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
