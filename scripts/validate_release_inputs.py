#!/usr/bin/env python3
# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Validate the reviewed release base-image and product-platform contract."""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import re
import sys
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
DATE_PATTERN = re.compile(r"[0-9]{4}-[0-9]{2}-[0-9]{2}\Z")
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


def validate_fields(
    name: str, record: Mapping[str, Any], required: set[str], optional: set[str]
) -> list[str]:
    """Require complete records without silently accepting unsupported fields."""

    findings = []
    if missing := required - record.keys():
        findings.append(f"{name} is missing required fields: {', '.join(sorted(missing))}")
    if unsupported := record.keys() - required - optional:
        findings.append(f"{name} has unsupported fields: {', '.join(sorted(unsupported))}")
    findings.extend(
        f"{name}.{field} must be a nonempty string"
        for field in sorted(optional & record.keys())
        if not isinstance(record[field], str) or not record[field].strip()
    )
    return findings


def valid_platforms(value: Any) -> bool:
    """Check element types before applying set operations to platform lists."""

    return (
        isinstance(value, list)
        and bool(value)
        and all(
            isinstance(item, str) and PLATFORM_PATTERN.fullmatch(item) is not None for item in value
        )
        and len(value) == len(set(value))
    )


def validate_image(name: str, image: Any) -> list[str]:
    """Validate one reviewed base-image record."""

    if not isinstance(image, dict):
        return [f"{name} must be an object"]
    findings = validate_fields(
        name,
        image,
        {"reference", "digest", "media_type", "platforms"},
        {"upstream_platform_evidence"},
    )
    reference = image.get("reference")
    digest = image.get("digest")
    media_type = image.get("media_type")
    platforms = image.get("platforms")
    if not isinstance(reference, str) or REFERENCE_PATTERN.fullmatch(reference) is None:
        findings.append(f"{name}.reference must be a tagged registry reference")
    if not isinstance(digest, str) or DIGEST_PATTERN.fullmatch(digest) is None:
        findings.append(f"{name}.digest must be a lowercase SHA-256 digest")
    if not isinstance(media_type, str) or media_type not in MEDIA_TYPES:
        findings.append(f"{name}.media_type is not an approved manifest media type")
    if not valid_platforms(platforms):
        findings.append(f"{name}.platforms must contain unique supported Linux platforms")
    elif (
        media_type == "application/vnd.docker.distribution.manifest.v2+json" and len(platforms) != 1
    ):
        findings.append(f"{name}.platforms must contain one platform for a single manifest")
    return findings


def validate_product(
    product_name: str,
    expected_bases: set[str],
    product: Any,
) -> list[str]:
    """Validate one product's complete typed record before resolving bases."""

    if not isinstance(product, dict):
        return [f"{product_name} must be an object"]
    findings = validate_fields(product_name, product, {"base_images", "platforms"}, {"limitation"})
    bases = product.get("base_images")
    platforms = product.get("platforms")
    if (
        not isinstance(bases, list)
        or not all(isinstance(item, str) for item in bases)
        or len(bases) != len(expected_bases)
        or set(bases) != expected_bases
    ):
        findings.append(
            f"{product_name}.base_images must contain exactly: {', '.join(sorted(expected_bases))}"
        )
    if not valid_platforms(platforms):
        findings.append(f"{product_name}.platforms must contain unique supported Linux platforms")
    return findings


def valid_review_date(value: Any) -> bool:
    """Require the reviewed calendar date's exact supported spelling and value."""

    if not isinstance(value, str) or DATE_PATTERN.fullmatch(value) is None:
        return False
    try:
        date.fromisoformat(value)
    except ValueError:
        return False
    return True


def validate_contract(contract: Mapping[str, Any]) -> list[str]:
    """Return every structural or semantic contract finding."""

    findings = validate_fields(
        "contract", contract, {"schema_version", "reviewed_at", "images", "products"}, set()
    )
    if type(contract.get("schema_version")) is not int or contract["schema_version"] != 1:
        findings.append("schema_version must equal 1")
    if not valid_review_date(contract.get("reviewed_at")):
        findings.append("reviewed_at must be an ISO calendar date")

    images = contract.get("images")
    if not isinstance(images, dict) or set(images) != IMAGE_NAMES:
        findings.append(f"images must contain exactly: {', '.join(sorted(IMAGE_NAMES))}")
        images = {}
    for name in sorted(IMAGE_NAMES):
        findings.extend(validate_image(name, images.get(name)))

    products = contract.get("products")
    if not isinstance(products, dict) or set(products) != set(PRODUCT_BASES):
        findings.append("products must contain exactly egolint and egolint-full")
        products = {}
    for product_name, expected_bases in PRODUCT_BASES.items():
        findings.extend(validate_product(product_name, expected_bases, products.get(product_name)))
    if findings:
        return findings
    for product_name, product in products.items():
        available_platforms = set.intersection(
            *(set(images[name]["platforms"]) for name in product["base_images"])
        )
        if set(product["platforms"]) != available_platforms:
            findings.append(
                f"{product_name}.platforms must equal its base-image platform intersection: "
                f"{', '.join(sorted(available_platforms))}"
            )
    return findings


def validate_environment(contract: Mapping[str, Any], environment: Mapping[str, str]) -> list[str]:
    """Require release variables to equal the checked-in immutable references."""

    findings = validate_contract(contract)
    if findings:
        return findings
    images = contract["images"]
    for name in sorted(IMAGE_NAMES):
        image = images.get(name)
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
    try:
        contract = load_contract()
    except (OSError, ValueError) as error:
        print(f"release-inputs: {error}", file=sys.stderr)
        return 1
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
