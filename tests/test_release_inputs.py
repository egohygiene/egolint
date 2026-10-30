# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

from __future__ import annotations

from copy import deepcopy
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

import yaml

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPOSITORY_ROOT / "scripts" / "validate_release_inputs.py"
SPEC = importlib.util.spec_from_file_location("validate_release_inputs", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError
release_inputs = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = release_inputs
SPEC.loader.exec_module(release_inputs)


class ReleaseInputTests(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = release_inputs.load_contract()


    def run_validator(self, contract, *, check_environment=False, environment=None):
        """Execute the real script against a disposable repository-shaped copy."""

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            script = root / "scripts" / "validate_release_inputs.py"
            script.parent.mkdir()
            shutil.copyfile(MODULE_PATH, script)
            contract_path = root / ".config" / "release" / "base-images.v1.json"
            contract_path.parent.mkdir(parents=True)
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            command = [sys.executable, str(script)]
            if check_environment:
                command.append("--check-environment")
            process_environment = {
                key: value
                for key, value in os.environ.items()
                if key not in release_inputs.IMAGE_NAMES
            }
            process_environment.update(environment or {})
            return subprocess.run(
                command, capture_output=True, text=True, check=False, env=process_environment
            )

    def assert_configuration_rejected(self, contract, diagnostic):
        for check_environment in (False, True):
            with self.subTest(check_environment=check_environment):
                result = self.run_validator(contract, check_environment=check_environment)
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn("release-inputs:", result.stderr)
                self.assertIn(diagnostic, result.stderr)
                self.assertNotIn("Traceback", result.stderr)
                self.assertNotIn("must equal reviewed reference", result.stderr)
                self.assertEqual(result.stdout, "")

    def test_cli_retains_valid_contract_and_environment_controls(self):
        result = self.run_validator(self.contract)
        self.assertEqual(result.returncode, 0, result.stderr)
        environment = {
            name: release_inputs.immutable_reference(image)
            for name, image in self.contract["images"].items()
        }
        result = self.run_validator(
            self.contract, check_environment=True, environment=environment
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        for name in release_inputs.IMAGE_NAMES:
            with self.subTest(image=name):
                missing = dict(environment)
                del missing[name]
                result = self.run_validator(
                    self.contract, check_environment=True, environment=missing
                )
                self.assertEqual(result.returncode, 1)
                self.assertIn(f"{name} must equal reviewed reference", result.stderr)
                drifted = dict(environment)
                drifted[name] += "-drift"
                result = self.run_validator(
                    self.contract, check_environment=True, environment=drifted
                )
                self.assertEqual(result.returncode, 1)
                self.assertIn(f"{name} must equal reviewed reference", result.stderr)

    def test_cli_rejects_missing_and_nonobject_records(self):
        for group in ("images", "products"):
            for name in self.contract[group]:
                for value in (None, False, 0, "record", []):
                    with self.subTest(group=group, name=name, value=value):
                        contract = deepcopy(self.contract)
                        contract[group][name] = value
                        self.assert_configuration_rejected(contract, f"{name} must be an object")
                with self.subTest(group=group, name=name, missing=True):
                    contract = deepcopy(self.contract)
                    del contract[group][name]
                    self.assert_configuration_rejected(contract, f"{group} must contain exactly")

    def test_cli_rejects_all_null_records_with_empty_environment(self):
        for group in ("images", "products"):
            self.contract[group] = dict.fromkeys(self.contract[group])
        self.assert_configuration_rejected(self.contract, "must be an object")

    def test_cli_rejects_malformed_nested_image_fields(self):
        malformed = {
            "reference": (None, [], {}, 1),
            "digest": (None, [], {}, 1),
            "media_type": (None, [], {}, 1),
            "platforms": (None, {}, [], [[]], [{}], [None], [1], ["linux/amd64"] * 2),
            "upstream_platform_evidence": (None, [], {}, 1, ""),
        }
        for field, values in malformed.items():
            for value in values:
                with self.subTest(field=field, value=value):
                    contract = deepcopy(self.contract)
                    contract["images"]["MEGALINTER_IMAGE"][field] = value
                    self.assert_configuration_rejected(contract, f"MEGALINTER_IMAGE.{field}")

    def test_cli_rejects_malformed_nested_product_fields(self):
        malformed = {
            "base_images": (
                None, {}, [], [[]], [{}], [None], [1],
                ["NODE_POLICY_IMAGE", "MEGALINTER_IMAGE", "NODE_POLICY_IMAGE"],
            ),
            "platforms": (None, {}, [], [[]], [{}], [None], [1], ["linux/amd64"] * 2),
            "limitation": (None, [], {}, 1, ""),
        }
        for field, values in malformed.items():
            for value in values:
                with self.subTest(field=field, value=value):
                    contract = deepcopy(self.contract)
                    contract["products"]["egolint-full"][field] = value
                    self.assert_configuration_rejected(contract, f"egolint-full.{field}")

    def test_cli_requires_every_record_field_and_rejects_unknown_fields(self):
        for group, name, fields in (
            ("images", "MEGALINTER_IMAGE", ("reference", "digest", "media_type", "platforms")),
            ("products", "egolint-full", ("base_images", "platforms")),
        ):
            for field in fields:
                with self.subTest(group=group, field=field):
                    contract = deepcopy(self.contract)
                    del contract[group][name][field]
                    self.assert_configuration_rejected(contract, name)
            contract = deepcopy(self.contract)
            contract[group][name]["unsupported"] = "value"
            self.assert_configuration_rejected(contract, f"{name} has unsupported fields")

    def test_cli_rejects_invalid_root_and_map_grammar(self):
        for group in ("images", "products"):
            for value in (None, [], "records", 1):
                with self.subTest(group=group, value=value):
                    contract = deepcopy(self.contract)
                    contract[group] = value
                    self.assert_configuration_rejected(contract, f"{group} must contain exactly")
        for value in (None, [], "contract", 1):
            with self.subTest(root=value):
                self.assert_configuration_rejected(value, "root must be an object")
        for field, value in (("schema_version", True), ("reviewed_at", "20260901")):
            with self.subTest(field=field):
                contract = deepcopy(self.contract)
                contract[field] = value
                self.assert_configuration_rejected(contract, field)
        self.contract["unsupported"] = None
        self.assert_configuration_rejected(self.contract, "contract has unsupported fields")

    def test_canonical_contract_is_valid(self) -> None:
        self.assertEqual(release_inputs.validate_contract(self.contract), [])

    def test_environment_must_match_every_reviewed_digest(self) -> None:
        environment = {
            name: release_inputs.immutable_reference(image)
            for name, image in self.contract["images"].items()
        }
        self.assertEqual(release_inputs.validate_environment(self.contract, environment), [])
        environment["MEGALINTER_IMAGE"] = "ghcr.io/oxsecurity/megalinter:v10.0.0"
        findings = release_inputs.validate_environment(self.contract, environment)
        self.assertTrue(any("MEGALINTER_IMAGE must equal" in item for item in findings))

    def test_product_platforms_are_the_base_intersection(self) -> None:
        self.assertEqual(
            self.contract["products"]["egolint"]["platforms"],
            ["linux/amd64", "linux/arm64"],
        )
        self.assertEqual(
            self.contract["products"]["egolint-full"]["platforms"],
            ["linux/amd64"],
        )

    def test_product_platform_overclaim_is_rejected(self) -> None:
        self.contract["products"]["egolint-full"]["platforms"].append("linux/arm64")
        findings = release_inputs.validate_contract(self.contract)
        self.assertTrue(any("egolint-full.platforms must equal" in item for item in findings))

    def test_release_workflow_uses_the_same_product_platforms(self) -> None:
        workflow = yaml.safe_load(
            (REPOSITORY_ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
        )
        matrix = workflow["jobs"]["build-images"]["strategy"]["matrix"]["include"]
        observed = {item["product"]: item["platforms"].split(",") for item in matrix}
        expected = {
            name: product["platforms"] for name, product in self.contract["products"].items()
        }
        self.assertEqual(observed, expected)
        workflow_text = (REPOSITORY_ROOT / ".github" / "workflows" / "release.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("validate_release_inputs.py --check-environment", workflow_text)

    def test_dockerfile_defaults_match_reviewed_tags(self) -> None:
        dockerfile = (REPOSITORY_ROOT / "Dockerfile").read_text(encoding="utf-8")
        full_dockerfile = (REPOSITORY_ROOT / "Dockerfile.full").read_text(encoding="utf-8")
        sources = {
            "RUST_BUILDER_IMAGE": dockerfile,
            "CLI_RUNTIME_IMAGE": dockerfile,
            "NODE_POLICY_IMAGE": full_dockerfile,
            "MEGALINTER_IMAGE": full_dockerfile,
        }
        for name, source in sources.items():
            with self.subTest(image=name):
                reference = self.contract["images"][name]["reference"]
                if reference.startswith("docker.io/library/"):
                    reference = reference.removeprefix("docker.io/library/")
                self.assertIn(f'ARG {name}="{reference}"', source)


if __name__ == "__main__":
    unittest.main()
