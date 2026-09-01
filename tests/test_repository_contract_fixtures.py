# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import tomllib
from typing import Any, ClassVar
import unittest

ROOT = Path(__file__).resolve().parents[1]
CONTRACTS = ROOT / "tests" / "fixtures" / "contracts"


class CanonicalRepositoryContractFixtureTests(unittest.TestCase):
    manifest: ClassVar[dict[str, Any]]

    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = json.loads((CONTRACTS / "install-manifest.json").read_text(encoding="utf-8"))

    def test_install_manifest_pins_two_canonical_contracts(self) -> None:
        self.assertEqual(1, self.manifest["schema_version"])
        self.assertEqual(2, len(self.manifest["artifacts"]))
        self.assertEqual(
            {"egohygiene/empathy", "egohygiene/hygiene"},
            {item["source_repository"] for item in self.manifest["artifacts"]},
        )

    def test_fixture_digests_and_embedded_provenance_match(self) -> None:
        for artifact in self.manifest["artifacts"]:
            path = ROOT / artifact["fixture"]
            contents = path.read_bytes()
            self.assertEqual(artifact["sha256"], hashlib.sha256(contents).hexdigest())
            contract = tomllib.loads(contents.decode("utf-8"))
            self.assertFalse(contract["provisional"])
            self.assertEqual(artifact["source_repository"], contract["source"]["repository"])
            self.assertEqual(artifact["source_revision"], contract["source"]["revision"])
            self.assertEqual(artifact["source_path"], contract["source"]["path"])
            self.assertEqual("git-commit", contract["source"]["revision-kind"])
            self.assertRegex(contract["source"]["revision"], re.compile(r"^[0-9a-f]{40}$"))

    def test_provisional_fixture_names_are_retired(self) -> None:
        self.assertFalse((CONTRACTS / "empathy-universal-provisional.toml").exists())
        self.assertFalse((CONTRACTS / "hygiene-context-provisional.toml").exists())


if __name__ == "__main__":
    unittest.main()
