// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { chmod, cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import test from "node:test";

const cleanOxlint = {
    diagnostics: [],
    number_of_errors: 0,
    number_of_warnings: 0,
};
const sarif = (results = []) => ({ version: "2.1.0", runs: [{ results }] });
const fail = {
    ruleId: "format",
    level: "error",
    message: { text: "Synthetic format error" },
};

// These trusted executable stubs exercise orchestration, not analyzer behavior.
async function invoke(
    t,
    {
        oxlint = cleanOxlint,
        biome = sarif(),
        exit = {},
        raw = {},
        signal = {},
        publication = "private",
        missingExecutable = false,
    } = {},
) {
    const root = await mkdtemp(join(tmpdir(), "egolint-js-protocol-"));
    t.after(() => rm(root, { recursive: true, force: true }));
    const bin = join(root, "bin");
    const consumer = join(root, "consumer");
    await mkdir(bin);
    await mkdir(consumer);
    await writeFile(
        join(consumer, "egolint.javascript-package-quality.json"),
        JSON.stringify({
            schema_version: 1,
            profile: "base",
            package_path: ".",
            publication,
        }),
    );
    await writeFile(
        join(consumer, "package.json"),
        JSON.stringify({
            name: "synthetic-protocol-consumer",
            version: "1.0.0",
            private: publication === "private",
        }),
    );
    const stub = join(bin, "pnpm");
    await writeFile(
        stub,
        `#!${process.execPath}
const tool = process.argv.includes("oxlint") ? "oxlint" : "biome";
const fixture = JSON.parse(process.env.EGOLINT_TEST_ADAPTERS);
if (process.argv.includes("pack")) process.exit(2);
if (fixture.signal[tool]) process.kill(process.pid, fixture.signal[tool]);
process.stdout.write(fixture.raw[tool] ?? JSON.stringify(fixture[tool]));
process.exit(fixture.exit[tool] ?? 0);
`,
    );
    await chmod(stub, 0o755);
    const result = spawnSync(
        process.execPath,
        [resolve("scripts/javascript-package-quality.mjs"), "--workspace", consumer],
        {
            encoding: "utf8",
            timeout: 30000,
            env: {
                ...process.env,
                PATH: missingExecutable
                    ? join(root, "missing-bin")
                    : `${bin}${delimiter}${process.env.PATH}`,
                EGOLINT_TEST_ADAPTERS: JSON.stringify({ oxlint, biome, exit, raw, signal }),
            },
        },
    );
    assert.ifError(result.error);
    let report;
    try {
        report = JSON.parse(
            await readFile(
                join(consumer, ".reports/egolint/javascript-package-quality.json"),
                "utf8",
            ),
        );
    } catch (error) {
        assert.fail(
            `CLI did not produce structured report (exit ${result.status}): ${result.stderr}; ${error.message}`,
        );
    }
    const projected = JSON.parse(
        await readFile(join(consumer, ".reports/egolint/javascript-package-quality.sarif"), "utf8"),
    );
    return {
        result,
        report,
        projected,
        statuses: Object.fromEntries(
            report.tool_results.map((item) => [item.tool_id, item.status]),
        ),
    };
}

test("adapter clean controls and SARIF pass semantics", {
    skip: process.platform === "win32",
}, async (suite) => {
    for (const result of [
        null,
        { ...fail, kind: "pass", level: "none" },
        { ruleId: "format", kind: "pass", message: { text: "Successful check" } },
        { ...fail, kind: "notApplicable", level: "none" },
    ]) {
        await suite.test(JSON.stringify(result), async (t) => {
            const {
                result: processResult,
                report,
                statuses,
                projected,
            } = await invoke(t, { biome: sarif(result ? [result] : []) });
            assert.equal(processResult.status, 0);
            assert.equal(statuses.OXLINT, "passed");
            assert.equal(statuses.BIOME, "passed");
            assert.equal(statuses.PUBLINT, "not_applicable");
            assert.deepEqual(report.findings, []);
            assert.deepEqual(projected.runs[0].results, []);
        });
    }
});

