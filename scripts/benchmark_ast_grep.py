#!/usr/bin/env python3
"""Reproduce the optional structural adapter's synthetic precision/latency controls."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
from pathlib import Path
import shutil

# The benchmark launches only resolved fixed-argv tools.
import subprocess  # nosec B404
import tempfile
import time
from typing import NoReturn


def fail(message: str) -> NoReturn:
    """Fail controls even when Python assertions are optimized away."""
    raise RuntimeError(message)


def exercise(binary: Path, fixtures: Path, label: str, repeats: int) -> dict[str, object]:
    """Run a real native invocation; never execute any fixture source."""
    with tempfile.TemporaryDirectory(prefix="egolint-structural-") as directory:
        root = Path(directory)
        git = shutil.which("git")
        if git is None:
            fail("Git is required for synthetic benchmark repositories")
        # Resolved Git and fixed argv; no shell.
        subprocess.run(  # noqa: S603  # nosec B603
            [str(Path(git).resolve()), "init", "-q", str(root)], check=True, capture_output=True
        )
        for index in range(repeats):
            for fixture in sorted(fixtures.iterdir()):
                if fixture.suffix in {".js", ".jsx", ".ts", ".tsx", ".py", ".rs"}:
                    shutil.copyfile(fixture, root / f"{fixture.stem}-{index:04}{fixture.suffix}")
                    if fixture.suffix == ".js":
                        for suffix in (".mjs", ".cjs"):
                            shutil.copyfile(fixture, root / f"{fixture.stem}-{index:04}{suffix}")
        started = time.perf_counter()
        # Explicit caller-selected native executable and fixed argv; no shell.
        process = subprocess.run(  # noqa: S603  # nosec B603
            [str(binary), "--workspace", str(root), "validate", "--ast-grep"],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        elapsed = round((time.perf_counter() - started) * 1000, 2)
        if process.returncode != 0:
            fail(f"{label}: native adapter exit {process.returncode}")
        report = json.loads((root / ".reports/egolint/run.json").read_text())
        findings = [f for f in report["findings"] if f["rule"]["tool_id"] == "AST_GREP"]
        tool = next(t for t in report["tool_results"] if t["tool_id"] == "AST_GREP")
        counts: dict[str, int] = {}
        for finding in findings:
            rule = finding["rule"]["rule_id"]
            counts[rule] = counts.get(rule, 0) + 1
            if "text" in finding or "lines" in finding:
                fail("Normalized evidence contains a source snippet")
            if not finding["location"]["path"].startswith("positive-"):
                fail("A negative structural control produced a finding")
        expected = {
            "EGO-AST-JS-EVAL-001": repeats * 4,
            "EGO-AST-TS-EVAL-001": repeats * 2,
            "EGO-AST-PY-SHELL-001": repeats,
            "EGO-AST-RS-EXIT-001": repeats,
        }
        if counts != expected:
            fail(f"{label}: structural match counts differ from the expected controls")
        if tool["enforcement"] != "advisory":
            fail("Structural findings must remain advisory")
        return {
            "dataset": label,
            "source_files": repeats * 16,
            "positive_matches": len(findings),
            "negative_matches": 0,
            "matches_by_rule": counts,
            "adapter_ms": tool["duration_ms"],
            "native_end_to_end_ms": elapsed,
        }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    arguments = parser.parse_args()
    binary = arguments.binary.resolve(strict=True)
    fixtures = Path(__file__).resolve().parents[1] / "tests/fixtures/structural"
    result = {
        "schema_version": 1,
        "native_executable_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "observed_at": dt.datetime.now(dt.UTC).isoformat(),
        "runtime": "ast-grep 0.51.1; native debug/release identity supplied by caller",
        "scope": "synthetic controls; no general precision or cross-platform claim",
        "datasets": [
            exercise(binary, fixtures, "small", 1),
            exercise(binary, fixtures, "medium", 32),
            exercise(binary, fixtures, "polyglot", 128),
        ],
    }
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
