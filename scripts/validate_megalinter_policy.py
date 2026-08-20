#!/usr/bin/env python3

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Validate and render the repository's offline MegaLinter v10 contract."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess  # nosec B404
import sys
from typing import TYPE_CHECKING

import yaml

if TYPE_CHECKING:
    from collections.abc import Mapping
    from typing import Any

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
CONTRACT_ROOT = REPOSITORY_ROOT / ".config" / "megalinter"
CATALOG_PATH = CONTRACT_ROOT / "v10-catalog.json"
POLICY_PATH = CONTRACT_ROOT / "policy.yml"
MATRIX_PATH = CONTRACT_ROOT / "tool-matrix.json"
SNAPSHOT_ROOT = CONTRACT_ROOT / "snapshots"

PROFILE_SNAPSHOT_PATHS = {
    "fast": SNAPSHOT_ROOT / "fast.json",
    "holistic": SNAPSHOT_ROOT / "holistic.json",
    "security": SNAPSHOT_ROOT / "security.json",
    "dependency-debt": SNAPSHOT_ROOT / "dependency-debt.json",
}
MEGALINTER_RELEASE = "v10.0.0"
MEGALINTER_COMMIT = "15e5b45552097e318c93de385779ce3b1084052c"


class ContractError(ValueError):
    """Raised when a MegaLinter policy contract is internally inconsistent."""


