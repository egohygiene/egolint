# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPOSITORY_ROOT / "scripts" / "render_image_profiles.py"
SPEC = importlib.util.spec_from_file_location("render_image_profiles", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    message = f"Unable to load image profile renderer from {MODULE_PATH}"
    raise RuntimeError(message)
renderer = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = renderer
SPEC.loader.exec_module(renderer)


class ImageProfileRendererTests(unittest.TestCase):
    def test_render_rebases_assets_without_rewriting_filter_patterns_twice(
        self,
    ) -> None:
        source = """---
EXTENDS:
  - .mega-linter.yml
LINTER_RULES_PATH: .config/lint
SECURITY_RULES: ${GITHUB_WORKSPACE}/.config/security/trivy
MARKDOWN_ARGUMENTS: --ignore-path .markdownlintignore
CHECK: test -f \"node_modules/@cspell/example\"
FILTER: '(^|/)\\.config/lint/'
ALREADY: /opt/egolint/.config/lint
"""

        rendered = renderer.render_profile(source)

        self.assertIn("- .mega-linter.yml", rendered)
        self.assertIn("LINTER_RULES_PATH: /opt/egolint/.config/lint", rendered)
        self.assertIn("SECURITY_RULES: /opt/egolint/.config/security/trivy", rendered)
        self.assertIn("--ignore-path /opt/egolint/.markdownlintignore", rendered)
        self.assertIn('test -f "/opt/egolint/node_modules/@cspell/example"', rendered)
        self.assertIn("FILTER: '(^|/)\\.config/lint/'", rendered)
        self.assertIn("ALREADY: /opt/egolint/.config/lint", rendered)
        self.assertNotIn("/opt/egolint//opt/", rendered)

    def test_directory_render_flattens_fast_profile_without_runtime_extends(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            fast = directory / "fast.yml"
            holistic = directory / "holistic.yml"
            fast.write_text(
                "EXTENDS:\n  - .mega-linter.yml\nRULES: fast\n",
                encoding="utf-8",
            )
            holistic.write_text(
                "RULES: .config/lint\nBASE_ONLY: true\n",
                encoding="utf-8",
            )

            renderer.render_directory(directory)

            rendered_fast = fast.read_text(encoding="utf-8")
            self.assertNotIn("EXTENDS", rendered_fast)
            self.assertIn("RULES: fast", rendered_fast)
            self.assertIn("BASE_ONLY: true", rendered_fast)
            self.assertEqual(
                holistic.read_text(encoding="utf-8"),
                "RULES: /opt/egolint/.config/lint\nBASE_ONLY: true\n",
            )

    def test_flatten_honors_megalinter_append_properties(self) -> None:
        holistic = "LIST:\n  - base\n"
        fast = """EXTENDS: .mega-linter.yml
CONFIG_PROPERTIES_TO_APPEND:
  - LIST
LIST:
  - overlay
"""

        flattened = renderer.flatten_fast_profile(holistic, fast)

        self.assertIn("- base", flattened)
        self.assertIn("- overlay", flattened)

    def test_directory_render_flattens_all_packaged_overlays(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            (directory / "holistic.yml").write_text(
                "BASE: .config/security\n",
                encoding="utf-8",
            )
            for name in ("fast.yml", "security.yml", "dependency-debt.yml"):
                (directory / name).write_text(
                    "EXTENDS: .mega-linter.yml\nPROFILE: " + name + "\n",
                    encoding="utf-8",
                )

            renderer.render_directory(directory)

            for name in ("fast.yml", "security.yml", "dependency-debt.yml"):
                rendered = (directory / name).read_text(encoding="utf-8")
                self.assertNotIn("EXTENDS", rendered)
                self.assertIn("BASE: /opt/egolint/.config/security", rendered)
                self.assertIn(f"PROFILE: {name}", rendered)


if __name__ == "__main__":
    unittest.main()
