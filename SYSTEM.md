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
updated: 2026-08-19
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

This document identifies Egolint's logical systems and responsibilities. It answers what the major systems do; [ARCHITECTURE.md](ARCHITECTURE.md) owns their structural organization and dependency rules.

## System inventory

| System | State | Responsibility |
| --- | --- | --- |
| Profile catalog | Target | Owns its bounded portion of a universal linting platform and extensible MegaLinter wrapper for consistent repository quality; exposes explicit inputs, outputs, failure states, and evidence. |
| MegaLinter adapter | Target | Owns its bounded portion of a universal linting platform and extensible MegaLinter wrapper for consistent repository quality; exposes explicit inputs, outputs, failure states, and evidence. |
| Tool configuration | Target | Owns its bounded portion of a universal linting platform and extensible MegaLinter wrapper for consistent repository quality; exposes explicit inputs, outputs, failure states, and evidence. |
| Applicability resolver | Target | Owns its bounded portion of a universal linting platform and extensible MegaLinter wrapper for consistent repository quality; exposes explicit inputs, outputs, failure states, and evidence. |
| Report normalizer | Target | Owns its bounded portion of a universal linting platform and extensible MegaLinter wrapper for consistent repository quality; exposes explicit inputs, outputs, failure states, and evidence. |
| CLI contract | Target | Owns its bounded portion of a universal linting platform and extensible MegaLinter wrapper for consistent repository quality; exposes explicit inputs, outputs, failure states, and evidence. |
| Test fixtures | Target | Owns its bounded portion of a universal linting platform and extensible MegaLinter wrapper for consistent repository quality; exposes explicit inputs, outputs, failure states, and evidence. |

## External systems

- Hygiene policy
- Empathy baseline
- Relay workflows
- Pace conformance
- Observatory reporting

External systems are integrations, not hidden implementation units. Each requires version, authentication, availability, data, error, and replacement boundaries appropriate to its risk.

## System interactions

Inputs enter through an adapter or validated contract, move through domain systems, produce artifacts and diagnostics, and leave through a stable interface. Evidence flows back to validation, review, and future decisions.

## Failure model

Systems fail closed at destructive, publication, privacy, and security boundaries. Partial results identify coverage and remain distinguishable from complete success.

## Evidence and uncertainty

- **Observed:** The repository README establishes the intended boundary as a universal linting platform and extensible MegaLinter wrapper for consistent repository quality; significant implementation remains incomplete.
- **Decided for this draft:** The repository owns the bounded concern described here and participates through versioned contracts.
- **Proposed:** Target systems and later roadmap phases remain proposals until accepted and implemented.
- **Open question:** Which parts of this draft should become active in the first independently versioned release?
