# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Contract tests for adaptive JSONSkooma selection."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess  # nosec B404
import sys
import tempfile
import unittest
from unittest import mock

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
SCRIPT_PATH = REPOSITORY_ROOT / "scripts/json_skooma.py"
SPECIFICATION = importlib.util.spec_from_file_location("json_skooma_adapter", SCRIPT_PATH)
if SPECIFICATION is None or SPECIFICATION.loader is None:
    load_error = f"Unable to load {SCRIPT_PATH}"
    raise RuntimeError(load_error)
JSON_SKOOMA = importlib.util.module_from_spec(SPECIFICATION)
sys.modules[SPECIFICATION.name] = JSON_SKOOMA
SPECIFICATION.loader.exec_module(JSON_SKOOMA)

FIXTURE_ROOT = REPOSITORY_ROOT / "tests/fixtures/json-skooma"
NEGATIVE_FIXTURE = REPOSITORY_ROOT / "tests/fixtures/negative/json-skooma"


class JSONSkoomaApplicabilityTests(unittest.TestCase):
    """Prove activation, overrides, and no-Ruby skip behavior."""

    def test_ruby_project_with_mapping_is_applicable(self) -> None:
        state = JSON_SKOOMA.evaluate_applicability(FIXTURE_ROOT / "valid")

        self.assertEqual(state.status, "applicable")
        self.assertEqual(state.reason, "ruby-schema-mapping")
        self.assertEqual(state.ruby_markers, ("Gemfile",))

    def test_invalid_instance_fixture_is_still_applicable(self) -> None:
        state = JSON_SKOOMA.evaluate_applicability(NEGATIVE_FIXTURE)

        self.assertEqual(state.status, "applicable")
        self.assertEqual(state.reason, "ruby-schema-mapping")

    def test_non_ruby_repository_skips_before_runtime_resolution(self) -> None:
        state = JSON_SKOOMA.evaluate_applicability(FIXTURE_ROOT / "non-ruby")

        self.assertEqual(state.status, "not_applicable")
        self.assertEqual(state.reason, "ruby-project-marker-absent")

    def test_schema_mapping_absence_is_an_explicit_skip(self) -> None:
        state = JSON_SKOOMA.evaluate_applicability(FIXTURE_ROOT / "schema-absent")

        self.assertEqual(state.status, "not_applicable")
        self.assertEqual(state.reason, "schema-mapping-absent")

    def test_disabled_override_skips_before_ruby_is_launched(self) -> None:
        fixture = FIXTURE_ROOT / "disabled"
        with tempfile.TemporaryDirectory() as temporary_directory:
            report = Path(temporary_directory) / "report.json"
            arguments = [
                str(SCRIPT_PATH),
                "--workspace",
                str(fixture),
                "--report",
                str(report),
                "--ruby-executable",
                "/runtime/must/not/start",
            ]
            with (
                mock.patch.object(sys, "argv", arguments),
                mock.patch.object(
                    JSON_SKOOMA.subprocess,
                    "run",
                ) as run,
            ):
                exit_code = JSON_SKOOMA.main()

            self.assertEqual(exit_code, 0)
            run.assert_not_called()
            payload = json.loads(report.read_text(encoding="utf-8"))
            self.assertEqual(payload["status"], "not_applicable")
            self.assertEqual(payload["reason"], "explicitly-disabled")

    def test_enabled_override_activates_without_a_ruby_manifest(self) -> None:
        state = JSON_SKOOMA.evaluate_applicability(FIXTURE_ROOT / "explicit-enabled")

        self.assertEqual(state.status, "applicable")
        self.assertEqual(state.reason, "explicitly-enabled")
        self.assertEqual(state.ruby_markers, ())

    def test_applicable_capability_uses_argv_and_a_bounded_timeout(self) -> None:
        fixture = FIXTURE_ROOT / "valid"
        completed: subprocess.CompletedProcess[str] = subprocess.CompletedProcess(
            args=[], returncode=0
        )
        arguments = [
            str(SCRIPT_PATH),
            "--workspace",
            str(fixture),
            "--report",
            "-",
            "--ruby-executable",
            "/opt/ruby",
            "--timeout-seconds",
            "17",
        ]
        with (
            mock.patch.object(sys, "argv", arguments),
            mock.patch.object(
                JSON_SKOOMA.subprocess,
                "run",
                return_value=completed,
            ) as run,
        ):
            exit_code = JSON_SKOOMA.main()

        self.assertEqual(exit_code, 0)
        command = run.call_args.args[0]
        self.assertEqual(command[0], "/opt/ruby")
        self.assertNotIn("sh", command)
        self.assertEqual(run.call_args.kwargs["timeout"], 17)
        self.assertEqual(run.call_args.kwargs["env"]["EGOLINT_JSON_SKOOMA_NETWORK"], "none")

    def test_mapping_paths_cannot_escape_the_workspace(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            workspace = Path(temporary_directory)
            (workspace / "Gemfile").write_text('source "https://rubygems.org"\n')
            config_directory = workspace / ".egolint"
            config_directory.mkdir()
            (config_directory / "json-skooma.json").write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "mode": "auto",
                        "mappings": [{"schema": "../outside.json", "instances": ["data.json"]}],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(  # noqa: PT027
                JSON_SKOOMA.ConfigurationError, "repository-relative"
            ):
                JSON_SKOOMA.evaluate_applicability(workspace)


if __name__ == "__main__":
    unittest.main()