def parse_arguments(arguments: list[str]) -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--check",
        action="store_true",
        help="Validate source contracts and verify generated files are current.",
    )
    mode.add_argument(
        "--write",
        action="store_true",
        help="Validate source contracts and update generated matrix/snapshots.",
    )
    parser.add_argument(
        "--import-upstream",
        type=Path,
        metavar="MEGALINTER_SOURCE",
        help="Import the compact catalog from an official MegaLinter v10.0.0 checkout.",
    )
    options = parser.parse_args(arguments)
    if options.import_upstream and not options.write:
        parser.error("--import-upstream requires --write")
    return options


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping with path-aware diagnostics."""
    try:
        content = yaml.safe_load(path.read_text(encoding="utf-8"))
    except (OSError, yaml.YAMLError) as error:
        raise ContractError(f"Unable to load YAML mapping {path}: {error}") from error
    if not isinstance(content, dict):
        raise ContractError(f"Expected a YAML mapping at {path}.")
    return content


def load_json(path: Path) -> dict[str, Any]:
    """Load a JSON mapping with path-aware diagnostics."""
    try:
        content = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ContractError(f"Unable to load JSON mapping {path}: {error}") from error
    if not isinstance(content, dict):
        raise ContractError(f"Expected a JSON mapping at {path}.")
    return content


def normalized_linter_id(descriptor_id: str, linter: Mapping[str, Any]) -> str:
    """Return the canonical MegaLinter ID for a descriptor linter entry."""
    explicit_name = linter.get("name")
    if isinstance(explicit_name, str) and explicit_name:
        return explicit_name.upper()
    linter_name = str(linter["linter_name"]).upper()
    normalized_name = re.sub(r"[^A-Z0-9]+", "_", linter_name).strip("_")
    return f"{descriptor_id}_{normalized_name}"


def extract_documented_version(
    source_root: Path,
    linter_id: str,
    descriptor_id: str,
    linter: Mapping[str, Any],
) -> str:
    """Read the exact embedded tool version from generated v10 documentation."""
    normalized_name = re.sub(r"[^a-z0-9]+", "_", str(linter["linter_name"]).lower()).strip("_")
    documentation_names = [
        linter_id.lower(),
        f"{descriptor_id.lower()}_{normalized_name}",
        str(linter.get("test_folder", "")),
    ]
    for documentation_name in documentation_names:
        documentation_path = source_root / "docs" / "descriptors" / f"{documentation_name}.md"
        if not documentation_path.is_file():
            continue
        version_match = re.search(
            r"Version in MegaLinter:\s*\*\*([^*]+)\*\*",
            documentation_path.read_text(encoding="utf-8"),
        )
        if version_match:
            return version_match.group(1).strip()

    install_text = yaml.safe_dump(linter.get("install", {}), sort_keys=False)
    version_match = re.search(r"ARG [A-Z0-9_]*VERSION=([^\s]+)", install_text)
    return version_match.group(1).strip("\"'") if version_match else "runtime-reported"


def extract_removed_inventory(source_root: Path) -> tuple[list[str], list[str]]:
    """Extract removed linter and descriptor IDs from the official matrix."""
    removed_path = source_root / "docs" / "removed-linters.md"
    text = removed_path.read_text(encoding="utf-8")
    linter_section, _, descriptor_section = text.partition("## Removed descriptors")
    identifier_pattern = re.compile(r"^\| `([A-Z0-9_]+)` \|", re.MULTILINE)
    return (
        sorted(set(identifier_pattern.findall(linter_section))),
        sorted(set(identifier_pattern.findall(descriptor_section))),
    )


def build_catalog(source_root: Path) -> dict[str, Any]:
    """Build a compact offline contract from an official MegaLinter checkout."""
    checkout = subprocess.run(  # nosec B603 B607
        ["git", "-C", str(source_root), "rev-parse", "HEAD"],
        check=False,
        capture_output=True,
        text=True,
    )
    checkout_commit = checkout.stdout.strip()
    if checkout.returncode != 0 or checkout_commit != MEGALINTER_COMMIT:
        raise ContractError(
            f"Expected MegaLinter {MEGALINTER_RELEASE} at {MEGALINTER_COMMIT}; "
            f"received {checkout_commit or 'an unreadable checkout'} from {source_root}."
        )

    schema_path = (
        source_root
        / "megalinter"
        / "descriptors"
        / "schemas"
        / "megalinter-configuration.jsonschema.json"
    )
    schema = load_json(schema_path)
    schema_properties = schema.get("properties")
    if not isinstance(schema_properties, dict):
        raise ContractError(f"MegaLinter schema has no property inventory: {schema_path}")

    tools: dict[str, dict[str, Any]] = {}
    descriptor_root = source_root / "megalinter" / "descriptors"
    for descriptor_path in sorted(descriptor_root.glob("*.megalinter-descriptor.yml")):
        descriptor = load_yaml(descriptor_path)
        descriptor_id = str(descriptor["descriptor_id"]).upper()
        for raw_linter in descriptor.get("linters", []):
            if not isinstance(raw_linter, dict):
                continue
            linter_id = normalized_linter_id(descriptor_id, raw_linter)
            tools[linter_id] = {
                "cli_lint_mode": raw_linter.get("cli_lint_mode"),
                "config_file_name": raw_linter.get("config_file_name"),
                "descriptor": descriptor_id,
                "file_extensions": raw_linter.get(
                    "file_extensions", descriptor.get("file_extensions", [])
                ),
                "file_names_regex": raw_linter.get(
                    "file_names_regex", descriptor.get("file_names_regex", [])
                ),
                "linter_name": raw_linter.get("linter_name"),
                "test_folder": raw_linter.get("test_folder"),
                "version": extract_documented_version(
                    source_root,
                    linter_id,
                    descriptor_id,
                    raw_linter,
                ),
            }

    removed_linters, removed_descriptors = extract_removed_inventory(source_root)
    deprecated_variables = sorted(
        key
        for key, value in schema_properties.items()
        if isinstance(value, dict) and value.get("deprecated") is True
    )

    return {
        "schema_version": 1,
        "megalinter_release": MEGALINTER_RELEASE,
        "megalinter_commit": MEGALINTER_COMMIT,
        "source": "https://github.com/oxsecurity/megalinter/tree/v10.0.0",
        "configuration_variables": sorted(schema_properties),
        "deprecated_variables": deprecated_variables,
        "removed_descriptors": removed_descriptors,
        "removed_linters": removed_linters,
        "tools": dict(sorted(tools.items())),
    }


def resolve_extended_configuration(
    path: Path,
    visited: frozenset[Path] = frozenset(),
) -> dict[str, Any]:
    """Resolve local EXTENDS entries using MegaLinter's root-relative model."""
    canonical_path = path.resolve()
    if canonical_path in visited:
        chain = " -> ".join(candidate.as_posix() for candidate in (*visited, canonical_path))
        raise ContractError(f"Cyclic MegaLinter EXTENDS chain: {chain}")

    configuration = load_yaml(canonical_path)
    merged: dict[str, Any] = {}
    extends = configuration.get("EXTENDS", [])
    if isinstance(extends, str):
        extends = [extends]
    if not isinstance(extends, list):
        raise ContractError(f"EXTENDS must be a string or list in {path}.")

    for reference in extends:
        if not isinstance(reference, str) or reference.startswith(("http://", "https://")):
            raise ContractError(
                f"Only local EXTENDS references are supported in {path}: {reference}"
            )
        extended_path = REPOSITORY_ROOT / reference
        if not extended_path.is_file():
            extended_path = canonical_path.parent / reference
        if not extended_path.is_file():
            raise ContractError(f"Missing EXTENDS target {reference} referenced by {path}.")
        merged.update(
            resolve_extended_configuration(extended_path, visited | frozenset({canonical_path}))
        )

    merged.update(configuration)
    return merged


