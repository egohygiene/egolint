# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/repository_release_plan.py"
CORPUS = ROOT / "tests/fixtures/repository-release"


class RepositoryReleasePlanTests(unittest.TestCase):
    def consumer(self, directory):
        root = Path(directory)
        for source, destination in (
            ("rust/declaration.json", ".egohygiene/release.json"),
            ("rust/Cargo.toml", "Cargo.toml"),
            ("common/CHANGELOG.md", "CHANGELOG.md"),
            ("common/Taskfile.yml", "Taskfile.yml"),
            ("common/release-workflow.yml", ".github/workflows/release.yml"),
        ):
            target = root / destination
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(CORPUS / source, target)
        return root

    def run_plan(self, root, *arguments):
        # The executable is the interpreter and a repository-owned fixed helper.
        return subprocess.run(  # noqa: S603
            [sys.executable, str(SCRIPT), "--workspace", str(root), *arguments],
            capture_output=True,
            text=True,
            check=False,
            env=os.environ,
        )

    def test_plan_records_source_facts_without_mutating_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.consumer(directory)
            before = {
                path.relative_to(root): path.read_bytes()
                for path in root.rglob("*")
                if path.is_file()
            }
            result = self.run_plan(root)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("egohygiene/rust-release-fixture", result.stdout)
            self.assertIn("0.1.0", result.stdout)
            self.assertIn("Pending maintainer version and changelog decision", result.stdout)
            self.assertIn(
                hashlib.sha256((root / "Cargo.toml").read_bytes()).hexdigest(), result.stdout
            )
            after = {
                path.relative_to(root): path.read_bytes()
                for path in root.rglob("*")
                if path.is_file()
            }
            self.assertEqual(before, after)
            again = self.run_plan(root)
            self.assertEqual(again.stdout, result.stdout)

    def test_preparation_creates_exclusive_review_handoff(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.consumer(directory)
            output = root / "review-plan.md"
            result = self.run_plan(root, "--output", str(output))
            self.assertEqual(result.returncode, 0, result.stderr)
            prepared = output.read_bytes()
            self.assertIn(b"Pending maintainer version and changelog decision", prepared)
            result = self.run_plan(root, "--output", str(output))
            self.assertEqual(result.returncode, 1)
            self.assertIn("release-plan:", result.stderr)
            self.assertNotIn("Traceback", result.stderr)
            self.assertEqual(output.read_bytes(), prepared)

    def test_missing_and_malformed_source_fail_with_controlled_diagnostics(self):
        for malformed in (False, True):
            with self.subTest(malformed=malformed), tempfile.TemporaryDirectory() as directory:
                root = self.consumer(directory)
                declaration = root / ".egohygiene/release.json"
                if malformed:
                    declaration.write_text(json.dumps({"repository": None}), encoding="utf-8")
                else:
                    declaration.unlink()
                result = self.run_plan(root)
                self.assertEqual(result.returncode, 1)
                self.assertIn("release-plan:", result.stderr)
                self.assertNotIn("Traceback", result.stderr)
                self.assertEqual(result.stdout, "")
