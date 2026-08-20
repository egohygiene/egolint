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
        self.assertIn("--network", result.stdout)
        self.assertIn("--cap-drop", result.stdout)
        self.assertIn("--security-opt", result.stdout)
        self.assertIn(":ro", result.stdout)
        self.assertNotIn(f"{REPOSITORY_ROOT}:/tmp/lint:rw", result.stdout)

    def test_direct_fix_is_rejected_in_favor_of_isolated_native_flow(self) -> None:
        for argument in ("--fix", "--fix=PYTHON_RUFF"):
            with self.subTest(argument=argument):
                result = self.run_wrapper(argument)
                self.assertEqual(result.returncode, 2)
                self.assertIn("isolated review boundary", result.stderr)
                self.assertIn("native Egolint CLI", result.stderr)

    def test_configuration_symlink_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            workspace = root / "workspace"
            workspace.mkdir()
            outside = root / "outside.yml"
            outside.write_text("---\n", encoding="utf-8")
            (workspace / "linked.yml").symlink_to(outside)

            result = self.run_wrapper(
                "--workspace",
                str(workspace),
                "--config",
                "linked.yml",
            )

        self.assertEqual(result.returncode, 2)
        self.assertIn("may not be a symbolic link", result.stderr)

    def test_rejects_removed_container_escape_hatches(self) -> None:
        for arguments in (
            ("--env", "GITHUB_TOKEN=value"),
            ("--env-file", ".env"),
            ("--volume", "/:/host"),
            ("--mount-docker-socket",),
            ("--runtime-arg", "--privileged"),
            ("--", "--privileged"),
        ):
            with self.subTest(arguments=arguments):
                result = self.run_wrapper(*arguments)
                self.assertEqual(result.returncode, 2)
                self.assertIn("removed", result.stderr)

    def test_rejects_custom_relative_report_directory(self) -> None:
        result = self.run_wrapper("--report-directory", "reports")

        self.assertEqual(result.returncode, 2)
        self.assertIn("fixed at .reports/egolint", result.stderr)


if __name__ == "__main__":
    unittest.main()
