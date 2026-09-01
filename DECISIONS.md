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
