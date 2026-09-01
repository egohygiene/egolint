# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import unittest

import yaml

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPOSITORY_ROOT / "scripts" / "validate_release_inputs.py"
SPEC = importlib.util.spec_from_file_location("validate_release_inputs", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load release input validator from {MODULE_PATH}")
release_inputs = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = release_inputs
SPEC.loader.exec_module(release_inputs)


class ReleaseInputTests(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = release_inputs.load_contract()

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
            (REPOSITORY_ROOT / ".github" / "workflows" / "release.yml").read_text(
                encoding="utf-8"
            )
        )
        matrix = workflow["jobs"]["build-images"]["strategy"]["matrix"]["include"]
        observed = {
            item["product"]: item["platforms"].split(",")
            for item in matrix
        }
        expected = {
            name: product["platforms"]
            for name, product in self.contract["products"].items()
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
