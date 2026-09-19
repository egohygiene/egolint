#!/usr/bin/env python3

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Select and launch the optional JSONSkooma schema capability."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import subprocess  # nosec B404
import time
from typing import Any

ADAPTER_VERSION = "1.0.0"
VALIDATOR_VERSION = "0.2.7"
DEFAULT_CONFIG = ".egolint/json-skooma.json"
DEFAULT_REPORT = ".reports/egolint/complementary/json-skooma/latest.json"
RUBY_PROJECT_MARKERS = ("Gemfile", ".ruby-version")
MAX_MAPPINGS = 128
MAX_INSTANCES_PER_MAPPING = 256


class ConfigurationError(ValueError):
    """Raised when the repository-owned mapping contract is invalid."""


@dataclass(frozen=True)
class Applicability:
    """Resolved capability state before Ruby is initialized."""

    status: str
    reason: str
    config_path: Path | None
    config: dict[str, Any] | None
    ruby_markers: tuple[str, ...]


def is_within(root: Path, candidate: Path) -> bool:
    """Return whether ``candidate`` is contained by ``root``."""

    try:
        candidate.relative_to(root)
    except ValueError:
        return False
    return True


def resolve_input_path(workspace: Path, raw_path: str, *, must_exist: bool = True) -> Path:
    """Resolve a repository-relative input without allowing escapes or symlinks."""

    candidate = Path(raw_path)
    if candidate.is_absolute() or ".." in candidate.parts:
        raise ConfigurationError(f"path must be repository-relative: {raw_path}")
    joined = workspace / candidate
    try:
        resolved = joined.resolve(strict=must_exist)
    except OSError as error:
        raise ConfigurationError(f"input does not exist: {raw_path}") from error
    if not is_within(workspace, resolved):
        raise ConfigurationError(f"path escapes the workspace: {raw_path}")
    if must_exist and not resolved.is_file():
        raise ConfigurationError(f"input must be a regular file: {raw_path}")
    return resolved


def ruby_project_markers(workspace: Path) -> tuple[str, ...]:
    """Return bounded root-level Ruby project evidence."""

    markers = [name for name in RUBY_PROJECT_MARKERS if (workspace / name).is_file()]
    markers.extend(path.name for path in sorted(workspace.glob("*.gemspec")) if path.is_file())
    return tuple(sorted(set(markers)))


def load_config(path: Path) -> dict[str, Any]:
    """Load the closed repository-owned JSON mapping document."""

    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ConfigurationError("JSONSkooma mapping must be valid UTF-8 JSON") from error
    if not isinstance(value, dict):
        raise ConfigurationError("JSONSkooma mapping must contain one object")
    if value.get("schema_version") != 1:
        raise ConfigurationError("JSONSkooma mapping schema_version must equal 1")
    allowed_keys = {"schema_version", "mode", "mappings"}
    unexpected = sorted(set(value) - allowed_keys)
    if unexpected:
        raise ConfigurationError(f"unsupported JSONSkooma mapping keys: {', '.join(unexpected)}")
    mode = value.get("mode", "auto")
    if mode not in {"auto", "enabled", "disabled"}:
        raise ConfigurationError("JSONSkooma mode must be auto, enabled, or disabled")
    value["mode"] = mode
    return value


def validate_mappings(workspace: Path, config: dict[str, Any]) -> None:
    """Validate bounded schema-to-instance mappings before starting Ruby."""

    mappings = config.get("mappings")
    if not isinstance(mappings, list) or not mappings:
        raise ConfigurationError("an applicable JSONSkooma capability requires mappings")
    if len(mappings) > MAX_MAPPINGS:
        raise ConfigurationError(f"JSONSkooma mappings exceed the {MAX_MAPPINGS}-entry limit")
    for index, mapping in enumerate(mappings):
        if not isinstance(mapping, dict) or set(mapping) != {"schema", "instances"}:
            raise ConfigurationError(f"mapping {index} must contain only schema and instances")
        schema = mapping.get("schema")
        instances = mapping.get("instances")
        if not isinstance(schema, str) or not schema:
            raise ConfigurationError(f"mapping {index} schema must be a non-empty path")
        if (
            not isinstance(instances, list)
            or not instances
            or not all(isinstance(path, str) and path for path in instances)
        ):
            raise ConfigurationError(f"mapping {index} instances must contain paths")
        if len(instances) > MAX_INSTANCES_PER_MAPPING:
            raise ConfigurationError(
                f"mapping {index} exceeds the {MAX_INSTANCES_PER_MAPPING}-instance limit"
            )
        resolve_input_path(workspace, schema)
        for instance in instances:
            resolve_input_path(workspace, instance)


