# JavaScript and TypeScript architecture profile

Egolint owns the architecture policy; dependency-cruiser is a pinned graph-analysis adapter.

## Boundary

```text
Hygiene / repository architecture
        ↓
Egolint profile + repository overlay
        ↓
dependency-cruiser 18.2.0
        ↓
normalized Egolint JSON + SARIF (+ optional DOT)
```

The canonical profile is `.config/rules/javascript-architecture.v1.json` and is embedded in the
Egolint binary. Consumer repositories do not copy or edit it. Repository-specific policy is
expressed through a versioned overlay containing only additional rules and owned exceptions.

## Canonical v1 rule families

- dependency cycles;
- unresolved dependencies;
- reusable package → application coupling;
- application → sibling application coupling;
- cross-feature/domain implementation imports;
- application deep imports that bypass package public entry points;
- browser-facing code → Node core modules;
- production → test/fixture dependencies;
- production → generated/build internals;
- orphan review as advisory evidence.

Complexity thresholds such as maximum fan-out/depth are intentionally absent from v1 until evidence
justifies them.

## Node package resolution

The generated dependency-cruiser configuration resolves modern Node package export maps, including
subpath exports. It reviews the `exports` manifest field under the `import`, `require`, `node`, and
`default` conditions so the canonical adapter works across Egolint's supported JavaScript and
TypeScript module surface without package-specific ignores.

The adapter intentionally retains dependency-cruiser's environment-derived extension ordering and
enhanced-resolve's default `main` handling. Consumer toolchains determine the supported parser
extensions, while packages that publish type-only entry points can opt into additional `mainFields`
through a future reviewed profile contract if evidence requires it.

## Run locally

Install the locked Node toolchain, then run:

```sh
corepack enable pnpm
pnpm install --frozen-lockfile --ignore-scripts
cargo run --locked --bin egolint-architecture -- \
  --workspace "." \
  --evaluation-date "2026-08-23"
```

Taskfile convenience:

```sh
task architecture:check EVALUATION_DATE="2026-08-23"
```

Optional graph evidence:

```sh
task architecture:graph EVALUATION_DATE="2026-08-23"
```

Outputs remain under `.reports/egolint/`:

- `javascript-architecture.json`
- `javascript-architecture.sarif`
- optional `javascript-architecture.dot`

## Repository overlays

Pass one or more workspace-relative overlays:

```sh
cargo run --locked --bin egolint-architecture -- \
  --workspace "." \
  --overlay ".egolint/javascript-architecture.json" \
  --evaluation-date "2026-08-23"
```

An overlay may add rules but cannot replace a canonical rule ID. Exceptions require an ID, rule ID,
owner, reason, and expiry date, and may narrow to an exact source/target edge. Egolint applies
exceptions only _after_ dependency-cruiser returns evidence, keeping architecture exceptions visible
in normalized reports instead of hiding them in an adapter baseline.

Expired exceptions never suppress a finding and make the architecture gate fail. Current but
unmatched exceptions remain visible for cleanup.

## Finding contract

Every adapter violation becomes the existing shared Egolint `Finding` contract and additionally
records:

- source module;
- target module when applicable;
- dependency/cycle path when supplied by the adapter;
- canonical Egolint rule ID and policy source;
- exact dependency-cruiser version;
- remediation guidance;
- suppression/exception ID when applicable.

This keeps Relay, SARIF consumers, hooks, editors, and future Observatory projections dependent on
Egolint contracts rather than dependency-cruiser output details.

## Versioning

The v1 profile pins dependency-cruiser `18.2.0`. Changing the adapter major version, rule semantics,
normalized output semantics, or overlay contract requires explicit review and corresponding
profile/schema versioning. Consumer repositories should use the same pinned adapter version locally
and in CI.
