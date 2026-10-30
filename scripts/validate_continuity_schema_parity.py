# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Check the native parity matrix independently against the exact Aether schema.

Requires jsonschema with its registered date-time and URI format dependencies.
Native outcomes are exercised by tests/repository_continuity_pipeline.rs.
"""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker
import yaml

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    """Assert actual format registration, digest, and every paired expectation."""
    schema_bytes = (ROOT / "vendor/aether/aether.repository-continuity.v1.schema.json").read_bytes()
    matrix = json.loads(
        (ROOT / "tests/fixtures/repository-continuity/schema-parity.json").read_text()
    )
    if hashlib.sha256(schema_bytes).hexdigest() != matrix["schema_sha256"]:
        message = "bundled schema digest differs from the paired fixture selection"
        raise ValueError(message)
    checker = FormatChecker()
    if "date-time" not in checker.checkers:
        message = "install the date-time format dependency"
        raise RuntimeError(message)
    if "uri" not in checker.checkers:
        message = "install the URI format dependency"
        raise RuntimeError(message)
    if checker.conforms("2026-09-08T1:00:00Z", "date-time"):
        message = "date-time format validation must reject unpadded clock fields"
        raise RuntimeError(message)
    schema = json.loads(schema_bytes)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=checker)
    fixture = (ROOT / "tests/fixtures/repository-continuity/valid-public.md").read_text()
    front = fixture.removeprefix("---\n").split("\n---\n", 1)[0]
    control = yaml.safe_load(front)
    control["state"]["base"]["revision"] = "unborn"
    for case in matrix["cases"]:
        metadata = copy.deepcopy(control)
        for mutation in case["changes"]:
            fields = mutation["path"].split(".")
            parent = metadata
            for field in fields[:-1]:
                parent = parent[int(field)] if isinstance(parent, list) else parent[field]
            if mutation["op"] == "yaml-number":
                parent[fields[-1]] = yaml.safe_load(mutation["value"])
            elif mutation["op"] == "remove":
                del parent[fields[-1]]
            else:
                parent[fields[-1]] = mutation["value"]
        actual = validator.is_valid(metadata)
        if actual != case["valid"]:
            message = f"{case['name']}: {list(validator.iter_errors(metadata))}"
            raise ValueError(message)
    print(f"{len(matrix['cases'])} paired cases match Draft202012Validator and registered formats")


if __name__ == "__main__":
    main()
