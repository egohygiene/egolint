#!/usr/bin/env python3
# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Bind a manual release request to reviewed source and a successful rehearsal."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess  # nosec B404
import sys
import tomllib
from typing import Any, Never

REPOSITORY = "egohygiene/egolint"
WORKFLOW = ".github/workflows/release.yml"
REVISION = re.compile(r"[0-9a-f]{40}\Z")
RUN_ID = re.compile(r"[1-9][0-9]{0,19}\Z")
VERSION = re.compile(
    r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?\Z"
)


class RequestError(ValueError):
    """The requested publication lacks exact local or hosted evidence."""


def reject(message: str) -> Never:
    """Reject evidence while preserving type narrowing at validation boundaries."""
    raise RequestError(message)


def require(condition: bool, message: str) -> None:
    """Reject a request without echoing raw API or environment content."""
    if not condition:
        reject(message)


def git_revision(workspace: Path, ref: str) -> str:
    """Resolve a fixed, validated ref without a shell or network access."""
    executable = shutil.which("git")
    if executable is None:
        reject("Git is required for release source verification")
    # The executable is resolved Git; the operation/ref are fixed or validated, without a shell.
    result = subprocess.run(  # nosec B603  # noqa: S603
        [executable, "rev-parse", "--verify", "--end-of-options", f"{ref}^{{commit}}"],
        cwd=workspace,
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )
    require(result.returncode == 0, "required source, main or version tag is unavailable")
    value = result.stdout.strip()
    require(REVISION.fullmatch(value) is not None, "source identity is not a full Git revision")
    return value


def load_rehearsal(path: Path) -> dict[str, Any]:
    """Read bounded metadata fetched from the authenticated Actions run endpoint."""
    metadata = path.lstat()
    require(
        path.is_file() and not path.is_symlink() and metadata.st_size <= 1024 * 1024,
        "rehearsal metadata must be a bounded regular JSON file",
    )
    value: object = json.loads(path.read_bytes())
    if not isinstance(value, dict):
        reject("rehearsal metadata must be an object")
    return value


def validate_request(
    workspace: Path, environment: dict[str, str], rehearsal: dict[str, Any] | None
) -> dict[str, str]:
    """Validate source identity and prior hosted rehearsal without publishing."""
    require(environment.get("GITHUB_REPOSITORY") == REPOSITORY, "unexpected release repository")
    require(
        environment.get("GITHUB_EVENT_NAME") == "workflow_dispatch",
        "release requests require workflow_dispatch",
    )
    require(
        environment.get("GITHUB_REF") == "refs/heads/main"
        and environment.get("GITHUB_REF_TYPE") == "branch",
        "dispatch the reviewed workflow from main",
    )
    operation = environment.get("REQUESTED_OPERATION", "")
    require(operation in {"rehearse", "publish"}, "unsupported release operation")
    revision = environment.get("REQUESTED_REVISION", "")
    require(REVISION.fullmatch(revision) is not None, "request an exact full source revision")
    require(environment.get("GITHUB_SHA") == revision, "dispatch revision differs from request")
    require(git_revision(workspace, "HEAD") == revision, "checkout differs from requested source")
    require(
        git_revision(workspace, "refs/remotes/origin/main") == revision,
        "requested source is no longer current main",
    )
    manifest = tomllib.loads((workspace / "Cargo.toml").read_text(encoding="utf-8"))
    package = manifest.get("package")
    if not isinstance(package, dict):
        reject("Cargo version authority must contain a package table")
    version = package.get("version")
    if not isinstance(version, str):
        reject("requested version must equal the Cargo version authority")
    require(
        VERSION.fullmatch(version) is not None and environment.get("REQUESTED_VERSION") == version,
        "requested version must equal the Cargo version authority",
    )
    tag = f"v{version}"
    if operation == "publish":
        prior_id = environment.get("REQUESTED_REHEARSAL_RUN", "")
        require(RUN_ID.fullmatch(prior_id) is not None, "publish requires a rehearsal run ID")
        require(prior_id != environment.get("GITHUB_RUN_ID"), "a run cannot approve itself")
        if rehearsal is None:
            reject("publish requires authenticated rehearsal metadata")
        require(
            type(rehearsal.get("id")) is int and rehearsal["id"] == int(prior_id),
            "rehearsal run identity differs from request",
        )
        require(
            isinstance(rehearsal.get("repository"), dict)
            and rehearsal["repository"].get("full_name") == REPOSITORY,
            "rehearsal belongs to another repository",
        )
        require(
            rehearsal.get("event") == "workflow_dispatch"
            and rehearsal.get("path") == WORKFLOW
            and rehearsal.get("head_branch") == "main"
            and rehearsal.get("head_sha") == revision,
            "rehearsal must select this workflow and exact main source",
        )
        require(
            rehearsal.get("status") == "completed" and rehearsal.get("conclusion") == "success",
            "rehearsal has not completed successfully",
        )
        require(
            rehearsal.get("display_title") == f"rehearse {version} @ {revision}",
            "prior run is not the exact requested non-publishing rehearsal",
        )
        require(
            git_revision(workspace, f"refs/tags/{tag}") == revision,
            "version tag differs from rehearsed source",
        )
    else:
        require(
            not environment.get("REQUESTED_REHEARSAL_RUN") and rehearsal is None,
            "rehearsal must not carry publication approval inputs",
        )
    return {"publish": str(operation == "publish").lower(), "version": version, "tag": tag}


def main() -> int:
    """Validate a request and emit bounded non-secret job outputs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, default=Path())
    parser.add_argument("--rehearsal-run", type=Path)
    arguments = parser.parse_args()
    try:
        rehearsal = load_rehearsal(arguments.rehearsal_run) if arguments.rehearsal_run else None
        result = validate_request(arguments.workspace, dict(os.environ), rehearsal)
        if output := os.environ.get("GITHUB_OUTPUT"):
            with Path(output).open("a", encoding="utf-8") as stream:
                stream.writelines(f"{key}={value}\n" for key, value in result.items())
        print(f"Validated manual release request: {result['tag']}; publish={result['publish']}")
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        print(f"release-request: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
