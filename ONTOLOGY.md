---
schema: aether.architecture-document/v1
id: egolint-ontology
title: Egolint Ontology
kind: architecture-document
version: 0.1.0
status: provisional
owners:
  - egohygiene
created: 2026-08-19
updated: 2026-08-19
governed_by:
  - architecture-ontology
depends_on:
  - egolint-purpose
  - egolint-vision
  - egolint-principles
  - egolint-epistemology
related:
  - egolint-pillars
  - egolint-manifesto
  - egolint-ai-constitution
  - egolint-personal-model
supersedes: []
---

# Egolint Ontology

## Domain scope

Egolint models the concepts needed for turn organization quality expectations into reproducible,
explainable, and extensible validation. The ontology names conceptual entities and relationships; it
is not a source-code class model, API schema, or database design.

## Canonical concepts

| Concept            | Meaning                                                                                                              |
| ------------------ | -------------------------------------------------------------------------------------------------------------------- |
| Quality profile    | A canonical concept in the Egolint domain whose exact fields belong to specifications or schemas, not this ontology. |
| Linter             | A canonical concept in the Egolint domain whose exact fields belong to specifications or schemas, not this ontology. |
| Formatter          | A canonical concept in the Egolint domain whose exact fields belong to specifications or schemas, not this ontology. |
| Finding            | A canonical concept in the Egolint domain whose exact fields belong to specifications or schemas, not this ontology. |
| Suppression        | A canonical concept in the Egolint domain whose exact fields belong to specifications or schemas, not this ontology. |
| Baseline           | A canonical concept in the Egolint domain whose exact fields belong to specifications or schemas, not this ontology. |
| Report             | A canonical concept in the Egolint domain whose exact fields belong to specifications or schemas, not this ontology. |
| Applicability rule | A canonical concept in the Egolint domain whose exact fields belong to specifications or schemas, not this ontology. |
| Quality gate       | A canonical concept in the Egolint domain whose exact fields belong to specifications or schemas, not this ontology. |

## Core relationships

- A repository or person provides source context to one or more domain artifacts.
- A specification constrains how an artifact is interpreted or produced.
- A plan separates proposed action from execution.
- Evidence supports a claim; a decision authorizes a durable direction.
- Provenance connects derived artifacts to their inputs and processing context.
- A consumer integrates through an explicit interface rather than internal structure.

## Boundaries

- Conceptual identity is distinct from filesystem path, database identifier, or display label.
- Observed state is distinct from desired state.
- Proposed relationships are not accepted facts.
- Neighboring repositories retain ownership of their domain concepts.

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
