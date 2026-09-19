#!/usr/bin/env python3

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Validate the immutable Aether and Hygiene release-contract source pins."""

from __future__ import annotations

import argparse
from datetime import date
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys
from typing import Any

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_LOCK = REPOSITORY_ROOT / ".config/rules/repository-release-sources.v1.json"
CONTRACT_ID = "egolint.repository-release-sources/v1"
SOURCE_FIELDS = {
    "id",
    "repository",
    "revision",
    "source_path",
    "vendored_path",
    "git_blob_sha1",
    "sha256",
}
EXPECTED_SOURCES = {
    "aether-repository-release-schema": {
        "repository": "egohygiene/aether",
        "source_path": "catalog/schemas/aether.repository-release.v1.schema.json",
        "vendored_path": "vendor/aether/aether.repository-release.v1.schema.json",
    },
    "hygiene-repository-release-profile": {
        "repository": "egohygiene/hygiene",
        "source_path": "catalog/repository-release-policy.json",
        "vendored_path": "vendor/hygiene/repository-release-policy.v1.json",
    },
}
COMMIT_PATTERN = re.compile(r"[0-9a-f]{40}\Z")
SHA256_PATTERN = re.compile(r"[0-9a-f]{64}\Z")


class SourceContractError(ValueError):
    """The release-source lock cannot be loaded as a JSON object."""

    @classmethod
    def root_not_object(cls, path: Path) -> SourceContractError:
        """Build the canonical root-type diagnostic."""

        return cls(f"{path}: root must be an object")


def load_json_object(path: Path) -> dict[str, Any]:
    """Load one UTF-8 JSON object."""

    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise SourceContractError.root_not_object(path)
    return value


def safe_relative_path(value: Any) -> bool:
    """Return whether a value is one normalized repository-relative path."""

    if not isinstance(value, str) or not value or "\\" in value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def git_blob_sha1(payload: bytes) -> str:
    """Return Git's SHA-1 object identity for file bytes."""

    header = f"blob {len(payload)}\0".encode()
    return hashlib.sha1(header + payload, usedforsecurity=False).hexdigest()


def read_vendored_source(
    repository_root: Path,
    source: dict[str, Any],
    findings: list[str],
) -> dict[str, Any] | None:
    """Validate and decode one pinned vendored source."""

    source_id = str(source.get("id", "unknown"))
    raw_path = source.get("vendored_path")
    if not safe_relative_path(raw_path):
        findings.append(f"{source_id}.vendored_path must be normalized and repository-relative")
        return None
    path = repository_root.joinpath(*PurePosixPath(raw_path).parts)
    if not path.is_file() or path.is_symlink():
        findings.append(f"{source_id}.vendored_path must identify one regular non-symlink file")
        return None
    payload = path.read_bytes()
    expected_sha256 = source.get("sha256")
    observed_sha256 = hashlib.sha256(payload).hexdigest()
    if expected_sha256 != observed_sha256:
        findings.append(f"{source_id}.sha256 does not match vendored bytes")
    expected_blob = source.get("git_blob_sha1")
    observed_blob = git_blob_sha1(payload)
    if expected_blob != observed_blob:
        findings.append(f"{source_id}.git_blob_sha1 does not match vendored bytes")
    try:
        value = json.loads(payload)
    except (UnicodeError, json.JSONDecodeError):
        findings.append(f"{source_id}.vendored_path must contain UTF-8 JSON")
        return None
    if not isinstance(value, dict):
        findings.append(f"{source_id}.vendored_path must contain one JSON object")
        return None
    return value


def validate_source_record(source: Any, index: int) -> list[str]:
    """Validate one closed immutable-source record."""

    label = f"sources[{index}]"
    if not isinstance(source, dict):
        return [f"{label} must be an object"]
    findings: list[str] = []
    if set(source) != SOURCE_FIELDS:
        findings.append(f"{label} fields must exactly match the v1 contract")
    source_id = source.get("id")
    expected = EXPECTED_SOURCES.get(source_id)
    if expected is None:
        findings.append(f"{label}.id is not a supported source")
    else:
        for field, value in expected.items():
            if source.get(field) != value:
                findings.append(f"{source_id}.{field} must equal {value}")
    findings.extend(
        f"{label}.{field} must be normalized and repository-relative"
        for field in ("source_path", "vendored_path")
        if not safe_relative_path(source.get(field))
    )
    for field in ("revision", "git_blob_sha1"):
        value = source.get(field)
        if not isinstance(value, str) or COMMIT_PATTERN.fullmatch(value) is None:
            findings.append(f"{label}.{field} must be a full lowercase Git object id")
    digest = source.get("sha256")
    if not isinstance(digest, str) or SHA256_PATTERN.fullmatch(digest) is None:
        findings.append(f"{label}.sha256 must be a lowercase SHA-256 digest")
    return findings


