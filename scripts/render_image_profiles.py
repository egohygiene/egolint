#!/usr/bin/env python3

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Render source MegaLinter profiles for the standalone full image."""

from __future__ import annotations

import argparse
from pathlib import Path
import re

import yaml

IMAGE_ROOT = "/opt/egolint"
POLICY_PATH = re.compile(r"(?<![/\\])\.config/(lint|security)(?=/|\s|$)")


def render_profile(contents: str) -> str:
    """Return an image-safe profile while preserving escaped filter patterns."""

    rendered = contents.replace(
        "${GITHUB_WORKSPACE}/.config/",
        f"{IMAGE_ROOT}/.config/",
    )
    rendered = POLICY_PATH.sub(
        rf"{IMAGE_ROOT}/.config/\1",
        rendered,
    )
    rendered = rendered.replace(
        "--ignore-path .markdownlintignore",
        f"--ignore-path {IMAGE_ROOT}/.markdownlintignore",
    )
    rendered = rendered.replace(
        '"node_modules/@',
        f'"{IMAGE_ROOT}/node_modules/@',
    )
    if f"{IMAGE_ROOT}/{IMAGE_ROOT}" in rendered or f"{IMAGE_ROOT}//opt/" in rendered:
        message = "profile rendering produced a duplicated image prefix"
        raise ValueError(message)
    return rendered


def parse_mapping(contents: str, *, profile_name: str) -> dict[str, object]:
    """Decode one profile and require a top-level mapping."""

    value = yaml.safe_load(contents)
    if not isinstance(value, dict):
        message = f"{profile_name} must contain one top-level YAML mapping"
        raise TypeError(message)
    return value


def flatten_fast_profile(holistic: str, fast: str) -> str:
    """Apply MegaLinter's v10 shallow EXTENDS merge at image-build time."""

    base = parse_mapping(holistic, profile_name="holistic.yml")
    overlay = parse_mapping(fast, profile_name="fast.yml")
    extends = overlay.pop("EXTENDS", None)
    if extends not in (".mega-linter.yml", [".mega-linter.yml"]):
        message = "fast.yml must extend only the source holistic profile"
        raise ValueError(message)

    append_properties = overlay.get("CONFIG_PROPERTIES_TO_APPEND", [])
    if not isinstance(append_properties, list) or not all(
        isinstance(key, str) for key in append_properties
    ):
        message = "CONFIG_PROPERTIES_TO_APPEND must be a list of strings"
        raise ValueError(message)

    flattened = dict(base)
    for key, value in overlay.items():
        if (
            key in append_properties
            and isinstance(flattened.get(key), list)
            and isinstance(value, list)
        ):
            flattened[key] = [*flattened[key], *value]
        else:
            flattened[key] = value

    return "---\n" + yaml.safe_dump(flattened, sort_keys=False)


def render_directory(directory: Path) -> None:
    """Render and flatten the image profiles in ``directory`` in place."""

    holistic_path = directory / "holistic.yml"
    fast_path = directory / "fast.yml"
    holistic = render_profile(holistic_path.read_text(encoding="utf-8"))
    fast = render_profile(fast_path.read_text(encoding="utf-8"))
    holistic_path.write_text(holistic, encoding="utf-8")
    fast_path.write_text(flatten_fast_profile(holistic, fast), encoding="utf-8")


def parse_arguments() -> argparse.Namespace:
    """Parse command-line arguments."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--directory",
        type=Path,
        required=True,
        help="Directory containing fast.yml and holistic.yml.",
    )
    return parser.parse_args()


def main() -> int:
    """Render the requested directory."""

    options = parse_arguments()
    if not options.directory.is_dir():
        message = f"profile directory does not exist: {options.directory}"
        raise SystemExit(message)
    render_directory(options.directory)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
