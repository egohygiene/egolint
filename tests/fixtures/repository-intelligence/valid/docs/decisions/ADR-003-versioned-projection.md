---
schema: egohygiene.architecture-decision/v1
id: ADR-003
title: Use a versioned projection
status: accepted
date: 2026-08-25
decision_scope: repository
visibility: public
owners: [egohygiene/egolint]
issue: https://github.com/egohygiene/egolint/issues/24
pull_request: https://github.com/egohygiene/egolint/pull/24
related: []
supersedes: [ADR-002]
superseded_by: []
affected_repositories: [egohygiene/egolint]
affected_contracts: [egohygiene.repository-intelligence/v1]
implementation_status: implemented
evidence:
  - type: implementation
    url: https://github.com/egohygiene/egolint/pull/24
    description: Reviewed implementation pull request.
exceptions: []
approval:
  date: 2026-08-25
  by: egohygiene-maintainer
  evidence: https://github.com/egohygiene/egolint/pull/24
extensions: {}
---

# ADR-003: Use a versioned projection

## Context
Consumers require structured output.
## Decision
Emit a versioned Repository Intelligence validation report.
## Alternatives considered and rejected
Prose parsing was rejected as unstable.
## Consequences and tradeoffs
The contract must be versioned and tested.
## Implementation and evidence links
Pull request 24 contains the implementation.
## Replacement or exit strategy
A future major contract can supersede this record.
## Follow-up work
Relay will consume the normalized artifact.
