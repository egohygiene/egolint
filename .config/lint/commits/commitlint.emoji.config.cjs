// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

const {
    allowedTypes,
    conventionalEmojiHeaderPattern,
    subjectStartsWithLowercase,
    validateCommitHeader,
} = require("./commit-policy.cjs");

/** @type {import("@commitlint/types").UserConfig} */
module.exports = {
    defaultIgnores: false,
    extends: ["@commitlint/config-conventional"],
    parserPreset: {
        parserOpts: {
            headerCorrespondence: ["type", "scope", "emoji", "subject"],
            headerPattern: conventionalEmojiHeaderPattern,
        },
    },
    plugins: [
        {
            rules: {
                "emoji-conventional-header": ({ raw }) => validateCommitHeader(raw),
                "subject-starts-lowercase": ({ subject }) => [
                    subjectStartsWithLowercase(subject ?? ""),
                    "Commit subjects must begin with a lowercase letter",
                ],
            },
        },
    ],
    rules: {
        "body-max-line-length": [2, "always", 100],
        "emoji-conventional-header": [2, "always"],
        "footer-max-line-length": [2, "always", 100],
        "header-max-length": [2, "always", 100],
        "scope-empty": [2, "never"],
        "subject-empty": [2, "never"],
        "subject-full-stop": [2, "never", "."],
        "subject-starts-lowercase": [2, "always"],
        "type-empty": [2, "never"],
        "type-enum": [2, "always", allowedTypes],
    },
};
