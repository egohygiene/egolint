# JavaScript package quality

Egolint owns a versioned JavaScript/TypeScript package-quality policy and treats
Oxlint, Biome, and publint as replaceable analysis adapters.

## Ownership

| Surface | Canonical owner | Adapter |
| --- | --- | --- |
| JavaScript/TypeScript semantic lint | Egolint | Oxlint 1.79.0 |
| Type-aware lint rules | Egolint | oxlint-tsgolint 7.0.2001 through Oxlint |
| Formatting | Egolint | Biome 2.5.9 |
| Import/export organization | Egolint | Biome `organizeImports` |
| npm package publishability | Egolint | publint 0.3.24 |
| Dependency architecture | Egolint | dependency-cruiser profile from issue #12 |

Biome's linter is disabled by policy. Oxlint therefore owns semantic linting,
while Biome owns deterministic formatting and safe import organization. Legacy
`no-duplicate-imports` coverage is intentionally assigned to Biome rather than
duplicated in Oxlint.

The profile does not use Oxlint's JavaScript-plugin bridge. The required React,
React Hooks, import, jsx-a11y, Vitest, Node, Promise, TypeScript, and react-perf
plugins are native Oxlint plugins.

### React Compiler boundary

Oxlint 1.79 includes React Compiler-powered lint rules in the native React
plugin. The default Egolint `react-library` profile keeps those experimental
compiler diagnostics explicitly disabled, even when Oxlint categories would
otherwise enable them. Traditional React, Hooks, accessibility, testing, and
performance rules remain enabled.

React Compiler diagnostics belong in a future explicit opt-in profile. They are
not silently inherited by the baseline package-quality contract.

## Consumer manifest

A repository opts into the profile with
`egolint.javascript-package-quality.json`:

```json
{
  "$schema": "path/to/javascript-package-quality-manifest.schema.json",
  "schema_version": 1,
  "profile": "react-library",
  "package_path": ".",
  "publication": "npm"
}
```

`profile` is either `base` or `react-library`.

`publication` is deliberately explicit:

- `npm` runs publint and rejects `package.json#private=true`.
- `private` requires `package.json#private=true` and records publint as
  `not_applicable`.

Egolint does not infer publication intent from incidental package metadata.

## Canonical command

Local development, editor tasks, hooks, and CI should call the same command:

```sh
node scripts/javascript-package-quality.mjs --workspace "."
```

The Taskfile wrapper is:

```sh
task javascript-quality:check JAVASCRIPT_QUALITY_WORKSPACE="."
```

The command is check-only. It never mutates source files. JSON and SARIF are
written beneath `.reports/egolint/` only.

### Editor task

A VS Code task can invoke the canonical command without creating another policy
surface:

```json
{
  "label": "egolint: javascript package quality",
  "type": "process",
  "command": "node",
  "args": [
    "scripts/javascript-package-quality.mjs",
    "--workspace",
    "${workspaceFolder}"
  ],
  "problemMatcher": []
}
```

Biome's editor extension may format and organize imports on save when it points
to `.config/lint/javascript/biome.package-quality.json`. Oxlint editor linting
must use the matching versioned Oxlint variant. Editor convenience does not
replace the canonical Egolint command in CI.

### Git hook

A pre-commit or pre-push adapter should invoke exactly:

```sh
node scripts/javascript-package-quality.mjs --workspace "."
```

Hooks should not invoke ESLint, Prettier, Biome lint rules, or publint
independently when this profile is enabled. That prevents local and CI policy
drift.

## Findings

Egolint normalizes adapter output into one deterministic report:

- `.reports/egolint/javascript-package-quality.json`
- `.reports/egolint/javascript-package-quality.sarif`

Every normalized finding records:

- Egolint `Finding` identity, severity, message, location, ownership, and
  fingerprint fields;
- source tool and exact source-tool version;
- selected profile mapping;
- fix safety;
- applicability;
- suppression state.

Adapter formats are deliberately isolated:

- Oxlint uses its documented JSON diagnostics.
- Biome uses SARIF rather than its experimental JSON reporter.
- publint uses its JavaScript API.

The combined SARIF file is an Egolint projection; consumers do not need to know
which adapter produced a finding.

## Fix policy

The default profile is check-only.

- Biome formatter and `organizeImports` findings are classified as safe-fix
  candidates, but are not applied automatically.
- Oxlint fixes are not requested by the package-quality command.
- publint findings are advisory/remediation evidence and are never auto-fixed.
- Dangerous or semantic-changing fixes require a separate reviewed workflow.

This keeps source mutation outside normal lint execution and leaves room for
Egolint's bounded fix-preview contract to absorb safe JavaScript fixes later.

## Migration from ESLint and Prettier

`.config/rules/javascript-eslint-migration.v1.json` is the migration ledger.
It records every intentional custom rule from the current ESLint profile and
its new owner. The existing ESLint and Prettier configuration remains in the
repository temporarily as compatibility evidence; the new package-quality
profile does not execute either tool.

The migration is complete only when:

1. the ledger accounts for every intentional legacy rule;
2. the native Oxlint configs load successfully and preserve reviewed rule
   options in both base and React variants;
3. the React variant proves its compiler-rule firewall;
4. Biome formatting matches the reviewed Prettier conventions that matter to
   the repository;
5. consumer fixtures remain clean;
6. the broken fixture produces normalized findings from Oxlint, Biome, and
   publint.

Removing legacy dependencies is a follow-up cleanup after those compatibility
signals are established across real consumers.

## React-library validation

The disposable React-library fixture proves the complete contract:

1. Oxlint parses TS/TSX and loads native React, Hooks, accessibility, import,
   test, and performance rules.
2. oxlint-tsgolint provides type-aware rule support while the React Compiler
   rule family remains explicitly disabled.
3. Biome validates deterministic formatting and import organization with its
   linter disabled.
4. Node's built-in test runner smoke-tests the published JavaScript entrypoint.
5. Egolint's dependency architecture profile from issue #12 runs against the
   same fixture.
6. publint validates the explicit npm publication surface.
7. Egolint emits normalized JSON and SARIF.

A separate private application fixture proves that publint remains
`not_applicable`, and a deliberately nonconforming library fixture proves that
all three adapter families can contribute findings to one report.

## Downstream materialization

Holon should materialize the consumer manifest and pinned toolchain wiring into
appropriate JavaScript blueprints. Relay should call the canonical Egolint
command in reusable CI. Neither downstream system should fork the actual rule
policy; versioned policy remains owned by Egolint.
