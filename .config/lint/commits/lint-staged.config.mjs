// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

const prettierConfiguration = ".config/lint/javascript/prettier.config.mjs";
const eslintConfiguration = ".config/lint/javascript/eslint.config.mjs";
const stylelintConfiguration = ".config/lint/css/stylelint.json";

function shellQuote(value) {
    return `'${value.replaceAll("'", `'\\''`)}'`;
}

function joinFilenames(filenames) {
    return filenames.map(shellQuote).join(" ");
}

function prettierCommand(filenames) {
    return [
        "prettier",
        `--config ${shellQuote(prettierConfiguration)}`,
        "--ignore-unknown",
        "--write",
        joinFilenames(filenames),
    ].join(" ");
}

function eslintCommand(filenames) {
    return [
        "eslint",
        `--config ${shellQuote(eslintConfiguration)}`,
        "--fix",
        "--max-warnings 0",
        joinFilenames(filenames),
    ].join(" ");
}

function stylelintCommand(filenames) {
    return [
        "stylelint",
        `--config ${shellQuote(stylelintConfiguration)}`,
        "--fix",
        joinFilenames(filenames),
    ].join(" ");
}

export default {
    "*.{cjs,cts,js,jsx,mjs,mts,ts,tsx}": [eslintCommand, prettierCommand],
    "*.{css,less,pcss,postcss,scss,sass}": [stylelintCommand, prettierCommand],
    "*.{graphql,html,json,json5,jsonc,md,mdx,yaml,yml}": prettierCommand,
};