def validate_cross_source_contract(
    sources: dict[str, dict[str, Any]],
    documents: dict[str, dict[str, Any]],
) -> list[str]:
    """Validate the Hygiene-to-Aether compatibility boundary."""

    aether = documents.get("aether-repository-release-schema")
    hygiene = documents.get("hygiene-repository-release-profile")
    if aether is None or hygiene is None:
        return []
    findings: list[str] = []
    aether_id = aether.get("$id")
    aether_version = aether.get("properties", {}).get("schema_version", {}).get("const")
    aether_contract = hygiene.get("aether_contract")
    if hygiene.get("schema") != "egohygiene.repository-release-policy/v1":
        findings.append("Hygiene source must contain repository-release-policy/v1")
    if not isinstance(aether_contract, dict):
        findings.append("Hygiene source must declare its Aether contract")
        return findings
    if aether_contract.get("id") != aether_version:
        findings.append("Hygiene Aether contract id must match the vendored schema")
    if aether_contract.get("schema_url") != aether_id:
        findings.append("Hygiene schema URL must match the vendored Aether schema id")
    aether_source = sources.get("aether-repository-release-schema", {})
    if aether_contract.get("revision") != aether_source.get("revision"):
        findings.append("Hygiene Aether revision must match the selected schema revision")
    release_profile = (
        aether.get("properties", {})
        .get("repository", {})
        .get("properties", {})
        .get("release_profile", {})
        .get("enum")
    )
    if hygiene.get("repository_profiles") != sorted(release_profile or []):
        findings.append("Hygiene repository profiles must match the Aether schema")
    lifecycle = (
        aether.get("properties", {})
        .get("repository", {})
        .get("properties", {})
        .get("lifecycle", {})
        .get("enum")
    )
    if hygiene.get("lifecycles") != lifecycle:
        findings.append("Hygiene lifecycles must match the Aether schema")
    return findings


def validate_lock(lock: dict[str, Any], repository_root: Path = REPOSITORY_ROOT) -> list[str]:
    """Return every structural, digest, and cross-source finding."""

    findings: list[str] = []
    if set(lock) != {"schema_version", "contract", "reviewed_at", "sources"}:
        findings.append("lock fields must exactly match the v1 contract")
    if lock.get("schema_version") != 1:
        findings.append("schema_version must equal 1")
    if lock.get("contract") != CONTRACT_ID:
        findings.append(f"contract must equal {CONTRACT_ID}")
    try:
        date.fromisoformat(str(lock.get("reviewed_at", "")))
    except ValueError:
        findings.append("reviewed_at must be an ISO calendar date")

    source_values = lock.get("sources")
    if not isinstance(source_values, list):
        findings.append("sources must be an array")
        return findings
    for index, source in enumerate(source_values):
        findings.extend(validate_source_record(source, index))
    typed_sources = [source for source in source_values if isinstance(source, dict)]
    source_ids = [source.get("id") for source in typed_sources]
    if set(source_ids) != set(EXPECTED_SOURCES) or len(source_ids) != len(set(source_ids)):
        findings.append("sources must contain each supported source exactly once")

    sources = {
        str(source["id"]): source
        for source in typed_sources
        if source.get("id") in EXPECTED_SOURCES
    }
    documents: dict[str, dict[str, Any]] = {}
    for source_id, source in sources.items():
        document = read_vendored_source(repository_root, source, findings)
        if document is not None:
            documents[source_id] = document
    findings.extend(validate_cross_source_contract(sources, documents))
    return sorted(set(findings))


def main() -> int:
    """Validate the checked-in release-source selection without network access."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lock", type=Path, default=DEFAULT_LOCK)
    arguments = parser.parse_args()
    try:
        lock = load_json_object(arguments.lock)
        findings = validate_lock(lock)
    except (OSError, UnicodeError, json.JSONDecodeError, SourceContractError) as error:
        print(f"repository-release-sources: {error}", file=sys.stderr)
        return 2
    if findings:
        for finding in findings:
            print(f"repository-release-sources: {finding}", file=sys.stderr)
        return 1
    print("Validated immutable Aether and Hygiene repository-release sources.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
