import importlib.util
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
MODULE = ROOT / "scripts/ci/rterm_consumer_profile.py"
LKG = "0e8ebd5de22758275cbb6a849c19c032268d7fac"
CANDIDATE = "a8430ed94c6c74ddd4d7df3b80cc58955030195c"
OLD_FONTS = b'[package]\nname = "rterm-fonts"\nversion = "0.1.0"\n'
MODERN_FONTS = OLD_FONTS + (
    b'\n[features]\ndefault = []\nshared-source-ownership = []\ndiagnostic-tools = []\n'
)
APP = b'''[package]
name = "rterm-app"
version = "0.1.0"

[features]
default = ["developer-full"]
developer-full = ["production-gui", "diagnostic-tools"]
production-gui = ["native-gui", "ssh", "production-fonts"]
production-fonts = ["rterm-fonts/shared-source-ownership"]
diagnostic-tools = ["rterm-fonts/diagnostic-tools"]
rterm-legacy-0-1 = []
native-gui = []
ssh = []

[dependencies]
rterm-fonts = { package = "rterm-fonts", path = "../rterm-fonts", version = "0.1.0" }

[package.metadata.fixture]
keep = "unchanged"
'''


class ConsumerProfileTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(MODULE.is_file(), "consumer profile implementation is missing")
        spec = importlib.util.spec_from_file_location("rterm_consumer_profile", MODULE)
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)

    def test_exact_frozen_identity_selects_legacy(self):
        self.assertEqual(self.module.select_profile(LKG, LKG, OLD_FONTS), "legacy-0.1")

    def test_declared_modern_capabilities_select_modern(self):
        self.assertEqual(
            self.module.select_profile(CANDIDATE, LKG, MODERN_FONTS), "modern"
        )

    def test_unknown_old_source_cannot_silently_select_legacy(self):
        with self.assertRaisesRegex(ValueError, "capabilities"):
            self.module.select_profile(CANDIDATE, LKG, OLD_FONTS)

    def test_partial_modern_declarations_fail_closed(self):
        for feature in (b"shared-source-ownership = []\n", b"diagnostic-tools = []\n"):
            with self.subTest(feature=feature), self.assertRaisesRegex(ValueError, "capabilities"):
                self.module.select_profile(CANDIDATE, LKG, MODERN_FONTS.replace(feature, b""))

    def test_frozen_identity_with_modern_manifest_is_inconsistent(self):
        with self.assertRaisesRegex(ValueError, "frozen"):
            self.module.select_profile(LKG, LKG, MODERN_FONTS)

    def test_both_refs_must_be_full_lowercase_commit_ids(self):
        for invalid in ("HEAD", "0e8ebd5", "G" * 40, "A" * 40, "a" * 41, "a" * 40 + "\n"):
            for source, lkg in ((invalid, LKG), (CANDIDATE, invalid)):
                with self.subTest(source=source, lkg=lkg), self.assertRaisesRegex(ValueError, "commit"):
                    self.module.select_profile(source, lkg, MODERN_FONTS)

    def test_fonts_manifest_must_be_the_expected_package(self):
        for manifest in (b"", b"not toml", OLD_FONTS.replace(b"rterm-fonts", b"other")):
            with self.subTest(manifest=manifest), self.assertRaises(ValueError):
                self.module.select_profile(LKG, LKG, manifest)

    def test_malformed_capability_arrays_are_rejected(self):
        for value in (b"false", b'"yes"', b"[42]", b"{}"):
            manifest = MODERN_FONTS.replace(b"shared-source-ownership = []", b"shared-source-ownership = " + value)
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.module.select_profile(CANDIDATE, LKG, manifest)

    def test_modern_manifest_is_byte_identical(self):
        for original in (APP, APP.replace(b"\n", b"\r\n")):
            self.assertEqual(self.module.prepare_app_manifest(original, "modern"), original)

    def test_repository_declares_an_inert_selector_for_verified_preparation(self):
        original = (ROOT / "crates/rterm-app/Cargo.toml").read_bytes()
        features = tomllib.loads(original.decode())["features"]
        self.assertEqual(features.get("rterm-legacy-0-1"), [])
        self.assertFalse(any("rterm-legacy-0-1" in values for values in features.values()))
        self.assertEqual(self.module.prepare_app_manifest(original, "modern"), original)
        self.module.prepare_app_manifest(original, "legacy-0.1")

    def test_legacy_changes_exactly_three_feature_arrays(self):
        before = tomllib.loads(APP.decode())
        expected = tomllib.loads(APP.decode())
        expected["features"]["production-fonts"] = []
        expected["features"]["diagnostic-tools"] = []
        expected["features"]["production-gui"].append("rterm-legacy-0-1")

        prepared = self.module.prepare_app_manifest(APP, "legacy-0.1")

        self.assertEqual(tomllib.loads(prepared.decode()), expected)
        self.assertEqual(before["features"]["production-fonts"], ["rterm-fonts/shared-source-ownership"])
        self.assertIn(b'keep = "unchanged"', prepared)

    def test_legacy_preserves_crlf(self):
        prepared = self.module.prepare_app_manifest(APP.replace(b"\n", b"\r\n"), "legacy-0.1")
        self.assertEqual(prepared.count(b"\n"), prepared.count(b"\r\n"))

    def test_unknown_profile_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "profile"):
            self.module.prepare_app_manifest(APP, "automatic")

    def test_selector_must_be_declared_and_inert(self):
        for replacement in (b"", b'rterm-legacy-0-1 = ["ssh"]\n'):
            manifest = APP.replace(b"rterm-legacy-0-1 = []\n", replacement)
            with self.subTest(replacement=replacement), self.assertRaisesRegex(ValueError, "selector"):
                self.module.prepare_app_manifest(manifest, "legacy-0.1")

    def test_no_profile_can_accept_a_pre_enabled_selector(self):
        manifest = APP.replace(b'default = ["developer-full"]', b'default = ["developer-full", "rterm-legacy-0-1"]')
        for profile in ("modern", "legacy-0.1"):
            with self.subTest(profile=profile), self.assertRaisesRegex(ValueError, "selector"):
                self.module.prepare_app_manifest(manifest, profile)

    def test_repeated_legacy_preparation_is_rejected(self):
        prepared = self.module.prepare_app_manifest(APP, "legacy-0.1")
        with self.assertRaises(ValueError):
            self.module.prepare_app_manifest(prepared, "legacy-0.1")

    def test_unexpected_forwarding_is_not_silently_erased(self):
        manifest = APP.replace(b'["rterm-fonts/diagnostic-tools"]', b'["rterm-fonts/diagnostic-tools", "ssh"]')
        with self.assertRaisesRegex(ValueError, "diagnostic-tools"):
            self.module.prepare_app_manifest(manifest, "legacy-0.1")

    def test_ambiguous_layout_is_rejected_without_guessing(self):
        manifest = APP.replace(b'production-fonts = ["rterm-fonts/shared-source-ownership"]', b'production-fonts = [\n"rterm-fonts/shared-source-ownership"\n]')
        with self.assertRaisesRegex(ValueError, "single-line"):
            self.module.prepare_app_manifest(manifest, "legacy-0.1")

    def test_non_feature_manifest_sections_cannot_be_transformed(self):
        manifest = APP + b'diagnostic-tools = ["rterm-fonts/diagnostic-tools"]\n'
        with self.assertRaises(ValueError):
            self.module.prepare_app_manifest(manifest, "legacy-0.1")

    def test_app_identity_and_production_font_edge_are_required(self):
        for manifest in (APP.replace(b'name = "rterm-app"', b'name = "other"'), APP.replace(b', "production-fonts"', b"")):
            with self.subTest(manifest=manifest), self.assertRaises(ValueError):
                self.module.prepare_app_manifest(manifest, "legacy-0.1")


if __name__ == "__main__":
    unittest.main()
