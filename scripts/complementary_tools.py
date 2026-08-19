#!/usr/bin/env python3

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Validate and execute the direct complementary-tool contract."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import fnmatch
from functools import cache, lru_cache
import json
import os
from pathlib import Path
import shutil
import subprocess  # nosec B404
import sys
import time
from typing import Any, cast

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = REPOSITORY_ROOT / ".config/toolchain/complementary-tools.json"
EXCLUDED_PARTS = {
    ".cache",
    ".git",
    ".reports",
    ".venv",
    "build",
    "dist",
    "node_modules",
    "target",
    "venv",
}
FIXTURE_ROOT = Path("tests/fixtures")


@dataclass(frozen=True)
class ToolState:
    """Resolved state for one complementary tool."""

    identifier: str
    applicability: str
    dependency: str
    matched_files: tuple[str, ...]


def load_manifest() -> dict[str, Any]:
    """Load the canonical complementary-tool manifest."""

    return cast(
        "dict[str, Any]",
        json.loads(MANIFEST_PATH.read_text(encoding="utf-8")),
    )


def is_excluded(path: Path) -> bool:
    """Return whether a relative path is outside the applicability surface."""

    if path.is_absolute():
        try:
            path = path.relative_to(REPOSITORY_ROOT)
        except ValueError:
            return True
    if FIXTURE_ROOT in path.parents or path == FIXTURE_ROOT:
        return True
    return any(part in EXCLUDED_PARTS for part in path.parts)


@lru_cache(maxsize=1)
def repository_files() -> tuple[Path, ...]:
    """Index repository files once while pruning generated and dependency trees."""

    files: list[Path] = []
    for directory, directory_names, file_names in os.walk(REPOSITORY_ROOT):
        directory_path = Path(directory)
        relative_directory = directory_path.relative_to(REPOSITORY_ROOT)
        directory_names[:] = sorted(
            name for name in directory_names if not is_excluded(relative_directory / name)
        )
        files.extend(
            relative_directory / name
            for name in sorted(file_names)
            if not is_excluded(relative_directory / name)
        )
    return tuple(files)


def matches_glob(path: Path, pattern: str) -> bool:
    """Match repository paths with recursive ``**`` semantics and slash boundaries."""

    path_parts = path.parts
    pattern_parts = Path(pattern).parts

    @cache
    def matches(path_index: int, pattern_index: int) -> bool:
        if pattern_index == len(pattern_parts):
            return path_index == len(path_parts)
        if pattern_parts[pattern_index] == "**":
            return matches(path_index, pattern_index + 1) or (
                path_index < len(path_parts) and matches(path_index + 1, pattern_index)
            )
        return (
            path_index < len(path_parts)
            and fnmatch.fnmatchcase(path_parts[path_index], pattern_parts[pattern_index])
            and matches(path_index + 1, pattern_index + 1)
        )

    return matches(0, 0)


def matching_files(pattern: str) -> list[str]:
    """Resolve one repository-relative glob without counting fixtures or generated data."""

    return [path.as_posix() for path in repository_files() if matches_glob(path, pattern)]


def resolve_applicability(tool: dict[str, Any]) -> tuple[str, tuple[str, ...]]:
    """Resolve all-of marker groups, where each group contains any-of patterns."""

    marker_groups = tool["applicability"]["marker_groups"]
    if not marker_groups:
        return "applicable", ()

    all_matches: list[str] = []
    for marker_group in marker_groups:
        group_matches: list[str] = []
        for pattern in marker_group:
            group_matches.extend(matching_files(pattern))
        if not group_matches:
            return "not_applicable", ()
        all_matches.extend(group_matches)
    return "applicable", tuple(sorted(set(all_matches)))


def runtime_available(tool: dict[str, Any]) -> bool:
    """Return whether the command's required runtime is currently available."""

    runtime = tool["runtime"]
    if runtime == "uvx":
        return shutil.which("uvx") is not None
    if runtime == "node":
        return shutil.which("node") is not None and (REPOSITORY_ROOT / "scripts/pnpm.sh").is_file()
    if runtime == "pre-commit":
        return (REPOSITORY_ROOT / "scripts/precommit.sh").is_file() and (
            (REPOSITORY_ROOT / ".venv/bin/pre-commit").is_file()
            or shutil.which("pre-commit") is not None
            or shutil.which("uvx") is not None
        )
    if runtime == "go":
        return shutil.which("go") is not None
    if runtime == "latexindent":
        return shutil.which("latexindent") is not None
    if runtime == "cargo-deny":
        return shutil.which("cargo") is not None and shutil.which("cargo-deny") is not None
    return shutil.which(runtime) is not None