def as_string_list(value: Any, key: str) -> list[str]:
    """Normalize a MegaLinter string-or-list selection variable."""
    if value is None:
        return []
    if isinstance(value, str):
        return [item.strip() for item in value.split(",") if item.strip()]
    if isinstance(value, list) and all(isinstance(item, str) for item in value):
        return list(value)
    raise ContractError(f"{key} must be a string or list of strings.")


def resolve_configuration_path(rules_path: str, config_file: str) -> Path:
    """Resolve a MegaLinter rules/config pair into the local workspace."""
    normalized_rules_path = rules_path
    for workspace_prefix in (
        "${GITHUB_WORKSPACE}/",
        "/github/workspace/",
        "/tmp/lint/",  # nosec B108
    ):
        candidate = normalized_rules_path.removeprefix(workspace_prefix)
        if candidate != normalized_rules_path:
            normalized_rules_path = candidate
            break
    rules_root = Path(normalized_rules_path)
    if not rules_root.is_absolute():
        rules_root = REPOSITORY_ROOT / rules_root
    return rules_root / config_file


def infer_tool_configuration(tool_id: str, configuration: Mapping[str, Any]) -> str | None:
    """Infer the repository-relative config consumed by one linter."""
    config_file = configuration.get(f"{tool_id}_CONFIG_FILE")
    if not isinstance(config_file, str) or config_file == "LINTER_DEFAULT":
        return None
    rules_path = configuration.get(
        f"{tool_id}_RULES_PATH",
        configuration.get("LINTER_RULES_PATH", "."),
    )
    if not isinstance(rules_path, str):
        return None
    resolved = resolve_configuration_path(rules_path, config_file)
    try:
        return resolved.relative_to(REPOSITORY_ROOT).as_posix()
    except ValueError:
        return resolved.as_posix()


def validate_configuration(
    path: Path,
    configuration: Mapping[str, Any],
    catalog: Mapping[str, Any],
) -> list[str]:
    """Validate one effective MegaLinter configuration against v10 inventory."""
    findings: list[str] = []
    allowed_variables = set(catalog["configuration_variables"])
    deprecated_variables = set(catalog["deprecated_variables"])
    supported_linters = set(catalog["tools"])
    supported_descriptors = {tool["descriptor"] for tool in catalog["tools"].values()}
    removed_linters = set(catalog["removed_linters"])
    removed_descriptors = set(catalog["removed_descriptors"])

    for key in configuration:
        if key not in allowed_variables:
            findings.append(f"{path}: unknown MegaLinter v10 variable {key}")
        elif key in deprecated_variables:
            findings.append(f"{path}: removed/deprecated MegaLinter variable {key}")

    for selection_key in ("ENABLE_LINTERS", "DISABLE_LINTERS", "DISABLE_ERRORS_LINTERS"):
        for linter_id in as_string_list(configuration.get(selection_key), selection_key):
            if linter_id in removed_linters:
                findings.append(f"{path}: {selection_key} references removed linter {linter_id}")
            elif linter_id not in supported_linters:
                findings.append(f"{path}: {selection_key} references unknown linter {linter_id}")

    for selection_key in ("ENABLE", "DISABLE"):
        for descriptor_id in as_string_list(configuration.get(selection_key), selection_key):
            if descriptor_id in removed_descriptors:
                findings.append(
                    f"{path}: {selection_key} references removed descriptor {descriptor_id}"
                )
            elif descriptor_id not in supported_descriptors:
                findings.append(
                    f"{path}: {selection_key} references unknown descriptor {descriptor_id}"
                )

    for linter_id in supported_linters:
        config_file = configuration.get(f"{linter_id}_CONFIG_FILE")
        if not isinstance(config_file, str) or config_file == "LINTER_DEFAULT":
            continue
        rules_path = configuration.get(
            f"{linter_id}_RULES_PATH",
            configuration.get("LINTER_RULES_PATH", "."),
        )
        if not isinstance(rules_path, str):
            findings.append(f"{path}: {linter_id}_RULES_PATH must be a string")
            continue
        resolved_path = resolve_configuration_path(rules_path, config_file)
        if not resolved_path.is_file():
            findings.append(
                f"{path}: {linter_id} config does not exist at "
                f"{resolved_path.relative_to(REPOSITORY_ROOT)}"
            )
    return findings


