---
schema: aether.architecture-document/v1
id: egolint-decisions
title: Egolint Decisions
kind: architecture-document
version: 0.1.0
status: provisional
owners:
  - egohygiene
created: 2026-08-19
updated: 2026-08-19
governed_by:
  - architecture-decisions
depends_on:
  - egolint-principles
  - egolint-epistemology
  - egolint-foundations
  - egolint-system
  - egolint-architecture
related:
  - egolint-purpose
  - egolint-vision
  - egolint-pillars
  - egolint-manifesto
supersedes: []
---

# Egolint Decisions

## Purpose

This document preserves significant accepted architectural choices and their rationale. Issues
coordinate work, proposals explore alternatives, and this file records decisions that constrain
future implementation.

## Governance

Do not rewrite historical context to fit current understanding. Amend a record for corrections that
do not change meaning; supersede it with a new record when the decision changes materially.

## Index

- ADR-001: Wrap rather than fork specialized linters
- ADR-002: Keep holistic and universal profiles distinct
- ADR-003: Aggregate reports under a stable repository-owned location
- ADR-004: Validate ignore semantics through a focused metadata-only command

## ADR-001: Wrap rather than fork specialized linters

- **Status:** Accepted as the current architectural direction
- **Date:** 2026-08-19
- **Context:** Repository evidence and ecosystem ownership require an explicit durable boundary.
- **Decision:** Wrap rather than fork specialized linters.
- **Consequences:** The choice improves ownership and predictability while requiring maintained
  contracts, validation, and migration discipline.
- **Reconsider when:** New evidence shows that the boundary prevents standalone usefulness, safety,
  portability, or maintainability.

## ADR-002: Keep holistic and universal profiles distinct

- **Status:** Accepted as the current architectural direction
- **Date:** 2026-08-19
- **Context:** Repository evidence and ecosystem ownership require an explicit durable boundary.
- **Decision:** Keep holistic and universal profiles distinct.
- **Consequences:** The choice improves ownership and predictability while requiring maintained
  contracts, validation, and migration discipline.
- **Reconsider when:** New evidence shows that the boundary prevents standalone usefulness, safety,
  portability, or maintainability.

## ADR-003: Aggregate reports under a stable repository-owned location

- **Status:** Accepted as the current architectural direction
- **Date:** 2026-08-19
- **Context:** Repository evidence and ecosystem ownership require an explicit durable boundary.
- **Decision:** Aggregate reports under a stable repository-owned location.
- **Consequences:** The choice improves ownership and predictability while requiring maintained
  contracts, validation, and migration discipline.
- **Reconsider when:** New evidence shows that the boundary prevents standalone usefulness, safety,
  portability, or maintainability.

## ADR-004: Validate ignore semantics through a focused metadata-only command

- **Status:** Proposed in issue #61; accepted only with maintainer review/merge.
- **Date:** 2026-09-19
- **Context:** Required-file checks cannot prove that nested ignore policies protect local state
  while keeping intended source visible. General native inventory reads ordinary file payloads,
  which is outside this capability's authorized evidence boundary.
- **Decision:** Add `egolint gitignore` with pinned Empathy inputs, exact composition validation,
  bounded policy/path inventory, isolated actual Git matching, and separate tracked-file checks.
  Reuse normalized reports and findings. Keep unavailable evidence and exact reviewed exceptions
  explicit. Do not fetch sources, generate policy, or change the consumer during validation.
- **Consequences:** Callers supply a reviewed source export, composition, selection, and date.
  Git is required; finite probes and supported topology limit coverage. The general `validate`
  and lint-engine paths do not implicitly enable this capability. Future Relay adapters invoke
  the focused command after acceptance; source reconciliation remains Empathy #92.
- **Reconsider when:** The native inventory gains an equivalent metadata-only boundary or a new
  accepted composition version needs a separately reviewed adapter.

## Open decisions

- Release and compatibility policy for the first stable version.
- Exact self-hosted, managed, and organization-integrated deployment boundaries.
- Which target systems must exist before the architecture status may become active.

## Evidence and uncertainty

- **Observed:** The repository README establishes the intended boundary as a universal linting
  platform and extensible MegaLinter wrapper for consistent repository quality; significant
  implementation remains incomplete.
- **Decided for this draft:** The repository owns the bounded concern described here and
  participates through versioned contracts.
- **Proposed:** Target systems and later roadmap phases remain proposals until accepted and
  implemented.
- **Open question:** Which parts of this draft should become active in the first independently
  versioned release?
