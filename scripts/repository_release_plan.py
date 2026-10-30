#!/usr/bin/env python3
# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Summarize local release source facts in a draft human review handoff."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys
import tomllib
from typing import Any


class PlanError(ValueError):
    """Local source facts cannot support a review handoff."""

    @classmethod
    def invalid(cls, field: str, expected: str) -> PlanError:
        """Build a controlled source-shape diagnostic."""

        return cls(f"{field} must be {expected}")

    @classmethod
    def root_not_object(cls) -> PlanError:
        """Build the declaration root-type diagnostic."""

        return cls("release declaration must be an object")

    @classmethod
    def unsupported_authority(cls) -> PlanError:
        """Keep this local source planner limited to Cargo authority."""

        return cls("version authority must be cargo-manifest for this source planner")


def object_field(record: dict[str, Any], key: str) -> dict[str, Any]:
    """Read one object without coercing malformed metadata."""

    value = record.get(key)
    if not isinstance(value, dict):
        raise PlanError.invalid(key, "an object")
    return value


def text_field(record: dict[str, Any], key: str) -> str:
    """Read one nonempty scalar for the bounded source summary."""

    value = record.get(key)
    if not isinstance(value, str) or not value.strip() or not value.isprintable():
        raise PlanError.invalid(key, "nonempty printable text")
    return value


def source_bytes(root: Path, name: str) -> bytes:
    """Read a regular source inside the selected repository."""

    relative = PurePosixPath(name)
    if relative.is_absolute() or ".." in relative.parts or "\\" in name:
        raise PlanError.invalid(name, "a safe repository source path")
    path = root.joinpath(*relative.parts)
    path.resolve(strict=True).relative_to(root)
    if path.is_symlink() or not path.is_file():
        raise PlanError.invalid(name, "a regular repository file")
    return path.read_bytes()


def render_plan(workspace: Path) -> str:
    """Read source facts without selecting a version or validating conformance."""

    root = workspace.resolve(strict=True)
    sources = {".egohygiene/release.json": source_bytes(root, ".egohygiene/release.json")}
    declaration = json.loads(sources[".egohygiene/release.json"])
    if not isinstance(declaration, dict):
        raise PlanError.root_not_object()
    repository = object_field(declaration, "repository")
    release = object_field(declaration, "release")
    changelog = object_field(declaration, "changelog")
    automation = object_field(declaration, "automation")
    github = object_field(automation, "github")
    components = declaration.get("components")
    if (
        not isinstance(components, list)
        or len(components) != 1
        or not isinstance(components[0], dict)
    ):
        raise PlanError.invalid(
            "components", "one declared Cargo component for this source planner"
        )
    authority = object_field(components[0], "version_authority")
    if text_field(authority, "kind") != "cargo-manifest":
        raise PlanError.unsupported_authority()
    version_path = text_field(authority, "path")
    changelog_path = text_field(changelog, "path")
    workflow_path = text_field(github, "workflow_path")
    for name in (
        version_path,
        changelog_path,
        text_field(automation, "taskfile_path"),
        workflow_path,
    ):
        sources[name] = source_bytes(root, name)
    manifest = tomllib.loads(sources[version_path].decode("utf-8"))
    version = text_field(object_field(manifest, "package"), "version")
    changelog_text = sources[changelog_path].decode("utf-8")
    headings = list(re.finditer(r"(?m)^## \[Unreleased\][ \t]*$", changelog_text))
    if len(headings) != 1:
        raise PlanError.invalid("changelog", "a document with exactly one ## [Unreleased] section")
    remainder = changelog_text[headings[0].end() :]
    unreleased = re.split(r"(?m)^## \[", remainder, maxsplit=1)[0].strip()
    lines = [
        "# Draft release source handoff",
        "",
        "**Pending maintainer version and changelog decision.**",
        "",
        f"- Repository: `{text_field(repository, 'id')}`",
        f"- Profile: `{text_field(repository, 'release_profile')}`",
        f"- Lifecycle: `{text_field(repository, 'lifecycle')}`",
        f"- Declared release state: `{text_field(release, 'state')}`",
        f"- Current version authority: `{version_path}` → `{version}`",
        "",
        "This source summary selects no bump or release tag. Review the version authority and",
        "Unreleased changes together in a release PR. Native conformance, artifact verification,",
        "hosted qualification and publication readiness require their own evidence.",
        "",
        "## Current handoff",
        "",
    ]
    workflow_text = sources[workflow_path].decode("utf-8")
    if "Record non-publishing rehearsal result" in workflow_text:
        lines += [
            "The current `release:publish` handoff dispatches the existing non-publishing rehearsal.",
            "The tag-driven publisher remains unchanged. Its manual-workflow check stays failed/advisory",
            "until the separately reviewed migration under issue #5; exit 0 does not mean conformance.",
        ]
    else:
        lines += [
            "Inspect native source validation before treating any handoff as publication-ready."
        ]
    lines += ["", "## Unreleased source text", "", unreleased or "No authored changes recorded."]
    lines += ["", "## Source SHA-256 identities", ""]
    lines += [
        f"- `{name}`: `{hashlib.sha256(payload).hexdigest()}`"
        for name, payload in sorted(sources.items())
    ]
    lines += [
        "",
        "## Review decisions still required",
        "",
        "- Review the chosen version and dated changelog promotion through a separate release PR.",
        "- Inspect `.reports/egolint/repository-release.json` and retain every failed/unavailable check.",
        "- Qualify the exact candidate locally and on the supported hosted/platform boundaries.",
        "- Obtain maintainer acceptance before any separately authorized publication handoff.",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    """Print a plan or exclusively create the caller-selected preparation file."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, default=Path())
    parser.add_argument(
        "--output", type=Path, help="new Markdown preparation file; existing paths are refused"
    )
    arguments = parser.parse_args()
    try:
        plan = render_plan(arguments.workspace)
        if arguments.output is None:
            print(plan, end="")
        else:
            arguments.output.parent.mkdir(parents=True, exist_ok=True)
            with arguments.output.open("x", encoding="utf-8") as stream:
                stream.write(plan)
            print("Created draft release source handoff; maintainer decisions remain pending.")
    except (OSError, ValueError) as error:
        print(f"release-plan: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
