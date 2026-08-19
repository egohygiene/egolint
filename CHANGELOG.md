# Changelog

All notable changes will be documented here. This project follows Keep a
Changelog and intends to use Semantic Versioning once its public contracts
stabilize.

## [Unreleased]

### Added

- Initial Rust CLI for configuration resolution, planning, runtime checks, and
  Docker/Podman orchestration.
- Fast and holistic MegaLinter policy profiles.
- Separate lightweight CLI and full lint-engine image definitions.
- Initial cargo-dist, security, configuration, and release documentation.
- Versioned config, execution-plan, and normalized-report JSON Schemas.
- Standalone MegaLinter and complementary-tool contract inventories with
  positive and negative fixtures.
- SHA-pinned CI validation for Rust, policy contracts, schemas, packaging, and
  the lightweight OCI image.

### Changed

- Rebased the complete non-generated lint subsystem from Empathy into an
  independent repository contract.
- Removed generated Empathy evidence, operating-system metadata, and language
  caches from version control.

## [0.1.0-alpha.1] - Unreleased

Reserved for the first reviewed alpha release. No artifact is asserted to have
been published under this version.

[Unreleased]: https://github.com/egohygiene/egolint/compare/main...HEAD
