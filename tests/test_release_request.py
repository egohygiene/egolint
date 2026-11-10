# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Exercise manual release authorization against real disposable Git sources."""

from __future__ import annotations

from copy import deepcopy
import json
import os
from pathlib import Path
import shutil
import subprocess  # nosec B404
import sys
import tempfile
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/validate_release_request.py"


class ReleaseRequestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        executable = shutil.which("git")
        if executable is None:
            self.fail("Git is required for the disposable release-request fixtures")
        self.git = executable
        self.run_git("init", "-q", "-b", "main")
        (self.root / "Cargo.toml").write_text('[package]\nversion = "0.1.0-alpha.1"\n')
        self.run_git("add", "Cargo.toml")
        self.run_git(
            "-c",
            "user.name=Synthetic reviewer",
            "-c",
            "user.email=review@example.invalid",
            "commit",
            "-qm",
            "fixture",
        )
        self.revision = self.run_git("rev-parse", "HEAD").stdout.strip()
        self.run_git("update-ref", "refs/remotes/origin/main", self.revision)
        self.environment = {
            **os.environ,
            "GITHUB_REPOSITORY": "egohygiene/egolint",
            "GITHUB_EVENT_NAME": "workflow_dispatch",
            "GITHUB_REF": "refs/heads/main",
            "GITHUB_REF_TYPE": "branch",
            "GITHUB_SHA": self.revision,
            "GITHUB_RUN_ID": "101",
            "GITHUB_OUTPUT": str(self.root / "job-output"),
            "REQUESTED_OPERATION": "rehearse",
            "REQUESTED_REVISION": self.revision,
            "REQUESTED_VERSION": "0.1.0-alpha.1",
            "REQUESTED_REHEARSAL_RUN": "",
        }
        self.rehearsal: dict[str, object] = {
            "id": 100,
            "repository": {"full_name": "egohygiene/egolint"},
            "event": "workflow_dispatch",
            "path": ".github/workflows/release.yml",
            "head_branch": "main",
            "head_sha": self.revision,
            "status": "completed",
            "conclusion": "success",
            "display_title": f"rehearse 0.1.0-alpha.1 @ {self.revision}",
        }

    def run_git(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        # Git is resolved during setup, and each test supplies a fixed argument vector.
        return subprocess.run(  # nosec B603  # noqa: S603
            [self.git, *arguments], cwd=self.root, capture_output=True, text=True, check=True
        )

    def invoke(
        self,
        environment: dict[str, str] | None = None,
        rehearsal: dict[str, object] | None = None,
        *,
        raw: str | None = None,
    ) -> subprocess.CompletedProcess[str]:
        command = [sys.executable, str(SCRIPT), "--workspace", str(self.root)]
        if rehearsal is not None or raw is not None:
            metadata = self.root / "prior-run.json"
            metadata.write_text(json.dumps(rehearsal) if raw is None else raw)
            command.extend(["--rehearsal-run", str(metadata)])
        # The current interpreter executes the fixed repository-owned script, without a shell.
        return subprocess.run(  # nosec B603  # noqa: S603
            command,
            env=self.environment | (environment or {}),
            capture_output=True,
            text=True,
            check=False,
            timeout=10,
        )

    def publish_environment(self) -> dict[str, str]:
        self.run_git("tag", "v0.1.0-alpha.1")
        return {"REQUESTED_OPERATION": "publish", "REQUESTED_REHEARSAL_RUN": "100"}

    def assert_rejected(self, process: subprocess.CompletedProcess[str]) -> None:
        self.assertEqual(process.returncode, 1, process.stdout + process.stderr)
        self.assertIn("release-request:", process.stderr)
        self.assertNotIn("Traceback", process.stderr)
        self.assertEqual(process.stdout, "")

    def test_rehearsal_emits_nonpublishing_outputs_without_creating_tag(self) -> None:
        before = self.run_git("show-ref").stdout
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("publish=false", result.stdout)
        self.assertEqual(
            (self.root / "job-output").read_text(),
            "publish=false\nversion=0.1.0-alpha.1\ntag=v0.1.0-alpha.1\n",
        )
        self.assertEqual(self.run_git("show-ref").stdout, before)

    def test_rejects_automatic_events_wrong_refs_and_ambiguous_inputs(self) -> None:
        cases = [
            {"GITHUB_EVENT_NAME": "push"},
            {"GITHUB_REPOSITORY": "another/repository"},
            {"GITHUB_REF": "refs/tags/v0.1.0-alpha.1", "GITHUB_REF_TYPE": "tag"},
            {"GITHUB_REF": "refs/heads/feature"},
            {"GITHUB_SHA": "a" * 40},
            {"REQUESTED_REVISION": "main"},
            {"REQUESTED_REVISION": "a" * 40},
            {"REQUESTED_VERSION": "0.2.0"},
            {"REQUESTED_OPERATION": "unknown"},
            {"REQUESTED_REHEARSAL_RUN": "100"},
        ]
        for case in cases:
            with self.subTest(case=case):
                self.assert_rejected(self.invoke(case))
        self.assertFalse((self.root / "job-output").exists())

    def test_rejects_malformed_version_authority_without_a_traceback(self) -> None:
        for source in ('package = "invalid"\n', "[package]\nversion = 1\n", "[bad"):
            with self.subTest(source=source):
                (self.root / "Cargo.toml").write_text(source)
                self.assert_rejected(self.invoke())

    def test_publish_requires_exact_successful_rehearsal_and_matching_tag(self) -> None:
        environment = self.publish_environment()
        before = self.run_git("show-ref").stdout
        result = self.invoke(environment, self.rehearsal)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("publish=true", result.stdout)
        self.assertEqual(self.run_git("show-ref").stdout, before)
        self.assert_rejected(self.invoke(environment))
        self.assert_rejected(
            self.invoke(environment | {"REQUESTED_REHEARSAL_RUN": "101"}, self.rehearsal)
        )

    def test_publish_rejects_unqualified_or_unrelated_hosted_evidence(self) -> None:
        environment = self.publish_environment()
        cases: list[tuple[str, object]] = [
            ("id", "100"),
            ("id", True),
            ("id", 99),
            ("repository", {"full_name": "another/repository"}),
            ("event", "push"),
            ("path", ".github/workflows/ci.yml"),
            ("head_branch", "feature"),
            ("head_sha", "a" * 40),
            ("status", "in_progress"),
            ("conclusion", "failure"),
            ("display_title", f"publish 0.1.0-alpha.1 @ {self.revision}"),
            ("display_title", f"rehearse 0.2.0 @ {self.revision}"),
        ]
        for key, value in cases:
            with self.subTest(field=key, value=value):
                run = deepcopy(self.rehearsal)
                run[key] = value
                self.assert_rejected(self.invoke(environment, run))
        for raw in ("null", "[]", "{bad"):
            with self.subTest(raw=raw):
                self.assert_rejected(self.invoke(environment, raw=raw))

    def test_publish_rejects_missing_or_moved_version_tag(self) -> None:
        environment = {"REQUESTED_OPERATION": "publish", "REQUESTED_REHEARSAL_RUN": "100"}
        self.assert_rejected(self.invoke(environment, self.rehearsal))
        self.run_git("tag", "v0.1.0-alpha.1")
        self.run_git(
            "-c",
            "user.name=Synthetic reviewer",
            "-c",
            "user.email=review@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "later",
        )
        later = self.run_git("rev-parse", "HEAD").stdout.strip()
        self.run_git("update-ref", "refs/remotes/origin/main", later)
        self.environment.update(GITHUB_SHA=later, REQUESTED_REVISION=later)
        self.rehearsal.update(head_sha=later, display_title=f"rehearse 0.1.0-alpha.1 @ {later}")
        self.assert_rejected(self.invoke(environment, self.rehearsal))

    def test_workflow_keeps_manual_publication_behind_candidate_and_release_environment(
        self,
    ) -> None:
        workflow = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())
        triggers = workflow.get("on", workflow.get(True))
        self.assertEqual(set(triggers), {"workflow_dispatch"})
        self.assertEqual(
            triggers["workflow_dispatch"]["inputs"]["operation"]["default"], "rehearse"
        )
        jobs = workflow["jobs"]
        self.assertEqual(jobs["candidate"]["permissions"], {"contents": "read", "actions": "read"})
        self.assertIn("needs.candidate.outputs.publish == 'true'", jobs["sign-candidate"]["if"])
        self.assertEqual(jobs["sign-candidate"]["needs"], "candidate")
        for name in ("sign-candidate", "build-images", "authorize-images", "announce"):
            self.assertEqual(jobs[name]["environment"], "release")


if __name__ == "__main__":
    unittest.main()
