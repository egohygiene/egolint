// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
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
async function invoke(t, { oxlint = cleanOxlint, biome = sarif(), exit = {}, raw = {} } = {}) {
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
            publication: "private",
        }),
    );
    await writeFile(join(consumer, "package.json"), JSON.stringify({ private: true }));
    const stub = join(bin, "pnpm");
    await writeFile(
        stub,
        `#!${process.execPath}
const tool = process.argv.includes("oxlint") ? "oxlint" : "biome";
const fixture = JSON.parse(process.env.EGOLINT_TEST_ADAPTERS);
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
                PATH: `${bin}${delimiter}${process.env.PATH}`,
                EGOLINT_TEST_ADAPTERS: JSON.stringify({ oxlint, biome, exit, raw }),
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
}, async (t) => {
    for (const result of [
        null,
        { ...fail, kind: "pass", level: "none" },
        { ruleId: "format", kind: "pass", message: { text: "Successful check" } },
        { ...fail, kind: "notApplicable", level: "none" },
    ]) {
        await t.test(JSON.stringify(result), async (t) => {
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
}, async (t) => {
    for (const [name, fixture, tool] of invalidCases) {
        await t.test(name, async (t) => {
            const { result, report, statuses } = await invoke(t, fixture);
            assert.equal(result.status, 3, result.stderr);
            assert.equal(statuses[tool], "execution_error");
            assert.equal(statuses[tool === "OXLINT" ? "BIOME" : "OXLINT"], "passed");
            assert.equal(report.findings.length, 0, "protocol errors are not lint findings");
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
