# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Contract tests for immutable repository-release source pins."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
from pathlib import Path, PurePosixPath
import sys
import tempfile
import unittest

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPOSITORY_ROOT / "scripts/validate_repository_release_sources.py"
SPECIFICATION = importlib.util.spec_from_file_location(
    "validate_repository_release_sources", MODULE_PATH
)
if SPECIFICATION is None or SPECIFICATION.loader is None:
    load_error = f"Unable to load {MODULE_PATH}"
    raise RuntimeError(load_error)
RELEASE_SOURCES = importlib.util.module_from_spec(SPECIFICATION)
sys.modules[SPECIFICATION.name] = RELEASE_SOURCES
SPECIFICATION.loader.exec_module(RELEASE_SOURCES)


class RepositoryReleaseSourceTests(unittest.TestCase):
    """Prove immutable bytes, closed records, and source compatibility."""

    def setUp(self) -> None:
        self.lock = RELEASE_SOURCES.load_json_object(RELEASE_SOURCES.DEFAULT_LOCK)

    def copy_sources(self, root: Path) -> None:
        """Copy only the two public fixtures into an isolated repository root."""

        for source in self.lock["sources"]:
            relative = PurePosixPath(source["vendored_path"])
            destination = root.joinpath(*relative.parts)
            destination.parent.mkdir(parents=True, exist_ok=True)
            source_path = REPOSITORY_ROOT.joinpath(*relative.parts)
            destination.write_bytes(source_path.read_bytes())

    def update_source_digest(self, source_id: str, payload: bytes) -> None:
        """Update one test lock after an intentional semantic mutation."""

        source = next(item for item in self.lock["sources"] if item["id"] == source_id)
        source["sha256"] = hashlib.sha256(payload).hexdigest()
        source["git_blob_sha1"] = RELEASE_SOURCES.git_blob_sha1(payload)

    def test_canonical_sources_are_valid(self) -> None:
        self.assertEqual(RELEASE_SOURCES.validate_lock(self.lock), [])

    def test_tampered_vendored_bytes_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            self.copy_sources(root)
            source = self.lock["sources"][0]
            path = root / source["vendored_path"]
            path.write_bytes(path.read_bytes() + b"\n")

            findings = RELEASE_SOURCES.validate_lock(self.lock, root)

        self.assertIn(
            "aether-repository-release-schema.sha256 does not match vendored bytes",
            findings,
        )
        self.assertIn(
            "aether-repository-release-schema.git_blob_sha1 does not match vendored bytes",
            findings,
        )

    def test_hygiene_must_pin_the_selected_aether_revision(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            self.copy_sources(root)
            path = root / self.lock["sources"][1]["vendored_path"]
            profile = json.loads(path.read_text(encoding="utf-8"))
            profile["aether_contract"]["revision"] = "0" * 40
            payload = json.dumps(profile, indent=2).encode() + b"\n"
            path.write_bytes(payload)
            self.update_source_digest("hygiene-repository-release-profile", payload)

            findings = RELEASE_SOURCES.validate_lock(self.lock, root)

        self.assertIn(
            "Hygiene Aether revision must match the selected schema revision",
            findings,
        )

    def test_unknown_or_duplicate_sources_are_rejected(self) -> None:
        self.lock["sources"].append(copy.deepcopy(self.lock["sources"][0]))

        findings = RELEASE_SOURCES.validate_lock(self.lock)

        self.assertIn("sources must contain each supported source exactly once", findings)

    def test_vendored_paths_cannot_escape_the_repository(self) -> None:
        self.lock["sources"][0]["vendored_path"] = "../schema.json"

        findings = RELEASE_SOURCES.validate_lock(self.lock)

        self.assertTrue(any("vendored_path must be normalized" in item for item in findings))


if __name__ == "__main__":
    unittest.main()
