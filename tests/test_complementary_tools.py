# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Contract tests for the direct complementary-tool platform."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
from typing import Any
import unittest
from unittest import mock

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
SCRIPT_PATH = REPOSITORY_ROOT / "scripts/complementary_tools.py"
SPECIFICATION = importlib.util.spec_from_file_location("complementary_tools", SCRIPT_PATH)
if SPECIFICATION is None or SPECIFICATION.loader is None:
    raise RuntimeError(f"Unable to load {SCRIPT_PATH}")
COMPLEMENTARY_TOOLS = importlib.util.module_from_spec(SPECIFICATION)
sys.modules[SPECIFICATION.name] = COMPLEMENTARY_TOOLS
SPECIFICATION.loader.exec_module(COMPLEMENTARY_TOOLS)


class ComplementaryToolContractTests(unittest.TestCase):
    """Verify inventory, applicability, task, and editor parity contracts."""

    manifest: dict[str, Any]
    tools: dict[str, dict[str, Any]]

    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = COMPLEMENTARY_TOOLS.load_manifest()
        cls.tools = {tool["id"]: tool for tool in cls.manifest["tools"]}

    def test_manifest_is_complete_and_valid(self) -> None:
        self.assertEqual(COMPLEMENTARY_TOOLS.validate_manifest(self.manifest), [])
        self.assertEqual(len(self.tools), 18)

    def test_generated_matrix_is_current(self) -> None:
        matrix_path = REPOSITORY_ROOT / self.manifest["generated_matrix"]
        expected = COMPLEMENTARY_TOOLS.rendered_json(
            COMPLEMENTARY_TOOLS.generated_matrix(self.manifest)
        )
        self.assertEqual(matrix_path.read_text(encoding="utf-8"), expected)

    def test_fixture_files_do_not_activate_project_aware_tools(self) -> None:
        for identifier in (
            "buf",
            "complexipy",
            "conftest",
            "deptry",
            "govulncheck",
            "interrogate",
            "knip",
            "regal",
            "vacuum",
            "vulture",
        ):
            with self.subTest(tool=identifier):
                state = COMPLEMENTARY_TOOLS.resolve_tool_state(self.tools[identifier])
                self.assertEqual(state.applicability, "not_applicable")
                self.assertEqual(state.dependency, "not_checked")

    def test_standalone_rust_project_activates_cargo_deny(self) -> None:
        state = COMPLEMENTARY_TOOLS.resolve_tool_state(self.tools["cargo-deny"])
        self.assertEqual(state.applicability, "applicable")
        self.assertIn("Cargo.toml", state.matched_files)
        self.assertTrue(any(path.startswith("src/") for path in state.matched_files))

    def test_universal_tools_are_applicable(self) -> None:
        for identifier in (
            "addlicense",
            "commitlint",
            "detect-secrets",
            "reuse",
            "tombi",
            "typos",
        ):
            with self.subTest(tool=identifier):
                state = COMPLEMENTARY_TOOLS.resolve_tool_state(self.tools[identifier])
                self.assertEqual(state.applicability, "applicable")

    def test_fixture_latex_does_not_activate_latexindent(self) -> None:
        state = COMPLEMENTARY_TOOLS.resolve_tool_state(self.tools["latexindent"])
        self.assertEqual(state.applicability, "not_applicable")
        self.assertEqual(state.matched_files, ())

    def test_all_of_marker_groups_require_every_project_marker(self) -> None:
        tool = self.tools["deptry"]
        with mock.patch.object(
            COMPLEMENTARY_TOOLS,
            "matching_files",
            side_effect=lambda pattern: ["pyproject.toml"] if pattern == "pyproject.toml" else [],
        ):
            applicability, matches = COMPLEMENTARY_TOOLS.resolve_applicability(tool)
        self.assertEqual(applicability, "not_applicable")
        self.assertEqual(matches, ())

    def test_file_placeholder_expands_without_shell_interpretation(self) -> None:
        tool = self.tools["vacuum"]
        state = COMPLEMENTARY_TOOLS.ToolState(
            identifier="vacuum",
            applicability="applicable",
            dependency="available",
            matched_files=("openapi.yaml", "services/api/openapi.json"),
        )
        command = COMPLEMENTARY_TOOLS.expand_command(tool, state)
        self.assertNotIn("{files}", command)
        self.assertEqual(command[-2:], ["openapi.yaml", "services/api/openapi.json"])

    def test_recursive_globs_match_zero_or_more_directories(self) -> None:
        self.assertTrue(COMPLEMENTARY_TOOLS.matches_glob(Path("pyproject.toml"), "**/*.toml"))
        self.assertTrue(COMPLEMENTARY_TOOLS.matches_glob(Path("src/app.py"), "src/**/*.py"))
        self.assertTrue(COMPLEMENTARY_TOOLS.matches_glob(Path("src/service/app.py"), "src/**/*.py"))
        self.assertFalse(COMPLEMENTARY_TOOLS.matches_glob(Path("tools/app.py"), "src/**/*.py"))

    def test_repository_index_prunes_generated_and_dependency_trees(self) -> None:
        indexed_paths = COMPLEMENTARY_TOOLS.repository_files()
        self.assertTrue(indexed_paths)
        self.assertFalse(any(COMPLEMENTARY_TOOLS.is_excluded(path) for path in indexed_paths))

    def test_missing_runtime_is_distinct_from_not_applicable(self) -> None:
        tool = dict(self.tools["tombi"])
        with mock.patch.object(COMPLEMENTARY_TOOLS, "runtime_available", return_value=False):
            state = COMPLEMENTARY_TOOLS.resolve_tool_state(tool)
        self.assertEqual(state.applicability, "applicable")
        self.assertEqual(state.dependency, "missing_dependency")

    def test_megalinter_native_supply_chain_tools_are_not_duplicated(self) -> None:
        forbidden = {"grype", "osv", "secretlint", "syft", "trivy", "trivy-sbom"}
        self.assertTrue(forbidden.isdisjoint(self.tools))

    def test_taskfile_exposes_stable_aggregate_interface(self) -> None:
        root_taskfile = (REPOSITORY_ROOT / "Taskfile.yml").read_text(encoding="utf-8")
        taskfile = (REPOSITORY_ROOT / "tasks/complementary.yml").read_text(encoding="utf-8")
        self.assertIn("taskfile: ./tasks/complementary.yml", root_taskfile)
        self.assertIn("flatten: false", root_taskfile)
        for task_name in (
            "default:",
            "contracts:",
            "contracts:write:",
            "status:",
            "versions:",
            "dependencies:python:",
            "dependencies:security:",
        ):
            with self.subTest(task=task_name):
                self.assertIn(task_name, taskfile)

    def test_report_destinations_are_unique_and_namespaced(self) -> None:
        report_paths = [tool["report_path"] for tool in self.tools.values()]
        self.assertEqual(len(report_paths), len(set(report_paths)))
        self.assertTrue(
            all(path.startswith(".reports/egolint/complementary/") for path in report_paths)
        )

    def test_python_tool_environment_has_a_checked_lockfile(self) -> None:
        lockfile = REPOSITORY_ROOT / "uv.lock"
        self.assertTrue(lockfile.is_file())
        lock_text = lockfile.read_text(encoding="utf-8")
        for package in (
            "complexipy",
            "deptry",
            "detect-secrets",
            "interrogate",
            "pre-commit",
            "tombi",
            "vulture",
        ):
            with self.subTest(package=package):
                self.assertIn(f'name = "{package}"', lock_text)


if __name__ == "__main__":
    unittest.main()