def resolve_tool_state(tool: dict[str, Any]) -> ToolState:
    """Resolve deliberate disablement, applicability, and dependency availability."""

    if tool["state"] == "disabled":
        return ToolState(tool["id"], "deliberately_disabled", "not_checked", ())

    applicability, files = resolve_applicability(tool)
    if applicability == "not_applicable":
        return ToolState(tool["id"], applicability, "not_checked", ())

    dependency = "available" if runtime_available(tool) else "missing_dependency"
    return ToolState(tool["id"], applicability, dependency, files)


def expand_command(tool: dict[str, Any], state: ToolState, key: str = "command") -> list[str]:
    """Expand the file-list placeholder in a tool command without invoking a shell."""

    target_files = state.matched_files
    if tool.get("target_patterns"):
        target_files = tuple(
            sorted(
                {
                    matched_file
                    for pattern in tool["target_patterns"]
                    for matched_file in matching_files(pattern)
                }
            )
        )

    command: list[str] = []
    for argument in tool[key]:
        if argument == "{files}":
            command.extend(target_files)
        else:
            command.append(argument)
    return command


def validate_manifest(manifest: dict[str, Any]) -> list[str]:
    """Return actionable contract errors without executing external tools."""

    errors: list[str] = []
    required_statuses = {
        "applicable",
        "not_applicable",
        "missing_dependency",
        "deliberately_disabled",
        "passed",
        "failed_findings",
        "execution_error",
        "timed_out",
    }
    if set(manifest.get("result_statuses", [])) != required_statuses:
        errors.append("result_statuses must contain the complete canonical status set")

    identifiers: set[str] = set()
    for tool in manifest.get("tools", []):
        identifier = tool.get("id", "<missing>")
        if identifier in identifiers:
            errors.append(f"duplicate tool id: {identifier}")
        identifiers.add(identifier)

        for key in (
            "name",
            "category",
            "version",
            "state",
            "runtime",
            "command",
            "version_command",
            "applicability",
            "fixtures",
            "report_path",
            "reason",
        ):
            if key not in tool:
                errors.append(f"{identifier}: missing {key}")

        version = str(tool.get("version", "")).strip()
        if not version or version in {"latest", "unknown", "runtime-reported"}:
            errors.append(f"{identifier}: version must be explicitly pinned")

        configuration_path = tool.get("configuration_path")
        if configuration_path and not (REPOSITORY_ROOT / configuration_path).is_file():
            errors.append(f"{identifier}: missing configuration {configuration_path}")

        fixtures = tool.get("fixtures", {})
        fixture_paths = fixtures.get("positive", []) + fixtures.get("negative", [])
        if not fixture_paths and not fixtures.get("blocker"):
            errors.append(f"{identifier}: requires fixture evidence or a blocker")
        if not fixtures.get("negative") and not fixtures.get("blocker"):
            errors.append(f"{identifier}: missing negative fixture or blocker")
        for fixture_path in fixture_paths:
            if not (REPOSITORY_ROOT / fixture_path).is_file():
                errors.append(f"{identifier}: missing fixture {fixture_path}")

        report_path = str(tool.get("report_path", ""))
        if not report_path.startswith(".reports/egolint/complementary/"):
            errors.append(f"{identifier}: report path is outside the complementary namespace")

        command_text = " ".join(tool.get("command", []))
        version_command_text = " ".join(tool.get("version_command", []))
        evidence_text = command_text + " " + version_command_text
        if version not in evidence_text and tool.get("runtime") in {"uvx", "go"}:
            errors.append(
                f"{identifier}: ephemeral command does not contain pinned version {version}"
            )

    expected_ids = {
        "addlicense",
        "buf",
        "cargo-deny",
        "commitlint",
        "complexipy",
        "conftest",
        "deptry",
        "detect-secrets",
        "govulncheck",
        "interrogate",
        "knip",
        "latexindent",
        "regal",
        "reuse",
        "tombi",
        "typos",
        "vacuum",
        "vulture",
    }
    if identifiers != expected_ids:
        errors.append(
            "tool inventory mismatch: "
            f"missing={sorted(expected_ids - identifiers)} extra={sorted(identifiers - expected_ids)}"
        )
    return errors


