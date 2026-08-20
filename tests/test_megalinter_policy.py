# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import shutil
import subprocess  # nosec B404
import sys
import tempfile
import textwrap
from typing import Any
import unittest

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPOSITORY_ROOT / "scripts" / "validate_megalinter_policy.py"
ESLINT_CONFIG_PATH = REPOSITORY_ROOT / ".config" / "lint" / "javascript" / "eslint.config.mjs"
SPEC = importlib.util.spec_from_file_location("validate_megalinter_policy", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load MegaLinter policy module from {MODULE_PATH}")
megalinter_policy = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = megalinter_policy
SPEC.loader.exec_module(megalinter_policy)


class MegaLinterPolicyTests(unittest.TestCase):
    catalog: dict[str, Any]
    matrix: dict[str, Any]
    matrix_by_id: dict[str, dict[str, Any]]

    @classmethod
    def setUpClass(cls) -> None:
        cls.catalog = json.loads(megalinter_policy.CATALOG_PATH.read_text(encoding="utf-8"))
        cls.matrix = json.loads(megalinter_policy.MATRIX_PATH.read_text(encoding="utf-8"))
        cls.matrix_by_id = {tool["id"]: tool for tool in cls.matrix["tools"]}

    def test_generated_contracts_are_current(self) -> None:
        result = subprocess.run(  # nosec B603
            [sys.executable, str(MODULE_PATH), "--check"],
            cwd=REPOSITORY_ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, f"{result.stdout}\n{result.stderr}")

    def test_catalog_is_pinned_to_the_supported_release(self) -> None:
        self.assertEqual(self.catalog["megalinter_release"], "v10.0.0")
        self.assertEqual(
            self.catalog["megalinter_commit"],
            "15e5b45552097e318c93de385779ce3b1084052c",
        )
        self.assertEqual(len(self.catalog["tools"]), 124)
        self.assertNotIn(
            "unknown",
            {tool["version"] for tool in self.catalog["tools"].values()},
        )

    def test_all_profile_snapshots_are_explicit(self) -> None:
        fast = json.loads(
            megalinter_policy.PROFILE_SNAPSHOT_PATHS["fast"].read_text(encoding="utf-8")
        )
        holistic = json.loads(
            megalinter_policy.PROFILE_SNAPSHOT_PATHS["holistic"].read_text(encoding="utf-8")
        )
        security = json.loads(
            megalinter_policy.PROFILE_SNAPSHOT_PATHS["security"].read_text(encoding="utf-8")
        )
        dependency_debt = json.loads(
            megalinter_policy.PROFILE_SNAPSHOT_PATHS["dependency-debt"].read_text(encoding="utf-8")
        )
        self.assertEqual(len(fast["selected"]), 12)
        self.assertFalse(fast["validate_all_codebase"])
        self.assertTrue(holistic["validate_all_codebase"])
        self.assertIn("REPOSITORY_BETTERLEAKS", fast["selected"])
        self.assertIn("PYTHON_RUFF_FORMAT", holistic["selected"])
        self.assertIn("ACTION_ZIZMOR", holistic["disabled_by_configuration"])
        self.assertNotIn("REPOSITORY_GITLEAKS", holistic["selected"])
        self.assertEqual(len(security["selected"]), 6)
        self.assertEqual(len(dependency_debt["selected"]), 6)
        self.assertIn("REPOSITORY_TRUFFLEHOG", security["selected"])
        self.assertIn("REPOSITORY_OSV_SCANNER", dependency_debt["selected"])
        self.assertNotIn("PYTHON_RUFF_FORMAT", security["selected"])

    def test_target_tools_have_truthful_config_and_fixture_contracts(self) -> None:
        expected_configuration_paths = {
            "ANSIBLE_ANSIBLE_LINT": ".config/lint/ansible/ansible-lint.yml",
            "ACTION_ZIZMOR": ".config/lint/actions/zizmor.yml",
            "GO_GOLANGCI_LINT": ".config/lint/go/golangci-lint.yml",
            "JSON_V8R": ".config/lint/yaml/.v8rrc.yml",
            "MARKDOWN_RUMDL": ".config/lint/markdown/.rumdl.toml",
            "PYTHON_RUFF_FORMAT": ".config/lint/python/ruff.toml",
            "REPOSITORY_SEMGREP": ".config/security/semgrep/semgrep.yml",
            "SPELL_CODESPELL": ".config/lint/prose/spell/.codespellrc",
        }
        for tool_id, expected_path in expected_configuration_paths.items():
            tool = self.matrix_by_id[tool_id]
            self.assertEqual(tool["configuration_path"], expected_path)
            self.assertTrue((REPOSITORY_ROOT / expected_path).is_file())
            fixture_contract = tool["fixtures"]
            self.assertTrue(
                fixture_contract.get("blocker")
                or (fixture_contract.get("positive") and fixture_contract.get("negative"))
            )

    def test_every_tool_exposes_structured_ownership_and_evidence(self) -> None:
        for tool in self.matrix["tools"]:
            with self.subTest(tool=tool["id"]):
                self.assertTrue(tool["owner"])
                self.assertTrue(tool["policy_source"])
                self.assertEqual(
                    tool["evidence"]["configuration"],
                    tool["configuration_path"],
                )
                self.assertEqual(
                    tool["evidence"]["fixtures"],
                    tool["fixtures"],
                )
                self.assertEqual(
                    tool["evidence"]["runtime_report"],
                    tool["report_path"],
                )

    def test_removed_linter_variables_and_selections_are_rejected(self) -> None:
        deprecated_findings = megalinter_policy.validate_configuration(
            Path("deprecated.yml"),
            {"API_SPECTRAL_CONFIG_FILE": ".spectral.yaml"},
            self.catalog,
        )
        removed_findings = megalinter_policy.validate_configuration(
            Path("removed.yml"),
            {
                "ENABLE": ["API"],
                "ENABLE_LINTERS": ["REPOSITORY_GITLEAKS"],
            },
            self.catalog,
        )
        self.assertTrue(any("removed/deprecated" in finding for finding in deprecated_findings))
        self.assertTrue(any("removed linter" in finding for finding in removed_findings))
        self.assertTrue(any("removed descriptor" in finding for finding in removed_findings))

    def test_resolve_configuration_path_accepts_workspace_anchored_rules(self) -> None:
        expected = REPOSITORY_ROOT / ".config" / "lint" / "terraform" / ".tflint.hcl"
        for rules_path in (
            "${GITHUB_WORKSPACE}/.config/lint/terraform",
            "/github/workspace/.config/lint/terraform",
            "/tmp/lint/.config/lint/terraform",  # noqa: S108  # nosec B108
        ):
            with self.subTest(rules_path=rules_path):
                resolved = megalinter_policy.resolve_configuration_path(
                    rules_path,
                    ".tflint.hcl",
                )
                self.assertEqual(resolved, expected)

    def test_selection_and_result_states_are_distinguishable(self) -> None:
        expected_result_states = {
            "configuration_error",
            "disabled_by_configuration",
            "disabled_by_profile",
            "execution_error",
            "failed_findings",
            "missing_from_image",
            "not_applicable",
            "passed",
            "passed_with_warnings",
            "selected",
            "timed_out",
        }
        self.assertEqual(set(self.matrix["result_statuses"]), expected_result_states)
        self.assertEqual(
            self.matrix_by_id["ACTION_ZIZMOR"]["profiles"]["holistic"],
            "disabled_by_configuration",
        )
        self.assertEqual(
            self.matrix_by_id["PYTHON_RUFF_FORMAT"]["profiles"]["fast"],
            "disabled_by_profile",
        )
        self.assertEqual(
            self.matrix_by_id["PYTHON_RUFF_FORMAT"]["profiles"]["holistic"],
            "selected",
        )
        self.assertEqual(
            self.matrix_by_id["PYTHON_RUFF_FORMAT"]["enforcement"],
            "blocking",
        )
        self.assertEqual(
            self.matrix_by_id["REPOSITORY_GRYPE"]["enforcement"],
            "advisory",
        )
        self.assertEqual(
            self.matrix_by_id["ACTION_ZIZMOR"]["enforcement"],
            "disabled",
        )
        self.assertEqual(
            self.matrix_by_id["REPOSITORY_OSV_SCANNER"]["profiles"]["dependency-debt"],
            "selected",
        )
        self.assertEqual(
            self.matrix_by_id["REPOSITORY_OSV_SCANNER"]["profile_enforcement"]["dependency-debt"],
            "blocking",
        )
        self.assertEqual(
            self.matrix_by_id["REPOSITORY_GRYPE"]["profile_enforcement"]["dependency-debt"],
            "advisory",
        )

    def test_root_policy_never_bootstraps_or_masks_results(self) -> None:
        configuration = megalinter_policy.resolve_extended_configuration(
            REPOSITORY_ROOT / ".mega-linter.yml"
        )
        self.assertNotIn("PRE_COMMANDS", configuration)
        self.assertNotIn("POST_COMMANDS", configuration)

    def test_fix_tasks_request_wrapper_write_capability_explicitly(self) -> None:
        taskfile = megalinter_policy.load_yaml(REPOSITORY_ROOT / "tasks" / "lint.yml")
        fix_tasks = {
            name: task
            for name, task in taskfile["tasks"].items()
            if name == "fix" or name.endswith(":fix")
        }
        self.assertGreater(len(fix_tasks), 1)
        for name, task in fix_tasks.items():
            self.assertNotIn("APPLY_FIXES", task.get("env", {}), name)
            command = "\n".join(str(value) for value in task.get("cmds", []))
            self.assertIn("--fix", command, name)

    def test_runtime_writes_stay_under_the_writable_report_mount(self) -> None:
        configuration = megalinter_policy.resolve_extended_configuration(
            REPOSITORY_ROOT / ".mega-linter.yml"
        )
        self.assertEqual(configuration["REPORT_OUTPUT_FOLDER"], ".reports/egolint")

        for linter_id in (
            "JAVASCRIPT_ES",
            "JSX_ESLINT",
            "TSX_ESLINT",
            "TYPESCRIPT_ES",
        ):
            arguments = str(configuration[f"{linter_id}_ARGUMENTS"])
            self.assertIn(".reports/egolint/cache/eslint/", arguments)
            self.assertNotIn("--cache-location .cache/", arguments)

        cspell = json.loads(
            (REPOSITORY_ROOT / ".config/lint/prose/spell/cspell.json").read_text(encoding="utf-8")
        )
        cspell_reporter = json.loads(
            (REPOSITORY_ROOT / ".config/lint/prose/spell/cspell.megalinter.json").read_text(
                encoding="utf-8"
            )
        )
        trivy = megalinter_policy.load_yaml(REPOSITORY_ROOT / ".config/security/trivy/trivy.yaml")
        trivy_sbom = megalinter_policy.load_yaml(
            REPOSITORY_ROOT / ".config/security/trivy/trivy-sbom.yaml"
        )
        self.assertEqual(
            cspell["cache"]["cacheLocation"],
            "${cwd}/.reports/egolint/cache/cspell/cspell-cache.json",
        )
        self.assertEqual(
            cspell_reporter["reporters"][1][1]["outFile"],
            "${cwd}/.reports/egolint/cspell/cspell-report.json",
        )
        self.assertEqual(trivy["cache"]["dir"], ".reports/egolint/cache/trivy")
        self.assertEqual(trivy_sbom["cache"]["dir"], ".reports/egolint/cache/trivy")

        self.assertTrue(
            all(
                tool["report_path"].startswith(".reports/egolint/") for tool in self.matrix["tools"]
            )
        )

    def test_eslint_config_only_registers_json_blocks_when_plugin_exists(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            workspace = Path(temporary_directory)
            config_directory = workspace / ".config" / "lint" / "javascript"
            node_modules = workspace / "node_modules"
            config_directory.mkdir(parents=True)
            node_modules.mkdir()
            config_source = ESLINT_CONFIG_PATH.read_text(encoding="utf-8")
            without_json_config_path = config_directory / "eslint.without-json.config.mjs"
            with_json_config_path = config_directory / "eslint.with-json.config.mjs"
            without_json_config_path.write_text(config_source, encoding="utf-8")
            with_json_config_path.write_text(config_source, encoding="utf-8")

            self._write_commonjs_module(
                node_modules,
                "@eslint/js",
                "module.exports = { configs: { recommended: { rules: {} } } };",
            )
            self._write_commonjs_module(
                node_modules,
                "@typescript-eslint/eslint-plugin",
                "module.exports = { configs: { recommended: { rules: {} } } };",
            )
            self._write_commonjs_module(
                node_modules,
                "@typescript-eslint/parser",
                "module.exports = { parseForESLint() { return {}; } };",
            )
            self._write_commonjs_module(
                node_modules,
                "eslint-plugin-react",
                "module.exports = { configs: { recommended: { rules: {} } } };",
            )
            self._write_commonjs_file(
                node_modules / "eslint" / "config.js",
                textwrap.dedent(
                    """
                    module.exports = {
                      defineConfig(config) {
                        return config;
                      },
                      globalIgnores(patterns, name) {
                        return { ignores: patterns, name };
                      },
                    };
                    """
                ).strip(),
            )
            self._write_commonjs_module(
                node_modules,
                "globals",
                textwrap.dedent(
                    """
                    module.exports = {
                      browser: {},
                      es2024: {},
                      jest: {},
                      mocha: {},
                      node: {},
                      vitest: {},
                    };
                    """
                ).strip(),
            )

            without_plugin = self._load_eslint_languages(without_json_config_path)
            self.assertNotIn("json/json", without_plugin)
            self.assertNotIn("json/jsonc", without_plugin)
            self.assertNotIn("json/json5", without_plugin)

            self._write_commonjs_module(
                node_modules,
                "@eslint/json",
                "module.exports = { configs: { recommended: { rules: { 'json/no-duplicate-keys': 'error' } } } };",
            )

            with_plugin = self._load_eslint_languages(with_json_config_path)
            self.assertIn("json/json", with_plugin)
            self.assertIn("json/jsonc", with_plugin)
            self.assertIn("json/json5", with_plugin)

    def _load_eslint_languages(self, config_path: Path) -> list[str]:
        node_executable = shutil.which("node")
        if node_executable is None:
            self.fail("Node.js is required to validate the ESLint configuration.")
        script = textwrap.dedent(
            """
            const { default: config } = await import(process.argv[1]);

            const languages = config
              .map((entry) => entry?.language)
              .filter((language) => typeof language === "string");

            console.log(JSON.stringify(languages));
            """
        ).strip()
        result = subprocess.run(  # nosec B603
            [node_executable, "--input-type=module", "--eval", script, config_path.as_uri()],
            cwd=config_path.parent,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, f"{result.stdout}\n{result.stderr}")
        languages = json.loads(result.stdout)
        self.assertIsInstance(languages, list)
        self.assertTrue(all(isinstance(language, str) for language in languages))
        return [str(language) for language in languages]

    def _write_commonjs_module(
        self,
        node_modules: Path,
        package_name: str,
        source: str,
    ) -> None:
        module_path = node_modules / Path(*package_name.split("/"))
        module_path.mkdir(parents=True, exist_ok=True)
        self._write_commonjs_file(module_path / "index.js", source)

    def _write_commonjs_file(self, path: Path, source: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"{source}\n", encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
