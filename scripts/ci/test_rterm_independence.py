"""Regression coverage for hidden optional and outside-checkout dependencies."""

import importlib.util
from pathlib import Path
import tempfile
import unittest


spec = importlib.util.spec_from_file_location(
    "boundary", Path(__file__).with_name("check-rterm-feature-boundary.py")
)
boundary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(boundary)


class IndependenceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.write("Cargo.toml", '[workspace]\nmembers = ["crates/rterm-types"]\n')
        self.write("crates/rterm-types/Cargo.toml", '[package]\nname = "rterm-types"\n')

    def write(self, path, text):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text, encoding="utf-8")

    def test_self_contained_checkout_passes(self):
        self.assertEqual(boundary.check_sources(self.root), [])

    def test_optional_target_dependency_cannot_hide_rssh(self):
        self.write("crates/rterm-app/Cargo.toml", '''[package]
name = "rterm-app"
[target.'cfg(windows)'.dependencies]
transport = { git = "https://github.com/a1112/R-SSH.git", optional = true }
''')
        self.assertTrue(any("Git dependency" in item for item in boundary.check_sources(self.root)))

    def test_sibling_dependency_is_rejected(self):
        self.write("crates/rterm-app/Cargo.toml", '''[package]
name = "rterm-app"
[dependencies]
transport = { path = "../../../R-SSH/crates/transport" }
''')
        self.assertTrue(any("external path" in item for item in boundary.check_sources(self.root)))

    def test_lockfile_cannot_retain_old_backend(self):
        self.write("Cargo.lock", '[[package]]\nname = "rssh-ssh"\nversion = "0.1.0"\n')
        self.assertTrue(any("R-SSH package" in item for item in boundary.check_sources(self.root)))

    def test_neutral_types_cannot_depend_on_transport(self):
        self.write("crates/rterm-types/Cargo.toml", '''[package]
name = "rterm-types"
[dependencies]
transport = "1"
''')
        self.assertTrue(any("dependency-free" in item for item in boundary.check_sources(self.root)))


if __name__ == "__main__":
    unittest.main()
