# Changelog

All notable changes will be documented here. This project follows Keep a
Changelog and intends to use Semantic Versioning once its public contracts
stabilize.

## [Unreleased]

### Added

- Initial Rust CLI for configuration resolution, planning, runtime checks, and
  Docker/Podman orchestration.
- Fast, holistic, security, and dependency-debt MegaLinter policy profiles.
- Separate lightweight CLI and full lint-engine image definitions.
- Initial cargo-dist, security, configuration, and release documentation.
- Versioned config, profile, finding, suppression, evidence, tool-result,
  execution-plan, normalized-report, and repository-contract JSON Schemas.
- First-class `validate`, `lint`, `explain`, and `doctor` commands with
  compatibility aliases for the initial alpha command surface.
- Canonical executable Empathy and Hygiene repository-contract fixtures pinned
  to immutable semantic source commits and verified by local digests.
- Standalone MegaLinter and complementary-tool contract inventories with
  positive and negative fixtures.
- SHA-pinned CI validation for Rust, policy contracts, schemas, packaging, and
  the lightweight OCI image.
- Native portability/repository-contract rules, expiring suppressions,
  normalized SARIF, compact dependency-debt evidence, and editor/pre-commit
  integrations.
- Immutable-tree fix previews plus digest/base/post-tree-gated, index-aware
  reviewed patch application.

### Changed

- Made the fast profile's mixed scope explicit: adapter linting targets changed
  files while native policy checks inspect the complete repository; legacy
  version-1 `changed_files` contracts remain readable.
- Aligned built-in profile purposes with the canonical MegaLinter policy and
  documented the boundary between structural JSON Schema checks and Rust/CLI
  semantic validation.
- Expanded run reports with honest completeness, normalized summary, finding,
  suppression, per-tool result, evidence, and ownership contracts.
- Made MegaLinter tool ownership, policy source, and evidence structured in the
  generated 124-tool inventory.
- Rebased the complete non-generated lint subsystem from Empathy into an
  independent repository contract.
- Removed generated Empathy evidence, operating-system metadata, and language
  caches from version control.

## [0.1.0-alpha.1] - Unreleased

Reserved for the first reviewed alpha release. No artifact is asserted to have
been published under this version.

[Unreleased]: https://github.com/egohygiene/egolint/compare/main...HEAD
