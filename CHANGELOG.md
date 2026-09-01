# Changelog

All notable changes will be documented here. This project follows Keep a Changelog and intends to
use Semantic Versioning once its public contracts stabilize.

## [Unreleased]

### Added

- Repository-owned JavaScript package-quality ignore scopes projected consistently into Oxlint and
  Biome.
- Initial Rust CLI for configuration resolution, planning, runtime checks, and Docker/Podman
  orchestration.
- Fast, holistic, security, and dependency-debt MegaLinter policy profiles.
- Separate lightweight CLI and full lint-engine image definitions.
- Initial cargo-dist, security, configuration, and release documentation.
- Versioned config, profile, finding, suppression, evidence, tool-result, execution-plan,
  normalized-report, and repository-contract JSON Schemas.
- First-class `validate`, `lint`, `explain`, and `doctor` commands with compatibility aliases for
  the initial alpha command surface.
- Canonical executable Empathy and Hygiene repository-contract artifacts pinned to immutable
  semantic source commits and recorded in a checksum-bearing install manifest.
- Standalone MegaLinter and complementary-tool contract inventories with positive and negative
  fixtures.
- SHA-pinned CI validation for Rust, policy contracts, schemas, packaging, and the lightweight OCI
  image.
- Native portability/repository-contract rules, expiring suppressions, normalized SARIF, compact
  dependency-debt evidence, and editor/pre-commit integrations.
- Immutable-tree fix previews plus digest/base/post-tree-gated, index-aware reviewed patch
  application.
- Versioned offline Repository Intelligence validation for ADR contracts and lineage, roadmap
  graphs, declared links, and optional commit trailers, with incremental adoption profiles and
  normalized remediation evidence.
- Versioned offline repository-presentation validation that consumes the pinned Hygiene profile and
  Identity manifest, resolves visibility/lifecycle applicability, fails badge claims closed, and
  emits privacy-safe evidence.

### Changed

- Made the fast profile's mixed scope explicit: adapter linting targets changed files while native
  policy checks inspect the complete repository; legacy version-1 `changed_files` contracts remain
  readable.
- Aligned built-in profile purposes with the canonical MegaLinter policy and documented the boundary
  between structural JSON Schema checks and Rust/CLI semantic validation.
- Expanded run reports with honest completeness, normalized summary, finding, suppression, per-tool
  result, evidence, and ownership contracts.
- Made MegaLinter tool ownership, policy source, and evidence structured in the generated 124-tool
  inventory.
- Rebased the complete non-generated lint subsystem from Empathy into an independent repository
  contract.
- Removed generated Empathy evidence, operating-system metadata, and language caches from version
  control.

### Fixed

- Resolved the repository-owned holistic baseline across Markdown, Python, package metadata, shell,
  YAML, repository naming, and secret-scan policy while preserving advisory findings for explicit
  follow-up.
- Made per-run linter overrides additive to built-in profile selection, preventing one
  `--disable-linter` from replacing the profile's canonical quarantine list and re-enabling tools
  that have not completed baseline review.
- Made offline dogfooding reproducible by selecting the preinstalled Rust toolchain and explicitly
  omitting analyzers that require SchemaStore, link, package-registry, or vulnerability-database
  network access.
- Made the fixed report boundary writable across differing host and container user IDs, and retained
  bounded private adapter output so pre-reporter engine failures remain diagnosable without exposing
  raw workflow output.
- Restored a clean self-dogfood package-quality baseline by excluding generated contracts, build
  output, and hostile fixtures from production-source checks; Biome SARIF is also forced color-free
  for deterministic JSON parsing.
- Routed the local commit hook through the canonical package-quality adapter instead of legacy
  flat-ESLint and abandoned Prettier-mirror invocations with incompatible path semantics and
  divergent tool pins.
- Distinguished absolute interpreter shebangs from Rust crate-level inner attributes when enforcing
  executable Git modes.
- Resolved Node package export maps and subpath exports in the JavaScript architecture adapter
  without package-specific exceptions.

## [0.1.0-alpha.1] - Unreleased

Reserved for the first reviewed alpha release. No artifact is asserted to have been published under
this version.

[Unreleased]: https://github.com/egohygiene/egolint/compare/main...HEAD