const invalidCases = [
    ["Oxlint nonzero without findings", { exit: { oxlint: 1 } }, "OXLINT"],
    ["Biome nonzero without findings", { exit: { biome: 1 } }, "BIOME"],
    ["Oxlint runtime exit", { exit: { oxlint: 2 } }, "OXLINT"],
    ["Biome runtime exit", { exit: { biome: 2 } }, "BIOME"],
    ["Oxlint empty envelope", { oxlint: {} }, "OXLINT"],
    ["Oxlint null", { oxlint: null }, "OXLINT"],
    ["Oxlint wrong diagnostics", { oxlint: { diagnostics: {} } }, "OXLINT"],
    ["Oxlint null diagnostic", { oxlint: { diagnostics: [null] } }, "OXLINT"],
    ["Oxlint contradictory counts", { oxlint: { ...cleanOxlint, number_of_errors: 1 } }, "OXLINT"],
    ["Oxlint wrong count type", { oxlint: { ...cleanOxlint, number_of_warnings: "0" } }, "OXLINT"],
    [
        "Oxlint unknown severity",
        { oxlint: { diagnostics: [{ message: "Synthetic", severity: "fatal" }] } },
        "OXLINT",
    ],
    ["Oxlint empty stdout", { raw: { oxlint: "" } }, "OXLINT"],
    ["Biome invalid JSON", { raw: { biome: "[" } }, "BIOME"],
    ["Biome empty envelope", { biome: {} }, "BIOME"],
    ["Biome wrong version", { biome: { ...sarif(), version: "2.0.0" } }, "BIOME"],
    ["Biome wrong runs", { biome: { version: "2.1.0", runs: {} } }, "BIOME"],
    ["Biome null run", { biome: { version: "2.1.0", runs: [null] } }, "BIOME"],
    ["Biome wrong results", { biome: { version: "2.1.0", runs: [{ results: {} }] } }, "BIOME"],
    ["Biome null result", { biome: sarif([null]) }, "BIOME"],
    ["Biome contradictory pass", { biome: sarif([{ ...fail, kind: "pass" }]) }, "BIOME"],
    ["Biome unknown kind", { biome: sarif([{ ...fail, kind: "unknown" }]) }, "BIOME"],
    ["Biome unknown level", { biome: sarif([{ ...fail, level: "critical" }]) }, "BIOME"],
];

test("invalid adapter evidence has non-clean structured outcomes", {
    skip: process.platform === "win32",
}, async (suite) => {
    for (const [name, fixture, tool] of invalidCases) {
        await suite.test(name, async (t) => {
            const { result, report, statuses, projected } = await invoke(t, fixture);
            assert.equal(result.status, 3, result.stderr);
            assert.equal(statuses[tool], "execution_error");
            assert.equal(statuses[tool === "OXLINT" ? "BIOME" : "OXLINT"], "passed");
            assert.equal(report.findings.length, 0, "protocol errors are not lint findings");
            assert.equal(projected.runs[0].invocations[0].executionSuccessful, false);
            assert.equal(projected.runs[0].results.length, 0);
        });
    }
});

test("valid finding keeps independent exit, status and normalized severity", {
    skip: process.platform === "win32",
}, async (t) => {
    const { result, report, statuses, projected } = await invoke(t, {
        biome: sarif([fail]),
        exit: { biome: 1 },
    });
    assert.equal(result.status, 1);
    assert.equal(statuses.BIOME, "failed_findings");
    assert.equal(report.findings.length, 1);
    assert.equal(report.findings[0].finding.severity, "error");
    assert.equal(projected.runs[0].results.length, 1);
});

test("SARIF kind and level matrix validates before deciding finding status", {
    skip: process.platform === "win32",
}, async (suite) => {
    for (const kind of [
        undefined,
        "fail",
        "pass",
        "notApplicable",
        "informational",
        "open",
        "review",
    ]) {
        for (const level of [undefined, "none", "note", "warning", "error"]) {
            await suite.test(`${kind}/${level}`, async (t) => {
                const isFail = kind === undefined || kind === "fail";
                const valid = isFail || level === undefined || level === "none";
                const findings = valid && !["pass", "notApplicable"].includes(kind) ? 1 : 0;
                const error = valid && isFail && level === "error";
                const { result, report, statuses } = await invoke(t, {
                    biome: sarif([{ ...fail, kind, level }]),
                    exit: { biome: error ? 1 : 0 },
                });
                assert.equal(result.status, valid ? (error ? 1 : 0) : 3);
                assert.equal(
                    statuses.BIOME,
                    !valid
                        ? "execution_error"
                        : error
                          ? "failed_findings"
                          : isFail && (level === undefined || level === "warning")
                            ? "passed_with_warnings"
                            : "passed",
                );
                assert.equal(report.findings.length, findings);
            });
        }
    }
});

