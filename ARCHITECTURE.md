---
schema: aether.architecture-document/v1
id: egolint-architecture
title: Egolint Architecture
kind: architecture-document
version: 0.1.0
status: provisional
owners:
  - egohygiene
created: 2026-08-19
updated: 2026-08-19
governed_by:
  - architecture-architecture
depends_on:
  - egolint-foundations
  - egolint-system
related:
  - egolint-purpose
  - egolint-vision
  - egolint-principles
  - egolint-pillars
supersedes: []
---

# Egolint Architecture

## Purpose and scope

Egolint uses a layered, contract-driven architecture. This document owns structural boundaries,
dependency direction, integration rules, and current-to-target evolution. Logical responsibilities
remain canonical in [SYSTEM.md](SYSTEM.md).

## Layer model

1. **Intent and contracts** — identity, policy, specifications, schemas, and accepted decisions.
2. **Domain** — canonical concepts and pure domain behavior.
3. **Application** — planning, orchestration, use cases, and state transitions.
4. **Adapters** — filesystems, providers, frameworks, renderers, and external tools.
5. **Interfaces** — CLI, library, site, reports, generated artifacts, and automation contracts.
6. **Evidence** — tests, diagnostics, provenance, manifests, and health projections.

Dependencies point inward toward stable contracts and domain behavior. External details do not
become canonical domain truth.

## Structural view

```mermaid
flowchart LR
  S1[Profile catalog]
  S2[MegaLinter adapter]
  S3[Tool configuration]
  S4[Applicability resolver]
  S5[Report normalizer]
  S6[CLI contract]
  S7[Test fixtures]
  S1 --> S2
  S2 --> S3
  S3 --> S4
  S4 --> S5
  S5 --> S6
  S6 --> S7
```

The diagram is conceptual. [SYSTEM.md](SYSTEM.md) remains authoritative for responsibilities and
implementation evidence determines current availability.

## Dependency rules

- Sibling domain capabilities integrate through versioned public contracts, not direct access to
  internals.
- Generated artifacts never become the canonical source unless an accepted decision explicitly
  changes ownership.
- Provider and platform adapters depend on application ports; core behavior does not depend on a
  provider implementation.
- Read, plan, apply, verify, publish, and recover remain separate authority boundaries when
  consequential.
- Cross-repository references use releases, immutable commits, schemas, packages, or documented APIs
  rather than mutable default-branch assumptions.

## Ecosystem interfaces

- Hygiene policy
- Empathy baseline
- Relay workflows
- Pace conformance
- Observatory reporting

## Deployment and portability

The architecture favors independently usable local and self-hosted operation. Optional managed
services may add availability, collaboration, support, and hosted infrastructure without becoming
the canonical holder of portable state.

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