def generated_matrix(manifest: dict[str, Any]) -> dict[str, Any]:
    """Build the deterministic matrix consumed by tests, tasks, and PR 4."""

    tools: list[dict[str, Any]] = []
    for tool in sorted(manifest["tools"], key=lambda item: item["id"]):
        state = resolve_tool_state(tool)
        tools.append(
            {
                "id": tool["id"],
                "name": tool["name"],
                "category": tool["category"],
                "version": tool["version"],
                "configuration_path": tool["configuration_path"],
                "declared_state": tool["state"],
                "repository_applicability": state.applicability,
                "applicability_markers": tool["applicability"]["marker_groups"],
                "fixtures": tool["fixtures"],
                "runtime": tool["runtime"],
                "report_path": tool["report_path"],
                "reason": tool["reason"],
            }
        )
    return {
        "schema_version": manifest["schema_version"],
        "generated_from": MANIFEST_PATH.relative_to(REPOSITORY_ROOT).as_posix(),
        "result_statuses": manifest["result_statuses"],
        "report_root": manifest["report_root"],
        "tools": tools,
    }


def rendered_json(value: Any) -> str:
    """Render stable repository JSON."""

    return json.dumps(value, indent=2, sort_keys=False) + "\n"


def write_or_check_matrix(manifest: dict[str, Any], write: bool) -> int:
    """Write or verify the generated complementary matrix."""

    matrix_path = REPOSITORY_ROOT / manifest["generated_matrix"]
    expected = rendered_json(generated_matrix(manifest))
    if write:
        matrix_path.parent.mkdir(parents=True, exist_ok=True)
        matrix_path.write_text(expected, encoding="utf-8")
        print(f"Wrote {matrix_path.relative_to(REPOSITORY_ROOT)}")
        return 0
    if not matrix_path.is_file() or matrix_path.read_text(encoding="utf-8") != expected:
        print("Complementary tool matrix is stale; run with --write.", file=sys.stderr)
        return 1
    print(f"Validated {len(manifest['tools'])} complementary tool contracts.")
    return 0


def report_result(tool: dict[str, Any], payload: dict[str, Any]) -> None:
    """Persist a normalized latest report for later publication by PR 4."""

    report_path = REPOSITORY_ROOT / tool["report_path"]
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(rendered_json(payload), encoding="utf-8")


