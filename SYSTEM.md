---
schema: aether.architecture-document/v1
id: egolint-system
title: Egolint System
kind: architecture-document
version: 0.1.0
status: provisional
owners:
  - egohygiene
created: 2026-08-19
updated: 2026-11-09
governed_by:
  - architecture-system
depends_on:
  - egolint-foundations
  - egolint-ontology
related:
  - egolint-purpose
  - egolint-vision
  - egolint-principles
  - egolint-pillars
supersedes: []
---

# Egolint System

## Purpose and scope

This document identifies Egolint's logical systems and responsibilities. It answers what the major
systems do; [ARCHITECTURE.md](ARCHITECTURE.md) owns their structural organization and dependency
rules.

## System inventory

| System                         | Source state                    | Responsibility and owner                                                                                                                                                                                                    |
| ------------------------------ | ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Profile catalog                | Implemented                     | `.config/megalinter/policy.yml`, snapshots and tool matrix own pinned fast, holistic, security and dependency-debt selections; `.config/toolchain` records complementary integrations.                                      |
| Configuration and CLI          | Implemented                     | `src/config.rs` resolves validated layered inputs with provenance; `src/main.rs` owns public commands and exit behavior. CI omits implicit user/local layers.                                                               |
| Native policy                  | Implemented                     | `src/rules` owns bounded portability, repository contracts, continuity, Intelligence, presentation, release and Git-ignore evaluation. Source locks preserve independent policy authority.                                  |
| Engine orchestration           | Implemented                     | `src/plan.rs` constructs inspectable Docker/Podman argv; the repository is mounted read-only and the engine owns MegaLinter execution.                                                                                      |
| Adapter normalization          | Implemented, coverage explicit  | `src/megalinter.rs` accepts supported evidence into `src/contracts.rs`; run JSON/SARIF distinguish adapter-only, partial and normalized coverage.                                                                           |
| JavaScript adapters            | Implemented, separately invoked | `src/javascript_architecture.rs` and `scripts/javascript-package-quality.mjs` normalize the pinned dependency-cruiser, Oxlint, Biome and publint integrations.                                                              |
| Debt and suppressions          | Implemented                     | `src/debt.rs` projects compact debt; `src/rules/suppressions.rs` preserves explicit, expiring suppression evidence.                                                                                                         |
| Fix preview/apply              | Implemented                     | `src/fix.rs` produces isolated bounded patches and applies only an exact reviewed checksum/base/post-tree combination.                                                                                                      |
| Adaptive adoption              | In review                       | [PR #112](https://github.com/egohygiene/egolint/pull/112) contains the general capability registry, progressive enforcement and optional advisory ast-grep adapter; it is separate from this release dependency candidate.  |
| Release adapter                | Source candidate                | `.github/workflows/release.yml` and `scripts/validate_release_request.py` bind manual publication to reviewed main, successful rehearsal and a matching immutable version tag. Signed publication/adoption remain unproven. |
| Fixtures and package contracts | Implemented                     | `tests`, `schemas` and `scripts/package_integrations.py` exercise source behavior and package consumer interfaces. A package verification does not establish a published distribution.                                      |

## External systems

- Hygiene and Aether own the immutable upstream policy/schema selections.
- Empathy owns baseline policy; Holon owns downstream materialization/recovery.
- Relay owns reusable acquisition, collection and publication profiles.
- Pace and Observatory can consume normalized evidence through their own adoption gates.

External systems are integrations, not hidden implementation units. Each requires version,
authentication, availability, data, error, and replacement boundaries appropriate to its risk.

## System interactions

The CLI resolves configuration and native policy, constructs an engine plan when needed, and
normalizes actual tool results before applying explicit suppressions and writing run JSON/SARIF.
Focused policy reports preserve their own semantic state. Fix and release workflows use separate
explicit paths; an ordinary validation run neither fixes source nor publishes artifacts.

## Failure model

Systems fail closed at destructive, publication, privacy, and security boundaries. Partial results
identify coverage and remain distinguishable from complete success.

## Evidence and uncertainty

- **Observed:** The source owners above implement the core CLI, adapters and reports; no GitHub
  release was observed on 2026-11-09.
- **Current limit:** CLI OCI platforms are AMD64 and ARM64; the full MegaLinter engine is AMD64-only.
  Planned products and their evidence requirements are in [release guidance](docs/releasing.md).
- **In review:** General adaptive adoption is tracked by PR #112, and manual publisher source by
  the current #5/#101 dependency candidate.
- **Deferred:** Publication, digest-pinned consumer adoption, broader platform support and fleet
  rollout require their owning acceptance evidence.
