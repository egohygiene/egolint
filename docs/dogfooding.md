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
their focused generators and tests instead of treating them as production package source.

## Canonical command

Run the complete local proof with:

```sh
task dogfood
```

The composed task performs three checks in order:

1. `dogfood:native` invokes the public `egolint validate` CLI against this repository.
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
remain blocking in the dedicated Rust core CI lane.

The generic Mypy contract keeps first-party strictness while accepting installed third-party
packages that do not publish type stubs. Secretlint exclusions are forwarded through a temporary
ignore file rooted in the analyzed workspace; packaged absolute ignore paths are intentionally not
used because Secretlint v13 treats them as cascading file names rather than portable policy paths.

That distinction matters: the dogfood proof must evaluate the current checkout, not a previously
published or mutable remote image.

## What the gate proves

A green dogfood run proves that the current repository can:

- decode and resolve its public configuration contract;
- execute native repository policy through the public CLI;
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
