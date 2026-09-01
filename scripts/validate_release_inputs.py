#!/usr/bin/env python3
# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Validate the reviewed release base-image and product-platform contract."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from datetime import date
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Mapping

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
CONTRACT_PATH = REPOSITORY_ROOT / ".config" / "release" / "base-images.v1.json"
IMAGE_NAMES = {
    "RUST_BUILDER_IMAGE",
    "CLI_RUNTIME_IMAGE",
    "NODE_POLICY_IMAGE",
    "MEGALINTER_IMAGE",
}
PRODUCT_BASES = {
    "egolint": {"RUST_BUILDER_IMAGE", "CLI_RUNTIME_IMAGE"},
    "egolint-full": {"NODE_POLICY_IMAGE", "MEGALINTER_IMAGE"},
}
DIGEST_PATTERN = re.compile(r"sha256:[0-9a-f]{64}\Z")
REFERENCE_PATTERN = re.compile(r"[a-z0-9][a-z0-9._/:+-]*:[A-Za-z0-9._-]+\Z")
PLATFORM_PATTERN = re.compile(r"linux/(?:amd64|arm64)\Z")
MEDIA_TYPES = {
    "application/vnd.oci.image.index.v1+json",
    "application/vnd.docker.distribution.manifest.v2+json",
}


class ContractError(ValueError):
    """The checked-in release input contract is incomplete or inconsistent."""

    @classmethod
    def root_not_object(cls, path: Path) -> ContractError:
        """Build the canonical root-type diagnostic."""

        return cls(f"{path}: root must be an object")


def load_contract(path: Path = CONTRACT_PATH) -> dict[str, Any]:
    """Load the release input contract as one JSON object."""

    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ContractError.root_not_object(path)
    return value


def immutable_reference(image: Mapping[str, Any]) -> str:
    """Return the exact digest-qualified reference for a reviewed image."""

    return f"{image['reference']}@{image['digest']}"


def validate_image(name: str, image: Any) -> list[str]:
    """Validate one reviewed base-image record."""

    findings: list[str] = []
    if not isinstance(image, dict):
        return findings
    reference = image.get("reference")
    digest = image.get("digest")
    media_type = image.get("media_type")
    platforms = image.get("platforms")
    if not isinstance(reference, str) or REFERENCE_PATTERN.fullmatch(reference) is None:
        findings.append(f"{name}.reference must be a tagged registry reference")
    if not isinstance(digest, str) or DIGEST_PATTERN.fullmatch(digest) is None:
        findings.append(f"{name}.digest must be a lowercase SHA-256 digest")
    if media_type not in MEDIA_TYPES:
        findings.append(f"{name}.media_type is not an approved manifest media type")
    if (
        not isinstance(platforms, list)
        or not platforms
        or len(platforms) != len(set(platforms))
        or any(
            not isinstance(item, str) or PLATFORM_PATTERN.fullmatch(item) is None
            for item in platforms
        )
    ):
        findings.append(
            f"{name}.platforms must contain unique supported Linux platforms"
        )
    elif (
        media_type == "application/vnd.docker.distribution.manifest.v2+json"
        and len(platforms) != 1
    ):
        findings.append(
            f"{name}.platforms must contain one platform for a single manifest"
        )
    return findings


def validate_product(
    product_name: str,
    expected_bases: set[str],
    product: Any,
    images: Mapping[str, Any],
) -> list[str]:
    """Validate one product against the intersection of its base platforms."""

    if not isinstance(product, dict):
        return []
    findings: list[str] = []
    bases = product.get("base_images")
    platforms = product.get("platforms")
    if not isinstance(bases, list) or set(bases) != expected_bases:
        findings.append(
            f"{product_name}.base_images must contain exactly: "
            f"{', '.join(sorted(expected_bases))}"
        )
        return findings
    if (
        not isinstance(platforms, list)
        or not platforms
        or len(platforms) != len(set(platforms))
        or any(
            not isinstance(item, str) or PLATFORM_PATTERN.fullmatch(item) is None
            for item in platforms
        )
    ):
        findings.append(
            f"{product_name}.platforms must contain unique supported Linux platforms"
        )
        return findings
    base_platforms = []
    for name in bases:
        image = images.get(name)
        base_platforms.append(
            set(image.get("platforms", [])) if isinstance(image, dict) else set()
        )
    available_platforms = set.intersection(*base_platforms)
    if set(platforms) != available_platforms:
        findings.append(
            f"{product_name}.platforms must equal its base-image platform intersection: "
            f"{', '.join(sorted(available_platforms))}"
        )
    return findings


def validate_contract(contract: Mapping[str, Any]) -> list[str]:
    """Return every structural or semantic contract finding."""

    findings: list[str] = []
    if contract.get("schema_version") != 1:
        findings.append("schema_version must equal 1")
    try:
        date.fromisoformat(str(contract.get("reviewed_at", "")))
    except ValueError:
        findings.append("reviewed_at must be an ISO calendar date")

    images = contract.get("images")
    if not isinstance(images, dict) or set(images) != IMAGE_NAMES:
        findings.append(
            f"images must contain exactly: {', '.join(sorted(IMAGE_NAMES))}"
        )
        images = {}
    for name in sorted(IMAGE_NAMES):
        findings.extend(validate_image(name, images.get(name)))

    products = contract.get("products")
    if not isinstance(products, dict) or set(products) != set(PRODUCT_BASES):
        findings.append("products must contain exactly egolint and egolint-full")
        products = {}
    for product_name, expected_bases in PRODUCT_BASES.items():
        findings.extend(
            validate_product(
                product_name, expected_bases, products.get(product_name), images
            )
        )
    return findings


def validate_environment(
    contract: Mapping[str, Any], environment: Mapping[str, str]
) -> list[str]:
    """Require release variables to equal the checked-in immutable references."""

    findings = validate_contract(contract)
    images = contract.get("images", {})
    if not isinstance(images, dict):
        return findings
    for name in sorted(IMAGE_NAMES):
        image = images.get(name)
        if (
            not isinstance(image, dict)
            or "reference" not in image
            or "digest" not in image
        ):
            continue
        expected = immutable_reference(image)
        observed = environment.get(name, "")
        if observed != expected:
            findings.append(f"{name} must equal reviewed reference {expected}")
    return findings


def main() -> int:
    """Validate the canonical contract and optionally the current environment."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check-environment",
        action="store_true",
        help="also require release image environment variables to match the contract",
    )
    arguments = parser.parse_args()
    contract = load_contract()
    findings = (
        validate_environment(contract, os.environ)
        if arguments.check_environment
        else validate_contract(contract)
    )
    if findings:
        for finding in findings:
            print(f"release-inputs: {finding}", file=sys.stderr)
        return 1
    print("Validated reviewed release base images and product platforms.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
