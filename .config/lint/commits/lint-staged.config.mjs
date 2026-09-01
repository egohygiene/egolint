// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

import { relative } from "node:path";

const prettierConfiguration = ".config/lint/javascript/prettier.config.mjs";
const stylelintConfiguration = ".config/lint/css/stylelint.json";
const immutablePathPrefixes = ["assets/identity/", "vendor/hygiene/"];

function shellQuote(value) {
    return `'${value.replaceAll("'", `'\\''`)}'`;
}

function joinFilenames(filenames) {
    return filenames.map(shellQuote).join(" ");
}

function editableFilenames(filenames) {
    return filenames.filter((filename) => {
        const repositoryPath = relative(process.cwd(), filename).replaceAll("\\", "/");
        return !immutablePathPrefixes.some((prefix) => repositoryPath.startsWith(prefix));
    });
}

function prettierCommand(filenames) {
    const editable = editableFilenames(filenames);
    if (editable.length === 0) {
        return [];
    }
    return [
        "prettier",
        `--config ${shellQuote(prettierConfiguration)}`,
        "--ignore-unknown",
        "--write",
        joinFilenames(editable),
    ].join(" ");
}

function stylelintCommand(filenames) {
    const editable = editableFilenames(filenames);
    if (editable.length === 0) {
        return [];
    }
    return [
        "stylelint",
        `--config ${shellQuote(stylelintConfiguration)}`,
        "--fix",
        joinFilenames(editable),
    ].join(" ");
}

export default {
    "*.{css,less,pcss,postcss,scss,sass}": [stylelintCommand, prettierCommand],
    "*.{graphql,html,json,json5,jsonc,md,mdx,yaml,yml}": prettierCommand,
};
