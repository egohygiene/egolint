// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

import assert from "node:assert/strict";
import test from "node:test";

import eslintConfiguration from "../.config/lint/javascript/eslint.config.mjs";

test("loads every JSON language from the package's default export", () => {
    assert.ok(Array.isArray(eslintConfiguration));

    const configuredLanguages = eslintConfiguration
        .map((configuration) => configuration.language)
        .filter((language) => typeof language === "string");

    assert.deepEqual(
        configuredLanguages.filter((language) => language.startsWith("json/")),
        ["json/json", "json/jsonc", "json/json5"],
    );
});
