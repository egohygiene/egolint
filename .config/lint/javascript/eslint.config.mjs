// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

import { createRequire } from "node:module";

// MegaLinter exposes its bundled plugins through NODE_PATH. Native ESM package
// resolution ignores that path, while createRequire deliberately honors it.
const require = createRequire(import.meta.url);
const js = require("@eslint/js");
const jsonPlugin = (() => {
    try {
        return require("@eslint/json");
    } catch (error) {
        if (
            error?.code === "MODULE_NOT_FOUND" &&
            String(error.message).includes("@eslint/json")
        ) {
            return null;
        }
        throw error;
    }
})();
const typescriptEslintPlugin = require("@typescript-eslint/eslint-plugin");
const typescriptParser = require("@typescript-eslint/parser");
const reactPlugin = require("eslint-plugin-react");
const { defineConfig, globalIgnores } = require("eslint/config");
const globals = require("globals");

const javascriptFiles = ["**/*.{js,jsx,mjs,cjs}"];
const typescriptFiles = ["**/*.{ts,tsx,mts,cts}"];
const reactFiles = ["**/*.{jsx,tsx}"];
const testFiles = [
    "**/*.{test,spec}.{js,jsx,ts,tsx}",
    "**/test/**/*.{js,jsx,ts,tsx}",
    "**/tests/**/*.{js,jsx,ts,tsx}",
    "**/__tests__/**/*.{js,jsx,ts,tsx}",
];
const jsonConfigs = jsonPlugin
    ? [
          // -----------------------------------------------------------------------------
          // JSON
          // -----------------------------------------------------------------------------

          {
              files: ["**/*.json"],
              plugins: {
                  json: jsonPlugin,
              },
              language: "json/json",
              rules: {
                  ...jsonPlugin.configs.recommended.rules,
              },
          },

          // -----------------------------------------------------------------------------
          // JSON with comments
          // -----------------------------------------------------------------------------

          {
              files: ["**/*.jsonc"],
              plugins: {
                  json: jsonPlugin,
              },
              language: "json/jsonc",
              rules: {
                  ...jsonPlugin.configs.recommended.rules,
              },
          },

          // -----------------------------------------------------------------------------
          // JSON5
          // -----------------------------------------------------------------------------

          {
              files: ["**/*.json5"],
              plugins: {
                  json: jsonPlugin,
              },
              language: "json/json5",
              rules: {
                  ...jsonPlugin.configs.recommended.rules,
              },
          },
      ]
    : [];

