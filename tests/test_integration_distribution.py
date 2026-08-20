# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

from __future__ import annotations

import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap
import unittest

import yaml

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]


class IntegrationDistributionTests(unittest.TestCase):
    def load_yaml(self, relative_path: str) -> object:
        with (REPOSITORY_ROOT / relative_path).open(encoding="utf-8") as stream:
            return yaml.safe_load(stream)

    def test_action_is_check_only_and_does_not_accept_tokens(self) -> None:
        action = self.load_yaml("action.yml")
        self.assertIsInstance(action, dict)
        serialized = json.dumps(action).lower()
        script = (REPOSITORY_ROOT / "scripts/github-action.sh").read_text(encoding="utf-8")

        self.assertNotIn("token", json.dumps(action["inputs"]).lower())
        self.assertNotIn(" fix", script)
        self.assertIn('"lint"', script)
        self.assertIn("@sha256:", script)
        self.assertGreaterEqual(script.count('"env" "-i"'), 2)
        self.assertGreaterEqual(script.count('"CI=true"'), 2)
        self.assertIn('${RUNNER_OS:-} == "Linux"', script)
        self.assertIn('cd "${GITHUB_ACTION_PATH}"', script)
        self.assertIn('"CARGO_HOME=${RUNNER_TEMP}/egolint-action-cargo-home"', script)
        self.assertNotIn("upload", serialized)
        self.assertNotRegex(script, r"\bgh\s+(release|api)\b")

    def test_pre_commit_hook_is_serial_check_only(self) -> None:
        hooks = self.load_yaml(".pre-commit-hooks.yaml")
        self.assertIsInstance(hooks, list)
        hook = hooks[0]

        self.assertEqual(hook["language"], "rust")
        self.assertFalse(hook["pass_filenames"])
        self.assertTrue(hook["require_serial"])
        self.assertIn(" lint ", f" {hook['entry']} ")
        self.assertNotIn(" fix ", f" {hook['entry']} ")

    def test_action_exposes_canonical_and_private_evidence_separately(self) -> None:
        action = self.load_yaml("action.yml")
        outputs = action["outputs"]
        self.assertEqual(
            set(outputs),
            {
                "run-report",
                "sarif-report",
                "debt-json",
                "debt-markdown",
                "raw-megalinter-json",
                "raw-megalinter-sarif",
                "exit-code",
            },
        )
        script = (REPOSITORY_ROOT / "scripts/github-action.sh").read_text(encoding="utf-8")
        for report in ("run.json", "egolint.sarif", "debt.json", "debt.md"):
            self.assertIn(report, script)
        for raw_report in ("mega-linter-report.json", "mega-linter-report.sarif"):
            self.assertIn(raw_report, script)

    def test_action_runtime_is_check_only_and_secret_isolated(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            temporary_root = Path(temporary_directory)
            fake_bin = temporary_root / "bin"
            runner_temp = temporary_root / "runner"
            workspace = temporary_root / "workspace"
            fake_home = temporary_root / "home"
            output = temporary_root / "action-output"
            for directory in (fake_bin, runner_temp, workspace, fake_home):
                directory.mkdir()

            fake_cargo = fake_bin / "cargo"
            fake_cargo.write_text(
                textwrap.dedent(
                    r"""
                    #!/usr/bin/env bash
                    set -o errexit
                    set -o nounset
                    set -o pipefail
                    printf '::warning::untrusted build output\n'
                    env > "${HOME}/cargo-environment"
                    target_directory=""
                    while [[ $# -gt 0 ]]; do
                      if [[ $1 == "--target-dir" ]]; then
                        shift
                        target_directory="$1"
                      fi
                      shift
                    done
                    test -n "${target_directory}"
                    mkdir -p "${target_directory}/release"
                    cat > "${target_directory}/release/egolint" <<'SCRIPT'
                    #!/usr/bin/env bash
                    set -o errexit
                    set -o nounset
                    set -o pipefail
                    env > "${HOME}/runtime-environment"
                    printf '%s\n' "$@" > "${HOME}/runtime-arguments"
                    SCRIPT
                    chmod 0755 "${target_directory}/release/egolint"
                    """
                ).lstrip(),
                encoding="utf-8",
            )
            fake_cargo.chmod(0o755)

            environment = os.environ.copy()
            environment.update(
                {
                    "PATH": f"{fake_bin}:/usr/bin:/bin",
                    "HOME": str(fake_home),
                    "GITHUB_ACTION_PATH": str(REPOSITORY_ROOT),
                    "GITHUB_WORKSPACE": str(workspace),
                    "GITHUB_OUTPUT": str(output),
                    "RUNNER_OS": "Linux",
                    "RUNNER_TEMP": str(runner_temp),
                    "INPUT_WORKSPACE": ".",
                    "INPUT_PROFILE": "dependency-debt",
                    "INPUT_IMAGE": f"example.invalid/egolint@sha256:{'0' * 64}",
                    "INPUT_ALLOW_UNPINNED_IMAGE": "false",
                    "INPUT_PULL_POLICY": "never",
                    "INPUT_NETWORK": "none",
                    "INPUT_MEGALINTER_CONFIG": "",
                    "INPUT_REPOSITORY_CONTRACT": "contracts/repository.toml",
                    "INPUT_SUPPRESSION": "policy/suppression.json",
                    "INPUT_EVALUATION_DATE": "2026-08-19",
                    "INPUT_CHANGED_ONLY": "false",
                    "GITHUB_TOKEN": "must-not-cross-the-boundary",
                    "UNRELATED_SECRET": "must-not-cross-the-boundary",
                }
            )
            # The executable is a repository-owned, fixed test target.
            completed = subprocess.run(  # noqa: S603
                [str(REPOSITORY_ROOT / "scripts/github-action.sh")],
                check=False,
                cwd=REPOSITORY_ROOT,
                env=environment,
                capture_output=True,
                text=True,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            stop_index = completed.stdout.index("::stop-commands::")
            untrusted_index = completed.stdout.index("::warning::untrusted build output")
            resume_matches = list(
                re.finditer(r"^::[0-9a-f-]{36}::$", completed.stdout, re.MULTILINE)
            )
            self.assertEqual(len(resume_matches), 1)
            self.assertLess(stop_index, untrusted_index)
            self.assertLess(untrusted_index, resume_matches[0].start())

            for environment_file in (
                fake_home / "cargo-environment",
                fake_home / "runtime-environment",
            ):
                child_environment = environment_file.read_text(encoding="utf-8")
                self.assertIn("CI=true", child_environment)
                self.assertNotIn("GITHUB_TOKEN", child_environment)
                self.assertNotIn("UNRELATED_SECRET", child_environment)
                self.assertNotIn("must-not-cross-the-boundary", child_environment)

            arguments = (fake_home / "runtime-arguments").read_text(encoding="utf-8").splitlines()
            self.assertIn("lint", arguments)
            self.assertNotIn("check", arguments)
            self.assertNotIn("fix", arguments)
            self.assertIn("--repository-contract", arguments)
            self.assertIn("contracts/repository.toml", arguments)
            self.assertIn("--suppression", arguments)
            self.assertIn("policy/suppression.json", arguments)
            self.assertIn("--evaluation-date", arguments)
            self.assertIn("2026-08-19", arguments)

            action_outputs = dict(
                line.split("=", 1) for line in output.read_text(encoding="utf-8").splitlines()
            )
            self.assertEqual(action_outputs["run-report"], ".reports/egolint/run.json")
            self.assertEqual(action_outputs["sarif-report"], ".reports/egolint/egolint.sarif")
            self.assertEqual(action_outputs["debt-json"], ".reports/egolint/debt.json")
            self.assertEqual(action_outputs["debt-markdown"], ".reports/egolint/debt.md")
            self.assertEqual(action_outputs["exit-code"], "0")

    def test_action_rejects_output_command_injection_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            temporary_root = Path(temporary_directory)
            workspace = temporary_root / "workspace"
            runner_temp = temporary_root / "runner"
            fake_home = temporary_root / "home"
            for directory in (workspace, runner_temp, fake_home):
                directory.mkdir()
            output = temporary_root / "action-output"
            environment = os.environ.copy()
            environment.update(
                {
                    "HOME": str(fake_home),
                    "GITHUB_ACTION_PATH": str(REPOSITORY_ROOT),
                    "GITHUB_WORKSPACE": str(workspace),
                    "GITHUB_OUTPUT": str(output),
                    "RUNNER_OS": "Linux",
                    "RUNNER_TEMP": str(runner_temp),
                    "INPUT_WORKSPACE": "safe\n::stop-commands::unsafe",
                    "INPUT_IMAGE": f"example.invalid/egolint@sha256:{'0' * 64}",
                }
            )
            # The executable is a repository-owned, fixed test target.
            completed = subprocess.run(  # noqa: S603
                [str(REPOSITORY_ROOT / "scripts/github-action.sh")],
                check=False,
                cwd=REPOSITORY_ROOT,
                env=environment,
                capture_output=True,
                text=True,
            )

            self.assertEqual(completed.returncode, 2)
            self.assertIn("workspace may not contain control characters", completed.stderr)
            self.assertFalse(output.exists())

    def test_focused_profile_tools_match_scanner_ownership_and_catalog(self) -> None:
        ownership = json.loads(
            (REPOSITORY_ROOT / ".config/security/scanner-ownership.json").read_text(
                encoding="utf-8"
            )
        )
        catalog_document = json.loads(
            (REPOSITORY_ROOT / ".config/megalinter/v10-catalog.json").read_text(encoding="utf-8")
        )
        catalog = catalog_document["tools"]
        configuration_variables = set(catalog_document["configuration_variables"])

        for profile_name, file_name in (
            ("security", ".mega-linter.security.yml"),
            ("dependency-debt", ".mega-linter.dependency-debt.yml"),
        ):
            profile = self.load_yaml(file_name)
            self.assertIsInstance(profile, dict)
            self.assertEqual(set(profile).difference(configuration_variables), set())
            self.assertEqual(
                profile["ENABLE_LINTERS"],
                ownership["profiles"][profile_name]["tools"],
            )
            enforcement = ownership["profiles"][profile_name]["enforcement"]
            classified = [tool for group in enforcement.values() for tool in group]
            self.assertEqual(set(classified), set(profile["ENABLE_LINTERS"]))
            self.assertEqual(len(classified), len(set(classified)))
            self.assertTrue(profile["VALIDATE_ALL_CODEBASE"])
            for tool in enforcement.get("advisory", []):
                self.assertTrue(profile[f"{tool}_DISABLE_ERRORS"])
            for tool in profile["ENABLE_LINTERS"]:
                self.assertIn(tool, catalog)
            for reporter in (
                "AZURE_COMMENT_REPORTER",
                "BITBUCKET_COMMENT_REPORTER",
                "EMAIL_REPORTER",
                "FILEIO_REPORTER",
                "GITHUB_COMMENT_REPORTER",
                "GITHUB_STATUS_REPORTER",
                "GITLAB_COMMENT_REPORTER",
            ):
                self.assertFalse(profile[reporter])

    def test_release_uses_pinned_actions_and_immutable_tags_only(self) -> None:
        workflow = (REPOSITORY_ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
        uses = re.findall(r"^\s*uses:\s*([^\s#]+)", workflow, flags=re.MULTILINE)
        self.assertTrue(uses)
        for reference in uses:
            if reference == "./":
                continue
            self.assertRegex(reference, r"^[^@\s]+@[0-9a-f]{40}$")

        self.assertIn('tags:\n      - "v*.*.*"', workflow)
        self.assertIn("cosign verify", workflow)
        self.assertIn("gh attestation verify", workflow)
        self.assertIn("linux/amd64,linux/arm64", workflow)
        self.assertIn("subject-digest:", workflow)
        self.assertRegex(
            workflow,
            r"\breport \\\n\s+debt \\\n\s+repository-contract; do",
        )
        self.assertNotRegex(workflow, r"tags:.*:(latest|edge)\s*$")

    def test_release_separates_candidate_build_from_trusted_promotion(  # noqa: PLR0915
        self,
    ) -> None:
        workflow = self.load_yaml(".github/workflows/release.yml")
        jobs = workflow["jobs"]

        candidate = jobs["candidate"]
        self.assertEqual(candidate["permissions"], {"contents": "read"})
        candidate_text = json.dumps(candidate)
        self.assertNotIn("id-token", candidate_text)
        self.assertNotIn("attestations", candidate_text)
        self.assertNotIn("cosign", candidate_text.lower())

        signer = jobs["sign-candidate"]
        self.assertEqual(signer["environment"], "release")
        self.assertEqual(signer["permissions"]["id-token"], "write")
        self.assertEqual(signer["permissions"]["attestations"], "write")
        signer_text = json.dumps(signer)
        self.assertNotIn("actions/checkout", signer_text)
        self.assertNotIn("scripts/", signer_text)
        self.assertIn("dist/*.tar.gz", signer_text)
        self.assertIn("dist/*.crate", signer_text)

        image_builder = jobs["build-images"]
        self.assertEqual(image_builder["environment"], "release")
        self.assertEqual(
            image_builder["permissions"],
            {"contents": "read", "packages": "write"},
        )
        image_builder_text = json.dumps(image_builder)
        self.assertNotIn("id-token", image_builder_text)
        self.assertNotIn("attestations", image_builder_text)
        self.assertNotIn("cosign", image_builder_text.lower())
        steps = image_builder["steps"]
        build_step = next(step for step in steps if step.get("id") == "build")
        self.assertIn("release-candidate-", build_step["with"]["tags"])
        self.assertIn("github.run_id", build_step["with"]["tags"])
        self.assertIn("github.run_attempt", build_step["with"]["tags"])
        self.assertNotIn("github.ref_name", build_step["with"]["tags"])
        self.assertNotIn("if", build_step)
        quarantine_step = next(
            step for step in steps if step["name"] == "Refuse a pre-existing attempt quarantine tag"
        )
        quarantine_script = quarantine_step["run"]
        self.assertIn(
            "Refusing to trust or replace pre-existing quarantine tag",
            quarantine_script,
        )
        self.assertNotIn("exists=true", quarantine_script)
        candidate_image_step = next(step for step in steps if step.get("id") == "candidate-image")
        self.assertEqual(
            candidate_image_step["env"]["BUILD_DIGEST"],
            "${{ steps.build.outputs.digest }}",
        )
        self.assertNotIn("EXISTING_DIGEST", json.dumps(candidate_image_step))
        smoke_step = next(
            step
            for step in steps
            if step["name"] == "Verify manifest platforms and execute both architectures"
        )
        smoke_script = smoke_step["run"]
        self.assertIn("tests/fixtures/consumers/clean", smoke_script)
        self.assertIn("MEGALINTER_CONFIG=/tmp/lint/.mega-linter.yml", smoke_script)
        self.assertIn("mega-linter-report.json", smoke_script)
        self.assertIn('--entrypoint "python3"', smoke_script)

        authorizer = jobs["authorize-images"]
        self.assertEqual(authorizer["environment"], "release")
        self.assertEqual(authorizer["permissions"]["id-token"], "write")
        self.assertEqual(authorizer["permissions"]["attestations"], "write")
        authorizer_text = json.dumps(authorizer)
        self.assertNotIn("actions/checkout", authorizer_text)
        self.assertNotIn("scripts/", authorizer_text)
        self.assertNotIn("release-candidate-", authorizer_text)
        steps = authorizer["steps"]
        evidence_step = next(step for step in steps if step.get("id") == "evidence")
        self.assertIn('imagetools inspect "${image}"', evidence_step["run"])
        step_names = [step["name"] for step in steps]
        sign_index = step_names.index("Sign and verify both candidate digests")
        attest_index = step_names.index("Attest full image release provenance")
        promote_index = step_names.index("Create or verify both write-once version tags")
        self.assertLess(sign_index, promote_index)
        self.assertLess(attest_index, promote_index)
        promotion_script = steps[promote_index]["run"]
        self.assertIn("imagetools create", promotion_script)
        self.assertIn('promote "egolint"', promotion_script)
        self.assertIn('promote "egolint-full"', promotion_script)

        announce_text = json.dumps(jobs["announce"])
        self.assertIn("Final tag for %s resolved", announce_text)

    def test_raw_evidence_is_private_and_suppression_fields_match_contract(
        self,
    ) -> None:
        ownership = json.loads(
            (REPOSITORY_ROOT / ".config/security/scanner-ownership.json").read_text(
                encoding="utf-8"
            )
        )
        suppression_schema = json.loads(
            (REPOSITORY_ROOT / "schemas/suppression.schema.json").read_text(encoding="utf-8")
        )

        self.assertIn("private by default", ownership["evidence"]["publication_rule"])
        self.assertEqual(ownership["evidence"]["currently_publishable_paths"], [])
        self.assertEqual(
            set(ownership["evidence"]["canonical_local_paths"]),
            {
                ".reports/egolint/run.json",
                ".reports/egolint/egolint.sarif",
                ".reports/egolint/debt.json",
                ".reports/egolint/debt.md",
            },
        )
        self.assertEqual(
            set(ownership["evidence"]["private_raw_paths"]),
            {
                ".reports/egolint/linters_logs/",
                ".reports/egolint/mega-linter-report.json",
                ".reports/egolint/mega-linter-report.sarif",
                ".reports/egolint/megalinter-summary.md",
                ".reports/egolint/IDE-config/",
            },
        )
        self.assertEqual(
            set(ownership["suppressions"]["required_fields"]),
            set(suppression_schema["required"]),
        )


if __name__ == "__main__":
    unittest.main()