def evaluate_applicability(workspace: Path, config_name: str = DEFAULT_CONFIG) -> Applicability:
    """Resolve auto/override behavior without importing or launching Ruby."""

    workspace = workspace.resolve(strict=True)
    config_candidate = workspace / config_name
    if not config_candidate.exists():
        return Applicability(
            "not_applicable",
            "schema-mapping-absent",
            None,
            None,
            ruby_project_markers(workspace),
        )

    config_path = resolve_input_path(workspace, config_name)
    config = load_config(config_path)
    markers = ruby_project_markers(workspace)
    if config["mode"] == "disabled":
        return Applicability(
            "not_applicable",
            "explicitly-disabled",
            config_path,
            config,
            markers,
        )
    if config["mode"] == "auto" and not markers:
        return Applicability(
            "not_applicable",
            "ruby-project-marker-absent",
            config_path,
            config,
            markers,
        )

    validate_mappings(workspace, config)
    reason = "explicitly-enabled" if config["mode"] == "enabled" else "ruby-schema-mapping"
    return Applicability("applicable", reason, config_path, config, markers)


def base_report(applicability: Applicability, duration_seconds: float) -> dict[str, Any]:
    """Create shared adapter evidence for skipped and unavailable states."""

    return {
        "schema_version": 1,
        "tool_id": "EGOLINT_JSON_SKOOMA",
        "adapter_version": ADAPTER_VERSION,
        "validator": {"name": "json_skooma", "version": VALIDATOR_VERSION},
        "status": applicability.status,
        "reason": applicability.reason,
        "ruby_markers": list(applicability.ruby_markers),
        "duration_seconds": round(duration_seconds, 6),
        "network": "disabled-by-contract",
        "mappings": [],
    }


def write_report(path: str, payload: dict[str, Any]) -> None:
    """Write stable JSON evidence to a file or standard output."""

    rendered = json.dumps(payload, indent=2, sort_keys=False) + "\n"
    if path == "-":
        print(rendered, end="")
        return
    destination = Path(path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(rendered, encoding="utf-8")


def parse_arguments() -> argparse.Namespace:
    """Parse the adapter launcher command line."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, default=Path())
    parser.add_argument("--config", default=DEFAULT_CONFIG)
    parser.add_argument("--report", default=DEFAULT_REPORT)
    parser.add_argument("--ruby-executable", default="ruby")
    parser.add_argument(
        "--ruby-adapter",
        type=Path,
        default=Path(__file__).with_name("json_skooma_adapter.rb"),
    )
    parser.add_argument("--timeout-seconds", type=int, default=60)
    parser.add_argument("--version", action="store_true")
    return parser.parse_args()


def main() -> int:
    """Resolve applicability and execute the pinned Ruby adapter when needed."""

    options = parse_arguments()
    if options.version:
        print(f"egolint-json-skooma {ADAPTER_VERSION} (json_skooma {VALIDATOR_VERSION})")
        return 0
    started_at = time.monotonic()
    try:
        workspace = options.workspace.resolve(strict=True)
        applicability = evaluate_applicability(workspace, options.config)
    except (ConfigurationError, OSError) as error:
        payload = {
            "schema_version": 1,
            "tool_id": "EGOLINT_JSON_SKOOMA",
            "adapter_version": ADAPTER_VERSION,
            "validator": {"name": "json_skooma", "version": VALIDATOR_VERSION},
            "status": "invalid_configuration",
            "reason": str(error),
            "duration_seconds": round(time.monotonic() - started_at, 6),
            "network": "disabled-by-contract",
            "mappings": [],
        }
        write_report(options.report, payload)
        return 2

    if applicability.status != "applicable":
        write_report(
            options.report,
            base_report(applicability, time.monotonic() - started_at),
        )
        return 0

    command = [
        options.ruby_executable,
        str(options.ruby_adapter),
        "--workspace",
        str(workspace),
        "--config",
        str(applicability.config_path),
        "--report",
        options.report,
    ]
    environment = os.environ.copy()
    environment["EGOLINT_JSON_SKOOMA_NETWORK"] = "none"
    try:
        completed = subprocess.run(  # nosec B603
            command,
            cwd=workspace,
            env=environment,
            check=False,
            text=True,
            timeout=options.timeout_seconds,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        payload = base_report(applicability, time.monotonic() - started_at)
        payload["status"] = "unavailable" if isinstance(error, OSError) else "timed_out"
        payload["reason"] = "ruby-adapter-unavailable" if isinstance(error, OSError) else "timeout"
        write_report(options.report, payload)
        return 3
    return int(completed.returncode)


if __name__ == "__main__":
    raise SystemExit(main())