def validate_policy(
    policy: Mapping[str, Any],
    catalog: Mapping[str, Any],
    holistic_configuration: Mapping[str, Any],
) -> list[str]:
    """Validate disabled reasons, metadata, and fixture evidence."""
    findings: list[str] = []
    megalinter_policy = policy.get("megalinter", {})
    if not isinstance(megalinter_policy, dict):
        return [f"{POLICY_PATH}: megalinter must be a mapping"]
    policy_release = megalinter_policy.get("release")
    policy_commit = megalinter_policy.get("commit")
    if policy_release != catalog["megalinter_release"]:
        findings.append(
            f"{POLICY_PATH}: release {policy_release} does not match catalog "
            f"{catalog['megalinter_release']}"
        )
    if policy_commit != catalog["megalinter_commit"]:
        findings.append(
            f"{POLICY_PATH}: commit {policy_commit} does not match catalog "
            f"{catalog['megalinter_commit']}"
        )
    ownership_policy = policy.get("ownership", {})
    if not isinstance(ownership_policy, dict):
        return [*findings, f"{POLICY_PATH}: ownership must be a mapping"]
    default_owner = ownership_policy.get("default_owner")
    policy_source = ownership_policy.get("policy_source")
    if not isinstance(default_owner, str) or not default_owner.strip():
        findings.append(f"{POLICY_PATH}: ownership.default_owner must be nonempty")
    if not isinstance(policy_source, str) or not policy_source.strip():
        findings.append(f"{POLICY_PATH}: ownership.policy_source must be nonempty")
    elif not (REPOSITORY_ROOT / policy_source).is_file():
        findings.append(f"{POLICY_PATH}: ownership.policy_source is missing: {policy_source}")
    supported_linters = set(catalog["tools"])
    disabled_linters = set(
        as_string_list(holistic_configuration.get("DISABLE_LINTERS"), "DISABLE_LINTERS")
    )
    advisory_linters = set(
        as_string_list(
            holistic_configuration.get("DISABLE_ERRORS_LINTERS"),
            "DISABLE_ERRORS_LINTERS",
        )
    )
    disabled_reasons = policy.get("disabled_reasons", {})
    if not isinstance(disabled_reasons, dict):
        return [f"{POLICY_PATH}: disabled_reasons must be a mapping"]

    for linter_id in sorted(disabled_linters):
        reason = disabled_reasons.get(linter_id)
        if not isinstance(reason, str) or not reason.strip():
            findings.append(f"{POLICY_PATH}: {linter_id} has no disabled reason")
    for linter_id in sorted(disabled_reasons):
        if linter_id not in disabled_linters:
            findings.append(f"{POLICY_PATH}: stale disabled reason for enabled {linter_id}")

    advisory_reasons = policy.get("advisory_reasons", {})
    if not isinstance(advisory_reasons, dict):
        return [*findings, f"{POLICY_PATH}: advisory_reasons must be a mapping"]
    for linter_id in sorted(advisory_linters):
        reason = advisory_reasons.get(linter_id)
        if not isinstance(reason, str) or not reason.strip():
            findings.append(f"{POLICY_PATH}: {linter_id} has no advisory reason")
    for linter_id in sorted(advisory_reasons):
        if linter_id not in advisory_linters:
            findings.append(f"{POLICY_PATH}: stale advisory reason for blocking {linter_id}")

    tool_policy = policy.get("tools", {})
    if not isinstance(tool_policy, dict):
        return [*findings, f"{POLICY_PATH}: tools must be a mapping"]
    for linter_id, metadata in tool_policy.items():
        if linter_id not in supported_linters:
            findings.append(f"{POLICY_PATH}: unknown tool metadata key {linter_id}")
            continue
        if not isinstance(metadata, dict):
            findings.append(f"{POLICY_PATH}: metadata for {linter_id} must be a mapping")
            continue
        configuration_path = metadata.get("configuration")
        if (
            isinstance(configuration_path, str)
            and not (REPOSITORY_ROOT / configuration_path).is_file()
        ):
            findings.append(
                f"{POLICY_PATH}: {linter_id} configuration is missing: {configuration_path}"
            )
        owner = metadata.get("owner", default_owner)
        if not isinstance(owner, str) or not owner.strip():
            findings.append(f"{POLICY_PATH}: {linter_id} owner must be nonempty")
        tool_policy_source = metadata.get("policy_source", policy_source)
        if not isinstance(tool_policy_source, str) or not tool_policy_source.strip():
            findings.append(f"{POLICY_PATH}: {linter_id} policy_source must be nonempty")

        fixture_metadata = metadata.get("fixtures")
        if fixture_metadata is None:
            continue
        if not isinstance(fixture_metadata, dict):
            findings.append(f"{POLICY_PATH}: {linter_id} fixtures must be a mapping")
            continue
        positive = fixture_metadata.get("positive", [])
        negative = fixture_metadata.get("negative", [])
        blocker = fixture_metadata.get("blocker")
        if (not positive or not negative) and not blocker:
            findings.append(
                f"{POLICY_PATH}: {linter_id} requires positive/negative fixtures or a blocker"
            )
        for fixture_path in [*positive, *negative]:
            if not (REPOSITORY_ROOT / fixture_path).is_file():
                findings.append(f"{POLICY_PATH}: {linter_id} fixture is missing: {fixture_path}")
    return findings


