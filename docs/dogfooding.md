# Dogfooding

Egolint is its own reference consumer.

The dogfood gate deliberately crosses the same public boundaries that a downstream repository uses
instead of calling implementation-only helpers.

```text
Egolint repository
      ↓
.config/dogfood/egolint.toml
      ↓
Taskfile public developer surface
      ↓
Egolint CLI
      ↓
local egolint-full:dogfood image
      ↓
MegaLinter + native Egolint adapters
      ↓
normalized .reports/egolint evidence
```

The repository also declares `egolint.javascript-package-quality.json`, so its real JavaScript
tooling is checked through the same package-quality manifest contract as external consumers. The
JavaScript architecture profile includes `scripts/` as a first-class production root. The
repository-owned manifest scope keeps generated contracts and deliberately hostile fixtures under
their focused generators and tests instead of treating them as production package source. Reviewed
Aether and Hygiene projections are excluded from formatters because their owning contracts verify
their exact bytes and digests instead.

## Canonical command

Run the complete local proof with:

```sh
task dogfood
```

The composed task performs three checks in order:

1. `dogfood:native` invokes the public `egolint validate` CLI against this repository, including the
   Hygiene v2 required-file contract, focused continuity comparison and universally discovered
   repository-release declaration.
2. `dogfood:javascript` runs the public dependency-architecture and JavaScript package-quality
   adapters against Egolint's own production tooling.
3. `dogfood:holistic` builds the current checkout's `Dockerfile.full` and invokes the public
   `egolint lint` CLI with the holistic profile.

The holistic self-consumer configuration is `.config/dogfood/egolint.toml`. It pins the runtime to
Docker, the image to the locally built `egolint-full:dogfood` tag, the pull policy to `never`, and
the lint container network to `none`. It also selects the image's preinstalled stable Rust
toolchain, avoiding an implicit rustup network check during the offline run.

The offline reference-consumer task explicitly disables SchemaStore validation, remote-link
checking, package-registry takeover checks, vulnerability-database scanners, and dependency-backed
compiler analysis. Those analyzers require network access, reviewed pre-seeded data, or vendored
consumer dependencies, so an offline failure cannot establish their quality or freshness. Their
policy remains active in the corresponding network-enabled profiles; the dogfood gate records the
bounded offline selection directly in its public execution plan. Rust formatting, Clippy, and tests
remain blocking in the dedicated Rust core CI lane. The workflow uses a full Git checkout so the
checkpoint's explicit base remains locally available; neither the task nor the validator silently
fetches history.

The generic Mypy contract keeps first-party strictness while accepting installed third-party
packages that do not publish type stubs. Secretlint consumes MegaLinter's complete filtered file
list instead of project mode, so the scanner honors the same exclusions without generating an ignore
file inside the read-only consumer workspace.

That distinction matters: the dogfood proof must evaluate the current checkout, not a previously
published or mutable remote image.

## What the gate proves

A green dogfood run proves that the current repository can:

- decode and resolve its public configuration contract;
- execute native repository policy through the public CLI;
- validate the root continuity checkpoint, managed agent block, immutable upstream projections, and
  explicit base/head evidence in observe mode;
- report its incubating release profile with six passed checks and the existing manual-workflow
  migration finding retained as a warning, separately from process success;
- analyze its own JavaScript tooling through the public package-quality manifest;
- analyze JavaScript dependency structure through the public architecture adapter;
- build the full lint-engine image from the same checkout;
- launch that image through Egolint's hardened host orchestration boundary;
- run the holistic MegaLinter-backed profile against the repository;
- normalize results into the public `.reports/egolint` evidence surface; and
- do all of that without granting lint execution broad repository writes or network access.

The GitHub Actions `Dogfood` workflow runs this same `task dogfood` entrypoint and uploads
`.reports/egolint/` as diagnostic evidence even when the gate fails. That private evidence includes
a bounded `mega-linter-adapter.log`, so startup and configuration failures remain diagnosable
without forwarding raw container output into workflow command channels.

## What the gate does not replace

Dogfooding complements rather than replaces component-level verification. Unit tests, schema drift
checks, fixture tests, platform-specific rule tests, and policy-image tests remain necessary because
a self-consumer run cannot force every error branch or compatibility case.

The self-run also does not dynamically fetch Hygiene, Empathy, or another sibling repository.
Cross-repository contracts remain reviewed, locally materialized inputs so lint execution stays
offline and reproducible.

## Findings and exceptions

A dogfood finding is treated as product feedback first. Prefer, in order:

1. fix the source or configuration;
2. refactor the architecture so the rule models the intended boundary correctly;
3. narrow a rule when the previous rule was objectively over-broad; or
4. use an explicit owned, reasoned, time-bounded exception when migration genuinely requires one.

Do not add Egolint-specific blanket exclusions merely to keep the self-run green. A dogfood
exception that downstream users could not reasonably justify is evidence that the policy or
architecture still needs work.

## Current release self-adoption boundary

`.egohygiene/release.json` records the early-alpha repository as incubating/advisory with one Cargo
version authority and no claimed release. The actual tag-push publisher remains unchanged; its manual
entrypoint only rehearses. Native evidence therefore remains `advisory` with a failed
`manual_workflow` check, even though the nonblocking process exits 0. A green local command cannot
upgrade that finding to conformance or publication readiness.

The broader native self-run currently also retains an `invalid` focused Intelligence result: ADR
evidence is explicitly unknown and six other roadmap steps still have pre-existing combined
state/dependency formatting warnings. Issue #101 owns that source/navigation follow-up. This candidate
updates only EGL-Q10's own section and does not claim repository-wide semantic conformance. The
continuity result remains valid with the expected unreleased-owner-contract observe advisory.

The planner and preparation helper provide local human review artifacts; the source-only
`release:publish` Task dispatches the existing rehearsal. The self-consumer tests invoke native
validation and the helper directly, without executing any release Task or workflow. See
[release validation](repository-release-validation.md#egolint-self-consumer) for the exact evidence
and [release design](releasing.md) for the separately gated publisher migration. Hosted Actions,
full-image and non-Linux qualification are deferred for this draft source checkpoint; issue #69 and
parent #29 retain their original acceptance and maintainer gates.