test("process outcomes survive valid payloads and failures in other adapters", {
    skip: process.platform === "win32",
}, async (suite) => {
    for (const code of [0, 2]) {
        await suite.test(`error diagnostic with exit ${code}`, async (t) => {
            const { result, report, statuses } = await invoke(t, {
                biome: sarif([fail]),
                exit: { biome: code },
            });
            assert.equal(result.status, 3);
            assert.equal(statuses.BIOME, "execution_error");
            assert.equal(report.findings.length, 1);
            assert.equal(report.adapter_outcomes.biome.exit_code, code);
            assert.equal(report.adapter_outcomes.biome.protocol_status, "valid");
        });
    }
    await suite.test("signal remains unavailable", async (t) => {
        const { result, report, statuses } = await invoke(t, { signal: { oxlint: "SIGTERM" } });
        assert.equal(result.status, 3);
        assert.equal(statuses.OXLINT, "execution_error");
        assert.equal(statuses.BIOME, "passed");
        assert.equal(report.adapter_outcomes.oxlint.signal, "SIGTERM");
        assert.equal(report.adapter_outcomes.oxlint.protocol_status, "unavailable");
    });
    await suite.test("missing executable remains unavailable", async (t) => {
        const { result, report, statuses } = await invoke(t, { missingExecutable: true });
        assert.equal(result.status, 3);
        assert.equal(statuses.OXLINT, "execution_error");
        assert.equal(statuses.BIOME, "execution_error");
        assert.equal(report.adapter_outcomes.oxlint.error_code, "ENOENT");
        assert.equal(report.adapter_outcomes.oxlint.protocol_status, "unavailable");
        assert.equal(report.findings.length, 0);
    });
    await suite.test("nonzero warning remains failed", async (t) => {
        const { result, report, statuses } = await invoke(t, {
            biome: sarif([{ ...fail, level: "warning" }]),
            exit: { biome: 1 },
        });
        assert.equal(result.status, 1);
        assert.equal(statuses.BIOME, "failed_findings");
        assert.equal(report.summary.errors, 0);
        assert.equal(report.summary.warnings, 1);
    });
    await suite.test("publint API failure keeps other tool evidence", async (t) => {
        const { result, report, statuses } = await invoke(t, { publication: "npm" });
        assert.equal(result.status, 3);
        assert.equal(statuses.PUBLINT, "execution_error");
        assert.equal(statuses.OXLINT, "passed");
        assert.equal(statuses.BIOME, "passed");
        assert.equal(report.findings.length, 0);
    });
});

test("real pinned analyzers retain authentic consumer controls", async (suite) => {
    for (const fixture of ["private-app", "react-library", "broken-library"]) {
        await suite.test(fixture, async (t) => {
            const root = await mkdtemp(join(tmpdir(), "egolint-real-js-"));
            t.after(() => rm(root, { recursive: true, force: true }));
            const consumer = join(root, fixture);
            await cp(resolve("tests/fixtures/javascript-package-quality", fixture), consumer, {
                recursive: true,
                filter: (path) => !path.includes(".reports"),
            });
            const result = spawnSync(
                process.execPath,
                [resolve("scripts/javascript-package-quality.mjs"), "--workspace", consumer],
                { encoding: "utf8", timeout: 60000, env: process.env },
            );
            assert.ifError(result.error);
            assert.equal(result.status, fixture === "broken-library" ? 1 : 0, result.stderr);
            const report = JSON.parse(
                await readFile(
                    join(consumer, ".reports/egolint/javascript-package-quality.json"),
                    "utf8",
                ),
            );
            assert.deepEqual(report.adapters, {
                oxlint: "1.79.0",
                oxlint_tsgolint: "7.0.2001",
                biome: "2.5.9",
                publint: "0.3.24",
            });
            assert.equal(report.adapter_outcomes.oxlint.protocol_status, "valid");
            assert.equal(report.adapter_outcomes.biome.protocol_status, "valid");
            if (fixture === "broken-library") {
                assert.deepEqual(
                    [...new Set(report.findings.map((item) => item.finding.rule.tool_id))].sort(
                        (left, right) => left.localeCompare(right),
                    ),
                    ["BIOME", "OXLINT", "PUBLINT"],
                );
            } else assert.equal(report.findings.length, 0);
        });
    }
});

test("source coordinates validate independently of protocol status", {
    skip: process.platform === "win32",
}, async (suite) => {
    for (const value of ["bad", 0, -1, null, 1.5]) {
        for (const tool of ["oxlint", "biome"]) {
            await suite.test(`${tool} line ${value}`, async (t) => {
                const diagnostic = {
                    message: "Synthetic warning",
                    severity: "warning",
                    filename: "source.ts",
                    labels: [{ span: { line: value, column: 1 } }],
                };
                const biomeResult = {
                    ...fail,
                    level: "warning",
                    locations: [
                        {
                            physicalLocation: {
                                artifactLocation: { uri: "source.ts" },
                                region: { startLine: value },
                            },
                        },
                    ],
                };
                const fixture =
                    tool === "oxlint"
                        ? { oxlint: { diagnostics: [diagnostic] } }
                        : { biome: sarif([biomeResult]) };
                const { result, report, statuses } = await invoke(t, fixture);
                assert.equal(result.status, 3);
                assert.equal(statuses[tool.toUpperCase()], "execution_error");
                assert.equal(report.findings.length, 0);
            });
        }
    }
    await suite.test("valid positive positions remain findings", async (t) => {
        const { result, report, statuses } = await invoke(t, {
            oxlint: {
                diagnostics: [
                    {
                        message: "Synthetic warning",
                        severity: "warning",
                        filename: "source.ts",
                        labels: [{ span: { line: 2, column: 3, length: 2 } }],
                    },
                ],
            },
        });
        assert.equal(result.status, 0);
        assert.equal(statuses.OXLINT, "passed_with_warnings");
        assert.deepEqual(report.findings[0].finding.location, {
            path: "source.ts",
            start_line: 2,
            start_column: 3,
            end_line: 2,
            end_column: 5,
        });
    });
});
