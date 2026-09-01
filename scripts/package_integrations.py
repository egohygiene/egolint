#!/usr/bin/env python3

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Validate and deterministically package Egolint consumer integrations."""

# Contract validation benefits from keeping each rejected invariant beside its
# explicit diagnostic instead of hiding the release boundary behind generic helpers.
# ruff: noqa: PLR0912, TRY003

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import tarfile
import tomllib
from typing import Any

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
CONTRACT_PATH = REPOSITORY_ROOT / "integrations" / "contract.json"


class IntegrationContractError(ValueError):
    """Raised when the source integration contract is unsafe or inconsistent."""


def load_json(path: Path) -> dict[str, Any]:
    """Load one JSON object from ``path``."""

    document = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise IntegrationContractError(f"{path} must contain a JSON object")
    return document


def package_version() -> str:
    """Return the Cargo package version that owns integration compatibility."""

    with (REPOSITORY_ROOT / "Cargo.toml").open("rb") as stream:
        return str(tomllib.load(stream)["package"]["version"])


def normalized_source_path(value: str) -> Path:
    """Resolve a safe repository-relative bundle source path."""

    pure_path = PurePosixPath(value)
    if pure_path.is_absolute() or ".." in pure_path.parts or str(pure_path) != value:
        raise IntegrationContractError(f"unsafe bundle path: {value}")
    source = REPOSITORY_ROOT.joinpath(*pure_path.parts)
    if not source.is_file() or source.is_symlink():
        raise IntegrationContractError(f"bundle source must be a regular file: {value}")
    return source


def validate_contract(contract: dict[str, Any], version: str) -> list[tuple[str, bytes]]:
    """Validate cross-surface invariants and return sorted bundle members."""

    if contract.get("schema_version") != 1:
        raise IntegrationContractError("schema_version must equal 1")
    if contract.get("version_source") != "Cargo.toml#package.version":
        raise IntegrationContractError("version_source must remain Cargo.toml#package.version")
    if contract.get("bundle_name") != "egolint-integrations":
        raise IntegrationContractError("bundle_name must equal egolint-integrations")

    surfaces = contract.get("surfaces")
    if not isinstance(surfaces, dict) or set(surfaces) != {
        "github_action",
        "megalinter_adapter",
        "pre_commit",
        "vscode",
    }:
        raise IntegrationContractError("the four supported integration surfaces must be explicit")
    for name, surface in surfaces.items():
        if not isinstance(surface, dict) or surface.get("check_only") is not True:
            raise IntegrationContractError(f"{name} must remain check-only")

    reports = contract.get("canonical_reports")
    if reports != {
        "json": ".reports/egolint/run.json",
        "sarif": ".reports/egolint/egolint.sarif",
    }:
        raise IntegrationContractError("canonical report paths changed without a contract version")

    autofix = contract.get("autofix")
    if not isinstance(autofix, dict) or autofix.get("default") != "disabled":
        raise IntegrationContractError("autofix must remain disabled by default")
    if autofix.get("source_worktree_write_during_preview") is not False:
        raise IntegrationContractError("fix preview must not write the source worktree")

    bundle_files = contract.get("bundle_files")
    if not isinstance(bundle_files, list) or not bundle_files:
        raise IntegrationContractError("bundle_files must be a non-empty array")
    if bundle_files != sorted(set(bundle_files)):
        raise IntegrationContractError("bundle_files must be sorted and unique")

    members: list[tuple[str, bytes]] = []
    checksums: dict[str, str] = {}
    for relative_path in bundle_files:
        if not isinstance(relative_path, str):
            raise IntegrationContractError("bundle paths must be strings")
        source = normalized_source_path(relative_path)
        content = source.read_bytes()
        checksums[relative_path] = hashlib.sha256(content).hexdigest()
        members.append((relative_path, content))

    adapter = load_json(REPOSITORY_ROOT / surfaces["megalinter_adapter"]["contract"])
    if adapter.get("schema_version") != 1:
        raise IntegrationContractError("MegaLinter adapter schema_version must equal 1")
    if adapter.get("image_product") != surfaces["megalinter_adapter"]["image"]:
        raise IntegrationContractError("MegaLinter image identity drifted between contracts")
    if adapter.get("normalization", {}).get("output_json") != reports["json"]:
        raise IntegrationContractError("MegaLinter JSON normalization target drifted")
    if adapter.get("normalization", {}).get("output_sarif") != reports["sarif"]:
        raise IntegrationContractError("MegaLinter SARIF normalization target drifted")

    release_manifest = {
        "schema_version": 1,
        "bundle_version": version,
        "contract": "integrations/contract.json",
        "files": checksums,
    }
    manifest_bytes = json.dumps(release_manifest, indent=2, sort_keys=True).encode("utf-8") + b"\n"
    members.append(("MANIFEST.json", manifest_bytes))
    return sorted(members)


def tar_bytes(members: list[tuple[str, bytes]], version: str, epoch: int) -> bytes:
    """Build one deterministic gzip-compressed tar archive."""

    archive_root = f"egolint-integrations-{version}"
    raw_tar = io.BytesIO()
    with tarfile.open(fileobj=raw_tar, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for relative_path, content in members:
            info = tarfile.TarInfo(f"{archive_root}/{relative_path}")
            info.size = len(content)
            info.mode = 0o755 if relative_path.endswith(".sh") else 0o644
            info.mtime = epoch
            info.uid = 0
            info.gid = 0
            info.uname = ""
            info.gname = ""
            archive.addfile(info, io.BytesIO(content))
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=epoch) as compressed:
        compressed.write(raw_tar.getvalue())
    return output.getvalue()


def main() -> int:
    """Validate the contract and optionally write the release bundle."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate without writing an archive")
    parser.add_argument("--output-directory", type=Path, default=Path("dist"))
    parser.add_argument("--source-date-epoch", type=int, default=0)
    parser.add_argument("--version", default=package_version())
    arguments = parser.parse_args()

    if arguments.version != package_version():
        raise IntegrationContractError("bundle version must equal the Cargo package version")
    if arguments.source_date_epoch < 0:
        raise IntegrationContractError("source-date-epoch must be non-negative")

    members = validate_contract(load_json(CONTRACT_PATH), arguments.version)
    if arguments.check:
        return 0

    output_directory = arguments.output_directory.resolve()
    output_directory.mkdir(parents=True, exist_ok=True)
    archive_name = f"egolint-integrations-{arguments.version}.tar.gz"
    archive_path = output_directory / archive_name
    archive_path.write_bytes(tar_bytes(members, arguments.version, arguments.source_date_epoch))
    print(archive_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