def build_matrix_and_snapshots(
    policy: Mapping[str, Any],
    catalog: Mapping[str, Any],
) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    """Build generated tool inventory and profile selection snapshots."""
    profiles = policy["profiles"]
    effective_profiles = {
        profile_name: resolve_extended_configuration(REPOSITORY_ROOT / profile["config"])
        for profile_name, profile in profiles.items()
    }
    holistic_configuration = effective_profiles["holistic"]
    disabled = set(as_string_list(holistic_configuration.get("DISABLE_LINTERS"), "DISABLE_LINTERS"))
    advisory = set(
        as_string_list(
            holistic_configuration.get("DISABLE_ERRORS_LINTERS"),
            "DISABLE_ERRORS_LINTERS",
        )
    )
    all_tool_ids = set(catalog["tools"])
    profile_selections = {
        profile_name: selected_tools(configuration, all_tool_ids)
        for profile_name, configuration in effective_profiles.items()
    }
    tool_policy = policy.get("tools", {})
    disabled_reasons = policy["disabled_reasons"]
    advisory_reasons = policy["advisory_reasons"]
    ownership_policy = policy["ownership"]
    default_owner = ownership_policy["default_owner"]
    default_policy_source = ownership_policy["policy_source"]

    matrix_tools: list[dict[str, Any]] = []
    for tool_id, catalog_tool in catalog["tools"].items():
        metadata = tool_policy.get(tool_id, {})
        inferred_configuration = infer_tool_configuration(tool_id, holistic_configuration)
        configuration_path = metadata.get("configuration", inferred_configuration)
        fixture_metadata = metadata.get("fixtures", {})
        is_disabled = tool_id in disabled
        applicability = metadata.get("applicability", [])
        state = "disabled" if is_disabled else "enabled"
        if not is_disabled and applicability and applicability != ["complete-repository"]:
            state = "conditional"
        if is_disabled:
            enforcement = "disabled"
        elif tool_id in advisory:
            enforcement = "advisory"
        else:
            enforcement = "blocking"

        matrix_tools.append(
            {
                "id": tool_id,
                "descriptor": catalog_tool["descriptor"],
                "name": catalog_tool["linter_name"],
                "version": catalog_tool["version"],
                "configuration_state": state,
                "configuration_path": configuration_path,
                "cli_lint_mode": catalog_tool["cli_lint_mode"],
                "applicability": applicability,
                "fixtures": fixture_metadata,
                "profiles": {
                    profile_name: profile_state(tool_id, selection)
                    for profile_name, selection in profile_selections.items()
                },
                "profile_enforcement": {
                    profile_name: profile_enforcement(
                        tool_id,
                        effective_profiles[profile_name],
                        selection[0],
                    )
                    for profile_name, selection in profile_selections.items()
                },
                "enforcement": enforcement,
                "owner": metadata.get("owner", default_owner),
                "policy_source": metadata.get("policy_source", default_policy_source),
                "reason": disabled_reasons.get(
                    tool_id,
                    advisory_reasons.get(
                        tool_id,
                        metadata.get(
                            "ownership",
                            "Selected by the holistic MegaLinter profile.",
                        ),
                    ),
                ),
                "report_path": f".reports/egolint/linters_logs/{tool_id}.log",
                "evidence": {
                    "configuration": configuration_path,
                    "fixtures": fixture_metadata,
                    "runtime_report": f".reports/egolint/linters_logs/{tool_id}.log",
                },
            }
        )

    matrix = {
        "schema_version": 1,
        "generated_from": {
            "catalog": CATALOG_PATH.relative_to(REPOSITORY_ROOT).as_posix(),
            "policy": POLICY_PATH.relative_to(REPOSITORY_ROOT).as_posix(),
        },
        "megalinter_release": catalog["megalinter_release"],
        "megalinter_commit": catalog["megalinter_commit"],
        "result_statuses": policy["result_statuses"],
        "tools": matrix_tools,
    }

    snapshots: dict[str, dict[str, Any]] = {}
    for profile_name, configuration in effective_profiles.items():
        selected, disabled_for_profile, disabled_by_configuration = profile_selections[profile_name]
        snapshots[profile_name] = {
            "schema_version": 1,
            "profile": profile_name,
            "config": profiles[profile_name]["config"],
            "validate_all_codebase": bool(configuration.get("VALIDATE_ALL_CODEBASE", True)),
            "selected": sorted(selected),
            "disabled_by_profile": sorted(disabled_for_profile),
            "disabled_by_configuration": sorted(disabled_by_configuration),
        }
    return matrix, snapshots


