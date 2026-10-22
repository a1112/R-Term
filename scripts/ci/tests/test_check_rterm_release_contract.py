import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
CHECKER = REPOSITORY_ROOT / "scripts" / "ci" / "check-rterm-release-contract.py"
CONTRACT = REPOSITORY_ROOT / "scripts" / "ci" / "rterm-release-contract.json"
HISTORY_MAP = REPOSITORY_ROOT / "docs" / "release" / "rterm-history-paths.txt"
CONSUMER = REPOSITORY_ROOT / "contracts" / "rterm-consumer"
LEGACY_CI_WORKFLOW = REPOSITORY_ROOT / "docs" / "trial" / "legacy-workflows" / "ci.yml"

EXPECTED_PACKAGES = {
    "rterm-types": "crates/rterm-types",
    "rterm-terminal": "crates/rterm-terminal",
    "rterm-runtime": "crates/rterm-runtime",
    "rterm-fonts": "crates/rterm-fonts",
    "rterm-render-core": "crates/rterm-render-core",
    "rterm-render-cpu": "crates/rterm-render-cpu",
    "rterm-render-wgpu": "crates/rterm-render-wgpu",
}


class RTermReleaseContractTests(unittest.TestCase):
    def run_checker(self, contract: Path = CONTRACT) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(CHECKER),
                "--repo-root",
                str(REPOSITORY_ROOT),
                "--contract",
                str(contract),
            ],
            cwd=REPOSITORY_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_repository_contract_is_complete_and_valid(self):
        result = self.run_checker()

        self.assertEqual(result.returncode, 0, result.stderr or result.stdout)
        report = json.loads(result.stdout)
        self.assertTrue(report["ok"])
        self.assertEqual(report["checked_ref"], "HEAD")
        self.assertEqual(set(report["packages"]), set(EXPECTED_PACKAGES))
        self.assertEqual(set(report["vendor_trees"]), {"glyphon", "gpu-allocator"})
        self.assertEqual(report["violations"], [])

    def test_contract_declares_exact_public_packages_and_vendor_strategy(self):
        contract = json.loads(CONTRACT.read_text(encoding="utf-8"))

        self.assertEqual(contract["schema_version"], 1)
        self.assertEqual(contract["api_compatibility_line"], "0.1")
        self.assertEqual(
            contract["last_known_good_rterm_ref"],
            "0e8ebd5de22758275cbb6a849c19c032268d7fac",
        )
        self.assertEqual(
            contract["vendor_patch_strategy"], "consumer-root-path-patch"
        )
        self.assertEqual(
            {entry["name"]: entry["path"] for entry in contract["packages"]},
            EXPECTED_PACKAGES,
        )
        for package in contract["packages"]:
            self.assertEqual(package["version"], "0.1.0")
            self.assertFalse(
                any(name.startswith("rssh-") for name in package["dependencies"]),
                package["name"],
            )

    def test_checker_rejects_mutable_refs_version_mismatch_and_reverse_dependencies(self):
        contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
        contract["last_known_good_rterm_ref"] = "main"
        contract["packages"][0]["version"] = "0.2.0"
        contract["packages"][0]["dependencies"].append("rssh-core")

        with tempfile.TemporaryDirectory() as directory:
            mutated = Path(directory) / "contract.json"
            mutated.write_text(json.dumps(contract), encoding="utf-8")
            result = self.run_checker(mutated)

        self.assertNotEqual(result.returncode, 0)
        report = json.loads(result.stdout)
        self.assertFalse(report["ok"])
        joined = "\n".join(report["violations"])
        self.assertIn("immutable 40-character lowercase commit", joined)
        self.assertIn("version", joined)
        self.assertIn("reverse dependency rssh-core", joined)

    def test_checker_rejects_missing_paths_and_vendor_tree_drift(self):
        contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
        contract["packages"][0]["path"] = "crates/does-not-exist"
        contract["vendor_trees"][0]["tree"] = "0" * 40

        with tempfile.TemporaryDirectory() as directory:
            mutated = Path(directory) / "contract.json"
            mutated.write_text(json.dumps(contract), encoding="utf-8")
            result = self.run_checker(mutated)

        self.assertNotEqual(result.returncode, 0)
        report = json.loads(result.stdout)
        joined = "\n".join(report["violations"])
        self.assertIn("missing package path", joined)
        self.assertIn("vendor tree drift", joined)

    def test_rehearsal_commands_cover_probe_product_and_transport_gates(self):
        contract = json.loads(CONTRACT.read_text(encoding="utf-8"))

        self.assertEqual(
            contract["standalone_probe"]["command"],
            ["cargo", "check", "--locked"],
        )
        commands = contract["consumer_commands"]
        self.assertIn(
            [
                "cargo",
                "check",
                "--locked",
                "-p",
                "rterm-app",
                "--no-default-features",
                "--features",
                "production-gui",
            ],
            commands,
        )
        for package in ("rterm-ssh", "rterm-pty", "rterm-native", "rterm-functional-tests"):
            self.assertIn(
                ["cargo", "test", "--locked", "-p", package, "--all-targets"],
                commands,
            )
        self.assertIn(
            [
                "cargo",
                "build",
                "--locked",
                "-p",
                "rterm-app",
                "--no-default-features",
                "--features",
                "production-gui,transfer-tools",
            ],
            commands,
        )

    def test_history_map_preserves_terminal_runtime_fonts_and_renderer_paths(self):
        rows = {
            tuple(line.split("|"))
            for line in HISTORY_MAP.read_text(encoding="utf-8").splitlines()
            if line and not line.startswith("#")
        }

        self.assertIn(
            (
                "terminal",
                "crates/rssh-terminal",
                "crates/rterm-terminal",
            ),
            rows,
        )
        self.assertIn(
            (
                "runtime",
                "crates/rssh-app/src/terminal_runtime.rs",
                "crates/rterm-runtime/src/terminal.rs",
            ),
            rows,
        )
        self.assertIn(("fonts", "crates/rssh-fonts", "crates/rterm-fonts"), rows)
        self.assertIn(
            (
                "render-cpu",
                "crates/rssh-renderer/src/text.rs",
                "crates/rterm-render-cpu/src/text.rs",
            ),
            rows,
        )
        self.assertIn(
            (
                "render-wgpu",
                "crates/rssh-renderer/src/gpu",
                "crates/rterm-render-wgpu/src/gpu",
            ),
            rows,
        )

    def test_standalone_consumer_declares_only_rterm_packages(self):
        manifest = (CONSUMER / "Cargo.toml").read_text(encoding="utf-8")
        workspace = (REPOSITORY_ROOT / "Cargo.toml").read_text(encoding="utf-8")

        self.assertIn('exclude = ["contracts/rterm-consumer"]', workspace)
        self.assertIn("[workspace]", manifest)
        dependency_block = manifest.split("[dependencies]", maxsplit=1)[1].split(
            "[patch.crates-io]", maxsplit=1
        )[0]
        dependency_names = set(
            re.findall(r"^([A-Za-z0-9_-]+)\s*=", dependency_block, re.MULTILINE)
        )
        self.assertEqual(dependency_names, set(EXPECTED_PACKAGES))
        self.assertNotIn('package = "rssh-', dependency_block)
        for name, path in EXPECTED_PACKAGES.items():
            self.assertIn(
                f'{name} = {{ path = "../../{path}", version = "0.1.0" }}',
                manifest,
            )
        self.assertTrue((CONSUMER / "Cargo.lock").is_file())

        source = (CONSUMER / "src" / "main.rs").read_text(encoding="utf-8")
        for public_surface in (
            "TerminalSize",
            "DamageRegion",
            "Terminal::new",
            "TerminalRuntime::new",
            "FontConfig::new",
            "TerminalRenderSnapshot::from_terminal",
            "PixelRenderer::new",
            "GpuContextOptions::default",
        ):
            self.assertIn(public_surface, source)

    def test_hosted_ci_enforces_candidate_and_rollback_without_absolute_gates(self):
        workflow = LEGACY_CI_WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("  rterm-consumer-contract:")
        remainder = workflow[start + 2 :]
        next_job = re.search(r"^  [a-z0-9-]+:\s*$", remainder, re.MULTILINE)
        job = workflow[start:] if next_job is None else workflow[start : start + 2 + next_job.start()]

        for required in (
            "runs-on: ubuntu-24.04",
            "timeout-minutes:",
            "persist-credentials: false",
            "fetch-depth: 0",
            "dtolnay/rust-toolchain@451ce45ce31d200b52705aadd15ce75018b006de",
            "python -m unittest scripts.ci.tests.test_check_rterm_release_contract scripts.ci.tests.test_rehearse_rterm_consumer -v",
            "python scripts/ci/check-rterm-release-contract.py --contract scripts/ci/rterm-release-contract.json",
            "cargo check --locked --manifest-path contracts/rterm-consumer/Cargo.toml",
            "python scripts/ci/rehearse-rterm-consumer.py",
            "--candidate-ref ${{ github.sha }}",
            "--consumer-ref ${{ github.sha }}",
            "evidence/rterm-release/candidate.json",
            "evidence/rterm-release/rollback.json",
            "if-no-files-found: error",
        ):
            self.assertIn(required, job)
        self.assertIn("permissions:\n  contents: read", workflow)
        upload = job[job.index("      - name: Upload R-Term release evidence"):]
        self.assertIn("if: always()", upload)
        for forbidden in (
            "run-ssh-gui-startup",
            "first_frame_private_bytes",
            "Warmups",
            "Samples",
            "process_to_first_present_ms",
        ):
            self.assertNotIn(forbidden, job)


if __name__ == "__main__":
    unittest.main()
