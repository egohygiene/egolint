// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { resolve } = require("node:path");
const { spawnSync } = require("node:child_process");
const test = require("node:test");

const {
    allowedTypes,
    parseCommitHeader,
    validateCommitHeader,
} = require("../.config/lint/commits/commit-policy.cjs");

const fixturePath = resolve(__dirname, "fixtures/commitlint/messages.json");
const fixtures = JSON.parse(readFileSync(fixturePath, "utf8"));

test("accepts every canonical valid commit fixture", () => {
    for (const message of fixtures.valid) {
        assert.deepEqual(validateCommitHeader(message), [
            true,
            "Commit header follows the canonical emoji policy",
        ]);
        assert.ok(parseCommitHeader(message));
    }
});

test("rejects every canonical invalid commit fixture", () => {
    for (const { message } of fixtures.invalid) {
        const [valid] = validateCommitHeader(message);
        assert.equal(valid, false, message);
    }
});

test("exports a unique standalone type catalog", () => {
    assert.ok(allowedTypes.includes("chore"));
    assert.ok(allowedTypes.includes("feat"));
    assert.equal(new Set(allowedTypes).size, allowedTypes.length);
});

test("Commitlint enforces every canonical fixture", () => {
    const commitlintExecutable = process.platform === "win32" ? "commitlint.cmd" : "commitlint";
    const commitlintConfiguration = resolve(
        __dirname,
        "../.config/lint/commits/commitlint.emoji.config.cjs",
    );

    for (const message of fixtures.valid) {
        const result = spawnSync(commitlintExecutable, ["--config", commitlintConfiguration], {
            encoding: "utf8",
            input: `${message}\n`,
        });
        assert.equal(result.status, 0, `${message}\n${result.stderr}${result.stdout}`);
    }

    for (const { message, reason } of fixtures.invalid) {
        const result = spawnSync(commitlintExecutable, ["--config", commitlintConfiguration], {
            encoding: "utf8",
            input: `${message}\n`,
        });
        assert.notEqual(result.status, 0, `${reason}: ${message}`);
    }
});
