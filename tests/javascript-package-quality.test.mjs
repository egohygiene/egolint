import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

async function json(path) {
    return JSON.parse(await readFile(path, "utf8"));
}

const expectedLegacyRules = [
    "eqeqeq",
    "no-constant-binary-expression",
    "no-duplicate-imports",
    "no-promise-executor-return",
    "no-self-compare",
    "no-template-curly-in-string",
    "no-unmodified-loop-condition",
    "no-unreachable-loop",
    "no-unused-private-class-members",
    "no-use-before-define",
    "no-useless-assignment",
    "object-shorthand",
    "prefer-const",
    "prefer-object-has-own",
    "prefer-promise-reject-errors",
    "prefer-template",
    "@typescript-eslint/consistent-type-exports",
    "@typescript-eslint/consistent-type-imports",
    "@typescript-eslint/no-explicit-any",
    "@typescript-eslint/no-import-type-side-effects",
    "@typescript-eslint/no-unused-vars",
    "@typescript-eslint/prefer-as-const",
    "react/jsx-boolean-value",
    "react/jsx-key",
    "react/jsx-no-comment-textnodes",
    "react/jsx-no-duplicate-props",
    "react/jsx-no-target-blank",
    "react/no-array-index-key",
    "react/no-danger",
    "react/no-unknown-property",
    "react/self-closing-comp",
].sort();

test("profile assigns non-overlapping adapter ownership", async () => {
    const profile = await json(".config/rules/javascript-package-quality.v1.json");
    assert.equal(profile.schema_version, 1);
    assert.equal(profile.adapters.oxlint.version, "1.79.0");
    assert.equal(profile.adapters.oxlint_tsgolint.version, "7.0.2001");
    assert.equal(profile.adapters.biome.version, "2.5.9");
    assert.equal(profile.adapters.publint.version, "0.3.24");
    assert.equal(profile.adapters.biome.linter_enabled, false);
    assert.equal(profile.adapters.biome.output, "sarif");
    assert.equal(profile.adapters.publint.applicability, "manifest_publication_npm");
    assert.deepEqual(
        new Set(Object.values(profile.adapters).map((adapter) => adapter.owner)).size,
        Object.keys(profile.adapters).length,
    );
});

test("Oxlint uses native plugins rather than the alpha JavaScript bridge", async () => {
    const base = await json(".config/lint/javascript/oxlint.base.json");
    const react = await json(".config/lint/javascript/oxlint.react-library.json");
    assert.equal(base.jsPlugins, undefined);
    assert.equal(react.jsPlugins, undefined);
    assert.equal(base.options.typeAware, true);
    assert.equal(base.options.typeCheck, false);
    assert.ok(react.plugins.includes("react"));
    assert.ok(react.plugins.includes("jsx-a11y"));
    assert.ok(react.plugins.includes("vitest"));
    assert.ok(react.plugins.includes("react-perf"));
    assert.equal(react.rules["react/react-compiler"], undefined);
});

test("Biome owns formatting and imports without enabling duplicate lint rules", async () => {
    const biome = await json(".config/lint/javascript/biome.package-quality.json");
    assert.equal(biome.formatter.enabled, true);
    assert.equal(biome.linter.enabled, false);
    assert.equal(biome.javascript.linter.enabled, false);
    assert.equal(biome.json.linter.enabled, false);
    assert.equal(biome.assist.actions.source.organizeImports, "on");
    assert.equal(biome.javascript.formatter.lineWidth, 100);
    assert.equal(biome.json.formatter.lineWidth, 80);
});

test("migration ledger accounts for every intentional legacy ESLint rule", async () => {
    const migration = await json(".config/rules/javascript-eslint-migration.v1.json");
    assert.deepEqual(Object.keys(migration.legacy_rules).sort(), expectedLegacyRules);
    for (const replacement of Object.values(migration.legacy_rules)) {
        assert.match(replacement, /^oxlint:/);
    }
    assert.equal(migration.policy.biome_linter, "disabled");
    assert.equal(migration.policy.alpha_oxlint_js_plugins, "forbidden_when_native_plugin_exists");
});

test("consumer manifests make publint applicability explicit", async () => {
    const publishable = await json(
        "tests/fixtures/javascript-package-quality/react-library/egolint.javascript-package-quality.json",
    );
    const privateApp = await json(
        "tests/fixtures/javascript-package-quality/private-app/egolint.javascript-package-quality.json",
    );
    assert.equal(publishable.profile, "react-library");
    assert.equal(publishable.publication, "npm");
    assert.equal(privateApp.profile, "base");
    assert.equal(privateApp.publication, "private");
});
