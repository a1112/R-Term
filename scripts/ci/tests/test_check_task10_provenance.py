import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
CHECKER = REPOSITORY_ROOT / "scripts" / "ci" / "check-task10-provenance.py"
CODEC = REPOSITORY_ROOT / "scripts" / "ci" / "task10_rust_test_body.py"
PROVENANCE = REPOSITORY_ROOT / "tests" / "fixtures" / "task10_trace_provenance.json"
RECORDS = (
    REPOSITORY_ROOT
    / "crates"
    / "rterm-runtime"
    / "tests"
    / "fixtures"
    / "task10_legacy_fixture_records.txt"
)


def run_checker(*extra: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(CHECKER), "--root", str(REPOSITORY_ROOT), *extra],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )


def load_codec():
    spec = importlib.util.spec_from_file_location("task10_rust_test_body", CODEC)
    if spec is None or spec.loader is None:
        raise AssertionError("load Task10 body codec")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class Task10ProvenanceCheckerTests(unittest.TestCase):
    def test_quality_workflow_fetches_c69_and_runs_provenance_gate(self):
        workflow = (REPOSITORY_ROOT / ".github" / "workflows" / "windows.yml").read_text(
            encoding="utf-8"
        )
        quality = workflow.split("  msrv:", maxsplit=1)[0]

        self.assertIn("fetch-depth: 0", quality)
        self.assertIn("python scripts/ci/check-task10-provenance.py", quality)

    def test_checked_in_evidence_recomputes_from_c69_git_objects(self):
        result = run_checker()

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertTrue(report["ok"])
        self.assertEqual(report["baseline_tests"], 356)
        self.assertEqual(report["trace_entries"], 356)

    def test_syn_compatible_body_codec_ignores_braces_in_rust_literals_and_comments(self):
        codec = load_codec()
        source = r'''mod tests {
    #[test]
    fn braces_are_lexical() {
        let normal = "}";
        let raw = r##"{ not a block }"##;
        let byte = b'}';
        // }
        /* { /* nested } */ } */
        assert_eq!(normal, "}");
    }
}
'''

        bodies = codec.test_body_sha256s(source.encode("utf-8"))

        start = source.index("{", source.index("fn braces_are_lexical"))
        end = source.rindex("    }\n}") + len("    }")
        expected = source[start:end]
        self.assertEqual(
            bodies,
            {"braces_are_lexical": hashlib.sha256(expected.encode("utf-8")).hexdigest()},
        )

    def test_synchronized_local_record_hashes_cannot_replace_c69_body(self):
        provenance = json.loads(PROVENANCE.read_text(encoding="utf-8"))
        lines = RECORDS.read_text(encoding="utf-8").splitlines()
        row_index = next(
            index
            for index, line in enumerate(lines)
            if line.startswith("record|") and "|exact|" in line
        )
        fields = lines[row_index].split("|")
        fields[6] = "0" * 64
        fields[9] = "0" * 64
        identity = b"\0".join(
            (fields[3].encode(), fields[4].encode(), fields[6].encode())
        )
        fields[1] = hashlib.sha256(identity).hexdigest()
        lines[row_index] = "|".join(fields)
        mutated = ("\n".join(lines) + "\n").encode()

        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            records = directory / "records.txt"
            records.write_bytes(mutated)
            provenance["artifacts"]["records"]["sha256"] = hashlib.sha256(mutated).hexdigest()
            provenance["artifacts"]["records"]["length"] = len(mutated)
            sidecar = directory / "provenance.json"
            sidecar.write_text(json.dumps(provenance), encoding="utf-8")
            result = run_checker(
                "--records",
                str(records),
                "--provenance",
                str(sidecar),
            )

        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("TASK10_PROVENANCE[BODY]", result.stderr)

    def test_provenance_rejects_a_synchronized_but_wrong_baseline_tree(self):
        provenance = json.loads(PROVENANCE.read_text(encoding="utf-8"))
        provenance["baseline"]["tree"] = "0" * 40

        with tempfile.TemporaryDirectory() as directory:
            sidecar = Path(directory) / "provenance.json"
            sidecar.write_text(json.dumps(provenance), encoding="utf-8")
            result = run_checker("--provenance", str(sidecar))

        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("TASK10_PROVENANCE[TREE]", result.stderr)


if __name__ == "__main__":
    unittest.main()
