# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

from __future__ import annotations

import os
from pathlib import Path
import stat
import subprocess  # nosec B404
import tempfile
import unittest

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
WRAPPER_PATH = REPOSITORY_ROOT / "scripts" / "megalinter.sh"


class MegaLinterWrapperTests(unittest.TestCase):
    def run_wrapper(
        self,
        *arguments: str,
        environment: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        process_environment = os.environ.copy()
        process_environment.pop("MEGALINTER_CONFIG", None)
        process_environment.pop("MEGALINTER_IMAGE", None)
        process_environment.pop("MEGALINTER_REPORT_DIRECTORY", None)
        process_environment.pop("MEGALINTER_VERSION", None)
        if environment:
            process_environment.update(environment)

        return subprocess.run(  # nosec B603 B607
            ["bash", str(WRAPPER_PATH), *arguments],
            cwd=REPOSITORY_ROOT,
            env=process_environment,
            check=False,
            capture_output=True,
            text=True,
        )

    def test_help_documents_current_defaults(self) -> None:
        result = self.run_wrapper("--help")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("MegaLinter v10.0.0", result.stdout)
        self.assertIn("Default: .reports/egolint", result.stdout)
        self.assertIn("MEGALINTER_CONFIG", result.stdout)

    def test_rejects_report_directory_outside_workspace(self) -> None:
        result = self.run_wrapper(
            "--report-directory",
            "/tmp/unsafe-reports",  # nosec
        )

        self.assertEqual(result.returncode, 2)
        self.assertIn("must be relative", result.stderr)

    def test_rejects_report_directory_traversal(self) -> None:
        result = self.run_wrapper("--report-directory", "../unsafe-reports")

        self.assertEqual(result.returncode, 2)
        self.assertIn("may not contain a '..' segment", result.stderr)

    def test_rejects_configuration_outside_workspace(self) -> None:
        with tempfile.NamedTemporaryFile(suffix=".yml") as configuration_file:
            result = self.run_wrapper("--config", configuration_file.name)

        self.assertEqual(result.returncode, 2)
        self.assertIn("must be inside the workspace", result.stderr)

    def test_dry_run_uses_standalone_configuration_and_report_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            docker_path = Path(temporary_directory) / "docker"
            docker_path.write_text("#!/usr/bin/env sh\nexit 0\n", encoding="utf-8")
            docker_path.chmod(docker_path.stat().st_mode | stat.S_IXUSR)
            environment = {
                "PATH": f"{temporary_directory}{os.pathsep}{os.environ['PATH']}",
                "MEGALINTER_CONFIG": ".mega-linter.yml",
            }

            result = self.run_wrapper(
                "--runtime",
                "docker",
                "--dry-run",
                environment=environment,
            )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("MEGALINTER_CONFIG=<redacted>", result.stdout)
        self.assertIn("REPORT_OUTPUT_FOLDER=<redacted>", result.stdout)
        self.assertIn("ghcr.io/oxsecurity/megalinter:v10.0.0", result.stdout)


if __name__ == "__main__":
    unittest.main()
