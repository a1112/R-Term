import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("package", Path(__file__).with_name("package-native-linux.py"))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PackageTests(unittest.TestCase):
    def test_embedded_fonts_include_verified_licenses(self):
        notices = package.font_inventory(Path(__file__).resolve().parents[1])
        self.assertEqual(len(notices), 7)
        for item in notices:
            self.assertEqual(item["license"], "OFL-1.1")
            self.assertEqual(len(item["sha256"]), 64)
            self.assertIn("SIL OPEN FONT LICENSE", next(iter(item["notices"].values())))

    def test_archive_rejects_wrong_commit_and_changed_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repo = root / "source"
            repo.mkdir()
            (repo / "Cargo.lock").write_bytes(b"locked")
            archive = root / "source.tar"
            with tarfile.open(archive, "w", pax_headers={"comment": "a" * 40}) as stream:
                info = tarfile.TarInfo("Cargo.lock")
                info.size = 6
                stream.addfile(info, io.BytesIO(b"locked"))
            package.verify_source(repo, archive, "a" * 40)
            with self.assertRaises(ValueError):
                package.verify_source(repo, archive, "b" * 40)
            (repo / "Cargo.lock").write_bytes(b"edited")
            with self.assertRaises(ValueError):
                package.verify_source(repo, archive, "a" * 40)

    def test_version_identity_rejects_stale_or_foreign_binary(self):
        value = {"name": "rterm", "version": "0.1.0", "target": "linux-x86_64",
                 "console": True, "pty_backend": "unix-pty", "native_ssh_backend": "disabled"}
        package.verify_version(value, "0.1.0")
        for key, bad in (("version", "0.0.1"), ("target", "windows-x86_64"),
                         ("pty_backend", "windows-conpty"), ("native_ssh_backend", "russh")):
            with self.subTest(key=key), self.assertRaises(ValueError):
                package.verify_version({**value, key: bad}, "0.1.0")


if __name__ == "__main__":
    unittest.main()