export default defineConfig([
    globalIgnores(
        [
            "**/node_modules/**",
            "**/.pnpm/**",
            "**/.yarn/**",

            "**/dist/**",
            "**/build/**",
            "**/coverage/**",
            "**/out/**",
            "**/generated/**",

            "**/.cache/**",
            "**/.eslintcache",
            "**/.next/**",
            "**/.nuxt/**",
            "**/.nyc_output/**",
            "**/.output/**",
            "**/.turbo/**",
            "**/.vitest/**",

            "**/cypress/screenshots/**",
            "**/cypress/videos/**",
            "**/playwright-report/**",
            "**/test-results/**",

            "**/*.min.js",
            "**/*.min.mjs",
            "**/*.tsbuildinfo",
            "**/*.map",
            "**/*.log",

            "**/.devcontainer/target/**",
            "**/.history/**",
            "**/.idea/**",
            "**/.vscode/**",
            "**/*.swp",
            "**/*.swo",
        ],
        "Ignore dependencies, generated files, build output, caches, and local tooling state",
    ),
    ...jsonConfigs,

    // -------------------------------------------------------------------------
    // Core JavaScript baseline
    // -------------------------------------------------------------------------

    {
        ...js.configs.recommended,
        files: javascriptFiles,

        languageOptions: {
            ecmaVersion: "latest",
            sourceType: "module",
            globals: {
                ...globals.es2024,
                ...globals.browser,
                ...globals.node,
            },
        },

        rules: {
            ...js.configs.recommended.rules,

            eqeqeq: ["error", "always"],
            "no-console": "off",
            "no-constant-binary-expression": "error",
            "no-duplicate-imports": "error",
            "no-promise-executor-return": "error",
            "no-self-compare": "error",
            "no-template-curly-in-string": "error",
            "no-unmodified-loop-condition": "error",
            "no-unreachable-loop": "error",
            "no-unused-private-class-members": "error",
            "no-use-before-define": [
                "error",
                {
                    functions: false,
                    classes: true,
                    variables: true,
                },
            ],
            "no-useless-assignment": "error",
            "object-shorthand": ["error", "always"],
            "prefer-const": "error",
            "prefer-object-has-own": "error",
            "prefer-promise-reject-errors": "error",
            "prefer-template": "error",
        },
    },

    // -------------------------------------------------------------------------
    // TypeScript baseline
    // -------------------------------------------------------------------------

    {
        files: typescriptFiles,

        languageOptions: {
            parser: typescriptParser,

            parserOptions: {
                ecmaVersion: "latest",
                sourceType: "module",

                // Resolve type information using the same TypeScript project
                // service used by editors. This avoids requiring one universal
                // tsconfig.eslint.json at the repository root.
                projectService: true,

                ecmaFeatures: {
                    jsx: true,
                },
            },

            globals: {
                ...globals.es2024,
                ...globals.browser,
                ...globals.node,
            },
        },

        plugins: {
            "@typescript-eslint": typescriptEslintPlugin,
        },

        rules: {
            ...typescriptEslintPlugin.configs.recommended.rules,

            // TypeScript performs these checks more accurately.
            "no-undef": "off",
            "no-unused-vars": "off",
            "no-use-before-define": "off",

            "@typescript-eslint/consistent-type-exports": "error",
            "@typescript-eslint/consistent-type-imports": [
                "error",
                {
                    prefer: "type-imports",
                    fixStyle: "separate-type-imports",
                },
            ],
            "@typescript-eslint/no-explicit-any": "warn",
            "@typescript-eslint/no-import-type-side-effects": "error",
            "@typescript-eslint/no-unused-vars": [
                "error",
                {
                    argsIgnorePattern: "^_",
                    caughtErrors: "all",
                    caughtErrorsIgnorePattern: "^_",
                    destructuredArrayIgnorePattern: "^_",
                    ignoreRestSiblings: true,
                    varsIgnorePattern: "^_",
                },
            ],
            "@typescript-eslint/prefer-as-const": "error",
        },
    },

    // -------------------------------------------------------------------------
    // React JSX and TSX
    // -------------------------------------------------------------------------

    {
        files: reactFiles,

        plugins: {
            react: reactPlugin,
        },

        languageOptions: {
            parserOptions: {
                ecmaFeatures: {
                    jsx: true,
                },
            },
        },

        settings: {
            react: {
                version: "detect",
            },
        },

        rules: {
            ...reactPlugin.configs.recommended.rules,

            // React 17+ JSX transform does not require importing React.
            "react/jsx-uses-react": "off",
            "react/react-in-jsx-scope": "off",

            "react/jsx-boolean-value": ["error", "never"],
            "react/jsx-key": "error",
            "react/jsx-no-comment-textnodes": "error",
            "react/jsx-no-duplicate-props": "error",
            "react/jsx-no-target-blank": [
                "error",
                {
                    allowReferrer: false,
                    enforceDynamicLinks: "always",
                },
            ],
            "react/no-array-index-key": "warn",
            "react/no-danger": "warn",
            "react/no-unknown-property": "error",
            "react/self-closing-comp": [
                "error",
                {
                    component: true,
                    html: true,
                },
            ],
        },
    },

    // -------------------------------------------------------------------------
    // Tests
    // -------------------------------------------------------------------------

    {
        files: testFiles,

        languageOptions: {
            globals: {
                ...globals.vitest,
                ...globals.jest,
                ...globals.mocha,
            },
        },

        rules: {
            "@typescript-eslint/no-explicit-any": "off",
            "no-console": "off",
        },
    },

    // -------------------------------------------------------------------------
    // CommonJS configuration and tooling files
    // -------------------------------------------------------------------------

    {
        files: ["**/*.cjs", "**/*.cts", "**/scripts/**/*.{js,ts,cjs,mjs,cts,mts}"],

        languageOptions: {
            sourceType: "commonjs",
            globals: {
                ...globals.node,
            },
        },
    },
]);