def selected_tools(
    configuration: Mapping[str, Any],
    all_tool_ids: set[str],
) -> tuple[set[str], set[str], set[str]]:
    """Partition a profile using MegaLinter's explicit selection semantics."""
    disabled = set(as_string_list(configuration.get("DISABLE_LINTERS"), "DISABLE_LINTERS"))
    enabled_value = configuration.get("ENABLE_LINTERS")
    if enabled_value is None:
        selected = all_tool_ids - disabled
    else:
        enabled = set(as_string_list(enabled_value, "ENABLE_LINTERS"))
        if configuration.get("ENABLE_DISABLE_LINTERS_PRIORITY") == "ENABLE":
            selected = enabled
        else:
            selected = enabled - disabled
    disabled_by_configuration = disabled - selected
    disabled_by_profile = all_tool_ids - selected - disabled_by_configuration
    return selected, disabled_by_profile, disabled_by_configuration


def profile_state(
    tool_id: str,
    selection: tuple[set[str], set[str], set[str]],
) -> str:
    """Return one explicit selection state for a tool in a profile."""
    selected, disabled_by_profile, disabled_by_configuration = selection
    if tool_id in selected:
        return "selected"
    if tool_id in disabled_by_configuration:
        return "disabled_by_configuration"
    if tool_id in disabled_by_profile:
        return "disabled_by_profile"
    raise ContractError(f"Tool {tool_id} is not classified by the profile selection")


