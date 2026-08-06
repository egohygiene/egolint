/**
 * =============================================================================
 * Ego Hygiene — Universal Prettier Configuration
 * =============================================================================
 *
 * Purpose:
 *   Provide deterministic formatting across supported source, configuration,
 *   documentation, and web file formats.
 *
 * Ownership:
 *   - Prettier owns source layout and whitespace.
 *   - Linters own correctness, safety, and architectural policy.
 *   - EditorConfig owns low-level editor behavior and file integrity.
 *
 * Configuration:
 * https://prettier.io/docs/configuration
 *
 * Options:
 * https://prettier.io/docs/options
 */

/** @type {import("prettier").Config} */
const prettierConfiguration = {
    // -------------------------------------------------------------------------
    // Core formatting
    // -------------------------------------------------------------------------

    printWidth: 100,
    tabWidth: 4,
    useTabs: false,

    semi: true,
    singleQuote: false,
    quoteProps: "as-needed",
    jsxSingleQuote: false,

    trailingComma: "all",

    // -------------------------------------------------------------------------
    // Expressions and delimiters
    // -------------------------------------------------------------------------

    arrowParens: "always",
    bracketSpacing: true,
    bracketSameLine: false,

    // Preserve intentional multiline object literals while allowing compact
    // objects to remain on one line.
    objectWrap: "preserve",

    // -------------------------------------------------------------------------
    // Documents and platform consistency
    // -------------------------------------------------------------------------

    proseWrap: "preserve",
    endOfLine: "lf",
    embeddedLanguageFormatting: "auto",

    // -------------------------------------------------------------------------
    // Language-specific overrides
    // -------------------------------------------------------------------------

    overrides: [
        {
            files: ["*.json", "*.json5", "*.jsonc"],
            options: {
                printWidth: 80,
                tabWidth: 2,
                trailingComma: "none",
            },
        },
        {
            files: ["*.md", "*.markdown", "*.mdx"],
            options: {
                tabWidth: 2,
                proseWrap: "preserve",
            },
        },
        {
            files: ["*.yaml", "*.yml"],
            options: {
                tabWidth: 2,
                singleQuote: false,
            },
        },
        {
            files: ["*.css", "*.less", "*.scss"],
            options: {
                tabWidth: 2,
                singleQuote: false,
            },
        },
        {
            files: ["*.html", "*.vue", "*.svelte"],
            options: {
                tabWidth: 2,
                singleAttributePerLine: false,
            },
        },
        {
            files: ["*.graphql", "*.gql"],
            options: {
                tabWidth: 2,
            },
        },
    ],
};

export default prettierConfiguration;