def runtime_version_matches(tool: dict[str, Any], environment: dict[str, str]) -> bool:
    """Verify locally resolved runtimes before allowing them to execute policy."""

    if tool["runtime"] in {"go", "pre-commit", "uvx"}:
        return True
    try:
        completed = subprocess.run(  # nosec B603
            tool["version_command"],
            cwd=REPOSITORY_ROOT,
            env=environment,
            check=False,
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False
    version_output = f"{completed.stdout}\n{completed.stderr}"
    return completed.returncode == 0 and tool["version"] in version_output


def run_tool(tool: dict[str, Any], timeout_seconds: int) -> int:
    """Run one applicable tool and emit a normalized result."""

    state = resolve_tool_state(tool)
    if state.applicability in {"not_applicable", "deliberately_disabled"}:
        print(f"{tool['id']}: {state.applicability}")
        return 0
    if state.dependency == "missing_dependency":
        print(f"{tool['id']}: missing dependency for runtime {tool['runtime']}", file=sys.stderr)
        return 3

    command = expand_command(tool, state)
    environment = os.environ.copy()
    environment["PATH"] = os.pathsep.join(
        (str(REPOSITORY_ROOT / ".venv/bin"), environment.get("PATH", ""))
    )
    environment.setdefault("CI", "true")
    cache_root = REPOSITORY_ROOT / ".reports/egolint/cache"
    environment.setdefault("COREPACK_HOME", str(cache_root / "corepack"))
    environment.setdefault("NPM_CONFIG_CACHE", str(cache_root / "npm"))
    environment.setdefault("PNPM_HOME", str(cache_root / "pnpm"))
    environment.setdefault("PRE_COMMIT_HOME", str(cache_root / "pre-commit"))
    environment.setdefault("XDG_CACHE_HOME", str(cache_root))
    environment.setdefault("UV_CACHE_DIR", str(cache_root / "uv"))
    environment.setdefault("UV_TOOL_DIR", str(cache_root / "uv/tools"))
    if not runtime_version_matches(tool, environment):
        print(
            f"{tool['id']}: runtime version does not match pinned {tool['version']}",
            file=sys.stderr,
        )
        return 3
    started_at = time.monotonic()
    try:
        completed = subprocess.run(  # nosec B603
            command,
            cwd=REPOSITORY_ROOT,
            env=environment,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
        )
        status = "passed" if completed.returncode == 0 else "failed_findings"
        payload = {
            "schema_version": 1,
            "tool": tool["id"],
            "version": tool["version"],
            "status": status,
            "exit_code": completed.returncode,
            "duration_seconds": round(time.monotonic() - started_at, 3),
            "command": command,
            "stdout": completed.stdout,
            "stderr": completed.stderr,
        }
    except subprocess.TimeoutExpired as error:
        payload = {
            "schema_version": 1,
            "tool": tool["id"],
            "version": tool["version"],
            "status": "timed_out",
            "exit_code": 124,
            "duration_seconds": round(time.monotonic() - started_at, 3),
            "command": command,
            "stdout": error.stdout or "",
            "stderr": error.stderr or "",
        }
    except OSError as error:
        payload = {
            "schema_version": 1,
            "tool": tool["id"],
            "version": tool["version"],
            "status": "execution_error",
            "exit_code": 3,
            "duration_seconds": round(time.monotonic() - started_at, 3),
            "command": command,
            "stdout": "",
            "stderr": str(error),
        }

    report_result(tool, payload)
    if payload["stdout"]:
        print(payload["stdout"], end="" if payload["stdout"].endswith("\n") else "\n")
    if payload["stderr"]:
        print(
            payload["stderr"], file=sys.stderr, end="" if payload["stderr"].endswith("\n") else "\n"
        )
    print(f"{tool['id']}: {payload['status']}")
    return int(payload["exit_code"])


def print_status(manifest: dict[str, Any], as_json: bool) -> int:
    """Print the current applicability and dependency state of every tool."""

    rows = []
    for tool in sorted(manifest["tools"], key=lambda item: item["id"]):
        state = resolve_tool_state(tool)
        rows.append(
            {
                "id": tool["id"],
                "version": tool["version"],
                "applicability": state.applicability,
                "dependency": state.dependency,
            }
        )
    if as_json:
        print(rendered_json({"tools": rows}), end="")
    else:
        for row in rows:
            print(
                f"{row['id']:<16} {row['version']:<10} "
                f"{row['applicability']:<24} {row['dependency']}"
            )
    return 0


def print_versions(manifest: dict[str, Any]) -> int:
    """Print the pinned version inventory without installing or executing tools."""

    for tool in sorted(manifest["tools"], key=lambda item: item["id"]):
        print(f"{tool['id']:<16} {tool['version']}")
    return 0


def parse_arguments() -> argparse.Namespace:
    """Parse the direct-tool command surface."""

    parser = argparse.ArgumentParser(description=__doc__)
    operation = parser.add_mutually_exclusive_group(required=True)
    operation.add_argument(
        "--check", action="store_true", help="Validate source and generated contracts."
    )
    operation.add_argument(
        "--write", action="store_true", help="Regenerate the checked-in tool matrix."
    )
    operation.add_argument(
        "--status", action="store_true", help="Print runtime applicability and availability."
    )
    operation.add_argument(
        "--versions", action="store_true", help="Print the pinned version inventory."
    )
    operation.add_argument("--run", metavar="TOOL", help="Run one complementary tool.")
    operation.add_argument(
        "--run-all", action="store_true", help="Run every applicable complementary tool."
    )
    parser.add_argument("--json", action="store_true", help="Use JSON output for --status.")
    parser.add_argument("--timeout-seconds", type=int, default=900)
    return parser.parse_args()


def main() -> int:
    """Execute the requested contract or tool operation."""

    arguments = parse_arguments()
    manifest = load_manifest()
    errors = validate_manifest(manifest)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    if arguments.write or arguments.check:
        return write_or_check_matrix(manifest, write=arguments.write)
    if arguments.status:
        return print_status(manifest, arguments.json)
    if arguments.versions:
        return print_versions(manifest)

    tools = {tool["id"]: tool for tool in manifest["tools"]}
    if arguments.run:
        if arguments.run not in tools:
            print(f"Unknown complementary tool: {arguments.run}", file=sys.stderr)
            return 2
        return run_tool(tools[arguments.run], arguments.timeout_seconds)

    exit_code = 0
    for tool in sorted(manifest["tools"], key=lambda item: item["id"]):
        result = run_tool(tool, arguments.timeout_seconds)
        if result != 0:
            exit_code = result
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