def profile_enforcement(
    tool_id: str,
    configuration: Mapping[str, Any],
    selected: set[str],
) -> str:
    """Return effective per-profile enforcement for one tool."""
    if tool_id not in selected:
        return "disabled"
    advisory = set(
        as_string_list(
            configuration.get("DISABLE_ERRORS_LINTERS"),
            "DISABLE_ERRORS_LINTERS",
        )
    )
    if configuration.get(f"{tool_id}_DISABLE_ERRORS") is True or tool_id in advisory:
        return "advisory"
    return "blocking"


def rendered_json(value: Mapping[str, Any]) -> str:
    """Render stable generated JSON."""
    return f"{json.dumps(value, indent=2, sort_keys=True)}\n"


def atomic_write(path: Path, content: str) -> None:
    """Write a generated contract atomically."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path = path.with_name(f".{path.name}.tmp")
    temporary_path.write_text(content, encoding="utf-8")
    temporary_path.replace(path)


def check_generated_file(path: Path, expected: str) -> str | None:
    """Return a drift diagnostic when generated content is stale."""
    if not path.is_file():
        return f"Generated contract is missing: {path.relative_to(REPOSITORY_ROOT)}"
    if path.read_text(encoding="utf-8") != expected:
        return (
            f"Generated contract is stale: {path.relative_to(REPOSITORY_ROOT)}; "
            "run the validator with --write"
        )
    return None


def validate_all_configurations(
    catalog: Mapping[str, Any],
    policy: Mapping[str, Any],
) -> list[str]:
    """Validate the source and effective form of every profile configuration."""
    findings: list[str] = []
    configuration_paths = [
        REPOSITORY_ROOT / profile["config"] for profile in policy["profiles"].values()
    ]
    for path in configuration_paths:
        configuration = resolve_extended_configuration(path)
        findings.extend(validate_configuration(path, configuration, catalog))
    return findings


def write_or_check_outputs(
    *,
    write: bool,
    matrix: Mapping[str, Any],
    snapshots: Mapping[str, Mapping[str, Any]],
) -> list[str]:
    """Write generated outputs or report drift."""
    findings: list[str] = []
    generated = {MATRIX_PATH: rendered_json(matrix)}
    generated.update(
        {
            PROFILE_SNAPSHOT_PATHS[name]: rendered_json(snapshot)
            for name, snapshot in snapshots.items()
        }
    )
    for path, content in generated.items():
        if write:
            atomic_write(path, content)
            print(f"Updated {path.relative_to(REPOSITORY_ROOT)}")
        else:
            finding = check_generated_file(path, content)
            if finding:
                findings.append(finding)
    return findings


def main(arguments: list[str] | None = None) -> int:
    """Validate policy and keep generated contracts synchronized."""
    options = parse_arguments(arguments or sys.argv[1:])
    if options.import_upstream:
        upstream_root = options.import_upstream.resolve()
        catalog = build_catalog(upstream_root)
        atomic_write(CATALOG_PATH, rendered_json(catalog))
        print(f"Imported {CATALOG_PATH.relative_to(REPOSITORY_ROOT)}")

    catalog = load_json(CATALOG_PATH)
    policy = load_yaml(POLICY_PATH)
    holistic_configuration = resolve_extended_configuration(REPOSITORY_ROOT / ".mega-linter.yml")
    findings = validate_all_configurations(catalog, policy)
    findings.extend(validate_policy(policy, catalog, holistic_configuration))
    if findings:
        for finding in findings:
            print(f"ERROR: {finding}", file=sys.stderr)
        return 1

    matrix, snapshots = build_matrix_and_snapshots(policy, catalog)
    generated_findings = write_or_check_outputs(
        write=options.write,
        matrix=matrix,
        snapshots=snapshots,
    )
    if generated_findings:
        for finding in generated_findings:
            print(f"ERROR: {finding}", file=sys.stderr)
        return 1

    print(
        f"MegaLinter {catalog['megalinter_release']} contract validated: "
        f"{len(catalog['tools'])} supported tools, "
        f"{len(snapshots['fast']['selected'])} fast, "
        f"{len(snapshots['holistic']['selected'])} holistic, "
        f"{len(snapshots['security']['selected'])} security, "
        f"{len(snapshots['dependency-debt']['selected'])} dependency-debt."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
